# Final Whistle — dots-adapter UI Toolkit fonts

Three font families are bundled with the dots adapter for the Slice-6 UI
Toolkit overlay (scoreboard / commentary / signature title-card). All
three are licensed under **SIL Open Font License 1.1** — free for
commercial use + redistribution + bundling with derivative works
(including game executables) provided the OFL license text travels
alongside the fonts.

## Bundled fonts

| Family | Weight(s) | Role | Upstream source | Pinned ref |
|---|---|---|---|---|
| **Anton** | Regular | Display / signature title-card | [github.com/google/fonts/raw/main/ofl/anton/](https://github.com/google/fonts/raw/main/ofl/anton/) | `main` @ 2026-05-04 |
| **JetBrains Mono** | Regular / Medium / Bold | Scoreboard digits / monospaced data / debug | [github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/](https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/) | `master` @ 2026-05-04 |
| **Rajdhani** | Light / Regular / Medium / SemiBold / Bold | Body / commentary / scoreboard team labels | [github.com/google/fonts/raw/main/ofl/rajdhani/](https://github.com/google/fonts/raw/main/ofl/rajdhani/) | `main` @ 2026-05-04 |

## License text

Each family ships its upstream `OFL.txt` alongside the TTFs:

- [`Anton/OFL.txt`](Anton/OFL.txt) — Copyright 2020 The Anton Project Authors
- [`JetBrainsMono/OFL.txt`](JetBrainsMono/OFL.txt) — Copyright 2020 The JetBrains Mono Project Authors
- [`Rajdhani/OFL.txt`](Rajdhani/OFL.txt) — Copyright (c) 2014, Indian Type Foundry

The full SIL OFL 1.1 text appears verbatim in each per-family `OFL.txt`.
The license is identical across all three families; the per-family file
is preserved so the copyright-author header lines stay intact (a
redistribution requirement under §2 of the OFL).

## Redistribution compliance checklist

Per SIL OFL 1.1 §2, when the fonts are redistributed (i.e. shipped with
a built game executable on Steam), the following conditions must hold:

1. **The fonts must remain under the OFL** — derivative ports / reissues
   must inherit the OFL. ✅ We do not modify the source TTFs; they ship
   byte-identical to the upstream pinned commits.
2. **The license file must accompany the fonts** — `OFL.txt` per family
   AND this `LICENSES.md` aggregate ship in the same directory tree.
   The Steam-build packaging step (Phase 7+) must include
   `unity-project/Assets/Viewer/Adapters/Dots/UI/Fonts/**` so the
   license artifacts reach the player install. ✅ Tracked here.
3. **The fonts must not be sold by themselves** — we never ship them as
   a standalone font product. They are bundled inside the game
   executable + Asset bundles. ✅
4. **The "Reserved Font Name" clause does not apply here** — none of
   the three OFLs declare a reserved name, so we are free to import
   the TTFs into Unity Font Assets / SDF Font Assets without renaming.
   ✅ Confirmed by reading each `OFL.txt` header.

## Authoring trail

- `steam-release/asset-licensing-tracker.csv` records each family with
  the pinned upstream source URL + redistribution flag + the
  Slice-6 check-in date. Slice 6 commit body cites the same URLs
  alongside the SHA + date of the upstream pinned reference.
- This file is the player-facing license trail (ships in the build);
  the tracker is the producer-facing audit trail.
