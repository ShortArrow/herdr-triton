# 0013. キーの配置はプラグインの設定ディレクトリで決める

[English](../../adr/0013-key-layout-in-plugin-config.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

ADR 0006 は、Jump を左、Approve を中、Select を右に置く。
キーパッドの持ち方は人によって違い、別の並びにしたいこともある。
QMK、ZMK、RMK といったキーボードのファームウェアは、VIA や Vial を使って位置を HID のキーコードに割り当て直す。
しかしキーパッドが SCPI で送るのはキーの位置で（ADR 0009）、それに意味を与えるのは `trident` なので、キーコードを割り当て直しても Jump、Approve、Select には届かない。
herdr は自分の `config.toml` の知らない節を無視するので、プラグインはそこに設定を置けない。
一方で herdr はプラグインごとに設定ディレクトリを用意し、プラグインのすべてのコマンドに `HERDR_PLUGIN_CONFIG_DIR` として渡し、ファイルの形式はプラグインに任せ、`herdr plugin config-dir <id>` でその場所を表示する（herdr のドキュメント plugins）。

## Decision

- `trident` は、プラグインの設定ディレクトリにある `config.toml` を読む。
  `trident run` のように `HERDR_PLUGIN_CONFIG_DIR` が無いときは、herdr の設定ディレクトリから同じパスを組み立て、どちらも同じファイルを読む
- `layout` は、左、中、右の位置に置くキーの名前を並べる。`"jump"`、`"approve"`、`"select"` の並べ替えで、既定は ADR 0006 の並び
- ファイルが無いとき、読めないとき、`layout` が並べ替えになっていないときは、既定の配置のまま動く。後の2つはログに書く
- `trident` の中心部は位置ではなくキーで考える。ランタイムが、押された位置をキーに、各キーの LED とフラッシュをそのキーの位置に対応付ける
- ファイルは `trident` の起動時に1回だけ読む

## Consequences

- 配置はキーパッドではなく herdr のインストールに付いて回る。別の PC では、その PC にもファイルが要る
- 配置を変えるには `trident stop` が要る。次のフックが起動するリスナーが読み直す

## Alternatives

- **`%APPDATA%\herdr-triton` や `~/.config/herdr-triton` という別のディレクトリ**: herdr から独立しているが、探す場所が1つ増え、herdr のプラグインの扱い（インストールや削除）の外に出る
- **SCPI のコマンドで配置をキーパッドに保存する**: 配置がデバイスに付いて回るが、flash への書き込みとプロトコルの変更が要る
- **QMK、ZMK、RMK と VIA や Vial**: キーパッドが HID キーボードになり、ADR 0009 の形から外れる
