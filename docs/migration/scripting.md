# Migration: Scripting (ScriptMgr framework)

> **C++ canonical path:** `src/server/game/Scripting/` (`ScriptMgr`, `ScriptReloadMgr`, `ScriptSystem`)
> **Rust target crate(s):** `crates/wow-script/` (the framework: `ScriptMgr` equivalent + every `*Script` trait + dispatch tables); content scripts live in `crates/wow-scripts/` (covered by `scripts.md`).
> **Layer:** L7 (Game systems framework — depends on virtually every L0–L6 layer because every script type takes pointers to game entities; depended on by `wow-scripts` and indirectly by every gameplay path that fires `ScriptMgr::On*` hooks).
> **Status:** ❌ not started — `crates/wow-script/src/lib.rs` is **0 bytes**. There is no `ScriptMgr`, no script trait, no dispatch table, no registration macro, no reload manager. **Every** `sScriptMgr->OnX(...)` callsite in C++ has no Rust counterpart yet, which means none of the ~160 script hooks fire. Boss AI, instance scripts, spell scripts, command scripts, item-use scripts, area triggers, gossip, and PlayerScript hooks (login/logout/zone change/level up/etc.) are all silent.
> **Audited vs C++:** ✅ audited 2026-05-01 (status confirmed ❌ — `wc -l` on `lib.rs` returns 0)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`ScriptMgr` is TrinityCore's universal extensibility hub. Game core code calls `sScriptMgr->OnX(args...)` whenever a scriptable event happens (player login, creature spawn, item use, packet send, map create, world tick, …); `ScriptMgr` fans the call out to every registered subscriber — boss AIs, spell scripts, command handlers, achievement criteria checks, transport hooks, etc. The framework also supplies the dynamic-library reload mechanism (`ScriptReloadMgr`, hot-reload of compiled `.so` modules) and a small `SystemMgr` for spline-chain data shared by scripts. Scripts inherit from one of ~35 `*Script` base classes, name themselves with a string at construction, and register themselves into a per-type `ScriptRegistry<T>` simply by being instantiated via `new bossname()` inside an `AddSC_*` function.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Scripting/ScriptMgr.cpp` | 3271 | `prefix` |
| `game/Scripting/ScriptMgr.h` | 1421 | `prefix` |
| `game/Scripting/ScriptReloadMgr.cpp` | 1626 | `prefix` |
| `game/Scripting/ScriptReloadMgr.h` | 85 | `prefix` |
| `game/Scripting/ScriptSystem.cpp` | 120 | `prefix` |
| `game/Scripting/ScriptSystem.h` | 55 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Scripting/ScriptMgr.h` | 1421 | Every `*Script` base class (~35), the `ScriptObject` root, `GenericSpellAndAuraScriptLoader`, `GenericCreatureScript<AI>`, `GenericGameObjectScript<AI>`, `GenericAreaTriggerEntityScript<AI>` template helpers, the `Register*` macros, `ScriptMgr` singleton facade with all `On*` fan-out methods (~115 hook entry points). |
| `src/server/game/Scripting/ScriptMgr.cpp` | 3271 | `ScriptObject` ctor/dtor (auto-registers via `RegisterSelf()`), the per-type `ScriptRegistry<T>` (`AddScript`, `GetScriptById`, name→id resolution, swap-context bookkeeping), every `ScriptMgr::OnX` implementation (uses `FOREACH_SCRIPT(T)` macros and per-call `LockModuleReferenceLock` for hot-reload safety), spell summary cache `FillSpellSummary()`, script-context machinery (`SetScriptContext` / `SwapScriptContext` / `ReleaseScriptContext`), `NotifyScriptIDUpdate` / `SyncScripts` for hotfix-driven re-id. |
| `src/server/game/Scripting/ScriptReloadMgr.h` | 85 | `ModuleReference` interface (a strong handle to a loaded shared library; keeps `.so` resident while any `SpellScript`/`AuraScript` from it is alive), `ScriptReloadMgr` singleton (Initialize/Update/Unload, `AcquireModuleReferenceOfContext`). |
| `src/server/game/Scripting/ScriptReloadMgr.cpp` | 1626 | The hot-reload runtime: filesystem watcher on `scripts/` source dirs, CMake build invocation, atomic `.so` swap, per-module reference counting, lazy unload. Heavy use of `boost::filesystem` and `boost::process`. |
| `src/server/game/Scripting/ScriptSystem.h` | 55 | `SystemMgr` singleton — narrowly scoped storage for `SplineChainLink` waypoint chains keyed by `(creature entry, chain id)`, used by `ScriptedAI::CreateSplineChain`. |
| `src/server/game/Scripting/ScriptSystem.cpp` | 120 | `LoadScriptSplineChains()` → `SELECT … FROM script_spline_chain_meta JOIN script_spline_chain_waypoints`, getter overloads. |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ScriptObject` | abstract base class | Every script inherits from this; stores `_name`, friend of `ScriptMgr`. Non-copyable, non-movable. |
| `SpellScriptLoader` | `*Script` base | Returns `SpellScript*` / `AuraScript*` factories (the actual logic lives in `wow-spell` companion classes). |
| `ServerScript` | `*Script` base | `OnNetworkStart/Stop`, `OnSocketOpen/Close`, `OnPacketSend/Receive` (mutates copy, original is untouched). |
| `WorldScript` | `*Script` base | `OnOpenStateChange`, `OnConfigLoad`, `OnMotdChange`, `OnShutdownInitiate/Cancel`, `OnUpdate(diff)`, `OnStartup`, `OnShutdown`. |
| `FormulaScript` | `*Script` base | Tweak XP/honor/group-rate calc: `OnHonorCalculation`, `OnGrayLevelCalculation`, `OnColorCodeCalculation`, `OnZeroDifferenceCalculation`, `OnBaseGainCalculation`, `OnGainCalculation`, `OnGroupRateCalculation`. |
| `MapScript<T>` | template helper | `OnCreate/Destroy`, `OnPlayerEnter/Leave`, `OnUpdate`. Instantiated as `WorldMapScript`, `InstanceMapScript`, `BattlegroundMapScript`. |
| `WorldMapScript` | concrete | Map-id-bound script; one per non-instance map. |
| `InstanceMapScript` | concrete | Provides `GetInstanceScript(InstanceMap*)` factory. **The bridge between MapManager and per-instance state machines** (`InstanceScript`). |
| `BattlegroundMapScript` | concrete | BG-map-id-bound; less common because BGs live mostly in `Battleground` subclasses. |
| `ItemScript` | `*Script` base | `OnQuestAccept`, `OnUse`, `OnExpire`, `OnRemove`, `OnCastItemCombatSpell`. |
| `UnitScript` | `*Script` base | `OnHeal`, `OnDamage`, `ModifyPeriodicDamageAurasTick`, `ModifyMeleeDamage`, `ModifySpellDamageTaken`. |
| `CreatureScript` | `*Script` base | `GetAI(Creature*)` factory — the hook point for every boss AI. |
| `GameObjectScript` | `*Script` base | `GetAI(GameObject*)` factory. |
| `AreaTriggerScript` | `*Script` base | `OnTrigger(player, trigger, entered)`. |
| `OnlyOnceAreaTriggerScript` | derived | Self-deactivating area trigger. |
| `BattlefieldScript` | `*Script` base | `GetBattlefield(Map*)` factory (Wintergrasp etc.). |
| `BattlegroundScript` | `*Script` base | `GetBattleground()` factory by `BattlegroundTypeId`. |
| `OutdoorPvPScript` | `*Script` base | `GetOutdoorPvP(Map*)` factory (Hellfire/Nagrand/Silithus/Terokkar/Zangarmarsh). |
| `CommandScript` | `*Script` base | Returns `vector<ChatCommandBuilder>` — every `.foo bar` GM command. |
| `WeatherScript` | `*Script` base | `OnChange(weather, state, grade)`, `OnUpdate`. |
| `AuctionHouseScript` | `*Script` base | `OnAuctionAdd/Remove/Successful/Expire`. |
| `ConditionScript` | `*Script` base | `OnConditionCheck` — custom predicate type accessible from `conditions` table. |
| `VehicleScript` | `*Script` base | `OnInstall/Uninstall/Reset/InstallAccessory/AddPassenger/RemovePassenger`. |
| `DynamicObjectScript` | `*Script` base | `OnUpdate`. |
| `TransportScript` | `*Script` base | `OnAddPassenger/AddCreaturePassenger/RemovePassenger/Relocate/Update`. |
| `AchievementScript` | `*Script` base | `OnCompleted`. |
| `AchievementCriteriaScript` | `*Script` base | `OnCheck` — used by criteria with `MODIFIER_TREE_TYPE_REQUIRED_SCRIPT`. |
| `PlayerScript` | `*Script` base | The fattest hook surface: ~30 methods covering kill/death, level/talent change, money, XP, reputation, duel, chat (5 overloads), emotes, spell cast, login/logout/create/delete/save, instance bind, zone change, map change, quest status, repop, movie complete, player-choice response. |
| `AccountScript` | `*Script` base | `OnAccountLogin`, `OnFailedAccountLogin`, `OnEmailChange/PasswordChange` (+ failure variants). |
| `GuildScript` | `*Script` base | Add/remove member, MOTD/info change, create, disband, bank deposit/withdraw/move, generic events. |
| `GroupScript` | `*Script` base | Add/invite/remove member, leader change, disband. |
| `AreaTriggerEntityScript` | `*Script` base | `GetAI(AreaTrigger*)` factory for the spawned-entity flavor of area triggers (distinct from server-side area triggers). |
| `ConversationScript` | `*Script` base | `OnConversationCreate/Start/LineStarted/Update`. |
| `SceneScript` | `*Script` base | `OnSceneStart/TriggerEvent/Cancel/Complete`. |
| `QuestScript` | `*Script` base | `OnQuestStatusChange`, `OnAcknowledgeAutoAccept`, `OnQuestObjectiveChange`. |
| `WorldStateScript` | `*Script` base | `OnValueChange(stateId, oldVal, newVal, Map*)`. |
| `EventScript` | `*Script` base | `OnTrigger(object, invoker, eventId)` — for `event_scripts` table dispatched by spell `EFFECT_SEND_EVENT` and gobject `GAMEOBJECT_TYPE_GOOBER`. |
| `ScriptRegistry<T>` | template | Per-script-type registry: `AddScript(T*)`, `GetScriptById(scriptId, contextSwap=true)`, name→id resolver, context swap state. Has macro `FOREACH_SCRIPT(T)` to iterate. |
| `ScriptMgr` | singleton class | Facade; holds the script-loader callback, per-type registries, current script context, the spell summary, the script-id-update notification flag. |
| `ModuleReference` | abstract | Strong handle keeping a hot-reloaded `.so` resident; carries module name + git revision hash + filesystem path. |
| `ScriptReloadMgr` | singleton | Watches `scripts/` source dirs, drives CMake builds, atomically swaps modules, lazy-unloads. |
| `SystemMgr` (`sScriptSystemMgr`) | singleton | Loads/serves `script_spline_chain_meta` waypoint data. |

