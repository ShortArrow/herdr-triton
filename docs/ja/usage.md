# 使い方

[English](../usage.md) | 日本語

## キーと LED

| キー | 動作 | LED |
|---|---|---|
| Jump（左） | 次の承認待ちのエージェントへ。無ければ次の作業を終えたエージェントへ、それも無ければ次のエージェントへ移る。「次」は herdr の agents ペインの一覧の順で、並びが `grouped` でも `priority` でもそれに従う。0.5 秒押し続けると、herdr のウィンドウを前面にも出す | 承認待ちが1つならオレンジ、2つ以上なら赤寄りのオレンジで呼吸、作業を終えたエージェントがあれば緑の呼吸、それ以外は白の呼吸 |
| Approve（中） | フォーカス中のプロンプトで、ハイライト中の選択肢を確定する | フォーカス中の pane を確定できるとき緑 |
| Select（右） | ハイライトを次の選択肢へ動かす。最後の次は最初 | フォーカス中の pane を確定できるとき青 |

押したキーは、herdr がリクエストを受け付ければ白く、受け付けなければ赤く1回光る。
Approve が効くのは、フォーカス中の pane が承認待ちのときの、そのプロンプトだけ。先に Jump で移る。

キーは前面のウィンドウに関係なく効くので、承認だけならゲームやブラウザを前面に置いたままにできる。
入力が要るエージェントでは Jump を押し続ける。herdr を表示している端末が前面に出て（最小化していれば元に戻り）、Jump が白く光る。ウィンドウが見つからなければ赤く光る。

| 3つの LED | 意味 |
|---|---|
| 薄い白 | trident が動いていない |
| 赤の点滅 | trident が herdr につながらない |
| 赤の点灯 | herdr が 0.9.1 より古い（`trident run` のときだけ。プラグインの trident は止まる） |

## キーの配置を変える

次のコマンドが表示するディレクトリに `config.toml` を置く。

```sh
herdr plugin config-dir shortarrow.herdr-triton
```

```toml
layout = ["select", "jump", "approve"]   # 左、中、右
```

`jump`、`approve`、`select` を1回ずつ書く。
`select_hold = "up"` を足すと、Select を 0.5 秒押し続けたときにハイライトが上へ動く。そのとき短い Select は、離したときに動く。
そのあと `.plugin/bin/trident stop` を実行すると、次にエージェントのステータスが変わったときに、新しい配置で herdr が trident を起動する。
キーは自分の LED を連れて移る。
使えないファイルなら既定の配置のまま動き、理由は `trident.log` に書かれる。

## プラグインを使わない

```sh
cargo run --release -p trident --bin trident -- run
```

で、止めるまでキーパッドを受け持つ。名前付きの herdr セッションなら `--session <name>` を付ける。

## キーパッドを直接操作する

キーパッドは、USB シリアル番号が `TRITON-` で始まる USB シリアルデバイスで、1行に1つの SCPI のテキストコマンドに答える。
先に trident を止めて（`.plugin\bin\trident.exe stop`）、どのシリアルターミナルや VISA ライブラリからでも次のように操作できる。

```text
*IDN?                     -> ShortArrow,herdr-triton,TRITON-…,<version>
LED1 #FF8000,BREathe      左の LED をオレンジで呼吸させる
LED:ALL #000000,OFF,#00FF00,SOLid,#0000FF,BLINk
KEY:EVENt?                -> LEFT,DOWN など、または NONE
SYSTem:ERRor?             -> 0,"No error"
```

コマンド、モード、エラーの一覧は仕様にある（[プロトコル](spec.md)）。

## 困ったとき

- **LED が薄い白のまま。** herdr がまだ trident を起動していない。herdr を再起動するか、エージェントのステータスが変わるのを待つ。プラグインの状態ディレクトリ（Windows では `%LOCALAPPDATA%\herdr\plugins\shortarrow.herdr-triton\`）の `trident.log` に、起動、受け持つポート、止まった理由が記録される
- **`cargo install` が `trident.exe` を置き換えられない。** trident が動いている。先に `.plugin/bin/trident stop` を実行する
- **シリアルターミナルでポートを開けない。** trident が使っている。先に `.plugin/bin/trident stop` を実行する
- **`cargo keypad` がボードを見つけない。** 初回は、ボードが BOOT モードで、Zadig で WinUSB を入れてある必要がある（README を見る）
- **Jump でエージェントではなく workspaces ペインに飛ぶ。** herdr の既知の問題。herdr の agents ペインが全エージェントを表示しきれずスクロールバーが出ていると、見切れたエージェントに herdr がフォーカスを移せない。エージェントのペインへのフォーカスは移るが、サイドバーのフォーカスは workspaces ペインに落ちる。agents ペインを全エージェントが収まる高さにする
- **他の機械のエージェントで Approve と Select が赤く光る。** herdr は保存した機械（`herdr machine`）のエージェントをローカルのものと並べて表示するが、`trident` が届くのはこの機械の herdr サーバーだけ。キーが効くのはローカルのエージェントだけ

## 開発用

| コマンド | 用途 |
|---|---|
| `cargo probe` | 各 LED を決まった色で点け、キーの押下を報告するファームウェアを書き込む。どのキーと LED がどこにあるかを確かめられる |
| `cargo run -p trident --bin herdr_probe` | herdr のバージョンとエージェントを表示する。`read select_id <pane>` はプロンプトのハイライト中の選択肢を表示する |
