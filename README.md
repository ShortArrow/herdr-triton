# Herder-Triton

English | [日本語](docs/ja/README.md)

A three-key RP2040-Keyboard-3 for herdr agents waiting on approval: one key jumps between the waiting panes, one moves the highlight through the prompt's options, and one confirms the highlighted option. The RGB LED under each key shows what is waiting and which keys would act.

## Keys

| Key | Name | Does | LED |
|---|---|---|---|
| left | Jump | Focus the next waiting agent; with none, the next finished one; with none of those either, the next agent | amber breathing while anything waits, green breathing while agents are finished, white breathing otherwise |
| middle | Approve | Confirm the highlighted option | green when the focused pane can be confirmed |
| right | Select | Move the highlight to the next option, wrapping to the first | blue when the focused pane can be confirmed |

To approve, press Jump, then Approve. To pick another option, such as No, press Select until it is highlighted, then Approve. A pressed key flashes white when herdr took the request and red when it did not.

All three LEDs dim white means no listener is running: herdr is not running, or the bridge failed. All three blinking red means herdr cannot be reached; a listener started by the plugin gives up after 5 s and the LEDs return to dim white.

## Setup

### 1. Install the tools

- Rust via rustup. `rust-toolchain.toml` adds the `thumbv6m-none-eabi` target on first build
- `cargo install flip-link drool`
- herdr 0.9.1 or later

### 2. Flash the firmware

```sh
cargo keypad
```

This builds the firmware and flashes it with [`drool`](https://crates.io/crates/drool), with no button, once the board runs firmware from this repository.

The first time only:

1. On Windows, install WinUSB for the RP2040 BOOTSEL interface, as [picotool's README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig) describes: hold BOOT while plugging the board in, run [Zadig](https://zadig.akeo.ie), select `RP2 Boot (Interface 1)`, choose `WinUSB`, and install.
2. Hold BOOT while plugging the board in, then run `cargo keypad`.

Plug in one of the two USB-C ports only.

### 3. Add the herdr plugin

From a terminal outside herdr, in this checkout:

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

`herdr plugin link` registers the plugin with the session the terminal reaches; set `HERDR_SESSION=<name>` first for a named session. It does not build anything, so run the same `cargo install` again after pulling. On Windows, run `.plugin/bin/bridge stop` first: the running listener holds its executable.

From then on herdr starts `bridge hook` at startup and whenever an agent's status changes. The first hook starts a resident listener; later ones wake it. The listener stays until herdr stops, asking herdr nothing while nothing waits (ADR 0012).

### 4. Check it

- Any serial terminal at the keypad's port answers `*IDN?` with `ShortArrow,herdr-triton,<serial>,<version>`. The port's USB serial number starts with `TRITON-`
- `bridge.log` in herdr's state directory for the plugin (`%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\` on Windows) shows each listener start, the port it serves, and why it exited

## Without the plugin

`cargo run -p bridge --bin bridge -- run` keeps a listener running until you stop it. Add `--session <name>` for a named session.

## Other tools

| Command | Use |
|---|---|
| `cargo probe` | Flash `io_probe`, which lights each LED in a fixed colour and reports key presses, for checking which key and LED sit where |
| `cargo run -p bridge --bin herdr_probe` | Print herdr's version and agents; `read select_id <pane>` prints a prompt's highlighted option |

## Documents

- [SPEC](docs/spec.md)
- [ADR](docs/adr/)

## Factor

- [herdr Socket API](https://herdr.dev/docs/socket-api/)
- [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

## Repository Reference

- [herdrdev/herdr](https://github.com/herdrdev/herdr)
