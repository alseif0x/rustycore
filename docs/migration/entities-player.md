# Migration: Entities / Player

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/`
> **Rust target crate(s):** `crates/wow-world/` (currently flat fields on `WorldSession`), proposed `crates/wow-entities/` (does not exist), partials in `crates/wow-database/`, `crates/wow-data/`, `crates/wow-spell/`, `crates/wow-loot/`
> **Layer:** L4 (Entity layer; subclass of `Unit` from `entities-unit.md`)
> **Status:** 🔧 broken (rewrite needed) — `Player` as a class **does not exist** in RustyCore. Per-character state is exploded into ~70+ flat fields on `WorldSession` (`crates/wow-world/src/session.rs:65-309`). There is no `Player::Update()` aggregation, no `LoadFromDB`/`SaveToDB`, no equip/store/inventory pipeline beyond a `HashMap<u8, InventoryItem>`, no talent/spec/glyph state, no skill state object, no rest manager, no taxi, no trade data, no scene manager, no cinematic manager, no kill rewarder, no equipment sets. The C# legacy port had `Player` split across many partial files; the Rust rewrite has not begun the entity model.
> **Audited vs C++:** ✅ complete (2026-05-01) — header-by-header inventory of `Player.h` (3189 lines) + spot audit of `Player.cpp` (29358 lines) and the eight neighbour managers
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Player` is the largest single class in TrinityCore: ~32500 LOC across `Player.h` + `Player.cpp` plus eight sibling managers (Cinematic, Collection, KillRewarder, RestMgr, SceneMgr, SocialMgr, TradeData, PlayerTaxi). It is the user-controlled `Unit` subclass that owns inventory, quests, reputations, skills, talents/specs/glyphs, spells, achievements, mail, social/friends, group state, BG/arena state, instance binds, taxi paths, played-time, played-rest accumulation, the `~50` sub-systems ticked from `Player::Update()`, and the *entire* `LoadFromDB` / `SaveToDB` persistence pipeline (~1500 lines each). Everything the client knows about its own avatar lives here. Without a faithful `Player` entity, every gameplay system above L4 (combat, spells, quests, loot, chat, social, BG, dungeons) has nowhere to attach state.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Player/CUFProfile.h` | 116 | `prefix` |
| `game/Entities/Player/CinematicMgr.cpp` | 178 | `prefix` |
| `game/Entities/Player/CinematicMgr.h` | 63 | `prefix` |
| `game/Entities/Player/CollectionMgr.cpp` | 939 | `prefix` |
| `game/Entities/Player/CollectionMgr.h` | 175 | `prefix` |
| `game/Entities/Player/EquipmentSet.h` | 71 | `prefix` |
| `game/Entities/Player/KillRewarder.cpp` | 304 | `prefix` |
| `game/Entities/Player/KillRewarder.h` | 59 | `prefix` |
| `game/Entities/Player/Player.cpp` | 29358 | `prefix` |
| `game/Entities/Player/Player.h` | 3189 | `prefix` |
| `game/Entities/Player/PlayerTaxi.cpp` | 229 | `prefix` |
| `game/Entities/Player/PlayerTaxi.h` | 95 | `prefix` |
| `game/Entities/Player/RestMgr.cpp` | 172 | `prefix` |
| `game/Entities/Player/RestMgr.h` | 92 | `prefix` |
| `game/Entities/Player/SceneDefines.h` | 37 | `prefix` |
| `game/Entities/Player/SceneMgr.cpp` | 247 | `prefix` |
| `game/Entities/Player/SceneMgr.h` | 87 | `prefix` |
| `game/Entities/Player/SocialMgr.cpp` | 313 | `prefix` |
| `game/Entities/Player/SocialMgr.h` | 163 | `prefix` |
| `game/Entities/Player/TradeData.cpp` | 153 | `prefix` |
| `game/Entities/Player/TradeData.h` | 90 | `prefix` |
| `game/Entities/Taxi/TaxiPathGraph.cpp` | 258 | `prefix` |
| `game/Entities/Taxi/TaxiPathGraph.h` | 34 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/`.

