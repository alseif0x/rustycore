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

Focused current/stale/detached Player tests and movement/login/spell checks are recorded in
`docs/migration/adr-map-runtime-entity-world.md`. The added same-map residence regression also
passes after changing the destination across a cell boundary. The full world suite was repeated
after that correction: 3,671 passed, zero failed, one ignored. The map suite (703 passed, zero
failed, one ignored) and quick validation had already passed. The release world-server build
also passes. Architecture check/self-test and the syntax-only Session ratchet pass with the
reviewed ledger. Final validation and live QA must be reported separately, not inferred from
a refreshed baseline.
