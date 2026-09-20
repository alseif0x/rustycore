# Player inventory storage — memory footprint plan

**Date:** 2026-09-20 · **Base:** `3.4.3` @ `81c74731` · **Host:** `alastia`, x86_64, 16 GB
(the guide's reference development host is aarch64; every measurement below is
machine-dependent and labelled as taken on this host).

This plan owns one defect: `PlayerInventoryStorage` inlines a full bag record into every
player slot, where the C++ target stores a pointer. It does not restate the port order,
which stays in [PORT_PLAN.md](../migration/PORT_PLAN.md) and GitHub #49.

## Goal

Bring `PlayerInventoryStorage` from **128,600 bytes to under 8,000 bytes** without any
observable change in gameplay, packets, persistence or slot semantics.

The goal is met when all four hold:

1. `PlayerInventoryStorage` measures < 8,000 bytes under `-Zprint-type-sizes`.
2. Every existing test that touches inventory, bags or banking passes unchanged — no test
   is edited to accommodate the new representation.
3. `is_bag_storage_slot` still accepts exactly the same slot set, and no caller changes
   which absolute slot number it passes.
4. `./tools/validation-v2 final --base origin/3.4.3 --timings` passes inside the ordinary
   600-second local acceptance budget.

**Status: 1, 2 and 3 met; 4 is blocked by a gate that is already red on `3.4.3` itself.**
See Phase 4. The compile-time memory reduction this work originally expected did not
materialise and has been withdrawn — see Phase 3.

## Measured baseline

Taken on this host at `81c74731`, release profile, warm cache.

| Measurement | Value | How |
|---|---|---|
| `world-server` lib peak RSS | 3,660,936 KB (3.5 GB) | `/usr/bin/time -v cargo rustc --release -p world-server --lib` |
| `world-server` lib wall clock | 1 m 48 s | same run |
| Types laid out in that unit | 61,817 | `-Zprint-type-sizes` |
| Distinct async futures | 522 | same |
| `PlayerInventoryStorage` | **128,600 B** | same |
| ├ `.bags` | 125,208 B (97 %) | same |
| ├ `.items` | 3,384 B | same |
| └ `.current_buyback_slot` + padding | 8 B | same |
| `PlayerBagStorage` | 888 B | same |
| `Player` | 22,160 B | same |

Reproduce with:

~~~bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_BUILD_JOBS=1
touch crates/world-server/src/app.rs
RUSTC_BOOTSTRAP=1 cargo rustc --release -p world-server --lib -- -Zprint-type-sizes \
  | grep -A4 'type: `wow_entities::PlayerInventoryStorage`'
~~~

`RUSTC_BOOTSTRAP=1` is a diagnostic escape hatch for this measurement only. It must not
appear in build scripts, CI, or any committed command.

## The defect

`crates/wow-entities/src/player/mod.rs:3336`

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInventoryStorage {
    pub items: [Option<ObjectGuid>; PLAYER_SLOT_END],        // 141 × 24 B
    pub bags: [Option<PlayerBagStorage>; PLAYER_SLOT_END],   // 141 × 888 B
    pub current_buyback_slot: u8,
}
~~~

`bags` reserves an 888-byte record for all 141 player slots. The same file's
`is_bag_storage_slot` already states which slots may hold a bag:

| Range | Constant | Slots |
|---|---|---|
| Equipped bags | `INVENTORY_SLOT_BAG_START..END` = 30..34 | 4 |
| Reagent bag | `REAGENT_BAG_SLOT_START..END` = 34..35 | 1 |
| Bank bags | `BANK_SLOT_BAG_START..END` = 87..94 | 7 |
| | | **12 of 141** |

129 slots can never hold a bag, so **114,552 bytes per player are unreachable by
construction** — 91 % of the field, 89 % of the struct.

`Copy` on a 128,600-byte type compounds it: every assignment and every by-value parameter
is a silent 125 KB `memcpy`, `PartialEq` compares 128,600 bytes, and `Debug` emits
formatting code for the whole array. `PlayerInventoryStorage` is passed by value at four
sites in `object_accessor/ops_1.rs`.

