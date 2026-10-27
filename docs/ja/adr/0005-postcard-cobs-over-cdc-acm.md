# 0005. CDC-ACM 上で postcard のメッセージを COBS で区切る

[English](../../adr/0005-postcard-cobs-over-cdc-acm.md) | 日本語

- Status: Withdrawn
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

この記録は、デバイスとホストが USB CDC-ACM 上で、postcard のメッセージを COBS で符号化し `0x00` で終えてやりとりすると決めていた。
通信の形式は今は [0009](0009-scpi-text-protocol.md) が決めており、その Alternatives にこの設計を載せている。
