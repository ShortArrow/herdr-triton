//! trident's state machine: herdr snapshots and key presses in,
//! herdr requests and LED output out. No IO.

use std::collections::BTreeMap;

use protocol::{Led, Mode, Rgb};

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

/// The keypad's keys, by what they do (specification, "Keys"). Which
/// position each takes is the runtime's concern (ADR 0013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Jump,
    Approve,
    Select,
}

impl Key {
    /// Every key, in the order of [`State::frame`].
    pub const ALL: [Key; 3] = [Key::Jump, Key::Approve, Key::Select];
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
    KeyDown(Key),
    /// A key has stayed down for [`crate::hold::HOLD`]; only Select acts on
    /// it, when `select_hold` asks for it (ADR 0017).
    KeyHeld(Key),
    /// The outcome of the last `Cmd::Focus` or `Cmd::SendKeys`.
    RequestDone {
        ok: bool,
    },
    /// The screen `Cmd::ReadScreen` asked for; `None` if it could not be read.
    Screen(Option<String>),
}

/// Outputs of [`State::update`], executed in order by the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    Poll,
    Focus {
        pane_id: String,
    },
    SendKeys {
        pane_id: String,
        keys: Vec<String>,
    },
    Flash {
        key: Key,
        rgb: Rgb,
    },
    /// Read the pane's visible screen, answered with `Msg::Screen`.
    ReadScreen {
        pane_id: String,
    },
}

/// Colours trident shows.
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
    /// Moves the highlight to the option above.
    pub back: Vec<String>,
    pub wrap: Wrap,
}

/// How Select gets from an agent's last option back to its first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wrap {
    /// The select keys wrap on their own.
    Native,
    /// The list stops at its last option: read the screen, and from the
    /// last option send the back keys once per option above it (ADR 0010).
    ByScreen,
}

/// Prompt keys per herdr agent id.
pub type AgentKeys = BTreeMap<String, PromptKeys>;

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
    /// A held Select: the back keys once (ADR 0017).
    Back,
}

impl PromptAction {
    fn key(self) -> Key {
        match self {
            PromptAction::Confirm => Key::Approve,
            PromptAction::Select | PromptAction::Back => Key::Select,
        }
    }
}

/// What trident is waiting for between messages.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    None,
    /// A prompt key was pressed; the refreshed snapshot decides.
    Refresh(PromptAction),
    /// Select or Back is waiting for the screen of `pane_id` to choose its
    /// keys; `lapsed` if the pane is in [`State::lapsed`] rather than the queue.
    Screen {
        action: PromptAction,
        pane_id: String,
        keys: PromptKeys,
        lapsed: bool,
    },
    /// A request from `key` is in flight; `sent` is the confirmation it recorded.
    Request {
        key: Key,
        sent: Option<(String, u64)>,
    },
}

