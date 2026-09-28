# herdr-triton 仕様

herdr のエージェントが承認待ち（`blocked`）になったら、RP2040-Keyboard-3 の3キーでその pane へ移動し、承認し、次の承認待ちへ進む。
各キーの RGB LED には、承認待ちの件数と、いま Approve を押せば承認が送られるかを出す。

## 構成

```
[RP2040-Keyboard-3] --USB CDC--> [bridge] --local socket--> [herdr]
   キーの押下/解放                  agent.list をポーリングしてキューを保持
   <-- LED フレーム --              LED の表示を決める
```

| コンポーネント | 責務 |
|---|---|
| `firmware` | キーの押下・解放をホストへ送り、受け取った LED フレームを描画する。ホストがいないことを検知する。herdr に関する判断は持たない |
| `bridge` | herdr から全エージェントの状態を定期取得して状態を保持し、キー入力を herdr へのリクエストに変換し、LED 表示を決める |
| `protocol` | `firmware` と `bridge` 間のメッセージ型。`no_std` |

デバイスは HID キーボードとして振る舞わない。
キー入力はシリアル経由でのみ `bridge` に届く。

## herdr

### 前提

- herdr 0.9.1 以降。
  0.9.0 以前は `agent.focus` で接続中のクライアントの表示が切り替わらず、手動のフォーカス移動も通知されない
- `bridge` は herdr と同じ OS ユーザーで動かす

`bridge` は接続時に `ping` を送り、返ってきた `version` が 0.9.1 未満なら `Incompatible` とする。

### 接続

herdr の API は1接続につき1リクエストしか受け付けない。
サーバーは最初の1行を読んでそれに応答するだけなので、`bridge` はリクエストのたびに接続を開く。

ソケットのパスは herdr と同じ順序で決める。

1. `HERDR_SOCKET_PATH`
2. `HERDR_SESSION` があれば `<config>/sessions/<name>/herdr.sock`
3. `<config>/herdr.sock`

`<config>` は、`$XDG_CONFIG_HOME` があれば `$XDG_CONFIG_HOME/herdr`、無ければ Windows で `%APPDATA%\herdr`、それ以外で `~/.config/herdr`。
Windows ではこのパス文字列がそのまま named pipe の名前になり（interprocess の `GenericNamespaced`）、パスにあるファイルは目印でしかない。
命名規則を揃えるため、`bridge` は herdr と同じ `interprocess` のマイナーバージョンを使う。

### 使うリクエスト

| リクエスト | 用途 |
|---|---|
| `ping` | 接続時のバージョン確認 |
| `agent.list` | 全エージェントの状態。`pane_id`, `agent`, `agent_status`, `focused`, `state_change_seq` を使う |
| `agent.focus {target}` | Jump と Next。ワークスペースとタブを切り替えて pane にフォーカスする |
| `agent.send_keys {target, keys}` | Approve。pane にいるエージェントが入れ替わっていれば herdr が拒否する |

`events.subscribe` は使わない（ADR 0004）。

## 配置

`bridge` はデバイスを挿したマシンで動かし、そこから herdr のソケットに届く必要がある。

| herdr の場所 | bridge の場所 | herdr への経路 |
|---|---|---|
| Windows | Windows | named pipe |
| 同じマシンの Linux / macOS | 同じマシン | Unix socket |
| WSL 2 | WSL 2。デバイスは `usbipd attach --wsl --auto-attach` で渡す | Unix socket |
| SSH 先のリモート | 手元のマシン | `ssh -L` で転送した Unix socket |

WSL 2 と SSH の行は未検証。

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

| 位置 | 名前 | 動作 |
|---|---|---|
| 左 | Jump | `queue` の先頭にフォーカスする |
| 中 | Approve | フォーカス中の pane が承認可能なら、承認キーを送る |
| 右 | Next | `queue` の中でフォーカス中の pane の次にフォーカスする。末尾の次は先頭 |

## bridge の状態

| 名前 | 型 | 意味 |
|---|---|---|
| `conn` | `Disconnected \| Incompatible \| Connected` | herdr に届くか |
| `device` | `Absent \| Present` | シリアルポートを開いているか |
| `queue` | `(pane_id, agent, state_change_seq)` の列 | `blocked` のエージェント。`bridge` が最初に blocked を観測した順 |
| `focused` | `pane_id` または無し | herdr が `focused` と返すエージェントの pane |
| `sent` | `(pane_id, state_change_seq)` の集合 | 承認キーを送ったあと、まだ状態変化が報告されていないもの |

不変条件:

- `queue` に同じ `pane_id` は2度現れない
- `queue` の各要素は、最新のスナップショットで `blocked` だった
- `sent` の各要素は、`pane_id` と `state_change_seq` の両方が `queue` のどれかと一致する

`approvable(p)` は次をすべて満たすときに成り立つ。

- `p` が `queue` にある
- `p` が `focused` である
- その要素の `agent` に承認キーが設定されている
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
| 任意 | `queue` と一致しなくなった `sent` の要素を捨てる |

`agent.list` が失敗したら `conn = Disconnected` にし、`queue`・`focused`・`sent` を空にする。
`bridge` は再接続を試み続ける。

### キー

| Given | When | Then |
|---|---|---|
| `conn ≠ Connected` | 任意のキー | エラー点滅 |
| `queue` が空 | Jump か Next | エラー点滅 |
| `queue` が空でない | Jump | 先頭へ `agent.focus`、再取得 |
| `queue` が空でなく `focused ∉ queue` | Next | 先頭へ `agent.focus`、再取得 |
| `queue` が空でなく `focused ∈ queue` | Next | `focused` の次（末尾なら先頭）へ `agent.focus`、再取得 |
| 任意 | Approve | 再取得する。`approvable(focused)` なら承認キーを `agent.send_keys` で送り、`(focused, seq)` を `sent` に加える。そうでなければエラー点滅 |
| 任意 | リクエストが失敗 | エラー点滅、再取得 |

