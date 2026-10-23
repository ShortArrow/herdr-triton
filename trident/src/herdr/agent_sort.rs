//! The sort mode of herdr's agents pane, which no API reports, read from
//! the files herdr keeps it in (ADR 0018; herdr
//! `src/client/shell/preferences.rs`, `src/config/io.rs`).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::herdr::socket_path::{config_dir, Platform};
use crate::state::AgentSort;

const APP_DIR: &str = "herdr";
const CONFIG_FILE: &str = "config.toml";
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// The sort mode for the herdr whose API socket is `api_socket`: the local
/// client's preferences, then herdr's `config.toml`, then `grouped`.
pub fn read(
    api_socket: &Path,
    env: &impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> AgentSort {
    let state = state_dir(env, platform, temp_dir);
    let preferences = std::fs::read_to_string(preferences_file(&client_socket(api_socket), &state));
    let config = std::fs::read_to_string(config_path(env, platform, temp_dir));
    decide(preferences.ok().as_deref(), config.ok().as_deref())
}

/// The mode the first of `preferences` (the client's JSON) and `config`
/// (herdr's TOML) names; `grouped` if neither names one.
pub fn decide(preferences: Option<&str>, config: Option<&str>) -> AgentSort {
    preferences
        .and_then(from_preferences)
        .or_else(|| config.and_then(from_config))
        .unwrap_or_default()
}

/// The client socket herdr pairs with `api_socket`: its sibling named after
/// the API socket's stem with `-client.sock`.
pub fn client_socket(api_socket: &Path) -> PathBuf {
    let stem = api_socket
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(APP_DIR);
    let parent = api_socket.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!("{stem}-client.sock"))
}

/// The client's preferences for `client_socket`, under herdr's state
/// directory `state_dir`.
pub fn preferences_file(client_socket: &Path, state_dir: &Path) -> PathBuf {
    let hash = client_socket
        .to_string_lossy()
        .bytes()
        .fold(FNV_OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        });
    state_dir
        .join("client-shell")
        .join(format!("local-{hash:016x}.json"))
}

/// herdr's state directory: `$XDG_STATE_HOME/herdr`, otherwise
/// `%LOCALAPPDATA%\herdr` on Windows and `~/.local/state/herdr` elsewhere.
pub fn state_dir(
    env: &impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> PathBuf {
    if let Some(dir) = env("XDG_STATE_HOME") {
        return PathBuf::from(dir).join(APP_DIR);
    }
    let home_state =
        || env("HOME").map(|home| PathBuf::from(home).join(format!(".local/state/{APP_DIR}")));
    let platform_dir = match platform {
        Platform::Windows => env("LOCALAPPDATA")
            .map(|dir| PathBuf::from(dir).join(APP_DIR))
            .or_else(|| {
                env("USERPROFILE").map(|profile| {
                    PathBuf::from(profile)
                        .join("AppData")
                        .join("Local")
                        .join(APP_DIR)
                })
            })
            .or_else(home_state),
        Platform::Unix => home_state(),
    };
    platform_dir.unwrap_or_else(|| temp_dir.join(format!("{APP_DIR}-state")))
}

/// herdr's `config.toml`: `HERDR_CONFIG_PATH`, otherwise in its
/// configuration directory.
pub fn config_path(
    env: &impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> PathBuf {
    match env("HERDR_CONFIG_PATH") {
        Some(path) => PathBuf::from(path),
        None => config_dir(env, platform, temp_dir).join(CONFIG_FILE),
    }
}

fn from_preferences(text: &str) -> Option<AgentSort> {
    #[derive(Deserialize)]
    struct Raw {
        agent_panel_sort: Option<String>,
    }
    let raw: Raw = serde_json::from_str(text).ok()?;
    named(raw.agent_panel_sort?.as_str())
}

fn from_config(text: &str) -> Option<AgentSort> {
    #[derive(Deserialize)]
    struct Raw {
        ui: Option<Ui>,
    }
    #[derive(Deserialize)]
    struct Ui {
        agent_panel_sort: Option<String>,
    }
    let raw: Raw = toml::from_str(text).ok()?;
    named(raw.ui?.agent_panel_sort?.as_str())
}

fn named(value: &str) -> Option<AgentSort> {
    match value {
        "spaces" | "workspaces" => Some(AgentSort::Grouped),
        "priority" => Some(AgentSort::Priority),
        _ => None,
    }
}
