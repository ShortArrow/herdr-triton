# 0000. Each record states the decision in force, and is rewritten when it changes

English | [日本語](../ja/adr/0000-records-state-the-decision-in-force.md)

- Status: Accepted
- Base: [`5d65935`](https://github.com/ShortArrow/herdr-triton/commit/5d65935e1803cf9c5ed68d50c5518b6e57e71089)

## Context

A record is read by someone who wants to know why the code is the way it is, and who has to decide whether that reason still holds. A decision that holds is only useful in the form it holds now. When later records each change part of an earlier one, the decision in force is spread over a chain, and the reader has to follow every link and apply the changes in order to recover it. git already keeps every earlier text and when it was written.

## Decision

- One record holds one decision, numbered in the order it was first made. A number is never reused
- When the decision changes, its record is rewritten in place, whatever its status, so that it states the decision in force. The design it replaces moves to `## Alternatives`, with the reason it was dropped. A change that is a decision of its own gets a new record
- The body is in the present tense. `## Context` states the forces as they are and where they pull against each other; `## Alternatives` states each refusal as one that holds while its reason does, and loses it when the reason goes
- A judgment that only its moment explains, made under a constraint that has since gone and touching no standing decision, is not a record. It goes in the message of the commit it decided
- A record refers only to records it depends on, never to those that build on it, so references never form a cycle
- A record whose decision is no longer in force, and whose subject no other record takes over, is `Withdrawn`. One whose subject another record now holds is also `Withdrawn`, and says which record that is; that record's `## Alternatives` carries the withdrawn design
- The English record and the Japanese record of the same number say the same thing and are changed in the same commit

### Header

```markdown
# NNNN. <the decision, as a sentence>

English | [日本語](../ja/adr/NNNN-<slug>.md)

- Status: Accepted
- Base: [`abc1234`](https://github.com/ShortArrow/herdr-triton/commit/<full hash>)
```

| Field | Meaning |
|---|---|
| Status | `Draft`, `Accepted` or `Withdrawn`. A `Draft` becomes `Accepted` once its `## Open questions` is empty |
| Base | The parent of the commit that last wrote the record. It links to that commit on GitHub, so the code the record talks about is one click away |

The record carries no date, since git records when each version was written. The Japanese record carries the same header, with the field names in English and `[English](../../adr/NNNN-<slug>.md) | 日本語` as its second line.

### Body

`## Context`, `## Decision`, `## Consequences` and `## Alternatives`, in that order. A `Draft` adds `## Open questions` last, one item per question still to be settled, and drops it when it is accepted. A `Withdrawn` record has only a paragraph that says what it decided and which record holds its subject now.

## Consequences

- Reading one record gives the decision in force; its history is `git log -p` on that file
- A change to a decision edits its record and every document that states the decision, in one commit
- A record's Alternatives grow with each design it replaces, so a reader also learns what was tried

## Alternatives

- **Freeze each record once accepted, and record a change as a new record that supersedes or amends it**: every text stays as it was written, but the decision in force has to be pieced together from a chain of records, and each link is a back-reference the earlier record has to carry
- **Write the decisions into the specification instead of records**: one place to read, but the specification states what the system does, and the forces and the rejected designs would crowd it
