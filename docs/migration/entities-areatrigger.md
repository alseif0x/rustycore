# Migration: Entities / AreaTrigger

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-data/`, `crates/wow-spell/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ⚠️ partial (legacy-table teleport triggers loaded; no entity, no shape eval, no spell-spawned ATs)
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`AreaTrigger` is the spell-spawned, client-visible (or server-side) volumetric trigger used by spells with attached AT visuals (Death and Decay, Cataclysm-era and post-WoLK extensively, but in 3.4 mostly persistent visuals + a few server-side polygon checks). Distinct from the legacy DBC `AreaTrigger.dbc` static map triggers (those are coordinates only and handled in `WorldSession::HandleAreaTriggerOpcode`); this entity is the *spawned* one with shape, duration, spline/orbit movement, and inside-units tracking. **In WoLK 3.4 the spell-spawned variant is rare** — most "area trigger" gameplay in WoLK is just legacy-table teleport/quest triggers, which are not entities.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/AreaTrigger/AreaTrigger.cpp` | 1453 | `prefix` |
| `game/Entities/AreaTrigger/AreaTrigger.h` | 238 | `prefix` |
| `game/Entities/AreaTrigger/AreaTriggerTemplate.cpp` | 111 | `prefix` |
| `game/Entities/AreaTrigger/AreaTriggerTemplate.h` | 271 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/AreaTrigger/AreaTrigger.h` | 238 | `AreaTrigger` class (final, WorldObject + GridObject + MapObject) |
| `src/server/game/Entities/AreaTrigger/AreaTrigger.cpp` | 1453 | Create, Update, shape evaluation, spline/orbit, target list, enter/exit actions |
| `src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h` | 271 | `AreaTriggerTemplate`, `AreaTriggerCreateProperties`, `AreaTriggerShapeInfo`, `AreaTriggerOrbitInfo`, action enums, flag enums |
| `src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.cpp` | 111 | Template helpers (max search radius, shape construction) |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `AreaTrigger` | class (final) | Spell/spawn-driven volumetric trigger entity |
| `AreaTriggerTemplate` | struct | Static template (id + flags + actions) |
| `AreaTriggerCreateProperties` | struct | Per-cast properties (shape, scale curve, spline, orbit) |
| `AreaTriggerShapeInfo` | struct | Sphere/Box/Polygon/Cylinder/Disk/BoundedPlane payload (variant) |
| `AreaTriggerOrbitInfo` | struct | Center, radius, ang vel, period for orbit AT |
| `AreaTriggerAction` | struct | One enter/exit action: `ActionType` × `TargetType` × `Param` |
| `AreaTriggerSpawn` | struct | DB row for static-spawn AT (rare in 3.4) |
| `AreaTriggerFlag` | enum | `IsServerSide` |
| `AreaTriggerShapeType` | enum | Sphere=0, Box=1, Polygon=3, Cylinder=4, Disk=5, BoundedPlane=6 |
| `AreaTriggerActionTypes` | enum | `CAST=0`, `ADDAURA=1`, `TELEPORT=2` |
| `AreaTriggerActionUserTypes` | enum | Any/Friend/Enemy/Raid/Party/Caster |
| `AreaTriggerCreatePropertiesFlag` | enum | HasAbsoluteOrientation, HasDynamicShape, HasAttached, HasFaceMovementDir, etc. |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `CreateAreaTrigger(propsId, pos, duration, caster, target, ...)` | Static factory; spell-spawned path | `Create`, `Map::AddToMap` |
| `LoadFromDB(spawnId, map, addToMap, allowDuplicate)` | Static-spawn AT (DB-driven) | — |
| `Update(uint32 diff)` | Tick: scale curves, spline pos, orbit pos, polygon vertex update, target list re-evaluation | `UpdateShape`, `UpdateTargetList`, `UpdateOrbitPosition`, `UpdateSplinePosition` |
| `UpdateShape()` | Recompute polygon vertices on rotation | `UpdatePolygonVertices` |
| `UpdateTargetList()` | Find units inside; fire enter/exit actions | `SearchUnits`, `HandleUnitEnterExit` |
| `HandleUnitEnterExit(targetList)` | Diff with `_insideUnits`; fire `DoActions` / `UndoActions` | `DoActions`, `UndoActions` |
| `SearchUnitInSphere/Box/Polygon/Cylinder/Disk/BoundedPlane` | Per-shape unit query | grid search |
| `InitSplineOffsets` / `InitSplines` | Configure path (e.g. moving frost cone) | `Movement::Spline` |
| `InitOrbit(orbitInfo, timeToTarget)` | Configure orbit motion | — |
| `SetDuration(int32)` / `Delay(delaytime)` | Lifetime control | `_UpdateDuration` |
| `SetOverrideScaleCurve` / `SetExtraScaleCurve` / `SetOverrideMoveCurve` | Client-side animation curves | `SetUpdateFieldValue` |
| `Remove()` | Despawn + AI destroy | `_ai->OnRemove`, `RemoveFromWorld` |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` / `GridObject` / `MapObject` (entity base)
- `AreaTriggerTemplate` / `AreaTriggerCreateProperties` (registry in `ObjectMgr` / `AreaTriggerDataStore`)
- `Spell` / `SpellInfo` / `AuraEffect` (origin spell — the trigger is born from a SpellEffect)
- `Movement::Spline` (path)
- `AreaTriggerAI` (script hook for custom logic)
- DBC: `AreaTriggerStore` (static visual-only entries), curves: `CurveStore`, `CurvePointStore`

**Depended on by:**
- `Spell::EffectCreateAreaTrigger` (the SpellEffect that spawns one)
- Scripts (`AreaTriggerAI` subclasses for boss mechanics)
- AuraEffects with attached AT visuals

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `areatrigger_template` | Per-AT-id flags, action list | world |
| `areatrigger_create_properties` | Per-cast properties (shape, scale, splines) | world |
| `areatrigger_create_properties_polygon_vertex` | Polygon vertices | world |
| `areatrigger_create_properties_spline_point` | Spline path points | world |
| `spawned_areatrigger` (3.4 unused) | Static-spawn AT rows | world |

**Legacy `AreaTrigger.dbc`** (different beast — coordinate-only static triggers used for teleport/quest objective entry; loaded by `wow-data/src/area_trigger.rs` already): not the same module, but commonly conflated.

DBC stores:

| Store | What it loads | Read by |
|---|---|---|
| `AreaTriggerStore` (DBC) | static visual-only AT positions | unrelated to spell ATs |
| `CurveStore` / `CurvePointStore` | client interpolation curves | scale/move curve fields |

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` (with AT block) | S → C | spawn — `m_areaTriggerData` UF block |
| `CMSG_AREA_TRIGGER` | C → S | **legacy** AT entered (DBC table) — handled in `WorldSession::HandleAreaTrigger` |
| `SMSG_AREA_TRIGGER_NO_CORPSE` | S → C | corpse-required AT failed | (`crates/wow-constants/src/opcodes.rs`: `AreaTriggerNoCorpse = 0x2716`) |
| `SMSG_AREA_TRIGGER_DENIED` | S → C | AT entry denied | (`AreaTriggerDenied = 0x2903`) |
| `SMSG_AREA_TRIGGER_RE_PATH` / `RE_SHAPE` | S → C | dynamic re-path / re-shape | (`AreaTriggerRePath`, `AreaTriggerReShape`) |
| `SMSG_AREA_TRIGGER_FORCE_SET_POSITION_AND_FACING` | S → C | server-forced position correction | (`AreaTriggerForceSetPositionAndFacing`) |
| `SMSG_AREA_TRIGGER_UNATTACH` | S → C | detach from caster | (`AreaTriggerUnattach`) |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-data` | `crate_dir` | 11 | 3505 | `exists_active` | crate exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-data/src/area_trigger.rs` | `file` | 1 | 312 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-data/src/area_trigger.rs` — `AreaTriggerData`, `AreaTriggerStore`, `AreaTriggerTeleport`, `TriggerShape`, `load_area_triggers` (DB loader). **This is the legacy DBC-table side**, not the entity.
- `crates/wow-world/src/handlers/misc.rs` — `handle_area_trigger` consumes `CMSG_AREA_TRIGGER` and consults `AreaTriggerStore` for teleport/quest checks
- `crates/wow-world/src/session.rs` — `active_area_trigger: Option<u32>`, `check_area_triggers` for movement-time entry detection
- `crates/wow-constants/src/object.rs` — `TypeId::AreaTrigger = 11`
- `crates/wow-core/src/guid.rs` — `HighGuid::AreaTrigger = 13`, `is_area_trigger`
- `crates/wow-constants/src/opcodes.rs` — full set of AT opcodes enumerated

**0 lines** of `AreaTrigger` *entity* logic (the spell-spawned, shape-bearing, target-tracking variant).

**What's implemented:** Legacy-table CMSG_AREA_TRIGGER pipeline (player walks into a coordinate, server runs teleport/quest progression). Opcodes enumerated.

**What's missing vs C++:** Entire 1453-line `AreaTrigger.cpp` entity — shape evaluation, spline/orbit, target list diff, enter/exit actions, AT AI hook, scale curves, the `AreaTriggerData` UF block.

**Suspicious / likely divergent:**
- The `wow-data/area_trigger.rs::TriggerShape` enum likely covers only sphere/box for legacy teleport zones; it is **not** a substitute for the entity-level polygon/cylinder/disk/bounded-plane shapes.
- `check_area_triggers` is a polling loop over the DBC-table set — it doesn't know about spawned entity ATs.

**Tests existing:** Whatever is in `wow-data/area_trigger.rs` for the loader; 0 entity tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_AREATRIGGER.WBS.001** Partir y cerrar la migracion auditada de `game/Entities/AreaTrigger/AreaTrigger.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1453 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ENTITIES_AREATRIGGER.WBS.002** Cerrar la migracion auditada de `game/Entities/AreaTrigger/AreaTrigger.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_AREATRIGGER.WBS.003** Cerrar la migracion auditada de `game/Entities/AreaTrigger/AreaTriggerTemplate.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_AREATRIGGER.WBS.004** Cerrar la migracion auditada de `game/Entities/AreaTrigger/AreaTriggerTemplate.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#AT.1** Decide scope for WoLK 3.4: minimum is `AreaTriggerShapeType::Sphere` + `Polygon` + server-side actions; spline/orbit/curves can be deferred (planning) (L)
- [ ] **#AT.2** Port `AreaTriggerFlag`, `AreaTriggerShapeType`, `AreaTriggerActionTypes`, `AreaTriggerActionUserTypes`, `AreaTriggerCreatePropertiesFlag` to `wow-constants` (L)
- [ ] **#AT.3** Port `AreaTriggerTemplate` + `AreaTriggerCreateProperties` + `AreaTriggerShapeInfo` to `wow-data` (M)
- [ ] **#AT.4** Define `AreaTrigger` entity struct in `wow-world/src/entities/area_trigger.rs` (M)
- [ ] **#AT.5** Implement `update_tick` + `update_target_list` + sphere/polygon containment checks (M)
- [ ] **#AT.6** Implement enter/exit action dispatch (`Cast` / `AddAura` / `Teleport`) (M)
- [ ] **#AT.7** `AreaTriggerData` UF block in `wow-packet/update.rs` + flip create bit (M)
- [ ] **#AT.8** Hook `Spell::effect_create_area_trigger` (depends on Spell migration) (M)
- [ ] **#AT.9** (Deferred) Spline/Orbit motion (H)
- [ ] **#AT.10** (Deferred) Scale/move curves (M)
- [ ] **#AT.11** AreaTriggerAI trait scaffold for scripts (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_AREATRIGGER.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 2073 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-spell` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_AREATRIGGER.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 2073 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_AREATRIGGER.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 2073 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_AREATRIGGER.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 2073 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-spell`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: sphere AT with radius R contains a point iff `dist² ≤ R²`
- [ ] Test: polygon containment with concave 6-vertex shape (winding test)
- [ ] Test: unit entering AT fires Enter action exactly once, leaving fires Exit once
- [ ] Test: `set_duration` past expiry triggers `remove` on next tick
- [ ] Test: `IsServerSide` ATs are not pushed to clients via UpdateObject

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 2073 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h` | `crates/wow-world/`, `crates/wow-data/`, `crates/wow-spell/`, `crates/wow-constants/` \| ⚠️ partial (legacy-table teleport triggers loaded; no entity, no shape eval, no spell-spawned ATs) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_AREATRIGGER.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 4 C++ files / 2073 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTriggerTemplate.h`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/AreaTrigger/AreaTrigger.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **WoLK 3.4 footprint is small.** The spell-spawned entity AT is a Cataclysm/MoP-era expansion of a 3.4 stub. Many fields (`AreaTriggerOrbitInfo`, scale curves, spline movement) exist in TC trunk but are **rarely used** in 3.4 spell data. Prioritize sphere + polygon + the three action types (Cast/AddAura/Teleport).
- Don't conflate this with `wow-data::AreaTriggerData` (legacy DBC table). The legacy module handles CMSG_AREA_TRIGGER coordinate triggers. Different lifecycle, different storage.
- `_insideUnits` diff is the source of truth for enter/exit. Mirror with `HashSet<ObjectGuid>` and recompute against fresh search each tick — don't try to incrementally maintain.
- Polygon vertices are stored in **local** offsets from `_stationaryPosition`. Rotate by `_stationaryPosition.GetOrientation()` each `UpdateShape()` if `HasDynamicShape`.
- `m_areaTriggerData->TimeToTarget` and `TimeToTargetScale` are **wire fields**: client interpolates between previous and current values. Don't try to be cute about partial updates.
- `Movement::Spline<int32>` uses int32 ms timestamps. WoLK ATs that move use TC's spline lib — port via `wow-math` once Movement migration lands.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class AreaTrigger : WorldObject` | `struct AreaTrigger` (composition) | no inheritance |
| `_insideUnits: GuidUnorderedSet` | `HashSet<ObjectGuid>` | recompute each tick |
| `_polygonVertices: Vec<Position>` | `Vec<Position>` | direct |
| `_spline: unique_ptr<Movement::Spline>` | `Option<Spline>` | `wow-math` once available |
| `Optional<AreaTriggerOrbitInfo>` | `Option<OrbitInfo>` | direct |
| `_areaTriggerTemplate: const*` | `&'static AreaTriggerTemplate` | from `wow-data` |
| `_ai: unique_ptr<AreaTriggerAI>` | `Option<Box<dyn AreaTriggerAi>>` | trait object |
| `enum class AreaTriggerShapeType` | `enum AreaTriggerShape { Sphere, Box, Polygon, Cylinder, Disk, BoundedPlane }` | direct |
| `std::variant<float, ScaleCurvePointsTemplate>` | `enum ScaleCurve { Const(f32), Points(...) }` | sum type |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class AreaTrigger` (entity) | no | — | ❌ missing |
| `struct AreaTriggerTemplate` | no | — | ❌ missing |
| `struct AreaTriggerCreateProperties` | no | — | ❌ missing |
| `enum AreaTriggerShapeType` | partial (`TriggerShape` for legacy) | `crates/wow-data/src/area_trigger.rs` | ⚠️ legacy-only |
| `AreaTriggerActionTypes` (Cast/AddAura/Teleport) | no | — | ❌ missing |
| `AreaTriggerOrbitInfo` | no | — | ❌ missing |
| `Movement::Spline` integration | no | — | ❌ missing |
| `_insideUnits` diff + enter/exit | no | — | ❌ missing |
| `UF::AreaTriggerData` UF block | no (bit hardcoded false) | `crates/wow-packet/src/packets/update.rs` | ❌ missing |
| Legacy DBC AT loader | yes | `crates/wow-data/src/area_trigger.rs` | ✅ unrelated module |
| `CMSG_AREA_TRIGGER` handler | yes | `crates/wow-world/src/handlers/misc.rs` | ✅ legacy path only |
| `TypeId::AreaTrigger = 11` | yes | `crates/wow-constants/src/object.rs` | ✅ present |
| `HighGuid::AreaTrigger = 13` | yes | `crates/wow-core/src/guid.rs` | ✅ present |
| AT-related opcodes (Denied/Message/RePath/etc.) | yes (constants) | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no senders |

**Verdict:** ⚠️ partial — legacy-table side is functional; **entity side is 0%**. Surface coverage of the 1453-line entity ≈ 0%.
