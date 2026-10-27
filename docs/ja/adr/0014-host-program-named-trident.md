# 0014. ホスト側のプログラムを trident と呼ぶ

[English](../../adr/0014-host-program-named-trident.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

ホスト側のプログラムは herdr とキーパッドをつなぎ、常駐プロセスとして動く。タスクマネージャーには実行ファイルの名前で表示される。
「bridge」のように役割だけを表す名前では、そこに herdr もキーパッドも表れず、文書ではプログラムとも役割とも読める。
アイコンには、先に3つのキーが付いた三叉の矛を持つトリトンが描かれている。

## Decision

- ホスト側のプログラムは `trident` とする。crate とそのディレクトリ、実行ファイル `trident.exe`、コマンド `trident run|hook|listen|stop|find-port|raise`、ログ `trident.log`
- 名前はアイコンに合わせる。プラグインがトリトンで、トリトンが持つトライデントの3つの刃先が3つのキーにつながる
- リスナーの mutex とイベントは `Local\herdr-triton-listener`、`-wake`、`-stop` とする。プログラムではなくプラグインの名前に基づく
- ファームウェアとプロトコルはプログラムの名前を持たない

## Consequences

- ホスト側のプログラムがまだ `bridge` の環境では、`trident` を入れる前に `.plugin/bin/bridge stop` が要る。古い `bridge.exe` と `bridge.log` は使われないまま残る
- 「triton」と「trident」は見た目が似ている。README で構図を一度描き、似ていることが1つの絵として読めるようにする

## Alternatives

- **`triton`**: リポジトリやプラグインと揃うが、キーに届く部分を呼ぶ名前が無くなり、2つのものを1つの名前で呼ぶことになる
- **`herdr-triton`**: 分かりやすいが、コマンドのたびに打つには長い
- **`bridge`**: 常駐プロセスが何なのか誰にも分からない
