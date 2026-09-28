use bridge::state::{palette::*, Agent, AgentKeys, Cmd, Msg, PromptKeys, State, Status};
use protocol::{Led, Mode, Position, Position::*};

fn agent(pane: &str, status: Status, seq: u64) -> Agent {
    Agent {
        pane_id: pane.into(),
        agent: Some("claude".into()),
        status,
        focused: false,
        state_change_seq: seq,
    }
}

fn blocked(pane: &str) -> Agent {
    agent(pane, Status::Blocked, 1)
}

fn focused(mut a: Agent) -> Agent {
    a.focused = true;
    a
}

fn prompt_keys(confirm: &str, select: &str) -> PromptKeys {
    PromptKeys { confirm: vec![confirm.into()], select: vec![select.into()] }
}

fn keys() -> AgentKeys {
    [
        ("claude".to_string(), prompt_keys("enter", "down")),
        ("custom".to_string(), prompt_keys("space", "tab")),
    ]
    .into()
}

fn with(snapshot: Vec<Agent>) -> State {
    let mut s = State::new(keys());
    s.update(Msg::Snapshot(snapshot));
    s
}

fn focus(pane: &str) -> Vec<Cmd> {
    vec![Cmd::Focus { pane_id: pane.into() }]
}

fn error(pos: Position) -> Vec<Cmd> {
    vec![Cmd::Flash { pos, rgb: RED }]
}

fn led(rgb: protocol::Rgb, mode: Mode) -> Led {
    Led { rgb, mode }
}

const DARK: Led = Led { rgb: OFF, mode: Mode::Off };

/// Presses `pos` and answers its refresh with `snapshot`.
fn press_after_refresh(s: &mut State, pos: Position, snapshot: Vec<Agent>) -> Vec<Cmd> {
    assert_eq!(s.update(Msg::KeyDown(pos)), vec![Cmd::Poll]);
    s.update(Msg::Snapshot(snapshot))
}

fn approve(s: &mut State, snapshot: Vec<Agent>) -> Vec<Cmd> {
    press_after_refresh(s, Middle, snapshot)
}

fn select(s: &mut State, snapshot: Vec<Agent>) -> Vec<Cmd> {
    press_after_refresh(s, Right, snapshot)
}

fn send(pane: &str, key: &str) -> Vec<Cmd> {
    vec![Cmd::SendKeys { pane_id: pane.into(), keys: vec![key.into()] }]
}

fn send_enter(pane: &str) -> Vec<Cmd> {
    send(pane, "enter")
}

mod snapshot {
    use super::*;

    #[test]
    fn blocked_agents_queue_in_list_order() {
        let mut s = with(vec![blocked("b"), agent("x", Status::Working, 1), blocked("a")]);
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("b"));
    }

    #[test]
    fn a_newly_blocked_agent_joins_the_tail() {
        let mut s = with(vec![blocked("b")]);
        s.update(Msg::Snapshot(vec![blocked("a"), blocked("b")]));
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("b"));
    }

    #[test]
    fn an_agent_no_longer_blocked_leaves_the_queue() {
        let mut s = with(vec![blocked("a"), blocked("b")]);
        s.update(Msg::Snapshot(vec![agent("a", Status::Working, 2), blocked("b")]));
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("b"));
    }

    #[test]
    fn an_agent_missing_from_the_snapshot_leaves_the_queue() {
        let mut s = with(vec![blocked("a"), blocked("b")]);
        s.update(Msg::Snapshot(vec![blocked("b")]));
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("b"));
    }

    #[test]
    fn a_blocked_agent_whose_seq_changed_keeps_its_position() {
        let mut s = with(vec![blocked("a"), blocked("b")]);
        s.update(Msg::Snapshot(vec![blocked("b"), agent("a", Status::Blocked, 9)]));
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("a"));
    }

    #[test]
    fn an_agent_blocked_across_snapshots_is_queued_once() {
        let mut s = with(vec![blocked("a")]);
        s.update(Msg::Snapshot(vec![blocked("a")]));
        assert_eq!(s.frame()[0], led(AMBER, Mode::Breathe));
    }

    #[test]
    fn a_snapshot_with_no_pending_action_emits_nothing() {
        let mut s = with(vec![]);
        assert_eq!(s.update(Msg::Snapshot(vec![blocked("a")])), vec![]);
    }

    #[test]
    fn a_failed_snapshot_disconnects_and_clears_the_queue() {
        let mut s = with(vec![blocked("a")]);
        s.update(Msg::SnapshotFailed);
        assert_eq!(s.update(Msg::KeyDown(Left)), error(Left));
        s.update(Msg::Snapshot(vec![]));
        assert_eq!(s.update(Msg::KeyDown(Left)), error(Left));
    }
}

