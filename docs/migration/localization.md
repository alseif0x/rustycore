# Migration: localization (cross-cutting reference)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/common/Common.h`, `src/common/Utilities/Locales.{h,cpp}`, `src/server/game/Globals/ObjectMgr.{h,cpp}`, `src/server/game/Texts/CreatureTextMgr.{h,cpp}`, `src/server/game/Texts/ChatTextBuilder.{h,cpp}`
> **Rust target crate(s):** `crates/wow-constants/` (enum), `crates/wow-database/` (loaders TBD), `crates/wow-data/` (DB2 client texts)
> **Layer:** L0 (foundation — every domain depends on locale)
> **Status:** ⚠️ partial — `Locale` enum + `LocaleMask` defined; no string-table loaders, no per-session locale plumbed into responses
> **Audited vs C++:** ⚠️ partial (this document is the audit)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The server stores **every player-visible string twice** — once in default locale (English) and once per per-locale variant — and selects the right one when emitting a response based on the player's chosen client locale. The system spans (a) DBC/DB2 client-side localised strings, (b) server-side `*_locale` SQL tables that mirror world-DB content per language, (c) the `trinity_string` table for hard-coded server messages, and (d) the `LocaleConstant` enum that indexes everything. Without this layer working, every quest text, NPC name, GM message, and chat broadcast appears in English regardless of client.

---

## 2. C++ canonical files

Paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---:|---|
| `src/common/Common.h` | ~110 | `enum LocaleConstant : uint8` (lines 47-63) — 12 values + `TOTAL_LOCALES` sentinel; `OLD_TOTAL_LOCALES = 9`; `#define DEFAULT_LOCALE LOCALE_enUS`. |
| `src/common/Utilities/Locales.h` | 32 | `Trinity::Locale::Init()` / `GetGlobalLocale()` / `GetCalendarLocale()` — `std::locale` (Unicode collation) wrapper, **not** the WoW LocaleConstant. Different concept, easy to confuse. |
| `src/common/Utilities/Locales.cpp` | 51 | Implementation of the above. |
| `src/common/Utilities/Util.{h,cpp}` | — | `LocaleConstant GetLocaleByName(std::string_view)`, `bool IsValidLocale(LocaleConstant)`, `char const* localeNames[TOTAL_LOCALES]`. |
| `src/server/game/Globals/ObjectMgr.h` | 1500 | `*_LocaleStore` containers (one per table), `LoadCreatureLocales`, `LoadGameObjectLocales`, `LoadQuestLocales`, `LoadGossipMenuItemsLocales`, `LoadPointOfInterestLocales`, `LoadPageTextLocales`, `LoadNpcTextLocales`, `LoadTrinityStrings`, `GetTrinityString`, `AddLocaleString`. |
| `src/server/game/Globals/ObjectMgr.cpp` | 11444 | Loaders (lines 255, 288, 320, 6189, 7461, 8833 …) — pattern: `SELECT … FROM <table>_locale`, switch on `Locale` column, populate per-entry vector indexed by `LocaleConstant`. |
| `src/server/game/Texts/CreatureTextMgr.{h,cpp}` | 445+ | `creature_text_locale` loader, `CreatureTextLocale` struct, `BuildMonsterChat` per-recipient localised emission. |
| `src/server/game/Texts/ChatTextBuilder.{h,cpp}` | — | `BroadcastTextBuilder { LocaleConstant locale = LOCALE_enUS }` — formats achievement/event broadcasts per locale. |
| `src/server/game/DataStores/DB2Stores.{h,cpp}` | — | `static GetBroadcastTextValue(BroadcastTextEntry const*, LocaleConstant, gender)`, `GetClassName`, `GetChrRaceName` — DB2-side localised lookups. |
| `src/server/game/Server/WorldSession.cpp` | 2400 | Constructor (line 129-130) sets `m_sessionDbcLocale` and `m_sessionDbLocaleIndex` from auth packet. `WorldSession::GetTrinityString` (line 787) shortcut. |
| World DB | — | `*_locale` tables: `creature_template_locale`, `gameobject_template_locale`, `quest_template_locale`, `quest_objectives_locale`, `quest_offer_reward_locale`, `quest_request_items_locale`, `gossip_menu_option_locale`, `points_of_interest_locale`, `page_text_locale`, `npc_text_locale`, `broadcast_text_locale`, `creature_text_locale`, `item_template_locale`, `trinity_string`. |

