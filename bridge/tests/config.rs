//! `config.toml` in the plugin's configuration directory (ADR 0013).

use std::path::{Path, PathBuf};

use bridge::config::{load, parse, path, Config};
use bridge::herdr::socket_path::Platform;
use bridge::layout::Layout;
use bridge::state::Key::*;

mod locating {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn herdrs_plugin_config_dir_wins() {
        let vars = [
            ("HERDR_PLUGIN_CONFIG_DIR", r"C:\cfg\triton"),
            ("APPDATA", r"C:\Roaming"),
        ];
        assert_eq!(
            path(env(&vars), Platform::Windows, Path::new("/tmp")),
            PathBuf::from(r"C:\cfg\triton").join("config.toml")
        );
    }

    #[test]
    fn without_it_the_path_is_built_from_herdrs_config_dir() {
        let vars = [("APPDATA", r"C:\Roaming")];
        assert_eq!(
            path(env(&vars), Platform::Windows, Path::new("/tmp")),
            PathBuf::from(r"C:\Roaming")
                .join("herdr")
                .join("plugins")
                .join("config")
                .join("shortarrow.herdr-triton")
                .join("config.toml")
        );
    }

    #[test]
    fn xdg_config_home_applies_as_it_does_for_herdr() {
        let vars = [("XDG_CONFIG_HOME", "/x"), ("HOME", "/home/u")];
        assert_eq!(
            path(env(&vars), Platform::Unix, Path::new("/tmp")),
            PathBuf::from("/x/herdr/plugins/config/shortarrow.herdr-triton/config.toml")
        );
    }
}

mod parsing {
    use super::*;

    #[test]
    fn layout_names_the_keys_left_to_right() {
        let config = parse(r#"layout = ["select", "jump", "approve"]"#).unwrap();
        assert_eq!(config.layout, Layout::new([Select, Jump, Approve]).unwrap());
    }

    #[test]
    fn an_empty_file_keeps_the_defaults() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert_eq!(parse("colour = \"red\"\n").unwrap(), Config::default());
    }

    #[test]
    fn a_layout_without_each_key_once_is_refused() {
        for text in [
            r#"layout = ["jump", "jump", "select"]"#,
            r#"layout = ["jump", "approve"]"#,
            r#"layout = ["jump", "approve", "select", "jump"]"#,
            r#"layout = ["jump", "approve", "skip"]"#,
            r#"layout = ["Jump", "approve", "select"]"#,
            r#"layout = "jump""#,
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn text_that_is_not_toml_is_refused() {
        assert!(parse("layout = [").is_err());
    }

    #[test]
    fn a_refusal_says_why() {
        let err = parse(r#"layout = ["jump", "jump", "select"]"#).unwrap_err();
        assert!(err.contains("layout"), "{err}");
    }
}

mod loading {
    use super::*;

    fn temp_file(name: &str, text: Option<&str>) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "herdr-triton-config-{}-{name}.toml",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        if let Some(text) = text {
            std::fs::write(&path, text).unwrap();
        }
        path
    }

    #[test]
    fn a_missing_file_gives_the_defaults_quietly() {
        assert_eq!(load(&temp_file("missing", None)), (Config::default(), None));
    }

    #[test]
    fn a_file_is_read() {
        let path = temp_file("read", Some(r#"layout = ["approve", "jump", "select"]"#));
        let (config, warning) = load(&path);
        assert_eq!(config.layout, Layout::new([Approve, Jump, Select]).unwrap());
        assert_eq!(warning, None);
    }

    #[test]
    fn a_bad_file_gives_the_defaults_and_a_warning_naming_it() {
        let path = temp_file("bad", Some("layout = ["));
        let (config, warning) = load(&path);
        assert_eq!(config, Config::default());
        let warning = warning.unwrap();
        assert!(warning.contains(&*path.to_string_lossy()), "{warning}");
    }
}
