//! SCPI-style text for the keypad (ADR 0009): typed commands and replies,
//! their text form, and line assembly. Independent of the transport.

use core::fmt;

use crate::{Edge, Led, Mode, Position, Rgb};

/// The version `SYSTem:PROTocol?` answers.
pub const PROTOCOL_VERSION: u16 = 2;

/// The longest line, terminator excluded.
pub const MAX_LINE: usize = 64;

/// A command from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `*IDN?`
    Identify,
    /// `SYSTem:PROTocol?`
    Protocol,
    /// `SYSTem:ERRor?`
    NextError,
    /// `KEY:EVENt?`
    NextKey,
    /// `LED:ALL`, left to right.
    SetAll([Led; 3]),
    /// `LED<n>`
    Set(Position, Led),
    /// `LED<n>:FLASh`
    Flash(Position, Rgb),
}

impl Command {
    /// Whether the device answers this command.
    pub fn is_query(&self) -> bool {
        matches!(
            self,
            Command::Identify | Command::Protocol | Command::NextError | Command::NextKey
        )
    }
}

/// SCPI error codes the keypad reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// -100
    CommandError,
    /// -113
    UndefinedHeader,
    /// -222
    DataOutOfRange,
    /// -350
    QueueOverflow,
}

impl ErrorCode {
    pub fn code(self) -> i16 {
        match self {
            ErrorCode::CommandError => -100,
            ErrorCode::UndefinedHeader => -113,
            ErrorCode::DataOutOfRange => -222,
            ErrorCode::QueueOverflow => -350,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            ErrorCode::CommandError => "Command error",
            ErrorCode::UndefinedHeader => "Undefined header",
            ErrorCode::DataOutOfRange => "Data out of range",
            ErrorCode::QueueOverflow => "Queue overflow",
        }
    }

    fn from_code(code: i16) -> Option<Self> {
        [
            ErrorCode::CommandError,
            ErrorCode::UndefinedHeader,
            ErrorCode::DataOutOfRange,
            ErrorCode::QueueOverflow,
        ]
        .into_iter()
        .find(|e| e.code() == code)
    }
}

/// A reply to a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply<'a> {
    Identity { serial: &'a str, version: &'a str },
    Protocol(u16),
    /// `None` is `0,"No error"`.
    Error(Option<ErrorCode>),
    /// `None` is `NONE`.
    Key(Option<(Position, Edge)>),
}

/// A reply line that does not answer the query it was read for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplyError;

/// Reads one command line.
pub fn parse_command(line: &str) -> Result<Command, ErrorCode> {
    let line = line.trim();
    if line.is_empty() {
        return Err(ErrorCode::CommandError);
    }
    let (header, params) = match line.split_once(char::is_whitespace) {
        Some((header, params)) => (header, params.trim()),
        None => (line, ""),
    };
    let (header, query) = match header.strip_suffix('?') {
        Some(h) => (h, true),
        None => (header, false),
    };
    let header = header.strip_prefix(':').unwrap_or(header);
    let mut nodes = header.split(':');
    let (first, second) = (nodes.next().unwrap_or(""), nodes.next());
    if nodes.next().is_some() {
        return Err(ErrorCode::UndefinedHeader);
    }

    let query_without_params = |cmd| match params.is_empty() {
        true => Ok(cmd),
        false => Err(ErrorCode::CommandError),
    };
    if query {
        return match (first, second) {
            (f, None) if f.eq_ignore_ascii_case("*IDN") => query_without_params(Command::Identify),
            (f, Some(s)) if is(f, "SYSTem") && is(s, "PROTocol") => query_without_params(Command::Protocol),
            (f, Some(s)) if is(f, "SYSTem") && is(s, "ERRor") => query_without_params(Command::NextError),
            (f, Some(s)) if is(f, "KEY") && is(s, "EVENt") => query_without_params(Command::NextKey),
            _ => Err(ErrorCode::UndefinedHeader),
        };
    }
    let mut values = [""; 6];
    let count = split_params(params, &mut values)?;
    match (first, second) {
        (f, Some(s)) if is(f, "LED") && is(s, "ALL") => {
            let [a, b, c, d, e, f] = expect_params::<6>(&values, count)?;
            Ok(Command::SetAll([led(a, b)?, led(c, d)?, led(e, f)?]))
        }
        (f, None) => {
            let pos = led_number(f)?;
            let [rgb, mode] = expect_params::<2>(&values, count)?;
            Ok(Command::Set(pos?, led(rgb, mode)?))
        }
        (f, Some(s)) if is(s, "FLASh") => {
            let pos = led_number(f)?;
            let [rgb] = expect_params::<1>(&values, count)?;
            Ok(Command::Flash(pos?, parse_rgb(rgb)?))
        }
        _ => Err(ErrorCode::UndefinedHeader),
    }
}

