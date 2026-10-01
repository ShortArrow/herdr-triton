# herdr-triton

English | [日本語](docs/ja/README.md)

Three keys for [herdr](https://github.com/herdrdev/herdr) agents waiting on approval: **Jump** to the waiting pane, **Select** an option, **Approve** it. The LED under each key shows what is waiting.

It talks to herdr's API instead of typing into the active window, runs on a cheap off-the-shelf board, stays under 1 MB of memory, and speaks plain SCPI over USB serial. Open source under MIT or Apache-2.0.

## What you need

- A [Waveshare RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3) and a USB-C cable
- herdr 0.9.1 or later on Windows. Linux and macOS are not tested yet
- Rust, installed with [rustup](https://rustup.rs)

## Install

### 1. Install the build tools

```sh
cargo install flip-link drool
```

### 2. Flash the keypad

The first time only, on Windows, let the flasher reach the board:

1. Hold **BOOT** on the board while plugging it in
2. In [Zadig](https://zadig.akeo.ie), select `RP2 Boot (Interface 1)`, pick `WinUSB`, and install it

Then, with the board still plugged in, run this from the checkout:

```sh
cargo keypad
```

When it finishes, the three LEDs glow dim white. Use only one of the board's two USB-C ports.

### 3. Add the herdr plugin

From a terminal outside herdr, in the checkout:

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

For a named herdr session, set `HERDR_SESSION=<name>` before `herdr plugin link`.

### 4. Check it

Restart herdr, or wait for any agent to change status. herdr then starts the bridge, and the left LED starts breathing: white while nothing waits, amber when an agent waits for approval.

If the LEDs stay dim white, see [Troubleshooting](docs/usage.md#troubleshooting).

## Use

When an agent asks for approval, the left LED breathes amber. Press **Jump** (left) to go to it, then **Approve** (middle) to confirm the highlighted option. To choose another option, press **Select** (right) until it is highlighted, then **Approve**.

[Usage](docs/usage.md) explains every key and LED, how to change the key layout, and how to drive the keypad without herdr.

## Update

```sh
git pull
.plugin/bin/bridge stop
cargo install --locked --path bridge --bin bridge --root .plugin
cargo keypad
```

`bridge stop` releases the running bridge so that `cargo install` can replace it; herdr starts the new one at the next agent status change. `cargo keypad` needs no button after the first time.

## Documents

- [Usage](docs/usage.md)
- [Specification](docs/spec.md)
- [Architecture decision records](docs/adr/)
