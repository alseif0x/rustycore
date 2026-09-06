# Migration: Achievements

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Achievements/` + `src/server/game/Handlers/AchievementHandler.cpp` + `src/server/game/Server/Packets/AchievementPackets.{h,cpp}`
> **Rust target crate(s):** `crates/wow-achievement/` (currently empty), with handler/wire-up in `crates/wow-world/src/handlers/` (and existing SMSG stubs in `crates/wow-packet/src/packets/misc.rs`).
> **Layer:** L6 (game systems; depends on L5 entities, L4 DB2, L3 database)
> **Status:** ❌ not started (placeholder crate confirmed empty 2026-05-01)
> **Audited vs C++:** ✅ complete
> **Audited vs Rust impl:** ✅ 2026-05-01
> **Last updated:** 2026-05-01
> **WoLK 3.4.3 relevance:** ✅ **Core WoLK content.** Achievements were added in patch 3.0 (Wrath of the Lich King). The full achievement UI, point system, guild achievements (added 4.0 Cataclysm but ported back to Wrath Classic), reward titles/items, and inspect-other-player achievement panel are all expected to work on a 3.3.5/3.4.3 realm. **This is a must-have for the WoLK Classic relaunch**, although real players can level to 80 without it; missing achievements just means an empty achievement window and no toast popups.

---

## 1. Purpose

Track player and guild progress against a hierarchy of criteria stored in client DB2 files; raise toast popups, send guild news, hand out title/item rewards, and surface "realm first" announcements when achievements complete. The same `CriteriaHandler` machinery that drives achievements is reused for **scenario step progression** and **quest objective bonus criteria**, so this is a foundational subsystem with five "owners" (player, guild, scenario, quest objective, account-wide), all driven by ~140 `CriteriaType::*` enum values.

Each "criteria type" corresponds to a verb the player performs (`KillCreature`, `LearnSpell`, `ReachLevel`, etc.). Game code calls `UpdateCriteria(type, miscValue1, miscValue2, miscValue3, ref, referencePlayer)` from anywhere in the codebase. The handler walks every active criteria of that type for the player, evaluates DB-stored conditions (`criteria_data` table) and DB2-stored modifier trees (`ModifierTree.db2`), accumulates progress, and on completion walks the criteria-tree and possibly grants the achievement.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Achievements/AchievementMgr.cpp` | 1365 | `prefix` |
| `game/Achievements/AchievementMgr.h` | 191 | `prefix` |
| `game/Achievements/CriteriaHandler.cpp` | 4285 | `prefix` |
| `game/Achievements/CriteriaHandler.h` | 421 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Achievements/AchievementMgr.h` | 191 | `AchievementMgr` (abstract base extending `CriteriaHandler`), `PlayerAchievementMgr`, `GuildAchievementMgr`, `AchievementGlobalMgr` (singleton: rewards, scripts, realm-firsts), `AchievementReward`, `AchievementRewardLocale`, `CompletedAchievementData`. |
| `src/server/game/Achievements/AchievementMgr.cpp` | 1365 | All three managers' implementations: `LoadFromDB`/`SaveToDB`/`DeleteFromDB`, `SendAllData` (the big startup payload), `SendAchievementInfo` (inspect-other), `SendAchievementEarned` (toast + broadcast), `CompletedAchievement` (rewards, titles, mail, guild news), `LoadCompletedAchievements` / `LoadRewards` / `LoadRewardLocales` / `LoadAchievementScripts`. |
| `src/server/game/Achievements/CriteriaHandler.h` | 421 | `CriteriaHandler` abstract base, `CriteriaMgr` singleton, `Criteria`, `CriteriaTree`, `ModifierTreeNode`, `CriteriaProgress`, `CriteriaData` (union of 25 data-type structs), `CriteriaDataSet`, enums `CriteriaDataType`, `CriteriaFlagsCu`, `ProgressType`. |
| `src/server/game/Achievements/CriteriaHandler.cpp` | 4285 | The machine: `UpdateCriteria` mega-switch over `CriteriaType` (140+ cases), `IsCompletedCriteria`, `RequirementsSatisfied`, `ConditionsSatisfied` (consults `criteria_data` rows), `ModifierTreeSatisfied` (walks `ModifierTree.db2`), `LoadCriteriaList` (builds the cross-index from `Criteria.db2` + `CriteriaTree.db2` + `Achievement.db2` + `ScenarioStep.db2` + `QuestObjective`), `LoadCriteriaModifiersTree`, `LoadCriteriaData`, `StartCriteria`/`FailCriteria` (timed criteria). |
| `src/server/game/Handlers/AchievementHandler.cpp` (NOT in this folder, but inseparable) | ~120 | `WorldSession::HandleQueryInspectAchievements`, `HandleGuildSetAchievementTracking`, `HandleGuildGetAchievementMembers`. The `SendAllData` is invoked from `Player::SendInitialPacketsBeforeAddToMap`. |
| `src/server/game/Server/Packets/AchievementPackets.h` | ~250 | All wire types: `AllAchievementData`, `AllAccountCriteria`, `CriteriaUpdate`, `CriteriaDeleted`, `AchievementEarned`, `AchievementDeleted`, `BroadcastAchievement`, `ServerFirstAchievement`, `RespondInspectAchievements`, `GuildAchievement*`. |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CriteriaHandler` | abstract | Owns `_criteriaProgress: map<criteriaId, CriteriaProgress>` + `_startedCriteria: map<criteriaId, time-left>`. Virtuals: `SendCriteriaUpdate`, `SendCriteriaProgressRemoved`, `SendPacket`, `CompletedCriteriaTree`, `GetOwnerInfo`, `GetCriteriaByType`, `RequiredAchievementSatisfied`. |
| `AchievementMgr : CriteriaHandler` | abstract | Adds `_completedAchievements: map<achId, CompletedAchievementData>`, `_achievementPoints: u32`, `CompletedAchievement(entry, refPlayer)` virtual. |
| `PlayerAchievementMgr : AchievementMgr` | concrete | Per-player. Owns DB persistence, sends to single `_owner: Player*`. Reads timezone offset from session for client-displayed timestamps. |
| `GuildAchievementMgr : AchievementMgr` | concrete | Per-guild. Sends to all online guild members. Has `SendAllTrackedCriterias` (UI tracker), `SendAchievementMembers` (who-has-this list). |
| `AchievementGlobalMgr` | singleton | Realm-wide. Owns `_allCompletedAchievements: map<achId, SystemTimePoint>` (realm-firsts), `_achievementListByReferencedId` (alias graph from `Achievement.SharesCriteria`), `_achievementRewards`, `_achievementRewardLocales`, `_achievementScripts`. |
| `CriteriaMgr` | singleton | Owns the entire static catalog: `_criteria`, `_criteriaTrees`, `_criteriaModifiers`, plus per-(type,asset) lookup tables for player/guild/scenario/quest-objective and per-event lookup tables for `CriteriaStartEvent`/`CriteriaFailEvent`. |
| `Criteria` | struct | `{ID, CriteriaEntry* Entry, ModifierTreeNode* Modifier, FlagsCu (player/account/guild/scenario/questObj)}`. |
| `CriteriaTree` | struct | Hierarchical: `{ID, CriteriaTreeEntry* Entry, AchievementEntry* Achievement, ScenarioStepEntry* ScenarioStep, QuestObjective* QuestObjective, Criteria* Criteria, vector<CriteriaTree*> Children}`. The same tree node can be claimed by an Achievement, a ScenarioStep, or a QuestObjective. |
| `ModifierTreeNode` | struct | `{ModifierTreeEntry* Entry, vector<children>}`. Tree of conditions evaluated AND/OR depending on `ModifierTreeEntry.Operator`. |
| `CriteriaProgress` | struct | Per-criteria runtime state: `{Counter: u64, Date: time_t, PlayerGUID, Changed: bool}`. |
| `CompletedAchievementData` | struct | Per-achievement: `{Date: time_t, CompletingPlayers: GuidSet (guild), Changed: bool}`. |
| `CriteriaData` | tagged-union | One of 25 variants (creature id / class+race / health % / aura / value+cmp / level / gender / map players / team / drunk / holiday / bg score range / equipped item / map id / known title / game event / item quality / script). Stored in `criteria_data` SQL table. |
| `CriteriaDataSet` | struct | `vector<CriteriaData>` ANDed for one criteria. |
| `CriteriaFlagsCu` | enum flag | `PLAYER=0x1, ACCOUNT=0x2, GUILD=0x4, SCENARIO=0x8, QUEST_OBJECTIVE=0x10` — server-side category derived from the ownership graph. |
| `CriteriaDataType` | enum | 17 used + 1 reserved (`MAP_DIFFICULTY=12 used on 3.3.5a branch`, `NTH_BIRTHDAY=22 used on 3.3.5a branch`). |
| `ProgressType` | enum | `PROGRESS_SET, PROGRESS_ACCUMULATE, PROGRESS_HIGHEST`. Determines how `SetCriteriaProgress` updates `Counter`. |
| `AchievementReward` | struct | `{TitleId[2] (alliance/horde), ItemId, SenderCreatureId, Subject, Body, MailTemplateId}`. From `achievement_reward` SQL. |
| `AchievementRewardLocale` | struct | Localized subject/body, indexed by `LocaleConstant`. From `achievement_reward_locale`. |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CriteriaHandler::UpdateCriteria(type, m1, m2, m3, ref, refPlayer)` | The single entry-point invoked from gameplay code. ~4000-line switch dispatches on `CriteriaType`. For each matching `Criteria*` of `type`: `CanUpdateCriteria` → `RequirementsSatisfied` → `ConditionsSatisfied` → `SetCriteriaProgress`. | every caller (`Player::KilledMonsterCredit`, `Spell::Cast`, `Quest::CompleteQuest`, etc.) |
| `CriteriaHandler::SetCriteriaProgress(criteria, value, refPlayer, progressType)` | Mutates `_criteriaProgress[id]`. SET writes value, ACCUMULATE adds, HIGHEST takes `max`. Calls `SendCriteriaUpdate` and walks `criteriaTreeByCriteria` to fire `CompletedCriteriaTree` if newly complete. | `CanCompleteCriteriaTree`, `CompletedCriteriaTree` |
| `CriteriaHandler::IsCompletedCriteria(criteria, requiredAmount)` | Compares `_criteriaProgress[id].Counter` against the threshold from `CriteriaTree.Amount` or `Criteria.Type`-specific rule. Some types ignore Amount entirely. | direct compare |
| `CriteriaHandler::ConditionsSatisfied(criteria, refPlayer)` | Walks `_criteriaDataMap[criteriaId]` (from SQL); each `CriteriaData` evaluated by `Meets(refPlayer, target, m1, m2)`. AND across all rows. | per-DataType branch: creature template, class/race lookup, aura check, drunk state, etc. |
| `CriteriaHandler::ModifierTreeSatisfied(node, m1, m2, ref, refPlayer)` | Recursively walks the DB2 modifier tree. Each `ModifierTreeEntry.Operator` is a 234-case enum (pet level, item enchant, year of game, garrison stuff, etc.) — many WoLK-irrelevant branches. | `ModifierSatisfied(modifier, ...)` per node |
| `CriteriaHandler::StartCriteria(startEvent, entry, timeLost)` | Begin a timed criteria: insert into `_startedCriteria[id]` with `Criteria.Entry->StartTimer - timeLost` ms. | `_criteriasByStartEvent` lookup |
| `CriteriaHandler::FailCriteria(failEvent, asset)` virtual | Default no-op; player overrides to send `CriteriaDeleted` and remove progress. | `RemoveCriteriaProgress` |
| `CriteriaHandler::UpdateTimedCriteria(timeDiff)` | Tick the `_startedCriteria` map; expired → `SendCriteriaProgressRemoved`. | `SendCriteriaUpdate(..., timedCompleted=true)` if it just completed before expiring |
| `AchievementMgr::CheckAllAchievementCriteria(refPlayer)` | Called at login. Calls `UpdateCriteria(t, 0, 0, 0, nullptr, refPlayer)` for every type 0..Count. Suppresses packets via `PlayerLoading`. | the 140-way switch |
| `AchievementMgr::CompletedCriteriaTree(tree, refPlayer)` | If `tree->Achievement && IsCompletedAchievement` → `CompletedAchievement(achievement, refPlayer)`. | `CompletedAchievement` (virtual) |
| `AchievementMgr::IsCompletedAchievement(entry)` | Counter-flag → never. SUMM-flag → walk tree, sum counters, compare vs `Amount`. Default → `IsCompletedCriteriaTree(tree)`. | `WalkCriteriaTree` lambda |
| `PlayerAchievementMgr::CompletedAchievement(ach, refPlayer)` | The reward path: skip if GM, faction filter, COUNTER flag, already-have. Set title (gender-special-case for ach 1793), `_achievementPoints += Points` unless TRACKING_FLAG, send guild news, send `SMSG_ACHIEVEMENT_EARNED`, persist with `Changed=true`, fire `UpdateCriteria(EarnAchievement)` and `UpdateCriteria(EarnAchievementPoints)`, run `sScriptMgr->OnAchievementCompleted`, deliver mail rewards. | `Guild::AddGuildNews`, `MailDraft::SendMailTo`, `Player::SetTitle` |
| `PlayerAchievementMgr::SendAllData(receiver)` | Builds `AllAchievementData` (every visible completed entry + every progress entry, both timezone-shifted) and `AllAccountCriteria` (only `FLAG_CU_ACCOUNT` ones), sends both at login. | DB2 lookups |
| `PlayerAchievementMgr::SendAchievementEarned(achievement)` | Two packets: `SMSG_ACHIEVEMENT_EARNED` to self, `SMSG_BROADCAST_ACHIEVEMENT` to all in 200-yard radius **but only once per achievement realm-wide** for `REALM_FIRST_*` flags, otherwise to nearby. | `Cell::VisitWorldObjects` with `LocalizedDo` |
| `PlayerAchievementMgr::LoadFromDB(achResult, criteriaResult)` | Two prepared queries' rows. Drops criteria with `StartTimer` already expired. Re-grants titles for retroactive achievements at login. | DB2 |
| `PlayerAchievementMgr::SaveToDB(trans)` | Walks `_completedAchievements` and `_criteriaProgress`, only writes those with `Changed=true`. DELETE-then-INSERT pattern, not UPDATE. | `CHAR_DEL_*` + `CHAR_INS_*` |
| `GuildAchievementMgr::SendAchievementMembers(receiver, achId)` | `SMSG_GUILD_ACHIEVEMENT_MEMBERS` with the `CompletingPlayers` GuidSet from the completed-data row. | direct |
| `AchievementGlobalMgr::IsRealmCompleted(ach)` | Used by `CanCompleteCriteriaTree` to gate REALM_FIRST achievements. Atomic on the `_allCompletedAchievements` map. | direct |
| `AchievementGlobalMgr::LoadAchievementReferenceList()` | Walks `sAchievementStore`; for each `SharesCriteria > 0`, inverts the index into `_achievementListByReferencedId` so `AfterCriteriaTreeUpdate` can ripple completion to "twin" achievements. | `sAchievementStore` |
| `AchievementGlobalMgr::LoadCompletedAchievements()` | `SELECT achievement FROM character_achievement GROUP BY achievement` to seed the realm-firsts map. | `CharacterDatabase` |
| `AchievementGlobalMgr::LoadRewards()` / `LoadRewardLocales()` | World DB. Validates titles exist (check both Alliance/Horde), validates ItemID, validates SenderCreatureId. | `WorldDatabase`, DB2 |
| `CriteriaMgr::LoadCriteriaList()` | The big startup pass: walks `sCriteriaTreeStore`, `sAchievementStore`, `sCriteriaStore`, `sScenarioStepStore`, plus quest objectives in `sObjectMgr`, builds: `_criteria`, `_criteriaTrees`, `_criteriaTreeByCriteria` reverse map, `_criteriasByType[type]`, `_criteriasByAsset[type][asset]`, `_guildCriteriasByType`, `_scenarioCriteriasByTypeAndScenarioId`, `_questObjectiveCriteriasByType`, `_criteriasByStartEvent`, `_criteriasByFailEvent`. Sets `Criteria.FlagsCu` per which owner type holds it. | DB2 stores, `sObjectMgr` |
| `CriteriaMgr::LoadCriteriaModifiersTree()` | Build `_criteriaModifiers` (`ModifierTreeEntry*` → `ModifierTreeNode*` tree). | `sModifierTreeStore` |
| `CriteriaMgr::LoadCriteriaData()` | Read `criteria_data` SQL rows, validate per criteria type, build `_criteriaDataMap`. | `WorldDatabase` |

---

## 5. Module dependencies

**Depends on:**
- DB2 stores: `Achievement.db2`, `Achievement_Category.db2`, `Criteria.db2`, `CriteriaTree.db2`, `ModifierTree.db2`, `ScenarioStep.db2`, `CharTitles.db2`. (All loaded by `wow-data` / `wow-data DB2Stores`.)
- `ObjectMgr` — quest objectives, item templates, creature templates, locale strings, `GetTrinityStringForDBCLocale`.
- `Player` — `GetGUID`, `GetTeam`, `GetSession()->GetTimezoneOffset`, `SetTitle`, `IsGameMaster`, RBAC.
- `Guild` — guild achievements, guild news.
- `Mail` — reward mail delivery.
- `Item` — `Item::CreateItem` for item rewards.
- `Spell`/`SpellAuras` — aura criteria-data check.
- `Battleground`/`BattlegroundMgr` — bg-score and arena win checks.
- `BattlePetMgr` (NYI in WoLK) — pet level criteria.
- `CollectionMgr` (NYI in WoLK) — toy/transmog/heirloom criteria.
- `Scenario` — scenario step progression.
- `ScriptMgr` — `OnCriteriaCheck`, `OnCriteriaProgress`, `OnAchievementCompleted`.
- `WorldStateMgr`, `GameEventMgr`, `DisableMgr`, `ReputationMgr`, `Map`, `MapManager`, `InstanceScript`, `LanguageMgr`, `RealmList`, `WowTime`.

**Depended on by:**
- Player login sequence (`SendAllData`).
- Almost every gameplay system (140 callsites of `UpdateCriteria` across `Player.cpp`, `Spell.cpp`, `Combat`, `Quest`, `Loot`, `Battleground`, `Trade`, etc.).
- Guild news / guild rosters.
- Inspect-other-player UI.
- Realm-first toast broadcast.

---

## 6. SQL / DB queries (if any)

Player side (character DB):

| Statement | Purpose |
|---|---|
| `CHAR_SEL_CHARACTER_ACHIEVEMENTS` (used in `Player::LoadFromDB` → passed to `LoadFromDB`) | Player's earned achievements. |
| `CHAR_SEL_CHARACTER_ACHIEVEMENT_PROGRESS` | Player's in-progress criteria. |
| `CHAR_DEL_CHAR_ACHIEVEMENT` / `CHAR_DEL_CHAR_ACHIEVEMENT_PROGRESS` | Reset/delete. |
| `CHAR_DEL_CHAR_ACHIEVEMENT_BY_ACHIEVEMENT` | Per-row dedupe before INSERT. |
| `CHAR_INS_CHAR_ACHIEVEMENT` | Persist completion. |
| `CHAR_DEL_CHAR_ACHIEVEMENT_PROGRESS_BY_CRITERIA` | Per-row dedupe. |
| `CHAR_INS_CHAR_ACHIEVEMENT_PROGRESS` | Persist progress (only if `Counter > 0`). |
| `CHAR_DEL_INVALID_ACHIEV_PROGRESS_CRITERIA` | Cleanup orphaned criteria rows from removed DB2 entries. |
| `CHAR_DEL_INVALID_ACHIEVMENT` | Cleanup orphaned achievement rows (note typo). |

Guild side (character DB):

| Statement | Purpose |
|---|---|
| `CHAR_DEL_ALL_GUILD_ACHIEVEMENTS`, `CHAR_DEL_ALL_GUILD_ACHIEVEMENT_CRITERIA` | Reset on guild disband. |
| `CHAR_DEL_INVALID_ACHIEV_PROGRESS_CRITERIA_GUILD` | Cleanup. |
| `CHAR_DEL_GUILD_ACHIEVEMENT` / `CHAR_INS_GUILD_ACHIEVEMENT` | Per-achievement. |
| `CHAR_DEL_GUILD_ACHIEVEMENT_CRITERIA` / `CHAR_INS_GUILD_ACHIEVEMENT_CRITERIA` | Per-criterion. |

World side (world DB) — startup only:

| Query | Purpose |
|---|---|
| `SELECT achievement FROM character_achievement GROUP BY achievement` (cross-DB!) | Realm-firsts seed (run from `LoadCompletedAchievements`). |
| `SELECT AchievementId, ScriptName FROM achievement_scripts` | Maps an achievement to a registered script. |
| `SELECT ID, TitleA, TitleH, ItemID, Sender, Subject, Body, MailTemplateID FROM achievement_reward` | Reward rows. |
| `SELECT ID, Locale, Subject, Body FROM achievement_reward_locale` | Localized rewards. |
| `SELECT ID, Type, Value1, Value2, ScriptName FROM criteria_data` (loaded by `CriteriaMgr::LoadCriteriaData`) | Per-criteria server-side condition rows. |

DB2/DBC stores used:

| Store | What it loads | Read by |
|---|---|---|
| `sAchievementStore` | `Achievement.db2` | AchievementMgr, CriteriaMgr |
| `sAchievement_CategoryStore` | `Achievement_Category.db2` | UI grouping |
| `sCriteriaStore` | `Criteria.db2` | CriteriaMgr |
| `sCriteriaTreeStore` | `CriteriaTree.db2` | CriteriaMgr |
| `sModifierTreeStore` | `ModifierTree.db2` | CriteriaHandler |
| `sScenarioStepStore` | `ScenarioStep.db2` | CriteriaMgr |
| `sCharTitlesStore` | `CharTitles.db2` | reward-title resolution |
| `sCriteriaCategoryStore` | `Criteria_Category.db2` (??) | minor |
| `sQuestV2Store` / quest objectives | via `sObjectMgr` | quest-objective criteria |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_QUERY_INSPECT_ACHIEVEMENTS` (0x3500) | client → server | `WorldSession::HandleQueryInspectAchievements` → `PlayerAchievementMgr::SendAchievementInfo(receiver)`. |
| `CMSG_GUILD_SET_ACHIEVEMENT_TRACKING` (0x306f) | client → server | `WorldSession::HandleGuildSetAchievementTracking`. |
| `SMSG_ALL_ACHIEVEMENT_DATA` (0x2570) | server → client | `PlayerAchievementMgr::SendAllData` at login. |
| `SMSG_ALL_ACCOUNT_CRITERIA` (0x2571) | server → client | Same — only if any criteria is `FLAG_CU_ACCOUNT`. |
| `SMSG_RESPOND_INSPECT_ACHIEVEMENTS` (0x2572) | server → client | Inspect response. |
| `SMSG_CRITERIA_UPDATE` (0x26e1) | server → client | `PlayerAchievementMgr::SendCriteriaUpdate` per criteria progress change. |
| `SMSG_CRITERIA_DELETED` (0x26e7) | server → client | Reset/timeout. |
| `SMSG_ACCOUNT_CRITERIA_UPDATE` (0x2868) | server → client | Account-wide criteria update. |
| `SMSG_ACHIEVEMENT_EARNED` (0x2643) | server → client | To self on completion. |
| `SMSG_BROADCAST_ACHIEVEMENT` (0x2bbc) | server → client | To others nearby (or world for realm-firsts). |
| `SMSG_ACHIEVEMENT_DELETED` (0x26e8) | server → client | On `Reset()` (GM command). |
| `SMSG_SERVER_FIRST_ACHIEVEMENTS` (0x264e) | server → client | Realm-first announcement bag. |
| `SMSG_GUILD_CRITERIA_UPDATE` (0x29c4) | server → client | Guild side of `CriteriaUpdate`. |
| `SMSG_GUILD_ACHIEVEMENT_EARNED` (0x29c5) | server → client | Guild completion. |
| `SMSG_GUILD_ACHIEVEMENT_DELETED` (0x29c6) | server → client | Guild reset. |
| `SMSG_GUILD_CRITERIA_DELETED` (0x29c7) | server → client | Guild criteria reset. |
| `SMSG_GUILD_ACHIEVEMENT_MEMBERS` (0x29c8) | server → client | "Show me everyone in the guild who has this." |
| `SMSG_ALL_GUILD_ACHIEVEMENTS` (0x29b8) | server → client | Guild login bag (all guild-completed). |
| `SMSG_SCENARIO_SHOW_CRITERIA` (0x2804) | server → client | Scenario UI panel. |

