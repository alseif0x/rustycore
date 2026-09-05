# Migration: game/World

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/World/`
> **Rust target crate(s):** `crates/world-server/`, `crates/wow-network/`
> **Layer:** L0 — Global state, update loop, server lifecycle
> **Status:** ❌ not started (core event loop skeleton exists; config/timers not ported)
> **Audited vs C++:** ❌ confirmed missing — 2026-05-01 audit; no `World` struct, no `World::Update()`, no `IntervalTimer` array, no `WUPDATE_*` timers, no `ShutdownServ`, no MapManager tick — world-server tick is per-session-only
> **Last updated:** 2026-05-01

---

## 1. Purpose

The World module is TrinityCore's **global game state manager**. It owns and orchestrates:
- Server configuration (bool/int/float config values, rates, limits)
- Global timers and periodic maintenance tasks (uptime, auctions, corpse cleanup, character deletion, database pings)
- World state variables (MOTD, shutdown state, player count, server status messages)
- Session broadcasting (send message to all players, shutdown notifications)
- MapManager update loop coordination
- Integration point for all managers (AuctionHouseMgr, BattlegroundMgr, GuildMgr, etc.)

The World tick is the heartbeat: it calls Update() on all subsystems every 10–50ms, ensuring consistent ordering and coordination.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/World/World.cpp` | 3971 | `prefix` |
| `game/World/World.h` | 934 | `prefix` |
| `game/World/WorldStates/WorldStateDefines.h` | 37 | `prefix` |
| `game/World/WorldStates/WorldStateMgr.cpp` | 283 | `prefix` |
| `game/World/WorldStates/WorldStateMgr.h` | 52 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/src/server/game/World/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `World.h` | 934 | Global singleton; config enums, timer list, manager ptrs, broadcast methods |
| `World.cpp` | 3971 | Initialization, LoadConfigSettings, Update loop, SQL queries |
| `WorldStates/WorldStateMgr.h` | ~150 | PersistentWorldVariable storage (quest state, event flags) |
| `WorldStates/WorldStateMgr.cpp` | ~300 | Load/save world state from DB |
| `WorldStates/WorldStateDefines.h` | ~100 | WorldState variable IDs (WS_ALLIANCE_CONTROLLED_etc) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `World` | singleton class | Global game state; config, timers, broadcaster, shutdown orchestration |
| `ServerMessageType` | enum | Shutdown/restart/broadcast message types (SERVER_MSG_SHUTDOWN_TIME, etc.) |
| `ShutdownMask` | enum | Restart vs Idle vs Force shutdown flags |
| `WorldTimers` | enum | Timer IDs (WUPDATE_AUCTIONS, WUPDATE_CORPSES, etc.) — indices into m_timers array |
| `WorldBoolConfigs` | enum | Config bool keys (CONFIG_DURABILITY_LOSS_IN_PVP, CONFIG_ADDON_CHANNEL, etc.) |
| `WorldIntConfigs` | enum | Config int keys (CONFIG_COMPRESSION, CONFIG_INTERVAL_SAVE, etc.) |
| `WorldFloatConfigs` | enum | Config float keys (CONFIG_SIGHT_MONSTER, CONFIG_LISTEN_RANGE_*, etc.) |
| `PersistentWorldVariable` | struct | (from WorldStateMgr) int32 variable stored in DB |
| `WorldStateManager` | class | Load/persist PersistentWorldVariable instances |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `World::World()` | Constructor; setup empty config, timers, rate values | (none, static init only) |
| `World::SetInitialWorldSettings()` | Load all config, DB data, spawn managers | LoadConfigSettings, sObjectMgr, sMapMgr |
| `World::LoadConfigSettings(bool reload)` | Parse config file, populate m_*_configs arrays | sConfigMgr, SetIntConfig, SetFloatConfig |
| `World::Update(uint32 diff)` | Main server tick; advance timers, dispatch maintenance | UpdateSessions, sMapMgr->Update, all manager->Update |
| `World::UpdateSessions(uint32 diff)` | Tick all WorldSession instances | WorldSessionMgr::Update |
| `World::SendWorldText(uint32, ...)` | Send localized text to all players | SendGlobalText |
| `World::SendGlobalText(const char*, WorldSession*)` | Broadcast raw text (optional self exclude) | WorldSession::SendPacket for each |
| `World::SendServerMessage(ServerMessageType, string)` | Send system message (shutdown, restart, etc.) | WorldPackets::Misc::ServerMessage packet |
| `World::SendGlobalMessage(WorldPacket const*, WorldSession*, Team)` | Broadcast WorldPacket to all (optional team filter) | iterator over sessions |
| `World::ShutdownServ(uint32 time, uint32 options, uint8 exitcode)` | Schedule server shutdown in N seconds with reason | m_stopEvent, m_ExitCode |
| `World::ShutdownMsg(bool, Player*)` | Announce shutdown to players or specific player | SendGlobalText |
| `World::KickAll()` | Disconnect all players and destroy sessions | WorldSessionMgr::KickAll |
| `World::setIntConfig(WorldIntConfigs, uint32)` | Update int config value (may trigger manager rescaling) | m_int_configs[], manager callbacks |
| `World::setBoolConfig(WorldBoolConfigs, bool)` | Update bool config value | m_bool_configs[] |
| `World::SetPersistentWorldVariable()` | Persist int32 world state to DB | WorldStateManager |
| `World::LoadPersistentWorldVariables()` | Load world state from DB on startup | WorldStateManager |

