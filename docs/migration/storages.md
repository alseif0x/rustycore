# Migration: Storages (in-memory snapshot containers — currently just WhoListStorage)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Storages/`
> **Rust target crate(s):** `crates/wow-social/` or `crates/wow-world/` (no dedicated crate needed)
> **Layer:** L1 (infrastructure — periodically-rebuilt cache of online-player state)
> **Status:** ❌ not started — confirmed via audit 2026-05-01 (zero hits across `crates/`)
> **Audited vs C++:** ✅ complete (2 files, 154 lines total) — Rust-side absence reverified 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

Tiny module — *one* class. `Storages/` is a parallel folder to `DataStores/` for things that are also "global look-up tables" but are populated from runtime state (online players) rather than static client data files. Today it contains only `WhoListStorage`: a periodically-rebuilt snapshot of every connected, world-loaded, non-loading player, used to answer the `/who` command without iterating the global player map under lock for every query. The folder is named `Storages/` (plural) because the historical intent was for more such caches (`PendingMailStorage`, `MarketStorage`, etc.) to land here, but in the current TrinityCore tree only the who-list survives.

This doc could reasonably be merged into `social.md` or `chat.md`, but a separate doc is cheaper than burying the rebuild semantics inside an unrelated module.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Storages/WhoListStorage.cpp` | 68 | `prefix` |
| `game/Storages/WhoListStorage.h` | 86 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Storages/WhoListStorage.h` | 86 | `WhoListPlayerInfo` POD-ish entry + `WhoListStorageMgr` (Meyers singleton) |
| `src/server/game/Storages/WhoListStorage.cpp` | 68 | `instance()` + `Update()` (the only mutator) |

That is the entire module.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `WhoListPlayerInfo` | class | Snapshot of one online player. 15 fields: `_guid: ObjectGuid`, `_team: uint32`, `_security: AccountTypes`, `_level: uint8`, `_class: uint8`, `_race: uint8`, `_zoneid: uint32`, `_gender: uint8`, `_visible: bool`, `_gamemaster: bool`, `_widePlayerName: wstring` (lower-cased), `_wideGuildName: wstring` (lower-cased), `_playerName: string` (display case), `_guildName: string` (display case), `_guildguid: ObjectGuid` |
| `WhoListInfoVector` | typedef | `std::vector<WhoListPlayerInfo>` |
| `WhoListStorageMgr` | class | Singleton; one private member: `WhoListInfoVector _whoListStorage` |

The class has only getters — no public mutator. The vector is replaced wholesale by `Update()`.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `WhoListStorageMgr::instance()` | Meyers singleton accessor (`#define sWhoListStorageMgr WhoListStorageMgr::instance()`) | — |
| `WhoListStorageMgr::Update()` | Rebuild the snapshot. Walks `ObjectAccessor::GetPlayers()` (the `HashMapHolder<Player>` global map). For each player: skip if not on a map (`FindMap == nullptr`) or if their session is still in the loading screen. Read name/guild via `Player::GetName` and `GuildMgr::GetGuildNameById(player.guildId)`. Convert both to wstring + lowercase via `Utf8toWStr` + `wstrToLower`. Emplace a `WhoListPlayerInfo`. Replaces `_whoListStorage` wholesale (initial `clear()` + `reserve(player_count + 1)`). | `ObjectAccessor::GetPlayers`, `Player::Get*`, `GuildMgr::GetGuildNameById`, `Player::IsVisible`/`IsGameMaster`, `Player::GetTeam`/`GetSecurity`/`GetLevel`/`GetClass`/`GetRace`/`GetZoneId`/`GetNativeGender`, `Utf8toWStr`, `wstrToLower` |
| `WhoListStorageMgr::GetWhoList() const -> WhoListInfoVector const&` | Read-only access. Caller iterates linearly to apply `/who` filters | — |
| `WhoListPlayerInfo::Get*` (15 trivial getters) | Field access | — |

---

## 5. Module dependencies

