# 0017. Holding Select moves the highlight up

English | [日本語](../ja/adr/0017-hold-select-to-move-up.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

A press of Select moves the highlight down. One press too many means pressing on through the whole list until it wraps back, which in Claude Code's question prompt also passes "Type something." and "Chat about this". A key that acts on `Down` is recognised as held only after it has already acted.

Jump acts on `Down` and adds the hold on top (ADR 0015), because focusing and raising the window go together. Moving down and moving up do not: a hold that sent `up` after a `down` would move the highlight twice to end one option up.

## Decision

- The hold is an option, `select_hold` in `config.toml`: with `"none"`, the default, Select acts on `Down`; `"up"` turns on the rest of this decision
- Select acts on `Up` when the key was down for less than 500 ms, and moves the highlight as a press does
- When Select has stayed down for 500 ms, it sends the agent's back keys once instead, and nothing more for that press, also when it is released
- The hold decides as a press does: it refreshes, needs `approvable(focused)` or `lapsed_focus(focused)`, and on a lapsed pane needs a highlighted numbered list on the screen
- Codex has back keys, `["up"]`, so that the hold works for every built-in agent

## Consequences

- Without the option nobody pays the delay below
- With `"up"`, a short Select moves the highlight when the key comes up, about 0.1–0.2 s after a press acting on `Down` would
- On the first option the hold sends the back keys anyway; whether the list then stays or wraps is the agent's
- With `"up"`, Select's flash comes after release for a short press and at 500 ms for a hold

## Alternatives

- **Send `down` on `Down` and undo it on the hold**: the highlight visibly moves down and then two up, and on a lapsed or last option the undo is not a plain `up`
- **Repeat `up` while held**: the number of moves would depend on how long the key stayed down, which the LED does not show
- **Always on**: every user would pay the delay on Select, including those who never hold it
- **Approve held moves up**: a press meant to confirm would wait for release, and a slow confirm would move the highlight instead
