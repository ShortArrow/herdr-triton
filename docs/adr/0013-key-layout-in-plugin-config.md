# 0013. The key layout is set in the plugin's configuration directory

English | [日本語](../ja/adr/0013-key-layout-in-plugin-config.md)

- Status: Accepted
- Base: [`b442f69`](https://github.com/ShortArrow/herdr-triton/commit/b442f691183e682c815705169ae359fa489f1090)
- Amends: 0006 (which position each key takes)

## Context

ADR 0006 fixes Jump on the left, Approve in the middle and Select on the right. People hold the keypad differently and may want another order. Keyboard firmwares such as QMK, ZMK and RMK, with VIA or Vial, remap positions to HID keycodes; the keypad sends key positions over SCPI instead (ADR 0009), and the bridge gives them their meaning, so a keycode remap would not reach Jump, Approve and Select. herdr 0.9.1 ignores unknown sections in its own `config.toml`, so a plugin cannot keep settings there. It does give every plugin a configuration directory, passes it to every plugin command as `HERDR_PLUGIN_CONFIG_DIR`, leaves the file format to the plugin, and prints the directory with `herdr plugin config-dir <id>` (herdr docs, plugins).

## Decision

- `bridge` reads `config.toml` from the plugin's configuration directory. Without `HERDR_PLUGIN_CONFIG_DIR`, as for `bridge run`, it builds the same path from herdr's configuration directory, so both read one file
- `layout` names the key at the left, middle and right position: a permutation of `"jump"`, `"approve"` and `"select"`, defaulting to ADR 0006's order
- A missing file, a file that does not parse, or a `layout` that is not a permutation leaves the default layout; the last two are logged
- The bridge's core works with keys, not positions; the runtime maps a pressed position to its key, and each key's LED and flash to that key's position
- The file is read once when `bridge` starts

## Consequences

- The layout follows the herdr installation, not the keypad: another machine needs its own file
- Changing the layout needs `bridge stop`; the next hook starts a listener that reads it
- The prompt keys of "Prompt keys" can move into the same file later

## Alternatives

- **A separate directory, `%APPDATA%\herdr-triton` or `~/.config/herdr-triton`**: independent of herdr, but a second place to find, outside herdr's plugin lifecycle
- **Store the layout on the keypad with an SCPI command**: the layout travels with the device, at the cost of flash writes and a protocol change
- **QMK, ZMK or RMK with VIA or Vial**: the keypad would become a HID keyboard again, which ADR 0009 left
