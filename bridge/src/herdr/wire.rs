//! herdr's newline-delimited JSON: the requests the bridge sends and the
//! responses it reads (herdr `src/api/schema.rs`, `src/api/schema/response.rs`).

use serde::Deserialize;
use serde_json::json;

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
        Request::PaneList { workspace_id } => {
            ("pane.list", json!({ "workspace_id": workspace_id }))
        }
    };
    let mut line = json!({ "id": id, "method": method, "params": params }).to_string();
    line.push('\n');
    line
}

/// One response line, decoded straight into the fields the bridge reads;
/// everything else is skipped without building a JSON tree (ADR 0012).
#[derive(Deserialize)]
struct Envelope {
    result: Option<RawResult>,
    error: Option<ErrorBody>,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: String,
}

/// The union of the result fields the bridge understands, keyed by `type`.
#[derive(Deserialize)]
struct RawResult {
    #[serde(rename = "type")]
    kind: Option<String>,
    version: Option<String>,
    agents: Option<Vec<AgentInfo>>,
    agent: Option<AgentInfo>,
    read: Option<ReadText>,
    workspaces: Option<Vec<WorkspaceInfo>>,
    panes: Option<Vec<PaneInfo>>,
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
    let envelope: Envelope =
        serde_json::from_str(line).map_err(|e| WireError::Malformed(e.to_string()))?;
    if let Some(error) = envelope.error {
        return Ok(Response::Error {
            code: error.code,
            message: error.message,
        });
    }
    let result = envelope
        .result
        .ok_or_else(|| WireError::Malformed("neither result nor error".into()))?;
    let kind = result
        .kind
        .ok_or_else(|| WireError::Malformed("result has no type".into()))?;
    let missing = |field: &str| WireError::Malformed(format!("{kind} has no {field}"));
    let nodes = |items: Vec<(String, bool)>| {
        items
            .into_iter()
            .map(|(id, focused)| Node { id, focused })
            .collect()
    };
    match kind.as_str() {
        "pong" => Ok(Response::Pong {
            version: result.version.ok_or_else(|| missing("version"))?,
        }),
        "agent_list" => Ok(Response::Agents(
            result
                .agents
                .ok_or_else(|| missing("agents"))?
                .into_iter()
                .map(Agent::from)
                .collect(),
        )),
        "agent_info" => Ok(Response::Agent(
            result.agent.ok_or_else(|| missing("agent"))?.into(),
        )),
        "pane_read" => Ok(Response::Screen(
            result.read.ok_or_else(|| missing("read"))?.text,
        )),
        "workspace_list" => Ok(Response::Workspaces(nodes(
            result
                .workspaces
                .ok_or_else(|| missing("workspaces"))?
                .into_iter()
                .map(|w| (w.workspace_id, w.focused))
                .collect(),
        ))),
        "pane_list" => Ok(Response::Panes(nodes(
            result
                .panes
                .ok_or_else(|| missing("panes"))?
                .into_iter()
                .map(|p| (p.pane_id, p.focused))
                .collect(),
        ))),
        "ok" => Ok(Response::Ok),
        _ => Err(WireError::Unexpected(kind)),
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
