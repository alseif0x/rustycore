# Migration: Time (GameTime + UpdateTime + WowTime)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Time/`
> **Rust target crate(s):** `crates/wow-core/` (already houses a partial `GameTime`)
> **Layer:** L0 (foundation — every module that ticks or timestamps something depends on this)
> **Status:** ⚠️ partial (RustyCore has a `GameTime` and `ServerTime` in `wow-core`, but lacks the global singleton-update model, `WowTime`, `UpdateTime`, and `Timezone` integration)
> **Audited vs C++:** ✅ complete (6 small files, ~520 lines)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Three concerns bundled together:

- **`GameTime` namespace** — the single, server-wide "current time" snapshot, refreshed once per main-loop tick by `UpdateGameTimers()`. Keeps `time_t`, `getMSTime()`, `system_clock::time_point`, `steady_clock::time_point`, a cached `localtime_r` `tm`, plus a UTC `WowTime` and a local-zone-shifted `WowTime`. Everything else asks `GameTime::GetGameTime()` instead of calling `time(nullptr)` directly so that all tick-side computations agree on "now".
- **`WowTime`** — the WoW client's packed time format (`year/month/day/weekday/hour/minute/flags/holidayOffset`). Used over the wire for calendar events, mail expiry, auctions, and BroadcastText timestamps. Has comparison, addition of `Seconds`, and `ByteBuffer` serialization.
- **`UpdateTime` / `WorldUpdateTime`** — a tiny accumulator that records the most recent main-loop diff for diagnostic / display purposes (`.server info`, performance tools).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Time/GameTime.cpp` | 112 | `prefix` |
| `game/Time/GameTime.h` | 59 | `prefix` |
| `game/Time/UpdateTime.cpp` | 36 | `prefix` |
| `game/Time/UpdateTime.h` | 45 | `prefix` |
| `game/Time/WowTime.cpp` | 219 | `prefix` |
| `game/Time/WowTime.h` | 90 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Time/GameTime.h` | 60 | Namespace declaration |
| `src/server/game/Time/GameTime.cpp` | 113 | Static state + accessors + `UpdateGameTimers()` |
| `src/server/game/Time/WowTime.h` | 91 | `WowTime` class with packed-time accessors, `<=>`, `+= Seconds`, ByteBuffer ops |
| `src/server/game/Time/WowTime.cpp` | ~155 | All WowTime impls (read but not pasted here; standard packed-time math) |
| `src/server/game/Time/UpdateTime.h` | 46 | `UpdateTime` base + `WorldUpdateTime` (extern global `sWorldUpdateTime`) |
| `src/server/game/Time/UpdateTime.cpp` | 37 | `UpdateWithDiff(uint32)` and `GetLastUpdateTime()` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `GameTime` | namespace | Global "current time" snapshot |
| `WowTime` | class | Packed wire-format time |
| `UpdateTime` | class | Last-diff accumulator (base) |
| `WorldUpdateTime` | class | Trivial subclass; one global instance `sWorldUpdateTime` |

State held by `GameTime` (namespace-private, not in the header):

