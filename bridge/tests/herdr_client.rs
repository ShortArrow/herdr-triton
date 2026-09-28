//! Runs the client against a fake herdr listening on a real local socket,
//! bound the same way herdr binds it.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use bridge::herdr::client::{CallError, Client};
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
