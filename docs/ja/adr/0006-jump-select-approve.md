# 0006. Jump でエージェントの間を移り、Select でハイライトを動かし、Approve で確定する

[English](../../adr/0006-jump-select-approve.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Claude Code と Codex は、承認プロンプトを選択肢のリストとして出す。
ハイライトは矢印キーで動き、`enter` で確定する。
拒否したり「今後も許可」を選んだりするには、確定の前にハイライトを動かす。

Codex の承認画面は、ショートカットのキーを先に処理し、それ以外のキーはリスト表示に渡す。
リストは矢印キーで移動し、`enter` で確定する（openai/codex `1cc7e23`、`codex-rs/tui/src/bottom_pane/approval_overlay.rs`）。
Codex は `y` も受け付けるが、`y` はハイライトに関係なく「yes」を選ぶ。

## Decision

- Jump は herdr のフォーカスを別のエージェントへ移し、巡回する
- Select は、フォーカス中の pane にそのエージェントの選択キー（`down`）を送り、ハイライトを次の選択肢へ動かす
- Approve は、フォーカス中の pane にそのエージェントの確定キー（`enter`）を送り、ハイライト中の選択肢を確定する
- Approve は、フォーカス中の pane が承認可能であること（ADR 0003）を条件にする。
  herdr が変化を報告するまでその pane が再び承認可能にならない規則もこれに含まれる。
  Select はフォーカス中の pane のプロンプトにだけ働き、少なくとも Approve が働けるときには働く
- キーは左から Jump、Approve、Select の順に並ぶ

## Consequences

- キーごとに役割が1つに決まる。
  Jump は pane の間、Select はプロンプトの中を動き、Approve が確定する
- プロンプトのどの選択肢もキーパッドから選べ、質問 UI にも意図して答えられる
- Codex も `enter` で確定するので、どのエージェントでも Select で Approve の結果が変わる
- Jump の効果はフォーカスの状態によらないので、押せば必ず先へ進む

## Alternatives

- **Jump、Approve、Next の構成で、Approve はエージェントごとの承認キーを送る**: キーパッドからは既定の選択肢しか受け入れられず、拒否や「今後も許可」にはキーボードが要る
- **Jump は先頭へ移り、キューの pane にフォーカス済みならハイライトを動かす。Next は残す**: 次の pane へ直接移る手段は残るが、Jump の効果がフォーカスの状態で変わる。
  後ろの要素を見ているときに、Jump で先頭へ戻れなくなる
- **Codex の承認キーを `y` のままにする**: `y` はハイライトに関係なく「yes」を選ぶので、Select で選んでも Approve の結果が変わらない
- **Approve の長押しで拒否する**: 1つのキーの意味が増える。拒否の選択肢へは Select で届く