mod jump {
    use super::*;

    #[test]
    fn with_focus_outside_the_queue_focuses_the_head() {
        let mut s = with(vec![blocked("a"), blocked("b"), focused(agent("x", Status::Idle, 1))]);
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("a"));
    }

    #[test]
    fn focuses_the_entry_after_the_focused_one() {
        let mut s = with(vec![blocked("a"), focused(blocked("b")), blocked("c")]);
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("c"));
    }

    #[test]
    fn wraps_from_the_tail_to_the_head() {
        let mut s = with(vec![blocked("a"), blocked("b"), focused(blocked("c"))]);
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("a"));
    }

    #[test]
    fn with_a_single_focused_entry_focuses_it_again() {
        let mut s = with(vec![focused(blocked("a"))]);
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("a"));
    }

    #[test]
    fn does_not_reorder_the_queue() {
        let mut s = with(vec![focused(blocked("a")), blocked("b")]);
        s.update(Msg::KeyDown(Left));
        s.update(Msg::RequestDone { ok: true });
        s.update(Msg::Snapshot(vec![blocked("a"), blocked("b")]));
        assert_eq!(s.update(Msg::KeyDown(Left)), focus("a"));
    }

    #[test]
    fn with_an_empty_queue_flashes_an_error() {
        let mut s = with(vec![agent("a", Status::Idle, 1)]);
        assert_eq!(s.update(Msg::KeyDown(Left)), error(Left));
    }

    #[test]
    fn a_successful_request_flashes_white_and_refreshes() {
        let mut s = with(vec![blocked("a")]);
        s.update(Msg::KeyDown(Left));
        assert_eq!(
            s.update(Msg::RequestDone { ok: true }),
            vec![Cmd::Flash { pos: Left, rgb: WHITE }, Cmd::Poll]
        );
    }

    #[test]
    fn a_failed_request_flashes_red_and_refreshes() {
        let mut s = with(vec![blocked("a")]);
        s.update(Msg::KeyDown(Left));
        assert_eq!(
            s.update(Msg::RequestDone { ok: false }),
            vec![Cmd::Flash { pos: Left, rgb: RED }, Cmd::Poll]
        );
    }
}

mod approve {
    use super::*;

    #[test]
    fn refreshes_then_sends_the_agents_keys_to_the_focused_blocked_pane() {
        let mut s = with(vec![]);
        assert_eq!(approve(&mut s, vec![blocked("a"), focused(blocked("b"))]), send_enter("b"));
    }

    #[test]
    fn uses_the_confirm_keys_configured_for_the_agent() {
        let mut s = with(vec![]);
        let mut custom = focused(blocked("a"));
        custom.agent = Some("custom".into());
        assert_eq!(approve(&mut s, vec![custom]), send("a", "space"));
    }

    #[test]
    fn decides_on_the_refreshed_snapshot_not_the_previous_one() {
        let mut s = with(vec![focused(blocked("a"))]);
        assert_eq!(approve(&mut s, vec![agent("a", Status::Working, 2)]), error(Middle));
    }

