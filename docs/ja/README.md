# Herder-Triton

[English](../../README.md) | 日本語

herdr のエージェントが承認待ちになったら、3キーの RP2040-Keyboard-3 で承認待ちの pane を順に移動し、プロンプトの選択肢を選び、確定する。キーの RGB LED には待ち状況と、いま効くキーを出す。

## キー

| キー | 名前 | 動作 | LED |
|---|---|---|---|
| 左 | Jump | 次の承認待ちのエージェントへ。無ければ次の作業を終えたエージェントへ、それも無ければ次のエージェントへ移る | 承認待ちがあれば琥珀の呼吸、作業を終えたエージェントがあれば緑の呼吸、それ以外は白 |
| 中 | Approve | ハイライト中の選択肢を確定する | フォーカス中の pane を確定できるとき緑 |
| 右 | Select | ハイライトを次の選択肢へ動かす。最後の次は最初 | フォーカス中の pane を確定できるとき青 |

承認するなら Jump、Approve の順に押す。
No など別の選択肢にするなら、それがハイライトされるまで Select を押してから Approve を押す。
押したキーは、herdr がリクエストを受け付ければ白く、受け付けなければ赤く1回光る。

3つとも薄い白なら、リスナーが動いていない。承認待ちが無い間はこれが普通の状態。
3つとも赤く点滅していれば、herdr につながっていない。
プラグインから起動したリスナーは 5 秒であきらめて終了し、LED は薄い白に戻る。

## セットアップ

### 1. ツールを入れる

- rustup で Rust を入れる。最初のビルドで `rust-toolchain.toml` が `thumbv6m-none-eabi` を追加する
- `cargo install flip-link drool`
- herdr 0.9.1 以降

### 2. ファームウェアを書き込む

```sh
cargo keypad
```

ファームウェアをビルドし、[`drool`](https://crates.io/crates/drool) で書き込む。
このリポジトリのファームウェアが一度動いていれば、ボタンは要らない。

初回だけ次を行う。

1. Windows では、RP2040 の BOOTSEL インターフェースに WinUSB を入れる。
   手順は [picotool の README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig) のとおりで、BOOT を押しながら USB を挿し、[Zadig](https://zadig.akeo.ie) で `RP2 Boot (Interface 1)` を選び、`WinUSB` を入れる
2. BOOT を押しながら USB を挿し、`cargo keypad` を実行する

USB-C は2つあるうち片方だけに挿す。

### 3. herdr プラグインを登録する

herdr の外のターミナルで、このリポジトリに移動して次を実行する。

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

`herdr plugin link` は、そのターミナルがつながるセッションにプラグインを登録する。
名前付きのセッションに登録するなら、先に `HERDR_SESSION=<name>` を設定する。
`herdr plugin link` はビルドをしないので、更新を取り込んだら同じ `cargo install` を実行し直す。

以後、エージェントのステータスが変わるたびに herdr が `bridge hook` を起動する。
キーパッドのポートを開けたフックがリスナーになり、承認待ちが 5 秒続けて無くなるまで動く（ADR 0008）。

### 4. 動作を確かめる

- どのシリアルターミナルからでも、キーパッドのポートに `*IDN?` を送ると `ShortArrow,herdr-triton,<シリアル番号>,<バージョン>` が返る。ポートの USB シリアル番号は `TRITON-` で始まる
- herdr がプラグインに用意する状態ディレクトリ（Windows では `%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\`）の `bridge.log` に、フックの起動、リスナー、終了の理由が記録される

## プラグインを使わない場合

`cargo run -p bridge --bin bridge -- run` で、止めるまで動き続けるリスナーを起動する。
名前付きのセッションなら `--session <name>` を付ける。

## その他のツール

| コマンド | 用途 |
|---|---|
| `cargo probe` | `io_probe` を書き込む。LED を決まった色で点け、キーの押下を報告するので、どのキーと LED がどこにあるかを確かめられる |
| `cargo run -p bridge --bin herdr_probe` | herdr のバージョンとエージェントを表示する。`read select_id <pane>` でプロンプトのハイライト位置を表示する |

## Documents

- [仕様](spec.md)
- [ADR](adr/)

## Factor

- [herdr Socket API](https://herdr.dev/ja/docs/socket-api/)
- [RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

## Repository Reference

- [herdrdev/herdr](https://github.com/herdrdev/herdr)
