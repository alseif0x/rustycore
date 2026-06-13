# Migration: Loot

> **C++ canonical path:** `src/server/game/Loot/` (+ `src/server/game/Handlers/LootHandler.cpp`)
> **Rust target crate(s):** `crates/wow-loot/` (foundation started: `LootStoreItem`, `LootTemplate`, `LootGroup`, `LootStore`), `crates/wow-packet/src/packets/loot.rs` (wire), `crates/wow-world/src/handlers/loot.rs` (session handlers)
> **Layer:** L6 (Game systems — depends on Entities/Creature/GameObject L4, Group L6, Conditions L7, Items L4, Player inventory L4)
> **Status:** 🔧 broken (rewrite needed) — `wow-loot` now has a contrasted `LootStoreItem`/`LootTemplate`/`LootGroup`/`LootStore` foundation, `world-server` loads the 12 C++ `LootTemplates_*` store foundations at startup, C++ `LootTemplates_Reference` plus structural loot-condition link diagnostics are ported as non-fatal startup checks, and a pure `Loot::FillLoot`/`LootTemplate::Process` foundation with basic `LootItem` constructor metadata exists in `wow-loot`. Full reusable `ConditionMgr` storage/evaluation, real `ItemRandomProperties` object shape, creature/gameobject loot ownership, money, quest loot, mail/spell/gameobject/fishing/skinning/pickpocketing/milling/prospecting runtime wiring and full group/master loot engine are still incomplete.
> **Audited vs C++:** ✅ audited 2026-05-01 (§13)
> **Last updated:** 2026-06-13

---

## 1. Purpose

The Loot module turns dead creatures, opened gameobjects, fishing nodes, gathering nodes, pickpocket targets, disenchanted items, milled herbs, prospected ores, mail attachments, and spell-spawned items into player-visible item pools. It owns the random-roll engine (`LootStoreItem::Roll`) that consumes drop chances, item references (chained pools), and conditions (`ConditionMgr`); the per-corpse `Loot` lifecycle (open → display → group roll → assign → release); the wire protocol that drives the loot UI; and the persistence of unlooted gameobject/item containers (`LootItemStorage`). Without it, mobs drop nothing and the game has no economy.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Loot/Loot.cpp` | 1083 | `prefix` |
| `game/Loot/Loot.h` | 378 | `prefix` |
| `game/Loot/LootItemStorage.cpp` | 362 | `prefix` |
| `game/Loot/LootItemStorage.h` | 95 | `prefix` |
| `game/Loot/LootItemType.h` | 29 | `prefix` |
| `game/Loot/LootMgr.cpp` | 1350 | `prefix` |
| `game/Loot/LootMgr.h` | 178 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Loot/Loot.h` | 378 | `Loot` struct, `LootItem`, `LootItemList`, `LootRoll` (group roll engine), `AELootResult` (auto-loot accumulator), enums (`LootMethod`, `LootType`, `RollType`, `RollVote`, `RollMask`, `LootError`, `LootSlotType`, `LootRollIneligibilityReason`); constants `LOOT_ROLL_TIMEOUT = 1min`, `MAX_NR_LOOT_ITEMS = 18` |
| `src/server/game/Loot/Loot.cpp` | 1083 | `Loot::FillLoot`, `Loot::AddItem`, `Loot::generateMoneyLoot`, `Loot::AutoStore`, `Loot::BuildLootResponse`, `LootItem::AllowedForPlayer`, `LootItem::GetUiTypeForPlayer`, `LootRoll::TryToStart/PlayerVote/UpdateRoll/Finish`, group-vs-personal loot decision tree |
| `src/server/game/Loot/LootMgr.h` | 178 | `LootStoreItem`, `LootStore`, `LootTemplate` (with private `LootGroup`); 12 global `LootStore` instances (Creature, Disenchant, Fishing, Gameobject, Item, Mail, Milling, Pickpocketing, Prospecting, Reference, Skinning, Spell); `GenerateDungeonEncounterPersonalLoot` |
| `src/server/game/Loot/LootMgr.cpp` | 1350 | `LootStore::LoadAndCollectLootIds` (master loader), per-store `LoadLootTemplates_*` functions, `LootTemplate::Process` (random draw), `LootGroup::HasQuestDropForPlayer`, `LootGroup::Roll`, ref-template resolution, `Verify`/`CheckLootRefs`, hot-fix re-load entry points |
| `src/server/game/Loot/LootItemStorage.h` | 95 | `LootItemStorage` singleton — persists in-progress (uncommitted) lootable item containers across server restarts (the bag/quiver/etc. that has live loot rolling) |
| `src/server/game/Loot/LootItemStorage.cpp` | 362 | DB load/save of `item_loot_storage` per-character; `AddNewStoredLoot`, `RemoveStoredLootForContainer`, `RemoveStoredLootItemForContainer`, `LoadStoredLoot` |
| `src/server/game/Loot/LootItemType.h` | 29 | `enum class LootItemType` — `Item`, `Currency` (used in `LootItem.context`) |
| `src/server/game/Handlers/LootHandler.cpp` | 508 | `WorldSession::HandleAutostoreLootItemOpcode`, `HandleLootMoneyOpcode`, `HandleLootOpcode` (CMSG_LOOT_UNIT), `HandleLootReleaseOpcode`, `HandleLootMasterGiveOpcode`, `HandleLootRoll`, `HandleSetLootSpecialization`, plus reply senders `SendLootError`, `SendLootRelease`, `DoLootRelease` |

Out-of-tree touchpoints:
- `src/server/game/Entities/Creature/Creature.cpp` — `Creature::IsLootRecipient`, `m_loot`, `m_personalLoot`, `HasLootRecipient`, `AllLootRemovedFromCorpse`, `GenerateLootForBody` (calls `Loot::FillLoot` with `LootTemplates_Creature`)
- `src/server/game/Entities/GameObject/GameObject.cpp` — `m_loot`, `Use` for chests/herbs/ore/fishing nodes
- `src/server/game/Entities/Player/Player.cpp` — `StoreLootItem`, `SendLoot`, `SendLootError`, `SendLootRelease`, `SendNotifyLootMoneyRemoved`, `SendNotifyLootItemRemoved`, `m_lootRolls`
- `src/server/game/Groups/Group.cpp` — `GetLootMethod`, `SendLootStartRoll`, `CountRollVote`, `EndRoll`, `Player::OnGameObjectLooted` group permission checks, master-loot list selection
- `src/server/game/Conditions/ConditionMgr.cpp` — `ConditionsReference` resolution per `LootStoreItem`

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Loot` | struct | One per lootable corpse/object: `items` vector, `gold`, `unlootedCount`, `roundRobinPlayer`, `loot_type`, `_lootMethod`, `_lootMaster`, `_allowedLooters`, `_rolls` map, `_dungeonEncounterId`. Owns the per-player FFA item map |
| `LootItem` | struct | Materialized drop: `itemid`, `LootListId`, `randomProperties` (suffix/random property), `context`, `count`, `is_looted`, `is_blocked` (rolling), `freeforall`, `is_underthreshold`, `is_counted`, `needs_quest`, `follow_loot_rules`, `allowedGUIDs`, `rollWinnerGUID`, `conditions`. Built from a `LootStoreItem` at draw time |
| `NotNormalLootItem` | struct | Per-player "FFA" or "personal" item view-stub (just `LootListId` + `is_looted`) for items that aren't shared |
| `LootStoreItem` | struct | One row from a `*_loot_template` table: `itemid`, `reference`, `chance`, `lootmode` (16-bit difficulty mask), `needs_quest`, `groupid`, `mincount`, `maxcount`, `conditions` |
| `LootStore` | class | One per loot table type (12 globals). Wraps `m_LootTemplates: unordered_map<uint32, LootTemplate*>`, name + entryName, ratesAllowed flag |
| `LootTemplate` | class | Per-loot-id collection: `LootStoreItemList Entries` (ungrouped) + `LootGroups Groups` (grouped, mutually exclusive). Methods: `Process`, `ProcessPersonalLoot`, `HasDropForPlayer`, `HasQuestDrop`, `HasQuestDropForPlayer` |
| `LootGroup` (private nested in `LootTemplate`) | class | A "mutually exclusive" set: server picks at most one entry by weighted chance. Used for `groupid > 0` rows |
| `LootMode` | uint16 (mask) | Difficulty mask: `LOOT_MODE_DEFAULT = 0x1`, normal/heroic/mythic/raid-difficulty bits 0x2/0x4/0x8/0x10... up to 16 modes. Drop only if `lootmode & active_mode != 0` |
| `LootMethod` | enum u8 | `FREE_FOR_ALL=0`, `ROUND_ROBIN=1`, `MASTER_LOOT=2`, `GROUP_LOOT=3`, `NEED_BEFORE_GREED=4`, `PERSONAL_LOOT=5` |
| `LootType` | enum u8 | `CORPSE=1`, `PICKPOCKETING=2`, `FISHING=3`, `DISENCHANTING=4`, `ITEM=5`, `SKINNING=6`, `GATHERING_NODE=8`, `CHEST=9`, `CORPSE_PERSONAL=14`, `FISHINGHOLE=20`, `INSIGNIA=21`, `FISHING_JUNK=22`, `PROSPECTING=23`, `MILLING=24`. `GetLootTypeForClient` collapses TC-internal types to client-known ones |
| `LootRoll` | class | Group roll engine for one item: `m_rollVoteMap` (player → `PlayerRollVote{Vote, RollNumber}`), `m_voteMask` (allowed roll types), `m_endTime`. Methods: `TryToStart`, `PlayerVote`, `UpdateRoll` (timeout tick), `Finish`, `AllPlayerVoted`. Lives inside `Loot::_rolls` |
| `RollType` | enum | `PASS=0`, `NEED=1`, `GREED=2`, `DISENCHANT=3`, `TRANSMOG=4` |
| `RollVote` | enum class | Strongly-typed mirror including `NotEmitedYet`, `NotValid` |
| `RollMask` | enum | Allowed-vote bitfield computed from `voteMask = NEED if usable + GREED + DISENCHANT if enchanting skill ≥ required` |
| `LootSlotType` | enum | UI marker: `ALLOW_LOOT=0`, `ROLL_ONGOING=1`, `LOCKED=2`, `MASTER=3`, `OWNER=4` |
| `LootError` | enum u8 | Failure codes for SMSG_LOOT_RESPONSE: `DIDNT_KILL=0`, `TOO_FAR=4`, `BAD_FACING=5`, `LOCKED=6`, `NOTSTANDING=8`, `STUNNED=9`, `MASTER_INV_FULL=12`, `MASTER_UNIQUE_ITEM=13`, `ALREADY_PICKPOCKETED=15`, `NO_LOOT=17` |
| `LootRollIneligibilityReason` | enum class u32 | NEED-roll rejection: `UnusableByClass`, `MaxUniqueItemCount`, `CannotBeDisenchanted`, `EnchantingSkillTooLow`, `NeedDisabled`, `OwnBetterItem` |
| `AELootResult` | class | Accumulator for area-effect (auto) loot — multiple corpses opened at once produce a single ordered result with item dedup |
| `LootItemStorage` | singleton | Persists in-progress containers (mining nodes, herb spots, chest GO, item-loot containers) across restart |
| `LootItemType` | enum class u8 | `Item=0`, `Currency=1` |
| `ItemContext` | enum class u8 | Context flags carried from drop → item creation (used for retail bonus IDs; ignored on 3.4.3 server but preserved on wire) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Loot::FillLoot(uint32 lootId, LootStore const& store, Player* lootOwner, bool personal, bool noEmptyError, uint16 lootMode, ItemContext context)` | Generate items by drawing from `LootTemplate->Process`; fill `items[]`; apply `lootMode` mask; auto-add quest items if `needs_quest` matches active quest objective | `LootTemplate::Process`, `LootStore::GetLootFor`, `LootItem` ctor (random suffix/property roll), `Loot::AddItem` |
| `Loot::generateMoneyLoot(uint32 minAmount, uint32 maxAmount)` | Roll uniform copper amount; apply rate config (`Rate.Drop.Money`) | `urand`, `sWorld->getRate` |
| `Loot::AddItem(LootStoreItem const& item)` | Roll mincount..maxcount; assign `LootListId`; copy conditions; respect `MAX_NR_LOOT_ITEMS = 18` cap | `urand`, `Item::GenerateItemRandomPropertyId` |
| `Loot::AutoStore(Player*, uint8 bag, uint8 slot, bool broadcast, bool createdByPlayer)` | Auto-loot path: iterate items and try `Player::StoreNewItem`; call `LootItemPickup` per success | `Player::CanStoreNewItem`, `Player::StoreNewItem`, group event |
| `Loot::LootMoney()` | Distribute `gold` across allowed lootees; emits `SMSG_LOOT_MONEY_NOTIFY` to round-robin player & group | `Player::ModifyMoney`, `Group::SendLooter`, money split per `Group::GetMembersCount` |
| `Loot::LootItemInSlot(uint32 lootListId, Player const* player, NotNormalLootItem** ffaItem)` | Resolve "this player picks slot N" — handles FFA item personal slot, master loot owner check, threshold permission | `LootItem::AllowedForPlayer`, `LootItem::HasAllowedLooter` |
| `Loot::BuildLootResponse(WorldPackets::Loot::LootResponse&, Player const*)` | Emit per-player view of the loot window (different items visible to different players for FFA / quest items / master loot) | `LootItem::GetUiTypeForPlayer`, group permission |
| `Loot::Update()` | Tick — drives `LootRoll::UpdateRoll` for each pending roll | iterates `_rolls` |
| `LootItem::AllowedForPlayer(Player const*, Loot const*)` | Multi-rule check: class restriction (`item_template.AllowableClass`), race, quest requirement, condition fulfilment, faction; static overload reused for "would-have-dropped" preview | `Player::CanRollNeedOnItem`, `ConditionMgr::IsObjectMeetingNotGroupedConditions`, `Player::HasQuest` |
| `LootItem::GetUiTypeForPlayer(Player const*, Loot const&) -> Optional<LootSlotType>` | Decide if the slot shows as `ALLOW_LOOT` / `ROLL_ONGOING` / `MASTER` / `LOCKED` / `OWNER` (or absent) for a given viewer | `Loot::GetLootMethod`, master GUID, `is_blocked`, `is_counted` |
| `LootStoreItem::Roll(bool rate)` | Random draw against `chance` (with global rate multiplier) | `urand`, config rate |
| `LootStore::LoadAndCollectLootIds(LootIdSet& ids_set)` | Master query of `<table_name>` rows → `LootStoreItem` → grouped/ungrouped insertion into `LootTemplate` | `WorldDatabase::Query`, `LootStoreItem::IsValid`, `LootTemplate::AddEntry` |
| `LootStore::CheckLootRefs(LootIdSet*)` | Walk every template and recursively verify `reference > 0` rows resolve to existing `LootTemplates_Reference` ids | `LootTemplate::CheckLootRefs` |
| `LootTemplate::Process(Loot& loot, bool rate, uint16 lootMode, uint8 groupId, Player const* personalLooter)` | Walk ungrouped + grouped entries; for refs recursively `Process` the referenced template | `LootStoreItem::Roll`, `LootGroup::Process`, recursion |
| `LootTemplate::ProcessPersonalLoot(...)` | Personal-loot variant: produce one `Loot*` per player whose drop list passed | per-player iteration |
| `LootTemplate::HasQuestDropForPlayer(...)` / `HasQuestDrop(...)` | True if any entry has `needs_quest` matching player's active quest (or any active quest with a matching ITEM objective) | `Quest::IsRequiredItemForObjective` |
| `LootTemplate::HasDropForPlayer(...)` | Used to suppress "this corpse has no loot" message when conditional loot exists | `LootStoreItem` walk |
| `LootTemplate::CopyConditions(LootItem*)` | Replicate conditions on a freshly-rolled item | — |
| `LootRoll::TryToStart(Map*, Loot&, uint32 lootListId, uint16 enchantingSkill)` | Decide if an item must be rolled (above-threshold + group loot method); compute `voteMask` (NEED based on usability; DISENCHANT based on skill); broadcast `SMSG_LOOT_START_ROLL` | `Group::isLootMethod*`, `LootItem::AllowedForPlayer`, `ItemDisenchantLootEntry` lookup |
| `LootRoll::PlayerVote(Player*, RollVote)` | Record vote; if all eligible voted → `Finish` | timestamp tick |
| `LootRoll::UpdateRoll()` | On `m_endTime` reached, treat unvoted as PASS, call `Finish` | timeout tick |
| `LootRoll::Finish(...)` | Determine winner (highest roll within winning vote category Need > Greed > Disenchant); emit `SMSG_LOOT_ROLL_WON`, mark `LootItem::is_looted` if winner is set, deliver item or auto-disenchant | `Player::AutoStoreLoot` |
| `Group::GroupLoot(uint64 playerGUID, Loot* loot, WorldObject* lootedObject)` | For each `is_blocked` item above threshold call `LootRoll::TryToStart` | calls roll engine |
| `Group::MasterLoot(...)` | For master-loot mode, emit `SMSG_MASTER_LOOT_CANDIDATE_LIST` and wait for `CMSG_LOOT_MASTER_GIVE` | candidate list, `_lootMaster` permission |
| `Group::NeedBeforeGreed(...)` | Like GroupLoot but only allow NEED if class can use the item | `Player::CanRollNeedOnItem` |
| `WorldSession::HandleLootOpcode(CMSG_LOOT_UNIT)` | Validate target is a dead, in-range creature the player may loot, generate `Loot` lazily, emit `SMSG_LOOT_RESPONSE` | `AELootCreatureCheck::IsValidLootTarget`, `Creature::IsLootRecipient`, `Loot::FillLoot`, `Loot::BuildLootResponse` |
| `WorldSession::HandleAutostoreLootItemOpcode(CMSG_LOOT_ITEM)` | Per-slot pickup: validate via `Loot::LootItemInSlot`, call `Player::StoreLootItem` | `Player::StoreLootItem`, `Loot::NotifyItemRemoved` |
| `WorldSession::HandleLootMoneyOpcode(CMSG_LOOT_MONEY)` | Trigger `Loot::LootMoney` | money distribution |
| `WorldSession::HandleLootMasterGiveOpcode(CMSG_LOOT_MASTER_GIVE)` | Master gives item to specific GUID; permission + uniqueness checks | master loot path |
| `WorldSession::HandleLootRoll(CMSG_LOOT_ROLL)` | Vote on a pending roll | `LootRoll::PlayerVote` |
| `WorldSession::HandleLootReleaseOpcode(CMSG_LOOT_RELEASE)` | Close window; if fully looted call `Creature::AllLootRemovedFromCorpse` to start corpse decay | `Loot::isLooted`, creature flag clear |
| `WorldSession::HandleSetLootMethod(CMSG_SET_LOOT_METHOD)` | Group leader changes loot method/threshold/master | `Group::SetLootMethod` |
| `WorldSession::HandleSetLootSpecialization(CMSG_SET_LOOT_SPECIALIZATION)` | Select loot specialization if `ChrSpecialization.ClassID` matches player class; `0` clears | `sChrSpecializationStore`, `Player::SetLootSpecId` |
| `LootItemStorage::AddNewStoredLoot(Loot*, Player*)` | Persist live container loot | DB write |
| `LootItemStorage::LoadStoredLoot(Item*, Player*)` | Restore on item open after restart | DB read |

