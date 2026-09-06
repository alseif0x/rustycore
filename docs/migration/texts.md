# Migration: Texts (CreatureTextMgr + ChatTextBuilder)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Texts/`
> **Rust target crate(s):** `crates/wow-chat/` for the senders/builders, `crates/wow-database/` for the loader, `crates/wow-world/` for the per-creature repeat-tracking glue
> **Layer:** L0 (foundation — every boss talk, NPC yell, scripted whisper goes through this)
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete (3 files, ~825 lines total)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`CreatureTextMgr` is the indirection layer between scripts/AI and the chat wire format. Scripts call `sCreatureTextMgr->SendChat(creature, textGroup, …)` with a *symbolic* group id (e.g. "boss-aggro = group 0"); the manager picks one text from that group at random (weighted by probability, avoiding immediate repeats), localizes it per-recipient, plays the associated sound/emote, and routes the resulting `SMSG_CHAT` packet to the right audience (range / area / zone / map / world / personal whisper). Every boss yell, every quest-giver line, every "Ow!" emote in a script flows through this. `ChatTextBuilder` provides the polymorphic builder objects that emit the correct `WorldPackets::Chat::Chat` from various source kinds (broadcast text, custom string, format-string, creature-text).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Texts/ChatTextBuilder.cpp` | 92 | `prefix` |
| `game/Texts/ChatTextBuilder.h` | 124 | `prefix` |
| `game/Texts/CreatureTextMgr.cpp` | 445 | `prefix` |
| `game/Texts/CreatureTextMgr.h` | 130 | `prefix` |
| `game/Texts/CreatureTextMgrImpl.h` | 164 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Texts/CreatureTextMgr.h` | 131 | Public API: `CreatureTextEntry`, `CreatureTextMap`, `CreatureTextMgr` singleton, `CreatureTextRange`, `SoundKitPlayType` |
| `src/server/game/Texts/CreatureTextMgr.cpp` | 446 | Loader (`LoadCreatureTexts`, `LoadCreatureTextLocales`), `SendChat`, range broadcasters, `GetLocalizedChatString` |
| `src/server/game/Texts/CreatureTextMgrImpl.h` | 165 | Templated `SendChatPacket<Builder>` + `CreatureTextLocalizer` (one packet cached per `LocaleConstant` per send) |
| `src/server/game/Texts/ChatTextBuilder.h` | 125 | `ChatPacketSender` + four builder kinds: `BroadcastTextBuilder`, `CustomChatTextBuilder`, `TrinityStringChatBuilder`, `CreatureTextTextBuilder` |
| `src/server/game/Texts/ChatTextBuilder.cpp` | ~110 | Builder operator() implementations (not read but inferred from header) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CreatureTextMgr` | class (singleton) | Registry + dispatch |
| `CreatureTextEntry` | struct | One row of `creature_text` (creature, group, id, text, type, lang, prob, emote, duration, sound, soundPlayType, broadcastTextId, range) |
| `CreatureTextLocale` | struct | `vector<string> Text[locale]` — translated variants |
| `CreatureTextId` | struct | `(entry, textGroup, textId)` triple — key for the locale map; has `<=>` for ordering |
| `CreatureTextRange` | enum | `NORMAL=0`, `AREA=1`, `ZONE=2`, `MAP=3`, `WORLD=4`, `PERSONAL=5` |
| `SoundKitPlayType` | enum class | `Normal=0`, `ObjectSound=1` |
| `CreatureTextHolder` | typedef | `unordered_map<u8 groupId, vector<CreatureTextEntry>>` |
| `CreatureTextMap` | typedef | `unordered_map<u32 creatureEntry, CreatureTextHolder>` |
| `LocaleCreatureTextMap` | typedef | `map<CreatureTextId, CreatureTextLocale>` |
| `Trinity::ChatPacketSender` | class | Holds an untranslated + lazily-translated `WorldPackets::Chat::Chat`; `operator()(Player const*)` sends the right one |
| `Trinity::BroadcastTextBuilder` | class | Wraps a `BroadcastText.db2` id + gender; `operator()(LocaleConstant)` returns a fresh `ChatPacketSender` |
| `Trinity::CustomChatTextBuilder` | class | Wraps an explicit `string_view` + language |
| `Trinity::TrinityStringChatBuilder` | class | Wraps a `trinity_string` ID with `va_list` args |
| `Trinity::CreatureTextTextBuilder` | class | Wraps `(speaker, gender, msgType, group, id, language)` for replays of cached creature text |
| `CreatureTextLocalizer<Builder>` | template | Per-send object that caches one `ChatPacketSender` per locale-index |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CreatureTextMgr::LoadCreatureTexts()` | Read `creature_text` (prepared `WORLD_SEL_CREATURE_TEXT`); validate Sound/Emote/Type/Lang/BroadcastTextId/Range against DB2 stores; populate `mTextMap` | DB2 stores, `LanguageMgr` |
| `CreatureTextMgr::LoadCreatureTextLocales()` | Read `creature_text_locale` into `mLocaleTextMap` | `ObjectMgr::AddLocaleString` |
| `CreatureTextMgr::SendChat(source, textGroup, whisperTarget?, msgType?, language?, range?, sound?, playType?, team?, gmOnly?, srcPlr?)` | Pick a random non-recently-said entry weighted by `probability`; resolve sound (override or BroadcastText-derived), emote, range; build a `CreatureTextTextBuilder`; call `SendChatPacket`; mark the id as said via `Creature::SetTextRepeatId`; **returns the `duration` field** (used by AI to schedule the next line) | `Containers::SelectRandomWeightedContainerElement`, `SendSound`, `SendEmote`, `SendChatPacket`, `Creature::GetTextRepeatGroup`/`SetTextRepeatId`/`ClearTextRepeatGroup` |
| `CreatureTextMgr::TextExist(entry, group)` | Existence probe used by AI sanity asserts | — |
| `CreatureTextMgr::GetLocalizedChatString(entry, gender, group, id, locale)` | Resolve the translated string for arbitrary entry/group/id (used outside the chat dispatch — e.g. spell logs) | `DB2Manager::GetBroadcastTextValue`, `ObjectMgr::GetLocaleString` |
| `CreatureTextMgr::SendSound(source, sound, msgType, whisperTarget?, range?, team?, gmOnly?, broadcastTextId?, playType?)` | Send `SMSG_PLAY_SOUND` or `SMSG_PLAY_OBJECT_SOUND` to the same audience as the chat would go | `SendNonChatPacket` |
| `CreatureTextMgr::SendEmote(unit, emote)` | Trigger a visual emote (`HandleEmoteCommand`) | `Unit::HandleEmoteCommand` |
| `CreatureTextMgr::SendChatPacket<Builder>(source, builder, msgType, …)` (template) | Generic dispatcher: instantiates `CreatureTextLocalizer<Builder>`, switches on range, calls the worker against the correct player set | `Cell::VisitWorldObjects`, `Group::BroadcastWorker`, `Map::GetPlayers`, `World::GetAllSessions` |
| `CreatureTextMgr::SendNonChatPacket(...)` | Same range-dispatch logic but for arbitrary packets (sound/object-sound) | same as above |
| `static GetRangeForChatType(msgType)` | YELL/EMOTE/SAY → distance from `CONFIG_LISTEN_RANGE_*` | `World::getFloatConfig` |
| `Trinity::ChatPacketSender::operator()(Player const*)` | Decide translated vs untranslated based on `LanguageMgr` + recipient skill, send the right cached packet | — |
| `Trinity::*Builder::operator()(LocaleConstant)` | Build and return a fresh `ChatPacketSender` for the requested locale | — |

