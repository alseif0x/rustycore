# Migration: Entities / SceneObject

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/`
> **Rust target crate(s):** `crates/wow-world/`, `crates/wow-data/`, `crates/wow-constants/`
> **Layer:** L4 (sub-modules)
> **Status:** ❌ not started — **n/a for WoLK 3.4** (post-WoLK feature, like Conversation)
> **Audited vs C++:** ⚠️ partial (header-level audit only)
> **Last updated:** 2026-05-01

---

## 1. Purpose

`SceneObject` is the post-WoLK entity that anchors **scripted scene packages** (`SCENESCRIPT_PACKAGE_ID`, `SceneScript.db2`) — the engine's mechanism for in-world cinematics, scripted minigames, and the Pet Battle UI surface. The scene id resolves a client-side script package which drives camera, animation, and UI. Two subtypes exist: `SceneType::Normal` (cinematic / scripted scene) and `SceneType::PetBattle` (pet battle 6v6 surface; introduced in MoP). **Like `Conversation`, this entity is dead infrastructure on the WotLK 3.4.3 retail client** — TYPEID is reserved but the client cannot render scenes nor pet battles.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Entities/SceneObject/SceneObject.cpp` | 207 | `prefix` |
| `game/Entities/SceneObject/SceneObject.h` | 89 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Entities/SceneObject/SceneObject.h` | 89 | `SceneObject` class (final, WorldObject + GridObject) |
| `src/server/game/Entities/SceneObject/SceneObject.cpp` | 207 | Create, Update, Remove, ShouldBeRemoved, factory `CreateSceneObject` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SceneObject` | class (final) | Scene/cinematic anchor entity |
| `SceneType` | enum class | `Normal=0`, `PetBattle=1` |
| `SceneTemplate` | struct (forward decl, defined in `SceneMgr` / `ObjectMgr`) | Per-scene-id template (script package id, flags) |
| `UF::SceneObjectData` | UF struct | Wire data: `ScriptPackageID`, `RndSeedVal`, `CreatedBy`, `SceneType` |

---

## 4. Critical public methods

| Symbol | Purpose | Calls into |
|---|---|---|
| `CreateSceneObject(sceneId, creator, pos, privateOwner)` | Static factory; resolves template, builds entity, adds to map | `Create`, `Map::AddToMap` |
| `Create(lowGuid, type, sceneId, scriptPackageId, map, creator, pos, privateOwner)` | Internal builder (sets UF fields) | UF mutators |
| `Update(uint32 diff)` | Tick: check `ShouldBeRemoved`, then base `WorldObject::Update` | `Remove` |
| `Remove()` | Despawn | `RemoveFromWorld` |
| `ShouldBeRemoved()` (private) | Lifetime predicate (creator gone, scene dismissed, etc.) | — |
| `SetCreatedBySpellCast(ObjectGuid castId)` | Tag the spell cast that birthed this scene | `_createdBySpellCast` |
| `GetCreatorGUID()` / `GetOwnerGUID()` | Resolve via UF `CreatedBy` | — |

---

## 5. Module dependencies

**Depends on:**
- `WorldObject` / `GridObject` (entity base)
- `SceneTemplate` (`scene_template` DB table — post-WoLK schema)
- `SceneScript.db2` (post-WoLK DB2; not present in 3.4 client)
- `Map` (AddToMap)
- Pet battle subsystem (only for `SceneType::PetBattle`)

**Depended on by:**
- `Spell::EffectSceneObject` (post-WoLK spell effect)
- `SceneMgr` on Player (manages active scenes per player)
- Pet battle initiation flow

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `scene_template` | Per-sceneId metadata (script package id, flags, type) | world (post-WoLK) |

DBC stores (post-WoLK):

| Store | What it loads | Read by |
|---|---|---|
| `SceneScriptStore` / `SceneScriptPackageStore` | `SceneScript.db2`, `SceneScriptPackage.db2` | template lookup |

**None of these tables/DB2 files exist in the 3.4.3 client.**

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_UPDATE_OBJECT` (with SceneObject block) | S → C | spawn (post-WoLK clients) |
| `SMSG_SCENE_OBJECT_EVENT` | S → C | scene script event broadcast | (`crates/wow-constants/src/opcodes.rs`: `SceneObjectEvent = 0x25e2`) |
| `SMSG_SCENE_OBJECT_PET_BATTLE_INITIAL_UPDATE` / `_FIRST_ROUND` / `_ROUND_RESULT` / `_REPLACEMENTS_MADE` / `_FINAL_ROUND` / `_FINISHED` | S → C | pet battle state machine | (`SceneObjectPetBattle*` constants present) |

**Caveat:** these opcodes do not exist in the 3.4.3.54261 retail client. Constants are reserved for parity only.

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
| `crates/wow-core/src/guid.rs` | `file` | 1 | 790 | `exists_active` | file exists |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/update.rs` | `file` | 1 | 3072 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/object.rs` — `TypeId::SceneObject = 12`, `HighGuid::SceneObject = 16`
- `crates/wow-core/src/guid.rs` — `HighGuid::SceneObject`, `is_scene_object`, type mapping
- `crates/wow-constants/src/opcodes.rs` — `SceneObjectEvent`, `SceneObjectPetBattleInitialUpdate/FirstRound/RoundResult/ReplacementsMade/FinalRound/Finished` enumerated (~7 opcodes)
- `crates/wow-packet/src/packets/update.rs` — SceneObject block bit hardcoded `false`
- **0 lines** of SceneObject entity logic.

**What's implemented:** type-id, GUID type, opcode constants. Update bit reserved to `false`.

**What's missing vs C++:** entire 207-line `SceneObject.cpp`. Not a priority — see status note.

**Suspicious / likely divergent:** none — feature does not exist on the target client.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ENTITIES_SCENEOBJECT.WBS.001** Cerrar la migracion auditada de `game/Entities/SceneObject/SceneObject.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ENTITIES_SCENEOBJECT.WBS.002** Cerrar la migracion auditada de `game/Entities/SceneObject/SceneObject.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h`
  Rust target: `crates/wow-world`, `crates/wow-data`, `crates/wow-constants`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#SCENE.1** **Decision gate:** confirm whether 3.4 backport content needs SceneObject (almost certainly no). If no — close as `n/a`. (L)
