# 0014. The host program is named trident

English | [日本語](../ja/adr/0014-host-program-named-trident.md)

- Status: Accepted
- Base: [`9374c56`](https://github.com/ShortArrow/herdr-triton/commit/9374c5698a877aee429227ef4e5c78be5aa6f3a8)

## Context

The host program that joins herdr and the keypad was called `bridge`: the crate, the executable, its commands and its log. The word describes a role, not this program. Task Manager showed a resident `bridge.exe` that named neither herdr nor the keypad, and the documents used "bridge" both for the program and for the role. The icon already shows a triton holding a trident topped with three keys.

## Decision

- The host program is `trident`: the crate and its directory, the executable `trident.exe`, the commands `trident run|hook|listen|stop|find-port`, and the log `trident.log`
- The names follow the icon: the plugin is the triton, and it holds the trident, whose three prongs reach the three keys
- The documents describe current behaviour with the new name. ADRs 0001 to 0013 stay as written; their "bridge" is this program
- The listener's mutex and events keep their names, `Local\herdr-triton-listener`, `-wake` and `-stop`: they are named after the plugin, not the program
- The firmware and the protocol do not change

## Consequences

- An installation from before this change must run `.plugin/bin/bridge stop` before installing the new program; the old `bridge.exe` and `bridge.log` are left behind unused
- "triton" and "trident" look alike; the README draws the picture once so that the likeness reads as one image

## Alternatives

- **`triton`**: matches the repository and plugin, but leaves no name for the part that reaches the keys, and calls two things by one name
- **`herdr-triton`**: plain, but long to type for every command
- **Keep `bridge`**: no migration, but a resident process nobody can identify
