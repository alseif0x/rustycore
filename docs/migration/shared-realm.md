# Migration: shared/Realm

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/Realm/`
> **Rust target crate(s):** `crates/bnet-server/` (`src/realm/mod.rs`)
> **Layer:** L1
> **Status:** ⚠️ partial (~80%) — 2026-06-13 slices fixed the BNet realm-address packing, strong `RealmHandle` contract, `HashMap<RealmHandleLikeCpp, Realm>` storage, typed `RealmFlagsLikeCpp`/`RealmTypeLikeCpp`, subregion filtering and `WriteSubRegions`, cfg timezone/category swap, type/security normalization, packed JoinRealm lookup plus realm-owned JoinRealm prep, hostname resolution/skip, normalized realm names, C++-shaped build_info hotfix/seed parsing, minor/major/bugfix build lookup and `JamJSONRealmEntry` for last-played character; remaining gaps include golden/e2e realm-list payloads and architectural unification with the world snapshot.
> **Audited vs C++:** ✅ audited 2026-06-13 against `Realm.h`, `Realm.cpp`, `RealmList.cpp` for the fixed BNet realm-list slice.
> **Last updated:** 2026-06-13

---

## 1. Purpose

Modela el conjunto de realms (servidores de juego) que el bnetserver ofrece al cliente WoW. Mantiene en memoria la lista refrescada periódicamente desde la tabla `realmlist` de la DB `auth`, almacena el `build_info` (versiones de cliente con sus `Win64AuthSeed`/`Mac64AuthSeed`), y serializa esa lista a JSON+zlib para los RPCs `JoinRealm` / `GetRealmList` del protocolo Battle.net. También expone `RealmHandle` (region/site/realm) como clave de direccionamiento de realms.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/Realm/Realm.cpp` | 61 | `prefix` |
| `shared/Realm/Realm.h` | 102 | `prefix` |
| `shared/Realm/RealmList.cpp` | 434 | `prefix` |
| `shared/Realm/RealmList.h` | 98 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/shared/Realm/Realm.h` | 102 | `Realm` struct, `RealmHandle`, `RealmFlags`, `RealmType` |
| `src/server/shared/Realm/Realm.cpp` | 61 | `SetName`, `GetAddressForClient`, `GetConfigId`, `RealmHandle` formatters |
| `src/server/shared/Realm/RealmList.h` | 98 | `RealmList` singleton + `RealmBuildInfo` struct |
| `src/server/shared/Realm/RealmList.cpp` | 434 | DB load, periodic refresh, `JoinRealm`, JSON+zlib emission |
| **TOTAL** | **~695** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Realm` | struct | Datos de un realm (Id, Build, addresses, port, name, type, flags…) |
| `Battlenet::RealmHandle` | struct | `(Region:u8, Site:u8, Realm:u32)` — clave compuesta + `GetAddress()` packed `u32` |
| `RealmFlags` | enum | `NONE / VERSION_MISMATCH / OFFLINE / SPECIFYBUILD / RECOMMENDED / NEW / FULL` |
| `RealmType` | enum | `NORMAL=0 / PVP=1 / NORMAL2=4 / RP=6 / RPPVP=8 / FFA_PVP=16` (custom) |
| `RealmBuildInfo` | struct | `Build`, `Major/Minor/BugfixVersion`, `HotfixVersion[4]`, `Win64AuthSeed[16]`, `Mac64AuthSeed[16]` |
| `RealmList` | singleton class | Holder global con `RealmMap = std::map<RealmHandle, Realm>` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `RealmList::Instance()` | Singleton accessor | — |
| `RealmList::Initialize(io, interval)` | Carga inicial de builds + realms; arma timer | `LoadBuildInfo`, `UpdateRealms` |
| `RealmList::UpdateRealms()` | Refresca desde DB y reprograma timer | `LOGIN_SEL_REALMLIST`, `Resolver::Resolve` |
| `RealmList::GetRealm(handle)` | Lookup thread-safe (shared_lock) | — |
| `RealmList::GetBuildInfo(build)` | Encuentra `RealmBuildInfo` por número de build | linear scan |
| `RealmList::GetRealmEntryJSON(id, build)` | Serializa un `RealmEntry` → "JamJSONRealmEntry:" + zlib | `JSON::Serialize` |
| `RealmList::GetRealmList(build, subRegion)` | Serializa lista entera filtrada por subregion → "JSONRealmListUpdates:" + zlib | `JSON::Serialize` |
| `RealmList::JoinRealm(addr, build, clientAddr, secret, ...)` | Construye respuesta JoinRealm: server IP + 32 bytes ServerSecret + persiste keyData en DB | `LOGIN_UPD_BNET_GAME_ACCOUNT_LOGIN_INFO` |
| `RealmList::WriteSubRegions(resp)` | Escribe set de subregiones a `GetAllValuesForAttributeResponse` | — |
| `Realm::SetName(name)` | Asigna `Name` y `NormalizedName` (sin spaces) | — |
| `Realm::GetAddressForClient(clientAddr)` | Selecciona address externa o local según red del cliente | `Trinity::Net::SelectAddressForClient` |
| `RealmHandle::GetAddress()` | Empaqueta `(Region<<24)|(Site<<16)|Realm` en `u32` | — |
| `RealmHandle::GetAddressString()` | Formato `"{Region}-{Site}-{Realm}"` | — |