- [ ] **#SCENE.2** Port `SceneType` enum to `wow-constants` (L)
- [ ] **#SCENE.3** Define `SceneObject` entity struct (`script_package_id`, `created_by`, `created_by_spell_cast`, `private_owner`) (L)
- [ ] **#SCENE.4** Implement `create` factory (M)
- [ ] **#SCENE.5** Implement `update_tick` + `should_be_removed` lifetime predicate (L)
- [ ] **#SCENE.6** `SceneObjectData` UF block + flip create bit (M)
- [ ] **#SCENE.7** `scene_template` DB schema + loader (M, post-WoLK schema)
- [ ] **#SCENE.8** Pet battle subsystem (huge — `SceneType::PetBattle` requires the entire pet-battle state machine; treat as separate epic, defer)

**Recommendation:** mark this module **`n/a`** in the master roadmap. Pet battle is post-WoLK; cinematic scenes are post-WoLK.

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ENTITIES_SCENEOBJECT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 296 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | `cargo test -p wow-constants && cargo test -p wow-data && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ENTITIES_SCENEOBJECT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 296 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ENTITIES_SCENEOBJECT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 296 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ENTITIES_SCENEOBJECT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 296 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h`. Rust target: `crates/wow-constants`, `crates/wow-data`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

(Only relevant if the decision gate in #SCENE.1 says yes.)

- [ ] Test: `create_scene_object` populates ScriptPackageID matching `scene_template`
- [ ] Test: `should_be_removed` true when creator GUID is gone
- [ ] Test: `set_created_by_spell_cast` populates the cast id field
- [ ] Test: private-owner GUID restricts visibility (only that player sees the entity)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 2 files / 296 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h` | `crates/wow-world/`, `crates/wow-data/`, `crates/wow-constants/` \| ❌ not started — **n/a for WoLK 3.4** (post-WoLK feature, like Conversation) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ENTITIES_SCENEOBJECT.DIV.001` | _none generated_ | 2 C++ files / 296 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Entities/SceneObject/SceneObject.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **WoLK 3.4.3 retail client cannot render scenes or pet battles.** All structural code is dead infrastructure. Spawning a SceneObject on a real 3.4 client will at best be ignored.
- **Pet battles are explicitly out of scope** for a 3.4 server even with full SceneObject support — they require their own DB tables (`battle_pet_*`), DB2 files, and a 6v6 turn-based combat engine. Do not start that work without explicit user direction.
- Private-object-owner pattern is shared with `Conversation`. If both end up being implemented, factor a `PrivateObjectFilter` into MapManager visibility checks.
- `_createdBySpellCast` is the **cast id** (a `Cast` instance GUID), not the spell id — do not conflate.
- `RndSeedVal` (UF field) is a per-entity random seed used to deterministically vary scene playback across clients; if implemented, generate at create time and never mutate.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class SceneObject : WorldObject` | `struct SceneObject` (composition) | if implemented |
| `enum class SceneType` | `enum SceneType { Normal, PetBattle }` | direct |
| `Position _stationaryPosition` | `Position` field | from `wow-core` |
| `ObjectGuid _createdBySpellCast` | `ObjectGuid` | direct |
| `m_sceneObjectData->CreatedBy` (UF ref) | `ObjectGuid` field | flatten UF into struct |
| `static SceneObject* CreateSceneObject(...)` | `fn create_scene_object(...) -> Self` | factory |

---

## 13. §13 Audit (vs `/home/server/woltk-trinity-legacy/`)

| C++ symbol | Found in Rust | File | Verdict |
|---|---|---|---|
| `class SceneObject` | no | — | ❌ missing (and likely n/a) |
| `enum class SceneType` | no | — | ❌ missing |
| `SceneObject::CreateSceneObject` | no | — | ❌ missing |
| `SceneObject::Update` / `Remove` / `ShouldBeRemoved` | no | — | ❌ missing |
| `SetCreatedBySpellCast` | no | — | ❌ missing |
| `UF::SceneObjectData` UF block | no (bit hardcoded false) | `crates/wow-packet/src/packets/update.rs` | ❌ missing |
| `TypeId::SceneObject = 12` | yes | `crates/wow-constants/src/object.rs` | ✅ present (constant only) |
| `HighGuid::SceneObject = 16` | yes | `crates/wow-core/src/guid.rs` | ✅ present (constant only) |
| `SMSG_SCENE_OBJECT_EVENT` opcode | yes (constant) | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, no sender (and client likely won't render) |
| `SMSG_SCENE_OBJECT_PET_BATTLE_*` opcodes (×6) | yes (constants) | `crates/wow-constants/src/opcodes.rs` | ⚠️ enumerated, pet battle out of scope |

**Verdict:** ❌ not started — **and recommended to stay that way for the 3.4 fork.** Surface coverage ≈ 0% (constants reserved). Mark `n/a` in master roadmap pending explicit user requirement (which will almost certainly not arrive for 3.4).