(All opcodes already enumerated in `crates/wow-constants/src/opcodes.rs`. Many SMSG bodies are not yet implemented.)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-achievement` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |
| `crates/wow-achievement/Cargo.toml` | `file` | 1 | 10 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-achievement/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-achievement/Cargo.toml` — declares deps on `wow-core`, `wow-constants`. **9 lines of metadata.**
- `crates/wow-achievement/src/lib.rs` — **0 bytes (empty).**
- `crates/wow-packet/src/packets/misc.rs` lines 899-918 — **stub `AllAccountCriteria` and `AllAchievementData` ServerPackets that serialize as empty.** They write only the opcode header; client interprets as "no achievements, no progress". Unit tests at lines 2317, 2326 confirm the empty-payload contract.
- `crates/wow-world/src/handlers/character.rs` lines 4319-4323 — login sends both empty packets in the initial-packets sequence. Comment markers `// 19. AllAccountCriteria (empty)` and `// 20. AllAchievementData (empty)`.
- `crates/wow-world/src/handlers/misc.rs` line 595 — `handle_guild_set_achievement_tracking` is a stub no-op (`async fn ... (&mut self, _pkt: WorldPacket) {}`), wired in `session.rs` line 1784.
- `crates/wow-constants/src/opcodes.rs` — every relevant opcode already enumerated (see section 7 above).

