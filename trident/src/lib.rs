//! Host side of herdr-triton. `state` is the pure core; adapters for herdr
//! and the serial device drive it.

#![warn(clippy::undocumented_unsafe_blocks)]

pub mod config;
pub mod device;
pub mod herdr;
pub mod layout;
pub mod listener;
pub mod position;
pub mod prompt_screen;
pub mod runtime;
pub mod state;
