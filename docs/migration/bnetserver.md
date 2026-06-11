# Migration: bnetserver (Battle.net auth server binary)

> **C++ canonical path:** `src/server/bnetserver/`
> **Rust target crate(s):** `crates/bnet-server/`
> **Layer:** binary (executable entry point)
> **Status:** ⚠️ partial (login flow works; missing freeze detector, IP-Location DB, soap, win32 service, multi-thread io_context, some REST endpoints, SecretMgr, DB keepalive)
> **Audited vs C++:** ⚠️ audited (2026-05-01) — see §13
> **Last updated:** 2026-05-01

---

## 1. Purpose

The standalone Battle.net authentication daemon. Listens on **port 1119** (BNet RPC, TLS, binary protobuf) and **port 8081** (`LoginREST`, HTTPS, JSON for the launcher's "log in" web form). Owns: BNet account login (SRPv1/v2), session-key generation, realm-list serving, IP-bans housekeeping. Independent of the world-server (different process; only shares the `auth` MariaDB).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `bnetserver/Main.cpp` | 432 | `prefix` |
| `bnetserver/REST/LoginHttpSession.cpp` | 118 | `prefix` |
| `bnetserver/REST/LoginHttpSession.h` | 61 | `prefix` |
| `bnetserver/REST/LoginRESTService.cpp` | 825 | `prefix` |
| `bnetserver/REST/LoginRESTService.h` | 96 | `prefix` |
| `bnetserver/Server/Session.cpp` | 842 | `prefix` |
| `bnetserver/Server/Session.h` | 191 | `prefix` |
| `bnetserver/Server/SessionManager.cpp` | 45 | `prefix` |
| `bnetserver/Server/SessionManager.h` | 45 | `prefix` |
| `bnetserver/Server/SslContext.cpp` | 66 | `prefix` |
| `bnetserver/Server/SslContext.h` | 34 | `prefix` |
| `bnetserver/Services/AccountService.cpp` | 32 | `prefix` |
| `bnetserver/Services/AccountService.h` | 43 | `prefix` |
| `bnetserver/Services/AuthenticationService.cpp` | 37 | `prefix` |
| `bnetserver/Services/AuthenticationService.h` | 44 | `prefix` |
| `bnetserver/Services/ConnectionService.cpp` | 55 | `prefix` |
| `bnetserver/Services/ConnectionService.h` | 44 | `prefix` |
| `bnetserver/Services/GameUtilitiesService.cpp` | 32 | `prefix` |
| `bnetserver/Services/GameUtilitiesService.h` | 43 | `prefix` |
| `bnetserver/Services/Service.h` | 45 | `prefix` |
| `bnetserver/Services/ServiceDispatcher.cpp` | 49 | `prefix` |
| `bnetserver/Services/ServiceDispatcher.h` | 72 | `prefix` |
| `bnetserver/resource.h` | 15 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/src/server/bnetserver/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `Main.cpp` | 432 | Entry point, config load, OpenSSL + protobuf init, DB pool, `boost::asio::io_context.run()`, signal handler, db-keepalive timer, ban-expiry timer, optional Win32 service mode |
| `REST/LoginRESTService.h` | 96 | `LoginRESTService` (singleton), HTTP handler registration |
| `REST/LoginRESTService.cpp` | 1100 | All login REST endpoints; SRP challenge/response; legacy password migration; portal redirects |
| `REST/LoginHttpSession.h` / `.cpp` | 50 + 100 | Per-connection HTTP session state (TLS, ban tracking) |
| `Server/Session.h` / `.cpp` | 200 + 1000 | `Battlenet::Session` — the **TLS RPC session** on port 1119: protobuf framing, BNet packet pump, RPC dispatch |
| `Server/SessionManager.h` / `.cpp` | 47 + 50 | `SocketMgr<Session>` specialization, `OnSocketAccept` |
| `Server/SslContext.h` / `.cpp` | 32 + 70 | Boost.Asio `ssl::context` initialization (loads `bnetserver.cert.pem` + `bnetserver.key.pem`) |
| `Services/Service.h` | 60 | Base for the per-session generated protobuf services |
| `Services/AccountService.h` / `.cpp` | 50 + 50 | `bgs::protocol::account::v1::AccountService` server impl (only stubs in TC) |
| `Services/AuthenticationService.h` / `.cpp` | 50 + 60 | `bgs::protocol::authentication::v1::AuthenticationService` — `Logon` ↔ `LogonResult` flow, ConnectTo |
| `Services/ConnectionService.h` / `.cpp` | 50 + 60 | `bgs::protocol::connection::v1::ConnectionService` — `Connect / Bind / Echo / KeepAlive` |
| `Services/GameUtilitiesService.h` / `.cpp` | 50 + 60 | `RealmListTicketRequest` / `JoinRealmRequest` (similar to worldserver's, but pre-login) |
| `Services/ServiceDispatcher.h` / `.cpp` | 70 + 70 | Per-session RPC dispatcher (mirrors `WorldserverServiceDispatcher` but routes to `Session*` not `WorldSession*`) |
| `bnetserver.conf.dist` | — | Default config (BindIP, BattlenetPort, LoginREST.Port, paths, MySQL, MaxPingTime, BanExpiryCheckInterval, …) |
| `bnetserver.cert.pem` / `bnetserver.key.pem` | — | Default development TLS keypair (replaced in prod) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `LoginRESTService` | class : HttpService<LoginHttpSession> (singleton `sLoginService`) | HTTPS login server (port 8081). Boost.Beast underneath. |
| `LoginHttpSession` | class : Trinity::Net::Http::SessionState | Per-connection HTTP/SRP state (challenge, public ephemeral B, expected M1) |
| `Battlenet::Session` | class : SocketMgr<Session>::SocketType | Per-client TLS BNet RPC session on 1119; owns `ServiceDispatcher` |
| `Battlenet::SessionManager` | class : SocketMgr<Session> (singleton `sSessionMgr`) | Pool of `Session` instances, listener, accept loop |
| `Battlenet::SslContext` | static | One-shot `ssl::context` factory + `instance()` accessor |
| `Battlenet::ServiceDispatcher` | class | Maps `serviceHash → ServiceMethod` for one session |
| `Battlenet::Services::AccountService / AuthenticationService / ConnectionService / GameUtilitiesService` | class | Per-service handlers |
| `SrpVersion` | enum class : int8 | `v1 = 1`, `v2 = 2` |
| `SrpHashFunction` | enum class | `Sha256 = 0`, `Sha512 = 1` |
| `BanMode` | enum | `BAN_IP = 0`, `BAN_ACCOUNT = 1` |
| `m_ServiceStatus` | global int | Win32 service mode: `-1`=non-service, `0`=stopped, `1`=running, `2`=paused |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `main(int argc, char** argv)` | Bootstrap: parse args, load config, init protobuf/openssl/SSL ctx/DB/SecretMgr/IPLocation, start `LoginRESTService`, start `SessionManager`, register signal/db-ping/ban-expiry timers, run `io_context` | All of below |
| `StartDB()` / `StopDB()` | Open/close `LoginDatabase` only (bnetserver does NOT touch characters/world/hotfix DBs) | DatabaseLoader |
| `SignalHandler(weak<IoContext>, error, signum)` | On SIGINT/SIGTERM (and SIGBREAK on Windows): call `ioContext->stop()` | — |
| `KeepDatabaseAliveHandler(weak<DeadlineTimer>, intervalMin, error)` | Periodic `LoginDatabase.KeepAlive()` then re-arm timer (interval = `MaxPingTime` minutes, default 30) | LoginDatabase |
| `BanExpiryHandler(weak<DeadlineTimer>, intervalSec, error)` | Periodic `DEL_EXPIRED_IP_BANS`, `UPD_EXPIRED_ACCOUNT_BANS`, `DEL_BNET_EXPIRED_ACCOUNT_BANNED` (interval = `BanExpiryCheckInterval` seconds, default 60) | LoginDatabase |
| `LoginRESTService::StartNetwork(io, ip, port)` | Resolves external+local hostnames, registers HTTP handlers (GET `/bnetserver/login/`, GET `/bnetserver/gameAccounts/`, GET `/bnetserver/portal/`, POST `/bnetserver/login/`, POST `/bnetserver/login/srp/`, POST `/login/srp/`, POST `/login/`, POST `/bnetserver/refreshLoginTicket/`), calls base `HttpService::StartNetwork` | DNS resolver, base http |
| `LoginRESTService::HandleGetForm(...)` | Returns the JSON `FormInputs` describing the login form (account_name, password, captcha?) | `_formInputs` |
| `LoginRESTService::HandlePostLogin(...)` | First-stage login: receive credentials, look up account, check ban, issue SRP challenge OR fall through to legacy password migration | `BattlenetAccountMgr`, `Crypto::SRP6` |
| `LoginRESTService::HandlePostLoginSrpChallenge(...)` | Second-stage: client sends `A` + `M1`, server verifies and replies with `LoginResult` (login ticket = `TC-<hex>`) | `BnetSRP6{v1,v2}` |
| `LoginRESTService::HandlePostBotSrpChallenge / HandlePostBotLogin` | Same flow at `/login/srp/` and `/login/` (mobile / "bot" client paths) | — |
| `LoginRESTService::HandlePostRefreshLoginTicket(...)` | Refresh an unexpired ticket, extending `loginTicketExpiry` by `LoginREST.TicketDuration` seconds | LoginDatabase |
| `LoginRESTService::HandleGetGameAccounts(...)` | Returns the list of game accounts (`<bnetId>#1`, `#2`, …) linked to the BNet account from the ticket | LoginDatabase |
| `LoginRESTService::HandleGetPortal(...)` | Returns "portal address" = `BattlenetPort` host:port for the next stage | config |
| `LoginRESTService::MigrateLegacyPasswordHashes() const` | One-shot upgrade for accounts still on SRPv1 → v2 | SRP6 |
| `Battlenet::Session::Start()` / `OnRead()` / `HandleAuthChallenge` / `HandleProtoMessage` | TLS handshake, framing, dispatch via `ServiceDispatcher` | dispatcher |
| `SessionManager::StartNetwork(io, ip, port)` / `OnSocketAccept(...)` | Boost acceptor on port 1119; on accept, allocate `Session`, hand it to `NetworkThread` | base |
| `SslContext::Initialize()` / `instance()` | Load cert+key once; expose context to `Session`s | OpenSSL |

---

## 5. Module dependencies

**Depends on:**
- `boost::asio` — `io_context`, `signal_set`, `deadline_timer`, `ssl::context`, `tcp::acceptor`
- `boost::beast` — HTTP/1.1 parser/serializer for `LoginRESTService`
- `boost::program_options` — CLI flags
- `boost::dll` — locate the running binary (for OpenSSL thread setup)
- `boost::filesystem` — config paths
- `OpenSSL` — TLS, crypto primitives, PRNG seed
- `protobuf` — BNet RPC messages
- `rapidjson` — REST JSON serialization (in addition to protobuf-JSON for the form schema)
- TC `common` libs: `Banner`, `Config`, `Log` (+ `AppenderDB`), `DatabaseLoader`, `LoginDatabase`, `MySQLThreading`, `OpenSSLCrypto`, `SecretMgr` (`SECRET_OWNER_BNETSERVER`), `IPLocation`, `IpNetwork::ScanLocalNetworks`, `Trinity::Asio::*`, `Trinity::Net::Http::*`, `Locale::Init`, `BigNumber`, `ProcessPriority`, `RealmList`
- `Battlenet::AccountMgr` (lives in game module, but bnetserver links against it for `CreateBattlenetAccount` flows in the legacy migrator)
- The protobuf-generated `*_service.pb.h` set: `account`, `authentication`, `connection`, `game_utilities`, `login`, `error_codes`

**Depended on by:**
- The WoW client (3.4.3 Classic launcher) — only consumer
- The world-server reads the `account.session_key_bnet` row that bnetserver writes; that's the only inter-process coupling

---

## 6. SQL / DB queries (if any)

bnetserver only touches the **`auth`** DB. Statements live in `LoginDatabase.cpp`.

| Statement | Purpose |
|---|---|
| `LOGIN_SEL_BNET_AUTHENTICATION` | Load `(id, srp_version, salt, verifier, login_ticket, login_ticket_expiry, last_login_info)` for a given email |
| `LOGIN_UPD_BNET_AUTHENTICATION` | Write back salt+verifier (post-migration) |
| `LOGIN_UPD_BNET_LAST_LOGIN_INFO` | Last IP / OS / locale on successful login |
| `LOGIN_INS_BNET_GAME_ACCOUNT_LOGIN_INFO` | Per-game-account last-login row |
| `LOGIN_SEL_BNET_GAME_ACCOUNT_LOGIN_TICKET` | Validate a `TC-xxx` ticket |
| `LOGIN_UPD_BNET_LOGIN_TICKET_EXPIRY` | Refresh ticket TTL |
| `LOGIN_DEL_EXPIRED_IP_BANS` / `LOGIN_UPD_EXPIRED_ACCOUNT_BANS` / `LOGIN_DEL_BNET_EXPIRED_ACCOUNT_BANNED` | Periodic ban housekeeping (every `BanExpiryCheckInterval` s) |
| `LOGIN_SEL_IP_INFO` | Pre-login IP-ban check |
| `LOGIN_INS_FAILEDLOGINS` / `LOGIN_INS_IP_AUTO_BANNED` | Wrong-pass auto-ban (controlled by `WrongPass.MaxCount`, `WrongPass.BanTime`, `WrongPass.BanType`) |
| `LOGIN_SEL_BNET_GAME_ACCOUNTS` | List linked game accounts for `/bnetserver/gameAccounts/` |
| `LOGIN_KEEP_ALIVE` | Tiny ping query, every `MaxPingTime` minutes |

DBC/DB2 stores: none.

---

## 7. Wire-protocol packets (if any)

Two **wholly separate** wire protocols:

### Port 1119 — BNet RPC (TLS, binary)
Frames carry protobuf messages. The serviceHash + methodId pair determines the call. Top-level "opcodes" (after TLS handshake):

| Opcode / Flow | Direction | Sent/Received in |
|---|---|---|
| `Battlenet::AuthChallenge` (initial protobuf) | client → server | `Session::HandleAuthChallenge` |
| `connection.v1.Connect / Bind / Echo / KeepAlive` | bidirectional | `ConnectionService` |
| `authentication.v1.Logon` ↔ `LogonResult` | client → server, then server → client | `AuthenticationService` |
| `game_utilities.v1.ProcessClientRequest` (`Command_RealmListRequest_v1`, `Command_RealmJoinRequest_v1`) | client → server | `GameUtilitiesService` |
| `account.v1.GetAccountState / GetGameAccountState` | client → server | `AccountService` (mostly stubs returning placeholder data) |

### Port 8081 — `LoginREST` (HTTPS, JSON / protobuf-JSON)
HTTP routes (verb + path):

| Route | Purpose |
|---|---|
| `GET  /bnetserver/login/` | Form schema (`FormInputs`) |
| `GET  /bnetserver/gameAccounts/` | List linked game accounts (auth: ticket header) |
| `GET  /bnetserver/portal/` | "BNet portal" host:port |
| `POST /bnetserver/login/` | First-stage login |
| `POST /bnetserver/login/srp/` | SRP challenge / response |
| `POST /bnetserver/refreshLoginTicket/` | Extend ticket expiry |
| `POST /login/`, `POST /login/srp/` | Mobile/"bot" client equivalents |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `crates/bnet-server/src/main.rs` | `file` | 1 | 245 | `exists_active` | file exists |
| `crates/bnet-server/src/state.rs` | `file` | 1 | 112 | `exists_active` | file exists |
| `crates/bnet-server/src/rest/mod.rs` | `file` | 1 | 198 | `exists_active` | file exists |
| `crates/bnet-server/src/rest/handlers.rs` | `file` | 1 | 573 | `exists_active` | file exists |
| `crates/bnet-server/src/rest/types.rs` | `file` | 1 | 86 | `exists_active` | file exists |
| `crates/bnet-server/src/rpc/mod.rs` | `file` | 1 | 42 | `exists_active` | file exists |
| `crates/bnet-server/src/rpc/session.rs` | `file` | 1 | 263 | `exists_active` | file exists |
| `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` | `declared_pattern` | 0 | 0 | `declared_pattern` | pattern/proposed path; not resolvable as one file or directory |
| `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/bnet-server/src/main.rs` — 245 lines — entry point, tokio runtime
- `crates/bnet-server/src/state.rs` — 112 lines — `AppState` shared across REST + RPC tasks
- `crates/bnet-server/src/rest/mod.rs` — 198 lines — raw HTTPS connection accept + parser
- `crates/bnet-server/src/rest/handlers.rs` — 573 lines — all REST endpoints inline
- `crates/bnet-server/src/rest/types.rs` — 86 lines — request/response DTOs
- `crates/bnet-server/src/rpc/mod.rs` — 42 lines — accept loop for port 1119
- `crates/bnet-server/src/rpc/session.rs` — 263 lines — equivalent of `Battlenet::Session`
- `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` — 110 + 337 + 83 + 384 lines — service handlers
- `crates/bnet-server/src/realm/mod.rs` — 392 lines — realm-list state & timers (subset of `RealmList`)

**What's implemented:**
- Tokio runtime, two `tokio::spawn`'d accept loops (REST + RPC).
- `tokio_rustls::TlsAcceptor` for both ports — separate `ServerConfig` instances; both pinned to TLS 1.2 to match the WoW client. REST has no ALPN (matches the C# port the project was forked from, and TC also doesn't set ALPN here).
- Cert loading: `bnet_cert.pem` + `bnet_key.pem`, falls back to `bnet_fullchain.pem` if present (Let's Encrypt). Note: TC reads paths from `CertificatesFile` config; Rust **hardcodes the filenames**.
- `LoginDatabase` connection via `sqlx` + `wow_database::LoginDatabase`.
- `wow_database::updater::DbUpdater` runs on startup (`Updates.AutoSetup` config) — executes pending `.sql` files in `sql/updates/` automatically. **This is a port of TC's `DBUpdater`, which lives in TC's common DB layer and is invoked by `DatabaseLoader`** — TC does this on the worldserver too (see worldserver doc), so behaviour matches.
- Ban-expiry timer (`tokio::time::interval`) running `DEL_EXPIRED_IP_BANS`, `UPD_EXPIRED_ACCOUNT_BANS`, `DEL_BNET_EXPIRED_ACCOUNT_BANNED` every `BanExpiryCheckInterval` seconds — direct functional port of `BanExpiryHandler`.
- `realm/` module owns the realm list state with a refresh interval (`RealmsStateUpdateDelay`).
- REST endpoints: `GET /bnetserver/login/` (form), `POST /bnetserver/login/srp/` (challenge), `POST /bnetserver/login/` (response), `GET /bnetserver/portal/`, `GET /bnetserver/gameAccounts/`. Wrong-password lockout flags (`wrong_pass_max`, `wrong_pass_ban_time`, `wrong_pass_ban_type`) are wired into `AppState`.
- BNet RPC services: `AccountService`, `AuthenticationService` (Logon flow with SRPv1+v2), `ConnectionService` (Bind/Echo), `GameUtilitiesService` (RealmList + RealmJoin).
- Shutdown: single `tokio::signal::ctrl_c()` await in `main`. On signal, drops `state.login_db` and exits.

**What's missing vs C++:**
- **No DB keep-alive timer**. TC pings every 30 minutes (`KeepDatabaseAliveHandler`) to keep MariaDB's `wait_timeout` from killing idle pool connections. `sqlx` does its own pool health-check on borrow, but TC's behaviour is not exactly equivalent and a long-idle bnetserver may see first-request latency spikes.
- **No PID-file creation** (TC: `PidFile` config → `CreatePIDFile(...)`).
- **No `IPLocation` DB load**. TC reads `IPLocation.File` (a CSV of GeoIP ranges → countries) and uses it for `lock_country` enforcement and per-country ban policies. Rust never touches this.
- **No `SecretMgr` initialization** for `SECRET_OWNER_BNETSERVER`. SecretMgr stores HMAC keys for various server-internal purposes (e.g. realm-list signing). Currently every signed message in the Rust port either uses a hardcoded test key or skips signing entirely.
- **No `Banner::Show`** equivalent (cosmetic, but useful for ops).
- **No legacy password migration** (`MigrateLegacyPasswordHashes`). Means accounts on SRPv1 can never be upgraded silently — they have to be deleted and recreated.
- **No `POST /login/`, `POST /login/srp/`** (the "bot" / mobile-client login routes).
- **No `POST /bnetserver/refreshLoginTicket/`**. Ticket TTL is fixed at issuance.
- **No `SOAP` / Win32 service / process priority / processor affinity** — Linux-only Rust target, accepted gap.
- **`MaxCoreStuckTime` / freeze detector** — bnetserver's main loop is event-driven (not a tight `World::Update` loop), so technically less of a concern, but no equivalent watchdog at all.
- **No multi-threaded io_context** (`ThreadPool` worker count). Rust runs on the default Tokio multi-thread runtime — fine, but means the `Network.Threads` / `LoginREST.ThreadCount` configs are silently ignored.
- **CLI args**: TC supports `--config`, `--config-dir`, `--update-databases-only`, `--service install/uninstall`, `--help`, `--version`. Rust currently parses none.
- **Cert loading hardcoded** to `bnet_cert.pem` / `bnet_key.pem` / `bnet_fullchain.pem`. Should read `CertificatesFile` from config.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **TLS 1.2 only** is correct (WoW 3.4.3 client doesn't speak 1.3) but the cert-loading path differs from TC's `SslContext::Initialize`, which uses Boost.Asio's OpenSSL backend with explicit cipher list pinning. Some clients on uncommon OS/TLS-stack combos may negotiate different ciphers. Worth a `openssl s_client` capture vs TC.
- **The "BNet RPC" framing** in `rpc/session.rs` claims to mirror C#/TC but the BNet protocol uses a 16-bit length prefix + protobuf with explicit `Header { service_hash, method_id, token, object_id, status, error[], timeout, size }` — verify byte-for-byte that what the client sends is actually decoded correctly (mismatched here ⇒ silent client kick).
- **`session_key_bnet` storage**: TC writes 64 hex chars (a 32-byte session key encoded as 64 ASCII). Rust expects 64 raw bytes (`varbinary(64)`) in the world-server reader. If bnetserver writes hex strings in Rust as well, the world-server's `try_read::<Vec<u8>>` would get the ASCII bytes, not 32 bytes of key material.
- **No legacy v1 migration** could be silent foot-gun if any pre-existing accounts in the DB are still v1 (most TC dumps are v2 since 2018, but worth checking).
- **Wrong-pass tracking is in-memory**. TC writes failed login attempts to `account_failedlogins` / `ip_auto_banned` so the ban survives a restart. Rust seems to count in `AppState` only.

**Tests existing:**
- A handful in `bnet-server/src/rest/` (likely unit tests for SRP). To confirm via `cargo test -p bnet-server`.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#BNETSERVER.WBS.001** Cerrar la migracion auditada de `bnetserver/Main.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.002** Cerrar la migracion auditada de `bnetserver/REST/LoginHttpSession.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginHttpSession.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.003** Cerrar la migracion auditada de `bnetserver/REST/LoginHttpSession.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginHttpSession.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.004** Partir y cerrar la migracion auditada de `bnetserver/REST/LoginRESTService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 825 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.005** Cerrar la migracion auditada de `bnetserver/REST/LoginRESTService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.006** Partir y cerrar la migracion auditada de `bnetserver/Server/Session.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 842 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.007** Cerrar la migracion auditada de `bnetserver/Server/Session.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.008** Cerrar la migracion auditada de `bnetserver/Server/SessionManager.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/SessionManager.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.009** Cerrar la migracion auditada de `bnetserver/Server/SessionManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/SessionManager.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.010** Cerrar la migracion auditada de `bnetserver/Server/SslContext.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/SslContext.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.011** Cerrar la migracion auditada de `bnetserver/Server/SslContext.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/SslContext.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.012** Cerrar la migracion auditada de `bnetserver/Services/AccountService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/AccountService.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.013** Cerrar la migracion auditada de `bnetserver/Services/AccountService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/AccountService.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.014** Cerrar la migracion auditada de `bnetserver/Services/AuthenticationService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/AuthenticationService.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.015** Cerrar la migracion auditada de `bnetserver/Services/AuthenticationService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/AuthenticationService.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.016** Cerrar la migracion auditada de `bnetserver/Services/ConnectionService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/ConnectionService.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.017** Cerrar la migracion auditada de `bnetserver/Services/ConnectionService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/ConnectionService.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.018** Cerrar la migracion auditada de `bnetserver/Services/GameUtilitiesService.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/GameUtilitiesService.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.019** Cerrar la migracion auditada de `bnetserver/Services/GameUtilitiesService.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/GameUtilitiesService.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.020** Cerrar la migracion auditada de `bnetserver/Services/Service.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/Service.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.021** Cerrar la migracion auditada de `bnetserver/Services/ServiceDispatcher.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/ServiceDispatcher.cpp`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.022** Cerrar la migracion auditada de `bnetserver/Services/ServiceDispatcher.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Services/ServiceDispatcher.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#BNETSERVER.WBS.023** Cerrar la migracion auditada de `bnetserver/resource.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/resource.h`
  Rust target: `crates/bnet-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#BNET.1** Add CLI parser (`clap`): `--config`, `--config-dir`, `--update-databases-only`, `--version`, `--help`. (L)
- [ ] **#BNET.2** Read `CertificatesFile` from config; support both PEM-bundle (`pkcs12`-equivalent) and the pair-of-files form. Fallback to current hardcoded names with a warning. (M)
- [ ] **#BNET.3** Implement DB keep-alive timer: every `MaxPingTime` minutes (default 30) issue a `SELECT 1` against `LoginDatabase`. (L)
- [ ] **#BNET.4** Implement `CreatePIDFile(path)` equivalent (`std::fs::write(pidFile, std::process::id().to_string())`). (L)
- [ ] **#BNET.5** Port `IPLocation` loader: parse CSV from `IPLocation.File`, build a sorted IP-range → country map; expose `lookup(ip)`. Used by `lock_country` and `WrongPass.BanType=BAN_IP` policies. (M)
- [ ] **#BNET.6** Port `SecretMgr::Initialize(SECRET_OWNER_BNETSERVER)`: persist a per-realm HMAC key in `secrets` table (or local file as TC does); use it for any internal signing. (M)
- [ ] **#BNET.7** Add the missing REST routes: `POST /login/`, `POST /login/srp/` (bot/mobile clients), `POST /bnetserver/refreshLoginTicket/`. (M)
- [ ] **#BNET.8** Persist wrong-pass attempts in `account_failedlogins` + `ip_auto_banned` (don't only track in-memory). (M)
- [ ] **#BNET.9** Implement `MigrateLegacyPasswordHashes()`: opt-in one-shot pass that re-derives v2 verifier on first successful v1 login. (M)
- [ ] **#BNET.10** Add a freeze-style watchdog: if any `tokio::spawn`'d listener task panics or exits unexpectedly, log error and exit non-zero (currently Rust just logs `"REST server stopped"` and waits on the other handle). (L)
- [ ] **#BNET.11** Verify `session_key_bnet` storage format vs TC: should be **hex string** (64 chars) in MySQL, not raw bytes — align bnetserver writer and world-server reader. (M, code-trace)
- [ ] **#BNET.12** Multi-thread option: read `Network.Threads` config and (optionally, since Tokio is already multi-threaded) document that the value is informational under the Tokio runtime. (L)
- [ ] **#BNET.13** Add `Banner::Show`-equivalent at startup logging (build hash, openssl version, rustls version, sqlx version). (L)
- [ ] **#BNET.14** Drop-in clean shutdown: on SIGINT/SIGTERM cancel timers, drain in-flight REST requests with a 5 s grace, then `db.close()`. Currently `db.close()` happens but in-flight requests are not drained. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#BNETSERVER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 23 files / 3266 lines; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp`. Rust target: `crates/bnet-server`. | `cargo test -p bnet-server` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#BNETSERVER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 23 files / 3266 lines; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp`. Rust target: `crates/bnet-server`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#BNETSERVER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 23 files / 3266 lines; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp`. Rust target: `crates/bnet-server`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#BNETSERVER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 23 files / 3266 lines; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp`. Rust target: `crates/bnet-server`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: a fresh BNet account created via `Battlenet::AccountMgr::CreateBattlenetAccount` can complete the SRPv2 challenge against `POST /bnetserver/login/srp/` end-to-end, getting a `TC-<hex>` ticket back.
- [ ] Test: after `LoginREST.TicketDuration` seconds the same ticket is rejected.
- [ ] Test: `POST /bnetserver/refreshLoginTicket/` extends an unexpired ticket's `loginTicketExpiry` by exactly `LoginREST.TicketDuration`.
- [ ] Test: `BanExpiryHandler` runs every `BanExpiryCheckInterval` seconds and clears expired rows. (Time-controlled with `tokio::time::pause`.)
- [ ] Test: with `WrongPass.MaxCount=3, WrongPass.BanType=BAN_IP, WrongPass.BanTime=600`, four bad SRP challenges from the same IP produce a row in `ip_banned` valid for 600 s. Restart the server — ban survives.
- [ ] Test: `realmlist` BNet realm-list response decompressed equals what TC produces for the same DB content (golden file).
- [ ] Test: TLS handshake on 1119 with the WoW client cipher list (`ECDHE-RSA-AES256-SHA384`, etc.) succeeds.
- [ ] Test: bnetserver continues to accept connections after a 30-minute idle window (validates DB keep-alive once #BNET.3 lands).
- [ ] Test: SIGTERM during an in-flight `POST /bnetserver/login/srp/` still finishes the response before the process exits.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 23 files / 3266 lines; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp` | `crates/bnet-server/` \| ⚠️ partial (login flow works; missing freeze detector, IP-Location DB, soap, win32 service, multi-thread io_context, some REST endpoints, SecretMgr, DB keepalive) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#BNETSERVER.DIV.001` | `crates/bnet-server/src/rpc/services/{account,authentication,connection,game_utilities}.rs` (`declared_pattern`, 0 Rust lines) | 23 C++ files / 3266 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/bnetserver/Server/Session.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/REST/LoginRESTService.cpp`, `/home/server/woltk-trinity-legacy/src/server/bnetserver/Main.cpp` | `declared_pattern` | Rust target is a pattern/proposal, not a concrete checked file/module. pattern/proposed path; not resolvable as one file or directory |

<!-- REFINE.023:END known-divergences -->

- **Two ports, two TLS contexts, one binary.** Don't accidentally share a `ServerConfig` — REST may have ALPN (the WoW launcher has been observed both with and without; TC and the C# fork stay safe by **not** advertising ALPN).
- **Boost.Asio `io_context.run()` blocks the main thread.** TC achieves multi-threading by spawning N additional threads that all call `run()` on the **same** context. Tokio's analogue is just "be on the multi-thread runtime" — the worldserver's `ThreadPool` config has no exact equivalent and shouldn't be ported literally.
- **Signal handlers register before `io_context.run()`.** TC uses `boost::asio::signal_set` with `async_wait`, which integrates with the io_context's poll loop. Tokio's `tokio::signal::ctrl_c` and `tokio::signal::unix::signal(SIGTERM)` both work but **only one** can be installed per signal — pick `ctrl_c` for SIGINT, install `signal(SIGTERM)` separately.
- **TC also handles `SIGBREAK` on Windows**. Linux target: skip.
- **Login REST is HTTPS even in development.** The default `bnetserver.cert.pem` / `.key.pem` are self-signed; the WoW client trusts whatever the launcher's CA bundle says — production needs a public CA cert (`bnet_fullchain.pem`).
- **Wrong-pass ban policy is configurable** but the default in `bnetserver.conf.dist` is **off** (`WrongPass.MaxCount = 0` ⇒ no auto-ban). Don't enable it on test servers without warning users.
- **`MigrateLegacyPasswordHashes`** runs once on startup *only if* a config flag is set. Don't run it transparently on every boot.
- **`OpenSSLCrypto::threadsSetup` / `threadsCleanup`** is OpenSSL ≤ 1.0.2 ABI compatibility (locking callbacks). OpenSSL 1.1+ doesn't need it. Rust uses `rustls`, so this whole concern is moot — but the cipher-suite negotiation has to match what TC's OpenSSL build offers.
- **`google::protobuf::ShutdownProtobufLibrary()`** is called from a smart-pointer destructor at process exit. The `prost` ecosystem doesn't need it; safe to ignore.
- The `bnetserver` binary in C# / Rust **does not** load creature/spell/item data. The DB pool only attaches `LoginDatabase`. Don't drag world data in.
- **`sLog->SetRealmId(0)`** is called after DB init — it tags `AppenderDB` log rows with realm 0 (= "this is the bnetserver, not a world"). Equivalent in Rust would be a `tracing` field on the global subscriber.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `int main(int argc, char** argv)` | `#[tokio::main] async fn main() -> anyhow::Result<()>` | — |
| `boost::asio::io_context` + `ThreadPool` | Tokio multi-thread runtime (implicit) | Tokio is the runtime; no explicit io_context to pass around. Use `Arc<AppState>` for shared data. |
| `boost::asio::signal_set signals(io, SIGINT, SIGTERM)` + `async_wait(SignalHandler)` | `tokio::select! { _ = tokio::signal::ctrl_c() => ..., _ = sigterm_stream.recv() => ..., }` | Install both in `main` after spawning listeners. |
| `boost::asio::ssl::context` (`SslContext`) | `rustls::ServerConfig` + `tokio_rustls::TlsAcceptor` | Two separate `ServerConfig`s (REST + RPC), pinned to TLS 1.2. |
| `LoginRESTService` (HttpService<LoginHttpSession>) | `crates/bnet-server/src/rest/{mod,handlers,types}.rs` (raw `tokio_rustls` + handwritten HTTP/1.1 parser) | TC uses `boost::beast`; Rust port uses a hand-rolled parser to avoid hyper's TLS CloseNotify quirk that the WoW client mishandles. |
| `RegisterHandler(verb, path, fn)` | `match (method, path)` in `rest::handle_rest_connection` | Switch from registration to dispatch. |
| `Battlenet::Session` (`SocketMgr<Session>`) | `crates/bnet-server/src/rpc/session.rs` (`BNetRpcSession`) | One `tokio::spawn`'d task per accepted TCP connection. |
| `Battlenet::ServiceDispatcher` | `bnet_server::rpc::dispatch_call(session, service_hash, method_id, token, payload)` | Hand-coded switch on `service_hash`; no generated tables. |
| `Trinity::Asio::DeadlineTimer` | `tokio::time::interval` + `interval.tick().await` in a loop | The `KeepDatabaseAliveHandler` recursive-rearm pattern becomes a normal `loop { interval.tick().await; ... }`. |
| `LoginDatabase.GetPreparedStatement(LOGIN_X)` | `state.login_db.prepare(LoginStatements::X)` | `wow-database` already exposes this. |
| `LoginDatabase.AsyncQuery(...).WithPreparedCallback(...)` | `state.login_db.query(&stmt).await?` | Naturally async in Rust. |
| `sConfigMgr->GetStringDefault("BindIP", "0.0.0.0")` | `wow_config::get_string_default("BindIP", "0.0.0.0")` | Already implemented. |
| `boost::program_options::variables_map` | `clap::Parser` derive | TODO (#BNET.1). |
| `MySQL::Library_Init()` | (none) | sqlx handles its own MariaDB client init. |
| `OpenSSLCrypto::threadsSetup(...)` | (none) | Not needed with rustls. |
| `Trinity::Banner::Show("bnetserver", ...)` | `tracing::info!("RustyCore BNet Server starting...")` | Log build hash with `env!("CARGO_PKG_VERSION")`. |
| `sIPLocation->Load()` | `wow_account::ip_location::load(path) -> IpLocationDb` (TODO) | New module. |
| `sSecretMgr->Initialize(SECRET_OWNER_BNETSERVER)` | `wow_account::secrets::initialize(SecretOwner::BnetServer).await` (TODO) | New module. |
| `WinServiceInstall / Uninstall / Run` | (none) | Linux target. |
| `CreatePIDFile(path)` | `std::fs::write(path, std::process::id().to_string())?` | TODO (#BNET.4). |
| `m_ServiceStatus` global + `ServiceStatusWatcher` | (none) | Linux target. |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

### 13.1 Audit summary

The Rust bnet daemon **reaches the same listening state as TrinityCore** for the happy path: it binds 1119 (TLS, BNet RPC) + 8081 (HTTPS, REST), opens a `LoginDatabase`, runs the `BanExpiryHandler`, polls `realmlist` every `RealmsStateUpdateDelay` s, and wires up the SRPv1/v2 → ticket → `VerifyWebCredentials` → `LogonResult` flow end-to-end. The five core REST endpoints needed for the WoW launcher are present and `cargo test --workspace` passes (395 tests).

What is **not** at parity with TC: cert path config, DB keep-alive ping, `SecretMgr` (HMAC keys), `IPLocation`, the two "bot" REST routes (`POST /login/`, `POST /login/srp/`), `MigrateLegacyPasswordHashes`, CLI args, PID file, and a graceful drain on SIGTERM. The ALPN / TLS-cipher pinning differs because Rust uses `rustls` (TLS 1.2-only `ServerConfig` with no ALPN) while TC uses Boost.Asio + OpenSSL with `TLS_method` (negotiates anything); for WoW 3.4.3 clients that converge on TLS 1.2 + an `ECDHE-RSA-AES*` suite this is functionally equivalent, but uncommon launcher builds may see different ciphers.

Resolved since the original audit: `extract_auth_ticket` now mirrors TC's `ExtractAuthorization` by stripping optional `"Basic "`, Base64-decoding the value, and truncating at the first `:`. The failed-login/autoban path now persists `WrongPass.*` effects to `battlenet_accounts.failed_logins`, `battlenet_account_bans`, or `ip_banned`.

### 13.2 Startup-sequence parity

| Step (TC `Main.cpp`) | TC behaviour | Rust `main.rs` | Parity |
|---|---|---|---|
| `signal(SIGABRT, AbortHandler)` | install crash handler | none | ❌ |
| `Trinity::Locale::Init()` | set process locale | none | ❌ (cosmetic) |
| `GetConsoleArguments()` | parse `--config` / `-cd` / `-u` / `-v` / `-h` | none — only `BNetServer.conf` lookup | ❌ |
| `GOOGLE_PROTOBUF_VERIFY_VERSION` | sanity-check protobuf ABI | n/a (`prost`) | ✅ N/A |
| Win32 service install/uninstall/run | optional | n/a (Linux) | ✅ accepted gap |
| `sConfigMgr->LoadInitial(...)` + `LoadAdditionalDir(...)` | base + per-dir overrides | `wow_config::load_config` of single file (with `.dist` fallback) | ⚠️ no `conf.d/` |
| `OverrideWithEnvVariablesIfAny` | env var → config override | none | ❌ |
| `sLog->Initialize` + `Banner::Show` | log to file/console + DB appender | `tracing_subscriber::fmt` + single info line | ⚠️ partial |
| `OpenSSLCrypto::threadsSetup` | OpenSSL ≤1.0.2 locking callbacks | n/a (rustls) | ✅ N/A |
| `CreatePIDFile(path)` if `PidFile` set | optional pid file | none | ❌ |
| `SslContext::Initialize()` | reads `CertificatesFile` + `PrivateKeyFile` + `PrivateKeyPassword`; one shared `ssl::context` | hardcoded `bnet_cert.pem` / `bnet_key.pem` (`bnet_fullchain.pem` if present); **two separate** rustls `ServerConfig`s (REST + RPC); reads `CertificatesFile` config but **does not actually use it** | ❌ cert path / password ignored |
| `StartDB()` | single MariaDB pool for `LoginDatabase` | identical (sqlx pool via `wow_database::LoginDatabase`) | ✅ |
| `--update-databases-only` short-circuit | run updaters then exit | runs `DbUpdater::populate` + `update`, never exits early | ⚠️ different semantics |
| `sSecretMgr->Initialize(SECRET_OWNER_BNETSERVER)` | persist HMAC key | none | ❌ |
| `sIPLocation->Load()` | parse GeoIP CSV | none | ❌ |
| `Trinity::Net::ScanLocalNetworks()` | enumerate own subnets for "client is local" check | none — Rust uses literal `127.0.0.1` / same-/24 logic in `realm/mod.rs::select_realm_ip_str` | ⚠️ partial |
| `sLoginService.StartNetwork(...)` (DNS-resolves `LoginREST.{External,Local}Address`, registers 8 handlers, calls `_acceptor->AsyncAcceptWithCallback<&OnSocketAccept>()`) | — | bind `tokio::net::TcpListener`, accept-loop spawns one task per conn, no DNS resolution of hostnames | ⚠️ no DNS resolve, fewer handlers |
| `sRealmList->Initialize(io, RealmsStateUpdateDelay)` | DB poll `LoginDatabase` every N s + initial `LoadBuildInfo` | `realm::init_realm_manager` does the same | ✅ |
| `sSessionMgr.StartNetwork(io, BindIP, BattlenetPort)` | TLS RPC acceptor on 1119 | identical, separate `TlsAcceptor` | ✅ |
| `boost::asio::signal_set(SIGINT, SIGTERM)` | graceful shutdown | only `tokio::signal::ctrl_c` (SIGINT); SIGTERM never installed | ❌ |
| `SetProcessPriority(...)` | priority/affinity | none | ✅ accepted gap (Linux) |
| `KeepDatabaseAliveHandler` (every `MaxPingTime` min) | `LoginDatabase.KeepAlive()` | none | ❌ |
| `BanExpiryHandler` (every `BanExpiryCheckInterval` s) | DEL/UPD expired bans (3 statements) | identical (`start_ban_expiry_timer`) | ✅ |
| `ServiceStatusWatcher` (Win32) | pump `m_ServiceStatus` | n/a | ✅ N/A |
| `ioContext->run()` | block main | `tokio::select! { rest_handle, rpc_handle, ctrl_c }` | ✅ |
| Shutdown: `signals.cancel()`, `LoginDatabase.Close()`, `MySQL::Library_End()` | clean drain | `state.login_db.close().await` only — no in-flight request drain, listener tasks just dropped | ⚠️ partial |

### 13.3 REST endpoint coverage

| Verb + path | TC | Rust | Notes |
|---|---|---|---|
| `GET /bnetserver/login/` | ✅ `HandleGetForm` | ✅ `get_form` | Rust adds extra `JSESSIONID` cookie that TC does not set (carry-over from C# fork). Form schema and `srp_url` match. |
| `POST /bnetserver/login/` | ✅ `HandlePostLogin` | ✅ `post_login` | Both accept (a) direct password (legacy) and (b) `public_A` + `client_evidence_M1`. Rust verifies via `BnetSrp6::verify_client_evidence`; TC does the same with `BnetSRP6Base::VerifyChallengeResponse`. On bad-credential path Rust now matches TC's `WrongPass.*` policy: returns `LoginResult{ state=DONE }`, optionally logs, increments `failed_logins`, inserts BNet-account/IP autobans at the threshold, and resets failed-login count. **Remaining nuance:** TC uses the connection remote IP; Rust REST uses first `X-Forwarded-For` hop when present, otherwise `LoginREST.ExternalAddress`. |
| `POST /bnetserver/login/srp/` | ✅ `HandlePostLoginSrpChallenge` | ✅ `post_login_srp_challenge` | Same SRP6 challenge response (modulus, generator, salt, public B, hash function name `"SHA-256"`). ✅ parity. |
| `GET /bnetserver/gameAccounts/` | ✅ `HandleGetGameAccounts` | ✅ `get_game_accounts` | Same query (`SEL_BNET_GAME_ACCOUNT_LIST`). Rust now matches TC `ExtractAuthorization`: optional `Basic ` prefix removal, Base64 decode, then truncate at `:`. |
| `GET /bnetserver/portal/` | ✅ `HandleGetPortal` | ✅ `get_portal` | TC returns `GetHostnameForClient(remoteIp):port`; Rust returns `X-Forwarded-For`-or-`external_address`:port. Different selection logic but same shape. |
| `POST /bnetserver/refreshLoginTicket/` | ✅ `HandlePostRefreshLoginTicket` | ✅ `refresh_login_ticket` | Rust now matches TC response shape: `LoginRefreshResult{login_ticket_expiry}` for valid unexpired tickets, or `is_expired=true` when missing/expired. Same DB write. |
| `POST /login/srp/` (bot/mobile) | ✅ `HandlePostBotSrpChallenge` | ❌ missing | route returns 404 |
| `POST /login/` (bot/mobile) | ✅ `HandlePostBotLogin` | ❌ missing | route returns 404 |
| `OPTIONS *` (CORS preflight) | ❌ none | ❌ none | ✅ parity |

### 13.4 Auth flow divergences (port 1119, BNet RPC)

| Stage | TC | Rust | Status |
|---|---|---|---|
| TCP accept → TLS handshake | Boost.Asio `ssl::stream` (TLS_method, OpenSSL cipher list) | `tokio_rustls::TlsAcceptor` (TLS 1.2 only, rustls default ciphers) | ⚠️ rustls cipher set ⊂ OpenSSL |
| ALPN | not advertised | not advertised | ✅ |
| `LOGIN_SEL_IP_INFO` ip-ban check on `Start()` | ✅ | ✅ — Rust now purges expired IP bans and checks `ip_banned` before TLS for both RPC and REST acceptors | ✅ |
| `ConnectionService::Connect/Bind/Echo/KeepAlive` | full | `Bind`/`Echo` implemented; `Connect`/`KeepAlive` partial | ⚠️ |
| `AuthenticationService.Logon` | validates program/platform/locale, optional `cached_web_credentials` shortcut, sends `ChallengeExternalRequest` | program+platform+locale validated with TC status codes; `cached_web_credentials` now short-circuits through the same VerifyWebCredentials path; challenge fallback remains | ✅ |
| `ChallengeListener::OnExternalChallenge` (web auth URL) | sent via `Service<ChallengeListener>` | sent via `send_request(CHALLENGE_LISTENER, 3, …)` | ✅ |
| `AuthenticationService.VerifyWebCredentials` | loads account + char counts + last-played in chained query callback; checks IP lock, country lock (via `IPLocation`), `IsBanned` / `IsPermanenetlyBanned`; sets `_authed` and dispatches `AuthenticationListener::OnLogonComplete` (method 5) | similar; but **no country lock** (no `IPLocation`); 64-byte `session_key` is fresh random per call (TC also random — ✅); error codes used: 3, 12 | ⚠️ no country lock |
| Error codes on auth failure | `ERROR_DENIED=3`, `ERROR_TIMED_OUT=2`, `ERROR_RISK_ACCOUNT_LOCKED=0xA413`, `ERROR_GAME_ACCOUNT_BANNED=0x34`, `ERROR_GAME_ACCOUNT_SUSPENDED=0x35` | Rust now returns the same RPC status codes for missing/invalid/expired tickets, IP lock mismatch, and permanent/temporary BNet account bans. Country-lock remains blocked on `IPLocation`. | ⚠️ |
| `GameUtilitiesService.ProcessClientRequest` (RealmList / RealmJoin / LastCharPlayed / RealmListTicket) | full | full | ✅ |
| `GameUtilitiesService.GetAllValuesForAttribute` (sub-region enumeration) | full | full | ✅ |
| `AccountService.GetAccountState/GetGameAccountState` | stubs | stubs | ✅ |
| `session_key_bnet` / `UPD_BNET_GAME_ACCOUNT_LOGIN_INFO` write | TC writes 64 raw bytes via `setBinary` | Rust writes `combined` (`client_secret ‖ server_secret`, expected 64 raw bytes) via `set_bytes` | ✅ assuming `client_secret` is 32 bytes from launcher |

### 13.5 Cookie / token signing

There is **no JWT or HMAC-signed cookie** anywhere. Both TC and Rust use:

- **Login ticket** = opaque random hex string (`TC-` + 20 random bytes, 40 hex chars). Stored verbatim in `battlenet_accounts.LoginTicket`. Validated by lookup, not signature. ✅ parity.
- **`JSESSIONID` cookie** = 16 random bytes hex, only meaningful as an SRP-state-bag key. Not signed. Rust uses `DashMap<String, RestSessionState>`. TC keeps it in the per-connection `LoginSessionState`. ⚠️ Rust persists across connections (multi-request SRP works behind a load balancer); TC does not — slight divergence but harmless.
- **Realm-list ticket** = literal ASCII `b"AuthRealmListTicket"` returned in `Param_RealmListTicket`. TC writes the same constant. ✅
- **`Param_JoinSecret`** = 32 random bytes per `RealmJoinRequest`. Combined with `client_secret` and stored as `session_key_bnet`. ✅

`SecretMgr::Initialize(SECRET_OWNER_BNETSERVER)` in TC does load an HMAC key — but **only worldserver consumes it** (for realm-list signing on the realmlist socket from the connect server). bnetserver itself initializes it but does not sign anything user-facing. So missing-`SecretMgr` is a worldserver-side gap, not a bnetserver one. (See worldserver doc.)

### 13.6 TLS specifics

| Concern | TC | Rust |
|---|---|---|
| Library | OpenSSL 1.1+/3.0 via Boost.Asio `ssl::context` | `rustls` 0.23 via `tokio-rustls` |
| Protocol versions | `tls` (= TLS_method, all versions enabled) | TLS 1.2 only (pinned via `builder_with_protocol_versions(&[&TLS12])`) |
| ALPN | not set | not set |
| Cert source | `CertificatesFile` (chain), `PrivateKeyFile`, `PrivateKeyPassword` | hardcoded `bnet_cert.pem` / `bnet_key.pem`, fallback `bnet_fullchain.pem`. **`CertificatesFile` config is read but never used.** |
| Client auth | none | none |
| Cipher list | OpenSSL default (`HIGH:!aNULL:!MD5` + system policy) | rustls TLS 1.2 default (ECDHE-{RSA,ECDSA}-AES{128,256}-GCM-SHA{256,384}, plus a few CHACHA20 variants) |
| Two contexts (REST vs RPC)? | one `ssl::context` shared | two separate `ServerConfig`s (functionally identical, just clones) |

### 13.7 Recommended sub-tasks

Add these to §9 (existing tasks #BNET.1–#BNET.14 stand):

- [x] **#BNET.15** Fix `extract_auth_ticket`: Base64-decode the optional-`Basic ` authorization value, then truncate at first `:`. Matches `LoginRESTService::ExtractAuthorization`.
- [ ] **#BNET.16** Wire `CertificatesFile` config (already read in `main.rs:68` but ignored). Add `PrivateKeyFile` + optional `PrivateKeyPassword`. Fall back to current hardcoded names with a `tracing::warn!`.
- [x] **#BNET.17** Validate locale in `handle_logon` (TC returns `ERROR_BAD_LOCALE`). Rust now uses the same allow-list as `Common.cpp::localeNames` and returns TC status codes for bad program/platform/locale.
- [x] **#BNET.18** Honour `cached_web_credentials` in `LogonRequest`: short-circuit straight to `VerifyWebCredentials` instead of always sending the web-auth challenge. Saves one client round-trip.
- [x] **#BNET.19** Distinguish error codes in `VerifyWebCredentials`: emit TC status codes for expired ticket (`ERROR_TIMED_OUT=2`), IP/country lock (`ERROR_RISK_ACCOUNT_LOCKED=0xA413`), permanent ban (`ERROR_GAME_ACCOUNT_BANNED=0x34`) and temporary suspension (`ERROR_GAME_ACCOUNT_SUSPENDED=0x35`).
- [x] **#BNET.20** Add `LOGIN_SEL_IP_INFO` check in `Session::Start`-equivalent: Rust now mirrors TC by deleting expired IP bans, querying `SEL_IP_INFO`, and closing before TLS when the remote IP is actively banned. Applied to both BNet RPC and REST acceptors because TC's `Session::Start()` and `LoginHttpSession::Start()` share this pre-handshake gate.
- [x] **#BNET.21** Persist failed login attempts and autobans per `WrongPass.MaxCount`/`BanTime`/`BanType`/`Logging` for BNet REST login. Rust writes `battlenet_accounts.failed_logins`, `battlenet_account_bans` or `ip_banned`, and resets failed-login count at the configured threshold like TC. Subsumes `#BNET.8`.
- [ ] **#BNET.22** Install SIGTERM handler alongside `ctrl_c` so `kill <pid>` shuts down cleanly.
- [x] **#BNET.23** Match `HandlePostRefreshLoginTicket` response shape: `{ login_ticket_expiry: <unix> }` or `{ is_expired: true }`, not `{ login_ticket: "…" }`.
- [ ] **#BNET.24** Resolve `LoginREST.{External,Local}Address` via DNS at startup (TC does, fails fast on bad hostname). Today Rust silently uses the literal string.

### 13.8 Header status update

Header status changed from `❌ not audited` → `⚠️ audited (2026-05-01)`. Functional state remains `⚠️ partial` because the audit confirmed gaps; remaining blockers include `IPLocation`/country lock parity, cert path config, bot/mobile REST routes, DB keep-alive, and graceful shutdown details.
