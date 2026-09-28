use keypad::{led_order, Keypad, Millis};
use protocol::scpi::{write_command, Command, ErrorCode, Reply, PROTOCOL_VERSION};
use protocol::{Edge, Led, Mode, Position, Position::*, Rgb};

const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };
const DARK: Rgb = Rgb { r: 0, g: 0, b: 0 };
/// Full brightness after the 64/255 cap.
const RED_OUT: Rgb = Rgb { r: 64, g: 0, b: 0 };
const WHITE_OUT: Rgb = Rgb { r: 64, g: 64, b: 64 };
const NO_HOST: Rgb = Rgb { r: 8, g: 8, b: 8 };
const SERIAL: &str = "TRITON-0123456789ABCDEF";
const VERSION: &str = "0.2.0";

fn keypad() -> Keypad {
    Keypad::new(SERIAL, VERSION)
}

/// Sends one line of text.
fn say(k: &mut Keypad, text: &str, now: Millis) -> Option<Reply<'static>> {
    k.line(Ok(text), now)
}

fn send(k: &mut Keypad, cmd: Command, now: Millis) -> Option<Reply<'static>> {
    let mut text = String::new();
    write_command(&cmd, &mut text).unwrap();
    say(k, &text, now)
}

fn set_all(leds: [(Rgb, Mode); 3]) -> Command {
    Command::SetAll(leds.map(|(rgb, mode)| Led { rgb, mode }))
}

/// A keypad with DTR high whose host spoke at t = 0 and set every LED red.
fn hosted() -> Keypad {
    let mut k = keypad();
    k.set_dtr(true);
    send(&mut k, set_all([(RED, Mode::Solid); 3]), 0);
    k
}

/// Holds `pressed` from `from` until `to` (inclusive) in 1 ms steps.
fn hold(k: &mut Keypad, pressed: [bool; 3], from: u64, to: u64) {
    for t in from..=to {
        k.scan(pressed, t);
    }
}

/// Reads key events until `NONE`.
fn drain_keys(k: &mut Keypad, now: Millis) -> Vec<(Position, Edge)> {
    std::iter::from_fn(|| match send(k, Command::NextKey, now) {
        Some(Reply::Key(event)) => event,
        other => panic!("{other:?}"),
    })
    .collect()
}

/// Reads errors until `0,"No error"`.
fn drain_errors(k: &mut Keypad, now: Millis) -> Vec<ErrorCode> {
    std::iter::from_fn(|| match send(k, Command::NextError, now) {
        Some(Reply::Error(code)) => code,
        other => panic!("{other:?}"),
    })
    .collect()
}

mod host_presence {
    use super::*;

    #[test]
    fn a_new_keypad_has_no_host_and_shows_dim_white() {
        assert_eq!(keypad().pixels(0), [NO_HOST; 3]);
    }

    #[test]
    fn a_line_with_dtr_high_ends_no_host_and_leds_start_dark() {
        let mut k = keypad();
        k.set_dtr(true);
        send(&mut k, Command::NextKey, 0);
        assert_eq!(k.pixels(0), [DARK; 3]);
    }

    #[test]
    fn a_mistyped_line_still_counts_as_the_host() {
        let mut k = keypad();
        k.set_dtr(true);
        say(&mut k, "HELLO?", 0);
        assert_eq!(k.pixels(0), [DARK; 3]);
    }

    #[test]
    fn a_line_while_dtr_is_low_is_answered_but_does_not_end_no_host() {
        let mut k = keypad();
        assert!(matches!(send(&mut k, Command::Identify, 0), Some(Reply::Identity { .. })));
        assert_eq!(k.pixels(0), [NO_HOST; 3]);
    }

    #[test]
    fn a_line_while_dtr_is_low_does_not_count_once_dtr_rises() {
        let mut k = keypad();
        send(&mut k, Command::NextKey, 0);
        k.set_dtr(true);
        assert_eq!(k.pixels(1), [NO_HOST; 3]);
    }

    #[test]
    fn three_seconds_without_a_line_is_no_host() {
        let k = hosted();
        assert_eq!(k.pixels(2999), [RED_OUT; 3]);
        assert_eq!(k.pixels(3000), [NO_HOST; 3]);
    }

    #[test]
    fn any_line_resets_the_three_seconds() {
        let mut k = hosted();
        send(&mut k, Command::NextKey, 2000);
        assert_eq!(k.pixels(4999), [RED_OUT; 3]);
    }

