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
- **Vendored sources.** An exact `vendored` row records the file path, upstream URL and
  SHA-256. Matching content stays in the inventory with `vendored` kind, outside size and
  cohesion budgets even in terminal mode. Rows cannot overlap another policy category or
  exempt siblings; content drift fails closed until the provenance is reviewed and rehashed.
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

On `forever`, C4-2 starts at `dfb4e3bff`. The acquisition loader already has a
private test module with 23 tests. The entire module moves to a sibling file
under the same test path. The acquisition catalog, its entry points and
startup order are preserved. The production prefix is byte-for-byte, and the
test module matches the former body after dedent and rustfmt. The loader
shrinks from 2,302 to 1,604 lines and its test sibling has 688 lines.

The world-server library starts at 1,995 lines after the earlier account
lookup extraction. C4-2 moves the active-session registry, registration
guard and cancellation into one private module. The session factory, map
loop and shutdown keep their existing calls, readiness checks and lock
boundaries. The retained authority is `World::AddSession_`,
`World::KickAll` and `World::UpdateSessions` in target `World.cpp` at
`02245dcd` (362, 2629 and 2948). The registry remains part of the same
world-server logical owner. Its bodies match the original after parent-scoped
visibility and rustfmt. The source guard for the creature runtime now also
scans the moved file. The library root shrinks from 1,995 to 1,777 lines, the
registry module has 243, and both legacy physical rows are retired. The
logical world-server row adds 25 production lines and removes nine test lines.

Committed candidate `78eda509ddf1d2443137d72e10038a835f9f1bbe` passed
`./tools/validation-v2 final --base dfb4e3bff --architecture --require-changes --timings --logs`
with Protoc 28.3 and one Cargo job on the warm x86_64 checkout. All 11 commands
passed from 12:43:50Z to 12:55:56Z on 2026-11-19 (726.392 seconds), exceeding
the 600-second target. Test-target compilation took 250.298 seconds, the
`world-server` library suite took 244.536 seconds, and the SQL module compositor
check took 140.892 seconds and restored the checkout. The library passed
645 tests with zero failures and one existing ignored test. All 646 test names
and statuses match the November 17 baseline with identical world-server source
before this split, including the 23 acquisition-loader tests. Verified manifest:
`target/validation-v2/manifests/20261119T124350.413750Z-2846039-final.json`.

On `forever`, C4-3 replaces the Detour legacy row with an exact vendored
attribution. Commit `d7d9959fbd77620a231467ea5dff7ae2d1885d44` introduced
`DetourNavMeshQuery.cpp` as part of the legacy TrinityCore
`dep/recastnavigation/Detour` copy. Its 3,663 lines and SHA-256
`b4c0de0d3066586f23fbc70cb8249e7dd899803a65c5b6e34d398a301c76a3e8`
are unchanged since that commit and match the upstream TrinityCore `3.3.5`
file byte for byte. The policy records that exact upstream URL and hash.
The checker keeps the file in the inventory as `vendored`, exempts only that
path from size and cohesion budgets, rejects provenance drift and overlapping
rows, and leaves sibling and handwritten wrapper files under the ordinary
budget. Focused fixtures cover terminal acceptance, both path boundaries,
content drift, missing upstream metadata and legacy-row overlap. The C++ file
is byte-identical before and after C4-3.

