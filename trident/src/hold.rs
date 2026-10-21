//! When a held key counts as a long press (ADR 0015).

/// How long, in milliseconds, a key stays down before it counts as held.
pub const HOLD: u64 = 500;

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

    /// The key went up.
    pub fn up(&mut self) {
        self.since = None;
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