/// Whether `word` is `mnemonic` in its long form or its short form (the
/// leading upper-case letters), ignoring case.
fn is(word: &str, mnemonic: &str) -> bool {
    let short = mnemonic.len() - mnemonic.trim_start_matches(|c: char| !c.is_ascii_lowercase()).len();
    word.eq_ignore_ascii_case(mnemonic) || word.eq_ignore_ascii_case(&mnemonic[..short])
}

/// `LED<n>`: an undefined header unless it is `LED` and digits; the inner
/// result is out of range unless `n` is 1 to 3.
fn led_number(word: &str) -> Result<Result<Position, ErrorCode>, ErrorCode> {
    let digits = match word.get(..3) {
        Some(led) if led.eq_ignore_ascii_case("LED") => &word[3..],
        _ => return Err(ErrorCode::UndefinedHeader),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErrorCode::UndefinedHeader);
    }
    Ok(match digits {
        "1" => Ok(Position::Left),
        "2" => Ok(Position::Middle),
        "3" => Ok(Position::Right),
        _ => Err(ErrorCode::DataOutOfRange),
    })
}

/// Splits comma-separated parameters into `out`; more than `out` holds is a
/// command error.
fn split_params<'a>(params: &'a str, out: &mut [&'a str; 6]) -> Result<usize, ErrorCode> {
    if params.is_empty() {
        return Ok(0);
    }
    let mut count = 0;
    for value in params.split(',') {
        *out.get_mut(count).ok_or(ErrorCode::CommandError)? = value.trim();
        count += 1;
    }
    Ok(count)
}

fn expect_params<'a, const N: usize>(values: &[&'a str; 6], count: usize) -> Result<[&'a str; N], ErrorCode> {
    if count != N {
        return Err(ErrorCode::CommandError);
    }
    Ok(core::array::from_fn(|i| values[i]))
}

fn led(rgb: &str, mode: &str) -> Result<Led, ErrorCode> {
    Ok(Led { rgb: parse_rgb(rgb)?, mode: parse_mode(mode)? })
}

fn parse_rgb(text: &str) -> Result<Rgb, ErrorCode> {
    let hex = text.strip_prefix('#').filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()));
    let hex = hex.ok_or(ErrorCode::DataOutOfRange)?;
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ErrorCode::DataOutOfRange);
    Ok(Rgb { r: byte(0)?, g: byte(2)?, b: byte(4)? })
}

const MODES: [(Mode, &str); 4] = [
    (Mode::Off, "OFF"),
    (Mode::Solid, "SOLid"),
    (Mode::Breathe, "BREathe"),
    (Mode::Blink, "BLINk"),
];

fn parse_mode(text: &str) -> Result<Mode, ErrorCode> {
    MODES
        .iter()
        .find(|(_, name)| is(text, name))
        .map(|(mode, _)| *mode)
        .ok_or(ErrorCode::DataOutOfRange)
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Off => "OFF",
        Mode::Solid => "SOLID",
        Mode::Breathe => "BREATHE",
        Mode::Blink => "BLINK",
    }
}

fn led_index(pos: Position) -> u8 {
    match pos {
        Position::Left => 1,
        Position::Middle => 2,
        Position::Right => 3,
    }
}

struct Hex(Rgb);

impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.0.r, self.0.g, self.0.b)
    }
}

const POSITIONS: [(Position, &str); 3] = [
    (Position::Left, "LEFT"),
    (Position::Middle, "MIDDLE"),
    (Position::Right, "RIGHT"),
];
const EDGES: [(Edge, &str); 2] = [(Edge::Down, "DOWN"), (Edge::Up, "UP")];

fn name_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table.iter().find(|(v, _)| *v == value).map(|(_, n)| *n).unwrap_or("")
}

