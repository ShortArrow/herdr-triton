# 0007. One trident serves one herdr session

English | [日本語](../ja/adr/0007-one-session-per-bridge.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Each herdr session is a separate server with its own socket, and each attached terminal shows one session. `agent.focus` switches the pane inside that session's clients; it does not bring the session's terminal window forward. herdr's API does not say which session's terminal the user is looking at.

## Decision

A `trident` connects to one session. `--session <name>` picks it and wins over `HERDR_SOCKET_PATH`; without it, the socket is resolved as herdr resolves it.

## Consequences

- The queue, Jump and the LEDs cover one session only
- Using the keypad with another session means stopping `trident` and starting it for that session
- A `trident` started inside a herdr pane without `--session` serves that pane's session, through the pane's `HERDR_SOCKET_PATH`

## Alternatives

- **Poll every session and merge their queues**: Jump to a pane in another session changes that session's focus, but its terminal stays where it is, so the jump looks like nothing happened
- **Follow the session on screen**: herdr's API has no way to tell which session's terminal is in front
