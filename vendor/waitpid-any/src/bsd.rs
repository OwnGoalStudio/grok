use std::io::{Error, ErrorKind, Result};
use std::mem::MaybeUninit;
use std::time::Duration;

use rustix::event::kqueue::{kevent, kqueue, Event, EventFilter, EventFlags, ProcessEvents};
use rustix::process::Pid;

// KQueue only emits the event once. So we will store the exited state to make `wait` returns
// `Ok(Some(()))` immediately after the event fires.
#[derive(Debug)]
pub enum WaitHandle {
    KQueue(rustix::fd::OwnedFd),
    Exited,
}

pub fn open(pid: i32) -> Result<WaitHandle> {
    let pid = Pid::from_raw(pid)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, format!("invalid PID {pid}")))?;
    let kqueue = kqueue()?;
    let event = Event::new(
        EventFilter::Proc {
            pid,
            flags: ProcessEvents::EXIT,
        },
        EventFlags::ADD,
        std::ptr::null_mut(),
    );
    let ret = unsafe { kevent::<_, &mut [Event; 0]>(&kqueue, &[event], &mut [], None)? };
    debug_assert_eq!(ret, 0);
    Ok(WaitHandle::KQueue(kqueue))
}

pub fn wait(handle: &mut WaitHandle, timeout: Option<Duration>) -> Result<Option<()>> {
    let kqueue = match handle {
        WaitHandle::KQueue(kqueue) => kqueue,
        WaitHandle::Exited => return Ok(Some(())),
    };

    let mut buf = [MaybeUninit::uninit()];
    let (events, _rest_buf) = unsafe { kevent(&kqueue, &[], &mut buf, timeout)? };
    // Timeout.
    if events.is_empty() {
        return Ok(None);
    }

    // Process exited. Fuse the handle.
    debug_assert!(matches!(
        events[0].filter(),
        EventFilter::Proc { flags, .. }
        if flags.contains(ProcessEvents::EXIT)
    ));
    *handle = WaitHandle::Exited;
    Ok(Some(()))
}
