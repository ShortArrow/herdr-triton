# Herder-Triton

[English](../../README.md) | 日本語

herdr のエージェントが承認待ちになったら、3キーの RP2040-Keyboard-3 で承認待ちの pane を順に移動し、プロンプトの選択肢を選び、確定する。キーの RGB LED には待ち状況と、いま効くキーを出す。

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

## herdr プラグイン

プラグインは、エージェントのステータスが変わるたびに `bridge hook` を起動する。
キーパッドのポートを開けたフックがリスナーになり、承認待ちが 5 秒続けて無くなるまで動く（ADR 0008）。
herdr 0.9.1 以降が要る。

このリポジトリで次を実行する。

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

`herdr plugin link` はビルドコマンドを実行しないので、更新を取り込んだら同じ `cargo install` で作り直す。
プラグインを使わない場合は、`bridge run`（必要なら `--session <name>`）でリスナーを手で動かし続ける。
リスナーのログは、プラグインの状態ディレクトリの `bridge.log` に出る。

## Factor

- [herdr Socket API](https://herdr.dev/ja/docs/socket-api/)
- [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

## Repository Reference

- [herdrdev/herdr](https://github.com/herdrdev/herdr)
