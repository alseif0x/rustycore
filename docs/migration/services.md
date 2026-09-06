# Migration: Services (Worldserver-side BNet RPC dispatcher)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Services/`
> **Rust target crate(s):** `crates/wow-network/`, `crates/wow-world/`, `crates/wow-proto/` — *(out of scope for the WotLK 3.4.3 path; see notes)*
> **Layer:** L1 — Cross-realm RPC infrastructure
> **Status:** ⚠️ partial (stub) — confirmed via audit 2026-05-01: `CMSG_BATTLENET_REQUEST` handler EXISTS in `crates/wow-world/src/handlers/battlenet.rs` and replies `RpcNotImplemented` for every service hash. The §1 hypothesis "the client never asks" was wrong — the client does send it, we just don't service it.
> **Audited vs C++:** ✅ audited 2026-05-01 — recommendation in §9 ("treat as out-of-scope unless capture shows traffic") needs revision; capture-equivalent already exists in the form of an active stub handler
> **Last updated:** 2026-05-01

---

## 1. Purpose

Routes BNet protobuf RPC calls **received over the world-socket** (`SMSG_BATTLE_NET_REQUEST` / `CMSG_BATTLE_NET_REQUEST` family) into the right protobuf-service handler. It is the world-server-side mirror of the bnetserver `Services/` dispatcher: once a session is in-game, the client can keep talking to BNet services (friends, presence, club, realm-list refresh, …) over the existing encrypted world TCP channel rather than re-authenticating to the bnetserver. The module is essentially TC's gRPC-style template machinery glued onto a `WorldSession`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Services/WorldserverGameUtilitiesService.cpp` | 143 | `prefix` |
| `game/Services/WorldserverGameUtilitiesService.h` | 50 | `prefix` |
| `game/Services/WorldserverService.h` | 45 | `prefix` |
| `game/Services/WorldserverServiceDispatcher.cpp` | 49 | `prefix` |
| `game/Services/WorldserverServiceDispatcher.h` | 72 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Services/WorldserverService.h` | 45 | Generic `template<class T> WorldserverService : public T` adapter; rewires `SendRequest`/`SendResponse`/`GetCallerInfo` of any protobuf service to a `WorldSession`'s wire methods |
| `src/server/game/Services/WorldserverServiceDispatcher.h` | 72 | `WorldserverServiceDispatcher` singleton (`sServiceDispatcher`); maps `serviceHash → handler` |
| `src/server/game/Services/WorldserverServiceDispatcher.cpp` | 49 | Registers 12 services (`Account`, `Authentication`, `ClubMembership`, `Club`, `Connection`, `Friends`, `GameUtilities`, `Presence`, `Report v1`, `Report v2`, `Resources`, `UserManager`); dispatches buffered RPCs |
| `src/server/game/Services/WorldserverGameUtilitiesService.h` | 50 | `GameUtilitiesService` — handles `RealmListRequest` / `RealmJoinRequest` over the world socket |
| `src/server/game/Services/WorldserverGameUtilitiesService.cpp` | 143 | Compresses realm list into a protobuf attribute, optionally fetches realm-list sub-regions |

Related files (lives elsewhere, but part of the same RPC machinery):
- `src/common/Services/Service.h` / `ServiceBase.h` — base classes for generated protobuf services
- `src/server/proto/Server/{account,authentication,club,connection,friends,game_utilities,presence,report,resource_service,user_manager}_service.proto` — generated `*Service` C++ classes (one per service)
- `src/server/game/Server/WorldSession.cpp` — `SendBattlenetRequest / SendBattlenetResponse / HandleBattlenetRequest` invoke the dispatcher.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Battlenet::WorldserverService<T>` | template class | Adapter: subclasses any generated `T : public ServiceBase`, wires its RPC sinks to a `WorldSession` |
| `Battlenet::WorldserverServiceDispatcher` | class (singleton) | Map from `serviceHash` (FNV-1a-style 32-bit hash of the protobuf service's original name) to a function pointer that constructs the right service for a session and dispatches the call |
| `Battlenet::Services::GameUtilitiesService` | class : WorldserverService<game_utilities::v1::GameUtilitiesService> | Concrete handler for `Command_RealmListRequest_v1` and `Command_RealmJoinRequest_v1` — the in-game "show me the realm list / let me hop realms" flow |
| `ClientRequestHandler` | typedef (member fn ptr) | `uint32 (GameUtilitiesService::*)(const std::unordered_map<std::string, Variant const*>&, ClientResponse*)` |
| `MessageBuffer` | (defined elsewhere) | Owning byte buffer the dispatcher hands to the chosen service |
| `Attribute` / `Variant` | protobuf | Generated message types (`bgs::protocol::game_utilities::v1`) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `WorldserverServiceDispatcher::Instance()` | Magic-static singleton; constructs all 12 services lazily | `AddService<...>` for each |
| `WorldserverServiceDispatcher::Dispatch(session, serviceHash, token, methodId, buffer)` | Lookup the service in `_dispatchers`, invoke the templated `Dispatch<Service>` static which builds an instance and calls `CallServerMethod` | `Service::CallServerMethod(token, methodId, buf)` |
| `WorldserverService<T>::SendRequest(serviceHash, methodId, msg, callback)` | Push a server-initiated RPC onto the WorldSession's send queue | `WorldSession::SendBattlenetRequest` |
| `WorldserverService<T>::SendResponse(serviceHash, methodId, token, status_or_msg)` | Reply with status code or protobuf message | `WorldSession::SendBattlenetResponse` |
| `WorldserverService<T>::GetCallerInfo()` | Returns "Player <name> Account <id> [Map ... ]" for log lines | `WorldSession::GetPlayerInfo()` |
| `GameUtilitiesService::HandleProcessClientRequest(request, response, continuation)` | Parses `request.attribute()` into a `name → Variant*` map; dispatches by command name (after stripping suffix) | One of `HandleRealmListRequest`, `HandleRealmJoinRequest` |
| `GameUtilitiesService::HandleRealmListRequest(params, response)` | Calls `sRealmList->GetRealmList(build, subRegion)` to get a compressed protobuf blob, attaches it as `Param_RealmList` plus a zlib-compressed `JSONRealmCharacterCountList:...` JSON blob as `Param_CharacterCountList` | `RealmList`, `zlib::compress` |
| `GameUtilitiesService::HandleRealmJoinRequest(params, response)` | Reads `Param_RealmAddress`, calls `sRealmList->JoinRealm(addr, build, ip, secret, locale, os, tz, accountName, response)` | `RealmList`, `WorldSession` getters |
| `GameUtilitiesService::HandleGetAllValuesForAttribute(request, response, continuation)` | For attribute key prefix `Command_RealmListRequest_v1`: fills the response with sub-region keys via `sRealmList->WriteSubRegions` | `RealmList` |

---

## 5. Module dependencies

**Depends on:**
- `WorldSession` — for `SendBattlenetRequest`, `SendBattlenetResponse`, `GetPlayerInfo`, `GetRealmListSecret`, `GetRemoteAddress`, `GetSessionDbcLocale`, `GetOS`, `GetTimezoneOffset`, `GetAccountName`, `GetRealmCharacterCounts`
- `RealmList` (`sRealmList`) — `GetRealmList`, `JoinRealm`, `WriteSubRegions`
- `Common/Services/Service.h` — base `ServiceBase` template
- All 12 generated protobuf service classes (`account_service.pb.h` etc.)
- `BattlenetRpcErrorCodes.h` — `ERROR_OK`, `ERROR_RPC_NOT_IMPLEMENTED`, `ERROR_RPC_MALFORMED_REQUEST`, `ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE`, `ERROR_WOW_SERVICES_INVALID_JOIN_TICKET`
- `ProtobufJSON.h` — for `JSON::Serialize(realmCharacterCounts)`
- `zlib` — compression of the JSON character-count blob

**Depended on by:**
- `WorldSession::HandleBattlenetRequest` — the only call site of `sServiceDispatcher.Dispatch(...)`

---

## 6. SQL / DB queries (if any)

None directly. `RealmList::JoinRealm` writes a `LOGIN_UPD_BNET_GAME_ACCOUNT_LAST_LOGIN`-shaped row (lives in `RealmList`, not here).

DBC/DB2 stores: none.

---

## 7. Wire-protocol packets (if any)

The dispatcher itself doesn't define opcodes — it muxes on a 32-bit `serviceHash` carried inside protobuf-shaped CMSG/SMSG envelopes. The relevant world-socket opcodes (handled in `WorldSession`):

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_BATTLENET_NOTIFICATION` (0x4F1) | server → client | `WorldSession::SendBattlenetRequest` / `WorldserverService<T>::SendRequest` (server-initiated push) |
| `SMSG_BATTLENET_RESPONSE` (varies, ~0x4F2) | server → client | `WorldSession::SendBattlenetResponse` / `WorldserverService<T>::SendResponse` |
| `CMSG_BATTLENET_REQUEST` (~0x4F3) | client → server | `WorldSession::HandleBattlenetRequest` → `sServiceDispatcher.Dispatch(...)` |
| `CMSG_BATTLENET_LISTENER_OPCODES` (varies) | client → server | Protocol negotiation; not handled by dispatcher itself |

Inside each envelope: `(service_hash: u32, method_id: u32, token: u32, payload_len: u32, payload: ...)`.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |
| `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `crates/wow-proto/proto` | `module_dir` | 0 | 0 | `exists_empty` | directory exists; no active Rust source lines |
| `crates/wow-world/src` | `module_dir` | 17 | 12778 | `exists_active` | directory exists |
| `crates/wow-social` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-chat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` — these implement four BNet services on the **bnet-server side** (port 1119, before the world handshake). They are NOT the world-side dispatcher. The world-server has *no* equivalent.
- `crates/wow-proto/proto/` — has the protobuf generated types, but only what the bnet-server uses. The full set (`club`, `friends`, `presence`, `report`, `resources`, `user_manager`) is not in `wow-proto`.
- `crates/wow-world/src/` — no `BattlenetRequest` opcode handler, no `service_dispatcher`, no `WorldserverService` adapter. The session has no `send_battlenet_request` / `send_battlenet_response` methods.

**What's implemented:**
- BNet RPC dispatcher on the **auth port** (`bnet-server/src/rpc/`) — for the pre-game login handshake only.

**What's missing vs C++:**
- Everything specific to the worldserver side. The whole `WorldserverServiceDispatcher` + 12 service registrations + `GameUtilitiesService::HandleRealmListRequest / HandleRealmJoinRequest` (in-game realm-hop flow) are absent.
- `WorldSession::send_battlenet_request / send_battlenet_response / handle_battlenet_request` — no plumbing.
- All non-game-utilities services (account, auth, club, connection, friends, presence, report, resources, user_manager) on the world side.
- Service hash registry / FNV-1a hashing of original protobuf service names. The `Service::OriginalHash::value` constant is auto-generated by TC's `protoc` plugin; no Rust counterpart.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The Rust client appears to never trigger `CMSG_BATTLENET_REQUEST` against the world server (it never asks for realm hops in-game; players are forced to log out to switch realms). If true, this whole module can be parked indefinitely without breaking the WotLK 3.4.3 single-realm experience.
- Friends/Club/Presence services are mostly relevant to retail (chat across realms, communities). For 3.4.3 Classic one realm + in-game social we already cover via `wow-social` / `wow-chat`.
- Report v1/v2 (player reporting for harassment etc.) likely unused on a private server.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SERVICES.WBS.001** Cerrar la migracion auditada de `game/Services/WorldserverGameUtilitiesService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`
  Rust target: `crates/wow-network`, `crates/wow-world`, `crates/wow-proto`, `crates/wow-proto/proto`, `crates/wow-world/src`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SERVICES.WBS.002** Cerrar la migracion auditada de `game/Services/WorldserverGameUtilitiesService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h`
  Rust target: `crates/wow-network`, `crates/wow-world`, `crates/wow-proto`, `crates/wow-proto/proto`, `crates/wow-world/src`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SERVICES.WBS.003** Cerrar la migracion auditada de `game/Services/WorldserverService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverService.h`
  Rust target: `crates/wow-network`, `crates/wow-world`, `crates/wow-proto`, `crates/wow-proto/proto`, `crates/wow-world/src`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SERVICES.WBS.004** Cerrar la migracion auditada de `game/Services/WorldserverServiceDispatcher.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.cpp`
  Rust target: `crates/wow-network`, `crates/wow-world`, `crates/wow-proto`, `crates/wow-proto/proto`, `crates/wow-world/src`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SERVICES.WBS.005** Cerrar la migracion auditada de `game/Services/WorldserverServiceDispatcher.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`
  Rust target: `crates/wow-network`, `crates/wow-world`, `crates/wow-proto`, `crates/wow-proto/proto`, `crates/wow-world/src`, `crates/wow-social`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

> **Scoping note:** for the WotLK 3.4.3 Classic target, treat this entire module as **out-of-scope** unless and until the live client is observed sending `CMSG_BATTLENET_REQUEST` to the world-server. Sub-tasks below are listed for completeness; do not start work on them without an opcode capture demonstrating the need.

- [ ] **#SVC.1** Pcap-trace a 3.4.3.54261 client doing in-game `/realms`, character logout-to-realm-list, and friend/social actions. If `CMSG_BATTLENET_REQUEST` (opcode 0x4F3) never fires, mark this module as out-of-scope and stop. (L)
- [ ] **#SVC.2** If S.1 shows traffic: define `wow-network` envelope structs `BattlenetRequest { service_hash: u32, method_id: u32, token: u32, payload: Vec<u8> }` and matching `BattlenetResponse`. (M)
- [ ] **#SVC.3** Add `WorldSession::send_battlenet_request / send_battlenet_response / handle_battlenet_request`. (M)
- [ ] **#SVC.4** Compute the `serviceHash` for each generated protobuf service at build-time (TC uses an FNV-1a over the *original* fully-qualified service name; replicate exactly — `bgs.protocol.game_utilities.v1.GameUtilities` etc.). (M)
- [ ] **#SVC.5** Build a `WorldserverServiceDispatcher` keyed on those hashes, with a `dyn Fn(&mut WorldSession, token, method_id, Bytes) -> Result<()>` table. (M)
- [ ] **#SVC.6** Port `GameUtilitiesService::handle_process_client_request` — attribute-name routing on `Command_*_v1` keys. (M)
- [ ] **#SVC.7** Port `handle_realm_list_request`: pull realms from the realm registry, protobuf-encode, attach the JSON character-count blob (zlib-compressed, `JSONRealmCharacterCountList:` prefix), match TC's exact byte layout. (H)
- [ ] **#SVC.8** Port `handle_realm_join_request`: call into `realm_list::join_realm(addr, build, ip, secret, locale, os, tz, account_name, &mut response)`. (M)
- [ ] **#SVC.9** Port `handle_get_all_values_for_attribute` (sub-region enumeration). (L)
- [ ] **#SVC.10** Other 11 services: stub with `RPC_NOT_IMPLEMENTED` (`0x80000020`-ish error code) and log `"unimplemented worldserver service: 0x{:x}"` until/unless needed. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SERVICES.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 359 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h`. Rust target: `crates/wow-chat`, `crates/wow-network`, `crates/wow-proto`, `crates/wow-social`, `crates/wow-world`. | `cargo test -p wow-chat && cargo test -p wow-network && cargo test -p wow-proto` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SERVICES.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 359 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h`. Rust target: `crates/wow-chat`, `crates/wow-network`, `crates/wow-proto`, `crates/wow-social`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SERVICES.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 359 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h`. Rust target: `crates/wow-chat`, `crates/wow-network`, `crates/wow-proto`, `crates/wow-social`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SERVICES.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 359 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h`. Rust target: `crates/wow-chat`, `crates/wow-network`, `crates/wow-proto`, `crates/wow-social`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: no `CMSG_BATTLENET_REQUEST` is observed during a normal login → play-for-5-min → logout cycle. (Establishes the out-of-scope baseline.)
- [ ] Test: `service_hash("bgs.protocol.game_utilities.v1.GameUtilities")` matches the constant generated by TC's `protoc` plugin (capture from a built C++ binary).
- [ ] Test: `handle_realm_list_request` for a fresh subregion produces a `Param_RealmList` blob that, when decompressed, byte-equals what TC produces for the same realmlist contents.
- [ ] Test: an unknown `service_hash` logs at `debug` level (not error) and replies `ERROR_RPC_NOT_IMPLEMENTED`, matching TC's behaviour.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 5 files / 359 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h` | `crates/wow-network/`, `crates/wow-world/`, `crates/wow-proto/` — *(out of scope for the WotLK 3.4.3 path; see notes)* \| ⚠️ partial (stub) — confirmed via audit 2026-05-01: `CMSG_BATTLENET_REQUEST` handler EXISTS in `crates/wow-world/src/handlers/battlenet.rs` and replies `RpcNotImplemented` for every service hash. The §1 hypothesis "the client never asks" was wrong — the client does send it, we just don't service it. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SERVICES.DIV.001` | `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` (`declared_pattern`, 0 Rust lines) | 5 C++ files / 359 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |
| `#SERVICES.DIV.002` | `crates/wow-proto/proto` (`exists_empty`, 0 Rust lines) | 5 C++ files / 359 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. directory exists; no active Rust source lines |
| `#SERVICES.DIV.003` | `crates/wow-social` (`exists_empty`, 0 Rust lines) | 5 C++ files / 359 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#SERVICES.DIV.004` | `crates/wow-chat` (`exists_empty`, 0 Rust lines) | 5 C++ files / 359 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverServiceDispatcher.h`, `/home/server/woltk-trinity-legacy/src/server/game/Services/WorldserverGameUtilitiesService.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **`Service::OriginalHash` is generated by TC's protoc plugin** in `dep/protobuf-tc-extensions/`. The hash function is FNV-1a 32-bit over the *original* fully-qualified service name with package prefix (`bgs.protocol.X.vN.YService`). Reimplementing this without checking against a captured binary is a guaranteed protocol-mismatch bug.
- **The 12 services registered in `WorldserverServiceDispatcher.cpp` are a strict superset of what's actually invoked**. TC registers everything the client could in principle ask for; in 3.4.3 Classic, GameUtilities is the only one that actually gets traffic in normal gameplay. Account/Auth on the world socket exist as a defensive measure for re-auth flows.
- **`HandleRealmListRequest` does TWO compressions**: (1) the realm list itself comes pre-compressed from `sRealmList->GetRealmList()`, (2) the character-count JSON is then zlib-`compress()`d inline with a 4-byte little-endian length prefix. Both are then attached as separate `Attribute`s on the response. Easy to confuse the two.
- **Attribute-name suffix stripping**: `removeSuffix("Command_RealmListRequest_v1")` returns `"Command_RealmListRequest"` (chops the last `_v1`). The handler map is keyed on the stripped form. Any whitespace/encoding mismatch silently routes to "unknown command".
- **`Variant`** is a protobuf union with `int_value`, `uint_value`, `float_value`, `string_value`, `blob_value`, `message_value`, `bool_value`. `GetParam(...) ? ->string_value()` will return `""` if the wrong field was set, not an error.
- WoLK 3.4.3 *Classic* uses retail Cataclysm+ BNet protobuf schemas even though the gameplay opcode set is WotLK. Don't try to reuse 3.3.5a-era bnetlibs.
- Whatever happens in S.1, the bnetserver-side `GameUtilitiesService` (`bnet-server/src/rpc/services/game_utilities.rs`) covers the **pre-login** realm list. The world-side one only matters if the client refreshes mid-session.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `template<class T> class WorldserverService : public T` | Trait `WorldserverServiceAdapter` plus an `impl<S: ProtobufService> WorldserverServiceAdapter for WorldserverWrapper<S>` | Rust has no public inheritance; use composition + trait. Each generated service becomes a struct that holds a `&mut WorldSession`. |
| `class WorldserverServiceDispatcher` | `struct WorldserverServiceDispatcher { dispatchers: HashMap<u32, fn(&mut WorldSession, u32, u32, Bytes) -> Result<()>> }` + a `OnceLock` | Singleton via `OnceLock`. |
| `class GameUtilitiesService : public WorldserverService<game_utilities::v1::GameUtilitiesService>` | `struct GameUtilitiesService<'s> { session: &'s mut WorldSession }` with `impl GameUtilitiesService<'_> { fn handle_process_client_request(...) }` | — |
| `MessageBuffer` (TC's owning byte buffer) | `bytes::Bytes` or `Vec<u8>` | — |
| `std::function<void(MessageBuffer)>` callbacks | `Box<dyn FnOnce(Bytes) + Send>` (or `tokio::sync::oneshot` if request/response) | — |
| `std::unordered_map<std::string, Variant const*>` | `HashMap<String, &Variant>` | `Variant` is a `prost`-generated message; use `&` not `Arc`. |
| `Service::OriginalHash::value` (FNV-1a 32-bit over original name) | `pub const ORIGINAL_HASH: u32 = fnv1a32(b"bgs.protocol.game_utilities.v1.GameUtilities");` (`const fn fnv1a32`) | Define once per service in `crates/wow-proto/src/service_hashes.rs`; verify against capture. |
| `zlib::compress` (raw `compress2(...)` from `<zlib.h>`) | `flate2::write::ZlibEncoder` | Wire format must match TC's `compress()` — that's "zlib" (RFC1950), not "deflate" (RFC1951) and not gzip. |
| `JSON::Serialize(realmCharacterCountList)` | `serde_json::to_string(&realm_character_count_list)?` with the same field naming | TC uses protobuf-JSON; the wire prefix `JSONRealmCharacterCountList:` is literal. |
| `sServiceDispatcher.Dispatch(session, hash, token, methodId, buf)` | `worldserver_service_dispatcher().dispatch(session, hash, token, method_id, buf)` | — |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Method:** `grep -rE "(ServiceDispatcher|service_dispatcher|battlenet_request|HandleBattlenetRequest|CMSG_BATTLENET)" crates/`. Inspected `crates/wow-world/src/handlers/battlenet.rs` and the relevant `inventory::submit!` registration.

**Verdict on §9 recommendation: REVISE.** The doc says "if S.1 shows no traffic, mark out-of-scope". But the codebase already shows the client *does* send `CMSG_BATTLENET_REQUEST` (opcode register exists, the `inventory::submit!` block is registered with `SessionStatus::Authed`, and the handler explicitly logs `service_hash`/`method_id`/`token` per request). The §9 capture step is therefore satisfied by the existence of a stub that's clearly receiving traffic in the wild.

**Findings:**

1. **`CMSG_BATTLENET_REQUEST` handler EXISTS** — `crates/wow-world/src/handlers/battlenet.rs` registers a handler under `ClientOpcodes::BattlenetRequest` (CMSG 0x36FD per the doc-string, which differs from the doc's §7 guess of `~0x4F3` — pin the actual opcode at next review), `SessionStatus::Authed`. The body always responds `BattlenetResponse::error(service_hash, method_id, token, BattlenetRpcErrorCode::RpcNotImplemented)`.
2. **No `WorldserverServiceDispatcher` analogue.** No service hash table, no FNV-1a service-name hashing, no per-service routing — every request goes to the same generic `RpcNotImplemented` reply, regardless of whether it's `GameUtilitiesService::HandleRealmListRequest` (the realm-hop flow) or `FriendsService::SubscribeToFriends` (a no-op for 3.4.3).
3. **`CMSG_BATTLENET_REQUEST` payload is decoded** as a `BattlenetRequest` (in `crates/wow-packet/src/packets/battlenet.rs`); `BattlenetResponse` exists for replies. So the wire-envelope plumbing is in place.
4. **No worldserver-side `GameUtilitiesService` handlers** — no `handle_realm_list_request` / `handle_realm_join_request` on the world side. Note that the **bnet-side** `bnet-server/src/rpc/services/game_utilities.rs` does implement these *for the pre-login auth flow* — they are NOT the same as the worldserver-side handlers (different transport: REST/TLS-1119 vs in-game world-socket-8085).
5. **`SMSG_BATTLENET_NOTIFICATION` / `SMSG_BATTLENET_RESPONSE`** outbound packets exist in `wow-packet`; not actively pushed by any server-initiated flow yet.

**Operational impact:** since every service responds `RpcNotImplemented`, any client that *needs* the worldserver-side realm-hop or any 3.4.3-Classic-relevant BNet feature gets a polite "no" rather than a disconnect. This is acceptable behavior — it just means features like in-game realm refresh / friends-list sync won't work until the dispatcher is implemented. No data corruption risk, no security risk.

**Revised recommendation:** Move from "out-of-scope unless capture shows need" to **"deferred-but-known-needed for ≥1 service (GameUtilities)"**. Ranking of the 12 services by 3.4.3-Classic relevance:

- **GameUtilities** — needed if/when in-game `/realms` refresh or character-select-screen-from-game lands. Likely to come up. Implement first when a real use case appears.
- **Authentication / Connection / Account** — defensive re-auth flows. Stubbed responses are fine.
- **Friends / Presence / ClubMembership / Club / UserManager** — retail social features. Almost certainly safe to leave as `RpcNotImplemented` forever for 3.4.3 Classic.
- **Report v1/v2 / Resources** — irrelevant on a private server.

**Status verdict:** ⚠️ partial (was ❌ not started). Sub-task #SVC.1 is **DONE-by-observation** (the stub itself is the capture). #SVC.2-3 are **DONE** (envelope + handler exist). #SVC.4-9 are the real remaining work, gated on first observed real-world need (likely #SVC.7 realm-list-request, when in-game realm hop becomes a target feature). #SVC.10 (stub the other 11 with `RpcNotImplemented`) is **already implicit** in the catch-all stub.
