# 0008. A herdr plugin starts the bridge as a listener that exits when idle

English | [日本語](../ja/adr/0008-listener-started-by-plugin-hooks.md)

- Status: Superseded by [0012](0012-resident-listener-quiet-when-idle.md)
- Base: [`6c97c0a`](https://github.com/ShortArrow/herdr-triton/commit/6c97c0ae35665db88c48818e64809397906e9867)

## Context

The bridge holds the serial port, reads key events and polls herdr, so it has to run for as long as keys can matter. herdr plugins are not long-running daemons in v1: startup and event hooks start a process per invocation. herdr does not time out or kill hook processes, and runs hooks for `pane.agent_status_changed` on every pane (herdr `0d5d6f1`, `src/app/api/plugins/runtime.rs`, `PLUGIN_HOOK_EVENT_KINDS`).

Keys only matter while an agent is waiting: with an empty queue, Jump, Approve and Select all do nothing. While no host listens, the firmware drops key presses (`NoHost`), so a press is never replayed later.

## Decision

A herdr plugin runs `bridge hook` from its `startup` and on `pane.agent_status_changed`. The process that opens the device's serial port becomes the listener; others retry for 500 ms and exit. The listener exits once the queue has been empty for 5 s, or herdr has been unreachable for 5 s. `bridge run` keeps the same loop without the idle exit, for use without the plugin.

## Consequences

- No process runs while nothing is waiting, and herdr starts and ends the listener through documented hooks
- herdr passes `HERDR_SOCKET_PATH`, so the listener serves the session it was started from (ADR 0007)
- The exclusive serial port is the single-instance lock
- A new waiting agent that appears as the listener closes the port is caught by the new hook's 500 ms retry
- The listener's output must not go to herdr's pipes for long; it logs to `HERDR_PLUGIN_STATE_DIR`

## Alternatives

- **A resident bridge started by hand or by the OS**: works without the plugin, but runs all the time and leaves its lifetime to the user; kept as `bridge run`
- **A resident bridge started from `[[startup]]` and detached**: relies on herdr never reaping hook children, which the plugin documentation does not promise
- **One-shot hooks with the device as a HID keyboard bound in herdr**: removes the listener, but key input then goes to whatever window has focus, and the device would stop being a plain IO instrument (ADR 0001)
