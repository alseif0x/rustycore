# Migration: Logging

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/common/Logging/` + `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.{h,cpp}`
> **Rust target crate(s):** `crates/wow-logging/`
> **Layer:** L0 (foundation; depended on by every other layer)
> **Status:** ⚠️ partial — `tracing` wrapper exists with 22 filter categories and convenience macros, but no file appender, no DB appender, no per-logger level config, no rotation, no realm-id binding, no async (`Asio::Strand`) emission path.
> **Audited vs C++:** ⚠️ partial (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

TrinityCore's `Trinity::Logging` is a configurable structured-logging framework: per-logger levels, per-appender targets (console / file / DB), per-line metadata (timestamp, level, filter type, optional realm ID), async dispatch via a `boost::asio::strand`, and reload via `worldserver.conf` / `bnetserver.conf` `Logger.*` and `Appender.*` keys. It feeds three sinks simultaneously: TTY for ops, rotated files for forensics, and the `auth.logs` table for cross-realm/critical events.

RustyCore replaces this with `tracing` + `tracing-subscriber`, exposed through `wow-logging`. The crate gives downstream code a `LogFilter` enum (22 subsystem categories) and one convenience macro per category (`log_server!`, `log_network!`, etc.), each attaching a structured `log_filter` field. As of 2026-05-01 only the console (TTY) sink is wired; file and DB sinks are not implemented.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `database/Logging/AppenderDB.cpp` | 47 | `prefix` |
| `database/Logging/AppenderDB.h` | 40 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/common/Logging/Log.h` | 171 | Public `Log` singleton, `TC_LOG_*` macros, appender registration |
| `src/common/Logging/Log.cpp` | 408 | Config parsing (`Logger.*`, `Appender.*`), routing, async strand |
| `src/common/Logging/LogCommon.h` | 59 | `LogLevel` (Disabled/Trace/Debug/Info/Warn/Error/Fatal), `AppenderType` (None/Console/File/DB), `AppenderFlags` |
| `src/common/Logging/Appender.h` | 61 | Abstract `Appender` base + `setRealmId` + `_write` virtual |
| `src/common/Logging/Appender.cpp` | 92 | Base `Appender::write` (prefix construction, level filter) |
| `src/common/Logging/AppenderConsole.h` | 62 | Stdout/stderr appender (color per level on TTY) |
| `src/common/Logging/AppenderConsole.cpp` | 211 | ANSI colour table + write |
| `src/common/Logging/AppenderFile.h` | 46 | File appender state |
| `src/common/Logging/AppenderFile.cpp` | 127 | Open/rotate/backup, max-file-size, dynamic `%s` substitution |
| `src/common/Logging/Logger.h` | 48 | Per-logger level + appender list |
| `src/common/Logging/Logger.cpp` | 60 | `Logger::write` dispatch to all bound appenders |
| `src/common/Logging/LogMessage.h` | 51 | `LogMessage` struct (mtime, type, level, text, prefix, param1) |
| `src/common/Logging/LogMessage.cpp` | 42 | Timestamp formatting |
| `src/common/Logging/LogOperation.h` | 41 | Async work item |
| `src/common/Logging/LogOperation.cpp` | 34 | Posts to strand |
| `src/server/database/Logging/AppenderDB.h` | 41 | DB appender (writes to `auth.logs` table) |
| `src/server/database/Logging/AppenderDB.cpp` | 48 | Async insert via `LOGIN_INS_LOG` prepared statement |
| **TOTAL** | **~1652** | — |

