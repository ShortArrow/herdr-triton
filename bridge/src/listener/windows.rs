//! The Windows mechanisms: a named mutex marks the listener running, named
//! events wake or stop it, and a hook spawns it detached from herdr's pipes.

use std::io;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use super::{Names, Signal};

use windows_sys::Win32::Foundation::{
    GetLastError, SetHandleInformation, ERROR_ALREADY_EXISTS, HANDLE, HANDLE_FLAG_INHERIT,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, OpenMutexW, SetEvent, WaitForMultipleObjects,
    CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS, EVENT_MODIFY_STATE,
    SYNCHRONIZATION_SYNCHRONIZE,
};

/// The running listener's mutex and events; dropping it releases them.
pub struct Claim {
    _mutex: OwnedHandle,
    stop: OwnedHandle,
    wake: OwnedHandle,
}

impl Claim {
    /// Becomes the listener, or `None` when another one already is.
    pub fn take(names: &Names) -> io::Result<Option<Claim>> {
        let mutex =
            owned(unsafe { CreateMutexW(std::ptr::null(), 0, wide(&names.mutex).as_ptr()) })?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Ok(None);
        }
        let event = |name: &str| {
            owned(unsafe { CreateEventW(std::ptr::null(), 0, 0, wide(name).as_ptr()) })
        };
        Ok(Some(Claim {
            _mutex: mutex,
            stop: event(&names.stop)?,
            wake: event(&names.wake)?,
        }))
    }

    /// Waits up to `timeout` for a signal; a stop wins over a wake.
    pub fn wait(&self, timeout: Duration) -> io::Result<Option<Signal>> {
        use std::os::windows::io::AsRawHandle;
        let handles: [HANDLE; 2] = [
            self.stop.as_raw_handle() as _,
            self.wake.as_raw_handle() as _,
        ];
        let ms = timeout.as_millis().min(u32::MAX as u128 - 1) as u32;
        match unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, ms) } {
            r if r == WAIT_OBJECT_0 => Ok(Some(Signal::Stop)),
            r if r == WAIT_OBJECT_0 + 1 => Ok(Some(Signal::Wake)),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(io::Error::last_os_error()),
        }
    }
}

/// Whether a listener holds the mutex.
pub fn running(names: &Names) -> bool {
    owned(unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, wide(&names.mutex).as_ptr()) })
        .is_ok()
}

/// Sets the listener's wake or stop event; `NotFound` without a listener.
pub fn send(names: &Names, signal: Signal) -> io::Result<()> {
    let name = match signal {
        Signal::Wake => &names.wake,
        Signal::Stop => &names.stop,
    };
    let event = owned(unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, wide(name).as_ptr()) })?;
    use std::os::windows::io::AsRawHandle;
    if unsafe { SetEvent(event.as_raw_handle() as _) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Starts `command` with no console, in its own process group, with null
/// standard streams, after making this process's standard handles
/// non-inheritable: Rust spawns with handle inheritance on, and a child
/// holding herdr's pipe keeps herdr waiting for the hook to end.
pub fn spawn_detached(command: &mut Command) -> io::Result<Child> {
    for std in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        unsafe { SetHandleInformation(GetStdHandle(std), HANDLE_FLAG_INHERIT, 0) };
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
}

/// Runs `command` to completion and collects its output without opening a
/// console window, which a listener without a console would otherwise get
/// for every console program it starts.
pub fn run_hidden(command: &mut Command) -> io::Result<Output> {
    command.creation_flags(CREATE_NO_WINDOW).output()
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Takes ownership of a handle a Win32 call returned, or its error.
fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedHandle::from_raw_handle(handle as _) })
    }
}
