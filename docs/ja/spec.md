# herdr-triton 仕様

[English](../spec.md) | 日本語

herdr のエージェントが承認待ち（`blocked`）になったら、RP2040-Keyboard-3 の3キーでその pane へ移動し、承認し、次の承認待ちへ進む。
各キーの RGB LED には、承認待ちの件数と、いま Approve を押せば承認が送られるかを出す。

## 構成

```
[herdr] --プラグインのイベント / startup--> [bridge hook]   リスナーは常に1つ
[RP2040-Keyboard-3] <--USB CDC, SCPI--> [bridge] --local socket--> [herdr]
   キーイベントを溜め、LED を描く          agent.list と KEY:EVENt? をポーリング
```

| コンポーネント | 責務 |
|---|---|
| `firmware` | チャタリングを除いたキーイベントを溜め、`KEY:EVENt?` に答える。指示どおりに LED を描く。ホストがいないことを検知する。herdr に関する判断は持たない |
| `bridge` | herdr の全エージェントの状態とデバイスのキーイベントを定期取得して状態を保持し、キー入力を herdr へのリクエストに変換し、LED 表示を決める |
| `protocol` | `firmware` と `bridge` が共有するメッセージ型と、その SCPI のテキスト表現。`no_std` |
| `keypad` | firmware のうちハードウェアに依存しない部分。`no_std` |
| プラグイン | `herdr-plugin.toml`。herdr の startup とイベントフックから `bridge hook` を起動する |

デバイスはシリアル回線の先にある IO 機器で、HID キーボードではない。
キー入力はシリアル経由でのみ `bridge` に届く。

## リスナーの生存期間

常駐する `bridge listen` が、自分を起動した herdr サーバーが動いている間、キーパッドを受け持つ（ADR 0012）。

| Given | When | Then |
|---|---|---|
| 任意 | herdr がプラグインの `startup` を実行するか、`pane.agent_status_changed` を発行する | herdr が `bridge hook` を起動する |
| リスナーが名前付きミューテックスを持っている | `bridge hook` が起動する | リスナーの起床イベントを発行して終了する |
| リスナーがいない | `bridge hook` が起動する | キーパッドのポートを探して `bridge listen --port <port>` を起動し、終了する。キーパッドが無ければそのまま終了する |
| リスナー | 起動した時点でミューテックスがすでに持たれている | 終了する |
| リスナー、静か | 起床イベントが発行されるか、キーが押される | スナップショットを取って動いている状態になる。キーはそのスナップショットで処理する |
| リスナー、動いている | `queue` も `done` も 5 秒続けて空 | 静かな状態になる |
| リスナー、動いている | herdr に 5 秒続けて届かないか、herdr のバージョンが対応外 | 終了する |
| リスナー、静か | herdr のエンドポイントが 5 秒続けて無い | 終了する |
| リスナー | `bridge stop` が停止イベントを発行する | 終了する |
| リスナー | キーパッドが故障するか見えなくなる | ポートを閉じ、キーパッドが戻るまで 1 秒ごとに `bridge find-port` を実行する |

| 状態 | herdr への問い合わせ | `KEY:EVENt?` | LED |
|---|---|---|---|
| 静か | 無し。1 秒ごとに、接続せずに herdr のエンドポイントがあることを確かめる | 250 ms ごと | Jump の LED だけ白で呼吸、ほかは消灯 |
| 動いている | 250 ms ごとと、リクエストの直後に `agent.list` | 20 ms ごと | 「LED」の節のとおり |

