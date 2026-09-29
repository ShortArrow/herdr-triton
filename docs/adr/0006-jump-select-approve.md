# 0006. Jump cycles the waiting panes, Select moves the highlight, Approve confirms it

English | [日本語](../ja/adr/0006-jump-select-approve.md)

- Status: Accepted
- Date: 2026-09-28
- Amends: 0003 (what Approve sends)

## Context

Claude Code and Codex show approval prompts as a list of options whose highlight moves with the arrow keys and is confirmed with `enter`. With Jump, Approve and Next, the keypad could only accept the default option or move between panes; rejecting, or choosing "always allow", needed the keyboard.

Codex's approval overlay handles its shortcut keys first and passes every other key to a list view that moves with the arrow keys and accepts with `enter` (openai/codex `1cc7e23`, `codex-rs/tui/src/bottom_pane/approval_overlay.rs`).

## Decision

- Left, Jump: focus the head of the queue, or the entry after the focused one, cycling
- Middle, Approve: send the agent's confirm keys (`enter`) to the focused pane
- Right, Select: send the agent's select keys (`down`) to the focused pane

Approve and Select both require the focused pane to be approvable (ADR 0003), including the `sent` rule.

## Consequences

- Each key has one role: Jump moves between panes, Select moves within a prompt, Approve commits
- Any option of a prompt can be chosen from the keypad, which also lets the user answer a question UI deliberately
- Approve confirms whatever is highlighted, so the default for Codex changes from `y` to `enter`
- Returning to the head of the queue takes as many Jump presses as there are entries after the focused one
- A long-press "reject" is no longer needed

## Alternatives

- **Jump focuses the head, and when a queued pane is already focused, moves the highlight instead; Next stays**: keeps a direct way to the next pane, but Jump's effect depends on focus, and a user focused on a later entry cannot return to the head with Jump
- **Keep `y` as Codex's approval key**: `y` picks "yes" regardless of the highlight, so Select would have no effect on what Approve does
