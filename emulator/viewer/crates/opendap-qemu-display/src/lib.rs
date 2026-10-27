//! A client of QEMU's D-Bus display (`-display dbus,p2p=yes`, interfaces `org.qemu.Display1`):
//! what a simulator window needs, and nothing more.
//!
//! - Attached peer to peer through QMP: a socket pair, one end handed to QEMU with `getfd` and
//!   `add_client` (no bus daemon). QMP is needed only for that; the display lives on its own
//!   connection after the QMP session closes.
//! - A listener registered on the first console: QEMU pushes each change as the guest draws it
//!   (`Scanout`, a whole frame; `Update`, a rectangle). Unlike VNC it does not back off while the
//!   display is idle, so a short transition arrives whole, at QEMU's display refresh (about
//!   30 ms).
//! - On the listener's connection QEMU is the server: it authenticates the client, so this end is
//!   the client.
//! - The absolute pointer through `org.qemu.Display1.Mouse` (QEMU routes it to the guest's
//!   tablet).
//! - The sound (`-audiodev dbus`, [`audio`]): listeners for the guest's output streams (the PCM
//!   the guest plays, exactly as it sent it) and input streams (what the guest records, supplied
//!   here). QEMU allows several listeners at once, so a window and a recorder can both listen.

use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

use zbus::blocking::{Connection, MessageIterator, Proxy, connection::Builder};

pub mod audio;
use zbus::zvariant::Fd;

/// The frame as QEMU last drew it: `width × height` pixels, B, G, R, X bytes, rows packed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgrx: Vec<u8>,
}

/// What a message from QEMU changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Pixels changed in this rectangle.
    Pixels { x: u32, y: u32, w: u32, h: u32 },
    /// A whole new frame, possibly of another size.
    Scanout,
    /// The display was turned off (the guest released it).
    Disabled,
}

/// pixman's formats whose bytes are B, G, R, X in memory (little-endian `0x00RRGGBB`): what
/// QEMU's virtio-gpu draws.
const X8R8G8B8: u32 = 0x2002_0888;
const A8R8G8B8: u32 = 0x2002_8888;

fn other(msg: impl Into<String>) -> io::Error {
    io::Error::other(msg.into())
}

/// Rows of `stride` bytes into rows of `w × 4`: the frame's own layout.
fn packed(data: &[u8], w: u32, h: u32, stride: u32) -> Option<Vec<u8>> {
    let row = w as usize * 4;
    let stride = stride as usize;
    if h == 0 || stride < row || data.len() < stride * (h as usize - 1) + row {
        return None;
    }
    Some(
        (0..h as usize)
            .flat_map(|y| &data[y * stride..y * stride + row])
            .copied()
            .collect(),
    )
}

impl Frame {
    /// A whole frame.
    fn scanout(&mut self, w: u32, h: u32, stride: u32, format: u32, data: &[u8]) -> bool {
        if format != X8R8G8B8 && format != A8R8G8B8 {
            return false;
        }
        let Some(p) = packed(data, w, h, stride) else {
            return false;
        };
        *self = Frame {
            width: w,
            height: h,
            bgrx: p,
        };
        true
    }

    /// A rectangle of it, clipped to the frame.
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        stride: u32,
        format: u32,
        data: &[u8],
    ) -> Option<Change> {
        if format != X8R8G8B8 && format != A8R8G8B8 || x < 0 || y < 0 || w <= 0 || h <= 0 {
            return None;
        }
        let (x, y, w, h) = (x as u32, y as u32, w as u32, h as u32);
        if x + w > self.width || y + h > self.height {
            return None;
        }
        let p = packed(data, w, h, stride)?;
        let (row, frame_row) = (w as usize * 4, self.width as usize * 4);
        for r in 0..h as usize {
            let at = (y as usize + r) * frame_row + x as usize * 4;
            self.bgrx[at..at + row].copy_from_slice(&p[r * row..(r + 1) * row]);
        }
        Some(Change::Pixels { x, y, w, h })
    }
}

type OnChange = Box<dyn Fn(&Frame, Change) + Send + Sync>;

struct Listener {
    frame: Arc<Mutex<Frame>>,
    on_change: Arc<OnChange>,
}

