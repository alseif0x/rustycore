# F6 duality audit — `represented_*` vs canonical, legacy runtime and legacy map manager

Read-only audit produced for issue #1263 criterion **F6**. It records what exists today with
`path:line` anchors in both trees and marks behaviour differences separately from structural
ones. **No Rust code was changed, and no repair is proposed inside a structural slice.**

- Rust tree: `/home/server/rustycore-1241`, branch `1263-f5-r33` at `bd7c106fd`.
- C++ reference: `/home/server/woltk-trinity-legacy` at `a5f8da2e` (3.4.3).
- Method: source inspection only — `grep`/`sed`/`git log`/file reads. No `cargo`, no tool suite,
  no `validation-v2`, no capture, no runtime. Every quantitative figure is labelled by how it
  was obtained.
- Anything not proven from source is written `unverified`; it is never inferred.

## 1. What "represented" and "canonical" mean in this tree

| Term | Concrete meaning | Anchor |
|---|---|---|
| **canonical** | The live `wow-map` map/entity world (`canonical_map_manager`) plus the one generation-checked `wow_entities::Player` behind `player_handle_like_cpp` | `crates/wow-world-core/src/session/state/session_core.rs:127-131` |
| **represented** | A Rust mirror of C++ state that lives on the `Session`/hub instead of on the C++ owner — sometimes the same owner as C++, sometimes a different (per-session) owner | `crates/wow-world-core/src/session/state/hub.rs:10-27`; `crates/wow-world-entities/src/state.rs:11-15` |
| **`resolved_*`** | The read seam that prefers canonical and returns `None` in production when no canonical owner exists | `crates/wow-world-core/src/session/player_vitals_adapter.rs:89-113` |

Three structural facts drive every row of the divergence table:

1. **`represented_*` is a naming convention, not a data-source guarantee.** 765 `fn represented_*`
   definitions exist under `crates/wow-world*/src` (`grep -rn 'fn represented_'`, counted
   2026-10-03); 89 of them read canonical APIs inside their own body (20-line window heuristic).
   Example where the name is a vestige and the source is canonical:
   `crates/wow-world-core/src/session/canonical_access/operations.rs:182` reads
   `canonical_player_snapshot_like_cpp`. **The name is not evidence of the owner.**
2. **A large `represented_*` subset is test-only and absent from production.** 222 of the 765
   definitions sit behind `cfg(any(test, feature = "test-fixtures"))` immediately above them
   (same heuristic, ±8-line window; exact split `unverified`). The `test-fixtures` feature is
   declared "Enabled only through dev-dependencies; **never in a production build**"
   (`crates/wow-world/Cargo.toml:9-11`), and `world-server` pulls it only under `[dev-dependencies]`
   (`crates/world-server/Cargo.toml:43`). The world-server `[dependencies]` block at
   `crates/world-server/Cargo.toml:10-36` does not enable it.
3. **There are two live map/entity stores, both injected in production.**
   `SessionCore::map_manager` (legacy, `wow-world-core::map_manager`) and
   `SessionCore::canonical_map_manager` (canonical `wow_map::MapManager`) are set side by side at
   `crates/world-server/src/session_factory.rs:414-415`. Both are reachable from the same session.

## 2. `represented_*` accessors, grouped by domain

Counts are ungated `fn represented_*` definition sites per path bucket (`grep` + path grouping,
2026-10-03). "Represented owner" and "canonical owner" are the actual storage owners read,
not the names.

