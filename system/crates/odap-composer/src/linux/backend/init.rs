//! Bringing the display up, step by step: the GPU and its renderer, the output and its mode, the
//! DRM compositor, the event sources, and the system layers at the display's size.

use smithay::reexports::drm::control::{Mode, connector, crtc};

use super::*;
use crate::display_policy::{ModeOffer, choose_mode, physical_size};

/// The display device and its buffers.
struct Gpu {
    drm: DrmDevice,
    notifier: smithay::backend::drm::DrmDeviceNotifier,
    gbm: GbmDevice<DrmDeviceFd>,
    node: Option<DrmNode>,
}

pub fn init(composer: &mut Composer) -> Result<(), Error> {
    let (mut session, notifier) =
        LibSeatSession::new().map_err(|e| Error::Session(e.to_string()))?;
    let seat = session.seat();
    let mut gpu = open_gpu(&mut session, &seat)?;
    let renderer = renderer(&gpu.gbm)?;
    let (connector, crtc, mode) = pick_output(&gpu.drm, composer)?;
    let (output, scale) = make_output(composer, &connector, mode);
    let compositor = make_compositor(&mut gpu, &output, crtc, mode, &connector, &renderer)?;
    composer
        .protocols
        .shm_state
        .update_formats(renderer.shm_formats());
    let libinput = event_sources(composer, &session, &seat, gpu.notifier, notifier)?;
    composer.output = Some(output);
    composer.backend = Some(Backend {
        splash: Splash::new((mode.size().0 as i32, mode.size().1 as i32).into()),
        splash_timer: false,
        touches_timer: false,
        session,
        libinput,
        renderer,
        compositor,
        drm: gpu.drm,
        start: std::time::Instant::now(),
        frame_pending: false,
        redraw: true,
        display_off: false,
        screenshot: false,
    });
    system_layers(composer, mode, scale);
    // The status bar, from the kernel's classes and the clock.
    super::super::status::start(composer);
    render_if_idle(composer);
    Ok(())
}

/// The seat's primary GPU, opened through the session.
fn open_gpu(session: &mut LibSeatSession, seat: &str) -> Result<Gpu, Error> {
    let path: PathBuf = udev::primary_gpu(seat)
        .ok()
        .flatten()
        .or_else(|| udev::all_gpus(seat).ok()?.into_iter().next())
        .ok_or(Error::NoGpu)?;
    info!(gpu = %path.display(), "display device");
    let fd = session
        .open(
            &path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )
        .map_err(|e| Error::Session(e.to_string()))?;
    let fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (drm, notifier) =
        DrmDevice::new(fd.clone(), true).map_err(|e| Error::Drm(e.to_string()))?;
    let gbm = GbmDevice::new(fd).map_err(|e| Error::Drm(e.to_string()))?;
    let node = DrmNode::from_file(gbm.as_fd()).ok();
    Ok(Gpu {
        drm,
        notifier,
        gbm,
        node,
    })
}

/// The GLES renderer on the GPU.
fn renderer(gbm: &GbmDevice<DrmDeviceFd>) -> Result<GlesRenderer, Error> {
    // Safety: the EGL display lives as long as the renderer, which this backend owns.
    let egl = unsafe { EGLDisplay::new(gbm.clone()) }.map_err(|e| Error::Render(e.to_string()))?;
    let context = EGLContext::new(&egl).map_err(|e| Error::Render(e.to_string()))?;
    unsafe { GlesRenderer::new(context) }.map_err(|e| Error::Render(e.to_string()))
}

/// The first connected connector, a CRTC it can use, and the mode to drive (display_policy).
fn pick_output(
    drm: &DrmDevice,
    composer: &Composer,
) -> Result<(connector::Info, crtc::Handle, Mode), Error> {
    let resources = drm
        .resource_handles()
        .map_err(|e| Error::Drm(e.to_string()))?;
    let connector = resources
        .connectors()
        .iter()
        .filter_map(|c| drm.get_connector(*c, true).ok())
        .find(|c| c.state() == connector::State::Connected)
        .ok_or(Error::NoConnector)?;
    let crtc = connector
        .encoders()
        .iter()
        .filter_map(|e| drm.get_encoder(*e).ok())
        .flat_map(|e| resources.filter_crtcs(e.possible_crtcs()))
        .next()
        .ok_or(Error::NoConnector)?;
    let native = composer
        .description
        .display
        .as_ref()
        .map(|d| (d.width_px as u16, d.height_px as u16));
    let modes = connector.modes();
    let offers: Vec<ModeOffer> = modes
        .iter()
        .map(|m| ModeOffer {
            size: m.size(),
            user_defined: m.mode_type().contains(ModeTypeFlags::USERDEF),
            preferred: m.mode_type().contains(ModeTypeFlags::PREFERRED),
        })
        .collect();
    let mode = choose_mode(&offers, native)
        .map(|i| modes[i])
        .ok_or(Error::NoConnector)?;
    Ok((connector, crtc, mode))
}