---

## 5. Module dependencies

**Depends on:**
- `MapManager` — Update all loaded maps and their grids per tick
- `ObjectMgr` — creature/gameobject/spell/item templates; faction, quest data
- `WorldSessionMgr` — tick all sessions, broadcast messages
- `Config` (sConfigMgr) — read world config from file
- `DatabaseEnv` — LoginDatabase, CharacterDatabase, WorldDatabase for queries
- `ScriptMgr` — hook events (startup, shutdown, config reload)
- `BattlegroundMgr` — update arenas, queue announcements
- `AuctionHouseMgr` — periodic auction expiry checks
- `GuildMgr` — auto-save guilds, reset perks
- `GroupMgr` — auto-save groups
- `AuctionHouseBot` — restock vendor listings
- `GameEventMgr` — check for event transitions
- `PoolMgr` — respawn pool creatures/objects
- `OutdoorPvPMgr` — update Wintergrasp, Tol Barad capture state
- `BattlefieldMgr` — outdoor PvP manager
- `LFGMgr` (Looking For Group) — queue updates
- `CharacterCache`, `CreatureTextMgr`, `TaxiPathGraph`, etc. — various lookups
- `SmartScriptMgr`, `CreatureAIRegistry` — AI behavior loading
- `WardenCheckMgr` — anti-cheat scans
- `Realm` (local) — realm info (name, type, zone, language)
- `GameTime` — time checks (events, auto-save intervals)

**Depended on by:**
- `main()` — World singleton is instantiated and ticked every server frame
- `ScriptMgr` — listens to World events
- All game systems via singleton accessor `sWorld`
- CLI commands — query player count, shutdown, broadcast messages

---

## 6. SQL / DB queries (if any)

World module issues numerous queries during init and periodic maintenance.

| Statement / Source | Purpose | DB |
|---|---|---|
| `LOGIN_SEL_REALMLIST_SECURITY_LEVEL` | Load realm security setting | login |
| `UPDATE realmlist SET icon=?, timezone=?` | Update realm type/timezone in DB | login |
| `INSERT INTO uptime VALUES(realm_id, start_time, 0, revision)` | Log server startup | login |
| Auction cleanup queries | Remove expired auctions | world |
| Guild bank log cleanup | Prune old guild transaction logs | character |
| Corpse cleanup | Delete abandoned corpses after N hours | character |
| Character deletion cleanup | Remove chars pending deletion | character |
| Database ping | Keep connection alive | any (SELECT 1) |

Timers that trigger DB maintenance:
- `WUPDATE_AUCTIONS` (60s) — expire old auctions
- `WUPDATE_UPTIME` (10min) — ping databases, log uptime
- `WUPDATE_CORPSES` (20min) — despawn old corpses
- `WUPDATE_CLEANDB` (hourly) — clean orphaned character data
- `WUPDATE_DELETECHARS` (daily) — finalize character deletions
- `WUPDATE_PINGDB` (30min) — keep DB connection warm

---

## 7. Wire-protocol packets (if any)

World module doesn't directly send packets; instead, it calls methods on sessions or managers that send packets. Notable broadcast scenarios:

