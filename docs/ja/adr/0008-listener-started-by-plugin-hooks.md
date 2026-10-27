# 0008. herdr プラグインが bridge を起動し、することが無くなれば終了させる

[English](../../adr/0008-listener-started-by-plugin-hooks.md) | 日本語

- Status: Withdrawn
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

この記録は、プラグインのフックがホストのプログラムをリスナーとして起動し、待つものが無くなって 5 秒で終了させると決めていた。
リスナーの起動と動き方は今は [0012](0012-resident-listener-quiet-when-idle.md) が決めており、その Alternatives にこの設計を載せている。