Default config dist: `src/server/worldserver/worldserver.conf.dist` `Appender.*` and `Logger.*` keys (the long commented block listing every available logger).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Log` | singleton class | Owner of appenders + loggers; entry point `sLog->ShouldLog` / `OutMessage` |
| `Logger` | class | One logger = (name, level, set of bound appenders) |
| `Appender` | abstract class | Base for sinks; `_write(LogMessage const*)` is the virtual |
| `AppenderConsole` | class | Stderr/stdout with ANSI colours per level |
| `AppenderFile` | class | Rotating file with optional `%s` dynamic name + max-size backup |
| `AppenderDB` | class | Inserts each message into `auth.logs` (LoginDB) via prepared statement |
| `LogMessage` | struct | POD: `mtime, type, level, text, prefix, param1` |
| `LogOperation` | struct | Async wrapper, posts a `LogMessage` to the configured `Asio::Strand` |
| `LogLevel` | enum | `Disabled / Trace / Debug / Info / Warn / Error / Fatal` (numeric 0..6) |
| `AppenderType` | enum | `None / Console / File / DB` |
| `AppenderFlags` | bitfield | `PrefixTimestamp / PrefixLogLevel / PrefixLogFilterType / UseTimestamp / MakeFileBackup` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Log::Initialize(IoContext*)` | Wires the strand for async writes; calls `LoadFromConfig` | `LoadFromConfig`, `RegisterAppender<Console/File/DB>` |
| `Log::LoadFromConfig()` | Parses `Appender.*` + `Logger.*` from config | `CreateAppenderFromConfigLine`, `CreateLoggerFromConfigLine` |
| `Log::ShouldLog(type, level)` | Fast filter check before formatting cost | hash lookup in `loggers` map |
| `Log::OutMessage(filter, level, fmt, args)` | Emit one record (formatted) | `OutMessageImpl` → strand post |
| `Log::SetRealmId(uint32)` | Bind realm ID into every `AppenderDB` | `Appender::setRealmId` per appender |
| `Log::SetSynchronous()` | Disable strand; main-thread shutdown only | — |
| `Logger::write(LogMessage*)` | Dispatch to all bound appenders ≥ logger.level | `Appender::write` |
| `Appender::write(LogMessage*)` | Build prefix, call `_write` if level passes | `_write` (virtual) |
| `AppenderFile::_write` | Append to file, rotate on `_maxFileSize`, optional `%s` substitution | `OpenFile`, `fprintf` |
| `AppenderDB::_write` | Skip if `type` contains `"sql"` (avoid recursion); insert via `LOGIN_INS_LOG` | `LoginDatabase.Execute` |
| `Trinity::Asio::Strand::post` | Serialises async writes off the calling thread | — |

`TC_LOG_TRACE/DEBUG/INFO/WARN/ERROR/FATAL(filterType, fmt, ...)` are the call-site macros; they short-circuit on `sLog->ShouldLog(filterType, level)` to avoid format cost when filtered.

---

## 5. Module dependencies

**Depends on:**

- `src/common/Define.h` — `uint8/32/64` typedefs.
- `src/common/Asio/AsioHacksFwd.h` + `boost::asio` — `IoContext`, `Strand`.
- `src/common/Configuration/Config.h` — reads `Logger.*` / `Appender.*` keys.
- `src/server/database/DatabaseEnv.h` — `LoginDatabase` for `AppenderDB`.
- `src/server/database/PreparedStatement.h` — `LOGIN_INS_LOG`.

**Depended on by:**

- Every `.cpp` in TC; the `TC_LOG_*` macros are in the global include path.
- `worldserver`, `bnetserver` main loops (`Initialize`, `LoadFromConfig`, `SetRealmId`, shutdown `SetSynchronous + Close`).

---

## 6. SQL / DB queries

| Statement | Purpose | DB |
|---|---|---|
| `LOGIN_INS_LOG` = `INSERT INTO logs (time, realm, type, level, string) VALUES (?, ?, ?, ?, ?)` (`LoginDatabase.cpp:63`, `CONNECTION_ASYNC`) | Each `AppenderDB::_write` row | auth |
| `LOGIN_DEL_OLD_LOGS` = `DELETE FROM logs WHERE (time + ?) < ? AND realm = ?` (`LoginDatabase.cpp:73`, `CONNECTION_ASYNC`) | Periodic prune of old log rows | auth |

Schema (`sql/base/auth_database.sql:791-797`):

```sql
CREATE TABLE `logs` (
  `time`   int unsigned NOT NULL,
  `realm`  int unsigned NOT NULL,
  `type`   varchar(250) NOT NULL,                -- the filter ("server", "network.opcode", "sql.dev", ...)
  `level`  tinyint unsigned NOT NULL DEFAULT 0,  -- LogLevel enum value
  `string` mediumtext                            -- formatted message
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
```

No DBC/DB2 stores.

---

## 7. Wire-protocol packets

None. Logging emits no client packets.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-logging` | `crate_dir` | 1 | 464 | `exists_active` | crate exists |
| `crates/wow-logging/Cargo.toml` | `file` | 1 | 15 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-config` | `crate_dir` | 1 | 397 | `exists_active` | crate exists |
| `crates/wow-logging/src/lib.rs` | `file` | 1 | 464 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**

