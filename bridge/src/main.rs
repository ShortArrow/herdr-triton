//! The `bridge` commands (ADR 0012).
//!
//! ```text
//! bridge [--session <name>] run                keep going until the device or
//!                                              herdr fails
//! bridge [--session <name>] hook               wake the listener, or start one
//! bridge [--session <name>] listen --port <p>  be the resident listener
//! bridge stop                                  stop the listener
//! bridge find-port                             print the keypad's port
//! ```
//!
//! `hook` is what the herdr plugin runs; it logs to
//! `$HERDR_PLUGIN_STATE_DIR/bridge.log`, and the listener it starts logs to
//! `bridge.log` in its working directory, the same state directory. `run`
//! logs to standard error. `listen` and `stop` need Windows; elsewhere,
//! `hook` runs the listener in its own process.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use bridge::config::{self, Config};
use bridge::device::{find_port, Device};
use bridge::herdr::client::Client;
use bridge::herdr::socket_path::{for_session, resolve, Platform};
use bridge::listener::Signal;
use bridge::runtime::{Exit, Mode, Runtime};
use bridge::state::{AgentKeys, PromptKeys, Wrap};
use protocol::scpi::PROTOCOL_VERSION;

/// How long a query waits for the keypad's reply.
const REPLY_TIMEOUT: Duration = Duration::from_millis(200);

type Keypad = Device<Box<dyn serialport::SerialPort>>;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let session = take_option(&mut args, "--session");
    let port = take_option(&mut args, "--port");
    let socket = || socket(session.as_deref());
    match (args.as_slice(), port) {
        ([cmd], None) if cmd == "run" => run(&socket()),
        ([cmd], None) if cmd == "hook" => hook(session.as_deref(), &socket()),
        ([cmd], Some(port)) if cmd == "listen" => listen(&socket(), port),
        ([cmd], None) if cmd == "stop" => stop(),
        ([cmd], None) if cmd == "find-port" => print_port(),
        _ => {
            eprintln!(
                "usage: bridge [--session <name>] run|hook|listen --port <port>\n       bridge stop|find-port"
            );
            ExitCode::FAILURE
        }
    }
}

/// The active loop on this process's own port search, logging to stderr.
fn run(socket: &Path) -> ExitCode {
    let mut log = Log::stderr();
    in_process(socket, Mode::Run, &mut log)
}

/// Finds the keypad here and serves it until the loop ends.
fn in_process(socket: &Path, mode: Mode, log: &mut Log) -> ExitCode {
    log.line(&format!("{mode:?}: herdr socket {}", socket.display()));
    let Some(name) = keypad_port() else {
        log.line("no TRITON- keypad found");
        return ExitCode::FAILURE;
    };
    let keypad = match open_keypad(&name) {
        Ok(keypad) => keypad,
        Err(e) => {
            log.line(&format!("{name}: {e}"));
            return ExitCode::FAILURE;
        }
    };
    log.line(&format!("serving {name}"));
    let config = read_config(log);
    let clock = Instant::now();
    let mut runtime = Runtime::new(Client::new(socket.into()), keypad, prompt_keys(), mode)
        .with_layout(config.layout);
    let ended = serve(&mut runtime, &clock, |d| {
        sleep(d);
        None
    });
    log.line(&format!("{ended:?}"));
    ended.code()
}