The Player module is **not split per-feature** in this branch (unlike the C# port `Player.QuestHandler.cs`/`Player.SpellMods.cs`/`Player.Stats.cs`). It is one monolith with neighbour managers.

| File | Lines | Purpose |
|---|---|---|
| `Player.h` | 3189 | `Player` class declaration; enums (`PlayerFlags`, `PlayerFlagsEx`, `PlayerLoginQueryIndex`, `PlayerDelayedOperations`, `PlayerCommandStates`, `CharDeleteMethod`, `PlayerCreateMode`, `PlayerSpellState`, `EnchantDuration`, `MirrorTimerType`, `ReputationSource`, `PlayerLoginErrorCode`); ~600 inline accessors; equipment/inventory slot constants (`PLAYER_SLOT_*`, `EQUIPMENT_SLOT_*`, `INVENTORY_SLOT_BAG_0`); `MAX_PLAYER_LOGIN_QUERY = 47` |
| `Player.cpp` | 29358 | The monolith: `Update()` at line 909, `LoadFromDB()` at 17066, `SaveToDB()` at 19318/19329, all inventory pipelines, all quest mutators, talent/spec/glyph application, spell book, skill table, reputation manager, mail polling, BG state, instance lock state, AFK reporting, anti-cheat speed/duel-distance checks, taxi launch, transmog, void storage, currency, trade window, rune state (DK), eclipse (Druid moonkin), holy power (Paladin), shadow orbs (Priest), soul shards (Warlock); battleground entry/exit; group sync packets |
| `CinematicMgr.h` | 63 | `CinematicMgr` — owned by `Player` (`Player::_cinematicMgr`), tracks active cinematic camera, ticked from `Player::Update()` line 925 |
| `CinematicMgr.cpp` | 178 | `BeginCinematic`, `NextCinematicCamera`, `UpdateCinematicLocation`, idle-fail watchdog |
| `CollectionMgr.h` | 175 | `CollectionMgr` (toys, mounts, transmog appearances, heirlooms) — *Wrath does not have this UI*; included because Trinity's Wrath branch carries 10.x stubs |
| `CollectionMgr.cpp` | 939 | Collection load/save/grant; not relevant for 3.4.3 client but compiled in |
| `CUFProfile.h` | 116 | Compact unit-frame raid profile (UI client opt) — settings blob persisted in `character_cuf_profiles` |
| `EquipmentSet.h` | 71 | `EquipmentSet`/`TransmogOutfit` POD for "manage equipment" UI saved sets |
| `KillRewarder.h` | 59 | `KillRewarder` — non-persistent helper struct |
| `KillRewarder.cpp` | 304 | Distributes XP/honor/rep/loot to a group on a kill; called from `Unit::Kill` (Unit.cpp) |
| `PlayerTaxi.h` | 95 | `PlayerTaxi` — owned by `Player` (`Player::m_taxi`), per-zone bitmask (`taximask` BLOB column) + active path queue |
| `PlayerTaxi.cpp` | 229 | Mask init by race/class/level, save/load, Append/Pop nodes |
| `RestMgr.h` | 92 | `RestMgr` — owned by `Player` (`Player::_restMgr`), in-tavern/in-city flag tracking + rested XP accrual |
| `RestMgr.cpp` | 172 | `Update()` ticked from `Player::Update()` line 1006; awards 5%/8h regular, 25%/8h tavern multiplier |
| `SceneDefines.h` | 37 | `SceneFlag`, `SceneTriggerEvent` enums |
| `SceneMgr.h` | 87 | `SceneMgr` — scenario/scene-template runner per Player |
| `SceneMgr.cpp` | 247 | `PlayScene`/`CancelScene`; very thin in 3.4.3, used by some questline cinematics |
| `SocialMgr.h` | 163 | Singleton-ish `sSocialMgr` + per-player `PlayerSocial` (friends/ignore/RealID); friends list cap = 50 |
| `SocialMgr.cpp` | 313 | Friend status broadcast (online/offline/zone change), DB load/save of `character_social` |
| `TradeData.h` | 90 | `TradeData` — owned by `Player::m_trade` while a trade window is open |
| `TradeData.cpp` | 153 | Accepted/locked state, item slots, money escrow, accept-state machine |

**Total Player module: ~36130 lines.**

Wider Player surface (declared elsewhere but Player-owned):
- `src/server/game/Achievements/PlayerAchievementMgr.h` — `Player::m_achievementMgr` ticked from `Update` line 1001
- `src/server/game/Combat/CombatManager.h` — inherited from `Unit` but heavily Player-driven (PvP combat checks line 953)
- `src/server/game/Spells/SpellHistory.h` — also inherited; Player owns the only persistent (DB-saved) spell-history
- `src/server/game/Mails/Mail.h` — `Player::m_mail`, polled at `Update` line 915
- `src/server/game/Maps/InstanceLockMgr.h` — `Player::_instanceResetTimes`, swept at `Update` line 1116-1124

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Player` | class (final, inherits `Unit` → `WorldObject` → `Object`) | The player character entity |
| `CinematicMgr` | class (per-Player) | Cinematic camera tick |
| `CollectionMgr` | class (per-Player) | Toys/mounts/heirlooms (10.x carryover) |
| `CUFProfile` | struct | Raid frame profile blob |
| `EquipmentSet` | struct | Saved gear set (15 max) |
| `TransmogOutfit` | struct | Saved transmog (10.x) |
| `KillRewarder` | class (stack-only helper) | XP/honor/rep distribution on kill |
| `PlayerTaxi` | class (per-Player) | Flightmaster mask + active path |
| `RestMgr` | class (per-Player) | Rested-XP accrual + zone rest flags |
| `SceneMgr` | class (per-Player) | Cinematic scene player |
| `SocialMgr` (singleton) + `PlayerSocial` | class | Friends/ignore/mutes |
| `TradeData` | class (per-Player, transient) | Trade window state |
| `PlayerLoginQueryIndex` | enum | 47 prepared-statement slots for `LoadFromDB` parallel queries |
| `PlayerFlags` | enum (uint32) | `GROUP_LEADER`, `AFK`, `DND`, `GM`, `GHOST`, `RESTING`, `CONTESTED_PVP`, `IN_PVP`, `HIDE_CLOAK/HELM`, `PLAYED_LONG_TIME`, `MENTOR`, etc. (32 flags) |
| `PlayerFlagsEx` | enum (uint32) | `MERCENARY_MODE`, `ARTIFACT_FORGE_CHEAT`, `IN_PVP_COMBAT`, … |
| `PlayerCreateMode` | enum (int8) | `Normal`, `NPE` (new player experience) |
| `PlayerDelayedOperations` | enum | `DELAYED_SAVE_PLAYER`, `DELAYED_RESURRECT_PLAYER`, `DELAYED_BG_*`, `DELAYED_SPELL_CAST_DESERTER` |
| `PlayerCommandStates` | enum | `.cheat` GM toggles: GOD/CASTTIME/COOLDOWN/POWER/WATERWALK |
| `CharDeleteMethod` | enum | `REMOVE` vs `UNLINK` (soft-delete) |
| `PlayerSpellState` | enum | spell row state machine (UNCHANGED/CHANGED/NEW/REMOVED) for delta save |
| `MirrorTimerType` | enum | `FATIGUE`, `BREATH`, `FIRE` (env damage timers) |
| `EnchantDuration` | struct | Temporary enchant timer entry |
| `QuestStatusData` | struct (Player.h) | Per-quest progress + objective counters + timer |
| `BgBattlegroundQueueID_Rec` | struct | Per-queue-slot BG queue state |
| `RuneInfo` (DK) | struct | 6 runes × {type, cooldown} |
| `SpellModifier` | struct | Talent-driven cast modifier (additive/multiplicative) |
| `Runes` | struct | Container of `RuneInfo[6]` |
| `Player::PlayerSpell` | struct | One row of spellbook (`active`, `disabled`, `state`) |
| `Player::TalentEntry`/`TalentTab` | struct alias | Live talent tree state (per spec) |
| `ResurrectionData` | struct | Pending resurrect offer |

The 3.4.3 schema also uses fields/enums declared elsewhere but **only ever set from `Player`**: `PlayerLogXPReason`, `ReferAFriendError`, `DisplayToastType`/`Method`, `BGSpamProtection`. They live in `Player.h` for proximity.

---

## 4. Critical public methods

There are ~900 public methods on `Player`. Top selection ordered by hit-count from grep across the codebase:

| Symbol | Purpose | Calls into |
|---|---|---|
| `Player::Update(uint32 p_time)` (909) | Aggregates ~50 sub-systems per tick (see §11 below) | `Unit::Update`, `RestMgr::Update`, `CinematicMgr::Update`, `_achievementMgr->UpdateTimedCriteria`, `RegenerateAll`, `SaveToDB`, `RepopAtGraveyard`, `UpdateZone`, `UpdateItemDuration`, `UpdateEnchantTime`, `UpdateHomebindTime`, `UpdateAfkReport`, `UpdatePvPFlag`, `UpdateContestedPvP`, `UpdateDuelFlag`, `CheckDuelDistance`, `HandleDrowning`, `HandleSobering`, `RemovePet`, `EndCombatBeyondRange`, `TeleportTo` |
| `Player::LoadFromDB(ObjectGuid, CharacterDatabaseQueryHolder)` (17066) | ~1500 lines: `PlayerLoadData` decode, race/class init, then 46 follow-up loaders (`_LoadInventory`, `_LoadAuras`, `_LoadSpells`, `_LoadTalents`, `_LoadGlyphs`, `_LoadQuestStatus`, `_LoadDailyQuestStatus`, `_LoadReputation`, `_LoadActions`, `_LoadSkills`, `_LoadMail`, `_LoadHomeBind`, `_LoadInstanceLockTimes`, `_LoadEquipmentSets`, `_LoadDeclinedNames`, `_LoadAccountData`, `_LoadBGData`, `_LoadCurrency`, `_LoadVoidStorage`, `_LoadSocial`, `_LoadHonor`, `_LoadPet`, `_LoadCorpseLocation`) | Uses `PLAYER_LOGIN_QUERY_*` enum (47 slots), DBC stores, `ObjectMgr` |
| `Player::SaveToDB(bool create)` / `(LoginDbTx, CharDbTx, bool create)` (19318/19329) | ~1500 lines: writes `characters` row + 30+ child tables in one transaction (`_SaveInventory`, `_SaveAuras`, `_SaveSpells`, `_SaveTalents`, `_SaveQuestStatus`, `_SaveDailyQuestStatus` + weekly/monthly/seasonal, `_SaveReputation`, `_SaveActions`, `_SaveSkills`, `_SaveMail`, `_SaveCurrency`, `_SaveEquipmentSets`, `_SaveBGData`, `_SaveStats`, `_SaveCharacter`, `_SaveCustomizations`, `_SaveSeasonalQuestStatus`) | `CharacterDatabase.GetPreparedStatement(CHAR_INS_/UPD_/DEL_*)` |
| `Player::EquipItem(uint16 pos, Item* pItem, bool update)` (header 1368) | Move item into equipment slot, apply enchants/sets, fire `OnEquip` script, update visible inventory mask | `_StoreItem`, `ApplyEnchantment`, `UpdateItemSetAuras`, `ApplyEquipCooldown`, `UpdateExpertise`, scripts |
| `Player::StoreItem(ItemPosCountVec, Item*, bool update)` (header 1366) | Multi-slot store; resolves `CanStoreItem` results | `_StoreItem`, `AddItemDurations` |
| `Player::CanStoreItem(...)` (4 overloads, header 1350-1376) | Pre-flight inventory check returning `InventoryResult` | `CanStoreItem_InSpecificSlot`, `_InBag`, `_InInventorySlots` |
| `Player::CanEquipItem(uint8 slot, uint16& dest, Item*, bool swap, bool not_loading)` (1353) | Pre-flight equip check | proficiency/class/req-level checks |
| `Player::RemoveItem(uint8 bag, uint8 slot, bool update)` (1426) | Detach item from a slot; does not destroy | `RemoveItemDependentAurasAndCasts`, `ApplyEnchantment(false)` |
| `Player::MoveItemFromInventory(uint8 bag, uint8 slot, bool update)` (1427) | RemoveItem + clear DB row + `RemoveItemFromUpdateQueueOf` | `RemoveItem`, `Item::DeleteFromInventoryDB` |
| `Player::MoveItemToInventory(ItemPosCountVec, Item*, bool update, bool in_characterInventoryDB)` (1429) | Inverse of above; used by mail attachment claim, BG reward | `StoreItem`, `Item::SaveToDB` |
| `Player::DestroyItem`/`DestroyItemCount` | Permanently remove (bag deletes, vendor-sell, salvage) | `MoveItemFromInventory`, `Item::SetState(REMOVED)` |
| `Player::SwapItem(uint16 src, uint16 dst)` | Drag-and-drop on bags | `CanStoreItem`+`CanEquipItem` matrix |
| `Player::AddItem(uint32 entry, uint32 count)` | Create + store an item (mail, quest reward) | `Item::CreateItem`, `StoreItem` |
| `Player::AddSpell(spellId, active, learning, dependent, disabled, fromSkill, traitDef)` (line ~2700) | Insert into spellbook; resolve overrides; auto-learn dependent spells | `RemoveSpell` (chain), `LearnSpell` |
| `Player::LearnSpell(spellId, …)` (3192) | Public form, routes to `AddSpell(active=true, learning=true)` | `AddSpell` |
| `Player::RemoveSpell(spellId, disabled, learn_low_rank, suppressMessaging)` (3236) | Spellbook delete; restore lower rank if requested | spellbook map |
| `Player::ResetTalents(no_cost)` / `ResetTalentSpecialization` | Wipe talents, refund cost, send `SMSG_TALENTS_INVOLUNTARILY_RESET`/respec packets | `RemoveTalent`, `_SaveTalents` |
| `Player::AddTalent(TalentEntry, spec, learning)` / `RemoveTalent(TalentEntry)` (2694) | Per-tree learn/unlearn | spellbook |
| `Player::ActivateTalentGroup(ChrSpecialization)` | Dual-spec switch | `_LoadActions(spec)`, swap glyph set |
| `Player::AddQuest(Quest*, Object* questGiver)` (14404) | Take a quest: write objectives, start timer, fire `OnAccept` | `m_QuestStatus[id]`, scripts |
| `Player::CompleteQuest(uint32)` (14502) | Mark complete (still unrewarded) | `m_QuestStatusSave[id] = QUEST_DEFAULT_SAVE_TYPE` |
| `Player::RewardQuest(Quest*, LootItemType, uint32 reward, Object* questGiver, bool announce)` | Hand in: items, XP, money, rep, currency | `GiveXP`, `ModifyMoney`, `RewardReputation`, `AddItem` |
| `Player::FailQuest(uint32)` | Timer expired or death-failable | criteria reset |
| `Player::SatisfyQuest{Level,Race,Class,Reputation,Skill,Status,Day,Week,…}` | Eligibility predicates (~12 of them) | quest template flags |
| `Player::UpdateQuestObjectiveProgress(QuestObjectiveType, objectId, count, victim)` (16187) | Hook called from kill/loot/explore/cast paths | `m_QuestStatus[].ObjectiveData[]` |
| `Player::SetSkill(id, step, newVal, maxVal)` (5635) | Add/update skill row | `mSkillStatus`, learn rank-up spells |
| `Player::UpdateSkill(uint32 skill_id, uint32 step)` | Random skill-up roll on craft/use | `SetSkill` |
| `Player::SetReputation(factionEntryId, value)` (27469) | Set absolute rep | `m_reputationMgr.SetReputation` |
| `Player::RewardReputation(Unit* victim, float rate)` / `(Quest*)` | Quest/kill rep grants | `ReputationMgr::ModifyReputation` |
| `Player::UpdateRating(CombatRating)` / `UpdateAllRatings` (5231/5357) | Stat recalc on gear/aura change | rating table |
| `Player::UpdateSkillsForLevel()` (5587) | Bump weapon-skill caps on level-up | `SetSkill` |
| `Player::GiveLevel(uint8 level)` | Level-up: stats, talent points, send `SMSG_LEVELUP_INFO` | `InitStatsForLevel`, `InitTalentForLevel` |
| `Player::GiveXP(uint32 xp, Unit* victim, float group_rate)` | XP grant w/ rest bonus + RAF + group split | `GiveLevel`, `RestMgr` |
| `Player::ApplyEnchantment(Item*, EnchantmentSlot, bool apply, bool apply_dur, bool ignore_cond)` (13070) | Apply/remove enchant stat bonuses + aura | `HandleStatModifier`, `CastSpell` (proc enchants) |
| `Player::AddEnchantmentDuration(Item*, EnchantmentSlot, uint32 ms)` | Track temp enchants for tick decrement | `m_enchantDuration` list |
| `Player::UpdateItemDuration(uint32 time, bool realtimeonly)` (12912) | Tick limited-life items (heroism flask etc.) | `Item::SetDuration` |
| `Player::UpdateEnchantTime(uint32 time)` (12930) | Tick temp enchants | `ApplyEnchantment(false)` on expire |
| `Player::UpdateZone(uint32 newZone, uint32 newArea)` (7362) | Big bag of side-effects: weather, channels, FFA flag, sanctuary, exploration XP | `UpdateLocalChannels`, `UpdateArea`, `UpdateHostileAreaState`, `RestMgr::SetRestFlag` |
| `Player::UpdateArea(uint32 newArea)` (7304) | Sub-zone update | overlay weather/aura |
| `Player::UpdateHostileAreaState(AreaTableEntry const*)` (7444) | FFA-PvP / sanctuary / contested toggles | `UpdatePvP` |
| `Player::TeleportTo(map, x,y,z,o, options)` / `TeleportToBGEntryPoint` | Same-map and far teleport | `WorldSession::SendPacket(SMSG_TRANSFER_PENDING)` |
| `Player::SendInitialPacketsBeforeAddToMap()` / `…AfterAddToMap()` | Login pipe send-out (~30 SMSG) | `SMSG_LOGIN_VERIFY_WORLD`, `SMSG_ACCOUNT_DATA_TIMES`, `SMSG_INITIAL_SPELLS`, `SMSG_INITIAL_FACTIONS`, `SMSG_ACTION_BUTTONS`, `SMSG_TIME_SYNC_REQUEST`, … |
| `Player::SendNewItem`/`SendEquipError`/`SendBuyError`/`SendSellError` | Inventory feedback packets | `SMSG_ITEM_PUSH_RESULT`, `SMSG_INVENTORY_CHANGE_FAILURE`, etc. |
| `Player::SetSelection(ObjectGuid)` / `SetTarget` | UI target sync | `SMSG_SET_SELECTION` |
| `Player::DuelComplete(DuelCompleteType)` / `RequestDuel(Player*)` | Dueling state machine | UpdateDuelFlag |
| `Player::HandleDrowning(uint32 p_time)` (called from `Update` 1069) | Mirror timers (breath/fatigue) | env damage |
| `Player::SaveToDB()` saved transactionally on logout, on `Update` `m_nextSave` countdown, on map change, on level-up | … | … |
| `Player::Create(ObjectGuid, WorldPackets::Character::CharacterCreateInfo const*)` | Character-create pipeline | DBC stat table, starting items, `SaveToDB(create=true)` |
| `Player::DeleteFromDB(ObjectGuid, uint32 accountId, bool updateRealmChars, bool deleteFinally)` (static) | 30+ child-table deletes in one transaction | `CHAR_DEL_CHARACTER`, `_INVENTORY`, `_QUESTSTATUS_DAILY/_WEEKLY/_MONTHLY/_SEASONAL`, `_AURA_STORED_LOCATIONS`, `_INSTANCE_LOCK_BY_GUID`, `_FAVORITE_AUCTIONS`, `_ARENA_STATS`, `_CUSTOMIZATIONS` |

---

## 5. Module dependencies

**Depends on (everything below L4):**
- `Unit` / `WorldObject` / `Object` — base class, virtual `Update`, aura/combat/threat
- `Map` / `MapManager` / `Grid` — for `IsInWorld()`, `GetMap()`, visibility, `Map::SummonCreature` (pet)
- `ObjectMgr` — quest templates, item templates, creature/gameobject templates, scripted overrides
- `DBCStores`/`DB2Stores` — `ChrClasses`, `ChrRaces`, `ChrSpecialization`, `Talent`, `SkillLine*`, `SpellInfo`, `MapEntry`, `AreaTableEntry`, `FactionEntry`, `ItemSparse`, `ItemBonus`, `RandomPropertiesPoints`, `CharStartOutfit`
- `WorldSession` — owning session, packet I/O, account flags
- `CharacterDatabase` (MariaDB) — load/save (47 prepared statements minimum)
- `LoginDatabase` — mute time, realm character count, ban check
- `AuctionMgr` — outbid mail, expired-item return
- `Mail` — `Player::m_mail` polling
- `Group` — group sync packets, loot distribution
- `Guild` — petition/charter, guild log, tabard
- `BattlegroundMgr` / `Battleground` — queue state, BG entry/exit
- `InstanceLockMgr` — instance binds, raid resets
- `ReputationMgr` — `m_reputationMgr` member
- `AchievementMgr` — `m_achievementMgr` ticked from Update
- `SpellHistory` — spell cooldowns persistence
- `LFGMgr` — dungeon finder queue
- `SocialMgr` — friends/ignore (`m_social`)
- `OutdoorPvPMgr` — Wintergrasp/zone PvP
- `BattlefieldMgr` — Tol Barad / Wintergrasp queue
- `ArenaTeamMgr` — arena team membership
- `WardenMgr` — anti-cheat hook
- Scripts (`SmartAI`, hooks `OnPlayerLogin`, `OnLevelChanged`, `OnEquip`, `OnLootItem`, `OnQuestComplete`, `OnDuelEnd`, `OnTalentsReset`)

**Depended on by (essentially all of L5+):**
- Combat (`SpellMgr`, `Spell.cpp`) — caster/target
- AI (`PlayerAI`, hostile-target selection)
- Loot (`LootMgr`, drop rate per-Player factor)
- Quests (handlers/AI hooks)
- Chat (sender source)
- Mail (recipient)
- BG/Arena (everything)
- Guild (member iter)
- Group (member iter)
- Trading (`TradeData`)
- Movement (`MovementHandler` reads/writes `MovementInfo`)
- LFG (queue identity)
- Anti-cheat (Warden, AntiHack)

---

## 6. SQL / DB queries

`Player` is the heaviest DB consumer in the server. Ordered by Player.h `PlayerLoginQueryIndex` (47 slots in this 10.x branch; 3.4.3 baseline subsets — `TRAIT_*`, `TRANSMOG_OUTFITS`, `VOID_STORAGE`, `CURRENCY`, `CUF_PROFILES`, `PET_SLOTS`, `BANNED`, `SEASONAL`/`MONTHLY`/`WEEKLY` are post-Wrath but appear in this tree).

### 6.1 Login (parallel queries via `CharacterDatabaseQueryHolder`)

| Statement | Purpose | DB |
|---|---|---|
| `PLAYER_LOGIN_QUERY_LOAD_FROM` | Master `characters` row + `character_fishingsteps` LEFT JOIN | character |
| `…_LOAD_CUSTOMIZATIONS` | `character_customizations` (skin/face/etc.) | character |
| `…_LOAD_GROUP` | `group_member` by char | character |
| `…_LOAD_AURAS` | `character_aura` | character |
| `…_LOAD_AURA_EFFECTS` | `character_aura_effect` | character |
| `…_LOAD_AURA_STORED_LOCATIONS` | `character_aura_stored_location` (translocate/portal auras) | character |
| `…_LOAD_SPELLS` | `character_spell` | character |
| `…_LOAD_SPELL_FAVORITES` | `character_spell_favorite` | character |
| `…_LOAD_QUEST_STATUS` | `character_queststatus` | character |
| `…_LOAD_QUEST_STATUS_OBJECTIVES` | `character_queststatus_objectives` | character |
| `…_LOAD_QUEST_STATUS_OBJECTIVES_CRITERIA` | `character_queststatus_objectives_criteria` | character |
| `…_LOAD_QUEST_STATUS_OBJECTIVES_CRITERIA_PROGRESS` | `…_progress` | character |
| `…_LOAD_DAILY_QUEST_STATUS` | `character_queststatus_daily` | character |
| `…_LOAD_WEEKLY_QUEST_STATUS` | `…_weekly` | character |
| `…_LOAD_MONTHLY_QUEST_STATUS` | `…_monthly` | character |
| `…_LOAD_SEASONAL_QUEST_STATUS` | `…_seasonal` | character |
| `…_LOAD_QUEST_STATUS_REW` | `character_queststatus_rewarded` | character |
| `…_LOAD_REPUTATION` | `character_reputation` | character |
| `…_LOAD_INVENTORY` | `character_inventory` JOIN `item_instance` | character |
| `…_LOAD_VOID_STORAGE` | `character_void_storage` (post-Wrath) | character |
| `…_LOAD_MAILS` | `mail` | character |
| `…_LOAD_MAIL_ITEMS` | `mail_items` JOIN `item_instance` | character |
| `…_LOAD_SOCIAL_LIST` | `character_social` | character |
| `…_LOAD_HOME_BIND` | `character_homebind` | character |
| `…_LOAD_SPELL_COOLDOWNS` | `character_spell_cooldown` | character |
| `…_LOAD_SPELL_CHARGES` | `character_spell_charges` | character |
| `…_LOAD_DECLINED_NAMES` | `character_declinedname` (Russian locales) | character |
| `…_LOAD_GUILD` | `guild_member` JOIN `guild` | character |
| `…_LOAD_ARENA_INFO` | `character_arena_stats` | character |
| `…_LOAD_ACHIEVEMENTS` | `character_achievement` | character |
| `…_LOAD_CRITERIA_PROGRESS` | `character_achievement_progress` | character |
| `…_LOAD_EQUIPMENT_SETS` | `character_equipmentsets` | character |
| `…_LOAD_TRANSMOG_OUTFITS` | `character_transmog_outfits` | character |
| `…_LOAD_BG_DATA` | `character_battleground_data` | character |
| `…_LOAD_GLYPHS` | `character_glyphs` | character |
| `…_LOAD_TALENTS` | `character_talent` | character |
| `…_LOAD_ACCOUNT_DATA` | `account_data` | character |
| `…_LOAD_SKILLS` | `character_skills` | character |
| `…_LOAD_RANDOM_BG` | `character_battleground_random` | character |
| `…_LOAD_BANNED` | `character_banned` (also login DB cross-check) | character |
| `…_LOAD_INSTANCE_LOCK_TIMES` | `character_instance` | character |
| `…_LOAD_CURRENCY` | `character_currency` | character |
| `…_LOAD_CUF_PROFILES` | `character_cuf_profiles` | character |
| `…_LOAD_CORPSE_LOCATION` | `corpse` | character |
| `…_LOAD_PET_SLOTS` | `character_pet_slots` | character |
| `…_LOAD_TRAIT_ENTRIES` | `character_trait_entry` (post-Wrath) | character |
| `…_LOAD_TRAIT_CONFIGS` | `character_trait_config` (post-Wrath) | character |

### 6.2 Save (transactional — `CharacterDatabaseTransaction`)

`CHAR_INS_CHARACTER` / `CHAR_UPD_CHARACTER` (master row), plus per-table delete-then-insert pattern for: `CHAR_*_CHARACTER_INVENTORY`, `_AURA`, `_AURA_EFFECT`, `_AURA_STORED_LOCATION`, `_SPELL`, `_SPELL_COOLDOWN`, `_SPELL_CHARGES`, `_QUESTSTATUS`, `_QUESTSTATUS_OBJECTIVES` (and crit/progress), `_QUESTSTATUS_DAILY`, `_QUESTSTATUS_WEEKLY`, `_QUESTSTATUS_MONTHLY`, `_QUESTSTATUS_SEASONAL`, `_QUESTSTATUS_REW`, `_REPUTATION`, `_ACTIONS`, `_SKILLS`, `_TALENT`, `_GLYPHS`, `_HOMEBIND`, `_BATTLEGROUND_DATA`, `_BATTLEGROUND_RANDOM`, `_INSTANCE`, `_EQUIPMENTSETS`, `_TRANSMOG_OUTFITS`, `_CURRENCY`, `_VOID_STORAGE`, `_CUF_PROFILES`, `_FISHINGSTEPS`, `_TRAIT_ENTRIES`, `_TRAIT_CONFIGS`, `_CUSTOMIZATIONS`, `_DECLINED_NAMES`. `CHAR_UPD_CHARACTER_POSITION` is also written on logout (line 20613).

### 6.3 Delete (`Player::DeleteFromDB`)

`CHAR_DEL_CHARACTER` cascades to all of the above tables plus `_FAVORITE_AUCTIONS_BY_CHAR`, `_ARENA_STATS`, `_INSTANCE_LOCK_BY_GUID`, `_AURA_STORED_LOCATIONS_BY_GUID`, `mail`+`mail_items`, `corpse`, `character_pet`, `character_pet_declinedname`, `character_gifts`. Soft-delete (`CHAR_DELETE_UNLINK`) merely renames and sets `account = 0` plus `deleteInfos_*` columns.

### 6.4 Login DB writes

`LOGIN_UPD_MUTE_TIME` (Update line 972), `LOGIN_UPD_REALM_CHARACTERS` after create/delete, `LOGIN_INS_LOG_CHARACTER_PURGE` audit row.

### 6.5 DBC/DB2 stores read

`ChrClasses`, `ChrRaces`, `ChrClassesXPowerTypes`, `CharStartOutfit`, `CharBaseInfo`, `Talent`, `TalentTab` (3.4.3), `ChrSpecialization` (10.x label), `SpellLearnSpell`, `SkillLine`, `SkillLineAbility`, `SkillRaceClassInfo`, `MapEntry`, `AreaTableEntry`, `WMOAreaTableEntry`, `AreaTriggerEntry`, `LiquidType`, `FactionEntry`, `FactionTemplateEntry`, `ItemSparse`, `ItemEffect`, `ItemBonus`, `ItemBonusListLevelDelta`, `RandomPropertiesPoints`, `CharTitlesEntry`, `GlyphProperties`, `GameObjectDisplayInfoEntry`, `BattlemasterListEntry`, `LfgDungeonsEntry`. Read indirectly via `ObjectMgr` and `sDBCStorageMgr`.

---

## 7. Wire-protocol packets (~150 opcodes touch Player)

Categorised; only headline opcodes listed.

### 7.1 Login pipeline (server-side ordered send)
| Opcode | Direction | Sent in |
|---|---|---|
| `CMSG_PLAYER_LOGIN` | C→S | `Player::LoadFromDB` triggered |
| `SMSG_LOGIN_VERIFY_WORLD` | S→C | `Player::SendInitialPacketsBeforeAddToMap` |
| `SMSG_ACCOUNT_DATA_TIMES` | S→C | `WorldSession::SendAccountDataTimes` |
| `SMSG_FEATURE_SYSTEM_STATUS` | S→C | login |
| `SMSG_MOTD` | S→C | login |
| `SMSG_INITIAL_SPELLS` | S→C | `_SendInitialSpells` |
| `SMSG_SEND_KNOWN_SPELLS` (post-3.4) | S→C | login |
| `SMSG_SEND_UNLEARN_SPELLS` | S→C | login |
| `SMSG_INITIAL_FACTIONS` | S→C | `ReputationMgr::SendInitialReputations` |
| `SMSG_ACTION_BUTTONS` | S→C | `_SendInitialActionButtons` |
| `SMSG_INSTANCE_INFO` | S→C | login |
| `SMSG_TIME_SYNC_REQUEST` | S→C | `Player::Update` (recurring) |
| `SMSG_LOGIN_SET_TIME_SPEED` | S→C | login |
| `SMSG_HOTFIX_NOTIFY_BLOB` | S→C | login |
| `SMSG_UPDATE_OBJECT` (CreateObject2 self) | S→C | `BuildCreateUpdateBlockForPlayer` |
| `SMSG_FRIEND_STATUS` | S→C | `SocialMgr::SendFriendStatus` (broadcast) |

### 7.2 Movement (consumed by `Player::SetPosition` → `MovementInfo`)
`MSG_MOVE_*` (HEARTBEAT, START_*, STOP_*, JUMP, FALL_LAND, SET_FACING, TELEPORT_ACK), `CMSG_FORCE_*_SPEED_CHANGE_ACK`, `SMSG_TRANSFER_PENDING`, `SMSG_NEW_WORLD`, `MSG_MOVE_TELEPORT_CHEAT`.

### 7.3 Inventory
`CMSG_AUTOEQUIP_ITEM`, `CMSG_AUTOSTORE_BAG_ITEM`, `CMSG_SWAP_ITEM`, `CMSG_SWAP_INV_ITEM`, `CMSG_SPLIT_ITEM`, `CMSG_DESTROY_ITEM`, `CMSG_USE_ITEM`, `CMSG_BUY_ITEM`, `CMSG_SELL_ITEM`, `CMSG_BUYBACK_ITEM`, `CMSG_REPAIR_ITEM`, `CMSG_SOCKET_GEMS`, `CMSG_WRAP_ITEM`, `SMSG_ITEM_PUSH_RESULT`, `SMSG_INVENTORY_CHANGE_FAILURE`, `SMSG_BUY_FAILED`, `SMSG_SELL_RESPONSE`.

### 7.4 Quests
`CMSG_QUEST_GIVER_ACCEPT_QUEST`, `_COMPLETE_QUEST`, `_REQUEST_REWARD`, `_CHOOSE_REWARD`, `CMSG_QUESTLOG_REMOVE_QUEST`, `CMSG_PUSHQUESTTOPARTY`, `SMSG_QUEST_GIVER_QUEST_DETAILS`, `SMSG_QUEST_FORCE_REMOVE`, `SMSG_QUEST_UPDATE_*`.

### 7.5 Talents/spec/glyph
`CMSG_LEARN_TALENT`, `CMSG_LEARN_TALENTS_MULTIPLE`, `CMSG_RESET_TALENTS` (or `_GROUP`), `CMSG_SET_ACTIVE_TALENT_GROUP_OBSOLETE`, `SMSG_TALENT_UPDATE`, `SMSG_RESPEC_WIPE_CONFIRM`, `SMSG_LEARN_TALENTS_FAILED`.

### 7.6 Spells/cooldowns
`CMSG_CAST_SPELL`, `CMSG_CANCEL_CAST`, `CMSG_CANCEL_AURA`, `CMSG_LEARN_SPELL`, `SMSG_SPELL_START`, `SMSG_SPELL_GO`, `SMSG_LEARNED_SPELL`, `SMSG_SUPERCEDED_SPELL`, `SMSG_SPELL_COOLDOWN`, `SMSG_CLEAR_COOLDOWN`, `SMSG_MODIFY_COOLDOWN`.

### 7.7 Combat & duel
`SMSG_DUEL_REQUESTED`, `SMSG_DUEL_COMPLETE`, `SMSG_PVP_CREDIT`, `SMSG_DURABILITY_DAMAGE_DEATH`, `SMSG_PLAY_SOUND_FILE` (death cry).

### 7.8 Reputation
`SMSG_SET_FACTION_VISIBLE`, `SMSG_SET_FACTION_STANDING`, `SMSG_SET_FACTION_AT_WAR`, `SMSG_SET_FORCED_REACTIONS`.

### 7.9 Chat/social
`CMSG_MESSAGECHAT_*`, `CMSG_WHO`, `SMSG_CHAT`, `SMSG_FRIEND_STATUS`, `SMSG_IGNORE_LIST`, `CMSG_AUTOSTORE_LOOT_ITEM`, `SMSG_LOOT_RESPONSE`.

### 7.10 Group/raid
`CMSG_GROUP_INVITE`, `CMSG_GROUP_ACCEPT`/`_DECLINE`, `SMSG_GROUP_LIST`, `SMSG_PARTY_MEMBER_STATS_FULL`/`_PARTIAL`, `SMSG_RAID_READY_CHECK`, `SMSG_REAL_GROUP_UPDATE`.

### 7.11 Logout
`CMSG_LOGOUT_REQUEST`, `CMSG_LOGOUT_CANCEL`, `SMSG_LOGOUT_RESPONSE`, `SMSG_LOGOUT_COMPLETE`, `SMSG_LOGOUT_CANCEL_ACK`.

### 7.12 BG/arena/LFG
`SMSG_BATTLEFIELD_STATUS_*` (4 variants), `CMSG_BATTLEMASTER_JOIN`, `SMSG_PVP_OPTIONS_ENABLED`, `CMSG_LFG_JOIN`, `SMSG_LFG_PROPOSAL_UPDATE`.

### 7.13 Anti-cheat / mirror timers
`SMSG_START_MIRROR_TIMER`, `SMSG_STOP_MIRROR_TIMER`, `SMSG_MOVE_KNOCK_BACK`, plus Warden module-specific opcodes.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-entities` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-loot` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/character.rs` | `file` | 1 | 4612 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/quest.rs` | `file` | 1 | 851 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/spell.rs` | `file` | 1 | 288 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/loot.rs` | `file` | 1 | 247 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/group.rs` | `file` | 1 | 467 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/social.rs` | `file` | 1 | 360 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/combat.rs` | `file` | 1 | 152 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/chat.rs` | `file` | 1 | 413 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/trainer.rs` | `file` | 1 | 432 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/inspect.rs` | `file` | 1 | 82 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**

- `crates/wow-world/src/session.rs` — 3138 lines — `WorldSession` struct holds the *flat* per-character state. Player-relevant fields live at lines 175-309: `total_played_time`, `level_played_time`, `player_gold`, `player_xp`, `player_next_level_xp`, `selection_guid`, `player_guid`, `inventory_items: HashMap<u8, InventoryItem>`, `current_map_id`, `player_race`, `player_class`, `player_level`, `player_gender`, `known_spells: Vec<i32>`, `player_position`, `player_name`, `combat_target`, `in_combat`, `visible_auras: HashMap<u8, AuraApplication>`, `active_spell_cast`, `last_spell_cast_time`, `last_spell_cast_time_per_spell`, `quest_store`, `quest_xp_store`, `player_xp_table`, `player_quests: HashMap<u32, PlayerQuestStatus>`, `rewarded_quests: HashSet<u32>`, `loot_table`, `visible_creatures`, `visible_gameobjects`, `last_visibility_pos`, `gossip_options`, `gossip_source_guid`, `active_area_trigger`, `pending_teleport`, `creature_query_cache`, `logout_time`, `login_time`. There is **no `Player` struct**.
- `crates/wow-world/src/handlers/character.rs` — 4611 lines — handles `PlayerLogin`, character create, char enum, equip, partial inventory ops; the largest single Rust file in the workspace and the closest equivalent to `Player.cpp`'s public API
- `crates/wow-world/src/handlers/quest.rs` — 851 lines — quest accept/complete/reward (subset of `Player::AddQuest`/`RewardQuest`)
- `crates/wow-world/src/handlers/spell.rs` — 288 lines — cast/cancel; no spellbook, no cooldown DB persistence
- `crates/wow-world/src/handlers/loot.rs` — 247 lines — loot window
- `crates/wow-world/src/handlers/group.rs` — 467 lines — invite/accept/decline; relies on shared `GroupRegistry`
- `crates/wow-world/src/handlers/social.rs` — 360 lines — friends/ignore (no `SocialMgr` singleton — runs per-session)
- `crates/wow-world/src/handlers/movement.rs` — 204 lines — `MovementInfo` decode + position update
- `crates/wow-world/src/handlers/combat.rs` — 152 lines — auto-attack toggle
- `crates/wow-world/src/handlers/chat.rs` — 413 lines — chat sender = `WorldSession`
- `crates/wow-world/src/handlers/trainer.rs` — 432 lines — trainer purchase wrapper
- `crates/wow-world/src/handlers/inspect.rs` — 82 lines — stub
- `crates/wow-world/src/handlers/misc.rs` — 661 lines — gossip, area triggers, query, etc.
- `crates/wow-database/src/statements/character.rs` — defines ~25 prepared statements against the `characters` DB; covers `CHAR_SEL_CHARACTER_LIST`, `_CHECK_NAME`, `_COUNT_BY_ACCOUNT`, `_INS_CHARACTER`, `_DEL_CHARACTER`, `_SEL_CHARACTER` (master row), `_SET_ONLINE`, `_SET_OFFLINE`, `_OWNED_BY_ACCOUNT`, `_MAX_GUID`, `_INVENTORY` (load, slot update, delete), `_SKILLS` (load only — no save), `_SPELL` (load only — no save), `_ACTIONS` (load+insert), `_PLAYED_TIME`, `_MONEY`, `_XP`, `_LEVEL_XP`, `_INS_INVENTORY`, `_INS_SPELL`. **No** statements yet for `character_aura`, `character_quest_status*`, `character_reputation`, `character_talent`, `character_glyphs`, `character_homebind`, `character_battleground_data`, `character_equipmentsets`, `character_currency`, `character_instance`, `character_achievement(_progress)`, `character_social`, `character_declinedname`, `character_customizations`, `character_spell_cooldown`, `character_spell_charges`, `character_aura_effect`, `character_aura_stored_location`, `character_void_storage`, `character_cuf_profiles`, `character_fishingsteps`, `character_battleground_random`, `character_pet*`, mail tables.

**What's implemented (% vs C++):**
- Player identity: account_id, guid, name, race, class, gender, level, position, map — ✅ ~95%
- Money + XP grant + level up table — ✅ ~70% (no rest bonus, no RAF, no group split)
- Spells: known_spells `Vec<i32>` + casting — ⚠️ ~25% (no `PlayerSpell` row state, no save delta, no auto-learn dependent spells)
- Auras visible_auras 0-254 — ⚠️ ~30% (in-memory only, no DB persistence)
- Quests: accept/complete/reward — ⚠️ ~30% (in-memory only, no objective progress hooks beyond kill, no daily/weekly/monthly/seasonal split)
- Inventory `HashMap<u8, InventoryItem>` — ⚠️ ~10% (no `Item` entity, no enchants, no durability tick, no equipment slots vs bag distinction enforced, no `CanStoreItem`/`CanEquipItem` matrix, no item-set bonuses)
- Loot windows — ⚠️ partial
- Group invite/accept — ⚠️ partial
- Movement — ⚠️ partial
- Combat target/in_combat flag — ⚠️ ~20% (no threat list, no pet-driven combat)
- Visibility — ⚠️ partial via `MapManager` 3×3 grid window
- Logout 20-s timer — ✅ basic countdown but no save side-effects
- Time sync — ✅
- Played time accumulation — ⚠️ accumulator exists, never persisted

**What's missing vs C++ (the long list):**
- `Player` as a struct/class entity. There is no aggregator type.
- `Player::Update()` aggregator. The session loop has separate ad-hoc tick fragments (`tick_creatures_sync`, time-sync timer, logout timer) but no equivalent of the ~50-call `Update` body.
- `LoadFromDB` parallel-query holder. Per-character load is ad-hoc inside `handle_player_login` in `handlers/character.rs:982`.
- `SaveToDB` transactional commit with 30+ child tables. Saves are scattered (money, XP, position) and incomplete.
- `DeleteFromDB` cascade.
- `Item` entity — no `Item.h` equivalent. `InventoryItem` is a struct of 4 fields.
- `CharmInfo`, pet, totem, vehicle integration.
- Talents, specs, glyphs (entire system).
- Skills (entire `character_skills` system; no `SetSkill`/`UpdateSkill`).
- Reputations (`ReputationMgr`).
- Achievement progress, criteria.
- Mail polling (`Update` 915).
- Rest manager (`Update` 1006).
- Cinematic manager (`Update` 925).
- Scene manager.
- Taxi.
- Trade.
- Duel state machine + distance check (`Update` 947-949).
- Anti-cheat/AFK reporting (`Update` 951).
- Honor/PvP flag tick (`Update` 943-947).
- Mirror timers (drowning/fatigue/fire) (`Update` 1069 → `HandleDrowning`).
- Drunk timer + sobering (`Update` 1080-1085).
- Pending bind (instance) (`Update` 1087-1098).
- Death/repop/graveyard automation (`Update` 1101-1111).
- Enchantment duration tick (`Update` 1113).
- Homebind tick (`Update` 1114).
- Instance reset times sweep (`Update` 1116-1124).
- Group out-of-range update timer (`Update` 1128-1133).
- Pet visibility/dismiss (`Update` 1135-1138).
- Hostile-reference cleanup (`Update` 1142-1149).
- Delayed teleport (`Update` 1154-1155).
- BG state, queue, score, ratings.
- Arena teams.
- Outdoor PvP / Battlefield / Wintergrasp.
- LFG queue identity.
- Currency.
- Equipment sets / "manage equipment" UI.
- Heirlooms / collections (10.x — explicitly out of scope for 3.4.3).
- Void storage (10.x — out of scope).
- Trait configs (10.x — out of scope).
- Random BG / random dungeon cooldowns.
- Account data storage (UI keybinds).
- Declined names (Russian locale).
- `KillRewarder` group XP/honor distribution.
- `PlayerCommandStates` GM cheats.
- Refer-a-Friend.
- Whisper-restriction / ChatFloodThrottle.

**Suspicious / likely divergent (hypothesis pre-audit):**
- Position is stored on `WorldSession.player_position: Option<Position>` — every login pipe currently runs without an `IsInWorld()` invariant comparable to C++. Removing the entity removes the invariant.
- `inventory_items: HashMap<u8, InventoryItem>` keys by a single `u8` slot; the C++ system is `(bag, slot)` with `bag = INVENTORY_SLOT_BAG_0 = 255` for the main backpack and bag-1..4 inside their own `Bag` containers. The Rust schema cannot represent items inside a non-default bag.
- `known_spells: Vec<i32>` lacks the `(active, disabled, state)` triple that drives delta save in C++ (`PlayerSpellState`). On migration this means *every* save is a full rewrite of `character_spell` — currently neither save path exists.
- Quest progress (`PlayerQuestStatus`) is in-memory only; on disconnect everything except `rewarded_quests` is lost.
- Auras have no persistence — relog wipes buff state. C++ persists via `_LoadAuras`/`_SaveAuras` (`character_aura` + `character_aura_effect`).
- Money is stored as `u64`. C++ caps at `MAX_MONEY_AMOUNT = 2^31 - 1`. RustyCore should clamp on every `ModifyMoney` to match the wire-protocol limit; current `player_gold: u64` does not.
- `player_level: u8` matches C++ `uint8` but does not enforce `max_player_level` from world config.

**Tests existing:**
- `cargo test -p wow-world` reports 395 workspace passes (AGENTS.md). None are end-to-end "load player → tick `Update` → save player" — that pipeline has no aggregator to test.
- `crates/wow-world/src/map_manager.rs` carries 12 tests for the new entity-shared map but these test creatures, not Player.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_PLAYER.WBS.001** Cerrar la migracion auditada de `game/Entities/Player/CUFProfile.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CUFProfile.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.002** Cerrar la migracion auditada de `game/Entities/Player/CinematicMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CinematicMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.003** Cerrar la migracion auditada de `game/Entities/Player/CinematicMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CinematicMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.004** Partir y cerrar la migracion auditada de `game/Entities/Player/CollectionMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 939 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.005** Cerrar la migracion auditada de `game/Entities/Player/CollectionMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.006** Cerrar la migracion auditada de `game/Entities/Player/EquipmentSet.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/EquipmentSet.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.007** Cerrar la migracion auditada de `game/Entities/Player/KillRewarder.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/KillRewarder.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.008** Cerrar la migracion auditada de `game/Entities/Player/KillRewarder.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/KillRewarder.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.009** Partir y cerrar la migracion auditada de `game/Entities/Player/Player.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 29358 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.010** Partir y cerrar la migracion auditada de `game/Entities/Player/Player.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3189 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.011** Cerrar la migracion auditada de `game/Entities/Player/PlayerTaxi.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/PlayerTaxi.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.012** Cerrar la migracion auditada de `game/Entities/Player/PlayerTaxi.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/PlayerTaxi.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.013** Cerrar la migracion auditada de `game/Entities/Player/RestMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/RestMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.014** Cerrar la migracion auditada de `game/Entities/Player/RestMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/RestMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.015** Cerrar la migracion auditada de `game/Entities/Player/SceneDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/SceneDefines.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.016** Cerrar la migracion auditada de `game/Entities/Player/SceneMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/SceneMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.017** Cerrar la migracion auditada de `game/Entities/Player/SceneMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/SceneMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.018** Cerrar la migracion auditada de `game/Entities/Player/SocialMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/SocialMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.019** Cerrar la migracion auditada de `game/Entities/Player/SocialMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/SocialMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.020** Cerrar la migracion auditada de `game/Entities/Player/TradeData.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/TradeData.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.021** Cerrar la migracion auditada de `game/Entities/Player/TradeData.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/TradeData.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.022** Cerrar la migracion auditada de `game/Entities/Taxi/TaxiPathGraph.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Taxi/TaxiPathGraph.cpp`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_PLAYER.WBS.023** Cerrar la migracion auditada de `game/Entities/Taxi/TaxiPathGraph.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Taxi/TaxiPathGraph.h`
  Rust target: `crates/wow-world`, `crates/wow-entities`, `crates/wow-database`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-loot`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

### 9.A Foundation — extract a `Player` aggregate

- [ ] **#PLAYER.1** Create `crates/wow-entities/` (proposed in `entities.md`) and define `pub struct Player` with the *minimum-viable* identity slice (guid, account_id, race, class, gender, level, name, position, map) + `Arc<RwLock<…>>` accessor (XL — pulls in map_manager refactor)
- [ ] **#PLAYER.2** Move from `WorldSession`: `player_guid`, `player_race`, `player_class`, `player_level`, `player_gender`, `player_name`, `player_position`, `current_map_id` into `Player` (M)
- [ ] **#PLAYER.3** Move money/XP/played-time block (`player_gold`, `player_xp`, `player_next_level_xp`, `total_played_time`, `level_played_time`) and add `MAX_MONEY_AMOUNT` clamp (M)
- [ ] **#PLAYER.4** Replace `inventory_items: HashMap<u8, InventoryItem>` with a `(bag: u8, slot: u8)`-keyed map; introduce `Item` entity (cross-ref `inventory.md`) and `Bag` container (XL)
- [ ] **#PLAYER.5** Move spell/quest/aura/loot fields to `Player`; deprecate the `WorldSession` copies (H)

### 9.B `Player::Update()` aggregator

- [ ] **#PLAYER.6** Skeleton `fn update(&mut self, p_time_ms: u32)` matching the 50-call C++ body, all sub-calls stubbed `unimplemented!` (M)
- [ ] **#PLAYER.7** Wire mail polling against `mail` table (currently no `Mail` module) — depends on `mails.md` (H)
- [ ] **#PLAYER.8** Cinematic timer (no DB) (L)
- [ ] **#PLAYER.9** PvP flag tick + duel flag tick + duel distance check (M)
- [ ] **#PLAYER.10** AFK report tick + chat-flood throttle (M)
- [ ] **#PLAYER.11** Item duration + soulbound trade-window expiry tick (depends on `Item` entity) (M)
- [ ] **#PLAYER.12** `RestMgr` port — ZONE_UPDATE_INTERVAL gate, in-tavern flag, rested XP accrual (H)
- [ ] **#PLAYER.13** Mirror timers (`HandleDrowning`) — fatigue, breath, environmental fire (H)
- [ ] **#PLAYER.14** Drunk timer + sobering (L)
- [ ] **#PLAYER.15** Death/JustDied → KillPlayer + corpse repop graveyard sweep (M)
- [ ] **#PLAYER.16** Pending instance bind countdown (L)
- [ ] **#PLAYER.17** Enchantment duration tick (depends on `Item` enchants) (M)
- [ ] **#PLAYER.18** Homebind tick (L)
- [ ] **#PLAYER.19** Instance reset times sweep (M)
- [ ] **#PLAYER.20** Out-of-range group sync timer (M)
- [ ] **#PLAYER.21** Pet leash dismissal (depends on `Pet` entity) (M)
- [ ] **#PLAYER.22** Hostile-reference cleanup beyond visibility range (depends on `CombatManager`) (M)
- [ ] **#PLAYER.23** `m_nextSave` countdown + recurring `SaveToDB` (M)
- [ ] **#PLAYER.24** Delayed far-teleport application (M)

### 9.C `LoadFromDB` / `SaveToDB`

- [ ] **#PLAYER.25** Define `CharacterLoginQueryHolder` with the 47 prepared-statement parallel queries (H — adds ~25 statements to `wow-database`)
- [ ] **#PLAYER.26** Implement `Player::load_from_db` master row decode (mirror `PlayerLoadData`) including powers, exploredZones BLOB, knownTitles BLOB, taximask BLOB (H)
- [ ] **#PLAYER.27** `_load_inventory` JOIN `item_instance` (depends on `Item`) (XL)
- [ ] **#PLAYER.28** `_load_auras` + `_load_aura_effects` (M)
- [ ] **#PLAYER.29** `_load_spells` (L)
- [ ] **#PLAYER.30** `_load_quest_status` + `_objectives` (M)
- [ ] **#PLAYER.31** `_load_daily_quest_status` (3.4.3 only; weekly/monthly/seasonal optional) (L)
- [ ] **#PLAYER.32** `_load_reputation` (M, depends on `reputation.md`)
- [ ] **#PLAYER.33** `_load_skills` (M, depends on `skills.md`)
- [ ] **#PLAYER.34** `_load_actions` (action bars) (L)
- [ ] **#PLAYER.35** `_load_talents` + `_load_glyphs` (H)
- [ ] **#PLAYER.36** `_load_home_bind` + `_load_instance_lock_times` + `_load_corpse_location` (M)
- [ ] **#PLAYER.37** `_load_mail` + `_load_mail_items` (M)
- [ ] **#PLAYER.38** `_load_social` + `_load_declined_names` (M)
- [ ] **#PLAYER.39** `_load_bg_data` + `_load_random_bg` (3.4.3) (M)
- [ ] **#PLAYER.40** `_load_achievements` + `_load_criteria_progress` (M)
- [ ] **#PLAYER.41** `_load_equipment_sets` (L)
- [ ] **#PLAYER.42** `_load_account_data` (UI client opts) (L)
- [ ] **#PLAYER.43** `_load_spell_cooldowns` + `_load_spell_charges` (M)
- [ ] **#PLAYER.44** `_load_arena_info` (3.4.3) (M)
- [ ] **#PLAYER.45** `_load_banned` cross-check against `auth` DB (L)
- [ ] **#PLAYER.46** `Player::save_to_db` transactional master-row + child-table delete-then-insert pattern; mirror `_SaveCharacter`, `_SaveInventory`, `_SaveAuras`, `_SaveSpells`, `_SaveQuestStatus`, `_SaveReputation`, `_SaveActions`, `_SaveSkills`, `_SaveTalents`, `_SaveGlyphs`, `_SaveStats`, `_SaveBGData`, `_SaveCustomizations`, `_SaveEquipmentSets`, `_SaveCurrency`, `_SaveDailyQuestStatus`, `_SaveSpellCooldowns` (XL)
- [ ] **#PLAYER.47** `Player::delete_from_db(method)` cascade across 30+ tables (H)

### 9.D Inventory pipeline (cross-ref `inventory.md`)

- [ ] **#PLAYER.48** `CanStoreItem` matrix (4 overloads: `_InSpecificSlot`, `_InBag`, `_InInventorySlots`) (XL)
- [ ] **#PLAYER.49** `CanEquipItem` (proficiency, class req, level req, faction req, unique-equipped, equip-cooldown) (H)
- [ ] **#PLAYER.50** `EquipItem` / `StoreItem` / `RemoveItem` / `MoveItemFromInventory` / `MoveItemToInventory` / `SwapItem` / `DestroyItem` family (XL)
- [ ] **#PLAYER.51** `ApplyEnchantment` + temp-enchant tick (H)
- [ ] **#PLAYER.52** Item-set bonus apply/remove + form-change reapply (M)
- [ ] **#PLAYER.53** Gem socket / repair / refund pipelines (M)
- [ ] **#PLAYER.54** `BuyBackSlot` ring buffer (12 slots) (L)

### 9.E Talents/specs/glyphs

- [ ] **#PLAYER.55** `TalentSpec` per active group (3.4.3 = 2 specs); `AddTalent`/`RemoveTalent` (H)
- [ ] **#PLAYER.56** `ResetTalents` with refund cost ladder (L)
- [ ] **#PLAYER.57** `ActivateTalentGroup` dual-spec switch — swap action bars + glyphs (H)
- [ ] **#PLAYER.58** `Glyph` slots (6 in 3.4.3: 3 major + 3 minor); `ApplyGlyph`/`RemoveGlyph` (M)

### 9.F Quests (cross-ref `quests.md`)

- [ ] **#PLAYER.59** Migrate `PlayerQuestStatus` from `WorldSession` to `Player`; add `m_QuestStatus` + `m_QuestStatusSave` delta tracking (M)
- [ ] **#PLAYER.60** `m_timedquests: HashSet<u32>` + per-tick fail-on-timeout (L)
- [ ] **#PLAYER.61** `UpdateQuestObjectiveProgress` hooks from kill/loot/explore/cast/talk paths (H)
- [ ] **#PLAYER.62** Daily/weekly/monthly/seasonal quest status (3.4.3 = daily only) (M)
- [ ] **#PLAYER.63** `SatisfyQuest*` predicates (12 of them) (M)

### 9.G Reputation / Skills / Spells (cross-ref domain docs)

- [ ] **#PLAYER.64** `ReputationMgr` member + `SetReputation` / `RewardReputation` / `SendInitialReputations` (cross-ref `reputation.md`) (H)
- [ ] **#PLAYER.65** Skill table + `SetSkill`/`UpdateSkill` random skill-up + `UpdateSkillsForLevel` (cross-ref `skills.md`) (H)
- [ ] **#PLAYER.66** Spellbook with `PlayerSpell { active, disabled, state }` triple + `LearnSpell`/`RemoveSpell`/`AddSpell` chain + auto-learn dependent (cross-ref `spells.md`) (H)
- [ ] **#PLAYER.67** `SpellHistory` persistence: `character_spell_cooldown` + `character_spell_charges` (cross-ref `spells.md`) (M)

### 9.H Movement / login pipeline (cross-ref `movement.md`)

- [ ] **#PLAYER.68** `Player::SendInitialPacketsBeforeAddToMap` ordered emit of ~30 SMSG (H)
- [ ] **#PLAYER.69** `Player::SendInitialPacketsAfterAddToMap` (M)
- [ ] **#PLAYER.70** `TeleportTo` + `TeleportToBGEntryPoint` + delayed-teleport queue (H)
- [ ] **#PLAYER.71** `UpdateZone` / `UpdateArea` / `UpdateHostileAreaState` (M)

### 9.I Misc tickables

- [ ] **#PLAYER.72** `KillRewarder` group XP/honor/rep/loot distribution (H)
- [ ] **#PLAYER.73** `PlayerTaxi` mask + active path (M)
- [ ] **#PLAYER.74** `TradeData` window state machine (M)
- [ ] **#PLAYER.75** `CinematicMgr` (L)
- [ ] **#PLAYER.76** `RestMgr` already in #PLAYER.12 — keep as cross-ref
- [ ] **#PLAYER.77** `SocialMgr` singleton + `PlayerSocial` (cross-ref `social.md`) (M)
- [ ] **#PLAYER.78** `SceneMgr` (L)
- [ ] **#PLAYER.79** `EquipmentSet` / "manage equipment" UI (M)
- [ ] **#PLAYER.80** `PlayerCommandStates` `.cheat` GM toggles (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_PLAYER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 23 files / 36422 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-loot`, `crates/wow-spell`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-loot` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_PLAYER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 23 files / 36422 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-loot`, `crates/wow-spell`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_PLAYER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 23 files / 36422 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-loot`, `crates/wow-spell`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_PLAYER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 23 files / 36422 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-loot`, `crates/wow-spell`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] **Identity**: load a fixture character row → `Player.guid/race/class/gender/level/name/position` match the row exactly
- [ ] **Money clamp**: `Player.modify_money(MAX_MONEY_AMOUNT * 2)` saturates at `MAX_MONEY_AMOUNT`
- [ ] **XP rollover**: `give_xp` past `next_level_xp` increments level once and credits the remainder
- [ ] **Played time**: 60s of `update(1000)` increments both `total_played_time` and `level_played_time` by 60
- [ ] **Logout countdown**: 20s timer fires `save_to_db` once and only once
- [ ] **Inventory roundtrip**: equip Hearthstone (entry 6948) into MAINHAND fails; into BAG_0 succeeds; load_from_db → save_to_db → load_from_db reproduces same `(bag, slot, item_guid)` triples
- [ ] **CanEquipItem**: cloth class on plate item returns `EQUIP_ERR_CANT_EQUIP_SKILL`; correct class + below req level returns `EQUIP_ERR_CANT_EQUIP_LEVEL_I`
- [ ] **Spell delta save**: learn spell A → save → relog → A is in `known_spells`; remove A → save → relog → A is gone
- [ ] **Aura persistence**: aura with `is_passive=false, duration_remaining=30s` survives logout/login with at most `duration_remaining` decremented by elapsed wall-clock
- [ ] **Quest objective**: kill creature contributing to a `KillCreature` objective increments the count; objective count survives logout/login
- [ ] **Daily quest cap**: after server-config daily reset hour, `m_DailyQuestChanged` clears
- [ ] **Reputation**: `set_reputation(faction, 21000)` sets standing to Exalted (2999/3000 in Revered-1, then 21000 from Exalted floor)
- [ ] **Skill cap on level-up**: weapon skill cap = `5 * level`; level 60 player has cap 300
- [ ] **Talent reset cost ladder**: 1g, 5g, 10g, 15g, 20g, 25g, 30g, 35g, 40g, 45g, 50g (3.4.3 schedule)
- [ ] **Dual spec switch**: `activate_talent_group(2)` swaps action bars + glyphs; original spec returns intact on switch back
- [ ] **Death pipeline**: `set_death_state(JustDied)` clears combo points, drunk value, dismisses pet, fails `CompletionNoDeath` quests, fires `DieOnMap` criteria
- [ ] **Mirror timer**: submerging starts `BREATH` timer; emerging stops it; expiry deals env damage
- [ ] **Duel distance**: dueling players >25 yd apart for >10 s ends duel with `DUEL_FLED`
- [ ] **AFK auto-flag**: 5 min no input → `PLAYER_FLAGS_AFK` set; >10 min in BG → kicked
- [ ] **Save transaction integrity**: kill power mid-save → on relog the partial state must be rolled back (no orphan `character_spell` rows pointing at non-existent character)
- [ ] **Delete cascade**: `delete_from_db(REMOVE)` removes rows from all 30+ child tables
- [ ] **Soft delete**: `delete_from_db(UNLINK)` keeps rows but renames char and zeroes `account` column

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 23 files / 36422 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp` | `crates/wow-world/` (currently flat fields on `WorldSession`), proposed `crates/wow-entities/` (does not exist), partials in `crates/wow-database/`, `crates/wow-data/`, `crates/wow-spell/`, `crates/wow-loot/` \| 🔧 broken (rewrite needed) — `Player` as a class **does not exist** in RustyCore. Per-character state is exploded into ~70+ flat fields on `WorldSession` (`crates/wow-world/src/session.rs:65-309`). There is no `Player::Update()` aggregation, no `LoadFromDB`/`SaveToDB`, no equip/store/inventory pipeline beyond a `HashMap<u8, InventoryItem>`, no talent/spec/glyph state, no skill state object, no rest manager, no taxi, no trade data, no scene manager, no cinematic manager, no kill rewarder, no equipment sets. The C# legacy port had `Player` split across many partial files; the Rust rewrite has not begun the entity model. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_PLAYER.DIV.001` | `crates/wow-entities` (`missing_declared_path`, 0 Rust lines) | 23 C++ files / 36422 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#ENTITIES_PLAYER.DIV.002` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 23 C++ files / 36422 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#ENTITIES_PLAYER.DIV.003` | `crates/wow-loot` (`exists_empty`, 0 Rust lines) | 23 C++ files / 36422 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/CollectionMgr.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