- `crates/wow-logging/Cargo.toml` — deps: `tracing`, `tracing-subscriber`, `wow-config`.
- `crates/wow-logging/src/lib.rs` (465 lines) — `LogFilter` enum (22 variants), 22 convenience macros, `init_logging(level: &str)` initializer.

**What's implemented:**

- `LogFilter` enum mirrors TC `Logger.*` categories at a coarser grain: `Server, Network, Database, Player, Chat, Spells, Maps, Entities, AI, Scripts, Commands, Arena, Battleground, Lfg, Misc, Loading, Guild, Achievement, Condition, Vehicle, Loot, Movement` (`crates/wow-logging/src/lib.rs:46-91`).
- One macro per filter (`log_server!`, `log_network!`, …, `log_movement!`), each delegating to `tracing::{trace,debug,info,warn,error}!` with a structured `log_filter = "..."` field (`lib.rs:194-394`).
- `init_logging("info")` builds a `tracing_subscriber::fmt` subscriber with: ISO-8601 timestamps, level, target, ANSI colours, no thread IDs, no file/line. Honours `RUST_LOG` env first, then the passed string (`lib.rs:158-179`).
- `install_panic_hook_like_cpp()` installs an idempotent panic hook that emits `tracing::error!(fatal=true, file, line, column, panic_message)` before delegating to Rust's default panic hook. `world-server` and `bnet-server` install it immediately after their subscriber setup.
- 5 unit tests (`lib.rs`): `LogFilter::as_str()` covers every variant, plus `Display`, macro compile-check, and panic-payload message extraction.

**What's missing vs C++:**

- **No file appender.** No rotation, no `%s` substitution, no max-size backup. `tracing-appender` (`rolling::daily`, `rolling::hourly`) would supply this; not added.
- **No DB appender.** No insert into `auth.logs`. The infinite-loop guard (`type.find("sql") != npos` per `AppenderDB.cpp:31`) has no Rust analogue because there is no DB sink at all.
- **No per-logger level config.** TC's `Logger.network.opcode=3,Console Server` lets ops dial network.opcode independently. Rust uses the global `RUST_LOG`/`EnvFilter` directive, which is similarly expressive (`wow_network=trace`) but the **subsystem categories** in `LogFilter` are not addressable as filter targets — `RUST_LOG=...,log_filter=network=debug` does not work because `log_filter` is a field, not a target. EnvFilter's field-filter syntax (`[{log_filter=network}]=debug`) does support this but is not documented for downstream code.
- **No realm-id binding.** TC's `setRealmId(realm)` injects realm into every DB row. Rust has no per-realm context.
- **No async dispatch / no strand.** Calls block the calling task on TTY write. `tracing-appender::non_blocking` would give an async sink; not used.
- **No `worldserver.conf` / `bnetserver.conf` integration.** The `wow-config` dep is declared in `Cargo.toml` but is not consumed in `lib.rs`. There's no way to specify "DB appender writes errors and above" per config; ops must use env vars.
- **No full `Fatal` level distinct from `Error`.** TC has 7 levels (Disabled..Fatal); `tracing` has 5 (`trace, debug, info, warn, error`). Rust panics now carry `fatal=true`, but ordinary fatal log macros and fatal-specific appender routing are still missing.
- **No `OutCommand(account, fmt)` GM-command audit path.** TC has a separate command audit channel that goes to file + DB.
- **No `OutCharDump(...)` character-dump sink.** Used by `.character dump` GM command.
- **No subscriber teardown.** `init_logging` calls `set_global_default` which is one-shot per process; `Log::Close()` semantics (flush + drop appenders) have no equivalent.

**Suspicious / likely divergent (hypotheses pre-audit):**

- `LogFilter` is a closed enum (22 variants). TC's `type` is a free-form string (e.g. `network.opcode`, `entities.player.skills`, `scripts.ai.escortai` — see `worldserver.conf.dist` `Logger.*` block). The Rust enum **cannot represent the dotted hierarchy** that TC uses. Filtering by `entities.player` matches `entities.player.skills` in TC; in Rust it would not, because `LogFilter::Entities` is one bucket.
- The `console-subscriber` `tokio-console` integration referenced in `migration-perf-strategy.md` would compete for `set_global_default`; if both `init_logging` and `console-subscriber` try to install, the second one fails silently.
- `init_logging`'s default for `with_file(false).with_line_number(false)` makes log lines harder to grep back to source. TC includes file:line via its own format; Rust opted not to.
- `log_filter` field is attached only via the convenience macros. Code that calls `tracing::info!(...)` directly bypasses the field, producing logs that are unfilterable by subsystem.
- `init_logging` returns `Box<dyn std::error::Error>` — opaque error type; callers can't reason about "subscriber already set" vs "bad filter directive".