**Depends on:**
- `Server/World` — `sWorld->GetPlayerCount()` for vector reservation.
- `Entities/Object/ObjectAccessor` — `GetPlayers()` returns the global `HashMapHolder<Player>::MapType const&`.
- `Entities/Player` — every getter on `Player` listed above; `IsVisible`, `IsGameMaster`, `GetGuild`, `GetGuildId`.
- `Server/WorldSession` — `GetSecurity()`, `PlayerLoading()` (skip-while-loading filter).
- `Guilds/GuildMgr` — `GetGuildNameById(guildId)`. Also `Guild::GetGUID()` for the guild GUID field.
- `ObjectGuid` — typed GUIDs.
- `Util` — `Utf8toWStr`, `wstrToLower`.
- `World/World` — referenced via `sWorld` macro.

**Depended on by:**
- `Handlers/CharacterHandler` (or `Handlers/MiscHandler`) — `WorldSession::HandleWhoOpcode` walks `sWhoListStorageMgr->GetWhoList()` applying the request's name/guild/zone/race/class/level filters and packages matches into `SMSG_WHO`.
- The world's main tick (`World::Update`) calls `Update()` periodically — typical interval is `5 * IN_MILLISECONDS` (5 s) configured by `CONFIG_WHO_LIST_UPDATE_INTERVAL`-equivalent. Snapshots stale by up to 5 s are acceptable for `/who`.

No other consumers — this is exclusively a `/who` accelerator.

---

## 6. SQL / DB queries (if any)

None. Pure in-memory; sources from runtime state.

| Statement / Source | Purpose | DB |
|---|---|---|
| (none) | — | — |

No DBC/DB2 stores either.

---

## 7. Wire-protocol packets (if any)