/// trident's state. Invariants: a pane appears in `queue` at most once,
/// every entry was blocked in the latest snapshot, every `sent` pair
/// matches a queue entry's pane and seq, and every `lapsed` pane was idle
/// in the latest snapshot and is not in `queue`.
pub struct State {
    agent_keys: AgentKeys,
    conn: Conn,
    queue: Vec<Entry>,
    focused: Option<String>,
    sent: Vec<(String, u64)>,
    /// The latest snapshot, in `agent.list` order.
    agents: Vec<Agent>,
    /// Entries that left `queue` because their pane turned idle, kept while
    /// it stays idle; only Select acts on them (ADR 0016).
    lapsed: Vec<Entry>,
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
            agents: Vec::new(),
            lapsed: Vec::new(),
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
                    Pending::Refresh(action) => error(action.key()),
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
            Msg::KeyDown(key) if self.conn != Conn::Connected => error(key),
            Msg::KeyDown(Key::Jump) => self.jump(),
            Msg::KeyDown(Key::Approve) => self.refresh_for(PromptAction::Confirm),
            Msg::KeyDown(Key::Select) => self.refresh_for(PromptAction::Select),
            Msg::KeyHeld(Key::Select) if self.conn != Conn::Connected => error(Key::Select),
            Msg::KeyHeld(Key::Select) => self.refresh_for(PromptAction::Back),
            Msg::KeyHeld(_) => Vec::new(),
            Msg::RequestDone { ok } => self.finish_request(ok),
            Msg::Screen(screen) => self.select_on_screen(screen.as_deref()),
        }
    }

    /// How many agents have finished and not been seen.
    pub fn finished(&self) -> usize {
        self.done().count()
    }

    /// How many agents are waiting.
    pub fn waiting(&self) -> usize {
        self.queue.len()
    }

    pub fn conn(&self) -> Conn {
        self.conn
    }

    /// The steady LED output for Jump, Approve and Select, in that order.
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
            Conn::Connected if self.queue.is_empty() && self.finished() == 0 => [
                Led {
                    rgb: palette::WHITE,
                    mode: Mode::Breathe,
                },
                dark,
                dark,
            ],
            Conn::Connected => {
                let jump = match self.queue.len() {
                    0 => Led {
                        rgb: palette::GREEN,
                        mode: Mode::Breathe,
                    },
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
        self.agents.clear();
        self.focused = None;
        self.sent.clear();
        self.lapsed.clear();
        self.pending = Pending::None;
    }

    fn apply_snapshot(&mut self, agents: &[Agent]) {
        let with_status = |pane: &str, status: Status| {
            agents
                .iter()
                .find(|a| a.pane_id == pane && a.status == status)
        };
        let blocked = |pane: &str| with_status(pane, Status::Blocked);
        let idle = |pane: &str| with_status(pane, Status::Idle);
        let sent = &self.sent;
        let newly_lapsed: Vec<Entry> = self
            .queue
            .iter()
            .filter(|e| {
                !sent
                    .iter()
                    .any(|(pane, seq)| *pane == e.pane_id && *seq == e.state_change_seq)
            })
            .filter_map(|e| idle(&e.pane_id).map(entry_of))
            .collect();
        self.lapsed.retain(|e| idle(&e.pane_id).is_some());
        self.lapsed.extend(newly_lapsed);
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
        self.agents = agents.to_vec();
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

    /// Agents that have finished and not been seen, in `agent.list` order.
    fn done(&self) -> impl Iterator<Item = &Agent> {
        self.agents.iter().filter(|a| a.status == Status::Done)
    }

    fn focused_index(&self) -> Option<usize> {
        let focused = self.focused.as_deref()?;
        self.queue.iter().position(|e| e.pane_id == focused)
    }

    /// Jump's tiers (ADR 0011): waiting agents, then done ones, then every agent.
    fn jump(&mut self) -> Vec<Cmd> {
        let focused = self.focused.as_deref();
        let within = |agents: Vec<&Agent>| {
            let ids: Vec<&str> = agents.iter().map(|a| a.pane_id.as_str()).collect();
            next_after(&ids, focused)
        };
        let target = self
            .jump_target()
            .or_else(|| within(self.done().collect()))
            .or_else(|| within(self.agents.iter().collect()));
        match target {
            Some(pane_id) => {
                self.pending = Pending::Request {
                    key: Key::Jump,
                    sent: None,
                };
                vec![Cmd::Focus { pane_id }]
            }
            None => error(Key::Jump),
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

    /// The focused lapsed entry and its prompt keys, if Select would read
    /// its screen (ADR 0016).
    fn lapsed_focus(&self) -> Option<(&Entry, &PromptKeys)> {
        let focused = self.focused.as_deref()?;
        let entry = self.lapsed.iter().find(|e| e.pane_id == focused)?;
        let keys = self.agent_keys.get(entry.agent.as_deref()?)?;
        Some((entry, keys))
    }

    fn act_on_prompt(&mut self, action: PromptAction) -> Vec<Cmd> {
        let key = action.key();
        let Some((entry, keys)) = self.approvable() else {
            return match (action, self.lapsed_focus()) {
                (PromptAction::Select | PromptAction::Back, Some((entry, keys))) => {
                    let pane_id = entry.pane_id.clone();
                    self.pending = Pending::Screen {
                        action,
                        pane_id: pane_id.clone(),
                        keys: keys.clone(),
                        lapsed: true,
                    };
                    vec![Cmd::ReadScreen { pane_id }]
                }
                _ => error(key),
            };
        };
        let pane_id = entry.pane_id.clone();
        let (keys, sent) = match action {
            PromptAction::Confirm => (
                keys.confirm.clone(),
                Some((pane_id.clone(), entry.state_change_seq)),
            ),
            PromptAction::Select if keys.wrap != Wrap::Native => {
                self.pending = Pending::Screen {
                    action,
                    pane_id: pane_id.clone(),
                    keys: keys.clone(),
                    lapsed: false,
                };
                return vec![Cmd::ReadScreen { pane_id }];
            }
            PromptAction::Select => (keys.select.clone(), None),
            PromptAction::Back => (keys.back.clone(), None),
        };
        if let Some(mark) = &sent {
            self.sent.push(mark.clone());
        }
        self.pending = Pending::Request { key, sent };
        vec![Cmd::SendKeys { pane_id, keys }]
    }

    /// Sends Select's keys once the screen is known: back to the first
    /// option from the last of a list, down otherwise (ADR 0010). A lapsed
    /// pane without a highlighted list gets nothing (ADR 0016); Back sends
    /// the back keys once (ADR 0017).
    fn select_on_screen(&mut self, screen: Option<&str>) -> Vec<Cmd> {
        let Pending::Screen {
            action,
            pane_id,
            keys,
            lapsed,
        } = std::mem::replace(&mut self.pending, Pending::None)
        else {
            return Vec::new();
        };
        let prompt = screen.and_then(crate::prompt_screen::parse);
        if lapsed && prompt.as_ref().and_then(|p| p.highlighted).is_none() {
            return error(Key::Select);
        }
        let at_last = prompt.and_then(|p| {
            let last = p.options.last()?.0;
            (last >= 2 && p.highlighted == Some(last)).then_some(last)
        });
        let keys = match (action, at_last, &keys.wrap) {
            (PromptAction::Back, _, _) => keys.back,
            (_, Some(n), Wrap::ByScreen) => keys
                .back
                .iter()
                .cloned()
                .cycle()
                .take(keys.back.len() * (n - 1) as usize)
                .collect(),
            _ => keys.select,
        };
        self.pending = Pending::Request {
            key: Key::Select,
            sent: None,
        };
        vec![Cmd::SendKeys { pane_id, keys }]
    }

    fn finish_request(&mut self, ok: bool) -> Vec<Cmd> {
        let Pending::Request { key, sent } = std::mem::replace(&mut self.pending, Pending::None)
        else {
            return Vec::new();
        };
        if !ok {
            if let Some(mark) = sent {
                self.sent.retain(|m| *m != mark);
            }
        }
        let rgb = if ok { palette::WHITE } else { palette::RED };
        vec![Cmd::Flash { key, rgb }, Cmd::Poll]
    }
}

fn entry_of(a: &Agent) -> Entry {
    Entry {
        pane_id: a.pane_id.clone(),
        agent: a.agent.clone(),
        state_change_seq: a.state_change_seq,
    }
}

/// The item after `focused` in `items`, cycling; the first if `focused` is
/// not among them; none if `items` is empty.
fn next_after(items: &[&str], focused: Option<&str>) -> Option<String> {
    let index = match focused.and_then(|f| items.iter().position(|i| *i == f)) {
        Some(i) => (i + 1) % items.len(),
        None => 0,
    };
    items.get(index).map(|i| i.to_string())
}

fn error(key: Key) -> Vec<Cmd> {
    vec![Cmd::Flash {
        key,
        rgb: palette::RED,
    }]
}
