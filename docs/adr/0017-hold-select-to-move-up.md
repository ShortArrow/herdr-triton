# 0017. Holding Select moves the highlight up

English | [日本語](../ja/adr/0017-hold-select-to-move-up.md)

- Status: Accepted
- Base: [`24bf396`](https://github.com/ShortArrow/herdr-triton/commit/24bf3960d30efa2dbd5ecfbbf06b497513417af6)

## Context

Select moved the highlight down only. One press too many meant pressing on through the whole list until it wrapped back, which in Claude Code's question prompt also passes "Type something." and "Chat about this". Select acted on `Down`, so a hold would be recognised only after the highlight had already moved.

ADR 0015 made Jump act on `Down` and added the hold on top, because focusing and raising the window go together. Moving down and moving up do not: a hold that sent `up` after a `down` would move the highlight twice to end one option up.

## Decision

- The hold is an option, `select_hold` in `config.toml`: `"none"`, the default, leaves Select as it was, acting on `Down`; `"up"` turns on the rest of this decision
- Select acts on `Up` when the key was down for less than 500 ms, and moves the highlight as before
- When Select has stayed down for 500 ms, it sends the agent's back keys once instead, and nothing more for that press, also when it is released
- The hold decides as a press does: it refreshes, needs `approvable(focused)` or `lapsed_focus(focused)`, and on a lapsed pane needs a highlighted numbered list on the screen
- Codex gets back keys, `["up"]`, so that the hold works for every built-in agent

## Consequences

- Without the option nothing changes, so nobody pays the delay below unless they ask for the hold
- A short Select moves the highlight when the key comes up, about 0.1–0.2 s later than before
- On the first option the hold sends the back keys anyway; whether the list then stays or wraps is the agent's
- Select's flash comes after release for a short press and at 500 ms for a hold

## Alternatives

- **Send `down` on `Down` and undo it on the hold**: the highlight visibly moves down and then two up, and on a lapsed or last option the undo is not a plain `up`
- **Repeat `up` while held**: the number of moves would depend on how long the key stayed down, which the LED does not show; it can be added later
- **Always on**: every user would pay the delay on Select, including those who never hold it
- **Approve held moves up**: a press meant to confirm would wait for release, and a slow confirm would move the highlight instead
