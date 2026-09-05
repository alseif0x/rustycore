# Session convergence checkpoint — 2026-09-04

Issue #578 remains open. This is an exact inventory reconciliation, not the terminal #153
audit, a full C++ parity approval, or a live-client acceptance report.

Reviewed source: `74daf3f9` plus the active-Player relocation and borrowed grid-capability
slice committed with this checkpoint. The prior runtime family membership was last edited
at `9a29e195`; the prior syntax snapshot was last edited at `26f72455`. Neither described the
current source. The historical persistence snapshot is deliberately unchanged: ordinary
iteration uses `session-ownership-check check --syntax-only`, not an exhaustive persistence scan.

## Exact membership and remaining work

The AST has **720 WorldSession fields: 292 production and 428 test fixtures**. The runtime
ledger previously assigned 726 identifiers and classified only 32 as test fixtures. Every
current `cfg(test)` identifier is now assigned exclusively to `test_only_fixtures`; production
members retain their semantic family. This classification does not prove that callers are thin
or that every fixture exercises production behavior.

The removed identifiers are `battle_pet_purchase_store_like_cpp`,
`gameobject_template_lifecycle_store`, `player_grid_load_resolver_like_cpp`, and the six
`represented_pet_{aura_effects,auras,declined_names,spell_charges,spell_cooldowns,spells}_like_cpp`
members. Three identifiers absent from the old runtime ledger are explicitly classified:

| Identifier | Classification and evidence |
|---|---|
| `gameobject_template_lifecycle_store_like_cpp` | Existing production immutable catalog, still installed by `SessionCoreCapabilitiesLikeCpp`; not a new state owner. |
| `object_mgr_catalogs_like_cpp` | Test-only injected catalog fixture. Production borrows the process catalog through dispatch capabilities. |
| `pet_load_query_holder_rows_like_cpp` | Production deferred Pet load staging, not the live Pet. C++ `Pet.cpp:157-203,386-408` defines six query results and resolves the current Player/Pet before applying them. |

The following are still open #578 work, not stable exceptions or work deferred to #153:

- 137 production catalog/configuration/service fields still reside on Session. Required
  construction is not enough: the owning vertical must consume the narrow capability.
- The map/runtime family still has 20 production fields, including both map-manager handles,
  creature scheduling/delivery state and GameObject state. Keep one clock per responsibility;
  remove Session map selection/gameplay and the remaining legacy/canonical bridges incrementally.
- Inventory/loot/economy has 15 remaining production members, spells/progression 20,
  movement/combat seven, social three, and the unresolved residual 18. The exact field lists
  remain executable ledger data; their inclusion does not endorse their current owner.
- Handler and external Session impl bodies still coordinate gameplay. Moving data to Player
  does not itself complete the decode/adapt/encode boundary.
- Public mutable Map access and final runtime-owned grid materialization remain open.
  The generation-checked lifetime coordinator still uses an outer manager mutex, not an actor
  handoff. Full persistence/bridge inventories and live acceptance remain terminal gates.

`SessionResources` has eight required aggregate fields (`core`, `inventory`, `player`, `spells`,
`world`, `progression`, `runtime`, `realm`), rather than 273 flat fields with 216 optional slots.
Their immediate capability types contain respectively 5, 30, 25, 34, 29, 21, 20 and six members:
**170 first-level members, plus further nested handler/persistence bundles**. The constructor
aggregate stays in world-server, not wow-network. Its `install_into_session_like_cpp` methods
still install many catalogs on Session, so eight fields are not evidence of final convergence.

## C++ contrast for this slice

Active position changes must use the owning Map: `Unit::UpdatePosition`
(`src/server/game/Entities/Unit/Unit.cpp:12257-12284`) calls `Map::PlayerRelocation`
(`src/server/game/Maps/Map.cpp:1015-1040`), which updates cell/grid membership as well as position.
Both the movement setter and the same-map residence path now call the generation-checked
`MapManager::relocate_player_like_cpp`, then private `MapRuntime` and the existing map relocation
operation. Detached preparation still edits the same detached Player value; stale generations
cannot relocate a replacement. This corrects a stale cell index, not merely a method location.
It does not claim new coverage of vehicle passenger relocation or all C++ visibility effects.

