# 0004. イベント購読ではなく agent.list のポーリングで状態を得る

[English](../../adr/0004-poll-agent-list.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

`trident` には、全エージェントのステータスと、どの pane にフォーカスがあるかが要る。
herdr 0.9.1（commit `0d5d6f1`）では次のとおりである。

- `pane.agent_status_changed` の購読には `pane_id` が必須で、全 pane をまとめて購読する手段は無い。
  存在しない pane を1つでも指定すると、リクエスト全体が失敗して接続が切られる
- 1接続で送れるリクエストは1つだけで、購読した接続はそれ以降イベントを流すだけになる
- タブやワークスペースを閉じたときや、ワークスペースをまたいで pane を移動したときは `pane.closed` が出ない
- `agent.list` は全エージェントについて `pane_id`, `agent`, `agent_status`, `focused`, `state_change_seq` を返す
- herdr 自身も Claude Code と Codex のステータスを 300 ms ごとの画面ポーリングで検出している

## Decision

`trident` は herdr の状態を `agent.list` から得る。
herdr をポーリングしている間は 250 ms ごとに、またリクエストを送った直後に呼ぶ。
前回のスナップショットとの差分から状態を作り、`events.subscribe` は使わない。

## Consequences

- pane がどんな理由で消えても、次のスナップショットで `queue` から外れる
- 接続時の初期化を別に用意する必要がなく、購読と一覧取得の順序の競合も起きない
- 1接続1リクエストなので、ポーリングしている間は 250 ms ごとに接続を1本開く
- ステータスの変化は、イベントで受けるより最大 250 ms 遅れて `trident` に届く。
  herdr 自身の検出の 300 ms はこれとは別にかかる
- 2回のスナップショットの間にステータスが変わって戻った場合は、`state_change_seq` の変化でしか分からない

## Alternatives

- **`pane.agent_detected` と `pane.created` を見て pane ごとに購読を張る**: 遅延は小さいが、エージェントごとに接続が1本要る。
  一覧取得と購読の間に pane が閉じるとリクエストごと失敗し、タブやワークスペースを閉じたときの後始末も別に要る
- **herdr のプラグインのイベントフックからステータスを得る**: herdr の中で全イベントに対して動くが、デバイスを持つプロセスへ届ける経路が結局別に要る
