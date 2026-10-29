//! One frame, when one is due (display_policy decides): the clients, the composer's own layers above
//! them, a screenshot when one is asked, the wakes the next frames need, and the frame queued.

use super::*;
use crate::display_policy::{FrameState, arm_wake, keep_drawing, keep_last_frame, may_draw};

/// Draws if something changed and no frame is waiting for the display.
pub fn render_if_idle(composer: &mut Composer) {
    let handle = composer.handle.clone();
    let Composer {
        space,
        output,
        backend,
        system,
        ..
    } = composer;
    let super::super::state::System {
        touches, layers, ..
    } = system;
    let (Some(backend), Some(output)) = (backend.as_mut(), output.as_ref()) else {
        return;
    };
    let state = FrameState {
        display_off: backend.display_off,
        frame_pending: backend.frame_pending,
        redraw: backend.redraw,
        session_active: backend.session.is_active(),
    };
    if !may_draw(state) {
        return;
    }
    backend.redraw = false;
    let scale = UtilScale::from(output.current_scale().fractional_scale());
    let windows = client_elements(space, backend, scale);
    if !windows.is_empty() {
        client_ready(backend);
    } else if keep_last_frame(
        false,
        backend.splash.done(),
        layers.as_ref().is_some_and(|l| l.power_open()),
    ) {
        return;
    }
    let (elements, touch_marks) = stack(touches, layers, backend, windows, scale);
    if std::mem::take(&mut backend.screenshot) {
        screenshot(
            &handle,
            backend,
            layers,
            output,
            &elements[touch_marks..],
            scale,
        );
    }
    if keep_drawing(
        backend.splash.animating(),
        touches.animating(),
        layers.as_ref().is_some_and(|l| l.animating()),
    ) {
        backend.redraw = true;
    }
    arm_wakes(&handle, backend, touches);
    submit(backend, space, output, &elements);
}

/// The clients' surfaces, where the space puts them.
fn client_elements(
    space: &smithay::desktop::Space<smithay::desktop::Window>,
    backend: &mut Backend,
    scale: UtilScale<f64>,
) -> Vec<ComposerElement> {
    let mut windows: Vec<ComposerElement> = Vec::new();
    for window in space.elements() {
        let location = space
            .element_location(window)
            .unwrap_or_default()
            .to_physical_precise_round(scale);
        windows.extend(window.render_elements::<ComposerElement>(
            &mut backend.renderer,
            location,
            scale,
            1.0,
        ));
    }
    windows
}

/// The product host's first frame lets the splash screen go (splash.rs), and booting ends: the
/// status LED stops its booting state.
fn client_ready(backend: &mut Backend) {
    backend.splash.client_ready();
    static READY: std::sync::Once = std::sync::Once::new();
    READY.call_once(|| {
        let _ = std::fs::write(blackbird_platform::run::COMPOSER_UI_READY, b"");
    });
}

/// Everything on screen, top first: the touch indicator, the system layers, the splash screen,
/// the clients; and where the touch indicator's elements end.
fn stack(
    touches: &mut super::super::touches::Touches,
    layers: &mut Option<super::super::layers::SystemLayers>,
    backend: &mut Backend,
    windows: Vec<ComposerElement>,
    scale: UtilScale<f64>,
) -> (Vec<ComposerElement>, usize) {
    let mut elements: Vec<ComposerElement> = Vec::with_capacity(windows.len() + 3);
    elements.extend(
        touches
            .elements(&mut backend.renderer, scale.x)
            .into_iter()
            .map(ComposerElement::Memory),
    );
    let touch_marks = elements.len();
    elements.extend(
        layers
            .as_mut()
            .and_then(|l| l.element(&mut backend.renderer))
            .map(ComposerElement::Memory),
    );
    elements.extend(
        backend
            .splash
            .element(&mut backend.renderer)
            .map(ComposerElement::Memory),
    );
    elements.extend(windows);
    (elements, touch_marks)
}

/// A screenshot shows what the display shows, without the touch indicator (a debug aid), and a
/// toast says whether it was saved.
fn screenshot(
    handle: &smithay::reexports::calloop::LoopHandle<'static, Composer>,
    backend: &mut Backend,
    layers: &mut Option<super::super::layers::SystemLayers>,
    output: &Output,
    shown: &[ComposerElement],
    scale: UtilScale<f64>,
) {
    let size = output.current_mode().map(|m| m.size).unwrap_or_default();
    let saved = super::super::screenshot::capture(&mut backend.renderer, shown, size, scale, CLEAR);
    let message = match saved {
        Ok(path) => {
            info!(path = %path.display(), "screenshot");
            "Screenshot saved"
        }
        Err(err) => {
            warn!(%err, "screenshot");
            "Screenshot failed"
        }
    };
    let Some(l) = layers.as_mut() else {
        return;
    };
    l.show_toast(message);
    let _ = handle.insert_source(Timer::from_duration(TOAST), |_, _, composer| {
        if let Some(l) = composer.system.layers.as_mut() {
            l.hide_toast();
        }
        if let Some(b) = composer.backend.as_mut() {
            b.schedule();
        }
        render_if_idle(composer);
        TimeoutAction::Drop
    });
    backend.redraw = true;
}

/// One-shot wakes, never a periodic timer: the resting pointer's bubble fading out, and the
/// splash screen's minimum ending (a static frame has no damage, so no refresh comes).
fn arm_wakes(
    handle: &smithay::reexports::calloop::LoopHandle<'static, Composer>,
    backend: &mut Backend,
    touches: &super::super::touches::Touches,
) {
    if let Some(at) = arm_wake(touches.wake_at(), backend.touches_timer) {
        backend.touches_timer = true;
        let _ = handle.insert_source(Timer::from_deadline(at), |_, _, composer| {
            if let Some(b) = composer.backend.as_mut() {
                b.touches_timer = false;
                b.redraw = true;
            }
            render_if_idle(composer);
            TimeoutAction::Drop
        });
    }
    if let Some(at) = arm_wake(backend.splash.wake_at(), backend.splash_timer) {
        backend.splash_timer = true;
        let _ = handle.insert_source(Timer::from_deadline(at), |_, _, composer| {
            if let Some(b) = composer.backend.as_mut() {
                b.splash_timer = false;
                b.redraw = true;
            }
            render_if_idle(composer);
            TimeoutAction::Drop
        });
    }
}

/// The frame rendered and queued; when nothing visible changed, no frame, but clients may draw
/// their next one.
fn submit(
    backend: &mut Backend,
    space: &smithay::desktop::Space<smithay::desktop::Window>,
    output: &Output,
    elements: &[ComposerElement],
) {
    match backend.compositor.render_frame(
        &mut backend.renderer,
        elements,
        CLEAR,
        FrameFlags::DEFAULT,
    ) {
        Ok(result) if !result.is_empty => match backend.compositor.queue_frame(()) {
            Ok(()) => backend.frame_pending = true,
            Err(err) => warn!(?err, "queue frame"),
        },
        Ok(_) => send_frames(space, output, backend.start.elapsed()),
        Err(err) => warn!(?err, "render frame"),
    }
}