Macros that scripts use to register themselves:

| Macro | Expands to | Used for |
|---|---|---|
| `RegisterCreatureAI(ai_name)` | `new GenericCreatureScript<ai_name>(#ai_name)` | Bind a `CreatureAI` subclass to the script name `ai_name`. |
| `RegisterCreatureAIWithFactory(ai, factory_fn)` | `new FactoryCreatureScript<ai, &factory_fn>(#ai)` | Same but with a factory function (instance-aware AIs). |
| `RegisterGameObjectAI(ai_name)` | `new GenericGameObjectScript<ai_name>(#ai_name)` | Bind a `GameObjectAI` subclass. |
| `RegisterGameObjectAIWithFactory` | `new FactoryGameObjectScript<...>` | Factory variant. |
| `RegisterAreaTriggerAI(ai_name)` | `new GenericAreaTriggerEntityScript<ai_name>(#ai_name)` | Bind an `AreaTriggerAI` subclass. |
| `RegisterSpellScript(spell_script)` | `new GenericSpellAndAuraScriptLoader<spell_script, ...>(#spell_script, std::make_tuple())` | Bind a `SpellScript` (no extra ctor args). |
| `RegisterSpellScriptWithArgs(s, name, ...)` | tuple-perfect-forwarding variant | Spell scripts that need ctor args. |
| `RegisterSpellAndAuraScriptPair(s1, s2)` | combined | Spell + aura sibling scripts under one name. |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ScriptMgr::Initialize()` | Bootstrap: invoke `_script_loader_callback` (the linker-generated `AddScripts()` from `ScriptLoader.cpp.in.cmake`), then `LoadDatabase()`, `FillSpellSummary()` | `ScriptObject` ctors → `ScriptRegistry<T>::AddScript`, `sObjectMgr` |
| `ScriptMgr::SetScriptContext(string)` / `SwapScriptContext` / `ReleaseScriptContext` | Three-phase context switch for hot-reload: stage adds, swap atomically, retire old | `ScriptRegistry<T>::SwapContext` for every type |
| `ScriptMgr::CreateSpellScripts(spellId, vec, invoker)` | For every registered SpellScriptLoader bound to `spellId` (via `spell_script_names` table), call `GetSpellScript()` and append | `ObjectMgr::GetSpellScriptsBounds`, `SpellScriptLoader::GetSpellScript` |
| `ScriptMgr::CreateAuraScripts(spellId, vec, invoker)` | Mirror of above for `AuraScript` | `SpellScriptLoader::GetAuraScript` |
| `ScriptMgr::GetCreatureAI(Creature*)` | Resolve `creature->GetScriptId()` → `CreatureScript*` → `GetAI(creature)` | `ScriptRegistry<CreatureScript>::GetScriptById`, `CreatureScript::GetAI` |
| `ScriptMgr::GetGameObjectAI(GameObject*)` | Same for gobjects | `ScriptRegistry<GameObjectScript>::GetScriptById` |
| `ScriptMgr::CreateInstanceData(InstanceMap*)` | Resolve `map->GetEntry()->ScriptId` → `InstanceMapScript*` → `GetInstanceScript(map)` | `ScriptRegistry<InstanceMapScript>::GetScriptById` |
| `ScriptMgr::CreateBattlefield(scriptId, Map*)` | Wintergrasp factory (extensible to other BFs) | `ScriptRegistry<BattlefieldScript>` |
| `ScriptMgr::CreateBattleground(typeId)` | BG factory by `BattlegroundTypeId` | `ScriptRegistry<BattlegroundScript>` |
| `ScriptMgr::CreateOutdoorPvP(scriptId, Map*)` | OPvP factory | `ScriptRegistry<OutdoorPvPScript>` |
| `ScriptMgr::OnAreaTrigger(Player*, AreaTriggerEntry, entered)` | Dispatch to the area trigger script bound by `areatrigger_scripts` table | `ScriptRegistry<AreaTriggerScript>::GetScriptByName` |
| `ScriptMgr::OnConditionCheck(Condition*, ConditionSourceInfo&)` | Custom-script condition type evaluator | `ScriptRegistry<ConditionScript>::GetScriptById` |
| `ScriptMgr::OnPlayerLogin(Player*, bool firstLogin)` | Fan out to every `PlayerScript` | `FOREACH_SCRIPT(PlayerScript)->OnLogin` |
| `ScriptMgr::OnWorldUpdate(diff)` | Fan out to every `WorldScript` | `FOREACH_SCRIPT(WorldScript)->OnUpdate` |
| `ScriptMgr::OnPacketReceive(WorldSession*, WorldPacket const&)` | Mutates a copy; called from `WorldSession::ReadDataHandler` | `FOREACH_SCRIPT(ServerScript)->OnPacketReceive` |
| `ScriptMgr::GetChatCommands()` | Aggregate every `CommandScript::GetCommands()` for the `Trinity::ChatCommands` table | `FOREACH_SCRIPT(CommandScript)` |
| `ScriptMgr::NotifyScriptIDUpdate()` / `SyncScripts()` | Hotfix-driven re-resolution when `script_id` columns change at runtime | `ObjectMgr::*::SetScriptId`, `MMapMgr` rebuild flags |
| `sScriptReloadMgr->Update()` | Watch tick: detects modified files, kicks CMake, swaps module, calls `ScriptMgr::SetScriptContext`/`SwapScriptContext` | filesystem watcher, `boost::process` |
| `sScriptSystemMgr->LoadScriptSplineChains()` | Load per-creature waypoint chain library | `WorldDatabase.Query` |
| `GetSplineChain(entry, chainId)` / `GetSplineChain(creature, id)` | Lookup spline chain | `m_mSplineChainsMap` |

The `FOREACH_SCRIPT(T)` macro is implemented as a lambda over `ScriptRegistry<T>::ScriptPointerList`, with `LockModuleReferenceLock` held to prevent module unload mid-call.

---

## 5. Module dependencies

**Depends on:**
- `ObjectMgr` — for `script_id` resolution and the `spell_script_names` / `areatrigger_scripts` / `creature_template.ScriptName` lookups.
- Every `*Script` base class takes pointers to game entities (`Creature`, `Player`, `Map`, `Item`, `Spell`, `Aura`, `Vehicle`, `Transport`, …), so transitively depends on Entities (L4), Spells (L5), Map (L4), DBC stores (L1), Quest (L6), Achievement (L7), AreaTrigger (L7), Conversation (L7), Scene (L7).
- `Trinity::ChatCommands` for `CommandScript`.
- `WorldDatabase` (script_spline_chain_meta in `ScriptSystem`).
- `boost::filesystem`, `boost::process`, dynamic-library loading (`dlopen`/`LoadLibrary`) in `ScriptReloadMgr`.

**Depended on by:**
- **Everything in `src/server/scripts/`** (~725 files, ~294k lines) — see `scripts.md`.
- `WorldSession` and `Player` (login/logout/zone change/etc. fire `sScriptMgr->OnPlayer*`).
- `Map`/`InstanceMap` (`OnCreate`/`OnDestroy`/`OnUpdate`/`OnPlayerEnter` and `CreateInstanceData`).
- `Spell`/`Aura` (instantiate scripts via `CreateSpellScripts`/`CreateAuraScripts`).
- `Battleground`, `OutdoorPvP`, `Battlefield` (created via factories).
- `WorldSocket` (`OnSocketOpen`/`Close`/`PacketReceive`/`Send`).
- `World::Update`, `World::ShutdownServ`, `World::LoadConfigSettings` (fire `WorldScript` hooks).
- `ConditionMgr` (custom condition type 33 = `CONDITION_SCRIPT` calls `OnConditionCheck`).

---

## 6. SQL / DB queries (if any)

Direct queries from this module:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, chainId, splineId, expectedDuration, msUntilNext FROM script_spline_chain_meta ORDER BY entry, chainId, splineId` | Spline chain metadata (which chains exist for a creature entry) | world |
| `SELECT entry, chainId, splineId, wpId, x, y, z FROM script_spline_chain_waypoints ORDER BY entry, chainId, splineId, wpId` | Waypoints inside each spline chain | world |