**Tests existing:**

- 23 in `crates/wow-logging/src/lib.rs::tests` (`as_str` round-trip, `Display`, macro-compile).
- Zero behavioural tests (does the subscriber actually format the field? does `EnvFilter` honour `log_filter=...`? does color output respect `NO_COLOR`?).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#LOGGING.WBS.001** Cerrar la migracion auditada de `database/Logging/AppenderDB.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`
  Rust target: `crates/wow-logging`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#LOGGING.WBS.002** Cerrar la migracion auditada de `database/Logging/AppenderDB.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h`
  Rust target: `crates/wow-logging`, `crates/wow-config`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#LOG.1** Add `tracing-appender = "0.2"` to workspace; expose a file-rolling layer in `init_logging`. Behaviour: daily rotation under `<logs_dir>/server.YYYY-MM-DD.log`, configurable max files. Resolves the `AppenderFile` gap. (M)
- [ ] **#LOG.2** Add `tracing-appender::non_blocking` for the file layer; document the `WorkerGuard` lifecycle (must outlive `main`). (M)
- [ ] **#LOG.3** Implement a custom `tracing::Layer` that writes records to `auth.logs` via the existing `wow-database` connection. Mirror the C++ `AppenderDB::_write` skip-on-`type contains "sql"` guard. Use a bounded `mpsc` channel to avoid blocking emitters. Resolves the `AppenderDB` gap. (H)
- [ ] **#LOG.4** Schema migration: ensure `auth.logs` table exists in RustyCore's auth DB schema (the table already ships in TC's `auth_database.sql:791-797`; confirm it's in the Rust seed schema). (L)
- [ ] **#LOG.5** Wire `Log::SetRealmId` equivalent: store the realm ID in a global `parking_lot::RwLock<Option<u32>>` or as a `tracing::Span` extension; the DB layer reads it for the `realm` column. (M)
- [ ] **#LOG.6** Read `Logger.*` and `Appender.*` keys from `WorldServer.conf` / `BNetServer.conf` via `wow-config` (already a dep). Translate to a programmatic `EnvFilter` directive at startup. Resolves the conf-integration gap. (H)
- [ ] **#LOG.7** Decide on dotted-hierarchy support: either widen `LogFilter` to a `&'static str` newtype with conventional names like `"entities.player.skills"`, or document that the closed enum is intentional. If widened, update all 22 macros. (H — touches every call site that uses the macros.)
- [ ] **#LOG.8** Map TC `Fatal` → Rust: panic-hook path emits `tracing::error!(fatal = true, ...)` already; still add a `log_*_fatal!` macro family and make DB/file appenders promote `fatal=true` records to a separate `*_fatal.log` file (matches TC's typical `Errors.log` config). (M)
- [ ] **#LOG.9** Add `OutCommand(account, fmt)` equivalent: a `log_command!(account, fmt, args)` macro that always emits at info, attaches `account` as a structured field, and is routed to a separate "GM" file appender by config. (M)
- [ ] **#LOG.10** Add `OutCharDump(account, guid, name, body)` equivalent for `.character dump` output (separate file appender, no DB). (M)
- [ ] **#LOG.11** Add `tracing::enabled!(target: "log_filter::network", Level::TRACE)` short-circuit guidance to `wow-logging`'s docs; per `migration-perf-strategy.md` §11 #5, structured-field formatting is non-zero cost in hot paths. (L)
- [ ] **#LOG.12** Test: install a `tracing_subscriber::fmt::TestWriter` in tests, emit one record per `LogFilter`, assert the rendered line contains `log_filter=<name>`. Cross-references `migration-test-strategy.md` §3 L0 layer. (M)
- [ ] **#LOG.13** Test: round-trip a fake `auth.logs` insert via the DB layer in `#LOG.3` against an in-memory MariaDB or test fixture; assert columns (`type`, `level`, `realm`, `string`) match input. (M)
- [ ] **#LOG.14** Document the call-site contract: every caller in `wow-*` crates uses `log_<filter>!`, never `tracing::*!` directly. Add a clippy `disallowed-macros` rule to enforce. (M)
- [ ] **#LOG.15** Decide on async vs sync emission. TC uses an `Asio::Strand`; Rust has `tracing-appender::non_blocking` for files but no equivalent for the DB layer (the `mpsc` proposed in `#LOG.3` is the equivalent). Document the contract: emitters never block longer than X µs in the hot path. (L — design decision, no code.)
- [ ] **#LOG.16** Consider `tracing-flame` or `tokio-console` integration behind a feature flag (cross-references `migration-perf-strategy.md` `#PERF.9`). (M)
- [ ] **#LOG.17** Reconcile log levels: emit a `LogLevel` translation table in this doc and in code (`fn from_tracing_level(Level) -> u8` matching TC's `0..6` enum) for the DB column. (L)
- [ ] **#LOG.18** Snapshot test (`insta`) on the rendered console output for one record per filter to lock the on-screen format. (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#LOGGING.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 87 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h`. Rust target: `crates/wow-config`, `crates/wow-logging`. | `cargo test -p wow-config && cargo test -p wow-logging` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#LOGGING.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 87 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h`. Rust target: `crates/wow-config`, `crates/wow-logging`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#LOGGING.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 87 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h`. Rust target: `crates/wow-config`, `crates/wow-logging`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#LOGGING.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 87 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h`. Rust target: `crates/wow-config`, `crates/wow-logging`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Each `LogFilter` variant's `as_str()` is stable (existing).
- [ ] A `log_<filter>!(level, "...")` call produces a tracing event with `log_filter = <name>` field (new, behavioural).
- [ ] `EnvFilter` directive `[{log_filter=network}]=debug` filters by field correctly.
- [ ] File appender rotates on day boundary (`#LOG.1` follow-up).
- [ ] DB appender skips records whose target contains `"sql"` (mirrors TC `AppenderDB.cpp:31` infinite-loop guard).
- [ ] DB appender attaches the global realm ID to every row.
- [ ] DB appender does not block the emitter for more than 1 ms p99 under load.
- [ ] `init_logging` is idempotent OR documents that it must be called exactly once.
- [ ] `WorkerGuard` from `tracing-appender::non_blocking` outlives `main`; flushed-on-drop semantics verified.
- [ ] TC `Fatal` records map to Rust `error` + `fatal=true` field; verified round-trip.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 87 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h` | `crates/wow-logging/` \| ⚠️ partial — `tracing` wrapper exists with 22 filter categories and convenience macros, but no file appender, no DB appender, no per-logger level config, no rotation, no realm-id binding, no async (`Asio::Strand`) emission path. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#LOGGING.DIV.001` | _none generated_ | 2 C++ files / 87 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Logging/AppenderDB.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **`LogFilter` is closed and coarse.** TC uses arbitrary dotted strings (`network.opcode`, `entities.player.skills`, `scripts.ai.escortai`). The Rust enum has 22 buckets. If you need finer subdivision, either add a variant or attach a `subsystem = "..."` field to the macro call. The closed enum is a deliberate ergonomics tradeoff; the cost shows up in operations who can't dial down `network.opcode` separately from `network.kick`.
2. **`tracing` levels are 5, TC levels are 7.** `trace, debug, info, warn, error` ↔ `Trace, Debug, Info, Warn, Error+Fatal collapsed`. `Disabled` collapses to "filter excludes everything". The DB column expects `0..6`; the translation function in `#LOG.17` is the load-bearing piece.
3. **`init_logging` is one-shot per process.** It uses `set_global_default`. Tests that need a different subscriber must use `tracing::subscriber::with_default(local)` per test; never call `init_logging` from tests.
4. **`tracing-appender::non_blocking` requires holding a `WorkerGuard`.** If the guard drops (e.g. it was a function-local in `main`), the worker thread shuts down and writes are silently lost. CLAUDE.md does not currently document this; `#LOG.2` adds the docs.
5. **The DB appender (`#LOG.3`) must avoid recursion**: any log statement inside `wow-database` could trigger an emit → DB write → `wow-database` log statement → emit. TC handles this with the `type.find("sql") != npos` guard at `AppenderDB.cpp:31`. The Rust port must do the same — easiest implementation: filter on `LogFilter::Database` records.
6. **`auth.logs` can grow without bound.** TC ships `LOGIN_DEL_OLD_LOGS` for periodic prune. The Rust port must either (a) wire that prepared statement and run it on a tokio interval, or (b) document that ops must run a cron job. `#LOG.3` should land with one of those.
7. **Production domain `wowchad.work.gd`** (CLAUDE.md "Runtime") will eventually want centralised log shipping (Loki, Vector, Fluent Bit). `tracing-loki` and `tracing-opentelemetry` are the standard Rust integrations; out of scope for `#LOG.*` but worth noting.
8. **`wow-logging` is depended on by every crate.** Changes to its public API ripple through the workspace. Prefer additive changes (new macros, new fields) over breaking ones.
9. **The C# legacy at `/home/server/woltk-server-core/Source/`** has its own `Logger` class (.NET `ILogger`-shaped); ignore it for protocol purposes — TC C++ is the canonical reference per CLAUDE.md.
10. **`PROTOC=...`** is required to build `wow-proto` but `wow-logging` does not depend on `wow-proto`; logging changes alone don't need protoc.