| Variable | Type | Meaning |
|---|---|---|
| `StartTime` | `time_t const` | Server boot time |
| `GameTime` | `time_t` | Current time (Unix seconds) |
| `GameMSTime` | `uint32` | Millisecond counter since process start (`getMSTime()`) |
| `GameTimeSystemPoint` | `SystemTimePoint` | `chrono::system_clock::now()` |
| `GameTimeSteadyPoint` | `TimePoint` | `chrono::steady_clock::now()` |
| `DateTime` | `tm` | `localtime_r(GameTime, ...)` cached |
| `UtcWow` | `WowTime` | WowTime constructed from UTC |
| `Wow` | `WowTime` | UtcWow shifted by system timezone offset |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `GameTime::GetStartTime()` | Server boot timestamp (constant after init) | — |
| `GameTime::GetGameTime()` | Cached `time_t` of "current" tick | — |
| `GameTime::GetGameTimeMS()` | Cached millisecond counter | — |
| `GameTime::GetSystemTime()` | Cached `system_clock::time_point` | — |
| `GameTime::Now()` | Cached `steady_clock::time_point` | — |
| `GameTime::GetTime<Clock>()` | Compile-time clock dispatch (specialized for `system_clock` and `steady_clock`) | — |
| `GameTime::GetUptime()` | `GameTime - StartTime` in seconds | — |
| `GameTime::GetDateAndTime()` | Pointer to cached `tm` | — |
| `GameTime::GetUtcWowTime()` / `GetWowTime()` | Pointers to cached UTC and local `WowTime` | — |
| `GameTime::UpdateGameTimers()` | Re-snapshot all of the above; called once per main-loop iteration | `time()`, `getMSTime()`, `chrono::*::now()`, `localtime_r`, `Trinity::Timezone::GetSystemZoneOffsetAt` |
| `WowTime::GetPackedTime()` / `SetPackedTime(u32)` | Encode/decode the 32-bit wire format | — |
| `WowTime::GetUnixTimeFromUtcTime()` / `SetUtcTimeFromUnixTime(time_t)` | Round-trip with `time_t` | — |
| `WowTime::IsInRange(from, to)` | Range check (used for daily/weekly resets) | — |
| `WowTime::operator+= Seconds` / `+ Seconds` | Time arithmetic | — |
| `WowTime::operator<<` / `>>` (ByteBuffer) | Wire serialization (packed u32 either side) | — |
| `UpdateTime::UpdateWithDiff(u32)` | Store the latest tick diff | — |
| `UpdateTime::GetLastUpdateTime()` | Read it back | — |

---

## 5. Module dependencies

**Depends on:**
- `Common/Timer.h` — `getMSTime()`, `SystemTimePoint`, `TimePoint` typedefs.
- `Common/Timezone.h` — `Trinity::Timezone::GetSystemZoneOffsetAt(SystemTimePoint)`.
- C++ `<ctime>`, `<chrono>`.

**Depended on by:** essentially everything that has a tick or a timestamp:
- All `Update(diff)` methods (indirectly via tick scheduling).
- `Mail` (expiry), `Auction` (bid expiry), `Calendar` (event start), `BattlePets` (cooldowns), `BlackMarket` (auctions), `Quests` (daily/weekly reset), `Achievements` (criteria timestamps).
- `Log` for log-line timestamps.
- `Maps/Map.cpp`, `Maps/MapManager.cpp` — call `UpdateGameTimers()` indirectly via the world tick.
- `World::Update(diff)` — calls `sWorldUpdateTime.UpdateWithDiff(diff)`.

---

## 6. SQL / DB queries (if any)

None. This module never touches the database. It also doesn't read DBC/DB2.

---

## 7. Wire-protocol packets (if any)

No opcodes are owned by this module, but `WowTime`'s packed format is sent inside many opcodes:

- `SMSG_LOGIN_SET_TIME_SPEED` (game time + speed sync on login).
- Calendar event start/end times in `SMSG_CALENDAR_*`.
- Mail expiry timestamps in `SMSG_MAIL_LIST_RESULT`.
- Auction expiry in `SMSG_AUCTION_LIST_RESULT`.
- Many `BroadcastText` timestamps.
- Holiday/event `WowTime` ranges in DB2-derived data.