---

## 5. Module dependencies

**Depends on:**
- `Database` — prepared `WORLD_SEL_CREATURE_TEXT`, plain query for locales.
- `DataStores/DB2Stores` — `sBroadcastTextStore`, `sSoundKitStore`, `sEmotesStore` for validity checks; `DB2Manager::GetBroadcastTextValue` for translation.
- `Globals/LanguageMgr` — language existence + skill-based translation gating.
- `Globals/ObjectMgr` — `GetScriptId`, `AddLocaleString`, `GetLocaleString`.
- `Server/Packets/ChatPackets` — `WorldPackets::Chat::Chat`.
- `Server/Packets/MiscPackets` — `WorldPackets::Misc::PlaySound`, `PlayObjectSound`.
- `Grids/Cell` + `GridNotifiers::PlayerDistWorker` — TEXT_RANGE_NORMAL distance broadcast.
- `Maps/Map` — `GetPlayers()` for AREA/ZONE/MAP ranges.
- `Server/World` — `GetAllSessions`, listen-range floats.
- `Groups/Group` — `BroadcastWorker` for `CHAT_MSG_MONSTER_PARTY`.

**Depended on by:**
- All `wow-script` boss/quest scripts — every `Talk(group)` call in C++ becomes a `CreatureTextMgr::SendChat`.
- AI: `SmartAI` / `ScriptedAI` use `Talk` extensively; the `duration` return value drives `Talk()`'s sleep-then-next behaviour.
- `Creature` itself — owns `m_textRepeat`, used to avoid saying the same line twice in a row.

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `WORLD_SEL_CREATURE_TEXT` (prepared) | All rows of `creature_text` | world |
| Inline `SELECT … FROM creature_text_locale` | Localized variants | world |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `sBroadcastTextStore` | `BroadcastText.db2` | LoadCreatureTexts (validity), SendChat (sound override), GetLocalizedChatString (translation) |
| `sSoundKitStore` | `SoundKit.db2` | LoadCreatureTexts (validity) |
| `sEmotesStore` | `Emotes.db2` | LoadCreatureTexts (validity) |

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_CHAT` | server → client | All `SendChat` paths via `WorldPackets::Chat::Chat` |
| `SMSG_PLAY_SOUND` | server → client | `SendSound` (Normal play type) |
| `SMSG_PLAY_OBJECT_SOUND` | server → client | `SendSound` (ObjectSound play type) |
| `SMSG_EMOTE` | server → client | `SendEmote` via `Unit::HandleEmoteCommand` |

This module *only* sends; nothing client-originated routes here.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-database/src/statements/world.rs:14` declares the prepared statement constant `SEL_CREATURE_TEXT` with the right column list (`SELECT CreatureID, GroupID, ID, Text, Type, Language, Probability, Emote, Duration, Sound, SoundPlayType, BroadcastTextId, TextRange FROM creature_text` at line 132). No loader, no struct, no `SendChat`, no builders.
- No `creature_text.rs`, no `text_mgr.rs` anywhere.
- Boss/scripted talks elsewhere in the codebase are stubbed out or short-circuit-routed through ad-hoc `SMSG_CHAT` constructions.

