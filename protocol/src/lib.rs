//! Messages between the herdr-triton firmware and bridge, and their wire
//! format: postcard, COBS-encoded, each frame terminated by `0x00`.

#![no_std]

pub mod scpi;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// The protocol version carried in [`DeviceMessage::Ready`].
pub const PROTOCOL_VERSION: u16 = 1;

/// The longest encoded frame, terminator included, of any message.
pub const MAX_FRAME_LEN: usize = 32;

/// A key and the LED under it, named by physical position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Left,
    Middle,
    Right,
}

/// Whether a key went down or came up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edge {
    Down,
    Up,
}

/// A colour as the host means it; the firmware maps it to the LED's byte order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// How an LED shows its colour; the firmware runs the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Off,
    Solid,
    Breathe,
    Blink,
}

/// One LED's steady state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Led {
    pub rgb: Rgb,
    pub mode: Mode,
}

/// Messages from the device to the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceMessage {
    /// Sent once on leaving `NoHost`.
    Ready { protocol: u16 },
    Key { pos: Position, edge: Edge },
}

/// Messages from the host to the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostMessage {
    /// Every LED, left to right.
    Frame([Led; 3]),
    Flash { pos: Position, rgb: Rgb },
}

/// A message type that travels on the wire. Implemented only for
/// [`DeviceMessage`] and [`HostMessage`], whose encoded size is bounded by
/// [`MAX_FRAME_LEN`].
pub trait Message: Serialize + DeserializeOwned + sealed::Sealed {}
impl Message for DeviceMessage {}
impl Message for HostMessage {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::DeviceMessage {}
    impl Sealed for super::HostMessage {}
}

/// Encodes `msg` into `buf` and returns the frame, terminator included.
pub fn encode<'a, M: Message>(msg: &M, buf: &'a mut [u8; MAX_FRAME_LEN]) -> &'a [u8] {
    postcard::to_slice_cobs(msg, buf).expect("every Message fits in MAX_FRAME_LEN")
}

/// Why a frame was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The frame was not valid COBS or not a valid message.
    Malformed,
    /// The frame exceeded [`MAX_FRAME_LEN`] before its terminator.
    Overflow,
}

/// Reassembles messages from a byte stream that may split or join frames
/// arbitrarily. After an error it resumes at the next terminator.
pub struct Decoder<M> {
    buf: [u8; MAX_FRAME_LEN],
    len: usize,
    overflowed: bool,
    _message: core::marker::PhantomData<M>,
}

impl<M: Message> Decoder<M> {
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_FRAME_LEN],
            len: 0,
            overflowed: false,
            _message: core::marker::PhantomData,
        }
    }

    /// Feeds one byte. Returns the outcome of a frame when `byte` ends one;
    /// an empty frame (a terminator with nothing before it) yields nothing.
    pub fn push(&mut self, byte: u8) -> Option<Result<M, DecodeError>> {
        if byte == 0 {
            let outcome = match (self.overflowed, self.len) {
                (true, _) => Some(Err(DecodeError::Overflow)),
                (false, 0) => None,
                (false, len) => Some(decode_frame(&mut self.buf[..len])),
            };
            self.len = 0;
            self.overflowed = false;
            return outcome;
        }
        if self.len == MAX_FRAME_LEN - 1 {
            self.overflowed = true;
        }
        if !self.overflowed {
            self.buf[self.len] = byte;
            self.len += 1;
        }
        None
    }
}

/// Decodes one COBS frame without its terminator, rejecting trailing bytes.
fn decode_frame<M: Message>(frame: &mut [u8]) -> Result<M, DecodeError> {
    let len = cobs::decode_in_place(frame).map_err(|_| DecodeError::Malformed)?;
    match postcard::take_from_bytes(&frame[..len]) {
        Ok((msg, [])) => Ok(msg),
        _ => Err(DecodeError::Malformed),
    }
}

impl<M: Message> Default for Decoder<M> {
    fn default() -> Self {
        Self::new()
    }
}