---

## 12. C++ → Rust mapping (high-level)

| C++ TC symbol | Rust equivalent | Notes |
|---|---|---|
| `class Log` (singleton) | `wow-logging::init_logging` + global `tracing` subscriber | No singleton struct exposed; the subscriber is the implicit global. |
| `LOGGER_ROOT` | `EnvFilter` global default | Same semantics: catches everything not otherwise routed. |
| `Logger` (one per `type`) | `tracing` `target` (module path) + `log_filter` field | Lossy: TC has independent levels per logger; Rust merges via `EnvFilter` directives. |
| `Appender` (abstract) | `tracing::Layer` impl | One layer per sink. |
| `AppenderConsole` | `tracing_subscriber::fmt::Layer` | Already wired in `init_logging`. |
| `AppenderFile` | `tracing-appender::rolling::*` + `non_blocking` | `#LOG.1` + `#LOG.2`. |
| `AppenderDB` | Custom `tracing::Layer` writing to `wow-database` (`auth.logs`) | `#LOG.3`. |
| `LogMessage` | `tracing::Event` + `Visit` | The `Layer` extracts fields via a `Visit` impl. |
| `LogLevel` (Disabled..Fatal, 0..6) | `tracing::Level` (Trace..Error, 5 values) + `fatal=true` field | Lossy — see `#LOG.17`. |
| `AppenderType` | Enumerable layer set | Concept doesn't translate; layers are values. |
| `AppenderFlags` (PrefixTimestamp, etc.) | `tracing_subscriber::fmt::Format::*` builder methods | Same prefixes available; not bitfield-shaped. |
| `Asio::Strand` async dispatch | `tracing-appender::non_blocking` (file) + `mpsc::channel` (DB) | One mechanism per sink. |
| `Trinity::FormatString<Args...>` | `tracing`'s `format_args!` (rust std) | Native. |
| `TC_LOG_INFO("network", ...)` | `log_network!(info, ...)` | Rust is filter-first, level-second; symmetric. |
| `OutCommand(account, fmt, args)` | `log_command!(account, fmt, args)` proposed (`#LOG.9`) | Includes `account` field. |
| `OutCharDump(...)` | `log_char_dump!(...)` proposed (`#LOG.10`) | Routes to dedicated file. |
| `SetRealmId(uint32)` | global `RwLock<Option<u32>>` set at server boot (`#LOG.5`) | DB layer reads on each emit. |
| `LOGIN_INS_LOG` prepared statement | `INSERT INTO logs (time, realm, type, level, string) VALUES (?, ?, ?, ?, ?)` (same SQL, registered in `wow-database`) | `#LOG.4`. |
| `LOGIN_DEL_OLD_LOGS` periodic prune | tokio interval task (`#LOG.6` follow-on) or external cron | Operational decision. |

