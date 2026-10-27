# 0005. Frame postcard messages with COBS over CDC-ACM

English | [日本語](../ja/adr/0005-postcard-cobs-over-cdc-acm.md)

- Status: Withdrawn
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

This record decided that the device and the host exchange postcard messages, COBS-encoded and terminated by `0x00`, over USB CDC-ACM. The wire format is now decided in [0009](0009-scpi-text-protocol.md), whose Alternatives carry this design.
