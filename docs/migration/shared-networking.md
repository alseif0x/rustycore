# Migration: shared/Networking

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/Networking/`
> **Rust target crate(s):** `crates/wow-network/`
> **Layer:** L1
> **Status:** ⚠️ done (~75%) — wire-protocol parity for the happy path verified; error-path parity is partial
> **Audited vs C++:** ⚠️ audited 2026-05-01 (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Framework genérico de sockets TCP asíncronos basado en CRTP (`Socket<T>`) sobre Boost.ASIO. Provee acceptor (`AsyncAcceptor`), thread pool (`NetworkThread<T>`) y manager singleton (`SocketMgr<T>`) que cualquier socket de aplicación (worldserver, bnetserver, realmserver) parametriza para tener I/O asíncrono uniforme. En Rust se sustituye por Tokio + estructuras propias en `wow-network` (~1716 líneas).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/Networking/AsyncAcceptor.h` | 153 | `prefix` |
| `shared/Networking/Http/BaseHttpSocket.cpp` | 115 | `prefix` |
| `shared/Networking/Http/BaseHttpSocket.h` | 191 | `prefix` |
| `shared/Networking/Http/HttpCommon.h` | 55 | `prefix` |
| `shared/Networking/Http/HttpService.cpp` | 267 | `prefix` |
| `shared/Networking/Http/HttpService.h` | 188 | `prefix` |
| `shared/Networking/Http/HttpSessionState.h` | 35 | `prefix` |
| `shared/Networking/Http/HttpSocket.h` | 75 | `prefix` |
| `shared/Networking/Http/HttpSslSocket.h` | 97 | `prefix` |
| `shared/Networking/NetworkThread.h` | 174 | `prefix` |
| `shared/Networking/Socket.h` | 313 | `prefix` |
| `shared/Networking/SocketMgr.h` | 142 | `prefix` |
| `shared/Networking/SslSocket.h` | 88 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/shared/Networking/Socket.h` | 313 | Template CRTP base; async I/O, read/write queues |
| `src/server/shared/Networking/SocketMgr.h` | 142 | Per-type singleton; acceptor factory + NetworkThread pool |
| `src/server/shared/Networking/AsyncAcceptor.h` | 153 | Acceptor Boost.ASIO con factory callback `OnSocketOpen` |
| `src/server/shared/Networking/NetworkThread.h` | 174 | Pool de threads; per-thread socket storage + update tick |
| `src/server/shared/Networking/SslSocket.h` | 88 | Wrapper SSL (no usado en WoLK; presente para BNet REST) |
| `src/server/shared/Networking/Http/HttpService.h` | ~250 | HTTP service (legacy) |
| `src/server/shared/Networking/Http/HttpSocket.h` | ~250 | HTTP socket subclass |
| `src/server/shared/Networking/Http/BaseHttpSocket.h` | ~250 | HTTP base |
| **TOTAL** | **~1893** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Socket<T, Stream>` | template class (CRTP) | Base de cualquier socket; lifecycle async I/O |
| `SocketMgr<T>` | template class | Singleton-per-tipo; factoría + start/stop |
| `AsyncAcceptor` | class | Boost.ASIO TCP acceptor con factory callback |
| `NetworkThread<T>` | template class | Worker thread; mantiene `_sockets` y `_newSockets` |
| `SslSocket<T>` | template class | Hereda `Socket`; SSL handshake antes de Start |
| `HttpSocket` | class | Especialización HTTP (legacy, no WoLK) |
| `HttpService` | class | HTTP listener + dispatcher |
| `MessageBuffer` | class (en `Packets/`) | Ring buffer I/O |
| `Trinity::Asio::IoContext` | wrapper | Boost.ASIO context |
| `AsyncCallback` | functor | Read handler tipado |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Socket::Start()` | Inicia primer async read | `AsyncRead`, `ReadHandler` |
| `Socket::AsyncRead()` | Encola async_read_some | Boost.ASIO |
| `Socket::AsyncReadWithCallback(cb)` | Read con callback custom | Boost.ASIO |
| `Socket::QueuePacket(buffer)` | Encola write | `AsyncProcessQueue` |
| `Socket::CloseSocket()` | Cierre inmediato + `OnClose` hook | shutdown |
| `Socket::DelayedCloseSocket()` | Marca para cerrar al vaciar write queue | flag |
| `Socket::Update()` | Frame tick (escenarios non-IOCP) | `HandleQueue` |
| `Socket::GetRemoteIpAddress()` | Address remota | — |
| `SocketMgr::StartNetwork(io, bind, port, n)` | Inicia listener + thread pool | `AsyncAcceptor`, `CreateThreads` |
| `SocketMgr::StopNetwork()` | Graceful shutdown | acceptor.Close, threads.Stop/Wait |
| `SocketMgr::OnSocketOpen(sock, idx)` | Factory callback (crea socket de app) | `T::ctor`, `Start` |
| `SocketMgr::SelectThreadWithMinConnections()` | Load balancer round-robin | — |
| `AsyncAcceptor::Bind()` / `Close()` | Bind/Close TCP listener | Boost.ASIO |
| `NetworkThread::Start()` / `Stop()` | Lifecycle thread | atomic flag |
| `NetworkThread::AddSocket(sock)` | Registra socket en thread | `_newSockets` |
| `NetworkThread::Update()` | Iter `_sockets`, llama `Socket::Update` | — |

---

## 5. Module dependencies

**Depends on:**
- `Packets/MessageBuffer.h` — ring buffer
- `Logging.h`, `Errors.h`, `Define.h` — primitivos
- **Boost.ASIO** (TCP, async I/O)
- **OpenSSL** (`SslSocket` only)

**Depended on by:**
- `game/Server/WorldSocket` — hereda `Socket`, usa `WorldPacketCrypt`
- `bnetserver/RealmList/...` — `Socket` para auth
- `HTTPServer` (legacy)

---

## 6. SQL / DB queries

N/A — Networking es infraestructura pura, sin queries propias.

---

## 7. Wire-protocol packets

N/A — `WorldSocket` (en `game/Server`) origina opcodes; Networking solo transporta bytes.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/wow-network/src/world_socket.rs` | `file` | 1 | 1023 | `exists_active` | file exists |
| `crates/wow-network/src/accept.rs` | `file` | 1 | 386 | `exists_active` | file exists |
| `crates/wow-network/src/session_mgr.rs` | `file` | 1 | 188 | `exists_active` | file exists |
| `crates/wow-network/src/lib.rs` | `file` | 1 | 19 | `exists_active` | file exists |
| `crates/wow-network/src/player_registry.rs` | `file` | 1 | 47 | `exists_active` | file exists |
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-network/src/world_socket.rs` — ~1023 líneas — equivalente a `Socket` + `WorldSocket`
- `crates/wow-network/src/accept.rs` — `start_world_listener()` (≈ `AsyncAcceptor`)
- `crates/wow-network/src/session_mgr.rs` — `SessionManager` (≈ `SocketMgr` para session pool)
- `crates/wow-network/src/lib.rs` — re-exports
- `crates/wow-network/src/player_registry.rs` — registry de jugadores online (no equivalente directo C++)

**What's implemented:**
- Async TCP listener (Tokio)
- Per-client `WorldSocket` con FSM (Uninitialized → ConnectionString → AuthChallenge → EncryptedMode)
- Read/write buffer rings + async handlers
- Account DB lookup + session key derivation
- Integración AES-GCM encryption/decryption
- Header encryption con counter monotonic per direction

**What's missing vs C++:**
- SSL socket (no usado en WoLK pero existe en BNet REST)
- HTTP socket (legacy, no scope)
- Dynamic load balancing (Tokio runtime hace algo equivalente pero distinto)
- IOCP optimization Windows (irrelevante con Tokio)
- Graceful drain on shutdown (actual close inmediato)

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `ServerCounter` (líneas históricas 65-81 world_socket.rs): contrastar el pre-set de Rust con `WorldPacketCrypt::EncryptSend/DecryptRecv`; ver la evidencia C++ acotada de §13.
- No tests de header encryption round-trip post-auth.
- No monitor de nonce reuse (potencial RIP AES-GCM).

**Tests existing:**
- Sin tests unitarios formales en `wow-network`. Tests integration probablemente en `world-server`.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_NETWORKING.WBS.001** Cerrar la migracion auditada de `shared/Networking/AsyncAcceptor.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/AsyncAcceptor.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.002** Cerrar la migracion auditada de `shared/Networking/Http/BaseHttpSocket.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.cpp`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.003** Cerrar la migracion auditada de `shared/Networking/Http/BaseHttpSocket.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.004** Cerrar la migracion auditada de `shared/Networking/Http/HttpCommon.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpCommon.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.005** Cerrar la migracion auditada de `shared/Networking/Http/HttpService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.006** Cerrar la migracion auditada de `shared/Networking/Http/HttpService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.007** Cerrar la migracion auditada de `shared/Networking/Http/HttpSessionState.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpSessionState.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.008** Cerrar la migracion auditada de `shared/Networking/Http/HttpSocket.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpSocket.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.009** Cerrar la migracion auditada de `shared/Networking/Http/HttpSslSocket.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpSslSocket.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.010** Cerrar la migracion auditada de `shared/Networking/NetworkThread.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/NetworkThread.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.011** Cerrar la migracion auditada de `shared/Networking/Socket.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.012** Cerrar la migracion auditada de `shared/Networking/SocketMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/SocketMgr.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_NETWORKING.WBS.013** Cerrar la migracion auditada de `shared/Networking/SslSocket.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/SslSocket.h`
  Rust target: `crates/wow-network`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#NET.1** Tests unitarios para `WorldSocket` FSM (5 estados, edge cases). (M)
