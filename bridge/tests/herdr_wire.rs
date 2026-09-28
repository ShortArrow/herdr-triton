use bridge::herdr::wire::*;
use bridge::state::{Agent, Status};
use serde_json::{json, Value};

fn encoded(request: &Request) -> Value {
    let line = encode_request("r1", request);
    assert!(line.ends_with('\n') && !line[..line.len() - 1].contains('\n'), "{line:?}");
    serde_json::from_str(&line).unwrap()
}

mod requests {
    use super::*;

    #[test]
    fn ping() {
        assert_eq!(encoded(&Request::Ping), json!({"id": "r1", "method": "ping", "params": {}}));
    }

    #[test]
    fn agent_list() {
        assert_eq!(
            encoded(&Request::AgentList),
            json!({"id": "r1", "method": "agent.list", "params": {}})
        );
    }

    #[test]
    fn agent_focus() {
        assert_eq!(
            encoded(&Request::AgentFocus { target: "w1:p2".into() }),
            json!({"id": "r1", "method": "agent.focus", "params": {"target": "w1:p2"}})
        );
    }

    #[test]
    fn agent_send_keys() {
        assert_eq!(
            encoded(&Request::AgentSendKeys { target: "w1:p2".into(), keys: vec!["down".into()] }),
            json!({"id": "r1", "method": "agent.send_keys", "params": {"target": "w1:p2", "keys": ["down"]}})
        );
    }
}

mod responses {
    use super::*;

    fn decode(v: Value) -> Result<Response, WireError> {
        decode_response(&v.to_string())
    }

    #[test]
    fn pong() {
        let got = decode(json!({"id": "r1", "result": {"type": "pong", "version": "0.9.1", "protocol": 3}}));
        assert_eq!(got, Ok(Response::Pong { version: "0.9.1".into() }));
    }

    #[test]
    fn agent_list_keeps_the_fields_the_bridge_uses_and_ignores_the_rest() {
        let got = decode(json!({"id": "r1", "result": {"type": "agent_list", "agents": [
            {"terminal_id": "t1", "agent": "claude", "agent_status": "blocked", "workspace_id": "w1",
             "tab_id": "w1:t1", "pane_id": "w1:p1", "focused": true, "state_change_seq": 7, "revision": 3},
            {"terminal_id": "t2", "agent_status": "working", "workspace_id": "w1",
             "tab_id": "w1:t1", "pane_id": "w1:p2", "focused": false, "revision": 1}
        ]}}));
        assert_eq!(
            got,
            Ok(Response::Agents(vec![
                Agent {
                    pane_id: "w1:p1".into(),
                    agent: Some("claude".into()),
                    status: Status::Blocked,
                    focused: true,
                    state_change_seq: 7,
                },
                Agent {
                    pane_id: "w1:p2".into(),
                    agent: None,
                    status: Status::Working,
                    focused: false,
                    state_change_seq: 0,
                },
            ]))
        );
    }

    #[test]
    fn every_agent_status() {
        for (wire, status) in [
            ("idle", Status::Idle),
            ("working", Status::Working),
            ("blocked", Status::Blocked),
            ("done", Status::Done),
            ("unknown", Status::Unknown),
        ] {
            let got = decode(json!({"id": "r1", "result": {"type": "agent_list", "agents": [
                {"agent_status": wire, "pane_id": "p", "focused": false}
            ]}}));
            let Ok(Response::Agents(agents)) = got else { panic!("{wire}: {got:?}") };
            assert_eq!(agents[0].status, status, "{wire}");
        }
    }

    #[test]
    fn ok() {
        assert_eq!(decode(json!({"id": "r1", "result": {"type": "ok"}})), Ok(Response::Ok));
    }

    #[test]
    fn error() {
        let got = decode(json!({"id": "r1", "error": {"code": "agent_not_found", "message": "no agent w9"}}));
        assert_eq!(
            got,
            Ok(Response::Error { code: "agent_not_found".into(), message: "no agent w9".into() })
        );
    }

    #[test]
    fn an_unexpected_result_type_is_reported_by_name() {
        let got = decode(json!({"id": "r1", "result": {"type": "pane_info", "pane": {}}}));
        assert_eq!(got, Err(WireError::Unexpected("pane_info".into())));
    }

    #[test]
    fn a_line_that_is_not_an_envelope_is_malformed() {
        for line in ["", "not json", "{}", r#"{"id":"r1"}"#, r#"{"id":"r1","result":{}}"#] {
            assert!(matches!(decode_response(line), Err(WireError::Malformed(_))), "{line:?}");
        }
    }
}

mod version {
    use super::*;

    #[test]
    fn at_or_above_the_minimum_is_supported() {
        for v in ["0.9.1", "0.9.2", "0.10.0", "1.0.0"] {
            assert!(is_supported(v), "{v}");
        }
    }

    #[test]
    fn below_the_minimum_is_not_supported() {
        for v in ["0.9.0", "0.8.9", "0.5.11"] {
            assert!(!is_supported(v), "{v}");
        }
    }

    #[test]
    fn an_unparseable_version_is_not_supported() {
        for v in ["", "0.9", "x.y.z", "0.9.1.2"] {
            assert!(!is_supported(v), "{v}");
        }
    }
}
