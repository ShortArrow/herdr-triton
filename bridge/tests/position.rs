mod select_json {
    use bridge::position::select_json;
    use bridge::prompt_screen::parse;
    use serde_json::json;

    #[test]
    fn names_workspace_pane_and_highlighted_option() {
        let prompt = parse(" ❯ 1. Yes\n   2. No\n");
        assert_eq!(
            select_json("w1:p2", prompt.as_ref()),
            json!({"w": "1", "p": "2", "s": 1, "options": ["Yes", "No"]})
        );
    }

    #[test]
    fn keeps_non_numeric_ids_as_herdr_prints_them() {
        let prompt = parse("   1. Yes\n › 2. No\n");
        assert_eq!(
            select_json("wY:p1", prompt.as_ref()),
            json!({"w": "Y", "p": "1", "s": 2, "options": ["Yes", "No"]})
        );
    }

    #[test]
    fn without_a_prompt_the_selection_is_null() {
        assert_eq!(
            select_json("w1:p1", None),
            json!({"w": "1", "p": "1", "s": null, "options": []})
        );
    }
}

mod select_id {
    use bridge::position::select_id;
    use bridge::prompt_screen::parse;

    #[test]
    fn is_the_highlighted_option_number() {
        assert_eq!(
            select_id(parse("   1. Yes\n ❯ 2. No\n").as_ref()),
            Some("2".into())
        );
    }

    #[test]
    fn is_none_without_a_highlight_or_a_prompt() {
        assert_eq!(select_id(parse("  1. a\n  2. b\n").as_ref()), None);
        assert_eq!(select_id(None), None);
    }
}

mod pane {
    use bridge::herdr::wire::Node;
    use bridge::position::{pane_id, pane_json, NotActive};
    use serde_json::json;

    fn panes() -> Vec<Node> {
        vec![
            Node {
                id: "w1:p1".into(),
                focused: false,
            },
            Node {
                id: "w1:p2".into(),
                focused: true,
            },
        ]
    }

    #[test]
    fn id_is_the_focused_pane_without_its_prefixes() {
        assert_eq!(pane_id(&panes()), Ok("2".into()));
    }

    #[test]
    fn without_a_focused_pane_the_workspace_is_not_active() {
        let mut unfocused = panes();
        unfocused[1].focused = false;
        assert_eq!(pane_id(&unfocused), Err(NotActive));
    }

    #[test]
    fn not_active_names_the_workspace_in_its_message() {
        assert_eq!(NotActive.message("w1"), "w1 is not the active workspace");
    }

    #[test]
    fn json_names_the_workspace_the_focused_pane_and_every_pane() {
        assert_eq!(
            pane_json("w1", &panes()),
            json!({"w": "1", "p": "2", "panes": ["1", "2"]})
        );
    }
}

mod workspace {
    use bridge::herdr::wire::Node;
    use bridge::position::{workspace_id, workspace_json};
    use serde_json::json;

    fn workspaces() -> Vec<Node> {
        vec![
            Node {
                id: "w1".into(),
                focused: false,
            },
            Node {
                id: "wY".into(),
                focused: true,
            },
        ]
    }

    #[test]
    fn id_is_the_focused_workspace_without_its_prefix() {
        assert_eq!(workspace_id(&workspaces()), Some("Y".into()));
    }

    #[test]
    fn json_names_the_session_the_focused_workspace_and_every_workspace() {
        assert_eq!(
            workspace_json("triton", &workspaces()),
            json!({"session": "triton", "w": "Y", "workspaces": ["1", "Y"]})
        );
    }

    #[test]
    fn json_has_a_null_workspace_when_none_is_focused() {
        let none = vec![Node {
            id: "w1".into(),
            focused: false,
        }];
        assert_eq!(
            workspace_json("default", &none),
            json!({"session": "default", "w": null, "workspaces": ["1"]})
        );
    }
}
