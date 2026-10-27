# 0003. 承認はフォーカス中かつ blocked の pane にだけ送る

[English](../../adr/0003-approve-only-focused-blocked-pane.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

herdr の `blocked` は、承認 UI だけでなく質問 UI を検出した場合にも付く。
ユーザーが見ていないプロンプトにキーを送ると、質問に意図しない回答をしてしまう恐れがある。

herdr はこれらの UI を画面のポーリングで検出するので、ステータスはエージェントの実際の状態より遅れる。
承認した直後も pane はしばらく `blocked` のままで、そこでもう一度押すと、エージェントが次に出した画面にキーが入る。

## Decision

- Approve は、herdr がフォーカス中と返す pane が `blocked` のときだけ、その pane に対して働く。
  「Jump したかどうか」を別の状態としては持たない
- Approve は判断の直前にスナップショットを取り直す
- Approve がキーを送った pane は、`state_change_seq` が変わるまで再び承認可能にならない

## Consequences

- クライアントが1つの通常の構成では、承認する前に必ず画面で内容を確認することになる
- herdr のクライアントが複数接続されていると、herdr のフォーカスはどれかのクライアントで最後に選ばれた pane になり、ユーザーが見ている pane と一致するとは限らない
- Jump のあとでも、手動でフォーカスした場合でも、同じ条件で承認できる
- 同じプロンプトに対する2回目の押下は、herdr が変化を報告するまで何もしない
- Approve の LED も同じ条件から導ける

## Alternatives

- **キュー先頭を即座に承認する**: 速いが、質問 UI に誤って回答する危険がある
- **Jump 済みフラグを持つ**: ユーザーが手動でフォーカスを移したときにフラグをどう扱うかという状態が増え、herdr の実際のフォーカスとずれうる
- **`pane.read` で承認 UI かを判定する**: エージェントごとの画面判定を `trident` が抱えることになり、herdr の検出ロジックと二重になる。
  読み違えれば、ユーザーが答えるつもりのないプロンプトを確定してしまう
