use bridge::prompt_screen::{parse, PromptOptions};

fn options(list: &[(u32, &str)], highlighted: Option<u32>) -> Option<PromptOptions> {
    Some(PromptOptions {
        options: list.iter().map(|(n, s)| (*n, s.to_string())).collect(),
        highlighted,
    })
}

/// A Claude Code screen read from herdr 0.9.1: a numbered list in the
/// conversation, then the permission prompt.
const CLAUDE_SCREEN: &str = "\
● Here are the candidates.
  1. Review the open pull request
  2. Tidy the untracked files
  3. Continue the previous task
❯ check
 PowerShell command
   │ gh pr view 42
 Do you want to proceed?
 ❯ 1. Yes
   2. No
 Esc to cancel · Tab to amend
";

#[test]
fn reads_the_claude_prompt_below_an_earlier_numbered_list() {
    assert_eq!(
        parse(CLAUDE_SCREEN),
        options(&[(1, "Yes"), (2, "No")], Some(1))
    );
}

#[test]
fn reads_a_highlight_on_a_later_option() {
    let screen = " Do you want to proceed?\n   1. Yes\n ❯ 2. Yes, and don't ask again\n   3. No\n";
    assert_eq!(
        parse(screen),
        options(
            &[(1, "Yes"), (2, "Yes, and don't ask again"), (3, "No")],
            Some(2)
        )
    );
}

#[test]
fn reads_the_codex_marker() {
    let screen = "  Would you like to run the following command?\n\
                  › 1. Yes, proceed (y)\n\
                  \x20 2. Yes, and don't ask again for this command (a)\n\
                  \x20 3. No, and tell Codex what to do differently (esc)\n";
    assert_eq!(
        parse(screen),
        options(
            &[
                (1, "Yes, proceed (y)"),
                (2, "Yes, and don't ask again for this command (a)"),
                (3, "No, and tell Codex what to do differently (esc)"),
            ],
            Some(1)
        )
    );
}

#[test]
fn a_list_without_a_marker_has_no_highlight() {
    assert_eq!(
        parse("  1. first\n  2. second\n"),
        options(&[(1, "first"), (2, "second")], None)
    );
}

#[test]
fn a_screen_without_numbered_options_has_none() {
    assert_eq!(parse("● working on it\n❯ \n"), None);
}

#[test]
fn a_prompt_line_that_is_not_an_option_is_not_read_as_one() {
    assert_eq!(parse("❯ check\n❯ 1.5 is not an option\n"), None);
}

/// A Claude Code question read from herdr 0.9.1: each option has a
/// description line, and a rule separates the last option.
const CLAUDE_QUESTION: &str = "\
❯ continue
────────────────────────────────────────
 ☐ Next step

│ What next?

  1. Commit now (Recommended)
     Stage the changed files and commit them locally.
  2. Commit and push
     Commit locally, then push to the remote.
  3. Move on without committing
     Leave the changes uncommitted and start the next task.
  4. Review the diff first
     Go through the diff together before committing.
  5. Type something.
────────────────────────────────────────
❯ 6. Chat about this

Enter to select · ↑/↓ to navigate · ctrl+g to edit in Notepad · Esc to cancel
";

#[test]
fn reads_a_question_whose_options_have_descriptions_and_a_rule() {
    assert_eq!(
        parse(CLAUDE_QUESTION),
        options(
            &[
                (1, "Commit now (Recommended)"),
                (2, "Commit and push"),
                (3, "Move on without committing"),
                (4, "Review the diff first"),
                (5, "Type something."),
                (6, "Chat about this"),
            ],
            Some(6)
        )
    );
}

#[test]
fn a_skipped_number_ends_the_list() {
    assert_eq!(parse("  1. a\n  2. b\n  4. d\n"), options(&[(1, "a"), (2, "b")], None));
}
