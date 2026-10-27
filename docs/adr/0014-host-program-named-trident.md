# 0014. The host program is named trident

English | [日本語](../ja/adr/0014-host-program-named-trident.md)

- Status: Accepted
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

## Context

The host program joins herdr and the keypad, and runs as a resident process that Task Manager lists by its executable's name. A name that describes only its role, such as "bridge", names neither herdr nor the keypad there, and in the documents it reads both as the program and as the role. The icon shows a triton holding a trident topped with three keys.

## Decision

- The host program is `trident`: the crate and its directory, the executable `trident.exe`, the commands `trident run|hook|listen|stop|find-port|raise`, and the log `trident.log`
- The names follow the icon: the plugin is the triton, and it holds the trident, whose three prongs reach the three keys
- The listener's mutex and events are `Local\herdr-triton-listener`, `-wake` and `-stop`: they are named after the plugin, not the program
- The firmware and the protocol carry no program name

## Consequences

- An installation whose host program is still `bridge` must run `.plugin/bin/bridge stop` before installing `trident`; the old `bridge.exe` and `bridge.log` are left behind unused
- "triton" and "trident" look alike; the README draws the picture once so that the likeness reads as one image

## Alternatives

- **`triton`**: matches the repository and plugin, but leaves no name for the part that reaches the keys, and calls two things by one name
- **`herdr-triton`**: plain, but long to type for every command
- **`bridge`**: a resident process nobody can identify
