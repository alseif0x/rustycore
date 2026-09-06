# Migration: Tools (CharacterDatabaseCleaner + PlayerDump)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Tools/`
> **Rust target crate(s):** likely `crates/wow-database/` (cleaner) and a future `crates/wow-tools/` or CLI subcommand of `world-server` (player dump)
> **Layer:** L1 (infrastructure — DB-only utilities, run at startup or on operator command)
> **Status:** ❌ not started
> **Audited vs C++:** ⚠️ partial (CharacterDatabaseCleaner fully read; PlayerDump ~36 KB read by interface only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Two unrelated DB utilities glued into the same source folder by Trinity:

- **CharacterDatabaseCleaner** — at world startup, optionally scans the `character.*` tables and deletes rows whose IDs no longer exist in DBC/world data (orphan achievements, removed talents, deprecated skills, retired spells). Driven by a flag-mask persisted in `world_variables` so each kind of cleanup runs only when the operator opts in.
- **PlayerDump** — character export/import used by the `.character dump` and `.character restore` GM commands. Serializes a single character's rows from `characters` + `mail` + `mail_items` + `item_instance*` + `character_pet`/`pet_*` into a portable text dump and re-creates them under a new account/GUID. Critical for cross-realm transfers, GM rollbacks, and bug-report attachments.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Tools/CharacterDatabaseCleaner.cpp` | 155 | `prefix` |
| `game/Tools/CharacterDatabaseCleaner.h` | 48 | `prefix` |
| `game/Tools/PlayerDump.cpp` | 1071 | `prefix` |
| `game/Tools/PlayerDump.h` | 120 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Tools/CharacterDatabaseCleaner.h` | 49 | Namespace + `CleaningFlags` enum + entry points |
| `src/server/game/Tools/CharacterDatabaseCleaner.cpp` | 156 | All 5 cleanup routines + `CheckUnique` helper |
| `src/server/game/Tools/PlayerDump.h` | 121 | `DumpTableType` enum, `PlayerDumpWriter` / `PlayerDumpReader` classes |
| `src/server/game/Tools/PlayerDump.cpp` | 1247 | All serialization, table walking, GUID remapping, item-instance fan-out |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CharacterDatabaseCleaner` | namespace | Stateless cleanup functions |
| `CleaningFlags` | enum | bitmask: `ACHIEVEMENT_PROGRESS=0x1`, `SKILLS=0x2`, `SPELLS=0x4`, `TALENTS=0x8`, `QUESTSTATUS=0x10` |
| `DumpTableType` | enum | Which table family a dump entry corresponds to (CHARACTER, CHAR_TABLE, CURRENCY, EQSET_TABLE, INVENTORY, CHAR_TRANSMOG, MAIL, MAIL_ITEM, ITEM, ITEM_GIFT, ITEM_TABLE, PET, PET_TABLE) |
| `DumpReturn` | enum | Result codes: `DUMP_SUCCESS`, `DUMP_FILE_OPEN_ERROR`, `DUMP_TOO_MANY_CHARS`, `DUMP_FILE_BROKEN`, `DUMP_CHARACTER_DELETED` |
| `PlayerDump` | class | Static base — `InitializeTables()` + `InitializeColumnDefinition()` |
| `PlayerDumpWriter` | class | `GetDump(guid, &out)`, `WriteDumpToFile`, `WriteDumpToString` |
| `PlayerDumpReader` | class | `LoadDumpFromFile`, `LoadDumpFromString`, internal `LoadDump(istream, account, name, guid)` |
| `DumpTable` / `TableStruct` / `StringTransaction` | internal structs (`PlayerDump.cpp`) | Schema metadata + accumulating SQL string for atomic apply |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CharacterDatabaseCleaner::CleanDatabase()` | Read flag bitmask from `world_variables` (`CharacterDatabaseCleaningFlagsVarId`); dispatch into per-flag cleaners; rewrite the persisted flag set | `World::GetPersistentWorldVariable`, `SetPersistentWorldVariable`, `SetCleaningFlags` |
| `CharacterDatabaseCleaner::CheckUnique(column, table, check_fn)` | Run `SELECT DISTINCT col FROM tbl`; for every id where `check(id)` is false, build one `DELETE FROM tbl WHERE col IN (a,b,c…)` and execute | `CharacterDatabase.PQuery / Execute` |
| `CleanCharacterAchievementProgress` | `CheckUnique("criteria", "character_achievement_progress", AchievementProgressCheck)` — `sCriteriaMgr->GetCriteria(id) != nullptr` | — |
| `CleanCharacterSkills` | `("skill", "character_skills", SkillCheck)` — `sSkillLineStore.LookupEntry(id)` | — |
| `CleanCharacterSpell` | `("spell", "character_spell", SpellCheck)` — spell exists AND is not `SPELL_ATTR0_CU_IS_TALENT` | `sSpellMgr->GetSpellInfo` |
| `CleanCharacterTalent` | First `DELETE FROM character_talent WHERE talentGroup > MAX_SPECIALIZATIONS`, then `CheckUnique("talentId", …)` validating the spec is still in `sChrSpecializationStore` | `sTalentStore.LookupEntry` |
| `CleanCharacterQuestStatus` | One direct `DELETE FROM character_queststatus WHERE status = 0` | — |
| `PlayerDumpWriter::WriteDumpToString(&out, guid)` | Walk `DumpTable` registry; for each enabled `DumpTableType`, run the SELECT, reformat into `INSERT` lines, accumulate into `out`; populate sub-collections (`_pets`, `_mails`, `_items`, `_itemSets`) on the fly | DB queries against `characters` |
| `PlayerDumpWriter::WriteDumpToFile(file, guid)` | Wrap `WriteDumpToString` and write to disk | — |
| `PlayerDumpReader::LoadDumpFromString(dump, account, name, guid)` | Parse the dump line-by-line, remap GUIDs, build a `StringTransaction`, atomically apply via `CharacterDatabase` | — |
| `PlayerDump::InitializeTables() / InitializeColumnDefinition()` | One-time registration of which tables/columns participate in dumps + how to remap their PKs | — |

---

## 5. Module dependencies

**CharacterDatabaseCleaner depends on:**
- `Database` — both query and prepared-style execute.
- `DataStores/DB2Stores` — `sSkillLineStore`, `sTalentStore`, `sChrSpecializationStore`.
- `Spells/SpellMgr` + `SpellInfo` — talent attribute check.
- `Achievements/CriteriaHandler` — `sCriteriaMgr->GetCriteria`.
- `Server/World` — flag persistence config + `SetCleaningFlags`.

**PlayerDump depends on:**
- `Database` — heavy use; runs dozens of SELECTs and accumulates a single multi-statement transaction.
- `Entities/Object/ObjectGuid` — for low-type GUIDs.
- `Mails/Mail` — schema knowledge for mail/mail_items.
- `Entities/Item` — schema for item_instance and the artifact/azerite extension tables.
- `Entities/Pet` — pet table set.

**Depended on by:**
- `Server/Worldserver/WorldRunnable` (or main loop) — calls `CleanDatabase()` once at startup.
- `Chat/Commands/cs_character.cpp` — `.character dump load/write` commands wrap `PlayerDumpReader`/`Writer`.

---

## 6. SQL / DB queries (if any)

CharacterDatabaseCleaner emits ad-hoc dynamic SQL only:

| Statement / Source | Purpose | DB |
|---|---|---|
| `CheckUnique` builder | `SELECT DISTINCT {col} FROM {tbl}` then `DELETE FROM {tbl} WHERE {col} IN (...)` | character |
| `CleanCharacterTalent` direct | `DELETE FROM character_talent WHERE talentGroup > MAX_SPECIALIZATIONS` | character |
| `CleanCharacterQuestStatus` direct | `DELETE FROM character_queststatus WHERE status = 0` | character |

PlayerDump emits a wide and undocumented set of selects/inserts driven by its `DumpTable` registry — see header comment for the table groups (`DTT_CHARACTER`, `DTT_CHAR_TABLE`, `DTT_CURRENCY`, `DTT_EQSET_TABLE`, `DTT_INVENTORY`, `DTT_CHAR_TRANSMOG`, `DTT_MAIL`, `DTT_MAIL_ITEM`, `DTT_ITEM`, `DTT_ITEM_GIFT`, `DTT_ITEM_TABLE`, `DTT_PET`, `DTT_PET_TABLE`). All character DB.

No DBC/DB2 stores are *consumed* by these tools beyond the validity probes already listed in the cleaner.

---

## 7. Wire-protocol packets (if any)

None. Both tools are local — they don't speak to clients. Output paths are: log lines, world-state flag persistence (cleaner), and stdout / files (dump). Errors propagate via `DumpReturn` / log messages, not via packets.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-tools` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None for either tool. No `database_cleaner.rs`, no `player_dump.rs`, no GM-command equivalent.

**What's implemented:** nothing.

**What's missing vs C++:** everything. There's also no GM-command framework yet to attach `.character dump` to, so the consumer side of `PlayerDump` is double-blocked.

**Suspicious / likely divergent:** N/A.

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#TOOLS.WBS.001** Cerrar la migracion auditada de `game/Tools/CharacterDatabaseCleaner.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`
  Rust target: `crates/wow-database`, `crates/wow-tools`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TOOLS.WBS.002** Cerrar la migracion auditada de `game/Tools/CharacterDatabaseCleaner.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.h`
  Rust target: `crates/wow-database`, `crates/wow-tools`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TOOLS.WBS.003** Partir y cerrar la migracion auditada de `game/Tools/PlayerDump.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`
  Rust target: `crates/wow-database`, `crates/wow-tools`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1071 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#TOOLS.WBS.004** Cerrar la migracion auditada de `game/Tools/PlayerDump.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h`
  Rust target: `crates/wow-database`, `crates/wow-tools`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#TLS.1** Implement `CharacterDatabaseCleaner` as `pub fn run(pool: &Pool, flags: u32) -> Result<u32 /* remaining flags */, sqlx::Error>` in `wow-database/src/cleaner.rs`. Five sub-routines mirroring C++. Use parameterized `IN` lists rather than string-concat (sqlx supports `Vec<u32>` binds against MariaDB). (complexity: **M**)
- [ ] **#TLS.2** Wire flag persistence: read `CharacterDatabaseCleaningFlagsVarId` from `world_variables`, write back the masked-down value after the run. Plumb through whatever world-config struct ends up holding world-variables. (complexity: **L**)
- [ ] **#TLS.3** Hook `cleaner::run` into the world startup sequence behind `CONFIG_CLEAN_CHARACTER_DB`. Cleaning happens **before** sessions are accepted. (complexity: **L**)
- [ ] **#TLS.4** Decide PlayerDump scope (full parity is XL): minimum useful is an export-only writer for the canonical character set (`characters`, `character_inventory`, `character_spell`, `character_skills`, `character_queststatus`, `character_action`, `character_aura`, `character_homebind`, `character_reputation`, `item_instance` for inventory items, `mail`, `mail_items`). Skip artifact/azerite/transmog tables (Battle for Azeroth-and-later schemas not used by 3.4.3). (complexity: **L** — design only)
- [ ] **#TLS.5** Implement `PlayerDumpWriter` per #TLS.4 scope: walk a registry of `(table, key_columns, owner_column)`, collect referenced item GUIDs / mail IDs / pet IDs, run the SELECTs, and emit a Trinity-compatible textual dump. (complexity: **H**)
- [ ] **#TLS.6** Implement `PlayerDumpReader`: parse the dump format, remap `guid`/`itemGuid`/`mailId` to fresh values via `ObjectMgr` GUID generators, build one transaction, apply atomically. (complexity: **XL** — splittable per table group; biggest risk is GUID re-keying correctness) → split into separate ticket once #TLS.5 lands.
- [ ] **#TLS.7** Expose both as `world-server` CLI subcommands (`world-server dump --guid …`, `world-server restore --file … --account … --name …`) until a GM-command layer exists. (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#TOOLS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 1394 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h`. Rust target: `crates/world-server`, `crates/wow-database`. | `cargo test -p world-server && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#TOOLS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 1394 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h`. Rust target: `crates/world-server`, `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#TOOLS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 1394 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h`. Rust target: `crates/world-server`, `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#TOOLS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 1394 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h`. Rust target: `crates/world-server`, `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

CharacterDatabaseCleaner:
- [ ] Seed `character_skills` with a mix of valid + invalid skill IDs; run cleaner with `CLEANING_FLAG_SKILLS`; assert only invalid rows are gone.
- [ ] `CleanCharacterTalent` removes any row with `talentGroup > MAX_SPECIALIZATIONS` *first*, before the IN-list deletion.
- [ ] `CleanCharacterQuestStatus` deletes exactly the rows with `status = 0`.
- [ ] Persisted flags are AND-masked by `CONFIG_PERSISTENT_CHARACTER_CLEAN_FLAGS` after the run (so non-persistent flags unset themselves automatically).
- [ ] Disabled config (`CONFIG_CLEAN_CHARACTER_DB = false`) skips the entire run even with non-zero flags.

PlayerDumpWriter:
- [ ] Round-trip: write dump for character `G`, restore on a new account/name/guid `G'`, then write `G'` — the two dumps must be identical except for the remapped GUIDs.
- [ ] Mail attachments: items inside a mail are exported and re-import as fresh `item_instance` rows attached to the new mail row.
- [ ] Empty-character edge case: zero inventory, zero mail, zero pets — dump still produces a valid restorable file.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 1394 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h` | likely `crates/wow-database/` (cleaner) and a future `crates/wow-tools/` or CLI subcommand of `world-server` (player dump) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#TOOLS.DIV.001` | `crates/wow-tools` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 1394 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/CharacterDatabaseCleaner.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Tools/PlayerDump.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Cleaner's `IN`-list builder uses raw string concatenation**: `ss << "DELETE FROM " << table << " WHERE " << column << " IN ("` — safe in C++ only because `column` and `table` are compile-time constants and `id` is a `uint32`. **In Rust, use bound parameters** (`sqlx`'s `bind` works for `Vec<u32>` against MariaDB). Don't replicate the C++ pattern.
- **`SpellCheck` excludes talents**: a spell is considered valid for `character_spell` only if it exists AND lacks `SPELL_ATTR0_CU_IS_TALENT`. Talents live in `character_talent`, not `character_spell`. Don't merge the two.
- **`CleanCharacterTalent`'s direct delete runs unconditionally** (even if subsequent `CheckUnique` finds no orphans). The two phases are independent — the direct delete is for the multi-spec-removal path on character upgrade.
- **`PlayerDump`'s `DumpTableType` enum lists tables that don't exist in 3.4.3**: artifact/azerite/transmog tables are MoP+/BfA-era. Pretend they don't exist when porting; they would fail SELECTs against the WoLK schema anyway.
- **GUID remapping** in `PlayerDumpReader` is the hardest part to port: the textual dump contains the *source* realm's GUIDs; the reader must allocate fresh GUIDs from `ObjectMgr`'s generator and substitute consistently across `characters`, `character_inventory`, `item_instance`, `mail`, `mail_items`. Item GUID collisions are silent corruption — assert generator monotonicity.
- **`StringTransaction`** in `PlayerDump.cpp` is just a string accumulator that runs as a single multi-statement query at the end — not a real DB transaction. Rust port should use a real `BEGIN`/`COMMIT` instead.
- **`character_ticket`** is mentioned in the `DTT_CHAR_TABLE` comment and is in scope for dumps. Coordinate with `support.md` on schema.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `namespace CharacterDatabaseCleaner` | `mod cleaner` (free fns) in `wow-database` | No state |
| `enum CleaningFlags` | `bitflags! struct CleaningFlags: u32 { … }` | — |
| `bool (*check)(uint32)` callback | `Fn(u32) -> bool` closure / `fn` pointer | The five concrete checks become module-private fns |
| `class PlayerDumpWriter` | `pub struct PlayerDumpWriter { items: HashSet<u64>, mails: HashSet<u32>, pets: HashSet<u32>, item_sets: HashSet<u64> }` | Same internal sets |
| `class PlayerDumpReader` | `pub struct PlayerDumpReader { /* GUID remap state */ }` | — |
| `enum DumpTableType` | `enum DumpTableKind { … }` | Drop the BfA-era variants |
| `enum DumpReturn` | `Result<(), DumpError>` with strongly-typed errors | More idiomatic than C-style return enum |
| `StringTransaction` | `sqlx::Transaction<'_, MySql>` | Real transaction, not string concat |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

