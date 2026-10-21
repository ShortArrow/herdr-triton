//! Bringing herdr's window to the front on Windows (ADR 0015).

use std::collections::BTreeSet;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindow, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, GW_OWNER, SW_RESTORE,
};

use super::{host, Process};

/// How long a minimised window is given to restore before it is raised.
const RESTORE: Duration = Duration::from_millis(500);

/// Brings the window showing herdr to the front, as the specification's
/// "Raising herdr's window" says; whether it is in front afterwards.
pub fn raise_herdr() -> bool {
    let windows = top_level_windows();
    let windowed: BTreeSet<u32> = windows.iter().map(|&(pid, _)| pid).collect();
    let Some(owner) = host(&processes(), &windowed) else {
        return false;
    };
    let Some(&(_, window)) = windows.iter().find(|&&(pid, _)| pid == owner) else {
        return false;
    };
    restore(window);
    bring_to_front(window)
}

/// Every running process, from a process snapshot.
fn processes() -> Vec<Process> {
    let mut list = Vec::new();
    // SAFETY: a snapshot of processes needs no other argument; the handle is
    // checked and closed below.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return list;
    }
    // SAFETY: PROCESSENTRY32W is plain data, valid when zeroed.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    // SAFETY: `snapshot` is a live process snapshot and `entry` has its size set.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while more {
        let len = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        list.push(Process {
            pid: entry.th32ProcessID,
            parent: entry.th32ParentProcessID,
            name: String::from_utf16_lossy(&entry.szExeFile[..len]),
        });
        // SAFETY: as for Process32FirstW.
        more = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    // SAFETY: `snapshot` is open and owned here.
    unsafe { CloseHandle(snapshot) };
    list
}

/// Visible top-level windows without an owner window, with their process.
fn top_level_windows() -> Vec<(u32, HWND)> {
    unsafe extern "system" fn collect(window: HWND, list: LPARAM) -> BOOL {
        // SAFETY: `list` is the Vec that `top_level_windows` passed, alive for
        // the whole EnumWindows call.
        let list = unsafe { &mut *(list as *mut Vec<(u32, HWND)>) };
        // SAFETY: EnumWindows passes a window that exists during the callback.
        let shown =
            unsafe { IsWindowVisible(window) != 0 && GetWindow(window, GW_OWNER).is_null() };
        if shown {
            let mut pid = 0;
            // SAFETY: as above; `pid` is a valid out pointer.
            unsafe { GetWindowThreadProcessId(window, &mut pid) };
            list.push((pid, window));
        }
        1
    }
    let mut list: Vec<(u32, HWND)> = Vec::new();
    // SAFETY: `collect` only reads the windows it is given and writes to
    // `list`, which outlives the call.
    unsafe { EnumWindows(Some(collect), &mut list as *mut _ as LPARAM) };
    list
}

/// Restores `window` if it is minimised, waiting up to [`RESTORE`].
fn restore(window: HWND) {
    // SAFETY: IsIconic and ShowWindow accept any window handle, and fail on
    // one that has gone.
    if unsafe { IsIconic(window) } == 0 {
        return;
    }
    // SAFETY: as above.
    unsafe { ShowWindow(window, SW_RESTORE) };
    let step = Duration::from_millis(10);
    for _ in 0..RESTORE.as_millis() / step.as_millis() {
        // SAFETY: as above.
        if unsafe { IsIconic(window) } == 0 {
            return;
        }
        std::thread::sleep(step);
    }
}

/// Brings `window` to the front while attached to the input thread of the
/// window in front, which Windows requires of a process that did not receive
/// the last input; whether `window` is in front afterwards.
fn bring_to_front(window: HWND) -> bool {
    // SAFETY: these calls take any window handle or thread id and fail on
    // ones that have gone; attaching is undone before returning.
    unsafe {
        let front = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        let me = GetCurrentThreadId();
        let attached = front != 0 && front != me && AttachThreadInput(me, front, 1) != 0;
        SetForegroundWindow(window);
        BringWindowToTop(window);
        if attached {
            AttachThreadInput(me, front, 0);
        }
        GetForegroundWindow() == window
    }
}
