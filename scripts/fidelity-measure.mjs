#!/usr/bin/env node
// Match-fidelity measurement harness (dev tool, non-canonical).
//
// Runs `target/release/dump_frames` over several seeds (full 5400-tick matches)
// and reports the metrics that gate the 2026-06 fidelity campaign:
//   - goals/match           — the goal-count REGRESSION GUARD (must not drop)
//   - offside%              — % of ticks with >=1 attacker beyond the 2nd-last defender, ahead of the ball
//   - ball-to-carrier dist  — median units between the ball and its possessor (should be ~0 when held)
//   - pass-flight visible   — median ticks the ball is loose & travelling between possessions
//
// Usage: node scripts/fidelity-measure.mjs [seedCount] [ticks]
// Determinism: dump_frames is seeded; same seeds -> same numbers. Compare runs
// before/after a sim change to prove an improvement without regressing goals.

import { execFileSync } from "node:child_process";

const SEED_COUNT = parseInt(process.argv[2] ?? "8", 10);
const TICKS = parseInt(process.argv[3] ?? "5400", 10);
const BIN = "target/release/dump_frames";
const SEEDS = Array.from({ length: SEED_COUNT }, (_, i) =>
  "0x" + (0x1000_0000 + i * 0x9e3779b1).toString(16),
);

const dist = (ax, ay, bx, by) => Math.hypot(ax - bx, ay - by);
const get = (f, slot) => f.players.find((p) => p.slot === slot);

function attackingTeam(f) {
  const s = f.possession;
  if (s == null || s < 0) return null;
  return s <= 10 ? "home" : "away";
}

function offsideCount(f) {
  const team = attackingTeam(f);
  if (!team) return 0;
  const atk = f.players.filter((p) => (team === "home" ? p.slot <= 10 : p.slot >= 11));
  const def = f.players.filter((p) => (team === "home" ? p.slot >= 11 : p.slot <= 10));
  const xs = def.map((p) => p.posX);
  let line;
  if (team === "home") { xs.sort((a, b) => b - a); line = xs[1]; }
  else { xs.sort((a, b) => a - b); line = xs[1]; }
  const ballX = f.ball.posX;
  let c = 0;
  for (const a of atk) {
    if (team === "home" && a.posX > line && a.posX > 0 && a.posX > ballX) c++;
    if (team === "away" && a.posX < line && a.posX < 0 && a.posX < ballX) c++;
  }
  return c;
}

function measure(seed) {
  const out = execFileSync(BIN, ["--seed", seed, "--ticks", String(TICKS), "--content", "content", "--compact"], {
    maxBuffer: 1 << 30,
  });
  const d = JSON.parse(out);
  const last = d[d.length - 1];
  const goals = last.homeScore + last.awayScore;

  let offTicks = 0, carrierTicks = 0, carrierDistSum = [];
  let flightSegs = [], cur = 0;
  for (const f of d) {
    if (offsideCount(f) >= 1) offTicks++;
    const s = f.possession;
    if (s != null && s >= 0) {
      const c = get(f, s);
      if (c) { carrierTicks++; carrierDistSum.push(dist(f.ball.posX, f.ball.posY, c.posX, c.posY)); }
      if (cur > 0) { flightSegs.push(cur); cur = 0; }
    } else {
      const sp = Math.hypot(f.ball.velX, f.ball.velY);
      if (sp > 1) cur++; else if (cur > 0) { flightSegs.push(cur); cur = 0; }
    }
  }
  carrierDistSum.sort((a, b) => a - b);
  flightSegs.sort((a, b) => a - b);
  const med = (arr) => (arr.length ? arr[Math.floor(arr.length / 2)] : 0);
  return {
    seed,
    goals,
    home: last.homeScore,
    away: last.awayScore,
    offsidePct: (100 * offTicks) / d.length,
    ballCarrierMed: med(carrierDistSum),
    flightMed: med(flightSegs),
    flights: flightSegs.length,
  };
}

const rows = SEEDS.map(measure);
const mean = (k) => rows.reduce((a, r) => a + r[k], 0) / rows.length;

console.log(`\nMatch-fidelity measurement — ${SEED_COUNT} seeds x ${TICKS} ticks\n`);
console.log("seed                goals  H-A    offside%  ballDist(med)  flight(med ticks, n)");
for (const r of rows) {
  console.log(
    `${r.seed.padEnd(18)} ${String(r.goals).padStart(5)}  ${r.home}-${r.away}    ${r.offsidePct.toFixed(1).padStart(6)}  ${r.ballCarrierMed.toFixed(1).padStart(11)}  ${String(r.flightMed).padStart(6)} (${r.flights})`,
  );
}
console.log("\n--- AGGREGATE ---");
console.log(`goals/match:        ${mean("goals").toFixed(2)}   (REGRESSION GUARD — must not drop materially)`);
console.log(`offside%:           ${mean("offsidePct").toFixed(1)}   (target ~0)`);
console.log(`ball-carrier dist:  ${mean("ballCarrierMed").toFixed(1)}   (target ~0 when held)`);
console.log(`pass-flight ticks:  ${mean("flightMed").toFixed(1)}   (target tens of ticks of visible travel)`);
