//! The bridge's loop: joins herdr, the state core and the keypad, and
//! decides when a listener is quiet, active, or done (ADR 0012).

use std::collections::VecDeque;
use std::io;

use protocol::{Edge, Led, Position, Rgb};

use crate::herdr::client::{CallError, Client};
use crate::herdr::wire::{is_supported, Request, Response};
use crate::state::{AgentKeys, Cmd, Conn, Msg, State};

/// Milliseconds on a monotonic clock.
pub type Millis = u64;

/// How often `agent.list` is polled.
pub const POLL: Millis = 250;
/// How often the frame is resent although it has not changed.
pub const RESEND: Millis = 1000;
/// How long a listener stays active with nothing waiting, and how long it
/// keeps going without herdr.
pub const LINGER: Millis = 5000;
/// How often an active loop reads the keys.
pub const ACTIVE_TICK: Millis = 20;
/// How often a quiet loop reads the keys.
pub const QUIET_TICK: Millis = 250;
/// How often a quiet listener checks that herdr is still there.
pub const PRESENCE: Millis = 1000;

/// herdr's API, one request at a time.
pub trait Herdr {
    fn call(&mut self, request: &Request) -> Result<Response, CallError>;
    /// Whether herdr's endpoint exists, found without sending a request.
    fn present(&mut self) -> bool;
}

/// The keypad's keys and LEDs.
pub trait Keys {
    fn next_key(&mut self) -> io::Result<Option<(Position, Edge)>>;
    fn show(&mut self, leds: [Led; 3]) -> io::Result<()>;
    fn flash(&mut self, pos: Position, rgb: Rgb) -> io::Result<()>;
}

/// `Run` stays active and keeps going; `Listen` goes quiet while nothing
/// waits, and exits when herdr is gone or unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Run,
    Listen,
}

/// Why the loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    HerdrGone,
    Incompatible,
    DeviceLost,
}

/// Active polls herdr; quiet only reads the keys and checks, every
/// [`PRESENCE`] since `checked`, that herdr is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Active,
    Quiet { checked: Millis },
}

pub struct Runtime<H, K> {
    state: State,
    herdr: H,
    keys: K,
    mode: Mode,
    phase: Phase,
    ping_due: bool,
    last_poll: Option<Millis>,
    shown: Option<([Led; 3], Millis)>,
    idle_since: Option<Millis>,
    gone_since: Option<Millis>,
}

impl<H: Herdr, K: Keys> Runtime<H, K> {
    pub fn new(herdr: H, keys: K, agent_keys: AgentKeys, mode: Mode) -> Self {
        Self {
            state: State::new(agent_keys),
            herdr,
            keys,
            mode,
            phase: Phase::Active,
            ping_due: true,
            last_poll: None,
            shown: None,
            idle_since: None,
            gone_since: None,
        }
    }

    pub fn herdr(&self) -> &H {
        &self.herdr
    }

    pub fn herdr_mut(&mut self) -> &mut H {
        &mut self.herdr
    }

    pub fn keys(&self) -> &K {
        &self.keys
    }

    pub fn keys_mut(&mut self) -> &mut K {
        &mut self.keys
    }

    /// Checks herdr's version and takes the first snapshot.
    pub fn start(&mut self, now: Millis) -> Result<(), Exit> {
        self.activate(now)?;
        self.watch(now)
    }

    /// A hook's wake: a quiet listener becomes active on a fresh snapshot.
    pub fn wake(&mut self, now: Millis) -> Result<(), Exit> {
        match self.phase {
            Phase::Quiet { .. } => {
                self.activate(now)?;
                self.watch(now)
            }
            Phase::Active => Ok(()),
        }
    }

    /// How long the caller waits between ticks.
    pub fn interval(&self) -> Millis {
        match self.phase {
            Phase::Active => ACTIVE_TICK,
            Phase::Quiet { .. } => QUIET_TICK,
        }
    }

    /// One pass: key events, the periodic poll, the LEDs, and the phase and
    /// exit rules. A key going down while quiet first makes the loop active.
    pub fn tick(&mut self, now: Millis) -> Result<(), Exit> {
        while let Some((pos, edge)) = self.keys.next_key().map_err(|_| Exit::DeviceLost)? {
            if edge == Edge::Down {
                self.wake(now)?;
                let cmds = self.state.update(Msg::KeyDown(pos));
                self.run(cmds, now)?;
            }
        }
        let due = self.last_poll.is_none_or(|t| now.saturating_sub(t) >= POLL);
        if self.phase == Phase::Active && due && self.state.conn() != Conn::Incompatible {
            self.run(vec![Cmd::Poll], now)?;
        }
        let frame = self.state.frame();
        let stale = self
            .shown
            .is_none_or(|(shown, at)| shown != frame || now.saturating_sub(at) >= RESEND);
        if stale {
            self.keys.show(frame).map_err(|_| Exit::DeviceLost)?;
            self.shown = Some((frame, now));
        }
        self.watch(now)
    }

