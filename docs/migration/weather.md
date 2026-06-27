# Migration: Weather

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Weather/`
> **Rust target crate(s):** `crates/wow-world/` (no dedicated crate; would live next to `MapManager` / zone systems)
> **Layer:** L8 (service — per-zone tick loop on top of map/zone)
> **Status:** ❌ not started
> **Audited vs C++:** ✅ complete (module is small: 4 files, ~480 lines total) — re-verified 2026-05-01: Rust-side absence confirmed
> **Last updated:** 2026-05-01

---

## 1. Purpose

Per-zone weather state machine. For every zone with a `game_weather` row, runs a Markov-chain regenerator that picks a `WeatherType` (fine/rain/snow/storm/blackrain/thunders) and an `intensity` float in [0, 1). The combination is mapped to a `WeatherState` opcode value (e.g. `WEATHER_STATE_LIGHT_RAIN = 3`) and broadcast via `SMSG_WEATHER` to every player in the zone every `CONFIG_INTERVAL_CHANGEWEATHER` ms (default 10 minutes). Also exposes `SetWeather` for scripted overrides (instance encounters, holiday events).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Weather/Weather.cpp` | 321 | `prefix` |
| `game/Weather/Weather.h` | 92 | `prefix` |
| `game/Weather/WeatherMgr.cpp` | 104 | `prefix` |
| `game/Weather/WeatherMgr.h` | 37 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Weather/Weather.h` | 92 | `Weather` class, `WeatherState` enum, `WeatherData` / `WeatherSeasonChances` structs |
| `src/server/game/Weather/Weather.cpp` | 322 | Per-zone state machine: `Update`, `ReGenerate`, `UpdateWeather`, `GetWeatherState` |
| `src/server/game/Weather/WeatherMgr.h` | 37 | `WeatherMgr` namespace (loader + lookup only) |
| `src/server/game/Weather/WeatherMgr.cpp` | 105 | `LoadWeatherData` from `world.game_weather`, `GetWeatherData(zone_id)` |

Note: the lifecycle owner (`std::map<uint32, std::unique_ptr<Weather>>`) lives in `Map.cpp` (see `Map::AddZoneDynamicInfo`/`UpdateZoneWeather`) — Weather objects are created lazily on first player entering a zone.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Weather` | class | Per-zone weather instance (state + 10-min timer) |
| `WeatherMgr` | namespace | Static loader for `WeatherData` keyed by `zone_id` |
| `WeatherData` | struct | 4 `WeatherSeasonChances` + `ScriptId` |
| `WeatherSeasonChances` | struct | `rainChance`, `snowChance`, `stormChance` (each 0-100) |
| `WeatherState` | enum (uint32) | Wire value sent to client (FINE=0, FOG=1, LIGHT_RAIN=3, …, BLACKSNOW=106) |
| `WeatherType` | enum (declared in `SharedDefines.h`) | Internal type: FINE, RAIN, SNOW, STORM, BLACKRAIN, THUNDERS |
| `WEATHER_SEASONS` | constexpr | 4 (spring/summer/fall/winter) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Weather::Weather(zoneId, WeatherData const*)` | Construct; pulls interval from `CONFIG_INTERVAL_CHANGEWEATHER` | — |
| `Weather::Update(uint32 diff)` | Tick; returns `false` when zone empty (caller drops it) | `ReGenerate`, `UpdateWeather`, `sScriptMgr->OnWeatherUpdate` |
| `Weather::ReGenerate()` | Markov chain: 30% no-change / 30% better / 30% worse / 10% radical; uses `urand`/`rand_norm` | `GameTime::GetGameTime` (for season via `localtime_r` & `tm_yday`) |
| `Weather::UpdateWeather()` | Build `SMSG_WEATHER` packet, send to zone via `World::SendZoneMessage`; logs change | `sScriptMgr->OnWeatherChange` |
| `Weather::SendWeatherUpdateToPlayer(Player*)` | Push current state to a single player (e.g. on zone enter) | — |
| `Weather::SendFineWeatherUpdateToPlayer(Player*)` | Force-send FINE (used on zone leave) | — |
| `Weather::SetWeather(WeatherType, float intensity)` | Scripted override; calls `UpdateWeather` immediately | — |
| `Weather::GetWeatherState() const` | Map (type, intensity) → `WeatherState` wire value | — |
| `WeatherMgr::LoadWeatherData()` | Read all rows of `world.game_weather`; clamp invalid % | `sObjectMgr->GetScriptId` |
| `WeatherMgr::GetWeatherData(zone_id)` | `MapGetValuePtr` lookup, returns `nullptr` if no data | — |

---

## 5. Module dependencies

**Depends on:**
- `Time` — `GameTime::GetGameTime()` for season computation in `ReGenerate`.
- `Server/World` — `sWorld->getIntConfig(CONFIG_INTERVAL_CHANGEWEATHER)`, `SendZoneMessage`.
- `Globals/ObjectMgr` — `GetScriptId(name)` for the `ScriptName` column.
- `Scripting/ScriptMgr` — `OnWeatherUpdate`, `OnWeatherChange` hooks.
- `Server/Packets/MiscPackets.h` — `WorldPackets::Misc::Weather` (`SMSG_WEATHER`).
- `Util/Random` — `urand`, `rand_norm`.

**Depended on by:**
- `Maps/Map.cpp` — owns the `Weather*` per zone, ticks them in `Map::Update`.
- A few scripts (boss encounters that force fog / blackrain).

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| Inline in `WeatherMgr::LoadWeatherData` | `SELECT zone, spring/summer/fall/winter * (rain/snow/storm)_chance, ScriptName FROM game_weather` | world |

No prepared statements; single startup query. No DBC/DB2 stores used.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_WEATHER` | server → client | `Weather::UpdateWeather`, `Weather::SendWeatherUpdateToPlayer`, `Weather::SendFineWeatherUpdateToPlayer` |

