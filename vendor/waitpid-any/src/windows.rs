use std::ffi::c_void;
use std::io::{Error, Result};
use std::ptr::NonNull;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
};

// windows_sys::Win32::Foundation::HANDLE = *mut c_void
#[derive(Debug)]
pub struct WaitHandle(NonNull<c_void>);

// SAFETY: HANDLE is transferrable and `wait()` takes a exclusive reference that prevents racing.
unsafe impl Send for WaitHandle {}

impl Drop for WaitHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0.as_ptr()) };
    }
}

pub fn open(pid: i32) -> Result<WaitHandle> {
    let hprocess = unsafe {
        OpenProcess(PROCESS_SYNCHRONIZE, 0 /* No inherit */, pid as u32)
    };
    let hprocess = NonNull::new(hprocess).ok_or_else(Error::last_os_error)?;
    Ok(WaitHandle(hprocess))
}

pub fn wait(hprocess: &mut WaitHandle, timeout: Option<Duration>) -> Result<Option<()>> {
    // `INFINITE` is `u32::MAX`.
    const _: [(); 1] = [(); (INFINITE == u32::MAX) as usize];

    let timeout = match timeout {
        // Do not set it to INFINITE when overflow.
        Some(dur) => dur
            .as_millis()
            .try_into()
            .unwrap_or(INFINITE - 1)
            .min(INFINITE - 1),
        None => INFINITE,
    };
    let ret = unsafe { WaitForSingleObject(hprocess.0.as_ptr(), timeout) };
    match ret {
        WAIT_OBJECT_0 => Ok(Some(())),
        WAIT_TIMEOUT => Ok(None),
        _ => Err(Error::last_os_error()),
    }
}
