# 0005. CDC-ACM 上で postcard のメッセージを COBS で区切る

[English](../../adr/0005-postcard-cobs-over-cdc-acm.md) | 日本語

- Status: Superseded by [0009](0009-scpi-text-protocol.md)
- Date: 2026-09-28

## Context

`firmware` と `bridge` はどちらも Rust で、メッセージ型を `protocol` で共有する。
`protocol` は `no_std` でビルドできる必要がある。
通信路はバイト列の流れなので、途中から読み始めた側や、壊れたフレームに出会った側が、自力で次のメッセージの境目を見つけられなければならない。

通信路として、シリアルの代わりに USB 経由のネットワーク（CDC-NCM）も検討した。

## Decision

通信路は USB CDC-ACM とする。
各メッセージは postcard でシリアライズし、COBS で符号化して、末尾に `0x00` を付ける。

## Consequences

- メッセージ型は1か所で定義し、両端でそのまま使える
- フレームの中に `0x00` は現れないので、読み手は次の `0x00` で同期し直せる
- シリアルターミナルで中身を読めず、確認するにはデコーダが要る
- 型は通信路に依存しない。
  パケット単位の通信路に移るなら、COBS を外して1パケット1メッセージにすればよい

## Alternatives

- **ASCII のテキスト行**: ターミナルで読めて手でも打てるが、firmware に手書きのパーサとフォーマッタが要り、両端で共有できるのは名前だけになる
- **CDC-NCM 上の UDP**: ホストでシリアルポートを探す必要がなくなり、mirrored ネットワークの WSL 2 なら usbipd なしで届く可能性がある。
  一方で RP2040 に TCP/IP スタックとアドレスの決め方が要り、ホストにはネットワークアダプタとファイアウォールの規則が増える。
  `usb-device` 0.3 向けの NCM クラスは無く、`embassy-usb` にはあるが ADR 0002 で使わないと決めている。
  DTR でホストの存在を知ることもできなくなる。
  Windows には NCM 用のドライバ（`UsbNcm.sys`）は標準で入っているが、ECM 用は無い