---

## 5. Module dependencies

**Depends on:**
- `shared/Database` — `LoginDatabase`, prepared statement `LOGIN_SEL_REALMLIST`, `LOGIN_UPD_BNET_GAME_ACCOUNT_LOGIN_INFO`
- `shared/Networking` — `Trinity::Asio::Resolver`, `DeadlineTimer`, `IoContext`
- `shared/JSON` — `JSON::Serialize` (ProtobufJSON wrapper)
- `proto` — `JSON::RealmList::RealmEntry`, `RealmListUpdates`, `RealmListServerIPAddresses`
- `shared/Crypto` — `Trinity::Crypto::GetRandomBytes<32>` para ServerSecret
- `zlib` — `compress`, `compressBound`

**Depended on by:**
- `bnetserver` — `RealmList::Initialize` en main, RPC handlers `JoinRealm`, `GetAllValuesForAttribute`
- `worldserver` — algunos servicios consultan `RealmList::Instance()` para sub-regions

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| inline `SELECT majorVersion, minorVersion, bugfixVersion, hotfixVersion, build, win64AuthSeed, mac64AuthSeed FROM build_info ORDER BY build ASC` | Carga build_info | auth |
| `LOGIN_SEL_REALMLIST` | Carga `realmlist` table (id, name, address, localAddress, port, icon, flag, timezone, allowedSecurityLevel, population, gamebuild, Region, Battlegroup) | auth |
| `LOGIN_UPD_BNET_GAME_ACCOUNT_LOGIN_INFO` | Persiste keyData (clientSecret + serverSecret) + locale + os + timezoneOffset + accountName tras `JoinRealm` | auth |

---

## 7. Wire-protocol packets

N/A directos. Genera **payloads JSON+zlib** que el RPC bnet incluye en atributos:

| Attribute name | Origin |
|---|---|
| `Param_RealmJoinTicket` | accountName |
| `Param_ServerAddresses` | "JSONRealmListServerIPAddresses:" + JSON + zlib (4-byte uncompressed prefix) |
| `Param_JoinSecret` | 32 bytes random |

Y para realm list updates:
- envelope `"JamJSONRealmEntry:" + json + '\0'` (single realm)
- envelope `"JSONRealmListUpdates:" + json + '\0'` (full list)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |
| `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/bnet-server/src/realm/mod.rs` — cubre ~80% del C++ shared/Realm surface
- `crates/wow-database/src/statements/login.rs` — declara `SEL_REALMLIST`, `SEL_REALMLIST_SECURITY_LEVEL`

