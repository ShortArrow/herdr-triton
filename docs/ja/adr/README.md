# 設計判断の記録（ADR）

[English](../../adr/README.md) | 日本語

1つの記録は1つの判断で、判断した順に番号を振り、採用したら書き換えない。
あとの記録がそれを変えるときは、あとの記録の見出しにそう書き、元の記録からはあとの記録へリンクする。

## 見出し

```markdown
# NNNN. <判断を1文で>

[English](../../adr/NNNN-<slug>.md) | 日本語

- Status: Accepted
- Amended by: [MMMM](MMMM-<slug>.md)
- Base: [`abc1234`](https://github.com/ShortArrow/herdr-triton/commit/<完全なハッシュ>)
- Supersedes: KKKK
- Amends: KKKK（変える内容）
```

| 項目 | 意味 |
|---|---|
| Status | `Accepted` か、`Superseded by [MMMM](…)` |
| Amended by | この記録の一部を変えた、あとの記録。無ければ書かない |
| Base | 判断の前提にしたコミット。記録を追加したコミットの親。GitHub のそのコミットへリンクするので、記録が話しているコードを1クリックで開ける |
| Supersedes、Amends | この記録が置き換える、または変える、前の記録。無ければ書かない |

同じ番号の英語版と日本語版は同じ見出しを持ち、項目名は英語のままにする。

## 本文

`## Context`、`## Decision`、`## Consequences`、`## Alternatives` をこの順に置く。
