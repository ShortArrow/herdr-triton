//! The bridge's state machine: herdr snapshots and key presses in,
//! herdr requests and LED output out. No IO.

use std::collections::HashMap;

use protocol::{Led, Mode, Position, Rgb};

/// An agent's status as `agent.list` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Working,
    Blocked,
    Done,
    Unknown,
}

/// One entry of an `agent.list` snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub pane_id: String,
    pub agent: Option<String>,
    pub status: Status,
    pub focused: bool,
    pub state_change_seq: u64,
}

/// Reachability of herdr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conn {
    Disconnected,
    Incompatible,
    Connected,
}

/// Inputs to [`State::update`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// `ping` answered with a version below the minimum.
    Incompatible,
    Snapshot(Vec<Agent>),
    SnapshotFailed,
    KeyDown(Position),
    /// The outcome of the last `Cmd::Focus` or `Cmd::SendKeys`.
    RequestDone {
        ok: bool,
    },
}

/// Outputs of [`State::update`], executed in order by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Poll,
    Focus { pane_id: String },
    SendKeys { pane_id: String, keys: Vec<String> },
    Flash { pos: Position, rgb: Rgb },
}

/// Colours the bridge shows.
pub mod palette {
    use protocol::Rgb;

    pub const OFF: Rgb = Rgb { r: 0, g: 0, b: 0 };
    pub const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
    pub const AMBER: Rgb = Rgb {
        r: 255,
        g: 140,
        b: 0,
    };
    pub const REDDISH_AMBER: Rgb = Rgb {
        r: 255,
        g: 70,
        b: 0,
    };
    pub const GREEN: Rgb = Rgb { r: 0, g: 255, b: 0 };
    pub const BLUE: Rgb = Rgb { r: 0, g: 0, b: 255 };
    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
}

/// The keys that drive an agent's approval prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptKeys {
    /// Confirms the highlighted option (Approve).
    pub confirm: Vec<String>,
    /// Moves the highlight to the next option (Select).
    pub select: Vec<String>,
}

/// Prompt keys per herdr agent id.
pub type AgentKeys = HashMap<String, PromptKeys>;

/// A blocked agent waiting in the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    pane_id: String,
    agent: Option<String>,
    state_change_seq: u64,
}

/// A key that acts on the focused prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptAction {
    Confirm,
    Select,
}

impl PromptAction {
    fn position(self) -> Position {
        match self {
            PromptAction::Confirm => Position::Middle,
            PromptAction::Select => Position::Right,
        }
    }
}

/// What the bridge is waiting for between messages.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    None,
    /// A prompt key was pressed; the refreshed snapshot decides.
    Refresh(PromptAction),
    /// A request from `pos` is in flight; `sent` is the confirmation it recorded.
    Request {
        pos: Position,
        sent: Option<(String, u64)>,
    },
}

/// The bridge's state. Invariants: a pane appears in `queue` at most once,
/// every entry was blocked in the latest snapshot, and every `sent` pair
/// matches a queue entry's pane and seq.
pub struct State {
    agent_keys: AgentKeys,
    conn: Conn,
    queue: Vec<Entry>,
    focused: Option<String>,
    sent: Vec<(String, u64)>,
    pending: Pending,
}

impl State {
    pub fn new(agent_keys: AgentKeys) -> Self {
        Self {
            agent_keys,
            conn: Conn::Disconnected,
            queue: Vec::new(),
            focused: None,
            sent: Vec::new(),
            pending: Pending::None,
        }
    }

    pub fn update(&mut self, msg: Msg) -> Vec<Cmd> {
        match msg {
            Msg::Incompatible => {
                self.lose_herdr(Conn::Incompatible);
                Vec::new()
            }
            Msg::SnapshotFailed => {
                let pending = std::mem::replace(&mut self.pending, Pending::None);
                self.lose_herdr(Conn::Disconnected);
                match pending {
                    Pending::Refresh(action) => error(action.position()),
                    _ => Vec::new(),
                }
            }
            Msg::Snapshot(agents) => {
                self.conn = Conn::Connected;
                self.apply_snapshot(&agents);
                match std::mem::replace(&mut self.pending, Pending::None) {
                    Pending::Refresh(action) => self.act_on_prompt(action),
                    other => {
                        self.pending = other;
                        Vec::new()
                    }
                }
            }
            Msg::KeyDown(pos) if self.conn != Conn::Connected => error(pos),
            Msg::KeyDown(Position::Left) => self.jump(),
            Msg::KeyDown(Position::Middle) => self.refresh_for(PromptAction::Confirm),
            Msg::KeyDown(Position::Right) => self.refresh_for(PromptAction::Select),
            Msg::RequestDone { ok } => self.finish_request(ok),
        }
    }

