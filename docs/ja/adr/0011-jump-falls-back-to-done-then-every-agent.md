# 0011. Jump は、承認待ちが無ければ完了報告へ、それも無ければ全エージェントへ移る

[English](../../adr/0011-jump-falls-back-to-done-then-every-agent.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

承認待ちだけへ移る Jump は、承認待ちをすべて片づけると、することが無くなる。
一方 herdr には、作業を終えてまだ見られていないエージェント（`done`）が残っている。
herdr は、`agent.focus` で画面に出したエージェントを確認済み、つまり `idle` にする（herdr `0d5d6f1`、`src/app/actions.rs`）。
エージェントのいない pane へは、キーパッドでもキーボードでも同じ手間で移れる。

## Decision

Jump は、空でない最初の段を選び、その中を `agent.focus` で巡回する。
段の順は、承認待ちのエージェント、`done` のエージェント、全エージェントとする。

## Consequences

- `done` のエージェントは表示すると `idle` になるので、Jump を押すたびに1つずつ消化される
- Jump の LED で段を見分けられる。承認待ちが1つならオレンジ、2つ以上なら赤寄りのオレンジ、`done` は緑、全エージェントは白
- どの段も `trident` がすでにポーリングしているスナップショットを使うので、Jump のリクエストはどの段でも1回で済む

## Alternatives

- **承認待ち、`done`、全エージェントを1本の輪にする**: 押すたびに先へ進むので、ほかのエージェントへ向かう途中で承認待ちを通り過ぎてしまう
- **最後の段で全ワークスペースを巡回する**: エージェントのいないワークスペースにも止まるが、そこへはキーボードでも同じ手間で移れる。押すたびに `workspace.list` と `workspace.focus` が要る