| Domain | Ungated `represented_*` fns (leading buckets) | Represented form reads | Canonical form reads | Can they disagree? | What C++ reads |
|---|---|---|---:|---|---|
| **Loot** | 42 `crates/wow-world/src/handlers/loot/*` (`rolls.rs:30`); 21 `crates/wow-world-loot` (`state/roll_access.rs:7`, `src/state.rs:82-84`); 4 `crates/wow-world-core/src/session/loot/requests.rs:5` | Session-local `LootState` tables: `loot_table`, `represented_loot_cache_generations_like_cpp`, `represented_unique_gameobject_uses`, `represented_gameobject_tap_lists`, `represented_loot_rolls` — `crates/wow-world-loot/src/state.rs:44-84`; held per session at `crates/wow-world/src/session/state.rs:134` | `OwnedLootAuthority` on the canonical map creature/gameobject; `LootReleaseAccessLikeCpp` — `crates/wow-world-core/src/session/canonical_access/loot_release.rs:203-282` | **Yes** — three copies of one C++ `Loot` (`Creature::m_loot`, `Creature.h:236`; `GameObject::m_loot`, `GameObject.h:322`) | The owning object's single `Loot`; `WorldSession::DoLootRelease` `Handlers/LootHandler.cpp:270`, `Loot::NotifyLootList` `Loot/Loot.cpp:637` |
| **Quest** | 13 `crates/wow-world/src/handlers/quest/*` (`rewards/validation.rs:11`); 10 `crates/wow-world/src/session/quest/*` (`rewards.rs:87`); 9 `crates/wow-world-application/src/quest/visibility/*` (`gameobject_flags.rs:28`); 6 `crates/wow-world/src/session/quest_dialog.rs:35` | `SessionQuestState`: hide-distance config, pending status updates, objective-progress event queue — `crates/wow-world-application/src/quest/session_state.rs:17-29` | `player.gameplay_state().quests` via `player_quest_gameplay_snapshot_like_cpp` — `crates/wow-world-core/src/session/canonical_access/quest_objectives.rs:102-107` | **No for the quest log** (production resolves canonical only, `crates/wow-world-application/src/quest/objective_progress.rs:291-306`); the represented queue is durable-across-await plumbing | `Player::CanCompleteQuest` `Entities/Player/Player.cpp:14123`, `Player::m_quests` |
| **Movement** | 5 `wow-world-core/src/session/movement/*` (`transfer.rs:169`) | Production movement reads go through `resolved_*`; the `SessionFixtures::movement` mirror (position, flags, fall time, forced speed, counter) is entirely `#[cfg(any(test, feature = "test-fixtures"))]` — `crates/wow-world-core/src/session/state/movement.rs:28-110` | Canonical `Unit::m_movementInfo` / `Player::m_lastFallTime` equivalents via `resolved_fall_information_like_cpp` (`crates/wow-world-core/src/session/movement/fall.rs:19`), `resolved_player_movement_flags_like_cpp` (`crates/wow-world-core/src/session/movement/state.rs:641`) | **No in production** (mirror not compiled); **yes in tests** | `Unit::m_movementInfo`, `Player::m_lastFallTime` on the single `Player`; `Player::Update` swing path `Entities/Player/Player.cpp` (`DoMeleeAttackIfReady`) |
| **Visibility** | 1 `wow-world-visibility`; 9 gated there | `client_visible_guids_like_cpp` — per-session, `Arc`-shared — `crates/wow-world-core/src/session/state/session_core.rs:147`; producers `crates/wow-world/src/session/spell_effects/ticks.rs:78,157`, `crates/wow-world/src/session/spell_effects/effects.rs:446,489` | Same set is the visibility authority; no second copy | **No** — the owner matches C++ (per-player set) | C++ `Player::m_clientGUIDs` (per-player), `Entities/Player/Player.h:2450`; mutated at `Player.cpp:23201/23213`; `HaveAtClient` at `Player.cpp:23031` |
| **Entities (creature/GO)** | 13 `crates/wow-world/src/session/world_entities/*` (`gameobject.rs:17`); 22 `crates/wow-world-entities` ungated | Legacy `map_manager::WorldCreature` (global store) **and** session-local `WorldEntitiesState`: `represented_creature_auras_like_cpp`, `represented_gameobject_use_states`, `represented_gameobject_phase_shifts` — `crates/wow-world-entities/src/state.rs:19-54` | Canonical `wow_map` creature/gameobject via `canonical_map_manager`; `mutate_world_creature` then `sync_canonical_creature_entity_like_cpp` — `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` | **Yes** — dual entity plus revision-gated whole-entity replacement, `crates/wow-world-core/src/session/creature_canonical_adapter.rs:29-70` | One `Creature` per spawn on the map: `Creature::Update` `Entities/Creature/Creature.cpp:696`; `GameObject::GetGoStateFor` `Entities/GameObject/GameObject.cpp:3795` |
| **Spell** | 56 `wow-world-spell/src/session/spell_state/*` (`spellbook.rs:39`); 30 `wow-world/src/session/spell_effects/*` (`effect_combat.rs:28`); 8 `player_cast/wire.rs:85`; 6 `melee_rules.rs:216` | `PlayerSpellRuntimeState` snapshots taken into `RepresentedPlayerSpellRuntimeLikeCpp` — `crates/wow-world-spell/src/records.rs:155-163`; aura/shapeshift mirrors in `wow-world-spell/src/session/spell_state/*` | The same `PlayerSpellRuntimeState` on the canonical Player, read through `spell_state`/`canonical_access/spell_acquisition.rs` | **Partly** — `records.rs:155` snapshots canonical state, so the mirror is derived; disagreement requires a snapshot held across a mutation | `Player::m_spells`, `Player::m_auraMap`, `Unit::m_shapeshiftForm` on the single Player |
| **Inventory / items** | 54 `wow-world-inventory` ungated (`equipment_slots.rs:25`, `modifiers.rs:21`, `bank.rs:20`, `void_storage.rs:54`, `items.rs:29`); 28 `wow-world-core/src/session/player_items/*` (`valuation.rs:191`) | Inventory/runtime mirrors on the session plus `wow_entities::Player` owned families | `OwnedInventoryAccessLikeCpp` / `player_inventory_runtime_like_cpp` — `crates/wow-world-inventory/src/storage.rs:245-250,510` | **Not established** (`unverified`) — the family migration is per-family and was not traced here | `Player::GetItemByPos`, `Player::m_items` |
| **Progression / stats / talents** | 8 `crates/wow-world-core/src/session/progression/*` (`talents.rs:125`); 10 `crates/wow-world-core/src/session/player_stat_queries.rs:78`; 5 `crates/wow-world-application/src/stats.rs:45` | Session progression mirrors (honor/XP/talent pending requests) | Canonical Player via `resolved_player_*` (`crates/wow-world-core/src/session/progression_adapters.rs:100-160`) | **Not established** (`unverified`) | `Player::m_xp`, `Player::m_talents` |
| **Instances / difficulty** | 11 `wow-world-instances` ungated (`difficulty.rs:79`, `fixture_access.rs:21`), 5 gated | Session instance-reset and difficulty mirrors | `InstanceLockMgr` (`wow_instances`) and the canonical map's difficulty | **Not established** (`unverified`) | `sObjectMgr->GetInstanceTemplate`, `Map::GetDifficulty` |
| **Battleground / collections / lifecycle** | 5 `wow-world-social`, 10 `wow-world-lifecycle` ungated plus large gated sets | `SessionFixtures`/state mirrors in `crates/wow-world-core/src/session/state/battleground.rs:15-52`, `crates/wow-world-core/src/session/state/collections.rs:25-50`, `crates/wow-world-lifecycle/src/state.rs:112-115` | Not traced | **Not established** (`unverified`) | `Battleground`/`CollectionMgr` |

