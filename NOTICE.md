# Final Whistle — Third-Party Notices

Final Whistle is proprietary software (see `LICENSE`). This file catalogues
the third-party software it consults or depends on, and the attribution each
requires.

## Bundled dependencies

Final Whistle's binary distribution links against third-party open-source
libraries — Rust crates and npm packages — all under permissive licences
(MIT, Apache 2.0, BSD, ISC, MPL-2.0, Zlib, CC0, 0BSD, Unicode-3.0).

The Cargo dependency licence policy is enforced by `cargo deny` against the
allowlist in `deny.toml`: permissive-only, GPL/AGPL fail CI. The npm
dependency tree was last audited 2026-05-22 and contains no copyleft
licences.

A comprehensive `THIRD-PARTY-LICENSES.txt` covering every transitive
bundled dependency will be generated as part of the Steam build pipeline
(Phase T5+) and shipped alongside the binary, per the attribution
requirements of the relevant licences.

## Research-only prior art (no code incorporated)

During T1 design research the match-engine architecture was informed in
part by surveying two open-source Rust football sims. The shipped Final
Whistle code was independently audited 2026-05-22 and is confirmed
independent of both — no copyrightable expression from either project is
reproduced in this codebase. The detailed research notes themselves are
preserved privately and are not part of this repository.

The audit verdict applies to the five patterns flagged in the private
research notes:

- **Match-loop architecture — INDEPENDENT.** Final Whistle runs a 60 Hz
  fixed-step integrator; one surveyed project ran a minute-stepper. The
  designs share no structure.
- **Ball-zone action dispatch — INDEPENDENT.** Final Whistle uses
  continuous Q32 coordinates plus behaviour trees; no zone-routed switch
  exists anywhere in the codebase.
- **MatchEvent shape — INDEPENDENT.** Idiomatic Rust tagged-union per
  variant vs the surveyed struct-with-tag pattern; different fields,
  types, and variants.
- **IPC command surface — INDEPENDENT.** Different return-struct shapes
  and granularity; functional API design is not protected by copyright
  (Oracle America, Inc. v. Google LLC, 593 U.S. 1 (2021)).
- **Player-attribute bias functions — PATTERN-LEVEL.** Separable pure
  functions returning multiplicative modifiers is an API pattern, not
  copyrightable expression; the implementations share zero lines.

Football attribute vocabulary (`finishing`, `passing`, `composure`, etc.)
used in both Final Whistle and several surveyed projects is
industry-standard terminology popularised by Football Manager over more
than two decades — not protected expression.

## Reporting

If you believe any portion of Final Whistle reproduces protected
expression from a third-party project that is not properly attributed
here, please contact osagberg@proton.me.

---

Last audit: 2026-05-22. Licence posture: All Rights Reserved (see `LICENSE`).
