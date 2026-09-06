# Migration: Entities / Vehicle

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/`
> **Rust target crate(s):** `crates/wow-world/` (entity logic), `crates/wow-data/` (DBC), `crates/wow-constants/` (flags/opcodes)
> **Layer:** L4 (sub-modules)
> **Status:** partial
> **Audited vs C++:** ⚠️ partial (VehicleKit/mount path and accessory lookup audited; full runtime still open)
> **Last updated:** 2026-05-14

---

## 1. Purpose

`Vehicle` is the seat-aware controller attached to a `Unit` that has a vehicle kit (DBC `VehicleEntry`). It implements `TransportBase`, manages the `SeatMap` of passengers (player or creature), schedules delayed join events for spell-driven mounting, owns the accessory list (gunners, decorative passengers, multi-passenger NPCs like Wintergrasp siege engines, Oculus drakes, Ulduar vehicles), and routes passenger position updates back to the global frame.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Vehicle/Vehicle.cpp` | 995 | `prefix` |
| `game/Entities/Vehicle/Vehicle.h` | 144 | `prefix` |
| `game/Entities/Vehicle/VehicleDefines.h` | 203 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Vehicle/Vehicle.h` | 144 | `Vehicle` class def; final, inherits `TransportBase`; `VehicleJoinEvent` |
| `src/server/game/Entities/Vehicle/Vehicle.cpp` | 995 | Install/Uninstall/Reset, AddVehiclePassenger, RemovePassenger, accessory install, immunity application |
| `src/server/game/Entities/Vehicle/VehicleDefines.h` | 203 | `PowerType`, `VehicleFlags`, `VehicleSpells`, `PassengerInfo`, `VehicleSeat`, `VehicleSeatAddon`, `VehicleAccessory`, `VehicleTemplate`, `SeatMap`, `TransportBase` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Vehicle` | class (final, `TransportBase`) | Per-Unit seat controller |
| `VehicleJoinEvent` | class (BasicEvent) | Delayed mount completion |
| `TransportBase` | abstract class | Shared base for Vehicle + Transport (offset/global xform) |
| `VehicleSeat` | struct | One seat: `SeatInfo`, `SeatAddon`, `Passenger` |
| `VehicleSeatAddon` | struct | DB-driven seat orientation/exit overrides |
| `VehicleAccessory` | struct | Auto-summoned passenger spec |
| `VehicleTemplate` | struct | Per-entry despawn delay |
| `PassengerInfo` | struct | Guid + IsUninteractible + IsGravityDisabled |
| `PowerType` | enum | Vehicle power resources (Steam, Pyrite, Heat, Ooze, Blood, etc.) |
| `VehicleFlags` | enum | NO_STRAFE, NO_JUMPING, FULLSPEEDTURNING, ALLOW_PITCHING, FIXED_POSITION, etc. |
| `VehicleSpells` | enum | `RIDE_HARDCODED = 46598`, `PARACHUTE = 45472` |
| `VehicleExitParameters` | enum class | None / Offset / Dest |
| `Vehicle::Status` | private enum | NONE / INSTALLED / UNINSTALLING |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `Vehicle(Unit*, VehicleEntry const*, uint32 creatureEntry)` | Construct from DBC | `Unit::SetVehicle` |
| `Install()` / `Uninstall()` | Lifecycle gates (status transitions) | `ApplyAllImmunities`, `RemoveAllPassengers` |
| `Reset(bool evading)` | Reset on creature respawn / evade | `InstallAllAccessories` |
| `InstallAllAccessories(bool evading)` | Spawn accessory creatures from `VehicleAccessoryContainer` | `InstallAccessory` |
| `InstallAccessory(entry, seatId, minion, type, summonTime)` | Summon one accessory NPC into a seat | `Map::SummonCreature`, `AddVehiclePassenger` |
| `AddVehiclePassenger(Unit*, int8 seatId)` | Schedule `VehicleJoinEvent`; -1 picks first empty | `GetNextEmptySeat`, `EventMgr` |
| `RemovePassenger(WorldObject*)` | Remove from seat, fire scripts, clear control aura | `Unit::RemoveAurasByType` |
| `RelocatePassengers()` | Push transport-frame moves to global | `TransportBase::CalculatePassengerPosition` |
| `GetSeatForPassenger(Unit const*)` | Lookup seat DBC entry | `SeatMap` find |
| `HasEmptySeat(int8)` / `GetAvailableSeatCount()` | Capacity queries | — |
| `GetNextEmptySeat(int8, bool next)` | Iterate seats wrap-around | — |
| `IsControllableVehicle()` | Any seat with `VEHICLE_SEAT_FLAG_CAN_CONTROL` | — |
| `GetDespawnDelay()` | From `VehicleTemplate` | — |
| `VehicleJoinEvent::Execute` | Two-phase apply on next tick (validates still alive, places passenger) | `Vehicle::AddPassenger` |