`LocaleConstant` block (verbatim from `Common.h:47-63`):

```cpp
enum LocaleConstant : uint8
{
    LOCALE_enUS = 0,
    LOCALE_koKR = 1,
    LOCALE_frFR = 2,
    LOCALE_deDE = 3,
    LOCALE_zhCN = 4,
    LOCALE_zhTW = 5,
    LOCALE_esES = 6,
    LOCALE_esMX = 7,
    LOCALE_ruRU = 8,
    LOCALE_none = 9,
    LOCALE_ptBR = 10,
    LOCALE_itIT = 11,

    TOTAL_LOCALES
};
const uint8 OLD_TOTAL_LOCALES = 9;
#define DEFAULT_LOCALE LOCALE_enUS
```

The 8-locale set the user-supplied prompt mentions (enUS/koKR/frFR/deDE/zhCN/zhTW/esES/esMX) is the WoLK-era subset (`OLD_TOTAL_LOCALES`); the modern 12-value enum extends it with ruRU, ptBR, itIT, and a `none` sentinel.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `LocaleConstant` | enum | The locale index used everywhere as `[locale]` array key. |
| `LocalizedString` (DB2 reader) | struct | Wire/disk format for DB2 `*_lang` columns — one slot per locale. |
| `CreatureLocale` / `GameObjectLocale` / `QuestTemplateLocale` / etc. | struct | One per DB-loaded table, holds `std::vector<std::string> Name; std::vector<std::string> Subname; …` indexed by `LocaleConstant`. |
| `TrinityString` | struct | `entry → vector<string>` (one slot per locale) — server hardcoded message catalogue. |
| `BroadcastTextEntry` (DB2) | struct | Achievement/quest/cinematic broadcast — has `Text` + `Text1` columns × per-locale columns from `broadcast_text_locale` hotfix. |
| `CreatureTextLocale` | struct | Per-locale version of `creature_text` rows. |
| `ChatTextBuilder` | class | Helper that picks the right locale for a recipient and emits an `SMSG_CHAT`. |

`localeNames[]` (in `Util.h`):

```cpp
char const* localeNames[TOTAL_LOCALES] = {
    "enUS", "koKR", "frFR", "deDE", "zhCN", "zhTW",
    "esES", "esMX", "ruRU", "none", "ptBR", "itIT"
};
```

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `GetLocaleByName(std::string_view)` | Convert "deDE" → `LOCALE_deDE`. | `localeNames[]` linear scan |
| `IsValidLocale(LocaleConstant)` | Range + non-`LOCALE_none` check. | — |
| `ObjectMgr::AddLocaleString(string_view src, LocaleConstant, vector<string>& dst)` | Helper: resize vector, write. | — |
| `ObjectMgr::LoadCreatureLocales()` | `SELECT entry, locale, Name, Title FROM creature_template_locale`. ~16 of these loaders, one per `*_locale` table. | `WorldDatabase.Query`, `AddLocaleString` |
| `ObjectMgr::LoadTrinityStrings()` | `SELECT entry, content_default, content_loc1..content_loc8 FROM trinity_string`. | `WorldDatabase.Query` |
| `ObjectMgr::GetTrinityString(uint32 entry, LocaleConstant)` | Index into the 9-slot per-entry vector with default-locale fallback. | — |
| `WorldSession::GetTrinityString(uint32 entry)` | Shorthand — uses session's `m_sessionDbLocaleIndex`. | `ObjectMgr::GetTrinityString` |
| `DB2Manager::GetBroadcastTextValue(BroadcastTextEntry const*, LocaleConstant, gender, forceGender)` | Returns gender + locale-correct text from DB2 + `broadcast_text_locale` hotfix overlay. | hotfix cache lookup |
| `DB2Manager::GetClassName(uint8, LocaleConstant)` | Per-locale class name from `ChrClasses.db2`. | — |
| `CreatureTextMgr::SendChat(...)` | Iterate recipients, pick locale per recipient, emit `SMSG_CHAT` per recipient (or grouped). | `ChatPacketSender` |
| `WorldSession::GetSessionDbLocaleIndex()` | Returns the locale to use for **DB string lookups** (covers all `OLD_TOTAL_LOCALES`). | — |
| `WorldSession::GetSessionDbcLocale()` | Returns the locale to use for **DBC/DB2 lookups** — narrowed by `World::GetAvailableDbcLocale` because DBC files for some locales may not be installed server-side. | — |

