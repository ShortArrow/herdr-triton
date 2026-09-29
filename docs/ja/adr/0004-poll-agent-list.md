# 0004. イベント購読ではなく agent.list のポーリングで状態を得る

[English](../../adr/0004-poll-agent-list.md) | 日本語

- Status: Accepted
- Date: 2026-09-28

## Context

`bridge` には、全エージェントのステータスと、どの pane にフォーカスがあるかが要る。
herdr 0.9.1（commit `0d5d6f1`）では次のとおりになっている。

- `pane.agent_status_changed` の購読には `pane_id` が必須で、全 pane をまとめて購読する手段は無い。
  存在しない pane を1つでも指定すると、リクエスト全体が失敗して接続が切られる
- 1接続で送れるリクエストは1つだけで、購読した接続はそれ以降イベントを流すだけになる
- タブやワークスペースを閉じたときや、ワークスペースをまたいで pane を移動したときは `pane.closed` が出ない
- `agent.list` は全エージェントについて `pane_id`, `agent`, `agent_status`, `focused`, `state_change_seq` を返す
- herdr 自身も Claude Code と Codex のステータスは 300 ms ごとの画面ポーリングで検出している

## Decision

`bridge` は `agent.list` を 250 ms ごとと、リクエストを送った直後に呼び、前回のスナップショットとの差分から状態を作る。
`events.subscribe` は使わない。

## Consequences

- pane がどんな理由で消えても、次のスナップショットで `queue` から外れる
- 接続時の初期化を別に用意する必要がなく、購読と一覧取得の順序の競合も起きない
- 1接続1リクエストなので、250 ms ごとに接続を1本開く
- ステータスの変化は、イベントで受けるより最大 250 ms 遅れて `bridge` に届く。
  herdr 自身の検出の 300 ms はこれとは別にかかる
- 2回のスナップショットの間にステータスが変わって戻った場合は、`state_change_seq` の変化でしか分からない

## Alternatives

- **`pane.agent_detected` と `pane.created` を見て pane ごとに購読を張る**: 遅延は小さいが、エージェントごとに接続が1本要る。
  一覧取得と購読の間に pane が閉じるとリクエストごと失敗し、タブやワークスペースを閉じたときの後始末も別に要る
- **herdr のプラグインのイベントフック**: herdr の中で全イベントに対して動くが、`bridge` が herdr のプラグイン API に依存し、デバイスへ届ける経路も結局別に要る