**What's implemented:** the prepared statement string. That's it.

**What's missing vs C++:**
- `CreatureTextMgr` registry and loader.
- `CreatureTextLocale` table loader.
- The weighted-random + repeat-avoidance `SendChat` pick logic.
- The four range broadcasters (AREA/ZONE/MAP/WORLD/PERSONAL) and the cell-visit fallback for NORMAL.
- Per-locale packet caching (`CreatureTextLocalizer`).
- `ChatPacketSender` and the four builder kinds.
- `Creature` text-repeat memory (the `m_textRepeat` field needs to live on the `WorldCreature` in `MapManager`).

**Suspicious / likely divergent:** any current Rust scripted boss talk uses a different code path that won't share repeat-avoidance with `CreatureTextMgr` once the mgr exists. Audit needed at integration time.

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#TEXTS.WBS.001** Cerrar la migracion auditada de `game/Texts/ChatTextBuilder.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/ChatTextBuilder.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TEXTS.WBS.002** Cerrar la migracion auditada de `game/Texts/ChatTextBuilder.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/ChatTextBuilder.h`
  Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TEXTS.WBS.003** Cerrar la migracion auditada de `game/Texts/CreatureTextMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`
  Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TEXTS.WBS.004** Cerrar la migracion auditada de `game/Texts/CreatureTextMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h`
  Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TEXTS.WBS.005** Cerrar la migracion auditada de `game/Texts/CreatureTextMgrImpl.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`
  Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#TXT.1** Add `CreatureTextEntry` struct + `CreatureTextRange`/`SoundKitPlayType` enums in `wow-chat`. Implement `CreatureTextMgr::load(pool)` reading via `SEL_CREATURE_TEXT`. Validate Sound/Emote/Lang/Type/BroadcastTextId/Range with the same warn-and-default behaviour. (complexity: **M**)
