# Honest progress audit — RustyCore port (R8-entities miniphase)

**Date:** 2026-06-19 · **Base commit:** `develop` after `#NEXT.R8.ENTITIES.1106`

This document exists to prevent the headline `97.99%` from being read as "almost a
finished, gap-free server." It records what the number actually measures, with three
honest percentages instead of one.

## Raw data (from `docs/migration/inventory/r8-entities-miniphase.tsv`)

1098 rows total. Breakdown by `status` column:

| status | rows | meaning |
|---|---:|---|
| `complete` | 419 | genuinely complete |
| `represented-complete` | 191 | complete **within the "represented" per-session model** (not live runtime) |
| `represented-partial` | 412 | **partial — carries documented open boundaries (gaps)** |
| `bugfix-partial` | 34 | bugfix slice with explicit remaining boundaries |
| `reviewed-validated` | 17 | validated |
| `pending` | 22 | not started |
| other (`partial` / `represented` / `test-fixture-unblock`) | 3 | — |

- **447 of the 1076 "addressed" rows (41.54%) are partial-boundary rows** (`represented-partial`, `bugfix-partial`, or `partial`) — each has open boundaries by definition.
- **Many rows explicitly declare `manual-test-ready` / `install/restart` as OPEN** — this audit must not be read as real-client/server validation unless a row says that validation was performed.

## Three honest percentages (not one)

| Metric | Value | Reading |
|---|---:|---|
| Items "addressed" (not `pending`) | **98.00%** (1076/1098) | the headline number — real but generous |
| No declared partial gaps (`complete` + `represented` + `represented-complete` + `reviewed-validated` + `test-fixture-unblock`) | **57.29%** (629/1098) | items with no open boundary |
| Live-runtime / manual-test-ready verified | **low / not globally quantified** | login/realm smoke has been exercised before, but most gameplay rows still explicitly lack live client/bot/manual validation |

## The two big caveats

1. **`97.99%` measures "represented game logic addressed", not "gap-free server."**
   The macro-gap — split-engine live runtime — is now partially bridged but not fully
   closed. Since the original #764 finding, the legacy `wow_world::MapManager` gained an
   experimental global creature runtime (`RustyCore.LegacyCreatureGlobalRuntime`, default
   off) for lifecycle, movement, aggro, and melee, while the canonical `wow_map::MapManager`
   still remains the structural destination. This is real progress, but it is not globally
   manual-test-ready and does not by itself prove full runtime parity.

2. **This is ONE miniphase (R8-entities).** The full port also has the r7-l1/l2/l3
   infra/packets/maps miniphases and more. 1095 R8 rows are not "the whole server".

## Honest one-line status

The bulk of the game logic is ported and contrasted against C++ in a per-session
"represented" model (~57.29% with no declared partial gaps, ~98.00% of inventory rows touched).
What remains is to convert represented-partial boundaries into live runtime behavior and
verify them on a running server/client path. The live-runtime roadmap (steps 2-7) is the
work that actually moves toward "no gaps"; closing more represented-partial items advances
the 70-97% metrics but does not by itself prove complete runtime parity.

## Live-runtime roadmap (from #764 analysis)

1. ✅ Characterize the engine split (regression anchor, test-only) — done (#764).
2. ✅ Give the legacy map its own experimental clock — default off, gated by config.
3. ✅ Creature lifecycle/movement fanout from the global tick exists behind that gate.
4. ✅ Creature aggro/melee resolution is represented in the global legacy runtime; player auto-attack remains session-owned like C++ `Player::Update`.
5. 🔧 Respawn ownership has moved from per-session to map-owned legacy state, but canonical single-source convergence is still incomplete.
6. 🔧 Single source of truth for creatures is still method-by-method work; the `_attic/` big-bang failed with 176 errors and remains the reason for incremental slices.
7. 🔧 Real `SendObjectUpdates` + scripts/weather/threat remain open beyond represented seams.