**What's implemented:**
- Struct `Realm` con todos los campos de `realmlist`
- `RealmBuildInfo` con `HotfixVersion[4]` y seeds Win/Mac `[u8;16]`, usando las reglas C++: hotfix solo copia si longitud `<4`; seeds solo parsean si el hex mide exactamente 32 chars.
- `RealmManager` con `realms: HashMap<RealmHandleLikeCpp, Realm>` + `builds: Vec<RealmBuildInfo>` + `sub_regions: Vec<String>`
- `Realm` guarda `name` y `normalized_name` como C++ `Realm::SetName`, eliminando whitespace ASCII.
- `init_realm_manager` con polling task Tokio
- `load_build_info`, `update_realms` directo SQL
- `get_realm_list_json` con flag `VERSION_MISMATCH` dinámico y `population_state` correcto
- `get_realm_entry_json` con selector loopback / same-/24 / external (similar a `Trinity::Net::SelectAddressForClient`)
- Envelopes correctos: `JSONRealmListUpdates:` / `JSONRealmListServerIPAddresses:` / `JSONRealmCharacterCountList:`
- Compresión zlib con prefijo `u32` little-endian de tamaño descomprimido
- `find_realm_by_address`, `get_build_info`
- `RealmHandleLikeCpp` cubre constructor `(region, battlegroup, realm)`, constructor desde packed address, `GetAddress()`, `GetAddressString()`, `GetSubRegionAddress()` y equality/order por `Realm` como C++.
- `get_realm_list_json` filtra por subregion como `RealmList::GetRealmList`, emite `wowRealmAddress` packed, `cfgTimezonesId=1`, `cfgCategoriesId=realm.Timezone`, `cfgConfigsId=Realm::GetConfigId()` y fallback de versión `6.2.4` cuando falta `build_info`.
- `update_realms` normaliza `REALM_TYPE_FFA_PVP -> REALM_TYPE_PVP`, `icon >= MAX_CLIENT_REALM_TYPE -> NORMAL`, y clampa `allowedSecurityLevel` a `SEC_ADMINISTRATOR`.
- `RealmFlagsLikeCpp` cubre todos los bits C++ de `RealmFlags`; `RealmTypeLikeCpp` es newtype tipado para preservar también valores válidos no nombrados `< MAX_CLIENT_REALM_TYPE`.
- `update_realms` resuelve `address` y `localAddress` con `tokio::net::lookup_host`, toma la primera IPv4 y salta el realm con error si alguna dirección no resuelve, igual que C++ `Resolver::Resolve`.
- `RealmManager::write_sub_regions_like_cpp` emite `Variant.string_value` por cada sub-region, igual que C++ `RealmList::WriteSubRegions`.
- `RealmManager::prepare_join_realm_like_cpp` posee la parte realm-owned de `RealmList::JoinRealm`: lookup por packed address, rechazo offline/build mismatch y payload comprimido `JSONRealmListServerIPAddresses`.
- `authentication` guarda `char_counts` y `last_played_chars.realm_address` con packed `RealmHandle::GetAddress()`, como C++ `Battlenet::Session`.
- `get_minor_major_bugfix_version_for_build_like_cpp` replica `RealmList::GetMinorMajorBugfixVersionForBuild` con semántica `lower_bound`.
- `get_realm_entry_json_like_cpp` genera `JamJSONRealmEntry` para `LastCharPlayed`, devuelve vacío si el realm está offline o el build no coincide, y ya no confunde ese payload con `JSONRealmListServerIPAddresses`.
- `RealmManager.realms` usa `RealmHandleLikeCpp` como clave, con `Hash`/`Eq` por `realm` solamente para reflejar el contrato C++ de `Battlenet::RealmHandle`.

**What's missing vs C++:**
- **`JoinRealm` effectful flow existe fuera de `RealmManager`** — la parte realm-owned ya vive en `RealmManager::prepare_join_realm_like_cpp`, pero `bnet-server/src/rpc/services/game_utilities.rs` todavía genera `Param_RealmJoinTicket`, `Param_JoinSecret`, random server secret y persiste keyData; falta concentrar o cubrir esos efectos como `RealmList::JoinRealm` si se quiere igualar ownership C++ completo.
- **Error path para `Resolver::Resolve`** — cerrado para realm load: Rust ahora resuelve y salta realms inválidos; falta e2e con DB real para cubrir el path visible.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- ✅ fixed 2026-06-13: `wow_realm_address: r.id as i32` perdía la información Region/Site/Realm packed que el cliente espera.
- ✅ fixed 2026-06-13: `cfg_timezones_id` / `cfg_categories_id` estaban cruzados frente a C++.
- ✅ fixed 2026-06-13: inbound `Param_RealmAddress` de `JoinRealm` buscaba el packed address completo como key, en vez de resolver el realm id como `RealmHandle(realmAddress)`.
- Sin `shared_mutex`, Rust usa `parking_lot::RwLock` (vía `state.realm_mgr.write()`) → equivalente.

