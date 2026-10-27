# 0001. キーボードは HID ではなくシリアルでホストのブリッジに繋ぐ

[English](../../adr/0001-serial-bridge-architecture.md) | 日本語

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

3キーで herdr のエージェントを操作し、各キーの LED に herdr の状態を表示する。
LED の表示には、ホストからデバイスへ情報を送る経路が要る。
herdr の操作は Socket API（改行区切り JSON）で行える。

## Decision

デバイスは USB CDC シリアルで、キーの押下と解放を伝え、指示どおりに LED を点けるだけにする。
ホスト側の常駐プロセス `trident` が herdr の Socket API に接続し、状態の保持、キー入力から herdr へのリクエストへの変換、LED 表示の決定をすべて担う。

## Consequences

- どのウィンドウにフォーカスがあっても、キーが誤入力にならない
- 振る舞いの変更は `trident` だけで済み、ファームウェアの書き換えが少ない
- 振る舞いを `trident` の純粋な状態遷移としてホスト上でテストできる
- `trident` が動いていないと、デバイスは何もしない

## Alternatives

- **HID キーボードとして F13〜F15 を送り、herdr のキーバインドで受ける**: LED へ状態を返す経路が別に要る。キー入力がフォーカス中のアプリに届くため、herdr 以外にフォーカスがあるときに効かないか、誤入力になる
- **判断をファームウェアに持たせる**: デバイスは herdr のソケットに直接触れないため、結局ホスト側の中継が要る。振る舞いを変えるたびに書き込みが必要になり、テストも実機に依存する