Indirect (resolved by name → id):

| Statement / Source | Purpose | DB |
|---|---|---|
| `creature_template.ScriptName` → `ScriptRegistry<CreatureScript>` | Resolve creature → AI script | world (loaded by ObjectMgr) |
| `gameobject_template.ScriptName` → `ScriptRegistry<GameObjectScript>` | Resolve gobject → AI script | world |
| `instance_template.ScriptName` (via `MapEntry::ScriptId`) → `InstanceMapScript` | Resolve map → instance script | world |
| `spell_script_names` (spellId, ScriptName) → `SpellScriptLoader` | Bind spell scripts to spell ids | world |
| `areatrigger_scripts` (entry, ScriptName) → `AreaTriggerScript` | Bind area trigger scripts | world |
| `conditions` (rows with `ConditionType=33`) → `ConditionScript` | Custom condition predicates | world |

No DBC/DB2 stores read directly by this module.

---

## 7. Wire-protocol packets (if any)

`ScriptMgr` itself sends no packets, but it observes the entire wire stream:

| Opcode | Direction | Hook |
|---|---|---|
| (any) | client → server | `ServerScript::OnPacketReceive(WorldSession*, WorldPacket&)` — copy of the inbound packet |
| (any) | server → client | `ServerScript::OnPacketSend(WorldSession*, WorldPacket&)` — copy of the outbound packet |
| `SMSG_SCRIPT_*` (none) | — | There is no opcode family for scripts; everything fans out internally. |

