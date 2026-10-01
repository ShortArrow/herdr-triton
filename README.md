# herdr-triton

English | [日本語](docs/ja/README.md)

Three keys and three RGB LEDs for [herdr](https://github.com/herdrdev/herdr) agents waiting on approval: jump to the waiting pane, pick an option, confirm it, without hunting for the pane.

- **Talks to herdr, not to your focus.** It is not a keyboard that types shortcuts into whatever window is active: the bridge calls herdr's socket API, so a press reaches the right pane, and Approve acts only on the focused prompt that is actually waiting
- **The LEDs show herdr's state.** Amber while an agent waits, green while one has finished, and each key lights only when it would act
- **An off-the-shelf board.** A Waveshare RP2040-Keyboard-3, far cheaper than a Stream Deck, with nothing to solder
- **Light.** The resident bridge uses about 0.7 MB in Task Manager on Windows 11 and asks herdr nothing while nothing waits
- **Plain SCPI over USB serial.** `*IDN?`, `LED1 #FF8000,BREathe`, `KEY:EVENt?`: any serial terminal or VISA library can drive the keypad, with or without herdr
- **Open source**, under MIT or Apache-2.0

## Keys

| Key | Does | LED |
|---|---|---|
| Jump (left) | Focus the next waiting agent; with none, the next finished one; with none of those either, the next agent | amber breathing while anything waits, green breathing while agents are finished, white breathing otherwise |
| Approve (middle) | Confirm the highlighted option | green when the focused pane can be confirmed |
| Select (right) | Move the highlight to the next option, wrapping to the first | blue when the focused pane can be confirmed |

To approve, press Jump, then Approve. To pick another option, such as No, press Select until it is highlighted, then Approve. A pressed key flashes white when herdr took the request and red when it did not.

All three dim white means the bridge is not running. All three blinking red means herdr cannot be reached.

## Setup

### 1. Install the tools

- Rust via rustup
- `cargo install flip-link drool`
- herdr 0.9.1 or later

### 2. Flash the firmware

The first time only, on Windows: hold BOOT while plugging the board in, run [Zadig](https://zadig.akeo.ie), select `RP2 Boot (Interface 1)` and install `WinUSB` ([picotool's README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig)). Then, holding BOOT while plugging it in again:

```sh
cargo keypad
```

Later updates need no button. Plug in one of the two USB-C ports only.

### 3. Add the herdr plugin

From a terminal outside herdr, in this checkout:

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

From then on herdr starts the bridge by itself, at its next start or the next time an agent's status changes. Set `HERDR_SESSION=<name>` before `herdr plugin link` for a named session. After pulling, run `.plugin/bin/bridge stop` and the same `cargo install` again.

### 4. Check it

Once the bridge runs, Jump's LED breathes white while nothing waits. `bridge.log` in the plugin's state directory (`%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\` on Windows) says which port the bridge serves, and why it stopped.

## Change the key layout

Put `config.toml` in the directory `herdr plugin config-dir shortarrow.herdr-triton` prints:

```toml
layout = ["select", "jump", "approve"]   # left, middle, right
```

Then run `.plugin/bin/bridge stop`; the next agent status change starts the bridge with the new layout. Each key takes its LED along.

## Without the plugin

`cargo run --release -p bridge --bin bridge -- run` serves the keypad until you stop it. Add `--session <name>` for a named session.

## Documents

- [Specification](docs/spec.md)
- [Architecture decision records](docs/adr/)
- [herdr socket API](https://herdr.dev/docs/socket-api/), [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

For development, `cargo probe` flashes a firmware that shows which key and LED sit where, and `cargo run -p bridge --bin herdr_probe` prints what herdr reports.
