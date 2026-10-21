# 0015. Holding Jump brings herdr's window to the front

English | [日本語](../ja/adr/0015-hold-jump-to-raise-herdr.md)

- Status: Accepted
- Base: [`8c3d6b6`](https://github.com/ShortArrow/herdr-triton/commit/8c3d6b6eb7b13146cdd34f2054f4038ebe0b334b)

## Context

The keys act on herdr through its API, whichever window has focus. That suits a prompt that only needs approving: the user can keep a game or a browser in front. An agent that needs typed input is different: after Jump, the user has to reach for the mouse to bring the terminal forward, then go back to the keyboard. The keypad then costs more than it saves.

Windows does not let a background process take the foreground with `SetForegroundWindow` alone. A spike in `work/` called it from a detached process with no console, as the listener is, and Windows refused. Attaching to the foreground window's input thread with `AttachThreadInput` first brought WezTerm in front of Chrome, also from a minimised state. herdr's console in WezTerm is a pseudo console with no owner window, so the window was found by walking from the herdr process up to the first ancestor with a visible top-level window, `wezterm-gui`.

`KEY:EVENt?` already reports `Up` as well as `Down`; `trident` acted on `Down` and ignored `Up`.

## Decision

- Holding Jump for 500 ms brings the window that shows herdr to the front. Jump still acts on `Down` as before, so a short press is unchanged and nothing waits for the release
- The window is raised once per hold, when the 500 ms pass; Jump's LED flashes white when it was raised and red when it was not
- The listener does not raise the window itself. It runs `trident raise` with `CREATE_NO_WINDOW`, as it runs `trident find-port`, so that `user32.dll` is loaded only in that short-lived process
- `trident raise` takes the herdr client process, walks up its ancestors to the first that owns a visible top-level window without an owner, restores that window if it is minimised, and brings it to the front after `AttachThreadInput` to the foreground window's thread

## Consequences

- A long press can take the foreground from a full-screen game; that is what the hold asks for
- With several herdr clients, or several windows in one terminal process, which window is raised is unspecified
- The listener's memory does not change
- Automatically raising the window when Jump lands on an agent that needs typing can be added later on top of the same command

## Alternatives

- **Select raises the window while herdr is not in front**: the same key would mean two things depending on a state the keypad does not show, and a press meant for the prompt could raise the window instead
- **Every Jump raises the window**: approving while a game stays in front, the reason for the API in the first place, would no longer work
- **Raise from the listener itself**: `user32.dll` and a UI thread in the resident process, against ADR 0012's memory target
- **Double press**: Jump acts on the first press, so a double press would also jump twice
