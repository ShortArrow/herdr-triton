# 0010. Select wraps to the first option by reading the screen

English | [日本語](../ja/adr/0010-select-wraps-by-reading-the-screen.md)

- Status: Accepted
- Date: 2026-09-29
- Amends: 0006 (what Select sends)

## Context

Select sends `down` to move a prompt's highlight (ADR 0006). Codex's list wraps from its last option to its first; Claude Code's stops at the last, so with six options the keypad could reach the sixth but never return to the first. herdr cannot tell where the highlight is, but the pane's screen shows it: Claude Code marks the highlighted option with `❯`, and `prompt_screen` already reads that for `herdr_probe`.

ADR 0003 declined `pane.read` for deciding whether to approve, because screen detection would duplicate herdr's and a misread there could confirm the wrong option.

## Decision

Each agent's prompt keys say whether its list wraps. For an agent whose list does not wrap, Select reads the visible screen with `agent.read`; if the highlight is on the last of the numbered options it sends the back key (`up`) once per option above, otherwise it sends the select key. A screen that cannot be read falls back to the select key.

## Consequences

- Select cycles through every option on both agents
- A Select on a non-wrapping agent costs one more request to herdr
- A change in how Claude Code draws its list makes Select stop wrapping, not act wrongly: the fallback is the plain `down` of ADR 0006
- The screen decides only where the highlight moves. Approve still confirms whatever is highlighted, and whether it may act still depends only on herdr's state (ADR 0003)

## Alternatives

- **Long-press Select sends `up`**: needs no screen, but returning from the sixth option to the first takes five long presses
- **Send the option's number**: Claude Code may confirm an option as soon as its number is pressed, which would approve without Approve