    #[test]
    fn with_focus_outside_the_queue_flashes_an_error() {
        let mut s = with(vec![]);
        assert_eq!(
            approve(&mut s, vec![blocked("a"), focused(agent("x", Status::Idle, 1))]),
            error(Middle)
        );
    }

    #[test]
    fn for_an_agent_without_configured_keys_flashes_an_error() {
        let mut s = with(vec![]);
        let mut other = focused(blocked("a"));
        other.agent = Some("pi".into());
        assert_eq!(approve(&mut s, vec![other]), error(Middle));
    }

    #[test]
    fn for_an_unnamed_agent_flashes_an_error() {
        let mut s = with(vec![]);
        let mut unnamed = focused(blocked("a"));
        unnamed.agent = None;
        assert_eq!(approve(&mut s, vec![unnamed]), error(Middle));
    }

    #[test]
    fn twice_on_the_same_prompt_sends_once() {
        let mut s = with(vec![]);
        assert_eq!(approve(&mut s, vec![focused(blocked("a"))]), send_enter("a"));
        s.update(Msg::RequestDone { ok: true });
        assert_eq!(approve(&mut s, vec![focused(blocked("a"))]), error(Middle));
    }

    #[test]
    fn is_available_again_once_the_seq_changes() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        s.update(Msg::RequestDone { ok: true });
        let again = focused(agent("a", Status::Blocked, 3));
        assert_eq!(approve(&mut s, vec![again]), send_enter("a"));
    }

    #[test]
    fn is_available_again_after_the_pane_leaves_and_reenters_the_queue_with_the_same_seq() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        s.update(Msg::RequestDone { ok: true });
        s.update(Msg::Snapshot(vec![]));
        assert_eq!(approve(&mut s, vec![focused(blocked("a"))]), send_enter("a"));
    }

    #[test]
    fn a_failed_refresh_flashes_an_error() {
        let mut s = with(vec![focused(blocked("a"))]);
        assert_eq!(s.update(Msg::KeyDown(Middle)), vec![Cmd::Poll]);
        assert_eq!(s.update(Msg::SnapshotFailed), error(Middle));
    }

    #[test]
    fn a_successful_send_flashes_white_and_refreshes() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        assert_eq!(
            s.update(Msg::RequestDone { ok: true }),
            vec![Cmd::Flash { pos: Middle, rgb: WHITE }, Cmd::Poll]
        );
    }

    #[test]
    fn a_failed_send_does_not_block_a_retry() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        s.update(Msg::RequestDone { ok: false });
        assert_eq!(approve(&mut s, vec![focused(blocked("a"))]), send_enter("a"));
    }
}

mod select {
    use super::*;

    #[test]
    fn refreshes_then_sends_the_agents_select_keys_to_the_focused_blocked_pane() {
        let mut s = with(vec![]);
        assert_eq!(select(&mut s, vec![blocked("a"), focused(blocked("b"))]), send("b", "down"));
    }

    #[test]
    fn uses_the_select_keys_configured_for_the_agent() {
        let mut s = with(vec![]);
        let mut custom = focused(blocked("a"));
        custom.agent = Some("custom".into());
        assert_eq!(select(&mut s, vec![custom]), send("a", "tab"));
    }

    #[test]
    fn with_focus_outside_the_queue_flashes_an_error() {
        let mut s = with(vec![]);
        assert_eq!(
            select(&mut s, vec![blocked("a"), focused(agent("x", Status::Idle, 1))]),
            error(Right)
        );
    }

    #[test]
    fn for_an_agent_without_configured_keys_flashes_an_error() {
        let mut s = with(vec![]);
        let mut other = focused(blocked("a"));
        other.agent = Some("pi".into());
        assert_eq!(select(&mut s, vec![other]), error(Right));
    }

