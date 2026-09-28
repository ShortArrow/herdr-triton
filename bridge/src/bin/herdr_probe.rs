//! Checks the herdr adapter against a running herdr.
//!
//! ```text
//! herdr_probe [--session <name>]                         version and every agent
//! herdr_probe [--session <name>] send <pane_id> <key>... send keys to one agent
//! herdr_probe [--session <name>] focus <pane_id>         focus one agent
//! herdr_probe [--json] [--session <name>] read select_id <pane_id>
//! herdr_probe [--json] [--session <name>] read pane_id <workspace_id>
//! herdr_probe [--json] read workspace_id <session>
//! ```
//!
//! `read` prints one id (`2` for `w1:p2`) or, with `--json`, the position
//! and the choices at that level. It prints nothing and exits 1 when there
//! is no highlighted option or focused pane or workspace. `--session` picks
//! a session by name and wins over `HERDR_SOCKET_PATH`, as in herdr.

use std::path::PathBuf;
use std::process::ExitCode;

use bridge::herdr::client::Client;
use bridge::herdr::socket_path::{for_session, resolve, Platform};
use bridge::herdr::wire::{is_supported, Request, Response};
use bridge::{position, prompt_screen};
use serde_json::Value;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = take_flag(&mut args, "--json");
    let session = take_option(&mut args, "--session");

    match args.as_slice() {
        [read, level, target] if read == "read" => {
            let session = if level == "workspace_id" { Some(target.clone()) } else { session };
            read_level(&Client::new(socket(session.as_deref())), level, target, json)
        }
        rest => {
            let path = socket(session.as_deref());
            println!("socket: {}", path.display());
            inspect(&Client::new(path), rest)
        }
    }
}

fn socket(session: Option<&str>) -> PathBuf {
    let env = |k: &str| std::env::var(k).ok();
    let temp = std::env::temp_dir();
    match session {
        Some(name) => for_session(name, env, Platform::current(), &temp),
        None => resolve(env, Platform::current(), &temp),
    }
}

/// Prints the id, or the JSON position, of one level.
fn read_level(client: &Client, level: &str, target: &str, json: bool) -> ExitCode {
    let found: Option<(Option<String>, Value)> = match level {
        "select_id" => match client.call(&Request::AgentRead { target: target.into() }) {
            Ok(Response::Screen(text)) => {
                let prompt = prompt_screen::parse(&text);
                Some((
                    position::select_id(prompt.as_ref()),
                    position::select_json(target, prompt.as_ref()),
                ))
            }
            other => fail(other),
        },
        "pane_id" => match client.call(&Request::PaneList { workspace_id: target.into() }) {
            Ok(Response::Panes(panes)) => {
                let id = position::pane_id(&panes);
                if let (Err(not_active), false) = (&id, json) {
                    eprintln!("{}", not_active.message(target));
                }
                Some((id.ok(), position::pane_json(target, &panes)))
            }
            other => fail(other),
        },
        "workspace_id" => match client.call(&Request::WorkspaceList) {
            Ok(Response::Workspaces(workspaces)) => Some((
                position::workspace_id(&workspaces),
                position::workspace_json(target, &workspaces),
            )),
            other => fail(other),
        },
        _ => {
            eprintln!("read: unknown level {level}; use select_id, pane_id or workspace_id");
            None
        }
    };
    match found {
        Some((_, value)) if json => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Some((Some(id), _)) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}

fn fail<T>(outcome: impl std::fmt::Debug) -> Option<T> {
    eprintln!("read: {outcome:?}");
    None
}

/// The version, every agent, and optionally one `send` or `focus`.
fn inspect(client: &Client, rest: &[String]) -> ExitCode {
    match client.call(&Request::Ping) {
        Ok(Response::Pong { version }) => {
            println!("version: {version} (supported: {})", is_supported(&version))
        }
        other => {
            println!("ping failed: {other:?}");
            return ExitCode::FAILURE;
        }
    }
    match client.call(&Request::AgentList) {
        Ok(Response::Agents(agents)) => {
            println!("agents: {}", agents.len());
            for a in agents {
                println!(
                    "  {:<12} {:<10} {:?}{} seq={}",
                    a.pane_id,
                    a.agent.as_deref().unwrap_or("-"),
                    a.status,
                    if a.focused { " focused" } else { "" },
                    a.state_change_seq
                );
            }
        }
        other => println!("agent.list failed: {other:?}"),
    }
    match rest {
        [cmd, target, keys @ ..] if cmd == "send" && !keys.is_empty() => {
            let request = Request::AgentSendKeys { target: target.clone(), keys: keys.to_vec() };
            println!("send_keys: {:?}", client.call(&request));
        }
        [cmd, target] if cmd == "focus" => {
            let request = Request::AgentFocus { target: target.clone() };
            println!("focus: {:?}", client.call(&request));
        }
        _ => {}
    }
    ExitCode::SUCCESS
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let before = args.len();
    args.retain(|a| a != flag);
    args.len() != before
}

fn take_option(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}
