# 0008. A herdr plugin starts the bridge as a listener that exits when idle

English | [日本語](../ja/adr/0008-listener-started-by-plugin-hooks.md)

- Status: Withdrawn
- Base: [`53063f0`](https://github.com/ShortArrow/herdr-triton/commit/53063f029d0e14e019ac27c4d803a87e51d2898b)

This record decided that the plugin's hooks start the host program as a listener, which exits once nothing has waited for 5 s. How the listener starts and runs is now decided in [0012](0012-resident-listener-quiet-when-idle.md), whose Alternatives carry this design.
