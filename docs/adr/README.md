# Architecture decision records

English | [日本語](../ja/adr/README.md)

Each record is one decision, numbered in the order it was made and frozen once accepted. A later record that changes it says so in its header, and the earlier record links forward to it.

## Header

```markdown
# NNNN. <the decision, as a sentence>

English | [日本語](../ja/adr/NNNN-<slug>.md)

- Status: Accepted
- Amended by: [MMMM](MMMM-<slug>.md)
- Base: [`abc1234`](https://github.com/ShortArrow/herdr-triton/commit/<full hash>)
- Supersedes: KKKK
- Amends: KKKK (what it changes)
```

| Field | Meaning |
|---|---|
| Status | `Accepted`, or `Superseded by [MMMM](…)` |
| Amended by | Later records that change part of this one; omitted when none |
| Base | The commit the decision was made against: the parent of the commit that adds the record. It links to that commit on GitHub, so the code the record talks about is one click away |
| Supersedes, Amends | Earlier records this one replaces or changes; omitted when none |

The Japanese record of the same number carries the same header, with the field names in English.

## Body

`## Context`, `## Decision`, `## Consequences` and `## Alternatives`, in that order.
