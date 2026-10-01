# 0012. A resident listener that stays quiet while nothing waits

English | [日本語](../ja/adr/0012-resident-listener-quiet-when-idle.md)

- Status: Accepted
- Date: 2026-10-01
- Supersedes: 0008
- Amends: 0009 (a `WAVe` LED mode for checking the LEDs, protocol 3), 0011 (the last tier at any time)

## Context

ADR 0011's last tier, Jump through every agent, only works while a listener runs, and ADR 0008's listener exits 5 s after nothing waits. Keeping a listener has costs that three adversarial reviews measured or reproduced on Windows 11 with herdr 0.9.1:

- Polling herdr every 250 ms forever is work nobody needs while nothing waits; a Jump needs herdr's state only when it is pressed
- A child spawned by a hook inherits the hook's stdout and stderr, which are herdr's pipes; herdr then counts the hook as running until the child exits (reproduced: EOF after the child's 6 s instead of 23 ms), and herdr allows 32 running plugin commands
- The current listener shows 1.2 MB in Task Manager's Memory column. An idle Rust loop on this machine starts near 0.9 MB of Private Bytes and drifts to about 1.2 MB over 10 minutes; `interprocess` adds about 300 KB, serial port enumeration about 260 KB (SetupAPI stays loaded), and decoding `agent.list` through `serde_json::Value` about 350 KB
- A listener that exits on a device error leaves the keypad dead until an agent's status next changes, for example after sleep and resume
- herdr treats named sessions as the exception: "Use workspaces first" (herdr docs, concepts)

## Decision

- `bridge hook` is short-lived. If a listener holds the named mutex, it signals the listener's wake event and exits. Otherwise it finds the keypad's port, spawns `bridge listen --port <COM>` and exits. Before spawning it clears the inherit flag on its own standard handles; the child is detached from any console, in a new process group, with its working directory outside the plugin
- `bridge listen` is resident for the life of the herdr server it serves. It holds the named mutex, and is quiet or active:
  - Quiet, while nothing waits or is `done`: no requests to herdr; `KEY:EVENt?` every 250 ms; only Jump's LED shows, breathing white, as Jump is the one key that still acts. Every second it checks that herdr's endpoint still exists without connecting to it (`WaitNamedPipeW` on Windows), so a herdr that stopped is noticed although it is never asked. A wake event or any key press makes it active, sending `ping` and taking a snapshot before acting on the key
  - Active: the loop of ADR 0004 and ADR 0011, back to quiet 5 s after nothing waits or is `done`
- The listener exits when herdr is unreachable, or its endpoint missing, for 5 s, when herdr's version is not supported, or on `bridge stop`. Every herdr call has a timeout. A device error does not end it: it looks for the keypad again every second through a short-lived `bridge find-port`, so SetupAPI never loads into the listener
- The first herdr session whose hook starts the listener keeps the keypad; no hand-over between sessions
- Memory target: at most 1 MB in Task Manager's Memory column, the highest value over a 1 h run. To get there the listener uses the C runtime statically, `std` named pipes (waiting when the pipe is busy) instead of `interprocess`, and decodes herdr's replies into typed structs without `Value`

## Consequences

- Jump works at any time while herdr runs, including the last tier
- While nothing waits, herdr sees no requests and the keypad is polled four times a second
- Rebuilding the plugin needs `bridge stop` first on Windows, since a running executable cannot be replaced
- A serial terminal can reach the keypad only after `bridge stop`
- While nothing waits, only Jump's LED breathes white instead of `NoHost`'s dim white on all three, so dim white now means no listener: herdr is not running or the bridge failed
- `WAVe`, a rainbow across the keys, was tried as the idle display and kept only for checking the LEDs
- The mechanisms named here (named mutex and event, detached spawn, handle inheritance) are Windows-specific; Linux and macOS need equivalents before they are supported

## Alternatives

- **Keep ADR 0008 and lengthen the linger**: no new process model, but Jump still does nothing after a long idle
- **A resident listener that keeps polling herdr every 250 ms**: simpler, but spends 345,600 herdr connections a day on nothing
- **Start `bridge run` at logon**: runs while herdr does not, and needs the operating system's autostart
- **Hand the keypad to another session when idle**: herdr's own guidance makes several sessions uncommon, and the hand-over needs its own protocol