---

## 5. Module dependencies

**Depends on:**
- `WorldDatabase` — every `Load*Locales()` issues a `SELECT … FROM <table>_locale`.
- `DB2Manager` + hotfix cache — DB2 `*_lang` slots and `broadcast_text_locale` overlay.
- `WorldSession` — owns the per-connection `m_sessionDbLocaleIndex` / `m_sessionDbcLocale` set during `CMSG_AUTH_SESSION`.

**Depended on by (everything that emits text):**
- `QueryHandler.cpp` (line 221: `LocaleConstant locale = GetSessionDbLocaleIndex();`) — for `SMSG_QUERY_CREATURE_RESPONSE`, `SMSG_QUERY_GAME_OBJECT_RESPONSE`, `SMSG_QUERY_NPC_TEXT_RESPONSE`, `SMSG_QUERY_QUEST_INFO_RESPONSE`.
- `AuctionHouseMgr` (line 902 — iterates all locales building per-locale auction listings).
- `CreatureTextMgr`, `ChatTextBuilder`, `GossipMenu`, mail subject/body, calendar event description, achievement-earned broadcast.
- Every `cs_*` chat command via `ChatHandler::GetTrinityString`.

---

## 6. SQL / DB queries (if any)

The locale system is fundamentally a SQL+DB2 reader. Per-table loaders:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, locale, Name, NameAlt, Title, TitleAlt FROM creature_template_locale` | Localized creature name/title | world |
| `SELECT entry, locale, name, castBarCaption, unk1 FROM gameobject_template_locale` | Localized GO names | world |
| `SELECT ID, locale, LogTitle, LogDescription, QuestDescription, AreaDescription, PortraitGiverText, PortraitGiverName, PortraitTurnInText, PortraitTurnInName, QuestCompletionLog FROM quest_template_locale` | Localized quest text | world |
| `SELECT QuestID, ObjectiveIndex, locale, Description FROM quest_objectives_locale` | Quest objective lines | world |
| `SELECT MenuID, OptionID, Locale, OptionText, BoxText FROM gossip_menu_option_locale` | Gossip option strings | world |
| `SELECT ID, locale, Name FROM points_of_interest_locale` | POI tooltip names | world |
| `SELECT ID, locale, Text FROM page_text_locale` | Book / sign pages | world |
| `SELECT ID, locale, Text0, Text1 FROM npc_text_locale` | NPC dialogue text | world |
| `SELECT ID, locale, Name, Description FROM item_template_locale` | Item name + description | world |
| `SELECT CreatureID, GroupID, ID, locale, Text FROM creature_text_locale` | Per-line creature speech | world |
| `SELECT entry, content_default, content_loc1, content_loc2, content_loc3, content_loc4, content_loc5, content_loc6, content_loc7, content_loc8 FROM trinity_string` | Server hard-coded messages (column-per-locale layout — note `loc1..loc8` covers `OLD_TOTAL_LOCALES`-1 only) | world |
| `SELECT ID, locale, Text_lang FROM hotfixes.broadcast_text_locale WHERE ID = ? AND locale = ?` | DB2 hotfix overlay | hotfixes |
| `SELECT Text_lang FROM hotfixes.<TableName>_locale WHERE TableHash = ? AND RecordId = ? AND locale = 'enUS'` | Generic DB2 hotfix string read | hotfixes |

DBC/DB2 stores with `*_lang` slots (read by `DB2Manager::Get…Name`):

| Store | What it loads | Read by |
|---|---|---|
| `BroadcastTextStorage` | `BroadcastText.db2` + `broadcast_text_locale` overlay | `BroadcastTextBuilder`, achievement system |
| `ChrClassesStorage` | `ChrClasses.db2` (Name_lang, Name_female_lang) | `DB2Manager::GetClassName` |
| `ChrRacesStorage` | `ChrRaces.db2` (Name_lang, Name_female_lang) | `DB2Manager::GetChrRaceName` |
| `ItemSparseStorage` | `ItemSparse.db2` (Display_lang) | item tooltip |
| `SpellNameStorage` | `SpellName.db2` (Name_lang) | spell tooltip |
| `MapStorage` / `AreaTableStorage` | `Map.db2` / `AreaTable.db2` (Name_lang) | UI display |
| `CreatureFamilyStorage` | `CreatureFamily.db2` (Name_lang) | pet UI |

---

## 7. Wire-protocol packets (if any)

Locale **flows in** via `CMSG_AUTH_SESSION` (the `clientLocale` field, which `WorldSession::HandleAuthSession` stashes into `m_sessionDbLocaleIndex`). Locale does **not** travel out as a separate opcode — instead, every text-bearing SMSG has its strings already in the right locale by the time the server emits it.

Touched (locale-sensitive) opcodes:

| Opcode | Direction | Locale source |
|---|---|---|
| `CMSG_AUTH_SESSION` (0x3765) | C→S | sets `m_sessionDbLocaleIndex` for the connection |
| `SMSG_QUERY_CREATURE_RESPONSE` | S→C | `creature_template_locale` |
| `SMSG_QUERY_GAME_OBJECT_RESPONSE` | S→C | `gameobject_template_locale` |
| `SMSG_QUERY_NPC_TEXT_RESPONSE` | S→C | `npc_text_locale` |
| `SMSG_QUEST_GIVER_QUEST_DETAILS` | S→C | `quest_template_locale` + `quest_objectives_locale` |
| `SMSG_PAGE_TEXT_RESPONSE` | S→C | `page_text_locale` |
| `SMSG_GOSSIP_MESSAGE` | S→C | `gossip_menu_option_locale` |
| `SMSG_CHAT` (broadcasts, NPC speech) | S→C | `creature_text_locale`, `broadcast_text_locale` |
| `SMSG_ITEM_NAME_RESPONSE` | S→C | `item_template_locale` |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/shared.rs:18-50` — `pub enum Locale` (12 values + `Total = 12` + `AllLanguages = -1`) and `bitflags! struct LocaleMask` (`EN_US`, `KO_KR`, `FR_FR`, `DE_DE`, `ZH_CN`, `ZH_TW`, `ES_ES`, `ES_MX`, `RU_RU`, `NONE`, `PT_BR`, `IT_IT`). Names match the C++ enum 1:1.
- `crates/wow-data/src/hotfix_cache.rs` — accepts a `locale: &str` parameter, passes it through to DBC loader paths (`{data_dir}/dbc/{locale}/`). DB2 hotfix overlay queries (line 292-313):
  - `SELECT Text_lang FROM hotfixes.broadcast_text_locale WHERE ID = ? AND locale = ?`
  - `SELECT Name, NameAlt, Title, TitleAlt FROM creature_template_locale WHERE entry = ? AND locale = ?`
