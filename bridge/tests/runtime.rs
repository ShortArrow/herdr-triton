//! The loop that joins herdr, the state core and the keypad, against fakes.

use std::collections::VecDeque;
use std::io;

use bridge::herdr::client::CallError;
use bridge::herdr::wire::{Node, Request, Response};
use bridge::runtime::{Exit, Herdr, Keys, Mode as RunMode, Runtime};
use bridge::state::{palette, Agent, AgentKeys, PromptKeys, Status, Wrap};
use protocol::{Edge, Led, Mode, Position, Rgb};

#[derive(Default)]
struct FakeHerdr {
    version: String,
    agents: Vec<Agent>,
    reachable: bool,
    refuse: bool,
    screen: String,
    workspaces: Vec<(String, bool)>,
    requests: Vec<Request>,
}

impl FakeHerdr {
    fn new(agents: Vec<Agent>) -> Self {
        Self {
            version: "0.9.1".into(),
            agents,
            reachable: true,
            refuse: false,
            screen: String::new(),
            workspaces: Vec::new(),
            requests: Vec::new(),
        }
    }
}

impl Herdr for FakeHerdr {
    fn call(&mut self, request: &Request) -> Result<Response, CallError> {
        self.requests.push(request.clone());
        if !self.reachable {
            return Err(CallError::Io(io::ErrorKind::NotFound.into()));
        }
        let reads = matches!(
            request,
            Request::Ping | Request::AgentList | Request::WorkspaceList | Request::AgentRead { .. }
        );
        if self.refuse && !reads {
            return Ok(Response::Error {
                code: "agent_not_found".into(),
                message: "gone".into(),
            });
        }
        Ok(match request {
            Request::Ping => Response::Pong {
                version: self.version.clone(),
            },
            Request::AgentList => Response::Agents(self.agents.clone()),
            Request::AgentFocus { target } => {
                let a = self
                    .agents
                    .iter()
                    .find(|a| &a.pane_id == target)
                    .cloned()
                    .unwrap();
                Response::Agent(a)
            }
            Request::AgentSendKeys { .. } => Response::Ok,
            Request::AgentRead { .. } => Response::Screen(self.screen.clone()),
            Request::WorkspaceList => Response::Workspaces(
                self.workspaces
                    .iter()
                    .map(|(id, focused)| Node {
                        id: id.clone(),
                        focused: *focused,
                    })
                    .collect(),
            ),
            Request::WorkspaceFocus { workspace_id } => Response::Workspace(Node {
                id: workspace_id.clone(),
                focused: true,
            }),
            other => panic!("unexpected {other:?}"),
        })
    }
}

#[derive(Default)]
struct FakeKeys {
    events: VecDeque<(Position, Edge)>,
    shown: Vec<[Led; 3]>,
    flashes: Vec<(Position, Rgb)>,
    broken: bool,
}

impl Keys for FakeKeys {
    fn next_key(&mut self) -> io::Result<Option<(Position, Edge)>> {
        if self.broken {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        Ok(self.events.pop_front())
    }
    fn show(&mut self, leds: [Led; 3]) -> io::Result<()> {
        self.shown.push(leds);
        Ok(())
    }
    fn flash(&mut self, pos: Position, rgb: Rgb) -> io::Result<()> {
        self.flashes.push((pos, rgb));
        Ok(())
    }
}

fn blocked(pane: &str, focused: bool) -> Agent {
    Agent {
        pane_id: pane.into(),
        agent: Some("claude".into()),
        status: Status::Blocked,
        focused,
        state_change_seq: 1,
    }
}

fn keys() -> AgentKeys {
    let keys = |wrap| PromptKeys {
        confirm: vec!["enter".into()],
        select: vec!["down".into()],
        wrap,
    };
    [
        ("claude".to_string(), keys(Wrap::Native)),
        (
            "stopper".to_string(),
            keys(Wrap::ByScreen {
                back: vec!["up".into()],
            }),
        ),
    ]
    .into()
}

fn started(agents: Vec<Agent>, mode: RunMode) -> Runtime<FakeHerdr, FakeKeys> {
    let mut rt = Runtime::new(FakeHerdr::new(agents), FakeKeys::default(), keys(), mode);
    rt.start(0).unwrap();
    rt
}

fn polls(rt: &Runtime<FakeHerdr, FakeKeys>) -> usize {
    rt.herdr()
        .requests
        .iter()
        .filter(|r| **r == Request::AgentList)
        .count()
}

mod starting {
    use super::*;