### `Player::Update()` is the spinal cord — preserve order

The C++ `Update()` body has subtle ordering invariants. Re-ordering will silently break:

1. Mail check **before** `Unit::Update` (line 915-922) so any mail-triggered aura applies on this tick
2. `SetCanDelayTeleport(true) → Unit::Update → SetCanDelayTeleport(false)` (lines 933-935) — guards against teleport during aura application; nested telepathic teleport will hang the world thread
3. `UpdateItemDuration(now - m_Last_tick)` is in **wall-clock seconds**, not `p_time` ms (line 962). It uses `time_t` second precision.
4. `m_Last_tick` is updated **after** played-time increment (line 1077), not before. Reordering double-counts the elapsed second.
5. `RegenerateAll` only fires while alive (line 1047)
6. `KillPlayer` runs **after** save countdown but **before** drowning (it must, or a dying-player save would write `JustDied` state)
7. `UpdateHomebindTime` runs every tick **but** only ticks the homebind warning timer if outside the homebind area
8. Pet leash check uses `Map::GetVisibilityRange()` not `GetVisibilityDistance()` — `Distance` is a deprecated alias that returns config visibility, while `Range` includes phasing
9. The Update body **must be re-entrant safe** for delayed teleport (line 1154) — `TeleportTo` may re-enter `Update` indirectly via map change