Payload: `WeatherState (uint32)`, `intensity (float, 0..1)`, `bool unk` (transition flag). No client-originated weather opcodes.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-weather` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- None. No `weather.rs`, no `crates/wow-weather`, no opcode binding for `SMSG_Weather`'s payload (the opcode constant `Weather = 0x26a6` exists in `wow-constants/src/opcodes.rs:1625` but no packet body type and no handler).

**What's implemented:** nothing.

**What's missing vs C++:** everything — loader for `game_weather`, per-zone state machine, season computation, `SendZoneMessage` hook, scripted `SetWeather`.

**Suspicious / likely divergent:** N/A (clean slate).

**Tests existing:** none.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#WEATHER.WBS.001** Cerrar la migracion auditada de `game/Weather/Weather.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`
  Rust target: `crates/wow-world`, `crates/wow-weather`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WEATHER.WBS.002** Cerrar la migracion auditada de `game/Weather/Weather.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h`
  Rust target: `crates/wow-world`, `crates/wow-weather`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WEATHER.WBS.003** Cerrar la migracion auditada de `game/Weather/WeatherMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`
  Rust target: `crates/wow-world`, `crates/wow-weather`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WEATHER.WBS.004** Cerrar la migracion auditada de `game/Weather/WeatherMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.h`
  Rust target: `crates/wow-world`, `crates/wow-weather`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#WTH.1** Add `WeatherData` / `WeatherSeasonChances` structs and a `WeatherMgr` (or function on `WorldExt`) that loads `world.game_weather` at startup, with the same 100-cap clamp + warn behaviour. (complexity: **L**)
- [ ] **#WTH.2** Implement `Weather` struct (`zone_id`, `kind`, `intensity`, timer, chances ref) with `update(diff_ms)` + `regenerate()` matching the 30/30/30/10 distribution exactly (use `rand` crate; document seed source). (complexity: **M**)
- [ ] **#WTH.3** Add `weather_state()` mapping (RAIN/SNOW/STORM × intensity bands → `WEATHER_STATE_*` u32), and the BLACKRAIN/THUNDERS pass-through. (complexity: **L**)
- [ ] **#WTH.4** Define `SmsgWeather` packet in `wow-packet` (`WeatherState u32`, `intensity f32`, `transition bool`) and wire `Weather::send_to_zone` through `MapManager`/`PlayerRegistry` zone iteration. (complexity: **M**)
- [ ] **#WTH.5** Hook ownership into `MapManager`: `HashMap<u32 zone_id, Weather>` per `Map`, ticked from the existing map update loop; drop entry when `update()` returns `false` (no players). (complexity: **M**)
- [ ] **#WTH.6** Public APIs: `send_weather_update_to_player(player)` on zone enter, `send_fine_weather_update_to_player` on leave, `set_weather(kind, intensity)` for scripts. (complexity: **L**)
- [ ] **#WTH.7** Wire script hooks (`on_weather_update`, `on_weather_change`) once the scripting layer exists; no-op stubs for now. (complexity: **L**)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#WEATHER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 554 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h`. Rust target: `crates/wow-world`. | `cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#WEATHER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 554 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h`. Rust target: `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#WEATHER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 554 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h`. Rust target: `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#WEATHER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 554 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h`. Rust target: `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] `regenerate` distribution: with a fixed seed, 1M iterations starting from a known state must match the C++ histogram for {fine, light, medium, heavy} within ±0.5%.
- [ ] Season computation: `(tm_yday - 78 + 365) / 91 % 4` matches the `localtime_r`-derived season for a battery of timestamps spanning a year.
- [ ] `intensity` clamping: any value ≥1 clamps to `0.9999`; <0 clamps to `0.0001` (verbatim from `UpdateWeather`).
- [ ] `WeatherState` mapping: table-test all (type, intensity) combinations against the C++ `GetWeatherState` switch.
- [ ] Empty zone: `Weather::update` on a zone with zero players returns `false` and is dropped from the map.
- [ ] Loader clamp: a row with `rain_chance=200` is replaced with `25` and an error is logged.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 554 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h` | `crates/wow-world/` (no dedicated crate; would live next to `MapManager` / zone systems) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#WEATHER.DIV.001` | `crates/wow-weather` (`missing_declared_path`, 0 Rust lines) | 4 C++ files / 554 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/WeatherMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Weather/Weather.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Season trick**: 78 days offset puts spring start at March 20, then 91-day blocks. Don't simplify — the magic constants come from US Naval Observatory data and fans noticed mismatches in earlier rewrites.
- **`Weather::Update` returning `false`** is the only way Weather objects get freed. The map tick must drop the `Weather` from its map when this happens, otherwise dead zones leak.
- The 30/30/30/10 in `ReGenerate` is misleading — read it carefully: the first `if (u<30) return false` short-circuits, then the next checks reuse the same `u` so the distributions overlap. The Rust port should preserve the exact branch order, not re-derive a "cleaner" distribution.
- WoLK 3.4.3-specific: `WEATHER_STATE_BLACKSNOW = 106` exists in the enum but no current `WeatherType` produces it; reserve for scripts only.
- `CONFIG_INTERVAL_CHANGEWEATHER` is in milliseconds in the conf even though the log line says "minutes".
- `sScriptMgr->OnWeatherUpdate` is called every tick (even when nothing changed), `OnWeatherChange` only when the broadcast actually fires. Don't conflate them.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Weather` | `pub struct Weather` in `crates/wow-world/src/weather.rs` | Plain struct; no inheritance involved |
| `WeatherMgr` namespace | `pub struct WeatherStore` (or free fns + `OnceLock<HashMap<u32, WeatherData>>`) | Loaded once at startup |
| `enum WeatherState : uint32` | `#[repr(u32)] enum WeatherState` | Preserve the exact integer values; client-visible |
| `enum WeatherType` | `#[repr(u8)] enum WeatherKind { Fine, Rain, Snow, Storm, BlackRain, Thunders }` | Internal only |
| `IntervalTimer m_timer` | `u32` ms accumulator + `interval_ms: u32` field | Use the same pattern as other tick code in `wow-world` |
| `World::SendZoneMessage` | iterate `PlayerRegistry` filtering by zone, push `SmsgWeather` bytes | Lives in `wow-network` registries |
| `sScriptMgr->OnWeatherUpdate` | trait hook on a future `ScriptMgr`; no-op stub initially | Defer until `wow-script` lands |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — completely absent in Rust.**