**Tests existing:**
- `cargo test -p bnet-server realm` cubre packed address/subregion, filtro de `GetRealmList`, cfg fields, fallback de versión, type normalization y packed lookup.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_REALM.WBS.001** Cerrar la migracion auditada de `shared/Realm/Realm.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_REALM.WBS.002** Cerrar la migracion auditada de `shared/Realm/Realm.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_REALM.WBS.003** Cerrar la migracion auditada de `shared/Realm/RealmList.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_REALM.WBS.004** Cerrar la migracion auditada de `shared/Realm/RealmList.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [x] **#REALM.1a** Implementar helpers `RealmHandle::GetAddress()` / `GetSubRegionAddress()` C++-like para BNet realm-list y JoinRealm. (L)
- [x] **#REALM.1b** Implementar `RealmHandle { region, site, realm }` completo con `get_address_string()` y semántica de equality/order explícita. (L)
- [x] **#REALM.2** Cambiar `RealmManager.realms` a `HashMap<RealmHandle, Realm>` y propagar a calls sites. (M)
- [x] **#REALM.3** Auditar y corregir `cfg_timezones_id` vs `cfg_categories_id` contra `RealmList.cpp:270/:272/:330/:332`. (L)
- [x] **#REALM.4a** Mantener `JoinRealm` flow existente: random ServerSecret + persistir `LOGIN_UPD_BNET_GAME_ACCOUNT_LOGIN_INFO` + atributos respuesta. (M)
- [x] **#REALM.4b.1** Reubicar la preparación realm-owned de `RealmList::JoinRealm` en `RealmManager`: lookup packed, offline/build gates y server-address payload comprimido. (L)
- [ ] **#REALM.4b** Reubicar/encapsular el flow completo como ownership `RealmList::JoinRealm` C++-like y añadir golden/integration; DB update, random server secret y response attributes siguen en el handler. (M)
- [x] **#REALM.5** Implementar `set_name` con `NormalizedName` (strip whitespace). (L)
- [x] **#REALM.6** Implementar `get_minor_major_bugfix_version_for_build` con semántica `lower_bound`. (L)
- [x] **#REALM.7** Tipar `RealmFlags` como `bitflags!` y `RealmType` como wrapper C++-like. Nota: C++ declara `RealmType` como enum, pero `Realm::Type` es `uint8` y `GetConfigId` indexa valores `< MAX_CLIENT_REALM_TYPE`; Rust usa newtype para no rechazar valores válidos no nombrados. (M)
- [x] **#REALM.8** Resolver hostnames (no solo IPs) con `tokio::net::lookup_host` en `update_realms`, tomando primera IPv4 y saltando el realm si external/local no resuelve como C++. (M)
- [x] **#REALM.9** Clamp `allowed_security_level` a `SEC_ADMINISTRATOR`. (L)
- [x] **#REALM.10a** Tests: packed address bit-layout, subregion filter, cfg fields, version_mismatch/fallback. (M)
- [ ] **#REALM.10b** Tests: parse build_info, `get_realm_entry_json`, JoinRealm DB side effect, golden payload. Parcial: parseo C++ de `build_info` hotfix/seeds, `JamJSONRealmEntry`, empty gates, server-address selection y preparación realm-owned de JoinRealm cubiertos por unit tests; faltan JoinRealm DB side effect y golden payload. (M)
- [x] **#REALM.11** Modelar `RealmList::WriteSubRegions` como método de `RealmManager` y delegar `GetAllValuesForAttribute` en él. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_REALM.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 695 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h`. Rust target: `crates/bnet-server`. | `cargo test -p bnet-server` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_REALM.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 695 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h`. Rust target: `crates/bnet-server`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_REALM.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 695 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h`. Rust target: `crates/bnet-server`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_REALM.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 695 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h`. Rust target: `crates/bnet-server`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [x] `RealmHandle::get_address()` empaqueta `(region<<24)|(site<<16)|realm` exact-match
- [x] `set_name("Foo Bar")` produce `normalized_name == "FooBar"`
- [x] Realm con `build != client_build` → emite `flags |= VERSION_MISMATCH` en JSON
- [x] Realm con `flags & OFFLINE` → `GetRealmEntryJSON` devuelve vacío y `GetRealmList` emite `population_state = 0`
- [x] `select_realm_ip_str` para client 127.0.0.1 → local; para client en /24 distinto → external
- [x] zlib output: 4-byte LE prefix == uncompressed length; flate2 inflates back to identical bytes
- [ ] Concurrent: 1000 readers + 1 writer con `update_realms` no causa race / panic

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 695 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h` | `crates/bnet-server/` (`src/realm/mod.rs`) \| ⚠️ partial (~79%) — BNet RealmHandle packing/storage/cfg swap, typed flags/types, hostname resolution, subregion writer, normalized names, build-info parsing, build-version lookup and `JamJSONRealmEntry` fixed; golden/e2e and ownership cleanup remain. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_REALM.DIV.001` | _none generated_ | 4 C++ files / 695 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/Realm.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Realm/RealmList.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **Region/Site/Realm packing:** WoW client transmite `wowRealmAddress` como `u32` packed. RustyCore corrigió el path BNet realm-list / char-counts / JoinRealm lookup y añadió `RealmHandleLikeCpp` como key de storage el 2026-06-13; todavía falta añadir golden/e2e.
2. **Envelope strings con `\0` final:** El C++ usa `json.length() + 1` para incluir el null terminator en la longitud comprimida. Rust replica con `format!("...:{}\0", json)`.
3. **`JamJSONRealmEntry` vs `JSONRealmListUpdates`:** dos serializaciones distintas para el mismo `RealmEntry` proto — el primero es single-shot (refresh de uno), el segundo plural updates.
4. **`HARDCODED_DEVELOPMENT_REALM_CATEGORY_ID = 1` naming trap:** in this C++ branch `cfgtimezonesid` is the constant `1` and `cfgcategoriesid` receives `realm.Timezone` in realm-list JSON. Rust matched this on 2026-06-13; keep the test because the naming is easy to invert again.
5. **`build_info.HotfixVersion`** es `array<char,4>` no null-terminated — Rust lo guarda como `String` (puede tener bytes basura si el DB devuelve >4 chars).
6. **`Realm.AllowedSecurityLevel`:** tipo `AccountTypes` (enum), C++ hace `min(value, SEC_ADMINISTRATOR)`. Rust clampa desde 2026-06-13, pero sigue usando `u8` en vez de enum tipado.
7. **`RealmType` no es enum cerrado en Rust:** C++ enumera tipos conocidos, pero la storage field es `uint8` y `ConfigIdByType` acepta cualquier valor `0..13`. Rust usa `RealmTypeLikeCpp(u8)` para no inventar rechazo de datos.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `class RealmList` (singleton) | `struct RealmManager` (en `AppState.realm_mgr: Arc<RwLock<RealmManager>>`) | Lifecycle vía `init_realm_manager` |
| `Battlenet::RealmHandle` | `RealmHandleLikeCpp` with packed address, address strings, equality/order/hash by `Realm` and RealmManager storage key | Done for BNet realm storage |
| `struct Realm` | `struct Realm` | 1:1 fields |
| `RealmFlags` enum | `RealmFlagsLikeCpp` bitflags | All C++ bits preserved |
| `RealmType` enum / `uint8 Type` | `RealmTypeLikeCpp(u8)` | Newtype preserves valid unnamed values `< MAX_CLIENT_REALM_TYPE` |
| `RealmBuildInfo` | `struct RealmBuildInfo` | `HotfixVersion[4]`, `Win64AuthSeed[16]`, `Mac64AuthSeed[16]` shape preserved |
| `std::map<RealmHandle, Realm>` | `HashMap<RealmHandleLikeCpp, Realm>` | Hash/Eq intentionally use only `realm`, matching C++ equality/order |
| `std::shared_mutex` | `parking_lot::RwLock` | Vía `AppState.realm_mgr` |
| `DeadlineTimer + async_wait` | `tokio::time::interval` + `tokio::spawn` | En `init_realm_manager` |
| `JSON::Serialize(proto)` | `serde_json::to_string(&struct)` | Pure serde, no protobuf |
| `compress()` (zlib) | `flate2::ZlibEncoder` | Mismo formato |
| `RealmList::JoinRealm` realm lookup/gates/server-address payload | `RealmManager::prepare_join_realm_like_cpp` | DB update, random secret and response attributes still in handler |
| `Trinity::Crypto::GetRandomBytes<32>` | `rand::thread_rng().fill` in `game_utilities::join_realm` | TODO #REALM.4b for full ownership/golden coverage |
| `Trinity::Asio::Resolver::Resolve` | `resolve_realm_address_like_cpp` + `tokio::net::lookup_host` | Takes first IPv4; skips realm on external/local failure |
| `RealmList::WriteSubRegions` | `RealmManager::write_sub_regions_like_cpp` | Emits `Variant.string_value` values in stored order |
| `Trinity::Net::SelectAddressForClient` | `select_realm_ip_str` | Más simple: solo IPv4 + /24 + loopback |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Method:** Read `crates/bnet-server/src/realm/mod.rs` and BNet RPC call sites, cross-checked against C++ `shared/Realm/{Realm.h, Realm.cpp, RealmList.cpp}`. Verified the two specific divergences flagged in §8 and the inbound `JoinRealm` packed-address path.

