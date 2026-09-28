# 0006. Jump で承認待ちを巡回し、Select でハイライトを動かし、Approve で確定する

- Status: Accepted
- Date: 2026-09-28
- Amends: 0003（Approve が送るキー）

## Context

Claude Code と Codex は、承認プロンプトを選択肢のリストとして出す。
ハイライトは矢印キーで動き、`enter` で確定する。
Jump・Approve・Next の構成では、キーパッドからできるのは既定の選択肢を受け入れることと pane の移動だけだった。
拒否したり「今後も許可」を選んだりするには、キーボードに手を移す必要があった。

Codex の承認画面は、ショートカットのキーを先に処理し、それ以外のキーはリスト表示に渡す。
リストは矢印キーで移動し、`enter` で確定する（openai/codex `1cc7e23`、`codex-rs/tui/src/bottom_pane/approval_overlay.rs`）。

## Decision

- 左 Jump: `queue` の先頭か、フォーカス中の要素の次（末尾なら先頭）にフォーカスする
- 中 Approve: フォーカス中の pane に、そのエージェントの確定キー（`enter`）を送る
- 右 Select: フォーカス中の pane に、そのエージェントの選択キー（`down`）を送る

Approve と Select はどちらも、フォーカス中の pane が承認可能であること（ADR 0003）を条件にする。
`sent` の規則もこれに含まれる。

## Consequences

- キーごとに役割が1つに決まる。
  Jump は pane の間、Select はプロンプトの中を動き、Approve が確定する
- プロンプトのどの選択肢もキーパッドから選べるようになり、質問 UI にも意図して答えられる
- Approve はハイライト中の選択肢を確定するので、Codex の既定のキーは `y` から `enter` に変わる
- `queue` の先頭に戻るには、フォーカス中の要素より後ろにある件数だけ Jump を押す
- 長押しで拒否する案は不要になる

## Alternatives

- **Jump は先頭へ移り、キューの pane にフォーカス済みならハイライトを動かす。Next は残す**: 次の pane へ直接移る手段は残るが、Jump の効果がフォーカスの状態で変わる。
  後ろの要素を見ているときに、Jump で先頭へ戻れなくなる
- **Codex の承認キーを `y` のままにする**: `y` はハイライトに関係なく「yes」を選ぶので、Select で選んでも Approve の結果が変わらない