---

## 5. Module dependencies

**Depends on:**
- `Unit` (`_me` is the underlying unit; aura type 236 `SPELL_AURA_CONTROL_VEHICLE` drives mounts)
- `Map` (passenger summons, position updates)
- `SpellAuras` (`Aura::AURA_REMOVE_BY_*` paths trigger Uninstall)
- `DBCStores` (`VehicleEntry`, `VehicleSeatEntry`)
- `EventProcessor` (delayed `VehicleJoinEvent`)
- `ObjectMgr` (`VehicleAccessoryContainer`, `VehicleTemplateContainer`)

**Depended on by:**
- `Unit` (owns `Vehicle*` via `m_vehicleKit`)
- `WorldSession::HandleRequestVehicleExit`, `HandleRequestVehicleSwitchSeat`, `HandleEjectPassenger` (`VehicleHandler.cpp`)
- `MovementHandler` (transport frame in MOVEMENT_INFO)
- Scripts (siege engines, Ulduar/ICC vehicle bosses)

---

## 6. SQL / DB queries

`Vehicle.cpp` does not query directly. Tables are loaded by `ObjectMgr`/`SpellMgr`:

| Statement / Source | Purpose | DB |
|---|---|---|
| `vehicle_template` | per-entry despawn delay | world; loaded in Rust with C++ 1ms fallback |
| `vehicle_template_accessory` | accessories by vehicle entry | world |
| `vehicle_accessory` | accessories by creature spawn | world |
| `vehicle_seat_addon` | seat orientation/exit overrides | world |

DBC stores:

| Store | What it loads | Read by |
|---|---|---|
| `VehicleStore` | `Vehicle.dbc` (kit definition, flags, seats[8]) | `Vehicle` ctor |
| `VehicleSeatStore` | `VehicleSeat.dbc` (per-seat flags, attachment offsets, exit) | `Vehicle::AddVehiclePassenger` |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_REQUEST_VEHICLE_EXIT` | C → S | `WorldSession::HandleRequestVehicleExit` |
| `CMSG_REQUEST_VEHICLE_PREV_SEAT` / `_NEXT_SEAT` / `_SWITCH_SEAT` | C → S | seat shuffle |
| `CMSG_RIDE_VEHICLE_INTERACT` | C → S | NPC vehicle entry |
| `CMSG_EJECT_PASSENGER` | C → S | driver ejects passenger |
| `CMSG_MOVE_DISMISS_VEHICLE` | C → S | full dismount |
| `CMSG_MOVE_CHANGE_VEHICLE_SEATS` | C → S | client-side seat swap ack |
| `SMSG_CONTROL_VEHICLE` (on board) | S → C | via aura `SPELL_AURA_CONTROL_VEHICLE` |
| `SMSG_SET_VEHICLE_REC_ID` / `SMSG_MOVE_SET_VEHICLE_REC_ID` (+ ack) | S → C | switch vehicle kit while seated |

(See `crates/wow-constants/src/opcodes.rs` — opcode IDs already defined: `RequestVehicleExit`, `RequestVehicleSwitchSeat`, `MoveDismissVehicle`, `RideVehicleInteract`, `SetVehicleRecId`, etc.)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-entities/src/vehicle.rs` — represented `Vehicle` state, seat map, passenger helpers, pending join markers, transport offset/global transforms, and accessory POD.
- `crates/wow-data/src/vehicle.rs` — `Vehicle.db2`/`VehicleSeat.db2` stores with hotfix overlays, C++ `VehicleSeatEntry` helper flags used by handlers, C++ `vehicle_template` despawn-delay lookup, and C++ `vehicle_accessory`/`vehicle_template_accessory` lookup.
- `crates/wow-world/src/session.rs` — represented mount VehicleKit create/remove path, owner vehicle-rec packets, movement ack validation, mount accessory row selection, collision-height and pet-mode side effects.
- `crates/wow-packet/src/packets/movement.rs` / `crates/wow-packet/src/packets/vehicle.rs` — movement vehicle id and represented vehicle-rec packets.
- `crates/wow-constants/src/vehicle.rs` / `crates/wow-constants/src/opcodes.rs` — C++ `VehicleDefines.h` power/flag/spell/exit constants and vehicle opcodes enumerated.
- `crates/wow-packet/src/packets/vehicle.rs` / `crates/wow-world/src/handlers/vehicle.rs` — vehicle packet layouts and dispatch metadata registered with C++ status/processing; handlers currently preserve C++ early-return behavior until live passenger/charm runtime exists.

