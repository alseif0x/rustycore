# Migration: DB Schemas (`auth` / `characters` / `world` / `hotfixes`)

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/sql/`
> **Rust target crate(s):** `crates/wow-database/` (pools + statement registry); deployment glue is currently **absent**.
> **Layer:** L1 (infrastructure — datastores; sits below `datastores.md` which loads DB2/hotfix into typed stores)
> **Status:** ⚠️ partial — DB pools exist (`Database<S>` + 4 statement enums) but **no schema-deploy / Updater equivalent**. Without the four TC schemas pre-applied to MariaDB by hand, every prepared statement fails with `ER_NO_SUCH_TABLE`.
> **Audited vs C++:** ⚠️ partial — schemas counted and grouped vs `crates/wow-database/src/statements/*` 2026-05-01.
> **Last updated:** 2026-05-01

---

## 1. Purpose

TrinityCore (and its wotlk_classic fork at `woltk-trinity-legacy`) ships **four MariaDB/MySQL schemas** that together store every piece of mutable + content data the server uses: account credentials, banlists, realms (`auth`); player save-state (`characters`); GM-curated game content — spawns, loot tables, quests, scripts (`world`); and the server-authored DB2 hotfix overlay that overrides client static data at runtime (`hotfixes`). The Trinity binary contains an **Updater** subsystem that diffs `sql/updates/<db>/<branch>/` against an `updates` table and auto-applies pending deltas at startup. RustyCore inherits the schemas verbatim (binary-compatible client data + identical prepared-statement SQL) but has **no automated apply path**: schemas must currently be loaded manually via `mysql < auth_database.sql` etc. before either binary will boot past pool initialization.

---

## 2. C++ canonical files

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `sql/create/create_mysql.sql` | 20 | Creates the `trinity` user + the four databases with `utf8mb4 / utf8mb4_unicode_ci` and grants. |
| `sql/base/auth_database.sql` | 2924 | Full mysqldump of `auth` (~31 tables). Includes seed rows for `realmlist`, `rbac_*`, `build_info`, `updates`. |
| `sql/base/characters_database.sql` | 3538 | Full mysqldump of `characters` (~114 tables). All tables empty except `updates`/`updates_include`. |
| `sql/base/dev/world_database.sql` | 4897 | Full mysqldump of `world` schema *only* (DDL ~236 tables, no content rows — wotlk content rows ship as separate dumps the project flagged as too large for git in `dev/DO_NOT_IMPORT_THESE_FILES.txt`). |
| `sql/base/dev/hotfixes_database.sql` | 10 764 | Full DDL of `hotfixes` (~443 tables: 3 control tables + ~440 per-DB2 mirror tables, each tracking `VerifiedBuild`). Empty seed. |
| `sql/updates/auth/wotlk_classic/*.sql` | per-delta | Per-version delta SQL applied by Updater in lex-sort order. Filename grammar: `YYYY_MM_DD_NN_<auth\|characters\|world\|hotfixes>[_<short>].sql`. |
| `sql/updates/characters/wotlk_classic/*.sql` | per-delta | Same convention, characters DB. |
| `sql/updates/world/wotlk_classic/*.sql` | per-delta | Same convention, world DB. The repo also keeps loose hand-written deltas at `sql/updates/world/2025_04_03_*.sql` (trainer fixes). |
| `sql/updates/hotfixes/wotlk_classic/*.sql` | per-delta | Same convention, hotfixes DB. |
| `sql/migrations/*.sql` | varies | Out-of-band one-shot migrations not tracked by Updater (the `2025_04_03_08…11_fix_*_trainers.sql` files). |
| `sql/custom/{auth,characters,world,hotfixes}/` | per-mod | Slot for server-operator custom DDL/DML; not run by Updater. |
| `sql/old/` | archived | Historical schemas; ignore. |

Each base dump uses MySQL 8 client format (`/*!40101 SET …*/` envelopes, per-table `LOCK TABLES … WRITE`). Charset is **`utf8mb4 / utf8mb4_unicode_ci`** at DB level; some character columns override to `utf8mb4_bin` for case-sensitive uniqueness (`characters.name`).

---

## 3. Classes / Structs / Enums

There is no C++ class hierarchy here — the canonical artifacts are SQL DDL grouped by domain. The table groups below are the structural unit.

### 3.1 `auth` (~31 tables) — login / realmlist / RBAC

| Group | Tables | Purpose |
|---|---|---|
| Account identity | `account`, `account_access`, `account_banned`, `account_muted`, `secret_digest`, `logs`, `logs_ip_actions` | Game-account creds (SRP6 `salt binary(32)` + `verifier binary(32)`), TOTP secret, banlist, mute list, IP audit log. **No SHA-1 password column** in this branch — auth is SRP6-only; legacy `BINARY(40)` is used for `session_key_auth` (the SRP6 K). |
| BNet identity | `battlenet_accounts`, `battlenet_account_bans`, `account_last_played_character` | Per-user (email) Battle.net account; one BNet account links 1–N `account` rows via `account.battlenet_account` FK. |
| BNet collections | `battlenet_account_heirlooms`, `battlenet_account_mounts`, `battlenet_account_toys`, `battlenet_account_transmog_illusions`, `battle_pets`, `battle_pet_slots`, `battle_pet_declinedname`, `battlenet_item_appearances`, `battlenet_item_favorite_appearances` | Cross-realm cosmetic/pet/transmog ownership keyed by `battlenetAccountId`. |
| Realm registry | `realmlist`, `realmcharacters`, `uptime`, `autobroadcast`, `build_info` | Realm directory served to BNet client; per-realm character counts; per-build telemetry. |
| Bans | `ip_banned`, `account_banned`, `battlenet_account_bans` | IP/account/BNet bans, all with `bandate`/`unbandate` UNIX timestamps; `unbandate=bandate` ⇒ permanent. |
| RBAC | `rbac_permissions`, `rbac_default_permissions`, `rbac_linked_permissions`, `rbac_account_permissions` | Role-based access control: per-account permission grants/denies, per-security-level defaults, transitive linkage. FKs to `account`/`rbac_permissions` with `ON DELETE CASCADE`. |
| Updater bookkeeping | `updates`, `updates_include` | Tracks every applied SQL filename + sha1 + state (`RELEASED`/`ARCHIVED`); Updater diffs filesystem against this on boot. |

### 3.2 `characters` (~114 tables) — player save state

| Group | Tables | Purpose |
|---|---|---|
| Identity & core | `characters`, `character_customizations`, `character_homebind`, `character_stats`, `character_declinedname`, `character_account_data`, `character_tutorial`, `account_data`, `account_tutorial` | The `characters` row (~80 columns) is the master record; `character_*` satellites partition by domain. PK: `guid bigint unsigned`. |
| Inventory | `character_inventory`, `item_instance`, `item_instance_gems`, `item_instance_transmog`, `item_loot_items`, `item_loot_money`, `item_refund_instance`, `item_soulbound_trade_data`, `character_void_storage`, `character_gifts` | `item_instance.guid` is the per-item GUID; `character_inventory(guid, bag, slot)` is the location index. |
| Progression | `character_skills`, `character_spell`, `character_spell_charges`, `character_spell_cooldown`, `character_spell_favorite`, `character_talent`, `character_pvp_talent`, `character_glyphs`, `character_action`, `character_aura`, `character_aura_effect`, `character_aura_stored_location`, `character_currency`, `character_reputation`, `character_trait_config`, `character_trait_entry` | Per-character spellbook, talents (incl. dual-spec via `spec` column), ability cooldowns, faction standings. |
| Quests | `character_queststatus`, `character_queststatus_objectives`, `character_queststatus_objectives_criteria`, `character_queststatus_objectives_criteria_progress`, `character_queststatus_rewarded`, `character_queststatus_daily`/`_weekly`/`_monthly`/`_seasonal`, `quest_tracker` | Per-quest state machine + reset bucketing. |
| Achievements | `character_achievement`, `character_achievement_progress`, `guild_achievement`, `guild_achievement_progress` | Per-character + per-guild achievement bookkeeping. |
| Social | `character_social`, `channels`, `arena_team`, `arena_team_member`, `character_arena_stats`, `petition`, `petition_sign`, `groups`, `group_member`, `lfg_data` | Friend/ignore lists, custom chat channels, arena teams, LFG state. |
| Guilds | `guild`, `guild_member`, `guild_member_withdraw`, `guild_rank`, `guild_bank_tab`, `guild_bank_item`, `guild_bank_right`, `guild_bank_eventlog`, `guild_eventlog`, `guild_newslog` | Guild bank, ranks, audit logs. |
| Mail | `mail`, `mail_items` | Persistent mailbox; `mail_items.mail_id` joins to `mail.id`. |
| Auction house | `auctionhouse`, `auction_items`, `auction_bidders`, `character_favorite_auctions`, `blackmarket_auctions` | Active auctions + bidder history. |
| Calendar | `calendar_events`, `calendar_invites` | Player-created events. |
| Pets | `character_pet`, `character_pet_declinedname`, `pet_aura`, `pet_aura_effect`, `pet_spell`, `pet_spell_charges`, `pet_spell_cooldown` | Per-pet save state keyed by `pet.id`. |
| Instances | `instance`, `character_instance_lock`, `account_instance_times`, `respawn`, `corpse`, `corpse_customizations`, `corpse_phases` | Bound instance IDs + saved corpse positions. |
| Battlegrounds / PvP | `character_battleground_data`, `character_battleground_random`, `pvpstats_battlegrounds`, `pvpstats_players` | Active BG state + season stats. |
| World state | `worldstates`-style: `world_state_value`, `world_variable`, `pool_quest_save`, `game_event_save`, `game_event_condition_save` | Per-realm shared state persisted across restarts. |
| GM / support | `bugreport`, `gm_bug`, `gm_complaint`, `gm_complaint_chatlog`, `gm_suggestion`, `warden_action` | In-game ticketing + Warden anti-cheat actions. |
| Misc cosmetic | `character_equipmentsets`, `character_cuf_profiles`, `character_fishingsteps`, `character_transmog_outfits`, `reserved_name` | Per-character UI/cosmetic prefs. |
| Updater | `updates`, `updates_include` | Same convention as `auth`. |

**No FOREIGN KEYs** in `characters` (verified: zero `FOREIGN KEY` rows). The schema is denormalized for write throughput; referential integrity is enforced application-side. Cleanup of an orphan character (after `DEL_CHARACTER`) requires deleting from ~30 satellite tables manually — TrinityCore does this in `Player::DeleteFromDB`.

### 3.3 `world` (~236 tables) — GM-curated game content

| Group | Representative tables | Purpose |
|---|---|---|
| Creatures (templates) | `creature_template`, `creature_template_difficulty`, `creature_template_addon`, `creature_template_locale`, `creature_template_model`, `creature_template_movement`, `creature_template_resistance`, `creature_template_spell`, `creature_template_sparring`, `creature_template_gossip`, `creature_classlevelstats`, `creature_equip_template`, `creature_model_info`, `creature_movement_info`, `creature_movement_override`, `creature_summon_groups`, `creature_summoned_data`, `creature_text`, `creature_text_locale` | Per-entry NPC archetype (no GUID). `entry` PK. |
| Creatures (spawns) | `creature`, `creature_addon`, `creature_formations`, `creature_questender`, `creature_queststarter`, `creature_questitem`, `creature_onkill_reputation`, `creature_trainer`, `linked_respawn`, `npc_spellclick_spells`, `npc_vendor`, `npc_text` | Per-spawn rows with `(guid, id, map, position_*, spawnDifficulties)`. `guid` PK. |
| GameObjects | `gameobject`, `gameobject_template`, `gameobject_template_addon`, `gameobject_template_locale`, `gameobject_addon`, `gameobject_overrides`, `gameobject_questender`, `gameobject_queststarter`, `gameobject_questitem` | Same template/spawn split as creatures. |
| Items | `item_template_addon`, `item_loot_template`, `item_random_bonus_list_template`, `item_random_enchantment_template`, `item_script_names` | Item base data ships in DB2 (`item`/`item_sparse`); `world` only carries server-side addons + loot tables. |
| Quests | `quest_template`, `quest_template_addon`, `quest_template_locale`, `quest_objectives`, `quest_objectives_completion_effect`, `quest_objectives_locale`, `quest_offer_reward`, `quest_offer_reward_conditional`, `quest_offer_reward_locale`, `quest_request_items`, `quest_request_items_conditional`, `quest_request_items_locale`, `quest_completion_log_conditional`, `quest_description_conditional`, `quest_details`, `quest_greeting`, `quest_greeting_locale`, `quest_mail_sender`, `quest_poi`, `quest_poi_points`, `quest_pool_members`, `quest_pool_template`, `quest_reward_choice_items`, `quest_reward_display_spell`, `quest_visual_effect` | Quest definitions. PK on `quest_template.ID`. |
| Loot | `creature_loot_template`, `gameobject_loot_template`, `item_loot_template`, `disenchant_loot_template`, `fishing_loot_template`, `mail_loot_template`, `milling_loot_template`, `pickpocketing_loot_template`, `prospecting_loot_template`, `reference_loot_template`, `skinning_loot_template`, `spell_loot_template` | Drop tables. All share `(Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount)` shape. |
| Spells (server-side overrides) | `spell_area`, `spell_custom_attr`, `spell_enchant_proc_data`, `spell_group`, `spell_group_stack_rules`, `spell_learn_spell`, `spell_linked_spell`, `spell_pet_auras`, `spell_proc`, `spell_required`, `spell_script_names`, `spell_scripts`, `spell_target_position`, `spell_threat`, `spell_totem_model`, `serverside_spell`, `serverside_spell_effect` | Patches applied on top of DB2 `spell_*` tables. |
| Scripting | `smart_scripts`, `event_scripts`, `event_script_names`, `spell_scripts`, `spell_script_names`, `achievement_scripts`, `areatrigger_scripts`, `script_spline_chain_meta`, `script_spline_chain_waypoints`, `waypoint_path`, `waypoint_path_node`, `criteria_data`, `conditions` | SmartAI rule rows (`smart_scripts.entryorguid` polarity: positive ⇒ creature_template, negative ⇒ creature spawn guid), waypoint paths, conditions DSL. |
| Conditions / disables | `conditions`, `disables`, `phase_area`, `phase_name`, `terrain_swap_defaults`, `terrain_worldmap` | Generic condition system + per-feature kill switches + phasing rules. |
| Battlegrounds / Outdoor PvP | `battleground_template`, `battlefield_template`, `battlemaster_entry`, `outdoorpvp_template`, `pvpstats_*` (in characters) | BG/Battlefield definitions. |
| LFG / Dungeons | `lfg_dungeon_rewards`, `lfg_dungeon_template`, `instance_template`, `instance_spawn_groups`, `access_requirement` | Dungeon Finder rewards + per-instance config. |
| Game events | `game_event`, `game_event_*` (~14 tables) | Holiday/seasonal event scheduling. |
| Player creation | `playercreateinfo`, `playercreateinfo_action`, `playercreateinfo_cast_spell`, `playercreateinfo_item`, `playercreateinfo_spell_custom`, `player_classlevelstats`, `player_racestats`, `player_xp_for_level`, `class_expansion_requirement`, `race_unlock_requirement`, `player_factionchange_*` | Starting position/spells/inventory per (race, class), XP table, faction-change mappings. |
| Pets | `pet_levelstats`, `pet_name_generation`, `mount_definitions` | Per-pet-family scaling, mount-by-spell mapping. |
| Vehicles | `vehicle_template`, `vehicle_template_accessory`, `vehicle_accessory`, `vehicle_seat_addon` | Per-vehicle behavior + passenger slots. |
| World infrastructure | `graveyard_zone`, `world_safe_locs`, `world_state`, `transports`, `areatrigger`, `areatrigger_*` (~9 tables), `gossip_menu`, `gossip_menu_addon`, `gossip_menu_option`, `gossip_menu_option_locale`, `points_of_interest`, `points_of_interest_locale`, `page_text`, `page_text_locale`, `npc_text`, `mail_level_reward` | Maps support tables: graveyards, AT scripts, gossip trees, transports. |
| Trainer (split rework) | `trainer`, `trainer_locale`, `trainer_spell`, `creature_trainer` | Replaced the old `npc_trainer` flat table — `creature_trainer.MenuID` → `trainer.Id` → `trainer_spell` rows. |
| Achievements / playerchoice / scenarios | `achievement_dbc`, `achievement_reward`, `achievement_reward_locale`, `playerchoice` (+5 satellites), `scenarios`, `scenario_poi`, `scenario_poi_points` | Server-side achievement rewards, modern playerchoice system, scenario module. |
| Misc | `command`, `trinity_string`, `version`, `game_tele`, `game_weather`, `reserved_name`, `warden_checks`, `jump_charge_params`, `garrison_*` (2 tables), `conversation_*` (3 tables), `scene_template`, `serverside_spell*` | Console-command registry, localized strings, version stamp, GM teleport list, weather, Warden checks, garrison/conversation/scene scaffolding (8.x carryover). |
| Updater | `updates`, `updates_include` | |

### 3.4 `hotfixes` (~443 tables) — DB2 overlay

| Group | Tables | Purpose |
|---|---|---|
| Control plane | `hotfix_data`, `hotfix_blob`, `hotfix_optional_data` | The 3 *behavioral* tables. `hotfix_data(Id, UniqueId, TableHash, RecordId, Status, VerifiedBuild)` lists every hotfix push the server announces; `hotfix_blob(TableHash, RecordId, locale, Blob, VerifiedBuild)` carries opaque DB2-row replacement bytes (one per locale); `hotfix_optional_data(TableHash, RecordId, locale, Key, Data, VerifiedBuild)` carries per-row tagged blobs (BroadcastText TACT keys). Read by `DB2Manager::LoadHotfixData/Blob/OptionalData`. |
| Per-DB2 mirror | `achievement`, `area_table`, `creature`, `creature_display_info`, `faction`, `faction_template`, `gameobjects`, `item_sparse`, `journal_encounter`, `map`, `quest_*` (~5 tables), `skill_line`, `skill_line_ability`, `spell_*` (~30 tables), `transmog_*` (~7 tables), `ui_map`, `ui_map_assignment`, `vehicle`, `vehicle_seat`, … (~440 total) | One MySQL table per DB2 file, with the same column layout as the DB2 records, plus a `VerifiedBuild` column on each row. Used as a typed alternative to opaque `hotfix_blob` for tools that want to edit individual fields; `DB2Manager` reads these tables via per-table prepared statements (~783 statements counted in the C# `HotfixDatabase`). |
| Locale shadows | `*_locale` (~150 tables) | Localization strips for the per-DB2 tables that carry localized strings (e.g. `achievement_locale`, `faction_locale`, `quest_v2`/`quest_xp` paired with their locale tables). |
| Other | `import_price_armor`, `import_price_weapon`, `import_price_quality` | Pricing helper tables used at content-import time, not at runtime. |
| Updater | `updates`, `updates_include` | |

### 3.5 Cross-DB enums encoded as columns (no FKs)

| Column pattern | Domain | Notes |
|---|---|---|
| `bigint unsigned` | ObjectGUID low part | `characters.guid`, `creature.guid`, `gameobject.guid`, `item_instance.guid`, `mail.id`. The full 64-bit packed GUID is not stored; high-part is reconstructed at load-time from `TypeID`. |
| `int unsigned` | DB2 record id / spell id / item id | E.g. `creature_template.entry`, `quest_template.ID`, `character_spell.spell`. |
| `binary(32)` | SRP6 salt / verifier | `account.salt`, `account.verifier`, `battlenet_accounts.salt`, `battlenet_accounts.verifier`. |
| `binary(40)` | SRP6 session-key K | `account.session_key_auth`. (This is the BINARY(40) size mentioned in older docs — it is **not** a SHA-1 password hash; this branch never stores SHA-1 of passwords.) |
| `varbinary(64)` / `varbinary(128)` | BNet session key / TOTP secret | `account.session_key_bnet`, `account.totp_secret`. |
| `tinyint unsigned` (0/1) | Boolean | `online`, `active`, `granted`, `locked`, `cinematic`, `is_logout_resting`. |
| `varchar(12) … utf8mb4_bin` | Player name | `characters.name` — `utf8mb4_bin` to preserve case-sensitive uniqueness at MySQL level. |
| `enum('RELEASED','ARCHIVED')` | Updater state | Only enum column type used. |

---

## 4. Critical public methods / functions

The schemas have no methods, but the **Updater** is the canonical operational entry-point.

| Symbol | Purpose | Calls into |
|---|---|---|
| `DBUpdater<T>::Update()` | Per-DB orchestrator; called once at boot per pool. | `GetBaseFile()`, `Apply()` |
| `DBUpdater<T>::GetBaseFile()` | Returns `sql/base/<db>_database.sql` (or `sql/base/dev/...`). | mysql client subprocess |
| `DBUpdater<T>::Populate()` | Initial-population path: runs the base dump if `updates` table is empty/missing. | `Apply()` |
| `UpdateFetcher::Update()` | Diffs `sql/updates/<db>/<branch>/` against the `updates` table; reorders by filename, computes sha1, applies each via subprocess. | `mysql` external binary |
| `MySQLConnectionInfo::Parse(str)` | Parses `127.0.0.1;3306;trinity;trinity;auth` 5-token connect string from `*.conf`. | DB pool init |
| `Field::GetUInt64()` / `GetBinary()` etc. | Per-cell typed accessors used by every prepared-statement call site. | — |

The Rust counterpart is `Database<S: StatementDef>` in `crates/wow-database/src/db.rs` (pool + prepared-statement registry). There is **no** `DBUpdater` analogue.

---

## 5. Module dependencies

**Depends on:**

- **MariaDB / MySQL 8** — `utf8mb4_unicode_ci` requires MySQL ≥ 5.6 / MariaDB ≥ 10.1. Trinity targets MySQL 8 / MariaDB 10.6+.
- **`mysql` client binary** — `DBUpdater` shells out to it for SOURCE-style imports of multi-statement dumps; sqlx in Rust would need to chunk-and-execute manually.
- **Filesystem layout** — `sql/base/`, `sql/updates/<db>/<branch>/` paths are hardcoded relative to source root.

**Depended on by:**

- `crates/wow-database/src/statements/login.rs` (148 statements) — every BNet/world-server credential check.
- `crates/wow-database/src/statements/character.rs` (~28 statements wired) — character list/create/load/save.
- `crates/wow-database/src/statements/world.rs` (~120 statements wired) — creature/loot/quest/spell content loaders.
- `crates/wow-database/src/statements/hotfix.rs` (`_PLACEHOLDER` only) — DB2 overlay (currently never queried).
- Every gameplay handler that touches persistent state (almost all of `crates/wow-world/src/handlers/`).
- `bnet-server` and `world-server` binaries — both refuse to start if `auth.realmlist` is missing.

---

## 6. SQL / DB queries (if any)

This module **is** the SQL queries. The exhaustive list lives in `crates/wow-database/src/statements/{login,character,world,hotfix}.rs`. Counts as of 2026-05-01:

| DB | Statements wired in Rust | Statements in C# `LoginStatements` / `CharStatements` / etc. | Coverage |
|---|---|---|---|
| `auth` | 148 (`login.rs`, **1 empty stub**: `SEL_BNET_ACCOUNT_SALT_BY_ID`) | 148 (matches) | ✅ ~99% |
| `characters` | 28 (`character.rs`) | ~280 in TC C++ | ⚠️ ~10% (only login + character-list + minimal save) |
| `world` | ~120 (`world.rs`, **1 empty stub**: `SEL_GAMEOBJECT_TARGET`) | ~200+ in TC C++ | ⚠️ ~60% |
| `hotfixes` | 0 (`hotfix.rs` `_PLACEHOLDER`) | ~783 (per-DB2 + 3 control) | ❌ 0% |

The 4 schemas are queried by Rust as follows (representative per DB):

| Statement (Rust) | Purpose | DB |
|---|---|---|
| `LoginStatements::SEL_REALMLIST` | `SELECT … FROM realmlist WHERE flag <> 3 ORDER BY name` | auth |
| `LoginStatements::SEL_BNET_AUTHENTICATION` | Resolve BNet email + SRP6 verifier | auth |
| `LoginStatements::SEL_CHECK_PASSWORD` | `SELECT salt, verifier FROM account WHERE id = ?` | auth |
| `LoginStatements::INS_ACCOUNT` | Create `account` row with SRP6 salt+verifier | auth |
| `CharStatements::SEL_ENUM` | Character list for a given account | characters |
| `CharStatements::INS_CHARACTER` | Create new player row (22 cols, 22 placeholders) | characters |
| `WorldStatements::SEL_CREATURE_TEMPLATE` | Load NPC archetype | world |
| `WorldStatements::SEL_CREATURES_IN_RANGE` | Spatial spawn loader for grid activation | world |
| `WorldStatements::SEL_HOTFIX_BLOB` | Cross-DB read of `hotfixes.hotfix_blob` from a `world` pool — **only** hotfix table currently queried | hotfixes (via world pool) |

---

## 7. Wire-protocol packets (if any)

Schemas don't carry opcodes directly, but two packet families are bound to the schema layout:

| Opcode | Direction | Schema dependency |
|---|---|---|
| `SMSG_REALM_LIST` (BNet) | server → client | `auth.realmlist` |
| `SMSG_AUTH_RESPONSE` | server → client | `auth.account` (`expansion`, `mutetime`), `auth.account_access`, `auth.account_banned` |
| `SMSG_CHAR_ENUM` | server → client | `characters.characters` + `characters.guild_member` (LEFT JOIN) |
| `SMSG_DB_REPLY` / `SMSG_HOTFIX_PUSH` / `CMSG_DB_QUERY_BULK` | both | `hotfixes.hotfix_data`, `hotfixes.hotfix_blob`, `hotfixes.hotfix_optional_data` |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**

- `crates/wow-database/src/lib.rs` — pool wrapper (`Database<S>`) over `sqlx::MySql`; one type per DB via the `StatementDef` marker trait.
- `crates/wow-database/src/statements/mod.rs` — 93 LOC; exports the 4 enums.
- `crates/wow-database/src/statements/login.rs` — 327 LOC, 148 variants — **complete vs C# `LoginStatements`**.
- `crates/wow-database/src/statements/character.rs` — 284 LOC, ~28 variants — **partial**, character-creation/login path only; missing inventory save, quest save, mail, AH, guild bank, social, calendar, BG, achievements, instance binds, corpse, talents/glyphs save (~250 missing).
- `crates/wow-database/src/statements/world.rs` — 371 LOC, ~120 variants — **partial**, focused on spawn/template loading + GM commands.
- `crates/wow-database/src/statements/hotfix.rs` — 25 LOC, **`_PLACEHOLDER` stub only**.
- `crates/wow-database/src/world_ext/` — additional helpers for spawn loading (referenced in CLAUDE.md).

**What's implemented:**

- 4 typed pools (`Database<LoginStatements>`, `<CharStatements>`, `<WorldStatements>`, `<HotfixStatements>`) read from `BNetServer.conf` / `WorldServer.conf`.
- Prepared-statement registry with compile-time DB-vs-statement type safety.
- Tests assert SQL non-empty and correct `?` count for the wired subset.

**What's missing vs C++:**

- **No schema deploy step.** The four `.sql` dumps are not invoked anywhere in `bnet-server` or `world-server` startup; an operator must run `mysql < auth_database.sql` etc. by hand. Without this, a fresh install fails on the first prepared-statement execution.
- **Updater equivalent now exists.** `DbUpdater::update(source_dir)` reads `updates_include`, walks the configured update directories, hashes files, applies pending deltas, tracks `updates`, and bootstraps default `updates_include` rows when the table is empty for known TC database families. This document's older "no updater" statement is superseded by `docs/migration/database-framework.md`.
- **No `sql/base/dev/world_database.sql` content rows.** Even if the DDL applies, the world DB is empty (TC ships content separately and the `dev/` dump is DDL-only). Importing TDB (`TDB_full_world_*.sql.7z`) is currently a manual operator step.
- **Hotfix overlay completely absent.** No `hotfix_data` / `hotfix_blob` / `hotfix_optional_data` reader, no `SMSG_HOTFIX_PUSH` originator, no per-DB2 mirror-table loaders.
- **Characters DB ~90% missing on the write path.** Save-on-logout for inventory/quest/spell-cooldowns/talents/glyphs/auras is not wired.
- **Charset/collation not enforced from Rust.** Pool URL doesn't append `?charset=utf8mb4` — mismatched client charset will produce `?` for non-ASCII names.

**Suspicious / likely divergent (hypothesis pre-audit):**

- `INS_CHARACTER` lists 22 columns but `xp` is **not** among them — defaults to 0 via the column default, which works only because `xp INT UNSIGNED NOT NULL DEFAULT '0'`. If the schema is updated to remove the default, character creation breaks silently.
- `SEL_CHAR_EQUIPMENT` reads `ci.bag = 0` but newer TC schemas use `bag = 0 AND slot < 19` to bound to equipment; the Rust version drops the `slot < 19` clause (compare `character.rs:166` vs `world_database` compatibility note in TC sources) — this returns body-bag content too if any.
- `SEL_BNET_AUTHENTICATION` includes `COALESCE(ba.salt, 0x000…)` — this hides a NULL salt during account migration. Worth flagging in the PR that wires up account creation.

**Tests existing:**

- 7 tests in `crates/wow-database/src/statements/mod.rs`: SQL-non-empty, expected-table-name, placeholder-count assertions for the wired subset. No integration tests against an actual MariaDB instance.

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** (<1h), **M** (1–4h), **H** (4–12h), **XL** (>12h).

- [ ] **#DBS.1** Add `crates/wow-database/src/installer.rs`: idempotent "run base SQL if `updates` table is empty" routine, called from each binary's startup before pool warmup. (M)
- [ ] **#DBS.2** Vendor (or git-submodule) the four `.sql` dumps from `woltk-trinity-legacy/sql/base/` into `rustycore/sql/base/` so `cargo build` can ship a self-contained installer. (L)
- [ ] **#DBS.3** Implement an `UpdateFetcher` Rust port: scan `sql/updates/<db>/wotlk_classic/`, hash each file (sha1), apply unapplied entries in lex order, INSERT into `updates(name, hash, state, timestamp, speed)`. (H)
- [ ] **#DBS.4** Wire pool URLs to append `?charset=utf8mb4&collation=utf8mb4_unicode_ci&time_zone=%2B00%3A00` so non-ASCII player names round-trip. (L)
- [ ] **#DBS.5** Populate `HotfixStatements` from C# `HotfixDatabase` (~783 statements) and the 3 control-table SELECTs. Either codegen from the C# file or hand-port the 3 controls + the ~30 most-used per-DB2 (`achievement`, `area_table`, `creature_*`, `faction*`, `item_sparse`, `map`, `quest*`, `skill_line*`, `spell_*`). (XL — split across phases) |
- [ ] **#DBS.6** Extend `CharStatements` to cover inventory save (`UPD_ITEM_INSTANCE`, `INS_CHARACTER_INVENTORY`, …), quest save (`INS_CHARACTER_QUESTSTATUS`, `INS_CHARACTER_QUESTSTATUS_OBJECTIVES`, …), social, mail, AH, guild bank — target ~120 of the ~280 C# statements. (XL — split per domain)
- [ ] **#DBS.7** Document operator install runbook (`docs/operations/db-bootstrap.md`): create user, create 4 DBs, source the 4 base dumps, run TDB world content import, smoke-test connection from `world-server`. (M)
- [ ] **#DBS.8** Add an integration test target: spin up MariaDB in CI, apply schemas, run pool-warmup + a SELECT on every wired statement to detect column drift before runtime. (H)
- [ ] **#DBS.9** Replace `SEL_GAMEOBJECT_TARGET` / `SEL_BNET_ACCOUNT_SALT_BY_ID` empty-string stubs with real SQL or remove the variants entirely. (L)
- [ ] **#DBS.10** Add a schema-version sentinel in code (`pub const REQUIRED_TDB_VERSION: &str = "TDB_full_world_3.4.3_…";`) compared against `world.version` table on boot. (L)

---

## 10. Regression tests to write

- [ ] Test: `installer::ensure_schema_applied` is idempotent (runs twice, second time does nothing).
- [ ] Test: each prepared statement in `LoginStatements` parses successfully against a freshly-applied `auth_database.sql` (no `ER_NO_SUCH_TABLE`, no column-name typo).
- [ ] Test: same as above for `CharStatements` against `characters_database.sql`.
- [ ] Test: same as above for `WorldStatements` against `world_database.sql`.
- [ ] Test: `INS_ACCOUNT` placeholder count (7) matches column count (7) — already exists; mirror it for every INS/UPD with > 5 placeholders.
- [ ] Test: charset round-trip — insert and retrieve a Cyrillic / CJK player name and verify byte-equality.
- [ ] Test: Updater applies a fixture delta SQL only once, records the sha1, and refuses to re-apply when the file is unchanged.
- [ ] Test: Updater detects a mutated update file (different sha1) and surfaces a warning rather than silently re-running.
- [ ] Test: `realmlist.gamebuild = 54261` after fresh install (current seed ships `54261`).
- [ ] Test: a canary `hotfix_blob` row round-trips through `SEL_HOTFIX_BLOB` and produces the same bytes given to it.

---

## 11. Notes / gotchas

- **`account.session_key_auth` is `binary(40)`**, not `binary(20)`. The 40-byte K is the SRP6 session key (not a SHA1 hash) — earlier docs/agents have confused this. SHA1 is 20 bytes; SRP6 K via `SHA1(A | B)`-derived interleave is 40.
- **`account.salt` and `account.verifier` are `binary(32)`** — not the AC/MaNGOS legacy `varchar(64)` of hex digits. Always handle as raw bytes; DBeaver displays them as hex but they are not.
- **`characters.name` collation is `utf8mb4_bin`** while every other text column is `utf8mb4_unicode_ci`. `idx_name` is therefore case-sensitive — `Foo` and `foo` are distinct names. Do not lowercase in WHERE clauses.
- **`creature_template.entry` is the world-DB primary key**, but at runtime the spawn rows in `creature` use a **separate** `guid` PK. The C# `CreatureCreateData` field name `entry` (used by the `MapManager` migration discussed in `CLAUDE.md`) refers to the **template** entry, not the spawn `guid` — losing this distinction was one of the failures in `_attic/`.
- **`smart_scripts.entryorguid` polarity**: positive value ⇒ `creature_template.entry`; negative value ⇒ `-creature.guid` (per-spawn override). Both are valid; loaders must handle both signs.
- **Charset configuration**: `create_mysql.sql` declares `utf8mb4 / utf8mb4_unicode_ci` at DB level; the per-table dumps then *redeclare* the same. A pool that connects without `charset=utf8mb4` may silently negotiate `utf8mb3` or `latin1` and corrupt non-ASCII text. The user-spec hypothesis "TC uses `utf8mb4_general_ci`" is **incorrect for this branch** — verified `_unicode_ci` in `sql/create/create_mysql.sql` and in every `CREATE TABLE` line in the four base dumps.
- **No `FOREIGN KEY` in `characters` schema** (verified: zero matches for `FOREIGN KEY` in `characters_database.sql`). All cross-table cleanup is application-side. `auth` does have FKs (`account.battlenet_account → battlenet_accounts.id`, all 4 `rbac_account_permissions` FKs with `ON DELETE CASCADE`).
- **Hotfix tables** in `hotfixes/` mirror DB2 structures one-for-one but do **not** currently carry the `Verified` constraint metadata that DB2 files do — `VerifiedBuild` is the only safety net. A row inserted for a build that doesn't match the client build will simply be ignored client-side.
- **Updater filename grammar**: `YYYY_MM_DD_NN_<db>[_<short>].sql`, where `NN` is a per-day sequence. `YYYY_MM_DD_NN` must be unique per DB — re-using the prefix in two files breaks the lex sort. Local hand-written deltas at `sql/migrations/2025_04_03_*.sql` deliberately omit the `_<db>_` token and live outside Updater's scope.
- **Charset / `mutetime`**: TrinityCore's `mutetime` (auth) is `bigint` with negative meaning "muted on next login"; truncating to `int` (RustyCore had this in an early commit) silently drops mutes set far in the future.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `LoginDatabaseConnection` | `Database<LoginStatements>` | Per-DB type-state guarantees only the right enum is used. |
| `CharacterDatabaseConnection` | `Database<CharStatements>` | Same. |
| `WorldDatabaseConnection` | `Database<WorldStatements>` | Same. |
| `HotfixDatabaseConnection` | `Database<HotfixStatements>` (`_PLACEHOLDER` only) | Stub. |
| `enum LoginDatabaseStatements` | `pub enum LoginStatements { … }` (148 variants) | 1:1. |
| `PreparedStatementBase` | `sqlx::query::Query<MySql, …>` bound at call site | sqlx prepares lazily. |
| `MySQLConnectionInfo` | Pool URL string in `*.conf`, parsed by `wow-config` | — |
| `DBUpdater<T>::Update()` | **TODO** — `wow_database::installer::ensure_applied(db)` (#DBS.1, #DBS.3) | Not yet written. |
| `UpdateFetcher` | **TODO** — `wow_database::updater::UpdateFetcher` (#DBS.3) | Not yet written. |
| `Field::GetUInt64()` etc. | `Row::try_get::<u64, _>(idx)` (sqlx) | Per-cell typed reads. |
| `bigint unsigned` (PK guid) | `u64` (ObjectGUID low part); high part reconstructed via `wow_core::ObjectGuid` | — |
| `binary(32)` (SRP6 salt/verifier) | `[u8; 32]` | Pass through `wow_crypto::srp6` unchanged. |
| `binary(40)` (`session_key_auth`) | `[u8; 40]` | SRP6 K, not SHA1. |
| `varbinary(64)` (`session_key_bnet`) | `Vec<u8>` (length 64) | — |
| `enum('RELEASED','ARCHIVED')` | `enum UpdateState { Released, Archived }` (TODO) | Updater port. |
| `utf8mb4_bin` collation | URL param + per-column comparison rules | Handle in pool config (#DBS.4). |
| `mutetime bigint` | `i64` | Don't truncate to `i32`. |

---

## 13. Audit (2026-05-01)

**Method:** Directly inspected `sql/base/auth_database.sql`, `sql/base/characters_database.sql`, `sql/base/dev/world_database.sql`, `sql/base/dev/hotfixes_database.sql`, `sql/create/create_mysql.sql` in the legacy tree; counted `^CREATE TABLE` matches; sampled per-table DDL for `account`, `realmlist`, `updates`, `characters`, `character_inventory`, `creature`, `creature_template`, `hotfix_blob`, `hotfix_data`, `hotfix_optional_data`. Cross-referenced against `crates/wow-database/src/statements/{login,character,world,hotfix}.rs` and `crates/wow-database/src/statements/mod.rs` test cases. Verified `FOREIGN KEY` count in each base dump (auth: 10; characters: 0; world: 0; hotfixes: 0).

**Findings:**

- **Table counts:** auth 31; characters 114; world 236; hotfixes 443 — totals confirmed by `grep -c '^CREATE TABLE'`. The user-supplied estimate ("auth `realmlist`/`rbac_*`/`account`", "characters ~80–100", "world ~200+", "hotfixes ~280") is correct for auth but **understates characters by ~10%, world by ~10%, and hotfixes by ~58%** — hotfixes ships ~440 per-DB2 mirror tables in this branch, not ~280.
- **Charset/collation:** `utf8mb4 / utf8mb4_unicode_ci` is the canonical setting (confirmed in `sql/create/create_mysql.sql` lines 5–11 and in every `ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci` clause in the 4 base dumps). The user spec hypothesized `utf8mb4_general_ci`; **this is wrong for this branch.** One column override exists: `characters.name` uses `utf8mb4_bin` for case-sensitive uniqueness.
- **Key-column types:** `account.salt` / `account.verifier` are `binary(32)` (SRP6, 256-bit). `account.session_key_auth` is `binary(40)` (SRP6 K, **not** SHA-1 password hash — clarified in §11). All GUID-bearing columns are `bigint unsigned`. Updater `hash` is `char(40)` for SHA-1 hex.
- **Updater filename grammar:** Confirmed via `ls sql/updates/*/wotlk_classic/`. Format: `YYYY_MM_DD_NN_<db>[_<short>].sql` (e.g. `2024_08_17_00_auth.sql`, `2025_04_03_01_fix_trainer_npcflag_16276.sql`). Filename lex-sort = apply order.
- **Foreign keys:** `auth` has 10 FK constraints (battlenet linkage, RBAC); `characters` / `world` / `hotfixes` have **zero** FKs. Migration order is therefore unconstrained at the DB level — the only ordering constraint is *within* `auth` (battlenet_accounts before account; rbac_permissions before rbac_account_permissions).
- **Rust statement coverage:** auth ~99% (148/148 wired, 1 empty stub); characters ~10% (28/~280); world ~60% (120/~200); hotfixes 0% (`_PLACEHOLDER`). Two intentionally-empty statements: `LoginStatements::SEL_BNET_ACCOUNT_SALT_BY_ID` and `WorldStatements::SEL_GAMEOBJECT_TARGET`.
- **Schema-deploy gap:** Confirmed by `grep -r 'auth_database.sql\|characters_database.sql\|create_mysql.sql' /home/server/rustycore` — **zero matches**. No code in either binary references the SQL dumps. Operators are silently expected to apply them by hand; without that step the binaries fail at first prepared-statement execution.
- **No Updater:** No `crates/wow-database/src/updater.rs`, no `installer.rs`, no `updates` table read or written from any Rust call site.
- **`world_database.sql` is DDL-only.** The `dev/` location and the sibling `DO_NOT_IMPORT_THESE_FILES.txt` (3 lines, names the large content dumps) confirm the actual game content (creature/quest/loot rows, ~hundreds of MB) ships out-of-tree as TDB (`TDB_full_world_<version>.sql.7z`). RustyCore needs a separate import step for content even after schema deploy.

**Critical points:**

1. **Without manual schema deploy, RustyCore is non-bootable on a clean MariaDB.** First contact between the binary and the DB is a prepared-statement fetch; that fails as `ER_NO_SUCH_TABLE`. This is the single highest-priority infrastructure gap.
2. **TrinityCore Updater auto-applies; RustyCore does not.** Per `worldserver.md` audit, no Rust analogue exists.
3. **Charset assumption to verify:** the user-spec said "TC uses `utf8mb4_general_ci` or similar"; **the actual answer is `utf8mb4_unicode_ci`** (with `utf8mb4_bin` on player names). RustyCore pool URLs do not currently pin a charset, so a wrong-default client locale silently corrupts non-ASCII text on insert.
4. **Hotfixes is the largest single coverage gap** (0/783 statements), but it is also the lowest-priority *for boot* — if the `hotfix_data` / `hotfix_blob` / `hotfix_optional_data` tables are simply empty (which happens on a fresh install), the server boots and runs with vanilla DB2 client data. The 3 control-table SELECTs are the minimum to be "non-broken"; the ~440 per-DB2 mirror selects can land per-feature as content is hand-edited.
5. **Characters DB write-side is thinner than reads.** A character can log in, but ~90% of state mutations (inventory pickup, quest accept/complete, talent learn, gold spend, mail receive, AH bid, etc.) lack save statements — meaning a server restart loses recent player progress in many domains.

**Status verdict:** ⚠️ partial. Pools + read-side schema knowledge are usable; deployment, write-side persistence, and hotfix overlay are not. Sub-tasks #DBS.1 (installer), #DBS.3 (Updater port), #DBS.5 (HotfixStatements), and #DBS.6 (CharStatements completion) are the highest-leverage next steps.

---

*Doc version: 1.0 (2026-05-01). Update when installer/Updater lands or when statement coverage % changes by ≥5pt.*
