# Herder-Triton

English | [日本語](docs/ja/README.md)

A three-key RP2040-Keyboard-3 for herdr agents waiting on approval: one key jumps between the waiting panes, one moves the highlight through the prompt's options, and one confirms the highlighted option. The RGB LED under each key shows what is waiting and which keys would act.

## Documents

- [SPEC](docs/spec.md)
- [ADR](docs/adr/)

## Flashing

`cargo probe` builds the hardware probe and flashes it with [`drool`](https://crates.io/crates/drool) (`cargo install drool`). No button is needed once the board runs firmware from this repository.

Before the first flash:

1. On Windows, install WinUSB for the RP2040 BOOTSEL interface once, as [picotool's README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig) describes: hold BOOT while plugging in, run [Zadig](https://zadig.akeo.ie), select `RP2 Boot (Interface 1)`, choose `WinUSB`, and install.
2. With the board still in BOOTSEL, run `cargo probe`.

## Factor

- [herdr Socket API](https://herdr.dev/docs/socket-api/)
- [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

## Repository Reference

- [herdrdev/herdr](https://github.com/herdrdev/herdr)