---

## 13. Audit (2026-05-01)

> Audited 2026-05-01. C++ ref: `/home/server/woltk-trinity-legacy` (commit `5100ce3d8fc6` per `crypto.md` §13 baseline). Rust ref: `crates/wow-logging/src/lib.rs` (465 lines), `crates/wow-logging/Cargo.toml`. Cross-checked against `worldserver.conf.dist` `Logger.*` / `Appender.*` block and `auth_database.sql:791-797`.

### 13.1 Feature-by-feature

| Capability | C++ ref | Rust ref | Status | Divergence |
|---|---|---|---|---|
| Console (TTY) sink | `AppenderConsole.cpp` (211 lines, ANSI palette per level) | `tracing_subscriber::fmt` in `init_logging` (`lib.rs:166-174`) | ✅ | Both write to TTY with ANSI colour. Format strings differ but functionally equivalent. |
| File sink | `AppenderFile.cpp` 127 lines: open, append, rotate on size, optional `%s` dynamic name, optional backup-on-rotate, `_logDir` from config | **None** | ❌ | Hard gap. `#LOG.1` + `#LOG.2`. |
| DB sink | `AppenderDB.cpp` 48 lines: `LOGIN_INS_LOG` async insert; skip on `type` containing `"sql"`; realm column from `setRealmId` | **None** | ❌ | Hard gap. `#LOG.3` + `#LOG.4` + `#LOG.5`. |
| Per-logger level | `Logger::level` set per `Logger.<name>=<level>,...` config | `EnvFilter` directive (`RUST_LOG`) | ⚠️ | Functionally possible, but the Rust filter syntax for the `log_filter` *field* is `[{log_filter=network}]=debug`, not the simple `network=debug` users expect. Undocumented. |
| Per-logger appender binding | `Logger.<name>=<level>,Console Server` (binds two appenders) | All layers receive every event; layer-side filter only | ⚠️ | Partial. Can be implemented via `Layer::with_filter(...)`. Today, every layer (only one exists) sees everything. |
| Filter hierarchy (`scripts` matches `scripts.ai.escortai`) | TC matches by prefix: `scripts.ai.escortai` falls through to `scripts` if not configured (`Log::GetLoggerByType` walks dotted hierarchy) | `LogFilter` is a closed flat enum — no hierarchy | ❌ | Significant divergence. `#LOG.7`. |
| 7 log levels (Disabled, Trace, Debug, Info, Warn, Error, Fatal) | `LogCommon.h:24-36` | 5 levels (`tracing::Level`) | ⚠️ | `Disabled` ≈ filter excludes; `Fatal` collapses to `Error`. `#LOG.8`, `#LOG.17`. |
| Realm-id binding | `Log::SetRealmId(uint32)` → `Appender::setRealmId` per appender, used as `realm` column in `INS_LOG` | None | ❌ | `#LOG.5`. |
| `OutCommand(account, fmt)` GM-command audit | `Log.h:77-84`, gated on `commands.gm` always-info | None | ❌ | `#LOG.9`. |
| `OutCharDump(...)` character-dump sink | `Log.h:86` | None | ❌ | `#LOG.10`. |
| Async dispatch (off the calling thread) | `Asio::Strand` posts each emit | Calls block (TTY synchronous) | ⚠️ | TTY blocking is usually fine; file/DB emits will need async (`#LOG.2`, `#LOG.3` design). |
| Config-driven setup | `worldserver.conf` `Appender.*` + `Logger.*` keys parsed by `Log::LoadFromConfig` | `wow-config` is a dep but not used; only `RUST_LOG` env / hardcoded string | ❌ | `#LOG.6`. |
| Format string safety | `Trinity::FormatString<Args...>` (compile-time check via `fmt::format_string`) | `tracing` macros → `format_args!` (compile-time check) | ✅ | Equivalent guarantee. |
| Subsystem coverage | TC has ~70 logger names in `worldserver.conf.dist` (commented examples) | Rust has 22 `LogFilter` variants | ⚠️ | Coverage gap: no `network.opcode`, no `entities.player.skills`, no `scripts.ai.escortai` distinction. Per-call structured fields could fill the gap without expanding the enum. |
| TTY colour | ANSI per level (`AppenderConsole.cpp` 211-line palette) | `tracing_subscriber::fmt` `with_ansi(true)` | ✅ | Equivalent. Colour table differs slightly but equivalent semantically. |
| File rotation triggers | Size-based (`_maxFileSize`), or daily via `%s` substitution | None today | ❌ | `tracing-appender::rolling::{daily,hourly,never}` covers the time-based case; no built-in size-based option (`#LOG.1` design choice). |
| Tests | TC has minimal unit tests for logging | 5 unit tests in Rust (`as_str` round-trip across variants + macro compile + panic-payload extraction only) | ⚠️ | None of the Rust tests verify behaviour (does a record actually appear at the configured target?). `#LOG.12`, `#LOG.13`, `#LOG.18`. |