    #[test]
    fn dtr_going_low_is_no_host_at_once_and_coming_back_waits_for_a_line() {
        let mut k = hosted();
        k.set_dtr(false);
        assert_eq!(k.pixels(1), [NO_HOST; 3]);
        k.set_dtr(true);
        assert_eq!(k.pixels(2), [NO_HOST; 3]);
        send(&mut k, Command::NextKey, 3);
        assert_eq!(k.pixels(3), [RED_OUT; 3]);
    }

    #[test]
    fn the_leds_keep_their_last_state_across_no_host() {
        let mut k = hosted();
        assert_eq!(k.pixels(5000), [NO_HOST; 3]);
        send(&mut k, Command::NextKey, 6000);
        assert_eq!(k.pixels(6000), [RED_OUT; 3]);
    }
}

mod keys {
    use super::*;

    #[test]
    fn a_press_held_for_5_ms_queues_one_down() {
        let mut k = hosted();
        hold(&mut k, [false, true, false], 10, 14);
        assert_eq!(drain_keys(&mut k, 14), vec![]);
        hold(&mut k, [false, true, false], 15, 30);
        assert_eq!(drain_keys(&mut k, 30), vec![(Middle, Edge::Down)]);
    }

    #[test]
    fn events_are_read_oldest_first() {
        let mut k = hosted();
        hold(&mut k, [true, false, false], 10, 20);
        hold(&mut k, [false; 3], 21, 30);
        hold(&mut k, [false, false, true], 31, 40);
        assert_eq!(
            drain_keys(&mut k, 40),
            vec![(Left, Edge::Down), (Left, Edge::Up), (Right, Edge::Down)]
        );
    }

    #[test]
    fn bouncing_shorter_than_5_ms_queues_nothing() {
        let mut k = hosted();
        for t in 10..40u64 {
            k.scan([false, false, t % 3 == 0], t);
        }
        assert_eq!(drain_keys(&mut k, 40), vec![]);
    }

    #[test]
    fn presses_without_a_host_are_dropped_not_queued() {
        let mut k = keypad();
        k.set_dtr(true);
        hold(&mut k, [true, false, false], 10, 20);
        assert_eq!(drain_keys(&mut k, 21), vec![]);
        hold(&mut k, [false; 3], 22, 40);
        assert_eq!(drain_keys(&mut k, 40), vec![(Left, Edge::Up)]);
    }

    #[test]
    fn presses_without_a_host_never_overflow_the_queue() {
        let mut k = keypad();
        k.set_dtr(true);
        let mut t = 10;
        for _ in 0..9 {
            hold(&mut k, [true, false, false], t, t + 5);
            hold(&mut k, [false; 3], t + 6, t + 11);
            t += 12;
        }
        assert_eq!(drain_errors(&mut k, t), vec![]);
    }

    #[test]
    fn entering_no_host_empties_the_queue() {
        let mut k = hosted();
        hold(&mut k, [true, false, false], 10, 20);
        assert_eq!(drain_keys(&mut k, 5000), vec![]);
    }

    #[test]
    fn a_full_queue_drops_new_events_and_records_queue_overflow() {
        let mut k = hosted();
        let mut t = 10;
        for _ in 0..9 {
            hold(&mut k, [true, false, false], t, t + 5);
            hold(&mut k, [false; 3], t + 6, t + 11);
            t += 12;
        }
        // Nine presses make 18 events; 16 fit and each of the other two
        // records an overflow.
        assert_eq!(drain_keys(&mut k, t).len(), 16);
        assert_eq!(drain_errors(&mut k, t), vec![ErrorCode::QueueOverflow; 2]);
    }
}

mod commands {
    use super::*;

    #[test]
    fn identify_names_the_keypad() {
        let mut k = keypad();
        assert_eq!(
            send(&mut k, Command::Identify, 0),
            Some(Reply::Identity { serial: SERIAL, version: VERSION })
        );
    }

    #[test]
    fn protocol_answers_the_version() {
        assert_eq!(send(&mut keypad(), Command::Protocol, 0), Some(Reply::Protocol(PROTOCOL_VERSION)));
    }

    #[test]
    fn settings_have_no_reply() {
        let mut k = hosted();
        assert_eq!(send(&mut k, set_all([(RED, Mode::Off); 3]), 1), None);
        assert_eq!(send(&mut k, Command::Flash(Left, RED), 1), None);
    }