**Verdicts on flagged hypotheses:**

1. **`RealmHandle` packing/storage — FIXED for BNet realm-list 2026-06-13.** Rust now emits `wowRealmAddress = (Region << 24) | (Site << 16) | uint16(Realm)`, filters by `GetSubRegionAddress()`, stores packed character counts / last-played realm address, resolves inbound `JoinRealm` packed addresses back to the low 16-bit realm id like `RealmHandle(realmAddress)`, and stores realms behind `RealmHandleLikeCpp` with C++ equality/order/hash by realm id. Remaining work: golden/e2e.
2. **`cfg_timezones_id` ↔ `cfg_categories_id` swap — FIXED 2026-06-13.** C++ `RealmList.cpp:270` and `:330` set `cfgtimezonesid = 1` (constant); `:272` and `:332` set `cfgcategoriesid = realm.Timezone`. Rust now mirrors this and has focused tests.

**Other findings during the audit:**

- **`JoinRealm` flow EXISTS** but in `crates/bnet-server/src/rpc/services/game_utilities.rs:233-303`, not under the realm module. Generates 32-byte server secret with `rand::thread_rng().fill`, persists `client_secret + server_secret` via `LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO`, returns the three response blobs (`Param_RealmJoinTicket`, `Param_ServerAddresses`, `Param_JoinSecret`). The §8 claim "JoinRealm flow no existe" was **WRONG** — update §8 to reflect that the flow lives at the RPC-handler layer rather than as a `RealmList::JoinRealm` method. Sub-task #REALM.4 is therefore partially-done: the wire-level behavior works; what's missing is the C++-style architectural placement (RealmList owning the join logic) and the Resolver hostname resolution.
- **`RealmHandle` decomposition** on inbound `JoinRealm` was fixed 2026-06-13 via `get_realm_by_realm_address_like_cpp`.
- **Hostname resolution — FIXED for realm load 2026-06-13.** Rust now resolves external/local addresses and skips the row on failed resolution, matching the C++ `continue` behavior after `Resolver::Resolve` failure.
- **Timezone field type:** Rust reads `r.timezone: u8` (line 273); C++ uses `uint8` for the realmlist column too — equivalent.

**Status verdict:** ⚠️ partial. The two flagged wire bugs are fixed and tested, and the §8 wording about JoinRealm has been corrected: the flow exists at the RPC-handler layer, but architectural ownership and golden/e2e coverage remain open.