Committed candidate `36bbc132ead698eda5a598282744dac8677668a0` passed
`./tools/validation-v2 final --base 3e94ff912 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eight commands passed from 15:44:29Z to 15:45:59Z on 2026-11-19
(89.789 seconds), within the 600-second target. The physical suite passed
29 tests, including the five new vendored cases, and the architecture-command
suite passed eight. The combined architecture check and syntax-ownership check
also passed. Verified manifest:
`target/validation-v2/manifests/20261119T154429.599026Z-2930023-final.json`.
Two earlier preflight invocations executed no checks: the historical Protoc
path was absent, then Rust was missing from the shell PATH. Using the documented
Protoc 28.3 binary and the pinned Rust toolchain resolved the environment
without changing the candidate or claiming either preflight as acceptance.

On `forever`, C4-4 starts at `d73c18842` and performs three same-owner Map
moves. `spawn_groups.rs` shrinks from 2,380 to 1,441 lines: its 941-line
optional-pool GameObject update moves to `spawn_groups/game_object_update.rs`
(955 lines). `game_object.rs` shrinks from 2,309 to 1,589 lines: the 722-line
summon/owner-slot family moves to `game_object/summon.rs` (736 lines).
`relocation.rs` shrinks from 2,176 to 1,825 lines: the 352-line move-list
family moves to `relocation/move_list.rs` (367 lines). All three legacy rows
are retired; every child is below 1,000 lines.

The moved regions match the base byte for byte except canonical rustfmt and
one visibility spelling. `pub(in crate::map)` preserves the former `pub(super)`
access for map siblings after the optional-pool update becomes a grandchild.
No caller, test definition, public method name or body order changes. The same
logical Map owner gains 48 production lines for headers, mounts and impl
wrappers; its reviewed ratchet records exactly +48 production/+0 tests/+48
total, with no general headroom or semantic retirement. The locked offline
`wow-map --all-targets` check passed on base `d73c18842`; its warnings are in
existing dependency files outside this move.

Committed candidate `fb04c41a4e1c337afb7d967816e97b6a1c4c4f53` passed
`./tools/validation-v2 final --base d73c18842 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All ten commands passed from 16:04:07Z to 16:11:52Z on 2026-11-19
(465.291 seconds), within the 600-second target. Affected test-target
compilation took 333.157 seconds. The `wow-map` library passed 774 tests with
zero failures and one existing ignored test. Independent source comparison
preserves every moved body, all 123 affected method definitions and all 799
source test identities. R1 is not applicable because no `wow-world/src` root
shrank. Verified manifest:
`target/validation-v2/manifests/20261119T160407.507359Z-2940155-final.json`.

The first candidate `07c24eacf1c900c0af9fb933003e770f7a832a87` stopped after
77.982 seconds when the logical Map ratchet rejected the reviewed +48 wiring
lines; physical policy and syntax ownership passed. Recording that exact delta
and amending the candidate produced `fb04c41a4`; no method body or test changed
between those candidates. The failed evidence is retained in
`target/validation-v2/manifests/20261119T160106.580451Z-2938316-final.json`
and is not claimed as acceptance.

On `forever`, C4-5 starts at `0002be75d` and splits both source-only fixture
guards into six sourced libraries each. The Detour root shrinks from 3,605 to
162 lines; snapshot, validation, journal, restoration, cleanup and evidence
children have 845, 563, 668, 872, 342 and 154 lines. The creature-spell root
shrinks from 2,144 to 93 lines; validation, snapshot, runtime, journal,
restoration and evidence children have 413, 509, 265, 628, 95 and 142 lines.
Both legacy rows are retired and every child is below 1,000 lines.

Each root loads its children through the same relative `BASH_SOURCE` boundary
used by the existing capture wrappers. The wrappers and recovery entry points
are unchanged. Reconstructing either original from its retained prefix and
ordered children matches byte for byte apart from removing separator blank
lines at child EOFs. Every guard, SQL statement, credential-safe temporary-file
rule and recovery phase is preserved. Only six source lines per root are added.
Tests still source the roots and now syntax-check every
child; their `sed` and security assertions point to the actual snapshot,
journal and evidence owners. No runtime, database or service operation is part
of this split.