Windows では、ミューテックス、起床イベント、停止イベントをユーザーのセッションごとの名前で作る（`Local\herdr-triton-listener`、`-wake`、`-stop`）。
herdr は、フックの標準出力と標準エラーが閉じるまでそのフックを実行中と数えるので、`bridge hook` は子プロセスを起動する前に、自分の標準ハンドルの `HANDLE_FLAG_INHERIT` を外す。
リスナーは `DETACHED_PROCESS` と `CREATE_NEW_PROCESS_GROUP` を付けて起動し、作業ディレクトリをプラグインの状態ディレクトリにして、そこの `bridge.log` にログを書く。
`bridge find-port` は `CREATE_NO_WINDOW` を付けて起動する。コンソールの無いプロセスからコンソールのプログラムを起動すると、コンソールのウィンドウが開くため。

リスナーを最初に起動した herdr のセッションが、そのサーバーが止まるまでキーパッドを使い続ける。
`bridge run` は、静かな状態も終了条件も持たない動いている状態のループで、プラグインを使わないときのためのもの。
リスナーがいない間、デバイスは `NoHost` になり、薄い白を点ける。

### メモリ

リスナーは、タスクマネージャーの「メモリ」列（アクティブなプライベート ワーキング セット）で 1 MB 以下を保つ。1 時間動かした間の最大値で判定する。
そのため、C ランタイムを静的にリンクし、herdr との通信に `interprocess` ではなく `std` の named pipe を使い、herdr の応答を `serde_json::Value` を経由せず型に直接読み込み、シリアルポートの列挙を自分では行わない。
`bridge` の実行ファイルは、ポートの列挙と乱数の種を使うハッシュだけが呼ぶ `setupapi.dll`、`cfgmgr32.dll`、`advapi32.dll`、`bcryptprimitives.dll` を遅延読み込みにするので、リスナーはどれも読み込まない。自分で使う表は、種の要らない順序付きの表にする。

## herdr

### 前提

- herdr 0.9.1 以降。
  0.9.0 以前は `agent.focus` で接続中のクライアントの表示が切り替わらず、手動のフォーカス移動も通知されない
- `bridge` は herdr と同じ OS ユーザーで動かす

`bridge` は接続時に `ping` を送り、返ってきた `version` が 0.9.1 未満なら `Incompatible` とする。

### 接続

herdr の API は1接続につき1リクエストしか受け付けない。
サーバーは最初の1行を読んでそれに応答するだけなので、`bridge` はリクエストのたびに接続を開く。
各リクエストの期限は 2 秒で、間に合わなければ失敗として扱う。
ループが動いている状態になるたび、また herdr に再び届くようになったら、`bridge` は次のスナップショットの前に `ping` を送る。
対応外のバージョンが返ったら、リスナーは終了し、`bridge run` は `Incompatible` を表示する。

1つの `bridge` が受け持つ herdr のセッションは1つだけとする（ADR 0007）。
`bridge --session <name>` でセッションを名前で選び（既定のセッションは `default`）、herdr 自身の `--session` と同じく環境変数より優先する。
`--session` が無ければ、ソケットのパスは herdr と同じ順序で決める。

1. `HERDR_SOCKET_PATH`
2. `HERDR_SESSION` があれば `<config>/sessions/<name>/herdr.sock`
3. `<config>/herdr.sock`

`<config>` は、`$XDG_CONFIG_HOME` があれば `$XDG_CONFIG_HOME/herdr`、無ければ Windows で `%APPDATA%\herdr`、それ以外で `~/.config/herdr`。
Windows ではこのパス文字列が named pipe の名前 `\\.\pipe\<path>` になり（herdr が使う interprocess の `GenericNamespaced` の対応付け）、パスにあるファイルは目印でしかない。
`bridge` はこれを `std` で開き、パイプのインスタンスがすべて使用中なら `WaitNamedPipeW` で空くのを待つ。
静かなリスナーは、接続しない `WaitNamedPipeW` だけでパイプを確かめる。インスタンスが空いているか、待ちがタイムアウトすれば、パイプはある。
Unix ではソケットのファイルがあることを確かめる。

herdr は自分の pane の中で `HERDR_SOCKET_PATH` を設定する。
そのため herdr の pane から起動した `bridge` は、`HERDR_SESSION` の値に関係なく、その pane のセッションにつながる。