- [ ] **#TXT.2** Add `CreatureTextLocale` and the `(entry, group, id) → Vec<String> per-locale` loader. (complexity: **L**)
- [ ] **#TXT.3** Implement `SendChat`: weighted pick (`rand` + `Vec<f32>` probabilities), repeat-avoidance via a `text_repeat: HashMap<u8 group, Vec<u8 ids>>` field on `WorldCreature`. Return the `duration` ms. (complexity: **M**)
- [ ] **#TXT.4** Define a `Builder` trait in `wow-chat` and the four impls (`BroadcastTextBuilder`, `CustomChatTextBuilder`, `RustyStringChatBuilder`, `CreatureTextTextBuilder`). Implement `ChatPacketSender` with per-locale lazy translation. (complexity: **M**)
- [ ] **#TXT.5** Implement the range dispatcher (generic over `Builder`): NORMAL = grid-cell visit through `MapManager`; AREA/ZONE/MAP = filter `Map`'s player list; WORLD = iterate `PlayerRegistry`; PERSONAL = single `whisperTarget`; `MONSTER_PARTY` = group-broadcast. (complexity: **H** — touches many subsystems)
- [ ] **#TXT.6** Implement `SendSound`/`SendEmote` and `SendNonChatPacket` (mirror dispatcher logic for arbitrary `Vec<u8>` payload, no `Builder`). (complexity: **M**)
- [ ] **#TXT.7** Implement `GetLocalizedChatString(entry, gender, group, id, locale)` for use outside SendChat (logs, debugging). (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#TEXTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 955 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h`. Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`. | `cargo test -p wow-chat && cargo test -p wow-database && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#TEXTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 955 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h`. Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#TEXTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 955 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h`. Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#TEXTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 955 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h`. Rust target: `crates/wow-chat`, `crates/wow-database`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Loader: a `creature_text` row with a non-existent `Sound` is loaded with `sound = 0` and an error logged.
- [ ] `SendChat` weighted pick with seed S over 10K iterations matches the C++ histogram for a fixture group.
- [ ] Repeat avoidance: after saying every id in a group, the `text_repeat` set clears and the cycle repeats — never returns `id = N` twice in a row except after `ClearTextRepeatGroup`.
- [ ] `duration` return value matches the picked entry's `duration` column.
- [ ] Range = ZONE: a player in a different zone on the same map does NOT receive the packet; a player in the same zone on a different map (impossible, but sanity) is filtered.
- [ ] `MONSTER_PARTY` whisper targeting a player in a group sends to all group members; targeting a soloer sends to that soloer only (NOT to the wider area).
- [ ] BroadcastText resolution: when `BroadcastTextId` is set, the recipient's locale-specific string is used; the gender variant is selected by the *speaker's* gender.
- [ ] Translation gating: a player without the required language skill receives the untranslated (gibberish) variant; with the skill, the translated one.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 5 files / 955 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h` | `crates/wow-chat/` for the senders/builders, `crates/wow-database/` for the loader, `crates/wow-world/` for the per-creature repeat-tracking glue \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#TEXTS.DIV.001` | `crates/wow-chat` (`exists_empty`, 0 Rust lines) | 5 C++ files / 955 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgrImpl.h`, `/home/server/woltk-trinity-legacy/src/server/game/Texts/CreatureTextMgr.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **The `duration` return value drives boss talk pacing**: `BossAI::Talk(group)` schedules its next line based on it. Don't change the return type or hide the field — many scripts implicitly depend on it.
- **`CreatureTextMgr` drives ALL boss talks, NPC yells, scripted whispers**. Any "talk" in TrinityCore code goes through `Creature::Talk(group)` which delegates here. There is essentially no other path. This is the single most-trafficked text system in the server.
- **`text_repeat` lives on the `Creature` (per-instance), not on the `CreatureTextMgr` (per-entry)**. Two instances of the same boss in two different copies of the same instance do *not* share repeat memory. Mirror that — put it on `WorldCreature`, not on the mgr.
- **`CHAT_MSG_ADDON` + `LANG_ADDON` are sentinels for "use the value from the DB row"**. `SendChat`'s default args mean "let the DB decide". Don't replace them with `Option::None` semantics — many callers pass `CHAT_MSG_ADDON` deliberately.
- **`CreatureTextLocalizer` caches one `ChatPacketSender` per `LocaleConstant`** (currently 12+1 locales) per send call. Important when broadcasting to hundreds of players in TEXT_RANGE_WORLD — without the cache, every recipient re-builds the packet.
- **`CHAT_MSG_MONSTER_WHISPER` and `CHAT_MSG_RAID_BOSS_WHISPER` short-circuit the range** when `range == TEXT_RANGE_NORMAL` — they go to the whisper target only. The `range != NORMAL` path falls through to the standard dispatcher and ignores `whisperTarget`. Easy to get backwards.
- **`creature_text_locale.Locale = enUS` rows are skipped** at load time (the base text in `creature_text` is already enUS). Replicate or you'll have duplicate-string ambiguity.
- **WoLK 3.4.3 specific**: `BroadcastText.db2` has a `SoundKitID[2]` column (male/female). The C++ uses `[GetGender() == GENDER_FEMALE ? 1 : 0]`. Female-voiced bosses are real; don't lose this.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CreatureTextMgr` (singleton) | `pub struct CreatureTextMgr` + `Arc<CreatureTextMgr>` injected | Avoid global; pass through registries |
| `struct CreatureTextEntry` | `#[derive(Clone)] pub struct CreatureTextEntry` | One per row |
| `unordered_map<u32, unordered_map<u8, vector<…>>>` | `HashMap<u32, HashMap<u8, Vec<CreatureTextEntry>>>` | Triple-nested OK |
| `template<class Builder> SendChatPacket(...)` | generic `fn send_chat_packet<B: ChatBuilder>(...)` | Trait `ChatBuilder { fn build(&self, locale) -> ChatPacketSender; }` |
| `Trinity::ChatPacketSender` | `pub struct ChatPacketSender { untranslated: Vec<u8>, translated: OnceCell<Vec<u8>>, … }` | Lazy translation via `OnceCell` |
| `Cell::VisitWorldObjects(source, worker, dist)` | iterate the 3×3 grid window in `MapManager` filtering by distance | `MapManager` already exposes a similar primitive |
| `Group::BroadcastWorker(localizer)` | `GroupRegistry::for_each_member(group_guid, |player_guid| { … })` | Existing primitive in `wow-network` |
| `World::GetAllSessions()` | `PlayerRegistry::iter()` | Existing |
| `Creature::GetTextRepeatGroup(group)` / `SetTextRepeatId` / `ClearTextRepeatGroup` | three methods on `WorldCreature` | Lives in `crates/wow-world/src/map_manager.rs` |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Loader / mgr presence.** `find … grep "CreatureText\|SendChat\|ChatTextBuilder\|text_repeat"` across all `crates/*/src/` returns **only** two hits, both in `crates/wow-database/src/statements/`:
- `world.rs:14` — enum variant `SEL_CREATURE_TEXT,`
- `mod.rs` — re-export of the same.

