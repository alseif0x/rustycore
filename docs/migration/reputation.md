# Migration: Reputation

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Reputation/` + `src/server/game/Server/Packets/ReputationPackets.cpp/.h`
> **Rust target crate(s):** `crates/wow-world/` (per-session ReputationMgr), `crates/wow-data/` (FactionEntry/FactionTemplateEntry DB2 readers), `crates/wow-database/` (character_reputation prepared statements), `crates/wow-packet/` (reputation packets)
> **Layer:** L6
> **Status:** ⚠️ stub (enums + opcode + dummy SMSG_INITIALIZE_FACTIONS only; no per-player state, no DB persistence, no spillover)
> **Audited vs C++:** ✅ complete
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-player reputation state with every faction defined in `Faction.db2`. Tracks an integer "standing" per faction, derives a discrete `ReputationRank` (Hated…Exalted) from configurable thresholds, applies "spillover" propagation to parent/sibling factions, supports forced reactions (PvP/quest overrides), Paragon (post-Legion repeat-reward at fixed `LevelThreshold`), Renown (Dragonflight currency-coupled), and FriendshipRep (Pandaria-style custom reaction tables). Sends initial reputation list on login, deltas on change, and emits visible/at-war/inactive flag updates.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Reputation/ReputationMgr.cpp` | 867 | `prefix` |
| `game/Reputation/ReputationMgr.h` | 178 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Reputation/ReputationMgr.h` | 178 | `ReputationMgr` class, `FactionState` struct, `ReputationFlags` bitmask, `RepListID` typedef, `ForcedReactions` map |
| `src/server/game/Reputation/ReputationMgr.cpp` | 867 | All logic: `Initialize`, `SetReputation` (incl. spillover), `SendInitialReputations`, `SendState`, `SendForceReactions`, paragon/renown calc, DB load/save, `ReputationToRank` thresholds |
| `src/server/game/Server/Packets/ReputationPackets.h` / `.cpp` | ~150 | `InitializeFactions`, `SetFactionStanding`, `SetFactionAtWar`, `SetForcedReactions` packet types |
| `src/server/game/Server/Packets/CharacterPackets.h` (`SetFactionVisible`, `SetFactionInactive`) | — | Faction visibility flag updates |
| `src/server/game/DataStores/DB2Stores.cpp` | — | `sFactionStore` (Faction.db2), `sFactionTemplateStore` (FactionTemplate.db2), `GetFriendshipRepReactions`, `GetParagonReputation`, `GetFactionTeamList` |
| `src/server/game/Globals/ObjectMgr.cpp` | — | `GetRepSpilloverTemplate` (DB-defined override of DBC spillover) |
| `src/server/game/Handlers/CharacterHandler.cpp` | — | `CMSG_SET_FACTION_ATWAR`, `CMSG_SET_FACTION_INACTIVE`, `CMSG_REQUEST_FORCED_REACTIONS`, faction-related CMSG |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ReputationMgr` | class (one per `Player`) | Owns `FactionStateList _factions`, `ForcedReactions _forcedReactions`, rank counters (visible/honored/revered/exalted), back-pointer to owner Player |
| `FactionState` | struct | `{ uint32 ID; RepListID ReputationListID; int32 Standing; int32 VisualStandingIncrease; EnumFlag<ReputationFlags> Flags; bool needSend; bool needSave; }` |
| `RepListID` | typedef `uint32` | Index from `FactionEntry::ReputationIndex` (the position in client's reputation pane) |
| `ForcedReactions` | typedef `std::map<uint32, ReputationRank>` | Override map: faction id → forced rank (e.g. `Player::HandleSpellEffect` for `SPELL_EFFECT_DISPEL_FACTION_STANDING`, scripts) |
| `ReputationFlags` | enum class (uint16, bitmask) | `None=0, Visible=0x01, AtWar=0x02, Hidden=0x04, Header=0x08, Peaceful=0x10, Inactive=0x20, ShowPropagated=0x40, HeaderShowsBar=0x80, CapitalCityForRaceChange=0x100, Guild=0x200, GarrisonInvasion=0x400` |
| `ReputationRank` | enum (in `SharedDefines.h`) | `REP_HATED=0…REP_EXALTED=7` (8 ranks, `MAX_REPUTATION_RANK=8`) |
| `RepSpilloverTemplate` | struct (ObjectMgr) | DB-driven override: `faction[5]`, `faction_rate[5]`, `faction_rank[5]` for `MAX_SPILLOVER_FACTIONS=5` |
| `FactionEntry` (DB2) | struct | From `Faction.db2`: `ID`, `ReputationIndex`, `ReputationBase[4]`, `ReputationMax[4]`, `ReputationFlags[4]`, `ParentFactionID`, `ParentFactionMod[2]`, `ParentFactionCap[2]`, `FriendshipRepID`, `RenownCurrencyID`, `ParagonFactionID` |
| `FactionTemplateEntry` (DB2) | struct | `Faction`, `Mask`, `Friend[4]`, `Enemy[4]`, `FactionGroup`, `FriendGroup`, `EnemyGroup` (used for unit reaction calc, NOT player-rep) |
| `ParagonReputationEntry` (DB2) | struct | `LevelThreshold`, `QuestID`, `RewardPackID` (paragon repeat reward) |
| `FriendshipRepReactionEntry` (DB2) | struct | `ReactionThreshold`, `Reaction[locale]` (custom rank table per faction) |
| `CurrencyTypesEntry` (DB2) | struct | Used by Renown (faction's `RenownCurrencyID` → currency level) |

Constants: `Reputation_Cap = 42000`, `Reputation_Bottom = -42000`, `MAX_SPILLOVER_FACTIONS = 5`, `MAX_REPUTATION_RANK = 8`. Thresholds set: `{-42000, -6000, -3000, 0, 3000, 9000, 21000, 42000}`.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ReputationMgr::Initialize()` | Iterate `sFactionStore`, create `FactionState` for every `CanHaveReputation()` faction with default flags from `GetDefaultStateFlags`, base reputation from race/class data index, rank counters | `sFactionStore`, `GetDefaultStateFlags`, `GetBaseRank`, `UpdateRankCounters` |
| `ReputationMgr::LoadFromDB(result)` | Read `character_reputation` rows; merge persisted standings/flags onto initialized states | `sFactionStore.LookupEntry`, `UpdateRankCounters` |
| `ReputationMgr::SaveToDB(trans)` | For each state with `needSave`, run `DEL_CHAR_REPUTATION_BY_FACTION` then `INS_CHAR_REPUTATION_BY_FACTION` | `CharacterDatabase` |
| `ReputationMgr::SetReputation(factionEntry, standing, incremental, spillOverOnly, noSpillover)` | Master modifier: applies DB-spillover-template override if present; otherwise computes spillover via `ParentFactionID`/`GetFactionTeamList` with `ParentFactionMod` weights; redirects to paragon parent when applicable; finally calls `SetOneFactionReputation` and `SendState` | `sObjectMgr->GetRepSpilloverTemplate`, `sDB2Manager.GetFactionTeamList`, `SetOneFactionReputation`, `SendState`, `sScriptMgr->OnPlayerReputationChange` |
| `ReputationMgr::SetOneFactionReputation(factionEntry, standing, incremental)` | Computes new clamped standing (caps via `GetMin/MaxReputation`), applies `RATE_REPUTATION_GAIN` config, updates rank counters, marks `needSend`/`needSave`, handles renown level-up (modifies currency), sets `_sendFactionIncreased` for visual; auto-applies `AtWar` when dropping to `REP_HOSTILE` | `GetMaxReputation`, `GetMinReputation`, `ReputationToRank`, `UpdateRankCounters`, `Player::ReputationChanged`, `Player::ModifyCurrency`, `SetVisible`, `SetAtWar` |
| `ReputationMgr::ReputationToRank(factionEntry, standing)` (static) | Looks up `FriendshipRepReactionSet` if faction has `FriendshipRepID`, else uses `ReputationRankThresholds`; returns `REP_HATED…REP_EXALTED` | `sDB2Manager.GetFriendshipRepReactions` |
| `ReputationMgr::GetReputation(faction)` | `BaseReputation + Standing` | — |
| `ReputationMgr::GetBaseReputation(faction)` | Pulls from `factionEntry->ReputationBase[dataIndex]` where `dataIndex = GetFactionDataIndexForRaceAndClass` | — |
| `ReputationMgr::GetMaxReputation(faction)` | Paragon: cap rolls forward by `LevelThreshold` per claimed reward; Renown: `RenownMaxLevel * RenownLevelThreshold`; FriendshipRep: top reaction threshold; default: `ReputationMax[dataIndex]` | `sDB2Manager.GetParagonReputation`, `Player::GetQuestStatus`, `GetRenown*`, `GetFriendshipRepReactions` |
| `ReputationMgr::GetRank(faction)` | Convenience: `ReputationToRank(faction, GetReputation(faction))` | — |
| `ReputationMgr::GetForcedRankIfAny(factionId/factionTemplate)` | Lookup in `_forcedReactions` map | — |
| `ReputationMgr::IsAtWar(faction)` | Reads `Flags.HasFlag(AtWar)` from state | — |
| `ReputationMgr::SetAtWar(repListID, on)` / `SetInactive(...)` | Toggles flag on a `FactionState`, marks `needSend`+`needSave`. `SetAtWar` blocked for `Peaceful`-flagged factions and `REP_HOSTILE` lock | — |
| `ReputationMgr::SetVisible(state)` | Sets `Visible` flag, increments `_visibleFactionCount`, sends `SetFactionVisible` | `SendVisible` |
| `ReputationMgr::ApplyForceReaction(factionId, rank, apply)` | Insert/erase from `_forcedReactions` | — |
| `ReputationMgr::SendInitialReputations()` | Build `WorldPackets::Reputation::InitializeFactions` with full faction list (flags + standings) sent on login | `Player::SendDirectMessage` |
| `ReputationMgr::SendState(faction)` | `SetFactionStanding` packet with deltas of all `needSend` states + visual flag | `Player::SendDirectMessage` |
| `ReputationMgr::SendForceReactions()` | `SetForcedReactions` packet with full `_forcedReactions` map | — |
| `ReputationMgr::IsParagonReputation(faction)` | Check if `ParagonReputation` row exists for faction id | `sDB2Manager.GetParagonReputation` |
| `ReputationMgr::IsRenownReputation(faction)` | `RenownCurrencyID > 0` | — |
| `ReputationMgr::GetParagonLevel(faction)` | `currentRep / paragonReputation->LevelThreshold` | — |
| `ReputationMgr::GetRenownLevel/Threshold/MaxLevel` | Look up linked currency; level = currency quantity, max = currency cap | `sCurrencyTypesStore`, `Player::GetCurrencyQuantity` |
| `ReputationMgr::CanGainParagonReputationForFaction(faction)` | True when faction has `ParagonFactionID` and parent is at-Exalted (so excess flows to paragon meter) | — |
| `ReputationMgr::UpdateRankCounters(oldRank, newRank)` | Maintains `_visibleFactionCount`, `_honoredFactionCount`, `_reveredFactionCount`, `_exaltedFactionCount` (used by achievements / "Ambassador" titles) | — |

Player-facing helpers (in Player.cpp/h, not ReputationMgr but in same module):
- `Player::GetReputation(factionId)` — delegates to mgr.
- `Player::GetReputationRank(factionId)` — delegates.
- `Player::ReputationChanged(faction, delta)` — fires criteria/scripts on change.
- `Player::RewardReputation(victim)` — kill rewards via `creature_onkill_reputation` table.
- `Player::RewardReputation(quest)` — quest rewards via `Quest::RewardFactionId/Value/Override/CapIn` arrays.

---

## 5. Module dependencies

**Depends on:**
- `Entities/Player` — owner pointer, `GetCurrencyQuantity`, `ModifyCurrency`, `GetQuestStatus`, `SendDirectMessage`, `GetRace`, `GetClass`.
- `DataStores` — `sFactionStore` (Faction.db2), `sFactionTemplateStore`, `sCurrencyTypesStore`, `DB2Manager::GetFriendshipRepReactions`, `GetParagonReputation`, `GetFactionTeamList`, `sParagonReputationStore`.
- `Globals/ObjectMgr` — `GetRepSpilloverTemplate` (DB override loaded from `reputation_spillover_template`).
- `Scripting/ScriptMgr` — `OnPlayerReputationChange` hook.
- `World/World` — `getRate(RATE_REPUTATION_GAIN)`.
- `Database/CharacterDatabase` — load/save prepared statements.
- `Server/Packets/ReputationPackets` — packet types.

**Depended on by:**
- `Quests` — `Quest::RewardFactionId[5]/Value[5]/Override[5]/CapIn[5]` reward chain.
- `Combat/Loot` — `Player::RewardReputation(creature)` and `creature_onkill_reputation` table.
- `Spells` — `SPELL_EFFECT_REPUTATION` and aura `SPELL_AURA_MOD_FACTION_REPUTATION_GAIN`.
- `AI/Unit` — `Unit::GetReactionTo(target)` checks player's faction reaction (forced reactions + standing rank).
- `AuctionHouse` — faction-restricted access checks.
- `Mails` — cross-faction send blocked unless account-bound + RBAC.
- `BattlePets/Battlegrounds/Achievements` — `Ambassador` and similar criteria read rank counters.
- `Vendor` — `EQUIP_ERR_CANT_EQUIP_REPUTATION` requires reputation rank check.

---

## 6. SQL / DB queries (if any)

DB: `character` and `world`.

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER_REPUTATION` | `SELECT faction, standing, flags FROM character_reputation WHERE guid = ?` | character |
| `CHAR_INS_CHAR_REPUTATION_BY_FACTION` | `INSERT INTO character_reputation (guid, faction, standing, flags) VALUES (?, ?, ?, ?)` | character |
| `CHAR_DEL_CHAR_REPUTATION_BY_FACTION` | `DELETE FROM character_reputation WHERE guid = ? AND faction = ?` | character |
| `CHAR_DEL_CHAR_REPUTATION` | Cascade on character delete | character |
| `CHAR_SEL_CHAR_REP_BY_FACTION` | Diagnostic / GM commands | character |
| `CHAR_DEL_CHAR_REP_BY_FACTION` | GM command | character |
| `CHAR_UPD_CHAR_REP_FACTION_CHANGE` | Faction-change service: re-target reputation rows | character |
| (world) `reputation_spillover_template` | DB override of DBC spillover topology | world |
| (world) `creature_onkill_reputation` | Kill-rewarded reputation per creature template | world |
| (world) `reputation_reward_rate` | Per-faction multipliers (kill, quest, spell, creature) | world |

DBC/DB2 stores read by reputation:

| Store | What it loads | Read by |
|---|---|---|
| `Faction.db2` (`sFactionStore`) | All factions, `ReputationIndex`, `ReputationBase/Max/Flags[4]` (race-class data slots), `ParentFactionID`, `ParentFactionMod[2]`, `ParentFactionCap[2]`, `FriendshipRepID`, `RenownCurrencyID`, `ParagonFactionID` | `ReputationMgr::Initialize`, `SetReputation`, `GetMaxReputation`, etc. |
| `FactionTemplate.db2` (`sFactionTemplateStore`) | Template id → faction id + Mask + Friend/Enemy lists | `Unit::GetReactionTo`, `GetForcedRankIfAny(template)` |
| `ParagonReputation.db2` | `LevelThreshold`, `QuestID`, `RewardPackID` per paragon faction | `IsParagonReputation`, `GetMaxReputation`, `GetParagonLevel` |
| `FriendshipRepReaction.db2` | Custom reaction tables with thresholds + localized labels | `ReputationToRank`, `GetReputationRankName` |
| `CurrencyTypes.db2` | Renown currency definitions (max, name) | `GetRenownMaxLevel`, `GetRenownLevel` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_INITIALIZE_FACTIONS` | server → client | `ReputationMgr::SendInitialReputations` (login) |
| `SMSG_SET_FACTION_STANDING` | server → client | `ReputationMgr::SendState` |
| `SMSG_SET_FACTION_VISIBLE` | server → client | `ReputationMgr::SendVisible` |
| `SMSG_SET_FACTION_AT_WAR` | server → client | `ReputationMgr::SetAtWar` (state pushed in next `SendState`) |
| `SMSG_SET_FORCED_REACTIONS` | server → client | `ReputationMgr::SendForceReactions` |
| `CMSG_SET_FACTION_ATWAR` | client → server | `WorldSession::HandleSetFactionAtWar` → `ReputationMgr::SetAtWar` |
| `CMSG_SET_FACTION_INACTIVE` | client → server | `WorldSession::HandleSetFactionInactiveOpcode` → `ReputationMgr::SetInactive` |
| `CMSG_REQUEST_FORCED_REACTIONS` | client → server | `WorldSession::HandleRequestForcedReactionsOpcode` → `SendForceReactions` |
| `CMSG_SET_WATCHED_FACTION` | client → server | Updates `Player::SetWatchedFactionIndex` (the faction shown on the XP bar) |
| `CMSG_SET_FACTION_NOT_AT_WAR` | client → server | Variant on some builds; same handler |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *(none specific to reputation)*

**What's implemented:**
- Nothing. No `ReputationMgr`, no faction state in session, no `character_reputation` load/save, no opcodes wired.

**What's missing vs C++:**
- Entire `ReputationMgr` per-player state machine.
- `Faction.db2`, `FactionTemplate.db2`, `ParagonReputation.db2`, `FriendshipRepReaction.db2` DB2 readers.
- `SMSG_INITIALIZE_FACTIONS` on login (client will not display faction pane correctly without it).
- Standing modifications: `SetReputation`, spillover propagation, paragon, renown.
- `character_reputation` table prepared statements + load/save.
- Forced reactions map + `SMSG_SET_FORCED_REACTIONS`.
- At-war / inactive flag toggling and corresponding CMSG handlers.
- Rank counters (visible/honored/revered/exalted) for achievements.
- Reputation rate multipliers (`reputation_reward_rate` table, `RATE_REPUTATION_GAIN` config).
- Quest reputation reward integration.
- Kill reputation reward (`creature_onkill_reputation` table) integration.
- `Player::GetReputation*` / `GetReactionTo` faction-aware helpers.
- Spell effect `SPELL_EFFECT_REPUTATION` and aura `SPELL_AURA_MOD_FACTION_REPUTATION_GAIN`.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Without `SMSG_INITIALIZE_FACTIONS` on login, the WoLK 3.4.3 client falls back to all-zero standings AND treats all factions as "neutral" — this means existing characters logging in cannot see their faction status, but it does not crash the client.
- Many TC features in this codebase (Paragon/Renown/FriendshipRep) are post-WoLK; for 3.4.3 only base-rep + spillover + AtWar/Inactive flags are needed at parity. Sub-tasks below tag those as "WoLK-3.4.3 in scope" vs "modern features defer".

**Tests existing:**
- 0 tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#REPUTATION.WBS.001** Partir y cerrar la migracion auditada de `game/Reputation/ReputationMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 867 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#REPUTATION.WBS.002** Cerrar la migracion auditada de `game/Reputation/ReputationMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.
Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [x] **#REP.1** Add `Faction.db2` reader to `crates/wow-data/src/faction.rs` (FactionRecord with race-class data slots — 4-element arrays of base/max/flags) (M) — closed by `#NEXT.R8.ENTITIES.667`; existing `FactionStore` now hydrates the C++ race/class reputation slots used by `ReputationMgr::Initialize`.
- [x] **#REP.2** Add `FactionTemplate.db2` reader (template id → faction id, Friend/Enemy masks for unit-reaction logic) (M) — closed by `#NEXT.R8.ENTITIES.682`; `FactionTemplateStore` loads the C++ fields and `FactionTemplateEntry` now owns the C++ friendship/hostility/neutral/flag helpers used by unit-reaction seams.
- [x] **#REP.3** Define `ReputationFlags` (bitflags), `ReputationRank` enum (`REP_HATED`..`REP_EXALTED`), thresholds set `{-42000, -6000, -3000, 0, 3000, 9000, 21000, 42000}` in `crates/wow-data/src/reputation.rs` (L) — closed by `#NEXT.R8.ENTITIES.665`; `wow_data::reputation` owns `ReputationFlagsLikeCpp`, `ReputationRankLikeCpp`, C++ cap/bottom constants, and the stock threshold rank helper. FriendshipRep override remains under `#REP.6`.
- [x] **#REP.4** Define `FactionState` struct + `ReputationMgr` per-session state in `crates/wow-world/src/reputation/mgr.rs` with `BTreeMap<RepListId, FactionState>` and `_forcedReactions: HashMap<u32, ReputationRank>` plus rank counters (M) — closed by `#NEXT.R8.ENTITIES.666`; state shape is installed in `WorldSession`, but `Initialize`/DB/payload fanout remain pending.
- [x] **#REP.5** Implement `ReputationMgr::initialize` — iterate Faction.db2 store, create state for each `can_have_reputation()` faction, apply default flags from race-class data slot (M) — closed by `#NEXT.R8.ENTITIES.667`; `WorldSession` initializes/reinitializes the state when faction/paragon stores and loaded player identity are available.
- [x] **#REP.6** Implement `reputation_to_rank(faction, standing)` (with FriendshipRep override path) (L) — closed by `#NEXT.R8.ENTITIES.668`; FriendshipRep reactions are loaded, grouped by `FriendshipRepID`, ordered by threshold, and used before falling back to the stock rank thresholds.
- [x] **#REP.7** Add prepared statements `SEL_CHARACTER_REPUTATION`, `INS_CHAR_REPUTATION_BY_FACTION`, `DEL_CHAR_REPUTATION_BY_FACTION`, `DEL_CHAR_REPUTATION` in `crates/wow-database/src/statements/character.rs` (L) — closed by `#NEXT.R8.ENTITIES.669`; SQL matches C++ `CharacterDatabase.cpp` exactly.
- [x] **#REP.8** Implement `ReputationMgr::load_from_db` and `save_to_db` — merge persisted standings/flags onto initialized states; only persist `needSave` rows (M) — closed by `#NEXT.R8.ENTITIES.669`/`#NEXT.R8.ENTITIES.681`; implemented as C++-parity row merge and delete+insert save statement plan, and runtime login/save now executes the `character_reputation` query/transaction hook like the C++ character login/save flow.
- [~] **#REP.9** Implement `ReputationMgr::set_one_faction_reputation` (rate mult, clamp, rank-counter update, AtWar auto-on at hostile, `_sendFactionIncreased` flag) (H) — `#NEXT.R8.ENTITIES.670` represents the standard non-renown/non-paragon mutation core; `#NEXT.R8.ENTITIES.671` adds the C++ Paragon max-cap calculation and reward-quest trigger evidence; `#NEXT.R8.ENTITIES.672` adds the C++ Renown max/threshold/remainder/currency-delta core. Actual paragon quest-template/AddQuest execution, actual renown `ModifyCurrency`, criteria updates, runtime `SendState`, and quest/kill mutation hookup remain open.
- [~] **#REP.10** Implement `ReputationMgr::set_reputation` master entry: DB-spillover-template lookup → fallback to DBC `ParentFactionID`/`GetFactionTeamList` chain → paragon redirect → final `SetOneFactionReputation` + `SendState` (XL — split spillover into own task) — `#NEXT.R8.ENTITIES.673` represents the DB-template spillover override path, primary mutation, `noSpillover`, Paragon redirect, script-event evidence, and `SendState` target evidence; `#NEXT.R8.ENTITIES.675` adds the represented C++ DBC fallback branch. Actual packet fanout and runtime mutation hooks remain open.
- [~] **#REP.11** Implement spillover: parse `reputation_spillover_template` worldserver table OR fall back to `Faction.db2` `ParentFactionID`/`ParentFactionMod[2]`/`ParentFactionCap[2]` propagation. Cap propagation by `ParentFactionCap[0]` rank check, weight by `ParentFactionMod[0]` (H) — `#NEXT.R8.ENTITIES.673` represents DB-template spillover semantics in the manager; `#NEXT.R8.ENTITIES.674` adds C++ SQL loading/validation/startup/session wiring for `reputation_spillover_template`; `#NEXT.R8.ENTITIES.675` represents the DBC fallback sub-faction/sister/parent `HeaderShowsBar` propagation. Actual runtime send/save/quest-kill hookup remains open.
- [x] **#REP.12** Implement packet types: `SMSG_INITIALIZE_FACTIONS`, `SMSG_SET_FACTION_STANDING`, `SMSG_SET_FACTION_VISIBLE`, `SMSG_SET_FACTION_AT_WAR`, `SMSG_SET_FORCED_REACTIONS` in `crates/wow-packet/src/packets/reputation.rs` (M) — closed by `#NEXT.R8.ENTITIES.676`; C++ byte layout is covered for `InitializeFactions`, `SetFactionStanding`, `SetForcedReactions`, visible/not-visible, and the faction CMSG readers. `SMSG_SET_FACTION_AT_WAR` is represented as an opcode/payload carrier because C++ registers the opcode but does not define a writer; normal C++ state fanout goes through `SMSG_SET_FACTION_STANDING`.
- [x] **#REP.13** Implement `send_initial_reputations` and call it during login flow after world enter (L) — closed by `#NEXT.R8.ENTITIES.677`/`#NEXT.R8.ENTITIES.681`; login now loads persisted `character_reputation` rows into the initialized manager before serializing current `ReputationMgr` flags/standings by reputation-list index and clearing `needSend` like C++.
- [x] **#REP.14** Implement `send_state` (delta) — collect `needSend` factions, build `SetFactionStanding`, clear flags, push (M) — closed by `#NEXT.R8.ENTITIES.678`; `ReputationMgrLikeCpp` now builds the C++ `SetFactionStanding` payload for a primary faction plus all pending `needSend` states, uses `VisualStandingIncrease` when nonzero, clears sent flags and resets `_sendFactionIncreased`. Runtime call sites from quest/kill mutation remain open.
- [x] **#REP.15** Implement `send_force_reactions` and `SMSG_SET_FORCED_REACTIONS` packet (L) — closed by `#NEXT.R8.ENTITIES.679`; `ForcedReactionsLikeCpp` now preserves C++ `std::map` faction-id order and builds the C++ `SetForcedReactions` payload.
- [x] **#REP.16** Implement CMSG handlers: `handle_set_faction_at_war`, `handle_set_faction_inactive`, `handle_request_forced_reactions`, `handle_set_watched_faction` (M) — closed by `#NEXT.R8.ENTITIES.680`; handlers now parse the C++ client packets, mutate represented reputation state for at-war/inactive, send forced reactions on request, and record `WatchedFactionIndex` as represented active-player state. Full watched-faction update-field fanout/persistence remains under `#REP.26`.
- [~] **#REP.17** Implement `Player::reward_reputation_from_kill` reading `creature_onkill_reputation` worldserver table (depends on creature template module) (M) — advanced by `#NEXT.R8.ENTITIES.691`/`#NEXT.R8.ENTITIES.692`/`#NEXT.R8.ENTITIES.693`/`#NEXT.R8.ENTITIES.694`/`#NEXT.R8.ENTITIES.695`/`#NEXT.R8.ENTITIES.696`/`#NEXT.R8.ENTITIES.697`; Rust now loads/validates the C++ `creature_onkill_reputation` table into a session-wired store, skips invalid rows like `ObjectMgr::LoadReputationOnKill`, and the represented creature-kill flow mutates `ReputationMgrLikeCpp` after XP and before kill credit, including reputation-disabled victim guard, kill-source low-level/rate calculation with exact C++ gray-level formula and represented FormulaScript gray-level override, team-dependent branch selection, cap gate, spillover-cap handoff, `SMSG_SET_FACTION_STANDING` fanout, represented championing faction override for Wrath max-level non-raid LFG dungeons, represented current-session party kill-rate math with dungeon full-rate override, represented generic/faction-specific reputation aura modifiers, and represented Recruit-A-Friend reputation bonus for grouped in-distance linked accounts. Full equipment/aura/tabard derivation of `GetChampioningFaction`, corpse-position RAF distance, raid-group rate state, and full live group fanout remain open.
- [~] **#REP.18** Implement `Player::reward_reputation_from_quest` reading `RewardFactionId[5]`, `RewardFactionValue[5]`, `RewardFactionOverride[5]`, `RewardFactionCapIn[5]` (depends on Quest module) (M) — advanced by `#NEXT.R8.ENTITIES.689`/`#NEXT.R8.ENTITIES.690`/`#NEXT.R8.ENTITIES.695`/`#NEXT.R8.ENTITIES.696`/`#NEXT.R8.ENTITIES.697`/`#NEXT.R8.ENTITIES.698`/`#NEXT.R8.ENTITIES.699`/`#NEXT.R8.ENTITIES.700`/`#NEXT.R8.ENTITIES.701`/`#NEXT.R8.ENTITIES.702`/`#NEXT.R8.ENTITIES.703`/`#NEXT.R8.ENTITIES.704`/`#NEXT.R8.ENTITIES.705`/`#NEXT.R8.ENTITIES.706`/`#NEXT.R8.ENTITIES.707`; quest reward reputation now belongs to a shared represented `RewardQuest` helper instead of only `CMSG_QUEST_GIVER_CHOOSE_REWARD`. Choose-reward, request-reward completion-before-offer, accepted/confirmed no-objective tracking-event auto-reward, source-item objective tracking-event auto-reward, represented monster objective tracking-event auto-reward, represented GameObject kill-credit objective tracking-event auto-reward, represented player-kill objective tracking-event auto-reward, represented talk-to-creature objective tracking-event auto-reward, represented criteria-tree objective tracking-event auto-reward, represented money objective tracking-event auto-reward, represented reputation objective tracking-event auto-reward, and represented currency objective tracking-event auto-reward paths now reuse that helper, which calculates quest reward reputation from C++ quest arrays/QuestFactionReward/generic aura/low-level/reward-rate/RAF gates using the exact C++ gray-level formula plus represented FormulaScript gray-level override, mutates the represented `ReputationMgr::set_reputation_like_cpp` path with C++ `ModifyReputation(..., noSpillover)`, and emits the represented `SMSG_SET_FACTION_STANDING` delta after the C++ reward packets. Scenario/LFG/GM callers remain open; full `QuestObjectiveCriteriaMgr`, generic `ModifyMoney`/`ModifyCurrency`/reputation runtime wiring, script AI/spell runtime wiring for TALKTO, live PvP/Honor/KillRewarder caller hook, and group cross-session GameObject kill-credit mutation are still open.
- [~] **#REP.19** Implement `reputation_reward_rate` worldserver table reader (per-faction kill/quest/spell/creature multipliers) and apply in `set_reputation` (M) — `#NEXT.R8.ENTITIES.663` loads/validates the table and applies quest-source rates in represented quest reward reputation; full `set_reputation`/kill/spell runtime application remains open.
- [x] **#REP.20** Implement `RATE_REPUTATION_GAIN` global config in `crates/wow-shared` and pull into `set_one_faction_reputation` (L) — closed by `#NEXT.R8.ENTITIES.688`; `Rate.Reputation.Gain` is loaded into `ReputationRatesLikeCpp`, carried into sessions, and `set_reputation_like_cpp`/`set_one_faction_reputation_like_cpp` apply it to incremental reputation gains with the C++ `floor(value * rate + 0.5)` rule. Quest/kill reputation mutation hookup remains under `#REP.17`/`#REP.18`.
- [x] **#REP.21** Implement rank counters (visible/honored/revered/exalted) with `update_rank_counters` and expose to achievement system (L) — closed by `#NEXT.R8.ENTITIES.687`; `ReputationMgrLikeCpp` maintains C++ rank counters and exposes represented criteria progress for `ReputationGained`, total exalted/revered/honored factions, and total factions encountered like `CriteriaHandler.cpp`. Full achievement persistence/fanout remains outside this reputation slice.
- [ ] **#REP.22** Defer Paragon support (`ParagonReputation.db2`, repeat-reward quest gating) — flag as `n/a (post-WoLK)` for 3.4.3 (L doc-only)
- [ ] **#REP.23** Defer Renown support (Dragonflight) — flag as `n/a (post-WoLK)` (L doc-only)
- [ ] **#REP.24** Defer FriendshipRep tables (Pandaria) — present in DB2 but unused in 3.4.3 client; reader optional (L doc-only)
- [~] **#REP.25** Implement `Unit::get_reaction_to(target)` consulting forced reactions + faction template Friend/Enemy + reputation rank (depends on Combat module) (M) — `#NEXT.R8.ENTITIES.683` represents the static `WorldObject::GetFactionReactionTo(factionTemplateEntry, target)` branch for target-player forced rank, contested guard, target-player reputation/AtWar clamp, and `FactionTemplateEntry` fallback when the target player owner is the current session. `#NEXT.R8.ENTITIES.684` represents the current-session `selfPlayerOwner` wrapper branches for self/summoner/owner, forced rank, player-controlled same-owner/duel/raid/FFA, and self reputation-vs-target faction. `#NEXT.R8.ENTITIES.685` makes the target-owner-only forced-reaction branch explicit without falsely reading the current session as another player's `ReputationMgr`. Full non-current-player `ReputationMgr` lookup, live object graph ownership, and live object fanout remain open.
- [x] **#REP.26** Implement `Player::set_watched_faction_index` mirror in player update fields (L) — closed by `#NEXT.R8.ENTITIES.686`; Rust `Player` now owns `ActivePlayerData::WatchedFactionIndex`, marks bit 92 on mutation, bridges the value into active-player update packets, and session snapshots apply the represented watched-faction index like C++ `Player::SetWatchedFactionIndex`.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#REPUTATION.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 1045 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#REPUTATION.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 1045 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#REPUTATION.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 1045 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#REPUTATION.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 1045 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-packet`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: fresh player → all factions initialized with race/class default base + default flags from Faction.db2 data slot.
- [ ] Test: `reputation_to_rank(faction, standing)` for stock faction at standings -42000/-5999/-2999/0/2999/8999/20999/41999/42000 returns Hated/Hostile/Unfriendly/Neutral/Friendly/Honored/Revered/Exalted/Exalted boundaries correctly.
- [x] Test: `set_reputation(faction, +500, incremental=true)` with `RATE_REPUTATION_GAIN=2.0` adds 1000 effective standing, rounded via `floor(x*rate + 0.5)` — covered by `#NEXT.R8.ENTITIES.688` focused `set_reputation_like_cpp_applies_reputation_gain_rate_to_primary_incremental_gain`.
- [ ] Test: clamp at `Reputation_Cap` (42000) and `Reputation_Bottom` (-42000).
- [ ] Test: drop into `REP_HOSTILE` auto-sets `AtWar` flag; not removable while hostile.
- [ ] Test: spillover via DB template overrides DBC: gain on faction A propagates to faction B with rate 0.5 only if current rank ≤ stored cap.
- [ ] Test: spillover via DBC: parent faction has `HeaderShowsBar` → spill goes to parent; otherwise to sibling list.
- [ ] Test: rank counters increment correctly when crossing thresholds (visible/honored/revered/exalted).
- [ ] Test: load/save round-trip — rep set in-memory, save, drop, load → identical state.
- [ ] Test: `SMSG_INITIALIZE_FACTIONS` on login contains all `CanHaveReputation()` factions with correct flags + standings.
- [ ] Test: `SMSG_SET_FACTION_STANDING` after `set_reputation` contains only the touched faction (and any spillover targets), with `ShowVisual=true` if rank increased.
- [ ] Test: `SetFactionAtWar` rejected for `Peaceful`-flagged factions.
- [ ] Test: `SetInactive` flips `Inactive` flag, marks `needSave`.
- [ ] Test: forced reaction map: applied → `SMSG_SET_FORCED_REACTIONS` lists `(factionId, rank)`; removed → not listed.
- [~] Test: kill creature with `creature_onkill_reputation` entry → both team1/team2 rep awarded with rate config — table SQL/load validation is covered by `#NEXT.R8.ENTITIES.691`; `#NEXT.R8.ENTITIES.692` covers represented runtime mutation, `SMSG_SET_FACTION_STANDING`, and team-dependent Horde branch selection; `#NEXT.R8.ENTITIES.693` covers represented championing override in Wrath max-level non-raid LFG dungeons; `#NEXT.R8.ENTITIES.694` covers represented current-session party kill-rate math and dungeon full-rate override; `#NEXT.R8.ENTITIES.695` covers represented generic and faction-specific aura modifiers; `#NEXT.R8.ENTITIES.696` covers represented RAF reputation bonus; `#NEXT.R8.ENTITIES.697` covers exact C++ gray-level formula plus represented FormulaScript gray-level override. Raid-group state/fanout and corpse-position RAF distance remain open.
- [~] Test: complete quest with `RewardFactionId/Value` → rep awarded; `RewardFactionOverride` overrides rate; `RewardFactionCapIn` blocks once at given rank — choose-reward path covered by `#NEXT.R8.ENTITIES.689` plus existing override/cap/rate tests, `#NEXT.R8.ENTITIES.690` verifies the represented `SMSG_SET_FACTION_STANDING` delta, `#NEXT.R8.ENTITIES.695` covers represented generic reputation aura and `noQuestBonus` gating, `#NEXT.R8.ENTITIES.696` covers represented RAF reputation bonus, `#NEXT.R8.ENTITIES.697` covers represented gray-level script adjustment, `#NEXT.R8.ENTITIES.698` covers shared `RewardQuest` reuse plus request-reward completion-before-offer and accepted/confirmed tracking-event auto-reward, `#NEXT.R8.ENTITIES.699` covers source-item objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.700` covers monster objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.701` covers GameObject kill-credit objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.702` covers player-kill objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.703` covers talk-to-creature objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.704` covers criteria-tree objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.705` covers money objective tracking-event auto-reward, `#NEXT.R8.ENTITIES.706` covers MIN/MAX/INCREASE reputation objective tracking-event auto-reward, and `#NEXT.R8.ENTITIES.707` covers CURRENCY/HAVE_CURRENCY/OBTAIN_CURRENCY objective tracking-event auto-reward. Scenario/LFG/GM callers remain open; full `QuestObjectiveCriteriaMgr`, generic `ModifyMoney`/`ModifyCurrency`/reputation runtime wiring, script AI/spell runtime wiring for TALKTO, live PvP/Honor/KillRewarder caller hook, and group cross-session GameObject kill-credit mutation are still open.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 1045 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h` | `crates/wow-world/` (per-session ReputationMgr), `crates/wow-data/` (FactionEntry/FactionTemplateEntry DB2 readers), `crates/wow-database/` (character_reputation prepared statements), `crates/wow-packet/` (reputation packets) \| ⚠️ stub (enums + opcode + dummy SMSG_INITIALIZE_FACTIONS only; no per-player state, no DB persistence, no spillover) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#REPUTATION.DIV.001` | _none generated_ | 2 C++ files / 1045 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Reputation/ReputationMgr.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **Race/class data index** (`GetFactionDataIndexForRaceAndClass`): a faction has 4 alt rep tracks indexed by `Faction.db2` columns. Selection logic: iterate the 4 slots, check if `Race & raceMask` and `Class & classMask` match — first match wins. Returns -1 → faction not applicable to this player → `GetReputation()` returns 0.
- **WoLK 3.4.3 simplification**: only `MAX_REPUTATION_RANK = 8` ranks (Hated…Exalted) and the standard 8 thresholds matter. Paragon/Renown/FriendshipRep types exist in DB2 but client UI does not render them.
- **`Reputation_Bottom = -42000`**: the lowest threshold is special — at exactly `-42000` rank is `REP_HATED` (rank 0). Threshold set order matters: iteration goes from `-42000` upward; rank index increments each crossed.
- **Spillover infinite-recursion guard**: spillover from A→B should NOT trigger B's spillover. C++ does this implicitly because `SetReputation` recurses via `SetOneFactionReputation` (no spillover), not via top-level `SetReputation`. Match precisely.
- **`ParentFactionMod[1]`**: when faction has parent and `ParentFactionMod[1] != 0`, propagation flows DOWN (to parent). When zero, propagation flows SIBLING via parent's team list.
- **`ParentFactionCap[0]`**: spillover beneficiary is skipped if its current rank exceeds this cap — used to stop "auto-friendly" creep into elite factions.
- **DB-template precedence**: if `reputation_spillover_template` row exists for source faction, it COMPLETELY OVERRIDES the DBC parent/team logic. Migration order: load DB → fall back to DBC.
- **`_sendFactionIncreased`**: visual sparkle effect, set when rank goes UP only. Reset after each `SendState`. Persist as transient (not saved).
- **`AtWar` lockout**: once at `REP_HOSTILE` or below, `AtWar` is forcibly true and CMSG_SET_FACTION_ATWAR cannot turn it off. Uppercase enforcement in `SetAtWar(state, false)` short-circuits.
- **`Inactive` flag** is purely client UI: hides faction from primary list. Does not affect gain/loss.
- **`UpdateRankCounters` skips FriendshipRep factions** — `if (!factionEntry->FriendshipRepID)` — because Pandaria-style ranks don't map to the canonical 8.
- **Renown writes currency**: when renown level changes, `Player::ModifyCurrency(currencyId, delta, RenownRepGain, Cheat)` is called. The `Cheat` destroy reason is intentional in TC for command-driven decreases.
- **`character_reputation.flags` column** stores the full `ReputationFlags` bitmask (Visible|AtWar|Hidden|...). Persist as `INT UNSIGNED`.
- **Login order**: `LoadFromDB` must run AFTER `Initialize()` because `Initialize` populates the map keys; `LoadFromDB` only updates standing/flags on existing keys (rows referencing unknown factions are skipped with a warning).
- **`ReputationListID == ReputationIndex`**: server stores `RepListID` (the DBC index) but the wire packet field is also called this — same value, no translation.
- **Performance**: `Initialize` iterates the entire faction store (~3000 entries in modern DB2, ~1500 in WoLK). Per-login cost is acceptable but never rebuild mid-session except on faction-change service (where `_factions.clear()` then `Initialize` is called).
- **Achievements depending on counters**: "Ambassador" (40 honored), "The Diplomat" (Timbermaw etc.), "The Argent Champion" — driven by `_visibleFactionCount`/`_exaltedFactionCount` deltas. Make sure `update_rank_counters` fires criteria.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ReputationMgr` | `struct ReputationMgr` in `crates/wow-world/src/reputation/mgr.rs`, owned by session | One per logged-in player; reset on faction-change service |
| `FactionStateList = std::map<RepListID, FactionState>` | `BTreeMap<RepListId, FactionState>` | BTree for stable iteration order (matches C++ map) |
| `ForcedReactions = std::map<uint32, ReputationRank>` | `HashMap<u32, ReputationRank>` | small map, hash fine |
| `EnumFlag<ReputationFlags>` | `bitflags!` `ReputationFlags(u16)` | preserve hex values |
| `static const ReputationRankThresholds` set | `const REPUTATION_RANK_THRESHOLDS: [i32; 8]` | array, sorted |
| `bool SetReputation(faction, standing, incr, spillOnly, noSpill)` | `fn set_reputation(&mut self, faction: &FactionRecord, standing: i32, opts: SetReputationOpts) -> bool` | `SetReputationOpts { incremental, spillover_only, no_spillover }` builder |
| `void SendInitialReputations()` | `fn send_initial_reputations(&self, session: &mut Session)` | session passed explicitly (no back-pointer) |
| `RepSpilloverTemplate` (ObjectMgr) | `struct RepSpilloverTemplate { faction: [u32; 5], rate: [f32; 5], rank: [u8; 5] }` | loaded from `reputation_spillover_template` worldserver table |
| `sFactionStore.LookupEntry(id)` | `faction_store.get(id) -> Option<&FactionRecord>` | DB2 reader |
| `sDB2Manager.GetFactionTeamList(parentId)` | `db2_mgr.faction_team_list(parent_id) -> &[u32]` | precomputed at DB2 load |
| `sDB2Manager.GetParagonReputation(id)` | `db2_mgr.paragon_reputation(id) -> Option<&ParagonReputation>` | post-WoLK, defer |
| `sDB2Manager.GetFriendshipRepReactions(repId)` | `db2_mgr.friendship_reactions(rep_id) -> Option<&FriendshipReactionSet>` | post-WoLK, defer |
| `Player* _player` back-pointer | passed-in session/player ref per call | avoid long-lived refs |
| `WorldPackets::Reputation::InitializeFactions` | `InitializeFactionsPacket` in `crates/wow-packet/src/packets/reputation.rs` | preserve flag/standing pair encoding |
| `sScriptMgr->OnPlayerReputationChange` | hook in script registry (when scripting module exists) | defer if no scripts yet |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Verdict: ⚠️ refuted on the headline bug — `SMSG_INITIALIZE_FACTIONS` IS sent on login. But the packet is a fixed-zero placeholder, so the symptom (rep pane appears empty/all-neutral) holds in practice.**

**Inventory verified:**
- Enums present in `crates/wow-constants/src/shared.rs:131-216`: `ReputationRank` (Hated..Exalted, with `None=-1` as sentinel — matches C++ `MAX_REPUTATION_RANK=8`), `ReputationSource` (Kill/Quest/Daily/Weekly/Monthly/Repeatable/Spell), `FactionTemplates` (~45 hard-coded constants), `ReputationFlags: u16` bitflags (Visible/AtWar/...). Pre-audit "❌ not started" overstates absence — these enum scaffolds exist.
- `crates/wow-packet/src/packets/misc.rs:704-725` — `pub struct InitializeFactions;` (unit struct, no fields). Its `write` method emits **1000 fixed-zero `(u16 flags, i32 standing)` pairs** then **1000 fixed-`false` bonus bits**. No per-player data — every login gets the exact same packet.
- `crates/wow-world/src/handlers/character.rs:4310-4311` — at login step 16: `self.send_packet(&InitializeFactions);`. **The packet IS sent.** The doc-flagged bug ("`SMSG_INITIALIZE_FACTIONS` not sent on login → rep pane empty") is **partially refuted**: the opcode goes out, the client should not freeze, but because every faction reports `flags=0, standing=0` and `bonus=false`, the rep pane will show every faction as default-neutral, hidden, no-bonus — visually indistinguishable from "not sent" for any player who has earned rep.
- `crates/wow-world/src/` has **no** `reputation/` module, no `ReputationMgr`, no `FactionState` (verified — full grep yields zero hits for those names).
- `crates/wow-data/src/` has **no** `Faction.db2` or `FactionTemplate.db2` reader (no `faction.rs`).
- `crates/wow-database/src/statements/character.rs` has **no** `SEL_CHARACTER_REPUTATION`, `INS_CHAR_REPUTATION_BY_FACTION`, etc.

**Refined bug status:**
- **Original claim "`SMSG_INITIALIZE_FACTIONS` not sent on login":** REFUTED — verified at character.rs:4311.
- **Underlying user-visible bug "rep pane empty":** STILL TRUE in practice — every standing is hard-coded to 0, no `character_reputation` load, no `RewardReputation` from kills/quests, so even if standings were transmitted faithfully there would be nothing to transmit.

**Largest missing surfaces (confirmed):**
- Whole `ReputationMgr` per-session state machine (`FactionStateList`, `ForcedReactions`, rank counters).
- `Faction.db2` + `FactionTemplate.db2` DB2 readers (the doc-§9 #REP.1/#REP.2 tasks).
- `SetReputation` + spillover propagation + `set_one_faction_reputation`.
- Faction CMSG handlers are now registered/implemented for `SetFactionAtWar`, `SetFactionInactive`, `RequestForcedReactions`, and `SetWatchedFaction`; watched-faction still needs the full update-field persistence/fanout work tracked by `#REP.26`.
- `SMSG_SET_FACTION_STANDING` / `_VISIBLE` / `_FORCED_REACTIONS` packet types are now present in `wow-packet::packets::reputation`; `SMSG_SET_FACTION_AT_WAR` remains only an opcode/payload carrier because C++ registers the opcode but normal reputation state fanout goes through `SMSG_SET_FACTION_STANDING`.
- `creature_onkill_reputation` now has a represented C++-validated reader/session-wired store and represented creature-kill mutation/fanout, including championing faction override for Wrath max-level non-raid LFG dungeons, represented current-session party kill-rate math with dungeon full-rate override, represented generic/faction-specific reputation aura modifiers, and represented RAF reputation bonus. `reputation_reward_rate` now has a represented C++-validated reader and is used by quest-source and creature-kill reputation gains, but exact raid-group fanout and corpse-position RAF distance remain open.
- Quest reputation reward integration (Quest module already exists but the reward-faction arrays are not consumed).
- `Player::GetReactionTo` / forced-reaction lookup for combat AI.

**Estimate: ~3% complete** — enums and opcode names defined, dummy `SMSG_INITIALIZE_FACTIONS` shell shipped, everything else absent.