/// The Wayland output for the connector, at the description's scale; it and the scale.
fn make_output(composer: &mut Composer, connector: &connector::Info, mode: Mode) -> (Output, i32) {
    let display = composer.description.display.as_ref();
    let (w_mm, h_mm) = physical_size(connector.size(), display.map(|d| (d.width_mm, d.height_mm)));
    let scale = display
        .map(|d| d.scale().round().max(1.0) as i32)
        .unwrap_or(1);
    let name = format!(
        "{}-{}",
        connector.interface().as_str(),
        connector.interface_id()
    );
    let output = Output::new(
        name.clone(),
        PhysicalProperties {
            size: (w_mm, h_mm).into(),
            subpixel: Subpixel::Unknown,
            make: "OpenDAP".into(),
            model: composer.description.identity.name.clone(),
            serial_number: String::new(),
        },
    );
    let output_mode = OutputMode::from(mode);
    output.set_preferred(output_mode);
    output.change_current_state(
        Some(output_mode),
        Some(Transform::Normal),
        Some(Scale::Integer(scale)),
        Some((0, 0).into()),
    );
    output.create_global::<Composer>(&composer.display_handle);
    composer.space.map_output(&output, (0, 0));
    info!(connector = %name, w = mode.size().0, h = mode.size().1, w_mm, h_mm, scale, "output");
    (output, scale)
}

/// The DRM compositor on the CRTC, with GBM buffers and the planes it can use.
fn make_compositor(
    gpu: &mut Gpu,
    output: &Output,
    crtc: crtc::Handle,
    mode: Mode,
    connector: &connector::Info,
    renderer: &GlesRenderer,
) -> Result<Compositor, Error> {
    let surface = gpu
        .drm
        .create_surface(crtc, mode, &[connector.handle()])
        .map_err(|e| Error::Drm(e.to_string()))?;
    let allocator = GbmAllocator::new(
        gpu.gbm.clone(),
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );
    let exporter = GbmFramebufferExporter::new(
        gpu.gbm.clone(),
        gpu.node.map(NodeFilter::Node).unwrap_or(NodeFilter::All),
    );
    let planes = gpu.drm.planes(&crtc).ok();
    Compositor::new(
        output,
        surface,
        planes,
        allocator,
        exporter,
        [
            Fourcc::Argb8888,
            Fourcc::Xrgb8888,
            Fourcc::Abgr8888,
            Fourcc::Xbgr8888,
        ],
        renderer.dmabuf_formats(),
        (64, 64).into(),
        Some(gpu.gbm.clone()),
    )
    .map_err(|e| Error::Render(e.to_string()))
}

/// Input from every device of the seat (libinput), the display's vblanks, and the session's
/// pauses and resumptions.
fn event_sources(
    composer: &mut Composer,
    session: &LibSeatSession,
    seat: &str,
    drm_notifier: smithay::backend::drm::DrmDeviceNotifier,
    notifier: smithay::backend::session::libseat::LibSeatSessionNotifier,
) -> Result<Libinput, Error> {
    let mut libinput =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());
    libinput
        .udev_assign_seat(seat)
        .map_err(|_| Error::Session("libinput cannot use the seat".into()))?;
    composer
        .handle
        .insert_source(
            LibinputInputBackend::new(libinput.clone()),
            |event, _, composer| super::super::input::process(composer, event),
        )
        .map_err(|e| Error::Session(e.to_string()))?;
    composer
        .handle
        .insert_source(drm_notifier, |event, _, composer| match event {
            DrmEvent::VBlank(_) => frame_done(composer),
            DrmEvent::Error(err) => warn!(?err, "display"),
        })
        .map_err(|e| Error::Drm(e.to_string()))?;
    composer
        .handle
        .insert_source(notifier, |event, _, composer| {
            let Some(backend) = composer.backend.as_mut() else {
                return;
            };
            match event {
                SessionEvent::PauseSession => {
                    info!("session paused");
                    backend.libinput.suspend();
                }
                SessionEvent::ActivateSession => {
                    info!("session active");
                    if backend.libinput.resume().is_err() {
                        warn!("libinput did not resume");
                    }
                    if let Err(err) = backend.compositor.reset_state() {
                        warn!(?err, "display state reset");
                    }
                    backend.frame_pending = false;
                    backend.redraw = true;
                    render_if_idle(composer);
                }
            }
        })
        .map_err(|e| Error::Session(e.to_string()))?;
    Ok(libinput)
}

/// The system layers, at the display's size, with `main` where the description puts it, and the
/// physical controls for the debug UI mode (one mark per role).
fn system_layers(composer: &mut Composer, mode: Mode, scale: i32) {
    let description = &composer.description;
    let main_y = description
        .display
        .as_ref()
        .zip(description.control(blackbird_device::Role::Main))
        .map(|(d, c)| d.mm_to_px(c.offset_mm) / scale as f32)
        .unwrap_or(mode.size().1 as f32 / scale as f32 / 4.0);
    let mut controls: Vec<(String, String, f32, f32)> = Vec::new();
    if let Some(d) = description.display.as_ref() {
        for c in &description.controls {
            let role = c.role.as_str().to_string();
            if controls.iter().any(|(r, ..)| *r == role) {
                continue;
            }
            controls.push((
                role,
                format!("{:?}", c.edge).to_lowercase(),
                d.mm_to_px(c.offset_mm) / scale as f32,
                d.mm_to_px(c.length_mm) / scale as f32,
            ));
        }
    }
    // Logical pixels per design pixel: the hardware's exact scale over the applied one.
    let unit = description
        .display
        .as_ref()
        .map(|d| d.scale() / scale as f32)
        .unwrap_or(1.0);
    composer.system.layers = super::super::layers::SystemLayers::new(
        (mode.size().0 as u32, mode.size().1 as u32),
        scale as f32,
        main_y,
        controls,
        unit,
    );
}