### `LoadFromDB` invariants

- The 47 parallel queries are dispatched as one async holder; if any of `LOAD_FROM`, `LOAD_INVENTORY`, `LOAD_AURAS`, `LOAD_SPELLS` fails, login aborts with `CHAR_LOGIN_NO_CHARACTER`. Other failures degrade silently.
- `inventorySlots` and `bankSlots` columns store the *capacity*, not contents; capacity defaults to 16/24 for new chars
- `online` column is stamped to 1 on login **before** the master row is read — if the same account logs in twice from two clients there's a race window. The C++ kicks the existing session in `WorldSession::HandlePlayerLogin` *before* dispatching the holder.
- `taximask` is a **comma-separated string** in 3.4.3 (gets parsed into `m_taxi.m_taximask: std::array<uint32, …>`), not a BLOB
- `exploredZones` is a 200-byte BLOB (1600 area bits)
- `knownTitles` is a 64-byte BLOB (512 title bits)
- `at_login` is a bitmask of pending one-shot login actions (rename, customize, faction-change, race-change, change-name, change-faction, force-realm-transfer-update); cleared field-by-field by `HandleCharFactionOrRaceChangeOpcode` / `HandleCharCustomize`
- A character with `online = 1` from a crashed previous session must still be loadable — check `is_logout_resting` to decide whether to re-apply the rested aura

