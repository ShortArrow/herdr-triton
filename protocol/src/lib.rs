//! What the herdr-triton firmware and bridge say to each other: the shared
//! types, and in [`scpi`] their SCPI-style text (ADR 0009).

#![no_std]

pub mod scpi;

/// A key and the LED under it, named by physical position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Left,
    Middle,
    Right,
}

/// Whether a key went down or came up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Down,
    Up,
}

/// A colour as the host means it; the firmware maps it to the LED's byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// How an LED shows its colour; the firmware runs the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Off,
    Solid,
    Breathe,
    Blink,
}

/// One LED's steady state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Led {
    pub rgb: Rgb,
    pub mode: Mode,
}
