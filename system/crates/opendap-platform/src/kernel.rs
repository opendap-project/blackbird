//! The kernel's event sources, as non-blocking descriptors for a program's event loop: uevents
//! (devices, batteries and chargers coming and going) and inotify (files written or removed in a
//! directory). Whatever wakes a descriptor, the program drains it and reads its state again; nothing polls.

use std::{
    ffi::CString,
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

/// What a directory watch reports: entries created, removed, written or moved in or out.
pub const DIR_CHANGES: u32 = libc::IN_CREATE
    | libc::IN_DELETE
    | libc::IN_CLOSE_WRITE
    | libc::IN_MOVED_TO
    | libc::IN_MOVED_FROM;

/// A socket that receives the kernel's uevents.
pub fn uevents() -> io::Result<OwnedFd> {
    // SAFETY: plain socket calls with a zeroed sockaddr_nl of the right size; the descriptor is
    // owned once created.
    unsafe {
        let fd = libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            libc::NETLINK_KOBJECT_UEVENT,
        );
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let owned = OwnedFd::from_raw_fd(fd);
        let mut addr: libc::sockaddr_nl = std::mem::zeroed();
        addr.nl_family = libc::AF_NETLINK as u16;
        addr.nl_groups = 1;
        if libc::bind(
            fd,
            (&raw const addr).cast(),
            std::mem::size_of::<libc::sockaddr_nl>() as u32,
        ) < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(owned)
    }
}

/// An inotify instance watching each of `dirs` for [`DIR_CHANGES`]; a directory that cannot be
/// watched is an error.
pub fn inotify(dirs: &[&str]) -> io::Result<OwnedFd> {
    // SAFETY: inotify_init1 has no preconditions; the descriptor is owned once created.
    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh descriptor nothing else owns.
    let owned = unsafe { OwnedFd::from_raw_fd(fd) };
    for d in dirs {
        if !watch(&owned, d) {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(owned)
}

/// Adds `dir` to an inotify instance (adding it twice is harmless); false when it cannot be
/// watched, for example because it does not exist yet.
pub fn watch(fd: &OwnedFd, dir: &str) -> bool {
    let Ok(c) = CString::new(dir) else {
        return false;
    };
    // SAFETY: a valid descriptor and C string.
    unsafe { libc::inotify_add_watch(fd.as_raw_fd(), c.as_ptr(), DIR_CHANGES) >= 0 }
}

/// Empties a non-blocking descriptor.
pub fn drain(fd: &impl AsRawFd) {
    let mut buf = [0u8; 8192];
    // SAFETY: reads at most the buffer's size into it.
    while unsafe { libc::read(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) } > 0 {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_file_wakes_the_watch() {
        let dir = std::env::temp_dir().join(format!("opendap-kernel-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fd = inotify(&[dir.to_str().unwrap()]).unwrap();
        std::fs::write(dir.join("flag"), b"1").unwrap();
        let mut p = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid pollfd.
        assert_eq!(unsafe { libc::poll(&mut p, 1, 1000) }, 1);
        drain(&fd);
        // SAFETY: as above; drained, nothing is pending.
        assert_eq!(unsafe { libc::poll(&mut p, 1, 0) }, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_directory_cannot_be_watched() {
        assert!(inotify(&["/nonexistent/opendap"]).is_err());
    }
}