**What's implemented:** represented VehicleKit state and mount integration, C++ `VehicleDefines.h` constants, DB2 seat construction and handler helper flags, C++ VehicleSeat attachment offsets/keep-pet flags, C++ vehicle template despawn-delay lookup, C++ vehicle accessory row lookup and `InstallAccessory`/`InstallAllAccessories` planning, C++ `InitMovementInfoForBase`, `AddVehiclePassenger`, `VehicleJoinEvent::Execute/Abort`, `RemovePassenger`, `RemoveAllPassengers`, `RelocatePassengers`, `GetDebugInfo`, `ApplyAllImmunities`, vehicle handler decision plans, and `Reset(bool evading)` planning, movement/vehicle-rec packet coverage for the mount path, and focused unit tests for the represented state.

**What's missing vs C++:** full live passenger runtime, accessory TempSummon/HandleSpellClick execution, live immunity application, script hooks, despawn timer, and join event scheduling. `wow_entities::Vehicle` now represents install/uninstall status, seat maps, usable-seat counts, passenger insert/remove helpers, C++ passenger seat info/addon lookup, C++ `InitMovementInfoForBase` MovementFlag2 mapping, C++ `AddVehiclePassenger` seat-selection/pending/displacement plans, C++ `VehicleJoinEvent::Execute/Abort` boarding side-effect plans, C++ `RemovePassenger` restoration/transport/charm/script plans, C++ pending-event abort plans, C++ `RemoveAllPassengers` pending-abort/aura/forced-exit plans, C++ `RelocatePassengers` transport-offset plan, C++ `InstallAccessory` summon/HandleSpellClick plan, C++ `InstallAllAccessories(bool evading)` remove/filter planning, C++ `ApplyAllImmunities` effect/state/mechanic plans, and the C++ `Reset(bool evading)` unit/alive gates; vehicle accessory row selection is loaded in `wow-data` with C++ GUID-first/template fallback semantics.

**Suspicious / likely divergent:** represented mount path records accessory rows but does not yet execute C++ `TempSummon` + `HandleSpellClick`; request handlers for exit/switch/eject/dismiss are registered and parse packets but are not wired to live passenger/charm state yet.