    #[test]
    fn a_bad_line_records_its_error_and_has_no_reply() {
        let mut k = hosted();
        assert_eq!(say(&mut k, "HELLO?", 1), None);
        assert_eq!(say(&mut k, "LED4 #000000,OFF", 1), None);
        assert_eq!(k.line(Err(ErrorCode::CommandError), 1), None);
        assert_eq!(
            drain_errors(&mut k, 1),
            vec![ErrorCode::UndefinedHeader, ErrorCode::DataOutOfRange, ErrorCode::CommandError]
        );
    }

    #[test]
    fn the_error_queue_holds_eight() {
        let mut k = hosted();
        for _ in 0..9 {
            say(&mut k, "HELLO?", 1);
        }
        assert_eq!(drain_errors(&mut k, 1).len(), 8);
    }

    #[test]
    fn an_empty_line_is_ignored() {
        let mut k = hosted();
        assert_eq!(say(&mut k, "", 1), None);
        assert_eq!(drain_errors(&mut k, 1), vec![]);
    }

    #[test]
    fn one_led_can_be_set_alone() {
        let mut k = hosted();
        send(&mut k, Command::Set(Right, Led { rgb: WHITE, mode: Mode::Solid }), 1);
        assert_eq!(k.pixels(1), [RED_OUT, RED_OUT, WHITE_OUT]);
    }
}

mod rendering {
    use super::*;

    fn showing(leds: [(Rgb, Mode); 3]) -> Keypad {
        let mut k = keypad();
        k.set_dtr(true);
        send(&mut k, set_all(leds), 0);
        k
    }

    #[test]
    fn off_is_dark_whatever_the_colour() {
        let k = showing([(RED, Mode::Off), (RED, Mode::Solid), (WHITE, Mode::Off)]);
        assert_eq!(k.pixels(0), [DARK, RED_OUT, DARK]);
    }

    #[test]
    fn blink_is_on_for_500_ms_then_off_for_500_ms() {
        let k = showing([(RED, Mode::Blink), (RED, Mode::Off), (RED, Mode::Off)]);
        assert_eq!(k.pixels(0)[0], RED_OUT);
        assert_eq!(k.pixels(499)[0], RED_OUT);
        assert_eq!(k.pixels(500)[0], DARK);
        assert_eq!(k.pixels(999)[0], DARK);
        assert_eq!(k.pixels(1000)[0], RED_OUT);
    }

    #[test]
    fn breathe_runs_from_10_to_100_percent_and_back_over_2_s() {
        let mut k = showing([(RED, Mode::Breathe), (RED, Mode::Off), (RED, Mode::Off)]);
        let red_at = |k: &Keypad, t: u64| k.pixels(t)[0].r;
        assert_eq!(red_at(&k, 0), 6);
        assert_eq!(red_at(&k, 1000), 64);
        send(&mut k, Command::NextKey, 2000);
        assert_eq!(red_at(&k, 2000), 6);
        let rising: Vec<u8> = (0..=1000).step_by(100).map(|t| red_at(&k, 2000 + t)).collect();
        assert!(rising.windows(2).all(|w| w[0] <= w[1]), "{rising:?}");
        let falling: Vec<u8> = (1000..=2000).step_by(100).map(|t| red_at(&k, 2000 + t)).collect();
        assert!(falling.windows(2).all(|w| w[0] >= w[1]), "{falling:?}");
    }

    #[test]
    fn a_flash_covers_its_key_for_150_ms() {
        let mut k = showing([(RED, Mode::Solid); 3]);
        send(&mut k, Command::Flash(Middle, WHITE), 100);
        assert_eq!(k.pixels(100), [RED_OUT, WHITE_OUT, RED_OUT]);
        assert_eq!(k.pixels(249), [RED_OUT, WHITE_OUT, RED_OUT]);
        assert_eq!(k.pixels(250), [RED_OUT; 3]);
    }

    #[test]
    fn a_flash_shows_on_a_dark_key() {
        let mut k = showing([(RED, Mode::Off); 3]);
        send(&mut k, Command::Flash(Right, RED), 0);
        assert_eq!(k.pixels(0), [DARK, DARK, RED_OUT]);
    }

    #[test]
    fn led_order_swaps_red_and_green() {
        assert_eq!(led_order(Rgb { r: 1, g: 2, b: 3 }), Rgb { r: 2, g: 1, b: 3 });
    }
}
