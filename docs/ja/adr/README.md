# 設計判断の記録（ADR）

[English](../../adr/README.md) | 日本語

- [0000. 記録は今有効な判断を書き、判断が変われば書き換える](0000-records-state-the-decision-in-force.md)
- [0001. キーボードは HID ではなくシリアルでホストのブリッジに繋ぐ](0001-serial-bridge-architecture.md)
- [0002. ファームウェアと trident を Rust で書き、drooling で書き込む](0002-rust-with-drooling.md)
- [0003. 承認はフォーカス中かつ blocked の pane にだけ送る](0003-approve-only-focused-blocked-pane.md)
- [0004. イベント購読ではなく agent.list のポーリングで状態を得る](0004-poll-agent-list.md)
- [0005. CDC-ACM 上で postcard のメッセージを COBS で区切る](0005-postcard-cobs-over-cdc-acm.md)（Withdrawn）
- [0006. Jump でエージェントの間を移り、Select でハイライトを動かし、Approve で確定する](0006-jump-select-approve.md)
- [0007. 1つの trident は herdr のセッションを1つだけ受け持つ](0007-one-session-per-bridge.md)
- [0008. herdr プラグインが bridge を起動し、することが無くなれば終了させる](0008-listener-started-by-plugin-hooks.md)（Withdrawn）
- [0009. CDC-ACM 上で SCPI 風のテキストを使う](0009-scpi-text-protocol.md)
- [0010. Select は画面を読んで最初の選択肢へ折り返す](0010-select-wraps-by-reading-the-screen.md)
- [0011. Jump は、承認待ちが無ければ完了報告へ、それも無ければ全エージェントへ移る](0011-jump-falls-back-to-done-then-every-agent.md)
- [0012. リスナーは常駐し、待つものが無い間は静かにする](0012-resident-listener-quiet-when-idle.md)
- [0013. キーの配置はプラグインの設定ディレクトリで決める](0013-key-layout-in-plugin-config.md)
- [0014. ホスト側のプログラムを trident と呼ぶ](0014-host-program-named-trident.md)
- [0015. Jump の長押しで herdr のウィンドウを前面に出す](0015-hold-jump-to-raise-herdr.md)
- [0016. herdr が blocked と報告しなくなったプロンプトでも Select を効かせる](0016-select-follows-a-lapsed-prompt.md)
- [0017. Select の長押しでハイライトを上へ動かす](0017-hold-select-to-move-up.md)
- [0018. Jump は herdr の agents ペインの並び順に従う](0018-jump-follows-the-agents-pane.md)
