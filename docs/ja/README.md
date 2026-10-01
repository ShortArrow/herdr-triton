# herdr-triton

[English](../../README.md) | 日本語

[herdr](https://github.com/herdrdev/herdr) のエージェントの承認待ちを、3つのキーと3つの RGB LED でさばく。
承認待ちの pane へ移り、選択肢を選び、確定する。pane を探し回る必要はない。

- **herdr と直接つながる。** アクティブなウィンドウにショートカットを打ち込むキーボードではない。bridge が herdr の socket API を呼ぶので、押せば目的の pane に届き、Approve は本当に承認待ちのフォーカス中のプロンプトにだけ効く
- **LED が herdr の状態を映す。** 承認待ちがあれば琥珀、作業を終えたエージェントがあれば緑。キーは効くときだけ光る
- **市販のボードそのまま。** Waveshare の RP2040-Keyboard-3 で、Stream Deck よりずっと安く、はんだ付けも要らない
- **軽い。** 常駐する bridge は、Windows 11 のタスクマネージャーで約 0.7 MB。待つものが無い間は herdr に何も問い合わせない
- **USB シリアル上の素の SCPI。** `*IDN?`、`LED1 #FF8000,BREathe`、`KEY:EVENt?` のように、どのシリアルターミナルや VISA ライブラリからでも、herdr 無しでもキーパッドを操作できる
- **オープンソース。** MIT または Apache-2.0

## キー

| キー | 動作 | LED |
|---|---|---|
| Jump（左） | 次の承認待ちのエージェントへ。無ければ次の作業を終えたエージェントへ、それも無ければ次のエージェントへ移る | 承認待ちがあれば琥珀の呼吸、作業を終えたエージェントがあれば緑の呼吸、それ以外は白の呼吸 |
| Approve（中） | ハイライト中の選択肢を確定する | フォーカス中の pane を確定できるとき緑 |
| Select（右） | ハイライトを次の選択肢へ動かす。最後の次は最初 | フォーカス中の pane を確定できるとき青 |

承認するなら Jump、Approve の順に押す。
No など別の選択肢にするなら、それがハイライトされるまで Select を押してから Approve を押す。
押したキーは、herdr がリクエストを受け付ければ白く、受け付けなければ赤く1回光る。

3つとも薄い白なら、bridge が動いていない。3つとも赤く点滅していれば、herdr につながっていない。

## セットアップ

### 1. ツールを入れる

- rustup で Rust を入れる
- `cargo install flip-link drool`
- herdr 0.9.1 以降

### 2. ファームウェアを書き込む

初回だけ、Windows では BOOT を押しながら USB を挿し、[Zadig](https://zadig.akeo.ie) で `RP2 Boot (Interface 1)` を選んで `WinUSB` を入れる（[picotool の README](https://github.com/raspberrypi/picotool/blob/develop/README.md#zadig)）。
そのあと、もう一度 BOOT を押しながら挿して、次を実行する。

```sh
cargo keypad
```

2回目以降の更新では、ボタンは要らない。USB-C は2つあるうち片方だけに挿す。

### 3. herdr プラグインを登録する

herdr の外のターミナルで、このリポジトリに移動して次を実行する。

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

以後、herdr は次の起動時か、次にエージェントのステータスが変わったときに、自分で bridge を起動する。
名前付きのセッションなら、`herdr plugin link` の前に `HERDR_SESSION=<name>` を設定する。
更新を取り込んだら、`.plugin/bin/bridge stop` のあと、同じ `cargo install` を実行し直す。

### 4. 動作を確かめる

bridge が動いていれば、待つものが無い間は Jump の LED が白く呼吸する。
プラグインの状態ディレクトリ（Windows では `%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\`）の `bridge.log` に、受け持つポートと、止まった理由が記録される。

## キーの配置を変える

`herdr plugin config-dir shortarrow.herdr-triton` が表示するディレクトリに `config.toml` を置く。

```toml
layout = ["select", "jump", "approve"]   # 左、中、右
```

そのあと `.plugin/bin/bridge stop` を実行すると、次にエージェントのステータスが変わったときに、新しい配置で bridge が起動する。
キーは自分の LED を連れて移る。

## プラグインを使わない場合

`cargo run --release -p bridge --bin bridge -- run` で、止めるまでキーパッドを受け持つ。
名前付きのセッションなら `--session <name>` を付ける。

## ドキュメント

- [仕様](spec.md)
- [設計判断の記録（ADR）](adr/)
- [herdr socket API](https://herdr.dev/docs/socket-api/)、[RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3)

開発用に、`cargo probe` はどのキーと LED がどこにあるかを示すファームウェアを書き込み、`cargo run -p bridge --bin herdr_probe` は herdr が報告する内容を表示する。
