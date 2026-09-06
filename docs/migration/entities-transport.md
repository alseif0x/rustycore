# Migration: Entities / Transport

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/` + `src/server/game/Maps/MapObject.h`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-map/`, `crates/wow-data/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ❌ not started
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`Transport` is the moving-platform entity: zeppelins, boats (Booty Bay → Ratchet, etc.), elevators (Undercity, Aldor Rise), Icebreaker (Wintergrasp). It is a `GameObject` *and* a `TransportBase` — it carries world objects (players, NPCs, gobs) along a scripted path stored in `TransportTemplate`/`TransportAnimation` and synchronously moves them by transforming their offset position into world coordinates each tick. `MapObject.h` provides the lightweight grid-cell tracking mixin used by Transports, Creatures, GameObjects, DynamicObjects, and AreaTriggers.

WoLK 3.4 distinguishes **MOTransport** (moving, world-spanning, follows TaxiPathNodes) from **static GameObject transports** (elevators inside a map; just `GAMEOBJECT_TYPE_TRANSPORT` rotating).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/Transport/Transport.cpp` | 740 | `prefix` |
| `game/Entities/Transport/Transport.h` | 129 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/Transport/Transport.h` | 129 | `Transport` class; final, inherits `GameObject` + `TransportBase` |
| `src/server/game/Entities/Transport/Transport.cpp` | 740 | path progression, passenger add/remove, NPC/GO passenger creation, teleport between maps |
| `src/server/game/Maps/MapObject.h` | 60 | `MapObject` mixin: `_currentCell`, `_moveState`, `_newPosition` |

(`TransportMgr` lives under `src/server/game/Maps/` and owns the `TransportTemplate` registry plus the `CreateTransport` factory.)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Transport` | class (final, GameObject + TransportBase) | World moving platform |
| `Transport::PassengerSet` | typedef `std::set<WorldObject*>` | Passenger registry |
| `MapObject` | class | Grid-cell tracking mixin |
| `MapObjectCellMoveState` | enum | NONE / ACTIVE / INACTIVE (cell-relocation queue) |
| `TransportTemplate` | struct (in `TransportMgr.h`) | Static template per entry |
| `TransportMovementState` | enum | Path state (in `TransportMgr.h`) |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `Transport::Create(guidlow, entry, x, y, z, ang)` | Construct from gameobject_template entry | `GameObject::Create` |
| `Transport::Update(uint32 diff)` | Advance `_pathProgress`, fire path-leg events, relocate passengers, possibly teleport between maps | `UpdatePosition`, `TeleportTransport` |
| `Transport::AddPassenger(WorldObject*)` | Insert into `_passengers`; convert global → offset position | `TransportBase::CalculatePassengerOffset` |
| `Transport::RemovePassenger(WorldObject*)` | Remove from `_passengers` or `_staticPassengers` | — |
| `Transport::CreateNPCPassenger(guid, CreatureData const*)` | Spawn NPC bound to transport | `Map::AddToMap` |
| `Transport::CreateGOPassenger(guid, GameObjectData const*)` | Spawn GO bound to transport | — |
| `Transport::SummonPassenger(entry, pos, summonType, ...)` | TempSummon a creature in transport offsets | `Map::SummonCreature` |
| `Transport::UpdatePosition(x, y, z, o)` | Set transport's own world position + propagate to passengers | `UpdatePassengerPositions` |
| `Transport::LoadStaticPassengers` / `UnloadStaticPassengers` | Load/unload spawn-table NPCs/GOs when grid activates | `ObjectMgr::Get*Data` |
| `Transport::TeleportTransport(oldMap, newMap, x, y, z, o)` | Cross-map jump (e.g. zeppelin OG↔Northrend) | `TeleportPassengersAndHideTransport` |
| `Transport::EnableMovement(bool)` | Pause/resume path progression | — |
| `Transport::GetExpectedMapId()` | Map id at current path progress | `TransportMgr` |

---

## 5. Module dependencies

**Depends on:**
- `GameObject` (base class — handles GAMEOBJECT_TYPE_MO_TRANSPORT)
- `TransportMgr` (`TransportTemplate` + `CreateTransport` factory; path animation timeline)
- `Map` / `MapManager` (cross-map teleport; passenger map membership)
- `ObjectMgr` (`CreatureData`, `GameObjectData` for static passengers)
- `TaxiPathNodeStore` (DBC) for path geometry
- `MovementInfo` (passengers carry `transport.guid` + offset)

