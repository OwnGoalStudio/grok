//! `waitpid(2)` but for arbitrary non-child processes.
//!
//! [`waitpid(2)`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/wait.html) can only
//! be used to wait for direct child processes, or it fails immediately.
//!
//! This crate provides a extension to wait for the exit of any process, not necessarily child
//! processes. Due to platform limitations, the exit reason and status codes still cannot be
//! retrieved.
//!
//! ## Implementation details
//!
//! - On Linux, [`pidfd_open(2)`](https://man7.org/linux/man-pages/man2/pidfd_open.2.html) and
//!   [`poll(2)`](https://man7.org/linux/man-pages/man2/poll.2.html) are used. Thus only Linux 5.3
//!   or later is supported.
//! - On Windows,
//!   [`OpenProcess`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)
//!   and
//!   [`WaitForSingleObject`](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject)
//!   are used.
//! - On *BSD, including macOS and iOS,
//!   [`kqueue(2)`](https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2) is used.
//! - Other platforms are not supported currently.
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
use std::io::Result;
use std::time::Duration;

#[cfg(target_os = "linux")]
#[path = "./linux.rs"]
mod imp;

#[cfg(any(
    target_os = "freebsd",
    target_os = "macos",
    target_os = "ios",
    target_os = "netbsd",
    target_os = "openbsd",
))]
#[path = "./bsd.rs"]
mod imp;

#[cfg(windows)]
#[path = "./windows.rs"]
mod imp;

#[cfg(test)]
mod tests;

/// A locked handle to a process.
///
/// See [`WaitHandle::open`] for more details.
#[derive(Debug)]
#[must_use = "`WaitHandle` does nothing unless you `wait` it"]
pub struct WaitHandle(imp::WaitHandle);

impl WaitHandle {
    /// Open an handle to the process with given PID.
    ///
    /// The opened handle always points to the same process entity, thus preventing race condition
    /// caused by PID reusing.
    ///
    /// # Errors
    ///
    /// Fails when the underlying syscall fails.
    ///
    /// # Caveats
    ///
    /// 1. PID itself does not own any resource in most platforms. Thus there is still a race
    ///    condition when the process pointed by the original PID is dead, reaped, and recycled all
    ///    before calling to this function. This is generally unavoidable. But you can try to
    ///    `open` the PID as soon as possible, before any potential `wait` operations, to mitigate
    ///    the issue.
    /// 2. If the given PID does not exists, it returns `ESRCH` on *NIX immediately. This can
    ///    also happen if the process is exited and reaped before this call. You may want to
    ///    regards this case as a successful wait, but the decision is up to you.
    pub fn open(pid: i32) -> Result<Self> {
        Ok(Self(imp::open(pid)?))
    }

    /// Blocks until the target process exits.
    ///
    /// Once the the target process exits, all following calls return `Ok(())` immediately.
    ///
    /// # Errors
    ///
    /// Fails when the underlying syscall fails. For *NIX platforms, `EINTR` may be returned in
    /// case of signals.
    // WAIT: https://github.com/rust-lang/rust-clippy/issues/11436
    #[allow(clippy::missing_panics_doc)]
    pub fn wait(&mut self) -> Result<()> {
        imp::wait(&mut self.0, None)?.expect("no timeout");
        Ok(())
    }

    /// Blocks until the target process exits, or timeout.
    ///
    /// If the process exited in time, `Ok(Some(()))` is returned immediately when the event
    /// triggers. If it is not exited in `timeout`, `Ok(None)` is returned.
    /// Once the the target process exits, all following calls return `Ok(Some(()))` immediately.
    ///
    /// # Errors
    ///
    /// Fails when the underlying syscall fails. For *NIX platforms, `EINTR` may be returned in
    /// case of signals.
    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<()>> {
        imp::wait(&mut self.0, Some(timeout))
    }
}
