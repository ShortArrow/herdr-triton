# 0011. Jump falls back to finished agents, then to workspaces

English | [日本語](../ja/adr/0011-jump-falls-back-to-done-then-workspaces.md)

- Status: Accepted
- Date: 2026-09-29
- Amends: 0006 (what Jump moves between), 0008 (when the listener exits)

## Context

Once every waiting agent is handled, Jump flashes red and the keypad has nothing to offer, though herdr still shows agents that have finished and not been looked at (`done`). herdr marks an agent seen, and so `idle`, when `agent.focus` brings it on screen (herdr `0d5d6f1`, `src/app/actions.rs`). `workspace.focus` switches workspaces.

## Decision

Jump picks the first non-empty tier and cycles within it: waiting agents, then `done` agents, then every workspace. The listener stays while anything waits or is `done`, and exits 5 s after both are empty; the workspace tier is available while a listener runs, including those 5 s and `bridge run`.

## Consequences

- Jump drains `done` agents one by one, since each becomes `idle` once shown
- The Jump LED tells the tiers apart: amber for waiting, green for `done`, white for workspaces
- With the plugin and nothing waiting or `done`, the keypad is in `NoHost` and Jump does nothing; cycling workspaces at any time needs `bridge run`
- A Jump in the workspace tier costs two requests, `workspace.list` and `workspace.focus`

## Alternatives

- **One ring through waiting, `done` and workspaces**: every press moves on, so a waiting agent can be passed over on the way to a workspace
- **Keep a listener running to cycle workspaces at any time**: undoes ADR 0008's rule that nothing runs while nothing waits