The script framework is fundamentally a server-side dispatcher; protocol effects come from the scripts themselves (e.g. a boss AI sending `SMSG_SPELL_GO`).

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-script` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-scripts` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-script/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-script/Cargo.toml` | `file` | 1 | 11 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-scripts/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-script/src/lib.rs` — **0 lines** (empty placeholder).
- `crates/wow-script/Cargo.toml` — declares deps on `wow-core`, `wow-constants`, `inventory`. No types yet.
- `crates/wow-scripts/src/lib.rs` — **0 lines** (placeholder for content scripts; see `scripts.md`).

**What's implemented:** **Nothing.** The `inventory` crate is already a workspace dep, which suggests the intended approach is the same `inventory::submit!` static-registration pattern already used for packet handlers in `wow-handler` (see `crates/wow-handler/src/lib.rs`). But not a single hook trait, dispatch table, or registration macro exists yet.

**What's missing vs C++:** Effectively the entire 6,578-line `Scripting/` subsystem:
- `ScriptObject` / per-type base traits (~35 of them).
- `ScriptMgr` singleton with ~115 fan-out `On*` methods.
- `ScriptRegistry<T>` per-type list + name→id resolution.
- The `Register*` macro family (`RegisterCreatureAI`, `RegisterSpellScript`, etc.).
- Script context machinery (`SetScriptContext` / `SwapScriptContext` / `ReleaseScriptContext`).
- `ScriptReloadMgr` (hot-reload of compiled modules — almost certainly out of scope for the Rust port; Rust dynamic loading via `libloading` is feasible but adds a lot of complexity).
- `SystemMgr` spline chain loader.
- Every callsite in the rest of the codebase that *would* call `sScriptMgr->OnX` — those calls don't exist yet either, so even adding the framework requires also wiring the call sites.