| Scenario | Packet(s) | Sent via |
|---|---|---|
| Server message (shutdown countdown, MOTD, content ready) | `ServerMessagePacket` | `World::SendServerMessage()` → per-session broadcast |
| Global text chat | `ChatMessage` | `World::SendGlobalText()` |
| Guild/party announcements | `PartyAlert`, `GuildAlert` | manager-specific methods |
| Ping/keepalive | `CMSG_PING` / `PONG` | WorldSocket periodically |
| Auction expiry notification | `AuctionClosedNotification` | AuctionHouseMgr |
| Instance reset warning | `InstanceReset` | InstanceLockMgr |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `crates/wow-world/src/lib.rs` | `file` | 1 | 13 | `exists_active` | file exists |
| `crates/wow-world/src/map_manager.rs` | `file` | 1 | 784 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/world-server/src/main.rs` — ~500 lines — TCP listener, session spawning, basic event loop skeleton
- `crates/wow-world/src/lib.rs` — module exports (session, map_manager, handlers)
- `crates/wow-world/src/map_manager.rs` — ~600 lines — MapManager equivalent (map loading, grid tick, NPC update)

**What's implemented:**
- ✅ Async event loop (tokio runtime)
- ✅ TCP listener accepting client connections
- ✅ Session creation per authenticated client
- ✅ Skeleton MapManager with update ticking
- ✅ Basic GUID generation per realm
- ⚠️ Partial map update logic (not all systems hooked)

**What's missing vs C++:**
- ❌ World singleton with config management (bool/int/float configs)
- ❌ Periodic timer system (WUPDATE_* timers not implemented)
- ❌ Server shutdown orchestration (graceful countdown, kick all players)
- ❌ MOTD, server status messages, shutdown announcements
- ❌ Global text/message broadcasting
- ❌ PersistentWorldVariable persistence (world state variables)
- ❌ Integration with managers: AuctionHouseMgr, GuildMgr, BattlegroundMgr, etc. — no lifecycle hooks
- ❌ Config hot-reload support
- ❌ Database maintenance tasks (corpse cleanup, character deletion, ping)
- ❌ Uptime logging to DB
- ❌ Realm info queries (realm name, type, zone, language)
- ❌ Game event orchestration
- ❌ Player count tracking and limits enforcement
- ❌ Security level per-realm loading from DB

**Suspicious / likely divergent (pre-audit hypothesis):**
- **No config system**: C++ World has massive `m_bool_configs[]`, `m_int_configs[]`, `m_float_configs[]` arrays. Rust version would need env vars or TOML file parser. Risk: **all behavior locked to compile-time constants; can't adjust gameplay without rebuild**.
- **Update loop ordering**: C++ has specific order in `World::Update()`: sessions first, then MapManager, then managers. Rust might update async without order guarantee. Risk: **race conditions, desync between player positions and NPC moves**.
- **Timer precision**: C++ uses IntervalTimer with millisecond precision. Rust likely uses coarse wall-clock ticks. Risk: **maintenance tasks fire at wrong time, auctions expire early, uptime log drifts**.
- **Shutdown is not orderly**: C++ `ShutdownServ()` waits, broadcasts countdowns, saves all, then exits. Rust might just kill the runtime. Risk: **data loss, player disconnects without save, orphaned records in DB**.
- **No session registry**: World doesn't own sessions (WorldSessionMgr does), but needs to broadcast to them. If WorldSessionMgr not yet ported, broadcast is a no-op. Risk: **players never get global messages, shutdown announcements don't reach clients**.

**Tests existing:**
- 0 tests for World logic
- Integration tests in CI check server startup only

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#WORLD.WBS.001** Partir y cerrar la migracion auditada de `game/World/World.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3971 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WORLD.WBS.002** Partir y cerrar la migracion auditada de `game/World/World.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`
  Rust target: `crates/world-server`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 934 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WORLD.WBS.003** Cerrar la migracion auditada de `game/World/WorldStates/WorldStateDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateDefines.h`
  Rust target: `crates/world-server`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLD.WBS.004** Cerrar la migracion auditada de `game/World/WorldStates/WorldStateMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp`
  Rust target: `crates/world-server`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WORLD.WBS.005** Cerrar la migracion auditada de `game/World/WorldStates/WorldStateMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.h`
  Rust target: `crates/world-server`, `crates/wow-network`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#WORLD.1** Create World singleton with config arrays (bool/int/float); load from TOML or environment; expose setter methods with hot-reload hook (complejidad: M)