### 使うリクエスト

| リクエスト | 用途 |
|---|---|
| `ping` | 接続時のバージョン確認 |
| `agent.list` | 全エージェントの状態。`pane_id`, `agent`, `agent_status`, `focused`, `state_change_seq` を使う |
| `agent.focus {target}` | Jump。ワークスペースとタブを切り替えて pane にフォーカスする。応答は `ok` ではなく `agent_info` |
| `agent.send_keys {target, keys}` | Approve と Select。pane にいるエージェントが入れ替わっていれば herdr が拒否する |
| `agent.read {target, source: "visible"}` | リストが折り返さないエージェントでの Select。ハイライトの位置を知るために画面を読む |

`events.subscribe` は使わない（ADR 0004）。

## 配置

`bridge` はデバイスを挿したマシンで動かし、そこから herdr のソケットに届く必要がある。

| herdr の場所 | bridge の場所 | herdr への経路 |
|---|---|---|
| Windows | Windows | named pipe |
| 同じマシンの Linux / macOS | 同じマシン | Unix socket |
| WSL 2 | WSL 2。デバイスは `usbipd attach --wsl --auto-attach` で渡す | Unix socket |
| SSH 先のリモート | 手元のマシン | `ssh -L` で転送した Unix socket |

いまは Windows だけに対応し、プラグインは `platforms = ["windows"]` と宣言する。
ほかの行は、リスナーに Linux と macOS での代わりができたときに `bridge` を動かす想定の場所で、どれも未検証。

## ハードウェア

| 項目 | 値 |
|---|---|
| MCU | RP2040。水晶は 12 MHz |
| フラッシュ | W25Q16JV, 2 MB（`BOOT_LOADER_W25Q080`） |
| キー | スイッチは GND へ落ちるだけなので内部プルアップを使い、押下で Low |
| RGB LED | WS2812B ×3。データ線は GP18、**RGB 順**、L1 → L2 → L3 の順に数珠つなぎ |
| GP25 | 単色の赤 LED。使わない |
| USB | マルチプレクサ経由の Type-C ×2。挿すのは片方だけ |

キーを横一列に並べて上から見たときの対応:

| 位置 | キーの GPIO | LED |
|---|---|---|
| 左 | GP14 | L1（数珠つなぎの先頭） |
| 中 | GP13 | L2 |
| 右 | GP12 | L3 |

LED の色順は RGB で、Waveshare の FastLED デモが指定し `ws2812-pio` が送る GRB とは違う。
firmware は書き込む前に赤と緑を入れ替える。

## キー

| 既定の位置 | 名前 | 動作 |
|---|---|---|
| 左 | Jump | 承認待ちの pane を巡回する。承認待ちが無ければ作業を終えたエージェントを、それも無ければ全エージェントを巡回する（ADR 0011） |
| 中 | Approve | フォーカス中のプロンプトで、ハイライトされている選択肢を確定する |
| 右 | Select | フォーカス中のプロンプトで、ハイライトを次の選択肢へ動かす。最後の次は最初 |

承認するなら Jump、Approve の順に押す。
拒否など別の選択肢にするなら、それがハイライトされるまで Select を押してから Approve を押す（ADR 0006）。

「設定」の節の `layout` で、キーを別の位置に移せる（ADR 0013）。
キーは自分の LED を連れて移る。キーの上の LED は、「LED」の節のとおりそのキーの状態を表し、そのキーのために光る。

## 設定

`bridge` は、プラグインの設定ディレクトリにある `config.toml` を読む。
herdr が `HERDR_PLUGIN_CONFIG_DIR` を設定していればそこ、無ければ `<config>/plugins/config/shortarrow.herdr-triton` で、`<config>` は「接続」の節のとおり。
`herdr plugin config-dir shortarrow.herdr-triton` でこの場所を表示できる。

