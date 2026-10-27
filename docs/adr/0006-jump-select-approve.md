# 0006. Jump moves between agents, Select moves the highlight, Approve confirms it

English | [日本語](../ja/adr/0006-jump-select-approve.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Claude Code and Codex show approval prompts as a list of options whose highlight moves with the arrow keys and is confirmed with `enter`. Rejecting, or choosing "always allow", means moving the highlight before confirming.

Codex's approval overlay handles its shortcut keys first and passes every other key to a list view that moves with the arrow keys and accepts with `enter` (openai/codex `1cc7e23`, `codex-rs/tui/src/bottom_pane/approval_overlay.rs`). Codex also accepts `y`, which picks "yes" whatever is highlighted.

## Decision

- Jump moves herdr's focus to another agent, cycling
- Select sends the agent's select keys (`down`) to the focused pane, moving the highlight to the next option
- Approve sends the agent's confirm keys (`enter`) to the focused pane, confirming the highlighted option
- Approve requires the focused pane to be approvable (ADR 0003), including the rule that a pane is not approvable again until herdr reports a change. Select acts only on a prompt in the focused pane, and at least whenever Approve may act
- The keys sit, from left to right, as Jump, Approve, Select

## Consequences

- Each key has one role: Jump moves between panes, Select moves within a prompt, Approve commits
- Any option of a prompt can be chosen from the keypad, which also lets the user answer a question UI deliberately
- Codex is confirmed with `enter`, so Select changes what Approve does on every agent
- Jump does not depend on what is focused, so the same press always moves on

## Alternatives

- **Jump, Approve and Next, with Approve sending each agent's accept key**: the keypad could only accept the default option, and rejecting or choosing "always allow" needed the keyboard
- **Jump focuses the head, and when a queued pane is already focused, moves the highlight instead; Next stays**: keeps a direct way to the next pane, but Jump's effect depends on focus, and a user focused on a later entry cannot return to the head with Jump
- **Keep `y` as Codex's approval key**: `y` picks "yes" regardless of the highlight, so Select would have no effect on what Approve does
- **A long press of Approve rejects**: one more meaning on one key, while Select already reaches the reject option