Next は `queue` の順序を変えない。
`queue` が1件でそれにフォーカスしていれば、Next は同じ pane に再度フォーカスする。

Approve は判断の直前にスナップショットを取り直すので、判断に使う状態はリクエスト1回ぶんより古くならない。
herdr が状態変化を報告するまでは `sent` が残るため、2回目の押下で同じプロンプトや、その後に出た別のプロンプトへ承認キーが送られることはない。

### 承認キー

herdr のエージェント ID ごとに、設定ファイルで定義する。

| エージェント ID | 承認キー（初期値） |
|---|---|
| `claude` | `["enter"]` |
| `codex` | `["y"]` |

Claude Code に `enter` を送ると、そのときハイライトされている選択肢が選ばれる。

## LED

常時の表示は `(conn, device, len(queue), approvable(focused))` の関数で決まる。
キー押下に対する1回きりの点滅を、その上に重ねる。

| 状態 | Jump | Approve | Next |
|---|---|---|---|
| `Disconnected` | 赤・遅い点滅 | 赤・遅い点滅 | 赤・遅い点滅 |
| `Incompatible` | 赤・点灯 | 赤・点灯 | 赤・点灯 |
| `queue` が空 | 消灯 | 消灯 | 消灯 |
| `queue` が1件 | 琥珀・呼吸 | `approvable(focused)` なら緑、それ以外は消灯 | 消灯 |
| `queue` が2件以上 | 赤寄りの琥珀・呼吸 | 同上 | 青 |

点滅:

- 成功なら、押したキーが白で1回光る。
  herdr がリクエストを受け付けたという意味で、エージェントが先へ進んだことまでは示さない
- 無効な操作やエラーなら、押したキーが赤で1回光る

## firmware

### ホスト不在

DTR が Low の間と、ホストからのフレームが 3 秒届かないとき、firmware は `NoHost` になる。
`NoHost` の間は次のように振る舞う。

- キーイベントをバッファに積まず、捨てる
- 3つの LED を薄い白で点ける

次に全体の LED フレームを受け取ったら `NoHost` を抜ける。

### 不変条件

- メインループが 1 ms を超えて止まらない。
  ブロックしそうな書き込みは捨てる
- アニメーションは毎周タイマーの値を比べて進め、delay では待たない

### 識別

USB の product 文字列は `herdr-triton`、シリアル番号はフラッシュのユニーク ID から作る。
`bridge` は product 文字列でデバイスを探し、VID/PID は識別に使わない。

### USB 構成

CDC シリアルと `drooling::PicotoolReset` の複合デバイスで、`usb_rev(Usb210)`・`max_packet_size_0(64)`・`composite_with_iads()`・`LangID::EN_US` を指定して組む。
`drool run` で書き込むときは、リセットを受け付けるデバイスを1台だけ挿しておく。

## firmware ↔ bridge プロトコル

USB CDC-ACM 上でやりとりする。
各メッセージは postcard でシリアライズして COBS で符号化し、末尾に `0x00` を付ける（ADR 0005）。
型は `protocol` crate に定義し、キーと LED は GPIO や数珠つなぎの番号ではなく位置（`Left`, `Middle`, `Right`）で呼ぶ。

| 向き | メッセージ | 意味 |
|---|---|---|
| デバイス → ホスト | `Ready { protocol }` | `NoHost` を抜けたときに1回送る。firmware のプロトコルバージョンを載せる |
| デバイス → ホスト | `Key { pos, edge }` | キーが `Down` または `Up` になった |
| ホスト → デバイス | `Frame([Led; 3])` | 左から順に、全 LED の色と表示モード（`Off`, `Solid`, `Breathe`, `Blink`） |
| ホスト → デバイス | `Flash { pos, rgb }` | 1つのキーの LED を1回光らせる |

デコーダは、復号できないフレームや最大長を超えるフレームに出会ったら、そのフレームをエラーとして報告し、次の `0x00` から読み直す。

`bridge` は表示が変わるたびに全体フレームを送り、変わらなくても 1 秒に1回は送る。
ポートを開いたら DTR を立て、すでに溜まっていた受信データを捨て、全体フレームを送る。
自分と同じプロトコルバージョンの `Ready` を受け取るまでは、キーイベントを無視する。
バージョンが違えばポートを閉じ、食い違いを報告する。

## 依存

| crate | バージョン | 理由 |
|---|---|---|
| `rp2040-hal` | 0.11 | `ws2812-pio` 0.9 が 0.12 とはビルドできない |
| `usb-device` | 0.3 | `drooling` と `usbd-serial` 0.2 が要求する |
| `interprocess` | 2.4 | named pipe の命名を herdr と揃える |
| `serialport` | 4 | Windows / Linux / macOS で product 文字列からポートを探せる |

## 未規定

- 長押しの割り当て（候補: Approve 長押しで「今後も許可」、Next 長押しで `esc` による拒否）
- `done` の表示
- Approve が効くまでに、pane が承認可能な状態で留まるべき最短時間。
  質問 UI が出た瞬間に、すでに押しかけていた Approve がそれに答えてしまうのを防ぐためのもの
- 承認後に herdr が状態変化を一度も報告しなかったときに `sent` をどう解除するか
- herdr のクライアントが複数接続されているとき、`focused` がどのクライアントの表示に対応するか
- 複数の herdr セッション、複数のデバイスの同時利用
