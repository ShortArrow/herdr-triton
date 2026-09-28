//! Host side of herdr-triton. `state` is the pure core; adapters for herdr
//! and the serial device drive it.

pub mod device;
pub mod herdr;
pub mod position;
pub mod prompt_screen;
pub mod state;