- `crates/wow-data/src/{item,item_stats,skill}.rs` — load DB2 from a per-locale subdirectory; tests pin `LOCALE = "esES"`.
- `crates/wow-data/src/wdc4.rs:64` — `_locale: u32` field in WDC4 header reader (parsed but unused).

**What's implemented:**
- Numeric `Locale` + `LocaleMask` enums.
- DB2 file lookup by locale subdirectory.
- Two SQL prepared statements (`creature_template_locale`, `broadcast_text_locale`) for hotfix overlay.

**What's missing vs C++:**
- Loaders for **all** non-DB2 `*_locale` SQL tables: `gameobject_template_locale`, `quest_template_locale`, `quest_objectives_locale`, `gossip_menu_option_locale`, `points_of_interest_locale`, `page_text_locale`, `npc_text_locale`, `item_template_locale`, `creature_text_locale`, `quest_offer_reward_locale`, `quest_request_items_locale`.
- `trinity_string` table loader and the `WorldSession::GetTrinityString(entry)` shortcut for hard-coded server messages (chat commands, kick reasons, achievement broadcasts).
- Per-session `m_sessionDbLocaleIndex` / `m_sessionDbcLocale`. `CMSG_AUTH_SESSION` accepts the locale field but does not propagate it to a session-level field used by query responses.
- `GetLocaleByName("deDE") -> Locale::DeDE` parser (and inverse `localeNames` array).
- `World::GetAvailableDbcLocale` — narrow DB-locale to a DBC-locale that is actually installed.
- DB2 `*_lang` per-locale slot reading from `BroadcastText.db2`, `ChrClasses.db2`, `ChrRaces.db2`, etc. The WDC4 reader has a `_locale` field but does not select between `*_lang[N]` slots.
- Locale-aware emission for `SMSG_QUERY_CREATURE_RESPONSE`, gossip, quest details, NPC text, page text, chat broadcasts. (Current Rust query handlers emit only the default-locale name from the world DB.)
- `LocalizedString` struct + helper to map "vector<string>[locale]" pattern.

