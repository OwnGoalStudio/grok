// `pid_t` and Rust's PID type diff in sign.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::WaitHandle;

// Prevent process leaking when something goes wrong.
const FUSE_DURATION_SEC: u32 = 60;
const WAIT_DURATION: Duration = Duration::from_secs(1);
const TOLERANCE: Duration = Duration::from_millis(500);

fn _assert_wait_handle_send(h: WaitHandle) -> impl Send {
    h
}

fn command_for_targets(unix_args: &[&str], windows_args: &[&str]) -> Command {
    let args = if cfg!(unix) {
        unix_args
    } else if cfg!(windows) {
        windows_args
    } else {
        unreachable!();
    };
    let mut command = Command::new(args[0]);
    command.args(&args[1..]);
    command
}

#[test]
fn timeout() {
    let pid = std::process::id() as i32;
    let mut pid = WaitHandle::open(pid).unwrap();
    assert!(pid.wait_timeout(Duration::ZERO).unwrap().is_none());

    let inst = Instant::now();
    assert!(pid.wait_timeout(WAIT_DURATION).unwrap().is_none());
    let elapsed = inst.elapsed();
    let diff = elapsed
        .checked_sub(WAIT_DURATION)
        .or_else(|| WAIT_DURATION.checked_sub(elapsed))
        .unwrap();
    assert!(diff < TOLERANCE, "poor timeout precision? {elapsed:?}");
}

#[test]
fn invalid() {
    WaitHandle::open(-1).expect_err("-1 should not be valid");
}

#[test]
fn non_existing() {
    WaitHandle::open(0xDEAD).expect_err("should not exist");
}

// Only Linux and Windows support opening zombie processes.
#[cfg(any(target_os = "linux", windows))]
#[test]
fn zombie() {
    use std::io::{ErrorKind, Read};

    let mut child = command_for_targets(&["true"], &["cmd", "/c"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn");
    let pid = child.id();

    // Wait the the children to exit without explicit `wait`.
    // So it is still an unreaped zombie.
    let mut stdout = child.stdout.take().unwrap();
    loop {
        match stdout.read(&mut [0u8]) {
            Ok(0) => break,
            Ok(_) => unreachable!(),
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => panic!("read failed: {e}"),
        }
    }

    // Lock the zombie.
    let mut pid = WaitHandle::open(pid as _).expect("should open zombie");

    // Reap it, but the locked handle is still valid below.
    child.wait().unwrap();

    let inst = Instant::now();
    pid.wait_timeout(Duration::ZERO).unwrap().unwrap();
    // Extra waits should also return immediately.
    pid.wait_timeout(Duration::new(1, 0)).unwrap().unwrap();
    pid.wait().unwrap();
    let elapsed = inst.elapsed();
    assert!(elapsed < TOLERANCE, "no blocking");
}

#[test]
fn child() {
    let mut child = command_for_targets(&["sleep", "5"], &["timeout", "5"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn");

    // Lock the child.
    let mut pid = WaitHandle::open(child.id() as _).unwrap();
    assert!(pid.wait_timeout(Duration::ZERO).unwrap().is_none());

    // Kill the child and wait for its exit.
    child.kill().unwrap();
    child.wait().unwrap();

    let inst = Instant::now();
    pid.wait_timeout(Duration::ZERO).unwrap().unwrap();
    // Extra waits should also return immediately.
    pid.wait_timeout(Duration::new(1, 0)).unwrap().unwrap();
    pid.wait().unwrap();
    let elapsed = inst.elapsed();
    assert!(elapsed < TOLERANCE, "no blocking");
}

#[test]
fn non_child() {
    let mut child = command_for_targets(
        &[
            "sh",
            "-c",
            &format!(
                "sleep {FUSE_DURATION_SEC} &\n\
                    echo $!"
            ),
        ],
        &[
            "powershell",
            "-Command",
            &format!(
                "(Start-Process \
                        -PassThru \
                        -FilePath timeout \
                        -ArgumentList {FUSE_DURATION_SEC} \
                        -WindowStyle Hidden \
                    ).Id"
            ),
        ],
    )
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
    .unwrap();

    // Get the output but keep the zombie to hold direct child's PID.
    let raw_pid = {
        let mut buf = String::new();
        BufReader::new(child.stdout.as_mut().unwrap())
            .read_line(&mut buf)
            .unwrap();
        buf.trim().parse::<i32>().unwrap()
    };

    // Sanity check.
    assert!(raw_pid >= 2);
    assert_ne!(child.id(), raw_pid as u32);

    // Grandchildren should not be children.
    #[cfg(unix)]
    let rustix_pid = {
        use rustix::io::Errno;
        use rustix::process::{waitpid, Pid, WaitOptions};

        let pid = Pid::from_raw(raw_pid).unwrap();
        assert_eq!(
            waitpid(Some(pid), WaitOptions::NOHANG).unwrap_err(),
            Errno::CHILD
        );
        pid
    };

    let mut pid = WaitHandle::open(raw_pid).unwrap();
    assert!(pid.wait_timeout(Duration::ZERO).unwrap().is_none());

    let wait_thread = thread::spawn(move || pid.wait());

    // Wait for some time to make sure no unexpected returns.
    thread::sleep(WAIT_DURATION);
    assert!(
        !wait_thread.is_finished(),
        "Returned while the process still alive: {:?}",
        wait_thread.join(),
    );

    let inst = Instant::now();

    // Kill the grandchild, and it should return in time.
    #[cfg(unix)]
    rustix::process::kill_process(rustix_pid, rustix::process::Signal::TERM).unwrap();
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };

        let hprocess = OpenProcess(PROCESS_TERMINATE, 0 /* No inherit */, raw_pid as u32);
        if hprocess.is_null() {
            panic!(
                "failed to open process: {}",
                std::io::Error::last_os_error()
            );
        }
        let ret = TerminateProcess(hprocess, 1);
        if ret == 0 {
            panic!(
                "failed to terminate process: {}",
                std::io::Error::last_os_error()
            );
        }
        CloseHandle(hprocess);
    }

    wait_thread
        .join()
        .expect("must not panic")
        .expect("must succeed");
    let elapsed = inst.elapsed();
    assert!(elapsed < TOLERANCE, "Wait for too long? {elapsed:?}");

    // Reap the direct child.
    child.wait().unwrap();
}
