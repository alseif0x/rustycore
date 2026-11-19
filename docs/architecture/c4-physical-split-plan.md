# #584:C4 physical-file legacy rows: analysis and sliced plan

Owner: GitHub #584 (required core, checkpoint C4), feeding the independent audit #153.
Measured on `origin/3.4.3` at `aef82502a` (2026-10-20). This note covers only the physical-size
debt tagged `584:C4` in `tools/architecture/physical-file-policy.json`. It is a plan, not a
completion claim. It does not close C4.

## How a legacy row is evaluated

`tools/architecture/physical_files.py` is the only enforcer.

- **What is counted.** It counts every tracked or untracked-unignored file with a source
  suffix (`SOURCE_SUFFIXES`), plus any file with a `#!` shebang. The line count is
  `len(text.splitlines())`, which includes comments and blank lines. SQL, documentation and
  data files are not counted.
- **Budget.** Every handwritten file has one budget: `TERMINAL_LIMIT = 2000` lines, unless a
  `legacy` or `exceptions` row replaces it with its own ceiling. `REVIEW_LIMIT = 1000` only
  feeds the cohesion-review counter. Budgets do not differ by category: production, test,
  tool and script files share the same 2000.
- **Row validation.** A row must have exactly these keys: `path`, `observed_lines`,
  `ceiling_lines`, `split` and `review_checkpoint`. It also needs `ceiling <= observed`, a
  non-empty split text and an existing path. A missing path fails as "audited physical path
  missing".
- **Stale rows go undetected.** If a file has shrunk under its ceiling, or under 2000, its row
  is tolerated silently. Nothing requires tightening it.
- **Closing C4.** If `584:C4` is added to `completed_checkpoints`, every row still tagged with
  it fails as expired. C4 can therefore close only once no row carries that tag.
- **`--terminal`.** This rejects any legacy row whose file is still over 2000 lines.

`tools/architecture/session_method_index.py` (`shim_row_errors`) also requires every
`crates/wow-world/src/handlers/*_test_shims.rs` file to have a `legacy` row. validation-v2
runs that check only when a `crates/wow-world/src/` path or an index file changes.

## Classification of the 140 rows

All 140 paths exist and all are tagged `584:C4`. No file has grown past its ceiling.

| Class | Rows | Treatment |
|---|---:|---|
| A: at or under 2000, stale | 83 | Retired in the C4 retire PR |
| A-shim: `handlers/*_test_shims.rs` | 44 | Kept: the shim-row rule requires them (see "Closing C4") |
| B: over 2000 but under the ceiling | 2 | Ceiling tightened in the C4 retire PR; split still owed |
| C: over 2000 at the ceiling | 11 | Real split work (slices below) |

The 83 retired rows include three rows whose split text carried an obligation for a child
file. That child file has no row of its own, so the obligation moves here:

- **`crates/wow-database/src/statements/character/`.** `identities.rs` (1791 lines) and
  `statement_def.rs` (1629) still owe the semantic separation of the statement catalogue.
- **`tools/architecture/handler-contract-check/src/persistence_access/body_visit.rs`.** At
  1686 lines on `0aba84cee` (1694 once PR #1447 lands, +8 lines for the #1437 generic
  guard hook) it needs a cohesion review. The retired row's text: "one Visit trait impl can
  delegate to private expression analyzers while preserving traversal order".
- **`tools/wow-test-bot/src/bot/void_storage_2.rs`.** At 1648 lines it has grown past the 1573
  its parent row recorded, and it has no guard.

All three are under 2000, so they pass the terminal budget. They stay in the >1000 cohesion
review.

Five more retired rows are files still between 1000 and 2000 lines. Their rows carried a
split or no-growth obligation of their own, which no longer has a guard; it is carried here
verbatim in substance (line counts at `0aba84cee`):

- **`crates/wow-world-inventory/src/vendor_session/buy.rs` (1075).** One function, the body of
  C++ `Player::BuyItemFromVendorSlot` (`handle_buy_item_with_generator_like_cpp`), moved
  verbatim by #1363 slice 8b. Split it by complete purchase phase (admission and price,
  extended-cost and currency plan, item store and publication) in a behaviour-preserving
  refactor that keeps the money boundary and publication order. The row was ratcheted at
  the live value by #1363 slice 8c: no growth is permitted.