The `ByteBuffer << WowTime` overload is the canonical encoder for all of these.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-core` | `crate_dir` | 4 | 1153 | `exists_active` | crate exists |
| `crates/wow-core/src/time.rs` | `file` | 1 | 166 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/misc.rs` | `file` | 1 | 2613 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-core/src/time.rs` — `pub struct GameTime(u64)` (Unix-seconds wrapper) with `now()`, `from_unix`, `as_secs`, `to_packed`, `has_passed`, `time_until`, `add_secs`. Also `pub struct ServerTime` wrapping `Instant` for elapsed-ms.
- `crates/wow-packet/src/packets/misc.rs:415` uses `wow_core::GameTime::now().to_packed()` for `SMSG_LOGIN_SET_TIME_SPEED`.

**What's implemented:**
- A minimal `GameTime` value type (Unix seconds + crude packed-time encoder).
- A `ServerTime` for "monotonic elapsed since this object was created".

**What's missing vs C++:**
- The **global tick snapshot** — RustyCore's `GameTime::now()` calls `SystemTime::now()` every time, which means two reads in the same tick can disagree by some microseconds. C++'s model is to refresh once per tick and read the cached value everywhere. This affects determinism and (less importantly) performance.
- `WowTime` proper — the existing `to_packed` on `GameTime` is described as approximate ("not accounting for all edge cases"); there's no `WowTime` struct with the documented field-by-field wire layout, no `IsInRange`, no `+= Seconds`, no ByteBuffer serializer.
- `UpdateTime` / `WorldUpdateTime` — there is no global "last tick diff" accumulator.
- `Timezone` integration — UtcWow vs Local Wow is unimplemented; `to_packed()` doesn't even know the local timezone.
- `chrono` integration is split between `SystemTime` and `Instant` but never cached.

**Suspicious / likely divergent:**
- The `to_packed` impl (`time.rs:59-80`) computes month/day with `(days / 30)` integer math — that drifts a couple of days per month vs `localtime_r`. For low-stakes uses (chat timestamp display) it's fine; for calendar event start times it will misfire. Treat as broken until replaced.
- Year encoding in `to_packed` uses `year - 100`, suggesting a 1900-anchored offset. The C++ `WowTime` packed format anchors on year-2000 (the WoW client interprets `year` as "years since 2000"). This is plausibly already wrong.

**Tests existing:**
- ~6 unit tests in `crates/wow-core/src/time.rs` for the existing API surface (line 140+); none of them validate the packed-time encoding against the client's expected values.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#TIME.WBS.001** Cerrar la migracion auditada de `game/Time/GameTime.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TIME.WBS.002** Cerrar la migracion auditada de `game/Time/GameTime.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.h`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TIME.WBS.003** Cerrar la migracion auditada de `game/Time/UpdateTime.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/UpdateTime.cpp`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TIME.WBS.004** Cerrar la migracion auditada de `game/Time/UpdateTime.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/UpdateTime.h`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TIME.WBS.005** Cerrar la migracion auditada de `game/Time/WowTime.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#TIME.WBS.006** Cerrar la migracion auditada de `game/Time/WowTime.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h`
  Rust target: `crates/wow-core`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#TIM.1** Add a `WorldClock` (or `GameClock`) struct in `wow-core` that holds the snapshot fields (`unix_secs`, `ms_since_start`, `system_now`, `steady_now`, `local_tm: chrono::DateTime<Local>`, `utc_wow: WowTime`, `local_wow: WowTime`) behind an `Arc<RwLock<...>>` or `ArcSwap`. Add `update_now()` to refresh all of them atomically. (complexity: **M**)
- [ ] **#TIM.2** Wire `WorldClock::update_now()` into the world-server main tick (immediately after `tick.recv()`). Replace ad-hoc `SystemTime::now()` calls in handler/tick code with `clock.unix_secs()` etc. (complexity: **M**)
- [ ] **#TIM.3** Replace the approximate `GameTime::to_packed` with a real `WowTime` struct mirroring `WowTime.h`: `year, month, month_day, week_day, hour, minute, flags, holiday_offset`. Implement `to_packed_u32` / `from_packed_u32` to bit-exact spec. Use `chrono::NaiveDateTime` for the `year/month/day/hour/minute` decomposition; **anchor year on 2000**, not 1970, not 1900. (complexity: **M**)
- [ ] **#TIM.4** Implement `WowTime::is_in_range(from, to)` mirroring the C++ semantics (uses lex-order on the packed fields). Implement `WowTime + Seconds` / `WowTime - Seconds` via round-trip through Unix time. (complexity: **L**)
- [ ] **#TIM.5** Add `WowTime` (de)serialization on top of the existing `wow-packet` `WorldPacket` writer (`write_u32(packed)`, `read_u32 → WowTime`). Replace the misc.rs:415 call with `clock.local_wow().to_packed()`. (complexity: **L**)
- [ ] **#TIM.6** Add `WorldUpdateTime` in `wow-world` (or as a field on the world tick state): `last_diff_ms: AtomicU32`. Update on every tick; expose for `.server info` style commands. (complexity: **L**)
- [ ] **#TIM.7** Local timezone offset: integrate via `chrono::Local` or `chrono-tz`. Document that the offset is computed once per snapshot, not once per server boot — DST transitions during long uptime must update the local `WowTime`. (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#TIME.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 561 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h`. Rust target: `crates/wow-core`. | `cargo test -p wow-core` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#TIME.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 561 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h`. Rust target: `crates/wow-core`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#TIME.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 561 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h`. Rust target: `crates/wow-core`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#TIME.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 561 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h`. Rust target: `crates/wow-core`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] `WowTime::to_packed` → `from_packed` round-trips for a battery of dates spanning 2000-2030.
- [ ] `WowTime::to_packed` matches a known fixture from a captured `SMSG_LOGIN_SET_TIME_SPEED` packet (capture from a real client, paste into test).
- [ ] `WowTime + Duration::from_secs(N)` matches `WowTime::set_utc_from_unix(unix + N)` for many N including DST boundaries.
- [ ] `is_in_range`: weekly reset at Tuesday 08:00 UTC excludes Tuesday 07:59:59 and includes Tuesday 08:00:01.
- [ ] `WorldClock::update_now()` is monotonic in `steady_now` even when `system_now` jumps backwards (NTP correction).
- [ ] After `WorldClock::update_now()`, two reads of `unix_secs()` 100 ms apart return the same value (proves snapshot semantics).
- [ ] `WorldUpdateTime::last_diff_ms` reflects the most recent diff.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 6 files / 561 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h` | `crates/wow-core/` (already houses a partial `GameTime`) \| ⚠️ partial (RustyCore has a `GameTime` and `ServerTime` in `wow-core`, but lacks the global singleton-update model, `WowTime`, `UpdateTime`, and `Timezone` integration) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#TIME.DIV.001` | _none generated_ | 6 C++ files / 561 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/GameTime.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Time/WowTime.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **C++ uses three `time_t`s and two `chrono::*::time_point`s in lockstep**. Don't model this with `SystemTime::now()` everywhere — even microsecond skew across the codebase would make race tests non-deterministic.
- **`WowTime` packs into a single `uint32`** — bit layout is documented in `WowTime.cpp`. Year anchor is **year-2000** in the packed encoding (the field stores "years since 2000"). Confirm against `WowTime.cpp` before committing the constant.
- **DST**: `Trinity::Timezone::GetSystemZoneOffsetAt(SystemTimePoint)` returns the offset *at a specific point* (handles DST). The Rust port must do the same — naively cached offset breaks twice a year for European/American servers.
- **`tm` from `localtime_r` is in local time**; the cached `DateTime` is the local view. UTC WowTime is constructed from `time_t` (UTC) via `SetUtcTimeFromUnixTime`, then shifted by `+offset` to derive the local `Wow`. Mirror this — don't construct local first and then "subtract DST" by hand.
- **`sWorldUpdateTime`** is a global. RustyCore should resist that — pass a `&UpdateTime` through the world tick context. There's only one user (`.server info`) and it's optional anyway.
- **`getMSTime()` is `uint32` and wraps every ~49 days**. Trinity's diff math uses `GetMSTimeDiffToNow(old)` to handle the wrap. If you keep the milliseconds-since-start counter, the Rust equivalent should similarly be wraparound-safe — easier path: use `Instant` and only convert to `u32` ms when the wire requires it.
- **Server-process restart resets `StartTime`** but not `GameTime` (which is wall-clock). Any persistence that uses uptime instead of wall-clock breaks across restarts.
- **WoLK 3.4.3 specific**: BroadcastText event timestamps and a few calendar packets use `WowTime` not `time_t`. Get the encoding right or holidays will be off.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `namespace GameTime` (with mutable globals) | `pub struct WorldClock { snapshot: ArcSwap<TimeSnapshot> }` | Immutable snapshot, atomically swapped on tick |
| `time_t GameTime` | `pub fn unix_secs(&self) -> i64` (or u64) | — |
| `uint32 GameMSTime` | `pub fn ms_since_start(&self) -> u32` (wraps) | Or `Duration since start` for safety |
| `SystemTimePoint GameTimeSystemPoint` | `chrono::DateTime<Utc>` | — |
| `TimePoint GameTimeSteadyPoint` | `Instant` | — |
| `tm DateTime` | `chrono::DateTime<Local>` | — |
| `WowTime UtcWow` / `Wow` | `pub struct WowTime { … }` two instances on snapshot | — |
| `class WowTime` | `pub struct WowTime { year: i32, month: i8, … }` | Implement `Ord`, `Add<Duration>`, packed (de)ser |
| `UpdateTime` | `pub struct UpdateTime { last_diff_ms: AtomicU32 }` | Plain field would also work |
| `extern WorldUpdateTime sWorldUpdateTime` | `Arc<UpdateTime>` injected into world tick | No globals |
| `Trinity::Timezone::GetSystemZoneOffsetAt(...)` | `chrono::Local.offset_from_utc_datetime(...)` or `chrono-tz` | Built-in is fine |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