### `SaveToDB` invariants

- The save uses `TRANSACTION` semantics: prepare transaction, execute all 30+ statements, commit. **A rollback must leave the character logically untouched** — DO NOT split into multiple smaller transactions.
- The transaction holds **two** databases: `LoginDatabaseTransaction` (for mute, realm characters) AND `CharacterDatabaseTransaction`. Atomic ordering is: login-tx commit → character-tx commit. Reverse will leave the realm character count out of sync on power loss.
- `save_to_db(create=true)` is called **once** during character creation, with `INS_CHARACTER`. Subsequent saves use `UPD_CHARACTER`. Forgetting `create=true` on first save corrupts AUTO_INCREMENT logic.
- `_SaveSpells` uses `PlayerSpellState::CHANGED/NEW/REMOVED` to emit only delta INS/UPD/DEL — full rewrite is wrong (~5000 spells per save = slow). The Rust port must replicate this, not delete-all + insert-all.
- Quest objectives table can grow to thousands of rows per active raider; `_SaveQuestStatus` similarly does delta with `m_QuestStatusSave: std::map<uint32, QuestSaveType>`.

### Inventory edge cases

- The `(bag, slot)` tuple uses `bag = INVENTORY_SLOT_BAG_0 = 255` for the main backpack and bags 19-22 (`INVENTORY_SLOT_BAG_START`) for player bag containers; bag 0..3 contents are slots 39..62 (`INVENTORY_SLOT_BAG_START+1` etc.). The Rust schema currently uses a single `u8` slot — this is structurally incompatible.
- An equipped two-handed weapon must clear OFFHAND atomically; `EquipItem` to MAINHAND with a 2H must fail if OFFHAND is non-empty (unless `Titan's Grip` talent is active for Fury Warrior).
- `RemoveItem` does **not** delete from DB — only `MoveItemFromInventory` does. Confusing the two leaks rows.
- `BuyBackSlot` (12 slots) is a separate ring buffer; sold items go there for vendor buyback, not destroyed immediately.

