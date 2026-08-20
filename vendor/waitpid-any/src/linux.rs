use std::io::{Error, ErrorKind, Result};
use std::time::Duration;

use rustix::event::{poll, PollFd, PollFlags};
use rustix::io::Errno;
use rustix::process::{pidfd_open, Pid, PidfdFlags};

pub type WaitHandle = rustix::fd::OwnedFd;

pub fn open(pid: i32) -> Result<WaitHandle> {
    let pid = Pid::from_raw(pid)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, format!("invalid PID {pid}")))?;
    let pidfd = pidfd_open(pid, PidfdFlags::empty())?;
    Ok(pidfd)
}

pub fn wait(pidfd: &mut WaitHandle, timeout: Option<Duration>) -> Result<Option<()>> {
    let timespec = match timeout {
        Some(dur) => Some(dur.try_into().map_err(|_| Errno::INVAL)?),
        // Infinite.
        None => None,
    };
    let mut fds = [PollFd::new(&pidfd, PollFlags::IN)];
    let ret = poll(&mut fds, timespec.as_ref())?;
    if ret == 0 {
        // Timeout.
        return Ok(None);
    }
    debug_assert!(fds[0].revents().contains(PollFlags::IN));
    Ok(Some(()))
}
