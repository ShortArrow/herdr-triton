//! Where the user is in herdr, level by level, for diagnostics
//! (`herdr_probe read`): the workspace in a session, the pane in a
//! workspace, and the highlighted option in a pane's prompt.

use crate::herdr::wire::Node;
use crate::prompt_screen::PromptOptions;

/// `{"w", "p", "s", "options"}` for `pane_id` (`w<workspace>:p<pane>`):
/// where the agent is and which option is highlighted.
pub fn select_json(pane_id: &str, prompt: Option<&PromptOptions>) -> serde_json::Value {
    let workspace = pane_id.split_once(':').map_or(pane_id, |(w, _)| w);
    let labels: Vec<&str> = prompt
        .map(|p| p.options.iter().map(|(_, label)| label.as_str()).collect())
        .unwrap_or_default();
    serde_json::json!({
        "w": short_workspace(workspace),
        "p": short_pane(pane_id),
        "s": prompt.and_then(|p| p.highlighted),
        "options": labels,
    })
}

/// The highlighted option's number.
pub fn select_id(prompt: Option<&PromptOptions>) -> Option<String> {
    prompt?.highlighted.map(|n| n.to_string())
}

/// The focused pane's id without its `w…:p` prefix.
pub fn pane_id(panes: &[Node]) -> Option<String> {
    focused(panes).map(|p| short_pane(&p.id).to_owned())
}

/// `{"w", "p", "panes"}`: the workspace, its focused pane, and every pane.
pub fn pane_json(workspace_id: &str, panes: &[Node]) -> serde_json::Value {
    let all: Vec<&str> = panes.iter().map(|p| short_pane(&p.id)).collect();
    serde_json::json!({
        "w": short_workspace(workspace_id),
        "p": pane_id(panes),
        "panes": all,
    })
}

/// The focused workspace's id without its `w` prefix.
pub fn workspace_id(workspaces: &[Node]) -> Option<String> {
    focused(workspaces).map(|w| short_workspace(&w.id).to_owned())
}

/// `{"session", "w", "workspaces"}`: the session, its focused workspace,
/// and every workspace.
pub fn workspace_json(session: &str, workspaces: &[Node]) -> serde_json::Value {
    let all: Vec<&str> = workspaces.iter().map(|w| short_workspace(&w.id)).collect();
    serde_json::json!({
        "session": session,
        "w": workspace_id(workspaces),
        "workspaces": all,
    })
}

fn focused(nodes: &[Node]) -> Option<&Node> {
    nodes.iter().find(|n| n.focused)
}

/// `Y` for `wY`.
fn short_workspace(id: &str) -> &str {
    id.strip_prefix('w').unwrap_or(id)
}

/// `2` for `w1:p2`.
fn short_pane(id: &str) -> &str {
    let pane = id.split_once(':').map_or(id, |(_, pane)| pane);
    pane.strip_prefix('p').unwrap_or(pane)
}
