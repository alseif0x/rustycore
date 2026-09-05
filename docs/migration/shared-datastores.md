# Migration: Shared DataStores (DB2/DBC binary readers)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/` (+ the engine-side primitives in `/home/server/woltk-trinity-legacy/src/common/DataStores/`)
> **Rust target crate(s):** `crates/wow-data/`
> **Layer:** L1 (infrastructure — binary file format readers + DB-driven hotfix overlay)
> **Status:** ⚠️ partial — confirmed via audit 2026-05-01 (raw WDC4 reader works; Storage<T>/hotfix overlay missing; only 5 .db2 files parsed of ~325)
> **Audited vs C++:** ⚠️ audited 2026-05-01 — gaps confirmed; one count corrected (5 files, not 8)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Binary readers and in-memory containers for the WoW client's DB2/DBC data files (`Achievement.db2`, `Item.db2`, `Map.db2`, …). The `shared/DataStores` folder hosts the **server-side template wrapper** (`DB2Storage<T>`) that turns a parsed file into a strongly-typed table indexed by record ID. It also hosts the **hotfix database loader** (`DB2DatabaseLoader`) which, after the `.db2` file is read from disk, queries the `hotfixes` MariaDB schema to overlay row-level patches and per-locale string overrides on top of the file-loaded data. The lower-level primitives (`DB2FileLoader`, `DB2FileSystemSource`, `DB2Meta`) live in `common/DataStores` and parse the raw WDC4 binary; this doc covers both because the server `Storage<T>` is unusable without them.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/DataStores/DB2DatabaseLoader.cpp` | 287 | `prefix` |
| `shared/DataStores/DB2DatabaseLoader.h` | 49 | `prefix` |
| `shared/DataStores/DB2Store.cpp` | 145 | `prefix` |
| `shared/DataStores/DB2Store.h` | 90 | `prefix` |
| `shared/DataStores/DBStorageIterator.h` | 74 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/shared/DataStores/DB2Store.h` | 90 | `DB2StorageBase` (untyped container) + `DB2Storage<T>` template (typed lookup) |
| `src/server/shared/DataStores/DB2Store.cpp` | 145 | `Load()` from disk + `LoadFromDB()` overlay + `WriteRecord()` for SMSG_DB_REPLY |
| `src/server/shared/DataStores/DB2DatabaseLoader.h` | 49 | `DB2LoadInfo` (extends `DB2FileLoadInfo` with `HotfixDatabaseStatements` enum) + `DB2DatabaseLoader` |
| `src/server/shared/DataStores/DB2DatabaseLoader.cpp` | 287 | Hotfix-DB SELECT loop, per-field type-switch into the in-memory record blob, locale string overlay |
| `src/server/shared/DataStores/DBStorageIterator.h` | 74 | Forward iterator that skips holes (`nullptr` slots) in the index table |
| `src/common/DataStores/DB2FileLoader.h` | 222 | `DB2Header`/`DB2SectionHeader` POD structs; `DB2FileLoader` class; `DB2Record` field accessor; `DB2FieldMeta` |
| `src/common/DataStores/DB2FileLoader.cpp` | 2138 | The actual WDC4 parser — section walk, bitpacked/pallet/common decoding, `AutoProduceData/Strings/RecordCopies` |
| `src/common/DataStores/DB2FileSystemSource.h` | 45 | Stdio-backed `DB2FileSource` impl |
| `src/common/DataStores/DB2FileSystemSource.cpp` | 68 | Same |
| `src/common/DataStores/DB2Meta.h` | 63 | `DB2Meta` struct (per-table layout: index field, parent index, file vs in-memory field counts, layout hash) |
| `src/common/DataStores/DB2Meta.cpp` | 214 | `GetRecordSize`, `GetIndexFieldOffset`, `IsSignedField`, etc. — derives offsets from the field type+arraysize array |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `DB2StorageBase` | class | Type-erased base: holds `_indexTable` (sparse `char**` indexed by record-id), `_dataTable`, `_stringPool`, `_tableHash`, `_layoutHash`, `_minId` |
| `DB2Storage<T>` | template class | Public typed front-end; `LookupEntry(id) -> T const*`, `iterator begin()/end()`, requires `T` standard-layout |
| `DBStorageIterator<T>` | template class | Forward iterator that skips `nullptr` slots — record IDs are *not* contiguous |
| `DB2FileLoadInfo` | struct | Field-meta array + `DB2Meta*` pointer; binds C++ struct → file format |
| `DB2LoadInfo` | struct (server-only) | `DB2FileLoadInfo` + `HotfixDatabaseStatements Statement` enum (the prepared-statement id used to fetch overlay rows) |
| `DB2FileLoader` | class | The WDC4 parser. `Load(source, loadInfo)` then `AutoProduceData/Strings/RecordCopies` |
| `DB2FileLoaderImpl` | pimpl | Hides the actual section/bitpack decoder |
| `DB2Header` | POD | The 72-byte WDC4 header (signature, record_count, field_count, record_size, string_table_size, table_hash, layout_hash, min_id, max_id, locale, flags, index_field, total_field_count, packed_data_offset, parent_lookup_count, column_meta_size, common_data_size, pallet_data_size, section_count) |
| `DB2SectionHeader` | POD (40 bytes) | Per-section header — `tact_id`, `file_offset`, `record_count`, `string_table_size`, `catalog_data_offset`, `id_table_size`, `parent_lookup_data_size`, `catalog_data_count`, `copy_table_count` |
| `DB2RecordCopy` | POD (8 bytes) | `{NewRowId, SourceRowId}` — a copy-table entry that aliases an existing record under a new ID |
| `DB2Record` | class | Cursor over one parsed record; type-checked field getters (`GetUInt32(field, arrayIndex)`, `GetString(fieldName)`, …) |
| `DB2FieldMeta` | struct | `{IsSigned, DBCFormer Type, char const* Name}` — one per *field-as-defined-in-struct* |
| `DB2MetaField` | struct | `{DBCFormer Type, uint8 ArraySize, bool IsSigned}` — one per *field-as-defined-in-file* (no name; arrays collapsed) |
| `DB2Meta` | struct | Per-table layout: `FileDataId`, `IndexField`, `ParentIndexField`, `FieldCount`, `FileFieldCount`, `LayoutHash`, `Fields[]` |
| `DB2EncryptedSectionHandling` | enum | `Skip` / `Process` — what to do with TACT-encrypted sections (the loader's caller decides) |
| `DBCFormer` | enum (in `Define.h`) | Field type tags: `FT_INT` (4 B), `FT_FLOAT` (4 B), `FT_BYTE` (1 B), `FT_SHORT` (2 B), `FT_LONG` (8 B), `FT_STRING` (`LocalizedString` = 1 ptr per locale), `FT_STRING_NOT_LOCALIZED` (one `char const*`) |
| `DB2FileSource` | abstract class | Read interface — `Read`, `GetPosition`, `SetPosition`, `GetFileSize`, `HandleEncryptedSection` |
| `DB2FileSystemSource` | class | Stdio implementation of `DB2FileSource`; only one in shipping code |
| `DB2FileLoadException` | class | Throws on malformed file |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `DB2StorageBase::Load(path, locale)` | (1) opens `<path><fileName>` via `DB2FileSystemSource`; (2) `DB2FileLoader::Load`; (3) `AutoProduceData` allocates flat record table + sparse index; (4) `AutoProduceStrings` copies localized strings into pool; (5) `AutoProduceRecordCopies` materializes the copy-table aliases | `DB2FileLoader::*`, `_stringPool` |
| `DB2StorageBase::LoadStringsFrom(path, locale)` | After main `Load`, layer additional locales by re-reading only the string-table portion of the same `.db2` from a sibling locale dir. Adds new `char*` to `_stringPool` and patches the `LocalizedString::Str[locale]` slots | `DB2FileLoader::AutoProduceStrings` |
| `DB2StorageBase::LoadFromDB()` | Reads from `hotfixes` MariaDB. Uses `_loadInfo->Statement` (e.g. `HOTFIX_SEL_ACHIEVEMENT`) and a follow-on `_MAX_ID` query to grow `_indexTableSize` if a hotfix introduces an ID past the file's `max_id`. Then loops rows, switches on each field's `FT_*`, writes into a fresh data block, and patches `_indexTable[id]` to point at the new row. **Two passes** — `custom=false` (regular hotfixes, table column `Verified=1`) then `custom=true` (private/dev) | `HotfixDatabase.GetPreparedStatement`, `DB2DatabaseLoader::AddString` |
| `DB2StorageBase::LoadStringsFromDB(locale)` | Same shape as `LoadFromDB` but only walks `FT_STRING` fields and writes into `LocalizedString::Str[locale]` | `HotfixDatabase`, `AddString` |
| `DB2StorageBase::WriteRecord(id, locale, ByteBuffer&)` | Serialize one record into a `ByteBuffer` for `SMSG_DB_REPLY` — used when the client's local DB2 cache is missing this entry. Walks the meta field-by-field, dereferences each (with array unrolling), writes raw little-endian | `LocalizedString::operator[]` |
| `DB2StorageBase::EraseRecord(id)` | `_indexTable[id] = nullptr`. Called by `DB2Manager` when a hotfix row has `Status::RecordRemoved` | — |
| `DB2Storage<T>::LookupEntry(id)` | `id >= _indexTableSize ? nullptr : reinterpret_cast<T const*>(_indexTable[id])` — O(1) sparse-array hit | — |
| `DB2Storage<T>::AssertEntry(id)` | `LookupEntry` + `ASSERT_NOTNULL` | — |
| `DB2FileLoader::Load(source, loadInfo)` | Drives the WDC4 parser. Reads main header → section headers → field meta → field-storage-info → pallet data → common data → per-section (records + string table + id list + relationship data + offset map + copy table). Stores everything in `_impl` for later access | — |
| `DB2FileLoader::AutoProduceData(indexTableSize, indexTable)` | Allocates a flat `char*` blob for all records concat, allocates the sparse `_indexTable` (size = `max(max_id + 1, …)`), and points each non-empty slot at its blob offset | — |
| `DB2FileLoader::AutoProduceStrings(indexTable, size, locale)` | For files with `FT_STRING`/`FT_STRING_NOT_LOCALIZED`, allocates a fresh string pool, copies bytes from the file's string table, and patches the `char const**` slots in each record blob | — |
| `DB2FileLoader::AutoProduceRecordCopies(records, indexTable, dataTable)` | For each `DB2RecordCopy{NewRowId, SourceRowId}`, point `_indexTable[NewRowId]` at the same blob as `SourceRowId`. The new row may differ only in fields explicitly overlaid by hotfixes |
| `DB2DatabaseLoader::Load(custom, records, indexTable, stringPool, minId)` | Workhorse for `LoadFromDB`. See above. Returns the `char*` data block for caller to track | `HotfixDatabase`, `Field::Get*` |
| `DB2DatabaseLoader::LoadStrings(custom, locale, …)` | Locale-only re-overlay; only walks `FT_STRING` fields | `HotfixDatabase` |
| `DB2DatabaseLoader::AddString(holder, value)` | Allocates a new `char[len+1]`, copies, stores pointer in `*holder`, returns the pointer for `stringPool` tracking | — |
| `DB2Meta::HasIndexFieldInData()` | Whether the index field is part of the record body or prepended by the loader (some DB2s store IDs out-of-line in the id_list) | — |
| `DB2Meta::GetRecordSize()` | Sum of (per-field bytes × array size) across all fields, plus 4 if `!HasIndexFieldInData()` | — |
| `DB2Meta::GetIndexFieldOffset()` | Byte offset of the ID field within the in-memory record | — |
| `DB2Meta::IsSignedField(idx)` | Whether to sign-extend bitpacked reads | — |

---

## 5. Module dependencies

**Depends on:**
- `common/DataStores/DB2FileLoader` — the actual binary parser; everything above is just a wrapper.
- `common/DataStores/DB2Meta` — provides `Fields[]` arrays for each table; auto-generated in `DB2Metadata.h` (a sibling header in `game/DataStores/`).
- `Database/HotfixDatabase` — `LoadFromDB` + `LoadStringsFromDB` issue prepared queries.
- `Database/HotfixDatabaseStatements` enum — the per-table statement IDs (`HOTFIX_SEL_*`, `HOTFIX_SEL_*_LOCALE`, `HOTFIX_*_MAX_ID`).
- `Common`/`Errors` — `LocaleConstant`, `ASSERT`, `ABORT_MSG`.
- `ByteBuffer` — for `WriteRecord` (consumer of SMSG_DB_REPLY).
- `Logging/Log` — `TC_LOG_ERROR("sql.sql", …)`.

**Depended on by:**
- `game/DataStores/DB2Stores` — declares ~261 `DB2Storage<…Entry>` globals (`sAchievementStore`, `sItemStore`, `sMapStore`, …) and orchestrates load order, hotfix overlay, and post-load index-building.
- `game/DataStores/DB2HotfixGenerator` — server-side runtime patches (e.g. fixing a known-broken record) that bypass the DB and force `DB2HotfixGeneratorBase::AddClientHotfix` to push the patched blob to clients.
- `Server/WorldSession::HandleDbQueryBulk` — calls `DB2Storage<T>::WriteRecord` to answer client `CMSG_DB_QUERY_BULK`.

---

## 6. SQL / DB queries (if any)

Only **DB2 stores** here; no per-module domain SQL. The DB-overlay layer issues queries via the `HotfixDatabase` prepared-statement registry. For each store (~261 of them), three statements:

| Statement / Source | Purpose | DB |
|---|---|---|
| `HOTFIX_SEL_<TABLE>` | Fetch all overlay rows for the main (`Verified=1`) hotfix data | hotfixes |
| `HOTFIX_<TABLE>_MAX_ID` | `SELECT MAX(ID) FROM <table>` — used to grow the index table beyond the file's max_id when a hotfix introduces a new row | hotfixes |
| `HOTFIX_SEL_<TABLE>_LOCALE` | Fetch only the localized string columns for a given locale | hotfixes |

The total prepared-statement count is **~3 × 261 ≈ 783** entries in the `HotfixDatabaseStatements` enum (these are historical estimates, not a regenerated statement inventory; adding a DB2 affects multiple statements).

The `hotfixes` DB schema mirrors each DB2 table 1-to-1 with a SQL table of the same name, all fields as separate columns plus a synthetic `Verified` column. There is **also** a `hotfix_data` table (rows: `Id`, `UniqueId`, `TableHash`, `RecordId`, `Status` ∈ {NotSet/Valid/RecordRemoved/Invalid/NotPublic}), a `hotfix_blob` table (rows: `TableHash`, `RecordId`, `locale`, `Blob`) for tables not loaded server-side but still pushed to the client, and a `hotfix_optional_data` table for ancillary key-blob pairs (e.g. `BroadcastText` TACT keys). Those three are read by `DB2Manager::LoadHotfix*` (covered in the `datastores.md` doc).

---

## 7. Wire-protocol packets (if any)

This module does not directly handle opcodes, but its `WriteRecord` is the body-builder for:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_DB_REPLY` | server → client | `WorldSession::SendDbReply` (consumer of `DB2StorageBase::WriteRecord`) |