**Suspicious / likely divergent (hypothesis pre-audit):**
- The eventual Rust shape will likely be a `ScriptRegistry` per hook trait built on `inventory::submit!` (compile-time registration) plus a runtime side-table for the name→script-id binding from DB. Hot-reload is unlikely to be supported in the first cut — consider it explicit non-goal.
- `PlayerScript::OnChat` has 5 overloads in C++ (no receiver / Player / Group / Guild / Channel). In Rust this will probably collapse into one method taking an `enum ChatTarget { None, Player(...), Group(...), Guild(...), Channel(...) }`.
- C++ scripts get raw `Player*`/`Creature*` pointers; the hot path is overlap with `WorldSession`/`MapManager` borrow rules. The Rust trait shapes need to match the project's actor-style "send via channel" pattern (`send_tx`) used inside tick methods, otherwise every script will hit the borrow-checker dragon described in `CLAUDE.md`.
- The `ScriptReloadMgr` hot-reload mechanism is heavy on `boost::process` and platform-specific dlopen quirks. Skipping it entirely is the sane move; ship as static linking + restart-to-reload.

**Tests existing:** **None** in `crates/wow-script/`. (`cargo test -p wow-script` runs zero tests.)

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SCRIPTING.WBS.001** Partir y cerrar la migracion auditada de `game/Scripting/ScriptMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3271 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTING.WBS.002** Partir y cerrar la migracion auditada de `game/Scripting/ScriptMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1421 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTING.WBS.003** Partir y cerrar la migracion auditada de `game/Scripting/ScriptReloadMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1626 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#SCRIPTING.WBS.004** Cerrar la migracion auditada de `game/Scripting/ScriptReloadMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.h`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTING.WBS.005** Cerrar la migracion auditada de `game/Scripting/ScriptSystem.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptSystem.cpp`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCRIPTING.WBS.006** Cerrar la migracion auditada de `game/Scripting/ScriptSystem.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptSystem.h`
  Rust target: `crates/wow-script`, `crates/wow-scripts`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbering: `#SCRIPTING.N`. Complexity legend: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h, split further).

### Phase A — bootstrap framework (no game-side wiring yet)

- [ ] **#SCRIPTING.1** Define `trait ScriptObject` (just `fn name(&self) -> &str`) and a `ScriptId` newtype (`u32`). Decide: trait objects (`Box<dyn ScriptObject>`) vs `enum`-per-hook. Recommend trait objects + `inventory::submit!`. (M)
- [ ] **#SCRIPTING.2** Build the per-hook static registry pattern. One example: `pub trait PlayerHook: Send + Sync { fn on_login(&self, /* ... */) {} … }` + `inventory::collect!(&'static dyn PlayerHook)` + a thin `ScriptMgr::on_player_login(...)` that iterates collected hooks. (M)
- [ ] **#SCRIPTING.3** Replicate the macro layer: `register_creature_ai!`, `register_game_object_ai!`, `register_spell_script!`, `register_command_script!`. Macros expand to `inventory::submit!(MyAi as &dyn CreatureScript)`. Mirror C++ `RegisterCreatureAI(ai_name)` ergonomics. (H)
- [ ] **#SCRIPTING.4** Implement the name→script-id lookup table loaded from `creature_template.ScriptName` / `gameobject_template.ScriptName` / `spell_script_names` / `areatrigger_scripts`. The C++ `ScriptNameContainer`-style interner now exists in `wow-data` and is wired into startup/session resources for currently loaded creature/gameobject template script names; `areatrigger_scripts` has a C++-validated represented store but is not wired at startup until `AreaTriggerStore` is authoritative; direct DB loader coverage for `spell_script_names` and `wow-script` consumers still need wiring. (M)

### Phase B — port the hook traits (one trait per C++ `*Script`)

Each line below is one trait. Bodies will fill in as game-side hookpoints land.