### 13.2 Critical findings

1. **❌ No file appender.** Production `world-server` and `bnet-server` have nowhere to persist a forensic log on disk. Every restart loses TTY history. **Must land before any production deployment.** `#LOG.1`.
2. **❌ No DB appender.** Critical events (auth failures, GM commands, suspected exploits) are **not** persisted to `auth.logs`. The C++ TC operations runbook assumes this table is populated; any tooling that grep's `auth.logs` for security audit will see nothing on a Rust-server-only deployment. `#LOG.3`.
3. **❌ No realm-id context.** Even after `#LOG.3`, the `realm` column has no value source. Multi-realm deployments (the typical TC topology) cannot disambiguate logs across realms. `#LOG.5`.
4. **❌ No conf-driven setup.** Ops cannot tune log levels per subsystem via `WorldServer.conf` / `BNetServer.conf` `Logger.*` keys — they must use `RUST_LOG` env var. This breaks the operational contract that TC ops expect. `#LOG.6`.
5. **⚠️ Closed `LogFilter` enum.** The 22-bucket flat enum cannot represent `network.opcode` vs `network.kick` vs `network.soap`. Either widen to dotted strings (`#LOG.7`, breaking) or attach a `subsystem` field to every macro call (additive, but every call site changes). Decide explicitly.
6. **⚠️ Behavioural test coverage is still shallow.** The tests verify enum strings, macro compilation and panic-payload extraction, but not that a `log_network!(info, "...")` call results in a `tracing::Event` with `log_filter = "network"` reaching a subscriber. Coupled with finding #1 + #2, an entire class of regressions (sink not wired, field dropped, level mismatch) would be invisible until production. `#LOG.12`–`#LOG.18`.
7. **⚠️ `Fatal` is partially represented.** Panics now produce `error` events with `fatal=true`, but normal fatal log calls and sink-level separation are still missing. TC distinguishes `Error` from `Fatal`; ops dashboards filter on `level >= Error` for "investigate" and `level == Fatal` for "page someone". `#LOG.8`.
8. **⚠️ DB appender recursion guard not yet specified.** Per `AppenderDB.cpp:31`, TC drops messages whose `type` contains `"sql"` to avoid infinite loops. The Rust design must include this from day one or risk a self-DoS the first time a DB error logs.

