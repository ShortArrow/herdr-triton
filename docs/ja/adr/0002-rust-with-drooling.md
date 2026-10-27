# 0002. ファームウェアと trident を Rust で書き、drooling で書き込む

[English](../../adr/0002-rust-with-drooling.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

ファームウェア（RP2040）と `trident`（ホスト）がやりとりするメッセージの型は、1か所で定義したい。
herdr は Windows、Linux、WSL、macOS で動き、ホスト側のプログラムも herdr が動くところで動く必要がある。
開発中はファームウェアを頻繁に書き換える。

## Decision

- ファームウェア、`trident`、両者のメッセージ型（`protocol`）、ハードウェアを除いたキーパッドの振る舞い（`keypad`）を1つの Cargo workspace に置き、すべて Rust で書く。`protocol` と `keypad` は `no_std` とする
- ファームウェアは `rp2040-hal` 0.11 と `usb-device` 0.3 を使い、USB を CDC シリアルと `drooling::PicotoolReset` の複合デバイスにする
- `drool run` を cargo runner にして、初回より後は BOOTSEL ボタンを押さずに書き込む

`rp2040-hal` を 0.11 に留めるのは、最新の `ws2812-pio` 0.9 が `rp2040-hal` 0.11 と `pio` 0.2 に依存し、`rp2040-hal` 0.12 は `pio` 0.3 を使うためである。
`drooling` 0.2 はどちらの `rp2040-hal` でも使える。

## Consequences

- メッセージ型の不一致がコンパイル時に検出される
- `trident` は単一バイナリで配布でき、ランタイムが要らない。
  ただし glibc の Linux では、既定の features の `serialport` が `libudev` を動的リンクする
- `usb-device` の `UsbClass` を実装した `drooling` を使うので、`embassy-usb` は使わない
- `drool` は Windows でしか検証されておらず、Linux と macOS からの書き込みは未検証である

## Alternatives

- **Arduino-Pico + ホスト側 Rust**: 公式サンプルがそのまま使えるが、メッセージ型を二重に定義することになる
- **CircuitPython + Python**: 試作は速いが、常駐するホスト側のプログラムに Python ランタイムが要り、型の共有もできない