**What's implemented:**
- The login wire-up only: client receives empty `SMSG_ALL_ACCOUNT_CRITERIA` + `SMSG_ALL_ACHIEVEMENT_DATA` and shows an empty achievement window. `CMSG_GUILD_SET_ACHIEVEMENT_TRACKING` is silently absorbed.

**What's missing vs C++:**
- The entire `CriteriaHandler` machine.
- `CriteriaMgr` static catalog builder.
- `AchievementMgr` / `PlayerAchievementMgr` / `GuildAchievementMgr` / `AchievementGlobalMgr`.
- DB2 store integration for `Achievement`, `Criteria`, `CriteriaTree`, `ModifierTree`, `ScenarioStep`, `CharTitles`.
- DB persistence (9 character-DB prepared statements + 4 world-DB queries).
- All ~140 `CriteriaType::*` dispatch arms and the corresponding `UpdateCriteria` callsites scattered across gameplay code.
- `criteria_data` SQL evaluation (25 data-types).
- `ModifierTree.db2` recursive evaluator (234-case `Operator` enum).
- Realm-first detection.
- Reward delivery (titles, items, mail, locale lookup).
- Inspect-other-player and guild members panels.
- All non-`AllAchievementData` SMSGs.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- N/A — there is no implementation to diverge.

