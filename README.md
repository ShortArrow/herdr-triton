# herdr-triton

English | [日本語](docs/ja/README.md)

<p align="center"><img src="docs/assets/triton.svg" width="200" alt="A ram-headed triton holding a trident topped with three keys"></p>

**Approve your agents without touching the mouse.**

Three keys and three LEDs, wired straight into herdr: **Jump** to the agent that waits, **Select** an option, **Approve** it.

- Talks to herdr's API, not to whatever window has focus
- LEDs follow herdr: amber waits, green is done
- A cheap off-the-shelf RP2040 board
- Under 1 MB resident, plain SCPI over USB

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

## License

The code is licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. The icon is licensed under [CC BY 4.0](docs/assets/LICENSE.md).
