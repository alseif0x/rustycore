# Honest progress audit — RustyCore port (R8-entities miniphase)

**Date:** 2026-05-29 · **Base commit:** `b285e92` (develop = main, clean)

This document exists to prevent the headline `96.97%` from being read as "almost a
finished, gap-free server." It records what the number actually measures, with three
honest percentages instead of one.

## Raw data (from `docs/migration/inventory/r8-entities-miniphase.tsv`)

759 rows total. Breakdown by `status` column:

| status | rows | meaning |
|---|---:|---|
| `complete` | 419 | genuinely complete |
| `represented-complete` | 121 | complete **within the "represented" per-session model** (not live runtime) |
| `represented-partial` | 177 | **partial — carries documented open boundaries (gaps)** |
| `reviewed-validated` | 17 | validated |
| `pending` | 22 | not started |
| other (`partial` / `represented` / `test-fixture-unblock`) | 3 | — |

- **177 of the 736 "closed" rows (24%) are `represented-partial`** — each has open boundaries by definition.
- **348 of the 759 rows (46%) explicitly declare `manual-test-ready` / `install/restart` as OPEN** — i.e. nearly half the inventory states it has not been exercised on a real running server.

## Three honest percentages (not one)

| Metric | Value | Reading |
|---|---:|---|
| Items "addressed" (not `pending`) | **96.97%** (736/759) | the headline number — real but generous |
| No declared partial gaps (`complete` + `represented-complete` + `reviewed-validated`) | **~73.4%** (557/759) | items with no open boundary |
| Live-runtime / manual-test-ready verified | **≈ 0%** | nothing has been run on a real server; the "represented" model is not the live runtime |

## The two big caveats

1. **`96.97%` measures "represented game logic addressed", not "gap-free server."**
   The macro-gap — the split-engine live runtime — sits *below* this inventory and is
   measured by none of these rows. See the runtime architecture finding in
   `current-session-handoff.md` (#NEXT.R8.ENTITIES.764): three coexisting world models,
   where the engine that runs AI/combat (legacy `wow_world::MapManager`) has no clock of
   its own, and the engine with the global tick (canonical `wow_map::MapManager`) does not
   dispatch AI/combat (`CreatureRuntimeUpdateContext::default()`, no `match` on the plan).

2. **This is ONE miniphase (R8-entities).** The full port also has the r7-l1/l2/l3
   infra/packets/maps miniphases and more. 759 is not "the whole server".

## Honest one-line status

The bulk of the game logic is ported and contrasted against C++ in a per-session
"represented" model (~73% with no declared partial gaps, ~97% of inventory rows touched).
What remains is to make that model live under a real runtime and to verify it on a
running server (live-runtime ≈ 0% verified). The live-runtime roadmap (steps 2-7) is the
work that actually moves toward "no gaps"; closing more represented-partial items advances
the 73-97% metrics but not the runtime one.

## Live-runtime roadmap (from #764 analysis)

1. ✅ Characterize the engine split (regression anchor, test-only) — done (#764).
2. Give the legacy map its own clock — sessionless creature-AI tick. **(first production change; real architectural risk)**
3. Creature-move fanout from the global tick via a per-map session registry.
4. Move combat resolution to the global clock (resolve once, not per-session).
5. Unify respawn (per-session queue → canonical).
6. Single source of truth for creatures (method-by-method; the `_attic/` big-bang failed with 176 errors).
7. Real `SendObjectUpdates` + scripts/weather/threat.
