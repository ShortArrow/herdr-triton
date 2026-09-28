//! The bridge's loop: joins herdr, the state core and the keypad, and
//! decides when a hook's listener exits (ADR 0008).

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
/// How long a hook's listener stays with nothing waiting, or without herdr.
pub const LINGER: Millis = 5000;

/// herdr's API, one request at a time.
pub trait Herdr {
    fn call(&mut self, request: &Request) -> Result<Response, CallError>;
}

/// The keypad's keys and LEDs.
pub trait Keys {
    fn next_key(&mut self) -> io::Result<Option<(Position, Edge)>>;
    fn show(&mut self, leds: [Led; 3]) -> io::Result<()>;
    fn flash(&mut self, pos: Position, rgb: Rgb) -> io::Result<()>;
}

/// `Run` keeps going; `Hook` exits when idle or when herdr is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Run,
    Hook,
}

/// Why the loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Idle,
    HerdrGone,
    DeviceLost,
}

pub struct Runtime<H, K> {
    state: State,
    herdr: H,
    keys: K,
    mode: Mode,
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
        let cmds = match self.herdr.call(&Request::Ping) {
            Ok(Response::Pong { version }) if is_supported(&version) => vec![Cmd::Poll],
            Ok(Response::Pong { .. }) => self.state.update(Msg::Incompatible),
            _ => self.state.update(Msg::SnapshotFailed),
        };
        self.run(cmds, now)?;
        self.watch(now)
    }

    /// One pass: key events, the periodic poll, the LEDs, and the exit rules.
    pub fn tick(&mut self, now: Millis) -> Result<(), Exit> {
        while let Some((pos, edge)) = self.keys.next_key().map_err(|_| Exit::DeviceLost)? {
            if edge == Edge::Down {
                let cmds = self.state.update(Msg::KeyDown(pos));
                self.run(cmds, now)?;
            }
        }
        let due = self.last_poll.is_none_or(|t| now.saturating_sub(t) >= POLL);
        if due && self.state.conn() != Conn::Incompatible {
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
                    match self.herdr.call(&Request::AgentList) {
                        Ok(Response::Agents(agents)) => Msg::Snapshot(agents),
                        _ => Msg::SnapshotFailed,
                    }
                }
                Cmd::Focus { pane_id } => {
                    let reply = self.herdr.call(&Request::AgentFocus { target: pane_id });
                    Msg::RequestDone { ok: matches!(reply, Ok(Response::Agent(_) | Response::Ok)) }
                }
                Cmd::SendKeys { pane_id, keys } => {
                    let reply = self.herdr.call(&Request::AgentSendKeys { target: pane_id, keys });
                    Msg::RequestDone { ok: matches!(reply, Ok(Response::Ok)) }
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

    /// Tracks how long nothing has waited and how long herdr has been gone,
    /// and ends a hook's loop after [`LINGER`] of either.
    fn watch(&mut self, now: Millis) -> Result<(), Exit> {
        let gone = self.state.conn() == Conn::Disconnected;
        let idle = self.state.waiting() == 0;
        let since = |flag: bool, clock: &mut Option<Millis>| match flag {
            true => Some(*clock.get_or_insert(now)),
            false => {
                *clock = None;
                None
            }
        };
        let gone_since = since(gone, &mut self.gone_since);
        let idle_since = since(idle, &mut self.idle_since);
        if self.mode == Mode::Run {
            return Ok(());
        }
        let lasted = |t: Option<Millis>| t.is_some_and(|t| now.saturating_sub(t) >= LINGER);
        if lasted(gone_since) {
            Err(Exit::HerdrGone)
        } else if lasted(idle_since) {
            Err(Exit::Idle)
        } else {
            Ok(())
        }
    }
}

impl Herdr for Client {
    fn call(&mut self, request: &Request) -> Result<Response, CallError> {
        Client::call(self, request)
    }
}