Committed candidate `13eac223855655827cdb22683ed91fa783e4fe05` passed
`./tools/validation-v2 final --base 0002be75d --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All 26 commands passed from 16:25:46Z to 16:28:19Z on 2026-11-19
(152.903 seconds). The `capture-diff` library passed all 26 tests with zero
failures or ignored tests. Both offline shell suites also passed at the same
candidate: Detour took 3.448 seconds and creature spell 1.501 seconds. The
complete accepted evidence window, starting with those suites, was 168.987
seconds, within the 600-second target. Verified manifest:
`target/validation-v2/manifests/20261119T162546.371603Z-2959331-final.json`.

The first candidate `37bf3128791b4848d70140e944ba17fcbb14c004` passed
architecture and syntax ownership but failed `git diff --check` after 93.009
seconds because ten new children retained a blank separator line at EOF.
Normalizing those EOFs produced `13eac2238`; no function or assertion changed.
Both shell suites were rerun and passed before the successful campaign. The
failed manifest is retained at
`target/validation-v2/manifests/20261119T162233.879264Z-2954441-final.json`.

On `forever`, C4-7a starts at `a90838079` and moves four DTO regions out of
`map/mod.rs`. The root shrinks from 5,122 to 2,817 lines and its physical
ceiling tightens to that value without retiring the row. `outcomes_update.rs`
has 966 lines, `outcomes_lifecycle.rs` 477, `outcomes.rs` 422 and `plans.rs`
483; each contains its complete source region below 1,000 lines.

Public `pub use` reexports preserve every existing `wow_map::map::*` path.
Restricted types, fields and helpers use `pub(in crate::map)` plus three named
reexports to preserve the former sibling-only access rather than exposing it.
The four regions match the base byte for byte apart from scoped visibility and
EOF formatting. No method body, caller or test definition changes. The same
logical Map owner gains 43 production lines for headers, mounts and reexports;
its ratchet records exactly +43 production/+0 tests/+43 total. The first
`wow-map --all-targets` compile located the restricted items; the corrected
candidate passes that check with warnings only in existing dependency files.
C4-7b still owns the remaining `impl Map` split.

Committed candidate `aa658d6043b2d332f85bf51f41b6d29669ca0d6b` passed
`./tools/validation-v2 final --base a90838079 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All ten commands passed from 16:38:10Z to 16:45:46Z on 2026-11-19
(456.443 seconds), within the 600-second target. Affected test-target
compilation took 346.950 seconds. The `wow-map` library passed 774 tests with
zero failures and one existing ignored test. Source comparison preserves all
167 affected type definitions and all 799 source test identities. R1 is not
applicable because no `wow-world/src` root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T163810.414742Z-2966769-final.json`.

On `forever`, C4-7b starts at `60c4bc58b` and splits the remaining `impl Map`
into three private modules. The root shrinks from 2,817 to 877 lines and its
legacy physical row is retired. `lifecycle.rs` has 850 lines,
`spawn_runtime.rs` 428 and `object_access.rs` 702. Every method and trait
implementation remains on the same Map type.

Moved source matches the base apart from canonical rustfmt, wrapper lines and
scoped visibility. Private free functions and inherent helpers retain their
original map-wide access through `pub(in crate::map)` and one restricted
`object_access` reexport; trait method signatures stay unchanged. The same
logical Map owner gains 62 production lines: 40 for headers, mounts and the
reexport, plus 22 from canonical rustfmt on widened signatures. Its ratchet
records exactly +62 production/+0 tests/+62 total. The corrected
`wow-map --all-targets` check passes with warnings only in existing dependency
files. C4-7 is complete physically; the logical Map owner remains under #584.

Committed candidate `f715081bd690f02a64da9d052473d5703e0eb6c3` passed
`./tools/validation-v2 final --base 60c4bc58b --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All ten commands passed from 17:01:18Z to 17:08:47Z on 2026-11-19
(448.403 seconds), within the 600-second target. Affected test-target
compilation took 342.082 seconds. The `wow-map` library passed 774 tests with
zero failures and one existing ignored test. Token comparison preserves all
80 moved method definitions and all 799 source test identities. R1 is not
applicable because no `wow-world/src` root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T170118.917983Z-2980469-final.json`.

The first candidate `81766521e4a99545feb8c65bfff2bc34360b3828` stopped after
69.032 seconds when the ratchet rejected the initial +40 estimate. The actual
measured delta is +62, including 22 canonical rustfmt lines. Recording that
measurement produced `f715081bd`; no method or test changed between candidates.
The failed manifest is retained at
`target/validation-v2/manifests/20261119T165834.090842Z-2978613-final.json`.

On `forever`, C4-8a starts at `680d44aab` and extracts the first typed
bootstrap stage. `app.rs` shrinks from 5,709 to 5,584 lines and its physical
ceiling tightens without retiring C4-8. The new private `databases.rs` has 191
lines and returns `StartupDatabases`: all four database connections, their
migration-schema validation and nine catalog adapters. Connection, validation,
construction and failure-drop order match the original token for token.

The app destructures the result at the original point before hotfix Arc and
RealmList setup. No database, service, gameplay owner or runtime operation
changes. The same logical world-server owner gains 66 production lines for the
typed result, function wrappers and canonical rustfmt. Five source guards add
26 test lines to compose app and database stages at the extraction point while
preserving every assertion and all 769 source test identities. The ratchet
records +66 production/+26 tests/+92 total. The `world-server` library check
and all five affected guards pass after these corrections. Later C4-8 stages
still owe catalog/dependency extraction and supervision/shutdown.

Committed candidate `be2429ae85b53e388ed0a3c14ba5e2a942fd9403` passed
`./tools/validation-v2 final --base 680d44aab --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 17:35:53Z to 17:37:37Z on 2026-11-19
(104.435 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T173553.355707Z-3005195-final.json`.

The first candidate `25df643c3d8eaa85e5e3a0f500df2d4b75fc9b2c` ran 519.581
seconds and failed five source guards whose `include_str!` anchors still
expected database composition in `app.rs`. Moving those anchors and composing
app/database sources at the extraction point produced `be2429ae8`; all five
focused guards then passed before the successful campaign. The failed manifest
is retained at
`target/validation-v2/manifests/20261119T172046.106687Z-2992217-final.json`.

On `forever`, C4-8b starts at `3b4738c8b` and extracts the initial item
catalog. `app.rs` shrinks from 5,584 to 5,475 lines and its physical ceiling
tightens without retiring C4-8. The new private `item_catalogs.rs` has 178
lines and returns DataDir, locale and all fifteen initial item stores. Catalog
loading and failure order match the original token for token.

The app destructures `ItemCatalogs` at the original point before hotfix
metadata and later stores. No service, database, gameplay owner or startup
order changes. The same logical world-server owner gains 69 production lines
for the typed result, wrappers and canonical rustfmt, recorded as +69
production/+0 tests/+69 total. The `world-server` library check passes with
warnings only in existing files. Later C4-8 stages still owe remaining catalog
and dependency extraction plus supervision/shutdown.

Committed candidate `212dac25c0d845724597987fb7801d7a0298e854` passed
`./tools/validation-v2 final --base 3b4738c8b --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 17:45:51Z to 17:51:53Z on 2026-11-19
(362.260 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T174551.367659Z-3011925-final.json`.

On `forever`, C4-8c starts at `d421e65da` and extracts world lifecycle
catalogs. `app.rs` shrinks from 5,475 to 5,372 lines and its physical ceiling
tightens without retiring C4-8. The new private `world_lifecycle_catalogs.rs`
has 181 lines and returns graveyard, gossip and its runtime adapter, spawn
groups, creature/gameobject templates, script names and scenes. Six explicit
inputs preserve every dependency and loading/failure order.

The moved body matches the original except fully qualified adapter types,
removing an extra reference at the new boundary and two unused `mut`
qualifiers. Consumer mutability is preserved in app destructuring. The gossip
source guard now reads the adapter/startup loader from the new module and the
runtime clone from app. The `world-server` library check passes with the same
35 existing warnings. The logical owner gains 78 production and one test line,
recorded as +78 production/+1 test/+79 total. Later C4-8 stages still owe
remaining catalogs, dependencies and supervision/shutdown.

Committed candidate `840a2fde7209cca26dd2af5a040736bc5623c17b` passed
`./tools/validation-v2 final --base d421e65da --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 18:08:40Z to 18:14:28Z on 2026-11-19
(347.961 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T180840.340609Z-3024626-final.json`.

On `forever`, C4-8d starts at `0ec60c778` and extracts display, vehicle and
spawn catalogs. `app.rs` shrinks from 5,372 to 5,282 lines and its physical
ceiling tightens without retiring C4-8. The new private
`display_vehicle_catalogs.rs` has 172 lines and returns all fifteen stores,
including the correct `WorldSpawnIdStore` types. Eight explicit inputs preserve
loading and failure order.

The moved body matches the original after adapting references to the new
signature and removing one redundant `as_ref`. The display guard checks the
original display stage before the new app call and locates the model loader in
the new module, without net test growth. The `world-server` library check
passes with the same 35 existing warnings. The logical owner gains 82
production lines for the typed result, wrappers and canonical rustfmt,
recorded as +82 production/+0 tests/+82 total. Later C4-8 stages still owe
skills/traits, remaining dependencies and supervision/shutdown.

Committed candidate `801bdec5ee04ae76c7a5ca9dabb51c54d605bd06` passed
`./tools/validation-v2 final --base 0ec60c778 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 18:26:12Z to 18:31:43Z on 2026-11-19
(331.083 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T182612.435265Z-3034820-final.json`.

On `forever`, C4-8e starts at `84724873b` and extracts skill, trait and talent
catalogs. `app.rs` shrinks from 5,282 to 5,229 lines and its physical ceiling
tightens without retiring C4-8. The new private `skill_trait_catalogs.rs` has
123 lines and returns twelve stores, including traits and glyphs. Five explicit
inputs preserve SkillLine, SkillLineAbility/SkillRaceClassInfo and all later
load/failure order.

The moved body matches the original except adapting `skill_tiers` to the new
reference boundary. The fishing/tiers guard checks the original fishing stage
before the new app call and locates tiers in the new module, adding two test
lines. The `world-server` library check passes with the same 35 existing
warnings. The logical owner gains 70 production and two test lines, recorded
as +70 production/+2 tests/+72 total. Later C4-8 stages still owe remaining
catalogs, dependencies and supervision/shutdown.

Committed candidate `76f5031e802d2296ebb1c2f89408f2984955a38f` passed
`./tools/validation-v2 final --base 84724873b --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 18:50:54Z to 18:56:17Z on 2026-11-19
(322.918 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T185054.076527Z-3048589-final.json`.

On `forever`, C4-8f starts at `2ea043809` and extracts core SpellInfo and pet
spell construction. `app.rs` shrinks from 5,229 to 5,130 lines and its physical
ceiling tightens without retiring C4-8. The new private `spell_core_catalogs.rs`
has 153 lines and returns both reused adapters plus SpellLevels, SpellName and
SpellStore. Eight explicit inputs preserve all dependencies and load/failure
order; pet levelup/default stores are still constructed and logged without
adding unread result fields.

The moved body matches the original after adapting references to the new
signature and removing an unnecessary `mut`. `spell_name_store` keeps its
original non-Arc type. The `world-server` library check passes with the same 35
existing warnings. The logical owner gains 54 production lines for the typed
result, wrappers and canonical rustfmt, recorded as +54 production/+0 tests/
+54 total. Later C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `da2f9e0d77899e7d592372506dbd98d16fc0de18` passed
`./tools/validation-v2 final --base 2ea043809 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 19:21:51Z to 19:27:07Z on 2026-11-19
(316.117 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T192151.593993Z-3065504-final.json`.

On `forever`, C4-8g starts at `e6dc17fba` and extracts spell metadata and
ability catalogs. `app.rs` shrinks from 5,130 to 4,971 lines and its physical
ceiling tightens without retiring C4-8. The new private
`spell_metadata_catalogs.rs` has 232 lines and returns twelve stores:
SpellCategory, AuraOptions, AuraRestrictions, CastingRequirements,
ClassOptions, Label, EquippedItems, TargetRestrictions, Misc, ProcsPerMinute,
Duration and Cooldowns. Seven explicit inputs preserve DataDir, locale,
removals, SpellCore persistence, creature family, skills, SpellLevels and
SpellStore. Pet family spells are still constructed and logged without adding
an unread result field.

The moved body matches the original after adapting references to the new
signature and removing one redundant `as_ref`. The `world-server` library
check passes with the same 35 existing warnings. The logical owner gains 73
production lines for the typed result, wrappers and canonical rustfmt,
recorded as +73 production/+0 tests/+73 total. Later C4-8 stages still owe
addons/scaling, remaining dependencies and supervision/shutdown.

Committed candidate `d83764e9f9ea96d0981e8268fb28ceb752fe5224` passed
`./tools/validation-v2 final --base e6dc17fba --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 19:44:19Z to 19:49:38Z on 2026-11-19
(298.641 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T194419.557204Z-3078132-final.json`.

On `forever`, C4-8h starts at `be324e849` and extracts creature addon,
condition reference and scaling catalogs. `app.rs` shrinks from 4,971 to 4,915
lines and its physical ceiling tightens without retiring C4-8. The new private
`addon_scaling_catalogs.rs` has 130 lines and returns nine stores:
SpellShapeshiftForm, CreatureAddon, ActiveEvent, WorldState, Trainer, Curve,
CurvePoint, ScalingStatDistribution and ScalingStatValues. Twelve explicit
inputs preserve DataDir, locale, both world catalog adapters and all eight
store dependencies.

The moved body matches the original token stream after adapting references to
the new signature. The `world-server` library check passes with the same 35
existing warnings. The logical owner gains 74 production lines for the typed
result, wrappers and canonical rustfmt, recorded as +74 production/+0 tests/
+74 total. Later C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `77d42f43e39d56a87a8d727c2031adbec2c2903a` passed
`./tools/validation-v2 final --base be324e849 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 19:57:23Z to 20:02:16Z on 2026-11-19
(293.049 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T195723.100426Z-3085847-final.json`.

On `forever`, C4-8i starts at `2a5427689` and extracts AreaTrigger template
loading and startup diagnostics. `app.rs` shrinks from 4,915 to 4,727 lines
and its physical ceiling tightens without retiring C4-8. The new private
`area_trigger_template_catalog.rs` has 211 lines and returns the single
AreaTriggerTemplateStore. Four explicit inputs preserve WorldDatabase,
WorldSafeLoc, Curve and the mutable script-name interner; all ten diagnostic
families and the final summary remain in their original order.

The moved body matches the original token stream after adapting references to
the new signature. The `world-server` library check passes with the same 35
existing warnings. The logical owner gains 23 production lines for the
function boundary and canonical rustfmt, recorded as +23 production/+0 tests/
+23 total. Later C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `2bc90e57019472c7f27d1db7f8d99ed7f95bf2c2` passed
`./tools/validation-v2 final --base 2a5427689 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 20:10:31Z to 20:15:06Z on 2026-11-19
(274.258 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T201031.862813Z-3094061-final.json`.

On `forever`, C4-8j starts at `bc2794971` and extracts map difficulty, LFG,
appearance and creature equipment catalogs. `app.rs` shrinks from 4,727 to
4,656 lines and its physical ceiling tightens without retiring C4-8. The new
private `map_equipment_catalogs.rs` has 122 lines and returns seven stores:
MapDifficulty, MapDifficultyXCondition, LfgDungeons, ItemAppearance,
ItemModifiedAppearance, ItemStats and CreatureEquipment. Five explicit inputs
preserve DataDir, locale, HotfixDatabase, the world object adapter and creature
templates before canonical SpawnStore loading.

The moved body matches the original token stream after adapting references and
canonical rustfmt. The `world-server` library check passes with the same 35
existing warnings. The logical owner gains 51 production lines for the typed
result, wrappers and formatting, recorded as +51 production/+0 tests/+51
total. Later C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `6342e6990fb98be0773ad5c13db090225644da6a` passed
`./tools/validation-v2 final --base bc2794971 --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 20:21:40Z to 20:26:08Z on 2026-11-19
(267.294 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T202140.672230Z-3100928-final.json`.

On `forever`, C4-8k starts at `3a339eb5e` and extracts canonical spawn,
respawn and world-state runtime startup. `app.rs` shrinks from 4,656 to 4,501
lines and its physical ceiling tightens without retiring C4-8. The new private
`spawn_runtime_catalogs.rs` has 213 lines and returns six values: the shared
ScriptNameContainer, game-event persistence, shared canonical spawn metadata,
persisted respawn times, respawn persistence and shared WorldStateMgr. Ten
explicit inputs preserve both databases and every catalog dependency.

The moved body matches the original token stream after adapting references,
canonical rustfmt and moving the required `mut` binding to the remaining app
consumer. The `world-server` library check passes with the same 35 existing
warnings. The logical owner gains 58 production lines for the typed result,
wrappers and formatting, recorded as +58 production/+0 tests/+58 total. Later
C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `a1aab9bad8e53c72a19dd132075aa672c12359b4` passed
`./tools/validation-v2 final --base 3a339eb5e --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 20:36:31Z to 20:40:49Z on 2026-11-19
(257.655 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T203631.705111Z-3109802-final.json`.

On `forever`, C4-8l starts at `9b71b94ab` and extracts mount catalogs.
`app.rs` shrinks from 4,501 to 4,440 lines and its physical ceiling tightens
without retiring C4-8. The new private `mount_catalogs.rs` has 99 lines and
returns Mount, MountDefinition, MountCapability, MountTypeXCapability and
MountXDisplay stores. Four explicit inputs preserve DataDir, locale and both
databases; every hotfix diagnostic remains in order.

The moved body matches the original token stream after adapting references and
canonical rustfmt. The `world-server` library check passes with the same 35
existing warnings. The logical owner gains 38 production lines for the typed
result, wrappers and formatting, recorded as +38 production/+0 tests/+38
total. Later C4-8 stages still owe remaining catalogs, dependencies and
supervision/shutdown.

Committed candidate `4421ef694ad7637e4e4e51f9b1866dbae66299c9` passed
`./tools/validation-v2 final --base 9b71b94ab --architecture --require-changes --timings --logs`
with Rust 1.98.0, Protoc 28.3 and one Cargo job on the warm x86_64 checkout.
All eleven commands passed from 20:46:39Z to 20:50:45Z on 2026-11-19
(246.323 seconds), within the 600-second target. The `world-server` library
passed 645 tests with zero failures and one existing ignored test; all 769
source test identities are preserved. The SQL module compositor also passed
and restored the checkout. R1 is not applicable because no `wow-world/src`
root shrank. Verified manifest:
`target/validation-v2/manifests/20261119T204639.365572Z-3116287-final.json`.

On `forever`, C4-8m starts at `3f67b0b15` and extracts collection, condition
and conversation catalogs. `app.rs` shrinks from 4,440 to 4,354 lines and its
physical ceiling tightens without retiring C4-8. The new private
`collection_condition_catalogs.rs` has 172 lines and returns eighteen results,
including condition presence and the filtered conversation-line template
store. Five explicit inputs preserve DataDir, locale, HotfixDatabase, DB2
removals and the world reference adapter. WorldStateExpression and
ConversationLine remain construction/log-only results with underscore names.

The moved body matches the original token stream after adapting references,
canonical rustfmt and the two unread result names. The `world-server` library
check passes with the same 35 existing warnings. The logical owner gains 86
production lines for the typed result, wrappers and formatting, recorded as
+86 production/+0 tests/+86 total. Later C4-8 stages still owe remaining
catalogs, dependencies and supervision/shutdown.

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