- [ ] **#SCRIPTING.5** `ServerScript` trait — `on_network_start/stop`, `on_socket_open/close`, `on_packet_receive/send`. (M)
- [ ] **#SCRIPTING.6** `WorldScript` trait — `on_open_state_change`, `on_config_load`, `on_motd_change`, `on_shutdown_initiate/cancel`, `on_update`, `on_startup`, `on_shutdown`. (M)
- [ ] **#SCRIPTING.7** `FormulaScript` trait — 7 honor/XP/group-rate hooks. (M)
- [ ] **#SCRIPTING.8** `WorldMapScript` / `InstanceMapScript` / `BattlegroundMapScript` traits. The `InstanceMapScript::get_instance_script(map)` factory is the bridge to `wow-instance` and is the most load-bearing one in this group. (H)
- [ ] **#SCRIPTING.9** `ItemScript` trait — `on_quest_accept`, `on_use`, `on_expire`, `on_remove`, `on_cast_item_combat_spell`. (M)
- [ ] **#SCRIPTING.10** `UnitScript` trait — `on_heal`, `on_damage`, `modify_periodic_damage_auras_tick`, `modify_melee_damage`, `modify_spell_damage_taken`. (M)
- [ ] **#SCRIPTING.11** `CreatureScript` trait + `GenericCreatureScript<AI>` shim — `get_ai(creature) -> Box<dyn CreatureAI>`. (H, blocks every boss script)
- [ ] **#SCRIPTING.12** `GameObjectScript` trait + `GenericGameObjectScript<AI>` shim. (H)
- [ ] **#SCRIPTING.13** `AreaTriggerScript` (server-side) + `OnlyOnceAreaTriggerScript` helper. (M)
- [ ] **#SCRIPTING.14** `BattlefieldScript` + factory. (M)
- [ ] **#SCRIPTING.15** `BattlegroundScript` + factory. (M)
- [ ] **#SCRIPTING.16** `OutdoorPvPScript` + factory. (M)
- [ ] **#SCRIPTING.17** `CommandScript` — returns a `Vec<ChatCommandBuilder>`-equivalent. **Cross-cuts** with the chat command parser in `wow-chat` / `wow-handler`; pick the syntax once and reuse. (H)
- [ ] **#SCRIPTING.18** `WeatherScript` — `on_change`, `on_update`. (L)
- [ ] **#SCRIPTING.19** `AuctionHouseScript` — 4 hooks. (L)
- [ ] **#SCRIPTING.20** `ConditionScript::on_check` (used by `ConditionType::SCRIPT` in `conditions` table). Coupled with `wow-conditions` work. (M)
- [ ] **#SCRIPTING.21** `VehicleScript` — 6 hooks. (M)
- [ ] **#SCRIPTING.22** `DynamicObjectScript::on_update`. (L)
- [ ] **#SCRIPTING.23** `TransportScript` — 5 hooks. (M)
- [ ] **#SCRIPTING.24** `AchievementScript::on_completed`. (L)
- [ ] **#SCRIPTING.25** `AchievementCriteriaScript::on_check`. (L)
- [ ] **#SCRIPTING.26** `PlayerScript` — **biggest trait, ~30 hooks**. Split chat into one method with a `ChatTarget` enum. **(H)**
- [ ] **#SCRIPTING.27** `AccountScript` — 6 hooks. (L)
- [ ] **#SCRIPTING.28** `GuildScript` — 11 hooks. (M)
- [ ] **#SCRIPTING.29** `GroupScript` — 5 hooks. (L)
- [ ] **#SCRIPTING.30** `AreaTriggerEntityScript::get_ai(at) -> Box<dyn AreaTriggerAI>`. (M)
- [ ] **#SCRIPTING.31** `ConversationScript` — 4 hooks. (M)
- [ ] **#SCRIPTING.32** `SceneScript` — 4 hooks. (L)
- [ ] **#SCRIPTING.33** `QuestScript` — 3 hooks. (L)
- [ ] **#SCRIPTING.34** `WorldStateScript::on_value_change`. (L)
- [ ] **#SCRIPTING.35** `EventScript::on_trigger`. (L)
- [ ] **#SCRIPTING.36** `SpellScriptLoader` + `SpellScript` / `AuraScript` traits. **XL** — `SpellScript`/`AuraScript` themselves are large hierarchies (effect handlers, hit/miss targets, `BeforeCast`/`AfterCast`/`OnHit`/`OnMiss`/`OnCalcCrit`, etc.) and should get their own migration doc under `crates/wow-spell/`. Split into ≥3 sub-tasks before doing.

### Phase C — wire game side

- [ ] **#SCRIPTING.37** Plumb `ScriptMgr::on_player_login` etc. into `WorldSession::login_handler` and friends. Audit ~40 callsites of `sScriptMgr->OnPlayer*` in C++. (H)
- [ ] **#SCRIPTING.38** Plumb `ScriptMgr::on_world_update(diff)` into the world tick. (L)
- [ ] **#SCRIPTING.39** Plumb `ScriptMgr::on_create_map` / `on_destroy_map` into `MapManager::create_map`/`destroy_map`. Currently `MapManager` has no hooks. (M)
- [ ] **#SCRIPTING.40** Plumb `ScriptMgr::create_instance_data(map)` into `InstanceMap::Create` (or Rust equivalent — `wow-instance` may not have this yet). Coupled with the `instances.md` migration. (H)
- [ ] **#SCRIPTING.41** Plumb `ScriptMgr::on_packet_receive/send` into `world_socket` after every successful read/before every write. **Performance hot path** — keep the no-subscribers fast path branchless. (M)
- [ ] **#SCRIPTING.42** Plumb `ScriptMgr::on_condition_check` into the `wow-conditions` evaluator. (L; depends on `#CONDITIONS.*`)

### Phase D — supporting systems