```toml
layout = ["jump", "approve", "select"]
```

| 項目 | 値 | 既定 |
|---|---|---|
| `layout` | 左、中、右の位置に置くキー。`"jump"`、`"approve"`、`"select"` を1回ずつ | `["jump", "approve", "select"]` |

| Given | Then |
|---|---|
| ファイルが無い | 既定のまま |
| TOML として読めないか、`layout` がそれぞれの名前を1回ずつ含まない | 既定のまま。理由をログに1行書く |
| この表に無い項目 | 無視する |

`bridge` はファイルを起動時に1回だけ読む。リスナーは `bridge stop` まで、起動時の設定を使い続ける。

## bridge の状態

| 名前 | 型 | 意味 |
|---|---|---|
| `conn` | `Disconnected \| Incompatible \| Connected` | herdr に届くか |
| `device` | `Absent \| Present` | シリアルポートを開いているか |
| `queue` | `(pane_id, agent, state_change_seq)` の列 | `blocked` のエージェント。`bridge` が最初に blocked を観測した順 |
| `focused` | `pane_id` または無し | herdr が `focused` と返すエージェントの pane |
| `sent` | `(pane_id, state_change_seq)` の集合 | 確定キーを送ったあと、まだ状態変化が報告されていないもの |
| `done` | `pane_id` の列 | `done`（作業を終え、まだ見られていない）のエージェント。`agent.list` の順 |
| `agents` | `pane_id` の列 | 全エージェント。`agent.list` の順 |

不変条件:

- `queue` に同じ `pane_id` は2度現れない
- `queue` の各要素は、最新のスナップショットで `blocked` だった
- `sent` の各要素は、`pane_id` と `state_change_seq` の両方が `queue` のどれかと一致する

`approvable(p)` は次をすべて満たすときに成り立つ。

- `p` が `queue` にある
- `p` が `focused` である
- その要素の `agent` にプロンプト用のキーが設定されている
- `(p, seq)` が `sent` に無い

## 振る舞い

### イベントループ

キー入力とスナップショットは1本のループで1つずつ処理する。
キー押下は、1つ前の押下で送ったリクエストと、その後の再取得が終わってから処理する。

### スナップショット

`bridge` は `agent.list` を 250 ms ごとに呼び、リクエストを送った直後にも呼ぶ。
取得した結果は次のように反映する。

| Given | Then |
|---|---|
| `queue` にあり、スナップショットに無いか `blocked` でない | 除去する |
| `queue` にあり、`blocked` のままで `state_change_seq` が変わった | 位置はそのままで `agent` と `state_change_seq` を更新する |
| スナップショットで `blocked`、`queue` に無い | 末尾に追加する。接続後の最初のスナップショットでは `agent.list` の順に追加する |
| 任意 | `focused` を `focused: true` のエージェントにする。無ければ無し |
| 任意 | `done` を `done` のエージェントに、`agents` を全エージェントにする |
| 任意 | `queue` と一致しなくなった `sent` の要素を捨てる |

`agent.list` が失敗したら `conn = Disconnected` にし、`queue`・`focused`・`sent` を空にする。
`bridge` は再接続を試み続ける。

### キー

| Given | When | Then |
|---|---|---|
| `conn ≠ Connected` | 任意のキー | エラー点滅 |
| `queue` が空でなく `focused ∉ queue` | Jump | 先頭へ `agent.focus`、再取得 |
| `queue` が空でなく `focused ∈ queue` | Jump | `focused` の次（末尾なら先頭）へ `agent.focus`、再取得 |
| `queue` が空で `done` が空でない | Jump | 上の2行と同じことを `done` に対して行う |
| `queue` も `done` も空 | Jump | 最初の2行と同じことを `agents` に対して行う。エージェントが1つも無ければエラー点滅 |
| 任意 | Approve | 再取得する。`approvable(focused)` なら確定キーを `agent.send_keys` で送り、`(focused, seq)` を `sent` に加える。そうでなければエラー点滅 |
| 任意 | Select | 再取得する。`approvable(focused)` なら「プロンプト用のキー」のとおりにハイライトを動かす。そうでなければエラー点滅 |
| 任意 | リクエストが失敗 | エラー点滅、再取得。失敗した確定は `sent` から外す |

