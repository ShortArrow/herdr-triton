# 0009. Speak SCPI-style text over CDC-ACM

English | [日本語](../ja/adr/0009-scpi-text-protocol.md)

- Status: Accepted
- Amended by: [0012](0012-resident-listener-quiet-when-idle.md)
- Date: 2026-09-28
- Supersedes: 0005

## Context

ADR 0005 chose postcard frames delimited by COBS. That traffic can only be read with a decoder of our own, so checking the device means building and running a tool from this repository. The device is a plain IO instrument (ADR 0001), and instruments of that kind are commonly driven with SCPI from any serial terminal.

## Decision

The host sends SCPI-style commands as text lines, and the device answers queries only. Key events wait in a queue on the device and are read with `KEY:EVENt?`. The `protocol` crate keeps typed commands and replies and adds their text form, so both ends still share one definition.

## Consequences

- Any serial terminal can identify the device, set LEDs and read keys, with no tool from this repository
- The device never speaks unasked, so a terminal session shows only replies to what was typed
- Key events reach the host by polling every 20 ms instead of being pushed; the poll doubles as the host's heartbeat
- The firmware carries a small parser and formatter, tested on the host through `protocol`
- The postcard and COBS dependencies go away

## Alternatives

- **Keep postcard and COBS (ADR 0005)**: compact and already working, but opaque to generic tools
- **Push key events as unsolicited lines**: lower latency, but interleaves with replies and is not how SCPI instruments behave
