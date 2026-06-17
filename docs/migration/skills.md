# Migration: Skills

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Skills/` + `src/server/game/Handlers/SkillHandler.cpp`
> **Rust target crate(s):** `crates/wow-data/` (DB2 readers — partial), `crates/wow-world/` (skill mgr per-session, handlers), `crates/wow-database/` (character_skills + skill_discovery_template + skill_extra_item_template + skill_perfect_item_template)
> **Layer:** L6
> **Status:** ⚠️ partial (DB2 readers only; no per-player skill state, no handlers, no skill-up logic)
> **Audited vs C++:** ✅ complete
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-player skill state: which crafting/combat/secondary skills the character knows, current rank vs max rank, profession assignment slot. Drives:
- **Auto-learn at level-up**: from `SkillRaceClassInfo.db2` + `SkillLineAbility.db2`, the player gets new spells (passive skill bonuses, recipe discovery, weapon skill).
- **Skill-up on use**: weapon skill on melee swing, profession skill on craft, secondary skill on gather. Rate × `RATE_SKILL_DISCOVERY` (and per-skill rates).
- **Recipe discovery** (`skill_discovery_template`): chance-driven random recipe unlock when an existing recipe is cast (used by alchemy/inscription mostly).
- **Extra & perfect items** (`skill_extra_item_template`, `skill_perfect_item_template`): proc bonus quantity / quality on craft.
- **Profession slot bookkeeping**: limit of 2 primary professions; `professionSlot` column = 1 or 2 in `character_skills`.

WoLK 3.4.3 has the classic skill system (skill ranks 0-450 typical, weapon skills max=5×level), pre-Legion "Crafting Quality Tiers" model.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Skills/SkillDiscovery.cpp` | 261 | `prefix` |
| `game/Skills/SkillDiscovery.h` | 31 | `prefix` |
| `game/Skills/SkillExtraItems.cpp` | 241 | `prefix` |
| `game/Skills/SkillExtraItems.h` | 35 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/Skills/SkillDiscovery.h` | 31 | `LoadSkillDiscoveryTable`, `GetSkillDiscoverySpell`, `HasDiscoveredAllSpells`, `HasDiscoveredAnySpell`, `GetExplicitDiscoverySpell` |
| `src/server/game/Skills/SkillDiscovery.cpp` | 261 | `SkillDiscoveryEntry { spellId, reqSkillValue, chance }`, in-memory `SkillDiscoveryStore` (map keyed by spell id or negative skill id), random-roll discovery with `RATE_SKILL_DISCOVERY` mult |
| `src/server/game/Skills/SkillExtraItems.h` | 35 | `CanCreatePerfectItem`, `LoadSkillPerfectItemTable`, `CanCreateExtraItems`, `LoadSkillExtraItemTable` |
| `src/server/game/Skills/SkillExtraItems.cpp` | 241 | In-memory stores keyed by spell id: `SkillExtraItemStore` (max-additional + chance), `SkillPerfectItemStore` (perfect-item entry + chance) |
| `src/server/game/Handlers/SkillHandler.cpp` | 117 | Talent + glyph + `HandleUnlearnSkillOpcode`, `HandleTradeSkillSetFavorite`, `HandleConfirmRespecWipeOpcode` (mixed file) |
| `src/server/game/Entities/Player/Player.cpp` (skill section) | massive | `LearnDefaultSkills`, `LearnSkillRewardedSpells`, `SetSkill`, `UpdateSkill`, `UpdateGatherSkill`, `UpdateCraftSkill`, `UpdateWeaponSkill`, `UpdateFishingSkill`, `GetSkillValue`, `GetMaxSkillValue`, `GetPureSkillValue`, `GetSkillTempBonusValue`, `GetSkillPermBonusValue`, `_LoadSkills`, `_SaveSkills` |
| `src/server/game/DataStores/DB2Stores.cpp` | — | `sSkillLineStore` (SkillLine.db2), `sSkillLineAbilityStore` + `GetSkillLineAbilityMapBounds` (SkillLineAbility.db2), `sSkillRaceClassInfoStore` (SkillRaceClassInfo.db2) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SkillStatus` | enum | `SKILL_UNCHANGED`, `SKILL_CHANGED`, `SKILL_NEW`, `SKILL_DELETED` (in-memory dirty tracking) |
| `SkillType` (`SharedDefines.h`) | enum | `SKILL_NONE=0`, `SKILL_FROST`, `SKILL_FIRE`, …, `SKILL_FISHING=356`, `SKILL_HERBALISM=182`, `SKILL_MINING=186`, `SKILL_ENCHANTING=333`, `SKILL_ALCHEMY=171`, `SKILL_BLACKSMITHING=164`, `SKILL_TAILORING=197`, `SKILL_LEATHERWORKING=165`, `SKILL_ENGINEERING=202`, `SKILL_INSCRIPTION=773`, `SKILL_JEWELCRAFTING=755`, `SKILL_COOKING=185`, `SKILL_FIRST_AID=129`, `SKILL_RIDING=762`, `SKILL_LOCKPICKING=633`, `SKILL_RUNEFORGING=776`, weapon skills (e.g. `SKILL_SWORDS=43`, `SKILL_BOWS=45`), defence (`SKILL_DEFENCE=95`), languages, … |
| `SkillCategory` | enum | `SKILL_CATEGORY_ATTRIBUTES`, `SKILL_CATEGORY_WEAPON`, `SKILL_CATEGORY_CLASS`, `SKILL_CATEGORY_ARMOR`, `SKILL_CATEGORY_SECONDARY` (cooking/fa/fish/archaeology), `SKILL_CATEGORY_LANGUAGES`, `SKILL_CATEGORY_PROFESSION`, `SKILL_CATEGORY_NOT_DISPLAYED` |
| `SkillLineEntry` (DB2) | struct | `ID`, `CategoryID`, `DisplayName`, `Description`, `SpellIconFileID`, `AlternateVerb`, `MinLevel`, `MaxLevel` (per-tier), `ParentSkillLineID`, `ParentTierIndex`, `Flags`, `SpellBookSpellID` |
| `SkillLineAbilityEntry` (DB2) | struct | `ID`, `RaceMask`, `SkillLine`, `Spell`, `MinSkillLineRank`, `ClassMask`, `SupercedesSpell`, `AcquireMethod` (0=None/1=OnSkillValue/2=OnSkillLearn), `TrivialSkillLineRankHigh/Low`, `Flags`, `NumSkillUps` |
| `SkillRaceClassInfoEntry` (DB2) | struct | `ID`, `RaceMask`, `SkillID`, `ClassMask`, `Flags` (`SKILL_FLAG_UNLEARNABLE`, `SKILL_FLAG_NOT_TRAINABLE`, `SKILL_FLAG_MAXIMIZED`, …), `Availability` (1=at-creation), `MinLevel`, `SkillTierID` |
| `SkillTiersEntry` (world SQL) | struct | 16 elements `Value[16]` per tier (e.g. apprentice=1-75, journeyman=75-150, expert=150-225, artisan=225-300, master=300-375, grand master=375-450 in WotLK) |
| `SkillDiscoveryEntry` (POD) | struct | `{ uint32 spellId; uint32 reqSkillValue; float chance }` |
| `SkillExtraItemEntry` (POD) | struct | `{ uint32 spellId; float additionalCreateChance; uint8 additionalMaxNum; uint32 requiredSpecialization }` |
| `SkillPerfectItemEntry` (POD) | struct | `{ uint32 spellId; uint32 requiredSpecialization; float perfectCreateChance; uint32 perfectItemType }` |
| `SkillLineAbilityFlag` | enum | `Flags` per ability (e.g. `SKILL_FLAG_NON_TRAINABLE`, `SKILL_FLAG_REWARDED_FROM_QUEST`) |