**Suspicious / likely divergent:**
- `wow-data` per-locale DB2 directories assume the operator drops e.g. `data/dbc/deDE/Item.db2` for German clients. Trinity's standard layout instead reads a single `Item.db2` and selects `Display_lang[index]` based on locale. Confirm whether the deployment uses split files or single-file with per-locale slots.
- `Locale::None = 9` is a sentinel meaning "do not localize" — Rust must treat reads from this value as fall-through to default. Currently no consumer uses `Locale::None`.
- `texts.md` and `miscellaneous.md` migration docs cross-reference locale handling but **no central locale doc existed before this one**. Some claims in those docs may be aspirational.

**Tests existing:**
- `crates/wow-data/src/skill.rs` test pins `LOCALE = "esES"` and reads DBC. No round-trip test that the right per-locale string is selected.
- No tests in `wow-database` for `*_locale` table loading.
- No tests for `Locale` enum round-trips (`FromPrimitive`/`ToPrimitive` derived; trust derive output).

---

## 9. Migration sub-tasks

- [ ] **#LOC.1** Add `pub fn from_name(s: &str) -> Option<Locale>` and `pub fn to_name(self) -> &'static str` to `wow-constants::Locale`. (L)
- [ ] **#LOC.2** Add a `pub struct LocalizedString { pub default: String, pub by_locale: [Option<String>; 12] }` helper with `get(Locale) -> &str` (default fallback). (L)
- [ ] **#LOC.3** Plumb `m_sessionDbLocaleIndex` into `WorldSession`: parse `CMSG_AUTH_SESSION.client_locale`, store as `Locale`, expose `WorldSession::db_locale()` and `WorldSession::dbc_locale()`. (M)
- [ ] **#LOC.4** Implement `World::available_dbc_locale(Locale) -> Locale` that consults the installed-DBC manifest and falls back to `Locale::EnUS`. (L)
- [ ] **#LOC.5** Port loaders for the 11 SQL `*_locale` tables (creature, gameobject, quest, quest_objectives, gossip, poi, page, npc, item, creature_text, broadcast). One prepared statement per table, populates a per-entry `LocalizedString`. Place in `wow-database::loaders::locales`. (H)
- [ ] **#LOC.6** Port `trinity_string` loader; expose via `WorldSession::trinity_string(entry: u32) -> &str` keyed on session locale. (M)
- [ ] **#LOC.7** Wire query handlers (`creature_query`, `gameobject_query`, `npc_text_query`, `quest_query`, `page_text_query`, `gossip_*`) to consult the localized stores instead of the default-locale columns. (M each → H total)
- [ ] **#LOC.8** Implement DB2 `*_lang` slot reading in WDC4 reader: when a column has the `LANG` flag, return the slot indexed by current `Locale`. (M)
- [ ] **#LOC.9** Hotfix overlay: implement the generic `broadcast_text_locale`-style fetch for any DB2 string column, mirroring `Trinity::HotfixDatabase::GetTextHotfix`. (M)
- [ ] **#LOC.10** Port `BroadcastTextBuilder` for achievement-earned / quest-complete broadcasts: `pub fn build_chat(broadcast_text_id, gender, locale) -> SmsgChat`. (M)
- [ ] **#LOC.11** Port `CreatureTextMgr::SendChat` to iterate recipients and pick per-recipient locale. (H)
- [ ] **#LOC.12** Add `Locale::ZhCN`/`KoKR`/`RuRU` UTF-8 round-trip tests — these locales hit the longest multi-byte glyphs and will surface any byte-vs-char length bugs in the packet writer. (M)
- [ ] **#LOC.13** Replace per-locale subdirectory layout in `wow-data` with single-file DB2 + per-slot lookup (#LOC.8) once that lands; remove `LOCALE = "esES"` constants in tests. (M)