    /// Runs commands, and those they lead to, in order.
    fn run(&mut self, cmds: Vec<Cmd>, now: Millis) -> Result<(), Exit> {
        let mut work: VecDeque<Cmd> = cmds.into();
        while let Some(cmd) = work.pop_front() {
            let msg = match cmd {
                Cmd::Poll => {
                    self.last_poll = Some(now);
                    self.snapshot()?
                }
                Cmd::Focus { pane_id } => {
                    let reply = self.herdr.call(&Request::AgentFocus { target: pane_id });
                    Msg::RequestDone {
                        ok: matches!(reply, Ok(Response::Agent(_) | Response::Ok)),
                    }
                }
                Cmd::SendKeys { pane_id, keys } => {
                    let reply = self.herdr.call(&Request::AgentSendKeys {
                        target: pane_id,
                        keys,
                    });
                    Msg::RequestDone {
                        ok: matches!(reply, Ok(Response::Ok)),
                    }
                }
                Cmd::ReadScreen { pane_id } => {
                    match self.herdr.call(&Request::AgentRead { target: pane_id }) {
                        Ok(Response::Screen(text)) => Msg::Screen(Some(text)),
                        _ => Msg::Screen(None),
                    }
                }
                Cmd::Flash { pos, rgb } => {
                    self.keys.flash(pos, rgb).map_err(|_| Exit::DeviceLost)?;
                    continue;
                }
            };
            work.extend(self.state.update(msg));
        }
        Ok(())
    }

    /// Makes the loop active: the next poll pings first, and it runs now.
    fn activate(&mut self, now: Millis) -> Result<(), Exit> {
        self.phase = Phase::Active;
        self.ping_due = true;
        self.idle_since = None;
        self.gone_since = None;
        self.run(vec![Cmd::Poll], now)
    }

    /// Lists the agents, pinging herdr first when it was unreachable or has
    /// not been asked since becoming active.
    fn snapshot(&mut self) -> Result<Msg, Exit> {
        if self.ping_due {
            match self.herdr.call(&Request::Ping) {
                Ok(Response::Pong { version }) if is_supported(&version) => {}
                Ok(Response::Pong { .. }) if self.mode == Mode::Listen => {
                    return Err(Exit::Incompatible)
                }
                Ok(Response::Pong { .. }) => return Ok(Msg::Incompatible),
                _ => return Ok(Msg::SnapshotFailed),
            }
            self.ping_due = false;
        }
        match self.herdr.call(&Request::AgentList) {
            Ok(Response::Agents(agents)) => Ok(Msg::Snapshot(agents)),
            Err(CallError::Io(_)) => {
                self.ping_due = true;
                Ok(Msg::SnapshotFailed)
            }
            _ => Ok(Msg::SnapshotFailed),
        }
    }

    /// Tracks how long nothing has waited and how long herdr has been gone.
    /// A listener goes quiet after [`LINGER`] of the first, and exits after
    /// [`LINGER`] of the second; while quiet, herdr is gone when its
    /// endpoint is.
    fn watch(&mut self, now: Millis) -> Result<(), Exit> {
        let gone = match self.phase {
            Phase::Active => Some(self.state.conn() == Conn::Disconnected),
            Phase::Quiet { checked } if now.saturating_sub(checked) >= PRESENCE => {
                self.phase = Phase::Quiet { checked: now };
                Some(!self.herdr.present())
            }
            Phase::Quiet { .. } => None,
        };
        let idle = self.state.waiting() == 0 && self.state.finished() == 0;
        let since = |flag: bool, clock: &mut Option<Millis>| match flag {
            true => Some(*clock.get_or_insert(now)),
            false => {
                *clock = None;
                None
            }
        };
        if let Some(gone) = gone {
            since(gone, &mut self.gone_since);
        }
        let idle_since = since(idle, &mut self.idle_since);
        if self.mode == Mode::Run {
            return Ok(());
        }
        let lasted = |t: Option<Millis>| t.is_some_and(|t| now.saturating_sub(t) >= LINGER);
        if lasted(self.gone_since) {
            Err(Exit::HerdrGone)
        } else {
            if self.phase == Phase::Active && lasted(idle_since) {
                self.phase = Phase::Quiet { checked: now };
            }
            Ok(())
        }
    }
}

impl Herdr for Client {
    fn call(&mut self, request: &Request) -> Result<Response, CallError> {
        Client::call(self, request)
    }

    fn present(&mut self) -> bool {
        Client::present(self)
    }
}
