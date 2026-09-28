# Herder-Triton

[English](../../README.md) | 日本語

herdr のエージェントが承認待ちになったら、3キーの RP2040-Keyboard-3 でその pane へ移動し、承認し、次の承認待ちへ進む。キーの RGB LED には待ち状況を出す。

## Documents

- [仕様](spec.md)
- [ADR](adr/)

## 書き込み

`cargo probe` で確認用ファームウェアをビルドし、[`drool`](https://crates.io/crates/drool)（`cargo install drool`）で書き込む。
このリポジトリのファームウェアが一度動いていれば、ボタンは要らない。

初回だけ次を行う。

1. Windows では、RP2040 の BOOTSEL インターフェースに WinUSB を一度入れる。
   手順は [picotool の README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig) のとおりで、BOOT を押しながら USB を挿し、[Zadig](https://zadig.akeo.ie) で `RP2 Boot (Interface 1)` を選び、`WinUSB` を入れる
2. BOOTSEL のまま `cargo probe` を実行する

## Factor

- [herdr Socket API](https://herdr.dev/ja/docs/socket-api/)
- [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

## Repository Reference

- [herdrdev/herdr](https://github.com/herdrdev/herdr)
