//! Which key sits at which position on the keypad (ADR 0013).

use protocol::{Led, Position};

use crate::state::Key;

/// The keys at the left, middle and right position; each key once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout([Key; 3]);

impl Default for Layout {
    /// ADR 0006's order: Jump, Approve, Select.
    fn default() -> Self {
        Layout(Key::ALL)
    }
}

impl Layout {
    /// The layout with `keys` at the left, middle and right position, or
    /// `None` unless each key appears once.
    pub fn new(keys: [Key; 3]) -> Option<Self> {
        Key::ALL
            .iter()
            .all(|k| keys.contains(k))
            .then_some(Layout(keys))
    }

    /// The key at `pos`.
    pub fn key_at(&self, pos: Position) -> Key {
        self.0[index(pos)]
    }

    /// Where `key` sits.
    pub fn position_of(&self, key: Key) -> Position {
        let i = self
            .0
            .iter()
            .position(|k| *k == key)
            .expect("every key once");
        [Position::Left, Position::Middle, Position::Right][i]
    }

    /// Puts each key's LED, given in [`Key::ALL`] order, at its position.
    pub fn arrange(&self, by_key: [Led; 3]) -> [Led; 3] {
        self.0
            .map(|key| by_key[Key::ALL.iter().position(|k| *k == key).expect("in ALL")])
    }
}

fn index(pos: Position) -> usize {
    match pos {
        Position::Left => 0,
        Position::Middle => 1,
        Position::Right => 2,
    }
}