    #[test]
    fn can_be_repeated_and_leaves_approve_available() {
        let mut s = with(vec![]);
        assert_eq!(select(&mut s, vec![focused(blocked("a"))]), send("a", "down"));
        s.update(Msg::RequestDone { ok: true });
        assert_eq!(select(&mut s, vec![focused(blocked("a"))]), send("a", "down"));
        s.update(Msg::RequestDone { ok: true });
        assert_eq!(approve(&mut s, vec![focused(blocked("a"))]), send_enter("a"));
    }

    #[test]
    fn after_a_confirmation_flashes_an_error_until_the_seq_changes() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        s.update(Msg::RequestDone { ok: true });
        assert_eq!(select(&mut s, vec![focused(blocked("a"))]), error(Right));
    }

    #[test]
    fn a_failed_refresh_flashes_an_error() {
        let mut s = with(vec![focused(blocked("a"))]);
        assert_eq!(s.update(Msg::KeyDown(Right)), vec![Cmd::Poll]);
        assert_eq!(s.update(Msg::SnapshotFailed), error(Right));
    }

    #[test]
    fn a_successful_send_flashes_white_and_refreshes() {
        let mut s = with(vec![]);
        select(&mut s, vec![focused(blocked("a"))]);
        assert_eq!(
            s.update(Msg::RequestDone { ok: true }),
            vec![Cmd::Flash { pos: Right, rgb: WHITE }, Cmd::Poll]
        );
    }
}

mod disconnected {
    use super::*;

    #[test]
    fn every_key_flashes_an_error() {
        let mut s = State::new(keys());
        for pos in [Left, Middle, Right] {
            assert_eq!(s.update(Msg::KeyDown(pos)), error(pos));
        }
    }

    #[test]
    fn incompatible_herdr_rejects_keys_too() {
        let mut s = with(vec![focused(blocked("a"))]);
        s.update(Msg::Incompatible);
        assert_eq!(s.update(Msg::KeyDown(Left)), error(Left));
    }
}

mod frame {
    use super::*;

    #[test]
    fn disconnected_blinks_red_everywhere() {
        let s = State::new(keys());
        assert_eq!(s.frame(), [led(RED, Mode::Blink); 3]);
    }

    #[test]
    fn incompatible_is_solid_red_everywhere() {
        let mut s = with(vec![]);
        s.update(Msg::Incompatible);
        assert_eq!(s.frame(), [led(RED, Mode::Solid); 3]);
    }

    #[test]
    fn an_empty_queue_is_dark() {
        assert_eq!(with(vec![]).frame(), [DARK; 3]);
    }

    #[test]
    fn one_waiting_breathes_amber_on_jump_only() {
        assert_eq!(
            with(vec![blocked("a")]).frame(),
            [led(AMBER, Mode::Breathe), DARK, DARK]
        );
    }

    #[test]
    fn two_waiting_breathe_reddish_amber() {
        assert_eq!(
            with(vec![blocked("a"), blocked("b")]).frame(),
            [led(REDDISH_AMBER, Mode::Breathe), DARK, DARK]
        );
    }

    #[test]
    fn approve_is_green_and_select_blue_when_the_focused_pane_is_approvable() {
        let frame = with(vec![focused(blocked("a"))]).frame();
        assert_eq!(frame[1..], [led(GREEN, Mode::Solid), led(BLUE, Mode::Solid)]);
    }

    #[test]
    fn approve_and_select_are_dark_after_a_confirmation_until_the_seq_changes() {
        let mut s = with(vec![]);
        approve(&mut s, vec![focused(blocked("a"))]);
        s.update(Msg::RequestDone { ok: true });
        assert_eq!(s.frame()[1..], [DARK, DARK]);
    }

    #[test]
    fn approve_and_select_are_dark_for_an_agent_without_configured_keys() {
        let mut other = focused(blocked("a"));
        other.agent = Some("pi".into());
        assert_eq!(with(vec![other]).frame()[1..], [DARK, DARK]);
    }
}