- [ ] **#WORLD.2** Implement periodic timer system: `IntervalTimer` array for WUPDATE_* timers; tick all timers in update loop, dispatch callbacks (complejidad: M)
- [ ] **#WORLD.3** Port server shutdown orchestration: `ShutdownServ()` method with N-second countdown, broadcast shutdown messages every 10s, save all players/guilds, graceful exit (complejidad: M)
- [ ] **#WORLD.4** Implement session registry integration: lookup sessions by account_id, broadcast messages (text, system packets) to all/filtered sessions (complejidad: L)
- [ ] **#WORLD.5** Implement global message methods: `SendWorldText()`, `SendGlobalMessage()`, `SendServerMessage()` using session broadcaster (complejidad: L)
- [ ] **#WORLD.6** Port PersistentWorldVariable system: load/save int32 world state vars from DB, persist on change (complejidad: L)
- [ ] **#WORLD.7** Integrate all managers into World::Update(): call AuctionHouseMgr, GuildMgr, BattlegroundMgr, etc. with ordered update sequence (complejidad: H)
- [ ] **#WORLD.8** Implement database maintenance timers: corpse cleanup, character deletion, uptime logging, DB ping (complejidad: M)
- [ ] **#WORLD.9** Load realm info from DB on startup: realm name, type, timezone, language, security level (complejidad: L)
- [ ] **#WORLD.10** Implement player count tracking + max player enforcement; track online count, reject login if at limit (complejidad: L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#WORLD.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 5 files / 5277 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp`. Rust target: `crates/world-server`, `crates/wow-network`. | `cargo test -p world-server && cargo test -p wow-network` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#WORLD.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 5 files / 5277 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp`. Rust target: `crates/world-server`, `crates/wow-network`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#WORLD.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 5 files / 5277 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp`. Rust target: `crates/world-server`, `crates/wow-network`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#WORLD.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 5 files / 5277 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp`. Rust target: `crates/world-server`, `crates/wow-network`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: World::Update() ticks all timers correctly (one timer fires, others don't)
- [ ] Test: Server shutdown: countdown broadcasts message every 10s, then saves all, then exits
- [ ] Test: Config hot-reload: change int value, World respects new value immediately
- [ ] Test: SendGlobalMessage broadcast reaches all N sessions exactly once
- [ ] Test: SendGlobalMessage with team filter only reaches team members
- [ ] Test: Session lifecycle: new login increments player count, logout decrements
- [ ] Test: Max player limit: 100 players online, 101st login rejected
- [ ] Test: Corpse cleanup timer fires every 20min, expires corpses > 2 hours old
- [ ] Test: Auction expiry timer fires every 60s, removes expired auctions from DB
- [ ] Test: Uptime logging timer fires every 10min, updates login.uptime table
- [ ] Test: PersistentWorldVariable change persists to DB on next flush

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 5 files / 5277 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp` | `crates/world-server/`, `crates/wow-network/` \| ❌ not started (core event loop skeleton exists; config/timers not ported) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#WORLD.DIV.001` | _none generated_ | 5 C++ files / 5277 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/World/World.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/World/World.h`, `/home/server/woltk-trinity-legacy/src/server/game/World/WorldStates/WorldStateMgr.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **World is a singleton** (`sWorld` global). C++ uses `World* sWorld = nullptr` declared globally, initialized in `main()`. Rust must use `lazy_static!` or `once_cell::sync::Lazy` for thread-safe access. **Do NOT use `Mutex` around the whole World** — it will serialize all updates and kill performance.

2. **Update order is critical**. C++ `World::Update()` has explicit order:
   - Tick all sessions (WorldSessionMgr)
   - Tick MapManager (all maps, grids, creatures, objects)
   - Tick managers (Auction, Battlefield, Battleground, etc.)
   - Update world timers (check if WUPDATE_* timers expired, dispatch cleanup)
   
   If Rust does this out of order (e.g., maps before sessions), players might move into loaded maps before sessions are ticked, causing race conditions.

3. **Timer precision matters**. Auctions expire in milliseconds. If a timer drifts by seconds, auctions could last too long or expire too soon. Use `Instant::now()` consistently, not system time.

