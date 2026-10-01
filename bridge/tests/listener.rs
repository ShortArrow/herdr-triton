//! The listener's Windows mechanisms, against the real kernel objects under
//! names unique to each test.
#![cfg(windows)]

use std::io::{self, Read};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use bridge::listener::{running, send, spawn_detached, Claim, Names, Signal};

fn names() -> Names {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    Names::new(&format!("herdr-triton-test-{}-{n}", std::process::id()))
}

#[test]
fn names_live_in_the_user_session() {
    let names = Names::new("herdr-triton-listener");
    assert_eq!(
        (
            names.mutex.as_str(),
            names.wake.as_str(),
            names.stop.as_str()
        ),
        (
            r"Local\herdr-triton-listener",
            r"Local\herdr-triton-listener-wake",
            r"Local\herdr-triton-listener-stop"
        )
    );
}

mod claim {
    use super::*;

    #[test]
    fn a_claim_marks_a_listener_running_until_it_is_dropped() {
        let names = names();
        assert!(!running(&names));
        let claim = Claim::take(&names).unwrap().unwrap();
        assert!(running(&names));
        drop(claim);
        assert!(!running(&names));
    }

    #[test]
    fn a_second_claim_is_refused_while_the_first_is_held() {
        let names = names();
        let _first = Claim::take(&names).unwrap().unwrap();
        assert!(Claim::take(&names).unwrap().is_none());
    }
}

mod signals {
    use super::*;

    #[test]
    fn a_wait_without_a_signal_times_out() {
        let names = names();
        let claim = Claim::take(&names).unwrap().unwrap();
        let started = Instant::now();
        assert_eq!(claim.wait(Duration::from_millis(50)).unwrap(), None);
        assert!(started.elapsed() >= Duration::from_millis(40));
    }

    #[test]
    fn a_wake_and_a_stop_each_end_one_wait() {
        let names = names();
        let claim = Claim::take(&names).unwrap().unwrap();
        send(&names, Signal::Wake).unwrap();
        assert_eq!(
            claim.wait(Duration::from_secs(1)).unwrap(),
            Some(Signal::Wake)
        );
        assert_eq!(claim.wait(Duration::from_millis(10)).unwrap(), None);
        send(&names, Signal::Stop).unwrap();
        assert_eq!(
            claim.wait(Duration::from_secs(1)).unwrap(),
            Some(Signal::Stop)
        );
    }

    #[test]
    fn a_stop_wins_over_a_wake() {
        let names = names();
        let claim = Claim::take(&names).unwrap().unwrap();
        send(&names, Signal::Wake).unwrap();
        send(&names, Signal::Stop).unwrap();
        assert_eq!(
            claim.wait(Duration::from_secs(1)).unwrap(),
            Some(Signal::Stop)
        );
    }

    #[test]
    fn a_signal_without_a_listener_is_not_found() {
        let err = send(&names(), Signal::Wake).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}

/// herdr reads a hook's stdout until EOF, so a listener spawned by a hook
/// must not inherit it (ADR 0012).
mod spawn {
    use super::*;
    use std::os::windows::io::{AsRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT};
    use windows_sys::Win32::System::Console::{GetStdHandle, SetStdHandle, STD_OUTPUT_HANDLE};

    /// The child: lives long enough that holding the pipe would show.
    #[test]
    #[ignore = "run as the spawned child"]
    fn child_sleeps() {
        std::thread::sleep(Duration::from_secs(3));
    }

    /// The child: writes whether it has a console to the file it is given.
    #[test]
    #[ignore = "run as the spawned child"]
    fn child_reports_its_console() {
        use windows_sys::Win32::System::Console::GetConsoleCP;
        let Some(out) = std::env::var_os("HERDR_TRITON_TEST_OUT") else {
            return;
        };
        let console = unsafe { GetConsoleCP() } != 0;
        std::fs::write(out, if console { "console" } else { "none" }).unwrap();
    }

    #[test]
    fn a_detached_child_has_no_console() {
        let out =
            std::env::temp_dir().join(format!("herdr-triton-test-console-{}", std::process::id()));
        let mut child = spawn_detached(
            Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "spawn::child_reports_its_console"])
                .env("HERDR_TRITON_TEST_OUT", &out),
        )
        .unwrap();
        child.wait().unwrap();
        let seen = std::fs::read_to_string(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        assert_eq!(seen, "none");
    }

    #[test]
    fn a_detached_child_does_not_hold_an_inherited_stdout_open() {
        let (mut reader, writer) = io::pipe().unwrap();
        let writer = OwnedHandle::from(writer);
        let raw = writer.as_raw_handle() as _;
        let saved = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        let mut child = unsafe {
            SetHandleInformation(raw, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT);
            SetStdHandle(STD_OUTPUT_HANDLE, raw);
            let child = spawn_detached(Command::new(std::env::current_exe().unwrap()).args([
                "--ignored",
                "--exact",
                "spawn::child_sleeps",
            ]));
            SetStdHandle(STD_OUTPUT_HANDLE, saved);
            child.unwrap()
        };
        drop(writer);
        let started = Instant::now();
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).unwrap();
        let waited = started.elapsed();
        let _ = child.kill();
        assert!(waited < Duration::from_secs(1), "EOF after {waited:?}");
    }
}
