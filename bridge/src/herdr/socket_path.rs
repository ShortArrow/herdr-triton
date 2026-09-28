//! Resolves herdr's API socket path the way herdr does
//! (herdr `src/session.rs`, `src/config/io.rs`).

use std::path::{Path, PathBuf};

const APP_DIR: &str = "herdr";
const SOCKET_FILE: &str = "herdr.sock";
const DEFAULT_SESSION: &str = "default";
const MAX_SESSION_NAME_LEN: usize = 64;

/// Which platform's config directory rules apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Unix,
}

impl Platform {
    pub const fn current() -> Self {
        if cfg!(windows) {
            Platform::Windows
        } else {
            Platform::Unix
        }
    }
}

/// herdr's API socket path, from `HERDR_SOCKET_PATH`, then `HERDR_SESSION`,
/// then the default session. `env` looks up environment variables and
/// `temp_dir` is the last-resort base, as in herdr.
pub fn resolve(
    env: impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> PathBuf {
    if let Some(path) = env("HERDR_SOCKET_PATH") {
        return PathBuf::from(path);
    }
    let config = config_dir(&env, platform, temp_dir);
    let data = match session_name(&env) {
        Some(name) => config.join("sessions").join(name),
        None => config,
    };
    data.join(SOCKET_FILE)
}

/// The API socket of the session called `name` (`default` for the default
/// session). An explicit session wins over `HERDR_SOCKET_PATH`, as in herdr.
pub fn for_session(
    name: &str,
    env: impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> PathBuf {
    let config = config_dir(&env, platform, temp_dir);
    let data = if name == DEFAULT_SESSION {
        config
    } else {
        config.join("sessions").join(name)
    };
    data.join(SOCKET_FILE)
}

/// A named session other than `default`; an invalid name counts as none.
fn session_name(env: &impl Fn(&str) -> Option<String>) -> Option<String> {
    env("HERDR_SESSION").filter(|name| name != DEFAULT_SESSION && is_valid_session_name(name))
}

fn is_valid_session_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_SESSION_NAME_LEN
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn config_dir(
    env: &impl Fn(&str) -> Option<String>,
    platform: Platform,
    temp_dir: &Path,
) -> PathBuf {
    if let Some(dir) = env("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join(APP_DIR);
    }
    let home_config =
        || env("HOME").map(|home| PathBuf::from(home).join(format!(".config/{APP_DIR}")));
    let platform_dir = match platform {
        Platform::Windows => env("APPDATA")
            .map(|dir| PathBuf::from(dir).join(APP_DIR))
            .or_else(|| {
                env("USERPROFILE").map(|profile| {
                    PathBuf::from(profile)
                        .join("AppData")
                        .join("Roaming")
                        .join(APP_DIR)
                })
            })
            .or_else(home_config),
        Platform::Unix => home_config(),
    };
    platform_dir.unwrap_or_else(|| temp_dir.join(APP_DIR))
}