**Reading rule for the table.** Where the represented owner equals the C++ owner (visibility), the
duality is structural. Where the represented owner is the *session* and the C++ owner is the
*object* or the *map* (loot tables, GO use state, tap lists, unique users), it is a behaviour
divergence and is listed in §5.

## 3. `legacy_runtime`

**What it is.** A private module of `wow-world::session` holding the creature/player runtime tick
bodies: 18 files, ~7,070 lines, listed in `crates/wow-world/src/session/legacy_runtime/mod.rs:6-19`.
Module header: *"Legacy creature and player runtime tick, separated from the Session root. Each
submodule owns one complete tick responsibility"* (`legacy_runtime/mod.rs:1-4`). Every submodule
header repeats *"Behaviour is preserved; the canonical owner of this state is unchanged"*
(`creature_tick.rs:1-4`, `creature_aggro_tick.rs:1-4`, `creature_lifecycle_tick.rs:1-4`,
`creature_melee_tick.rs:1-4`, `creature_movement_tick.rs:1-4`, `creature_spell_tick.rs:1-4`,
`player_tick.rs:1-4`).

**Sibling legacy-runtime shims in the same family:**

| Surface | Anchor | Note |
|---|---|---|
| Private module + glob re-export | `crates/wow-world/src/session/mod.rs:42,47-49` | comment explains the external path `wow_world::session::run_legacy_*` is preserved |
| Seven public tick entry points | `legacy_runtime/mod.rs:50-56` | "only these seven are public" |
| `current_legacy_runtime_map_key_like_cpp` | `crates/wow-world-core/src/session/instances/map_key.rs:53-62` | represented map key resolution, see D-03 |
| `legacy_creature_*` config/aggro contracts | `crates/wow-world-core/src/session/creature_aggro_contracts.rs` (referenced from `legacy_runtime/*`) | session config, not runtime state |
| `rebind_legacy_creature_loot_authority_like_cpp` | `crates/wow-world-core/src/session/loot/operations.rs:44-78` | writes the legacy store, see D-01 |

