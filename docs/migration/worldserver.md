# Migration: worldserver (game server binary)

> **C++ canonical path:** `src/server/worldserver/`
> **Rust target crate(s):** `crates/world-server/`
> **Layer:** binary (executable entry point)
> **Status:** ⚠️ partial (boot, DB init, listener spawn, session-per-connection work; freeze detector + RA + SOAP + CLI thread + `World::Update` tick loop are all missing)
> **Audited vs C++:** ⚠️ audited 2026-05-01 — **breaking divergence**: no global tick (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The "Trinityd" daemon. Hosts the game world: accepts encrypted WoW client TCP connections on **port 8085** (realm) and **port 8086** (instance), runs `WorldSession` per character, ticks `World::Update(diff)` at a fixed minimum-diff cadence, manages the in-memory game state (`MapManager`, `BattlegroundMgr`, `OutdoorPvPMgr`, `InstanceLockMgr`, `TerrainMgr`, scripts), and exposes the GM admin surface (CLI on stdin, optional Remote Access port, optional SOAP).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `worldserver/CommandLine/CliRunnable.cpp` | 180 | `prefix` |
| `worldserver/CommandLine/CliRunnable.h` | 29 | `prefix` |
| `worldserver/Main.cpp` | 742 | `prefix` |
| `worldserver/RemoteAccess/RASession.cpp` | 192 | `prefix` |
| `worldserver/RemoteAccess/RASession.h` | 56 | `prefix` |
| `worldserver/TCSoap/TCSoap.cpp` | 152 | `prefix` |
| `worldserver/TCSoap/TCSoap.h` | 68 | `prefix` |
| `worldserver/resource.h` | 15 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/src/server/worldserver/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `Main.cpp` | 742 | Entry point; bootstraps everything |
| `CommandLine/CliRunnable.h` / `.cpp` | 30 + 130 | `CliThread()` — stdin reader thread; reads commands, forwards to `World::QueueCliCommand` |
| `RemoteAccess/RASession.h` / `.cpp` | 50 + 150 | `RASession` — per-connection RA console (telnet-like); `Send`, `ReadString`, `CheckAccessLevel`, `ProcessCommand` |
| `TCSoap/TCSoap.h` / `.cpp` | 60 + 160 | `TCSoapThread(host, port)` — gSOAP web service for command exec; `SOAPCommand` future-promise wrapper |
| `worldserver.conf.dist` | — | All ~600 config keys (Network.Threads, MaxCoreStuckTime, Console.Enable, Ra.*, SOAP.*, Updates.*, MaxPingTime, RealmID, Expansion, GM.*, lots more) |

`Main.cpp` is the meat. Notable file-local symbols:
- `class FreezeDetector` — watchdog that aborts the process if `World::m_worldLoopCounter` stops advancing for `MaxCoreStuckTime` ms.
- `void WorldUpdateLoop()` — the actual game tick loop.
- `void SignalHandler(...)` — sets `World::StopNow(SHUTDOWN_EXIT_CODE)`.
- `bool StartDB() / void StopDB()` — opens/closes the four DB pools.
- `AsyncAcceptor* StartRaSocketAcceptor(...)` — RA listener.
- `void ClearOnlineAccounts()` — DB cleanup on start/stop.
- `bool LoadRealmInfo()` — pulls this realm's row from the realmlist.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `FreezeDetector` | class (file-local in Main.cpp) | Holds a `DeadlineTimer`, polls `World::m_worldLoopCounter` every 1 s, aborts if stuck > `MaxCoreStuckTime` ms |
| `RASession` | class : enable_shared_from_this | One per RA telnet client; auth + command exec |
| `SOAPCommand` | class | Holds a `std::promise<void>` for sync command result |
| `m_ServiceStatus` | global int | Win32 service mode (`-1`, `0`, `1`, `2`) |
| `_TRINITY_CORE_CONFIG` / `_DIR` | macro | `"worldserver.conf"` / `"worldserver.conf.d"` |
| `SHUTDOWN_EXIT_CODE` / `ERROR_EXIT_CODE` | constant (in `World.h`) | `0` / `1` exit codes |

Singletons used (not defined here, but instantiated in `main`):
- `sConfigMgr`, `sLog`, `sWorld`, `sScriptMgr`, `sScriptReloadMgr`, `sBattlegroundMgr`, `sOutdoorPvPMgr`, `sMapMgr`, `sTerrainMgr`, `sInstanceLockMgr`, `sSecretMgr`, `sMetric`, `sRealmList`, `sWorldSocketMgr`.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `int main(int argc, char** argv)` | Bootstrap (described in §11 step-by-step) | All below |
| `WorldUpdateLoop()` | Main game tick: while `!World::IsStopped()`, increment `m_worldLoopCounter`, compute diff, sleep if `diff < MinWorldUpdateTime`, call `sWorld->Update(diff)` | `World::Update`, `getMSTime` |
| `SignalHandler(error, signum)` | On SIGINT/SIGTERM (+ SIGBREAK Windows): `World::StopNow(SHUTDOWN_EXIT_CODE)` — sets the stop flag; the tick loop exits at next iteration | World |
| `FreezeDetector::Handler(weak<FreezeDetector>, error)` | Every 1 s: read `World::m_worldLoopCounter`; if unchanged for > `MaxCoreStuckTime` ms, `ABORT_MSG("World Thread hangs for %u ms")` (deliberate crash so a supervisor restarts the process) | `getMSTime`, World |
| `StartDB()` | Open all four pools: `LoginDatabase`, `CharacterDatabase`, `WorldDatabase`, `HotfixDatabase`; read `RealmID`; call `ClearOnlineAccounts`; insert version info; `sWorld->LoadDBVersion()` | DatabaseLoader, World |
| `StopDB()` | Close in reverse order: Hotfix, World, Character, Login | — |
| `StartRaSocketAcceptor(io)` | Bind+listen on `Ra.IP:Ra.Port` (default 0.0.0.0:3443); `acceptor->AsyncAccept<RASession>()` | AsyncAcceptor |
| `LoadRealmInfo()` | Copy this realm's row out of `sRealmList` into the global `realm` struct | RealmList |
| `ClearOnlineAccounts()` | DB cleanup: `account.online = 0` for accounts with chars on this realm; `characters.online = 0`; reset `character_battleground_data.instanceId` | LoginDatabase, CharacterDatabase |
| `ShutdownCLIThread(thread*)` | Cancel pending stdin read on Windows (via `CancelSynchronousIo` + simulated keypress); `join()` + `delete` on Linux | — |
| `CliThread()` | Reads lines from stdin, posts to `World::QueueCliCommand`; supports `quit`, `exit`, `server *`, etc. | World |
| `RASession::Start()` | TCP connect → request username/password → `CheckAccessLevel` (gmlevel 3+ only) → loop reading commands and feeding to `CliCommandHolder` | World, AccountMgr |
| `TCSoapThread(host, port)` | Spawn gSOAP server, `process_message` per request, set `SOAPCommand` promise on completion | World |

### `main()` step-by-step (TC behaviour)

1. Install `SIGABRT` → `Trinity::AbortHandler` (writes a coredump preamble).
2. `Trinity::VerifyOsVersion()` — refuses to run on too-old Win.
3. `Trinity::Locale::Init()` — sets `setlocale(LC_ALL, "C")`.
4. Parse CLI: `--config`, `--config-dir`, `--update-databases-only`, `--service`, `--help`, `--version`.
5. `GOOGLE_PROTOBUF_VERIFY_VERSION` + RAII shutdown handle.
6. Win32: install/uninstall/run service if requested. Set `timeBeginPeriod(1ms)` for sub-15ms `Sleep` resolution.
7. `sConfigMgr->LoadInitial(configFile, argv)` then `LoadAdditionalDir(configDir)` then `OverrideWithEnvVariablesIfAny()`.
8. **Build a single `boost::asio::io_context`** shared by network, log async, signal handler, freeze detector, metrics.
9. `sLog->RegisterAppender<AppenderDB>(); sLog->Initialize(asyncLogIo)`.
10. `Trinity::Banner::Show("worldserver-daemon", ...)`.
11. `OpenSSLCrypto::threadsSetup(...)` (RAII `threadsCleanup`).
12. `BigNumber seed; seed.SetRand(16*8)` — pre-seed the OpenSSL PRNG so first login isn't slow.
13. `CreatePIDFile(PidFile)` if config sets it.
14. `boost::asio::signal_set signals(*io, SIGINT, SIGTERM)` → `async_wait(SignalHandler)`.
15. **`ThreadPool threadPool(numThreads = ThreadPool config or 1)`** + post `numThreads` workers each calling `io->run()`. RAII handle stops the io_context on drop.
16. `SetProcessPriority(...)` (affinity + high-priority flag).
17. `StartDB()`. RAII `StopDB`. If `--update-databases-only`, exit.
18. `Trinity::Net::ScanLocalNetworks()` — caches local subnets for `realm` IP-routing decisions.
19. `LoginDatabase.DirectPExecute("UPDATE realmlist SET flag = flag | OFFLINE WHERE id = realm.Id")` — set realm offline for boot duration.
20. `sRealmList->Initialize(*io, RealmsStateUpdateDelay)`.
21. `LoadRealmInfo()`.
22. `sMetric->Initialize(realmName, *io, lambda)` — emits `online_players`, `db_queue_*` periodically.
23. `sScriptMgr->SetScriptLoader(AddScripts)`. RAII unloads ScriptMgr + ScriptReloadMgr.
24. `sSecretMgr->Initialize(SECRET_OWNER_WORLDSERVER)`.
25. **`sWorld->SetInitialWorldSettings()` — the big one.** Loads ~150 DB tables, all DB2 stores, scripts, etc.
26. RAII handle that on shutdown unloads BG templates, OutdoorPvP, MapMgr, TerrainMgr, InstanceLockMgr.
27. `if (Ra.Enable) StartRaSocketAcceptor(*io)`.
28. `if (SOAP.Enabled) std::thread(TCSoapThread, ip, port)` (joined by RAII).
29. `sWorldSocketMgr.StartWorldNetwork(*io, BindIP, WorldServerPort, InstanceServerPort, NetworkThreads)`.
30. RAII shutdown: `KickAll`, `UpdateSessions(1)` to flush, `StopNetwork`, `ClearOnlineAccounts`.
31. `LoginDatabase.DirectPExecute(... clear OFFLINE flag ...)`.
32. `if (MaxCoreStuckTime > 0) FreezeDetector::Start(...)`.
33. `sScriptMgr->OnStartup()`.
34. `if (Console.Enable) std::thread(CliThread)` (RAII shutdown).
35. **`WorldUpdateLoop()`** — blocks until shutdown.
36. Cleanup: `ConnectTo::ShutdownEncryption()`, `EnterEncryptedMode::ShutdownEncryption()`, drop `ioContextStopHandle`, drop `threadPool`, `sLog->SetSynchronous()`, `sScriptMgr->OnShutdown()`, set realm OFFLINE, return `World::GetExitCode()`.

---

## 5. Module dependencies

**Depends on:**
- Everything in `src/server/game/` (transitively): `World`, `WorldSocket`, `WorldSocketMgr`, `MapManager`, `TerrainMgr`, `BattlegroundMgr`, `OutdoorPvPMgr`, `InstanceLockMgr`, `ScriptMgr`, `ScriptReloadMgr`, `Metric`
- `src/common/`: `Config`, `Log` (`AppenderDB`), `DatabaseLoader`, `LoginDatabase` / `CharacterDatabase` / `WorldDatabase` / `HotfixDatabase`, `MySQLThreading`, `OpenSSLCrypto`, `SecretMgr`, `IoContext`, `AsyncAcceptor`, `ThreadPool`, `ProcessPriority`, `Realm`, `RealmList`, `IpNetwork`, `Banner`, `BigNumber`, `Locale`
- `boost`: `asio`, `program_options`, `dll`, `filesystem`
- `OpenSSL`, `protobuf`
- `gsoap` (for TCSoap)

**Depended on by:**
- The WoW client (game-state side; the launcher hits bnetserver, in-game session hits worldserver)
- An optional supervisor (systemd / runit) that restarts the binary on freeze-detector abort

---

## 6. SQL / DB queries (if any)

Direct queries in `Main.cpp` (the rest is in modules):

| Statement / Source | Purpose | DB |
|---|---|---|
| `UPDATE realmlist SET flag = flag \| OFFLINE WHERE id = ?` | Mark realm offline at boot / shutdown | login |
| `UPDATE realmlist SET flag = flag & ~OFFLINE, population = 0 WHERE id = ?` | Mark realm online after listener starts | login |
| `UPDATE version SET core_version = ?, core_revision = ?` | One-shot version-info record | world |
| `UPDATE account SET online = 0 WHERE online > 0 AND id IN (SELECT acctid FROM realmcharacters WHERE realmid = ?)` | Cleanup: clear stale online flags | login |
| `UPDATE characters SET online = 0 WHERE online <> 0` | Cleanup: clear stale character online flags | characters |
| `UPDATE character_battleground_data SET instanceId = 0` | BG instance ids are per-restart | characters |

DBC/DB2 stores: not loaded here; loaded by `World::SetInitialWorldSettings` (covered in `world.md`).

DBUpdater (auto-applies pending `.sql` files) is invoked by `DatabaseLoader::Load()` for each pool. See `wow-database/src/updater.rs` for the Rust port.

---

## 7. Wire-protocol packets (if any)

`worldserver` doesn't define opcodes. It opens listeners on:

| Port | Protocol | Used by |
|---|---|---|
| `WorldServerPort` (def. 8085) | Encrypted WoW TCP (header HMAC-SHA256, body AES-GCM) | The WoW 3.4.3.54261 client realm connection |
| `InstanceServerPort` (def. 8086) | Same, but for instance-bound transfers (`SMSG_REDIRECT_CLIENT` / `SMSG_NEW_WORLD`) | Same client, after `ConnectTo` |
| `Ra.Port` (def. 3443) | Plain TCP, telnet-like | `RASession` |
| `SOAP.Port` (def. 7878) | HTTP SOAP | gsoap |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/world-server/src/main.rs` — 818 lines — entry point, async runtime, account-lookup struct, listener spawn, session create

**What's implemented:**
- Tokio async runtime (`#[tokio::main]`), `tracing` subscriber.
- Config loading (`WorldServer.conf` then `WorldServer.conf.dist` fallback).
- Four DB pools opened: login, characters, world, hotfix.
- `wow_database::updater::DbUpdater::populate(...)` + `update(...)` for the auth and characters DBs (and `update` only for world + hotfix). Equivalent to TC's `DatabaseLoader` + `DBUpdater` running auto-update.
- `ObjectGuidGenerator` initialized from `MAX(guid)` in `characters`.
- DB2 / static-data stores loaded into `Arc`s and shared via `SessionResources`: `ItemStore`, `PlayerStatsStore`, `ItemStatsStore`, `HotfixBlobCache`, `SkillStore`, `SpellStore`, `area_trigger_store`, `quest_store`, `QuestXpStore`, `player_xp_table`.
- Realm gamebuild + `Win64AuthSeed` loaded from `realmlist` + `build_info`.
- Realm external/local addresses loaded from `realmlist`.
- `wow_handler::build_dispatch_table()` constructs the per-opcode handler registry (same role as TC's `OpcodeTable::Initialize()`).
- `DbAccountLookup` impl of `wow_network::world_socket::AccountLookup` — resolves `(realm_id, ticket)` → `AccountInfo` (account_id, session_key_bnet, expansion, security, ban flags, locale, OS, …).
- Two listeners spawned: `start_world_listener(realm_addr, ...)` and `start_instance_listener(instance_addr, ...)` — see `wow-network` for the accept-loop implementation.
- Per-accepted-connection `create_session(...)` builds a `WorldSession`, wires all the resource Arcs into it, calls `send_session_init_packets` and runs a 50 ms tick loop:
  - `session.update(50)` — process inbound queued packets
  - `session.process_pending().await` — async DB callbacks
  - `if disconnecting break` else `tokio::time::sleep(50ms)`
- Shutdown via `tokio::select! { ctrl_c => ..., listener_join => ... }`.
- `get_address_for_client` — replicates TC's `Trinity::Net::SelectAddressForClient`-style "loopback or same /24 → local, else external".

**What's missing vs C++:**
- **`World::Update(diff)` global tick loop.** TC's `WorldUpdateLoop` is a single thread that drives **all** gameplay state for the whole server. RustyCore's design is per-session: each session runs its own 50 ms tick. There is **no global `WorldSessionMgr::Update`, no `MapManager::Update`, no global creature ticks aggregated across sessions, no `World::m_worldLoopCounter`**. This is the most fundamental architectural divergence between the two stacks. The `_attic/` README documents the partial migration toward `MapManager`-shared state; the global-tick driver is still future work.
- **`FreezeDetector`** — there is no equivalent. If the runtime hangs, only an external supervisor (systemd `WatchdogSec=`) catches it.
- **`MaxCoreStuckTime` config**: read but ignored.
- **`MinWorldUpdateTime` config**: ignored. The Rust tick is hardcoded `Duration::from_millis(50)`.
- **`World::IsStopped()` / `World::StopNow(code)` / `World::GetExitCode()`**: there is no `World` singleton. Shutdown is just `ctrl_c` → drop listeners. Exit code always 0.
- **DB keep-alive timer (`MaxPingTime`)**: not implemented (see same gap in bnetserver).
- **`AppenderDB`** (logs into `logs.logs` table): not implemented; tracing only goes to stderr / journald.
- **`Banner::Show`**: only a single `info!("RustyCore World Server starting...")`.
- **`OpenSSLCrypto` setup / `BigNumber::SetRand(...)` warmup**: irrelevant (rustls / `getrandom`).
- **`CreatePIDFile` present**: Rust reads `PidFile`, writes the current process id before DB/network startup, and aborts startup on write failure like TC.
- **`SecretMgr::Initialize(SECRET_OWNER_WORLDSERVER)`**: missing.
- **`ScanLocalNetworks`**: there is `get_address_for_client` which approximates TC's behaviour but only checks `/24` against `realm_local_address`, not the full set of host interfaces.
- **`ClearOnlineAccounts` present** at boot and shutdown: Rust clears `account.online` for accounts with characters on this realm, clears `characters.online`, and resets `character_battleground_data.instanceId` like TC.
- **`UPDATE realmlist SET flag = flag | OFFLINE`** at boot / `& ~OFFLINE` after listener-up / on shutdown: present; Rust marks the realm offline after DB cleanup, online after listeners/runtime tasks are up, and offline again during shutdown.
- **`LoadRealmInfo`** equivalent: only loads gamebuild/seed/addresses; doesn't populate a global `realm` struct equivalent — addresses are passed via `SessionResources`.
- **`sRealmList->Initialize(io, RealmsStateUpdateDelay)`** background refresh of cross-realm registry: missing. (Single-realm setups don't notice; multi-realm wouldn't work.)
- **`sMetric->Initialize`** — no metrics subsystem.
- **`sScriptMgr->OnStartup() / OnShutdown()`** hooks: missing (the wow-script dispatch infrastructure exists but startup hook isn't wired).
- **`Trinity::Asio::DeadlineTimer`-driven periodic tasks**: none of TC's housekeeping timers are present (DB ping, metric tick, script reload check).
- **CLI thread (`CliThread`)** reading from stdin and dispatching `.commands`: missing. There's no GM console.
- **Remote Access (`Ra.Enable`, port 3443)**: missing.
- **SOAP (`SOAP.Enabled`, port 7878)**: missing.
- **Win32 service mode**: out of scope.
- **`SetProcessPriority` / processor affinity**: out of scope.
- **`ThreadPool` config**: ignored — Tokio handles threading.
- **CLI args (`--config`, `--update-databases-only`, `--version`, `--help`)**: not parsed.
- **`KickAll` + `UpdateSessions(1)` flush at shutdown**: missing — sessions are dropped abruptly.
- **`WorldPackets::Auth::ConnectTo::ShutdownEncryption / EnterEncryptedMode::ShutdownEncryption`** — these tear down the static keys. RustyCore probably has them per-session, so the global tear-down may be a no-op.
- **`sBattlegroundMgr->DeleteAllBattlegrounds()`, `sOutdoorPvPMgr->Die()`, `sMapMgr->UnloadAll()`, `sTerrainMgr.UnloadAll()`, `sInstanceLockMgr.Unload()` in shutdown order**: only a partial `MapManager` exists; rest is missing.
- **`AbortHandler` on `SIGABRT`**: missing.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The 50 ms-per-session sleep is **independent across sessions** — there's no global "we ticked everyone simultaneously". If a 25-player raid all has slightly different tick offsets, AoE/aura ticks visible to different clients will land on different real-time millisecond boundaries. Likely visible in PvP combat timing.
- Without `MapManager` running its own tick (creature AI / spawn respawn / movement interpolation between sessions), creatures only "update" when *some* session looks at them. Idle creatures freeze.
- `tokio::time::sleep(Duration::from_millis(50))` is the *floor*, not the period. If `session.update(50)` blocks for >50 ms, the next tick is delayed. TC's `MinWorldUpdateTime` enforces a *floor* explicitly; we get the same effective behaviour, but TC also logs a warning if `sleepTime >= MaxCoreStuckTime / 2`.
- Without `sRealmList`, the BNet realm-list response is built from a stale snapshot loaded once at boot; adding a realm row to MySQL won't be picked up.
- Database connection pool sizing: TC opens three pools per database (`SyncPool` + `AsyncPool` + a small one for callbacks). The Rust `wow-database` opens a single `sqlx::Pool<MySql>` per logical DB. Sizing under heavy login load may differ.

**Tests existing:**
- 0 in `world-server` itself; integration tests live in `wow-world` and `wow-network`.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#WORLDSERVER.WBS.001** Cerrar la migracion auditada de `worldserver/CommandLine/CliRunnable.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.002** Cerrar la migracion auditada de `worldserver/CommandLine/CliRunnable.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.h`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.003** Partir y cerrar la migracion auditada de `worldserver/Main.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 742 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.004** Cerrar la migracion auditada de `worldserver/RemoteAccess/RASession.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.005** Cerrar la migracion auditada de `worldserver/RemoteAccess/RASession.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.h`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.006** Cerrar la migracion auditada de `worldserver/TCSoap/TCSoap.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/TCSoap/TCSoap.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.007** Cerrar la migracion auditada de `worldserver/TCSoap/TCSoap.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/TCSoap/TCSoap.h`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLDSERVER.WBS.008** Cerrar la migracion auditada de `worldserver/resource.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/resource.h`
  Rust target: `crates/world-server`, `crates/wow-network`, `crates/wow-database`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#WS.1** Implement `World` singleton (or equivalent state holder): `is_stopped() -> bool`, `stop_now(exit_code: i32)`, `get_exit_code() -> i32`, `m_world_loop_counter: AtomicU32`. (M)
- [ ] **#WS.2** Implement a global `WorldUpdateLoop` task that ticks `MapManager` + (eventually) every `WorldSession` from one place, at a `MinWorldUpdateTime` cadence. Increment the loop counter each iteration. (XL — coupled to migrating sessions off per-task ticks; cf. `_attic/` notes)
- [ ] **#WS.3** Implement `FreezeDetector`: `tokio::time::interval(1s)`; reads `m_world_loop_counter`; if unchanged for `MaxCoreStuckTime` ms, `tracing::error!` + `std::process::abort()`. (M)
- [x] **#WS.4** Implement `ClearOnlineAccounts()` — called at boot and shutdown; mirrors TC's three queries: account online flags for this realm, character online flags, and battleground instance ids.
- [x] **#WS.5** Implement realmlist OFFLINE flag toggle at boot + listener-ready + shutdown: mirrors TC's `flag | OFFLINE` at boot/shutdown and `flag & ~OFFLINE, population = 0` once connectable.
- [ ] **#WS.6** DB keep-alive: every `MaxPingTime` minutes, `SELECT 1` against each of the 4 pools. (L)
- [ ] **#WS.7** Implement `AppenderDB` equivalent for `tracing`: a layer that batches log records into `logs.logs` table. Optional. (M)
- [ ] **#WS.8** Add CLI thread: `tokio::task::spawn_blocking` reading stdin, posting commands to a `CliCommandQueue` consumed in the main tick. Wire a small set of commands first (`server info`, `server shutdown`, `account create`). (H)
- [ ] **#WS.9** Implement RA listener (`Ra.Enable`): bind on `Ra.IP:Ra.Port`, per-connection auth (gmlevel ≥ `Ra.MinLevel`), pipe commands into the same `CliCommandQueue`. (H)
- [ ] **#WS.10** Decide on SOAP: drop or implement. The XML/SOAP surface is rarely used today; recommend **drop** with config warning. (L if drop, XL if implement)
- [ ] **#WS.11** Wire `wow_account::secrets::initialize(SecretOwner::WorldServer).await` (depends on `accounts.md` #ACC.6 / new SecretMgr port). (M)
- [ ] **#WS.12** Wire `sRealmList`-equivalent background refresh (`RealmsStateUpdateDelay`): polls `realmlist` periodically, updates an in-memory `Arc<RwLock<Vec<RealmEntry>>>`. Needed once RustyCore supports more than one realm row. (M)
- [ ] **#WS.13** Implement `sMetric` equivalent: emit `online_players`, `db_queue_*` to an OpenTelemetry / Prometheus exporter. (M)
- [ ] **#WS.14** Implement `sScriptMgr->on_startup()` / `on_shutdown()` hooks. (L)
- [ ] **#WS.15** Implement clean shutdown: kick all sessions (send `SMSG_LOGOUT_RESPONSE` then drop), wait up to N seconds for character saves, close listeners, drop registries, close DBs, set realm OFFLINE. (H)
- [ ] **#WS.16** CLI args via `clap`: `--config`, `--config-dir`, `--update-databases-only`, `--version`, `--help`. (L)
- [x] **#WS.17** PID file (`PidFile` config): writes `std::process::id()` before DB/network startup and fails startup if the file cannot be created.
- [x] **#WS.18** `SIGTERM` handler in addition to `ctrl_c`: Unix `SIGTERM` and Ctrl-C both drive the same shutdown branch.
- [ ] **#WS.19** Pre-listener startup banner with build hash, sqlx version, rustls version, DB versions (one log line per connected DB). (L)
- [ ] **#WS.20** Replace per-session `tokio::time::sleep(50ms)` with a `tokio::sync::broadcast` "tick" signal driven by the global `WorldUpdateLoop`. (M, depends on #WS.2)
- [ ] **#WS.21** Connection-pool sizing: expose `LoginDatabaseInfo.{Sync,Async}.PoolSize` config; pass to `sqlx::PoolOptions`. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#WORLDSERVER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 8 files / 1434 lines; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp`. Rust target: `crates/world-server`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | `cargo test -p world-server && cargo test -p wow-database && cargo test -p wow-network` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#WORLDSERVER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 8 files / 1434 lines; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp`. Rust target: `crates/world-server`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#WORLDSERVER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 8 files / 1434 lines; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp`. Rust target: `crates/world-server`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#WORLDSERVER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 8 files / 1434 lines; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp`. Rust target: `crates/world-server`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `World::is_stopped()` flips to `true` on `SIGINT` and the `WorldUpdateLoop` exits within one tick interval.
- [ ] Test: `FreezeDetector` aborts the process if the global tick counter doesn't advance for > `MaxCoreStuckTime` ms (use a deliberately blocking handler in a test build).
- [ ] Test: `ClearOnlineAccounts` at boot zeroes `account.online` for accounts with characters on the current realm and **only** that realm.
- [ ] Test: realmlist `flag` column has `REALM_FLAG_OFFLINE` set during startup (after `StartDB`, before `WorldUpdateLoop`) and again after shutdown.
- [ ] Test: DB keep-alive ping fires every `MaxPingTime` minutes (use `tokio::time::pause` for time-skipping).
- [ ] Test: with `Console.Enable=true`, sending `server info\n` to stdin produces an info log; `server shutdown\n` triggers `World::stop_now(SHUTDOWN_EXIT_CODE)`.
- [ ] Test: with `Ra.Enable=true`, a TCP connection to `Ra.Port` requires a username + password, validates `gmlevel >= Ra.MinLevel`, then runs the same command surface.
- [ ] Test: shutdown sequence — when SIGINT arrives, every connected session receives `SMSG_LOGOUT_RESPONSE`, character data is saved, then listeners close. Total time bounded by `ShutdownLatencyMax`.
- [ ] Test: `cargo run --bin world-server -- --update-databases-only` runs DBUpdater for all four DBs and exits 0 without binding any port.
- [ ] Test: `cargo run --bin world-server -- --version` prints `RustyCore worldserver <hash>` and exits.
- [ ] Test: with `MaxCoreStuckTime=0`, `FreezeDetector::Start` is not called (TC behaviour: 0 disables).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 8 files / 1434 lines; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp` | `crates/world-server/` \| ⚠️ partial (boot, DB init, listener spawn, session-per-connection work; freeze detector + RA + SOAP + CLI thread + `World::Update` tick loop are all missing) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#WORLDSERVER.DIV.001` | _none generated_ | 8 C++ files / 1434 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/RemoteAccess/RASession.cpp`, `/home/server/woltk-trinity-legacy/src/server/worldserver/CommandLine/CliRunnable.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **The `World::Update` tick is the single source of truth for game time.** TC's `getMSTime()` is sampled once per loop iteration; everything inside that iteration sees the same `realCurrTime`. Per-session ticks in RustyCore mean each session has its own time. Combat timing, aura tick alignment, AoE radii on moving targets — all have subtle dependencies on a global tick. Plan the migration to a global tick early or expect bugs that "only show up in raids".
- **`MinWorldUpdateTime` default is 1 ms**, but TC uses `getMSTime` which is *millisecond-resolution*. So the loop effectively runs as fast as the OS scheduler allows. `MaxCoreStuckTime` default is 60 seconds.
- **`ABORT_MSG` in `FreezeDetector::Handler` is intentional**: TC wants the process to crash and be restarted by the supervisor. This is more reliable than trying to recover. Replicate this verbatim — `std::process::abort()` (which dumps core), not `std::process::exit(1)`.
- **The freeze-detector polls `World::m_worldLoopCounter`, not "is the io_context running"**. If only the game logic stalls (DB query stuck, infinite loop in a script), but the network thread keeps reading, the freeze detector still catches it because it's gated on the counter. Replicate that property — *don't* gate it on Tokio's I/O.
- **Shutdown order matters**: BattlegroundMgr → OutdoorPvP → MapMgr → TerrainMgr → InstanceLockMgr. Reversing gives use-after-free in TC, where each manager holds raw pointers into the previous. Rust's borrow checker will help you find equivalent ordering bugs at compile time, but the semantic ordering still matters because of background tasks.
- **TC's `Trinity::ThreadPool`** runs N threads against a single `io_context`. **Tokio is not the same** — Tokio's worker count is set at runtime construction, and there's no equivalent of "this thread polls io_context until shutdown". Don't try to expose `Network.Threads` literally.
- **`UPDATE realmlist SET flag = flag | OFFLINE`** at startup tells the bnetserver "don't list this realm to clients". The `flag = flag & ~OFFLINE` after the listener is up reverses it. If you crash between the two, the realm stays OFFLINE in the DB until the next clean run.
- **WoW client behaviour at shutdown**: if you just close the socket, the client shows "connection lost" and may try to reconnect. To trigger the proper "World server is shutting down" UI, you have to send `SMSG_NOTIFICATION_TEXT` or the queued-shutdown packets from `World::ShutdownServ`. Currently RustyCore does neither.
- **DB connection-pool sizing**: under heavy login (server reboot at peak), all four pools see a thundering herd. TC tunes `Login.SynchPool=1, Login.AsyncPool=4`. The default sqlx `max_connections=10` may be too low or too high depending on workload.
- **`character_battleground_data.instanceId = 0`** at boot: if a player was inside a BG when the server crashed, this lets them log back into a fresh location instead of into the dead BG instance.
- **`Console.Enable`** on Linux: the C++ `CliThread` reads from stdin via `fgets`. If the binary is run via `nohup` or in a systemd unit without `StandardInput=tty`, stdin reads return EOF immediately and the thread exits. Replicate this: `tokio::task::spawn_blocking` reading from stdin, gracefully exit on EOF, do **not** trigger shutdown.
- **`AsyncAcceptor`** in TC is a thin wrapper over `boost::asio::tcp::acceptor`. The Rust counterpart is whatever `wow-network` exposes (`start_world_listener`, `start_instance_listener`).
- **All four DB connections must succeed** for the binary to start. Failure to reach `world` or `hotfixes` is fatal. Currently `world-server/main.rs` matches this — `.context(...)?` short-circuits.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `int main(int argc, char** argv)` | `#[tokio::main] async fn main() -> anyhow::Result<()>` | — |
| `boost::asio::io_context` (single, shared) | Tokio multi-thread runtime (implicit) | TC has *one* io_context for everything; Rust just spawns tasks. |
| `Trinity::ThreadPool threadPool(N)` posting `io->run()` | Tokio worker thread count (`tokio::runtime::Builder::worker_threads`) | Don't expose `ThreadPool` config literally. |
| `boost::asio::signal_set signals(io, SIGINT, SIGTERM)` + `async_wait` | `tokio::select! { _ = tokio::signal::ctrl_c() => ..., _ = sigterm.recv() => ..., }` | Both must drive the same shutdown path. |
| `class FreezeDetector { DeadlineTimer; uint32 _worldLoopCounter; }` | `struct FreezeDetector { last_change_ms: AtomicU64, max_stuck_ms: u64 }` polled by a `tokio::time::interval(1s)` task | Increment a global `AtomicU32` from the global tick. |
| `World::m_worldLoopCounter` | `static WORLD_LOOP_COUNTER: AtomicU32` (or field on a `World` singleton) | — |
| `WorldUpdateLoop()` | `async fn world_update_loop(world: Arc<World>)` running until `world.is_stopped()` | The body uses `tokio::time::sleep` for floor enforcement, *not* an `interval` (interval would catch up on missed ticks). |
| `World::StopNow(code)` | `World::stop_now(&self, code: i32)` setting `AtomicBool::store(true)` and `AtomicI32::store(code)` | — |
| `Trinity::Asio::DeadlineTimer` (recursive rearm) | `tokio::time::interval` + `loop { tick().await; ... }` | The recursive `async_wait`-rearm pattern in TC becomes a flat loop. |
| `class RASession` | `async fn ra_session(stream: TcpStream)` task | Plain TCP. Auth + `match cmd { ... }`. |
| `void TCSoapThread(host, port)` | (recommend dropping) | If kept: `axum` server with a SOAP-XML handler. |
| `void CliThread()` | `tokio::task::spawn_blocking(stdin_reader)` + a `tokio::sync::mpsc` channel | Don't `.await` blocking stdin in async code. |
| `sWorld->SetInitialWorldSettings()` | The accumulation of all `*Store::load(...)` calls + handler-table build | Lives in `world.md` — already partly done. |
| `sScriptMgr->OnStartup() / OnShutdown()` | `wow_scripts::lifecycle::{on_startup, on_shutdown}().await` | Both currently no-ops. |
| `sMetric->Initialize(realmName, io, lambda)` | `wow_metric::initialize(realm_name, ||{ emit_periodic() })` (TODO crate) | — |
| `Trinity::Net::ScanLocalNetworks()` | `wow_network::scan_local_networks()` (TODO; only `/24` heuristic right now) | — |
| `LoginDatabase.DirectPExecute("UPDATE realmlist ...", flag, realmId)` | `login_db.direct_execute(&format!("UPDATE realmlist SET ... WHERE id = {realm_id}"))?` | Already used in `load_realm_addresses`. |
| `MySQL::Library_Init() / End()` | (none) | sqlx handles libmysqlclient internally; nothing to call. |
| `BigNumber seed; seed.SetRand(16*8)` | (none) | rustls / `getrandom` seed automatically. |
| `OpenSSLCrypto::threadsSetup(...)` | (none) | rustls. |
| `ABORT_MSG("World Thread hangs ...")` | `tracing::error!(...); std::process::abort();` | `abort()` not `exit(1)` — for coredump. |
| `sLog->Initialize(asyncIo)` + `AppenderDB` | `tracing_subscriber::fmt().init()` (currently no DB sink) | TODO #WS.7. |
| `CreatePIDFile(path)` | `std::fs::write(path, std::process::id().to_string())?` | Implemented in `create_pid_file_like_cpp`; called only when `PidFile` is non-empty. |
| `boost::program_options::variables_map` | `clap::Parser` | TODO #WS.16. |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Audited:**
- C++: `/home/server/woltk-trinity-legacy/src/server/worldserver/Main.cpp` (742 lines), `World.cpp::Update`, `World.cpp::UpdateSessions`.
- Rust: `/home/server/rustycore/crates/world-server/src/main.rs` (818 lines), `crates/wow-world/src/session.rs::WorldSession::update` (line 1063), `crates/wow-world/src/map_manager.rs` (no `update` / `tick` method exists), `crates/wow-database/src/database.rs::Database::open` (line 36), `crates/wow-database/src/updater.rs`.

### 13.1 Audit summary

The doc body's pre-audit hypothesis was correct on the most important point: **there is no `World::Update(diff)` global tick driver in RustyCore.** Each `WorldSession` is owned by its own per-connection Tokio task, runs `session.update(50)` followed by `tokio::time::sleep(50ms)`, and that is the only thing driving creature AI, combat, and aura ticks (`session.rs:1109-1124` calls `tick_creatures_sync`, `tick_combat_sync`, `tick_auras` modulo `creature_tick`). `MapManager` exists as shared state (`SharedMapManager = Arc<RwLock<MapManager>>`) but has **no `update()` / `tick()` method at all** — it is a passive container of grids and creatures, never updated from a single source. The `m_worldLoopCounter` analogue does not exist. Sessions independently tick "their" creatures, which means an idle creature on a map with no nearby session simply does not tick.

Otherwise the boot sequence is largely on-parity for what's implemented (4 DB pools, DB updater, DB2/DBC store loads, dispatch table, listeners on 8085 + 8086, ConnectTo flow, `get_address_for_client` heuristic). The big gaps are in lifecycle/operational infrastructure: no freeze detector, no realmlist OFFLINE flag toggle, no CLI/RA/SOAP, no graceful shutdown drain (sessions are dropped abruptly when listeners close), no `--config` / `--update-databases-only` CLI args.

### 13.2 Startup parity

| TC step (Main.cpp) | Rust equivalent | Parity |
|---|---|---|
| `signal(SIGABRT, AbortHandler)` | — | ❌ missing |
| `Trinity::Locale::Init()` | — | ❌ irrelevant (Rust uses UTF-8 by default) |
| Parse `--config`, `--update-databases-only`, `--version` | — | ❌ missing (#WS.16) |
| Win32 service / `timeBeginPeriod(1ms)` | — | n/a (Linux only) |
| `sConfigMgr->LoadInitial(...) + LoadAdditionalDir + OverrideEnv` | `wow_config::load_config("WorldServer.conf")` w/ `.dist` fallback | ⚠️ no `conf.d/` dir, no env override |
| `boost::asio::io_context` shared | implicit Tokio runtime | ✅ acceptable divergence |
| `sLog->RegisterAppender<AppenderDB>(); Initialize(asyncIo)` | `tracing_subscriber::fmt().with_env_filter(...)` | ⚠️ no DB sink (#WS.7) |
| `Trinity::Banner::Show(...)` | one `info!("RustyCore World Server starting...")` | ⚠️ #WS.19 |
| `OpenSSLCrypto::threadsSetup` + `BigNumber::SetRand` warmup | — | ✅ irrelevant (rustls + getrandom) |
| `CreatePIDFile(PidFile)` | `create_pid_file_from_config_like_cpp` before DB/network startup | ✅ |
| `signal_set(SIGINT, SIGTERM)` | `shutdown_signal()` waits for Ctrl-C or Unix `SIGTERM` | ✅ |
| `ThreadPool(numThreads)` posting `io->run()` | implicit Tokio workers | ✅ acceptable divergence (don't expose `Network.Threads` literally) |
| `SetProcessPriority(...)` | — | ❌ out of scope |
| `StartDB()` opens 4 pools (Login/Character/World/Hotfix) | `LoginDatabase::open` + `CharacterDatabase::open` + `WorldDatabase::open` + `HotfixDatabase::open` (lines 177-228) | ✅ four pools present |
| `DatabaseLoader::Load()` runs `DBUpdater` per pool | `DbUpdater::new(...).populate(...).await` + `update(...).await` for auth/characters; `update` only for world/hotfix (lines 232-272) | ✅ implemented |
| `realm.Id.Realm` from config; bail if 0 | `RealmID` config (line 390); defaults to 1, no validation | ⚠️ no zero-check |
| `--update-databases-only` early exit | — | ❌ missing (#WS.16) |
| `Trinity::Net::ScanLocalNetworks()` | `get_address_for_client` /24 heuristic (line 757) | ⚠️ partial |
| `UPDATE realmlist SET flag\|=OFFLINE` at boot | `set_realm_offline(&login_db, realm_id)` after DB cleanup | ✅ |
| `sRealmList->Initialize(io, RealmsStateUpdateDelay)` background refresh | — | ❌ missing (#WS.12) |
| `LoadRealmInfo()` | `load_realm_auth_seed` + `load_realm_addresses` (lines 530, 728) | ⚠️ partial (no global `realm` struct) |
| `sMetric->Initialize(realmName, io, lambda)` | — | ❌ missing (#WS.13) |
| `sScriptMgr->SetScriptLoader(AddScripts)` | — | ❌ missing (script registration is implicit but `OnStartup`/`OnShutdown` hooks aren't called) |
| `sSecretMgr->Initialize(SECRET_OWNER_WORLDSERVER)` | — | ❌ missing (#WS.11) |
| `sWorld->SetInitialWorldSettings()` (the big one) | scattered: `ItemStore::load`, `PlayerStatsStore::load`, `ItemStatsStore::load`, `build_hotfix_blob_cache`, `SkillStore::load`, `SpellStore::load`, `load_area_triggers`, `quest::load_quests`, `QuestXpStore::load` (lines 302-388) | ⚠️ partial (covered by `world.md`) |
| `if (Ra.Enable) StartRaSocketAcceptor(io)` | — | ❌ missing (#WS.9) |
| `if (SOAP.Enabled) std::thread(TCSoapThread, ...)` | — | ❌ recommend drop (#WS.10) |
| `sWorldSocketMgr.StartWorldNetwork(io, ip, worldPort, instancePort, networkThreads)` | `start_world_listener(realm_addr, ...)` + `start_instance_listener(instance_addr, ...)` (lines 473-505) | ✅ functional equivalence |
| `UPDATE realmlist SET flag &= ~OFFLINE` after listener | `set_realm_online(&login_db, realm_id)` after listeners/runtime tasks are spawned | ✅ |
| `if (MaxCoreStuckTime > 0) FreezeDetector::Start(...)` | — | ❌ missing (#WS.3) |
| `sScriptMgr->OnStartup()` | — | ❌ missing (#WS.14) |
| `if (Console.Enable) std::thread(CliThread)` | — | ❌ missing (#WS.8) |
| `WorldUpdateLoop()` (the meat) | per-session `loop { session.update(50); session.process_pending().await; sleep(50ms); }` | ❌ **breaking divergence** |

### 13.3 Shutdown parity

| TC step | Rust equivalent | Parity |
|---|---|---|
| `signals.async_wait(SignalHandler)` → `World::StopNow(SHUTDOWN_EXIT_CODE)` | `shutdown_signal()` handles Ctrl-C + Unix SIGTERM, then drops listener handles | ⚠️ no global stop flag, sessions don't see "stopping" state |
| `sWorld->KickAll()` (save + send logout) | — | ❌ missing (#WS.15) |
| `sWorld->UpdateSessions(1)` final flush | — | ❌ missing |
| `sWorldSocketMgr.StopNetwork()` | listener task drop | ⚠️ implicit, no drain |
| `ClearOnlineAccounts()` | `clear_online_accounts_like_cpp` at boot + shutdown | ✅ |
| `WorldPackets::Auth::ConnectTo::ShutdownEncryption()` / `EnterEncryptedMode::ShutdownEncryption()` | — | ✅ irrelevant (per-session keys in Rust) |
| `ioContextStopHandle.reset()` | — | ✅ implicit (Tokio runtime drops) |
| `threadPool.reset()` | — | ✅ implicit |
| `sLog->SetSynchronous()` | — | ⚠️ tracing flush not explicit |
| `sScriptMgr->OnShutdown()` | — | ❌ missing (#WS.14) |
| `UPDATE realmlist SET flag\|=OFFLINE` on exit | `set_realm_offline(&login_db, realm_id)` in the shared shutdown branch | ✅ |
| `BattlegroundMgr::DeleteAllBattlegrounds → OutdoorPvPMgr::Die → MapMgr::UnloadAll → TerrainMgr::UnloadAll → InstanceLockMgr::Unload` | only partial `MapManager` exists; rest missing | ❌ missing |
| `return World::GetExitCode()` | always `Ok(())` (exit code 0) | ⚠️ no error-path code |

### 13.4 Main loop architectural divergence — verdict

**Verdict: BREAKING DIVERGENCE.**

TC's `WorldUpdateLoop()` is the single source of game time:

```cpp
while (!World::IsStopped()) {
    ++World::m_worldLoopCounter;
    realCurrTime = getMSTime();
    diff = realCurrTime - realPrevTime;
    if (diff < minUpdateDiff) sleep_for(minUpdateDiff - diff);
    sWorld->Update(diff);          // → UpdateSessions(diff) + sMapMgr->Update(diff) + ...
    realPrevTime = realCurrTime;
}
```

- One thread, one `getMSTime()` per iteration shared by everything inside.
- `sWorld->Update(diff)` calls `UpdateSessions(diff)` (line 2704 of `World.cpp`) which iterates **all** `m_sessions` once per tick, **then** calls `sMapMgr->Update(diff)` (line 2748) which ticks every loaded grid, every creature AI, every spawn-respawn timer, every BG timer, every transport.
- `m_worldLoopCounter` is incremented from this thread and read by the freeze detector — if the thread hangs, the watchdog crashes the process so a supervisor can restart.

RustyCore has **no equivalent**:

- Each `WorldSession` runs in its own Tokio task with a `tokio::time::sleep(Duration::from_millis(50))` floor (`world-server/src/main.rs:705-721`).
- Creature AI / combat / auras are ticked from inside `WorldSession::update` (`session.rs:1109-1124`) — i.e. each session ticks **its own copy** of creatures (`self.creatures: HashMap<ObjectGuid, CreatureAI>`), not the shared `MapManager`.
- `MapManager` (`crates/wow-world/src/map_manager.rs`) has no `update()`, `tick()`, or any periodic method. It is a passive container.
- There is no global counter, no freeze detector, no `World::IsStopped()`, no shared time base.

**Concrete consequences**:

1. With 0 connected sessions, no creature in the world ever updates (idle creatures freeze, BG timers don't run, respawn timers don't fire).
2. With N connected sessions, each creature is ticked from N different real-time ms boundaries depending on each session's tick offset. AoE / aura ticks visible to different clients land at different real-time boundaries — visible in PvP and group play.
3. The `MapManager` migration documented in `_attic/README.md` and `CLAUDE.md` ("two places: legacy per-session HashMap vs shared MapManager") is **a prerequisite** to fixing this, not just a refactor — the global tick driver can't usefully exist until creature state lives in `MapManager` only.
4. There is no `MinWorldUpdateTime` enforcement — the per-session `sleep(50ms)` is hardcoded. No `MaxCoreStuckTime` warning when a tick takes too long.

This is the most fundamental architectural difference between the two stacks and must be the root of the §9 sub-task tree.

### 13.5 Connection-pool sizing

TC opens **3 sub-pools per logical DB** (`SyncPool`, `AsyncPool`, callback pool) configured via `<DB>DatabaseInfo.{Synch,Async}.PoolSize` keys.

Rust opens **one `sqlx::Pool<MySql>` per logical DB** with `max_connections=10` hardcoded in `Database::open` (`crates/wow-database/src/database.rs:36`). `open_with_pool_size` exists but is not used by `world-server/main.rs`. The four config keys `LoginDatabaseInfo.PoolSize` / `CharacterDatabaseInfo.PoolSize` / `WorldDatabaseInfo.PoolSize` / `HotfixDatabaseInfo.PoolSize` are not read.

Under heavy login churn (server reboot at peak) this single 10-connection pool can become a bottleneck. (#WS.21)

### 13.6 DB updater

TC's `DBUpdater` (called by `DatabaseLoader::Load`) hashes every `.sql` in `sql/updates/<db>/`, compares against `updates` table, and applies pending files. Rust port (`crates/wow-database/src/updater.rs`) implements `populate(base_sql)` + `update(source_dir)` and is wired in `world-server/main.rs` lines 232-272. **Parity: ✅ implemented**, including the `auto_setup` flag from config. Failures during populate/update only emit `tracing::warn!` rather than aborting — milder than TC, which `return false` from `StartDB`.

### 13.7 Signal handling

| TC | Rust |
|---|---|
| `signal_set(io, SIGINT, SIGTERM)` + Win32 SIGBREAK → `World::StopNow(SHUTDOWN_EXIT_CODE)` | `shutdown_signal()` waits for Ctrl-C or Unix `SIGTERM` → break out of `tokio::select!` and run the shared shutdown branch |
| `signal(SIGABRT, AbortHandler)` writes coredump preamble | — |
| Signal sets a flag; tick loop notices on next iteration | No flag; the listener tasks are simply abandoned |

Gaps:
- **SIGTERM is handled on Unix** through the same shutdown branch as Ctrl-C. The remaining gap is graceful drain: sessions are still dropped abruptly after the select exits.
- No graceful drain. Sessions are dropped mid-packet.
- No "kick all + save + close listener + flush DB + exit" sequence.

(#WS.18, #WS.15)

### 13.8 Freeze detector

**Missing entirely.** No equivalent of `FreezeDetector` class, no `m_worldLoopCounter`, no `ABORT_MSG`. If the runtime hangs (DB query stuck, scheduler livelock, deadlock), only an external systemd `WatchdogSec=` would catch it — and no `sd_notify(WATCHDOG=1)` is being emitted, so it wouldn't either.

This must be implemented as `tokio::time::interval(1s)` reading an `AtomicU32` global tick counter, and calling `std::process::abort()` (not `exit(1)`) so the supervisor gets a coredump. (#WS.3)

### 13.9 Console / RA / SOAP

| Surface | TC | Rust |
|---|---|---|
| `CliThread` (stdin reader, `.commands`) | `CommandLine/CliRunnable.cpp` (130 lines) | ❌ missing (#WS.8) |
| Remote Access (`Ra.Enable`, port 3443) | `RemoteAccess/RASession.cpp` (150 lines) | ❌ missing (#WS.9) |
| SOAP (`SOAP.Enabled`, port 7878) | `TCSoap/TCSoap.cpp` (160 lines) | ❌ recommend drop (#WS.10) |

GMs currently have **no in-process admin surface** — all administration must go through direct DB writes or restart cycles.

### 13.10 Session tick architecture — CMSG arrival trace

**TC path (single-threaded on the world thread):**

```
TCP read (network thread)
  → WorldSocket::ReadHandler decodes header + decrypts body
  → constructs WorldPacket
  → enqueues into WorldSession::m_recvQueue (lockfree MPSC)
WorldUpdateLoop (world thread, single):
  → sWorld->Update(diff)
    → World::UpdateSessions(diff) — for each session in m_sessions:
      → WorldSession::Update(diff, packetFilter)
        → drains m_recvQueue up to MAX_PACKETS_PER_UPDATE (100)
        → for each packet: dispatches to opcode handler INLINE on this thread
        → handler runs to completion before next packet (per-session serialization)
    → sMapMgr->Update(diff) — ticks all maps' creatures/grids
```

All gameplay state mutation happens on **one thread**, so packet handlers don't need locks against each other. The `PacketFilter` selects which opcodes can run before login complete; handlers `_HandleNonReady` are deferred.

**Rust path (per-session task):**

```
TCP read (per-session async task in wow-network)
  → WorldSocket reads header (decrypts via AES-GCM HMAC-SHA256)
  → sends wow_packet::WorldPacket through flume::Sender<WorldPacket>
WorldSession owning task (per session, in world-server::create_session):
  → loop:
    → session.update(50)            — drains pkt_rx into pending_packets (up to MAX_PACKETS_PER_UPDATE)
                                       AND ticks creatures/combat/auras every 2-4 calls
    → session.process_pending().await — async dispatches via wow_handler::build_dispatch_table()
    → if disconnecting: break
    → tokio::time::sleep(50ms)
```

Each session's task is independent; there is no global ordering. Handlers that mutate **shared** state (e.g. `MapManager` via `Arc<RwLock<...>>`, `PlayerRegistry`, `GroupRegistry`) must take locks. The `tick_creatures_sync` etc. inside `session.update` mutate **per-session** `self.creatures` (legacy field), which is the migration-in-progress called out in `CLAUDE.md`.

This is acceptable divergence **for packet dispatch** (Tokio gives us the per-session serialization for free), but **breaks for shared world state** — which is the §13.4 verdict.

### 13.11 Missing infrastructure (consolidated)

| Item | Severity | Sub-task |
|---|---|---|
| Global `World` singleton + `is_stopped()` / `stop_now(exit)` flag | High | #WS.1 |
| Global `WorldUpdateLoop` driving `MapManager::update(diff)` + session ticks | **Critical** | #WS.2 |
| `FreezeDetector` (process abort on tick stall) | High | #WS.3 |
| `ClearOnlineAccounts` at boot + shutdown | Medium | ✅ #WS.4 |
| Realmlist OFFLINE flag toggle (boot / listener-up / shutdown) | Medium | ✅ #WS.5 |
| DB keep-alive ping (`MaxPingTime`) | Medium | #WS.6 |
| `AppenderDB` for `tracing` (logs into `logs.logs` table) | Low | #WS.7 |
| CLI thread (`Console.Enable`) | Medium | #WS.8 |
| RA listener (`Ra.Enable`, port 3443) | Medium | #WS.9 |
| SOAP — recommend **drop** with config warning | Low (drop) | #WS.10 |
| `SecretMgr::initialize(WorldServer)` | Medium | #WS.11 |
| `RealmList` background refresh | Medium | #WS.12 |
| Metrics subsystem (`online_players`, `db_queue_*`) | Low | #WS.13 |
| `ScriptMgr::on_startup` / `on_shutdown` hooks | Low | #WS.14 |
| Graceful shutdown (kick + save + drain + close + DB cleanup) | High | #WS.15 |
| CLI args (`--config`, `--update-databases-only`, `--version`) | Low | #WS.16 |
| PID file (`PidFile` config) | Low | ✅ #WS.17 |
| SIGTERM handler | High | ✅ #WS.18 |
| Pre-listener startup banner (build hash, DB versions) | Low | #WS.19 |
| Replace per-session sleep with broadcast tick from global loop | Medium | #WS.20 |
| Connection-pool sizing config | Low | #WS.21 |

### 13.12 Recommended sub-task ordering / additions

The §9 list is largely correct; recommended **reorder by priority** (no renumbering — sub-task IDs are referenced elsewhere):

1. **#WS.1, #WS.2, #WS.20** (the global tick + driver) — **prerequisite for everything else**, depends on completing the `_attic/`-flagged `MapManager` migration off `WorldSession.creatures`.
2. **#WS.15** (graceful shutdown) — depends on #WS.1.
3. **#WS.3** (freeze detector) — depends on #WS.2 (needs the loop counter).
4. **#WS.21** (pool sizing) — independent, trivial.
5. **#WS.6** (DB keep-alive) — independent.
6. Everything else as time allows.

**Add new sub-tasks not currently in §9:**

- [ ] **#WS.22** Migrate `tick_creatures_sync`, `tick_combat_sync`, `tick_auras` out of `WorldSession::update` into a `MapManager::update(diff)` method called from #WS.2's global loop. Coupled to the `_attic/` MapManager migration documented in `CLAUDE.md`. (XL)
- [ ] **#WS.23** Move `time_sync_timer_ms` and `logout_time` ticks (currently in `session.rs:1130-1144`) to the global tick after #WS.2 lands; per-session sleep should only drive packet drain, not gameplay timers. (M)
- [ ] **#WS.24** Add `sd_notify(WATCHDOG=1)` from the freeze detector when running under systemd, so the supervisor can do its own watchdog independent of `MaxCoreStuckTime`. (S)
- [ ] **#WS.25** Wire `LoginDatabase.WarnAboutSyncQueries(true)` equivalent: log a warning when a "sync" query is issued from inside a tick (TC's safety net for accidental synchronous DB calls on the world thread). Likely impl: a debug-only `tokio::task::block_in_place` audit. (M)
