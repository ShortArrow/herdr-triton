# 0002. Write the firmware and the bridge in Rust, and flash with drooling

English | [日本語](../ja/adr/0002-rust-with-drooling.md)

- Status: Accepted
- Date: 2026-09-28

## Context

The firmware (RP2040) and the bridge (host) exchange messages whose types should be defined once. herdr currently runs on native Windows, and Linux, WSL and macOS are planned. The firmware is reflashed often during development.

## Decision

- The firmware, the bridge and their message types (`protocol`, `no_std`) live in one Cargo workspace, all in Rust
- The firmware uses `rp2040-hal` 0.11 and `usb-device` 0.3, with USB as a composite of CDC serial and `drooling::PicotoolReset`
- `drool run` is the cargo runner, so flashing needs no BOOTSEL button

`rp2040-hal` is held at 0.11 because `ws2812-pio` 0.9, its latest release, depends on `rp2040-hal` 0.11 and `pio` 0.2, while `rp2040-hal` 0.12 uses `pio` 0.3. `drooling` 0.2 accepts both.

## Consequences

- A mismatch in message types fails to compile
- The bridge ships as a single binary with no runtime to install. On glibc Linux, `serialport` with default features links `libudev` dynamically
- Reusing `drooling`, which implements `usb-device`'s `UsbClass`, rules out `embassy-usb`
- `drool` is verified on Windows only, so flashing from Linux or macOS is unverified

## Alternatives

- **Arduino-Pico firmware with a Rust host**: Waveshare's sample works as-is, but the message types would be defined twice
- **CircuitPython with a Python host**: fast to prototype, but the resident bridge needs a Python runtime and the types cannot be shared
