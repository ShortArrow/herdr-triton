# 0004. Poll agent.list instead of subscribing to events

- Status: Accepted
- Date: 2026-09-28

## Context

`bridge` needs the status of every agent and which pane is focused. In herdr 0.9.1 (commit `0d5d6f1`):

- A `pane.agent_status_changed` subscription requires a `pane_id`. There is no subscription covering all panes, and a subscription naming a pane that does not exist fails the whole request and closes the connection
- A connection carries one request. A subscription connection only streams events afterwards
- Closing a tab or a workspace, or moving a pane across workspaces, emits no `pane.closed`
- `agent.list` returns every agent with `pane_id`, `agent`, `agent_status`, `focused` and `state_change_seq`
- herdr itself detects Claude Code and Codex status by screen polling every 300 ms

## Decision

`bridge` calls `agent.list` every 250 ms and after each request it sends, and derives its state from the difference between snapshots. It does not use `events.subscribe`.

## Consequences

- A pane that disappears for any reason leaves `queue` on the next snapshot
- Connecting needs no separate initial sync, and there is no ordering race between a subscription and a list
- One request per connection costs a connection every 250 ms
- Status changes reach `bridge` up to 250 ms later than an event would, on top of herdr's own 300 ms detection
- A status that goes out and back between two snapshots is seen only through `state_change_seq`

## Alternatives

- **Per-pane subscriptions managed from `pane.agent_detected` and `pane.created`**: keeps latency low, but every agent needs its own connection, a pane closing between listing and subscribing kills the request, and tab or workspace closes still need a separate reconciliation
- **A herdr plugin event hook**: runs inside herdr on every event, but ties `bridge` to herdr's plugin interface and still needs a channel out to the device
