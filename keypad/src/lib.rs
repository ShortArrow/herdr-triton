//! The keypad's behaviour without its hardware: whether a host is there,
//! debounced key events waiting to be read, answers to SCPI commands, and
//! what each LED shows at a given time.

#![no_std]

use heapless::Deque;
use protocol::scpi::{parse_command, Command, ErrorCode, Reply, PROTOCOL_VERSION};
use protocol::{Edge, Led, Mode, Position, Rgb};

/// Milliseconds since boot.
pub type Millis = u64;

/// How long the host may stay silent before the keypad counts it as gone.
pub const HOST_TIMEOUT: Millis = 3000;
/// How long a key's level must hold before it counts.
pub const DEBOUNCE: Millis = 5;
pub const FLASH: Millis = 150;
pub const BLINK_PERIOD: Millis = 1000;
pub const BREATHE_PERIOD: Millis = 2000;
/// The brightness full colour is scaled to, out of 255.
pub const MAX_LEVEL: u32 = 64;
pub const KEY_QUEUE: usize = 16;
pub const ERROR_QUEUE: usize = 8;
/// Breathe's dimmest point, in per mille of full brightness.
const BREATHE_FLOOR: u32 = 100;
const NO_HOST: Rgb = Rgb { r: 8, g: 8, b: 8 };
const DARK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const POSITIONS: [Position; 3] = [Position::Left, Position::Middle, Position::Right];

/// One key's debouncer: the accepted level, and the level seen since when.
#[derive(Clone, Copy)]
struct Debounce {
    stable: bool,
    seen: bool,
    since: Millis,
}

impl Debounce {
    const fn new() -> Self {
        Self { stable: false, seen: false, since: 0 }
    }

    /// The new accepted level, if `pressed` has now held for [`DEBOUNCE`].
    fn update(&mut self, pressed: bool, now: Millis) -> Option<bool> {
        if pressed != self.seen {
            self.seen = pressed;
            self.since = now;
        }
        let settled = now.saturating_sub(self.since) >= DEBOUNCE;
        (settled && self.seen != self.stable).then(|| {
            self.stable = self.seen;
            self.stable
        })
    }
}

/// The keypad's state. Keys and LEDs are ordered left, middle, right.
/// A host is present while DTR is high and a line arrived within
/// [`HOST_TIMEOUT`]; otherwise the keypad is in `NoHost`, where key events
/// are dropped and the queue is empty.
pub struct Keypad {
    serial: &'static str,
    version: &'static str,
    dtr: bool,
    /// When the last line arrived since DTR last went high.
    last_line: Option<Millis>,
    leds: [Led; 3],
    flash: Option<(Position, Rgb, Millis)>,
    keys: [Debounce; 3],
    events: Deque<(Position, Edge), KEY_QUEUE>,
    errors: Deque<ErrorCode, ERROR_QUEUE>,
}

impl Keypad {
    /// A keypad that identifies itself with `serial` and firmware `version`.
    pub const fn new(serial: &'static str, version: &'static str) -> Self {
        Self {
            serial,
            version,
            dtr: false,
            last_line: None,
            leds: [Led { rgb: DARK, mode: Mode::Off }; 3],
            flash: None,
            keys: [Debounce::new(); 3],
            events: Deque::new(),
            errors: Deque::new(),
        }
    }

    /// The CDC port's DTR line. Dropping it ends the host, so only a new
    /// line ends `NoHost`.
    pub fn set_dtr(&mut self, dtr: bool) {
        if !dtr {
            self.last_line = None;
            self.events.clear();
        }
        self.dtr = dtr;
    }

