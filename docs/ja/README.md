# herdr-triton

[English](../../README.md) | 日本語

<p align="center"><img src="../assets/triton.svg" width="200" alt="3つのキーを付けた三叉の矛を持つ、羊頭のトリトン"></p>

[herdr](https://github.com/herdrdev/herdr) のエージェントの承認待ちをさばく3つのキー。
**Jump** で承認待ちの pane へ移り、**Select** で選択肢を選び、**Approve** で確定する。キーの下の LED が、何が待っているかを示す。

アクティブなウィンドウに打ち込むのではなく herdr の API と直接話し、安い市販のボードで動き、メモリは 1 MB 未満、USB シリアル上の素の SCPI で操作できる。MIT または Apache-2.0 のオープンソース。

## 必要なもの

- [Waveshare RP2040-Keyboard-3](https://www.waveshare.com/wiki/RP2040-Keyboard-3) と USB-C ケーブル
- Windows 上の herdr 0.9.1 以降。Linux と macOS はまだ試していない
- [rustup](https://rustup.rs) で入れた Rust

## インストール

### 1. ビルド用のツールを入れる

```sh
cargo install flip-link drool
```

### 2. キーパッドに書き込む

初回だけ、Windows では書き込みツールがボードに届くようにする。

1. ボードの **BOOT** を押しながら USB を挿す
2. [Zadig](https://zadig.akeo.ie) で `RP2 Boot (Interface 1)` を選び、`WinUSB` を入れる

そのまま挿した状態で、このリポジトリで次を実行する。

```sh
cargo keypad
```

終わると、3つの LED が薄い白で点く。USB-C は2つあるうち片方だけに挿す。

### 3. herdr プラグインを登録する

herdr の外のターミナルで、このリポジトリに移動して次を実行する。

```sh
cargo install --locked --path bridge --bin bridge --root .plugin
herdr plugin link .
```

名前付きの herdr セッションなら、`herdr plugin link` の前に `HERDR_SESSION=<name>` を設定する。

### 4. 確かめる

herdr を再起動するか、どれかのエージェントのステータスが変わるのを待つ。
herdr が bridge を起動し、左の LED が呼吸を始める。待つものが無ければ白、承認待ちがあれば琥珀。

LED が薄い白のままなら、[困ったとき](usage.md#困ったとき) を見る。

## 使う

エージェントが承認を求めると、左の LED が琥珀で呼吸する。
**Jump**（左）でその pane へ移り、**Approve**（中）でハイライト中の選択肢を確定する。
別の選択肢にするなら、それがハイライトされるまで **Select**（右）を押してから **Approve** を押す。

キーと LED のすべての意味、キーの配置の変え方、herdr 無しでの操作は [使い方](usage.md) にある。

## 更新する

```sh
git pull
.plugin/bin/bridge stop
cargo install --locked --path bridge --bin bridge --root .plugin
cargo keypad
```

`bridge stop` で動いている bridge を止めると、`cargo install` が実行ファイルを置き換えられる。新しい bridge は、次にエージェントのステータスが変わったときに herdr が起動する。
2回目以降の `cargo keypad` ではボタンは要らない。

## ドキュメント

- [使い方](usage.md)
- [仕様](spec.md)
- [設計判断の記録（ADR）](adr/)