- [ ] **#NET.2** Audit `ServerCounter`/`ClientCounter` matching exacto con C++. (M)
- [ ] **#NET.3** Regression test: header encryption round-trip post-CMSG_AUTH_SESSION. (M)
- [ ] **#NET.4** Implementar graceful shutdown drain (write queue flush antes close). (H)
- [ ] **#NET.5** Benchmark async read/write Rust vs C++ on same hardware. (H)
- [ ] **#NET.6** Load test: 10k concurrent sockets, memory/CPU profile. (H)
- [ ] **#NET.7** Implementar session timeout (30min idle disconnect). (M)
- [ ] **#NET.8** SOCKS5 proxy support para legacy clients. (M, opcional)
- [ ] **#NET.9** Métricas: per-socket latency, packet rate, crypto timing. (M)
- [ ] **#NET.10** Document buffer sizing heuristics (READ_BLOCK_SIZE 4096 vs Rust tuning). (L)
- [ ] **#NET.11** Verificar nonce counter monotónico, alertar en overflow approach (2^60). (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_NETWORKING.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 13 files / 1893 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h`. Rust target: `crates/world-server`, `crates/wow-network`. | `cargo test -p world-server && cargo test -p wow-network` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_NETWORKING.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 13 files / 1893 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h`. Rust target: `crates/world-server`, `crates/wow-network`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_NETWORKING.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 13 files / 1893 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h`. Rust target: `crates/world-server`, `crates/wow-network`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_NETWORKING.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 13 files / 1893 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h`. Rust target: `crates/world-server`, `crates/wow-network`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Header encryption toggle (pre/post EnterEncryptedMode)
- [ ] Nonce counter monotonic increase per direction
- [ ] Reconnect mismo IP resetea estado correctamente
- [ ] Graceful close drena write queue antes TCP FIN
- [ ] Load balancer distribuye uniforme entre N threads (si aplica)
- [ ] Concurrent connect: 1000 clients en <30s sin race conditions

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 13 files / 1893 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h` | `crates/wow-network/` \| ⚠️ done (~75%) — wire-protocol parity for the happy path verified; error-path parity is partial |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_NETWORKING.DIV.001` | _none generated_ | 13 C++ files / 1893 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Socket.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/HttpService.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Networking/Http/BaseHttpSocket.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **ServerCounter:** el pre-set de Rust requiere contraste con el incremento canónico incluso antes del cifrado; ver §13 y las líneas históricas de `world_socket.rs` L67-81.
2. **Nonce reuse = RIP AES-GCM:** Per-direction counter NUNCA puede colisionar entre direcciones. Counter overflow tras 2^64 causa IV reuse. Mitigación: session timeout << 2^64 packets (en práctica imposible alcanzar).
3. **Boost.ASIO IOCP vs epoll:** C++ multiplexa según OS; Tokio abstrae ambos.
4. **READ_BLOCK_SIZE = 4096:** Razonable; WoW max packet ~65KB world. Ring buffer ≥ 2× max packet.
5. **Sin graceful drain en C++:** Socket cierra inmediato on error. Rust replica el comportamiento C++; si se quiere mejor, es divergencia consciente.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `Socket<T, Stream>` (CRTP) | `WorldSocket` struct + Tokio TcpStream | Pattern divergente; mismo resultado |
| `SocketMgr<T>` | `SessionManager` (en `session_mgr.rs`) | Singleton session tracker |
| `AsyncAcceptor` | `start_world_listener()` (en `accept.rs`) | Tokio TcpListener |
| `NetworkThread<T>` | Tokio runtime + tasks | Async executor reemplaza OS threads |
| `Socket::AsyncRead()` | `TcpStream::read()` + poll loop | Tokio nativo |
| `Socket::QueuePacket()` | `flume::Sender<Vec<u8>>` | Async MPMC channel |
| `Socket::Update()` | (no necesario) | Tokio event-driven |
| `MessageBuffer` | `BytesMut` (crate `bytes`) | Same ring semantics |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

