# 0011. Jump falls back to finished agents, then to every agent

English | [日本語](../ja/adr/0011-jump-falls-back-to-done-then-every-agent.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Once every waiting agent is handled, a Jump that only reaches waiting agents has nothing to offer, though herdr still shows agents that have finished and not been looked at (`done`). herdr marks an agent seen, and so `idle`, when `agent.focus` brings it on screen (herdr `0d5d6f1`, `src/app/actions.rs`). Panes without an agent are reached as well with the keyboard as with the keypad.

## Decision

Jump picks the first non-empty tier and cycles within it with `agent.focus`: waiting agents, then `done` agents, then every agent.

## Consequences

- Jump drains `done` agents one by one, since each becomes `idle` once shown
- The Jump LED tells the tiers apart: orange for one waiting agent and reddish orange for two or more, green for `done`, white for every agent
- Every tier uses the snapshot `trident` already polls, so a Jump costs one request in each

## Alternatives

- **One ring through waiting, `done` and every agent**: every press moves on, so a waiting agent can be passed over on the way to an idle one
- **Cycle every workspace in the last tier**: it also stops at workspaces with no agent, which the keyboard reaches as easily, and needs `workspace.list` and `workspace.focus` on each press
