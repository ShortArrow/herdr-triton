//! When a held key counts as a long press (ADR 0015, ADR 0017).

/// How long, in milliseconds, a key stays down before it counts as held.
pub const HOLD: u64 = 500;

/// What holding Select does, `select_hold` in `config.toml` (ADR 0017).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectHold {
    /// Nothing; Select acts when it goes down.
    #[default]
    None,
    /// The back keys once; a short Select acts when it comes up.
    Up,
}

/// One key's press, in milliseconds on the runtime's clock. Invariant:
/// `since` is set only while the key is down and has not yet been due.
#[derive(Debug, Default)]
pub struct Hold {
    since: Option<u64>,
}

impl Hold {
    /// The key went down at `now`.
    pub fn down(&mut self, now: u64) {
        self.since = Some(now);
    }

    /// The key went up; whether the press was still short, not yet due.
    pub fn up(&mut self) -> bool {
        self.since.take().is_some()
    }

    /// Whether the press has just been held for [`HOLD`] at `now`: true once
    /// per press, and never for a key released earlier.
    pub fn due(&mut self, now: u64) -> bool {
        let held = self.since.is_some_and(|t| now.saturating_sub(t) >= HOLD);
        if held {
            self.since = None;
        }
        held
    }
}