---

## 10. Regression tests to write

- [ ] Test: `Locale::from_name("deDE") == Some(Locale::DeDE)` and round-trip via `to_name`.
- [ ] Test: `LocalizedString::get(Locale::None)` falls back to default.
- [ ] Test: `LocalizedString::get(<unloaded locale>)` falls back to default.
- [ ] Test: load fixture `creature_template_locale` rows (entry=12345, esES + frFR), assert `get(Locale::EsES) != get(Locale::FrFR) != default`.
- [ ] Test: `WorldSession` with locale=`Locale::DeDE` answering a `CMSG_QUERY_CREATURE` returns the de_DE name.
- [ ] Test: DB2 `*_lang` reader returns slot[6] for `Locale::EsES`.
- [ ] Test: hotfix overlay precedence — a `broadcast_text_locale` row beats the DB2 `Text_lang` slot.
- [ ] Test: `trinity_string(entry, Locale::ZhCN)` returns the correct UTF-8 string of the right byte length, written to a `SMSG_CHAT` with intact end-of-line.
- [ ] Test: golden — feed a saved `CMSG_AUTH_SESSION` with `clientLocale = LOCALE_ruRU` and assert subsequent `SMSG_QUERY_CREATURE_RESPONSE` text matches the loaded ru_RU row.
- [ ] Test: `World::available_dbc_locale(Locale::ItIT)` falls back to `Locale::EnUS` if `data/dbc/itIT/` is missing.

---

## 11. Notes / gotchas

