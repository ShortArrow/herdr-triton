use std::path::{Path, PathBuf};

use bridge::herdr::socket_path::{resolve, Platform};

const TEMP: &str = "tmp";

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let vars: Vec<(String, String)> =
        vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |key| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn path(parts: &[&str]) -> PathBuf {
    parts.iter().collect()
}

fn resolve_with(vars: &[(&str, &str)], platform: Platform) -> PathBuf {
    resolve(env(vars), platform, Path::new(TEMP))
}

#[test]
fn herdr_socket_path_wins_over_everything() {
    let got = resolve_with(
        &[("HERDR_SOCKET_PATH", "custom.sock"), ("HERDR_SESSION", "work"), ("APPDATA", "ad")],
        Platform::Windows,
    );
    assert_eq!(got, PathBuf::from("custom.sock"));
}

#[test]
fn a_named_session_lives_under_sessions() {
    let got = resolve_with(&[("HERDR_SESSION", "work"), ("APPDATA", "ad")], Platform::Windows);
    assert_eq!(got, path(&["ad", "herdr", "sessions", "work", "herdr.sock"]));
}

#[test]
fn the_session_named_default_is_the_default_session() {
    let got = resolve_with(&[("HERDR_SESSION", "default"), ("APPDATA", "ad")], Platform::Windows);
    assert_eq!(got, path(&["ad", "herdr", "herdr.sock"]));
}

#[test]
fn an_invalid_session_name_falls_back_to_the_default_session() {
    for name in ["", ".", "..", "a/b", "a b"] {
        let got = resolve_with(&[("HERDR_SESSION", name), ("APPDATA", "ad")], Platform::Windows);
        assert_eq!(got, path(&["ad", "herdr", "herdr.sock"]), "session {name:?}");
    }
}

#[test]
fn xdg_config_home_wins_on_both_platforms() {
    for platform in [Platform::Windows, Platform::Unix] {
        let got = resolve_with(&[("XDG_CONFIG_HOME", "xdg"), ("APPDATA", "ad"), ("HOME", "h")], platform);
        assert_eq!(got, path(&["xdg", "herdr", "herdr.sock"]), "{platform:?}");
    }
}

#[test]
fn windows_uses_appdata() {
    let got = resolve_with(&[("APPDATA", "ad"), ("USERPROFILE", "up")], Platform::Windows);
    assert_eq!(got, path(&["ad", "herdr", "herdr.sock"]));
}

#[test]
fn windows_without_appdata_uses_the_roaming_profile() {
    let got = resolve_with(&[("USERPROFILE", "up"), ("HOME", "h")], Platform::Windows);
    assert_eq!(got, path(&["up", "AppData", "Roaming", "herdr", "herdr.sock"]));
}

#[test]
fn windows_without_a_profile_uses_home() {
    let got = resolve_with(&[("HOME", "h")], Platform::Windows);
    assert_eq!(got, path(&["h", ".config/herdr", "herdr.sock"]));
}

#[test]
fn unix_uses_home() {
    let got = resolve_with(&[("HOME", "h"), ("APPDATA", "ad")], Platform::Unix);
    assert_eq!(got, path(&["h", ".config/herdr", "herdr.sock"]));
}

#[test]
fn without_any_base_the_temp_dir_is_used() {
    for platform in [Platform::Windows, Platform::Unix] {
        assert_eq!(resolve_with(&[], platform), path(&[TEMP, "herdr", "herdr.sock"]), "{platform:?}");
    }
}
