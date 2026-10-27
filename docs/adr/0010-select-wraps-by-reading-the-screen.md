# 0010. Select wraps to the first option by reading the screen

English | [日本語](../ja/adr/0010-select-wraps-by-reading-the-screen.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Select sends `down` to move a prompt's highlight (ADR 0006). Codex's list wraps from its last option to its first; Claude Code's stops at the last, so with `down` alone the keypad reaches the sixth of six options but never returns to the first. herdr does not report where the highlight is, but the pane's screen shows it: Claude Code marks the highlighted option with `❯`.

Whether Approve may act never depends on the screen (ADR 0003), because screen detection would duplicate herdr's and a misread there could confirm the wrong option. Where the highlight moves carries no such risk: a wrong move is undone with another press.

## Decision

Each agent's prompt keys say whether its list wraps. For an agent whose list does not wrap, Select reads the visible screen with `agent.read`; if the highlight is on the last of the numbered options it sends the back key (`up`) once per option above, otherwise it sends the select key. A screen that cannot be read falls back to the select key.

## Consequences

- Select cycles through every option on both agents
- A Select on a non-wrapping agent costs one more request to herdr
- A change in how Claude Code draws its list makes Select stop wrapping, not act wrongly: the fallback is a plain `down`
- The screen decides only where the highlight moves. Approve still confirms whatever is highlighted, and whether it may act still depends only on herdr's state (ADR 0003)

## Alternatives

- **Long-press Select sends `up`**: needs no screen, but returning from the sixth option to the first takes five long presses
- **Send the option's number**: Claude Code may confirm an option as soon as its number is pressed, which would approve without Approve