4. **Shutdown must be graceful**. C++ `ShutdownServ()` sets `m_stopEvent = true`, then the main loop checks this flag. While flag is true, it waits for all sessions to disconnect, saves all data, then exits. If Rust just kills the tokio runtime, character data is lost. Must implement a clean shutdown signal.

5. **Session affinity with broadcast**: when World broadcasts (e.g., SendGlobalMessage), it iterates WorldSessionMgr's session pool. If a session is being destroyed concurrently, iterator might panic. Use Arc<Mutex<Session>> and lock all session access consistently.

6. **Config reload is tricky**. Some config changes require manager restart (e.g., CONFIG_INTERVAL_MAPUPDATE changes MapManager tick interval). Others are just value swaps (CONFIG_DURABILITY_LOSS_IN_PVP). C++ calls manager-specific callbacks on config change. Rust needs similar hook system.

7. **Database pings keep connections alive**. MySQL connections timeout after 8 hours of inactivity. The timer `WUPDATE_PINGDB` fires every 30 minutes with a `SELECT 1` query to prevent timeout. Without this, long-running servers disconnect from DB and crash.

8. **Corpse cleanup can't be instant**. C++ waits N hours (config `CharacterDatabaseCleanerInterval`; default 3600s = 1 hour for deaths < 2 hours). Client expects corpses to linger; instant cleanup breaks UI.

9. **Account-level vs character-level limits**. C++ tracks per-account max sessions and global max player limit. If account tries to log in 2 characters at once (abnormal), either new login fails or old one is kicked. Must enforce in SessionRegistry before creating new WorldSession.

10. **Realm info is static after startup**. C++ loads realm data once in SetInitialWorldSettings. If you change realm name in DB later, server won't see it until restart. Rust should do the same to avoid locks.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class World` | `struct World` (or `struct GameState` to avoid keyword conflict) | singleton; no inheritance |
| `World* sWorld` | `lazy_static!(static ref WORLD: World = ...)` | thread-safe lazy init |
| `std::array<IntervalTimer, WUPDATE_COUNT>` | `Vec<IntervalTimer>` or `[IntervalTimer; WUPDATE_COUNT]` | fixed-size timer array |
| `uint32 m_bool_configs[BOOL_CONFIG_VALUE_COUNT]` | `[bool; BOOL_CONFIG_VALUE_COUNT]` | flat bool array |
| `uint32 m_int_configs[INT_CONFIG_VALUE_COUNT]` | `[u32; INT_CONFIG_VALUE_COUNT]` | flat int array |
| `float m_float_configs[FLOAT_CONFIG_VALUE_COUNT]` | `[f32; FLOAT_CONFIG_VALUE_COUNT]` | flat float array |
| `std::unordered_map<uint32, WorldSession*>` | `HashMap<u32, Arc<Mutex<WorldSession>>>` | session pool (via WorldSessionMgr) |
| `void World::Update(uint32)` | `fn update(&mut self, diff_ms: u32)` | main tick dispatcher |
| `void World::SendGlobalMessage(WorldPacket const*)` | `fn broadcast_packet(&self, packet: &WorldPacket)` | iterate sessions, send |
| `void World::ShutdownServ(...)` | `async fn shutdown(&mut self, delay_sec: u32)` | tokio-aware async shutdown |
| `IntervalTimer` (C++) | `struct IntervalTimer { interval: u32, passed: u32 }` | manual timer tracking |
| `WorldStateManager` | planned `struct PersistentWorldState` | world variable persistence |

---

## 13. Integration with Maps.md and Handlers.md

**Link to Maps:** World ticks MapManager once per update with the same `diff_ms`. MapManager then ticks all loaded maps. See `docs/migration/maps.md` for map update order and creature AI tick.

**Link to Handlers:** When World broadcasts a message (e.g., shutdown announcement), it sends a WorldPacket. Handlers then process SMSG_SERVER_MESSAGE in WorldSession. See `docs/migration/handlers.md` for opcode dispatch details.

**Link to Entities/Player:** When player logs in, MapManager spawns a Player object on the world. World doesn't own the Player directly; it's owned by WorldSession (which got it from ObjectAccessor). But World updates the session, which ticks player's spell casting, movement, etc.

---

*Template version: 1.0. Status: ❌ not started — skeleton exists, major subsystems not ported.*

---

## 13. Audit (2026-05-01)

