//! Sends one request per connection, as herdr's server reads only the first
//! line of each connection (herdr `src/api/server.rs`).

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use interprocess::local_socket::{prelude::*, Stream};

use super::wire::{decode_response, encode_request, Request, Response, WireError};

/// Why a call failed.
#[derive(Debug)]
pub enum CallError {
    Io(io::Error),
    Wire(WireError),
}

/// A herdr API endpoint at a socket path.
pub struct Client {
    path: PathBuf,
}

impl Client {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Connects, sends `request`, reads one response line, and disconnects.
    pub fn call(&self, request: &Request) -> Result<Response, CallError> {
        let mut stream = connect(&self.path).map_err(CallError::Io)?;
        stream
            .write_all(encode_request("herdr-triton", request).as_bytes())
            .map_err(CallError::Io)?;
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).map_err(CallError::Io)?;
        decode_response(line.trim_end()).map_err(CallError::Wire)
    }
}

/// Connects the way herdr does: a file-path socket on Unix, and on Windows a
/// named pipe named after the full path string (herdr `src/ipc.rs`).
fn connect(path: &Path) -> io::Result<Stream> {
    #[cfg(unix)]
    let name = {
        use interprocess::local_socket::GenericFilePath;
        path.to_fs_name::<GenericFilePath>()?
    };
    #[cfg(windows)]
    let name = {
        use interprocess::local_socket::GenericNamespaced;
        path.to_string_lossy().to_string().to_ns_name::<GenericNamespaced>()?
    };
    Stream::connect(name)
}
