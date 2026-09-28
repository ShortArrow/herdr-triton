//! `bridge run` and `bridge hook` (ADR 0008).
//!
//! ```text
//! bridge [--session <name>] run    keep going until the device or herdr fails
//! bridge [--session <name>] hook   become the listener if the device is free,
//!                                  and exit once idle
//! ```
//!
//! `hook` is what the herdr plugin runs; it logs to
//! `$HERDR_PLUGIN_STATE_DIR/bridge.log`, `run` logs to standard error.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bridge::device::{find_port, Device};
use bridge::herdr::client::Client;
use bridge::herdr::socket_path::{for_session, resolve, Platform};
use bridge::runtime::{Exit, Mode, Runtime};
use bridge::state::{AgentKeys, PromptKeys};
use protocol::scpi::PROTOCOL_VERSION;

/// How long a hook keeps trying a port another bridge holds.
const PORT_RETRY: Duration = Duration::from_millis(500);
const TICK: Duration = Duration::from_millis(20);
/// How long a query waits for the keypad's reply.
const REPLY_TIMEOUT: Duration = Duration::from_millis(200);

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let session = take_option(&mut args, "--session");
    let mode = match args.as_slice() {
        [cmd] if cmd == "run" => Mode::Run,
        [cmd] if cmd == "hook" => Mode::Hook,
        _ => {
            eprintln!("usage: bridge [--session <name>] run|hook");
            return ExitCode::FAILURE;
        }
    };
    let mut log = Log::open(mode);
    let socket = socket(session.as_deref());
    log.line(&format!("{mode:?}: herdr socket {}", socket.display()));

    let ports = serialport::available_ports().unwrap_or_default();
    let Some(name) = find_port(&ports) else {
        log.line("no TRITON- keypad found");
        return if mode == Mode::Hook { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    };
    let Some(port) = open(&name, mode) else {
        log.line(&format!("{name} is held by another bridge"));
        return if mode == Mode::Hook { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    };
    let mut device = Device::new(port);
    match device.protocol() {
        Ok(PROTOCOL_VERSION) => {}
        other => {
            log.line(&format!("{name}: protocol {other:?}, expected {PROTOCOL_VERSION}"));
            return ExitCode::FAILURE;
        }
    }
    log.line(&format!("listening on {name}"));

    let clock = Instant::now();
    let now = || clock.elapsed().as_millis() as u64;
    let mut runtime = Runtime::new(Client::new(socket), device, prompt_keys(), mode);
    let exit = run_until_exit(&mut runtime, now);
    log.line(&format!("exit: {exit:?}"));
    match exit {
        Exit::Idle | Exit::HerdrGone => ExitCode::SUCCESS,
        Exit::DeviceLost => ExitCode::FAILURE,
    }
}

fn run_until_exit<H, K>(runtime: &mut Runtime<H, K>, now: impl Fn() -> u64) -> Exit
where
    H: bridge::runtime::Herdr,
    K: bridge::runtime::Keys,
{
    if let Err(exit) = runtime.start(now()) {
        return exit;
    }
    loop {
        sleep(TICK);
        if let Err(exit) = runtime.tick(now()) {
            return exit;
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

/// Opens `name` with DTR high and the input discarded. A hook retries for
/// [`PORT_RETRY`], since the listener it replaces may be closing the port.
fn open(name: &str, mode: Mode) -> Option<Box<dyn serialport::SerialPort>> {
    let deadline = Instant::now() + if mode == Mode::Hook { PORT_RETRY } else { Duration::ZERO };
    loop {
        let opened = serialport::new(name, 115_200)
            .timeout(REPLY_TIMEOUT)
            .dtr_on_open(true)
            .open();
        match opened {
            Ok(port) => {
                let _ = port.clear(serialport::ClearBuffer::Input);
                return Some(port);
            }
            Err(_) if Instant::now() < deadline => sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
}

/// The built-in prompt keys (specification, "Prompt keys").
fn prompt_keys() -> AgentKeys {
    let keys = || PromptKeys { confirm: vec!["enter".into()], select: vec!["down".into()] };
    [("claude".to_string(), keys()), ("codex".to_string(), keys())].into()
}

fn take_option(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}

/// Standard error for `run`; for `hook`, a file in the plugin's state
/// directory, so that herdr's pipe is not the listener's log.
struct Log(Box<dyn Write>);

impl Log {
    fn open(mode: Mode) -> Self {
        let file = std::env::var_os("HERDR_PLUGIN_STATE_DIR")
            .filter(|_| mode == Mode::Hook)
            .and_then(|dir| {
                OpenOptions::new().create(true).append(true).open(PathBuf::from(dir).join("bridge.log")).ok()
            });
        match file {
            Some(f) => Log(Box::new(f)),
            None => Log(Box::new(io::stderr())),
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.0, "[pid {}] {text}", std::process::id());
    }
}
