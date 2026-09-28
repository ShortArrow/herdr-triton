//! Reads an agent's approval prompt off its screen text: the numbered
//! options and which one is highlighted.
//!
//! This depends on how each agent draws its prompt, so it only ever decides
//! where Select moves a highlight (ADR 0010) and what `herdr_probe read`
//! prints; whether Approve may confirm never depends on it (ADR 0003).

/// The options of the last numbered list on a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOptions {
    pub options: Vec<(u32, String)>,
    /// The option marked with Claude Code's `❯` or Codex's `›`.
    pub highlighted: Option<u32>,
}

/// Markers the agents put before the highlighted option.
const MARKERS: [char; 2] = ['❯', '›'];

/// The last numbered list on `screen`, or none. A list starts at an option
/// numbered 1 and runs through options numbered one more each time; other
/// lines between them, such as descriptions and rules, do not end it.
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
            // Descriptions and rules between options keep the list open.
            None => {}
            Some(_) => last = current.take().or(last),
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
