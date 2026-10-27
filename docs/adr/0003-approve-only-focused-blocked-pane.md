# 0003. Send approval only to the focused pane, and only when it is blocked

English | [日本語](../ja/adr/0003-approve-only-focused-blocked-pane.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

herdr marks an agent `blocked` when it detects a question UI as well as an approval UI. Keys sent to a prompt the user has not looked at can answer a question with an option the user never chose.

herdr detects these UIs by polling the screen, so its status lags the agent. Right after an approval the pane can still read `blocked`, and a second press would land on whatever the agent shows next.

## Decision

- Approve acts only on the pane herdr reports as focused, and only when that pane is `blocked`. Whether the user pressed Jump is not kept as separate state
- Approve refreshes the snapshot immediately before deciding
- After Approve sends its keys, the pane is not approvable again until its `state_change_seq` changes

## Consequences

- In the usual single-client setup, the user has the prompt on screen before approving it
- With several herdr clients attached, herdr's focused pane is the last one any client selected, which may not be the one the user is looking at
- Approval works the same way after Jump and after focusing a pane by hand
- A second press on the same prompt does nothing until herdr reports a change
- The Approve LED follows from the same condition

## Alternatives

- **Approve the head of the queue immediately**: faster, but can answer a question UI by mistake
- **Keep a "jumped" flag**: adds a state whose meaning is unclear once the user moves focus by hand, and it can drift from herdr's real focus
- **Use `pane.read` to tell approval UIs from question UIs**: `trident` would carry per-agent screen detection that duplicates herdr's own, and a misread would confirm a prompt the user did not mean to answer
