# 0009. Speak SCPI-style text over CDC-ACM

English | [日本語](../ja/adr/0009-scpi-text-protocol.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

The device is a plain IO instrument (ADR 0001), and instruments of that kind are commonly driven with SCPI from any serial terminal. A format that only a decoder from this repository can read means building and running that tool just to check the device. `firmware` and `trident` are both Rust and share their message types through `protocol`, which builds under `no_std`, and a typed definition on both ends catches a mismatch at compile time. The link is a byte stream, so a reader that starts mid-stream or meets a corrupted message has to find the next boundary on its own.

## Decision

The transport is USB CDC-ACM. The host sends SCPI-style commands as text lines, and the device answers queries only. Key events wait in a queue on the device and are read with `KEY:EVENt?`. The `protocol` crate holds typed commands and replies together with their text form, so both ends share one definition.

## Consequences

- Any serial terminal can identify the device, set LEDs and read keys, with no tool from this repository
- The device never speaks unasked, so a terminal session shows only replies to what was typed
- A newline ends every message, so a reader resynchronises at the next line
- Key events reach the host by polling instead of being pushed; the poll doubles as the host's heartbeat
- The firmware carries a small parser and formatter, tested on the host through `protocol`

## Alternatives

- **postcard messages framed with COBS and terminated by `0x00`**: compact, and the message types are used unchanged on both ends, but the traffic is opaque to a serial terminal and checking the device needs a decoder of this repository's own
- **Push key events as unsolicited lines**: lower latency, but interleaves with replies and is not how SCPI instruments behave
- **CDC-NCM with UDP**: the host would need no serial port lookup, and WSL 2 with mirrored networking might reach the device without usbipd. It needs a TCP/IP stack on the RP2040, an address scheme, a new network adapter and firewall rule on the host, and an NCM class for `usb-device` 0.3, which does not exist; `embassy-usb` has one but is excluded by ADR 0002. It also loses DTR as a host-presence signal. Windows ships a driver for NCM (`UsbNcm.sys`) but not for ECM