    /// Takes one line from the host and returns the reply to send, if the
    /// line was a query. A line that fails to read or parse records its
    /// error; an empty line is ignored. Any line counts as the host.
    pub fn line(&mut self, line: Result<&str, ErrorCode>, now: Millis) -> Option<Reply<'static>> {
        self.forget_if_gone(now);
        if self.dtr {
            self.last_line = Some(now);
        }
        let cmd = match line.map(str::trim) {
            Ok("") => return None,
            Ok(text) => parse_command(text),
            Err(e) => Err(e),
        };
        match cmd {
            Ok(cmd) => self.handle(cmd, now),
            Err(e) => {
                self.record(e);
                None
            }
        }
    }

    /// Takes the raw key levels (`true` = pressed) and queues the debounced
    /// edges; none while there is no host.
    pub fn scan(&mut self, pressed: [bool; 3], now: Millis) {
        let hosted = self.hosted(now);
        for (i, key) in self.keys.iter_mut().enumerate() {
            if let Some(down) = key.update(pressed[i], now) {
                let edge = if down { Edge::Down } else { Edge::Up };
                if hosted && self.events.push_back((POSITIONS[i], edge)).is_err() {
                    let _ = self.errors.push_back(ErrorCode::QueueOverflow);
                }
            }
        }
    }

    /// What the LEDs show at `now`, as colours; see [`led_order`].
    pub fn pixels(&self, now: Millis) -> [Rgb; 3] {
        if !self.hosted(now) {
            return [NO_HOST; 3];
        }
        core::array::from_fn(|i| match self.flash {
            Some((pos, rgb, until)) if pos == POSITIONS[i] && now < until => scale(rgb, 1000),
            _ => render(self.leds[i], now),
        })
    }

    fn handle(&mut self, cmd: Command, now: Millis) -> Option<Reply<'static>> {
        match cmd {
            Command::Identify => Some(Reply::Identity { serial: self.serial, version: self.version }),
            Command::Protocol => Some(Reply::Protocol(PROTOCOL_VERSION)),
            Command::NextError => Some(Reply::Error(self.errors.pop_front())),
            Command::NextKey => Some(Reply::Key(self.events.pop_front())),
            Command::SetAll(leds) => {
                self.leds = leds;
                None
            }
            Command::Set(pos, led) => {
                self.leds[index(pos)] = led;
                None
            }
            Command::Flash(pos, rgb) => {
                self.flash = Some((pos, rgb, now + FLASH));
                None
            }
        }
    }

    fn record(&mut self, error: ErrorCode) {
        let _ = self.errors.push_back(error);
    }

    /// Empties the key queue once the host is gone.
    fn forget_if_gone(&mut self, now: Millis) {
        if !self.hosted(now) {
            self.events.clear();
        }
    }

    fn hosted(&self, now: Millis) -> bool {
        self.dtr && self.last_line.is_some_and(|t| now.saturating_sub(t) < HOST_TIMEOUT)
    }
}

fn index(pos: Position) -> usize {
    match pos {
        Position::Left => 0,
        Position::Middle => 1,
        Position::Right => 2,
    }
}

/// One LED at `now`, before [`led_order`].
fn render(led: Led, now: Millis) -> Rgb {
    match led.mode {
        Mode::Off => DARK,
        Mode::Solid => scale(led.rgb, 1000),
        Mode::Blink if now % BLINK_PERIOD < BLINK_PERIOD / 2 => scale(led.rgb, 1000),
        Mode::Blink => DARK,
        Mode::Breathe => scale(led.rgb, breathe_level(now)),
    }
}

/// Breathe's brightness at `now` in per mille: a triangle from
/// [`BREATHE_FLOOR`] up to 1000 and back over [`BREATHE_PERIOD`].
fn breathe_level(now: Millis) -> u32 {
    let half = BREATHE_PERIOD / 2;
    let phase = now % BREATHE_PERIOD;
    let rise = if phase < half { phase } else { BREATHE_PERIOD - phase };
    BREATHE_FLOOR + ((1000 - BREATHE_FLOOR) as Millis * rise / half) as u32
}

/// `rgb` at `per_mille` brightness, capped at [`MAX_LEVEL`].
fn scale(rgb: Rgb, per_mille: u32) -> Rgb {
    let c = |v: u8| (v as u32 * MAX_LEVEL * per_mille / (255 * 1000)) as u8;
    Rgb { r: c(rgb.r), g: c(rgb.g), b: c(rgb.b) }
}

/// `rgb` in the byte order the LEDs need when driven through `ws2812-pio`,
/// which sends GRB to LEDs that take RGB.
pub fn led_order(rgb: Rgb) -> Rgb {
    Rgb { r: rgb.g, g: rgb.r, b: rgb.b }
}