### 13.3 Recommended action — priority queue

1. **`#LOG.1` + `#LOG.2` — file appender (HIGH).** Lowest-effort biggest-value gap; `tracing-appender` is a one-day integration. Land before any production deploy.
2. **`#LOG.6` — config integration (HIGH).** Operational expectation. Without it, ops have to pass `RUST_LOG` on every invocation; will be forgotten.
3. **`#LOG.3` + `#LOG.4` + `#LOG.5` — DB appender (HIGH together).** Land as one stack so realm context, schema, and recursion guard all ship together. ~3-5 days.
4. **`#LOG.7` — hierarchy decision (MEDIUM).** Decide explicitly before more code accumulates against the closed enum. Either widen now or commit to the field-based pattern and document.
5. **`#LOG.12`–`#LOG.18` — tests (MEDIUM).** Cross-references `migration-test-strategy.md`; close coverage gap.
6. **`#LOG.8` — `Fatal` rehydration (LOW).** Adds a field; non-breaking. Land after the sinks exist.
7. **`#LOG.9` + `#LOG.10` — `OutCommand` / `OutCharDump` (LOW).** Required for full GM-command parity; not blocking for pre-GM-tooling phases.
8. **`#LOG.16` — `tokio-console` (LOW).** Cross-references perf strategy; engineer-only.

### 13.4 Justifying the index status

Header status is `⚠️ partial` because the foundation (filter taxonomy, macro surface, structured field plumbing) is in place and tested at compile-time, but every sink other than TTY is missing, the conf integration is absent, and the realm-binding doesn't exist. Status moves to `✅ done` once `#LOG.1`, `#LOG.3`, `#LOG.5`, `#LOG.6`, and `#LOG.12` are all landed. Status moves to `🔧 broken` if `init_logging` ever silently fails (e.g. double-init in a test) without a clear error.

---

*Template version: 1.0 (2026-05-01).*