Verified the section 8 "Suspicious" bullets against live source.

**`to_packed` impl** (`crates/wow-core/src/time.rs:59`): broken as documented.
- Month is computed `(remaining_days / 30).clamp(0, 11)` (line 70). 30-day uniform months drift by 5–6 days per year — by mid-year a `month` value is off by ~1.
- Day is `(remaining_days % 30).clamp(0, 30)` (line 71). Same root cause; never reaches 31.
- Year encoding: `((year.wrapping_sub(100)) & 0x1F) << 24` (line 79). `year` is `(days/365.25)` since Unix epoch (line 68), so `year` is "years since 1970". Subtracting 100 anchors on year **2070**, not year-2000 as the WoW client expects. **Bit-exact wrong** — packets carrying this value place the date ~70 years in the future. Doc line 139 says "1900-anchored offset"; the actual anchor is 1970 + (-100) = **1870**, but because `wrapping_sub` produces `u32::MAX-99 ≈ 4_294_967_196` for current dates and is then masked to 5 bits, the effective year value cycles arbitrarily. Either way, broken.
- Bit layout in the comment (line 58) is correct vs `WowTime.cpp` — fields are right; arithmetic to fill them is wrong.
- Tests at `crates/wow-core/src/time.rs:140+` exercise `now()`, `from_unix`, `has_passed`, `time_until` only; **zero tests assert any packed-time field value**. Doc claim "none of them validate the packed-time encoding" is correct.
- One real consumer: `crates/wow-packet/src/packets/misc.rs:415` calls `wow_core::GameTime::now().to_packed()` for `SMSG_LOGIN_SET_TIME_SPEED`. Login-time clock display is therefore wrong on the client.

**Snapshot-per-tick model**: not implemented. `GameTime::now()` (line 37+) calls `SystemTime::now()` directly each invocation; no `WorldClock`/`update_now()` accumulator.

**`WowTime` struct, `UpdateTime`, timezone, `+= Seconds`, `is_in_range`, ByteBuffer ser**: all absent (no occurrences in the crate).

**Verdict:** ⚠️ partial confirmed; closer to ❌. Replace ⚠️ with ❌ once #TIM.3 lands a real `WowTime`. The `to_packed` is not just "approximate" — it is functionally broken in two independent ways (month math + year anchor). #TIM.3 should be **P0**, not just M-complexity in arbitrary ordering.