- **`crates/world-server/src/runtime/map.rs` (1295).** Keep the remaining respawn/startup
  projections and recipient delivery scoped, with no I/O under map guards; the canonical
  World/Map producer already lives in `map/update_loop.rs` (#787). The row was tightened to
  the observed size on 2026-09-25: no growth is permitted.
- **`crates/capture-diff/scripts/capture-rust.sh` (1810).** Separate the runtime swap, the
  packet-dump provenance and the recovery orchestration, preserving the existing
  capture/fixture safeguards and the explicit runtime authorization.
- **`crates/world-server/src/main_tests.rs` (1533).** Separate the configuration,
  composition, shutdown, session-supervision and runtime-timing suites with narrow fixture
  builders. #628 already moved the test functions into bounded `main_tests/scenarios_*.rs`
  modules; only shared helpers stay here, because a child module sees its parent's private
  items. A new scenario module adds only its three mount lines (comment, `#[path]`, `mod`).
- **`crates/world-server/src/spawn_store_loader_tests.rs` (1007).** Move the catalog-specific
  loader fixtures and negative cases beside their loader responsibilities; #628 already moved
  the test functions into bounded scenario modules, keeping only shared helpers here.

## Remaining split work

The C and B rows exceed the 2000 budget by 15,958 lines. Each slice is one PR, except
C4-8, which lands in several staged PRs (C4-6 and C4-7 are already split into a/b slices).
Each PR retires or tightens its own rows in the same change. Each split is a move with no behaviour change:
R1/R2 apply, and the test count must be the same before and after.

| Slice | Paths | Lines over 2000 | Effort | Cut |
|---|---|---:|---|---|
| C4-1 | `crates/wow-social/src/group/tests.rs` (2508) | 508 | S | Test-only (80 tests). Split into `group/tests/{membership,leader_looter,ready_check,load_db,target_icons}.rs`, mirroring the production modules. |
| C4-2 | `crates/world-server/src/spell/acquisition_loader.rs` (2267), `crates/world-server/src/lib.rs` (2121, B) | 388 | S | Move the inline `#[cfg(test)] mod tests` of the loader (about 700 lines, 23 tests) to a sibling file. Move `RespawnDbRetryQueue` with `spawn_respawn_db_writer`, or `ActiveWorldSessionRegistryLikeCpp`, out of `lib.rs` into a private module. |
| C4-3 | `crates/wow-recastdetour/vendor/Detour/Source/DetourNavMeshQuery.cpp` (3663) | 1663 | S | Vendored upstream code: do not split. Either turn it into an `exceptions` row with a non-C4 tracking issue and bounded dates, or implement the vendored exemption that structure-and-conventions §10 describes, with `test_physical_files.py` cases. |
| C4-4 | `crates/wow-map/src/map/{spawn_groups,game_object,relocation}.rs` (2384 / 2311 / 2176) | 871 | M | Each is a single `impl Map` block. Move `update_game_object_with_optional_pool_update_like_cpp` (941 lines), the summon/owner-slot functions (about 720) and the move-list family (about 350) into child `impl` modules. Review `pub(super)` visibility. |
| C4-5 | `crates/capture-diff/scripts/{detour-chase,creature-spell-casting}-fixture-common.sh` (3605 / 2144) | 1749 | M | Split into sourced sub-libraries: snapshot, journal, restore and validate. Update the sourcing in `capture-*.sh`/`recover-*.sh` and the `sed`/`bash -n` path pins in `crates/capture-diff/tests/*_fixture_common.sh`. These scripts carry runtime-authorization safeguards, so make no behaviour change. |
| C4-6a | `crates/wow-entities/src/player/mod.rs` (4726) | 2726 | L | Move the lifecycle/load records, the item position/args/swap/destroy plan types and the enchantment types plus functions (about 1490 lines) to `records.rs`, `items/plans.rs` and `items/enchantment*.rs`. |
| C4-6b | `crates/wow-entities/src/player/items/storage.rs` (3298) | 1298 | L | Split by phase: admission (`can_store`/`can_bank`/`can_use`), swap plans, destroy/remove, store/bag-object, and buyback/durations/send-new-item. This overlaps `player-inventory-storage-plan.md`, so coordinate the two. |
| C4-7a | `crates/wow-map/src/map/mod.rs` (5133), first half | ~1900 | L | Move the outcome/status DTOs and the relocation-plan types into `map/outcomes*.rs` and `map/plans.rs`, re-exported with `pub use`. |
| C4-7b | `crates/wow-map/src/map/mod.rs`, second half | ~1233 | L | Split the 1460-line `impl Map` into child `impl` modules. |
| C4-8 | `crates/world-server/src/app.rs` (5622, B) | 3622 | L (multi-PR) | One `async fn run_inner` of about 5590 lines. Stage it into typed results, as its module doc requires: config/catalog bootstrap, dependency construction, then task supervision/shutdown. Keep the C++ `Main.cpp` order. |

Suggested order: C4-1, C4-2, C4-3, C4-4, C4-5, C4-6a, C4-6b, C4-7a, C4-7b, C4-8. The pieces
should land at 1000 lines or less, so they do not add new cohesion-review flags.

On `forever`, C4-1 organizes the 80 existing Group tests into private
membership, instance, leader/looter, ready-check, database-load and target-icon
modules. The database-load tests retain the real DB2-backed difficulty
validator. The Group registry, production modules and public API stay intact.
Against base `10b0ea6b3`, all 80 test definitions, including the expected-panic
attribute, and the test-only validator implementation match byte for byte.
The test root shrinks from 2,508 to 31 lines. The six children range from 104
to 808 lines. The legacy physical row is retired. Mounts, headers and imports
add 62 test lines to the same logical Group owner.

Committed candidate `776ad9c667d5bc2bea0c7ba81e9ee733e903ed63` passed
`./tools/validation-v2 final --base 10b0ea6b3 --architecture --require-changes --timings --logs`
with Protoc 28.3 and one Cargo job on the warm x86_64 checkout. The campaign
ran from 11:51:22Z to 11:58:52Z on 2026-11-19 (450.343 seconds). All 10
commands passed, including downstream test-target checks and all 80
`wow-social` tests, with zero failures or ignored tests. The execution log
matches every original test after mapping its module path and confirms the
expected-panic case. The verified manifest is
`target/validation-v2/manifests/20261119T115122.159416Z-2818247-final.json`.

On `forever`, the C4-6a lifecycle/load record split moves ten public types and
their existing implementations to the private `player/records.rs` module. The
root shrinks from 4,713 to 4,397 lines and the new module has 333. The logical
Player owner retains all descendants and gains 17 lines of module wiring.
On base `637f02e38`, the moved source passed the worker's offline locked
`wow-entities` check and five existing lifecycle/load-plan tests.

The `forever` item-plan split moves 47 type definitions and their existing
methods verbatim into `player/items/plans.rs`. The root shrinks from 4,397 to
3,806 lines. The new module has 614 lines, and the same logical Player owner
gains 24 production lines for module wiring, imports and re-exports.
On base `440636916`, the moved source passed the worker's offline locked
`wow-entities` check, 124 item tests and two swap-orchestration tests.

The `forever` enchantment split moves 27 types into the private
`player/items/enchantment_records.rs` module and moves the duration-slot table
and six helpers to the existing `player/items/enchantment.rs`. The item-bonus
caller uses the same stat helper through a Player-scoped import. The root
shrinks from 3,806 to 2,974 lines. The destination files have 524 and 806 lines,
respectively, and the logical owner gains 21 production lines of wiring. The
random-suffix array length constant retains its existing Player path.
On base `0f83e9822`, the corrected source passed the worker's offline locked
`wow-entities` check, 53 enchantment tests and two item-stat bonus tests.

The `forever` Player root closeout relocates construction, item bonus action
planning, equipment/store helpers and stack destruction/split planning.
Nine source regions match the base apart from scoped visibility and signature
formatting. The root shrinks from 2,974 to 1,932 lines and its legacy physical
row is retired. The changed child files are all below 1,000 lines. The same
logical owner gains 68 production lines of wiring, headers, and imports.
The moved source passed the locked offline `wow-entities` check against
base `4ce6d2d18` after correcting its imports.
Full C4 acceptance and the remaining logical owner obligations continue.

On `forever`, the C4-6b split starts from `e54b871d6`, where prior inventory
work has already reduced `player/items/storage.rs` to 1,908 lines. The retained
admission methods still delegate to `ItemStorageView`. The placement, removal,
swap, buyback, item tracking and new-item projection methods remain inherent
methods on Player in private storage children.

The root now has 431 lines. Its six new children have 318 (placement), 434
(removal), 473 (swap), 157 (buyback), 128 (item tracking) and 79 (publication).
The 45 moved and 44 retained method definitions match the base byte for byte.
The legacy storage row is retired. Headers, mounts and imports add 113 lines
to the same Player owner, including two test-attributed import lines. The
locked offline `wow-entities` check passed after retaining two imports under
`cfg(test)` in the Player root.

The retained target anchors are `Player.cpp` at `02245dcd` for
`StoreItem`/`_StoreItem` (11618/11646), the ordered removal, transfer and
destruction family (12164–12683), `SwapItem` (12824), buyback (13210/13272),
trade and duration updates (13488/13511), and
`SendItemDurations`/`SendNewItem` (14076/14092). The move retains the existing
Rust phase order, side effects, and represented gaps. The logical Player
owner remains open under #584.

Committed candidate `a8deeada7876b66461bd42c200320b87f8e63e9e` passed
`./tools/validation-v2 final --base e54b871d6 --architecture --require-changes --timings --logs`
on the warm x86_64 checkout with Protoc 28.3 and one Cargo job. The campaign
ran from 16:36:10Z to 16:41:22Z on 2026-11-18 (311.908 seconds).
It passed all 10 commands, including downstream test-target checks and all
1,309 `wow-entities` tests, with zero failures or ignored tests. The verified
manifest is `target/validation-v2/manifests/20261118T163610.086177Z-2321107-final.json`.

## Closing C4

1. Land C4-1 … C4-8 so that no `584:C4` row remains above 2000.
2. Move the 44 shim rows off `584:C4`. Either re-tag them to the shim retirement owner, or
   make `shim_row_errors` read a separate registry, and update its self-test to match. The
   rows exist to ratchet test shims, not to track physical debt.
3. Add `584:C4` to `completed_checkpoints`. Then `physical-files --terminal` must pass.