#[zbus::interface(name = "org.qemu.Display1.Listener")]
impl Listener {
    fn scanout(&self, width: u32, height: u32, stride: u32, pixman_format: u32, data: Vec<u8>) {
        let mut f = self.frame.lock().unwrap_or_else(|p| p.into_inner());
        if f.scanout(width, height, stride, pixman_format, &data) {
            (self.on_change)(&f, Change::Scanout);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        stride: u32,
        pixman_format: u32,
        data: Vec<u8>,
    ) {
        let mut f = self.frame.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(c) = f.update(x, y, w, h, stride, pixman_format, &data) {
            (self.on_change)(&f, c);
        }
    }

    fn disable(&self) {
        let f = self.frame.lock().unwrap_or_else(|p| p.into_inner());
        (self.on_change)(&f, Change::Disabled);
    }

    // The guest's own pointer drawing: a simulator window shows the finger, not a cursor.
    fn mouse_set(&self, _x: i32, _y: i32, _on: i32) {}
    fn cursor_define(&self, _width: i32, _height: i32, _hot_x: i32, _hot_y: i32, _data: Vec<u8>) {}
}

/// The display, attached: its frame, its pointer, and the end of the session.
pub struct Display {
    session: Session,
    _listener: Connection,
    frame: Arc<Mutex<Frame>>,
}

/// A QMP command and its reply (events in between are skipped).
fn qmp(
    stream: &UnixStream,
    reader: &mut BufReader<UnixStream>,
    cmd: &serde_json::Value,
) -> io::Result<serde_json::Value> {
    let mut s = stream;
    s.write_all(format!("{cmd}\n").as_bytes())?;
    read_reply(reader)
}

fn read_reply(reader: &mut BufReader<UnixStream>) -> io::Result<serde_json::Value> {
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Err(other("QMP closed the connection"));
        }
        let v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| other(format!("QMP: {e}")))?;
        if v.get("event").is_some() {
            continue;
        }
        if let Some(e) = v.get("error") {
            return Err(other(format!(
                "QMP: {}",
                e["desc"].as_str().unwrap_or("error")
            )));
        }
        return Ok(v);
    }
}