    /// The steady LED output, left to right.
    pub fn frame(&self) -> [Led; 3] {
        let dark = Led {
            rgb: palette::OFF,
            mode: Mode::Off,
        };
        let solid = |rgb| Led {
            rgb,
            mode: Mode::Solid,
        };
        match self.conn {
            Conn::Disconnected => {
                [Led {
                    rgb: palette::RED,
                    mode: Mode::Blink,
                }; 3]
            }
            Conn::Incompatible => [solid(palette::RED); 3],
            Conn::Connected => {
                let jump = match self.queue.len() {
                    0 => dark,
                    1 => Led {
                        rgb: palette::AMBER,
                        mode: Mode::Breathe,
                    },
                    _ => Led {
                        rgb: palette::REDDISH_AMBER,
                        mode: Mode::Breathe,
                    },
                };
                let (approve, select) = match self.approvable() {
                    Some(_) => (solid(palette::GREEN), solid(palette::BLUE)),
                    None => (dark, dark),
                };
                [jump, approve, select]
            }
        }
    }

    fn lose_herdr(&mut self, conn: Conn) {
        self.conn = conn;
        self.queue.clear();
        self.focused = None;
        self.sent.clear();
        self.pending = Pending::None;
    }

    fn apply_snapshot(&mut self, agents: &[Agent]) {
        let blocked = |pane: &str| {
            agents
                .iter()
                .find(|a| a.pane_id == pane && a.status == Status::Blocked)
        };
        let mut queue: Vec<Entry> = self
            .queue
            .iter()
            .filter_map(|e| blocked(&e.pane_id).map(entry_of))
            .collect();
        for a in agents.iter().filter(|a| a.status == Status::Blocked) {
            if !queue.iter().any(|e| e.pane_id == a.pane_id) {
                queue.push(entry_of(a));
            }
        }
        self.queue = queue;
        self.focused = agents.iter().find(|a| a.focused).map(|a| a.pane_id.clone());
        let queue = &self.queue;
        self.sent.retain(|(pane, seq)| {
            queue
                .iter()
                .any(|e| &e.pane_id == pane && e.state_change_seq == *seq)
        });
    }

    /// The entry after the focused one, cycling; the head if focus is elsewhere.
    fn jump_target(&self) -> Option<String> {
        let index = match self.focused_index() {
            Some(i) => (i + 1) % self.queue.len(),
            None => 0,
        };
        self.queue.get(index).map(|e| e.pane_id.clone())
    }

    fn focused_index(&self) -> Option<usize> {
        let focused = self.focused.as_deref()?;
        self.queue.iter().position(|e| e.pane_id == focused)
    }

    fn jump(&mut self) -> Vec<Cmd> {
        match self.jump_target() {
            Some(pane_id) => {
                self.pending = Pending::Request {
                    pos: Position::Left,
                    sent: None,
                };
                vec![Cmd::Focus { pane_id }]
            }
            None => error(Position::Left),
        }
    }

    fn refresh_for(&mut self, action: PromptAction) -> Vec<Cmd> {
        self.pending = Pending::Refresh(action);
        vec![Cmd::Poll]
    }

    /// The focused queue entry and its prompt keys, if Approve and Select would act.
    fn approvable(&self) -> Option<(&Entry, &PromptKeys)> {
        let entry = &self.queue[self.focused_index()?];
        let keys = self.agent_keys.get(entry.agent.as_deref()?)?;
        let already_sent = self
            .sent
            .iter()
            .any(|(pane, seq)| *pane == entry.pane_id && *seq == entry.state_change_seq);
        (!already_sent).then_some((entry, keys))
    }

    fn act_on_prompt(&mut self, action: PromptAction) -> Vec<Cmd> {
        let pos = action.position();
        let Some((entry, keys)) = self.approvable() else {
            return error(pos);
        };
        let pane_id = entry.pane_id.clone();
        let (keys, sent) = match action {
            PromptAction::Confirm => (
                keys.confirm.clone(),
                Some((pane_id.clone(), entry.state_change_seq)),
            ),
            PromptAction::Select => (keys.select.clone(), None),
        };
        if let Some(mark) = &sent {
            self.sent.push(mark.clone());
        }
        self.pending = Pending::Request { pos, sent };
        vec![Cmd::SendKeys { pane_id, keys }]
    }

    fn finish_request(&mut self, ok: bool) -> Vec<Cmd> {
        let Pending::Request { pos, sent } = std::mem::replace(&mut self.pending, Pending::None)
        else {
            return Vec::new();
        };
        if !ok {
            if let Some(mark) = sent {
                self.sent.retain(|m| *m != mark);
            }
        }
        let rgb = if ok { palette::WHITE } else { palette::RED };
        vec![Cmd::Flash { pos, rgb }, Cmd::Poll]
    }
}

fn entry_of(a: &Agent) -> Entry {
    Entry {
        pane_id: a.pane_id.clone(),
        agent: a.agent.clone(),
        state_change_seq: a.state_change_seq,
    }
}

fn error(pos: Position) -> Vec<Cmd> {
    vec![Cmd::Flash {
        pos,
        rgb: palette::RED,
    }]
}
