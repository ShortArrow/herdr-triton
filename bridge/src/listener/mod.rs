//! One resident listener per user session (ADR 0012): what a hook or
//! `bridge stop` can ask of it, and the names it is found by.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

/// The base name the plugin's listener uses.
pub const LISTENER: &str = "herdr-triton-listener";

/// The kernel object names for one base name, in the user's session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Names {
    pub mutex: String,
    pub wake: String,
    pub stop: String,
}

impl Names {
    pub fn new(base: &str) -> Self {
        Self {
            mutex: format!(r"Local\{base}"),
            wake: format!(r"Local\{base}-wake"),
            stop: format!(r"Local\{base}-stop"),
        }
    }
}

/// What a hook or `bridge stop` asks of the listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Wake,
    Stop,
}
