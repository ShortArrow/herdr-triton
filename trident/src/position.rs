//! Where the user is in herdr, level by level, for diagnostics
//! (`herdr_probe read`): the workspace in a session, the pane in a
//! workspace, and the highlighted option in a pane's prompt.

use serde::Serialize;

use crate::herdr::wire::Node;
use crate::prompt_screen::PromptOptions;

/// `{"w", "p", "s", "options"}` for `pane_id` (`w<workspace>:p<pane>`):
/// where the agent is and which option is highlighted.
pub fn select_json(pane_id: &str, prompt: Option<&PromptOptions>) -> String {
    #[derive(Serialize)]
    struct Select<'a> {
        w: &'a str,
        p: &'a str,
        s: Option<u32>,
        options: Vec<&'a str>,
    }
    let workspace = pane_id.split_once(':').map_or(pane_id, |(w, _)| w);
    to_json(&Select {
        w: short_workspace(workspace),
        p: short_pane(pane_id),
        s: prompt.and_then(|p| p.highlighted),
        options: prompt
            .map(|p| p.options.iter().map(|(_, label)| label.as_str()).collect())
            .unwrap_or_default(),
    })
}

/// The highlighted option's number.
pub fn select_id(prompt: Option<&PromptOptions>) -> Option<String> {
    prompt?.highlighted.map(|n| n.to_string())
}

/// herdr marks a pane focused only in the workspace on screen, so a
/// workspace without a focused pane is not the active one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotActive;

impl NotActive {
    pub fn message(self, workspace_id: &str) -> String {
        format!("{workspace_id} is not the active workspace")
    }
}

/// The focused pane's id without its `w…:p` prefix.
pub fn pane_id(panes: &[Node]) -> Result<String, NotActive> {
    focused(panes)
        .map(|p| short_pane(&p.id).to_owned())
        .ok_or(NotActive)
}

/// `{"w", "p", "panes"}`: the workspace, its focused pane, and every pane.
pub fn pane_json(workspace_id: &str, panes: &[Node]) -> String {
    #[derive(Serialize)]
    struct Pane<'a> {
        w: &'a str,
        p: Option<String>,
        panes: Vec<&'a str>,
    }
    to_json(&Pane {
        w: short_workspace(workspace_id),
        p: pane_id(panes).ok(),
        panes: panes.iter().map(|p| short_pane(&p.id)).collect(),
    })
}

/// The focused workspace's id without its `w` prefix.
pub fn workspace_id(workspaces: &[Node]) -> Option<String> {
    focused(workspaces).map(|w| short_workspace(&w.id).to_owned())
}

/// `{"session", "w", "workspaces"}`: the session, its focused workspace,
/// and every workspace.
pub fn workspace_json(session: &str, workspaces: &[Node]) -> String {
    #[derive(Serialize)]
    struct Workspace<'a> {
        session: &'a str,
        w: Option<String>,
        workspaces: Vec<&'a str>,
    }
    to_json(&Workspace {
        session,
        w: workspace_id(workspaces),
        workspaces: workspaces.iter().map(|w| short_workspace(&w.id)).collect(),
    })
}

/// Field order is declaration order, which the probe's output keeps.
fn to_json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("plain structs always serialize")
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
