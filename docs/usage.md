# Usage

English | [日本語](ja/usage.md)

## Keys and LEDs

| Key | Does | LED |
|---|---|---|
| Jump (left) | Focus the next waiting agent; with none, the next finished one; with none of those either, the next agent. "Next" follows the list in herdr's agents pane, whether it is sorted `grouped` or `priority`. Hold it for half a second to also bring herdr's window to the front | orange breathing while one agent waits, reddish orange while two or more wait, green breathing while agents are finished, white breathing otherwise |
| Approve (middle) | Confirm the highlighted option of the focused prompt | green when the focused pane can be confirmed |
| Select (right) | Move the highlight to the next option, wrapping to the first | blue when the focused pane can be confirmed |

A pressed key flashes white when herdr took the request and red when it did not. Approve acts only on the prompt of the focused pane, and only while that pane is waiting for approval; Jump first.

The keys work whichever window is in front, so a game or a browser can stay there while you approve. When an agent needs typing instead, hold Jump: the terminal that shows herdr comes to the front, restored if it was minimised, and Jump flashes white, or red if no window was found.

| All three LEDs | Means |
|---|---|
| dim white | `trident` is not running |
| blinking red | `trident` cannot reach herdr |
| solid red | herdr is older than 0.9.1 (`trident run` only; the plugin's `trident` stops instead) |

## Change the key layout

Put `config.toml` in the directory this prints:

```sh
herdr plugin config-dir shortarrow.herdr-triton
```

```toml
layout = ["select", "jump", "approve"]   # left, middle, right
```

Name each of `jump`, `approve` and `select` once. Add `select_hold = "up"` to make a half-second hold of Select move the highlight up; a short Select then acts when you release it. Then run `.plugin/bin/trident stop`; herdr starts `trident` with the new layout at the next agent status change. Each key takes its LED along. A file that cannot be used leaves the default layout, and `trident.log` says why.

## Without the plugin

```sh
cargo run --release -p trident --bin trident -- run
```

serves the keypad until you stop it. Add `--session <name>` for a named herdr session.

## Drive the keypad yourself

The keypad is a USB serial device whose USB serial number starts with `TRITON-`, answering SCPI text commands one per line. Stop `trident` first (`.plugin\bin\trident.exe stop`), then from any serial terminal or VISA library:

```text
*IDN?                     -> ShortArrow,herdr-triton,TRITON-…,<version>
LED1 #FF8000,BREathe      breathe the left LED orange
LED:ALL #000000,OFF,#00FF00,SOLid,#0000FF,BLINk
KEY:EVENt?                -> LEFT,DOWN, … or NONE
SYSTem:ERRor?             -> 0,"No error"
```

The specification lists every command, mode and error ([protocol](spec.md#firmware--trident-protocol)).

## Troubleshooting

- **The LEDs stay dim white.** herdr has not started `trident` yet: restart herdr, or wait for an agent to change status. Check `trident.log` in the plugin's state directory, `%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\` on Windows; it records each start, the port served, and why `trident` stopped
- **`cargo install` cannot replace `trident.exe`.** `trident` is running; run `.plugin/bin/trident stop` first
- **A serial terminal cannot open the port.** `trident` holds it; run `.plugin/bin/trident stop` first
- **`cargo keypad` finds no board.** The first time, the board must be in BOOT mode with the WinUSB driver from Zadig (see the README)
- **Jump lands in the workspaces pane instead of on the agent.** A known herdr issue: when herdr's agents pane is too short for every agent and shows a scroll bar, herdr cannot move its focus to an agent scrolled out of view. herdr does focus the agent's pane, but its sidebar falls back to the workspaces pane. Make the agents pane tall enough to show every agent
- **Jump skips the agents of another machine, and Approve and Select flash red on them.** herdr shows agents of a saved machine (`herdr machine`) next to local ones, but `trident` reaches only this machine's herdr server, and no API switches the machine the client shows. The keys act on local agents only

## For development

| Command | Use |
|---|---|
| `cargo probe` | Flash a firmware that lights each LED in a fixed colour and reports key presses, to see which key and LED sit where |
| `cargo run -p trident --bin herdr_probe` | Print herdr's version and agents; `read select_id <pane>` prints a prompt's highlighted option |