/// Wakes the running listener, or starts one on the keypad's port.
#[cfg(windows)]
fn hook(session: Option<&str>, _socket: &Path) -> ExitCode {
    use bridge::listener::{running, send, spawn_detached, Names, LISTENER};

    let names = Names::new(LISTENER);
    if running(&names) {
        let _ = send(&names, Signal::Wake);
        return ExitCode::SUCCESS;
    }
    let mut log = Log::in_state_dir();
    let Some(port) = keypad_port() else {
        log.line("hook: no TRITON- keypad found");
        return ExitCode::SUCCESS;
    };
    let mut command = match std::env::current_exe() {
        Ok(exe) => std::process::Command::new(exe),
        Err(e) => {
            log.line(&format!("hook: {e}"));
            return ExitCode::FAILURE;
        }
    };
    if let Some(name) = session {
        command.args(["--session", name]);
    }
    command
        .args(["listen", "--port", &port])
        .current_dir(state_dir());
    match spawn_detached(&mut command) {
        Ok(child) => {
            log.line(&format!("hook: started listener {} on {port}", child.id()));
            ExitCode::SUCCESS
        }
        Err(e) => {
            log.line(&format!("hook: could not start the listener: {e}"));
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn hook(_session: Option<&str>, socket: &Path) -> ExitCode {
    in_process(socket, Mode::Listen, &mut Log::in_state_dir())
}

/// Holds the listener's mutex and serves the keypad until herdr goes or
/// `bridge stop`; while the keypad is missing, looks for it every second.
#[cfg(windows)]
fn listen(socket: &Path, mut port: String) -> ExitCode {
    use bridge::listener::{Claim, Names, LISTENER};

    let mut log = Log::file(Path::new("bridge.log"));
    let claim = match Claim::take(&Names::new(LISTENER)) {
        Ok(Some(claim)) => claim,
        Ok(None) => {
            log.line("listen: another listener is running");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            log.line(&format!("listen: {e}"));
            return ExitCode::FAILURE;
        }
    };
    let wait = |d| claim.wait(d).ok().flatten();
    log.line(&format!("listen: herdr socket {}", socket.display()));
    let config = read_config(&mut log);
    let clock = Instant::now();
    loop {
        match open_keypad(&port) {
            Ok(keypad) => {
                log.line(&format!("listen: serving {port}"));
                let mut runtime = Runtime::new(
                    Client::new(socket.into()),
                    keypad,
                    prompt_keys(),
                    Mode::Listen,
                )
                .with_layout(config.layout);
                let ended = serve(&mut runtime, &clock, wait);
                log.line(&format!("listen: {ended:?}"));
                if ended != Ended::Exit(Exit::DeviceLost) {
                    return ended.code();
                }
            }
            Err(e @ KeypadError::Protocol(_)) => {
                log.line(&format!("listen: {port}: {e}"));
                return ExitCode::FAILURE;
            }
            Err(e) => log.line(&format!("listen: {port}: {e}")),
        }
        match search(&Client::new(socket.into()), wait) {
            Search::Found(found) => port = found,
            Search::Ended(ended) => {
                log.line(&format!("listen: {ended:?} while the keypad was missing"));
                return ended.code();
            }
        }
    }
}

#[cfg(not(windows))]
fn listen(_socket: &Path, _port: String) -> ExitCode {
    eprintln!("bridge listen needs Windows; use bridge hook or bridge run");
    ExitCode::FAILURE
}

/// Sets the listener's stop event; with no listener there is nothing to do.
#[cfg(windows)]
fn stop() -> ExitCode {
    use bridge::listener::{send, Names, LISTENER};

    match send(&Names::new(LISTENER), Signal::Stop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("no listener is running");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bridge stop: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn stop() -> ExitCode {
    eprintln!("bridge stop needs Windows");
    ExitCode::FAILURE
}

/// Prints the keypad's port, for a listener that must not enumerate ports
/// itself (ADR 0012).
fn print_port() -> ExitCode {
    match keypad_port() {
        Some(port) => {
            println!("{port}");
            ExitCode::SUCCESS
        }
        None => ExitCode::FAILURE,
    }
}

fn keypad_port() -> Option<String> {
    find_port(&serialport::available_ports().unwrap_or_default())
}

/// How a served keypad's loop ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ended {
    Exit(Exit),
    Stopped,
}

impl Ended {
    fn code(self) -> ExitCode {
        match self {
            Ended::Exit(Exit::HerdrGone) | Ended::Stopped => ExitCode::SUCCESS,
            Ended::Exit(Exit::Incompatible | Exit::DeviceLost) => ExitCode::FAILURE,
        }
    }
}

/// Runs the loop, waiting between ticks through `wait`, which may return
/// early with a signal.
fn serve<H, K>(
    runtime: &mut Runtime<H, K>,
    clock: &Instant,
    mut wait: impl FnMut(Duration) -> Option<Signal>,
) -> Ended
where
    H: bridge::runtime::Herdr,
    K: bridge::runtime::Keys,
{
    let now = || clock.elapsed().as_millis() as u64;
    if let Err(exit) = runtime.start(now()) {
        return Ended::Exit(exit);
    }
    loop {
        match wait(Duration::from_millis(runtime.interval())) {
            Some(Signal::Stop) => return Ended::Stopped,
            Some(Signal::Wake) => {
                if let Err(exit) = runtime.wake(now()) {
                    return Ended::Exit(exit);
                }
            }
            None => {}
        }
        if let Err(exit) = runtime.tick(now()) {
            return Ended::Exit(exit);
        }
    }
}

/// What a listener without its keypad found.
#[cfg(windows)]
enum Search {
    Found(String),
    Ended(Ended),
}

/// How often a listener without its keypad looks for it again.
#[cfg(windows)]
const SEARCH: Duration = Duration::from_secs(1);

/// Runs `bridge find-port` every [`SEARCH`] until it names a port, herdr's
/// endpoint has been missing for five checks, or a stop arrives.
#[cfg(windows)]
fn search(herdr: &Client, mut wait: impl FnMut(Duration) -> Option<Signal>) -> Search {
    let mut missing = 0;
    loop {
        if wait(SEARCH) == Some(Signal::Stop) {
            return Search::Ended(Ended::Stopped);
        }
        missing = if herdr.present() { 0 } else { missing + 1 };
        if missing >= 5 {
            return Search::Ended(Ended::Exit(Exit::HerdrGone));
        }
        if let Some(port) = find_port_hidden() {
            return Search::Found(port);
        }
    }
}

#[cfg(windows)]
fn find_port_hidden() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let output =
        bridge::listener::run_hidden(std::process::Command::new(exe).arg("find-port")).ok()?;
    let port = String::from_utf8(output.stdout).ok()?;
    (output.status.success() && !port.trim().is_empty()).then(|| port.trim().to_owned())
}

/// Why the keypad could not be served.
#[derive(Debug)]
enum KeypadError {
    Unavailable(serialport::Error),
    Protocol(io::Result<u16>),
}

impl std::fmt::Display for KeypadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeypadError::Unavailable(e) => write!(f, "cannot open: {e}"),
            KeypadError::Protocol(got) => {
                write!(f, "protocol {got:?}, expected {PROTOCOL_VERSION}")
            }
        }
    }
}

