//! Sends one request per connection, as herdr's server reads only the first
//! line of each connection (herdr `src/api/server.rs`).

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::wire::{decode_response, encode_request, Request, Response, WireError};

/// How long one call may take, from connecting to the end of its reply.
pub const DEADLINE: Duration = Duration::from_secs(2);

/// Why a call failed.
#[derive(Debug)]
pub enum CallError {
    Io(io::Error),
    Wire(WireError),
}

/// A herdr API endpoint at a socket path.
pub struct Client {
    path: PathBuf,
    deadline: Duration,
}

impl Client {
    pub fn new(path: PathBuf) -> Self {
        Self::with_deadline(path, DEADLINE)
    }

    pub fn with_deadline(path: PathBuf, deadline: Duration) -> Self {
        Self { path, deadline }
    }

    /// Connects, sends `request`, reads one response line, and disconnects,
    /// failing with `TimedOut` once the deadline passes.
    pub fn call(&self, request: &Request) -> Result<Response, CallError> {
        let deadline = Instant::now() + self.deadline;
        let mut stream = Deadlined {
            stream: connect(&self.path, deadline).map_err(CallError::Io)?,
            deadline,
        };
        stream
            .write_all(encode_request("herdr-triton", request).as_bytes())
            .map_err(CallError::Io)?;
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .map_err(CallError::Io)?;
        decode_response(line.trim_end()).map_err(CallError::Wire)
    }

    /// Whether herdr's endpoint exists. Nothing connects to it, so herdr
    /// sees no request.
    pub fn present(&self) -> bool {
        present(&self.path)
    }
}

/// A connection whose every read waits no later than `deadline`.
struct Deadlined<S> {
    stream: S,
    deadline: Instant,
}

impl<S: Readable> Read for Deadlined<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.wait_readable(self.deadline)?;
        self.stream.read(buf)
    }
}

impl<S: Write> Write for Deadlined<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// A stream that can wait, up to a deadline, until a read would not block.
trait Readable: Read {
    fn wait_readable(&mut self, deadline: Instant) -> io::Result<()>;
}

/// The time left before `deadline`, or `TimedOut` when none is.
fn remaining(deadline: Instant) -> io::Result<Duration> {
    match deadline.checked_duration_since(Instant::now()) {
        Some(left) if !left.is_zero() => Ok(left),
        _ => Err(io::ErrorKind::TimedOut.into()),
    }
}

#[cfg(unix)]
use std::os::unix::net::UnixStream;

/// herdr listens on a file-path socket on Unix (herdr `src/ipc.rs`).
#[cfg(unix)]
fn connect(path: &Path, _deadline: Instant) -> io::Result<UnixStream> {
    UnixStream::connect(path)
}

#[cfg(unix)]
fn present(path: &Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::metadata(path).is_ok_and(|m| m.file_type().is_socket())
}

#[cfg(unix)]
impl Readable for UnixStream {
    fn wait_readable(&mut self, deadline: Instant) -> io::Result<()> {
        self.set_read_timeout(Some(remaining(deadline)?))
    }
}

#[cfg(windows)]
use std::fs::File;

/// herdr listens on Windows on a named pipe named after the full path string,
/// `\\.\pipe\<path>` (herdr `src/ipc.rs`, interprocess `GenericNamespaced`).
/// While every instance is busy, waits for one until `deadline`; std reports
/// the wait running out as `TimedOut`.
#[cfg(windows)]
fn connect(path: &Path, deadline: Instant) -> io::Result<File> {
    use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    let (name, wide) = pipe_name(path);
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&name)
        {
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
                let wait = remaining(deadline)?.as_millis().clamp(1, u32::MAX as u128) as u32;
                if unsafe { WaitNamedPipeW(wide.as_ptr(), wait) } == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            opened => return opened,
        }
    }
}

/// The pipe's name, and the same as a null-terminated wide string.
#[cfg(windows)]
fn pipe_name(path: &Path) -> (String, Vec<u16>) {
    let name = format!(r"\\.\pipe\{}", path.display());
    let wide = name.encode_utf16().chain([0]).collect();
    (name, wide)
}

/// A pipe exists while an instance is free, or while every one is busy and
/// waiting for it times out.
#[cfg(windows)]
fn present(path: &Path) -> bool {
    use windows_sys::Win32::Foundation::ERROR_SEM_TIMEOUT;
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;

    let (_, wide) = pipe_name(path);
    let free = unsafe { WaitNamedPipeW(wide.as_ptr(), 1) } != 0;
    free || io::Error::last_os_error().raw_os_error() == Some(ERROR_SEM_TIMEOUT as i32)
}

/// Polls the pipe for bytes, as a synchronous pipe read cannot time out.
/// A closed pipe counts as readable, so the read reports it.
#[cfg(windows)]
impl Readable for File {
    fn wait_readable(&mut self, deadline: Instant) -> io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Pipes::PeekNamedPipe;

        loop {
            let mut available = 0u32;
            let peeked = unsafe {
                PeekNamedPipe(
                    self.as_raw_handle() as _,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            };
            if peeked == 0 || available > 0 {
                return Ok(());
            }
            remaining(deadline)?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