The grid callback is a separate capability extraction: one required callback is built in `app.rs`
and borrowed by movement, embedded spell movement and login. The captured stores/managers and
call boundaries are unchanged; no new timer, queue, SQL statement or opcode registration is added.
C++ `Map::EnsureGridLoadedForActiveObject` / `AddPlayerToMap` (`Map.cpp:348-363,427-445`) anchors
the grid responsibility and the login grid gate remains before success publication.

The preceding syntax changes retain one Player value across residence (`MapManager::CreateMap`,
`MapManager.cpp:139-232`), replace external borrowed Creature access with closure-scoped queries,
and route represented creature combat through MapRuntime commands. C++ `Unit::Attack`
(`Unit.cpp:5645-5745`), `Unit::CombatStop` (`5802-5821`) and `CombatManager::SetInCombatWith`
(`CombatManager.cpp:187-228`) anchor reciprocal combat ownership before publication. These
anchors explain ownership and phase constraints; this inventory refresh does not certify all
earlier gameplay changes or complete AI/script callback parity.

## Reviewed syntax delta

Relative to the checked-in policy at `26f72455`:

- Remove the grid-resolver field, its setter and `ensure_player_grid_loaded_like_cpp`.
- Add `ensure_canonical_player_owner_exists_like_cpp`: adopt the current incarnation or create
  the one detached value, with revalidation under the manager lock, before map selection effects.
- Replace `is_represented_seer_kind_like_cpp` with `represented_seer_kinds_like_cpp`, retaining
  the same Player/Creature/Pet/DynamicObject kind set for the narrowed lookup API.
- Borrow the required grid capability through the six changed login/movement/spell/connection
  signatures. No registration metadata or dispatch admission rule changes.
- Retain all 65 discovered bridge rows. Eighteen fingerprints change for the reviewed
  Creature lookup/combat/residence paths, factory callback relocation, WorldSession declaration
  and corresponding world-server fixtures. No bridge is accepted as retired by renaming it.
- Registry accesses, SessionCommand vocabulary and generated-surface inputs are unchanged.

## Logical size reconciliation

These are exact logical-owner counts on the aarch64 development worktree, including reviewed
private descendants, not physical-file size or performance measurements. Both increases and
reductions are recorded. The ceilings continue to reject further unreviewed growth; this
checkpoint does not waive #578's semantic acceptance criteria.

| Logical owner | Production old → current | Tests old → current |
|---|---:|---:|
| Session | 73,339 → 81,427 | 97,325 → 102,131 |
| Map | 15,396 → 16,167 | 18,273 → 18,728 |
| Character handlers | 19,786 → 20,587 | 12,899 → 12,803 |
| Loot handlers | 13,415 → 13,939 | 16,383 → 16,478 |
| World-server crate | 29,273 → 28,896 | 26,605 → 27,008 |
| Player | 9,536 → 10,370 | 8,891 → 9,273 |
| Quest handlers | 8,325 → 8,857 | 10,591 → 10,620 |

Session/handler growth includes explicit unavailable-owner handling, scoped canonical queries,
capability parameters and fixture migration; it remains a large application monolith. Player
growth is private canonical substates and their tests. Map growth is private EntityWorld/runtime
boundaries, residence/command/relocation operations and tests, without an additional mutable
representation. World-server production shrinks while canonical runtime fixtures grow. The
unchanged Group hotspot ceiling is left untouched.

## Validation boundaries

### Login-stream follow-up — 2026-09-05

Bot/guard commit `10684ccb` closes the premature smoke-test disconnect described below.
Ordinary login-only QA now retains both sockets until an instance `SMSG_UPDATE_OBJECT` has
arrived and the streams have been quiet for one second (30-second absolute budget). It uses
cancellation-safe peeks before reading complete encrypted frames, rejects connection closure,
and responds to time sync. Anchors: C++ `Map.cpp:427-446,1826` (`AddPlayerToMap` / `SendInitSelf`)
and `MiscPackets.cpp:156-167` (time-sync request/response). It does not decode the self CREATE
or prove full world visibility/gameplay; `login_stream_drained` names this bounded evidence.
The runtime guard now requires that field, not just `player_login_verified`.

