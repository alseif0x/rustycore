# Migration: Quests

> **C++ canonical path:** `src/server/game/Quests/` (+ `src/server/game/Pools/QuestPools.*`, `src/server/game/Handlers/QuestHandler.cpp`)
> **Rust target crate(s):** `crates/wow-data/` (templates + XP), `crates/wow-packet/src/packets/quest.rs` (wire), `crates/wow-world/src/handlers/quest.rs` (session handlers). No dedicated `wow-quest` crate yet.
> **Layer:** L6 (Game systems — depends on Entities/Player L4, Loot L6 quest items, Conditions L7, Achievements L7, Pools L7)
> **Status:** ⚠️ partial — Phase 1 + 2 complete (template loading, accept/complete/abandon, kill credit, XP via QuestXP.db2). No quest pool, no daily/weekly/monthly rotation, no escort/timed quests, no area-trigger objectives, no objective groups, no item collection objective, no repeatable cooldown logic, no quest condition checks, no faction/reputation rewards, no spell rewards, no QuestRequestItems flow.
> **Audited vs C++:** ✅ audited 2026-05-01 (§13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The Quests module is the player-progression spine of TrinityCore: it loads `quest_template` and `quest_objectives` rows from world DB into immutable `Quest` objects, exposes the lifecycle (`CanTakeQuest` → accept → kill/loot/talkto/explore credit → `CanCompleteQuest` → `RewardQuest`), persists per-character state in `character_queststatus*`, and emits/consumes ~25 quest opcodes that drive the client UI (questgiver gossip, log entries, objective trackers, reward dialogs). It also owns daily/weekly/monthly resets, quest pools (rotation), seasonal/world-event quests, and the escort/timed quest scaffolding consumed by AI scripts.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Quests/QuestDef.cpp` | 724 | `prefix` |
| `game/Quests/QuestDef.h` | 835 | `prefix` |
| `game/Quests/QuestObjectiveCriteriaMgr.cpp` | 312 | `prefix` |
| `game/Quests/QuestObjectiveCriteriaMgr.h` | 64 | `prefix` |
| `game/Quests/enuminfo_QuestDef.cpp` | 259 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Quests/QuestDef.h` | 835 | `Quest` class declaration; `QuestObjective`, `QuestObjectiveAction`, `QuestGreeting`, `QuestRewards`; **all enums** (`QuestStatus`, `QuestFlags`, `QuestFlagsEx`, `QuestFlagsEx2`, `QuestSpecialFlags`, `QuestObjectiveType`, `QuestObjectiveFlags`, `QuestFailedReason`, `QuestPushReason`, `QuestGiverStatus`, `QuestTagType`, `QuestCompleteSpellType`, `QuestTradeSkill`); `MAX_QUEST_LOG_SIZE = 25` |
| `src/server/game/Quests/QuestDef.cpp` | 724 | `Quest::LoadQuestDetails/RewardChoiceItems/RewardItems/RewardKillCredit/Mail` etc. row loaders; `BuildQuestRewardsBlock`; reward distribution helpers |
| `src/server/game/Quests/QuestObjectiveCriteriaMgr.h` | 64 | `QuestObjectiveCriteriaMgr` per-player progress tracker for `QUEST_OBJECTIVE_CRITERIA_TREE` (achievement-style criteria embedded in quests) |
| `src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp` | 312 | Criteria progress save/load to `character_queststatus_objectives_criteria*`, criteria fulfilment dispatch |
| `src/server/game/Quests/enuminfo_QuestDef.cpp` | 259 | Auto-generated enum reflection table |
| `src/server/game/Pools/QuestPools.h` | 65 | `QuestPool` struct (`poolId`, `numActive`, members vector-of-vectors, activeQuests set); `QuestPoolMgr` singleton |
| `src/server/game/Pools/QuestPools.cpp` | 293 | `LoadFromDB`, `Regenerate`, daily/weekly/monthly pool selection + persistence |
| `src/server/game/Handlers/QuestHandler.cpp` | 849 | `WorldSession::HandleQuestgiverHello/QueryQuest/AcceptQuest/CompleteQuest/ChooseReward/RequestReward/Cancel/StatusQuery/PushToParty/QueryQuestInfoOpcode/QuestPOIQuery/QuestLogRemoveQuest/QuestConfirmAccept` and reply senders |
| `src/server/game/DataStores/QuestPackets.cpp` (under WorldPackets) | n/a | Quest packet structs serializers |

Player-side touchpoints (out-of-tree):
- `src/server/game/Entities/Player/Player.cpp` (massive — `CanTakeQuest`, `CanCompleteQuest`, `RewardQuest`, `KilledMonsterCredit`, `KilledPlayerCredit`, `CastedCreatureOrGO`, `TalkedToCreature`, `MoneyChanged`, `ReputationChanged`, `AreaExploredOrEventHappens`, `GroupEventHappens`, `ItemAddedQuestCheck`, `ItemRemovedQuestCheck`, `m_QuestStatus`, `m_QuestStatusSave`, `m_RewardedQuests`, `m_DailyQuestChanged`)
- `src/server/game/Globals/ObjectMgr.cpp` (`LoadQuests`, `LoadQuestObjectives`, `LoadQuestStartersAndEnders`, `LoadQuestPOI`, `LoadQuestRelations`, `LoadQuestTemplateAddon`, `LoadQuestTemplateLocale`)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Quest` | class | Immutable post-load template — wraps one `quest_template` row + joined objectives, addons, locales, POI, mail rewards |
| `QuestObjective` | struct | One progress slot per quest (kill X, gather Y, explore Z); `Type`, `ObjectID`, `Amount`, `StorageIndex`, `Flags`, `VisualEffects`, optional `CompletionEffect` |
| `QuestObjectiveAction` | struct | Optional spell/event/conversation/phase trigger fired when an objective completes |
| `QuestGreeting` | struct | NPC greeting text + emote when player approaches with no available quest |
| `QuestTemplateLocale` | struct | Localized log title / log description / quest description / area description / portrait texts / completion log per locale |
| `QuestRequestItemsLocale` | struct | Localized completion text for "bring me X items" turn-in dialog |
| `QuestObjectivesLocale` | struct | Localized objective descriptions |
| `QuestOfferRewardLocale` | struct | Localized turn-in reward text |
| `QuestStatusData` | struct (in Player.h) | Per-character runtime quest state: `Status`, `Timer`, `Explored`, per-objective counters, `ObjectiveData[QUEST_ITEM_DROP_COUNT]` flags |
| `QuestPool` | struct | One rotation pool: `poolId`, `numActive`, `members` (groups of equivalent quests), `activeQuests` (currently-active subset) |
| `QuestPoolMgr` | singleton | Owns daily/weekly/monthly pools, regenerates on reset, persists chosen quests to `pool_quest_save` |
| `QuestObjectiveCriteriaMgr` | per-player class | Tracks achievement-criteria objectives embedded in quests |
| `QuestStatus` | enum u8 | `NONE=0`, `COMPLETE=1`, `INCOMPLETE=3`, `FAILED=5`, `REWARDED=6` (DB never stores REWARDED) |
| `QuestFlags` | enum u32 | 32 flags including `SHARABLE`, `RAID_GROUP_OK`, `DAILY=0x1000`, `WEEKLY=0x8000`, `AUTO_COMPLETE`, `AUTO_ACCEPT`, `TRACKING_EVENT`, `HIDE_REWARD`, `PLAYER_CAST_ACCEPT/COMPLETE`, `FAIL_ON_LOGOUT`, `REMOVE_SURPLUS_ITEMS` |
| `QuestFlagsEx` / `QuestFlagsEx2` | enum u32 | Extended flags (`ACCOUNT`, `LEGENDARY`, `IS_WORLD_QUEST`, `KEEP_PROGRESS_ON_FACTION_CHANGE`, periodic-reset flags) — most retail-only |
| `QuestSpecialFlags` | enum u32 | Server-side TC-only flags: `REPEATABLE=0x001`, `AUTO_PUSH_TO_PARTY=0x002`, `AUTO_ACCEPT=0x004`, `DF_QUEST=0x008`, `MONTHLY=0x010`, `SEQUENCED_OBJECTIVES=0x020` (computed) |
| `QuestObjectiveType` | enum | 21 types: `MONSTER`, `ITEM`, `GAMEOBJECT`, `TALKTO`, `CURRENCY`, `LEARNSPELL`, `MIN/MAX_REPUTATION`, `MONEY`, `PLAYERKILLS`, `AREATRIGGER`, `WINPETBATTLE…`, `CRITERIA_TREE`, `PROGRESS_BAR`, `HAVE/OBTAIN_CURRENCY`, `INCREASE_REPUTATION`, `AREA_TRIGGER_ENTER/EXIT` |
| `QuestObjectiveFlags` | enum | `TRACKED_ON_MINIMAP`, `SEQUENCED`, `OPTIONAL`, `HIDDEN`, `HIDE_CREDIT_MSG`, `PRESERVE_QUEST_ITEMS`, `PART_OF_PROGRESS_BAR`, `KILL_PLAYERS_SAME_FACTION`, `NO_SHARE_PROGRESS`, `IGNORE_SOULBOUND_ITEMS` |
| `QuestFailedReason` | enum u32 | Wire codes for SMSG_QUEST_GIVER_QUEST_FAILED (`LOW_LEVEL=1`, `WRONG_RACE=6`, `ALREADY_DONE=7`, `MISSING_ITEMS=21`, `NOT_ENOUGH_MONEY=23`, `ALREADY_DONE_DAILY=26`, `FAILED_SPELL=28`) |
| `QuestPushReason` | enum u8 | Wire codes for share-quest replies (42 reasons) |
| `QuestGiverStatus` | enum u64 (flag) | Bitfield emitted in SMSG_QUESTGIVER_STATUS (Available, Reward, Trivial, Daily, Legendary, Important, Journey, CovenantCalling, …) |
| `QuestType` | u8 column in `quest_template` | 0=AutoComplete, 1=Disabled, 2=Normal, 3=AutoCompleteFromArea, 4=Raid (TC convention) |
| `QuestRelations` | typedef | `std::multimap<uint32 /*entry*/, uint32 /*questId*/>` for creature/gameobject quest starter & ender |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Player::CanTakeQuest(Quest const*, bool sendMsgToPlayer)` | Aggregate eligibility check: level, race, class, faction, prerequisites, exclusive group, daily-already-done, log full, profession skill | `SatisfyQuestLevel/Race/Class/Skill/PreviousQuest/NextChain/PrevChain/Day/Week/Month/Seasonal/Reputation/Status/ConditionalForActiveQuests/Log/Exclusive/MinLevel/MaxLevel/BreadcrumbQuest`, `ConditionMgr::IsObjectMeetingNotGroupedConditions` |
| `Player::CanCompleteQuest(uint32 questId, ...)` | Validate every objective fulfilled and any progress-bar threshold reached | iterates `QuestObjective` list, `IsQuestObjectiveComplete`, `GetItemCount` for ITEM, currency check, reputation check |
| `Player::AddQuestAndCheckCompletion(Quest const*, Object* questGiver)` | Reserve slot in `m_QuestStatus`, set INCOMPLETE, fire `OnQuestStatusChange`, push to party if AUTO_PUSH, immediately `CompleteQuest` if zero objectives | `SetQuestStatus`, `AdjustQuestObjectiveProgress`, `SendQuestUpdate`, scripting hooks |
| `Player::RewardQuest(Quest const*, LootItemType rewardType, uint32 rewardId, Object* questGiver, bool announce)` | Give items/money/XP/reputation/honor/spell/title; mark REWARDED; persist to `character_queststatus_rewarded`; remove required items if not `EX_NO_ITEM_REMOVAL`; daily/weekly/monthly bookkeeping | `GiveXP`, `ModifyMoney`, `RewardReputation`, `RewardHonor`, `LearnQuestRewardedSpells`, `SetDailyQuestStatus`, `SetWeeklyQuestStatus`, `AddItem` |
| `Player::FailQuest(uint32 questId)` | Set FAILED state, send SMSG_QUEST_GIVER_QUEST_FAILED, optionally remove timed quest from log | `SetQuestStatus`, `SendQuestFailed`, `RemoveTimedQuest` |
| `Player::AbandonQuest(uint32 questId)` | Remove from log on player request (CMSG_QUEST_LOG_REMOVE_QUEST), drop quest items if AUTO_COMPLETE, free slot | `TakeQuestSourceItem`, `RemoveActiveQuest`, `SendQuestUpdate` |
| `Player::GiveXP(uint32 xp, Unit* victim, float groupRate)` | Apply rest, group XP rate, level-up trigger, send `SMSG_LOG_XP_GAIN` | `GiveLevel`, `RestMgr`, `Group::UpdateLooterGuid` |
| `Player::IsQuestRepeatable(Quest const*)` / `IsDailyQuest` / `IsWeeklyQuest` / `IsMonthlyQuest` / `IsSeasonalQuest` | Classify reset cadence | flag inspection |
| `Player::KilledMonsterCredit(uint32 entry, ObjectGuid guid)` | Increment `MONSTER` objectives matching `entry`; respects credit groups; pushes to party if SHARABLE | `AdjustQuestObjectiveProgress`, `SendQuestUpdate`, group iteration |
| `Player::CastedCreatureOrGO(uint32 entry, ObjectGuid guid, uint32 spell_id)` | Counterpart for kill-by-spell objectives + GO interactions | objective iteration |
| `Player::TalkedToCreature(uint32 entry, ObjectGuid guid)` | Increment TALKTO objectives | objective iteration |
| `Player::AreaExploredOrEventHappens(uint32 questId)` | Toggle ExploreRequired flag in QuestStatus | `SetQuestStatus`, `SendQuestComplete` |
| `Player::ItemAddedQuestCheck` / `ItemRemovedQuestCheck(uint32 itemId, uint32 count)` | Re-evaluate item-collection objectives on inventory change | `IncompleteQuest`/`CompleteQuest` flips |
| `Player::AdjustQuestObjectiveProgress(QuestObjective const&, int32 increment)` | Single mutator that mutates per-character objective counter, persists to `character_queststatus_objectives` | DB queue, `SendQuestUpdate` |
| `WorldSession::HandleQuestgiverHello` | Open quest list / details based on relation | `SendQuestGiverQuestList` / `SendQuestGiverQuestDetails` |
| `WorldSession::HandleQuestgiverAcceptQuest` | Validate `CanTakeQuest`, call `AddQuestAndCheckCompletion`, optionally cast accept spell | `Player::CanTakeQuest`, `Player::AddQuest` |
| `WorldSession::HandleQuestgiverChooseReward` | Validate reward index in `RewardChoiceItems`, call `RewardQuest` | `Player::CanRewardQuest`, `Player::RewardQuest` |
| `WorldSession::HandleQuestgiverRequestReward` | Re-send offer screen for completed quest | `Player::PlayerTalkClass->SendQuestGiverOfferReward` |
| `WorldSession::HandleQuestPushResult` | Group quest sharing reply | `Group::PushQuest` outcome |
| `WorldSession::HandleQuestPOIQuery` | Bulk POI fetch for tracker minimap pins | `ObjectMgr::GetQuestPOIData` |
| `QuestPoolMgr::LoadFromDB()` | Hydrate `pool_template`, `pool_quest`, `pool_quest_save` into pool members + active sets | `WorldDatabase::Query` |
| `QuestPoolMgr::ChangeDailyQuests/WeeklyQuests/MonthlyQuests` | Roll new active set, persist to `pool_quest_save` | `Regenerate` |
| `QuestPoolMgr::IsQuestActive(uint32 questId)` | Used by quest eligibility for pooled quests | _poolLookup HashMap |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Player** — `Player::m_QuestStatus`, log slot allocation, inventory/item APIs (`AddItem`, `GetItemCount`, `TakeQuestSourceItem`), money/XP/reputation appliers; `m_RewardedQuests` set; `m_DailyQuest`, `m_WeeklyQuest`, `m_MonthlyQuest`, `m_SeasonalQuests`
- **Loot** — quest-only drops (`LootItem::needs_quest`, `LootStoreItem` flag), `Loot::FillNotNormalLootFor` filters items per active quest, `LootTemplate::HasQuestDropForPlayer`
- **Conditions** (`ConditionMgr`) — `CONDITION_QUEST_TAKEN`, `CONDITION_QUEST_COMPLETE`, `CONDITION_QUEST_NONE`, `CONDITION_QUEST_OBJECTIVE_COMPLETE`; eligibility filters via `IsObjectMeetingNotGroupedConditions`
- **Pools** (`QuestPools.cpp`) — daily/weekly/monthly rotation source of truth
- **Achievements** — `CRITERIA_TYPE_COMPLETE_QUEST`, `CRITERIA_TYPE_COMPLETE_QUESTS_IN_ZONE`, `CRITERIA_TYPE_COMPLETE_QUEST_COUNT`, `CRITERIA_TYPE_COMPLETE_DAILY_QUEST`, `CRITERIA_TYPE_QUEST_REWARDED_GOLD`; `QuestObjectiveCriteriaMgr` for embedded criteria-tree objectives
- **Reputation** (`ReputationMgr`) — `RewardReputation` from `RewardFactionId[5]` + `RewardFactionValue[5]`
- **Group** — quest sharing (`AUTO_PUSH_TO_PARTY`), kill credit propagation, area-event sharing (`GroupEventHappens`)
- **DBCStores / DB2** — `QuestXP.db2` for XP table, `QuestFactionReward.db2` for rep amounts, `QuestSort.db2`, `QuestInfo.db2`, `Faction.db2`
- **ObjectMgr** — owner of all loaded `Quest*` and quest-relation maps
- **WorldPackets/Quest** — every quest opcode struct

**Depended on by:**
- **AI scripts** (`zone_*` SAI/CreatureScript) — `OnQuestAccept`, `OnQuestComplete` hooks; `Player::IsActiveQuest` queries
- **Spell system** — quest-required spell casts (`SPELL_EFFECT_QUEST_COMPLETE`, `SPELL_EFFECT_KILL_CREDIT`, `SPELL_EFFECT_KILL_CREDIT_PERSONAL`)
- **GameObject** — quest-tied containers, gossip menus filtered by `quest_template_addon.RequiredAuraSpell`
- **Mail** — `Quest::RewardMailTemplateId` triggers post-turn-in mail
- **Calendar/Events** — seasonal quests gated on `GameEvent`
- **Achievements** (above)
- **Battlegrounds** — daily BG quests (`HK_TYPE_PVP`)

---

## 6. SQL / DB queries

### World DB (templates — loaded once at startup)

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM quest_template ORDER BY ID` | Master quest table (~200 columns: title, level, faction, requirements, rewards, money, XP-difficulty index, allowed races/classes, prev/next, breadcrumb, exclusive group) | world |
| `SELECT * FROM quest_objectives ORDER BY QuestID, OrderIndex` | Per-quest objective list (Type, ObjectID, Amount, StorageIndex, Flags, ProgressBarWeight, Description) | world |
| `SELECT * FROM quest_visual_effect` | Cosmetic VFX per objective | world |
| `SELECT * FROM quest_template_addon` | TC-only extended fields: `MaxLevel`, `AllowableClasses`, `SourceSpellID`, `PrevQuestID`, `NextQuestID`, `ExclusiveGroup`, `BreadcrumbForQuestID`, `RewardMailTemplateID`, `RewardMailDelay`, `RequiredSkillID`, `RequiredSkillPoints`, `RequiredMinRepFaction`, `RequiredMaxRepFaction`, `RequiredMinRepValue`, `RequiredMaxRepValue`, `ProvidedItemCount`, `SpecialFlags`, `ScriptName` | world |
| `SELECT * FROM quest_offer_reward` | Turn-in dialog: emote slots + reward text + emote delays | world |
| `SELECT * FROM quest_request_items` | "Bring me X" dialog: completion text, emote, requested money override | world |
| `SELECT * FROM quest_template_locale` | Per-locale text overrides | world |
| `SELECT * FROM quest_objectives_locale` | Per-locale objective descriptions | world |
| `SELECT * FROM quest_offer_reward_locale` | Per-locale reward text | world |
| `SELECT * FROM quest_request_items_locale` | Per-locale completion text | world |
| `SELECT * FROM quest_greeting` / `quest_greeting_locale` | Generic NPC greeting when no quest available | world |
| `SELECT * FROM quest_poi` + `quest_poi_points` | Minimap POI polygons per objective | world |
| `SELECT * FROM quest_mail_sender` | Override mail-sender entry per quest | world |
| `SELECT * FROM quest_details` | Accept-screen emotes (4 emote/delay pairs) | world |
| `SELECT * FROM creature_queststarter`, `creature_questender`, `gameobject_queststarter`, `gameobject_questender` | NPC ↔ quest relations | world |
| `SELECT * FROM areatrigger_involvedrelation` | Areatrigger-completed quests | world |
| `SELECT * FROM pool_template`, `pool_quest`, `pool_quest_save` | Quest-pool rotation (daily/weekly/monthly) | world (save = characters since 4.x) |
| `SELECT * FROM disables WHERE sourceType = 1` | Disabled quests | world |

### Character DB (per-character runtime state)

| Statement (TC enum name) | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER_QUESTSTATUS` | `SELECT quest, status, explored, acceptTime, endTime FROM character_queststatus WHERE guid = ?` | characters |
| `CHAR_REP_CHAR_QUESTSTATUS` | `REPLACE INTO character_queststatus (guid, quest, status, explored, acceptTime, endTime) VALUES (?,?,?,?,?,?)` | characters |
| `CHAR_DEL_CHAR_QUESTSTATUS_BY_QUEST` | Drop one quest from log on abandon | characters |
| `CHAR_SEL_CHARACTER_QUESTSTATUS_OBJECTIVES` | Per-objective counters | characters |
| `CHAR_REP_CHAR_QUESTSTATUS_OBJECTIVES` | Save counter | characters |
| `CHAR_DEL_CHAR_QUESTSTATUS_OBJECTIVES_BY_QUEST` | Remove all objectives for a quest | characters |
| `CHAR_SEL_CHARACTER_QUESTSTATUS_OBJECTIVES_CRITERIA` / `_PROGRESS` | Embedded criteria-tree progress | characters |
| `CHAR_INS_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA(_PROGRESS)` | Insert criteria progress | characters |
| `CHAR_DEL_CHAR_QUESTSTATUS_OBJECTIVES_CRITERIA(_PROGRESS_BY_CRITERIA)` | Cleanup criteria | characters |
| `CHAR_SEL_CHARACTER_QUESTSTATUSREW` | `SELECT quest FROM character_queststatus_rewarded WHERE guid = ? AND active = 1` | characters |
| `CHAR_INS_CHAR_QUESTSTATUS_REWARDED` | Insert on reward | characters |
| `CHAR_DEL_CHAR_QUESTSTATUS_REWARDED_BY_QUEST` | GM/script removal | characters |
| `CHAR_UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE` / `_ACTIVE` / `_ACTIVE_BY_QUEST` | Faction change + active toggling | characters |
| `CHAR_SEL_CHARACTER_QUESTSTATUS_DAILY` / `_WEEKLY` / `_MONTHLY` / `_SEASONAL` | Reset cohorts | characters |
| `CHAR_INS_CHARACTER_QUESTSTATUS_DAILY/WEEKLY/MONTHLY/SEASONAL` | Persist on completion | characters |
| `CHAR_DEL_CHARACTER_QUESTSTATUS_DAILY/WEEKLY/MONTHLY/SEASONAL` (per-guid) | Per-character reset | characters |
| `CHAR_DEL_RESET_CHARACTER_QUESTSTATUS_DAILY/WEEKLY/MONTHLY` | Server-wide reset (cron) | characters |
| `CHAR_DEL_RESET_CHARACTER_QUESTSTATUS_SEASONAL_BY_EVENT` | End-of-season cleanup | characters |
| `CHAR_DEL_CHAR_QUESTSTATUS` / `CHAR_DEL_CHAR_QUESTSTATUS_OBJECTIVES` / `_REWARDED` (no WHERE quest) | Full purge on character delete | characters |
| `CHAR_DEL_INVALID_QUEST_PROGRESS_CRITERIA` | Cleanup orphaned criteria | characters |

### DBC/DB2 stores

| Store | What it loads | Read by |
|---|---|---|
| `QuestXPStorage` | `QuestXP.db2` (level → 10 difficulty tiers of XP) | `Quest::XPValue`, `Player::GiveQuestSourceItem`, `Player::RewardQuestPackage` |
| `QuestSortStorage` | `QuestSort.db2` (UI category) | client display only |
| `QuestInfoStorage` | `QuestInfo.db2` (Tag/Profession/PvP/Dungeon/Raid) | `Quest::IsRaidQuest`, `Quest::GetQuestTagType` |
| `QuestFactionRewardStorage` | `QuestFactionReward.db2` (rep amount per index) | `Player::RewardReputation` |
| `QuestPackageItemStorage` | `QuestPackageItem.db2` (loot-table-style choice rewards) | `Player::CanRewardQuest`, `Player::RewardQuestPackage` |
| `QuestV2CliTaskStorage` | `QuestV2CliTask.db2` (world-quest cli tasks) | retail-only, ignore for 3.4.3 |
| `QuestObjectiveStorage` | `QuestObjective.db2` (client mirror) | not server-authoritative |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_QUESTGIVER_HELLO` | client → server | `WorldSession::HandleQuestgiverHello` |
| `CMSG_QUEST_GIVER_QUERY_QUEST` | client → server | `HandleQuestgiverQueryQuest` (request details for one quest) |
| `CMSG_QUEST_GIVER_ACCEPT_QUEST` | client → server | `HandleQuestgiverAcceptQuest` |
| `CMSG_QUEST_GIVER_COMPLETE_QUEST` | client → server | `HandleQuestgiverCompleteQuest` (dialog confirm) |
| `CMSG_QUEST_GIVER_CHOOSE_REWARD` | client → server | `HandleQuestgiverChooseReward` |
| `CMSG_QUEST_GIVER_REQUEST_REWARD` | client → server | `HandleQuestgiverRequestReward` (re-open turn-in) |
| `CMSG_QUEST_GIVER_CANCEL` | client → server | `HandleQuestgiverCancel` |
| `CMSG_QUEST_GIVER_STATUS_QUERY` | client → server | `HandleQuestgiverStatusQueryOpcode` (single NPC) |
| `CMSG_QUEST_GIVER_STATUS_MULTIPLE_QUERY` | client → server | `HandleQuestgiverStatusMultipleQuery` (all visible NPCs) |
| `CMSG_QUEST_LOG_REMOVE_QUEST` | client → server | `HandleQuestLogRemoveQuest` (abandon) |
| `CMSG_QUEST_CONFIRM_ACCEPT` | client → server | `HandleQuestConfirmAccept` (group push reply) |
| `CMSG_QUEST_PUSH_RESULT` | client → server | `HandleQuestPushResult` |
| `CMSG_PUSH_QUEST_TO_PARTY` | client → server | `HandlePushQuestToParty` |
| `CMSG_QUEST_POI_QUERY` | client → server | `HandleQuestPOIQuery` (bulk minimap pins) |
| `CMSG_QUERY_QUEST_INFO` | client → server | `HandleQueryQuestInfo` (sync quest db cache) |
| `SMSG_QUEST_GIVER_QUEST_LIST_MESSAGE` | server → client | `PlayerMenu::SendQuestGiverQuestList` |
| `SMSG_QUEST_GIVER_STATUS` | server → client | `Player::SendQuestGiverStatusMultiple`, single status |
| `SMSG_QUEST_GIVER_STATUS_MULTIPLE` | server → client | bulk reply |
| `SMSG_QUEST_GIVER_QUEST_DETAILS` | server → client | accept-screen contents |
| `SMSG_QUEST_GIVER_REQUEST_ITEMS` | server → client | "bring me X" turn-in screen |
| `SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE` | server → client | reward selection screen |
| `SMSG_QUEST_GIVER_QUEST_COMPLETE` | server → client | post-reward animation/text |
| `SMSG_QUEST_GIVER_QUEST_FAILED` | server → client | `Player::SendCanTakeQuestResponse` (`QuestFailedReason`) |
| `SMSG_QUEST_GIVER_INVALID_QUEST` | server → client | invalid quest reference |
| `SMSG_QUERY_QUEST_INFO_RESPONSE` | server → client | full quest definition (cache) |
| `SMSG_QUEST_LOG_FULL` | server → client | sent when accept attempted past `MAX_QUEST_LOG_SIZE = 25` |
| `SMSG_QUEST_UPDATE_ADD_KILL` | server → client | objective progress increment (kill/cast) |
| `SMSG_QUEST_UPDATE_ADD_CREDIT` | server → client | objective progress (talk/explore) |
| `SMSG_QUEST_UPDATE_ADD_PVP_CREDIT` | server → client | PvP-kill objective |
| `SMSG_QUEST_UPDATE_COMPLETE` | server → client | objective fully done |
| `SMSG_QUEST_UPDATE_FAILED` / `_FAILED_TIMER` | server → client | failure/timed expiration |
| `SMSG_QUEST_PUSH_RESULT` | server → client | share-quest outcome |
| `SMSG_QUEST_CONFIRM_ACCEPT` | server → client | mirror to other party member |
| `SMSG_QUEST_FORCE_REMOVED` | server → client | scripted removal |
| `SMSG_QUEST_POI_QUERY_RESPONSE` | server → client | POI bulk reply |
| `SMSG_DAILY_QUESTS_RESET` | server → client | client log purge daily |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-packet/src/packets/quest.rs` | `file` | 1 | 603 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/quest.rs` | `file` | 1 | 851 | `exists_active` | file exists |
| `crates/wow-quest` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-data/src/quest.rs` | `file` | 1 | 337 | `exists_active` | file exists |
| `crates/wow-data/src/quest_xp.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/quest.rs` — 337 lines — `QuestObjective`, `QuestTemplate`, `QuestStore`, `load_quests` (templates + objectives + creature starter/ender from world DB). Has `is_repeatable`, `is_available_for(race, class, level)` helpers. Covers ~30% of `Quest` C++ surface.
- `crates/wow-data/src/quest_xp.rs` — 116 lines — `QuestXpStore` loads `QuestXP.db2`, `calculate_xp(quest_level, player_level, xp_difficulty)` mirrors `Quest::XPValue` + `RoundXPValue` (round-to-5).
- `crates/wow-packet/src/packets/quest.rs` — 603 lines — packet structs: `QuestGiverStatus`, `QuestGiverQuestList`, `QuestListEntry`, `QuestGiverQuestDetails`, `QuestRewardsBlock`, `QuestObjectiveSimple`, `QuestObjectiveInfo`, `QuestGiverOfferReward`, `QuestGiverRequestItems`, `QuestGiverQuestComplete`, `QuestUpdateComplete`, `QueryQuestInfoResponse`. Read/write only, no validation.
- `crates/wow-world/src/handlers/quest.rs` — 851 lines — registers + implements `QuestGiverStatusQuery`, `QuestGiverHello`, `QuestGiverQueryQuest`, `QuestGiverAcceptQuest`, `QuestLogRemoveQuest`, `QueryQuestInfo`, `QuestGiverRequestReward`, `QuestGiverCompleteQuest`, `QuestGiverChooseReward`. Persists basic quest accept/abandon/reward to `character_queststatus` and `character_queststatus_rewarded`. Calls `KilledMonsterCredit` from combat path.

**What's implemented (Phase 1 + 2 per release-v1 WIP):**
- Quest template + objective loading at startup
- `QuestStore` lookup by ID and by NPC starter/ender
- Race / class / level eligibility filter (`is_available_for`)
- XP table loaded from `QuestXP.db2` and applied on reward
- Accept → save row in `character_queststatus`
- Abandon (CMSG_QUEST_LOG_REMOVE_QUEST)
- Kill credit (`MONSTER` objective only) via combat hook
- Choose reward → grant items + money + XP + spawn `SMSG_QUEST_GIVER_QUEST_COMPLETE`
- Quest log full guard (`MAX_QUEST_LOG_SIZE = 25`)
- Repeatable detection (flags 0x1 / DAILY 0x4000 — note: real DAILY flag is 0x1000, see "Suspicious")
- Reward choice items (up to 6 slots)

**What's missing vs C++:**
- No `quest_template_addon` table loading → no `MaxLevel`, `AllowableClasses` ext, `PrevQuestID`/`NextQuestID`, `ExclusiveGroup`, `BreadcrumbForQuestID`, `RequiredSkillID/Points`, `RequiredMinRepFaction/Value`, `RequiredMaxRepFaction/Value`, `RewardMailTemplateID/Delay`, `SourceSpellID`, `SpecialFlags`, `ScriptName`
- No `quest_offer_reward`, `quest_request_items`, `quest_details`, `quest_greeting`, `quest_poi` loading
- No `quest_template_locale` / `quest_objectives_locale` / `quest_offer_reward_locale` / `quest_request_items_locale`
- No quest pool (no `pool_template`, `pool_quest`, `pool_quest_save` rotation; `QuestPoolMgr` equivalent missing)
- No daily / weekly / monthly / seasonal reset cycle (no `character_queststatus_daily/weekly/monthly/seasonal`, no scheduler, no SMSG_DAILY_QUESTS_RESET)
- No timed (escort) quests — no `Quest::LimitTime`, no `m_TimedQuests`, no failure on logout (`FAIL_ON_LOGOUT`)
- No area-trigger objectives (`AREATRIGGER`, `AREA_TRIGGER_ENTER/EXIT`)
- No GameObject objectives (`GAMEOBJECT`)
- No item-collection objectives (`ITEM`) — `ItemAddedQuestCheck` / `ItemRemovedQuestCheck` not wired
- No talk-to objectives (`TALKTO`)
- No currency / reputation / spell-learn / money / playerkills / progress-bar / criteria-tree objectives
- No quest objective groups / sequenced ordering (`QUEST_OBJECTIVE_FLAG_SEQUENCED`, `QUEST_SPECIAL_FLAGS_SEQUENCED_OBJECTIVES`)
- No prerequisite chain (`PrevQuestID` checked but not enforced) / breadcrumb / exclusive group / next-chain / class-quest gating
- No reputation rewards (`RewardFactionId[5]`, `RewardFactionValue[5]`)
- No honor / title / spell rewards / skill rewards / `Quest::RewardCurrencyId`
- No quest sharing (`AUTO_PUSH_TO_PARTY`, `CMSG_PUSH_QUEST_TO_PARTY`, `SMSG_QUEST_PUSH_RESULT`)
- No mail rewards (`RewardMailTemplateId`, `RewardMailDelay`)
- No quest condition checks (`ConditionMgr::CONDITION_QUEST_*`)
- No QuestObjectiveCriteriaMgr (criteria-tree progress)
- No POI / minimap tracking
- No achievements integration on quest reward
- No scripting hooks (`OnQuestAccept`, `OnQuestComplete`, `OnQuestStatusChange`)
- No `Quest::IsAutoComplete`, `IsAutoAccept`, `IsTracking`, `IsRaidQuest`
- No `SMSG_DISPLAY_QUEST_POPUP`, `SMSG_QUEST_FORCE_REMOVED`, `SMSG_QUEST_PUSH_RESULT`
- No daily-reset cron, no `WorldSession::ResetDailyQuestStatus`
- No translation/locale dispatch on quest send
- No CharacterDB stmts beyond minimal status (no `_objectives`, no `_daily/weekly/monthly`)

**Suspicious / likely divergent (pre-audit hypothesis):**
- `QuestTemplate::is_repeatable` checks `flags & 0x4000` for daily, but C++ uses `QUEST_FLAGS_DAILY = 0x1000` and `QUEST_FLAGS_WEEKLY = 0x8000`. **Bug — daily detection is wrong.** 0x4000 is `QUEST_FLAGS_DEPRECATED`.
- `allowable_races` loaded as `i64` then cast to `u64` — TC stores it as 64-bit unsigned bitmask of `Races`; on negative/MSB-set bitmasks the cast may truncate. Verify worldserver schema type.
- Race/class bit derivation uses `1 << (race-1)` — TC since 8.x uses `RaceMask` directly without -1; check 3.4.3 schema (race IDs 1..11 historically).
- `prev_quest_id` is stored on `QuestTemplate` but the `quest_template_addon` columns `PrevQuestID`/`NextQuestID`/`ExclusiveGroup`/`BreadcrumbForQuestID` are TC-only and need a separate loader; the value here is only correct if the loading SQL already JOINs the addon.
- XP rounding `((xp + 2) / 5) * 5` is "+2 then floor" — close but not identical to TC `RoundXPValue` (which special-cases ranges; check exact table boundaries).
- `calculate_xp` uses `nearest()` fallback for missing levels — TC zeros-out grey quests instead. Risk: free XP at over-cap levels.
- `KilledMonsterCredit` (in `handlers/quest.rs`) does not propagate to group members and does not respect `QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION`.
- Reward path probably writes items synchronously — no rollback on DB failure / log-full mid-grant.
- `CMSG_QUEST_PUSH_RESULT` opcode is registered but the handler likely no-ops (group-quest sharing untested).
- `loot_table` field carries quest items but `needs_quest` filter not checked at fill time.

**Tests existing:**
- 0 unit tests in `wow-data::quest` or `wow-data::quest_xp`.
- 0 integration tests for quest accept/complete/abandon flow in `wow-world`.
- Compilation + manual smoke is the only validation.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#QUESTS.WBS.001** Partir y cerrar la migracion auditada de `game/Quests/QuestDef.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`
  Rust target: `crates/wow-data`, `crates/wow-quest`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 724 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#QUESTS.WBS.002** Partir y cerrar la migracion auditada de `game/Quests/QuestDef.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`
  Rust target: `crates/wow-data`, `crates/wow-quest`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 835 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#QUESTS.WBS.003** Cerrar la migracion auditada de `game/Quests/QuestObjectiveCriteriaMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp`
  Rust target: `crates/wow-data`, `crates/wow-quest`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#QUESTS.WBS.004** Cerrar la migracion auditada de `game/Quests/QuestObjectiveCriteriaMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.h`
  Rust target: `crates/wow-data`, `crates/wow-quest`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#QUESTS.WBS.005** Cerrar la migracion auditada de `game/Quests/enuminfo_QuestDef.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/enuminfo_QuestDef.cpp`
  Rust target: `crates/wow-data`, `crates/wow-quest`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, split).