**Tests existing:** focused `wow-entities`, `wow-data`, and `wow-world` unit tests for seat/passenger helpers, transforms, accessory lookup, and represented mount VehicleKit create/remove behavior.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_VEHICLE.WBS.001** Partir y cerrar la migracion auditada de `game/Entities/Vehicle/Vehicle.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 995 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_VEHICLE.WBS.002** Cerrar la migracion auditada de `game/Entities/Vehicle/Vehicle.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_VEHICLE.WBS.003** Cerrar la migracion auditada de `game/Entities/Vehicle/VehicleDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [x] **#VEH.1** Define `VehicleEntry` / `VehicleSeatEntry` DBC readers in `wow-data` (L)
- [x] **#VEH.2** Port `VehicleFlags`, `PowerType`, `VehicleExitParameters`, `VehicleSpells` to `wow-constants` (L)
- [x] **#VEH.3** Define `VehicleSeat`, `VehicleSeatAddon`, `VehicleAccessory`, `VehicleTemplate`, `PassengerInfo` POD types in `wow-world` (L) — represented types live in `wow-entities`.
- [x] **#VEH.4** Port `TransportBase` xform (`CalculatePassengerPosition`/`Offset`) as free fns in `wow-entities` (L)
- [ ] **#VEH.5** Implement `Vehicle` struct + `Install/Uninstall/Reset` lifecycle (M) — partial: install/uninstall and C++ `Reset(evading)` planning represented; live `Unit` ownership, script hooks, and side-effect execution pending.
- [ ] **#VEH.6** Implement `AddVehiclePassenger`/`RemovePassenger`/`HasEmptySeat`/`GetNextEmptySeat` (M) — partial: pure seat-map helpers, passenger seat info/addon lookup, pending-aware available-seat count, C++ controllable-seat detection, C++ `AddVehiclePassenger` join scheduling plan, C++ `RemovePassenger` side-effect plan, and C++ `RemoveAllPassengers` side-effect plan represented; live aura/script/passenger runtime side effects pending.
- [ ] **#VEH.7** Implement `InstallAllAccessories` + `InstallAccessory` (depends on TempSummon) (M) — partial: C++ accessory row load/selection, uninstalling guard, minion accessory mask, HandleSpellClick seat plan, and `evading` minion filter/remove-passenger planning covered; TempSummon/HandleSpellClick execution pending.
- [ ] **#VEH.8** Implement delayed join (`VehicleJoinEvent` analog) on session/map tick (M) — partial: pending-event remove/abort plans and C++ `Execute`/`Abort` side-effect plans represented; live tick execution pending.
- [ ] **#VEH.9** Wire vehicle opcodes (Exit/SwitchSeat/Eject/Dismiss) into `wow-handler` (M) — partial: packet parsers, C++ `CanSwitchFromSeat`/`IsEjectable` helper flags, dispatch metadata, and C++ handler branch/action plans covered; live `ExitVehicle`/`ChangeSeat`/`HandleSpellClick` execution pending.
- [ ] **#VEH.10** Apply immunities (`ApplyAllImmunities` from VehicleEntry flags) (L) — partial: C++ immunity/root plan represented; live `Unit::ApplySpellImmune`/`SetControlled` application pending.
- [ ] **#VEH.11** Hook `RelocatePassengers` into MapManager tick (M) — partial: C++ transport-offset relocation plan represented; live `Map::UpdatePassengerPosition` execution pending.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_VEHICLE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 3 files / 1342 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_VEHICLE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 3 files / 1342 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_VEHICLE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 3 files / 1342 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_VEHICLE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 3 files / 1342 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: empty vehicle has correct `GetAvailableSeatCount` from DBC seat array
- [ ] Test: `AddVehiclePassenger(seatId=-1)` picks first empty in DBC index order
- [ ] Test: passenger removal clears seat and fires control-aura removal path
- [ ] Test: `CalculatePassengerPosition(offset, transO)` round-trips with `CalculatePassengerOffset`
- [x] Test: `Vehicle::GetDespawnDelay` returns `vehicle_template.despawnDelayMs` and defaults to 1ms
- [x] Test: accessory row lookup prefers `vehicle_accessory` spawn GUID rows and falls back to `vehicle_template_accessory`
- [ ] Test: accessory install summons N creatures matching `vehicle_accessory` rows
- [ ] Test: `IsControllableVehicle` true iff any seat has `CAN_CONTROL` flag

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 3 files / 1342 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h` | `crates/wow-world/` (entity logic), `crates/wow-data/` (DBC), `crates/wow-constants/` (flags/opcodes) \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_VEHICLE.DIV.001` | _none generated_ | 3 C++ files / 1342 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/VehicleDefines.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Vehicle/Vehicle.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- Aura 236 `SPELL_AURA_CONTROL_VEHICLE` is the single drive of mount/dismount — `Vehicle::AddVehiclePassenger` is **always** invoked from aura apply, never directly from a CMSG. Don't write a "mount handler" — write an aura effect.
- `VehicleJoinEvent` exists because aura apply happens mid-spell-cast and you cannot complete the seat reservation synchronously without re-entering the spell system. Mirror this: defer the actual seat write by one tick.
- `_status` (NONE/INSTALLED/UNINSTALLING) gates against double-uninstall during script callbacks. Mirror with an enum, not a bool.
- WoLK 3.4: `VehicleSeat.dbc` flags include `CAN_ENTER_OR_EXIT`, `CAN_CONTROL`, `CAN_ATTACK`, `UNCONTROLLED`, `KICKABLE_BY_HOSTILE`. Don't drop `UNCONTROLLED` — used by Oculus drakes.
- Hardcoded spell `46598` (RIDE_HARDCODED) is the generic dummy ride aura applied when scripts force-mount.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Vehicle : TransportBase` | `struct Vehicle` + `impl TransportBase for Vehicle` | trait, not class |
| `Unit* _me` | `ObjectGuid` + lookup via `MapManager` | avoid back-pointer |
| `SeatMap = std::map<int8, VehicleSeat>` | `BTreeMap<i8, VehicleSeat>` | seat order matters |
| `VehicleEntry const*` | `&'static VehicleEntry` from DBC store | — |
| `VehicleJoinEvent : BasicEvent` | enqueue on map's `pending_events: Vec<DelayedEvent>` | tick-deferred |
| `GuidSet vehiclePlayers` | `HashSet<ObjectGuid>` | parking_lot or per-map lock |
| `std::list<VehicleJoinEvent*> _pendingJoinEvents` | `Vec<VehicleJoinEvent>` | small N |
| `Status _status` enum | `enum VehicleStatus { None, Installed, Uninstalling }` | direct |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Vehicle` | partial | `crates/wow-entities/src/vehicle.rs`; `crates/wow-world/src/session.rs` | ⚠️ represented state/lifecycle only; live Unit ownership and scripts pending |
| `class VehicleJoinEvent` | no | — | ❌ missing |
| `class TransportBase` | partial | `crates/wow-entities/src/vehicle.rs` | ⚠️ offset/global transform helpers ported; trait integration pending |
| `enum VehicleFlags` | yes | `crates/wow-constants/src/vehicle.rs` | ✅ present |
| `enum PowerType` (vehicle powers) | yes | `crates/wow-constants/src/vehicle.rs` | ✅ present as `VehiclePowerType` |
| `struct VehicleSeat` / `VehicleSeatAddon` / `VehicleAccessory` / `VehicleTemplate` / `PassengerInfo` | yes | `crates/wow-entities/src/vehicle.rs` | ✅ represented POD/state present |
| `enum VehicleExitParameters` | yes | `crates/wow-constants/src/vehicle.rs` | ✅ present as `VehicleExitParameter` |
| `VEHICLE_SPELL_RIDE_HARDCODED` (46598) | yes | `crates/wow-constants/src/vehicle.rs` | ✅ present as `VehicleSpell::RideHardcoded` |
| `TypeId::Vehicle = 9` | yes | `crates/wow-constants/src/object.rs` | ✅ present (constant only) |
| `CMSG_REQUEST_VEHICLE_*` opcodes | yes (constants) | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no handler |
| `MOVEMENT_INFO::Transport.vehicle_id` | yes | `crates/wow-packet/src/packets/movement.rs` | ✅ wire-format only |

**Verdict:** partial. `Vehicle` entity state, seat definitions, represented mount VehicleKit creation/removal packets, movement ack validation, and C++ accessory-row lookup are covered. Full passenger/aura-control runtime, accessory summoning, transport transforms, immunities, and script integration remain open.
