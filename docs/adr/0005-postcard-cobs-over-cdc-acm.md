# 0005. Frame postcard messages with COBS over CDC-ACM

- Status: Superseded by 0009
- Date: 2026-09-28

## Context

`firmware` and `bridge` are both Rust and share the message types in `protocol`, which must build under `no_std`. The link is a byte stream, so a reader that starts mid-stream or meets a corrupted frame has to find the next message boundary on its own.

A USB network link (CDC-NCM) was considered as the transport instead of serial.

## Decision

The transport is USB CDC-ACM. Each message is serialised with postcard, COBS-encoded, and terminated by `0x00`.

## Consequences

- The message types are defined once and used unchanged on both ends
- `0x00` never appears inside a frame, so a reader resynchronises at the next `0x00`
- The traffic is not readable in a serial terminal; inspecting it needs a decoder
- The types do not depend on the transport. Moving to a datagram transport would drop COBS and send one message per packet

## Alternatives

- **ASCII text lines**: readable and typeable in a terminal, but the firmware would carry a hand-written parser and formatter, and the two ends would share only names
- **CDC-NCM with UDP**: the host would need no serial port lookup, and WSL 2 with mirrored networking might reach the device without usbipd. It needs a TCP/IP stack on the RP2040, an address scheme, a new network adapter and firewall rule on the host, and an NCM class for `usb-device` 0.3, which does not exist; `embassy-usb` has one but is excluded by ADR 0002. It also loses DTR as a host-presence signal. Windows ships a driver for NCM (`UsbNcm.sys`) but not for ECM