### 3.4.3 vs 10.x branch divergence

This Trinity branch is a 10.x backport built on top of 3.4.3 schema. The `PlayerLoginQueryIndex` enum lists 47 slots, but several only exist in 10.x:
- `LOAD_TRAIT_ENTRIES`, `LOAD_TRAIT_CONFIGS` (10.x only — talent trees v3)
- `LOAD_TRANSMOG_OUTFITS` (Cataclysm+)
- `LOAD_VOID_STORAGE` (MoP+)
- `LOAD_CURRENCY` (Cataclysm+ for general currencies; 3.4.3 has only honor/arena)
- `LOAD_CUF_PROFILES` (Cataclysm+)
- `LOAD_PET_SLOTS` (MoP+)
- `LOAD_AURA_STORED_LOCATIONS` (some specific portal auras backported)
- `LOAD_SPELL_FAVORITES` (post-Wrath UI)

For the 3.4.3 client target, ignore these. The Wrath canonical loaders are ~33 of the 47.

### Performance hotspots

- `Player::Update()` is called **every world tick** (`100 ms` target) for every online player. With 1000 players online, this is ~50000 sub-system invocations per second.
- `_SaveCharacter` write rate is governed by `PLAYER_SAVE_INTERVAL` (default 15 minutes); reducing this without batching *will* saturate the character DB
- Inventory iteration in `CanStoreItem_InBag` is naive O(bags * slots * items) — the C++ uses a small optimisation cache `m_canEquipCache` that the Rust port should not skip
- `UpdateVisibilityOf` is per-tick per-player but is gated by movement-delta; preserve the `last_visibility_pos` gate or visibility recalc dominates the profile