`find /home/server/rustycore -name "*.rs" -path "*/src/*" | xargs grep -l "PlayerDump\|player_dump\|CharacterDatabaseCleaner\|database_cleaner\|CleaningFlags\|clean_character"` returns **zero matches**. Confirmed: neither `CharacterDatabaseCleaner` nor `PlayerDump` (Writer or Reader) has any analogue — no module, no function, no GM command stub.

Also confirmed:
- `wow-database/src/statements/world.rs` carries no cleaner-shaped DELETE statements (no `DEL_ORPHAN_*`, no `CLEAN_*`).
- `wow-database/src/statements/character.rs` is character-row CRUD only (player save/load), no bulk cleanup or dump-walk queries.
- World startup sequence in `wow-world` does **not** invoke any cleanup pass before accepting sessions; section 9 #TLS.3 ("Hook `cleaner::run` into the world startup sequence") still wholly applicable.
- No `world-server` CLI subcommand for dump/restore (the binary's argv parsing in `bins/world-server/src/main.rs` only takes config-file paths).
- No GM-command framework exists yet to host `.character dump load/write` (consumer side double-blocked, as section 8 notes).

**Wrath-only schema caveat from section 11 confirmed**: nothing in `wow-database/src/statements/character.rs` mentions `character_artifact*`, `character_azerite*`, `character_transmog*` tables — so when #TLS.5 ports `PlayerDumpWriter`, dropping those `DumpTableType` variants is safe and matches the existing schema scope.

**Verdict:** ❌ not started, confirmed exactly. No course correction to the doc needed; section 8 ("nothing… everything is missing") is accurate verbatim.