    #[test]
    fn pings_then_polls() {
        let rt = started(vec![], RunMode::Run);
        assert_eq!(rt.herdr().requests, vec![Request::Ping, Request::AgentList]);
    }

    #[test]
    fn an_unsupported_herdr_shows_solid_red() {
        let mut herdr = FakeHerdr::new(vec![]);
        herdr.version = "0.9.0".into();
        let mut rt = Runtime::new(herdr, FakeKeys::default(), keys(), RunMode::Run);
        rt.start(0).unwrap();
        rt.tick(0).unwrap();
        assert_eq!(
            rt.keys().shown.last(),
            Some(
                &[Led {
                    rgb: palette::RED,
                    mode: Mode::Solid
                }; 3]
            )
        );
    }
}

mod polling {
    use super::*;

    #[test]
    fn agent_list_is_polled_every_250_ms() {
        let mut rt = started(vec![], RunMode::Run);
        for t in (10..=740).step_by(10) {
            rt.tick(t).unwrap();
        }
        assert_eq!(polls(&rt), 3, "at 0, 250 and 500");
    }
}

mod keys_to_herdr {
    use super::*;

    #[test]
    fn jump_focuses_then_flashes_and_polls() {
        let mut rt = started(vec![blocked("a", false)], RunMode::Run);
        rt.keys_mut().events.push_back((Position::Left, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(
            rt.herdr().requests[2..],
            [
                Request::AgentFocus { target: "a".into() },
                Request::AgentList
            ]
        );
        assert_eq!(rt.keys().flashes, vec![(Position::Left, palette::WHITE)]);
    }

    #[test]
    fn a_refused_request_flashes_red() {
        let mut rt = started(vec![blocked("a", false)], RunMode::Run);
        rt.herdr_mut().refuse = true;
        rt.keys_mut().events.push_back((Position::Left, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(rt.keys().flashes, vec![(Position::Left, palette::RED)]);
    }

    #[test]
    fn approve_polls_then_sends_the_confirm_keys() {
        let mut rt = started(vec![blocked("a", true)], RunMode::Run);
        rt.keys_mut()
            .events
            .push_back((Position::Middle, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(
            rt.herdr().requests[2..],
            [
                Request::AgentList,
                Request::AgentSendKeys {
                    target: "a".into(),
                    keys: vec!["enter".into()]
                },
                Request::AgentList,
            ]
        );
    }

    #[test]
    fn select_on_a_non_wrapping_agent_reads_the_screen_then_goes_back_from_the_last() {
        let mut stopper = blocked("a", true);
        stopper.agent = Some("stopper".into());
        let mut rt = started(vec![stopper], RunMode::Run);
        rt.herdr_mut().screen = "   1. Yes\n   2. Always\n ❯ 3. No\n".into();
        rt.keys_mut()
            .events
            .push_back((Position::Right, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(
            rt.herdr().requests[2..],
            [
                Request::AgentList,
                Request::AgentRead { target: "a".into() },
                Request::AgentSendKeys {
                    target: "a".into(),
                    keys: vec!["up".into(), "up".into()]
                },
                Request::AgentList,
            ]
        );
    }

    #[test]
    fn jump_with_nothing_waiting_or_done_moves_to_the_next_workspace() {
        let mut rt = started(vec![], RunMode::Run);
        rt.herdr_mut().workspaces = vec![("w1".into(), true), ("w2".into(), false)];
        rt.keys_mut().events.push_back((Position::Left, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(
            rt.herdr().requests[2..],
            [
                Request::WorkspaceList,
                Request::WorkspaceFocus {
                    workspace_id: "w2".into()
                },
                Request::AgentList,
            ]
        );
        assert_eq!(rt.keys().flashes, vec![(Position::Left, palette::WHITE)]);
    }

    #[test]
    fn a_refused_workspace_focus_flashes_red() {
        let mut rt = started(vec![], RunMode::Run);
        rt.herdr_mut().workspaces = vec![("w1".into(), true), ("w2".into(), false)];
        rt.herdr_mut().refuse = true;
        rt.keys_mut().events.push_back((Position::Left, Edge::Down));
        rt.tick(10).unwrap();
        assert_eq!(rt.keys().flashes, vec![(Position::Left, palette::RED)]);
    }

    #[test]
    fn releases_are_ignored() {
        let mut rt = started(vec![blocked("a", false)], RunMode::Run);
        rt.keys_mut().events.push_back((Position::Left, Edge::Up));
        rt.tick(10).unwrap();
        assert_eq!(rt.herdr().requests.len(), 2);
    }

    #[test]
    fn every_queued_event_is_handled_in_one_tick() {
        let mut rt = started(vec![blocked("a", false)], RunMode::Run);
        rt.keys_mut().events.extend([
            (Position::Left, Edge::Down),
            (Position::Left, Edge::Up),
            (Position::Left, Edge::Down),
        ]);
        rt.tick(10).unwrap();
        let focuses = rt
            .herdr()
            .requests
            .iter()
            .filter(|r| matches!(r, Request::AgentFocus { .. }))
            .count();
        assert_eq!(focuses, 2);
    }
}

mod leds {
    use super::*;

    #[test]
    fn a_frame_is_sent_on_change_and_repeated_every_second() {
        let mut rt = started(vec![], RunMode::Run);
        for t in (0..=990).step_by(10) {
            rt.tick(t).unwrap();
        }
        assert_eq!(rt.keys().shown.len(), 1);
        rt.tick(1000).unwrap();
        assert_eq!(rt.keys().shown.len(), 2, "resent a second after the first");
        rt.herdr_mut().agents = vec![blocked("a", false)];
        for t in (1010..=1250).step_by(10) {
            rt.tick(t).unwrap();
        }
        assert_eq!(
            rt.keys().shown.len(),
            3,
            "sent again when the 1250 poll changed it"
        );
    }
}

mod exiting {
    use super::*;

    #[test]
    fn a_hook_exits_after_five_idle_seconds() {
        let mut rt = started(vec![], RunMode::Hook);
        assert!(rt.tick(4990).is_ok());
        assert_eq!(rt.tick(5000).unwrap_err(), Exit::Idle);
    }

    #[test]
    fn a_done_agent_keeps_a_hook_listening() {
        let mut done = blocked("d", false);
        done.status = Status::Done;
        let mut rt = started(vec![done], RunMode::Hook);
        for t in (250..=10_000).step_by(250) {
            rt.tick(t).unwrap();
        }
    }

    #[test]
    fn a_waiting_agent_restarts_the_idle_clock() {
        let mut rt = started(vec![], RunMode::Hook);
        rt.tick(3000).unwrap();
        rt.herdr_mut().agents = vec![blocked("a", false)];
        rt.tick(3250).unwrap();
        rt.herdr_mut().agents = vec![];
        rt.tick(3500).unwrap();
        assert!(rt.tick(8490).is_ok());
        assert_eq!(rt.tick(8500).unwrap_err(), Exit::Idle);
    }

    #[test]
    fn a_hook_exits_after_five_seconds_without_herdr() {
        let mut rt = started(vec![blocked("a", false)], RunMode::Hook);
        rt.herdr_mut().reachable = false;
        rt.tick(250).unwrap();
        assert!(rt.tick(5240).is_ok());
        assert_eq!(rt.tick(5250).unwrap_err(), Exit::HerdrGone);
    }

    #[test]
    fn run_mode_never_exits_for_idleness_or_a_missing_herdr() {
        let mut rt = started(vec![], RunMode::Run);
        rt.herdr_mut().reachable = false;
        for t in (0..=20_000).step_by(250) {
            rt.tick(t).unwrap();
        }
    }

    #[test]
    fn a_device_error_ends_the_loop() {
        let mut rt = started(vec![], RunMode::Run);
        rt.keys_mut().broken = true;
        assert_eq!(rt.tick(10).unwrap_err(), Exit::DeviceLost);
    }
}