The module itself does not emit/receive packets; its consumer does:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_WHO` | client → server | `WorldSession::HandleWhoOpcode` (consumer of `GetWhoList`) |
| `SMSG_WHO` | server → client | `WorldSession::HandleWhoOpcode` (consumer of `GetWhoList`) |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None. Searches for `WhoList`, `WhoListStorage`, `who_list` across `crates/` return zero hits.
- `PlayerRegistry` (in `wow-network`, ~referenced from `AGENTS.md`) is a related primitive — it tracks online sessions — but it is not the same shape. The who-list cache is a denormalized read-only snapshot containing fields (level, class, zone, guild name, lowercase wide name) that aren't necessarily on the registry entry. Even if they were, hitting the live registry per `/who` query under any contention would lock-bottleneck — the explicit snapshot pattern exists exactly to avoid that.

**What's implemented:** nothing.

**What's missing vs C++:** the entire module. As long as `CMSG_WHO` is unimplemented or stubbed, the gap is invisible. As soon as a meaningful `/who` lands, the implementation will need either (a) the snapshot pattern, or (b) a non-locking iterator over `PlayerRegistry`. Pick (a) — TrinityCore picked it for a reason and the cost is one Vec rebuilt every 5 seconds.

**Suspicious / likely divergent:** N/A.

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#STORAGES.WBS.001** Cerrar la migracion auditada de `game/Storages/WhoListStorage.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp`
  Rust target: `crates/wow-social`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#STORAGES.WBS.002** Cerrar la migracion auditada de `game/Storages/WhoListStorage.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`
  Rust target: `crates/wow-social`, `crates/wow-world`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#STR.1** Define `pub struct WhoListPlayerInfo` mirroring the C++ class. Use owned `String` (UTF-8) for `player_name`/`guild_name` and pre-lowercased `String` for `wide_player_name_lower`/`wide_guild_name_lower`. WoW `wstrToLower` on UTF-16 is *not* the same as `to_lowercase` on UTF-8 for some Unicode codepoints — for the WoLK 3.4.3 client this rarely matters (Latin-1 supplement only) but document the divergence. (complexity: **L**)
- [ ] **#STR.2** Define `pub struct WhoListStorage { snapshot: ArcSwap<Vec<WhoListPlayerInfo>> }` (use `arc_swap` for lock-free reads — already a workspace-aligned pattern). Provide `pub fn snapshot(&self) -> Arc<Vec<WhoListPlayerInfo>>`. (complexity: **L**)
- [ ] **#STR.3** Implement `WhoListStorage::update(&self, player_registry: &PlayerRegistry, guild_mgr: &GuildMgr)`. Walk the registry, skip sessions still loading (`PlayerLoading()` equivalent — likely a flag on `WorldSession`), build a fresh `Vec` with `with_capacity(player_count + 1)`, replace `snapshot`. Match TrinityCore's filters: `FindMap == None` → skip; `PlayerLoading() == true` → skip. (complexity: **M**)
- [ ] **#STR.4** Wire periodic invocation: every 5 seconds in the world tick (`World::Update` analogue). Make the interval a config knob `wholist_update_interval_ms` defaulting to 5000. (complexity: **L**)
- [ ] **#STR.5** Implement `CMSG_WHO` handler that consumes the snapshot, applies the filter (name substring, guild substring, level range, race mask, class mask, zone list, server-string mask), caps at 49 results, packages into `SMSG_WHO`. (complexity: **M** — packet shape; the snapshot read is trivial.)
- [ ] **#STR.6** Lower-case wide-string handling: provide a `whostr::wstrtolower(input: &str) -> String` helper. For accuracy parity with TC, mirror `Utf8toWStr` (UTF-8 → UTF-16 LE) + `wstrToLower` (per-`wchar` `towlower`); a pure UTF-8 `to_lowercase` will be ≈99 % equivalent for Latin-script names but diverges for some German/French sharp-s and Greek edge cases. Acceptable hack: pure UTF-8 lower with a TODO. (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#STORAGES.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 154 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp`. Rust target: `crates/wow-network`, `crates/wow-social`, `crates/wow-world`. | `cargo test -p wow-network && cargo test -p wow-social && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#STORAGES.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 154 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp`. Rust target: `crates/wow-network`, `crates/wow-social`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#STORAGES.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 154 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp`. Rust target: `crates/wow-network`, `crates/wow-social`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#STORAGES.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 154 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp`. Rust target: `crates/wow-network`, `crates/wow-social`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] After `update()` with N online players in a fixture registry (some loading, some not on a map), `snapshot().len() == count(online ∧ on_map ∧ !loading)`.
- [ ] Names are stored in *both* display case (`player_name`) and lowercase (`wide_player_name_lower`).
- [ ] Two consecutive `update()` calls without state change produce vectors of equal length and content (idempotence).
- [ ] `update()` reservation is exact: `Vec::capacity() == len()` after build (asserts the `reserve(count + 1)` + immediate emplace pattern is preserved — minor perf concern).
- [ ] Concurrent `snapshot()` reads during an `update()` write never see partial state (use `loom` or stress test).
- [ ] Filter pipeline: an online level-80-warrior in zone Stormwind matches a `/who` request for "level 80 warrior stormwind"; the same player loading does not.
- [ ] Lower-case match: a player named "Áéíóú" matches a search for "áéíóú" after lowercasing.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 154 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp` | `crates/wow-social/` or `crates/wow-world/` (no dedicated crate needed) \| ❌ not started — confirmed via audit 2026-05-01 (zero hits across `crates/`) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#STORAGES.DIV.001` | `crates/wow-social` (`exists_empty`, 0 Rust lines) | 2 C++ files / 154 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.h`, `/home/server/woltk-trinity-legacy/src/server/game/Storages/WhoListStorage.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`Update()` is intentionally non-incremental**: the cost of detecting per-player deltas would dwarf the cost of a full rebuild. Don't try to make this incremental — it's 5-second-stale-by-design.
- **Lower-case both the player name and the guild name as wide-strings**. The C++ converts UTF-8 → UTF-16 → lower → keeps the wide form. The `/who` filter compares wide-string to wide-string. If you simplify to UTF-8 lower in Rust, document that searches for non-ASCII names may differ by a handful of codepoints from C++.
- **`_widePlayerName` is wide and lower; `_playerName` is UTF-8 and display-case** (used for the response packet). Same dual representation for guild. The Rust port should preserve the dual form to avoid converting per-`/who` query.
- **`PlayerLoading()` filter**: a session that has authenticated but is mid-`SMSG_LOGIN_VERIFY_WORLD` should not appear in `/who`. The C++ uses `WorldSession::PlayerLoading()` which is true between `HandlePlayerLogin` and the post-`SMSG_INITIAL_SPELLS`-or-equivalent point. Mirror this in Rust.
- **Guild GUID may be empty**: only set if `Player::GetGuild()` returns non-null. C++ assigns `ObjectGuid()` (zero) otherwise. Rust should use `Option<ObjectGuid>` or zero-guid sentinel.
- **`AccountTypes` (security level)**: `_security` is included so `/who` can hide GM accounts from non-GM searchers. The visibility filter is in the *consumer* (`HandleWhoOpcode`), not in `Update()` — every player goes into the snapshot regardless of GM status.
- **No tests in TrinityCore**: this module has no unit tests in the C++ tree either. The semantics are tiny enough that a regression here would be obvious from `/who` behaving wrong.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class WhoListStorageMgr` (singleton) | `pub struct WhoListStorage` + `Arc<WhoListStorage>` injected through `WorldServer` | Avoid global; pass through DI |
| `class WhoListPlayerInfo` | `pub struct WhoListPlayerInfo { … }` (#[derive(Clone)]) | All public fields or all getters — TC's choice of getters is C++-idiomatic; Rust public fields are fine for a POD |
| `WhoListInfoVector (typedef vector)` | `Vec<WhoListPlayerInfo>` | Wrapped in `ArcSwap` for lock-free reads |
| `Update()` | `update(&self, registry: &PlayerRegistry, guilds: &GuildMgr)` | Take both refs explicitly; no global state |
| `instance()` (Meyers singleton) | `OnceLock<Arc<WhoListStorage>>` if a global is required | Prefer DI |
| `std::wstring` (lower-cased) | `String` (UTF-8 lower-cased) | Mirror TC's wide-then-lower if exact behavior needed |
| `Utf8toWStr + wstrToLower` | `str::to_lowercase()` | Approximation; diverges for some Unicode codepoints — see gotcha |
| `ObjectGuid _guildguid` (default-constructed = zero) | `Option<ObjectGuid>` | Or `ObjectGuid::EMPTY` constant |
| `_security: AccountTypes` | `_security: AccountType` (enum from `wow-constants`) | — |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Method:** `grep -rEi "(WhoList|who_list|wholist)" crates/` and inspection of `wow-network` / `wow-social` / `wow-world` for any equivalent snapshot pattern.

**Findings:**

- **Zero matches** for `WhoList` / `who_list` / `wholist` anywhere under `crates/`. The module is genuinely entirely absent — confirms §8.
- `PlayerRegistry` in `wow-network` (workspace-wide registry of online sessions, mentioned in `AGENTS.md`) is the closest existing primitive but is fundamentally a different shape: it tracks live session state, not the denormalized `WhoListPlayerInfo` snapshot (level/class/zone/guild-name/lower-cased wide-name) that `/who` needs. There is no `ArcSwap`-style snapshot wrapper either.
- The `CMSG_WHO` / `SMSG_WHO` opcodes have no handler registered in `wow-world/src/handlers/` — confirmed by absence of any `WhoOpcode` / `who_opcode` / `handle_who` symbols. This means the gap is currently invisible to clients (the `/who` UI shows an empty list rather than wrong data), exactly as §8 predicted.
- `wow-social` has a `social.rs` module but it deals with friend/ignore lists, not the live-player snapshot.

**Suspicious / divergent:** none. The C++ module is 154 lines of self-contained code with no observable Rust counterpart; nothing to validate against.

**Status verdict:** ❌ not started (no change). The doc is accurate and the migration sub-tasks (#STR.1–#STR.6) remain the right plan. Suggested priority: **low** — `/who` is a quality-of-life feature; it does not block any other migration. Pick this up after `wow-handler` chat-command dispatch lands and after `PlayerRegistry` exposes a stable iterator. The `arc_swap`-based pattern in #STR.2 is the right shape; do not regress to a `Mutex<Vec<...>>`.
