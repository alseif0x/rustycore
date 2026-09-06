# Migration: Cache (CharacterCache)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Cache/`
> **Rust target crate(s):** `crates/wow-database/` (or a thin `crates/wow-cache/`); consumers in `wow-world` and `wow-social`
> **Layer:** L1 (infrastructure — read-mostly in-memory index over `characters` table)
> **Status:** ❌ not started — confirmed via audit 2026-05-01 (zero hits across `crates/`)
> **Audited vs C++:** ✅ complete (module is 2 files, ~315 lines) — Rust-side absence reverified 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

In-memory index of every character row that exists on the realm — name, GUID, account, race/class/sex, level, guild id, arena team ids, deletion flag. Loaded once at startup from `characters.characters`, then kept in sync via `Update*` calls on every relevant gameplay event. Avoids repeated DB round-trips for things that need a name or class given only a GUID (mail, friends, guilds, who-list, chat, GM commands). The TrinityCore folder is named `Cache/` and contains only `CharacterCache` — there is no separate `GuildCache` (guild lookup goes through `GuildMgr`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Cache/CharacterCache.cpp` | 316 | `prefix` |
| `game/Cache/CharacterCache.h` | 77 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Cache/CharacterCache.h` | 78 | `CharacterCacheEntry` struct, `CharacterCache` class (Meyers singleton) |
| `src/server/game/Cache/CharacterCache.cpp` | 316 | All implementations + the two namespace-private `unordered_map`s |

That is the entire module. No other files.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `CharacterCacheEntry` | struct | The cached row: `Guid`, `Name`, `AccountId`, `Class`, `Race`, `Sex`, `Level`, `GuildId`, `ArenaTeamId[3]`, `IsDeleted` |
| `CharacterCache` | class | Meyers-singleton manager; `instance()` returns the global |
| anonymous namespace | — | `_characterCacheStore: unordered_map<ObjectGuid, CharacterCacheEntry>` and `_characterCacheByNameStore: unordered_map<string, CharacterCacheEntry*>` |

The two stores are kept consistent: the by-name store points into the by-guid store (so deletion order matters — name first, then guid).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `CharacterCache::LoadCharacterCacheStorage()` | Startup; `SELECT guid, name, account, race, gender, class, level, deleteDate FROM characters` | `AddCharacterCacheEntry` |
| `AddCharacterCacheEntry(guid, accountId, name, gender, race, class, level, isDeleted)` | Insert; only adds to name store if `!isDeleted` | — |
| `DeleteCharacterCacheEntry(guid, name)` | Remove from both stores | — |
| `UpdateCharacterData(guid, name, gender?, race?)` | Rename (and optional gender/race change); broadcasts `SMSG_INVALIDATE_PLAYER` to ALL sessions | `World::SendGlobalMessage` |
| `UpdateCharacterGender / Level / AccountId / GuildId` | Single-field mutators (silent — no broadcast) | — |
| `UpdateCharacterArenaTeamId(guid, slot, teamId)` | `ASSERT(slot < 3)` then assign | — |
| `UpdateCharacterInfoDeleted(guid, deleted, name)` | Toggle deletion; sync name store; **also rewrites `Name`** | — |
| `HasCharacterCacheEntry(guid)` | Existence probe | — |
| `GetCharacterCacheByGuid(guid)` / `ByName(name)` | Pointer-or-null entry lookups | — |
| `GetCharacterGuidByName(name)` | Name → GUID (returns `ObjectGuid::Empty` if missing) | — |
| `GetCharacterNameByGuid(guid, &out)` | GUID → name via out-param + bool | — |
| `GetCharacterTeamByGuid(guid)` | GUID → `Team` via `Player::TeamForRace(race)` | `Player::TeamForRace` |
| `GetCharacterAccountIdByGuid / ByName` | Account-id getters | — |
| `GetCharacterLevelByGuid / GuildIdByGuid / ArenaTeamIdByGuid(guid, type)` | Field getters | `ArenaTeam::GetSlotByType` |
| `GetCharacterNameAndClassByGUID(guid, &name, &cls)` | Combined getter (used by chat / GM) | — |

Convention: GUID → entry calls via `_characterCacheStore`; Name → entry via `_characterCacheByNameStore` (which holds raw pointers into the first store — not strings). This is unsafe-by-construction: any `unordered_map` rehash would invalidate the name pointers if the by-guid store grew, but in practice the cache is sized once at startup and only shrinks via deletion.

---

## 5. Module dependencies

**Depends on:**
- `Entities/Object/ObjectGuid` — typed GUIDs.
- `Database` — `CharacterDatabase.Query` + `Field`.
- `Server/World` — `SendGlobalMessage` for `SMSG_INVALIDATE_PLAYER`.
- `Server/Packets/MiscPackets` — `WorldPackets::Misc::InvalidatePlayer`.
- `Entities/Player::TeamForRace` — race → team mapping.
- `Battlegrounds/ArenaTeam::GetSlotByType` — arena team slot resolver.

**Depended on by:** Almost everything that needs a name/level/account-id without owning the `Player*`. Notably:
- `Mails/` — sender/recipient name resolution.
- `Social/` — friends/ignores list rendering.
- `Guilds/` — member roster (level + class).
- `Chat/` — `/who`, whisper-by-name, channel join.
- `Handlers/CharacterHandler` — name reservation on character creation/rename.
- `Petitions/`, `Calendar/`, `Auctions/` — recipient-by-name lookups.

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| Inline `Query` in `LoadCharacterCacheStorage` | `SELECT guid, name, account, race, gender, class, level, deleteDate FROM characters` | character |

No prepared statements live inside this module — updates are done in-memory only. The actual DB writes for renames/level-ups/etc. happen in the calling code (`Player::SaveToDB`, character handlers); the cache is updated separately by callers after their own DB write. Any divergence between the cache and the DB is therefore caller bugs, not cache bugs.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_INVALIDATE_PLAYER` | server → client (broadcast) | `UpdateCharacterData` (only — no other mutator broadcasts) |

Used to tell clients to drop their client-side name caches and re-query for that GUID. Sent only on rename / appearance change.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-cache` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None. There is no character-cache crate or module. Look-ups by name today go through ad-hoc DB queries in handlers, or through `PlayerRegistry` (which only contains *online* players) — meaning offline-name resolution is broken or stubbed.

**What's implemented:** nothing.

**What's missing vs C++:** the entire cache. Specifically, every flow that calls `sCharacterCache->Get*ByName` in C++ must either (a) hit a fresh DB query, (b) use `PlayerRegistry` and miss offline names, or (c) be unimplemented. Mail, guild rosters, friends-list, and `/who` will all be incomplete until this lands.

**Suspicious / likely divergent:** any current Rust code that "looks up by name" and works only for online players is silently wrong — that bug class will surface as soon as the cache exists.

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#CACHE.WBS.001** Cerrar la migracion auditada de `game/Cache/CharacterCache.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`
  Rust target: `crates/wow-database`, `crates/wow-cache`, `crates/wow-world`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#CACHE.WBS.002** Cerrar la migracion auditada de `game/Cache/CharacterCache.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h`
  Rust target: `crates/wow-database`, `crates/wow-cache`, `crates/wow-world`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#CCH.1** Define `CharacterCacheEntry` struct (mirror C++ exactly: `guid: ObjectGuid`, `name: String`, `account_id: u32`, `class_id: u8`, `race: u8`, `sex: u8`, `level: u8`, `guild_id: u64`, `arena_team_id: [u32; 3]`, `is_deleted: bool`). (complexity: **L**)
- [ ] **#CCH.2** Implement `CharacterCache` as `Arc<RwLock<Inner>>` or `DashMap`-based — `by_guid: HashMap<ObjectGuid, CharacterCacheEntry>` + `by_name_lower: HashMap<String, ObjectGuid>` (use lowercase key — see gotcha). Avoid the C++ raw-pointer trick. (complexity: **M**)
- [ ] **#CCH.3** Implement `load_from_db(pool)` running the startup SELECT and populating both maps; deleted characters go into `by_guid` only. (complexity: **L**)
- [ ] **#CCH.4** Add the mutators: `add_entry`, `delete_entry`, `update_data` (rename + optional gender/race + broadcast), `update_gender`, `update_level`, `update_account_id`, `update_guild_id`, `update_arena_team_id`, `update_info_deleted`. Broadcast hook only on `update_data`. (complexity: **M**)
- [ ] **#CCH.5** Add the getters returning owned `Option<CharacterCacheEntry>` (or `Arc<CharacterCacheEntry>` if hot path) and the convenience derivatives (`team_for_guid` via `race_to_team`, `name_and_class_by_guid`). (complexity: **L**)
- [ ] **#CCH.6** Wire `SMSG_INVALIDATE_PLAYER` packet — find or define it in `wow-packet`, broadcast through the global `PlayerRegistry`. (complexity: **L**)
- [ ] **#CCH.7** Audit-pass: every call site in the Rust crates that resolves a character name today must be rewired to consult the cache; offline-name flows that were silently failing should now work. (complexity: **M**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#CACHE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 393 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h`. Rust target: `crates/wow-database`, `crates/wow-social`, `crates/wow-world`. | `cargo test -p wow-database && cargo test -p wow-social && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#CACHE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 393 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h`. Rust target: `crates/wow-database`, `crates/wow-social`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#CACHE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 393 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h`. Rust target: `crates/wow-database`, `crates/wow-social`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#CACHE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 393 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h`. Rust target: `crates/wow-database`, `crates/wow-social`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] After `load_from_db` of a fixture with N characters (some deleted), `by_name` count == count where `is_deleted = false` and `by_guid` count == N.
- [ ] `update_data` rewrites both maps consistently and emits exactly one `SMSG_INVALIDATE_PLAYER`.
- [ ] `delete_entry` removes from both maps; subsequent `get_by_name` returns `None`.
- [ ] `update_info_deleted(true)` removes from `by_name` but keeps `by_guid`; `update_info_deleted(false, new_name)` re-inserts with the new name.
- [ ] Name lookup is case-insensitive (WoW client normalizes capitalization on the wire).
- [ ] Concurrent reads while a single writer mutates do not race (loom or stress test).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 393 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h` | `crates/wow-database/` (or a thin `crates/wow-cache/`); consumers in `wow-world` and `wow-social` \| ❌ not started — confirmed via audit 2026-05-01 (zero hits across `crates/`) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#CACHE.DIV.001` | `crates/wow-cache` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 393 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#CACHE.DIV.002` | `crates/wow-social` (`exists_empty`, 0 Rust lines) | 2 C++ files / 393 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Cache/CharacterCache.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Name keys**: in C++ the by-name map uses the raw `std::string` from the DB. WoW clients send names with the canonical first-letter-cap spelling but the cache happens to be looked up via exact match. Trinity gets away with it because all writes use the same canonicalization. **For the Rust port, lower-case the key** — it's safer and avoids future foot-guns.
- **Two-store invariant**: the C++ name-store holds a `CharacterCacheEntry*` into the guid-store. Any `rehash` of the guid-store invalidates those pointers. Trinity is "safe" only because it sizes the cache once and never bulk-inserts after that. **Don't replicate this** — store the GUID in the name map, then chain to the guid map.
- **`UpdateCharacterInfoDeleted` is the only mutator that takes `name` separately and rewrites the entry's `Name` field**. The reason: undelete sends the new name directly. Don't merge it with `update_data`.
- The arena team `slot` parameter takes a *type* (2/3/5), not a slot index. `ArenaTeam::GetSlotByType(type)` does the conversion. Mirror this — don't expose raw slots.
- `IsDeleted` rows still occupy a `by_guid` slot (so undelete can restore the original entry); don't filter them out at load time.
- `LoadCharacterCacheStorage` is called *after* `Player::TeamForRace` is usable, but *before* `GuildMgr` and `ArenaTeamMgr` load — guild/arena fields are zeroed on insertion and patched in by those loaders. Order matters; document in the crate README.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class CharacterCache` (singleton) | `pub struct CharacterCache` + `pub static CHARACTER_CACHE: OnceLock<Arc<CharacterCache>>` | Or pass `Arc<CharacterCache>` explicitly through `WorldSession` — preferred over global |
| `struct CharacterCacheEntry` | `#[derive(Clone)] pub struct CharacterCacheEntry` | Cheap to clone (one `String`, ~40 bytes) |
| `unordered_map<ObjectGuid, CharacterCacheEntry>` | `DashMap<ObjectGuid, CharacterCacheEntry>` | Already in workspace deps |
| `unordered_map<string, CharacterCacheEntry*>` | `DashMap<String, ObjectGuid>` | Two-step lookup — no aliasing UB |
| `Optional<uint8>` | `Option<u8>` | — |
| `void Update*(...)` (returns void; silent) | `pub fn update_*(&self, ...) -> bool` (true if entry existed) | Saner than C++'s no-op-on-miss |
| `World::SendGlobalMessage(...)` | `PlayerRegistry::broadcast_all(bytes)` | Existing primitive in `wow-network` |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Method:** `grep -rEi "(CharacterCache|character_cache)" crates/` returns **zero matches**. Inspected `wow-database` and `wow-network` for any partial substitute (e.g. a `by_name`/`by_guid` map keyed off `characters` rows).

**Findings:**

- **Module is entirely absent.** No `CharacterCache` struct, no `CharacterCacheEntry`, no by-guid or by-name maps loaded from `characters.characters` at startup. `crates/wow-database/src/statements/character.rs` does not declare a `SEL_ALL_CHARACTERS` startup query.
- **Raw-pointer trick correctly avoided** — there is nothing to avoid yet, but when the migration lands, the §11 gotcha (don't replicate the `unordered_map<string, Entry*>` aliasing) is still load-bearing advice.
- `PlayerRegistry` (online-only) is the only adjacent primitive. As §8 predicts, every offline-name flow (mail recipient lookup, guild roster level/class display, friends list rendering, `/who` filters that hit logged-out characters) is currently either broken or hits the DB ad-hoc per query.
- **`SMSG_INVALIDATE_PLAYER` packet** — search returns no hits in `crates/wow-packet/`; not yet defined. Sub-task #CCH.6 will need to define it from scratch.
- **No tests** as predicted.
- The two-store invariant (by-guid owning + by-name pointer-into-guid) is a C++-only artifact; the §12 mapping (`DashMap<String, ObjectGuid>` + chain to `DashMap<ObjectGuid, CharacterCacheEntry>`) remains the correct Rust shape.

**Suspicious / divergent:** none. The doc accurately describes "everything missing".

**Status verdict:** ❌ not started (no change). Priority: **medium** — blocks correctness of mail/guild/friends/who once those surfaces light up. The migration sub-tasks (#CCH.1–#CCH.7) are well-scoped; #CCH.2 (the storage shape) is the only design call still open — recommend `DashMap` over `RwLock<HashMap>` for the read-heavy access pattern. Coordinate with the `accounts.md` audit (RBAC needs `account_id` lookups against the same row), but this cache should not depend on `wow-account`.
