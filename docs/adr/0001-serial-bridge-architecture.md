# 0001. Connect the keyboard to a host bridge over serial, not as HID

English | [日本語](../ja/adr/0001-serial-bridge-architecture.md)

- Status: Accepted
- Date: 2026-09-28

## Context

Three keys operate herdr agents, and the LED under each key shows herdr's state. Showing state on the LEDs needs a path from the host to the device. herdr is operated through its Socket API (newline-delimited JSON).

## Decision

The device sends key down and key up over USB CDC serial and receives LED commands, and does nothing else. A resident host process, `bridge`, connects to herdr's Socket API. It holds the state, turns key input into herdr requests and decides what the LEDs show.

## Consequences

- A key press is never typed into whichever window has focus
- Behaviour changes are made in `bridge`; the firmware is reflashed less often
- The behaviour can be tested on the host as a pure state transition in `bridge`
- Without `bridge` running, the device does nothing

## Alternatives

- **Send F13–F15 as a HID keyboard and bind them in herdr**: the LEDs would still need a separate path back from the host. The key presses go to the focused application, so they either do nothing or type stray input when herdr is not focused
- **Make the decisions in firmware**: the device cannot reach herdr's socket, so a host relay is needed anyway. Every behaviour change would need a reflash, and tests would depend on the hardware
