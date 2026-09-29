# 0011. Jump falls back to finished agents, then to every agent

English | [日本語](../ja/adr/0011-jump-falls-back-to-done-then-every-agent.md)

- Status: Accepted
- Date: 2026-09-29
- Amends: 0006 (what Jump moves between), 0008 (when the listener exits)

## Context

Once every waiting agent is handled, Jump flashes red and the keypad has nothing to offer, though herdr still shows agents that have finished and not been looked at (`done`). herdr marks an agent seen, and so `idle`, when `agent.focus` brings it on screen (herdr `0d5d6f1`, `src/app/actions.rs`). Panes without an agent are reached as well with the keyboard as with the keypad.

## Decision

Jump picks the first non-empty tier and cycles within it with `agent.focus`: waiting agents, then `done` agents, then every agent in `agent.list` order. The listener stays while anything waits or is `done`, and exits 5 s after both are empty; the last tier is available while a listener runs, including those 5 s and `bridge run`.

## Consequences

- Jump drains `done` agents one by one, since each becomes `idle` once shown
- The Jump LED tells the tiers apart: amber for waiting, green for `done`, white for every agent
- With the plugin and nothing waiting or `done`, the keypad is in `NoHost` and Jump does nothing; cycling agents at any time needs `bridge run`
- Every tier uses the snapshot the bridge already polls, so a Jump costs one request in each

## Alternatives

- **One ring through waiting, `done` and every agent**: every press moves on, so a waiting agent can be passed over on the way to an idle one
- **Cycle every workspace in the last tier**: tried first; it also stops at workspaces with no agent, which the keyboard reaches as easily, and needs `workspace.list` and `workspace.focus` on each press
- **Keep a listener running to cycle agents at any time**: undoes ADR 0008's rule that nothing runs while nothing waits