---

## 5. Module dependencies

**Depends on:**
- **Entities/Creature** — `Creature::m_loot`, `Creature::IsLootRecipient`, `m_lootRecipient`, `m_lootRecipientGroup`, kill-tap logic, `AllLootRemovedFromCorpse` (sets corpse decay timer)
- **Entities/GameObject** — chests, mining/herb nodes, fishing bobbers, quest containers; `GameObject::m_loot`, `GameObject::Use` triggers `Loot::FillLoot`
- **Entities/Player** — inventory store/take APIs (`StoreNewItem`, `StoreLootItem`, `CanStoreNewItem`), `m_lootRolls` per-player roll tracking, money modifications, `OnLootedObject` quest hook, `SendLootError`, `SendNotifyLootItemRemoved`
- **Entities/Item** — random property/suffix generation (`Item::GenerateItemRandomPropertyId`), bag containers, item template (for `AllowableClass`/`AllowableRace`/`Bonding`)
- **Conditions** — `ConditionsReference` per `LootStoreItem`; `CONDITION_SOURCE_TYPE_LOOT_TEMPLATE`, `CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE`. Recursive resolution mid-roll
- **Group** — `GetLootMethod`, `GetLootThreshold`, `GetMasterLooterGuid`, `Group::GroupLoot`/`MasterLoot`/`NeedBeforeGreed`/`SendLooter`/`CountRollVote`/`EndRoll`. Roll permissions: round-robin owner, master GUID
- **Quests** — `Quest::IsRequiredItemForObjective` so `needs_quest` items only drop for players with matching active objective
- **DBC/DB2** — `ItemDisenchantLoot.db2` (store foundation exists in `wow-data`; full disenchant rolls still pending), `ItemClass.db2` (loaded for the C++ item price modifier foundation), `ItemRandomProperties.dbc`, `ItemRandomSuffix.dbc`, `MapDifficulty.dbc` (lootMode mapping), `SpellEffect.dbc` (skinning skill)
- **WorldPackets/Loot** — every loot opcode struct
- **ScriptMgr** — `OnLootCreatureLoot`, `OnLootItem`, `OnLootMoney` hooks per script
- **DB rates** (`Rate.Drop.Item.Poor/Normal/Uncommon/Rare/Epic/Legendary/Artifact/Referenced`, `Rate.Drop.Money`)

