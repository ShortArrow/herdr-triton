//! `config.toml` in the plugin's configuration directory (ADR 0013).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::herdr::socket_path::{config_dir, Platform};
use crate::layout::Layout;
use crate::state::Key;

/// The plugin id herdr names the configuration directory after.
const PLUGIN_ID: &str = "shortarrow.herdr-triton";
const FILE: &str = "config.toml";

/// What the bridge reads from `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    pub layout: Layout,
}

/// `config.toml` in `HERDR_PLUGIN_CONFIG_DIR`, or where herdr puts that
/// directory: `<herdr config>/plugins/config/<plugin id>`.
pub fn path(env: impl Fn(&str) -> Option<String>, platform: Platform, temp_dir: &Path) -> PathBuf {
    let dir = match env("HERDR_PLUGIN_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => config_dir(&env, platform, temp_dir)
            .join("plugins")
            .join("config")
            .join(PLUGIN_ID),
    };
    dir.join(FILE)
}

/// Reads `path`: the defaults when it is missing, and the defaults with a
/// warning when it cannot be read or used.
pub fn load(path: &Path) -> (Config, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Config::default(), None),
        Err(e) => return (Config::default(), Some(format!("{}: {e}", path.display()))),
    };
    match parse(&text) {
        Ok(config) => (config, None),
        Err(e) => (
            Config::default(),
            Some(format!("{}: {e}; using the defaults", path.display())),
        ),
    }
}

/// Parses the text of `config.toml`; fields it does not know are ignored.
pub fn parse(text: &str) -> Result<Config, String> {
    #[derive(Deserialize)]
    struct Raw {
        layout: Option<Vec<String>>,
    }
    let raw: Raw = toml::from_str(text).map_err(|e| e.message().to_owned())?;
    let layout = match raw.layout {
        Some(names) => layout(&names)?,
        None => Layout::default(),
    };
    Ok(Config { layout })
}

fn layout(names: &[String]) -> Result<Layout, String> {
    let refuse = || {
        format!("layout must name \"jump\", \"approve\" and \"select\" once each, got {names:?}")
    };
    let keys: Vec<Key> = names
        .iter()
        .map(|name| match name.as_str() {
            "jump" => Some(Key::Jump),
            "approve" => Some(Key::Approve),
            "select" => Some(Key::Select),
            _ => None,
        })
        .collect::<Option<_>>()
        .ok_or_else(refuse)?;
    let keys: [Key; 3] = keys.try_into().map_err(|_| refuse())?;
    Layout::new(keys).ok_or_else(refuse)
}