- [ ] **#QUESTS.1** Create dedicated `crates/wow-quest/` crate; move `quest.rs` + `quest_xp.rs` out of `wow-data`; expose `QuestStore`, `QuestPoolMgr`, `QuestObjectiveCriteriaMgr` (M)
- [ ] **#QUESTS.2** Fix `QuestTemplate::is_repeatable` daily/weekly flag values (`0x1000` daily, `0x8000` weekly, `QUEST_SPECIAL_FLAGS_REPEATABLE = 0x001` from addon); add `is_daily/is_weekly/is_monthly/is_seasonal/is_world_quest` (L)
- [ ] **#QUESTS.3** Load `quest_template_addon` table → extend `QuestTemplate` with `max_level`, `allowable_classes`, `prev_quest_id`, `next_quest_id`, `exclusive_group`, `breadcrumb_for_quest_id`, `required_skill_id`, `required_skill_points`, `required_min_rep_faction/value`, `required_max_rep_faction/value`, `provided_item_count`, `special_flags`, `script_name`, `reward_mail_template_id`, `reward_mail_delay` (M)
- [ ] **#QUESTS.4** Implement complete `Player::CanTakeQuest` equivalent — split into `satisfy_quest_level/race/class/skill/reputation/previous_quest/next_chain/exclusive_group/day/week/month/seasonal/log/conditions/breadcrumb/min_level/max_level` and return typed `QuestFailedReason` (H)
- [ ] **#QUESTS.5** Implement all `QuestObjectiveType` variants (currently only MONSTER): ITEM (inventory hook), GAMEOBJECT, TALKTO, CURRENCY, LEARNSPELL, MIN/MAX_REPUTATION, MONEY, PLAYERKILLS, AREATRIGGER, AREA_TRIGGER_ENTER/EXIT, PROGRESS_BAR, CRITERIA_TREE, HAVE/OBTAIN_CURRENCY, INCREASE_REPUTATION (XL → split per type)
- [ ] **#QUESTS.6** Per-objective progress storage: extend `character_queststatus_objectives (guid, quest, objective, data)`; load + persist on every `AdjustQuestObjectiveProgress` (M)
- [ ] **#QUESTS.7** Sequenced objectives: implement `QUEST_OBJECTIVE_FLAG_SEQUENCED` + computed `QUEST_SPECIAL_FLAGS_SEQUENCED_OBJECTIVES` so client only sees later objectives once earlier ones complete (M)
- [ ] **#QUESTS.8** Item-collection objectives: hook `ItemAddedQuestCheck` / `ItemRemovedQuestCheck` from inventory module; respect `QUEST_FLAGS_REMOVE_SURPLUS_ITEMS` and `QUEST_FLAGS_EX_NO_ITEM_REMOVAL` (M)
- [ ] **#QUESTS.9** Quest item drops: load `LootStoreItem::needs_quest`; hook `LootTemplate::HasQuestDropForPlayer` so quest items only drop for players with the right active quest (M, depends on #LOOT.10)
- [ ] **#QUESTS.10** Reputation rewards: load `RewardFactionId[5]` + `RewardFactionValue[5]` + `QuestFactionReward.db2`; integrate with reputation module (M)
- [ ] **#QUESTS.11** Honor / title / spell / skill rewards on `RewardQuest` (M)
- [ ] **#QUESTS.12** Mail rewards: integrate `RewardMailTemplateId` + delay with mail module (M)
- [ ] **#QUESTS.13** Daily / weekly / monthly / seasonal: add `character_queststatus_daily/weekly/monthly/seasonal`, persist on reward, enforce on `CanTakeQuest`, schedule daily reset cron (server-time 06:00 default), send `SMSG_DAILY_QUESTS_RESET` on rollover (H)
  - ✅ **Partial (2026-06-16):** the `enforce on CanTakeQuest` sliver is done. `can_take_quest` now applies the `SatisfyQuestDay`/`SatisfyQuestWeek`/`SatisfyQuestMonth` accept gates (C++ `Player::CanTakeQuest` terms, `Player.cpp:14093-14102`; bodies `SatisfyQuestDay` `Player.cpp:15393-15407`, `SatisfyQuestWeek` `Player.cpp:15409-15418`, `SatisfyQuestMonth` `Player.cpp:15445-15454`): DF quests gate on `df_quests_like_cpp`, dailies on `daily_quests_completed_like_cpp`, weeklies/monthlies on their cooldown sets — closing a re-accept hole for already-completed periodic quests. Tests: `can_take_quest_blocks_{daily,df_quest,weekly,monthly}_already_completed_like_cpp` + not-yet-completed positives. Still pending: `SatisfyQuestTimed` (no `m_timedquests` set yet — see **#QUESTS.15**), the `character_queststatus_*` reset cron, and `SMSG_DAILY_QUESTS_RESET`.
- [ ] **#QUESTS.14** `QuestPoolMgr`: load `pool_template` + `pool_quest` + `pool_quest_save`, regenerate active set on each periodic reset, gate `IsQuestActive` in `CanTakeQuest` (H)
- [ ] **#QUESTS.15** Timed (escort) quests: store `quest.LimitTime`; track `m_TimedQuests`; expire on tick or logout (`QUEST_FLAGS_FAIL_ON_LOGOUT`); send `SMSG_QUEST_UPDATE_FAILED_TIMER` (M)
- [ ] **#QUESTS.16** Area-trigger objectives: hook `Map::HandleAreaTriggerEnter/Exit`; map to `AREATRIGGER`, `AREA_TRIGGER_ENTER`, `AREA_TRIGGER_EXIT`, `AreaExploredOrEventHappens` (M)
- [ ] **#QUESTS.17** Quest sharing: handle `CMSG_PUSH_QUEST_TO_PARTY` + `CMSG_QUEST_CONFIRM_ACCEPT` + `CMSG_QUEST_PUSH_RESULT`; emit `SMSG_QUEST_PUSH_RESULT` per `QuestPushReason` (M)
- [ ] **#QUESTS.18** Group kill-credit propagation: emit credit to all group members on kill if `SHARABLE` (and not `RAID_GROUP_OK`-restricted); respect `KILL_PLAYERS_SAME_FACTION` (M)
- [ ] **#QUESTS.19** Conditions integration: gate quest accept on `CONDITION_QUEST_*` and `IsObjectMeetingNotGroupedConditions`; gate objective credit on per-objective conditions (M)
- [ ] **#QUESTS.20** `QuestObjectiveCriteriaMgr`: load + persist `character_queststatus_objectives_criteria(_progress)`, dispatch criteria fulfilment for `CRITERIA_TREE` objectives (H, depends on Achievements module)
- [ ] **#QUESTS.21** Quest POI: load `quest_poi` + `quest_poi_points`, respond to `CMSG_QUEST_POI_QUERY` with `SMSG_QUEST_POI_QUERY_RESPONSE` (L)
- [ ] **#QUESTS.22** Locales: load `quest_template_locale`, `quest_objectives_locale`, `quest_offer_reward_locale`, `quest_request_items_locale`; pick string by session locale on every quest packet send (M)
- [ ] **#QUESTS.23** Auto-complete / auto-accept (`QUEST_FLAGS_AUTO_COMPLETE`, `QUEST_FLAGS_AUTO_ACCEPT`, `QUEST_SPECIAL_FLAGS_AUTO_ACCEPT`): server-side immediate accept/complete on relevant trigger (L)
- [ ] **#QUESTS.24** Tracking quests (`QUEST_FLAGS_TRACKING_EVENT`): never write to log; auto-reward (L)
- [ ] **#QUESTS.25** `SMSG_QUEST_GIVER_STATUS_MULTIPLE` for grid-visible NPCs; bulk emit on player movement (M)
- [ ] **#QUESTS.26** Achievement criteria emission on quest reward (`CRITERIA_TYPE_COMPLETE_QUEST`, `_QUEST_COUNT`, `_DAILY_QUEST`, `_QUESTS_IN_ZONE`, `_QUEST_REWARDED_GOLD`) (M, depends on Achievements)
- [ ] **#QUESTS.27** Scripting hooks: `OnQuestAccept`, `OnQuestComplete`, `OnQuestReward`, `OnQuestStatusChange`, `OnQuestObjectiveProgress` (M)
- [ ] **#QUESTS.28** Faction-change quest survival (`QUEST_FLAGS_EX_KEEP_REPEATABLE_QUEST_ON_FACTION_CHANGE`, `_KEEP_PROGRESS_ON_FACTION_CHANGE`); rewrite quest IDs via `CHAR_UPD_CHAR_QUESTSTATUS_REWARDED_FACTION_CHANGE` (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#QUESTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 2194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#QUESTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 2194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#QUESTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 2194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#QUESTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 2194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: loading `quest_template` + `quest_objectives` → store size matches DB row count
- [ ] Test: `is_available_for(race=Human, class=Mage, level=10)` for a horde-only quest → false
- [ ] Test: `QuestXpStore::calculate_xp(level=20, player=20, difficulty=5)` matches TC `Quest::XPValue` for fixture quest 13146
- [ ] Test: XP for grey quest (player 30 levels above) returns 0, not nearest-tier
- [ ] Test: XP rounding for `XPValue=2873` returns 2875 (round-to-5)
- [ ] Test: `QuestStatus::None → Incomplete` on accept; row in `character_queststatus`
- [ ] Test: `Incomplete → Complete` only when every objective amount reached (incl. multi-objective)
- [ ] Test: abandoning a quest deletes its row from `character_queststatus` and frees a log slot
- [ ] Test: 26th quest accept rejected with `SMSG_QUEST_LOG_FULL`
- [ ] Test: kill credit increments only the matching `QUEST_OBJECTIVE_MONSTER` objective; ignores wrong entry
- [ ] Test: kill-credit groups (`spell_target_creature_kill_credit`) — N kills of any of M creatures count
- [ ] Test: item collection — adding Y of `ObjectID` flips objective to complete; removing reverts
- [ ] Test: sequenced objectives — second objective hidden until first reaches Amount
- [ ] Test: daily quest cannot be re-accepted before reset; can after `ResetDailyQuestStatus`
- [ ] Test: timed quest fails on timer expiration; player log slot freed
- [ ] Test: quest with `RewardChoiceItems` honors player's chosen index; rejects invalid index
- [ ] Test: `SatisfyPreviousQuest` rejects if `PrevQuestID > 0` and player not rewarded
- [ ] Test: `ExclusiveGroup` — accepting any quest in group locks all others
- [ ] Test: race/class/min/max level boundary cases (lvl 79 cannot take 80-min quest, lvl 80 can)
- [ ] Test: `QuestPool::Regenerate` picks `numActive` distinct members; subset persists to `pool_quest_save`
- [ ] Test: rewarded quest writes `character_queststatus_rewarded` and removes from `character_queststatus`
- [ ] Test: quest-required loot drops only for players with active matching quest objective
- [ ] Test: `SatisfyQuestStatus` rejects already-rewarded non-repeatable quest
- [ ] Test: `QuestGiverStatus::DailyQuest` flag emitted on NPC visible status when daily available
- [ ] Test: faction-change rewrites Alliance↔Horde quest IDs in `character_queststatus_rewarded` per `QUEST_FLAGS_EX_KEEP_PROGRESS_ON_FACTION_CHANGE`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 5 files / 2194 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp` | `crates/wow-data/` (templates + XP), `crates/wow-packet/src/packets/quest.rs` (wire), `crates/wow-world/src/handlers/quest.rs` (session handlers). No dedicated `wow-quest` crate yet. \| ⚠️ partial — Phase 1 + 2 complete (template loading, accept/complete/abandon, kill credit, XP via QuestXP.db2). No quest pool, no daily/weekly/monthly rotation, no escort/timed quests, no area-trigger objectives, no objective groups, no item collection objective, no repeatable cooldown logic, no quest condition checks, no faction/reputation rewards, no spell rewards, no QuestRequestItems flow. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#QUESTS.DIV.001` | `crates/wow-quest` (`missing_declared_path`, 0 Rust lines) | 5 C++ files / 2194 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.h`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestDef.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Quests/QuestObjectiveCriteriaMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`MAX_QUEST_LOG_SIZE = 25`** is wire-protocol-baked — sending more breaks the client UI; never increase without confirming client patch.
- **DAILY flag is `0x1000`, NOT `0x4000`.** `0x4000 = QUEST_FLAGS_DEPRECATED`. Current Rust check is wrong (#QUESTS.2). WEEKLY = `0x8000`. Repeatable comes from `quest_template_addon.SpecialFlags & 0x001`, not the regular flags.
- **Quest-objective groups vs sequencing.** TC uses `OrderIndex` + `QUEST_OBJECTIVE_FLAG_SEQUENCED` to gate visibility, plus computed `QUEST_SPECIAL_FLAGS_SEQUENCED_OBJECTIVES` to mark the whole quest as sequenced. Items obtained out-of-order on a sequenced quest must still be queued, not rejected.
- **`StorageIndex = -1`** means the objective contributes to the progress bar only and has no per-objective slot in `character_queststatus_objectives.data`. Keep this signed.
- **Quest items vs reward items.** `Quest.RewardItem[4]` are auto-granted; `Quest.RewardChoiceItems[6]` require client picks one. Quest source items (loot drops with `needs_quest`) are *removed* on turn-in unless `QUEST_FLAGS_EX_NO_ITEM_REMOVAL` is set. `QUEST_FLAGS_REMOVE_SURPLUS_ITEMS` removes *all* matching items, not just `Amount`.
- **Faction-specific quests.** Two patterns: (a) `AllowableRaces` bitmask filters by race → faction; (b) twin Alliance/Horde quests with mirroring `quest_template_addon.AllowableClasses` and a script-set `ExclusiveGroup`. Never assume one canonical "Alliance version".
- **`QuestStatus::REWARDED = 6`** is server-only — never written to `character_queststatus`. Rewarded quests live in `character_queststatus_rewarded` (with `active=1`); the row is moved (delete from queststatus, insert into rewarded) atomically on `RewardQuest`.
- **Repeatable types.** Daily resets at server `quest.daily.reset.time` (default 06:00 server time). Weekly resets Wednesday 03:00. Monthly resets first day of month 03:00. Seasonal resets when the gating `GameEvent` ends. Each has its own `character_queststatus_*` table — never reuse `character_queststatus`.
- **`CanTakeQuest` chains 14+ predicates** in TC. Fail-fast order matters because `sendMsgToPlayer` sends only the first failure reason. Mirror order: status → log full → conditions → reputation → skill → race → class → level (min) → level (max) → previous quest → next chain → exclusive group → breadcrumb → daily/weekly cooldown.
- **Class quests** (`QUEST_TRSKILL_*`) are quests with `RequiredSkillID` that *also* gate by class; both must match.
- **`Quest::LimitTime`** is in seconds, but the client wire field is `EndTime` absolute timestamp. Compute `endTime = acceptTime + limitTime` and persist; on load if `now > endTime` mark FAILED before sending the log.
- **Group event happens** (`GroupEventHappens`) propagates `AreaExploredOrEventHappens` to nearby party members, not `KilledMonsterCredit` — kill credit uses its own group-iteration path with raid-OK gating.
- **`character_queststatus.explored = 1`** is separate from objective completion; it tracks `QUEST_OBJECTIVE_AREATRIGGER` / `AreaExploredOrEventHappens` via a single boolean even when there are multiple area triggers (TC simplifies to "any area trigger satisfied").
- **`AUTO_PUSH_TO_PARTY`** triggers a `CMSG_QUEST_CONFIRM_ACCEPT` round-trip per party member — not a unilateral push. Decline must not corrupt the originator's quest state.
- **POI loading is huge** (~50k rows in retail world DB). Use a sparse `HashMap<questId, Vec<Poi>>` with lazy materialization on `CMSG_QUEST_POI_QUERY`.
- **Reward XP at max level** is converted to gold per `Player::ConvertQuestRewardedXPToMoney` unless `QUEST_FLAGS_NO_MONEY_FOR_XP`. Conversion rate from `BattlemasterEntries`-adjacent table; in 3.4.3 use the hardcoded `xp_to_gold` ratio.
- **WotLK-specific:** no `QUEST_FLAGS_EX2_*` should be required; they are 8.x+ flags. Loading the column is fine, but no enforcement should depend on them.
- **`QUEST_FLAGS_TRACKING_EVENT`** quests are auto-rewarded on completion *and never appear in the log*. This is the mechanism for "kill 5000 of X for an achievement-only reward".

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Quest` | `struct QuestTemplate` (`crates/wow-data/src/quest.rs`) | Should move to `wow-quest`; immutable post-load |
| `struct QuestObjective` | `struct QuestObjective` (same file) | One-to-one |
| `struct QuestStatusData` (per-player) | TBD — currently inlined in session/handler | Needs `PlayerQuestState { active: HashMap<u32, QuestStatusEntry>, rewarded: HashSet<u32>, daily/weekly/monthly: HashSet<u32> }` |
| `class QuestPoolMgr` (singleton) | `struct QuestPoolMgr` global in `wow-quest` | Use `OnceLock<RwLock<…>>` not raw singleton |
| `class QuestObjectiveCriteriaMgr` (per-player) | per-player struct under `wow-quest::criteria_mgr` | One per Player |
| `enum QuestStatus : uint8` | `#[repr(u8)] enum QuestStatus { None=0, Complete=1, Incomplete=3, Failed=5, Rewarded=6 }` | Skip 2 and 4 (legacy holes) |
| `EnumFlag<QuestGiverStatus>` (uint64) | `bitflags! struct QuestGiverStatus: u64` | Use `bitflags` crate |
| `std::multimap<uint32, uint32>` (relations) | `HashMap<u32, Vec<u32>>` | Already used in `QuestStore` |
| `Player::CanTakeQuest(...)` | `fn can_take_quest(player: &PlayerCtx, quest: &QuestTemplate) -> Result<(), QuestFailedReason>` | Return typed reason instead of bool + side-channel |
| `Player::RewardQuest(...)` | `async fn reward_quest(session: &mut WorldSession, quest: &QuestTemplate, choice: Option<u8>) -> anyhow::Result<()>` | Async because of DB writes |
| `Player::KilledMonsterCredit` | `fn killed_monster_credit(state: &mut PlayerQuestState, entry: u32, group_members: &[…]) -> Vec<QuestUpdate>` | Pure function returning side-effects to apply |
| `QuestXPRecord (DB2)` | `struct QuestXpRow` (`quest_xp.rs`) | Already exists |
| `WorldPackets::Quest::QuestRewards` | `struct QuestRewardsBlock` (`wow-packet`) | Already exists |
| `Optional<QuestObjectiveAction>` | `Option<QuestObjectiveAction>` | — |
| `LootItem::needs_quest` | `LootEntry { needs_quest: bool, .. }` | Wire to `QuestStore::has_active_quest_for_objective` |

---

*Generated: 2026-05-01. Mark `Audited vs C++ : ✅` only after a side-by-side line-by-line audit of `Quest::CanTakeQuest`/`RewardQuest` against the Rust port.*

---

## 13. Audit (2026-05-01)

Side-by-side audit of `crates/wow-data/src/quest.rs` + `crates/wow-world/src/handlers/quest.rs` + `crates/wow-world/src/session.rs::on_creature_killed` vs `src/server/game/Quests/QuestDef.{h,cpp}` + `Player::KilledMonsterCredit`.

### Flagged divergences — verdicts

**1. `is_repeatable` daily flag bug — CONFIRMED.**
`crates/wow-data/src/quest.rs:101-104` — `flags & 0x4000` is checked. C++ `QuestDef.h:208` defines `QUEST_FLAGS_DEPRECATED = 0x00004000` and `QuestDef.h:206` `QUEST_FLAGS_DAILY = 0x00001000`. The Rust check is therefore selecting deprecated quests as "repeatable" and missing every actual daily. Trinity also routes true repeatable status through `_specialFlags & QUEST_SPECIAL_FLAGS_REPEATABLE = 0x001` (`QuestDef.h:298`, `IsRepeatable()` at `QuestDef.h:643`) — that field is loaded from `quest_template_addon.SpecialFlags`, which the Rust loader does not read at all. Both halves of the check are wrong.

**2. Kill credit fires on creature death — CONFIRMED working (doc was conservative).**
The doc says "Kill credit (`MONSTER` objective only) via combat hook"; tracing the call chain confirms it: `crates/wow-world/src/session.rs:2959` invokes `on_creature_killed(entry, guid)` from the kill-resolution branch, and `session.rs:753-829` walks `player_quests`, matches `obj.obj_type == OBJ_TYPE_MONSTER (0)` against `obj.object_id == creature_entry`, increments `objective_counts[storage_index]`, sends `SMSG_QUEST_UPDATE_ADD_CREDIT`, and on full completion sends `SMSG_QUEST_UPDATE_COMPLETE` and flips `qs.status = 2` in memory. So basic single-player kill credit works end-to-end. Gaps vs C++ `Player::KilledMonsterCredit` (Trinity `Player.cpp`): no group propagation (group members on the same map don't get credit), no `KillCreditId[2]` proxy entries (Trinity reads two extra `creature_template.KillCredit1/2` columns and credits those entries too — note `character.rs:2251-2336` already loads them as `proxy_creature_ids`, but `on_creature_killed` only matches the primary entry), no `QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION`, no spell-cast credit (`SPELL_EFFECT_KILL_CREDIT_PERSONAL`), and the new state is never persisted to `character_queststatus_objectives` — restart wipes mid-quest progress.

### Quest objective type coverage

| C++ `QuestObjectiveType` | id | Rust impl |
|---|---|---|
| MONSTER | 0 | ✅ `session.rs:764` (`OBJ_TYPE_MONSTER`) |
| ITEM | 1 | ❌ no inventory hook |
| GAMEOBJECT | 2 | ❌ |
| TALKTO | 3 | ❌ |
| CURRENCY | 4 | ❌ |
| LEARNSPELL | 5 | ❌ |
| MIN_REPUTATION | 6 | ❌ |
| MAX_REPUTATION | 7 | ❌ |
| MONEY | 8 | ❌ |
| PLAYERKILLS | 9 | ❌ |
| AREATRIGGER | 10 | ❌ |
| WINPETBATTLE / CRITERIA_TREE / PROGRESS_BAR / HAVE/OBTAIN_CURRENCY / INCREASE_REPUTATION / AREA_TRIGGER_ENTER/EXIT | 11+ | ❌ |

**1 of ~21** objective types implemented.

### Quest opcode handler coverage (`handlers/quest.rs`)

`QuestGiverStatusQuery` ✅, `QuestGiverHello` ✅, `QuestGiverQueryQuest` ✅, `QuestGiverAcceptQuest` ✅, `QuestLogRemoveQuest` ✅, `QueryQuestInfo` ✅, `QuestGiverRequestReward` ✅, `QuestGiverCompleteQuest` ✅, `QuestGiverChooseReward` ✅. Missing: `QuestGiverStatusMultipleQuery`, `QuestGiverCancel`, `QuestConfirmAccept`, `QuestPushResult`, `PushQuestToParty`, `QuestPOIQuery`. The dispatcher at `handlers/quest.rs:120-548` covers ~9 of 14 client→server quest opcodes.

### Other observed bugs

- `quest.rs:111` — `1u64 << (race.saturating_sub(1))`. `race=0` produces `1u64 << 0xFF` which is UB in debug and wraps to 0 in release. C++ uses `1 << (race-1)` only for race ids 1..11 — should clamp/validate.
- `quest.rs:251` — `result.try_read::<i64>(35).map(|v| v as u64)`. If the column is `BIGINT UNSIGNED` with the high bit set this is fine, but a negative legacy value would silently wrap. The TC schema is `bigint(20) unsigned`.
- `quest_template_addon` is never queried, so `prev_quest_id` (`quest.rs:93`) is wired in but never populated; chain prerequisites cannot work.

**Verdict:** quest module is ~30% of C++. The flagged daily-flag bug is real (`0x4000` vs `0x1000`); kill credit *does* fire and updates progress correctly for the single MONSTER objective type, but stops short of group propagation, persistence, and the other 20 objective types.