Audited C++ tree: `/home/server/woltk-trinity-legacy/src/server/game/World/{World.h:934, World.cpp:3971}`. Audited Rust tree: `crates/world-server/src/main.rs:720` (only file in `world-server`); `crates/wow-world/src/{lib.rs:23, session.rs:2820, map_manager.rs:784}`. **Search confirms** no `struct World`, no `impl World`, no `WORLD: Lazy<...>`, no `world_update_loop`, no `WUPDATE_` symbol exists in any Rust source under `/home/server/archived/rustycore_ARCHIVED_20260312/crates/`.

### 13.1 Coverage table

| C++ symbol (file:line) | Rust equivalent | Status |
|---|---|---|
| `class World` singleton (World.h:42L+) | None — no `World`/`GameState` struct anywhere | ❌ |
| `World* sWorld` global accessor | None | ❌ |
| `World::SetInitialWorldSettings()` (World.h:651) — orchestrates ObjectMgr, MapManager, ScriptMgr, etc. on boot | None — `world-server/main.rs::main` (line 152-438) inlines DB connect → store loads → `tokio::spawn(start_world_listener)` directly, no orchestrator | ❌ |
| `World::LoadConfigSettings(bool reload)` (World.h:660L) | `wow_config::load_config("WorldServer.conf")` (main.rs:164) — single TOML/INI parse, no hot reload, no setter API | ⚠️ |
| `World::Update(uint32 diff)` (World.h:670L) — main game-loop tick | **MISSING** — no ticker exists. `main.rs` ends at `tokio::select! { ctrl_c \| realm_handle \| instance_handle }` (line 421) and exits | ❌ **breaking divergence** |
| `World::UpdateSessions(uint32 diff)` (World.h:675) | None at world-level — each connection runs its own loop in `create_session()` (main.rs:606-623): `loop { session.update(50); session.process_pending().await; tokio::time::sleep(50ms) }`. Sessions are unaware of each other | ⚠️ |
| `IntervalTimer m_timers[WUPDATE_COUNT]` (World.h:830) | None | ❌ |
| `enum WUPDATE_* { WUPDATE_AUCTIONS, WUPDATE_AUCTIONS_PENDING, WUPDATE_UPTIME, WUPDATE_CORPSES, WUPDATE_EVENTS, WUPDATE_CLEANDB, WUPDATE_AUTOBROADCAST, WUPDATE_MAILBOXQUEUE, WUPDATE_DELETECHARS, WUPDATE_AHBOT, WUPDATE_PINGDB, WUPDATE_GUILDSAVE, WUPDATE_BLACKMARKET, WUPDATE_CHECK_FILECHANGES, WUPDATE_WHO_LIST, WUPDATE_CHANNEL_SAVE, WUPDATE_COUNT }` (World.h:82-98) | None — zero of the 16 maintenance timers exists | ❌ |
| `uint32 m_int_configs[INT_CONFIG_VALUE_COUNT]` (World.h:847) | None — `wow_config::get_value(...)` is direct lookup against in-memory file map; no typed array, no enum keys | ⚠️ |
| `bool m_bool_configs[BOOL_CONFIG_VALUE_COUNT]` (World.h:849) | None | ❌ |
| `float m_float_configs[FLOAT_CONFIG_VALUE_COUNT]` (World.h:850) | None | ❌ |
| `World::SendWorldText / SendGlobalText / SendGlobalMessage / SendServerMessage` (World.h:660-680L) | None — `PlayerRegistry` (network crate) is per-session iteration, no central broadcast helper | ⚠️ partial |
| `World::ShutdownServ(uint32 time, uint32 options, uint8 exitcode, std::string reason)` (World.h:666) — graceful countdown + KickAll + save-all + exit | None — only `tokio::signal::ctrl_c()` (main.rs:422) → log → exit, no countdown, no save, no broadcast | ❌ |
| `World::ShutdownMsg(bool, Player*)` | None | ❌ |
| `World::KickAll()` | None | ❌ |
| `World::LoadPersistentWorldVariables / SetPersistentWorldVariable` (World.h:680L+) — int32 world state DB persistence | None | ❌ |
| MapManager integration in tick (`sMapMgr->Update(diff)`) | None — no Rust call site invokes `MapManager::*update*` because no such function exists in `crates/wow-world/src/map_manager.rs:455-613`. Confirmed against worldserver audit finding | ❌ |
| Manager update integration: AuctionHouseMgr, GuildMgr, BattlegroundMgr, BattlefieldMgr, OutdoorPvPMgr, PoolMgr, GameEventMgr, AuctionHouseBot, LFGMgr, WardenCheckMgr, etc. | None — these crates either don't exist or have no tick hook surfaced | ❌ |
| `World::SetInitialWorldSettings()` realm-info DB load | Inlined in main.rs: `load_realm_auth_seed` (main.rs:443) and `load_realm_addresses` (main.rs:630). One-shot, no `World` ownership | ⚠️ |

