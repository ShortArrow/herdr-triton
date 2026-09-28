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
