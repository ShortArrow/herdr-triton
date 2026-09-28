//! Where the user is in herdr, level by level, for diagnostics
//! (`herdr_probe read`): the workspace in a session, the pane in a
//! workspace, and the highlighted option in a pane's prompt.

use crate::prompt_screen::PromptOptions;

/// `{"w", "p", "s", "options"}` for `pane_id` (`w<workspace>:p<pane>`):
/// where the agent is and which option is highlighted.
pub fn select_json(pane_id: &str, prompt: Option<&PromptOptions>) -> serde_json::Value {
    let (workspace, pane) = pane_id.split_once(':').unwrap_or((pane_id, ""));
    let labels: Vec<&str> = prompt
        .map(|p| p.options.iter().map(|(_, label)| label.as_str()).collect())
        .unwrap_or_default();
    serde_json::json!({
        "w": workspace.strip_prefix('w').unwrap_or(workspace),
        "p": pane.strip_prefix('p').unwrap_or(pane),
        "s": prompt.and_then(|p| p.highlighted),
        "options": labels,
    })
}