Jump は `queue` の順序を変えない。
`queue` が1件でそれにフォーカスしていれば、Jump は同じ pane に再度フォーカスする。

Approve と Select は判断の直前にスナップショットを取り直すので、判断に使う状態はリクエスト1回ぶんより古くならない。
herdr が状態変化を報告するまでは `sent` が残るため、2回目の押下で同じプロンプトや、その後に出た別のプロンプトを確定してしまうことはない。
`sent` がある間は Select も効かないので、プロンプトが閉じたあとのエージェントの入力欄に矢印キーが入ることもない。

### プロンプト用のキー

herdr のエージェント ID ごとに定める。いまは組み込みで、「設定」の節のファイルにはまだ含めない。

| エージェント ID | 確定キー（初期値） | 選択キー（初期値） | リストの折り返し | 戻りキー（初期値） |
|---|---|---|---|---|
| `claude` | `["enter"]` | `["down"]` | しない | `["up"]` |
| `codex` | `["enter"]` | `["down"]` | する | |

Codex のリストは、最後の選択肢で `down` を押すと最初に戻る（openai/codex `1cc7e23`、`scroll_state.rs` の `move_down_wrap`）。
Claude Code のリストは最後で止まる。
リストが折り返さないエージェントでは、Select は `agent.read` で pane の画面を読み、番号付きの選択肢とハイライトの位置を見つける（ADR 0010）。

| Given | Then |
|---|---|
| ハイライトが `n` 個（`n ≥ 2`）の選択肢の最後にある | 戻りキーを `n − 1` 回送る |
| それ以外、画面にハイライト付きの番号リストが無い、または `agent.read` が失敗した | 選択キーを送る |

画面を使うのは Select がハイライトをどこへ動かすかの判断だけで、Approve で確定してよいかの判断には使わない。

どちらのエージェントも、承認プロンプトは矢印キーでハイライトが動き `enter` で確定するリストになっている。
Codex は `y` も受け付けるが、`y` はハイライトの位置に関係なく「yes」を選ぶので使わない。

## LED

常時の表示は `(conn, device, len(queue), len(done), approvable(focused))` の関数で決まる。
キー押下に対する1回きりの点滅を、その上に重ねる。

| 状態 | Jump | Approve | Select |
|---|---|---|---|
| `Disconnected` | 赤・遅い点滅 | 赤・遅い点滅 | 赤・遅い点滅 |
| `Incompatible` | 赤・点灯 | 赤・点灯 | 赤・点灯 |
| `queue` が空で `done` が空でない | 緑・呼吸 | 消灯 | 消灯 |
| `queue` も `done` も空 | 白・呼吸（Jump で全エージェントを巡回できる） | 消灯 | 消灯 |
| `queue` が1件 | 琥珀・呼吸 | `approvable(focused)` なら緑、それ以外は消灯 | `approvable(focused)` なら青、それ以外は消灯 |
| `queue` が2件以上 | 赤寄りの琥珀・呼吸 | 同上 | 同上 |

点滅:

- 成功なら、押したキーが白で1回光る。
  herdr がリクエストを受け付けたという意味で、エージェントが先へ進んだことまでは示さない
- 無効な操作やエラーなら、押したキーが赤で1回光る

## firmware

### ホスト不在

DTR が Low の間と、ホストからのコマンドが 3 秒届かないとき、firmware は `NoHost` になる。
`NoHost` の間は次のように振る舞う。

