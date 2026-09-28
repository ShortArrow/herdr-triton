//! The serial adapter against a simulated keypad running the firmware's own
//! `keypad` core.

use std::collections::VecDeque;
use std::io::{self, Read, Write};

use bridge::device::{find_port, Device};
use keypad::Keypad;
use protocol::scpi::{LineBuffer, PROTOCOL_VERSION};
use protocol::{Edge, Led, Mode, Position, Rgb};
use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

/// A keypad on the other end of a serial line, with DTR high.
struct Sim {
    keypad: Keypad,
    now: u64,
    lines: LineBuffer,
    out: VecDeque<u8>,
}

impl Sim {
    fn new() -> Self {
        let mut keypad = Keypad::new("TRITON-SIM", "0.0.0");
        keypad.set_dtr(true);
        Self { keypad, now: 0, lines: LineBuffer::new(), out: VecDeque::new() }
    }

    /// Holds a key down, then releases it, 10 ms each.
    fn tap(&mut self, pos: Position) {
        let pressed = |p: Position| [p == Position::Left, p == Position::Middle, p == Position::Right];
        for _ in 0..10 {
            self.now += 1;
            self.keypad.scan(pressed(pos), self.now);
        }
        for _ in 0..10 {
            self.now += 1;
            self.keypad.scan([false; 3], self.now);
        }
    }
}

impl Write for Sim {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &b in buf {
            if let Some(line) = self.lines.push(b) {
                if let Some(reply) = self.keypad.line(line, self.now) {
                    let mut text = String::new();
                    protocol::scpi::write_reply(&reply, &mut text).unwrap();
                    self.out.extend(text.bytes().chain([b'\n']));
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Sim {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.out.is_empty() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let n = buf.len().min(self.out.len());
        for slot in &mut buf[..n] {
            *slot = self.out.pop_front().unwrap();
        }
        Ok(n)
    }
}

/// A port that answers every line with `reply`, or never answers.
struct Canned(Option<&'static str>, VecDeque<u8>);

impl Write for Canned {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.contains(&b'\n') {
            if let Some(reply) = self.0 {
                self.1.extend(reply.bytes());
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for Canned {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.1.pop_front() {
            Some(b) => {
                buf[0] = b;
                Ok(1)
            }
            None => Err(io::ErrorKind::TimedOut.into()),
        }
    }
}

const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };

#[test]
fn asks_the_protocol_version() {
    let mut dev = Device::new(Sim::new());
    assert_eq!(dev.protocol().unwrap(), PROTOCOL_VERSION);
}

#[test]
fn reads_key_events_oldest_first_then_none() {
    let mut dev = Device::new(Sim::new());
    dev.protocol().unwrap();
    dev.port_mut().tap(Position::Right);
    dev.port_mut().tap(Position::Left);
    let events: Vec<_> = std::iter::from_fn(|| dev.next_key().unwrap()).collect();
    assert_eq!(
        events,
        vec![
            (Position::Right, Edge::Down),
            (Position::Right, Edge::Up),
            (Position::Left, Edge::Down),
            (Position::Left, Edge::Up),
        ]
    );
}

#[test]
fn shows_a_frame_and_flashes_a_key() {
    let mut dev = Device::new(Sim::new());
    dev.show([Led { rgb: WHITE, mode: Mode::Solid }; 3]).unwrap();
    dev.flash(Position::Middle, Rgb { r: 255, g: 0, b: 0 }).unwrap();
    let sim = dev.port_mut();
    assert_eq!(
        sim.keypad.pixels(sim.now),
        [Rgb { r: 64, g: 64, b: 64 }, Rgb { r: 64, g: 0, b: 0 }, Rgb { r: 64, g: 64, b: 64 }]
    );
}

#[test]
fn a_reply_that_does_not_answer_the_query_is_invalid_data() {
    let mut dev = Device::new(Canned(Some("banana\n"), VecDeque::new()));
    assert_eq!(dev.next_key().unwrap_err().kind(), io::ErrorKind::InvalidData);
}

#[test]
fn a_silent_device_times_out() {
    let mut dev = Device::new(Canned(None, VecDeque::new()));
    assert_eq!(dev.protocol().unwrap_err().kind(), io::ErrorKind::TimedOut);
}

fn usb(name: &str, serial: Option<&str>) -> SerialPortInfo {
    SerialPortInfo {
        port_name: name.into(),
        port_type: SerialPortType::UsbPort(UsbPortInfo {
            vid: 0x2e8a,
            pid: 0x000a,
            serial_number: serial.map(Into::into),
            manufacturer: None,
            product: None,
        }),
    }
}

#[test]
fn finds_the_port_by_its_triton_serial_number_ignoring_case() {
    let ports = vec![
        usb("COM9", Some("IO-PROBE")),
        SerialPortInfo { port_name: "COM1".into(), port_type: SerialPortType::Unknown },
        usb("COM10", Some("triton-0123456789abcdef")),
    ];
    assert_eq!(find_port(&ports), Some("COM10".into()));
}

#[test]
fn finds_nothing_without_a_triton_serial_number() {
    assert_eq!(find_port(&[usb("COM9", Some("IO-PROBE")), usb("COM11", None)]), None);
}
