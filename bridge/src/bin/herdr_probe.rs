//! Checks the herdr adapter against a running herdr: prints the socket path,
//! the version, and every agent `agent.list` reports.
//!
//! `herdr_probe` reads only. `herdr_probe send <pane_id> <key>...` also sends
//! keys to one agent, for checking key names such as `down` and `enter`.

use bridge::herdr::client::Client;
use bridge::herdr::socket_path::{resolve, Platform};
use bridge::herdr::wire::{is_supported, Request, Response};

fn main() {
    let path = resolve(|k| std::env::var(k).ok(), Platform::current(), &std::env::temp_dir());
    println!("socket: {}", path.display());
    let client = Client::new(path);

    match client.call(&Request::Ping) {
        Ok(Response::Pong { version }) => println!("version: {version} (supported: {})", is_supported(&version)),
        other => {
            println!("ping failed: {other:?}");
            std::process::exit(1);
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

    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, target, keys @ ..] = args.as_slice() {
        if cmd == "send" && !keys.is_empty() {
            let request = Request::AgentSendKeys { target: target.clone(), keys: keys.to_vec() };
            println!("send_keys: {:?}", client.call(&request));
        }
    }
}
