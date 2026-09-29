# 0002. ファームウェアとブリッジを Rust で書き、drooling で書き込む

[English](../../adr/0002-rust-with-drooling.md) | 日本語

- Status: Accepted
- Date: 2026-09-28

## Context

ファームウェア（RP2040）とブリッジ（ホスト）の間でメッセージ型を共有したい。
herdr は現在 Windows ネイティブで使っているが、Linux / WSL / macOS でも使う予定がある。
開発中はファームウェアを頻繁に書き換える。

## Decision

- ファームウェア、ブリッジ、両者のメッセージ型（`protocol`, `no_std`）を1つの Cargo workspace に置き、すべて Rust で書く
- ファームウェアは `rp2040-hal` 0.11 と `usb-device` 0.3 を使い、USB を CDC シリアルと `drooling::PicotoolReset` の複合デバイスにする
- `drool run` を cargo runner にして、BOOTSEL ボタンを押さずに書き込む

`rp2040-hal` を 0.11 に留めるのは、最新の `ws2812-pio` 0.9 が `rp2040-hal` 0.11 と `pio` 0.2 に依存し、`rp2040-hal` 0.12 は `pio` 0.3 を使うため。
`drooling` 0.2 はどちらの `rp2040-hal` でも使える。

## Consequences

- メッセージ型の不一致がコンパイル時に検出される
- ブリッジは単一バイナリで配布でき、ランタイムが不要。
  ただし glibc の Linux では、既定の features の `serialport` が `libudev` を動的リンクする
- `usb-device` の `UsbClass` を実装した `drooling` を使うので、`embassy-usb` は使わない
- `drool` は Windows でしか検証されておらず、Linux / macOS からの書き込みは未検証

## Alternatives

- **Arduino-Pico + ホスト側 Rust**: 公式サンプルがそのまま使えるが、メッセージ型を二重に定義することになる
- **CircuitPython + Python**: 試作は速いが、ブリッジの常駐に Python ランタイムが必要で、型の共有もできない