All 142 bot tests pass, including successful drain, realm closure, and missing object
publication; the 69 runtime-guard checks pass. Bot build and formatting/diff checks pass.
The installed optimized server remains code `d568f3aa`, SHA-256
`91663b7c21888f4de5e280ddd1a22c5f811e7ecca844eeed154ab65deee191ca`.
Guarded report: `/tmp/rustycore-578-drained-login-runtime.json`; private bot evidence:
`/tmp/rustycore-login-qa.crzSbP/bot.json`. All four auth/enumeration/login/drain flags are true.
Candidate PID 45080 logged `Login sequence complete` at **00:31:48.563330 UTC**; the following
`World::KickAll` at 00:31:49 is the guard's shutdown for restoration, not the previous
`login packet sequence failed` error. This validates one automated login on the local fixture,
not manual-client readiness, sustained gameplay, LFG, or fresh C++ capture parity.
The guard reports `passed-restored`; the deployed binary's original SHA-256 was independently
verified and both services are active. No push or merge was performed.
Final validation on code HEAD `10684ccb` also passed:
`target/validation-v2/manifests/20260905T003127.014359Z-45213-final.json`
(6,745 library tests, 315 contract-checker tests, and the isolated bot check). This evidence
predates only the documentation closeout commit; the 142 bot tests were run separately.

### User-prioritized next front: automatic Dungeon Finder

The user approved this order: repair and verify current login first, then audit and scope LFG
as its own issue/branch. This does not close or silently defer #578's remaining ownership work.
No LFG gameplay implementation or database repair is included in this checkpoint.

Preliminary evidence (not a completed subsystem audit):

- Rust registers information/status/blacklist handlers in `handlers/misc/lfg.rs`, but the
  production handler search does not find DFJoin/DFLeave/DFProposalResponse/DFSetRoles/DFTeleport.
  Its LFG-list status explicitly represents removed-from-queue while the manager is unported.
- C++ does contain automatic matching: `LFGHandler.cpp:31-104`, registrations at
  `Opcodes.cpp:425-430`, `LFGMgr.cpp:286,397,945,1052,1357,1472`, and
  `LFGQueue.cpp:288,358`. Existing code is not proof of complete client-3.4.3 behavior.
  Manual listings are a different surface: `LFGHandler.cpp:584-632` returns zero search results
  and an explicitly unimplemented application response, despite a partial `LFGListManager`.
