# 0014. ホスト側のプログラムを trident と呼ぶ

[English](../../adr/0014-host-program-named-trident.md) | 日本語

- Status: Accepted
- Base: [`9374c56`](https://github.com/ShortArrow/herdr-triton/commit/9374c5698a877aee429227ef4e5c78be5aa6f3a8)

## Context

herdr とキーパッドをつなぐホスト側のプログラムは `bridge` と呼ばれていた。crate、実行ファイル、コマンド、ログのどれもその名前だった。
この語は役割を表すだけで、このプログラムを指していない。
タスクマネージャーには常駐する `bridge.exe` が出ていたが、herdr もキーパッドも名前に表れていなかった。文書でも「bridge」が、プログラムと役割の両方に使われていた。
アイコンにはすでに、先に3つのキーが付いた三叉の矛を持つトリトンが描かれている。

## Decision

- ホスト側のプログラムは `trident` とする。crate とそのディレクトリ、実行ファイル `trident.exe`、コマンド `trident run|hook|listen|stop|find-port`、ログ `trident.log`
- 名前はアイコンに合わせる。プラグインがトリトンで、トリトンが持つトライデントの3つの刃先が3つのキーにつながる
- 文書は、今の振る舞いを新しい名前で書く。ADR 0001〜0013 は書かれたまま残し、そこでの「bridge」はこのプログラムのこと
- リスナーの mutex とイベントは `Local\herdr-triton-listener`、`-wake`、`-stop` のまま。プログラムではなくプラグインの名前に基づいている
- ファームウェアとプロトコルは変えない

## Consequences

- この変更より前に入れた環境では、新しいプログラムを入れる前に `.plugin/bin/bridge stop` が要る。古い `bridge.exe` と `bridge.log` は使われないまま残る
- 「triton」と「trident」は見た目が似ている。README で構図を一度描き、似ていることが1つの絵として読めるようにする

## Alternatives

- **`triton`**: リポジトリやプラグインと揃うが、キーに届く部分を呼ぶ名前が無くなり、2つのものを1つの名前で呼ぶことになる
- **`herdr-triton`**: 分かりやすいが、コマンドのたびに打つには長い
- **`bridge` のまま**: 移行は要らないが、常駐プロセスが何なのか誰にも分からない