**Who calls it.** Only `world-server`, through the `wow_world::session::run_legacy_*` re-exports:

| Caller | Anchor |
|---|---|
| Tick fanout | `crates/world-server/src/runtime/delivery.rs:1575-1668` (`run_legacy_creature_runtime_tick_with_input_and_deliver_once_like_cpp`) |
| Loop task | `crates/world-server/src/runtime/delivery.rs:1777-1795` (`spawn_legacy_creature_runtime_update_loop_like_cpp`) |
| Spawn site | `crates/world-server/src/app.rs:5365-5378` |
| Shutdown join | `crates/world-server/src/app.rs:5611-5623` (`"legacy-creature-runtime"` producer) |

**Is it dead, transitional or load-bearing?** **Load-bearing, and enabled by default.**

- `legacy_creature_global_runtime_enabled_from_config_like_cpp()` returns
  `.unwrap_or(true)` — default **on** — `crates/world-server/src/bootstrap/config.rs:152-156`.
- When on, startup flips the legacy map manager to `RuntimeTickOwner::GlobalLegacy` and logs
  *"legacy creature tick owner set to GlobalLegacy"* — `crates/world-server/src/app.rs:5319-5332`.
- The enum documents this as the production default: *"Production startup flips this to
  `GlobalLegacy` by default so a global map clock drives creature runtime like C++ and
  session-level creature ticks are skipped to avoid double resolution"* —
  `crates/wow-world-core/src/map_manager/runtime_state.rs:196-210`.
- `player_tick.rs:8-20` states the reason it is load-bearing: *"Every logged-in session used to
  run it on its own clock and write shared legacy creature state ungated, so with N players there
  were N+1 concurrent writers of that state."*

**C++ equivalent.** There is no separate C++ "legacy runtime". C++ runs one path:
`World::Update` → `sMapMgr->Update(diff)` (`src/server/game/World/World.cpp:2748`) →
`MapManager::Update` (`src/server/game/Maps/MapManager.cpp:287`) → per-map `Map::Update` →
`Creature::Update` (`src/server/game/Entities/Creature/Creature.cpp:696`), plus
`Player::Update`/`DoMeleeAttackIfReady` on the same clock. Rust instead runs **two production
producer tasks**: `spawn_canonical_map_update_loop` (`crates/world-server/src/app.rs:5345`) and
`spawn_legacy_creature_runtime_update_loop_like_cpp` (`crates/world-server/src/app.rs:5365`).

**Is any of it a behaviour difference rather than a structural artifact?** **Yes — see D-08.**
The tick-owner flag itself (`RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME`) has no C++ counterpart
(`grep -rn 'LegacyCreatureGlobalRuntime'` over `src/` and `sql/` in the C++ checkout: no match);
the Rust-only key is declared at `crates/world-server/src/lib.rs:182` and read at
`crates/world-server/src/bootstrap/config.rs:153`. On top of that, the tick bodies hold canonical
and legacy guards in an order that only exists because two managers exist
(`player_tick.rs:14-20`: "the execute phase takes canonical then legacy").