**Depended on by:**
- `Player` (transport guid in MOVEMENT_INFO; logout/login restoration)
- `Creature` (NPC passengers)
- `WorldSession::HandleMoveWorldportAck` (cross-map zeppelin teleport)
- Scripts (Wintergrasp, Strand of the Ancients use scripted transports)

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT entry, name, ... FROM gameobject_template WHERE type = 11 OR type = 15` | MO transports | world |
| `transports` (table) | Spawn list of MOTransports per map | world |
| `creature` rows with `transport.guid` set | Static passengers | world |
| `gameobject` rows with transport binding | Static GO passengers | world |

DBC stores:

| Store | What it loads | Read by |
|---|---|---|
| `TaxiPathNodeStore` | TaxiPathNode.dbc | `TransportMgr::GenerateWaypoints` |
| `TransportAnimationStore` | TransportAnimation.dbc | per-tick interpolation |
| `TransportRotationStore` | TransportRotation.dbc | rotational interpolation |
| `GameObjectDisplayInfoStore` | display info for the model | rendering |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `MSG_MOVE_*` (with `MOVEMENTFLAG_ONTRANSPORT`) | both | passenger position carries transport guid + offset |
| `SMSG_UPDATE_OBJECT` (transport) | S → C | path state, model, level=period |
| `CMSG_MOVE_CHANGE_TRANSPORT` | C → S | client acks transport switch |
| `CMSG_QUERY_CORPSE_TRANSPORT` | C → S | corpse-on-transport query |
| `SMSG_CORPSE_TRANSPORT_QUERY` | S → C | response |
| `SMSG_TRANSFER_PENDING` | S → C | sent before zeppelin map jump |

(In `crates/wow-constants/src/opcodes.rs`: `MoveChangeTransport`, `QueryCorpseTransport`, `CorpseTransportQuery`.)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-map` | `crate_dir` | 3 | 558 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/object.rs` — `TypeId::Transport = 6` defined
- `crates/wow-packet/src/packets/movement.rs` — `TransportInfo` struct in MOVEMENT_INFO; transport guid serialized
- `crates/wow-constants/src/opcodes.rs` — `MoveChangeTransport`, `CorpseTransportQuery`, `QueryCorpseTransport` constants
- **0 lines** of `Transport` entity logic, no `MapObject`, no `TransportMgr`, no path animation.

**What's implemented:** wire format awareness only (passenger movement info can encode "I am on transport X with offset Y").

**What's missing vs C++:** entire 740-line `Transport.cpp` — path progression, passenger relocation, NPC/GO summon on transport, cross-map teleport, static passenger load/unload. `TransportMgr` and `TransportTemplate` infrastructure.

**Suspicious / likely divergent:** none — nothing exists.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_TRANSPORT.WBS.001** Partir y cerrar la migracion auditada de `game/Entities/Transport/Transport.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`
  Rust target: `crates/wow-world`, `crates/wow-map`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 740 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_TRANSPORT.WBS.002** Cerrar la migracion auditada de `game/Entities/Transport/Transport.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h`
  Rust target: `crates/wow-world`, `crates/wow-map`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#TRP.1** Port `MapObject` mixin (`_currentCell`, `_moveState`, `_newPosition`) — likely a struct used by Creature/GameObject/etc. (L)
- [ ] **#TRP.2** Define `TransportTemplate` + waypoint timeline in `wow-data` (M)
- [ ] **#TRP.3** Implement `TransportMgr` (load templates, generate waypoints from TaxiPathNode.dbc) (H)
- [ ] **#TRP.4** Define `Transport` struct as a specialized GameObject + `TransportBase` impl (M)
- [ ] **#TRP.5** Implement `Update` path progression + interpolation (M)
- [ ] **#TRP.6** Implement `AddPassenger`/`RemovePassenger` + global↔offset xform (L) (shares math with Vehicle #VEH.4)
- [ ] **#TRP.7** Implement `LoadStaticPassengers`/`UnloadStaticPassengers` from `creature`/`gameobject` rows with transport binding (M)
- [ ] **#TRP.8** Implement `TeleportTransport` cross-map (e.g. Orgrimmar→Borean) — coordinate with MapManager (H)
- [ ] **#TRP.9** Wire `MOVEMENTFLAG_ONTRANSPORT` decode → resolve passenger transport-frame moves (L) (already partially in `movement.rs`)
- [ ] **#TRP.10** Static (in-map elevator) GAMEOBJECT_TYPE_TRANSPORT rotation tick — separate from MO (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_TRANSPORT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 869 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-map`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-map` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_TRANSPORT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 869 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-map`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_TRANSPORT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 869 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-map`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_TRANSPORT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 869 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-map`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: passenger added at world (X,Y,Z) gets correct local offset given transport pose
- [ ] Test: applying that offset back through `CalculatePassengerPosition` returns the original world point (round-trip)
- [ ] Test: path progression wraps `_pathProgress mod period`
- [ ] Test: cross-map teleport moves all `_passengers` to new map and restores their offsets
- [ ] Test: `LoadStaticPassengers` spawns expected count of NPCs from world DB
- [ ] Test: `EnableMovement(false)` halts `_pathProgress`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 869 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h` | `crates/wow-world/`, `crates/wow-map/`, `crates/wow-data/`, `crates/wow-constants/` \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_TRANSPORT.DIV.001` | _none generated_ | 2 C++ files / 869 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/Transport/Transport.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- Two distinct entity classes share the name "transport" in WoLK 3.4: **MOTransport** (`GAMEOBJECT_TYPE_MO_TRANSPORT = 15`, this class) vs. static elevator (`GAMEOBJECT_TYPE_TRANSPORT = 11`, plain `GameObject`). Don't conflate; only the former is what `class Transport` represents.
- `_pathProgress` is in **milliseconds modulo period**; `period = m_gameObjectData->Level` (yes, the level field is reused as period — this is intentional 3.4 wire format).
- `_eventsToTrigger` is a `boost::dynamic_bitset` over path events; in Rust use a `Vec<bool>` or `BitVec`.
- Cross-map teleport (e.g. Orgrimmar zeppelin → Borean Tundra) requires cooperative dance: server moves transport to new map, sends `SMSG_TRANSFER_PENDING` to passengers, awaits `CMSG_MOVE_WORLDPORT_ACK`. Easy to break.
- Static passengers are loaded/unloaded based on **grid activity** of the transport's current cell — when no player nearby, drop the NPCs to save memory.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Transport : GameObject, TransportBase` | `struct Transport` containing `GameObjectBase` + `impl TransportBase` | composition not inheritance |
| `PassengerSet = std::set<WorldObject*>` | `HashSet<ObjectGuid>` | resolve via MapManager |
| `std::unique_ptr<boost::dynamic_bitset<u8>>` | `bitvec::BitVec` or `Vec<bool>` | small N |
| `TransportTemplate const*` | `&'static TransportTemplate` from `wow-data` | DBC-loaded |
| `Optional<uint32> _requestStopTimestamp` | `Option<u32>` | direct |
| `TimeTracker _positionChangeTimer` | own `TimeTracker` in `wow-core` (already exists for cooldowns) | reuse |
| `class MapObject` (mixin) | `struct CellLocation { current_cell, move_state, new_position }` field | composition |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class Transport` | no | — | ❌ missing |
| `class MapObject` | no | — | ❌ missing |
| `enum MapObjectCellMoveState` | no | — | ❌ missing |
| `Transport::Update / AddPassenger / RemovePassenger` | no | — | ❌ missing |
| `Transport::TeleportTransport` | no | — | ❌ missing |
| `Transport::LoadStaticPassengers` / `UnloadStaticPassengers` | no | — | ❌ missing |
| `class TransportBase` | no | — | ❌ missing (shared with Vehicle) |
| `TypeId::Transport = 6` | yes | `crates/wow-constants/src/object.rs` | ✅ present |
| `MOVEMENT_INFO::Transport` field | yes | `crates/wow-packet/src/packets/movement.rs` | ✅ wire-format only |
| `CMSG_MOVE_CHANGE_TRANSPORT` opcode constant | yes | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no handler |

**Verdict:** ❌ not started. Surface coverage ≈ 1%. No entity, no path, no MapObject mixin, no handler.
