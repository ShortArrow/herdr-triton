//! Which key sits at which position (ADR 0013).

use bridge::layout::Layout;
use bridge::state::Key::{self, *};
use protocol::{Led, Mode, Position, Rgb};

fn led(r: u8) -> Led {
    Led {
        rgb: Rgb { r, g: 0, b: 0 },
        mode: Mode::Solid,
    }
}

#[test]
fn the_default_is_jump_approve_select() {
    assert_eq!(
        Layout::new([Jump, Approve, Select]),
        Some(Layout::default())
    );
}

#[test]
fn a_layout_holds_each_key_once() {
    assert!(Layout::new([Select, Jump, Approve]).is_some());
    assert_eq!(Layout::new([Jump, Jump, Select]), None);
    assert_eq!(Layout::new([Select, Select, Select]), None);
}

mod swapped {
    use super::*;

    fn layout() -> Layout {
        Layout::new([Select, Jump, Approve]).unwrap()
    }

    #[test]
    fn a_position_names_the_key_placed_there() {
        let keys: Vec<Key> = [Position::Left, Position::Middle, Position::Right]
            .into_iter()
            .map(|p| layout().key_at(p))
            .collect();
        assert_eq!(keys, [Select, Jump, Approve]);
    }

    #[test]
    fn a_key_names_its_position() {
        assert_eq!(layout().position_of(Jump), Position::Middle);
        assert_eq!(layout().position_of(Approve), Position::Right);
        assert_eq!(layout().position_of(Select), Position::Left);
    }

    #[test]
    fn each_keys_led_moves_to_its_position() {
        let (jump, approve, select) = (led(1), led(2), led(3));
        assert_eq!(
            layout().arrange([jump, approve, select]),
            [select, jump, approve]
        );
    }
}