`CMSG_DB_QUERY_BULK` is the client request that triggers `WriteRecord`; `SMSG_HOTFIX_LIST` and `SMSG_HOTFIX_PUSH` notify the client of available hotfixes for its local DB2 cache (the manager-level concern — see `datastores.md`).

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-data/src/wdc4.rs` | `file` | 1 | 915 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | 111 | `exists_active` | file exists |
| `crates/wow-data/src/{item,item_stats,player_stats,skill,area_trigger,spell,quest,quest_xp}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/wdc4.rs` — 915 lines — the WDC4 parser. Covers the equivalent of `DB2FileLoader` + `DB2FileLoaderImpl` + `DB2Record` accessors. **Does not** cover the `DB2Storage<T>` typed wrapper or hotfix-DB overlay.
- `crates/wow-data/src/hotfix_cache.rs` — 111 lines — `HotfixBlobCache`: pre-loads raw record bytes from `.db2` files at startup so they can be served verbatim via `SMSG_DB_REPLY`. **Bypasses** the typed-storage layer entirely; just a `(table_hash, record_id) → Vec<u8>` map. Does not consult the `hotfixes` DB.
- `crates/wow-data/src/{item,item_stats,player_stats,skill,area_trigger,spell,quest,quest_xp}.rs` — 8 hand-rolled per-table readers (~2.4 KLoC) that each open one DB2 file with `Wdc4Reader`, hard-code field indices, and produce a `HashMap<u32, Record>`. **No** auto-generated meta — every column index is a manually-written constant. **No** hotfix-DB overlay; if you change a record in `hotfixes.item_sparse`, the Rust server doesn't notice.
- `crates/wow-database/src/statements/hotfix.rs` — 25 lines — placeholder enum with one `_PLACEHOLDER` variant. The 783-statement registry is **not** populated.

**What's implemented:**
- Raw WDC4 binary parsing — all six compression types (`None`, `Bitpacked`, `BitpackedSigned`, `Common`, `Pallet`, `PalletArray`), section walk, id-list, copy-table, offset-map for variable-length records (`ItemSparse`).
- 64-bit field reads (`get_field_i64`) for things like `RaceMask`.
- Array-element reads (`get_array_i16`, `get_array_i8`).
- A blob cache for `Item.db2` + `ItemSparse.db2` so the server can answer `CMSG_DB_QUERY_BULK` for those two.
- Eight ad-hoc per-table reader modules (Item, ItemSparse stats, PlayerLevelStats, Skill*, AreaTrigger, Spell, Quest, QuestXP).

**What's missing vs C++:**
1. **`DB2Storage<T>` template equivalent** — there's no generic typed wrapper. Each table is a bespoke module.
2. **Auto-generated meta** — no equivalent of `DB2Metadata.h` (788 structs) or `DB2LoadInfo.h` (325 LoadInfo structs). Field indices are hard-coded magic numbers in each per-table reader.
3. **`DB2Structure.h` equivalent** — no auto-generated full struct definitions for the ~325 tables. Only the 8 hand-picked tables exist.
4. **Hotfix-DB overlay** — `LoadFromDB` and `LoadStringsFromDB` have no analogue. The `HotfixStatements` enum in `wow-database` is a stub.
5. **`hotfix_data` / `hotfix_blob` / `hotfix_optional_data`** — none of the three control tables are read.
6. **`AutoProduceRecordCopies`** equivalent at the typed layer — `Wdc4Reader::iter_records` does emit copies via the `copy_table`, but consumers receive `(record_id, source_record_idx)` and have to handle aliasing themselves; the C++ side materializes it once into the `_indexTable` so consumers see a real `T const*`.
7. **`SMSG_DB_REPLY` writer** — the equivalent of `WriteRecord` is implicit because the blob cache stores raw file bytes, but it's locale-blind and won't reflect server-side hotfixes.
8. **Locale stacking** — no `LoadStringsFrom` analogue. A multilingual realm cannot serve non-default locales.
9. **`DB2HotfixGenerator`** — no runtime in-memory patching API (used by Trinity's `db2_hotfixes.cpp` to fix known-broken records on the fly).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The blob cache is keyed by *file* table_hash. If `hotfixes.item_sparse` changes a row, the cached blob is stale until the server restarts. C++ TC's `LoadFromDB` re-serializes after overlay so this does not happen there.
- The hand-rolled field index constants in each per-table reader will silently break the next time the WoW client patches that DB2 — there's no `LayoutHash` check at the `Wdc4Reader` level (it's exposed via `table_hash()` but no consumer asserts it).
- `Wdc4Reader::record_bytes` returns the raw file bytes including offset-map padding for variable-length records. For `SMSG_DB_REPLY` the client expects the *decoded* record format, not the bitpacked one — this currently happens to work for `Item.db2` (which is fixed-size, no compression) but will produce wrong wire output for any bitpacked DB2.

**Tests existing:** none of the modules in `wow-data` have unit tests at the `Wdc4Reader` level for compression-type round-trips. The 12 tests cited for `MapManager` (in AGENTS.md) are unrelated. Per-table modules likely have a couple of integration tests but nothing systematic.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_DATASTORES.WBS.001** Cerrar la migracion auditada de `shared/DataStores/DB2DatabaseLoader.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DATASTORES.WBS.002** Cerrar la migracion auditada de `shared/DataStores/DB2DatabaseLoader.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.h`
  Rust target: `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DATASTORES.WBS.003** Cerrar la migracion auditada de `shared/DataStores/DB2Store.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`
  Rust target: `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DATASTORES.WBS.004** Cerrar la migracion auditada de `shared/DataStores/DB2Store.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h`
  Rust target: `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DATASTORES.WBS.005** Cerrar la migracion auditada de `shared/DataStores/DBStorageIterator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DBStorageIterator.h`
  Rust target: `crates/wow-data`, `crates/wow-database`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#SDS.1** Audit `Wdc4Reader` against `DB2FileLoader::Load` for compression-type parity. Specifically: `Pallet` `val1/val2/val3` interpretation (which TC reads as bit-packed-offset / pallet-start / array-count) and `Common` default-value fallback. Add round-trip tests fed by the actual shipping `.db2` files for at least Item, ItemSparse, Map, Spell. (complexity: **M**)
- [ ] **#SDS.2** Add a `LayoutHash` assertion API on `Wdc4Reader` so per-table consumers can pin to an expected layout and fail loud when the client patches the format. (complexity: **L**)
- [ ] **#SDS.3** Define the Rust equivalent of `DB2Meta` / `DB2FieldMeta`: a `pub struct FieldMeta { ty: FieldType, array_size: u8, signed: bool, name: &'static str }` plus `pub struct TableMeta { file_data_id: u32, index_field: i32, parent_index_field: i32, field_count: u32, file_field_count: u32, layout_hash: u32, fields: &'static [FieldMeta] }`. (complexity: **L**)
- [ ] **#SDS.4** Build a code generator (Python or Rust `build.rs`) that consumes `DB2Metadata.h` from the C++ tree and emits `crates/wow-data/src/generated/meta.rs` with a `TableMeta` constant per table — same shape as TC's `Achievement_CategoryMeta::Instance` etc. ~325 tables. (complexity: **H**, splitter — first 50 tables manual to validate the generator, then loop the rest.)
- [ ] **#SDS.5** Build the equivalent code generator for `DB2Structure.h` → `crates/wow-data/src/generated/structs.rs`. Each struct becomes a `#[repr(C)]` Rust struct with `LocalizedString`/`Cstr` typedefs. (complexity: **XL** — 4538 lines of C++ structs; split per logical group: items / spells / chars / maps / quests / misc.)
- [ ] **#SDS.6** Implement `pub struct Storage<T: 'static>` parameterised by `TableMeta` and a deserializer trait `FromDb2<'a>` that reads a `Wdc4Reader` row + meta into `T`. Mirror `DB2Storage<T>::LookupEntry`, `iter`, `record_count`. Use `Vec<Option<Box<T>>>` indexed by `record_id` (sparse) — same as TC's `_indexTable`. (complexity: **M**)
- [ ] **#SDS.7** Materialize copy-table aliases at load time by cloning the source `T` into the new ID's slot — equivalent of `AutoProduceRecordCopies`. (complexity: **L**)
- [ ] **#SDS.8** Add `LocalizedString = [Option<&'static str>; LOCALE_COUNT]` (or `[Arc<str>; …]`) and a string-pool allocator. Re-implement `AutoProduceStrings` to copy into the pool and patch each record's string slots. (complexity: **M**)
- [ ] **#SDS.9** Implement `Storage<T>::load_strings_from(path, locale)` to overlay a sibling locale's `.db2` strings on top of an already-loaded store. (complexity: **M**)
- [ ] **#SDS.10** Populate `crates/wow-database/src/statements/hotfix.rs`: replace the `_PLACEHOLDER` enum with the full ~783-statement registry, generated alongside the meta. Each table needs `SEL_<TABLE>`, `<TABLE>_MAX_ID`, `SEL_<TABLE>_LOCALE`. (complexity: **H**)
- [ ] **#SDS.11** Implement `Storage<T>::load_from_db(pool)` — the equivalent of `LoadFromDB`. Two-pass (`custom=false` then `custom=true`). Walk the per-field `FieldMeta` and either grow the index-table size (if a new ID appears) or overwrite an existing slot. (complexity: **H**)
- [ ] **#SDS.12** Implement `Storage<T>::load_strings_from_db(pool, locale)`. Same loop but only `FieldType::String` fields. (complexity: **M**)
- [ ] **#SDS.13** Implement `Storage<T>::write_record(id, locale, &mut Vec<u8>)` mirroring TC's `WriteRecord` — used by `SMSG_DB_REPLY`. Walk the meta, write little-endian bytes per field, expand `LocalizedString` to the requested locale. (complexity: **M**)
- [ ] **#SDS.14** Replace the existing 8 hand-rolled per-table readers with calls to `Storage<T>` once the generator is operational; keep `Item`, `ItemSparse`, `Map`, `Spell` as the first wave. (complexity: **M** for the first four; **L** each for the rest.)
- [ ] **#SDS.15** Refit `HotfixBlobCache` to consult `Storage<T>::write_record` post-overlay rather than reading raw file bytes. (complexity: **L**)
- [ ] **#SDS.16** Document the hotfix-DB schema in `crates/wow-database/migrations/` so a fresh-install MariaDB has the 261+3 tables. (complexity: **M**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_DATASTORES.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 645 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | `cargo test -p wow-data && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_DATASTORES.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 645 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_DATASTORES.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 645 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_DATASTORES.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 645 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h`. Rust target: `crates/wow-data`, `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Round-trip: read `Item.db2`, dump every record's first-five field bytes, compare against a fixture produced by TC's `DB2FileLoader` on the same file → exact match.
- [ ] LayoutHash mismatch detection: corrupt the layout_hash in a fixture, expect `Wdc4Reader::open` to fail when a `TableMeta::LAYOUT_HASH` is asserted.
- [ ] Copy-table materialization: a record that exists only as a copy is reachable via `Storage::lookup` and its data is byte-identical to the source.
- [ ] Hotfix overlay: insert a row in `hotfixes.item_sparse` with `Verified=1`, run `load_from_db`, assert `lookup(id)` returns the patched value and *not* the file value.
- [ ] Hotfix grows index table: `hotfixes.item_sparse` row with ID > file's max_id; the `_MAX_ID` query result drives the resize; the new row is reachable.
- [ ] String-pool locale stacking: load `enUS` then `LoadStringsFrom("frFR")`; both locales return the right string for an `FT_STRING` field.
- [ ] `RecordRemoved` status erases an entry from the index after `LoadHotfixData` triggers `EraseRecord`.
- [ ] `SMSG_DB_REPLY` body bytes from `Storage::write_record(id, enUS)` are identical to TC's output for the same record (capture from a running TC binary as a reference fixture).
- [ ] Bitpacked field with `IsSigned=true` produces sign-extended values for negative inputs.
- [ ] Variable-length record (offset-map): `ItemSparse` records of differing string lengths read back the right per-record byte size.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 5 files / 645 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h` | `crates/wow-data/` \| ⚠️ partial — confirmed via audit 2026-05-01 (raw WDC4 reader works; Storage<T>/hotfix overlay missing; only 5 .db2 files parsed of ~325) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_DATASTORES.DIV.001` | `crates/wow-data/src/{item,item_stats,player_stats,skill,area_trigger,spell,quest,quest_xp}.rs` (`declared_pattern`, 0 Rust lines) | 5 C++ files / 645 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2DatabaseLoader.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/DataStores/DB2Store.h` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |

<!-- REFINE.023:END known-divergences -->

- **WDC4 vs WDC3**: Wrath of the Lich King Classic 3.4.3 ships **WDC4**, not WDC3 (despite the game version being WoLK). Magic is `'WDC4'` LE. The eight extra fields over WDC3 are: `TotalFieldCount`, `PackedDataOffset`, `LookupColumnCount` (renamed `ParentLookupCount`), `ColumnMetaSize` (renamed in headers as `field_storage_info_size`), `CommonDataSize`, `PalletDataSize`, `SectionCount` (multi-section files arrived in WDC4). Older docs referencing WDC3 *will* mislead.
- **Encrypted sections**: WDC4 supports per-section TACT encryption (`tact_id` field). The retail client gets keys via Battle.net; private servers ship `DUMMY_KNOWN_TACT_ID = 0x5452494E49545900` ("TRINITY\0") and `DB2EncryptedSectionHandling::Process` — meaning the section is passed through verbatim without decryption, which works because the test data on private realms is unencrypted by convention. Don't gate on real TACT decryption — it's not needed.
- **`HasIndexFieldInData()`**: Some DB2 files store the index/ID field inline with the rest of the record (offset 0 or wherever `IndexField` says); others store it in an out-of-band `id_list` and `record_size` excludes it. The 4-byte-prefix dance in `LoadFromDB`/`WriteRecord` (see `DB2DatabaseLoader.cpp:88-93`) is exactly to harmonize the in-memory layout: when the file *doesn't* store the index inline, the in-memory record gets the ID prepended so `T const*` dereferences are uniform across tables.
- **`min_id` may not be 0**: Sparse tables (e.g. some quest tables start at 100k) have `_indexTable` sized `[min_id..max_id]` plus the 0..min_id range as nullptr. Don't index naively — go through `LookupEntry`.
- **Two-store overlay order**: Files first (`Load(path, locale)`), then DB (`LoadFromDB`), then per-locale strings (`LoadStringsFromDB(locale)` for each non-default locale). Reverse order produces empty strings.
- **`LocalizedString` is wide**: each instance is `LOCALE_COUNT * sizeof(char*)` = 13 pointers on WoLK 3.4.3 (12 locales + 1 sentinel, varies by build). When serializing a record for the client, only one locale is written, but in-memory the cost is fixed-per-string × per-record. Several DB2s have many string fields (BroadcastText has 2 strings × ~92 k records = ~24 MB of string-pool pointers alone). Plan memory accordingly.
- **`WriteRecord` ASSERT**: TC's `WriteRecord` ASSERTs `id < _indexTableSize` and the index entry is not null. The Rust port should return `Result` instead — the client is allowed to query missing IDs (that's the point of `SMSG_DB_REPLY` returning a "not found" body), and aborting the world server because of a bad client query is a bug surface we don't want.
- **Endianness**: WDC4 is little-endian everywhere. Trinity does not byte-swap on big-endian hosts. The Rust port using `read_u32_le` is correct; don't accidentally use `from_be_bytes`.
- **`FT_LONG` is rare**: only RaceMask and a handful of bitfields. Most "64-bit" things are split into Hi/Lo pairs as two `FT_INT` fields. The Rust port's `get_field_i64` is hand-rolled to handle this.
- **Field meta vs file meta count**: `DB2Meta::FieldCount` is the count *as the C++ struct sees them* (with arrays expanded into a single field); `FileFieldCount` is the count *as the file stores them*. They can differ when the file has trailing inline relationship fields stripped from the in-memory representation. The Rust port currently reads `total_field_count.max(field_count)` without distinguishing — fine for current tables, fragile for future ones.
- **Hotfix `Status::NotPublic`**: rows with this status are loaded server-side but not advertised to the client (used for staff-only items / spells). The Rust port has no equivalent gate yet.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class DB2FileLoader` | `pub struct Wdc4Reader` (in `wdc4.rs`) | Already partial — add `LayoutHash` assertions, copy-table materialization helpers |
| `class DB2FileLoaderImpl` (pimpl) | inlined into `Wdc4Reader` | No virtual dispatch needed in Rust |
| `struct DB2Header` | `struct Wdc4Header` (private) | Field names mapped 1-to-1 |
| `struct DB2SectionHeader` | `struct SectionHeader` (private) | 9 fields, same byte order |
| `class DB2Record` | implicit via `Wdc4Reader::get_field_*(record_idx, field)` | No `Record` cursor object — just index-based getters |
| `class DB2StorageBase` | (missing) — to add: `pub struct StorageBase` | Type-erased base for `Storage<T>`; holds `Vec<Option<Vec<u8>>>` index table + `StringPool` |
| `template<class T> class DB2Storage<T>` | (missing) — to add: `pub struct Storage<T>` | Generic; one global per table, like TC |
| `class DB2DatabaseLoader` | (missing) — to add: `pub struct HotfixOverlayLoader<'a>` | Borrowing a `&HotfixDatabase` and a `&TableMeta` |
| `struct DB2LoadInfo` | (missing) — `pub struct LoadInfo { meta: &'static TableMeta, statement: HotfixStatements }` | Mirror TC; bind a table to its hotfix-DB statement |
| `struct DB2Meta` / `DB2MetaField` | `pub struct TableMeta` / `pub struct FieldMeta` | See sub-task #SDS.3 |
| `struct DB2FieldMeta` | (collapse with `FieldMeta`) | TC has two near-identical types because of how the array-size dance interacts with the auto-generator; one `FieldMeta` is enough in Rust |
| `enum DBCFormer` (`FT_INT`, `FT_FLOAT`, …) | `enum FieldType` (`Int`, `Float`, `Byte`, `Short`, `Long`, `String`, `StringNotLocalized`) | Add a `byte_size(&self) -> Option<usize>` method (None for variable-length strings) |
| `class DBStorageIterator<T>` | `impl Iterator for Storage<T>::Iter` | Skip `None` slots; same forward-only semantics |
| `LocalizedString` | `pub type LocalizedString = [Option<&'static str>; LOCALE_COUNT]` | Or `[Arc<str>; …]` if the static lifetime is too constraining |
| `DB2FileSource` (abstract) | `pub trait Db2Source: Read + Seek` | Stdio impl free via `std::fs::File` |
| `DB2FileSystemSource` | `std::fs::File` directly | No wrapper needed |
| `class DB2FileLoadException` | `Wdc4Error` enum (`anyhow`-compatible) | Already loose `anyhow::Error` in Rust today |
| `DB2HotfixGenerator<T>` | `pub struct HotfixGenerator<'s, T>` with `apply_hotfix(id, fixer: impl Fn(&mut T))` | Used for runtime in-memory patches |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

