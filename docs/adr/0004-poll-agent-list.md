# 0004. Poll agent.list instead of subscribing to events

English | [日本語](../ja/adr/0004-poll-agent-list.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

`trident` needs the status of every agent and which pane is focused. In herdr 0.9.1 (commit `0d5d6f1`):

- A `pane.agent_status_changed` subscription requires a `pane_id`. There is no subscription covering all panes, and a subscription naming a pane that does not exist fails the whole request and closes the connection
- A connection carries one request. A subscription connection only streams events afterwards
- Closing a tab or a workspace, or moving a pane across workspaces, emits no `pane.closed`
- `agent.list` returns every agent with `pane_id`, `agent`, `agent_status`, `focused` and `state_change_seq`
- herdr itself detects Claude Code and Codex status by screen polling every 300 ms

## Decision

`trident` takes herdr's state from `agent.list`: every 250 ms while it polls herdr, and after each request it sends. It derives its state from the difference between snapshots, and does not use `events.subscribe`.

## Consequences

- A pane that disappears for any reason leaves `queue` on the next snapshot
- Connecting needs no separate initial sync, and there is no ordering race between a subscription and a list
- One request per connection costs a connection every 250 ms while polling
- Status changes reach `trident` up to 250 ms later than an event would, on top of herdr's own 300 ms detection
- A status that goes out and back between two snapshots is seen only through `state_change_seq`

## Alternatives

- **Per-pane subscriptions managed from `pane.agent_detected` and `pane.created`**: keeps latency low, but every agent needs its own connection, a pane closing between listing and subscribing kills the request, and tab or workspace closes still need a separate reconciliation
- **Take the status from a herdr plugin event hook**: runs inside herdr on every event, but still needs a channel to the process that holds the device
