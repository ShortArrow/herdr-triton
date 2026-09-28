//! Reads an agent's approval prompt off its screen text: the numbered
//! options and which one is highlighted.
//!
//! This depends on how each agent draws its prompt, so it serves
//! diagnostics (`herdr_probe read`) only; the bridge's decisions do not use
//! it (ADR 0003).

/// The options of the last numbered list on a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOptions {
    pub options: Vec<(u32, String)>,
    /// The option marked with Claude Code's `❯` or Codex's `›`.
    pub highlighted: Option<u32>,
}

/// Markers the agents put before the highlighted option.
const MARKERS: [char; 2] = ['❯', '›'];

/// The last numbered list on `screen`, or none. A list is a run of
/// consecutive lines numbered from 1.
pub fn parse(screen: &str) -> Option<PromptOptions> {
    let mut last: Option<PromptOptions> = None;
    let mut current: Option<PromptOptions> = None;
    for line in screen.lines() {
        match option_line(line) {
            Some((marked, 1, label)) => {
                last = current.take().or(last);
                current = Some(PromptOptions {
                    options: vec![(1, label)],
                    highlighted: marked.then_some(1),
                });
            }
            Some((marked, n, label)) if continues(&current, n) => {
                let list = current.as_mut().expect("continues() checked it is some");
                list.options.push((n, label));
                if marked {
                    list.highlighted = Some(n);
                }
            }
            _ => last = current.take().or(last),
        }
    }
    current.or(last)
}

fn continues(list: &Option<PromptOptions>, n: u32) -> bool {
    list.as_ref()
        .is_some_and(|l| l.options.last().is_some_and(|(prev, _)| prev + 1 == n))
}

/// `(highlighted, number, label)` of a line such as ` ❯ 2. No`.
fn option_line(line: &str) -> Option<(bool, u32, String)> {
    let text = line.trim_start();
    let (marked, text) = match text.strip_prefix(MARKERS) {
        Some(rest) => (true, rest.trim_start()),
        None => (false, text),
    };
    let digits = text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let number = text[..digits].parse().ok()?;
    let label = text[digits..].strip_prefix(". ")?;
    Some((marked, number, label.trim().to_owned()))
}

/// `{"w", "p", "s", "options"}` for `pane_id` (`w<workspace>:p<pane>`):
/// where the agent is and which option is highlighted.
pub fn location(pane_id: &str, prompt: Option<&PromptOptions>) -> serde_json::Value {
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