## C++ anchor

`/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`

| Line | Code | Meaning |
|---|---|---|
| 615 | `PLAYER_SLOT_END = 141` | identical to the Rust constant |
| 671–672 | `INVENTORY_SLOT_BAG_START = 30, _END = 34` | identical |
| 677–678 | `REAGENT_BAG_SLOT_START = 34, _END = 35` | identical |
| 695–696 | `BANK_SLOT_BAG_START = 87, _END = 94` | identical |
| 2950 | `Item* m_items[PLAYER_SLOTS_COUNT];` | **141 pointers = 1,128 bytes** |
| 1314 | `Bag* GetBagByPos(uint8 slot) const;` | bag resolved from the item, by pointer |

The slot constants match exactly, so the slot model is already faithful. The divergence is
purely in representation: C++ holds 8 bytes per slot and reaches the bag through a pointer;
the Rust port inlines 888 bytes per slot. Shrinking the field moves the port **towards** the
target, not away from it.

## Design decision

| Option | `bags` size | Struct | Indexing | `Copy` |
|---|---|---|---|---|
| **A** — compact 12-entry array + slot→index map | 10,656 B | ~14,048 B | needs a mapping at every site | kept |
| **B** — `[Option<Box<PlayerBagStorage>>; 141]` | 1,128 B | **~4,520 B** | unchanged, absolute slot | must be dropped |

**Decision: option B.** It is the smaller result, it is what the C++ target actually does
(a pointer per slot), and it leaves all 17 `.bags` index expressions untouched — no
slot-arithmetic is introduced, so no new off-by-one surface. The cost is dropping `Copy`,
which is desirable on its own terms: a 125 KB implicit `memcpy` is not something this type
should offer. At most 12 boxes exist per player, allocated only when a bag is equipped.

Option A is the fallback if dropping `Copy` turns out to reach further than the survey below
suggests. Record the reason here before switching.

## Blast radius

Surveyed at `81c74731` with `rg`:

| Surface | Count | Where |
|---|---|---|
| `.bags[...]` index expressions | 17 | 12 in `wow-entities/src/player/items/storage.rs`, rest spread over 4 files |
| `PlayerInventoryStorage` mentions | 25 | mostly `object_accessor` construction |
| Sites relying on `Copy` of the inventory | ~47 | to be classified in Phase 0 |

## Execution

Behaviour must not change in any phase. Per the guide, a latent bug found along the way is
repaired in its own commit, never folded into this refactor.

### Phase 0 — classify the `Copy` dependents — DONE 2026-09-20

The ~47 figure was a grep artefact. The real count is **four production sites plus the
`Default` impl**; everything else already borrows and compiles unchanged against
`Option<Box<_>>`, because `Option::as_ref`/`as_mut` yield `&Box<T>`/`&mut Box<T>` and
method calls auto-deref.

Sites that **need no change** (already borrowing):

| Site | Form |
|---|---|
| `wow-entities/.../items/enchantment.rs:191` | `.get(..).and_then(Option::as_ref)` |
| `wow-entities/.../items/storage.rs:331` | `.get(..).and_then(Option::as_ref)` |
| `wow-entities/.../items/storage.rs:1757` | `.get(..).and_then(Option::as_ref)` |
| `wow-entities/.../items/storage.rs:1864` | `.get(..).and_then(Option::as_ref)` |
| `wow-entities/.../items/storage.rs:2664` | `.get_mut(..).and_then(Option::as_mut)` |
| `wow-entities/.../items/storage.rs:2687` | `.get_mut(..).and_then(Option::as_mut)` |
| `wow-entities/.../items/storage.rs:2764` | `.get_mut(..).and_then(Option::as_mut)` |
| `wow-entities/.../items/storage.rs:2827` | `.get_mut(..).and_then(Option::as_mut)` |
| `wow-entities/.../items/storage.rs:2857` | `.get_mut(..).and_then(Option::as_mut)` |
| `wow-entities/src/player/mod.rs:3999` | `.get(..).and_then(Option::as_ref)` |
| `wow-world/.../player_items/storage_bags.rs:34` | `.get(..).and_then(Option::as_ref)?` |
| `wow-world/.../tests/scenarios_spell_state_3.rs:267` | `.get(..).and_then(Option::as_ref)` — fixture |
| `wow-entities/.../items/storage.rs:1705` | `= None` assignment |

