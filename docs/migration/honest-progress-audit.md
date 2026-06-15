# Honest progress audit — RustyCore port (R8-entities miniphase)

**Date:** 2026-06-15 · **Base commit:** `develop` after `#NEXT.R8.ENTITIES.940`

This document exists to prevent the headline `97.64%` from being read as "almost a
finished, gap-free server." It records what the number actually measures, with three
honest percentages instead of one.

## Raw data (from `docs/migration/inventory/r8-entities-miniphase.tsv`)

932 rows total. Breakdown by `status` column:

| status | rows | meaning |
|---|---:|---|
| `complete` | 419 | genuinely complete |
| `represented-complete` | 189 | complete **within the "represented" per-session model** (not live runtime) |
| `represented-partial` | 282 | **partial — carries documented open boundaries (gaps)** |
| `reviewed-validated` | 17 | validated |
| `pending` | 22 | not started |
| other (`partial` / `represented` / `test-fixture-unblock`) | 3 | — |

- **282 of the 910 "addressed" rows (30.99%) are `represented-partial`** — each has open boundaries by definition.
- **Many rows explicitly declare `manual-test-ready` / `install/restart` as OPEN** — this audit must not be read as real-client/server validation unless a row says that validation was performed.

## Three honest percentages (not one)

| Metric | Value | Reading |
|---|---:|---|
| Items "addressed" (not `pending`) | **97.64%** (910/932) | the headline number — real but generous |
| No declared partial gaps (`complete` + `represented` + `represented-complete` + `reviewed-validated` + `test-fixture-unblock`) | **67.27%** (627/932) | items with no open boundary |
| Live-runtime / manual-test-ready verified | **low / not globally quantified** | login/realm smoke has been exercised before, but most gameplay rows still explicitly lack live client/bot/manual validation |

## The two big caveats

1. **`97.64%` measures "represented game logic addressed", not "gap-free server."**
   The macro-gap — the split-engine live runtime — sits *below* this inventory and is
   measured by none of these rows. See the runtime architecture finding in
   `current-session-handoff.md` (#NEXT.R8.ENTITIES.764): three coexisting world models,
   where the engine that runs AI/combat (legacy `wow_world::MapManager`) has no clock of
   its own, and the engine with the global tick (canonical `wow_map::MapManager`) does not
   dispatch AI/combat (`CreatureRuntimeUpdateContext::default()`, no `match` on the plan).

2. **This is ONE miniphase (R8-entities).** The full port also has the r7-l1/l2/l3
   infra/packets/maps miniphases and more. 931 R8 rows are not "the whole server".

## Honest one-line status

The bulk of the game logic is ported and contrasted against C++ in a per-session
"represented" model (~67.27% with no declared partial gaps, ~97.64% of inventory rows touched).
What remains is to convert represented-partial boundaries into live runtime behavior and
verify them on a running server/client path. The live-runtime roadmap (steps 2-7) is the
work that actually moves toward "no gaps"; closing more represented-partial items advances
the 70-97% metrics but does not by itself prove complete runtime parity.

## Live-runtime roadmap (from #764 analysis)

1. ✅ Characterize the engine split (regression anchor, test-only) — done (#764).
2. Give the legacy map its own clock — sessionless creature-AI tick. **(first production change; real architectural risk)**
3. Creature-move fanout from the global tick via a per-map session registry.
4. Move combat resolution to the global clock (resolve once, not per-session).
5. Unify respawn (per-session queue → canonical).
6. Single source of truth for creatures (method-by-method; the `_attic/` big-bang failed with 176 errors).
7. Real `SendObjectUpdates` + scripts/weather/threat.
