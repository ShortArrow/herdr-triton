use keypad::{led_order, Keypad};
use protocol::{
    DeviceMessage, Edge, HostMessage, Led, Mode, Position, Position::*, Rgb, PROTOCOL_VERSION,
};

const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };
const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };
const DARK: Rgb = Rgb { r: 0, g: 0, b: 0 };
/// Full brightness after the 64/255 cap.
const RED_OUT: Rgb = Rgb { r: 64, g: 0, b: 0 };
const WHITE_OUT: Rgb = Rgb { r: 64, g: 64, b: 64 };
const NO_HOST: Rgb = Rgb { r: 8, g: 8, b: 8 };

const READY: DeviceMessage = DeviceMessage::Ready { protocol: PROTOCOL_VERSION };

fn frame(leds: [(Rgb, Mode); 3]) -> HostMessage {
    HostMessage::Frame(leds.map(|(rgb, mode)| Led { rgb, mode }))
}

fn solid_red() -> HostMessage {
    frame([(RED, Mode::Solid), (RED, Mode::Solid), (RED, Mode::Solid)])
}

/// A keypad whose host has sent one frame at t = 0.
fn hosted() -> Keypad {
    let mut k = Keypad::new();
    k.set_dtr(true);
    k.receive(solid_red(), 0);
    k
}

fn key(pos: Position, edge: Edge) -> DeviceMessage {
    DeviceMessage::Key { pos, edge }
}

/// Holds `pressed` from `from` until `to` (inclusive) in 1 ms steps and
/// returns every message produced.
fn hold(k: &mut Keypad, pressed: [bool; 3], from: u64, to: u64) -> Vec<DeviceMessage> {
    (from..=to).flat_map(|t| k.scan(pressed, t)).flatten().collect()
}

mod host_presence {
    use super::*;

    #[test]
    fn a_new_keypad_has_no_host_and_shows_dim_white() {
        assert_eq!(Keypad::new().pixels(0), [NO_HOST; 3]);
    }

    #[test]
    fn the_first_frame_with_dtr_high_sends_ready() {
        let mut k = Keypad::new();
        k.set_dtr(true);
        assert_eq!(k.receive(solid_red(), 0), Some(READY));
        assert_eq!(k.pixels(0), [RED_OUT; 3]);
    }

    #[test]
    fn later_frames_do_not_send_ready_again() {
        let mut k = hosted();
        assert_eq!(k.receive(solid_red(), 1000), None);
    }

    #[test]
    fn a_frame_while_dtr_is_low_does_not_end_no_host() {
        let mut k = Keypad::new();
        assert_eq!(k.receive(solid_red(), 0), None);
        assert_eq!(k.pixels(0), [NO_HOST; 3]);
    }

    #[test]
    fn a_flash_does_not_end_no_host() {
        let mut k = Keypad::new();
        k.set_dtr(true);
        assert_eq!(k.receive(HostMessage::Flash { pos: Left, rgb: RED }, 0), None);
        assert_eq!(k.pixels(0), [NO_HOST; 3]);
    }

    #[test]
    fn three_seconds_without_a_frame_is_no_host() {
        let k = hosted();
        assert_eq!(k.pixels(2999), [RED_OUT; 3]);
        assert_eq!(k.pixels(3000), [NO_HOST; 3]);
    }

    #[test]
    fn a_frame_resets_the_three_seconds() {
        let mut k = hosted();
        k.receive(solid_red(), 2000);
        assert_eq!(k.pixels(4999), [RED_OUT; 3]);
    }

    #[test]
    fn dtr_going_low_is_no_host_at_once() {
        let mut k = hosted();
        k.set_dtr(false);
        assert_eq!(k.pixels(1), [NO_HOST; 3]);
    }

    #[test]
    fn dtr_coming_back_waits_for_a_new_frame_and_sends_ready() {
        let mut k = hosted();
        k.set_dtr(false);
        k.set_dtr(true);
        assert_eq!(k.pixels(2), [NO_HOST; 3]);
        assert_eq!(k.receive(solid_red(), 3), Some(READY));
    }