**Tests existing:**
- 2 packet tests in `crates/wow-packet/src/packets/misc.rs` confirming the empty-payload SMSGs.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ACHIEVEMENTS.WBS.001** Partir y cerrar la migracion auditada de `game/Achievements/AchievementMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`
  Rust target: `crates/wow-achievement`, `crates/wow-world/src/handlers`, `crates/wow-core`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1365 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ACHIEVEMENTS.WBS.002** Cerrar la migracion auditada de `game/Achievements/AchievementMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.h`
  Rust target: `crates/wow-achievement`, `crates/wow-world/src/handlers`, `crates/wow-core`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ACHIEVEMENTS.WBS.003** Partir y cerrar la migracion auditada de `game/Achievements/CriteriaHandler.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`
  Rust target: `crates/wow-achievement`, `crates/wow-world/src/handlers`, `crates/wow-core`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4285 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ACHIEVEMENTS.WBS.004** Cerrar la migracion auditada de `game/Achievements/CriteriaHandler.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h`
  Rust target: `crates/wow-achievement`, `crates/wow-world/src/handlers`, `crates/wow-core`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

Strategy: build the static catalog (criteria mgr) first, then the per-player runtime, then bring up criteria-type dispatch incrementally (start with the 5-10 types covering 80% of WoLK content: KillCreature, ReachLevel, EarnHonorableKill, CompleteQuest, EarnAchievement). Defer the 100+ NYI / non-WoLK types behind feature flags or `unimplemented!()` panics so they're easy to find.

