# 0018. Jump follows the order of herdr's agents pane

English | [日本語](../ja/adr/0018-jump-follows-the-agents-pane.md)

- Status: Accepted
- Base: [`309db6e`](https://github.com/ShortArrow/herdr-triton/commit/309db6ecf4e15a358ded32282488ad4812a79b5d)

## Context

Jump cycled waiting agents in the order `trident` first saw them blocked, and finished or other agents in `agent.list` order (ADR 0011). herdr's agents pane sorts its list in one of two modes, `grouped` and `priority`, which its header toggles. In `priority` mode the list put the newest prompt on top, while Jump went to the oldest first, so Jump seemed to skip around the list.

herdr 0.9.3 (`src/client/shell/agent_sidebar.rs`) orders the pane as follows:

- `grouped`: `agent.list` order, which is workspace, then tab, then pane order
- `priority`: by status, `blocked`, `done`, `working`, `idle`, `unknown`, then by `state_change_seq` from the highest; a stable sort, so ties keep `agent.list` order

No API reports the mode. The client keeps it in its own preferences file, `<herdr state>/client-shell/local-<hash>.json`, as `agent_panel_sort` once the header has been clicked. Before that, `[ui] agent_panel_sort` in herdr's `config.toml` sets it. The hash is the FNV-1a 64 of the client socket's path.

## Decision

- Jump keeps ADR 0011's tiers, and within a tier goes to the next agent in the pane's order instead of in `queue` or `agent.list` order
- `trident` reads the mode each time Jump goes down: from the client's preferences file, then from herdr's `config.toml`, and otherwise takes `grouped`
- A file that is missing, unreadable, or holds a value other than `spaces`, `workspaces` or `priority` counts as not saying, and the next source decides
- `queue` no longer keeps an order, since nothing else read it

## Consequences

- In `priority` mode Jump goes to the newest waiting prompt first, as the list shows it, not to the one that has waited longest
- Toggling the pane's sort takes effect at the next Jump, without restarting `trident`
- `trident` depends on a file that is herdr's own, not its API. If herdr moves or changes it, Jump falls back to `config.toml` and then to `grouped`, without an error
- Several clients each keep their own file; `trident` reads the one of the local client socket, the one herdr's own client uses
- A view that a hook sets with `agent.view.set` reorders the pane but not Jump

## Alternatives

- **A `jump_order` field in `trident`'s `config.toml`**: no dependence on herdr's files, but the field has to be changed by hand whenever the pane's header is clicked, and the two drift apart
- **Keep the oldest waiting prompt first**: Jump would keep its own order, which is the confusion this record removes
- **Ask herdr for an API that reports the order**: upstream work, which this project does not request