Player-side per-skill data (in `PlayerStorage.cpp` / `Player.h`):
- `SkillStatusData { uint8 pos; SkillStatus status; }` map keyed by `skillId`.
- `mSkillStatus: SkillStatusMap` (map<uint32, SkillStatusData>).
- 4 update fields per skill slot in player update fields (PlayerData::SkillInfo): `SkillID`, `Step`, `Rank`, `StartingRank`, `MaxRank`, `TempBonus`, `PermBonus`. Up to `PLAYER_MAX_SKILLS = 256` slots in WoLK update fields.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Player::LearnDefaultSkills()` | At character create / faction-change: iterate `sSkillRaceClassInfoStore` for race/class with `Availability=1`, call `LearnDefaultSkill` for each | DB2 lookup |
| `Player::LearnDefaultSkill(rcInfo, skillValue)` | Compute starting rank/maxrank per `SkillTiersEntry` (or weapon skill = level×5), call `SetSkill` then `LearnSkillRewardedSpells` | `ObjectMgr::_skillTiers`, `SetSkill`, `LearnSkillRewardedSpells` |
| `Player::LearnSkillRewardedSpells(skillId, skillValue, race)` | Iterate `SkillLineAbility` records for that skill, learn each spell whose `MinSkillLineRank <= skillValue` and race/class mask matches | `sSkillLineAbilityStore`, `LearnSpell` |
| `Player::SetSkill(skillId, step, currentValue, maxValue)` | Find or allocate skill slot (one of 256 update-field slots); update SkillInfo fields; mark `SKILL_NEW`/`SKILL_CHANGED`; setting value=0 marks `SKILL_DELETED` and queues `LearnDefaultSkill` removal of associated spells | update fields, `mSkillStatus` |
| `Player::UpdateSkill(skillId, chance)` | Generic "tick by 1 if roll succeeds" used for non-craft/non-gather skill increments | RNG, `SetSkill` |
| `Player::UpdateCraftSkill(spellId)` | On successful craft, computes chance via `red/yellow/green/grey` brackets vs current skill value relative to recipe trivial range | RNG |
| `Player::UpdateGatherSkill(skillId, currentValue, redLevel, multiplicator)` | Mining/herb skill-up roll based on node level vs skill | RNG |
| `Player::UpdateWeaponSkill(victim, attType)` | Roll weapon skill increment after melee swing; capped at `5 * level` | RNG |
| `Player::UpdateFishingSkill()` | Roll on successful fishing catch | RNG |
| `Player::GetSkillValue(skillId)` | Returns `Rank + TempBonus + PermBonus`, with rank cap at `MaxRank` | update fields |
| `Player::GetPureSkillValue(skillId)` | `Rank` only | — |
| `Player::GetMaxSkillValue(skillId)` | `MaxRank + PermBonus` | — |
| `Player::HasSkill(skillId)` | Slot present and rank > 0 | — |
| `Player::ModifySkillBonus(skillId, val, perm)` | Add temp/perm bonus (e.g. enchant +5, profession-specific gear) | update fields |
| `Player::_LoadSkills(result)` | Load `character_skills` rows into mSkillStatus + update fields | DB |
| `Player::_SaveSkills(trans)` | Persist NEW/CHANGED/DELETED slots via `INS_CHAR_SKILLS` / `UPD_CHAR_SKILLS` / `DEL_CHARACTER_SKILL` | DB |
| `LoadSkillDiscoveryTable()` | Load `skill_discovery_template`; key by `reqSpell` (positive) or `-skillId` (negative) | DB |
| `GetSkillDiscoverySpell(skillId, spellId, player)` | Iterate matching entries, roll each chance × `RATE_SKILL_DISCOVERY`, return first non-known spell, else 0 | `Player::HasSpell`, `World::getRate` |
| `GetExplicitDiscoverySpell(spellId, player)` | Used for "Discovery" recipe spells (e.g. flask discoveries) — full-pool weighted roll among unknown discoveries gated by `reqSkillValue` | RNG |
| `LoadSkillExtraItemTable()` | Load `skill_extra_item_template` | DB |
| `CanCreateExtraItems(player, spellId, addChance, addMax)` | Lookup, return whether proc applies and chance/max | — |
| `LoadSkillPerfectItemTable()` | Load `skill_perfect_item_template` | DB |
| `CanCreatePerfectItem(player, spellId, perfectChance, perfectItemType)` | Lookup, return whether perfect-quality variant rolls | — |
| `WorldSession::HandleUnlearnSkillOpcode(packet)` | Validate `SkillRaceClassInfo` allows unlearn (`SKILL_FLAG_UNLEARNABLE`); `Player::SetSkill(skillId, 0, 0, 0)` to drop | `sDB2Manager.GetSkillRaceClassInfo` |
| `WorldSession::HandleTradeSkillSetFavorite(packet)` | Mark a recipe as "favorite" in player spell preferences (Dragonflight feature; no-op visual on 3.4.3) | `Player::SetSpellFavorite` |
| `WorldSession::HandleShowTradeSkill(packet)` | Sends `SMSG_SHOW_TRADE_SKILL_RESPONSE` with another player's profession (inspect tradeskill) | — |

---

## 5. Module dependencies

**Depends on:**
- `Entities/Player` — owns the per-player skill map, update fields, spell book.
- `DataStores` / `ObjectMgr` — `sSkillLineStore`, `sSkillLineAbilityStore`, `sSkillRaceClassInfoStore`, `ObjectMgr::_skillTiers`, `sSpellMgr->GetSkillLineAbilityMapBounds(spellId)`, `sSpellNameStore`.
- `Spells/SpellMgr` — to look up which spells back a skill ability and which spells are "explicit discovery" / `MECHANIC_DISCOVERY`.
- `Database/CharacterDatabase` + `WorldDatabase` — `character_skills` (load/save), `skill_discovery_template`, `skill_extra_item_template`, `skill_perfect_item_template`.
- `World/World` — `RATE_SKILL_DISCOVERY` rate, profession config (`CONFIG_MAX_PRIMARY_TRADE_SKILL`, default 2).
- `Random` — `roll_chance_f`, `rand_chance` for proc calc.
- `Loot` — gather-skill nodes (Mining/Herb/Skinning) trigger `UpdateGatherSkill` from Loot completion.

**Depended on by:**
- `Spells/SpellEffects` — many effects gated by skill (lockpicking opens chests if `GetSkillValue(SKILL_LOCKPICKING) >= lockTier`; pet training; …).
- `Combat/Unit` — weapon skill roll affects melee miss/glance/crit chance.
- `Quests` — `Quest::RequiredSkillId/Value` to start; `RewardSkillId/Value` to grant.
- `Loot` — gather skill check on locked nodes (`Lock.db2` requires SKILL_HERBALISM/MINING/SKINNING ≥ X).
- `Trainer` (NPC) — sells recipes gated on skill value.
- `Item` — equipping items requires `EQUIP_ERR_CANT_EQUIP_SKILL` if `RequiredSkill > 0` and player skill lower.
- `Achievements` — exalted profession rank, "Master Chef" (cooking 350+), etc.

---

## 6. SQL / DB queries (if any)

DB: `character` and `world`.

| Statement / Source | Purpose | DB |
|---|---|---|
| `CHAR_SEL_CHARACTER_SKILLS` | `SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ?` | character |
| `CHAR_INS_CHAR_SKILLS` | `INSERT INTO character_skills (guid, skill, value, max, professionSlot) VALUES (?, ?, ?, ?, ?)` | character |
| `CHAR_UPD_CHAR_SKILLS` | `UPDATE character_skills SET value = ?, max = ?, professionSlot = ? WHERE guid = ? AND skill = ?` | character |
| `CHAR_DEL_CHAR_SKILL_BY_SKILL` / `DEL_CHARACTER_SKILL` | Remove a skill | character |
| `CHAR_DEL_CHAR_SKILLS` | Cascade on character delete | character |
| `CHAR_DEL_CHAR_SKILL_LANGUAGES` / `INS_CHAR_SKILL_LANGUAGE` | Faction-change service: drop old language skills, insert new at value=300/max=300 | character |
| (world) `skill_discovery_template` | Discovery procs (id columns `spellId, reqSpell, reqSkillValue, chance`) | world |
| (world) `skill_extra_item_template` | Extra-item procs (`spellId, requiredSpecialization, additionalCreateChance, additionalMaxNum`) | world |
| (world) `skill_perfect_item_template` | Perfect-item procs (`spellId, requiredSpecialization, perfectCreateChance, perfectItemType`) | world |
| (world) `skill_fishing_base_level` | Per-area minimum skill required for fishing | world |
| (world) `skill_tiers` | C++ `ObjectMgr::LoadSkillTiers`; canonical for this legacy tree | world |

DBC/DB2 stores read by skills:

| Store | What it loads | Read by |
|---|---|---|
| `SkillLine.db2` (`sSkillLineStore`) | All skill metadata | `Player`, GM commands |
| `SkillLineAbility.db2` (`sSkillLineAbilityStore` + `SpellMgr::GetSkillLineAbilityMapBounds`) | Maps spell → skill, with rank/race/class requirements | `LearnSkillRewardedSpells`, `GetSkillDiscoverySpell`, `HandleUnlearnSkill` |
| `SkillRaceClassInfo.db2` (`sSkillRaceClassInfoStore`) | Per-race/class skill availability, min level, tier | `LearnDefaultSkills`, `HandleUnlearnSkillOpcode` |
| `skill_tiers` (`ObjectMgr::_skillTiers`) | Value per skill tier (apprentice→grand master) | `LearnDefaultSkill` for max-rank computation |
| `SpellName.db2` | Used by discovery validator to enumerate explicit-discovery spells | `LoadSkillDiscoveryTable` |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_UNLEARN_SKILL` | client → server | `WorldSession::HandleUnlearnSkillOpcode` |
| `CMSG_TRADE_SKILL_SET_FAVORITE` | client → server | `WorldSession::HandleTradeSkillSetFavorite` |
| `CMSG_SHOW_TRADE_SKILL` | client → server | `WorldSession::HandleShowTradeSkill` |
| `SMSG_SHOW_TRADE_SKILL_RESPONSE` | server → client | response (post-WoLK) |
| `SMSG_TRADE_SKILL_DETAILS` | server → client | (Dragonflight, not 3.4.3) |
| `SMSG_UPDATE_SKILL` | server → client | (legacy; in WoLK 3.4.3 skill updates flow through `SMSG_UPDATE_OBJECT` on PlayerData::SkillInfo update field) |
| `SMSG_LEARN_TALENT_FAILED` | server → client | (talent module, included in same handler file) |