fn value_of<T: Copy>(table: &[(T, &'static str)], name: &str) -> Option<T> {
    table.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(v, _)| *v)
}

/// Writes `cmd` in its short form, without a line terminator.
pub fn write_command(cmd: &Command, out: &mut impl fmt::Write) -> fmt::Result {
    match cmd {
        Command::Identify => out.write_str("*IDN?"),
        Command::Protocol => out.write_str("SYST:PROT?"),
        Command::NextError => out.write_str("SYST:ERR?"),
        Command::NextKey => out.write_str("KEY:EVEN?"),
        Command::SetAll(leds) => {
            out.write_str("LED:ALL ")?;
            for (i, l) in leds.iter().enumerate() {
                let sep = if i == 0 { "" } else { "," };
                write!(out, "{sep}{},{}", Hex(l.rgb), mode_name(l.mode))?;
            }
            Ok(())
        }
        Command::Set(pos, l) => write!(out, "LED{} {},{}", led_index(*pos), Hex(l.rgb), mode_name(l.mode)),
        Command::Flash(pos, rgb) => write!(out, "LED{}:FLAS {}", led_index(*pos), Hex(*rgb)),
    }
}

/// Writes `reply` without a line terminator.
pub fn write_reply(reply: &Reply, out: &mut impl fmt::Write) -> fmt::Result {
    match reply {
        Reply::Identity { serial, version } => write!(out, "ShortArrow,herdr-triton,{serial},{version}"),
        Reply::Protocol(v) => write!(out, "{v}"),
        Reply::Error(None) => out.write_str("0,\"No error\""),
        Reply::Error(Some(e)) => write!(out, "{},\"{}\"", e.code(), e.message()),
        Reply::Key(None) => out.write_str("NONE"),
        Reply::Key(Some((pos, edge))) => {
            write!(out, "{},{}", name_of(&POSITIONS, *pos), name_of(&EDGES, *edge))
        }
    }
}

/// Reads the reply `line` to `query`.
pub fn parse_reply<'a>(query: &Command, line: &'a str) -> Result<Reply<'a>, ReplyError> {
    let line = line.trim();
    match query {
        Command::Identify => {
            let mut parts = line.splitn(4, ',');
            match (parts.next(), parts.next(), parts.next(), parts.next()) {
                (Some("ShortArrow"), Some("herdr-triton"), Some(serial), Some(version)) => {
                    Ok(Reply::Identity { serial, version })
                }
                _ => Err(ReplyError),
            }
        }
        Command::Protocol => line.parse().map(Reply::Protocol).map_err(|_| ReplyError),
        Command::NextError => {
            let (code, _) = line.split_once(',').ok_or(ReplyError)?;
            match code.parse::<i16>().map_err(|_| ReplyError)? {
                0 => Ok(Reply::Error(None)),
                n => ErrorCode::from_code(n).map(|e| Reply::Error(Some(e))).ok_or(ReplyError),
            }
        }
        Command::NextKey if line.eq_ignore_ascii_case("NONE") => Ok(Reply::Key(None)),
        Command::NextKey => {
            let (pos, edge) = line.split_once(',').ok_or(ReplyError)?;
            let pos = value_of(&POSITIONS, pos).ok_or(ReplyError)?;
            let edge = value_of(&EDGES, edge).ok_or(ReplyError)?;
            Ok(Reply::Key(Some((pos, edge))))
        }
        _ => Err(ReplyError),
    }
}

/// Assembles lines from a byte stream.
pub struct LineBuffer {
    /// One byte beyond [`MAX_LINE`] holds a trailing `\r`.
    buf: [u8; MAX_LINE + 1],
    len: usize,
    overflowed: bool,
}

impl LineBuffer {
    pub const fn new() -> Self {
        Self { buf: [0; MAX_LINE + 1], len: 0, overflowed: false }
    }

    /// Feeds one byte. At `\n` returns the line, without a trailing `\r`,
    /// or `CommandError` for a line over [`MAX_LINE`] or not UTF-8.
    pub fn push(&mut self, byte: u8) -> Option<Result<&str, ErrorCode>> {
        if byte != b'\n' {
            match self.buf.get_mut(self.len) {
                Some(slot) if !self.overflowed => {
                    *slot = byte;
                    self.len += 1;
                }
                _ => self.overflowed = true,
            }
            return None;
        }
        let (len, overflowed) = (self.len, self.overflowed);
        self.len = 0;
        self.overflowed = false;
        let line = &self.buf[..len];
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if overflowed || line.len() > MAX_LINE {
            return Some(Err(ErrorCode::CommandError));
        }
        Some(core::str::from_utf8(line).map_err(|_| ErrorCode::CommandError))
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}
