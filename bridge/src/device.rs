//! The keypad over serial: finds its port and speaks SCPI to it (ADR 0009).

use std::io::{self, Read, Write};

use protocol::scpi::{parse_reply, write_command, Command, Reply, MAX_LINE};
use protocol::{Edge, Led, Position, Rgb};
use serialport::{SerialPortInfo, SerialPortType};

const SERIAL_PREFIX: &str = "TRITON-";

/// The port whose USB serial number starts with `TRITON-`, ignoring case;
/// Windows upper-cases serial numbers and hides the product string.
pub fn find_port(ports: &[SerialPortInfo]) -> Option<String> {
    ports.iter().find_map(|p| match &p.port_type {
        SerialPortType::UsbPort(usb) => usb
            .serial_number
            .as_deref()
            .and_then(|s| s.get(..SERIAL_PREFIX.len()))
            .filter(|prefix| prefix.eq_ignore_ascii_case(SERIAL_PREFIX))
            .map(|_| p.port_name.clone()),
        _ => None,
    })
}

/// An SCPI session with the keypad over any byte stream. The stream's read
/// timeout bounds how long a query waits for its reply.
pub struct Device<T> {
    port: T,
}

impl<T: Read + Write> Device<T> {
    pub fn new(port: T) -> Self {
        Self { port }
    }

    pub fn port_mut(&mut self) -> &mut T {
        &mut self.port
    }

    /// `SYSTem:PROTocol?`
    pub fn protocol(&mut self) -> io::Result<u16> {
        match self.query(Command::Protocol)? {
            Answer::Protocol(v) => Ok(v),
            _ => Err(invalid()),
        }
    }

    /// `KEY:EVENt?`: the oldest queued key event.
    pub fn next_key(&mut self) -> io::Result<Option<(Position, Edge)>> {
        match self.query(Command::NextKey)? {
            Answer::Key(event) => Ok(event),
            _ => Err(invalid()),
        }
    }

    /// `LED:ALL`
    pub fn show(&mut self, leds: [Led; 3]) -> io::Result<()> {
        self.send(&Command::SetAll(leds))
    }

    /// `LED<n>:FLASh`
    pub fn flash(&mut self, pos: Position, rgb: Rgb) -> io::Result<()> {
        self.send(&Command::Flash(pos, rgb))
    }

    fn send(&mut self, cmd: &Command) -> io::Result<()> {
        let mut line = String::new();
        write_command(cmd, &mut line).map_err(|_| invalid())?;
        line.push('\n');
        self.port.write_all(line.as_bytes())?;
        self.port.flush()
    }

    fn query(&mut self, cmd: Command) -> io::Result<Answer> {
        self.send(&cmd)?;
        let line = self.read_line()?;
        match parse_reply(&cmd, &line).map_err(|_| invalid())? {
            Reply::Protocol(v) => Ok(Answer::Protocol(v)),
            Reply::Key(event) => Ok(Answer::Key(event)),
            _ => Err(invalid()),
        }
    }

    fn read_line(&mut self) -> io::Result<String> {
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match self.port.read(&mut byte)? {
                0 => return Err(io::ErrorKind::UnexpectedEof.into()),
                _ if byte[0] == b'\n' => break,
                _ if line.len() > MAX_LINE + 1 => return Err(invalid()),
                _ => line.push(byte[0]),
            }
        }
        String::from_utf8(line).map_err(|_| invalid())
    }
}

/// The replies the bridge asks for, owned.
enum Answer {
    Protocol(u16),
    Key(Option<(Position, Edge)>),
}

fn invalid() -> io::Error {
    io::ErrorKind::InvalidData.into()
}
