# 0012. A resident listener that stays quiet while nothing waits

English | [日本語](../ja/adr/0012-resident-listener-quiet-when-idle.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

`trident` holds the serial port, reads key events and talks to herdr, so something has to run for as long as a key can matter, and Jump's last tier (ADR 0011) matters at any time. herdr plugins are not long-running daemons in v1: startup and event hooks start a process per invocation. herdr does not time out or kill hook processes, nor the processes they start, and the plugin documentation does not promise that it never will. herdr runs hooks for `pane.agent_status_changed` on every pane (herdr `0d5d6f1`, `src/app/api/plugins/runtime.rs`, `PLUGIN_HOOK_EVENT_KINDS`). While no host listens, the firmware drops key presses (`NoHost`), so a press is never replayed later.

Keeping a process running has costs, measured or reproduced on Windows 11 with herdr 0.9.1:

- Polling herdr every 250 ms is work nobody needs while nothing waits; a Jump needs herdr's state only when it is pressed
- A child spawned by a hook inherits the hook's stdout and stderr, which are herdr's pipes; herdr then counts the hook as running until the child exits (reproduced: EOF after the child's 6 s instead of 23 ms), and herdr allows 32 running plugin commands
- An idle Rust loop starts near 0.9 MB of Private Bytes and drifts to about 1.2 MB over 10 minutes; `interprocess` adds about 300 KB, serial port enumeration about 260 KB (SetupAPI stays loaded), and decoding `agent.list` through `serde_json::Value` about 350 KB
- A process that exits on a device error leaves the keypad dead until an agent's status next changes, for example after sleep and resume
- herdr treats named sessions as the exception: "Use workspaces first" (herdr docs, concepts)

## Decision

- The plugin runs `trident hook` from its `startup` and on `pane.agent_status_changed`. herdr passes `HERDR_SOCKET_PATH`, so the listener serves the session it was started from (ADR 0007)
- `trident hook` is short-lived. If a listener holds the named mutex, it signals the listener's wake event and exits. Otherwise it finds the keypad's port, spawns `trident listen --port <COM>` and exits. Before spawning it clears the inherit flag on its own standard handles; the child is detached from any console, in a new process group, with its working directory in the plugin's state directory (`HERDR_PLUGIN_STATE_DIR`), where it logs to `trident.log`
- `trident listen` is resident for the life of the herdr server it serves. It holds the named mutex, and is quiet or active:
  - Quiet, while nothing waits or is `done`: no requests to herdr; `KEY:EVENt?` every 250 ms; only Jump's LED shows, breathing white, as Jump is the one key that still acts. Every second it checks that herdr's endpoint still exists without connecting to it (`WaitNamedPipeW` on Windows), so a herdr that stopped is noticed although it is never asked. A wake event or any key press makes it active, sending `ping` and taking a snapshot before acting on the key
  - Active: the loop of ADR 0004 and ADR 0011, with `KEY:EVENt?` every 20 ms, back to quiet 5 s after nothing waits or is `done`
- The listener exits when herdr is unreachable, or its endpoint missing, for 5 s, when herdr's version is not supported, or on `trident stop`. Every herdr call has a timeout. A device error does not end it: it looks for the keypad again every second through a short-lived `trident find-port`, so SetupAPI never loads into the listener
- The first herdr session whose hook starts the listener keeps the keypad; no hand-over between sessions
- Memory target: at most 1 MB in Task Manager's Memory column, the highest value over a 1 h run. To get there the listener uses the C runtime statically, `std` named pipes (waiting when the pipe is busy) instead of `interprocess`, and decodes herdr's replies into typed structs without `Value`
- `trident run` runs the active loop in the foreground, without quiet mode or exit rules, for use without the plugin

## Consequences

- Jump works at any time while herdr runs, including the last tier
- While nothing waits, herdr sees no requests and the keypad is polled four times a second
- The exclusive serial port and the named mutex keep one listener per user session
- Rebuilding the plugin needs `trident stop` first on Windows, since a running executable cannot be replaced
- A serial terminal can reach the keypad only after `trident stop`
- While nothing waits, only Jump's LED breathes white, so dim white on all three (`NoHost`) means no listener: herdr is not running or `trident` failed
- `WAVe`, a rainbow across the keys, is kept only for checking the LEDs
- The mechanisms named here (named mutex and event, detached spawn, handle inheritance) are Windows-specific; Linux and macOS need equivalents before they are supported

## Alternatives

- **A listener that each hook starts and that exits 5 s after nothing waits**: nothing runs while nothing waits, but Jump does nothing after a long idle, and the last tier of ADR 0011 is out of reach
- **A resident listener that keeps polling herdr every 250 ms**: simpler, but spends 345,600 herdr connections a day on nothing
- **Start `trident run` at logon**: runs while herdr does not, and needs the operating system's autostart
- **A resident process started from `[[startup]]` without detaching**: herdr counts the hook as running for the life of the process, against its limit of 32 running plugin commands
- **One-shot hooks with the device as a HID keyboard bound in herdr**: removes the listener, but key input then goes to whatever window has focus, and the device would stop being a plain IO instrument (ADR 0001)
- **Hand the keypad to another session when idle**: herdr's own guidance makes several sessions uncommon, and the hand-over needs its own protocol