Confirmed: **no `creature_text.rs`, no `text_mgr.rs`, no `chat_text_builder.rs`, no `WorldCreature::text_repeat` field, no `Talk(group)` method**. The prepared-statement variant `SEL_CREATURE_TEXT` is reserved but has no consumer (no callsite of `db.execute(SEL_CREATURE_TEXT, …)` anywhere). Section 8's "the prepared statement string. That's it." is precisely correct.

**`SendChat`-returns-`duration` impl gap.** Confirmed unimplemented because `SendChat` itself does not exist. The C++ `CreatureTextMgr::SendChat` (`CreatureTextMgr.cpp` ~line 200, signature returns `uint32`) is the source of the `duration` value `BossAI::Talk(group)` reads to schedule the next line — without a Rust analogue, no boss script can self-pace. Any current Rust scripted talks must use ad-hoc `SMSG_CHAT` constructions (none found in handlers grep either; scripts currently silent).

**Range dispatcher / locale cache / repeat-avoidance / 4 builders / `SendSound` / `SendEmote`**: all absent.

**Verdict:** ❌ not started, confirmed. Doc badge correct as-is. Recommend escalating priority because every L6+ boss/quest script is gated on this; section 9 #TXT.3 (returns `duration`) is the unblocker — the rest of the L6 AI work cannot ship a faithful boss-line cadence without it.
