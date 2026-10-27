# 0016. Select keeps working on a prompt herdr stopped reporting as blocked

English | [日本語](../ja/adr/0016-select-follows-a-lapsed-prompt.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Claude Code's question prompt lists its options, then "Type something." and, below a rule, "Chat about this". When the highlight is on "Type something.", herdr 0.9.3 reports the pane as `idle` instead of `blocked` while the prompt stays on screen. A pane that is not `blocked` is not in `queue` and not approvable, so a keypad that acts only on approvable panes cannot move the highlight off that option, neither on to "Chat about this" nor back; the keyboard's arrow keys still move it.

Whether Approve may confirm never depends on the screen (ADR 0003). Select reads the screen to decide where the highlight moves (ADR 0010).

## Decision

- An entry that leaves `queue` because its pane turned `idle` is kept as lapsed while the pane stays `idle`. An entry whose confirmation is in `sent` is not kept: that prompt was answered
- Select acts on the focused lapsed pane only if its screen shows a highlighted numbered list, and then moves the highlight as for a blocked pane. Otherwise it flashes red
- Approve does not act on a lapsed pane, and the LEDs do not show one: both would depend on the screen, against ADR 0003, and the LEDs would need a screen read every 250 ms

## Consequences

- Select gets the highlight off "Type something." and on to "Chat about this" or back to the options. When herdr reports the pane `blocked` again, it re-enters `queue` with a new `state_change_seq`
- Approve on "Type something." flashes red; confirming that option means typing, which the keypad does not do
- An arrow key reaches an idle agent only while its screen shows a highlighted numbered list. A list the agent prints in its own output, with `❯` before a number, also counts

## Alternatives

- **Document it and leave it to the keyboard**: the keypad then strands the user on that option, the case the keypad exists to avoid
- **Treat every focused idle pane with a highlighted list as approvable**: Approve would depend on the screen, against ADR 0003
- **Ask herdr to keep reporting `blocked`**: herdr's detection is upstream; this project does not open PRs or feature requests there
