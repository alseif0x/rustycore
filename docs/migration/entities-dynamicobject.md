# Migration: Entities / DynamicObject

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-spell/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ❌ not started
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`DynamicObject` is the persistent area-effect entity spawned by area-of-effect spells (Blizzard, Consecration, Hurricane, Death and Decay, Rain of Fire). It owns a center position, a radius, a duration, an optional `Aura*` (which the dynobj keeps alive while it ticks), and a back-pointer to its caster. It is the world-visible footprint that periodic area auras attach to so clients render the spell on the ground. Also used for the `FARSIGHT_FOCUS` type — eye-of-the-beast style remote-camera anchor.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/DynamicObject/DynamicObject.cpp` | 322 | `prefix` |
| `game/Entities/DynamicObject/DynamicObject.h` | 95 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/DynamicObject/DynamicObject.h` | 95 | `DynamicObject` class def; final, inherits `WorldObject`, `GridObject<DynamicObject>`, `MapObject` |
| `src/server/game/Entities/DynamicObject/DynamicObject.cpp` | 322 | Create/Update/Remove, SetDuration/Delay, BindToCaster/Unbind, viewpoint plumbing |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `DynamicObject` | class (final) | Persistent area-spell footprint |
| `DynamicObjectType` | enum | `PORTAL=0` (unused), `AREA_SPELL=1`, `FARSIGHT_FOCUS=2` |
| `UF::DynamicObjectData` | UF struct | Wire data: `Caster`, `SpellID`, `Radius`, `CastTime`, etc. |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `CreateDynamicObject(guidlow, caster, spellInfo, pos, radius, type, visual)` | Build at world position, register with caster | `Map::AddToMap`, `BindToCaster` |
| `Update(uint32 p_time)` | Tick duration; remove if expired | `Remove` |
| `Remove()` | Despawn; unbind aura/caster | `RemoveAura`, `UnbindFromCaster`, `RemoveFromWorld` |
| `SetDuration(int32)` / `GetDuration()` / `Delay(int32)` | Lifetime mgmt for non-aura dynobjs (aura-driven ones use the aura's duration) | — |
| `SetAura(Aura*)` / `RemoveAura()` | Bind to a periodic area aura (the dynobj outlives the cast) | `Aura::*` |
| `SetCasterViewpoint()` / `RemoveCasterViewpoint()` | For `FARSIGHT_FOCUS` type — switch caster's active mover | `Player::SetViewpoint` |
| `BindToCaster()` / `UnbindFromCaster()` | Add to caster's `m_dynObj` list so removing the caster removes its dynobjs | `Unit::_RegisterDynObject` |
| `GetCaster()` / `GetCasterGUID()` / `GetSpellId()` / `GetRadius()` | Read-only accessors | — |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` / `GridObject` / `MapObject` (entity base + grid tracking)
- `Aura` (`SetAura`/`RemoveAura`; aura's removal must drop the dynobj)
- `Unit` (caster back-ref; `_RegisterDynObject` / `_UnregisterDynObject`)
- `SpellInfo` (radius, school, target filtering)
- `Map` (AddToMap/RemoveFromWorld)

**Depended on by:**
- `Spell::EffectPersistentAreaAura` (creates DynamicObject for periodic AoE)
- `Aura::AreaTrigger`-style ticking: `Aura::Update` walks units in `dynObj.GetRadius()`
- `Player::SetViewpoint` (FARSIGHT_FOCUS)
- `Unit::RemoveDynObject` (cleanup on death/zone)

---

## 6. SQL / DB queries

None directly. DynamicObjects are runtime-only.

DBC stores: `SpellRadius.dbc` is read indirectly via `SpellInfo` to determine `radius`.

---

## 7. Wire-protocol packets

DynamicObjects are pushed via the generic `SMSG_UPDATE_OBJECT` path (object type `TYPEID_DYNAMICOBJECT = 5`, high-guid `HighGuid::DynamicObject`). No dedicated opcodes — the client reads `DynamicObjectData` block (Caster, SpellID, Radius, CastTime, Type) from the update packet. Removal is just an UpdateObject "out-of-range" or destroy.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-spell` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-constants/src/object.rs` | `file` | 1 | 173 | `exists_active` | file exists |
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/object.rs` — `TypeId::DynamicObject = 9`, `HighGuid::DynamicObject = 12`
- `crates/wow-core/src/guid.rs` — GUID type recognition; `is_dynamic_object` predicate
- **0 lines** of `DynamicObject` entity logic.

**What's implemented:** type-id + GUID type only.

**What's missing vs C++:** entire 322-line `DynamicObject.cpp` — Create, Update, Remove, SetDuration, BindToCaster, aura linkage, viewpoint, the `DynamicObjectData` UF block in `update.rs` (currently the create/update bitmask just writes `false` for the dynobj block).

**Suspicious / likely divergent:** none — nothing exists.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_DYNAMICOBJECT.WBS.001** Cerrar la migracion auditada de `game/Entities/DynamicObject/DynamicObject.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`
  Rust target: `crates/wow-world`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_DYNAMICOBJECT.WBS.002** Cerrar la migracion auditada de `game/Entities/DynamicObject/DynamicObject.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h`
  Rust target: `crates/wow-world`, `crates/wow-spell`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#DYNOBJ.1** Port `DynamicObjectType` enum to `wow-constants` (L)
- [ ] **#DYNOBJ.2** Define `DynamicObject` struct in `wow-world/src/entities/dynamic_object.rs` with caster GUID, spell id, radius, duration, type, aura ref (L)
- [ ] **#DYNOBJ.3** Implement `create` / `update_tick` / `remove` lifecycle (M)
- [ ] **#DYNOBJ.4** Implement `set_aura` / `remove_aura` linkage (couples to Aura migration) (M)
- [ ] **#DYNOBJ.5** Implement `bind_to_caster` / `unbind_from_caster` registry on `Unit` (L)
- [ ] **#DYNOBJ.6** Add `DynamicObjectData` UF block to `wow-packet/src/packets/update.rs` and flip the create/update bits when present (M)
- [ ] **#DYNOBJ.7** Wire MapManager to track dynobjects per cell (so AoE search uses them) (M)
- [ ] **#DYNOBJ.8** `set_caster_viewpoint`/`remove_caster_viewpoint` for FARSIGHT_FOCUS (L)
- [ ] **#DYNOBJ.9** Cleanup hooks: caster despawn → drop owned dynobjs (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_DYNAMICOBJECT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 417 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-spell && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_DYNAMICOBJECT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 417 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_DYNAMICOBJECT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 417 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_DYNAMICOBJECT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 417 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h`. Rust target: `crates/wow-constants`, `crates/wow-spell`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `create` populates Caster/SpellID/Radius/Type matching inputs
- [ ] Test: `update_tick` past duration calls `remove`
- [ ] Test: `set_aura` then aura removal via `RemoveAura` despawns the dynobj
- [ ] Test: caster despawn unbinds and removes all bound dynobjs
- [ ] Test: `Delay(+N ms)` extends remaining duration

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 417 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h` | `crates/wow-world/`, `crates/wow-spell/`, `crates/wow-constants/` \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_DYNAMICOBJECT.DIV.001` | `crates/wow-spell` (`exists_empty`, 0 Rust lines) | 2 C++ files / 417 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/DynamicObject/DynamicObject.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Aura coupling is the trap.** A periodic AoE spell (e.g. Blizzard) creates a DynamicObject, then attaches an `Aura` to it. The Aura's `Update` ticks; the Aura's removal removes the dynobj. But for the FARSIGHT_FOCUS type there is **no** aura — `_duration` ticks down independently. Both paths must coexist.
- `m_dynamicObjectData->Caster` is the **caster GUID**, but `_caster` is a raw `Unit*` cached for speed. In Rust use only the GUID and resolve via MapManager — back-pointers cause cleanup bugs.
- WoLK 3.4 has no Polygon shape on DynamicObjects (that's AreaTrigger territory). Always a sphere of `radius`.
- DynamicObjects are *not* serialized to DB; on server crash they vanish. Don't add SQL.
- `CastTime` field in `UF::DynamicObjectData` is the world-time of creation, used by client for visual interpolation (e.g. Blizzard ramp-up). Don't omit it.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class DynamicObject : WorldObject` | `struct DynamicObject` (composition over WorldObject base) | no inheritance |
| `Aura* _aura` | `Option<AuraId>` (resolve via SpellAuras registry) | weak ref pattern |
| `Unit* _caster` | `ObjectGuid` only | resolve at use |
| `int32 _duration` | `Option<i32>` (None when aura-bound) | aura-bound dynobjs get duration from aura |
| `enum DynamicObjectType` | `enum DynamicObjectType { Portal, AreaSpell, FarsightFocus }` | direct |
| `BindToCaster`/`UnbindFromCaster` | `Unit::register_dyn_object(guid)` / `unregister` | maintains owned set |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class DynamicObject` | no | — | ❌ missing |
| `enum DynamicObjectType` | no | — | ❌ missing |
| `CreateDynamicObject` | no | — | ❌ missing |
| `DynamicObject::Update` | no | — | ❌ missing |
| `DynamicObject::SetAura` / `RemoveAura` | no | — | ❌ missing |
| `BindToCaster` / `UnbindFromCaster` | no | — | ❌ missing |
| `SetCasterViewpoint` (FARSIGHT) | no | — | ❌ missing |
| `UF::DynamicObjectData` block | no (bit hardcoded false) | `crates/wow-packet/src/packets/update.rs` | ❌ missing |
| `TypeId::DynamicObject = 9` | yes | `crates/wow-constants/src/object.rs` | ✅ present |
| `HighGuid::DynamicObject = 12` | yes | `crates/wow-core/src/guid.rs` | ✅ present |

**Verdict:** ❌ not started. Surface coverage ≈ 1% (constants + GUID type). No entity, no lifecycle, no aura binding.
