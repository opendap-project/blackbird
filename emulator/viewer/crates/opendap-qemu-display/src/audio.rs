//! The sound of QEMU's D-Bus display (`-audiodev dbus`, interfaces `org.qemu.Display1.Audio*`).
//!
//! - Each stream the guest opens arrives as `Init` with its format, then `SetEnabled`, then its
//!   data, then `Fini`. With the audiodev's `fixed-settings=off` the format is the guest's own (the
//!   rate, sample format and channels the guest's driver chose), so what [`Sink::write`] receives
//!   is the PCM the guest played, unconverted.
//! - Output: QEMU sends `Write` without waiting for an answer; a slow sink only delays its own
//!   connection.
//! - Input: QEMU calls `Read` and waits for the answer on its audio path, so a [`Source`] answers
//!   at once from what it has (fewer bytes, or silence, rather than waiting).

use std::io;
use std::sync::Arc;

use zbus::blocking::Connection;

use crate::{Session, serve};

/// A stream's PCM format, as QEMU describes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub bits: u8,
    pub signed: bool,
    pub float: bool,
    pub rate: u32,
    pub channels: u8,
    pub bytes_per_frame: u32,
    pub big_endian: bool,
}

/// Where the guest's output goes. Called on the listener's thread.
pub trait Sink: Send + Sync + 'static {
    fn init(&self, stream: u64, format: Format);
    fn fini(&self, stream: u64);
    fn enabled(&self, stream: u64, on: bool);
    /// The guest's mixer: muted, and a volume per channel (0–255).
    fn volume(&self, _stream: u64, _mute: bool, _volume: &[u8]) {}
    fn write(&self, stream: u64, data: &[u8]);
}

/// Where the guest's input comes from. Called on the listener's thread; `read` answers at once.
pub trait Source: Send + Sync + 'static {
    fn init(&self, stream: u64, format: Format);
    fn fini(&self, stream: u64);
    fn enabled(&self, stream: u64, on: bool);
    fn volume(&self, _stream: u64, _mute: bool, _volume: &[u8]) {}
    /// Up to `size` bytes of whole frames.
    fn read(&self, stream: u64, size: usize) -> Vec<u8>;
}

#[allow(clippy::too_many_arguments)]
fn format(
    bits: u8,
    signed: bool,
    float: bool,
    rate: u32,
    channels: u8,
    bytes_per_frame: u32,
    big_endian: bool,
) -> Format {
    Format {
        bits,
        signed,
        float,
        rate,
        channels,
        bytes_per_frame,
        big_endian,
    }
}

struct OutListener(Arc<dyn Sink>);

#[zbus::interface(name = "org.qemu.Display1.AudioOutListener")]
impl OutListener {
    #[allow(clippy::too_many_arguments)]
    fn init(
        &self,
        id: u64,
        bits: u8,
        is_signed: bool,
        is_float: bool,
        freq: u32,
        nchannels: u8,
        bytes_per_frame: u32,
        _bytes_per_second: u32,
        be: bool,
    ) {
        self.0.init(
            id,
            format(
                bits,
                is_signed,
                is_float,
                freq,
                nchannels,
                bytes_per_frame,
                be,
            ),
        );
    }
    fn fini(&self, id: u64) {
        self.0.fini(id);
    }
    fn set_enabled(&self, id: u64, enabled: bool) {
        self.0.enabled(id, enabled);
    }
    fn set_volume(&self, id: u64, mute: bool, volume: Vec<u8>) {
        self.0.volume(id, mute, &volume);
    }
    fn write(&self, id: u64, data: Vec<u8>) {
        self.0.write(id, &data);
    }
}

struct InListener(Arc<dyn Source>);

#[zbus::interface(name = "org.qemu.Display1.AudioInListener")]
impl InListener {
    #[allow(clippy::too_many_arguments)]
    fn init(
        &self,
        id: u64,
        bits: u8,
        is_signed: bool,
        is_float: bool,
        freq: u32,
        nchannels: u8,
        bytes_per_frame: u32,
        _bytes_per_second: u32,
        be: bool,
    ) {
        self.0.init(
            id,
            format(
                bits,
                is_signed,
                is_float,
                freq,
                nchannels,
                bytes_per_frame,
                be,
            ),
        );
    }
    fn fini(&self, id: u64) {
        self.0.fini(id);
    }
    fn set_enabled(&self, id: u64, enabled: bool) {
        self.0.enabled(id, enabled);
    }
    fn set_volume(&self, id: u64, mute: bool, volume: Vec<u8>) {
        self.0.volume(id, mute, &volume);
    }
    fn read(&self, id: u64, size: u64) -> Vec<u8> {
        self.0.read(id, size as usize)
    }
}

/// A listener while it is attached; dropping it detaches it.
pub struct Listening {
    _connection: Connection,
}

const AUDIO: &str = "/org/qemu/Display1/Audio";
const AUDIO_IFACE: &str = "org.qemu.Display1.Audio";

impl Session {
    /// The guest's output streams, to `sink` (QEMU started with `-audiodev dbus`).
    pub fn audio_out(&self, sink: Arc<dyn Sink>) -> io::Result<Listening> {
        let c = serve(
            self,
            AUDIO,
            AUDIO_IFACE,
            "RegisterOutListener",
            "/org/qemu/Display1/AudioOutListener",
            OutListener(sink),
        )?;
        Ok(Listening { _connection: c })
    }

    /// The guest's input streams, from `source`.
    pub fn audio_in(&self, source: Arc<dyn Source>) -> io::Result<Listening> {
        let c = serve(
            self,
            AUDIO,
            AUDIO_IFACE,
            "RegisterInListener",
            "/org/qemu/Display1/AudioInListener",
            InListener(source),
        )?;
        Ok(Listening { _connection: c })
    }
}