## 4. Legacy `map_manager`

**What it is.** `wow-world-core::map_manager`, a global store of `WorldCreature` (canonical
`wow_entities::Creature` **plus** a `CreatureCreateData` packet bridge and a legacy move spline):
module doc *"Legacy map manager and its creature runtime"* — `crates/wow-world-core/src/map_manager/mod.rs:7-11`;
`WorldCreature` at `crates/wow-world-core/src/map_manager/mod.rs:71-90`; 22 files.

**Callers.** It is re-exported from `wow-world` (`crates/wow-world/src/lib.rs:23,50`) and injected
into every session (`crates/world-server/src/session_factory.rs:414`). Within `wow-world*` there
are 128 non-test references to `map_manager` outside the canonical one
(`grep -rn '\.map_manager|map_manager:' crates/wow-world/src crates/wow-world-core/src`, excluding
`unit_tests` and `canonical_map_manager`; the count covers only those two crates); representative
anchors:

| Caller | Anchor |
|---|---|
| Session field | `crates/wow-world-core/src/session/state/session_core.rs:123-125` |
| Creature read/mutate | `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` |
| Legacy loot authority read/rebind | `crates/wow-world-core/src/session/canonical_access/loot_release/authority.rs:228-240`; `crates/wow-world-core/src/session/loot/operations.rs:44-78` |
| Legacy runtime ticks | `crates/wow-world/src/session/legacy_runtime/creature_movement_tick.rs:363-386`, `creature_spell_tick.rs:15-59`, `creature_lifecycle_tick.rs:21-61`, `creature_aggro_tick.rs:258-275`, `creature_melee_tick.rs:179-190`, `player_tick.rs:25-37` |
| Tick-owner decision | `crates/wow-world-core/src/map_manager/runtime_state.rs:360` (`shared_runtime_tick_owner_like_cpp`) |

**C++ counterpart — verified, not assumed.** The reference uses **`MapManager` (map ownership)
plus `sObjectMgr` (static data)**; it has no second mutable creature store. Verified anchors:
`MapManager::Update` `src/server/game/Maps/MapManager.cpp:287`; `sMapMgr->Update` call site
`src/server/game/World/World.cpp:2748`. There is no C++ "legacy map manager" to name: the Rust
`map_manager` is a Rust-side transitional store, and the legacy runtime is its only production
clock.

**Dead, transitional or load-bearing?** **Load-bearing while
`RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME` is non-zero (default: on)** — §3. It is transitional in
intent (`runtime_state.rs:196-210` calls the `Session` owner the "test/local default") but
reachable and enabled in production today.

## 5. Recorded divergences

Disposition values: `unknown` (not enough evidence to classify), `intentional-departure contract
needed`, `repair needed outside the refactor`. **No row is authorised for repair by this audit.**