- [ ] **#SCRIPTING.43** Port `SystemMgr` / spline chain loader. Two world-DB tables (`script_spline_chain_meta`, `script_spline_chain_waypoints`). (M)
- [ ] **#SCRIPTING.44** Decide on hot-reload strategy. **Recommend**: explicit non-goal for v1; ship static + restart. If revisited, gate behind a feature flag and use `libloading`. (None / XL if revisited)
- [ ] **#SCRIPTING.45** Spell script summary cache (`FillSpellSummary`) — used by SpellScriptLoader to skip work when no scripts exist for a spell. (M; depends on #SCRIPTING.36)
- [ ] **#SCRIPTING.46** Script context machinery for any future hot-reload. **Skip if #SCRIPTING.44 stays a non-goal.** (None / H)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SCRIPTING.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 6578 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-handler`, `crates/wow-script`, `crates/wow-scripts`. | `cargo test -p wow-constants && cargo test -p wow-core && cargo test -p wow-handler` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SCRIPTING.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 6578 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-handler`, `crates/wow-script`, `crates/wow-scripts`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SCRIPTING.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 6578 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-handler`, `crates/wow-script`, `crates/wow-scripts`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SCRIPTING.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 6578 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-handler`, `crates/wow-script`, `crates/wow-scripts`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: a `register_creature_ai!(MyBoss)` call ends up resolvable by `ScriptMgr::get_creature_ai` for a creature whose template has `ScriptName="MyBoss"`.
- [ ] Test: registering N hooks on `WorldScript::on_update` results in all N being called once per tick.
- [ ] Test: `PlayerScript::on_login` is fired with `first_login=true` exactly once for newly-created characters.
- [ ] Test: `AreaTriggerScript::on_trigger` fires when a player enters the configured trigger AABB and not when they leave (`entered=true` only on enter).
- [ ] Test: `OnlyOnceAreaTriggerScript` only fires once per (player, trigger) pair across reloads — needs character-scoped persistence.
- [ ] Test: `ConditionScript::on_check` is reachable from a `conditions` row with `ConditionType=33, ConditionValue1=<scriptId>`.
- [ ] Test: `InstanceMapScript::get_instance_script` returns a fresh state machine per map instance, not a shared one.
- [ ] Test: `ServerScript::on_packet_receive` sees a copy of an inbound packet that the dispatcher subsequently still parses correctly (mutation of the copy must not affect dispatch).
- [ ] Test: `SystemMgr::get_spline_chain(entry, chainId)` returns the loaded waypoints in `wpId` order.
- [ ] Test: registering a `CommandScript` that returns `["foo bar"]` makes `.foo bar` reachable through the chat command parser.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 6 files / 6578 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h` | `crates/wow-script/` (the framework: `ScriptMgr` equivalent + every `*Script` trait + dispatch tables); content scripts live in `crates/wow-scripts/` (covered by `scripts.md`). \| ❌ not started — `crates/wow-script/src/lib.rs` is **0 bytes**. There is no `ScriptMgr`, no script trait, no dispatch table, no registration macro, no reload manager. **Every** `sScriptMgr->OnX(...)` callsite in C++ has no Rust counterpart yet, which means none of the ~160 script hooks fire. Boss AI, instance scripts, spell scripts, command scripts, item-use scripts, area triggers, gossip, and PlayerScript hooks (login/logout/zone change/level up/etc.) are all silent. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SCRIPTING.DIV.001` | `crates/wow-script` (`exists_empty`, 0 Rust lines) | 6 C++ files / 6578 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTING.DIV.002` | `crates/wow-scripts` (`exists_empty`, 0 Rust lines) | 6 C++ files / 6578 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SCRIPTING.DIV.003` | `crates/wow-script/src/lib.rs` (`exists_empty`, 0 Rust lines) | 6 C++ files / 6578 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |
| `#SCRIPTING.DIV.004` | `crates/wow-scripts/src/lib.rs` (`exists_empty`, 0 Rust lines) | 6 C++ files / 6578 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptReloadMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scripting/ScriptMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **Two-step dispatch is mandatory** in this codebase. Even after a hook trait exists, every callsite in the rest of the workspace needs a literal `script_mgr.on_x(...)` line — the same trap as the packet handlers' `match arm + inventory::submit!` rule (see `CLAUDE.md`). Forgetting either silently does nothing.
- The C++ pattern of `new boss_lord_marrowgar()` inside `AddSC_boss_lord_marrowgar()` performs the registration as a side effect of object construction. The Rust analog is `inventory::submit! { MyAi { } as &dyn CreatureScript }` at module scope; both are *fully static* and run once.
- C++ `ScriptObject` ctor takes `char const*` and stores `std::string` — names are interned per script. Rust will use `&'static str`; do **not** allow runtime-generated names.
- C++ `FOREACH_SCRIPT(T)` holds a `LockModuleReferenceLock` to keep modules pinned during dispatch. If hot-reload is dropped (recommended), the lock is unnecessary.
- `SpellScript` / `AuraScript` are deceptively the deepest part of this system. Each scripted spell can attach effect handlers per `EFFECT_INDEX`, hit-target filters, multiple `On*` hooks (`BeforeCast`, `OnCast`, `AfterCast`, `OnHit`, `OnTakeAura`, `OnDispel`, `OnEffectHitTarget`, `OnEffectLaunch`, …). The ~3,000 spell scripts in `src/server/scripts/Spells/` would be unimplementable without that machinery — plan it as its own crate-scale effort under `wow-spell`.
- `ScriptMgr::OnPlayerChat` has **5 overloads**, but TrinityCore code routinely overrides only one or two. The Rust trait should default-impl all of them to no-op so a script overriding "private chat" alone doesn't have to stub four others.
- `ConditionSourceInfo` carries up to 3 targets (`mConditionTargets[3]`) plus a `Map*` and a "last failed condition" pointer for client error feedback. The Rust `ConditionScript::on_check` signature must preserve that shape.
- **WoLK 3.4.3 specific**: `Conversation`, `Scene`, `WorldState`, `PlayerChoice`, `AreaTriggerEntityScript`, and `BattlegroundMapScript` are post-WoLK additions back-ported into TrinityCore 3.4.x. They exist in this codebase but are barely used by the WoLK script content — implement late if at all.
- `EventScript::OnTrigger` and the GameEvents `events.md` system are **different things despite the names**. `EventScript` handles `event_scripts` table rows triggered by `SPELL_EFFECT_SEND_EVENT` / `GAMEOBJECT_TYPE_GOOBER`; GameEvents (`events.md`) is the holiday/world-event scheduler. Don't merge them.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ScriptObject` (with `_name`) | `trait ScriptObject { fn name(&self) -> &'static str; }` | Drop runtime-allocated names. |
| `class FooScript : public ScriptObject` | `trait FooHook: Send + Sync` | One trait per script type; default-impl every method. |
| `ScriptRegistry<T>::AddScript(this)` (in ctor) | `inventory::submit! { MyImpl as &dyn FooHook }` | Compile-time static registration. Same crate already uses `inventory` for packet handlers. |
| `FOREACH_SCRIPT(T)->OnX(args)` | `for h in inventory::iter::<&dyn FooHook> { h.on_x(args) }` (wrapped behind `ScriptMgr::on_x`) | No module reference lock if hot-reload is dropped. |
| `RegisterCreatureAI(MyAI)` macro | `register_creature_ai!(MyAI)` macro_rules expanding to `inventory::submit!` | Mirror ergonomics. |
| `RegisterSpellScript(spell_foo)` | `register_spell_script!(spell_foo)` | Coupled to whatever shape `wow-spell` settles on for `SpellScript`/`AuraScript`. |
| `void AddSC_xxx()` aggregate function | The `inventory::submit!` calls *replace* this — no `AddScripts()` aggregator needed. | Cross-crate: `wow-scripts` will pull in submodules per zone (`mod ulduar; mod naxxramas; ...`) and each submodule does its own `inventory::submit!`. |
| `ScriptMgr` singleton | `struct ScriptMgr { … }` + `pub static SCRIPT_MGR: OnceLock<ScriptMgr> = …` | Keep it stateful (script-id table) but immutable after init. |
| `ScriptId` (`uint32`) | `pub struct ScriptId(pub u32)` | Newtype. |
| `sScriptMgr->OnX(args)` | `ScriptMgr::get().on_x(args)` or free function `script::on_x(args)` | Pick one and keep callsites uniform. |
| `ModuleReference` / `ScriptReloadMgr` | (none / out of scope) | Drop. If revisited, `libloading` crate. |
| `SystemMgr::GetSplineChain` | `crate::spline_chains::get(entry, chain_id) -> Option<&[SplineChainLink]>` | Plain global `RwLock<HashMap>`. |
| `Trinity::ChatCommands::ChatCommandBuilder` | TBD — coordinate with `chat.md` and the chat command parser. | Cross-cuts with multiple migrations. |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — `crates/wow-script/src/lib.rs` is empty (0 lines).**

```
$ wc -l crates/wow-script/src/lib.rs
0 crates/wow-script/src/lib.rs
```

`crates/wow-script/Cargo.toml` declares deps on `wow-core`, `wow-constants`, `inventory` — the framework dependencies are queued up but no source has been written. No `ScriptObject` trait, no `ScriptMgr`, no per-hook trait, no `inventory::collect!`/`submit!` registration, no name→script-id resolver. `cargo test -p wow-script` runs **0 tests**.

Cross-cutting confirmation: zero callsites of any `sScriptMgr->OnX`-equivalent across `crates/`. Every game-side hookpoint listed in §9 Phase C is also absent — `WorldSession::login_handler`, `MapManager::create_map`, world tick loop, `world_socket` packet observer, `wow-conditions` evaluator. The framework gap is mirrored by an equally-wide game-side gap.

**No silent-default bug per se** — the absence is total, so there's no half-wired hook firing the wrong default. Risk is "feature class entirely missing," not "feature incorrectly true." Adding the framework will require also adding ~40 `script_mgr.on_x(...)` callsites across the workspace (the two-step dispatch trap from `CLAUDE.md`).

**Recommendation:** Tackle in the order from §9 — Phase A (#SCRIPTING.1–4) → minimal hook subset (Phase B for `WorldScript`, `PlayerScript`, `CreatureScript`, `CommandScript` first) → Phase C wiring. Drop `ScriptReloadMgr` (#SCRIPTING.44) as explicit non-goal. `SpellScript`/`AuraScript` (#SCRIPTING.36) are correctly flagged XL — they need their own migration doc and should not be attempted in the first pass.

Coupling: ConditionMgr (#COND.*) blocks `ConditionScript::on_check` (#SCRIPTING.20). Otherwise wow-script is independent of other ❌ modules.
