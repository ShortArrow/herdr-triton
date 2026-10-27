# 0018. Jump は herdr の agents ペインの並び順に従う

[English](../../adr/0018-jump-follows-the-agents-pane.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

Jump は、承認待ち、作業を終えたエージェント、全エージェントのうち1つの段を選び、その中で次のエージェントへ移る（ADR 0011）。
ユーザーが見ているのは herdr の agents ペインで、ペインは見出しで切り替える `grouped` と `priority` の2つのモードで一覧を並べる。
Jump の「次」がほかの順に従うと、一覧の上を飛び回るように見える。`priority` モードでは一番新しいプロンプトが一番上に来る。

herdr 0.9.3（`src/client/shell/agent_sidebar.rs`）はペインを次のように並べる。

- `grouped`: `agent.list` の順。ワークスペース、タブ、pane の順になる
- `priority`: ステータス（`blocked`、`done`、`working`、`idle`、`unknown`）の順、次に `state_change_seq` の大きい順。安定ソートなので、並びが決まらないものは `agent.list` の順のまま

モードを報告する API は無い。
クライアントは、見出しがクリックされたあと、モードを `agent_panel_sort` として自分の設定ファイル `<herdr state>/client-shell/local-<hash>.json` に持つ。
それまでは herdr の `config.toml` の `[ui] agent_panel_sort` で決まる。
ハッシュはクライアントソケットのパスの FNV-1a 64 である。

## Decision

- 段の中では、Jump はペインの順で次のエージェントへ移る
- trident は Jump が押されるたびにモードを読む。クライアントの設定ファイル、次に herdr の `config.toml` の順で探し、どちらにも無ければ `grouped` とする
- ファイルが無い、読めない、または `spaces`、`workspaces`、`priority` 以外の値を持つときは、そのファイルは何も言っていないものとして次で決める
- `queue` は順序を持たない

## Consequences

- `priority` モードでは、Jump は一番長く待っているプロンプトではなく、一覧の表示どおり一番新しいプロンプトへ先に移る
- ペインの並び替えは、trident を再起動しなくても次の Jump から効く
- trident が頼るのは herdr の API ではなく herdr 自身のファイルである。herdr がそれを移したり変えたりすると、Jump はエラーを出さずに `config.toml`、そして `grouped` に戻る
- クライアントが複数あればそれぞれが自分のファイルを持つ。trident が読むのは、herdr 自身のクライアントが使うローカルのクライアントソケットのものである
- フックが `agent.view.set` で設定したビューはペインを並べ替えるが、Jump は並べ替えない

## Alternatives

- **trident が各エージェントを最初に blocked と見た順、古いものから**: 一番古いプロンプトが先に来るが、Jump の順がユーザーの読む一覧と食い違う
- **trident の `config.toml` に `jump_order` を置く**: herdr のファイルに頼らずに済むが、ペインの見出しをクリックするたびに手で書き換える必要があり、2つがずれていく
- **並び順を報告する API を herdr に求める**: upstream の作業になり、このプロジェクトでは依頼しない