### Historical bugs to NOT reintroduce

- Trinity issue **#23344**: dual-spec swap during a `_SaveTalents` mid-flight could write the wrong spec's talents to DB. Fix in C++ takes a snapshot before swap; the Rust port must mirror.
- Trinity issue **#26155**: `CHAR_DEL_CHARACTER_AURA_STORED_LOCATIONS_BY_GUID` was missing from the cascade. Trinity 10.x added it; 3.4.3 schema may not need the table but if present, must cascade.
- Trinity issue **#21712**: `Player::Update` could re-enter via `KillPlayer → BuildPlayerRepop → SendUpdateToOutOfRangeGroupMembers → group.UpdateMember → Player::Update` if the group member is on the same tick. Avoid by checking `m_can_delay_teleport` style re-entry guards.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent (proposed) | Notes |
|---|---|---|
| `class Player : public Unit` | `pub struct Player` (in `crates/wow-entities/player.rs`) | No inheritance — embed/compose `Unit` state via a `Unit` field or shared-trait `EntityCommon` |
| `Player*` ownership | `Arc<RwLock<Player>>` | Multi-session access (group members read each other's stats) — `dashmap` keyed by `ObjectGuid` is the natural store, e.g. `MapManager.players: DashMap<ObjectGuid, Arc<RwLock<Player>>>` |
| `Player::m_session: WorldSession*` | `Arc<WorldSession>` back-pointer | Player must reach session for packet send; weak-arc cycle break |
| `void Player::Update(uint32 p_time)` | `fn update(&mut self, p_time_ms: u32, ctx: &mut TickContext)` | `TickContext` aggregates DB handle + map + registries — avoids 12-arg signatures |
| `Player::LoadFromDB(ObjectGuid, CharacterDatabaseQueryHolder const&)` | `async fn load_from_db(guid: ObjectGuid, db: &CharacterDatabase) -> Result<Player>` | Use `tokio::join!` to fan out the 33 (3.4.3-relevant) queries in parallel; collect into a `LoadHolder` struct |
| `Player::SaveToDB(LoginDatabaseTransaction, CharacterDatabaseTransaction, bool create)` | `async fn save_to_db(&self, login_tx: &mut LoginTx, char_tx: &mut CharTx, create: bool) -> Result<()>` | sqlx transaction handles |
| `Player::EquipItem(uint16 pos, Item* pItem, bool update)` | `fn equip_item(&mut self, pos: SlotPos, item: &mut Item, update: bool) -> Result<&Item, EquipError>` | `SlotPos = (bag: u8, slot: u8)`; `Item` is a separate entity |
| `Player::CanStoreItem(...)` | `fn can_store_item(&self, args: CanStoreArgs) -> Result<ItemPosCountVec, InventoryResult>` | Wrap the 4 overloads in one builder; keep `InventoryResult` as the rich error enum |
| `Player::AddSpell(...)` returning bool | `fn add_spell(&mut self, args: AddSpellArgs) -> Result<AddSpellOutcome, AddSpellError>` | Return the auto-learn cascade for caller to record-keep |
| `Player::m_QuestStatus: std::unordered_map<uint32, QuestStatusData>` | `pub quest_status: HashMap<u32, QuestStatusData>` | Already partial in `WorldSession.player_quests`; needs migrating |
| `Player::m_QuestStatusSave: std::map<uint32, QuestSaveType>` | `pub quest_status_save: HashMap<u32, QuestSaveType>` | Drives delta save |
| `Player::m_spells: PlayerSpellMap` | `pub spells: HashMap<u32, PlayerSpell>` | `PlayerSpell { active: bool, disabled: bool, state: SpellState }` |
| `Player::m_taxi: PlayerTaxi` | `pub taxi: PlayerTaxi` | New struct |
| `Player::_restMgr: std::unique_ptr<RestMgr>` | `pub rest: RestMgr` | Owned inline; small POD |
| `Player::_cinematicMgr: std::unique_ptr<CinematicMgr>` | `pub cinematic: Option<CinematicMgr>` | Allocated lazily on cinematic start |
| `Player::m_achievementMgr: std::unique_ptr<PlayerAchievementMgr>` | `pub achievements: AchievementMgr` | Owned inline; cross-ref `achievements.md` |
| `Player::m_reputationMgr: ReputationMgr` | `pub reputation: ReputationMgr` | Cross-ref `reputation.md` |
| `Player::m_social: PlayerSocial*` | `pub social: PlayerSocial` | Singleton manager elsewhere |
| `Player::m_trade: TradeData*` | `pub trade: Option<TradeData>` | None when no window open |
| `KillRewarder` | `fn kill_rewarder::reward_kill(killer: &Player, victim: &Unit, group: Option<&Group>)` | Free function; no state |
| `Player::m_mail: std::list<Mail*>` | `pub mail: Vec<Mail>` | In-memory cache; cross-ref `mails.md` |
| `Player::m_items[PLAYER_SLOTS_COUNT]` (raw `Item*` array) | `pub items: HashMap<SlotPos, Arc<RwLock<Item>>>` | Items are entities — shared (`Item` may move between `Player`s via mail/trade) |
| `MAX_MONEY_AMOUNT` | `pub const MAX_MONEY_AMOUNT: u64 = 2_147_483_647` | Saturating add on `modify_money` |
| `PlayerLoginQueryIndex` | `enum PlayerLoadQuery` | One variant per query; drives the parallel join |
| `PLAYER_SLOTS_COUNT = 142` | `pub const PLAYER_SLOTS_COUNT: usize = 142` | `INVENTORY_SLOT_BAG_0 = 255` |
| `PlayerFlags` (u32 bitflags) | `bitflags::bitflags! { pub struct PlayerFlags: u32 { … } }` | Use `bitflags` crate |
| `enum class PlayerCreateMode : int8` | `#[repr(i8)] enum PlayerCreateMode { Normal = 0, NPE = 1 }` | — |

---

## 13. Audit (vs C++)

**Conclusion**: 🔧 broken — `Player` as a class **does not exist** in RustyCore.

**Evidence cross-checked 2026-05-01:**

1. **No `Player` struct anywhere in the workspace.**
   - `grep -rn "struct Player\b" /home/server/rustycore/crates/` → 0 hits
   - `grep -rn "pub struct Player " /home/server/rustycore/crates/` → 0 hits
   - The string `Player` appears in field/handler names (`player_guid`, `player_level`, `handle_player_login`) but never as a type.

2. **State exploded onto `WorldSession`.** Inventory of Player-belonging fields on `WorldSession` (lines 175-309 in `crates/wow-world/src/session.rs`): 70+ flat fields across 13 groups (account, played-time, money/XP, selection, identity, inventory, map, race/class/level/gender, known_spells, position+name, creatures, combat, auras, casting, quests, loot, visibility, gossip, area-trigger, query-cache). C++ encapsulates these inside `Player`; Rust does not.

3. **No `Player::Update()` aggregator.** The session loop in `session.rs` (the `update()` method around line 1300+) processes packet I/O, time-sync timer, logout countdown, and `tick_creatures_sync`. It does NOT perform: mail polling, cinematic tick, PvP flag tick, contested-PvP tick, duel flag, duel distance check, AFK report, item duration, soulbound trade item expiry, mute expiry, timed quests, achievements timed criteria, melee swing readiness, weapon swap timer, zone update, regen, death state, save countdown, drowning, played-time accumulation, drunk timer, pending bind, death timer, enchant time, homebind time, instance reset times sweep, group out-of-range update, pet leash, hostile-reference cleanup, delayed teleport. **All 28 sub-systems of C++ `Update` are absent.**

4. **No `LoadFromDB`/`SaveToDB` aggregator.** Loading is partial inside `handle_player_login` (4611-line monolith `handlers/character.rs`); saving is scattered. There is no `CharacterDatabaseQueryHolder` parallel-query pattern. The 47-slot enum has no Rust counterpart.

5. **Database surface incomplete.** `crates/wow-database/src/statements/character.rs` covers only ~25 statements vs C++ ~200+. Missing: `character_aura(_effect)`, `character_quest_status(_objectives, _objectives_criteria, _objectives_criteria_progress)`, `character_queststatus_daily/weekly/monthly/seasonal`, `character_reputation`, `character_talent`, `character_glyphs`, `character_homebind`, `character_battleground_data/random`, `character_equipmentsets`, `character_currency`, `character_instance`, `character_achievement(_progress)`, `character_social`, `character_declinedname`, `character_customizations`, `character_spell_cooldown`, `character_spell_charges`, `character_aura_stored_location`, `character_arena_stats`, `character_pet*`, mail tables, save-side INS/UPD/DEL families for skills/spell/inventory.

6. **No `Item` entity.** `InventoryItem` (4 fields: guid, entry_id, db_guid, inventory_type) is a hash-map value, not an entity. C++ `Item` has ~150 methods (durability, enchants, soulbound timer, gem sockets, item-set membership, refundable timer, BoP/BoE state, etc.). 0% migrated.

7. **No talents / specs / glyphs.** 0%. The string `talent` appears in `session.rs:664` only as `num_new_talents: 0` (a hardcoded zero in a packet builder).

8. **No skills, no reputation manager.** 0%. Cross-references to `skills.md` and `reputation.md` are pending.

9. **No `RestMgr`, no `PlayerTaxi`, no `TradeData`, no `CinematicMgr`, no `SceneMgr`, no `KillRewarder`, no `EquipmentSet`, no `CollectionMgr`, no `CUFProfile`.** All 8 sibling managers absent.

10. **`PlayerSocial` partial.** `handlers/social.rs` (360 lines) handles invite/accept/ignore opcodes per-session but does not own a `SocialMgr` singleton with cross-session friend-status broadcast.

11. **Inventory addressing wrong shape.** `inventory_items: HashMap<u8, InventoryItem>` keys by single `u8`. C++ uses `(bag: u8, slot: u8)` with `bag = 255` for backpack. Cannot represent items inside player bags 1-4.

12. **Money cap not enforced.** `player_gold: u64` admits values >= `MAX_MONEY_AMOUNT = 2^31 - 1`, the wire-protocol cap. No clamp.

13. **Auras not persistent.** `visible_auras: HashMap<u8, AuraApplication>` is in-memory only. Logout wipes buffs. C++ persists via `_LoadAuras` + `_LoadAuraEffects` + `_SaveAuras`.

14. **Quests not persistent.** `player_quests: HashMap<u32, PlayerQuestStatus>` is in-memory; only `rewarded_quests` is incidentally persisted by virtue of the existing minimal schema. Objective progress is lost on disconnect.

15. **Spellbook not persistent.** `known_spells: Vec<i32>` is loaded once from DB but has no `(active, disabled, state)` triple, no learn/forget delta save, no auto-learn-dependent cascade.

16. **No `SaveToDB` transaction.** `_save_character`-style atomic commit is absent. Money / XP / position writes are direct un-batched updates in their respective handlers.

17. **No delete cascade.** Character delete (if implemented at all) cannot cascade to the 30+ child tables because most do not exist as prepared statements.

**Audit verdict**: Player is the largest gap in the RustyCore migration. It blocks ~all of L5+ (Combat, Spells, Quests, Loot, Social, BG, Guild, Mail, Trade, Group sync, Anti-cheat). The proposed roadmap (§9, 80 sub-tasks split A-I) is the minimum to reach feature parity with the 3.4.3 subset; the 10.x carryovers (TraitConfig, VoidStorage, Currency, CUF profiles, transmog outfits, collections) are explicitly out of scope for this client target.

**Recommended next action**: start with **#PLAYER.1** — `Player` aggregate struct in a new `crates/wow-entities/` crate — and migrate `WorldSession.player_*` fields one by one with `cargo check -p wow-world` green between each, mirroring the `_attic/` lessons learned (don't introduce parallel bridge files; don't pre-write 22 files of unimplemented stubs).

---

*Cross-references in this doc:*
- `entities-unit.md` — base Unit class (§3, §5, §12)
- `entities.md` — entity layer overview, proposed `wow-entities` crate
- `inventory.md` — Item entity + bag containers (§9.D)
- `quests.md` — quest template + objective progress (§9.F)
- `reputation.md` — `ReputationMgr` (§9.G)
- `skills.md` — skill table (§9.G)
- `spells.md` — spellbook + cooldowns (§9.G)
- `movement.md` — `MovementInfo`, teleport (§9.H)
- `mails.md` — mail polling on `Update` (§9.B)
- `social.md` — `SocialMgr` singleton (§9.I)
- `achievements.md` — `AchievementMgr` ticked from `Update` (§4)
- `groups.md` — group sync (§9.B)
- `battlegrounds.md` — BG state save (§9.C)
- `instances.md` — instance lock times sweep (§9.B)
