# 0018. Jump は herdr の agents ペインの並び順に従う

[English](../../adr/0018-jump-follows-the-agents-pane.md) | 日本語

- Status: Accepted
- Base: [`309db6e`](https://github.com/ShortArrow/herdr-triton/commit/309db6ecf4e15a358ded32282488ad4812a79b5d)

## Context

Jump は、承認待ちのエージェントを trident が最初に blocked と見た順に、作業を終えたエージェントとそれ以外を `agent.list` の順に巡回していた（ADR 0011）。
herdr の agents ペインは、見出しで切り替える `grouped` と `priority` の2つのモードで一覧を並べる。
`priority` モードでは一覧の一番上に一番新しいプロンプトが来るのに、Jump は一番古いものへ先に移ったので、一覧の上を飛び回るように見えた。

herdr 0.9.3（`src/client/shell/agent_sidebar.rs`）はペインを次のように並べる。

- `grouped`: `agent.list` の順。ワークスペース、タブ、pane の順になる
- `priority`: ステータス（`blocked`、`done`、`working`、`idle`、`unknown`）の順、次に `state_change_seq` の大きい順。安定ソートなので、並びが決まらないものは `agent.list` の順のまま

モードを報告する API は無い。
クライアントは、見出しがクリックされたあと、モードを `agent_panel_sort` として自分の設定ファイル `<herdr state>/client-shell/local-<hash>.json` に持つ。
それまでは herdr の `config.toml` の `[ui] agent_panel_sort` で決まる。
ハッシュはクライアントソケットのパスの FNV-1a 64 である。

## Decision

- Jump は ADR 0011 の段階を保ち、段階の中では `queue` や `agent.list` の順ではなく、ペインの順で次のエージェントへ移る
- trident は Jump が押されるたびにモードを読む。クライアントの設定ファイル、次に herdr の `config.toml` の順で探し、どちらにも無ければ `grouped` とする
- ファイルが無い、読めない、または `spaces`、`workspaces`、`priority` 以外の値を持つときは、そのファイルは何も言っていないものとして次で決める
- 順序を読むものがほかに無くなったので、`queue` は順序を持たない

## Consequences

- `priority` モードでは、Jump は一番長く待っているプロンプトではなく、一覧の表示どおり一番新しいプロンプトへ先に移る
- ペインの並び替えは、trident を再起動しなくても次の Jump から効く
- trident が頼るのは herdr の API ではなく herdr 自身のファイルである。herdr がそれを移したり変えたりすると、Jump はエラーを出さずに `config.toml`、そして `grouped` に戻る
- クライアントが複数あればそれぞれが自分のファイルを持つ。trident が読むのは、herdr 自身のクライアントが使うローカルのクライアントソケットのものである
- フックが `agent.view.set` で設定したビューはペインを並べ替えるが、Jump は並べ替えない

## Alternatives

- **trident の `config.toml` に `jump_order` を置く**: herdr のファイルに頼らずに済むが、ペインの見出しをクリックするたびに手で書き換える必要があり、2つがずれていく
- **一番長く待っているプロンプトを先にしたままにする**: Jump が独自の順を持ち続け、この記録が取り除く混乱が残る
- **並び順を報告する API を herdr に求める**: upstream の作業になり、このプロジェクトでは依頼しない
