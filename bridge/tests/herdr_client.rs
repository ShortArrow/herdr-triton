//! Runs the client against a fake herdr listening on a real local socket,
//! bound the same way herdr binds it.

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use bridge::herdr::client::{CallError, Client, DEADLINE};
use bridge::herdr::wire::{Request, Response};
use interprocess::local_socket::{prelude::*, ListenerOptions, Stream};

fn unique_path() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("herdr-triton-test-{}-{n}.sock", std::process::id()))
}

fn bind(path: &Path) -> interprocess::local_socket::Listener {
    #[cfg(windows)]
    let name = {
        use interprocess::local_socket::GenericNamespaced;
        path.to_string_lossy()
            .to_string()
            .to_ns_name::<GenericNamespaced>()
            .unwrap()
    };
    #[cfg(unix)]
    let name = {
        use interprocess::local_socket::GenericFilePath;
        let _ = std::fs::remove_file(path);
        path.to_fs_name::<GenericFilePath>().unwrap()
    };
    ListenerOptions::new().name(name).create_sync().unwrap()
}

/// Serves `replies` on successive connections, one line in and one line out
/// per connection, then returns the request lines it received.
fn serve(replies: Vec<&'static str>) -> (PathBuf, thread::JoinHandle<Vec<String>>) {
    let path = unique_path();
    let listener = bind(&path);
    let handle = thread::spawn(move || {
        let mut received = Vec::new();
        for reply in replies {
            let conn: Stream = listener.accept().unwrap();
            let mut reader = BufReader::new(conn);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            received.push(line);
            let mut conn = reader.into_inner();
            conn.write_all(reply.as_bytes()).unwrap();
            conn.write_all(b"\n").unwrap();
        }
        received
    });
    (path, handle)
}

#[test]
fn a_call_sends_one_line_and_reads_one_response() {
    let (path, server) = serve(vec![
        r#"{"id":"1","result":{"type":"pong","version":"0.9.1","protocol":3}}"#,
    ]);
    let got = Client::new(path).call(&Request::Ping).unwrap();
    assert_eq!(
        got,
        Response::Pong {
            version: "0.9.1".into()
        }
    );
    let received = server.join().unwrap();
    assert_eq!(received.len(), 1);
    let sent: serde_json::Value = serde_json::from_str(&received[0]).unwrap();
    assert_eq!(sent["method"], "ping");
}

#[test]
fn each_call_opens_its_own_connection() {
    let (path, server) = serve(vec![
        r#"{"id":"1","result":{"type":"ok"}}"#,
        r#"{"id":"2","result":{"type":"ok"}}"#,
    ]);
    let client = Client::new(path);
    assert_eq!(
        client
            .call(&Request::AgentFocus { target: "p".into() })
            .unwrap(),
        Response::Ok
    );
    assert_eq!(
        client
            .call(&Request::AgentFocus { target: "q".into() })
            .unwrap(),
        Response::Ok
    );
    assert_eq!(server.join().unwrap().len(), 2);
}

#[test]
fn a_missing_socket_is_an_io_error() {
    let err = Client::new(unique_path()).call(&Request::Ping).unwrap_err();
    assert!(matches!(err, CallError::Io(_)), "{err:?}");
}

#[test]
fn a_garbled_reply_is_a_wire_error() {
    let (path, server) = serve(vec!["not json"]);
    let err = Client::new(path).call(&Request::Ping).unwrap_err();
    assert!(matches!(err, CallError::Wire(_)), "{err:?}");
    server.join().unwrap();
}

#[test]
fn a_reply_that_never_comes_times_out_at_the_deadline() {
    let path = unique_path();
    let listener = bind(&path);
    let server = thread::spawn(move || {
        let conn: Stream = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(&conn).read_line(&mut line).unwrap();
        thread::sleep(Duration::from_millis(1500));
    });
    let started = Instant::now();
    let err = Client::with_deadline(path, Duration::from_millis(300))
        .call(&Request::Ping)
        .unwrap_err();
    assert!(
        matches!(&err, CallError::Io(e) if e.kind() == io::ErrorKind::TimedOut),
        "{err:?}"
    );
    assert!(started.elapsed() < Duration::from_millis(1000));
    server.join().unwrap();
}

#[test]
fn the_default_deadline_is_two_seconds() {
    assert_eq!(DEADLINE, Duration::from_secs(2));
}

/// A pipe with a single instance, which one raw client keeps busy, so the
/// next `Client` finds every instance taken.
#[cfg(windows)]
mod busy_pipe {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_TYPE_BYTE, PIPE_WAIT,
    };

    fn pipe_name(path: &Path) -> String {
        format!(r"\\.\pipe\{}", path.display())
    }

    /// Creates the only instance and connects one raw client to it.
    fn single_busy_instance(path: &Path) -> (OwnedHandle, File) {
        let wide: Vec<u16> = pipe_name(path).encode_utf16().chain([0]).collect();
        let raw = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                std::ptr::null(),
            )
        };
        let server = unsafe { OwnedHandle::from_raw_handle(raw as _) };
        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .open(pipe_name(path))
            .unwrap();
        (server, holder)
    }

    /// After `after`, drops the holder and answers the next client once.
    fn free_after(server: OwnedHandle, holder: File, after: Duration) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            thread::sleep(after);
            drop(holder);
            let h = server.as_raw_handle() as _;
            unsafe {
                DisconnectNamedPipe(h);
                ConnectNamedPipe(h, std::ptr::null_mut());
            }
            let mut conn = File::from(server);
            let mut line = String::new();
            BufReader::new(&conn).read_line(&mut line).unwrap();
            conn.write_all(b"{\"id\":\"1\",\"result\":{\"type\":\"ok\"}}\n")
                .unwrap();
        })
    }

    #[test]
    fn a_busy_pipe_is_waited_for() {
        let path = unique_path();
        let (server, holder) = single_busy_instance(&path);
        let server = free_after(server, holder, Duration::from_millis(200));
        let got = Client::new(path)
            .call(&Request::AgentFocus { target: "p".into() })
            .unwrap();
        assert_eq!(got, Response::Ok);
        server.join().unwrap();
    }

    #[test]
    fn a_pipe_busy_past_the_deadline_times_out() {
        let path = unique_path();
        let (_server, _holder) = single_busy_instance(&path);
        let started = Instant::now();
        let err = Client::with_deadline(path, Duration::from_millis(300))
            .call(&Request::Ping)
            .unwrap_err();
        assert!(
            matches!(&err, CallError::Io(e) if e.kind() == io::ErrorKind::TimedOut),
            "{err:?}"
        );
        assert!(started.elapsed() < Duration::from_millis(1000));
    }
}

mod presence {
    use super::*;

    #[test]
    fn a_listening_herdr_is_present_and_sees_no_connection() {
        let path = unique_path();
        let listener = bind(&path);
        assert!(Client::new(path.clone()).present());
        listener
            .set_nonblocking(interprocess::local_socket::ListenerNonblockingMode::Accept)
            .unwrap();
        assert!(listener.accept().is_err(), "present() must not connect");
    }

    #[test]
    fn a_missing_herdr_is_not_present() {
        assert!(!Client::new(unique_path()).present());
    }
}