    #[test]
    fn ready_is_sent_again_after_no_host() {
        let mut k = hosted();
        assert_eq!(k.receive(solid_red(), 5000), Some(READY));
    }
}

mod keys {
    use super::*;

    #[test]
    fn a_press_held_for_5_ms_sends_down_once() {
        let mut k = hosted();
        assert_eq!(hold(&mut k, [false, true, false], 10, 14), vec![]);
        assert_eq!(hold(&mut k, [false, true, false], 15, 30), vec![key(Middle, Edge::Down)]);
    }

    #[test]
    fn a_release_held_for_5_ms_sends_up() {
        let mut k = hosted();
        hold(&mut k, [true, false, false], 10, 20);
        assert_eq!(hold(&mut k, [false; 3], 21, 40), vec![key(Left, Edge::Up)]);
    }

    #[test]
    fn bouncing_shorter_than_5_ms_sends_nothing() {
        let mut k = hosted();
        let mut sent = Vec::new();
        for t in 10..40u64 {
            sent.extend(k.scan([false, false, t % 3 == 0], t).into_iter().flatten());
        }
        assert_eq!(sent, vec![]);
    }

    #[test]
    fn keys_are_independent() {
        let mut k = hosted();
        assert_eq!(
            hold(&mut k, [true, false, true], 10, 20),
            vec![key(Left, Edge::Down), key(Right, Edge::Down)]
        );
    }

    #[test]
    fn presses_without_a_host_are_dropped_not_queued() {
        let mut k = Keypad::new();
        k.set_dtr(true);
        assert_eq!(hold(&mut k, [true, false, false], 10, 20), vec![]);
        k.receive(solid_red(), 21);
        assert_eq!(hold(&mut k, [true, false, false], 22, 40), vec![]);
        assert_eq!(hold(&mut k, [false; 3], 41, 60), vec![key(Left, Edge::Up)]);
    }
}

mod rendering {
    use super::*;

    fn showing(leds: [(Rgb, Mode); 3]) -> Keypad {
        let mut k = Keypad::new();
        k.set_dtr(true);
        k.receive(frame(leds), 0);
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
        k.receive(frame([(RED, Mode::Breathe), (RED, Mode::Off), (RED, Mode::Off)]), 2000);
        assert_eq!(red_at(&k, 2000), 6);
        let rising: Vec<u8> = (0..=1000).step_by(100).map(|t| red_at(&k, 2000 + t)).collect();
        assert!(rising.windows(2).all(|w| w[0] <= w[1]), "{rising:?}");
        let falling: Vec<u8> = (1000..=2000).step_by(100).map(|t| red_at(&k, 2000 + t)).collect();
        assert!(falling.windows(2).all(|w| w[0] >= w[1]), "{falling:?}");
    }

    #[test]
    fn a_flash_covers_its_key_for_150_ms() {
        let mut k = showing([(RED, Mode::Solid), (RED, Mode::Solid), (RED, Mode::Solid)]);
        k.receive(HostMessage::Flash { pos: Middle, rgb: WHITE }, 100);
        assert_eq!(k.pixels(100), [RED_OUT, WHITE_OUT, RED_OUT]);
        assert_eq!(k.pixels(249), [RED_OUT, WHITE_OUT, RED_OUT]);
        assert_eq!(k.pixels(250), [RED_OUT; 3]);
    }

    #[test]
    fn a_flash_shows_on_a_dark_key() {
        let mut k = showing([(RED, Mode::Off), (RED, Mode::Off), (RED, Mode::Off)]);
        k.receive(HostMessage::Flash { pos: Right, rgb: RED }, 0);
        assert_eq!(k.pixels(0), [DARK, DARK, RED_OUT]);
    }

    #[test]
    fn led_order_swaps_red_and_green() {
        assert_eq!(led_order(Rgb { r: 1, g: 2, b: 3 }), Rgb { r: 2, g: 1, b: 3 });
    }
}