### Findings table

| # | Pre-audit claim (sections 8–11) | Verified result | Evidence |
|---|---|---|---|
| 1 | "8 hand-rolled per-table readers (~2.4 KLoC)" | **PARTIALLY REFUTED.** Only **5 client `.db2` files** are parsed via `Wdc4Reader`: `Item.db2` (`item.rs`), `ItemSparse.db2` (`item_stats.rs`), `QuestXP.db2` (`quest_xp.rs`), `SkillLineAbility.db2` and `SkillRaceClassInfo.db2` (`skill.rs`). The other modules listed in sec. 8 — `spell.rs`, `quest.rs`, `area_trigger.rs`, `player_stats.rs` — load from **MariaDB world tables** via `HotfixDatabase` / `WorldDatabase`, not from .db2. So of the ~325 client DB2 tables, **5 are parsed (1.5%)**, not 8 (2.5%). LoC count is roughly right (~2.0 KLoC across the 5 .db2 readers — `item.rs` 123, `item_stats.rs` 424, `skill.rs` 608, `quest_xp.rs` 116, `area_trigger.rs` 312 [but that one's DB-driven]). | grep `Wdc4Reader::open` + `.db2` filename joins |
| 2 | `LayoutHash` validation absent | **CONFIRMED.** `crates/wow-data/src/wdc4.rs:61` declares `_layout_hash: u32` (note the leading underscore — Rust convention for "intentionally unused"). It is read from the file (`:618`) and stored, but no public accessor exists and no consumer asserts it. Compare to TC's `DB2Storage::Load` which calls `loadInfo->Meta->LayoutHash` and aborts on mismatch. | `wdc4.rs:61, 618`; public-API grep shows `pub fn` list excludes layout_hash |
| 3 | `RecordSize` validation against meta absent | **CONFIRMED.** `Wdc4Reader::record_size` (read at `wdc4.rs:58, 186`) is used as a stride for fixed-record offset math but is never compared against an expected `TableMeta::RecordSize`. Combined with #2, the next client patch can silently misread DB2s — there is no guard. | `wdc4.rs:186-330` |
| 4 | `DB2Meta::Instance()` / per-table struct + meta autogen pattern absent | **CONFIRMED.** `grep -rn "DB2Meta\|TableMeta\|FieldMeta\|LoadInfo\|DB2Storage\|Storage<"` over `crates/wow-data/src/` returns **zero matches**. Each per-table reader hard-codes column indices as integer literals; there is no field-meta table, no `pub struct TableMeta`, and no codegen pipeline. | grep |
| 5 | `HotfixBlobCache` is misnamed (preloads files, not DB hotfixes) | **CONFIRMED with nuance.** `crates/wow-data/src/hotfix_cache.rs:30-110` only opens `Item.db2` and `ItemSparse.db2` from disk and stores raw record bytes keyed by `(table_hash, record_id)`. It does **not** read from `hotfixes.hotfix_blob` or `hotfixes.hotfix_data`. **Nuance:** the runtime `DBQueryBulk` handler (`crates/wow-world/src/handlers/character.rs:1126-1142`) does fall back to a direct SQL query on `hotfixes.hotfix_blob` when the disk cache misses — using `WorldStatements::SEL_HOTFIX_BLOB` (`wow-database/src/statements/world.rs:290-293`). That fallback is **single-shot**, **enUS-only** (the SQL hard-codes `locale = 'enUS'`), and bypasses any in-memory `Storage<T>`. None of the structural overlay logic from `DB2DatabaseLoader::Load` (two-pass custom flag, `hotfix_data.Status`, `_MAX_ID` resize, locale stacking) is implemented. | `hotfix_cache.rs` (full file); `character.rs:1115-1149`; `world.rs:290-293` |
| 6 | `HotfixStatements` enum is `_PLACEHOLDER` (not the ~783 statements TC needs) | **CONFIRMED.** `crates/wow-database/src/statements/hotfix.rs:14-25` contains exactly one variant (`_PLACEHOLDER`) returning empty SQL. The doc's "419+" comment in that file is itself an undercount — TC has 3 statements per table × ~261 tables ≈ 783. | `hotfix.rs` (full file) |
| 7 | Doc claim re. C++ canonical files exists at the listed paths | **CONFIRMED.** `find /home/server/woltk-trinity-legacy/src/{common,server/shared}/DataStores` resolves to all 11 files at the line-counts cited (within ±5%). The audit metadata "✅ complete (5 files in shared/, 6 in common/)" is accurate. | filesystem |

### Critical findings

1. **Silent DB2 misreads on next client patch (gap #2 + #3).** The `_layout_hash` field is read but discarded; combined with no per-table meta, a TBC/Cata/Wrath-Classic content patch that re-orders a column will be parsed without error and produce silently wrong fields. This is the single highest-risk gap and is the prerequisite for #SDS.4/#SDS.5 (codegen) to be useful — without an assertion the codegen output drifts from reality. **Action:** add `Wdc4Reader::layout_hash() -> u32` accessor + an `expect_layout_hash` parameter to `open()`; have each per-table reader pin its hash. ~30 min, **L** complexity.
2. **`HotfixBlobCache` name is actively misleading.** It loads from disk only; the runtime DB fallback lives in `handlers/character.rs`, not in this module. Recommend renaming to `Db2BlobCache` or `Db2DiskBlobCache` and documenting the runtime-path overlay in this doc's sec. 8.
3. **Locale lock-in.** `SEL_HOTFIX_BLOB` (`world.rs:292`) hard-codes `locale = 'enUS'`. Multi-locale realms cannot serve localized hotfix blobs — this is a future regression even before the typed `Storage<T>` work begins.
4. **Table coverage is even smaller than the doc admits.** 5 / ~325 ≈ 1.5% of the DB2 catalogue, not 8 / 325. The 3 doc-listed modules that are actually MariaDB-backed (`spell`, `quest`, `area_trigger`, `player_stats`) operate against the world DB and don't go through `Wdc4Reader` at all — they belong in a different migration doc, not this one.

### Status verdict

**Keep ⚠️ partial — but tighten the partial-coverage qualifier.** No upgrade is warranted (still ~5/325 of the table surface, no overlay, no meta, no `Storage<T>`). No downgrade either — the WDC4 file parser itself genuinely works. The header now says "1.5% of tables, no LayoutHash assertion, no DB overlay" so a casual reader doesn't mistake "⚠️ partial" for "almost done".

### Recommended sub-task priority shuffle

| Move | Reason |
|---|---|
| **#SDS.2 (LayoutHash assertion API) → top**, before #SDS.1. Promote complexity to a hard-blocker, even if tiny. | This is the only finding that produces silently-wrong output today. |
| #SDS.3 (`TableMeta` / `FieldMeta` Rust types) **stays second**. | Required for both #SDS.4 codegen and #SDS.6 `Storage<T>`. |
| **(new) #SDS.2.5** Rename `HotfixBlobCache` → `Db2BlobCache`; relocate the `SEL_HOTFIX_BLOB` runtime fallback documentation to this doc. **L**. | Reduces confusion for the next porter. |
| #SDS.4 + #SDS.5 (codegen for meta and structs) — **split**: do meta first (~325 entries), structs second. The doc already says XL — keep that, but mark structs as blocked on meta. | Meta unlocks `Storage<T>::lookup`; struct shape can land in waves. |
| #SDS.10 (populate `HotfixStatements` registry) — **promote** ahead of #SDS.6 (`Storage<T>`). | `Storage<T>::load_from_db` (#SDS.11) cannot land without it. |
| #SDS.11 / #SDS.12 (overlay loading) — **after** the runtime DB fallback in `character.rs:1126` is widened to all locales. | Avoid landing typed overlay while the wire path stays enUS-only. |

### Header status

Updated to **⚠️ partial — confirmed via audit 2026-05-01**. "Audited vs C++" line clarified to call out that the audit corrected one inaccurate count (5 .db2 files parsed, not 8). No ✅ → ❌ downgrade applies because the WDC4 parser itself is a real working implementation; the Storage<T>/overlay portion is what's missing.
