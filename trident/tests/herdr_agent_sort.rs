//! Where herdr keeps its agents pane's sort mode, and reading it (ADR 0018).

use std::path::{Path, PathBuf};

use trident::herdr::agent_sort::{
    client_socket, config_path, decide, preferences_file, read, state_dir,
};
use trident::herdr::socket_path::Platform;
use trident::state::AgentSort;

const TEMP: &str = "tmp";

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let vars: Vec<(String, String)> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |key| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

mod locating {
    use super::*;

    #[test]
    fn the_client_socket_sits_next_to_the_api_socket() {
        assert_eq!(
            client_socket(Path::new("/run/herdr/herdr.sock")),
            Path::new("/run/herdr").join("herdr-client.sock")
        );
    }

    #[test]
    fn the_client_socket_keeps_a_custom_stem() {
        assert_eq!(
            client_socket(Path::new("/run/work.sock")),
            Path::new("/run").join("work-client.sock")
        );
    }

    #[test]
    fn the_preferences_file_is_named_after_the_client_sockets_hash() {
        assert_eq!(
            preferences_file(Path::new("/run/herdr/herdr-client.sock"), Path::new("st")),
            PathBuf::from("st")
                .join("client-shell")
                .join("local-4ceb3d6c21443f21.json")
        );
    }

    #[test]
    fn xdg_state_home_wins() {
        let vars = [("XDG_STATE_HOME", "xs"), ("LOCALAPPDATA", "la")];
        assert_eq!(
            state_dir(&env(&vars), Platform::Windows, Path::new(TEMP)),
            PathBuf::from("xs").join("herdr")
        );
    }

    #[test]
    fn windows_uses_local_app_data() {
        let vars = [("LOCALAPPDATA", "la"), ("USERPROFILE", "up")];
        assert_eq!(
            state_dir(&env(&vars), Platform::Windows, Path::new(TEMP)),
            PathBuf::from("la").join("herdr")
        );
    }

    #[test]
    fn windows_falls_back_to_the_profile() {
        let vars = [("USERPROFILE", "up")];
        assert_eq!(
            state_dir(&env(&vars), Platform::Windows, Path::new(TEMP)),
            PathBuf::from("up")
                .join("AppData")
                .join("Local")
                .join("herdr")
        );
    }

    #[test]
    fn unix_uses_local_state_under_home() {
        let vars = [("HOME", "/home/u")];
        assert_eq!(
            state_dir(&env(&vars), Platform::Unix, Path::new(TEMP)),
            PathBuf::from("/home/u").join(".local/state/herdr")
        );
    }

    #[test]
    fn without_any_the_temp_dir_is_used() {
        assert_eq!(
            state_dir(&env(&[]), Platform::Unix, Path::new(TEMP)),
            PathBuf::from(TEMP).join("herdr-state")
        );
    }

    #[test]
    fn herdr_config_path_wins_for_the_config() {
        let vars = [("HERDR_CONFIG_PATH", "c.toml"), ("APPDATA", "ad")];
        assert_eq!(
            config_path(&env(&vars), Platform::Windows, Path::new(TEMP)),
            PathBuf::from("c.toml")
        );
    }

    #[test]
    fn otherwise_the_config_is_in_herdrs_config_dir() {
        let vars = [("APPDATA", "ad")];
        assert_eq!(
            config_path(&env(&vars), Platform::Windows, Path::new(TEMP)),
            PathBuf::from("ad").join("herdr").join("config.toml")
        );
    }
}

mod deciding {
    use super::*;

    const PRIORITY_PREFS: &str = r#"{ "sidebar_width": 36, "agent_panel_sort": "priority" }"#;
    const SPACES_CONFIG: &str = "[ui]\nagent_panel_sort = \"spaces\"\n";
    const PRIORITY_CONFIG: &str = "[ui]\nagent_panel_sort = \"priority\"\n";

    #[test]
    fn with_nothing_it_is_grouped() {
        assert_eq!(decide(None, None), AgentSort::Grouped);
    }

    #[test]
    fn the_preferences_win_over_the_config() {
        assert_eq!(
            decide(Some(PRIORITY_PREFS), Some(SPACES_CONFIG)),
            AgentSort::Priority
        );
        assert_eq!(
            decide(
                Some(r#"{"agent_panel_sort":"spaces"}"#),
                Some(PRIORITY_CONFIG)
            ),
            AgentSort::Grouped
        );
    }

    #[test]
    fn preferences_without_the_field_leave_it_to_the_config() {
        assert_eq!(
            decide(Some(r#"{"sidebar_width":36}"#), Some(PRIORITY_CONFIG)),
            AgentSort::Priority
        );
    }

    #[test]
    fn workspaces_is_another_name_for_grouped() {
        assert_eq!(
            decide(None, Some("[ui]\nagent_panel_sort = \"workspaces\"\n")),
            AgentSort::Grouped
        );
    }

    #[test]
    fn a_file_that_does_not_parse_or_holds_another_value_says_nothing() {
        for prefs in [
            "{",
            r#"{"agent_panel_sort":"newest"}"#,
            r#"{"agent_panel_sort":3}"#,
        ] {
            assert_eq!(
                decide(Some(prefs), Some(PRIORITY_CONFIG)),
                AgentSort::Priority,
                "{prefs}"
            );
        }
        for config in [
            "[ui",
            "[ui]\nagent_panel_sort = \"Priority\"\n",
            "agent_panel_sort = \"priority\"\n",
        ] {
            assert_eq!(decide(None, Some(config)), AgentSort::Grouped, "{config}");
        }
    }
}

mod reading {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "herdr-triton-agent-sort-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn reads_the_preferences_of_the_sockets_client() {
        let root = temp_root("prefs");
        let api = root.join("herdr.sock");
        let state = root.join("state");
        let prefs = preferences_file(&client_socket(&api), &state.join("herdr"));
        std::fs::create_dir_all(prefs.parent().unwrap()).unwrap();
        std::fs::write(&prefs, r#"{"agent_panel_sort":"priority"}"#).unwrap();
        let state = state.to_string_lossy().into_owned();
        let config = root.join("none.toml").to_string_lossy().into_owned();
        let vars = [
            ("XDG_STATE_HOME", state.as_str()),
            ("HERDR_CONFIG_PATH", config.as_str()),
        ];
        assert_eq!(
            read(&api, &env(&vars), Platform::current(), Path::new(TEMP)),
            AgentSort::Priority
        );
    }

    #[test]
    fn without_preferences_reads_herdrs_config() {
        let root = temp_root("config");
        let config = root.join("config.toml");
        std::fs::write(&config, "[ui]\nagent_panel_sort = \"priority\"\n").unwrap();
        let state = root.join("state").to_string_lossy().into_owned();
        let config = config.to_string_lossy().into_owned();
        let vars = [
            ("XDG_STATE_HOME", state.as_str()),
            ("HERDR_CONFIG_PATH", config.as_str()),
        ];
        assert_eq!(
            read(
                &root.join("herdr.sock"),
                &env(&vars),
                Platform::current(),
                Path::new(TEMP)
            ),
            AgentSort::Priority
        );
    }
}