Sites that **need editing**:

| Site | Current | Change | Kind |
|---|---|---|---|
| `storage.rs:1545` | `Some(PlayerBagStorage::new(..))` | wrap in `Box::new` | construction |
| `storage.rs:2842` | `bags[bag as usize].map(..)` | `.as_ref().map(..)` | borrow |
| `storage.rs:3288` | `let Some(bag) = bags[slot] else` | `.as_ref()` | borrow |
| `mod.rs:3353` | `.filter_map(\|bag\| *bag)` | `.filter_map(Option::as_ref)` | borrow |
| `mod.rs:3361` (`Default`) | `bags: [None; PLAYER_SLOT_END]` | `[const { None }; PLAYER_SLOT_END]` | array repeat needs `Copy` |

Two further facts found while surveying:

- `Player` already stores `inventory: Box<PlayerInventoryStorage>`
  (`wow-entities/src/player/mod.rs:3518`), which is why `Player` measures 22,160 B rather
  than 128,600 B. The inline copy lives in `object_accessor/state.rs:56`.
- No site copies the whole inventory through a deref (`*inventory`), so dropping `Copy`
  has no hidden value-semantics dependents. The by-value parameters in
  `object_accessor/{state,ops_1}.rs` are ownership transfers and stay valid as moves.

### Phase 1 — box the bag records

Change `bags` to `[Option<Box<PlayerBagStorage>>; PLAYER_SLOT_END]`, drop `Copy`, add
`Clone`, and fix the sites Phase 0 identified. Keep every index expression as it is.

**Done when:** `cargo check -p wow-entities -p wow-world -p world-server` is clean, and
`PlayerInventoryStorage` measures < 8,000 bytes by the command in the baseline section.

### Phase 2 — acceptance

~~~bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_BUILD_JOBS=1
cargo fmt --all -- --check
git diff --check
./tools/validation-v2 final --base origin/3.4.3 --timings
~~~

**Done when:** the campaign passes within 600 s and no inventory, bag or banking test was
edited to make it pass.

### Phase 3 — record the result — DONE 2026-09-20

**Representation, `-Zprint-type-sizes` on `world-server --lib`:**

| | Before | After | Change |
|---|---|---|---|
| `PlayerInventoryStorage` | 128,600 B | **4,520 B** | −96.5 % |
| └ `.bags` | 125,208 B | **1,128 B** | −99.1 % |
| └ `.items` | 3,384 B | 3,384 B | unchanged |

1,128 B is exactly 141 pointers — the same footprint as the C++ `Item* m_items[141]` the
field now mirrors. Goal criterion 1 (< 8,000 B) is met.

**Projected runtime saving**, inventory storage only:

| Players | Before | After |
|---|---|---|
| 1000 | 122.6 MB | 4.3 MB |
| 3000 | 367.9 MB | 12.9 MB |

**Compile-time memory: no improvement.** The `world-server` lib peaked at 3,720,224 KB
after the change against 3,660,936 KB before — 1.6 % higher, inside run-to-run noise. The
expectation that a smaller type would lower compile-time memory did not hold; the 3.5 GB
ceiling is set by the unit's overall size (61,817 types, 522 futures), not by this struct.
Wall clock is not comparable between the two runs (different `CARGO_BUILD_JOBS` and
concurrent load) and is not claimed either way.

The delivered win is runtime footprint and the removal of an implicit 125 KB `memcpy`.
The compile-time motivation is withdrawn, not deferred.