/// Opens `name` with DTR high and the input discarded, and checks that the
/// keypad speaks this bridge's protocol.
fn open_keypad(name: &str) -> Result<Keypad, KeypadError> {
    let port = serialport::new(name, 115_200)
        .timeout(REPLY_TIMEOUT)
        .dtr_on_open(true)
        .open()
        .map_err(KeypadError::Unavailable)?;
    let _ = port.clear(serialport::ClearBuffer::Input);
    let mut keypad = Device::new(port);
    match keypad.protocol() {
        Ok(PROTOCOL_VERSION) => Ok(keypad),
        other => Err(KeypadError::Protocol(other)),
    }
}

/// Reads `config.toml` once (ADR 0013), logging why it was not used.
fn read_config(log: &mut Log) -> Config {
    let env = |k: &str| std::env::var(k).ok();
    let path = config::path(env, Platform::current(), &std::env::temp_dir());
    let (config, warning) = config::load(&path);
    if let Some(warning) = warning {
        log.line(&warning);
    }
    config
}

fn socket(session: Option<&str>) -> PathBuf {
    let env = |k: &str| std::env::var(k).ok();
    let temp = std::env::temp_dir();
    match session {
        Some(name) => for_session(name, env, Platform::current(), &temp),
        None => resolve(env, Platform::current(), &temp),
    }
}

/// The plugin's state directory, or the temporary directory outside herdr.
fn state_dir() -> PathBuf {
    std::env::var_os("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// The built-in prompt keys (specification, "Prompt keys"): Codex's list
/// wraps, Claude Code's stops at its last option (ADR 0010).
fn prompt_keys() -> AgentKeys {
    let keys = |wrap| PromptKeys {
        confirm: vec!["enter".into()],
        select: vec!["down".into()],
        wrap,
    };
    [
        (
            "claude".to_string(),
            keys(Wrap::ByScreen {
                back: vec!["up".into()],
            }),
        ),
        ("codex".to_string(), keys(Wrap::Native)),
    ]
    .into()
}

fn take_option(args: &mut Vec<String>, name: &str) -> Option<String> {
    let i = args.iter().position(|a| a == name)?;
    args.remove(i);
    (i < args.len()).then(|| args.remove(i))
}

/// A log of lines tagged with the process id, so that a hook's lines and
/// the listener's can share a file; never herdr's pipe.
struct Log(Box<dyn Write>);

impl Log {
    fn stderr() -> Self {
        Log(Box::new(io::stderr()))
    }

    /// `bridge.log` in the plugin's state directory.
    fn in_state_dir() -> Self {
        Self::file(&state_dir().join("bridge.log"))
    }

    /// Appends to `path`, or falls back to standard error.
    fn file(path: &Path) -> Self {
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(f) => Log(Box::new(f)),
            Err(_) => Self::stderr(),
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.0, "[pid {}] {text}", std::process::id());
    }
}