### 13.1 Audit summary

The Rust world-socket replicates TrinityCore's wire protocol on the **happy path**
with high fidelity: connection-string handshake bytes are byte-identical
(`b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"`, 53 bytes), all six
crypto seeds (`AuthCheckSeed`, `SessionKeySeed`, `ContinuedSessionSeed`,
`EncryptionKeySeed`, `EnableEncryptionSeed`, `EnableEncryptionContext`) match TC
exactly, the AES-128-GCM nonce layout matches (`[u64 counter LE | u32 suffix LE]`
with `"SRVR"`/`"CLNT"` magic), the 12-byte tag size matches the WoW protocol
deviation from standard GCM, and the `EnterEncryptedMode` Ed25519ctx signing
flow matches.

The previously suspicious `ServerCounter`/`ClientCounter` pre-set logic is in
fact **correct in this recorded contrast** — it was a faithful port of
TC's `WorldPacketCrypt` semantics. Counter behaviour is now fully justified
(see §13.3).

The audit found three real divergences (all on error/shutdown paths, none on
the success path), one minor-defensive gap (`PeekDecryptRecv` for malformed
header logging), and confirmed that the prior status of `~80%` overstates
robustness on adversarial inputs but is honest for the well-behaved-client
case. Status is now `~75%` (downgraded one notch to flag the missing error
flush; happy path is unchanged).

