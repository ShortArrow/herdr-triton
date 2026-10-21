//! When a held Jump brings herdr's window to the front (ADR 0015).

use trident::hold::{Hold, HOLD};

#[test]
fn a_key_never_pressed_is_never_due() {
    let mut hold = Hold::default();
    assert!(!hold.due(10_000));
}

#[test]
fn a_press_is_not_due_before_the_hold_time() {
    let mut hold = Hold::default();
    hold.down(1_000);
    assert!(!hold.due(1_000 + HOLD - 1));
}

#[test]
fn a_press_is_due_once_the_hold_time_has_passed() {
    let mut hold = Hold::default();
    hold.down(1_000);
    assert!(hold.due(1_000 + HOLD));
}

#[test]
fn a_press_is_due_only_once() {
    let mut hold = Hold::default();
    hold.down(1_000);
    assert!(hold.due(1_000 + HOLD));
    assert!(!hold.due(1_000 + HOLD + 1_000));
}

#[test]
fn a_key_released_before_the_hold_time_is_never_due() {
    let mut hold = Hold::default();
    hold.down(1_000);
    hold.up();
    assert!(!hold.due(1_000 + HOLD));
}

#[test]
fn the_next_press_is_due_again() {
    let mut hold = Hold::default();
    hold.down(1_000);
    assert!(hold.due(1_000 + HOLD));
    hold.up();
    hold.down(5_000);
    assert!(!hold.due(5_000 + HOLD - 1));
    assert!(hold.due(5_000 + HOLD));
}

#[test]
fn the_hold_time_is_500_ms() {
    assert_eq!(HOLD, 500);
}