**Depended on by:**
- **Quests** — quest item drops route through `LootTemplate::HasQuestDropForPlayer`; `ItemAddedQuestCheck` chained from `StoreLootItem`
- **Achievements** — `CRITERIA_TYPE_LOOT_ITEM`, `CRITERIA_TYPE_LOOT_TYPE`, `CRITERIA_TYPE_LOOT_MONEY`
- **AI/Scripts** — many boss scripts call `Loot::AddItem` post-fact for guaranteed drops
- **Mail** — `mail_loot_template` drives mail attachments
- **Battlegrounds** — insignia loot type
- **AHBot** — generates fake auction listings from `LootTemplates_Creature`
- **Calendar/Events** — seasonal loot (Hallow's End candy, etc.) gated by `ConditionMgr` event conditions

---

## 6. SQL / DB queries

### World DB — loot template tables (one per `LootStore` instance)

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount FROM creature_loot_template` | Drops per creature entry | world |
| `SELECT … FROM gameobject_loot_template` | Drops per GO entry (chests, herb/ore nodes) | world |
| `SELECT … FROM reference_loot_template` | Shared sub-tables referenced via `Reference > 0` | world |
| `SELECT … FROM disenchant_loot_template` | Per `DisenchantID` (column on `Item.dbc`) | world |
| `SELECT … FROM fishing_loot_template` | Keyed by area ID (or fishing-hole entry) | world |
| `SELECT … FROM pickpocketing_loot_template` | Keyed by `creature_template.pickpocketloot` | world |
| `SELECT … FROM skinning_loot_template` | Keyed by `creature_template.skinloot` (skinning, gathering, salvage) | world |
| `SELECT … FROM mail_loot_template` | Mail attachments by mail-template id | world |
| `SELECT … FROM milling_loot_template` | Inscription milling, keyed by herb item id | world |
| `SELECT … FROM prospecting_loot_template` | Jewelcrafting prospecting, keyed by ore item id | world |
| `SELECT … FROM item_loot_template` | Bag/lockbox contents, keyed by item id | world |
| `SELECT … FROM spell_loot_template` | Items created by spell effects (random-item spells) | world |
| `SELECT * FROM conditions WHERE SourceTypeOrReferenceId IN (1, 5, 7, 18, 21, 22, 23, 24, 25)` (loot-source types) | Per-loot conditions joined into `LootStoreItem.conditions` | world |

### Character DB — runtime persistence

| Statement (TC enum) | Purpose | DB |
|---|---|---|
| `CHAR_SEL_ITEM_LOOT_ITEMS` | `SELECT containerGUID, itemId, count, itemIndex, followLootRules, freeForAll, isBlocked, isCounted, isUnderThreshold, needsQuest, randomBonusListId, context, bonusListIDs FROM item_loot_items` | characters |
| `CHAR_SEL_ITEM_LOOT_MONEY` | `SELECT containerGUID, money FROM item_loot_money` | characters |
| `CHAR_INS_ITEM_LOOT_ITEMS` | Persist a freshly-dropped container item | characters |
| `CHAR_INS_ITEM_LOOT_MONEY` | Persist money in a container | characters |
| `CHAR_DEL_ITEM_LOOT_ITEMS` (per containerGUID + itemIndex) | Remove looted item slot | characters |
| `CHAR_DEL_ITEM_LOOT_MONEY` (per containerGUID) | Remove money | characters |
| `CHAR_DEL_ITEM_LOOT_ALL_ITEMS` (per containerGUID) | Drop container | characters |
| `CHAR_DEL_NONEXISTENT_ITEM_LOOT` | Cleanup orphaned rows | characters |
| `CHAR_INS_BG_LOOT_LIST` (when applicable for BG insignia) | optional | characters |

### DBC / DB2 stores

| Store | What it loads | Read by |
|---|---|---|
| `ItemDisenchantLootStore` | `ItemDisenchantLoot.db2` (class/subclass/quality/level/expansion → DisenchantID) | `LootRoll::GetItemDisenchantLoot`, disenchant-type rolls |
| `ItemRandomPropertiesStore` | `ItemRandomProperties.dbc` | `LootItem` ctor random property generation |
| `ItemRandomSuffixStore` | `ItemRandomSuffix.dbc` | `LootItem` ctor suffix generation |
| `MapDifficultyStore` | difficulty bit → `lootMode` mask | `Loot::FillLoot` |
| `SpellEffectStore` (skinning skill effect) | skinning required-skill values | `Creature::IsSkinnedBy` |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_LOOT_UNIT` | client → server | `WorldSession::HandleLootOpcode` (right-click corpse / GO) |
| `CMSG_LOOT_ITEM` | client → server | `HandleAutostoreLootItemOpcode` (one-by-one or AE batch) |
| `CMSG_LOOT_RELEASE` | client → server | `HandleLootReleaseOpcode` (close window) |
| `CMSG_LOOT_MONEY` | client → server | `HandleLootMoneyOpcode` |
| `CMSG_LOOT_ROLL` | client → server | `HandleLootRoll` (Pass / Need / Greed / Disenchant) |
| `CMSG_LOOT_MASTER_GIVE` | client → server | `HandleLootMasterGiveOpcode` (master assigns to GUID) |
| `CMSG_SET_LOOT_METHOD` | client → server | `HandleSetLootMethod` (party leader changes method/threshold/master) |
| `CMSG_SET_LOOT_SPECIALIZATION` | client → server | `HandleSetLootSpecialization` (select/clear loot spec) |
| `CMSG_OPT_OUT_OF_LOOT` | client → server | `HandleOptOutOfLoot` (player opts out of group rolls) |
| `CMSG_AE_LOOT_TARGETS` / `CMSG_AE_LOOT_TARGET_ACK` | client → server | Not present as handlers in the contrasted Trinity 3.4.3 tree; AE target streaming is server-driven from `HandleLootOpcode` |
| `SMSG_LOOT_RESPONSE` | server → client | `Loot::BuildLootResponse` (window contents) |
| `SMSG_LOOT_RELEASE` (legacy SMSG_LOOT_RELEASE_RESPONSE) | server → client | `WorldSession::SendLootRelease` |
| `SMSG_LOOT_RELEASE_ALL` | server → client | broadcast on bulk close (AE looting end) |
| `SMSG_LOOT_REMOVED` | server → client | `Loot::NotifyItemRemoved` per slot |
| `SMSG_LOOT_MONEY_NOTIFY` | server → client | per-player share notification |
| `SMSG_LOOT_LIST` | server → client | `Loot::NotifyLootList` owner/loot-object plus optional master and round-robin winner |
| `SMSG_LOOT_ALL_PASSED` | server → client | every roller passed |
| `SMSG_LOOT_ROLL` | server → client | individual vote broadcast |
| `SMSG_LOOT_ROLL_WON` | server → client | winner announcement |
| `SMSG_LOOT_ROLLS_COMPLETE` | server → client | all rolls done |
| `SMSG_START_LOOT_ROLL` | server → client | a roll begins (per item, per recipient) |
| `SMSG_MASTER_LOOT_CANDIDATE_LIST` | server → client | master loot picker UI |
| `SMSG_SET_LOOT_METHOD_FAILED` | server → client | rejection of CMSG_SET_LOOT_METHOD |
| `SMSG_LEGACY_LOOT_RULES` | server → client | toggle for "old rules" personal loot |
| `SMSG_GUILD_ITEM_LOOTED_NOTIFY` | server → client | guild news on epic+ drop |
| `SMSG_AE_LOOT_TARGETS` / `_ACK` | server → client | AE loot target count and per-target ack |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-loot` | `crate_dir` | 1 | active | `exists_active` | foundation started: `LootStoreItem`, `LootTemplate`, `LootGroup`, `LootStore` |
| `crates/wow-packet/src/packets/loot.rs` | `file` | 1 | 210 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `crates/wow-loot/src/lib.rs` | `file` | 1 | active | `exists_active` | foundation started by #NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.271 |
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-loot/src/lib.rs` — foundation started in `#NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.271`: contrasted `LootStoreItem`, `LootTemplate`, `LootGroup`, `LootStoreKind`, `LootStoreDefinition`, pure `LootStore` row loading, C++ `LootTemplates_Reference` cross-store verification, structural loot-condition link validation and pure `Loot::FillLoot`/`LootTemplate::Process` materialization with `LootItem` constructor metadata including random properties id/seed and unit coverage. `world-server` startup loading/injection exists in `#NEXT.R8.ENTITIES.272`, reference diagnostics in `#NEXT.R8.ENTITIES.273`, condition source/link diagnostics in `#NEXT.R8.ENTITIES.274`, represented creature corpse generation consumes `LootTemplates_Creature` in `#NEXT.R8.ENTITIES.277`, context-aware fill callbacks now expose the C++ condition source tuple for represented `AllowedForPlayer` gates in `#NEXT.R8.ENTITIES.279`, represented loot-condition row/evaluator helpers plus C++ `CompareValues` ordering are shared from `wow-loot` in `#NEXT.R8.ENTITIES.280`, represented condition-reference template expansion exists in `#NEXT.R8.ENTITIES.281`, startup diagnostics validate missing condition-reference templates in `#NEXT.R8.ENTITIES.282`, self-referencing condition-reference rows are skipped in `#NEXT.R8.ENTITIES.283`, ElseGroup evaluation now uses C++ map semantics in `#NEXT.R8.ENTITIES.284`, deterministic C++ load-skip guards exist in `#NEXT.R8.ENTITIES.285`, playable class/race mask rules are represented in `#NEXT.R8.ENTITIES.286`, comparison/target load skips are extended in `#NEXT.R8.ENTITIES.287`, remaining deterministic target/mask skips are represented in `#NEXT.R8.ENTITIES.288`, legacy object/type-mask condition normalization plus represented player object/type-mask evaluation exist in `#NEXT.R8.ENTITIES.289`, and pure personal loot template processing for entries/references/groups exists in `#NEXT.R8.ENTITIES.318`. `LootRoll`, full canonical `ConditionMgr` storage and full canonical gameplay runtime wiring remain unimplemented.
- `crates/wow-packet/src/packets/loot.rs` — packet structs only: `LootUnit`, `LootItemPkt`, `LootItemRequest`, `LootRelease`, `LootReleaseAll`, `LootItemData`, `LootCurrencyData`, `LootResponse`, `LootRemoved`, `SLootRelease`, C++ `LootType` constants needed for represented wire parity, plus the in-memory `CreatureLoot` / `LootEntry` helpers used by the world handler.
- `crates/wow-data/src/dungeon_encounter.rs` — minimal C++ `sDungeonEncounterStore` DB2 reader added in `#NEXT.R8.ENTITIES.301` for `DungeonEncounterEntry` fields needed by loot/lock parity (`ID`, `MapID`, `DifficultyID`, `OrderIndex`, `Bit`, `Flags`, `Faction`). Startup/session injection exists in `#NEXT.R8.ENTITIES.302`; hotfix overlay, script boss mappings and `InstanceScript::GetBossDungeonEncounter` remain pending.
- `crates/wow-entities/src/game_object.rs` — C++-indexed gameobject template loot source helpers added in `#NEXT.R8.ENTITIES.306`: `GetLootId` parity for chest/fishing-hole/gathering-node and chest `Data1`/`Data15`/`Data25`/`Data30` extraction for loot id, group-loot rules, dungeon encounter and personal loot.
- `crates/wow-world/src/handlers/loot.rs` — registers + implements `CMSG_LOOT_UNIT`, `CMSG_LOOT_ITEM`, `CMSG_LOOT_MONEY`, `CMSG_LOOT_RELEASE` plus a represented direct inventory store bridge. Storage is still `WorldSession::loot_table: HashMap<ObjectGuid, CreatureLoot>` (per-session, lost on disconnect). Creature corpse generation now resolves `CreatureDifficulty::LootID`/gold bounds, consumes `LootTemplates_Creature`, preserves random properties id/seed/context, applies `item_template_addon.FlagsCu` for `ITEM_FLAGS_CU_FOLLOW_LOOT_RULES`, initializes represented `LootType` as `LOOT_CORPSE`, maps C++ internal loot types through `GetLootTypeForClient` for `LootResponse.AcquireReason`, preserves represented creature `dungeon_encounter_id` once a source provides it, filters candidate drops through a represented `LootItem::AllowedForPlayer` subset including DB loot conditions with C++ condition-reference templates, faction flags, quest gates, `QuestLogItemId` and `ITEM_FLAGS_CU_IGNORE_QUEST_STATUS`, and mirrors the represented creature shared-loot summary (`gold`, `unlooted_count`) into canonical `Creature` state so release can ask the canonical `Creature::IsFullyLooted` shape before corpse-decay decisions (#NEXT.R8.ENTITIES.277/#NEXT.R8.ENTITIES.278/#NEXT.R8.ENTITIES.279/#NEXT.R8.ENTITIES.281/#NEXT.R8.ENTITIES.298/#NEXT.R8.ENTITIES.300). Canonical `Creature`/`GameObject` summary state now also models the C++ `GetLootForPlayer` precedence: shared loot is returned only while the personal-loot map is empty; once any personal loot exists, only that player's personal entry is visible. Shared-loot sync clears stale personal summaries so this precedence remains truthful across represented state transitions, and represented item pickup/master-loot removal refreshes the canonical summary immediately after the C++ `--loot->unlootedCount` equivalent instead of waiting for release. This summary sync is not full `Creature::m_loot` / `GameObject::m_loot` ownership and does not represent real `Creature::m_personalLoot` generation yet. Represented `CMSG_GAME_OBJ_USE` chest opening now consumes the #NEXT.R8.ENTITIES.306 source helper and `LootTemplates_Gameobject` with `LOOT_CHEST`, group-rule gating and carried `DungeonEncounterID` (#NEXT.R8.ENTITIES.307), chest money now uses `gameobject_template_addon.mingold/maxgold` through the shared C++ money helper (#NEXT.R8.ENTITIES.308), `chestPushLoot` is generated/stored directly for the C++ no-`GetLootId()` branch (#NEXT.R8.ENTITIES.309), fishing-hole/gathering-node gameobject loot opens are represented (#NEXT.R8.ENTITIES.310), fishing bobber fish/junk opens now consume `LootTemplates_Fishing` with C++ parent-area/default-zone fallback (#NEXT.R8.ENTITIES.322), use the represented bobber area rather than the player area while base-skill rolls consume `skill_fishing_base_level` (#NEXT.R8.ENTITIES.324), resolve `GetProfessionSkillForExp(SKILL_FISHING, 0)` through `SkillLine.db2` before reading represented skill values (#NEXT.R8.ENTITIES.325), and read canonical `GameObjectData::CreatedBy` for represented owned-gameobject loot distance exceptions when a typed gameobject exists (#NEXT.R8.ENTITIES.326), gathering-node XP now uses `QuestXP[player level].Difficulty[xpDifficulty]` plus exact C++ `RoundXPValue` (#NEXT.R8.ENTITIES.311), represented gameobject use records C++ triggered-event/linked-trap hook points for chest and gathering (#NEXT.R8.ENTITIES.312), and session-local represented GO use state now tracks chest/gathering `GO_ACTIVATED`, gathering max-loot depletion/no-interact, despawn delay and spell-cast hook points (#NEXT.R8.ENTITIES.313); actual UpdateObject fanout, GameEvents/trap execution and remaining canonical `GameObject::Use` runtime are still pending.
- `crates/wow-world/src/session.rs` — session-local represented `GameObject::m_tapList` support added in `#NEXT.R8.ENTITIES.314` for the C++ `chestPersonalLoot + DungeonEncounter` tapper selection branch. `#NEXT.R8.ENTITIES.315` adds represented `(player, dungeonEncounterId)` lockout markers and prevents personal encounter opening from auto-authorizing the current player outside `m_personalLoot`. `#NEXT.R8.ENTITIES.316` adds represented per-tapper money for personal encounter loot so `LootResponse`/`CMSG_LOOT_MONEY` read and consume money by `(gameobject, player)` instead of using the shared represented `CreatureLoot.coins`. `#NEXT.R8.ENTITIES.317/#NEXT.R8.ENTITIES.318` assign represented personal encounter items through the pure C++-shaped personal loot processor and rebuild FFA/unlooted counting from `FillNotNormalLootFor`. `#NEXT.R8.ENTITIES.319` evaluates represented personal-loot candidates against the actual tapper GUID, using registry race/class/sex/level for remote faction/class/race/gender/level gates and failing closed for remote inventory/quest/spell/objective gates that Rust cannot yet verify. `#NEXT.R8.ENTITIES.320` publishes known spells, active quest statuses and rewarded quests into `PlayerBroadcastInfo`, keeps them synchronized on quest/trainer mutations, and lets remote represented personal-loot conditions evaluate C++ `Player::HasSpell`/`GetQuestStatus`-style gates without falling back to the owner session. `#NEXT.R8.ENTITIES.321` publishes represented inventory item counts and active quest objective counters, syncs them on quest/loot inventory mutations, and uses them for remote `Player::HasQuestForItem`, `Quest::ItemDrop`, item-count conditions and quest-objective-progress conditions. Represented loot now tracks eligible tap-list player GUIDs, with the same current-player fallback C++ uses when the tap list is empty; full canonical per-player `Loot` objects/items, real `InstanceLockMgr` lockout derivation and canonical `m_personalLoot` remain pending.

**What's implemented:**
- `CMSG_LOOT_UNIT` → checks represented target through the C++ invalid-target shape (dead-player, non-creature GUID, alive creature and >30yd return silently), applies represented consumed/allowed-looter gates, interrupts an active represented non-melee cast and removes represented visible auras with the C++ `SpellAuraInterruptFlags::Looting` bit only after those successful-target guards, releases any existing non-item main loot view before opening a new main view, generates loot lazily from resolved `CreatureDifficulty::LootID` through `LootTemplates_Creature`, applies `ITEM_FLAGS_CU_FOLLOW_LOOT_RULES` for quest-required generated drops, filters candidate rows through represented `AllowedForPlayer` conditions/faction/quest gates including condition-reference templates, and sends acquired `SMSG_LOOT_RESPONSE` only when there is represented money or at least one represented visible loot row. Creature loot now uses separate represented `Owner` and `HighGuid::LootObject` values like C++ `Loot::GetOwnerGUID()`/`Loot::GetGUID()`, represented `LootResponse.AcquireReason` uses C++ `GetLootTypeForClient`, represented `dungeon_encounter_id` is preserved through creature state/respawn/generation when nonzero, represented money uses loaded `GoldMin..GoldMax` when available, `EnableAELoot` gates represented server-driven `SMSG_AE_LOOT_TARGETS`/`SMSG_AE_LOOT_TARGET_ACK` fanout for additional dead in-range represented creatures, first-open master loot sends represented candidate lists after the response, represented group loot fans out `SMSG_LOOT_LIST`, and first-open `GROUP_LOOT`/`NEED_BEFORE_GREED` starts represented `SMSG_START_LOOT_ROLL` prompts for connected allowed looters (#NEXT.R8.ENTITIES.206/#NEXT.R8.ENTITIES.207/#NEXT.R8.ENTITIES.208/#NEXT.R8.ENTITIES.214/#NEXT.R8.ENTITIES.217/#NEXT.R8.ENTITIES.218/#NEXT.R8.ENTITIES.219/#NEXT.R8.ENTITIES.220/#NEXT.R8.ENTITIES.221/#NEXT.R8.ENTITIES.244/#NEXT.R8.ENTITIES.245/#NEXT.R8.ENTITIES.246/#NEXT.R8.ENTITIES.277/#NEXT.R8.ENTITIES.278/#NEXT.R8.ENTITIES.279/#NEXT.R8.ENTITIES.281/#NEXT.R8.ENTITIES.298/#NEXT.R8.ENTITIES.300). Full canonical aura ownership, interrupt source-ignore checks and `AURA_REMOVE_BY_INTERRUPT` side effects remain under aura runtime work; canonical `InstanceScript::GetBossDungeonEncounter` source resolution remains pending.
- `CMSG_LOOT_ITEM` → represented direct inventory store bridge for current loot entries, including active-view guards, C++ `SMSG_LOOT_RELEASE_ALL` for represented blocked/disallowed-looter/non-winning-roll `Player::StoreLootItem` refusal branches, FFA removal guards, active creature `LOOT_ERROR_TOO_FAR`, missing-creature `LOOT_ERROR_NO_LOOT`, missing-gameobject release branches, C++ `SMSG_LOOT_RELEASE` owner semantics, represented `SMSG_LOOT_REMOVED` owner semantics, C++ 3.4.3 `LootItemData` row shape, represented LootObject-to-owner active-view lookup, secondary active loot-object lookup inside the represented view, canonical Creature/GameObject loot-summary refresh after item removal/master-loot removal follows the same point where C++ decrements `unlootedCount`, and represented `Loot::NotifyItemRemoved` now prunes stale `PlayersLooting` entries that cannot be resolved on the current map like C++ (#NEXT.R8.ENTITIES.195/#NEXT.R8.ENTITIES.198/#NEXT.R8.ENTITIES.201/#NEXT.R8.ENTITIES.202/#NEXT.R8.ENTITIES.203/#NEXT.R8.ENTITIES.211/#NEXT.R8.ENTITIES.212/#NEXT.R8.ENTITIES.213/#NEXT.R8.ENTITIES.214/#NEXT.R8.ENTITIES.215/#NEXT.R8.ENTITIES.223); still not the full canonical Trinity `Player::StoreLootItem` across AE/group/criteria/news side effects
- `CMSG_LOOT_MONEY` → represented active-loot money pickup consumes current coins across every represented active loot view, sends `SMSG_COIN_REMOVED` before each `SMSG_LOOT_MONEY_NOTIFY`, uses the represented LootObject in `SMSG_COIN_REMOVED` like C++ `Loot::NotifyMoneyRemoved`, prunes stale `PlayersLooting` entries that cannot be resolved on the current map like C++, persists the current player's share, removes stored item-container money, no-ops stale active views without represented loot rows, still emits zero-money notifications like C++, splits represented corpse money to near group members with C++ integer division/SoleLooter semantics, and consumes represented personal encounter chest money per current player without zeroing other tappers' personal entries (#NEXT.R8.ENTITIES.200/#NEXT.R8.ENTITIES.209/#NEXT.R8.ENTITIES.210/#NEXT.R8.ENTITIES.214/#NEXT.R8.ENTITIES.215/#NEXT.R8.ENTITIES.234/#NEXT.R8.ENTITIES.316); other-session money persistence, aura modifiers, criteria and script behavior remain pending
- `CMSG_LOOT_RELEASE` / logout cleanup → represented active-view release guard ignores spoofed GUIDs (#NEXT.R8.ENTITIES.204), emits C++-ordered `SMSG_LOOT_RELEASE` (`LootObj`, player `Owner`) for represented active loot views (#NEXT.R8.ENTITIES.211/#NEXT.R8.ENTITIES.216), keeps unlooted represented object loot rows on close (#NEXT.R8.ENTITIES.205), removes loot from the session map only when fully looted, removes `UNIT_DYNFLAG_LOOTABLE`, forces the represented `DynamicFlags` field update, and schedules corpse decay from `Creature::m_corpseDelay * Rate.Corpse.Decay.Looted` (with immediate fully-looted `LOOT_SKINNING` decay) when fully looted, represented logout request sends `SMSG_LOOT_RELEASE_ALL` before `LogoutResponse` when a loot GUID is active (#NEXT.R8.ENTITIES.225), represented logout completion calls the same `DoLootReleaseAll` helper before clearing player state like `Player::RemoveFromWorld` (#NEXT.R8.ENTITIES.224), represented disconnect cleanup calls that helper before unregistering/clearing shared runtime state like `WorldSession::LogoutPlayer` (#NEXT.R8.ENTITIES.226), and represented `CMSG_SWAP_INV_ITEM` calls it when a moved bag contains active item loot like `Player::SwapItem` (#NEXT.R8.ENTITIES.227)
- Represented creature money no longer fabricates fallback copper when `GoldMax == 0`; it now follows C++ `Loot::generateMoneyLoot` by leaving gold at zero unless the DB range has a positive max amount.

**What's missing vs C++:**
- **`crates/wow-loot/` is still a foundation, but creature corpse generation now consumes it.** `LootStoreItem`, `LootTemplate`, `LootGroup`, `LootStoreKind`, `LootStoreDefinition`, pure `LootStore` row loading/reference verification and `fill_loot_like_cpp` materialization exist after `#NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.277`; startup load/injection exists after `#NEXT.R8.ENTITIES.272`. There is still no canonical shared `Loot`, `LootRoll`, `LootMode` wrapper or full runtime ownership.
- Async startup `*_loot_template` loading now exists for all 12 stores (#NEXT.R8.ENTITIES.272), C++ `LootTemplates_Reference` verification now runs non-fatally (#NEXT.R8.ENTITIES.273), structural loot-condition source/link validation now runs non-fatally (#NEXT.R8.ENTITIES.274), pure store materialization exists in `wow-loot` (#NEXT.R8.ENTITIES.275/#NEXT.R8.ENTITIES.276), and represented creature corpse loot consumes `LootTemplates_Creature` plus item-template addon and represented `AllowedForPlayer` condition/faction/quest gates including condition-reference templates (#NEXT.R8.ENTITIES.277/#NEXT.R8.ENTITIES.278/#NEXT.R8.ENTITIES.279/#NEXT.R8.ENTITIES.281), but the reusable runtime still lacks full canonical `ConditionMgr` storage/evaluation.
- Reference rows are loaded into store foundations and cross-store verification is ported, but recursive runtime processing through canonical `Loot::FillLoot` remains pending.
- Drop chance roll and pure `LootTemplate::Process` materialization exist in `wow-loot` (#NEXT.R8.ENTITIES.275), with represented creature corpse loot wired to it for `LootTemplates_Creature` (#NEXT.R8.ENTITIES.277).
- Grouped vs ungrouped entry splitting exists in the `wow-loot` foundation and represented disenchant path (#NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.271), pure `fill_loot_like_cpp` now processes entries/groups/references (#NEXT.R8.ENTITIES.275), represented creature corpse ownership is wired to `LootTemplates_Creature` (#NEXT.R8.ENTITIES.277), and represented chest opening consumes `LootTemplates_Gameobject` (#NEXT.R8.ENTITIES.307), but canonical shared `Loot` ownership is not wired.
- `lootMode` is loaded into `LootStoreItem` and pure `fill_loot_like_cpp` applies active-mode filtering (#NEXT.R8.ENTITIES.275), but real map difficulty to loot mode wiring remains pending at gameplay call sites.
- No canonical quest-only drops in `wow-loot`; represented `CMSG_OPEN_ITEM` item-loot bridge handles `QuestRequired`, `Quest::ItemDrop`, `ItemSparse.StartQuestID`, `QuestLogItemId` and `ITEM_FLAGS_CU_IGNORE_QUEST_STATUS` (#NEXT.R8.ENTITIES.189/#NEXT.R8.ENTITIES.191). Represented item, creature, gameobject, fishing and personal-loot `LootTemplate::Process`-style chance/group/count rolls now use RNG derived from the session-owned runtime RNG instead of ad-hoc `thread_rng`; random-property/suffix generation for those generated loot rows consumes the same fill RNG like the C++ `Loot::AddItem` -> `GenerateRandomProperties` flow.
- No canonical `LootStoreItem.conditions` evaluator in reusable `wow-loot`; structural source/link diagnostics exist for all 12 loot condition source types (#NEXT.R8.ENTITIES.274), represented `CMSG_OPEN_ITEM` item-loot bridge loads/evaluates the player-condition subset for `item_loot_template` and `reference_loot_template` (#NEXT.R8.ENTITIES.192), and represented creature corpse loot now applies the same subset for `creature_loot_template`/`reference_loot_template` candidates (#NEXT.R8.ENTITIES.279). The shared represented row/evaluator and C++ `CompareValues` order live in `wow-loot` (#NEXT.R8.ENTITIES.280), the represented item/creature bridges now preload recursive C++ condition-reference templates (#NEXT.R8.ENTITIES.281), startup diagnostics report missing condition-reference templates (#NEXT.R8.ENTITIES.282), self-referencing reference rows are skipped like C++ (#NEXT.R8.ENTITIES.283), represented ElseGroup evaluation no longer depends on contiguous row order (#NEXT.R8.ENTITIES.284), deterministic C++ load-invalid row skips are represented (#NEXT.R8.ENTITIES.285), class/race mask behavior now matches playable mask plus C++ race-bit remapping (#NEXT.R8.ENTITIES.286), more comparison/target invalid rows are skipped like C++ (#NEXT.R8.ENTITIES.287), remaining deterministic target/mask invalid rows are skipped like C++ (#NEXT.R8.ENTITIES.288), legacy object/type-mask rows are normalized/evaluated for the represented player target (#NEXT.R8.ENTITIES.289), represented personal encounter generation evaluates the current tapper instead of the owner session (#NEXT.R8.ENTITIES.319), remote known-spell/quest-status conditions now use published registry state (#NEXT.R8.ENTITIES.320), and remote inventory/objective-backed item-count, quest-objective-progress and `HasQuestForItem`/`Quest::ItemDrop` gates now use published registry state (#NEXT.R8.ENTITIES.321). Full canonical `ConditionMgr` storage and non-loot condition consumers remain pending.
- No canonical `LootItem` group/master metadata behavior; represented `CMSG_OPEN_ITEM` item-loot bridge preserves `follow_loot_rules`, `freeforall`, `blocked`, `counted`, `under_threshold` and `needs_quest` through generated/stored item loot (#NEXT.R8.ENTITIES.193), applies the `FREE_FOR_ALL` item-view metadata in `LootResponse` (#NEXT.R8.ENTITIES.194), filters represented response rows by `allowedGUIDs` (#NEXT.R8.ENTITIES.197), and keeps represented FFA removal state per player instead of using global `taken` (#NEXT.R8.ENTITIES.198)
- `CMSG_LOOT_ITEM` still lacks full canonical `StoreLootItem`, but the represented bridge now checks the active loot view, sends C++ empty `SMSG_LOOT_RELEASE_ALL` for blocked/disallowed-looter/non-winning roll refusal branches without clearing server-side active loot state (#NEXT.R8.ENTITIES.223), sends represented `LOOT_ERROR_TOO_FAR` for active creature loot beyond 30 yards (#NEXT.R8.ENTITIES.201), sends represented `LOOT_ERROR_NO_LOOT` when the active creature owner no longer resolves (#NEXT.R8.ENTITIES.202), releases missing active gameobject owners (#NEXT.R8.ENTITIES.203), uses the C++ `SMSG_LOOT_RELEASE` `LootObj`/player-`Owner` payload shape (#NEXT.R8.ENTITIES.211), uses represented loot-owner semantics for `SMSG_LOOT_REMOVED` (#NEXT.R8.ENTITIES.212), serializes `LootItemData` with the real C++ 3.4.3 row layout (#NEXT.R8.ENTITIES.213), resolves represented creature `LootObject` GUIDs back to their active owner like C++ `m_AELootView` (#NEXT.R8.ENTITIES.214), and supports secondary active loot-object lookup in that represented view (#NEXT.R8.ENTITIES.215)
- Loot rate config is wired for the represented `CMSG_OPEN_ITEM` item-loot bridge (#NEXT.R8.ENTITIES.190), represented creature corpse item/reference amount generation consumes the existing `LootDropRatesLikeCpp` callbacks (#NEXT.R8.ENTITIES.277), represented chest item/reference amount generation reuses those callbacks (#NEXT.R8.ENTITIES.307), represented item/corpse/chest money now uses the session-owned runtime RNG through the C++-shaped `Loot::generateMoneyLoot` helper instead of ad-hoc `thread_rng`, represented item/corpse/chest/gameobject/fishing loot item rolls and direct-store random properties also consume session-owned RNG, and represented chest money uses `gameobject_template_addon.mingold/maxgold` (#NEXT.R8.ENTITIES.308); player corpse stores and a canonical shared `Loot` rate owner remain pending.
- No personal loot (`PERSONAL_LOOT`) — every drop is FFA
- No canonical group loot roll engine yet (`LootRoll`, timeout, vote/winner rules); packet parsing/serialization exists for `CMSG_LOOT_ROLL`, `SMSG_START_LOOT_ROLL`, `SMSG_LOOT_ROLL`, `SMSG_LOOT_ROLL_WON`, `SMSG_LOOT_ALL_PASSED` and `SMSG_LOOT_ROLLS_COMPLETE` (#NEXT.R8.ENTITIES.228), represented first-open `GROUP_LOOT`/`NEED_BEFORE_GREED` starts `SMSG_START_LOOT_ROLL` prompts for connected allowed looters while unblocking/marking under-threshold single-candidate items (#NEXT.R8.ENTITIES.246), represented current-session `CMSG_LOOT_ROLL` broadcasts the immediate `LootRoll::PlayerVote` roll packet for valid blocked group/NBG entries (#NEXT.R8.ENTITIES.247), represented all-voted state can finish a current-session shared roll, select a Need-over-Greed winner, unblock the item, store `roll_winner` and emit `SMSG_LOOT_ROLL_WON` variants (#NEXT.R8.ENTITIES.248), represented all-pass finish now follows the contrasted C++ branch by not sending `SMSG_LOOT_ALL_PASSED` to valid pass voters (#NEXT.R8.ENTITIES.249), represented owner-session `LootRollVote` commands can apply a connected remote player's vote to the pending owner roll state (#NEXT.R8.ENTITIES.250), represented remote `CMSG_LOOT_ROLL` can discover the owner session through published pending roll keys and route the vote there (#NEXT.R8.ENTITIES.251), represented roll timeout expiry can finish the current winner like `LootRoll::UpdateRoll` (#NEXT.R8.ENTITIES.252), represented winner finish replays final `SMSG_LOOT_ROLL` vote values before `SMSG_LOOT_ROLL_WON` like `SendLootRollWon` (#NEXT.R8.ENTITIES.253), represented stored Need/Greed/Disenchant roll numbers now use session-owned inclusive 1..100 RNG like `urand(1, 100)` and Pass leaves the stored roll untouched like `LootRoll::PlayerVote` (#NEXT.R8.ENTITIES.254), represented vote/winner criteria hook points now match the C++ callsites without pretending the missing achievements backend exists (#NEXT.R8.ENTITIES.255), represented roll startup now keeps `NotValid` disconnected/off-map allowed looters plus C++ auto-pass fanout for pass-on-group-loot voters (#NEXT.R8.ENTITIES.256), represented non-disenchant winners now route through current-session or connected remote-session `StoreLootItem` before the owner marks/removes the represented slot (#NEXT.R8.ENTITIES.257), represented `SMSG_START_LOOT_ROLL.ValidRolls` clears Need for `ITEM_FLAG2_CAN_ONLY_ROLL_GREED` items (#NEXT.R8.ENTITIES.258), `wow-data` has the contrasted ordered `ItemDisenchantLoot.db2` store/filter foundation (#NEXT.R8.ENTITIES.259), world-server now injects that store into sessions (#NEXT.R8.ENTITIES.260), `ItemSparse` now exposes the area/map/expansion fields required by the C++ disenchant gate (#NEXT.R8.ENTITIES.261), `ItemCurrencyCost.db2`/`HasItemCurrencyCost` is available for the no-sell-price disenchant gate (#NEXT.R8.ENTITIES.262), `ItemClass.db2` plus the `ItemSparse` price fields are available for the C++ `GetBuyPrice`/`GetSellPrice` foundation (#NEXT.R8.ENTITIES.263), `ImportPriceArmor/Quality/Shield/Weapon.db2` are loaded for the C++ `Item::GetBuyPrice` formula (#NEXT.R8.ENTITIES.264), `ItemPriceBase.db2` is loaded for the C++ armor/weapon base factors (#NEXT.R8.ENTITIES.265), represented `GetBuyPrice`/`GetSellPrice` helpers exist with the contrasted `standardPrice=false` behavior (#NEXT.R8.ENTITIES.266), the represented full `Item::GetDisenchantLoot` gate helper now returns `DisenchantID` plus `SkillRequired` (#NEXT.R8.ENTITIES.267), represented `SMSG_START_LOOT_ROLL.ValidRolls` now includes/removes Disenchant using the C++ `ROLL_ALL_TYPE_MASK` plus `SkillRequired > maxEnchantingSkill` gate (#NEXT.R8.ENTITIES.268), represented `ROLL_DISENCHANT` winners now generate/store `disenchant_loot_template`/reference rows before removing the original rolled item (#NEXT.R8.ENTITIES.269), `wow-loot` now owns the shared `LootStoreItem`/`LootTemplate`/`LootGroup` foundation used by represented disenchant including `GroupId` and reference-to-group processing (#NEXT.R8.ENTITIES.270), `wow-loot` now has the pure `LootStore` metadata/row-loader foundation plus SQL statements for all 12 table shapes (#NEXT.R8.ENTITIES.271), `world-server` now loads/injects the 12 foundation stores at startup (#NEXT.R8.ENTITIES.272), `wow-loot`/`world-server` now run the C++ `LootTemplates_Reference` missing/unused reference diagnostics (#NEXT.R8.ENTITIES.273), and structural C++ loot-condition source/link diagnostics now run for source types 1..12 (#NEXT.R8.ENTITIES.274). Real canonical shared `Loot`/`LootRoll` ownership, full `LootStoreItem.conditions` evaluation, all-or-mail fallback semantics, real per-player skill-state feed, real `ConditionMgr`/`AllowedForPlayer` invalid-voter checks and real achievements/criteria backend integration remain pending.
- No full master loot engine yet (`MASTER_LOOT`, candidate/assignment/inventory rules). Packet parsing/serialization exists for `CMSG_MASTER_LOOT_ITEM` and `SMSG_MASTER_LOOT_CANDIDATE_LIST` (#NEXT.R8.ENTITIES.228), represented `CMSG_MASTER_LOOT_ITEM` dispatch covers the initial C++ `LOOT_ERROR_DIDNT_KILL` guard for non represented master-looters (#NEXT.R8.ENTITIES.229), represented group state carries `master_looter_guid`/`PartyUpdate.LootMaster` for the permission check (#NEXT.R8.ENTITIES.230), represented target-not-found / target-ineligible errors now match C++ `LOOT_ERROR_PLAYER_NOT_FOUND` and `LOOT_ERROR_MASTER_OTHER` (#NEXT.R8.ENTITIES.235), current-session self-target inventory preflight maps `CanStoreItem` failures to C++ master-loot errors (#NEXT.R8.ENTITIES.238), non-master loot views return silently like C++ `GetLootMethod() != MASTER_LOOT` (#NEXT.R8.ENTITIES.239), loot-level allowed-looter rejection is represented (#NEXT.R8.ENTITIES.240), current-session self-target successful gives now store the item then mark/remove the represented loot slot (#NEXT.R8.ENTITIES.241), first-open master-looter `SMSG_MASTER_LOOT_CANDIDATE_LIST` fanout is represented in C++ packet order (#NEXT.R8.ENTITIES.244), and represented `SMSG_LOOT_LIST` fanout reaches connected allowed looters on the same map with optional `Master` when an over-threshold item exists (#NEXT.R8.ENTITIES.245). Connected remote target inventory routing exists via #NEXT.R8.ENTITIES.242, with finite unavailable-target command handling via #NEXT.R8.ENTITIES.243; live DB-backed successful assignment validation remains pending.
- No round-robin owner tracking (`roundRobinPlayer`)
- No canonical loot threshold (`is_underthreshold`) engine; represented first-open group-roll fallback now marks single-candidate blocked items under-threshold via #NEXT.R8.ENTITIES.246
- No full `LootMethod::NEED_BEFORE_GREED` rules; represented first-open roll prompting shares the group-loot start path via #NEXT.R8.ENTITIES.246, but need eligibility still lacks `Player::CanRollNeedOnItem`
- No round-robin order
- Represented corpse-money `SMSG_LOOT_MONEY_NOTIFY` now splits to near represented group members like C++ integer division and sets `SoleLooter` from recipient count (#NEXT.R8.ENTITIES.234). Full money persistence/modification for other live sessions, aura money modifiers, criteria and script hooks remain pending under canonical loot/session-state work.
- Fishing loot is represented for ready bobbers: fish/junk opens consume `LootTemplates_Fishing` with C++ parent-area fallback and default zone `1` (#NEXT.R8.ENTITIES.322), represented fishing-hole pool search now mirrors the C++ nearest `LookupFishingHoleAround(20.0f + CONTACT_DISTANCE)` plus `fishingHole.radius` gate (#NEXT.R8.ENTITIES.323), bobber rolls consume loaded `skill_fishing_base_level` and `SkillLine.db2` child-skill resolution (#NEXT.R8.ENTITIES.324/#NEXT.R8.ENTITIES.325), represented owner state is mirrored to typed canonical `GameObjectData::CreatedBy` for owner distance exceptions (#NEXT.R8.ENTITIES.326), and successful catch/pool delegation clears canonical bobber `spell_id` like C++ `SetSpellId(0)` while preserving the player owner handoff (#NEXT.R8.ENTITIES.327). Remaining gaps are spell-created bobber owner/channel provenance, terrain-derived bobber area, real fishing skill mutation and canonical `GameObject::m_loot`.
- No pickpocketing (`LOOT_PICKPOCKETING`, `Creature::CanBePickPocketed`, `m_pickpocketLootTime`)
- No skinning (`LOOT_SKINNING`, skinning skill check, corpse-skinned flag)
- No full canonical disenchant rolls yet (`LOOT_DISENCHANTING`, canonical `LootStore`, conditions and mail fallback). `wow-data` has the ordered `ItemDisenchantLoot.db2` store and C++-shaped DB2 filter helper (#NEXT.R8.ENTITIES.259), world-server injects it into sessions (#NEXT.R8.ENTITIES.260), the needed `ItemSparse` area/map/expansion fields are loaded (#NEXT.R8.ENTITIES.261), `ItemCurrencyCost.db2`/`HasItemCurrencyCost` exists (#NEXT.R8.ENTITIES.262), `ItemClass.db2` plus the `ItemSparse` price fields are loaded for the C++ sell-price foundation (#NEXT.R8.ENTITIES.263), `ImportPrice*.db2` stores are loaded for the C++ buy-price formula (#NEXT.R8.ENTITIES.264), `ItemPriceBase.db2` is loaded for armor/weapon base factors (#NEXT.R8.ENTITIES.265), represented `Item::GetBuyPrice`/`GetSellPrice` helpers exist (#NEXT.R8.ENTITIES.266), grouped-roll `ValidRolls` now applies `SkillRequired` masking (#NEXT.R8.ENTITIES.268), represented `ROLL_DISENCHANT` winners now store generated `disenchant_loot_template`/reference drops before removing the original item (#NEXT.R8.ENTITIES.269), and represented disenchant generation now uses shared `wow-loot` `LootTemplate` grouping including `GroupId` rows and references to a group with session-owned RNG for chance/group/count rolls (#NEXT.R8.ENTITIES.270).
- No canonical prospecting (`LOOT_PROSPECTING`) / milling (`LOOT_MILLING`) / mail loot (`LOOT_MAIL`) / full item loot (`LOOT_ITEM` containers like lockboxes / bags) / spell loot (random-item spells). Represented `CMSG_OPEN_ITEM` item-container loot now marks `LOOT_ITEM`, and represented `LootResponse.AcquireReason` aliases `LOOT_PROSPECTING`/`LOOT_MILLING` to `LOOT_DISENCHANTING` like C++ `GetLootTypeForClient` (#NEXT.R8.ENTITIES.298); the real source runtimes are still missing.
- Represented chest, mining/herb, fishing-hole and ready fishing-bobber loot opening exists for `CMSG_GAME_OBJ_USE`; full canonical `GameObject::Use` side effects remain pending.
- No random properties / random suffix on dropped items
- No item context / bonus list IDs persistence (placeholder fields exist on wire, never populated)
- No canonical `AELootResult` accumulator for area-effect (auto) loot; represented server packet fanout exists for `CMSG_LOOT_UNIT` (#NEXT.R8.ENTITIES.217)
- No `LootItemStorage` (in-progress containers vanish on restart)
- No `Group::GroupLoot/MasterLoot/NeedBeforeGreed` integration
- No `Player::CanRollNeedOnItem` (class usability check)
- No `LootRollIneligibilityReason` reporting
- `SMSG_LOOT_LIST` serializer and represented open-time fanout exist (#NEXT.R8.ENTITIES.233/#NEXT.R8.ENTITIES.245), but the canonical C++ callsite from creature-death group loot creation in `Unit.cpp` is still pending because Rust does not yet have shared `Loot` ownership. The contrasted Trinity 3.4.3 tree has no `CMSG_LOOT_LIST`/`HandleLootList`; `SMSG_LOOT_LIST` is emitted by `Loot::NotifyLootList` from canonical group/master/round-robin loot state. `CMSG_SET_LOOT_METHOD` parser/dispatch exists but is intentionally a no-op for this Trinity branch because the C++ mutation block is commented out (#NEXT.R8.ENTITIES.231). `CMSG_OPT_OUT_OF_LOOT` parser/dispatch stores represented `pass_on_group_loot` and represented first-open roll prompting now consumes the current-session flag to suppress the local prompt while still counting the player (#NEXT.R8.ENTITIES.232/#NEXT.R8.ENTITIES.246). `CMSG_MASTER_LOOT_ITEM` has represented parser/dispatch/permission guard, current-session self-target `CanStoreItem` preflight, the C++ master-loot-method guard, the loot-level allowed-looter guard and current-session self-target store/removal, connected remote target command routing, finite command wait, represented first-open candidate-list fanout and represented loot-list fanout (#NEXT.R8.ENTITIES.228/#NEXT.R8.ENTITIES.229/#NEXT.R8.ENTITIES.230/#NEXT.R8.ENTITIES.238/#NEXT.R8.ENTITIES.239/#NEXT.R8.ENTITIES.240/#NEXT.R8.ENTITIES.241/#NEXT.R8.ENTITIES.242/#NEXT.R8.ENTITIES.243/#NEXT.R8.ENTITIES.244/#NEXT.R8.ENTITIES.245), but no canonical persisted LootRoll/candidate fanout engine.
- No script hooks (`OnLootCreatureLoot`, `OnLootItem`, `OnLootMoney`)
- `SMSG_LOOT_LIST` serializer is represented in C++ order (#NEXT.R8.ENTITIES.233); group/master loot roll packet shapes are covered by #NEXT.R8.ENTITIES.228, `SMSG_LOOT_RELEASE_ALL` serializer exists with C++ empty payload (#NEXT.R8.ENTITIES.222), represented `StoreLootItem` refusal callsites are wired (#NEXT.R8.ENTITIES.223), logout-request client close is wired (#NEXT.R8.ENTITIES.225), logout `DoLootReleaseAll` cleanup is wired (#NEXT.R8.ENTITIES.224), disconnect `LogoutPlayer` loot cleanup is wired (#NEXT.R8.ENTITIES.226), and represented moved-bag `SwapItem` cleanup is wired (#NEXT.R8.ENTITIES.227), but broader runtime callsites remain pending
- No achievements integration on loot pickup
- No money split on group loot
- No complete `LootError` reporting. Represented paths cover active creature `LOOT_ERROR_TOO_FAR`, missing active creature `LOOT_ERROR_NO_LOOT`, master-not-authorized `LOOT_ERROR_DIDNT_KILL`, missing master target `LOOT_ERROR_PLAYER_NOT_FOUND`, master ineligible target/other errors and self-target master `InvFull`/unique mappings, but FFA, facing, locked, stunned and remote-target master inventory failures remain pending.
- The old direct-pickup `SMSG_LOOT_REMOVED` without inventory update bug is closed for represented current-session `CMSG_LOOT_ITEM`; master-loot self-target store/removal is represented by #NEXT.R8.ENTITIES.241. Remote master-loot target inventory mutation still needs canonical mutable target-session state.

**Suspicious / likely divergent (pre-audit hypothesis):**
- `loot_table` is a **per-session** HashMap on `WorldSession`. In TC, `Loot` belongs to the `Creature`/`GameObject`, so multiple players can see the same loot window. Current design forbids group loot from working at all.
- The old arbitrary coin formula (`level * 200 + seed`) is no longer used for represented creature money; remaining money divergence is canonical shared `Loot` ownership plus exact `lootMode`/side-effect wiring.
- Represented fully-looted creature release now schedules corpse despawn from `Creature::m_corpseDelay` × `Rate.Corpse.Decay.Looted` and honors `ignore_corpse_decay_ratio`. Remaining gap is full canonical corpse/remove/respawn ownership on `Creature`/`Map`, not the fixed 30s fallback.
- The `taken: bool` flag is server-only; TC uses `is_looted` + `is_blocked` (separating "taken" from "rolling on it"). When rolls land, `is_blocked` clears and `is_looted` is set on the winner side.
- Represented `LootItemData::ui_type` now consumes the shared C++ decision helper for hidden, FFA, quest, round-robin, master, group/NBG and personal-loot rows. Remaining gap is canonical shared `Loot::BuildLootResponse` ownership across live sessions.
- Represented `LootResponse::loot_method` is now sourced from the represented loot view (#NEXT.R8.ENTITIES.239), and represented `LootResponse::acquire_reason` is now sourced from stored `loot_type` via the C++ `GetLootTypeForClient` mapping (#NEXT.R8.ENTITIES.298).
- `LootResponse::threshold` remains the C++ packet default 2 (`LootPackets.h`); group threshold belongs to loot-item `is_underthreshold`/roll startup, not this response byte. Remaining gaps are canonical shared roll ownership and complete threshold use at every source.
- `LootResponse` wire now supports C++ currency entries and count (#NEXT.R8.ENTITIES.236), but represented loot generation still does not populate currency rows.
- Canonical `LootError` reporting is still incomplete. `CMSG_LOOT_UNIT` was re-checked against C++ and now returns silently for invalid dead-player/non-creature/live/too-far targets and consumed/empty/not-allowed represented loot views (#NEXT.R8.ENTITIES.206/#NEXT.R8.ENTITIES.207/#NEXT.R8.ENTITIES.208/#NEXT.R8.ENTITIES.218); represented `CMSG_LOOT_ITEM` covers active creature loot too far with `LOOT_ERROR_TOO_FAR = 4` (#NEXT.R8.ENTITIES.201) and missing active creature owner with `LOOT_ERROR_NO_LOOT = 17` (#NEXT.R8.ENTITIES.202). Facing/locked/stunned/master-loot errors remain pending.
- Represented coins are consumed by `CMSG_LOOT_MONEY` and now emit `SMSG_COIN_REMOVED` before `SMSG_LOOT_MONEY_NOTIFY` (#NEXT.R8.ENTITIES.200), with represented near-group corpse-money notify splitting covered by #NEXT.R8.ENTITIES.234. Aura money modifiers, criteria, script hooks and other-session money persistence are still absent.

**Tests existing:**
- `wow-loot` now has focused foundation unit tests for `LootStoreItem::IsValid`-style guards and `LootTemplate::AddEntry` group splitting (#NEXT.R8.ENTITIES.270).
- 0 integration tests for loot in `wow-world`
- 0 tests against any of the 12 loot-template tables

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#LOOT.WBS.001** Partir y cerrar la migracion auditada de `game/Loot/Loot.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1083 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#LOOT.WBS.002** Cerrar la migracion auditada de `game/Loot/Loot.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#LOOT.WBS.003** Cerrar la migracion auditada de `game/Loot/LootItemStorage.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootItemStorage.cpp`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#LOOT.WBS.004** Cerrar la migracion auditada de `game/Loot/LootItemStorage.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootItemStorage.h`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#LOOT.WBS.005** Cerrar la migracion auditada de `game/Loot/LootItemType.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootItemType.h`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#LOOT.WBS.006** Partir y cerrar la migracion auditada de `game/Loot/LootMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1350 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#LOOT.WBS.007** Cerrar la migracion auditada de `game/Loot/LootMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.h`
  Rust target: `crates/wow-loot`, `crates/wow-core`, `crates/wow-constants`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, split).

- [ ] **#LOOT.1** Populate `crates/wow-loot/src/lib.rs` skeleton: `LootStoreItem`, `LootTemplate`, `LootGroup`, `LootStoreKind` and pure `LootStore` are started by `#NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.271`; still add/re-export canonical `LootItem`, `Loot` (server-side), `LootRoll`, `LootMethod`, `LootType`, `RollType`, `LootError`, `LootSlotType`, `LootMode` and runtime store registry ownership (L)
- [ ] **#LOOT.2** Define `LootStoreItem { itemid, reference, chance, lootmode, needs_quest, groupid, mincount, maxcount, conditions }` and `LootStoreItem::roll(rate: f32) -> bool` (L)
- [ ] **#LOOT.3** Define `LootTemplate { entries: Vec<LootStoreItem>, groups: Vec<LootGroup> }` with `process(loot, rate, loot_mode, group_id, personal_looter)` recursive over reference templates (M). Pure non-personal process foundation exists in `#NEXT.R8.ENTITIES.275`, pure personal-loot processing exists in `#NEXT.R8.ENTITIES.318`, and represented per-tapper condition context exists in `#NEXT.R8.ENTITIES.319`; canonical `Loot` ownership and reusable full `ConditionMgr` gates remain.
- [ ] **#LOOT.4** Define `LootGroup` with weighted-without-replacement single-pick semantics; `process_with_total_chance` matches TC `LootGroup::Process` (M). Pure explicit-before-equal group process foundation exists in `#NEXT.R8.ENTITIES.275`; real condition/player gates remain.
- [ ] **#LOOT.5** Define `LootStore { name, entry_name, rates_allowed, templates: HashMap<u32, LootTemplate> }` and 12 globals: `Creature`, `Disenchant`, `Fishing`, `Gameobject`, `Item`, `Mail`, `Milling`, `Pickpocketing`, `Prospecting`, `Reference`, `Skinning`, `Spell` (M). Foundation exists as pure metadata/row loader in `#NEXT.R8.ENTITIES.271`; runtime global/store-registry ownership remains.
- [ ] **#LOOT.6** SQL loaders: one per `*_loot_template` table → populate corresponding store; `Verify` + `CheckLootRefs` (verify `Reference > 0` resolves) (H — XL if including conditions join). Full-table SQL statement coverage and startup loading/injection exist for all 12 tables in `#NEXT.R8.ENTITIES.271/#NEXT.R8.ENTITIES.272`; reference verification exists in `#NEXT.R8.ENTITIES.273`; structural condition source/link validation exists in `#NEXT.R8.ENTITIES.274`; actual condition storage/evaluation remains.
- [ ] **#LOOT.7** Implement `Loot` struct with `items: Vec<LootItem>`, `gold`, `unlooted_count`, `loot_type`, `dungeon_encounter_id`, `loot_method`, `loot_master`, `round_robin_player`, `allowed_looters`, `rolls: HashMap<u32, LootRoll>`. Move ownership from session to `Creature::loot` / `GameObject::loot` (H). Represented `CreatureLoot` carries `unlooted_count`/`is_counted` bridge semantics through #NEXT.R8.ENTITIES.296 and represented `loot_type`/`dungeon_encounter_id` fields plus `GetLootTypeForClient` acquire-reason mapping through #NEXT.R8.ENTITIES.298. The represented dungeon-encounter ID now propagates through roll packets and direct/master/roll-winner `SendNewItem` item-push paths through #NEXT.R8.ENTITIES.299. Canonical `Creature`/`GameObject` now have shared/personal loot-summary shapes, represented release syncs summaries for `IsFullyLooted`-style decisions, and represented item/master-loot removal refreshes those summaries after the local `unlooted_count` decrement. Canonical shared `Loot` object ownership, real `m_personalLoot`, real `SetDungeonEncounterId` source callsites, AE result aggregation and criteria delivery remain.
- [ ] **#LOOT.8** Implement `Loot::fill_loot(loot_id, store, owner, personal, no_empty_error, loot_mode, context)` calling `LootTemplate::process` (M). Pure store materialization exists in `#NEXT.R8.ENTITIES.275`; basic generated-item metadata/context/random-property hook exists in `#NEXT.R8.ENTITIES.276`; real `Loot` owner/group access-right marking and handler wiring remain.
- [x] **#LOOT.9** Implement canonical `Loot::generate_money_loot(min, max)` honoring `Rate.Drop.Money` config in `wow-loot` (L). Pure helper `wow_loot::generate_money_loot_with_rate_like_cpp` is closed by #NEXT.R8.ENTITIES.291 and is consumed by represented `CMSG_OPEN_ITEM` item money loot and represented creature corpse money. Full canonical `Loot` ownership still remains under #LOOT.7/#LOOT.14.
- [ ] **#LOOT.10** `LootItem::allowed_for_player`: class/race/quest/condition/faction checks; `needs_quest` → only if player has matching active ITEM objective (M, depends on #QUESTS.5/8/9)
- [ ] **#LOOT.11** `LootItem::get_ui_type_for_player` → `LootSlotType` decision tree (M). Pure C++ decision helper and represented `LootResponse` item consumption are covered by #NEXT.R8.ENTITIES.292. Represented creature loot now carries `_lootMaster` and `roundRobinPlayer` equivalents through #NEXT.R8.ENTITIES.293, and represented per-player `GetPlayerFFAItems()` state through #NEXT.R8.ENTITIES.294; full closure still requires canonical shared `Loot` ownership and cross-session loot views.
- [ ] **#LOOT.12** Hook `Creature` death → `generate_loot_for_body` populating `m_loot` from `LootTemplates_Creature` (M)
- [ ] **#LOOT.13** Hook `GameObject::use` for chest / herb / ore / fishing-bobber / lockbox; per-type LootStore selection + skill check where applicable (M)
- [ ] **#LOOT.14** Replace per-session `loot_table` HashMap with `Creature::m_loot` / `GameObject::m_loot` so multiple sessions share the same Loot view (H — touches creature/gameobject types). Represented active viewer tracking now keeps `PlayersLooting` separate from `_allowedLooters` via #NEXT.R8.ENTITIES.295, but ownership is still session-backed.
- [ ] **#LOOT.15** `LootRoll` engine: `try_to_start`, `player_vote`, `update_roll` (1-minute timeout), `finish` (Need > Greed > Disenchant ranking, highest roll wins) (H)
- [ ] **#LOOT.16** Group-loot integration: `Group::GroupLoot`/`MasterLoot`/`NeedBeforeGreed` equivalents; `LootMethod` per group; threshold (`item_template.Quality >= threshold` enters roll) (H)
- [ ] **#LOOT.17** Master loot: `SMSG_MASTER_LOOT_CANDIDATE_LIST`, `CMSG_LOOT_MASTER_GIVE`, permission + uniqueness check (M)
- [ ] **#LOOT.18** Round-robin owner: track per-`Loot`; coin distribution to all eligible group members (M). Represented creature loot now stores and clears a round-robin owner for corpse group loot via #NEXT.R8.ENTITIES.293, but canonical shared `Loot` ownership and full coin distribution remain pending.
- [ ] **#LOOT.19** Fishing loot: `CMSG_USE_ITEM` on fishing bobber → roll skill, pick `LootTemplates_Fishing` by area; junk fallback (M)
- [ ] **#LOOT.20** Skinning: corpse flag after death, skill check, `LootTemplates_Skinning` (M)
- [ ] **#LOOT.21** Pickpocketing: rogue stealth+target check, `LootTemplates_Pickpocketing`, cooldown (M)
- [ ] **#LOOT.22** Disenchant: consume the `wow-data` `ItemDisenchantLoot.db2` store/resource from #NEXT.R8.ENTITIES.259/#NEXT.R8.ENTITIES.260, `ItemCurrencyCost.db2` from #NEXT.R8.ENTITIES.262, the `ItemClass`/price-field foundation from #NEXT.R8.ENTITIES.263, `ImportPrice*.db2` from #NEXT.R8.ENTITIES.264 and `ItemPriceBase.db2` from #NEXT.R8.ENTITIES.265; port the full C++ `Item::GetDisenchantLoot` gates (`CanDisenchant`, conjured/no-disenchant/quest-binding, area/map/max-stack, full `GetSellPrice`/currency-cost, class/subclass/quality/level/expansion), resolve `disenchant_loot_template` through `LootTemplates_Disenchant`, trigger on `ROLL_DISENCHANT` win, and apply grouped-roll `SkillRequired` masking (min skill 175 behavior comes from C++ roll mask eligibility) (M)
- [ ] **#LOOT.23** Prospecting / milling: trade-skill spells → consume reagents → `LootTemplates_Prospecting`/`Milling` (M)
- [ ] **#LOOT.24** Mail loot: `mail_loot_template` resolved at mail send/open (M, depends on Mail module)
- [ ] **#LOOT.25** Item loot containers (lockboxes, bags): `LootItemStorage` persistence in `item_loot_items` + `item_loot_money` (H)
- [ ] **#LOOT.26** Random properties / suffix generation (`ItemRandomProperties.dbc`, `ItemRandomSuffix.dbc`) on `LootItem` creation (M)
- [ ] **#LOOT.27** Canonical loot conditions: hook reusable `ConditionMgr::is_object_meeting_not_grouped_conditions` for each `LootStoreItem` across all loot stores. Partial bridges exist for represented `CMSG_OPEN_ITEM` item loot (#NEXT.R8.ENTITIES.192) and represented creature corpse loot (#NEXT.R8.ENTITIES.279); shared represented row/evaluator helpers live in `wow-loot` (#NEXT.R8.ENTITIES.280), condition-reference template expansion is covered for those bridges (#NEXT.R8.ENTITIES.281), startup diagnostics report missing condition-reference templates (#NEXT.R8.ENTITIES.282), self-referencing reference rows are skipped like C++ (#NEXT.R8.ENTITIES.283), represented ElseGroup evaluation now follows C++ order-independent map semantics (#NEXT.R8.ENTITIES.284), deterministic C++ load-invalid row skips are represented (#NEXT.R8.ENTITIES.285), class/race playable mask behavior is represented (#NEXT.R8.ENTITIES.286), comparison/target load skips are extended (#NEXT.R8.ENTITIES.287), remaining deterministic target/mask skips are represented (#NEXT.R8.ENTITIES.288), and legacy object/type-mask normalization plus represented player object/type-mask evaluation is covered (#NEXT.R8.ENTITIES.289). (M)
- [ ] **#LOOT.27A** Canonical `LootItem` metadata behavior: route `follow_loot_rules`, `freeforall`, `is_blocked`, `is_counted`, `is_underthreshold`, `needs_quest`, `allowedGUIDs` and `rollWinnerGUID` through group/master/NBG, threshold and per-player FFA views. Represented item-container persistence exists only for `CMSG_OPEN_ITEM` (#NEXT.R8.ENTITIES.193), with `FREE_FOR_ALL` response metadata covered by #NEXT.R8.ENTITIES.194, represented active-view guard by #NEXT.R8.ENTITIES.195, represented allowed-looter/blocked/roll-winner release-all refusal behavior by #NEXT.R8.ENTITIES.223, represented `LootResponse` allowed-looter visibility by #NEXT.R8.ENTITIES.197, represented per-player FFA removal by #NEXT.R8.ENTITIES.198 and represented `GetPlayerFFAItems()` rows by #NEXT.R8.ENTITIES.294. (M)
- [ ] **#LOOT.28** Full canonical `CMSG_LOOT_MONEY` handlers and `SMSG_LOOT_LIST` runtime fanout from canonical `Loot::NotifyLootList` state (M). Note: contrasted Trinity 3.4.3 has no `CMSG_LOOT_LIST` or `CMSG_AE_LOOT_TARGETS/ACK` handlers; represented server-driven AE target packets are covered by #NEXT.R8.ENTITIES.217. `CMSG_SET_LOOT_METHOD` is covered as a represented no-op for this C++ branch by #NEXT.R8.ENTITIES.231; mutable loot-method behavior would be a deliberate behavior change or a port from another branch. `CMSG_OPT_OUT_OF_LOOT` parser/dispatch/state is covered by #NEXT.R8.ENTITIES.232; represented `PlayersLooting` fanout and stale-looter pruning for item/money removal is covered by #NEXT.R8.ENTITIES.295; canonical `LootRoll` consumption remains pending.
- [ ] **#LOOT.29** Complete canonical `Player::StoreLootItem` parity for every loot source. Represented direct inventory pickup now exists for current loot entries (#NEXT.R8.ENTITIES.178/#NEXT.R8.ENTITIES.187/#NEXT.R8.ENTITIES.195/#NEXT.R8.ENTITIES.223), but full AE loot, group/master/NBG routing, per-player FFA, criteria/news, quest hooks and script side effects remain pending. (M)
- [ ] **#LOOT.30** Finish canonical money-loot ownership and call-site parity. Represented creature money now uses `creature_template_difficulty.GoldMin..GoldMax` plus `Rate.Drop.Money` and no longer fabricates fallback copper when `GoldMax == 0`; canonical shared `Loot`, exact `lootMode` gating and remaining money side effects are still pending. (L)
- [ ] **#LOOT.31** `LootError` reporting (`SMSG_LOOT_RESPONSE.failure_reason`) for every refusal path (`DIDNT_KILL`, `TOO_FAR`, `LOCKED`, `STUNNED`, `NO_LOOT`, master-only errors) (L)
- [ ] **#LOOT.32** Achievement criteria: `LOOT_ITEM`, `LOOT_TYPE`, `LOOT_MONEY` (M, depends on Achievements)
- [ ] **#LOOT.33** Script hooks: `on_loot_creature`, `on_loot_item`, `on_loot_money` (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#LOOT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 7 files / 3475 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-loot`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-core && cargo test -p wow-loot` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#LOOT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 7 files / 3475 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-loot`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#LOOT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 7 files / 3475 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-loot`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#LOOT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 7 files / 3475 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h`. Rust target: `crates/wow-constants`, `crates/wow-core`, `crates/wow-loot`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `creature_loot_template` fixture row drops `itemid` at exactly `chance%` (statistical: 10k samples → ±2σ)
- [ ] Test: a `Reference > 0` row resolves through `reference_loot_template` and returns its rolled item, never the literal `itemid=0`
- [ ] Test: `LootGroup` with N entries summing to 100% always picks exactly one
- [ ] Test: `LootGroup` with sum < 100% can produce zero items
- [ ] Test: `lootMode = HEROIC (0x2)` skips entries without `lootmode & 0x2`
- [ ] Test: `needs_quest = 1` row drops only for player with matching active ITEM objective
- [ ] Test: `LootItem::allowed_for_player` rejects warrior on a `AllowableClass` mage-only ref
- [ ] Test: `MAX_NR_LOOT_ITEMS = 18` cap enforced — 19th potential drop ignored
- [ ] Test: `Loot::generate_money_loot` produces value in `[min, max]` × `Rate.Drop.Money`
- [ ] Test: looting an item triggers `Player::store_loot_item` and inventory count increments by `LootItem.count`
- [x] Test: `CMSG_LOOT_RELEASE` after every item taken sets corpse decay and removes lootable flag (`creature_owned_loot_release_fully_consumed_uses_canonical_is_fully_looted_like_cpp`, `creature_owned_loot_release_fully_consumed_removes_lootable_dynflag_like_cpp`)
- [ ] Test: group loot with `GROUP_LOOT` method + uncommon threshold → uncommon item enters roll, common item is free pickup
- [ ] Test: `LootRoll` with all 5 members PASSing emits `SMSG_LOOT_ALL_PASSED` and item becomes free
- [ ] Test: `LootRoll` after 60s timeout treats unvoted as PASS; winner determined from voted set
- [ ] Test: NEED outranks GREED outranks DISENCHANT in winner selection regardless of roll number
- [ ] Test: master loot — non-master players see `LootSlotType::MASTER`; master sees `ALLOW_LOOT`
- [ ] Test: master-loot give to player with full inventory returns `LOOT_ERROR_MASTER_INV_FULL`
- [ ] Test: pickpocket on already-picked target returns `ALREADY_PICKPOCKETED`
- [ ] Test: round-robin coin split — gold = 600c, 3 group members → each gets 200c
- [x] Test: FFA item visible only to its player; non-eligible players see slot absent (`represented_ffa_loot_uses_player_ffa_items_like_cpp`)
- [ ] Test: random suffix items get distinct `random_property_id` per drop
- [ ] Test: `LootItemStorage` round-trip — open container, kill server, restart, container retains items
- [ ] Test: condition gate — quest-condition row drops only when player has active quest

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 7 files / 3475 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h` | `crates/wow-loot/` has the `#NEXT.R8.ENTITIES.270` `LootStoreItem`/`LootTemplate`/`LootGroup` foundation, `crates/wow-packet/src/packets/loot.rs` covers wire, and `crates/wow-world/src/handlers/loot.rs` has represented session handlers. Full C++ loot remains in scope; canonical loaders/runtime are still incomplete. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#LOOT.DIV.001` | `crates/wow-loot` has `LootStoreItem`/`LootTemplate`/`LootGroup`/pure `LootStore` foundation, reference verification, structural loot-condition diagnostics, pure fill materialization, basic generated item metadata, and `world-server` loads/injects stores | 7 C++ files / 3475 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h` | `partial_foundation` | `#NEXT.R8.ENTITIES.270/#NEXT.R8.ENTITIES.276` removed the empty-crate, no-loader, no-reference-verification, no-structural-condition-diagnostic, no-pure-fill and no-basic-lootitem-metadata divergences, but canonical `Loot`, `LootRoll`, condition evaluation and gameplay runtime wiring are still absent. |
| `#LOOT.DIV.002` | `crates/wow-loot/src/lib.rs` has tested foundation structs/store loader, reference checks, structural condition link checks, pure fill materialization and basic generated item metadata only | 7 C++ files / 3475 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Loot/LootMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Loot/Loot.h` | `partial_foundation` | File is no longer empty; remaining divergence is the missing canonical runtime beyond pure store materialization, reference/condition diagnostics, generated item metadata and startup injection. |

<!-- REFINE.023:END known-divergences -->

- **`LOOT_ROLL_TIMEOUT = 60s`** is wire-implicit; client expects winner before then or it gets confused. Don't extend.
- **`MAX_NR_LOOT_ITEMS = 18`** — the loot window cannot show more than 18 items + 1 money slot (slot 19 reserved). Excess drops are silently dropped on the server side.
- **FFA vs group loot.** `LootItem::freeforall` rows go in a `NotNormalLootItem` per-player map, not the shared `items[]`. Each player sees their own copy with its own `is_looted` flag. A common bug is treating FFA items as global → one player loots, others can't see it anymore.
- **Loot threshold gate.** Items below `Group::GetLootThreshold` (default Uncommon=2) bypass group rolls and become free-pickup. The threshold is per-group and per-item-quality (`item_template.Quality`). Check `LootItem::is_underthreshold` flag before queuing into `LootRoll`.
- **Master loot permission.** Only the `_lootMaster` GUID can send `CMSG_LOOT_MASTER_GIVE`. Validating this on the server is critical — the client sends the GUID but a malicious client can spoof. Cross-check against `Group::GetMasterLooterGuid`.
- **Item link broadcast.** When a NEED roll is won, TC emits `SMSG_LOOT_ROLL_WON` to the entire group, but the *source* of `SMSG_LOOT_REMOVED` is per-player (only those who could see the slot). For above-threshold items, the slot disappears for everyone. For below-threshold (free-pick), only the looter's view updates.
- **`needs_quest` items consume an objective slot.** When loot is generated for a creature, only items whose `needs_quest=1` AND `LootItem::allowed_for_player` matches an active objective are added. Re-evaluate per-player at view time (multiple party members may need the same item; each gets an independent FFA copy).
- **Reference recursion can loop.** TC has an explicit `LootStoreItem::IsValid` + `LootStore::CheckLootRefs` pass at startup that detects cycles. Reproduce or risk infinite recursion at draw time.
- **`lootMode` is a bitmask.** Difficulty selection (10/25 normal vs heroic) maps to 0x1, 0x2, 0x4, 0x8 etc. via `MapDifficulty.dbc`. Multiple bits can be set simultaneously (e.g. RDF dungeons use a special mode).
- **Drop rate config.** `Rate.Drop.Item.Poor`, `Normal`, `Uncommon`, `Rare`, `Epic`, `Legendary`, `Artifact`, `Heirloom`, `Referenced` — multiplies per-quality. `Rate.Drop.Money` multiplies coin amount. Don't hardcode rates.
- **Quest-required gameobjects.** Some quest items ONLY drop from gameobjects with `gameobject_questrelation`. Treat as `gameobject_loot_template` + condition, not creature.
- **Personal loot ≠ FFA.** `PERSONAL_LOOT` (raid finder) generates one independent `Loot*` per player at draw time; `FREE_FOR_ALL` shares one `Loot` but partitions visibility per `freeforall` flag. The two paths must not be conflated.
- **`SMSG_LOOT_RESPONSE` differs per viewer.** Build it via `BuildLootResponse(packet, viewer)`, not once and broadcast. Different players see different items based on FFA, master, threshold, conditions.
- **Skinning is a state transition** on the corpse (`CREATURE_FLAG_LOOTABLE` → `CREATURE_FLAG_SKINNABLE`); it can only happen *after* the looter releases the corpse. Track separate "loot-released" timestamps to know when skinning is unlocked.
- **Pickpocketing** spawns a *separate* `Loot` instance (`LOOT_PICKPOCKETING`) that does not consume the corpse drops; the same creature can be killed and have its real loot rolled. Each pickpocket cooldown is per-creature, not per-player.
- **Loot in containers persists across server restart** via `LootItemStorage`. If you forget the persistence, players who close their bag with rolling items mid-progress lose them at restart.
- **`TC LootItem(LootStoreItem)` ctor generates the random property/suffix** — this happens once at draw, not at view. Multiple viewers see the same suffix.
- **`AELootResult`** — modern auto-loot batches multiple corpses into one window; deduping by item GUID matters because the same dungeon-encounter ID may attribute the same drop to multiple kills.
- **`is_counted`** — internal flag preventing double-counting/decrement of `unlootedCount`; represented open/pickup bridge semantics are covered by #NEXT.R8.ENTITIES.296 and represented master-loot/roll-winner decrement by #NEXT.R8.ENTITIES.297, while canonical roll-loss/mail fallback paths remain loot work.
- **`is_blocked`** — set while a roll is pending; once roll concludes it clears regardless of winner. Don't auto-loot blocked items.
- **`creature_template.skinloot = 0`** still allows skinning if `creature_template.type_flags & CREATURE_TYPEFLAGS_SKINNABLE` is set (skinning skill produces leather scraps from a default fallback).
- **Conditions on loot are LIVE** — condition fulfilment is checked at `Loot::FillLoot` time AND at view time. A player whose quest expires mid-loot-window sees the item disappear.
- **`LootMethod::PERSONAL_LOOT`** in 3.4.3 is mostly LFR/LFD only; classic dungeons use Group Loot or Free For All by default.
- **Coin display rounding.** Wire format is raw copper; the client formats gold/silver/copper itself. Don't pre-divide.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `struct Loot` | `struct Loot` (`crates/wow-loot/src/loot.rs`) | One per Creature/GameObject, NOT per session |
| `struct LootItem` | `struct LootItem` | Same field shape; bitfields → individual `bool` |
| `struct LootStoreItem` | `struct LootStoreItem` | Already row-shaped, easy port |
| `class LootStore` (12 globals) | `struct LootStore` × 12 in `OnceLock<RwLock<LootStore>>` | Avoid raw mutable globals |
| `class LootTemplate` (with private nested `LootGroup`) | `struct LootTemplate { entries: Vec<LootStoreItem>, groups: Vec<LootGroup> }` | Drop nested-class; `LootGroup` is a sibling type |
| `class LootRoll` | `struct LootRoll { vote_map: HashMap<ObjectGuid, PlayerRollVote>, end_time: Instant, … }` | Drive from `Loot::update(diff_ms)` tick |
| `enum LootMethod` (uint8) | `#[repr(u8)] enum LootMethod { Ffa=0, RoundRobin=1, Master=2, Group=3, NeedBeforeGreed=4, Personal=5 }` | — |
| `enum LootType` (uint8) | `#[repr(u8)] enum LootType { Corpse=1, … }` | Skip 0; preserve gaps (7, 10–13, 15–19) |
| `enum RollType` / `enum class RollVote` | unify into `enum RollVote { Pass, Need, Greed, Disenchant, Transmog }` plus `RollState` enum for not-yet-voted | — |
| `EnumFlag<RollMask>` | `bitflags! struct RollMask: u8` | — |
| `Loot::FillLoot(...)` | `fn fill_loot(&mut self, loot_id: u32, store: &LootStore, owner: &Player, personal: bool, no_empty_error: bool, loot_mode: u16, context: ItemContext) -> bool` | Port one-to-one |
| `LootTemplate::Process(...)` recursive | `fn process(&self, loot: &mut Loot, rate: bool, loot_mode: u16, group_id: u8, personal_looter: Option<&Player>, store: &LootStore)` | Pass `store` to resolve refs |
| `LootStoreItem::Roll(bool rate)` | `fn roll(&self, rate: f32) -> bool` | Use owner/session-provided RNG where the runtime owner exists; avoid ad-hoc `thread_rng()` in gameplay paths |
| `std::unordered_map<ObjectGuid, std::unique_ptr<NotNormalLootItemList>>` | `HashMap<ObjectGuid, Vec<NotNormalLootItem>>` | No need for unique_ptr in Rust |
| `Creature::m_loot: std::unique_ptr<Loot>` | `creature.loot: Option<Loot>` | Lazy creation; `take()` on release |
| `LootItemStorage` singleton | `static LOOT_ITEM_STORAGE: OnceLock<RwLock<LootItemStorage>>` | Same pattern as quest pools |
| `WorldPackets::Loot::LootResponse` | `LootResponse` (already in `wow-packet/src/packets/loot.rs`) | Build per-viewer in handler |
| `ConditionsReference` per `LootStoreItem` | `Arc<Vec<Condition>>` or `ConditionRef` (depends on Conditions module shape) | Tied to #LOOT.27 |
| `urand(min, max)` | `rand::Rng::gen_range(min..=max)` | Inclusive range to match TC |

---

*Generated: 2026-05-01. Mark `Audited vs C++ : ✅` only after a side-by-side audit of `Loot::FillLoot` + `LootTemplate::Process` + `LootRoll::Finish` against the Rust port. The current Rust impl is a stub; the real work begins by populating `crates/wow-loot/src/lib.rs`.*

---

## 13. Audit (2026-05-01)

Side-by-side audit of `crates/wow-loot/src/lib.rs` + `crates/wow-world/src/handlers/loot.rs` vs `src/server/game/Loot/{Loot.cpp,LootMgr.cpp}` + `Handlers/LootHandler.cpp` + `Player::StoreLootItem`.

### Flagged divergence — updated verdict

The original 2026-05-01 audit correctly flagged `CMSG_LOOT_ITEM` as silent-loss at that time. That specific represented direct-pickup gap is now closed for current loot entries: Rust creates/stacks inventory items, persists represented random-property/context metadata, checks active loot view (#NEXT.R8.ENTITIES.195), and now sends C++ `SMSG_LOOT_RELEASE_ALL` for represented blocked, disallowed-looter and non-winning-roll refusal branches (#NEXT.R8.ENTITIES.223). This is still not full Trinity `Player::StoreLootItem`: AE loot, group/master/NBG roll routing, per-player FFA state, criteria/news hooks and script side effects remain open.

### Crate state

- `crates/wow-loot/src/lib.rs` — **active represented/canonical-foundation port** (2479 lines as of the 2026-05-10 stabilization checkpoint). It contains the contrasted loot-store/template/group/fill foundation, represented loot-condition helpers, startup diagnostics and generated-loot metadata used by the current bridge. This is no longer an empty stub, but it is still not full Trinity parity: canonical shared `Loot` ownership, full `LootRoll`, reusable global `ConditionMgr`, all-or-mail fallback and every object-specific runtime side effect remain open.
  - `crates/wow-world/src/handlers/loot.rs` — registers and implements represented `LootUnit`, `LootItem`, `LootMoney`, `LootRelease`, plus represented `LootRoll`/`MasterLootItem` dispatch guards (#NEXT.R8.ENTITIES.229). `GroupInfo` carries represented `master_looter_guid` and `PartyUpdate` emits `LootMaster` only for `MASTER_LOOT` (#NEXT.R8.ENTITIES.230). `CMSG_SET_LOOT_METHOD` is registered/parsed/dispatched as a no-op matching this C++ branch (#NEXT.R8.ENTITIES.231). `CMSG_OPT_OUT_OF_LOOT` stores represented pass-on-group-loot state (#NEXT.R8.ENTITIES.232). Packet structs exist for `LootRoll`/`MasterLootItem` and related SMSG roll packets (#NEXT.R8.ENTITIES.228), `SMSG_LOOT_LIST` serialization is represented (#NEXT.R8.ENTITIES.233), current-session self-target master-loot `CanStoreItem` errors are represented (#NEXT.R8.ENTITIES.238), represented master-loot give now checks the loot view method (#NEXT.R8.ENTITIES.239), represented loot-level allowed-looter state is checked (#NEXT.R8.ENTITIES.240), current-session self-target gives store/remove represented loot (#NEXT.R8.ENTITIES.241), connected remote target gives route through target session commands (#NEXT.R8.ENTITIES.242) with finite unavailable-target waits (#NEXT.R8.ENTITIES.243), represented first-open master-loot candidate-list fanout now follows C++ `Player::SendLoot`/`Loot::OnLootOpened` order (#NEXT.R8.ENTITIES.244), and represented loot-list fanout now sends `SMSG_LOOT_LIST` to connected same-map allowed looters (#NEXT.R8.ENTITIES.245), but there is still no canonical `LootRoll` engine or C++ `Unit.cpp` creature-death `Loot::NotifyLootList` hook.
- `WorldSession.loot_table` is a `HashMap<ObjectGuid, CreatureLoot>` per session (loot.rs:99-104), not shared on the creature; group loot impossible by construction.

### Coverage matrix — C++ → Rust

| C++ symbol | Rust | Verdict |
|---|---|---|
| `class Loot`, `LootItem`, `LootStoreItem`, `LootStore`, `LootTemplate`, `LootGroup`, `LootRoll`, `AELootResult`, `LootItemStorage` | none | ❌ all missing |
| 12 `LootStore` globals (Creature/Disenchant/Fishing/Gameobject/Item/Mail/Milling/Pickpocketing/Prospecting/Reference/Skinning/Spell) | none | ❌ |
| `LoadLootTemplates_*` (12 SQL loaders) | none | ❌ |
| `LootStoreItem::Roll` (chance roll) | none | ❌ |
| `LootTemplate::Process` (recursive ref resolution) | none | ❌ |
| `LootGroup` weighted single-pick | none | ❌ |
| `Loot::FillLoot` | represented creature/gameobject/item/fishing paths consume the `wow-loot` foundation in selected call sites, but canonical shared `Loot::FillLoot` ownership is not yet wired | ⚠️ partial |
| `Loot::generateMoneyLoot` honoring `Rate.Drop.Money` | represented `CMSG_OPEN_ITEM`, creature corpse and gameobject chest money use the C++-shaped helper with session-owned RNG when template min/max rows exist; represented creature money now stays zero when `GoldMax == 0` like C++; canonical shared `Loot` ownership and exact kill-time `lootMode` call-site parity are still pending | ⚠️ partial |
| `LootItem::AllowedForPlayer` (class/quest/race/condition) | represented `CMSG_OPEN_ITEM` item-loot bridge covers quest/status and a player-condition subset; canonical `wow-loot` still missing | ⚠️ partial |
| `LootItem` metadata (`follow_loot_rules`, FFA, blocked/counted/under-threshold, needs_quest) | represented `CMSG_OPEN_ITEM` item-loot bridge computes/preserves metadata via #NEXT.R8.ENTITIES.193, applies `FREE_FOR_ALL` `LootResponse` view metadata via #NEXT.R8.ENTITIES.194, filters represented response rows by `allowedGUIDs` via #NEXT.R8.ENTITIES.197 and keeps represented FFA removal per player via #NEXT.R8.ENTITIES.198; canonical group/master/NBG behavior still missing | ⚠️ partial |
| `Loot::BuildLootResponse` per-viewer | one global view; represented `loot_method` is stored on the loot view via #NEXT.R8.ENTITIES.239, represented `threshold=2` matches the C++ `LootResponse` packet default, represented `FREE_FOR_ALL` item UI type is covered by #NEXT.R8.ENTITIES.194, the first allowed-looter visibility gate by #NEXT.R8.ENTITIES.197, C++ owner semantics by #NEXT.R8.ENTITIES.212, C++ `LootItemData` packet shape by #NEXT.R8.ENTITIES.213, separate represented creature `LootObject` identity by #NEXT.R8.ENTITIES.214 and C++ `LootCurrency` packet shape by #NEXT.R8.ENTITIES.236; canonical per-player group/master/NBG view behavior and runtime currency loot rows are still not generated | ⚠️ partial |
| `HandleAutostoreLootItemOpcode` → `StoreLootItem` | represented direct inventory store path exists for current loot entries, with active-view guards covered by #NEXT.R8.ENTITIES.195, represented blocked/disallowed-looter/non-winning-roll `SMSG_LOOT_RELEASE_ALL` refusal branches by #NEXT.R8.ENTITIES.223, represented creature-distance `LOOT_ERROR_TOO_FAR` by #NEXT.R8.ENTITIES.201, represented missing-creature `LOOT_ERROR_NO_LOOT` by #NEXT.R8.ENTITIES.202, represented missing-gameobject release by #NEXT.R8.ENTITIES.203, represented `LootObject` request lookup by #NEXT.R8.ENTITIES.214 and secondary active loot-object lookup by #NEXT.R8.ENTITIES.215; full AE loot target discovery, group eligibility broadcasts, per-player FFA and criteria/news side effects still missing | ⚠️ partial |
| `HandleLootMoneyOpcode` | represented active view exists, including `SMSG_COIN_REMOVED` before `SMSG_LOOT_MONEY_NOTIFY` via #NEXT.R8.ENTITIES.200, stale-view no-op semantics via #NEXT.R8.ENTITIES.209, zero-money notification behavior via #NEXT.R8.ENTITIES.210, represented `LootObject` notification identity via #NEXT.R8.ENTITIES.214, multi-entry active-view iteration via #NEXT.R8.ENTITIES.215 and represented near-group corpse-money notify split via #NEXT.R8.ENTITIES.234; other-session money persistence, aura modifiers, criteria and script behavior still missing | ⚠️ partial |
| `HandleLootMasterGiveOpcode` | parser and represented dispatch guard exist via #NEXT.R8.ENTITIES.228/#NEXT.R8.ENTITIES.229; represented `master_looter_guid` and `PartyUpdate.LootMaster` wiring exists via #NEXT.R8.ENTITIES.230; represented target-not-found and target-ineligible error paths exist via #NEXT.R8.ENTITIES.235; current-session self-target `CanStoreItem` error mapping exists via #NEXT.R8.ENTITIES.238; non-master loot views return silently via #NEXT.R8.ENTITIES.239; loot-level allowed-looter rejection exists via #NEXT.R8.ENTITIES.240; current-session self-target store/removal exists via #NEXT.R8.ENTITIES.241; connected remote target session CanStore/Store routing exists via #NEXT.R8.ENTITIES.242; finite unavailable-target command handling exists via #NEXT.R8.ENTITIES.243; represented first-open master-loot candidate-list fanout exists via #NEXT.R8.ENTITIES.244; canonical LootRoll/candidate fanout and live DB-backed remote success validation remain pending | ⚠️ partial |
| `HandleLootRoll` (CMSG_LOOT_ROLL) | parser and represented dispatch exist via #NEXT.R8.ENTITIES.228/#NEXT.R8.ENTITIES.229; represented first-open group/NBG loot starts `SMSG_START_LOOT_ROLL` prompts and handles the single-candidate under-threshold fallback via #NEXT.R8.ENTITIES.246; current-session valid votes broadcast immediate `SMSG_LOOT_ROLL` packets via #NEXT.R8.ENTITIES.247; represented all-voted state can select a Need-over-Greed winner, unblock the item, store `roll_winner` and emit `SMSG_LOOT_ROLL_WON` via #NEXT.R8.ENTITIES.248; represented all-pass closure matches C++ no-extra-packet behavior for valid pass voters via #NEXT.R8.ENTITIES.249; owner-session remote vote command foundation exists via #NEXT.R8.ENTITIES.250; represented remote `CMSG_LOOT_ROLL` owner-session routing exists via #NEXT.R8.ENTITIES.251; represented timeout expiry closes the current winner via #NEXT.R8.ENTITIES.252; represented final value replay before `SMSG_LOOT_ROLL_WON` exists via #NEXT.R8.ENTITIES.253; represented Need/Greed/Disenchant stored roll RNG now uses session-owned `urand(1, 100)` semantics and Pass stores no roll number like C++ `LootRoll::PlayerVote` via #NEXT.R8.ENTITIES.254; represented criteria hook points exist via #NEXT.R8.ENTITIES.255; represented NotValid/pass-on startup state exists via #NEXT.R8.ENTITIES.256; real canonical shared roll ownership, ConditionMgr/AllowedForPlayer invalid checks, achievements backend integration and winner item storage still missing | ⚠️ partial |
| `HandleSetLootMethod` | parser/registration/dispatch exists via #NEXT.R8.ENTITIES.231 and intentionally performs no mutation/no response because the contrasted C++ branch comments out the implementation under `not allowed to change` | ✅ represented |
| `HandleSetLootSpecialization` | parser/registration/dispatch exists via #NEXT.R8.ENTITIES.237; represented state clears on `SpecID=0` and accepts nonzero only when loaded `ChrSpecialization.db2` has a row with `ClassID == player_class`; full update-field/DB persistence and loot-specialization consumers remain pending | ⚠️ partial |
| `HandleOptOutOfLoot` | parser/registration/dispatch exists via #NEXT.R8.ENTITIES.232; loaded sessions store represented `pass_on_group_loot`, unloaded sessions ignore it like C++; represented first-open roll prompting consumes the current-session flag via #NEXT.R8.ENTITIES.246, while canonical persisted `LootRoll` vote initialization remains pending | ⚠️ partial |
| `SMSG_LOOT_LIST` / `Loot::NotifyLootList` | serializer present via #NEXT.R8.ENTITIES.233; represented same-map allowed-looter fanout with optional master over-threshold bit exists via #NEXT.R8.ENTITIES.245; canonical C++ creature-death creation hook and round-robin winner state still require shared Loot ownership | ⚠️ partial |
| `HandleLootReleaseOpcode` | represented active-view guard ignores spoofed release GUIDs via #NEXT.R8.ENTITIES.204, keeps unlooted represented object loot rows on release via #NEXT.R8.ENTITIES.205, emits C++-ordered `SMSG_LOOT_RELEASE` with player owner via #NEXT.R8.ENTITIES.211 and accepts secondary active owners via #NEXT.R8.ENTITIES.216; fully-looted represented creature release now uses `Rate.Corpse.Decay.Looted * m_corpseDelay`, honors `ignore_corpse_decay_ratio`, and despawns fully-looted `LOOT_SKINNING` immediately like `Creature::AllLootRemovedFromCorpse`; full canonical GO/corpse/item side effects remain pending | ⚠️ partial |
| `HandleLootOpcode` (CMSG_LOOT_UNIT) | represented invalid live/too-far targets return silently via #NEXT.R8.ENTITIES.206, consumed/empty/not-allowed represented loot views return silently via #NEXT.R8.ENTITIES.207, non-creature/non-vehicle GUIDs return silently via #NEXT.R8.ENTITIES.208, dead-player opens return silently via #NEXT.R8.ENTITIES.218, represented creature loot emits a separate `HighGuid::LootObject` via #NEXT.R8.ENTITIES.214, successful represented opens interrupt active non-melee casts via #NEXT.R8.ENTITIES.219, remove represented looting-interrupt auras via #NEXT.R8.ENTITIES.220, release existing non-item main loot views before opening a new main view via #NEXT.R8.ENTITIES.221, emit first-open master-loot candidate lists after the response via #NEXT.R8.ENTITIES.244, fan out represented `SMSG_LOOT_LIST` via #NEXT.R8.ENTITIES.245, and start represented first-open group/NBG loot-roll prompts via #NEXT.R8.ENTITIES.246 | ⚠️ partial |
| `LootError` failure codes (15+ values) | represented `CMSG_LOOT_ITEM` covers active creature loot `LOOT_ERROR_TOO_FAR=4` via #NEXT.R8.ENTITIES.201 and missing active creature owner `LOOT_ERROR_NO_LOOT=17` via #NEXT.R8.ENTITIES.202; `CMSG_LOOT_UNIT` invalid/consumed/not-allowed behavior is silent per #NEXT.R8.ENTITIES.206/#NEXT.R8.ENTITIES.207; facing/locked/stunned/master-loot errors still missing | ⚠️ partial |
| `LootItemStorage` (item-loot persistence) | represented `CMSG_OPEN_ITEM` item-loot load/save for item money and rows exists; item-container chance/group/count rolls use session-owned RNG; full reusable item-loot storage remains pending | ⚠️ partial |
| `ItemRandomProperties.dbc`/`ItemRandomSuffix.dbc` rolls | represented store-time random property/suffix generation exists for `CMSG_LOOT_ITEM`; full canonical loot-item creation integration remains pending | ⚠️ partial |

### Other observed bugs

- Represented `CMSG_LOOT_UNIT` no longer uses stale `failure_reason: 2`; after re-checking C++ it now returns silently for invalid dead-player/non-creature/live/too-far targets and consumed/empty/not-allowed represented loot views (#NEXT.R8.ENTITIES.206/#NEXT.R8.ENTITIES.207/#NEXT.R8.ENTITIES.208/#NEXT.R8.ENTITIES.218), interrupts active non-melee casts on successful represented loot opens (#NEXT.R8.ENTITIES.219), removes represented visible auras with `SpellAuraInterruptFlags::Looting` (#NEXT.R8.ENTITIES.220), uses the represented loot owner instead of the player for `LootResponse.Owner` (#NEXT.R8.ENTITIES.212), and emits a separate represented `HighGuid::LootObject` as `LootResponse.LootObj` (#NEXT.R8.ENTITIES.214). Represented `CMSG_LOOT_ITEM` now emits `LOOT_ERROR_TOO_FAR = 4` for active creature loot beyond 30 yards (#NEXT.R8.ENTITIES.201), `LOOT_ERROR_NO_LOOT = 17` when the active creature owner is missing (#NEXT.R8.ENTITIES.202), `SMSG_LOOT_RELEASE` when the active gameobject owner is missing (#NEXT.R8.ENTITIES.203), uses the C++ `LootObj` then player `Owner` payload shape (#NEXT.R8.ENTITIES.211), resolves represented creature loot-object requests back to the owner before store/removal (#NEXT.R8.ENTITIES.214), and can resolve secondary represented active loot objects (#NEXT.R8.ENTITIES.215). Represented `CMSG_LOOT_MONEY` sends C++-shaped `SMSG_COIN_REMOVED` before money notify (#NEXT.R8.ENTITIES.200), uses the represented `LootObject` in that packet (#NEXT.R8.ENTITIES.214), iterates all represented active loot views (#NEXT.R8.ENTITIES.215), and sends represented corpse-money shares to near group members with C++ integer division/SoleLooter semantics (#NEXT.R8.ENTITIES.234). Full `LootError`, other-session money persistence, aura modifiers, criteria and script coverage remains under canonical loot work.
- Represented `CMSG_LOOT_RELEASE` now ignores spoofed/non-active release GUIDs instead of acknowledging unknown loot (#NEXT.R8.ENTITIES.204), closing an unlooted represented object no longer deletes its loot row (#NEXT.R8.ENTITIES.205), release can target a secondary represented active owner without clearing the rest of the active view (#NEXT.R8.ENTITIES.216), opening a new represented non-AE main loot view releases existing non-item main active views first (#NEXT.R8.ENTITIES.221), represented logout request sends `SMSG_LOOT_RELEASE_ALL` before `LogoutResponse` (#NEXT.R8.ENTITIES.225), represented logout calls the shared release-all cleanup before clearing player state (#NEXT.R8.ENTITIES.224), represented disconnect cleanup calls the same helper before shared runtime cleanup (#NEXT.R8.ENTITIES.226), and represented moved-bag `SwapItem` cleanup calls it when the bag contains active item loot (#NEXT.R8.ENTITIES.227). `SMSG_LOOT_RELEASE_ALL` also covers represented `StoreLootItem` refusal branches (#NEXT.R8.ENTITIES.223), while broader release-all callsites, round-robin reset and canonical object-specific release side effects remain pending.
- `loot.rs:99-104` — loot is generated lazily inside `WorldSession`; if two players try to loot the same corpse, each sees a fresh independent roll — duplicated drops, no FFA semantics.
- Represented creature money no longer uses the old GUID/level fallback when `GoldMax == 0`; remaining money gaps are canonical shared `Loot` ownership, exact kill-time `lootMode` gating and non-creature money side effects.
- Represented fully-looted creature release no longer hardcodes 30s; it consumes the creature corpse delay and C++ `Rate.Corpse.Decay.Looted`. Full canonical corpse/remove/respawn lifecycle remains under runtime/entity ownership work.

**Verdict:** canonical loot remains incomplete but is no longer mostly absent. `crates/wow-loot/` has a contrasted `LootStoreItem`/`LootTemplate`/`LootGroup`/`LootStore` foundation plus startup reference/condition diagnostics, pure fill materialization, represented condition evaluation and generated item metadata. The old `CMSG_LOOT_ITEM` silent-loss finding is no longer true for the represented direct inventory path, but #LOOT.1-#LOOT.33 still track the complete Trinity loot system needed to close C++ parity.
