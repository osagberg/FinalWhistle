# Tauri bundle icons

Required at T4-7 (game-shell polish). For T0-2, generate placeholder icons with
the Tauri CLI:

```
pnpm tauri icon path/to/source-1024x1024.png
```

This populates this directory with the full per-OS icon set referenced from
`tauri.conf.json`:

- `32x32.png`, `128x128.png`, `128x128@2x.png` — Linux
- `icon.icns` — macOS
- `icon.ico` — Windows

`pnpm tauri build` will fail until those files exist; `pnpm tauri dev` runs
fine without them.