- [ ] **#ACHIEV.1** Confirm DB2 stores load: ensure `wow-data` exposes `sAchievementStore`, `sCriteriaStore`, `sCriteriaTreeStore`, `sModifierTreeStore`, `sCharTitlesStore`. Add load tests against the live `dbc/` files. (M)
- [ ] **#ACHIEV.2** Define POD types in `wow-achievement::types`: `Criteria`, `CriteriaTree`, `ModifierTreeNode`, `CriteriaProgress`, `CriteriaData` (Rust enum, not C union), `CriteriaDataSet`, `CompletedAchievementData`, `AchievementReward`, `AchievementRewardLocale`. (M)
- [ ] **#ACHIEV.3** Port `CriteriaMgr::LoadCriteriaList` — build `_criteria`, `_criteriaTrees`, `_criteriaTreeByCriteria`, all the per-(type,asset) lookup tables. Tag each `Criteria.FlagsCu` correctly (player vs guild vs scenario vs questobj vs account). (XL — split: tree topology pass, then index-build pass, then flagscu tagging pass.)
- [ ] **#ACHIEV.4** Port `CriteriaMgr::LoadCriteriaModifiersTree` — recursive build of `_criteriaModifiers` from `ModifierTree.db2`. (M)
- [ ] **#ACHIEV.5** Port `CriteriaMgr::LoadCriteriaData` — read `criteria_data` SQL, validate per type. Reject unsupported types per the C++ filter (lines 60-95 of `CriteriaHandler.cpp`). (M)
- [ ] **#ACHIEV.6** Port `CriteriaHandler` base trait + `_criteriaProgress` map + `_startedCriteria` map. Define associated abstract methods. (M)
- [ ] **#ACHIEV.7** Port `CriteriaHandler::SetCriteriaProgress` (SET/ACCUMULATE/HIGHEST) + `IsCompletedCriteria` + `RemoveCriteriaProgress`. (M)
- [ ] **#ACHIEV.8** Port `CriteriaHandler::UpdateTimedCriteria(diff)` + `StartCriteria` / default `FailCriteria`. (M)
- [ ] **#ACHIEV.9** Port `ConditionsSatisfied` for the 25 `CriteriaDataType` arms. Many WoLK arms (creature, class/race, value/level/gender, drunk, holiday, bg-score, equipped item, map id, known title, item quality) are simple field reads; `S_AURA`/`T_AURA` and `INSTANCE_SCRIPT`/`SCRIPT` need integration with spell/instance/script systems and can stub initially. (H — split: per-DataType subtask if needed.)
- [ ] **#ACHIEV.10** Port `ModifierTreeSatisfied` recursion. The 234-case `Operator` switch: implement the ~30 WoLK-relevant operators first (level, faction, class, race, item count, currency, quest completion, reputation rank, map id, area id, gender, drunk, group size, on team), stub the BfA+/Garrison/BattlePet/Heritage/Transmog operators with `false`. (XL)
- [ ] **#ACHIEV.11** Port `CriteriaHandler::UpdateCriteria` mega-switch. Implement the 10 most-fired WoLK types first: `KillCreature`, `KilledByCreature`, `ReachLevel`, `Login`, `EarnAchievement`, `EarnAchievementPoints`, `EarnHonorableKill`, `LootAnyItem`, `ObtainAnyItem`, `CompleteQuest`. (XL — incremental, one type per PR ideal.)
- [ ] **#ACHIEV.12** Port `AchievementMgr` base abstract. (L)
- [ ] **#ACHIEV.13** Port `PlayerAchievementMgr`: `LoadFromDB`/`SaveToDB`/`DeleteFromDB`/`Reset`/`CheckAllAchievementCriteria`. Use the 9 prepared statements in `wow-database::statements::character`. (H)
- [ ] **#ACHIEV.14** Port `PlayerAchievementMgr::SendAllData` + `SendAchievementInfo` + `SendAchievementEarned`. **This will replace the empty stubs in `crates/wow-packet/src/packets/misc.rs`.** Build proper `AllAchievementData` body (Earned[] + Progress[]). (H)
- [ ] **#ACHIEV.15** Port `PlayerAchievementMgr::CompletedAchievement` reward path: title, points, mail, guild news, script hook, faction filter, GM filter. (H)
- [ ] **#ACHIEV.16** Port `AchievementGlobalMgr` realm-firsts: `IsRealmCompleted`, `SetRealmCompleted`, `LoadCompletedAchievements`. (M)
- [ ] **#ACHIEV.17** Port `AchievementGlobalMgr::LoadRewards` + `LoadRewardLocales` + `LoadAchievementScripts` + `LoadAchievementReferenceList`. (M)
- [ ] **#ACHIEV.18** Port `GuildAchievementMgr` parallel to player one. Defer if guilds aren't ported yet — same level as the guild module. (XL)
- [ ] **#ACHIEV.19** Wire `UpdateCriteria` callsites across the Rust gameplay code. Every place `Player.cpp` etc. call it gets a Rust equivalent. ~140 callsites; this is grindy but mechanical. (XL — coordinate with each gameplay system's owner.)
- [ ] **#ACHIEV.20** Replace `crates/wow-packet/src/packets/misc.rs` `AllAccountCriteria` / `AllAchievementData` empty stubs with real serializers. (M, depends on #ACHIEV.13)
- [ ] **#ACHIEV.21** Real `handle_guild_set_achievement_tracking` (replace the no-op at `handlers/misc.rs:595`). (L, depends on #ACHIEV.18)
- [ ] **#ACHIEV.22** Implement `CMSG_QUERY_INSPECT_ACHIEVEMENTS` handler → `SendAchievementInfo`. (M)
- [ ] **#ACHIEV.23** Add scripts-bridge: `OnAchievementCompleted`, `OnCriteriaCheck`, `OnCriteriaProgress` hooks for `wow-script`. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ACHIEVEMENTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 6262 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h`. Rust target: `crates/wow-achievement`, `crates/wow-constants`, `crates/wow-core`. | `cargo test -p wow-achievement && cargo test -p wow-constants && cargo test -p wow-core` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ACHIEVEMENTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 6262 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h`. Rust target: `crates/wow-achievement`, `crates/wow-constants`, `crates/wow-core`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ACHIEVEMENTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 6262 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h`. Rust target: `crates/wow-achievement`, `crates/wow-constants`, `crates/wow-core`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ACHIEVEMENTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 6262 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h`. Rust target: `crates/wow-achievement`, `crates/wow-constants`, `crates/wow-core`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `CriteriaProgress::Counter` SET vs ACCUMULATE vs HIGHEST behave per spec (HIGHEST never decreases).
- [ ] Test: `IsCompletedAchievement` for an achievement with `FLAG_COUNTER` returns `false` even when counter ≥ amount.
- [ ] Test: `IsCompletedAchievement` for an achievement with `FLAG_SUMM` returns `true` iff sum of all child criteria counters ≥ tree amount.
- [ ] Test: `CompletedAchievement` short-circuits if `HasAchieved(id)` returns true.
- [ ] Test: `CompletedAchievement` does not award if player is GameMaster with GM mode on.
- [ ] Test: Faction filter: Horde-only achievement skipped for Alliance player.
- [ ] Test: REALM_FIRST ach: only the first player to complete it succeeds; second player's `CanCompleteCriteriaTree` returns `false`.
- [ ] Test: Achievement 1793 (gender-special) picks title from `_owner->GetNativeGender()` index, not team.
- [ ] Test: Reward title reapplied at login from `LoadFromDB`.
- [ ] Test: `LoadFromDB` drops criteria with `StartTimer` already expired (timestamp older than now − StartTimer).
- [ ] Test: `SaveToDB` only writes rows with `Changed = true`; second consecutive save is a no-op.
- [ ] Test: `_achievementPoints` accumulates correctly except for `FLAG_TRACKING_FLAG` achievements.
- [ ] Test: `UpdateCriteria(EarnAchievement, achId, ...)` is fired by `CompletedAchievement`, allowing meta-achievements to trigger.
- [ ] Test: Wire format: empty `AllAchievementData` serializes to header-only (matches existing test); non-empty serializes with per-Earned timezone-shifted timestamps.
- [ ] Test: Round-trip: insert into character_achievement, restart, verify `HasAchieved` returns `true` and points re-counted.
- [ ] Test: `CriteriaData::Meets` for `T_CREATURE` matches when target is creature with given entry; mismatches otherwise.
- [ ] Test: `criteria_data` row with `DataType` not in the per-criteria-type whitelist is rejected at load with error log.
- [ ] Test: `ConditionsSatisfied` AND-semantics across multiple `CriteriaData` rows.
- [ ] Test: `ModifierTreeSatisfied` AND vs OR per `Operator` value (root op vs node op).
- [ ] Test (regression for the empty-stub replacement): A character with one completed achievement receives an `AllAchievementData` payload with `Earned.len() == 1` and `Progress.len() == 0`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 6262 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h` | `crates/wow-achievement/` (currently empty), with handler/wire-up in `crates/wow-world/src/handlers/` (and existing SMSG stubs in `crates/wow-packet/src/packets/misc.rs`). \| ❌ not started (placeholder crate confirmed empty 2026-05-01) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ACHIEVEMENTS.DIV.001` | `crates/wow-achievement` (`exists_empty`, 0 Rust lines) | 4 C++ files / 6262 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#ACHIEVEMENTS.DIV.002` | `crates/wow-achievement/src/lib.rs` (`exists_empty`, 0 Rust lines) | 4 C++ files / 6262 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/AchievementMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Achievements/CriteriaHandler.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **`CriteriaTree` is shared between three owners.** A single `CriteriaTreeEntry` row can be claimed by an Achievement, a ScenarioStep, *and* a QuestObjective. Don't model "the achievement owns the tree" — it's the tree that owns pointers to its possible owners. The `FlagsCu` of each `Criteria` records which owner type it's for; this drives `CriteriaMgr::GetCriteriaByType` to return only criteria of the matching ownership.
- **Two `CriteriaDataType` values are 3.3.5a-only:** `MAP_DIFFICULTY = 12` and `NTH_BIRTHDAY = 22` (header comment says "used on 3.3.5a branch"). The Wrath Classic 3.4.3 file retains both; keep them implemented.
- **`Achievement.Faction`** is `0=Horde` / `1=Alliance` / `-1=both` (i.e. "no faction filter"). The C++ code uses negative test on the constant `ACHIEVEMENT_FACTION_HORDE/_ALLIANCE` enum values (which are 0/1, not bitfields).
- **Achievement 1793** is the gender-special-cased title award (`The Patient` / `The Hallowed`-era). Don't refactor away the magic constant in `CompletedAchievement` — it's the only achievement that needs gender-based (vs faction-based) title selection. Comment in C++ says future work could move this to the conditions system.
- **Realm-first dual flags.** `REALM_FIRST_REACH` and `REALM_FIRST_KILL` both gate via `IsRealmCompleted`; `RBAC_PERM_CANNOT_EARN_REALM_FIRST_ACHIEVEMENTS` denies a player from competing.
- **Counter achievements never complete.** `FLAG_COUNTER` ones are tally-only; if you don't filter them in `IsCompletedAchievement` you'll spuriously trigger toast popups.
- **DELETE-then-INSERT, not UPDATE.** Both `PlayerAchievementMgr::SaveToDB` and `GuildAchievementMgr::SaveToDB` use this pattern with `CHAR_DEL_..._BY_ACHIEVEMENT` then `CHAR_INS_...`. Important if you add ON DUPLICATE KEY UPDATE schemas — don't mismatch.
- **Login retroactive title award.** `PlayerAchievementMgr::LoadFromDB` walks `_completedAchievements` and re-applies `reward->TitleId[teamIndex]`. This is how a player who earned a title before the server got the achievement-reward row gets it on next login.
- **Two-side mail subject in `BlackMarket`-style format.** Achievement reward mail uses `MailDraft(subject, body)` with localized subject/body or a `MailTemplateId`; not a parseable subject string like BMAH.
- **`AfterCriteriaTreeUpdate` ripple.** When a criteria-tree finishes, *referenced* achievements (those in `_achievementListByReferencedId`) are also re-checked for completion. This is how meta achievements "finish" when their last child does, even if the child achievement's tree doesn't directly reference them.
- **Timezone fudge.** `earned.Date += _owner->GetSession()->GetTimezoneOffset()` — the date is sent in client-local time, not UTC. Match this exactly or the achievement timestamps display wrong.
- **`SUMM` flag on achievements.** Their target count is in the `CriteriaTree.Amount`, not the achievement entry. `IsCompletedAchievement` walks the tree and sums counters.
- **Performance hotspot.** `UpdateCriteria(KillCreature, ...)` fires on every mob death. `_criteriasByAsset[KillCreature][creatureId]` lookup must be O(1) — preserve the C++ structure, don't fall back to linear scans.
- **Account-wide criteria.** `FLAG_CU_ACCOUNT` triggers a separate `AllAccountCriteria` packet at login keyed by Battle.net account GUID (not character GUID). Only fired if there's at least one such criteria with progress.
- **Empty `lib.rs` gotcha.** The current `crates/wow-achievement/src/lib.rs` is 0 bytes. When implementation begins, check that the crate even compiles before adding tests.
- **`AchievementHandler.cpp` is small** (~120 lines) but the work is in `AchievementMgr.cpp` (1365) and especially `CriteriaHandler.cpp` (4285). 80% of the migration cost is the 4k-line file.
- **WoLK doesn't have battle pets / collections / garrison / heritage.** Many `CriteriaType` and `ModifierTreeOperator` values are dead on a WoLK realm. Either stub them with `unimplemented!()` (quick to find at runtime) or `return false` (safer in production). Pick one and be consistent.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CriteriaHandler` (abstract w/ pure virtuals) | `pub trait CriteriaHandler` + a `CriteriaState` struct holding `_criteriaProgress`/`_startedCriteria` | Trait for the polymorphic surface, struct for shared state. Implementors hold a `CriteriaState` field. |
| `class AchievementMgr : CriteriaHandler` | `pub trait AchievementMgr: CriteriaHandler` (super-trait) | Adds `_completedAchievements` + `_achievementPoints` to a state struct. |
| `class PlayerAchievementMgr : AchievementMgr` | `pub struct PlayerAchievementMgr { state: ..., owner_guid: ObjectGuid }` | Hold session/player by GUID, not by `&Player` (avoids lifetime knot). Lookups via `WorldSession`/`MapManager`. |
| `class GuildAchievementMgr : AchievementMgr` | `pub struct GuildAchievementMgr { state: ..., guild_id: u32 }` | Same pattern. |
| `class AchievementGlobalMgr` (singleton) | `pub struct AchievementGlobalMgr` + `OnceLock<RwLock<AchievementGlobalMgr>>` | Realm-firsts mutate at runtime — needs interior mutability. |
| `class CriteriaMgr` (singleton, immutable post-load) | `pub struct CriteriaMgr` + `OnceLock<CriteriaMgr>` | All static after load — no `RwLock` needed. |
| `unordered_map<u32, Criteria*>` (owns Criteria) | `HashMap<u32, Arc<Criteria>>` or `HashMap<u32, Criteria>` | If `CriteriaTree` holds `*const Criteria`, use `Arc<Criteria>` and store ids; or use `slotmap`/indices into a `Vec`. The latter is more idiomatic Rust and avoids `unsafe` chains. |
| `CriteriaTree* (with raw children pointers)` | `pub struct CriteriaTree { id: u32, children: Vec<u32> }` + indirection through `_criteriaTrees: HashMap<u32, CriteriaTree>` | Replace pointer recursion with id-based recursion. Easier to test, easier to serialize. |
| `union CriteriaData { ... 25 structs ... }` + `DataType` tag | `pub enum CriteriaData { Creature{id}, Aura{spell, idx}, ... }` | Native Rust tagged union; no manual discriminant. |
| `enum CriteriaDataType` | The discriminant is the Rust enum, no separate type. | Drop `MAX_CRITERIA_DATA_TYPE`. |
| `enum ProgressType` | `pub enum ProgressType { Set, Accumulate, Highest }` | Direct map. |
| `_criteriasByType[CriteriaType::Count]` (array of vec) | `[Vec<Arc<Criteria>>; CRITERIA_TYPE_COUNT]` or `Vec<Vec<...>>` | If `CriteriaType::Count` is large (~250), prefer `Vec<Vec<...>>` to avoid blowing stack. |
| `_criteriasByAsset[type][asset]: map<u32, vec>` | `[HashMap<u32, Vec<Arc<Criteria>>>; CRITERIA_TYPE_COUNT]` | Same trade-off. |
| `void UpdateCriteria(CriteriaType, u64, u64, u64, WorldObject*, Player*)` | `fn update_criteria(&mut self, ty: CriteriaType, m1: u64, m2: u64, m3: u64, ref_: Option<&dyn WorldObjectRef>, player: &Player)` | Make `ref_` an `Option`-trait-object — most callsites pass `nullptr`. |
| `_completedAchievements: map<u32, CompletedAchievementData>` | `HashMap<u32, CompletedAchievementData>` | Unordered iteration is fine (TC also uses `unordered_map`). |
| `Trinity::Containers::WalkCriteriaTree(tree, lambda)` | `fn walk_criteria_tree<F>(tree_id: u32, mgr: &CriteriaMgr, mut f: F)` post-order | Generic over closure. |
| `MailDraft(...)` | Existing `wow-world::mail::MailDraft` | Reuse. |
| `Player::SetTitle(titleEntry)` | Method on `Player` in `wow-world::entities::player` | Reuse. |
| `sScriptMgr->OnAchievementCompleted(player, achievement)` | `wow_script::achievement::on_completed(player_guid, ach_id)` | Bridge through script registry. |
| `WorldPackets::Achievement::AllAchievementData::Write()` | `impl ServerPacket for AllAchievementData { fn body_to_bytes(...) }` in `wow-packet` | **Replaces the existing empty-payload stub at `crates/wow-packet/src/packets/misc.rs:912`.** |
| `SystemTimePoint` (realm-first ms) | `chrono::DateTime<Utc>` or `i64` epoch ms | Pick one and stick with it; the wire format is shifted by timezone, so keep raw UTC and shift at write-time. |
| `GuidSet` | `HashSet<ObjectGuid>` | Direct. |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| `wow-achievement` crate is 0-byte stub | ✅ | `wc -l crates/wow-achievement/src/lib.rs` = `0`; only file in crate |
| C++ canonical 4285+1365 lines | n/a | not re-counted; original audit stands |
| Login sends empty `SMSG_ALL_ACHIEVEMENT_DATA` | ✅ | `crates/wow-packet/src/packets/misc.rs:912-918` defines `pub struct AllAchievementData;` with empty payload (`ServerPacket` impl writes 0 bytes); `crates/wow-world/src/handlers/character.rs:4322-4323` sends it during login sequence (step 20). Client treats this as "you have no achievements." |
| One stub handler `handle_guild_set_achievement_tracking` | ✅ | `handlers/misc.rs:595` body = `{}`; dispatched at `session.rs:1784` |
| No `AchievementMgr` / `CriteriaMgr` / `AchievementGlobalMgr` | ✅ | `grep -rn "AchievementMgr\|CriteriaMgr\|AchievementGlobalMgr" crates/ → 0` |
| No achievement DB schema / prepared statements | ✅ | grep `character_achievement\|character_achievement_progress\|achievement_reward` in `crates/wow-database` → 0 |

**Silent-hang risk:** none. Client receives a well-formed empty packet and the achievement tab simply renders empty — no UI hang. `CMSG_QUERY_INSPECT_ACHIEVEMENTS` and other unsolicited client-side opcodes will fall through unhandled but won't block gameplay.

---

*Template version: 1.0 (2026-05-01).* When the crate gets actual content, flip Status from `❌ not started` to `⚠️ partial` and update `Last updated`.
