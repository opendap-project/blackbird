//! The host's audio streams, on a thread of their own (cpal's are not `Send` everywhere).

use super::*;

/// What the host's audio thread is asked.
pub(super) enum Cmd {
    /// Open the default output for `rate` × `channels`; the answer is the rate and channels it
    /// plays at, and a note when they differ.
    Output {
        rate: u32,
        channels: u16,
        ring: Arc<Mutex<Ring>>,
        reply: mpsc::Sender<Result<(u32, usize), String>>,
    },
    CloseOutput,
    /// Open an input (by id; the default without one); its samples go to `ring` at `rate` ×
    /// `channels`.
    Microphone {
        device: Option<String>,
        rate: u32,
        channels: usize,
        ring: Arc<Mutex<Ring>>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    CloseMicrophone,
}

pub(super) fn host_thread(rx: mpsc::Receiver<Cmd>) {
    let host = cpal::default_host();
    let (mut output, mut microphone): (Option<cpal::Stream>, Option<cpal::Stream>) = (None, None);
    for cmd in rx {
        match cmd {
            Cmd::Output {
                rate,
                channels,
                ring,
                reply,
            } => {
                output = None;
                let opened = open_output(&host, rate, channels, ring);
                let _ = reply.send(opened.map(|(stream, plays)| {
                    output = Some(stream);
                    plays
                }));
            }
            Cmd::CloseOutput => output = None,
            Cmd::Microphone {
                device,
                rate,
                channels,
                ring,
                reply,
            } => {
                microphone = None;
                let opened = open_microphone(&host, device.as_deref(), rate, channels, ring);
                let _ = reply.send(opened.map(|stream| microphone = Some(stream)));
            }
            Cmd::CloseMicrophone => microphone = None,
        }
    }
    drop((output, microphone));
}

/// The default output, playing what `ring` holds: at the guest's own rate and channels when the
/// device takes them, at its default otherwise. The stream, and the rate and channels it plays.
fn open_output(
    host: &cpal::Host,
    rate: u32,
    channels: u16,
    ring: Arc<Mutex<Ring>>,
) -> Result<(cpal::Stream, (u32, usize)), String> {
    let device = host
        .default_output_device()
        .ok_or("no output on this computer")?;
    let play = |config: cpal::StreamConfig| {
        let ring = ring.clone();
        device.build_output_stream::<f32, _, _>(
            config,
            move |data, _| ring.lock().unwrap_or_else(|p| p.into_inner()).pull(data),
            |e| eprintln!("blackbird-emulator-viewer: sound out: {e}"),
            None,
        )
    };
    // The guest's own rate and channels first; the device's default otherwise.
    let exact = cpal::StreamConfig {
        channels,
        sample_rate: rate,
        buffer_size: cpal::BufferSize::Default,
    };
    let (stream, config) = play(exact).map(|s| (s, exact)).or_else(|_| {
        let config = device
            .default_output_config()
            .map_err(|e| e.to_string())?
            .config();
        play(config).map(|s| (s, config)).map_err(|e| e.to_string())
    })?;
    let _ = stream.play();
    Ok((stream, (config.sample_rate, usize::from(config.channels))))
}

/// An input (by id; the default without one), its samples resampled to `rate` × `channels`
/// into `ring`.
fn open_microphone(
    host: &cpal::Host,
    device: Option<&str>,
    rate: u32,
    channels: usize,
    ring: Arc<Mutex<Ring>>,
) -> Result<cpal::Stream, String> {
    let device = match device {
        Some(id) => host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.id().is_ok_and(|i| i.to_string() == id))
            .ok_or("that input is gone")?,
        None => host
            .default_input_device()
            .ok_or("no input on this computer")?,
    };
    let config = device
        .default_input_config()
        .map_err(|e| e.to_string())?
        .config();
    let mut resampler = Resampler::new(
        config.sample_rate,
        usize::from(config.channels),
        rate,
        channels,
    );
    let mut converted = Vec::new();
    let stream = device
        .build_input_stream::<f32, _, _>(
            config,
            move |data, _| {
                converted.clear();
                resampler.process(data, &mut converted);
                ring.lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push(&converted);
            },
            |e| eprintln!("blackbird-emulator-viewer: sound in: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