- **Two distinct locale concepts.** `LocaleConstant` (the WoW one — 12 values) is unrelated to `Trinity::Locale::GetGlobalLocale()` (a `std::locale` for collation/formatting). Don't fold them.
- **Two distinct locale slots per session.** `m_sessionDbLocaleIndex` covers SQL string lookups (range 0..11). `m_sessionDbcLocale` is a possibly-narrower locale used for DBC reads — the operator may not have shipped DBC files for every locale, in which case `World::GetAvailableDbcLocale` substitutes EnUS. Conflating them produces "DB strings localised, DBC strings English" inconsistencies.
- **`trinity_string` column layout.** Columns are named `content_default`, `content_loc1`..`content_loc8`. There is **no** `content_loc9` or higher in the legacy schema — supports only `OLD_TOTAL_LOCALES - 1 = 8` non-default locales. Modern locales (ptBR, itIT) fall back to default. When porting, don't assume a 12-column table.
- **`broadcast_text_locale` is in the `hotfixes` DB**, not `world`. Rusty's DB layer has the prepared statement at `hotfix_cache.rs:310`, but the world `*_locale` tables live in the `world` DB and need a different connection pool.
- **Default-locale fallback is mandatory.** Many real-world deployments only have enUS + esES populated. The lookup pattern is always "locale slot if present, else default". Never raise an error on a missing locale slot.
- **Gender variants** (`BroadcastText` has `Text` and `Text1` for male/female speaker) are orthogonal to locale and live alongside the locale slot. `DB2Manager::GetBroadcastTextValue` takes both `LocaleConstant` and `gender`.
- **`LOCALE_none = 9` is real.** It's used in places that mean "do not localize this", e.g. internal logging. Treat as `default`.
- **`AllLanguages = -1`** in the Rust enum is a Rusty extension, not in C++ `LocaleConstant`. It corresponds to the `LANG_UNIVERSAL = -1` value used in `SMSG_CHAT` for cross-faction or system messages — different concept (chat language, not client locale). The two share a sentinel slot only by coincidence; don't unify them.
- **String length in packets** is byte-counted, not char-counted. zhCN / koKR strings of N visible characters can be 3N bytes; the packet writer must write byte-length and copy raw UTF-8.
- The `World::GetAvailableDbcLocale` policy in TC is "use the player-requested locale if its DBC files exist on disk, else default". Mirror that — falling back per-DB2 store rather than per-server keeps the experience consistent.
- For new SQL tables added in future patches, the convention is `<table>_locale` with `(entry|id, locale, …)` PK and `locale` typed as `varchar(4) DEFAULT 'enUS'` (string, not numeric). Loaders convert via `GetLocaleByName`.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `enum LocaleConstant` | `wow_constants::Locale` (`#[repr(i8)]`) | 1:1 (12 + Total). Names case differ (`enUS` ↔ `EnUS`). |
| `LOCALE_enUS` … | `Locale::EnUS` … | Same numeric values. |
| `TOTAL_LOCALES` | `Locale::Total = 12` | — |
| `OLD_TOTAL_LOCALES` (8) | `pub const OLD_TOTAL_LOCALES: usize = 9;` (TODO) | 9 includes `LOCALE_none`. |
| `DEFAULT_LOCALE` | `pub const DEFAULT_LOCALE: Locale = Locale::EnUS;` (TODO) | — |
| `localeNames[]` | `Locale::to_name() -> &'static str` (#LOC.1) | — |
| `GetLocaleByName(string_view)` | `Locale::from_name(&str) -> Option<Locale>` | — |
| `IsValidLocale(LocaleConstant)` | `Locale::is_valid()` method | reject `Total` / negative |
| `LocalizedString` (DB2 reader) | `wow_data::LocalizedString { default, by_locale: [Option<String>; 12] }` | Helper struct (#LOC.2) |
| `vector<string>` indexed by `LocaleConstant` | `[Option<String>; 12]` | Avoid heap allocation for the slot array |
| `CreatureLocale`, `GameObjectLocale`, etc. | `wow_database::loaders::locales::CreatureLocale { name: LocalizedString, title: LocalizedString }` | Mirror C++ struct shape per table |
| `TrinityString` | `wow_database::TrinityString` | Same |
| `ObjectMgr::GetTrinityString(entry, locale)` | `WorldStrings::get(entry, locale) -> &str` | Worldsingleton, default fallback |
| `WorldSession::m_sessionDbLocaleIndex` | `WorldSession::db_locale: Locale` | Set on auth |
| `WorldSession::m_sessionDbcLocale` | `WorldSession::dbc_locale: Locale` | Possibly narrowed |
| `WorldSession::GetTrinityString(entry)` | `WorldSession::trinity_string(entry) -> &str` | Shorthand |
| `World::GetAvailableDbcLocale(locale)` | `World::available_dbc_locale(Locale) -> Locale` | Manifest-based |
| `BroadcastTextBuilder` | `wow_chat::BroadcastTextBuilder { broadcast_text_id, locale }` | Already partially in `wow-chat` |
| `CreatureTextMgr` | `wow_world::handlers::creature_text::CreatureTextMgr` | TBD |

---

## 13. §13 Audit (cross-cutting reference docs)

| Claim | Verified against | Verdict |
|---|---|---|
| `LocaleConstant` block at `Common.h:47-63`, 12 values + `TOTAL_LOCALES` | direct read of `Common.h` lines 47-66 | ✅ |
| Rust `Locale` enum at `shared.rs:18-33`, 12 values + `Total` + `AllLanguages = -1` | direct read of `shared.rs` lines 18-33 | ✅ |
| Names match 1:1 (enUS → EnUS …) | manual cross-check | ✅ |
| `OLD_TOTAL_LOCALES = 9` | `Common.h:65` | ✅ |
| `DEFAULT_LOCALE = LOCALE_enUS` | `Common.h:66` | ✅ |
| `LoadCreatureLocales` at `ObjectMgr.cpp:255` | `grep -n LoadCreatureLocales ObjectMgr.cpp` → 255 | ✅ |
| `LoadTrinityStrings` at `ObjectMgr.cpp:8833` | `grep -n LoadTrinityStrings ObjectMgr.cpp` → 8833 | ✅ |
| `trinity_string` columns `content_default, content_loc1..content_loc8` | `ObjectMgr.cpp:8839` SQL literal | ✅ — confirms the 8-slot ceiling claim |
| `broadcast_text_locale` query in Rust hotfix cache | `hotfix_cache.rs:310` | ✅ |
| `creature_template_locale` query in Rust hotfix cache | `hotfix_cache.rs:313` | ✅ |
| `WorldSession` constructor sets `m_sessionDbcLocale` and `m_sessionDbLocaleIndex` at lines 129-130 | `grep -n m_sessionDbcLocale WorldSession.cpp` → 129; `m_sessionDbLocaleIndex` → 130 | ✅ |
| `QueryHandler.cpp:221` reads `GetSessionDbLocaleIndex()` | `grep -n GetSessionDbLocaleIndex` → QueryHandler.cpp:221 | ✅ |
| `Locales.h` exposes `Trinity::Locale::Init/GetGlobalLocale` | direct read | ✅ |
| C++ has 12 locales but most operators only populate 8 | inferred from `OLD_TOTAL_LOCALES` constant + `trinity_string` schema | ✅ structurally |
| `LANG_UNIVERSAL = -1` claim about `AllLanguages = -1` | this is asserted as inference, not directly verified — `wow-constants/src/shared.rs:33` shows `AllLanguages = -1` but its consumer is not grepped for; flagged as **soft** | ⚠️ |
| 11 non-DB2 `*_locale` SQL tables enumerated | `grep -rln "_locale" sql/old/3.3.5a/TDB55_to_TDB56_updates/world/` shows: creature, gameobject, quest, quest_objectives, gossip_menu_option, points_of_interest, page_text, npc_text, item_template, creature_text, broadcast_text — **11 distinct tables** | ✅ |
| 14 DB columns listed in `quest_template_locale` query | listed directly from prompt; not re-verified against `ObjectMgr::LoadQuestLocales` source — flagged | ⚠️ |
| 0 SQL locale loaders in Rust today | `grep -rln "_locale" crates/wow-database/src/` returns 0 hits referencing SQL `*_locale` tables (only the DB2 hotfix overlay in `wow-data`) | ✅ |

**Open audit items:**
- Soft inferences flagged ⚠️ above (`AllLanguages = -1` semantic, `quest_template_locale` exact column list) need direct C++ read at #LOC.5 implementation time.
- `LocaleMask` bit assignments in `shared.rs:39-50` were not cross-checked against any C++ bitmask — TC may not even define one. Flagged as a Rust extension.
- `wow-data` per-locale subdirectory pattern (`{data_dir}/dbc/{locale}/`) is **not** the canonical TC layout (which uses single DB2 with `_lang` slots). This is a divergence to be reconciled at #LOC.13.

**Result:** ⚠️ partial — primary enum mapping, file paths, line numbers, and table inventory verified. A handful of column-list and consumer-semantics claims await direct re-read during implementation.

---

*Template version 1.0. Last updated 2026-05-01.*