| id | Area | Rust anchor | C++ anchor | What differs | Behaviour or structural? | Disposition |
|---|---|---|---|---|---|---|
| D-01 | Loot authority duplication | `crates/wow-world-core/src/session/canonical_access/loot_release/authority.rs:41-176`; `crates/wow-world-core/src/session/creature_canonical_adapter.rs:178-210` | `Creature.h:236` (`std::unique_ptr<Loot> m_loot`), `GameObject.h:322`, `Item.h:249` — one `Loot` per object | Two independently mutable `OwnedLootAuthority` mirrors (legacy `WorldCreature` + canonical map creature) are reconciled by an 8-iteration compare/exchange; `(Active, Active)` collapses to a retired tombstone; sustained contention returns `None` (the request fails) — `authority.rs:150-152` | **Behaviour** — C++ cannot fail or fabricate a tombstone; it mutates the one object | `repair needed outside the refactor` (needs an explicit contract first) |
| D-02 | Creature entity dual authority | `crates/wow-world-core/src/session/creature_canonical_adapter.rs:29-70`; `crates/wow-world-core/src/session/world_entities/creature_registry.rs:30-59` | `Creature::Update` `Entities/Creature/Creature.cpp:696`; `Object.h:604` (`GetMap`) | A legacy mutation is applied to `WorldCreature`, cloned, then pushed to the canonical map; the push can be **rejected** by a health-state-revision/ABA guard, so the mutation stays visible only in the legacy store | **Behaviour** — C++ has one object and no rejection path | `repair needed outside the refactor` |
| D-03 | Map identity fallback | `crates/wow-world-core/src/session/instances/map_key.rs:53-62` | `Object.h:604` `Map* GetMap() const { ASSERT(m_currMap); return m_currMap; }` | When no canonical player map key resolves, the "legacy runtime map key" silently becomes the represented `current_map_id` with `instance_id = 0`; canonical lookups then use that key (`authority.rs:60-64`, `loot/operations.rs:50`, `world_entities/creature_registry.rs:8`, `creature_query.rs:38`) | **Behaviour** — C++ `GetMap()` is authoritative and asserts instead of falling back to `instance 0` | `repair needed outside the refactor` |
| D-04 | Loot-list delivery identity and fanout | `crates/wow-world-core/src/session/canonical_access/loot_release.rs:109-142` (map_id represented, instance_id canonical, owner skipped); caller split `crates/wow-world-application/src/loot_release/mod.rs:318-321` | `Loot::NotifyLootList` `src/server/game/Loot/Loot.cpp:637-655` — one loop over all `_allowedLooters` via `ObjectAccessor::GetPlayer(map, guid)` | The Rust delivery uses `player_map_id_like_cpp()` for the map and the canonical key only for the instance, and the recipient filter is `registry.loot_delivery_recipient(...)`; the owner's copy is sent on a separate path before the others | **Behaviour** (mixed map identity can select the wrong instance's recipient). The send *order* claim is `unverified`: C++ `_allowedLooters` is `GuidUnorderedSet` (`Loot.h:350`), so no C++ order is defined | `intentional-departure contract needed` |
| D-05 | GameObject per-viewer state owner | `crates/wow-world-entities/src/state.rs:40-42` (session-local `BTreeMap`); reader `crates/wow-world-application/src/quest/visibility/gameobject_flags.rs:28-40` | `GameObject.h:512` (`m_perPlayerState`), `GameObject.cpp:3795-3803` (`GetGoStateFor` falls back to the object's `GetGoState()`), created at `GameObject.cpp:1734` | C++ keeps per-viewer GO state **on the object**; Rust keeps it per **session**. The Rust fallback is `state.go_state.unwrap_or(GoState::Ready)` — a session-local mirror — where C++ falls back to the object's current `m_goState` | **Behaviour** (cross-session visibility of a chest already opened by another player; fallback value) | `repair needed outside the refactor` |
| D-06 | GameObject tap list / unique users owner | `crates/wow-world-loot/src/state.rs:77-84` (`represented_unique_gameobject_uses`, `represented_gameobject_tap_lists`); readers `state/cache_access.rs:119-123`, `state/gameobject.rs:12` | `GameObject.h:464` (`m_unique_users`), `GameObject.h:483` (`m_tapList`); populated `GameObject.cpp:1708`, read `GameObject.cpp:2617,3005-3006,3893` | C++ `m_unique_users`/`m_tapList` are object-owned and read cross-session (e.g. `GameObject.cpp:2617` tests whether *this* player has used it; `3005-3006` picks a random prior user) | **Behaviour** | `repair needed outside the refactor` |
| D-07 | GameObject phase shift owner | `crates/wow-world-entities/src/state.rs:54` (`represented_gameobject_phase_shifts`) | `GameObject::GetPhaseShift` (object-owned phase shift; see `GameObject.cpp:2166-2167` per-player state precedent) | Phase shift for a GO is mirrored per session rather than read from the object | **Behaviour** (the comment itself says "until canonical gameobject map ownership lands") | `intentional-departure contract needed` |
| D-08 | Two production tick owners | `crates/world-server/src/app.rs:5345` (`spawn_canonical_map_update_loop`) + `crates/world-server/src/app.rs:5365` (`spawn_legacy_creature_runtime_update_loop_like_cpp`), gated by `bootstrap/config.rs:152-156` (default `true`) and `app.rs:5319-5332` | `World.cpp:2748` → `MapManager.cpp:287` → `Creature.cpp:696` (one clock, one map, one entity) | Rust runs the creature/melee/aggro/lifecycle/spell runtime on a second producer over a second entity store; C++ has exactly one update path and no such config key | **Behaviour** (timing, phase coupling and double-resolution surface) | `repair needed outside the refactor` |
| D-09 | Two map/entity stores | `crates/wow-world-core/src/session/state/session_core.rs:123-128`; injected `crates/world-server/src/session_factory.rs:414-415` | `MapManager.cpp:287`; single map set under `sMapMgr` | Every session holds both a legacy and a canonical manager; which one a reader observes depends on the path taken | **Behaviour** — the same operation can read different creature state | `repair needed outside the refactor` |
| D-10 | Canonical-object lookup order differs between two loot paths | Prefer-canonical: `canonical_access/loot_release/authority.rs:156-162`; prefer-represented: `crates/wow-world-loot/src/state/authority_access.rs:28-34` | `Loot` is reached from the owning object on its own map (`LootHandler.cpp:270`) | For the same gameobject/creature, one path resolves the map from `current_canonical_player_map_key_like_cpp()`, the other from `canonical_object_lookup_map_key_like_cpp(player_map_id_like_cpp())`; with a detached player the two can select different maps | **Behaviour** | `unknown` — the reachable difference needs a detached-player reproduction before classification |
| D-11 | `represented_*` that are test-only | 222 of 765 definitions behind `cfg(any(test, feature = "test-fixtures"))` (heuristic, ±8 lines); e.g. `crates/wow-world-core/src/session/state/movement.rs:28-110` | n/a | Tests exercise represented mirrors and their fallbacks that **do not exist in the production binary**; passing unit tests therefore prove a path production does not take | **Structural** (build-configuration divergence, not a C++ behaviour divergence) | `intentional-departure contract needed` — the F6 and #153 audits must not treat these tests as parity evidence |
| D-12 | Owner-first loot-list send ordering | `crates/wow-world-application/src/loot_release/mod.rs:318-321` | `Loot.cpp:637-655`, container `Loot.h:350` (`GuidUnorderedSet`) | The Rust split sends the owner first and the other allowed looters afterwards | **Structural** — C++ order is unspecified for an unordered set, so no divergence is provable | `unknown` (order claim `unverified`) |
| D-13 | `canonical_access` facade rename | `crates/wow-world-core/src/session/canonical_access/mod.rs:1-76` (facade structs); `canonical_access/operations.rs:182` reads canonical | n/a | The facade re-partitions borrows without changing the reads; the name `represented_*` survives on 19 functions there, 2 of which read the canonical Player | **Structural** | none — no behaviour work |

**Count: 13 divergences recorded. 9 are behaviour differences** (D-01…D-09), **1 is a behaviour
difference candidate pending reproduction** (D-10), **2 are structural** (D-11, D-12), **1 is
structural with no action** (D-13). Behaviour-difference total including D-10 as a candidate:
**10 of 13**.

## 6. Concrete work items for the F6 slices

Ordered by risk, each sized to one bounded slice and stated with the evidence that bounds it.
**Nothing here authorises a gameplay repair inside a structural slice, and nothing here has been
executed.**

| # | Slice | Bounded by | Risk |
|---|---|---|---|
| F6-1 | **Inventory of the production-reachable `represented_*` surface.** Produce the exact list of `represented_*` definitions compiled into a production build of `world-server` (not the heuristic in §1), grouped by the stage-2 buckets. | Purely mechanical: `cargo check` feature resolution or a build-script-free `cfg` scan. No behaviour touched. `crates/wow-world/Cargo.toml:9-11` and `crates/world-server/Cargo.toml:10-43` bound what can be included. | lowest |
| F6-2 | **Re-derive each `resolved_*` seam's absent-owner contract.** For every `resolved_*` that returns `Option`, record what production does when the canonical owner is absent, and whether any caller turns `None` into a packet value. | `crates/wow-world-core/src/session/player_vitals_adapter.rs:89-113` is the reference shape; the set of `resolved_*` names is enumerable by grep (40+ sites, see §2 anchors). Read-only; no code change. | low |
| F6-3 | **Name/source reconciliation for `represented_*` that read canonical.** Split the 89 definitions whose body reads canonical APIs into "rename" and "genuine mirror", starting with `canonical_access/operations.rs:182` and `canonical_access/quest_reward_owner.rs:561`. | Bounded by a fixed candidate list; pure rename if the F6-1 inventory confirms no owner change. | low |
| F6-4 | **D-03 map-key fallback removal analysis.** Enumerate every `current_legacy_runtime_map_key_like_cpp` caller (grep: 20 sites incl. `wow-world-loot`, `wow-world-core`, `wow-world-entities`, `wow-world` tests) and state, per caller, what C++ reads there. | `crates/wow-world-core/src/session/instances/map_key.rs:53-62`; caller list is closed and grep-verifiable. Read-only. | medium |
| F6-5 | **D-12/D-04 loot fanout identity and order.** Verify from source whether `loot_delivery_recipient(_, map_id, instance_id)` can select a recipient on a different instance than the canonical one, and whether C++ order is observable at all (container is `GuidUnorderedSet`, `Loot.h:350`). | Two files plus one container type; the D-04 order claim is already flagged `unverified`. | medium |
| F6-6 | **D-05/D-06/D-07 session-local GameObject state.** Decide, per field (`world-entities/src/state.rs:40-42,54`; `wow-world-loot/src/state.rs:77-84`), whether the canonical `GameObject` already carries the equivalent before any code moves. | Field-by-field; each decision is evidence-checkable against `GameObject.h:464,483,512` and `GameObject.cpp:3795`. | medium |
| F6-7 | **D-01/D-02/D-10 loot-authority and creature-entity reconciliation contract.** Write the explicit contract for the mirror reconciliation (including the fail-closed `None` and the retired tombstone) or classify it as an intentional departure. | `authority.rs:41-176` and `creature_canonical_adapter.rs:29-70,178-210` are the whole surface; the C++ side is one `unique_ptr<Loot>` per object (`Creature.h:236`). | high |
| F6-8 | **D-08/D-09 legacy runtime and map manager retirement plan.** Decide the single owner, state the C++ phase order it must reproduce (`World.cpp:2748` → `MapManager.cpp:287` → `Creature.cpp:696`), and list the two-producer and two-store call sites to retire. | Bounded by §3 and §4 caller tables. **Not a structural slice** — it changes which clock and which store produce observable state. | highest |

## 7. Explicitly unverified

| Item | Why |
|---|---|
| The exact production/test split of the 765 `represented_*` definitions (222/543 figure) | ±8-line lookback heuristic over `grep` output, not a compiled-cfg enumeration. F6-1 exists to replace it. |
| The 89-of-765 "body reads canonical" figure | 20-line window over `grep` output; nested calls outside the window are missed and same-line mentions may be comments. |
| C++ `_allowedLooters` send order (D-12) | Container is `GuidUnorderedSet` (`Loot.h:350`), so no C++ order is defined; Rust's owner-first split cannot be shown divergent from source alone. |
| Inventory/items, progression, instances, battleground, collections, lifecycle duality | Not traced in this audit ("Not established" in §2). |
| Whether the owner's loot-list delivery path reproduces the C++ packet for the owner in every case | The owner branch was located (`loot_release/mod.rs:318-320`) but its packet content was not compared byte-for-byte with `Loot::NotifyLootList`. |
| Whether any `resolved_*` `None` currently reaches a client-visible packet field | Callers not enumerated. |
| The C++ counterpart of `RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME` | Verified absent from `src/` and `sql/` in the C++ checkout; absence of a match is not proof that no equivalent mechanism exists elsewhere. |
| Runtime reachability of D-10 | Requires a detached-player or far-teleport reproduction that this read-only audit could not run. |