Note: in WoLK 3.4.3 most skill state propagation goes through **player update-fields**, not dedicated opcodes. Skill rank changes mark the corresponding update-field slot dirty and ride out on the next ObjectUpdate.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-data/src/skill.rs` | `file` | 1 | 608 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/skill.rs` — 608 lines — `SkillStore` reader for `SkillLineAbility.db2` + `SkillRaceClassInfo.db2`. Provides `default_starting_skills(race, class)` and per-skill ability iteration. Used by `LearnDefaultSkills` equivalent in character handler.
- (no per-player skill state)
- (no skill discovery / extra item / perfect item tables)
- (no handlers besides any auto-learn loop in character creation / login)

**What's implemented:**
- DB2 reading for `SkillLineAbility.db2` (auto-learn list) and `SkillRaceClassInfo.db2` (starting skill list).
- Used at character creation / login to compute the starting spell set ("LearnDefaultSkills" equivalent), but **without** persisting per-skill rank/max state.

**What's missing vs C++:**
- Per-player skill state (`mSkillStatus` map + update-field slot allocation).
- `character_skills` table prepared statements + load/save.
- `SetSkill`, `UpdateSkill`, `UpdateCraftSkill`, `UpdateGatherSkill`, `UpdateWeaponSkill`, `UpdateFishingSkill`.
- `GetSkillValue` / `GetMaxSkillValue` / `GetPureSkillValue` / `HasSkill` / `ModifySkillBonus`.
- `skill_tiers` world-table runtime consumption (needed for max-rank step calc).
- `skill_discovery_template` loader + `GetSkillDiscoverySpell` / `GetExplicitDiscoverySpell`.
- `skill_extra_item_template` loader + `CanCreateExtraItems`.
- `skill_perfect_item_template` loader + `CanCreatePerfectItem`.
- `skill_fishing_base_level` loader for per-area fishing.
- Profession slot bookkeeping (max 2 primary professions, primary vs secondary).
- `CMSG_UNLEARN_SKILL` handler.
- `CMSG_TRADE_SKILL_SET_FAVORITE` (no-op for 3.4.3 but should not log warning).
- `CMSG_SHOW_TRADE_SKILL` (inspect another player's tradeskill).
- Skill-locked equip rejection (`EQUIP_ERR_CANT_EQUIP_SKILL`).
- Skill-locked spell cast rejection (lockpicking, gathering nodes).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `crates/wow-data/src/skill.rs` reads `acquire_method` and `min_skill_line_rank` but no caller currently re-evaluates the auto-learn list when skill rank goes up — only the initial creation set is granted. Players who level a profession will not get the next-tier auto-learn spells.
- No update-field slot allocator — when skills are added later, the SkillInfo array would not get populated, so the client UI Skills pane would be empty.
- Faction-change service path completely absent (skills get re-mapped between Horde/Alliance language skills).

**Tests existing:**
- A handful in `crates/wow-data/src/skill.rs` for DB2 reading; none for skill-up logic, persistence, or handlers.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SKILLS.WBS.001** Cerrar la migracion auditada de `game/Skills/SkillDiscovery.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SKILLS.WBS.002** Cerrar la migracion auditada de `game/Skills/SkillDiscovery.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SKILLS.WBS.003** Cerrar la migracion auditada de `game/Skills/SkillExtraItems.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SKILLS.WBS.004** Cerrar la migracion auditada de `game/Skills/SkillExtraItems.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h`
  Rust target: `crates/wow-data`, `crates/wow-world`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.
Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [~] **#SKILLS.1** Add C++ `skill_tiers` world-table representation to `crates/wow-data/src/skill.rs` exposing `Value[16]` per tier id. `SkillTiersStoreLikeCpp` and SQL statement exist; remaining work is wiring max-rank consumers into runtime skill state. (M)
- [ ] **#SKILLS.2** Extend `SkillLine.db2` coverage beyond the represented reader now used by `GetProfessionSkillForExp` (#NEXT.R8.ENTITIES.325): wire all remaining skill metadata consumers (`CategoryID`, `ParentSkillLineID`, `ParentTierIndex`, flags) into canonical skill state/commands (M)
- [ ] **#SKILLS.3** Define `SkillStatus` enum, `SkillStatusData { pos: u8, status: SkillStatus }`, `PlayerSkillState` per-session struct in `crates/wow-world/src/skill/state.rs` (M)
- [ ] **#SKILLS.4** Implement update-field slot allocator: 256 slots in PlayerData::SkillInfo, find first free / reuse pos on `SetSkill` (M)
- [ ] **#SKILLS.5** Add prepared statements: `SEL_CHARACTER_SKILLS`, `INS_CHAR_SKILLS`, `UPD_CHAR_SKILLS`, `DEL_CHARACTER_SKILL`, `DEL_CHAR_SKILLS`, `DEL_CHAR_SKILL_LANGUAGES`, `INS_CHAR_SKILL_LANGUAGE` in `crates/wow-database/src/statements/character.rs` (L)
- [ ] **#SKILLS.6** Implement `Player::load_skills` (read rows → populate `mSkillStatus` + update-field slots) (M)
- [ ] **#SKILLS.7** Implement `Player::save_skills` (write NEW/CHANGED/DELETED rows via `_SaveSkills` semantics) (M)
- [ ] **#SKILLS.8** Implement `Player::set_skill(skillId, step, currentValue, maxValue)` with full TC behavior: value=0 → mark DELETED + drop linked spells via `SkillLineAbility` reverse lookup (H)
- [ ] **#SKILLS.9** Implement `Player::get_skill_value(skillId)` / `get_pure_skill_value` / `get_max_skill_value` / `has_skill` / `modify_skill_bonus` (M)
- [ ] **#SKILLS.10** Implement `Player::learn_default_skills` triggered at character creation AND after level-up tier transitions (call `LearnSkillRewardedSpells` for each gained skill) (M)
- [ ] **#SKILLS.11** Implement `Player::learn_skill_rewarded_spells(skillId, skillValue, race)` — iterate `SkillLineAbility` map for skill, grant any spell with rank ≤ value matching race/class (M)
- [ ] **#SKILLS.12** Implement `Player::update_craft_skill(spellId)` — red/yellow/green/grey thresholds vs `TrivialSkillLineRankHigh/Low` of the spell's ability record (M)
- [ ] **#SKILLS.13** Implement `Player::update_gather_skill(skillId, current, redLevel, multiplicator)` — used by Mining/Herb/Skinning loot completion (M)
- [ ] **#SKILLS.14** Implement `Player::update_weapon_skill(victim, attType)` — bumps SKILL_SWORDS etc. capped at 5*level (M)
- [ ] **#SKILLS.15** Implement `Player::update_fishing_skill` — roll on every successful catch (L)
- [ ] **#SKILLS.16** Implement `LoadSkillDiscoveryTable` — loader for `skill_discovery_template` worldserver table; key by `reqSpell` (positive) or `-skillId` (negative) (M)
- [ ] **#SKILLS.17** Implement `get_skill_discovery_spell(skillId, spellId, player)` and `get_explicit_discovery_spell(spellId, player)` matching C++ logic (chance × `RATE_SKILL_DISCOVERY`) (M)
- [ ] **#SKILLS.18** Implement `LoadSkillExtraItemTable` + `can_create_extra_items(player, spellId)` (M)
- [ ] **#SKILLS.19** Implement `LoadSkillPerfectItemTable` + `can_create_perfect_item(player, spellId)` (M)
- [ ] **#SKILLS.20** Implement `LoadSkillFishingBaseLevel` worldserver table + `can_fish_in_area(player, areaId)` (L)
- [ ] **#SKILLS.21** Implement `handle_unlearn_skill(packet)` — validate `SKILL_FLAG_UNLEARNABLE` on the SkillRaceClassInfo, then `set_skill(id, 0, 0, 0)` (L)
- [ ] **#SKILLS.22** Implement `handle_trade_skill_set_favorite(packet)` — store favorite in player's spell preferences (L; no-op visual on 3.4.3 but persist anyway)
- [ ] **#SKILLS.23** Implement `handle_show_trade_skill(packet)` — return another player's profession data via `SMSG_SHOW_TRADE_SKILL_RESPONSE` (M)
- [ ] **#SKILLS.24** Profession slot bookkeeping: enforce `CONFIG_MAX_PRIMARY_TRADE_SKILL` (default 2), populate `professionSlot` column on insert (1 or 2 for primary, NULL for secondary) (M)
- [ ] **#SKILLS.25** Equip-error `EQUIP_ERR_CANT_EQUIP_SKILL` integration: on `Player::can_use_item`, check `RequiredSkill > 0 && skillValue < RequiredSkillRank` (depends on Inventory/Item module) (L)
- [ ] **#SKILLS.26** Lockpicking integration: `Lock.db2` lookup + skill check in spell effect / object interaction (depends on Spells module) (M)
- [ ] **#SKILLS.27** Quest skill-reward integration: `Quest::RewardSkillId/Value` on quest completion (depends on Quest module) (L)
- [ ] **#SKILLS.28** Faction-change service: drop old language skills, set new language skill to (300, 300) via `DEL_CHAR_SKILL_LANGUAGES` + `INS_CHAR_SKILL_LANGUAGE` (M)
- [ ] **#SKILLS.29** Auto-relearn on tier-up: when `set_skill` raises rank past a `SkillTiers::Value[N]` threshold, re-run `learn_skill_rewarded_spells` to grant new abilities (e.g. expert → artisan first aid) (M)
- [ ] **#SKILLS.30** GM command parity: `.lookup skill`, `.skill set`, `.skill all`, `.unlearn skill` (defer until GM commands module) (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SKILLS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 568 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | `cargo test -p wow-data && cargo test -p wow-database && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SKILLS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 568 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SKILLS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 568 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SKILLS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 568 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h`. Rust target: `crates/wow-data`, `crates/wow-database`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: create new Human Warrior → has SKILL_SWORDS (43) at value=`level*5`, SKILL_DEFENCE (95) same, languages `SKILL_LANG_COMMON` (98) at 300/300.
- [ ] Test: `set_skill(SKILL_FIRST_AID, 1, 1, 75)` allocates new update-field slot, INSERT row, `mSkillStatus[129].status = SKILL_NEW`.
- [ ] Test: `set_skill(SKILL_FIRST_AID, 0, 0, 0)` marks DELETED, removes linked passive spells (spells whose `SkillLineAbility.SkillLine = 129`).
- [ ] Test: `set_skill` of an already-known skill marks `SKILL_CHANGED`, persists via UPDATE (not duplicate INSERT).
- [ ] Test: `learn_default_skills(human, warrior, 1)` grants exactly the set listed in `SkillRaceClassInfo` with `Availability=1` and matching race/class masks.
- [ ] Test: `update_craft_skill(spellId)` rolls success: above-trivial (grey) → no skill-up; at-trivial → rare; below-trivial → frequent. Match TC's chance brackets.
- [ ] Test: `update_weapon_skill` capped at `level * 5`.
- [ ] Test: `get_skill_discovery_spell(skillId=171 alchemy, spellId=11479, player)` with chance=100%, reqSkillValue=275, player at 280 → returns the discovery spell once; subsequent calls return 0 if HasSpell.
- [ ] Test: `get_explicit_discovery_spell(spellId, player)` weighted-roll picks among unknown discoveries gated by reqSkillValue.
- [ ] Test: `can_create_extra_items(player, spellId)` returns chance/max for spell with row in `skill_extra_item_template`, false otherwise.
- [ ] Test: `can_create_perfect_item(player, spellId)` returns true with chance and item type; specialization-gated entries skipped if `requiredSpecialization` not learned.
- [ ] Test: load/save round-trip — set 5 skills, save, drop, load → identical SkillInfo update fields and `mSkillStatus`.
- [ ] Test: `handle_unlearn_skill` rejected if skill flag lacks `SKILL_FLAG_UNLEARNABLE`.
- [ ] Test: profession slot enforcement — try to learn 3rd primary profession → rejected at trainer level (depends on trainer module wiring).
- [ ] Test: tier-up auto-learn — set Alchemy from 74 → 75 grants Journeyman Alchemy passive spell.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 568 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h` | `crates/wow-data/` (DB2 readers — partial), `crates/wow-world/` (skill mgr per-session, handlers), `crates/wow-database/` (character_skills + skill_discovery_template + skill_extra_item_template + skill_perfect_item_template) \| ⚠️ partial (DB2 readers only; no per-player skill state, no handlers, no skill-up logic) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SKILLS.DIV.001` | _none generated_ | 4 C++ files / 568 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillDiscovery.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Skills/SkillExtraItems.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **WoLK 3.4.3 specific**: max profession skill is 450 (Grand Master) for primary professions, 450 cooking, 450 first aid, 450 fishing. Riding skill cap is 300 (epic flying). Weapon skills cap at `5 * level` (so 400 at lvl 80).
- **Skill update-field slot allocator** is critical: WoLK 3.4.3 reserves up to 256 slots in `PlayerData::SkillInfo` (4 fields per slot: ID/Step/Rank/MaxRank or similar). When a skill is `SKILL_DELETED`, its slot must be zero-cleared but not freed for reuse until `_SaveSkills` runs (allocator must respect "deleted but pending save" entries).
- **`SkillTiersEntry` in this WoLK legacy** is loaded from world SQL `skill_tiers` and has 16 `Value` slots; only first 6 are normally used (apprentice/journeyman/expert/artisan/master/grand master). Do not port this as `SkillTiers.db2` unless the canonical C++ source changes.
- **`SkillRaceClassInfo.SkillTierID = 0`** means "weapon skill" (or other no-tier). Compute max as `level * 5` instead of looking up tier value.
- **`SkillLineAbility.AcquireMethod`**:
  - `0` = teach manually (trainer)
  - `1` = auto-learn when skill value reaches `MinSkillLineRank`
  - `2` = auto-learn when skill is first learned
- **`SkillLineAbility.NumSkillUps`** controls how many points the skill goes up per cast (default 1). E.g. fast-leveling profession recipes can have NumSkillUps=2.
- **Discovery: positive `reqSpell` vs negative `-skillId`**: the `skill_discovery_template.reqSpell` column is overloaded:
  - `> 0`: discovery is gated by knowing a specific recipe spell that has `MECHANIC_DISCOVERY` or is `IsExplicitDiscovery()`.
  - `0`: discovery is keyed off a SkillLineAbility — the discovered spell must appear in `SkillLineAbility.dbc` and the gate is the parent skill line.
  - `< 0`: ERROR (TC logs and skips).
  Migration must replicate exact two-mode lookup table (`SkillDiscoveryStore[reqSpell]` and `SkillDiscoveryStore[-skillId]`).
- **`RATE_SKILL_DISCOVERY`**: world config rate; NOT applied to `GetExplicitDiscoverySpell` — only to chance-based `GetSkillDiscoverySpell`.
- **`MECHANIC_DISCOVERY = 28`** is the spell mechanic flag for "this recipe procs discovery on use".
- **`IsExplicitDiscovery()`** is a SpellInfo helper detecting "100% chance random discovery" spells (Northrend Alchemy Research, Minor Inscription Research, etc.).
- **Cooking/Fishing/First Aid are SECONDARY**: they don't count toward primary profession slot limit.
- **Faction-change drops language skills**: `DEL_CHAR_SKILL_LANGUAGES` deletes skills 98, 113, 759, 111, 313, 109, 115, 315, 673, 137 (all the racial languages) before re-inserting the new faction's language at 300/300.
- **`Player::SetSkill` with value=0 is a "delete" intent**: it loops `SkillLineAbility` records for that skill and unlearns each linked spell that the player has, then marks `SKILL_DELETED`. Be careful not to double-unlearn auto-learn spells that originated from a different skill.
- **Step vs Rank**: `Step` is the tier ordinal (1-6 in WoLK), `Rank` is the integer skill value (1-450). Some old code paths conflate them; use only `Rank` for value comparisons.
- **`character_skills.professionSlot`** column: 0 or NULL = secondary, 1 = primary slot 1, 2 = primary slot 2. Used by Faction-Change and "unlearn primary profession" logic to know which slot frees up.
- **TrinityCore's `Player::HandleSpellEffect` for SPELL_EFFECT_SKILL_STEP** is what bumps you between tiers (Apprentice→Journeyman). It's NOT a skill-rank gain — it raises `MaxRank` to the next tier value.
- **Achievement criterion `CRITERIA_TYPE_REACH_SKILL_LEVEL`** fires on every `set_skill` value increase — wire into criteria mgr.
- **Inspect tradeskill (`SHOW_TRADE_SKILL`)**: in WoLK clients, this includes recipe list + materials of the inspected player's professions. Need full recipe list from their spell book filtered by `SkillLineAbility`.
- **Performance**: DO NOT iterate full SkillLineAbility store on every craft — index by skill_line at load time. The C++ uses `sSpellMgr->GetSkillLineAbilityMapBounds(spellId)` which is a precomputed multimap.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `SkillStatus` enum | `enum SkillStatus { Unchanged, Changed, New, Deleted }` | — |
| `SkillStatusData` | `struct SkillStatusData { pos: u8, status: SkillStatus }` | — |
| `mSkillStatus: std::map<uint32, SkillStatusData>` | `BTreeMap<u16, SkillStatusData>` on session | u16 since SkillID fits |
| `SkillDiscoveryEntry` | `struct SkillDiscoveryEntry { spell_id: u32, req_skill_value: u16, chance: f32 }` | — |
| `SkillDiscoveryStore: std::unordered_map<int32, SkillDiscoveryList>` | `HashMap<i32, Vec<SkillDiscoveryEntry>>` | preserve negative-key idiom |
| `SkillExtraItemEntry` | `struct SkillExtraItemEntry { req_specialization: u32, additional_create_chance: f32, additional_max_num: u8 }` | spell_id is the key |
| `SkillPerfectItemEntry` | `struct SkillPerfectItemEntry { req_specialization: u32, perfect_create_chance: f32, perfect_item_type: u32 }` | — |
| `Player::SetSkill(...)` | `fn set_skill(&mut self, skill_id: u16, step: u16, value: u16, max: u16)` | — |
| `Player::GetSkillValue(id)` | `fn get_skill_value(&self, skill_id: u16) -> u16` | |
| `Player::UpdateCraftSkill(spellId)` | `fn update_craft_skill(&mut self, spell_id: u32) -> bool` | returns whether skill went up |
| `LoadSkillDiscoveryTable()` | `fn load_skill_discovery_table(world_db: &Database) -> Result<SkillDiscoveryStore>` | call once at world startup |
| `WorldPackets::Spells::UnlearnSkill` | `pub struct UnlearnSkill { skill_line: u16 }` in `crates/wow-packet/src/packets/spell.rs` | already exists likely |
| `WorldPackets::Spells::TradeSkillSetFavorite` | `pub struct TradeSkillSetFavorite { recipe_id: u32, is_favorite: bool }` | new |
| `sObjectMgr->GetSkillTier(tierId)` | `skill_tiers_store.get_skill_tier_like_cpp(tier_id) -> Option<&SkillTiersEntryLikeCpp>` | world SQL `skill_tiers` representation (#SKILLS.1) |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Verdict: ⚠️ partial — confirmed, but slightly more complete than the doc claims.** Status header stays at "DB2 readers only", however **one** CMSG handler is wired and `character_skills` is read at login.

**Inventory verified:**
- `crates/wow-data/src/skill.rs` — reads `SkillLineAbility.db2` + `SkillRaceClassInfo.db2`, and represents C++ `ObjectMgr::_skillTiers` from world SQL `skill_tiers`. Public API: `load`, `starting_skill_info(race, class, level)`, `starting_spells(...)`, `racial_spells(race)`, `trade_skill_spells(skill_id, known_spells)`, `SkillTiersStoreLikeCpp`. `crates/wow-data/src/skill_talent.rs` also now reads `SkillLine.db2` for represented `GetProfessionSkillForExp` consumers (#NEXT.R8.ENTITIES.325).
- `crates/wow-database/src/statements/character.rs` — has `SEL_CHARACTER_SKILLS`: `SELECT skill, value, max, professionSlot FROM character_skills WHERE guid = ?`. Note: represented consumers can read persisted `value`, but canonical `SkillStatus`, `max`, save/update and `professionSlot` behavior remain incomplete.
- `crates/wow-world/src/handlers/character.rs:1483-1551` — at login: queries `SEL_CHARACTER_SKILLS`, builds `known_skill_ids: HashSet<u16>`, then calls `skill_store.starting_skill_info(race, class, level)` to populate `skill_info_tuples` for the SkillInfo update field array. So per-character skill **slots** are sent on the initial UpdateObject — but their `rank` / `max_rank` come from a hard-coded computation (`level * 5` for weapon-ish skills, tier defaults) rather than persisted DB values.
- `crates/wow-world/src/handlers/character.rs:4466-4501` — `handle_show_trade_skill` IS implemented (the doc says it isn't). It returns `SMSG_SHOW_TRADE_SKILL_RESPONSE` with `skill_rank = level*5, skill_max_rank = level*5` and the known recipe spell IDs filtered by `trade_skill_spells`. Registered via `inventory::submit!` at `character.rs:411`.
- `CMSG_UNLEARN_SKILL` and `CMSG_TRADE_SKILL_SET_FAVORITE`: **not** wired (confirmed — no `inventory::submit!`, no match arm).

**Confirmed bugs / divergences (doc §8 hypotheses):**
1. **No re-evaluation of auto-learn on tier-up**: confirmed. `starting_spells` is called once at login. There is no `SetSkill`/`set_skill` function anywhere — searching `crates/` found 0 matches for `set_skill`, `update_craft_skill`, `update_weapon_skill`, `update_gather_skill`, `update_fishing_skill`. Skill rank can never go up.
2. **No update-field slot allocator**: skills come pre-baked from `starting_skill_info(race, class, level)`. There is no dynamic add/remove. If a player learns a new profession at runtime, no slot is allocated.
3. **`character_skills.max, professionSlot` ignored and bonuses absent**: the login path now reads represented skill values for consumers such as fishing rolls, but there is still no canonical `SkillStatus` state, max-rank tracking, temp/perm bonuses or save/update path.
4. **Faction-change language remap**: confirmed absent (no `DEL_CHAR_SKILL_LANGUAGES`, no `INS_CHAR_SKILL_LANGUAGE`).

**Largest missing surfaces (confirmed):**
- All skill-up / skill-modify functions: `set_skill`, `update_craft_skill`, `update_gather_skill`, `update_weapon_skill`, `update_fishing_skill`, `modify_skill_bonus`, `get_skill_value`, `get_pure_skill_value`, `get_max_skill_value`, `has_skill`.
- `character_skills` write path: no `INS_CHAR_SKILLS`, `UPD_CHAR_SKILLS`, `DEL_CHARACTER_SKILL` prepared statements (verified).
- `skill_discovery_template`, `skill_extra_item_template`, `skill_perfect_item_template` worldserver tables — none loaded. `skill_fishing_base_level` is loaded for represented fishing bobber rolls only (#NEXT.R8.ENTITIES.324).
- Runtime skill-state consumers for `SkillTiersStoreLikeCpp`. `SkillLine.db2` is loaded for represented `GetProfessionSkillForExp` consumers only (#NEXT.R8.ENTITIES.325).
- Profession slot bookkeeping (max 2 primary).
- `CMSG_UNLEARN_SKILL`, `CMSG_TRADE_SKILL_SET_FAVORITE` handlers.
- Equip rejection on skill mismatch and lockpicking integration.
- `mSkillStatus` dirty-tracking map and `_LoadSkills`/`_SaveSkills` semantics.

**Refined estimate: ~10–15% complete** (vs the doc's "partial; DB2 readers only"). Login presents a static skill snapshot to the client, plus profession recipe inspection works for known recipes. Everything dynamic (gain rank, save back, learn next tier) is absent.