`grep -rn -i "weather\|WeatherState\|WeatherType" crates/ --include='*.rs'` returns just three hits, none of them implementation:

- `wow-constants/src/shared.rs:416` — bitflag `DISABLE_SHARED_WEATHER_SYSTEMS = 0x40000` (a config-side disable bit; never read because there's no weather system to disable).
- `wow-constants/src/spell.rs:261` — `SpellCastResult::WrongWeather = 170` enum variant (used by `Spells.WeatherRequired` checks; currently dead code).
- `wow-constants/src/opcodes.rs:1625` — `Weather = 0x26a6` SMSG opcode definition only.

Zero hits for the implementation surface: no `Weather` struct, no `WeatherMgr`, no `WeatherData` / `WeatherSeasonChances`, no `LoadWeatherData`, no `ReGenerate`, no Markov chain, no per-zone tick, no zone-message broadcast, no DB loader of `world.game_weather`. `MapManager` (the post-MapManager-landing replacement for per-zone state in `crates/wow-world/src/map_manager.rs`) does not own a `weather` map.

**No silent-default bug** — without the SMSG_WEATHER opcode being emitted, the client renders "fine" everywhere, which is the natural visual default. No incorrect behaviour is being masked.

**Recommendation:** Tractable, self-contained, low-risk migration target. Module is genuinely small (~480 LOC C++, four files). The `Map`-side ownership model from C++ should map to a `HashMap<u32 /* zone_id */, Weather>` field on `MapManager` (since RustyCore has unified Map ownership through `MapManager` per `AGENTS.md`). Markov chain is a pure function. `World::SendZoneMessage` translates to iterating `PlayerRegistry` filtered by zone (per §12 mapping) — `wow-network` already exposes this. Defer until ConditionMgr lands only if you also want `CONDITION_ACTIVE_EVENT`-style per-zone weather overrides; otherwise can be done standalone.