The previous doc note "**`✅ done (~80%)`**" — with most of the missing 20%
being SSL, HTTP, and Windows IOCP irrelevances — was over-confident on
graceful-shutdown parity but otherwise honest. Re-read as `~75%`.

### 13.2 FSM state-by-state comparison

| Step | TC C++ (`WorldSocket.cpp`) | Rust (`world_socket.rs`) | Verdict |
|---|---|---|---|
| 0. Accept | `AsyncAcceptor` factory creates `WorldSocket(socket)` ; `_serverChallenge` is filled with `Crypto::GetRandomBytes` (L64) | `start_world_listener` accept loop spawns task ; `WorldSocket::new` fills challenge with `rand::thread_rng().fill` (L199-200) | ✅ same |
| 1. IP-ban check | `Start()` → async DB `LOGIN_SEL_IP_INFO` → `CheckIpCallback`; ban → `DelayedCloseSocket()` (L82-110) | **No IP-ban check before the connection initializer.** Rust enters the handshake unconditionally (`socket.start()` L246) | ❌ divergent — see §13.4 finding F1 |
| 2. Send `ServerConnectionInitialize` | Raw `MessageBuffer initializer; QueuePacket(...)` — bypasses `WritePacketToBuffer` and `EncryptSend`, **counter NOT incremented** (L116-122) | Raw `stream.write_all(SERVER_CONNECTION_INIT).await` — bypasses `send_unencrypted_packet`, **counter NOT incremented** (L250) | ✅ same |
| 3. Read `ClientConnectionInitialize` | `AsyncReadWithCallback(InitializeHandler)` — bypasses `DecryptRecv`, **counter NOT incremented** (L114, L124-200) | `stream.read_exact(&mut buf).await` — bypasses `read_unencrypted_packet`, **counter NOT incremented** (L255-256) | ✅ same |
| 4. Validate init | Parses `ClientConnectionInitialize` + `\n` terminator; mismatch → `CloseSocket()` immediate (L156-176) | Compares full 53-byte slice incl. `\n`; mismatch → `Err(InvalidConnectionString)` → drop = immediate close (L258-261) | ✅ same |
| 5. Init zlib | `deflateInit2(... CONFIG_COMPRESSION ...)` (L184) | zlib state owned by `compression::PacketCompressor` (init lazy in `SocketWriter`) | ✅ semantically same |
| 6. Send `SMSG_AUTH_CHALLENGE` | `HandleSendAuthSession` → `SendPacket` → enqueue `EncryptablePacket(packet, _authCrypt.IsInitialized() == false)` → `WritePacketToBuffer` → `EncryptSend` with `_initialized=false` → tag is memset(0), `_serverCounter ++ → 1` (L242-250, L535, L577, L71-83) | `send_auth_challenge` → `send_unencrypted_packet` → writes `[size, tag=[0;12]] || data`, `unencrypted_packets_sent ++ → 1` (L271-285, L445-461) | ✅ same — both write tag=0 and bump counter to 1 |
| 7. Read `CMSG_AUTH_SESSION` | `ReadHandler` loop → `ReadHeaderHandler` (size check) → `ReadDataHandler` → `_authCrypt.DecryptRecv` with `_initialized=false` → memset(tag,0) AND `_clientCounter ++ → 1` (L260-319, L353-413, L56-69) | `authenticate` → `read_unencrypted_packet` → reads 16-byte header, then `header.size` bytes raw, `unencrypted_packets_received ++ → 1` (L555-572, L511-526) | ✅ same — both bump counter to 1 |
| 8. DB lookup → digest validation | Async `LOGIN_SEL_ACCOUNT_INFO_BY_NAME` callback → `HandleAuthSessionCallback` → SHA256(KeyData‖Win64AuthSeed) → HMAC-SHA256(localChallenge‖serverChallenge‖AuthCheckSeed) → memcmp digest[..24] (L685-731) | Sync `account_lookup.lookup_account` future → `handle_auth_session` → SHA256(key_data‖platform_seed) → HMAC-SHA256(local_challenge‖server_challenge‖AUTH_CHECK_SEED) → `server_digest[..24] != auth_session.digest` (L580-588, L296-345) | ✅ same algorithm, same byte order |
| 9. Derive `_sessionKey` | SHA256(KeyData) → HMAC(serverChallenge‖localChallenge‖SessionKeySeed) → SessionKeyGenerator(SHA256, 40 bytes) (L733-744) | SHA256(key_data) → HMAC(server_challenge‖local_challenge‖SESSION_KEY_SEED) → `SessionKeyGenerator256.generate(40)` (L349-367) | ✅ same |
| 10. Derive `_encryptKey` | HMAC(_sessionKey, localChallenge‖serverChallenge‖EncryptionKeySeed)[..16] (L746-753) | HMAC(session_key, local_challenge‖server_challenge‖ENCRYPTION_KEY_SEED)[..16] (L370-379) | ✅ same |
| 11. WorldSession + Warden + RBAC | Constructs `WorldSession(...)`, optional `InitWarden(_sessionKey)`, async `LoadPermissionsAsync` callback (L884-896) | No Warden, no RBAC; account info just stored → callback into session-creation closure later (L383-388) | ⚠️ scope-divergent (Warden/RBAC out of scope; documented under §13.5 #NET.12) |
| 12. Send `SMSG_ENTER_ENCRYPTED_MODE` | `LoadSessionPermissionsCallback` → `SendPacketAndLogOpcode(EnterEncryptedMode(_encryptKey, true))` → still `_authCrypt.IsInitialized()==false` → tag=0, `_serverCounter ++ → 2` (L899-905) | `send_enter_encrypted_mode` → `send_unencrypted_packet`, `unencrypted_packets_sent ++ → 2` (L392-407) | ✅ same — both reach server_counter=2 pre-Init |
| 13. Read `CMSG_ENTER_ENCRYPTED_MODE_ACK` | `ReadDataHandler` case → `DecryptRecv` with `_initialized=false` → tag=0, `_clientCounter ++ → 2` → `HandleEnterEncryptedModeAck` (L467-470, L1014-1021) | `authenticate` second `read_unencrypted_packet` → opcode check → `handle_enter_encrypted_mode_ack`, `unencrypted_packets_received ++ → 2` (L594-606) | ✅ same — both reach client_counter=2 pre-Init |
| 14. Init AES | `_authCrypt.Init(_encryptKey)` — sets `_initialized=true`. Counters preserved at (server=2, client=2). `sWorld->AddSession(_worldSession)` (L1014-1021) | `WorldCrypt::new_with_server_counter(key, 2)` and `new_with_client_counter(key, 2)` constructed in `split_for_io` (L711-731) | ✅ same — both arrive at (2, 2) starting nonce |
| 15. First encrypted packet (S→C) | Next `SendPacket` → `EncryptablePacket(_, true)` → `EncryptSend` with `_serverCounter=2` → real GCM. Counter post-call = 3 | First `SocketWriter::write_encrypted` → `WorldCrypt::encrypt` with `server_counter=2` → real GCM. Counter post-call = 3 | ✅ identical nonce sequence |
| 16. First encrypted packet (C→S) | Next `ReadDataHandler` → `DecryptRecv` with `_clientCounter=2` → real GCM verify. Counter post-call = 3 | `SocketReader::run` → `WorldCrypt::decrypt` with `client_counter=2` → real GCM verify. Counter post-call = 3 | ✅ identical nonce sequence |

**FSM verdict:** the four-state Rust enum
(`Uninitialized → ConnectionStringSent → AuthChallengeSent → AuthSessionReceived → EncryptedModeEnabled`)
matches the implicit C++ FSM that's spread across `_authed`, `_authCrypt._initialized`,
and the `_packetBuffer`/`_headerBuffer` state. **No state-transition divergence on
the success path.**

### 13.3 ServerCounter / ClientCounter analysis — verdict

**The "ServerCounter quirk" hypothesis was wrong about it being a quirk.** Both
counters increment **unconditionally on every call to EncryptSend/DecryptRecv**,
including pre-Init. This is a deliberate TC design (`WorldPacketCrypt.cpp`
L67, L82) and the WoW client matches it: the client increments its own send
and receive counters in `Encrypt()`/`Decrypt()` regardless of init state.

| Counter event | TC `WorldPacketCrypt` | Rust pre-Init tracking | Rust crypto Init |
|---|---|---|---|
| `WorldSocket::WorldSocket()` | `_serverCounter=0, _clientCounter=0, _initialized=false` (L22) | `unencrypted_packets_sent=0, unencrypted_packets_received=0` (L213-214) | crypt = `None` |
| Pre-Init `EncryptSend` (S→C unencrypted) | `tag=0; ++_serverCounter` (L80-82) | `send_unencrypted_packet` writes tag=0; `unencrypted_packets_sent ++` (L452-460) | n/a |
| Pre-Init `DecryptRecv` (C→S unencrypted) | `tag=0; ++_clientCounter` (L65-68) | `read_unencrypted_packet` reads (and ignores) tag; `unencrypted_packets_received ++` (L515-523) | n/a |
| Init point | `_initialized=true`, counters preserved (L26-31) | `split_for_io` constructs `WorldCrypt::new_with_server_counter(k, n_sent)` + `new_with_client_counter(k, n_recv)` (L711-731) | crypt = `Some(WorldCrypt)` |
| Post-Init `EncryptSend` | real GCM with nonce(`_serverCounter`, "SRVR"); `++_serverCounter` (L73-83) | `WorldCrypt::encrypt` with nonce(`server_counter`, `SERVER_SUFFIX=0x52565253`); `server_counter ++` (L113-136) | ✅ |
| Post-Init `DecryptRecv` | real GCM with nonce(`_clientCounter`, "CLNT"); `++_clientCounter` (L60-69) | `WorldCrypt::decrypt` with nonce(`client_counter`, `CLIENT_SUFFIX=0x544E4C43`); `client_counter ++` (L146-172) | ✅ |

The Rust split between two-stage tracking (`unencrypted_packets_*` counters
on `WorldSocket`, then promoted into `WorldCrypt` at `split_for_io`) is a
structural divergence (TC has a single counter that's just not yet used
cryptographically) but is **algebraically equivalent**. After both servers
process the same packets, the AES-GCM nonce values used for the first
real-encrypted packet are byte-identical: `[02 00 00 00 00 00 00 00 53 52 56 52]`
for S→C and `[02 00 00 00 00 00 00 00 43 4C 4E 54]` for C→S.

**Verdict on the doc's pre-audit hypothesis:** Rust did NOT over-correct.
The pre-set counters are necessary for protocol parity. The previous comment
in `world_socket.rs` (L184-193) — `"The WoW client increments its receive
counter for every header it reads, even for unencrypted packets"` — is
literally what TC also does, just on the server side. Note that the comment
needs an exact `WorldPacketCrypt` anchor. The recorded contrast found no
behavior defect on this point; it is not a new audit of current networking.

### 13.4 Critical findings

#### F1 — Missing IP-ban pre-check (low severity, scope)
- **C++:** `WorldSocket::Start` (`WorldSocket.cpp` L82-89) issues `LOGIN_SEL_IP_INFO`
  asynchronously *before* sending the server connection string; banned IPs get
  `DelayedCloseSocket()` and never receive `ServerConnectionInitialize`.
- **Rust:** `start_world_listener` (`accept.rs` L108-115) calls `socket.start()`
  unconditionally; banned IPs receive `ServerConnectionInitialize` and may
  attempt full auth before being rejected (or, currently, not rejected at all
  — there's no IP-ban table query anywhere in the realm-accept path).
- **Impact:** Banned-IP enforcement is silently disabled. Not a wire-protocol
  divergence, but a security/parity gap.

#### F2 — No `SendAuthResponseError` flush before close (medium severity, UX)
- **C++:** Auth-failure paths (`HandleAuthSessionCallback`) call
  `SendAuthResponseError(ERROR_DENIED)` (L781, L789, L800, L814, L826, L849,
  L861) **before** `DelayedCloseSocket()`. The error packet is queued; the
  next `Update()` flushes it, then the TCP connection drains the write queue
  and closes. The client UI shows the specific error code.
- **Rust:** Auth-failure paths (`world_socket.rs` L294-345, `accept.rs`
  L113-115, L118-121) return `Err(WorldSocketError::AuthFailed(...))` and let
  the task drop the `TcpStream`. The client sees a TCP RST/FIN with no in-band
  error code and falls back to a generic "Disconnected from server" message.
- **Impact:** Player-facing UX divergence; not a protocol-violation per se
  (the wire frames the client sees up to that point are valid), but breaks
  parity with TC's diagnostics. Captured under #NET.13.

#### F3 — No `DelayedCloseSocket` (a.k.a. graceful drain) on the realm path (medium severity, UX)
- **C++:** `Socket::DelayedCloseSocket` sets `_closing=true`. `Update()` keeps
  draining `_writeQueue` until empty, then `CloseSocket()` runs (`Socket.h`
  L150-167). Used everywhere on the auth-error path so the last server packet
  reaches the client before TCP FIN.
- **Rust:** No equivalent. `SocketWriter::run` exits when *all senders are
  dropped*, which guarantees inflight packets in the channel have been picked
  up — but the writer task is fully decoupled from the reader, so an error
  on the reader side immediately drops the `OwnedReadHalf` and (because the
  whole `WorldSocket` was consumed by `split_for_io`) the writer continues
  running on its own. **There is no path that signals the writer to drain
  and then close on read errors.** This actually means the writer keeps
  blocking on `send_rx.recv_async` forever once the reader exits, until the
  session task itself drops `send_tx`. In practice writes do drain, but
  asymmetrically and only because `flume::Sender` drop happens to propagate.
  This is a fragile parity rather than a real graceful-shutdown design.
- **Impact:** §11 gotcha #5 ("C++ closes immediately on error, Rust replicates")
  is **inaccurate**. C++ uses `DelayedCloseSocket` *more* often than
  `CloseSocket`; the realm auth path is mostly delayed-close. Doc fix needed.

#### F4 — `PeekDecryptRecv` not implemented (low severity, logging only)
- **C++:** `WorldSocket::ReadHeaderHandler` (L335-346) — when a header has an
  invalid size, TC peeks (no integrity check) at the encrypted opcode to log
  it for diagnostics, and explicitly allows oversize `CMSG_HOTFIX_REQUEST` once.
- **Rust:** `read_encrypted_packet` (L529-548) and `SocketReader::run` (L766-832)
  return an `InvalidSize` error and close on any size > MAX. No oversize
  hotfix request allowance.
- **Impact:** A client requesting hotfixes legitimately may get disconnected
  if the hotfix payload exceeds the standard cap. Minor for current
  hotfix-stub state; a real concern once hotfix delivery is implemented.

#### F5 — `unencrypted_packets_*` framing comment lacks its canonical anchor (cosmetic)
- **Rust:** comments at `world_socket.rs` L184-193, L709-728, and `world_crypt.rs`
  L67-96 do not cite the relevant canonical behavior. The source is
  TC's `WorldPacketCrypt::EncryptSend/DecryptRecv` (always increment).
- **Impact:** Doc-only. Fix in the same pass as #NET.2.

#### F6 — `SocketReader` vs `SocketWriter` use independent `WorldCrypt` instances (informational, not a defect)
- **C++:** Single `WorldPacketCrypt _authCrypt` instance with both
  `_clientCounter` and `_serverCounter` (`WorldSocket.h` L155).
- **Rust:** `split_for_io` (L693-738) builds **two independent `WorldCrypt`
  instances** — one for the reader (with `client_counter` set), one for the
  writer (with `server_counter` set). Each cipher is fed the same key so
  AES-128 round keys are duplicated.
- **Impact:** Memory cost negligible (`WowAesGcm` ~ a few hundred bytes).
  Functionally equivalent because the two counters in TC are also independent
  state — only the suffix bytes (`SRVR`/`CLNT`) and the directionality differ,
  and Rust enforces this by construction. Noted for completeness; no action.

#### F7 — Nonce monotonicity / reuse audit
- TC: each direction has a `u64` counter, incremented after every `EncryptSend` /
  `DecryptRecv`. Suffix bytes (`"SRVR"`/`"CLNT"`) cannot collide between
  directions. No reset path. Same key for both directions, but the AAD-empty
  GCM nonce uniqueness is guaranteed by `(counter, suffix)` uniqueness.
- Rust: same structure — `WorldCrypt::server_counter` and `client_counter`
  are `u64`, incremented in `encrypt` (L134) and `decrypt` (L170) only. The
  `new_with_*_counter` constructors take a *positive offset* and never reset
  to a smaller value. No code path resets a counter mid-session. The
  `set_encrypt_key` re-seed (L423-425) only runs **before** `WorldCrypt` is
  constructed (encrypt_key holder, not the cipher itself).
- **Verdict:** ✅ no (key, nonce) reuse possible under non-pathological
  control flow. Overflow at 2^64 is unreachable in practice (>10^19 packets
  would require >10^11 seconds at 100M packets/sec). One residual risk:
  **a `WorldSocket` reused for a second authenticate() round would re-initialize
  counters at (n, m) for the new key** — this is fine because it's a *new key*,
  so nonce reuse can't occur. Confirmed safe.

### 13.5 Recommended sub-tasks (#NET.X) — priority shuffle

Re-prioritised after this audit. Existing tasks #NET.1 through #NET.11
remain valid; the relative ordering changes and three new items are added.

**Priority H (do first):**
- [ ] **#NET.4** Implement graceful drain on auth-error and read-error paths
  (mirror C++ `DelayedCloseSocket`). Currently writer keeps running after
  reader exits with no explicit drain signal — fix by having `SocketReader`
  drop a sentinel into `send_rx` or close `send_rx` explicitly. **Promoted
  from previous H.** *(see F3)*
- [ ] **#NET.13** *(new)* Send `SMSG_AUTH_RESPONSE` with the appropriate
  `ERROR_*` code on every auth-failure path before closing the socket. Match
  TC's `SendAuthResponseError(code)` callsites. Drives the client UI to show
  the correct message instead of generic "Disconnected". *(see F2)*
- [ ] **#NET.3** Regression test: header encryption round-trip
  post-`CMSG_AUTH_SESSION` + post-`CMSG_ENTER_ENCRYPTED_MODE_ACK`. Pin the
  exact pre-Init counter values (server=2, client=2) and the exact GCM nonces
  used for the first encrypted packet in each direction. **Critical because
  any future refactor of `unencrypted_packets_sent/received` will silently
  break this if no test exists.** *(promoted from M)*
- [ ] **#NET.14** *(new)* Add an integration test that mocks an entire client
  handshake against `WorldSocket` (raw bytes in, raw bytes out) and asserts
  the byte-exact server output for steps 2, 6, 12. Catches regressions in
  any of the seeds, in the connection-string literal, or in the pre-Init
  tag handling. *(promoted from spirit of #NET.1)*

**Priority M:**
- [ ] **#NET.2** Reword the counter-increment doc comments
  (`world_socket.rs` L184-193, `world_crypt.rs` L67-96) to attribute the
  behaviour to TC's `WorldPacketCrypt::EncryptSend/DecryptRecv` (always
  increment). *(see F5; demoted from M to L now that the
  semantics are confirmed correct.)*
- [ ] **#NET.7** Session timeout (30min idle) — unchanged.
- [ ] **#NET.9** Per-socket metrics (latency, packet rate, crypto timing) —
  unchanged.
- [ ] **#NET.15** *(new)* IP-ban pre-check before sending
  `ServerConnectionInitialize`, mirroring `WorldSocket::Start` →
  `LOGIN_SEL_IP_INFO` → `CheckIpCallback`. *(see F1)*
- [ ] **#NET.1** Unit tests for `WorldSocket` FSM — split into "happy path"
  (covered by #NET.14) and "error transitions" (each `SocketState` →
  `WorldSocketError` mapping). *(unchanged.)*

**Priority L (cosmetic / longer-term):**
- [ ] **#NET.10** Document buffer sizing — unchanged.
- [ ] **#NET.11** Verify nonce counter monotonic; alert on overflow approach.
  Confirmed safe today (see F7); leave as documentation-only TODO.
- [ ] **#NET.16** *(new)* Allow oversize `CMSG_HOTFIX_REQUEST` on the
  realm socket (one-shot, mirror TC's `_canRequestHotfixes` flag). Only
  becomes relevant once hotfix delivery is wired up. *(see F4)*
- [ ] **#NET.5** Benchmark Rust vs C++ — unchanged, blocked on a stable
  C++ build of the legacy server.
- [ ] **#NET.6** 10k-socket load test — unchanged.
- [ ] **#NET.8** SOCKS5 proxy support — unchanged, optional.

### 13.6 Summary verdict

| Aspect | Status |
|---|---|
| Connection-string literal bytes | ✅ exact match |
| Crypto seeds (6 of 6) | ✅ exact match |
| AES-128-GCM nonce layout (counter‖suffix) | ✅ exact match |
| 12-byte tag size | ✅ matches WoW protocol deviation |
| Pre-Init counter increment semantics | ✅ exact match (TC also always-increments) |
| Post-Init starting nonces (2, 2) | ✅ exact match |
| Ed25519ctx signing flow | ✅ exact match |
| FSM happy-path transitions | ✅ exact match |
| `SMSG_AUTH_RESPONSE` error flush | ❌ missing (#NET.13) |
| `DelayedCloseSocket` drain | ❌ missing (#NET.4) |
| IP-ban pre-check | ❌ missing (#NET.15) |
| `PeekDecryptRecv` malformed-header logging | ⚠️ stub only (#NET.16) |
| Nonce reuse safety | ✅ verified safe |

The pre-audit doc claim of `✅ done (~80%)` is **partially honest**: the wire
protocol and crypto layer are at parity for well-behaved clients; error-path
parity (UX-affecting) is at maybe 40%. Net status downgraded to `~75%` to
flag the missing flush.