- キーイベントをキューに積まずに捨てる。`NoHost` に入るときにキューも空にする
- 3つの LED を薄い白で点ける

DTR が High の間にコマンドが届くと `NoHost` を抜け、LED は最後に指示された状態（電源投入後は消灯）に戻る。
リスナーは少なくとも 250 ms ごとに `KEY:EVENt?` を送るので、その間デバイスは `NoHost` にならない。

### キー

キーの状態が 5 ms 変わらなかったら、`Down` か `Up` のイベントを1つ作る。
イベントはホストが読むまで、16 件まで溜めておく。
キューが一杯のときに来たイベントは捨て、エラー -350 を記録する。

### 描画

| モードや状態 | 出力 |
|---|---|
| `Solid` | その色 |
| `Off` | 消灯 |
| `Blink` | 500 ms 点灯、500 ms 消灯 |
| `Breathe` | その色で、明るさが 2 秒かけて 10 % から 100 % まで上がって戻る |
| `Flash` | そのキーに 150 ms だけ点滅の色を重ねる |
| `Wave` | LED の動作確認用で、bridge は使わない。キーをまたいで流れる虹色。LED `n` は色相 `360° × t / 3 秒 + 120° × (n − 1)`、彩度は最大で、明るさは `Breathe` と同じく呼吸する。送られた色は使わない |
| `NoHost` | 3つとも 8/255 の白 |

WS2812 を全開で光らせると直視しづらいので、どの色も最大の明るさが 64/255 になるよう縮める。
LED は RGB 順で受け取り、`ws2812-pio` は GRB 順で送るので、firmware は最後に赤と緑を入れ替える。

### 不変条件

- メインループが 1 ms を超えて止まらない。
  ブロックしそうな書き込みは捨てる
- アニメーションは毎周タイマーの値を比べて進め、delay では待たない

### 識別

USB のシリアル番号は `TRITON-` にフラッシュのユニーク ID の16進表記を続けたもので、product 文字列は `herdr-triton` とする。
`bridge` はシリアル番号が `TRITON-` で始まることで、大文字小文字を区別せずにデバイスを探し、VID/PID は識別に使わない。
Windows の `serialport` は CDC ポートのシリアル番号を大文字にして返すが、product 文字列の代わりにポートの表示名を返すため、product 文字列では探せない。

### USB 構成

CDC シリアルと `drooling::PicotoolReset` の複合デバイスで、`usb_rev(Usb210)`・`max_packet_size_0(64)`・`composite_with_iads()`・`LangID::EN_US` を指定して組む。
`drool run` で書き込むときは、リセットを受け付けるデバイスを1台だけ挿しておく。

デバイスは、drooling の例と同じく、Raspberry Pi の VID `0x2E8A` と Pico SDK の CDC 用 PID `0x000A` を使う。
このリポジトリから作る趣味のデバイスで、販売しないので、自分の PID は取らない。
この組に依存するものは無い。`bridge` はキーパッドを `TRITON-` で始まるシリアル番号で探し、`drool` はリセット用のインターフェースを記述子で探す。

## firmware ↔ bridge プロトコル

USB CDC-ACM 上で、SCPI 風のテキストを1行1コマンドでやりとりする（ADR 0009）。
ホストがコマンドを送り、デバイスは問い合わせにだけ答えて自分からは何も送らない。
そのため、どのシリアルターミナルからでも操作できる。

- 行は `\n` で終わる。直前の `\r` は無視する。1行は 64 バイトまで
- ヘッダは大文字小文字を区別せず、表の長い形と、表で大文字の部分だけの短い形の両方を受け付ける（`EVENt` なら `EVEN` と `EVENT`）
- `<n>` は左から `1`・`2`・`3`、`<rgb>` は `#RRGGBB`、`<mode>` は `OFF`・`SOLid`・`BREathe`・`BLINk`・`WAVe`
- `?` の無いコマンドには返事をしない。問い合わせには必ず1行で答える