- Local Hotfix rows 256/258 are named Random Lich King Heroic/Normal with build 12340; the
  other 97 rows have build 52237. Provenance remains unknown. The downloaded
  [Wago LFGDungeons export for 3.4.3.54261](https://wago.tools/db2/LFGDungeons/csv?build=3.4.3.54261)
  (SHA-256 `fe615884df9b32a1a281d94499509dd4f80da61160156e31d8f39523815e1d47`)
  has empty descriptions for Random Lich King Dungeon/Heroic at IDs 261/262. IDs 256/258 instead
  mean Halls of Reflection/Random Classic Dungeon. Do not infer missing descriptions or rewrite
  these local IDs without auditing references and the effective local DB2/Hotfix overlay.
- Issues #550/#552 closed loader/capability extraction only, not the gameplay system.

The next scope should cover roles, join/leave, matching, proposals, group creation, teleport,
completion/reward and cancellation/disconnect/retry cases. First audit C++ gaps, data integrity,
current Player/Group/Map dependencies and packet-capture availability. Missing C++ behavior
requires exact-build client/capture evidence. Manual LFG List remains a separate scope, and no
queue owner or new runtime clock is chosen here.

### Runtime follow-up and production-only construction regression

On `d9f1e5ee`, final validation passed:
`target/validation-v2/manifests/20260904T234919.430148Z-4193605-final.json` (6,745 library
tests and 315 checker tests). The two production-linked login tests and 157 capture-diff
regressions passed separately. The release build completed on aarch64 in 8m13s.

Installed candidate `8281cd5aebdedd7ae792493d8da356937fff0791b3ed416855025a7993a9c1fc`
passed initial mail hydration but stopped at `canonical Player currency owner unavailable during
login`, after map selection. Guarded QA reported `failed-restored` in
`/tmp/rustycore-578-login-owner-runtime.json`; private evidence is
`/tmp/rustycore-login-qa.8sJ1lB`. The original executable was restored and serving.

The next regression reproduces this difference without a live DB: initial map selection followed
by collection reads and interleaved map ticks passes in dev but fails in release. Root cause:
`Map::insert_map_object_record` put its actual insertion inside `debug_assert!`, so optimized
builds erased the mutation. Moving the insertion into an unconditional statement adds exactly
one production Map line (16,167 -> 16,168; tests stay 18,728; total 34,895 -> 34,896). This is the
only hotspot-ceiling adjustment; field/bridge/syntax policy is not refreshed. It is a behavior
repair of the staged storage change, not completion of #578.

Post-fix evidence on `d568f3aa` (2026-09-05, aarch64):

- Final validation passed: `target/validation-v2/manifests/20260905T001226.991927Z-24382-final.json`
  (6,745 library tests and 315 checker tests). All three production-linked login regressions
  pass in both dev and release. The optimized world-server build completed in 12m54s.
- Installed candidate SHA-256
  `91663b7c21888f4de5e280ddd1a22c5f811e7ecca844eeed154ab65deee191ca`
  returned bot status zero; `/tmp/rustycore-578-map-insertion-runtime.json` records the guarded
  result. Private bot evidence is `/tmp/rustycore-login-qa.TFANN8/bot.json`: authentication,
  character enumeration and `player_login_verified` are true. Candidate PID 38971 reached
  aura hydration and the later "continuing login" phase at 00:24:44 UTC, beyond both repaired
  mail/currency-owner failures.
- This is **bounded login verification, not full world-entry acceptance**. The bot's ordinary
  login loop exits on `SMSG_LOGIN_VERIFY_WORLD` (`main.rs:5704-5733`) and closes the sockets;
  the candidate subsequently reports connection reset/broken pipe and "login packet sequence
  failed". Extend the maintained bot's completion criterion before claiming stable world entry
  or starting LFG runtime acceptance. No new client packet layout is inferred from this run.
- The guard restored the original executable, SHA-256
  `c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`;
  world-server and bnet-server are active. No manual-client readiness or fresh C++ capture-diff
  is claimed, and no LFG gameplay or local LFG row changes were made.

The local final gate passed on `e1daed4c` and again on `fbd762c6`; the latter manifest is
`target/validation-v2/manifests/20260904T230645.707038Z-3-final.json` (6,745 library tests
passed). These are historical evidence, not validation of subsequent changes.

Guarded login QA exposed three independent boundaries:

- Local DB schemas were already materialized, but the official migration history was absent.
  The official `rustycore-db` transition-import path adopted the four existing auth/characters
  migrations without replaying their DDL. All four databases then validated compatible.
  Before adoption, full auth/characters and schema-only world/hotfixes backups were saved under
  private `/tmp/rustycore-578-db-backup.Ay81P8`. These contain sensitive runtime data and must
  never be committed. No LFG rows were edited.
- SQL NULL LFG descriptions exposed the separate loader repair in `fbd762c6`.
- Candidate `64a95e7eb6572577498776d09bd39b692a695c9ef93d6716e14dba68265ad028`
  authenticated, enumerated characters and linked the instance socket, then kicked during
  mail hydration because initial Player construction required its own canonical inventory.
  The old build was restored; this run did **not** pass login QA.

The construction fix limits Session-to-Player equipment hydration to old unit fixtures.
Production starts with the new Player's empty equipment and uses the existing later inventory
load; it adds no fallback for unresolved active/stale owners. C++ anchors and the failure are
recorded in `EXISTING-CODE-DEFECTS.md`. The new integration test compiles wow-world without
`cfg(test)`, reaches PetStable only after successful mail/scalar hydration, and rejects a missing
manager before that point. The positive case fails on the original code and passes with the fix;
the negative case passes in both. Architecture check/self-test and the syntax-only ratchet pass
without baseline changes. Subsequent final and bounded installed-login evidence is recorded
above; complete world-entry acceptance remains pending.

Focused current/stale/detached Player tests and movement/login/spell checks are recorded in
`docs/migration/adr-map-runtime-entity-world.md`. The added same-map residence regression also
passes after changing the destination across a cell boundary. The full world suite was repeated
after that correction: 3,671 passed, zero failed, one ignored. The map suite (703 passed, zero
failed, one ignored) and quick validation had already passed. The release world-server build
also passes. Architecture check/self-test and the syntax-only Session ratchet pass with the
reviewed ledger. Final validation and live QA must be reported separately, not inferred from
a refreshed baseline.
