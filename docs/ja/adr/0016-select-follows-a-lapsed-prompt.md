# 0016. herdr が blocked と報告しなくなったプロンプトでも Select を効かせる

[English](../../adr/0016-select-follows-a-lapsed-prompt.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Claude Code の質問プロンプトは、選択肢の後に「Type something.」を、区切り線の下に「Chat about this」を並べる。
ハイライトが「Type something.」にあると、プロンプトは画面に残ったまま、herdr 0.9.3 はその pane を `blocked` ではなく `idle` と報告する。
`blocked` でない pane は `queue` に無く、承認可能でもない。承認可能な pane にだけ効くキーパッドでは、ハイライトをその選択肢から動かせず、「Chat about this」へも元の選択肢へも行けない。キーボードの矢印キーでは動く。

Approve で確定してよいかは画面に依らない（ADR 0003）。
Select は、ハイライトをどこへ動かすかを画面を読んで決める（ADR 0010）。

## Decision

- pane が `idle` になったために `queue` から外れた要素は、その pane が `idle` の間、lapsed として残す。確定を `sent` に記録した要素は残さない。そのプロンプトには答えたからである
- Select は、フォーカス中の lapsed の pane に対して、画面にハイライト付きの番号リストがあるときだけ、blocked の pane と同じようにハイライトを動かす。無ければ赤く光る
- Approve は lapsed の pane に効かせず、LED にも出さない。どちらも画面に依ることになり ADR 0003 に反するうえ、LED は 250 ms ごとに画面を読む必要が出る

## Consequences

- Select で「Type something.」からハイライトを外し、「Chat about this」へも元の選択肢へも動かせる。herdr が再び `blocked` と報告すると、その pane は新しい `state_change_seq` で `queue` に戻る
- 「Type something.」で Approve を押すと赤く光る。その選択肢の確定には入力が要り、キーパッドでは入力しない
- idle のエージェントに矢印キーが届くのは、その画面にハイライト付きの番号リストがある間だけ。エージェントが自分の出力に `❯` と番号を付けたリストを出した場合も、それに当たる

## Alternatives

- **既知の問題として書き、キーボードに任せる**: キーパッドを使うと、その選択肢から動けなくなる。キーパッドが避けるはずの場面そのもの
- **ハイライト付きのリストがあるフォーカス中の idle の pane をすべて approvable とする**: Approve が画面に依ることになり、ADR 0003 に反する
- **herdr に `blocked` と報告し続けてもらう**: herdr の検出は上流にあり、このプロジェクトは上流に PR や機能要望を出さない