| コマンド | 返事 | 意味 |
|---|---|---|
| `*IDN?` | `ShortArrow,herdr-triton,<シリアル番号>,<firmware のバージョン>` | デバイスの識別 |
| `SYSTem:PROTocol?` | `3` | プロトコルのバージョン。3 で `WAVe` を追加した |
| `SYSTem:ERRor?` | `<コード>,"<メッセージ>"` | 最も古いエラー。無ければ `0,"No error"` |
| `LED:ALL <rgb>,<mode>,<rgb>,<mode>,<rgb>,<mode>` | | 左から順に全 LED を設定する |
| `LED<n> <rgb>,<mode>` | | 1つの LED を設定する |
| `LED<n>:FLASh <rgb>` | | 1つの LED を1回光らせる |
| `KEY:EVENt?` | `LEFT,DOWN` … `RIGHT,UP`、または `NONE` | 最も古いキーイベントを取り出す |

エラーは 8 件まで溜め、`SYSTem:ERRor?` で読む。

| コード | メッセージ | 条件 |
|---|---|---|
| -100 | `Command error` | 解析できない行 |
| -113 | `Undefined header` | 知らないヘッダ |
| -222 | `Data out of range` | LED の番号、色、モードが不正 |
| -350 | `Queue overflow` | キーイベントかエラーが、一杯のキューに来た |

型付きのコマンドと返事、およびそのテキスト表現は `protocol` crate に置き、両端で使う。

`bridge` はポートを開いたら DTR を立て、すでに溜まっていた受信データを捨て、`SYSTem:PROTocol?` を問い合わせる。
自分と違うバージョンならポートを閉じ、食い違いを報告する。
その後は表示が変わるたびに `LED:ALL` を送り、変わらなくても 1 秒に1回は送る。
また、「リスナーの生存期間」の間隔で `KEY:EVENt?` を `NONE` が返るまで問い合わせる。

## 依存

| crate | バージョン | 理由 |
|---|---|---|
| `rp2040-hal` | 0.11 | `ws2812-pio` 0.9 が 0.12 とはビルドできない |
| `usb-device` | 0.3 | `drooling` と `usbd-serial` 0.2 が要求する |
| `windows-sys` | 0.59 | 名前付きミューテックスとイベント、`WaitNamedPipeW`、ハンドルの継承、プロセス作成のフラグ |
| `toml` | 0.9 | `config.toml` を読む。パーサーと serde 対応だけを使う |
| `serialport` | 4 | Windows / Linux / macOS でポートを USB のシリアル番号付きで列挙できる |

プラグインには、プラグイン機能を持つ herdr が要る（`min_herdr_version` は 0.9.1。この仕様がもともと要求するバージョン）。

## 未規定

- 長押しの割り当て
- `done` の表示
- Approve が効くまでに、pane が承認可能な状態で留まるべき最短時間。
  質問 UI が出た瞬間に、すでに押しかけていた Approve がそれに答えてしまうのを防ぐためのもの
- 承認後に herdr が状態変化を一度も報告しなかったときに `sent` をどう解除するか
- リスナーのミューテックス、イベント、切り離した起動の、Linux と macOS での代わり。それまではプラグインは Windows だけを宣言する。そこでビルドした `bridge hook` は自分のプロセスでキーパッドを受け持ち（herdr はそのフックを実行中と数える）、キーパッドが故障したら終了する
- herdr のクライアントが複数接続されているとき、`focused` がどのクライアントの表示に対応するか
- 複数のデバイスの同時利用
- ネットワークの通信路: CDC-NCM 上で、TCP ポート 5025 に SCPI をそのまま流す方式。VISA からは `TCPIP::<アドレス>::5025::SOCKET`、telnet や nc からも使える。NCM のクラスを持つ `embassy-usb` に `drooling` が対応するのを待つ。SCPI のコマンドと解析処理はそのまま使う