**Test evidence** (this host, `CARGO_BUILD_JOBS=4`):

| Suite | Result |
|---|---|
| `cargo test -p wow-entities --lib` | 946 passed, 0 failed |
| `cargo test -p wow-world --lib` | 4041 passed, 0 failed |
| `cargo test -p world-server --lib` | 597 passed, 0 failed |

No test was edited. One run of the `wow-world` suite reported
`scenarios_world_entities_34::legacy_creature_melee_tick_once_splits_creature_victim_damage_like_cpp`
as failed while other builds were running concurrently. It passes in isolation and in four
subsequent full-suite runs — two with this change applied and two with it stashed — so it is
recorded as a flake in an unrelated area, not a regression. It is not silently dropped: if it
recurs, it needs its own issue.

### Phase 4 — blocked: the acceptance gate is already red on base

`./tools/validation-v2 final --base origin/3.4.3 --timings` fails at
`check_architecture.py hotspot-ratchet`, which reports 17 grown entries including
`wow-map/src/map/mod.rs` (+363), `wow-world/src/handlers/character/mod.rs` (+1,697) and
`world-server/src/lib.rs` (+569) — files this change does not touch.

Running the same check with this change stashed produces **the same 17 entries**, so the
gate is red on `3.4.3` itself: `tools/architecture/runtime-ownership-ledger.json` has drifted
behind the branch. This change contributes 16 lines to one already-failing entry
(`wow-entities/src/player/mod.rs` 16,832 → 16,848) and introduces no new one.

Issue #299, "[ARCH.LEDGER] Resync architecture-issue-ledger and re-arm the two guards its
drift disables", is **closed**, so the drift it fixed has returned. Re-arming the guard is
its own work and is not folded in here.

The checks that do run all pass: physical source ratchet, trailing whitespace,
`cargo fmt --all --check`, `git diff --check`.

**Two notes on the documented environment, found while running this:**

- `AGENTS.md` gives `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`. On this host the path is
  `/home/cdmonio/.local/protoc/bin/protoc`. The `/home/cdmonio/.local/bin/protoc` on `PATH`
  is libprotoc 24.4 and the runner rejects it; it requires 28.3.
- `physical-file-policy.json` pins `crates/wow-entities/src/player/items/storage.rs` at
  `ceiling_lines: 3298` with `observed_lines: 3298`, so the file cannot grow by even one
  line. This is why `PlayerBagStorage::boxed` and `PlayerInventoryStorage::bag_at` live in
  `player/mod.rs`, which has headroom: the call sites stay one line each and the file lands
  back on 3,298 exactly. The ceiling was not raised.

## Risks

- **Allocation in a hot path.** Bags are created on equip, not per tick, so the allocation
  is cold. If a profile later shows otherwise, option A remains available.
- **Serialization coupling.** If any persistence or packet path relies on the inventory
  being `Copy` or on its byte layout, Phase 0 must surface it before Phase 1 starts.
- **`Default` construction.** `PlayerInventoryStorage::default()` currently builds a
  128,600-byte value on the stack. Boxing removes that, but check no caller depends on the
  struct being constructible in a `const` context.

## Out of scope

Recorded here so they are not lost, each needing its own issue and its own C++ anchor:

- `session_factory::create_session` produces a **49,584-byte future**; tokio wraps it in a
  49,792-byte per-task `Cell`. Sibling session futures are 42,864 B and 42,752 B.
- `app::run_inner` is a single `async fn` of **5,645 lines** with 151 `.await` points,
  561 `let` bindings and 36 levels of nesting.

Neither is touched by this plan.

## Open process item

Commit `81c74731` (the build-profile change that produced these measurements) was pushed
directly to `3.4.3` at the user's explicit request. GitHub reported
`Bypassed rule violations`: the branch requires a pull request and 2 status checks, and
both were skipped. This plan's own work must follow the normal route — issue branch via
`gh issue develop <N> --base 3.4.3 --checkout`, PR into `3.4.3` with `Closes #<N>`.
