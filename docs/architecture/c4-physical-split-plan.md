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
  1686 lines it needs a cohesion review.
- **`tools/wow-test-bot/src/bot/void_storage_2.rs`.** At 1648 lines it has grown past the 1573
  its parent row recorded, and it has no guard.

All three are under 2000, so they pass the terminal budget. They stay in the >1000 cohesion
review.

## Remaining split work

The C and B rows exceed the 2000 budget by 15,958 lines. Each slice is one PR. Each PR retires
or tightens its own rows in the same change. Each split is a move with no behaviour change:
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

## Closing C4

1. Land C4-1 … C4-8 so that no `584:C4` row remains above 2000.
2. Move the 44 shim rows off `584:C4`. Either re-tag them to the shim retirement owner, or
   make `shim_row_errors` read a separate registry, and update its self-test to match. The
   rows exist to ratchet test shims, not to track physical debt.
3. Add `584:C4` to `completed_checkpoints`. Then `physical-files --terminal` must pass.