### 13.2 Critical divergences

1. **No global tick.** This is the headline finding the worldserver audit flagged. C++ has `WorldRunnable::run() { while(!stopEvent) { World::Update(diff); MapManager::Update(diff); ... } }` driving every subsystem on a single clock. Rust has no clock at all at the world level. The closest analog is `loop { session.update(50) }` per-connection (`world-server/src/main.rs:606-623`); maps, timers, respawns, scripts, auctions, corpses, DB pings, uptime logs all sit dormant. The `Arc<RwLock<MapManager>>` (`map_manager.rs:616`) is mutated by handlers but never ticked.
2. **No graceful shutdown.** `tokio::select!` on `ctrl_c` (main.rs:421-435) logs "Shutdown signal received" and falls out of `main`. Sessions are not given a chance to save; characters in flight (open trades, cast bars, in-combat status) lose state. C++ `ShutdownServ(time, options, exitcode, reason)` runs a 30-min/5-min/1-min countdown, broadcasts `SMSG_SERVER_MESSAGE`, calls `WorldSessionMgr::KickAll`, flushes all `CharacterDatabase` transactions, logs uptime, then `_exit(exitcode)`.
3. **No config typing.** C++ exposes 200+ keyed config values via `getBoolConfig(CONFIG_DURABILITY_LOSS_IN_PVP)` etc. (World.h:685-717). Rust `wow_config::get_value(&str)` returns by string lookup with `.unwrap_or(default)`; no compile-time enumeration of which keys exist, no default registry, no callback on change. Hot-reload (C++ `LoadConfigSettings(reload=true)`) is impossible.
4. **No DB maintenance.** `WUPDATE_PINGDB` (every 30 min) prevents MariaDB connections from being torn down by `wait_timeout` (default 8h). RustyCore has no equivalent — long-idle sessions on staging/prod will silently lose DB connectivity. Same risk for `WUPDATE_CORPSES` (corpse cleanup), `WUPDATE_DELETECHARS` (finalize deletions), `WUPDATE_UPTIME` (`uptime` table never updated → no "online time" telemetry), `WUPDATE_AUCTIONS` (no auction expiry).
5. **No central session registry.** C++ `WorldSessionMgr` owns the `std::unordered_map<uint32, WorldSession*>`. Rust uses `PlayerRegistry` (in `wow-network`) for chat/movement broadcast and `SessionManager` (main.rs:369) for the BNet→world `ConnectTo` redirect; neither exposes "iterate all sessions for World::Update". Without this, even if a `World` struct were added later, `SendGlobalMessage` would have nowhere to deliver.
6. **No world-state persistence.** C++ `WorldStateMgr` loads `PersistentWorldVariable` rows on boot and persists changes — used by Wintergrasp/Tol Barad capture state, holiday flags, etc. Rust has neither the table interface nor the in-memory store.

### 13.3 Verdict

❌ **not started — keep status as-is, escalate audit field to ❌ confirmed.** The world-server binary is more accurately described as "two TCP listeners + per-connection async session driver"; it has no `World`-layer abstraction. The previous worldserver audit's claim that `MapManager::update()` does not exist is corroborated here from the **caller** side: even if such a function existed, no code path in `crates/world-server/` would invoke it. Bringing this module to parity is a green-field port, not an upgrade. The natural seam is to add a `wow-world::game_state::World` struct owning `Arc<RwLock<MapManager>>`, an `IntervalTimer` array indexed by a `WorldTimer` enum, and a `tokio::task` spawned from `main.rs` that runs `loop { world.update(diff_ms).await; tokio::time::sleep_until(next_tick).await; }`. Until that exists, every other module that needs a periodic tick (Maps, Grids, AuctionHouse, BG queues, respawns, scripts) is dead code in production. Treat #WORLD.1 + #WORLD.2 + #WORLD.7 as the unblock chain for the entire server.