/// `getfd` with the descriptor in the same message (SCM_RIGHTS), as QMP requires.
fn getfd(stream: &UnixStream, name: &str, fd: &OwnedFd) -> io::Result<()> {
    let msg = format!(
        "{}\n",
        serde_json::json!({"execute": "getfd", "arguments": {"fdname": name}})
    );
    // SAFETY: a message header over a live buffer and a control buffer of the size CMSG_SPACE
    // gives for one descriptor; the descriptor is valid for the call.
    unsafe {
        let mut iov = libc::iovec {
            iov_base: msg.as_ptr() as *mut _,
            iov_len: msg.len(),
        };
        let space = libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize;
        let mut control = vec![0u8; space];
        let mut m: libc::msghdr = std::mem::zeroed();
        m.msg_iov = &mut iov;
        m.msg_iovlen = 1;
        m.msg_control = control.as_mut_ptr().cast();
        m.msg_controllen = space as _;
        let c = libc::CMSG_FIRSTHDR(&m);
        (*c).cmsg_level = libc::SOL_SOCKET;
        (*c).cmsg_type = libc::SCM_RIGHTS;
        (*c).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as _;
        std::ptr::write_unaligned(libc::CMSG_DATA(c) as *mut i32, fd.as_raw_fd());
        if libc::sendmsg(stream.as_raw_fd(), &m, 0) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// A session with QEMU's D-Bus display: what [`Session::display`] and [`Session::audio_out`] /
/// [`Session::audio_in`] attach to.
pub struct Session {
    console: Connection,
}

// `unix_stream` is deprecated only for builds that enable zbus's `tokio` feature, which this crate
// never does (async-io is its executor).
#[allow(deprecated)]
/// Connects to the D-Bus display of the QEMU whose QMP socket is `qmp` (a QMP session of its own:
/// QMP serves one client at a time, and this one closes once the display is reached).
pub fn connect(qmp_socket: &Path) -> io::Result<Session> {
    let stream = UnixStream::connect(qmp_socket)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    read_reply(&mut reader)?; // the greeting
    qmp(
        &stream,
        &mut reader,
        &serde_json::json!({"execute": "qmp_capabilities"}),
    )?;
    let (ours, theirs) = UnixStream::pair()?;
    getfd(&stream, "opendap-display", &OwnedFd::from(theirs))?;
    read_reply(&mut reader)?;
    qmp(&stream, &mut reader, &serde_json::json!({"execute": "add_client", "arguments": {"protocol": "@dbus-display", "fdname": "opendap-display"}}))
        .map_err(|e| other(format!("{e} (is QEMU started with -display dbus,p2p=yes?)")))?;
    drop(reader);
    drop(stream);
    let console = Builder::unix_stream(ours).p2p().build().map_err(zerr)?;
    Ok(Session { console })
}

fn zerr(e: zbus::Error) -> io::Error {
    other(format!("D-Bus display: {e}"))
}

/// A listener served on a connection of its own, the way QEMU's display wants them: one end of a
/// socket pair handed to QEMU with `method` on `object`, whose server QEMU is; it answers our
/// handshake only once it has the descriptor, so the handshake runs on a thread meanwhile.
#[allow(deprecated)]
fn serve<I: zbus::object_server::Interface>(
    session: &Session,
    object: &str,
    interface: &str,
    method: &str,
    path: &'static str,
    listener: I,
) -> io::Result<Connection> {
    let (mine, qemus) = UnixStream::pair()?;
    let handshake = std::thread::spawn(move || {
        Builder::unix_stream(mine)
            .p2p()
            .serve_at(path, listener)?
            .build()
    });
    let proxy = Proxy::new(&session.console, "org.qemu", object, interface).map_err(zerr)?;
    let qemus = OwnedFd::from(qemus);
    proxy
        .call::<_, _, ()>(method, &(Fd::from(&qemus),))
        .map_err(zerr)?;
    handshake
        .join()
        .map_err(|_| other("D-Bus display: a listener's handshake failed"))?
        .map_err(zerr)
}

/// Attaches to the D-Bus display of the QEMU whose QMP socket is `qmp` ([`connect`], then
/// [`Session::display`]).
pub fn attach(
    qmp_socket: &Path,
    on_change: impl Fn(&Frame, Change) + Send + Sync + 'static,
) -> io::Result<Display> {
    connect(qmp_socket)?.display(on_change)
}

impl Session {
    /// The display: a listener on the first console. `on_change` runs on the display's thread
    /// after each change, with the frame as it now is.
    pub fn display(
        self,
        on_change: impl Fn(&Frame, Change) + Send + Sync + 'static,
    ) -> io::Result<Display> {
        let frame = Arc::new(Mutex::new(Frame::default()));
        let on_change: Arc<OnChange> = Arc::new(Box::new(on_change));
        let listener = Listener {
            frame: frame.clone(),
            on_change,
        };
        let listener = serve(
            &self,
            "/org/qemu/Display1/Console_0",
            "org.qemu.Display1.Console",
            "RegisterListener",
            "/org/qemu/Display1/Listener",
            listener,
        )?;
        Ok(Display {
            session: self,
            _listener: listener,
            frame,
        })
    }

    /// Blocks until QEMU closes the session (the device stopped).
    pub fn wait_closed(&self) {
        for m in MessageIterator::from(&self.console) {
            if m.is_err() {
                break;
            }
        }
    }
}

impl Display {
    /// The frame as it is now.
    pub fn frame(&self) -> Frame {
        self.frame.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    fn mouse(&self) -> io::Result<Proxy<'_>> {
        Proxy::new(
            &self.session.console,
            "org.qemu",
            "/org/qemu/Display1/Console_0",
            "org.qemu.Display1.Mouse",
        )
        .map_err(|e| other(format!("D-Bus display: {e}")))
    }

    /// The pointer at a frame pixel; `pressed` presses or releases the primary button there.
    pub fn pointer(&self, x: u32, y: u32, pressed: Option<bool>) -> io::Result<()> {
        let m = self.mouse()?;
        let zerr = |e: zbus::Error| other(format!("D-Bus display: {e}"));
        m.call::<_, _, ()>("SetAbsPosition", &(x, y))
            .map_err(zerr)?;
        match pressed {
            Some(true) => m.call::<_, _, ()>("Press", &(0u32,)).map_err(zerr),
            Some(false) => m.call::<_, _, ()>("Release", &(0u32,)).map_err(zerr),
            None => Ok(()),
        }
    }

    /// Blocks until QEMU closes the display (the device stopped).
    pub fn wait_closed(&self) {
        self.session.wait_closed();
    }

    /// The session, to attach the sound to the same display.
    pub fn session(&self) -> &Session {
        &self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32) -> Frame {
        Frame {
            width: w,
            height: h,
            bgrx: vec![0; (w * h * 4) as usize],
        }
    }

    #[test]
    fn a_scanout_with_a_padded_stride_packs_its_rows() {
        let mut f = Frame::default();
        // 2 x 2, rows of 12 bytes (4 of padding).
        let data = [
            1, 1, 1, 1, 2, 2, 2, 2, 9, 9, 9, 9, 3, 3, 3, 3, 4, 4, 4, 4, 9, 9, 9, 9,
        ];
        assert!(f.scanout(2, 2, 12, X8R8G8B8, &data));
        assert_eq!(f.bgrx, [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4]);
    }

    #[test]
    fn an_update_lands_in_its_rectangle_and_nowhere_else() {
        let mut f = frame(3, 2);
        let c = f.update(1, 1, 2, 1, 8, X8R8G8B8, &[7, 7, 7, 7, 8, 8, 8, 8]);
        assert_eq!(
            c,
            Some(Change::Pixels {
                x: 1,
                y: 1,
                w: 2,
                h: 1
            })
        );
        assert_eq!(&f.bgrx[..12], &[0; 12], "the first row untouched");
        assert_eq!(&f.bgrx[12..], &[0, 0, 0, 0, 7, 7, 7, 7, 8, 8, 8, 8]);
    }

    #[test]
    fn what_does_not_fit_or_is_not_bgrx_is_ignored() {
        let mut f = frame(2, 2);
        assert_eq!(
            f.update(1, 1, 2, 1, 8, X8R8G8B8, &[0; 8]),
            None,
            "past the right edge"
        );
        assert_eq!(
            f.update(0, 0, 1, 1, 4, 0x1234, &[0; 4]),
            None,
            "another pixel format"
        );
        assert!(
            !f.scanout(2, 2, 8, X8R8G8B8, &[0; 10]),
            "data shorter than its rows"
        );
        assert_eq!(f, frame(2, 2));
    }
}
