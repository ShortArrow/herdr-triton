//! herdr's newline-delimited JSON: the requests the bridge sends and the
//! responses it reads (herdr `src/api/schema.rs`, `src/api/schema/response.rs`).

use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::{Agent, Status};

/// The requests the bridge sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Ping,
    AgentList,
    AgentFocus {
        target: String,
    },
    AgentSendKeys {
        target: String,
        keys: Vec<String>,
    },
    /// The visible screen, as plain text.
    AgentRead {
        target: String,
    },
    WorkspaceList,
    PaneList {
        workspace_id: String,
    },
}

/// A workspace or pane: its herdr id and whether it has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub focused: bool,
}

/// The responses the bridge understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Pong {
        version: String,
    },
    Agents(Vec<Agent>),
    /// The reply to `agent.focus`: the agent now focused.
    Agent(Agent),
    /// The reply to `agent.read`: the screen text.
    Screen(String),
    Workspaces(Vec<Node>),
    Panes(Vec<Node>),
    Ok,
    Error {
        code: String,
        message: String,
    },
}

/// Why a response line could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// Not JSON, or not a success or error envelope.
    Malformed(String),
    /// A well-formed result of a type the bridge does not expect.
    Unexpected(String),
}

/// The oldest herdr the bridge works with.
pub const MIN_VERSION: (u64, u64, u64) = (0, 9, 1);

/// Encodes `request` as one line, newline included.
pub fn encode_request(id: &str, request: &Request) -> String {
    let (method, params) = match request {
        Request::Ping => ("ping", json!({})),
        Request::AgentList => ("agent.list", json!({})),
        Request::AgentFocus { target } => ("agent.focus", json!({ "target": target })),
        Request::AgentSendKeys { target, keys } => {
            ("agent.send_keys", json!({ "target": target, "keys": keys }))
        }
        Request::AgentRead { target } => (
            "agent.read",
            json!({ "target": target, "source": "visible" }),
        ),
        Request::WorkspaceList => ("workspace.list", json!({})),
        Request::PaneList { workspace_id } => ("pane.list", json!({ "workspace_id": workspace_id })),
    };
    let mut line = json!({ "id": id, "method": method, "params": params }).to_string();
    line.push('\n');
    line
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Envelope {
    Success { result: Value },
    Failure { error: ErrorBody },
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum KnownResult {
    Pong { version: String },
    AgentList { agents: Vec<AgentInfo> },
    AgentInfo { agent: AgentInfo },
    PaneRead { read: ReadText },
    WorkspaceList { workspaces: Vec<WorkspaceInfo> },
    PaneList { panes: Vec<PaneInfo> },
    Ok {},
}

#[derive(Deserialize)]
struct WorkspaceInfo {
    workspace_id: String,
    focused: bool,
}

#[derive(Deserialize)]
struct PaneInfo {
    pane_id: String,
    focused: bool,
}

#[derive(Deserialize)]
struct ReadText {
    text: String,
}

#[derive(Deserialize)]
struct AgentInfo {
    pane_id: String,
    #[serde(default)]
    agent: Option<String>,
    agent_status: WireStatus,
    focused: bool,
    #[serde(default)]
    state_change_seq: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireStatus {
    Idle,
    Working,
    Blocked,
    Done,
    Unknown,
}

impl From<KnownResult> for Response {
    fn from(result: KnownResult) -> Self {
        match result {
            KnownResult::Pong { version } => Response::Pong { version },
            KnownResult::AgentList { agents } => {
                Response::Agents(agents.into_iter().map(Agent::from).collect())
            }
            KnownResult::AgentInfo { agent } => Response::Agent(agent.into()),
            KnownResult::PaneRead { read } => Response::Screen(read.text),
            KnownResult::WorkspaceList { workspaces } => Response::Workspaces(
                workspaces
                    .into_iter()
                    .map(|w| Node { id: w.workspace_id, focused: w.focused })
                    .collect(),
            ),
            KnownResult::PaneList { panes } => Response::Panes(
                panes
                    .into_iter()
                    .map(|p| Node { id: p.pane_id, focused: p.focused })
                    .collect(),
            ),
            KnownResult::Ok {} => Response::Ok,
        }
    }
}

impl From<AgentInfo> for Agent {
    fn from(info: AgentInfo) -> Self {
        Agent {
            pane_id: info.pane_id,
            agent: info.agent,
            status: match info.agent_status {
                WireStatus::Idle => Status::Idle,
                WireStatus::Working => Status::Working,
                WireStatus::Blocked => Status::Blocked,
                WireStatus::Done => Status::Done,
                WireStatus::Unknown => Status::Unknown,
            },
            focused: info.focused,
            state_change_seq: info.state_change_seq,
        }
    }
}

/// Decodes one response line.
pub fn decode_response(line: &str) -> Result<Response, WireError> {
    let malformed = |e: serde_json::Error| WireError::Malformed(e.to_string());
    match serde_json::from_str::<Envelope>(line).map_err(malformed)? {
        Envelope::Failure { error } => Ok(Response::Error {
            code: error.code,
            message: error.message,
        }),
        Envelope::Success { result } => match result.get("type").and_then(Value::as_str) {
            None => Err(WireError::Malformed("result has no type".into())),
            Some("pong" | "agent_list" | "agent_info" | "pane_read" | "workspace_list" | "pane_list" | "ok") => {
                Ok(serde_json::from_value::<KnownResult>(result)
                    .map_err(malformed)?
                    .into())
            }
            Some(other) => Err(WireError::Unexpected(other.to_owned())),
        },
    }
}

/// Whether `version` (`major.minor.patch`) is at least [`MIN_VERSION`].
/// A version that does not parse is not supported.
pub fn is_supported(version: &str) -> bool {
    let parts: Vec<Option<u64>> = version.split('.').map(|p| p.parse().ok()).collect();
    match parts.as_slice() {
        [Some(major), Some(minor), Some(patch)] => (*major, *minor, *patch) >= MIN_VERSION,
        _ => false,
    }
}
