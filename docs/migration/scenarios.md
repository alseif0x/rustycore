# Migration: Scenarios

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Scenarios/`
> **Rust target crate(s):** *N/A para WoLK 3.4.x* — placeholder en `crates/wow-world/src/scenarios/` (skeleton only)
> **Layer:** L7
> **Status:** ❌ not started — **N/A para 3.4.x classic**
> **Audited vs C++:** ✅ n/a confirmed (2026-05-01) — post-WoLK MoP feature; zero impact on 3.4.3
> **Last updated:** 2026-05-01

---

## 1. Purpose

Scenarios son *3-player adventures* introducidos en **Mists of Pandaria (5.0, 2012)**. Son instancias mini-historia para compositions arbitrarios (sin requerir tank/healer/dps), con sistema propio de criteria-tree y POI. **No aplican a Wrath of the Lich King 3.4.x** (Blizzard no los retroportó al cliente clásico). El módulo existe en este fork como código *upstream-merged* pero **muerto**: ninguna `lfg_dungeon_template` con `type==SCENARIO`, ninguna entrada en `LFGDungeons.dbc` 3.4.x, ningún cliente WoLK envía `CMSG_SCENARIO_*`.

> **Conclusión:** documentar como esqueleto stub-only. NO se debe implementar para 3.4.3. Se conserva la doc para que la próxima sesión no investigue desde cero.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Scenarios/InstanceScenario.cpp` | 112 | `prefix` |
| `game/Scenarios/InstanceScenario.h` | 37 | `prefix` |
| `game/Scenarios/Scenario.cpp` | 362 | `prefix` |
| `game/Scenarios/Scenario.h` | 111 | `prefix` |
| `game/Scenarios/ScenarioMgr.cpp` | 224 | `prefix` |
| `game/Scenarios/ScenarioMgr.h` | 127 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Scenarios/Scenario.h` | 111 | `Scenario : CriteriaHandler`, `ScenarioStepState` enum |
| `src/server/game/Scenarios/Scenario.cpp` | 362 | Implementación: step transitions, criteria updates, packet builders |
| `src/server/game/Scenarios/InstanceScenario.h` | 37 | `InstanceScenario : Scenario` (variante para `InstanceMap`) |
| `src/server/game/Scenarios/InstanceScenario.cpp` | 112 | DB persistence de criteria progress |
| `src/server/game/Scenarios/ScenarioMgr.h` | 127 | Singleton, `ScenarioData`, `ScenarioDBData`, `ScenarioPOI`, enums |
| `src/server/game/Scenarios/ScenarioMgr.cpp` | 224 | `LoadDB2Data`, `LoadDBData`, `LoadScenarioPOI`, `CreateInstanceScenario` |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Scenario` | class (extends `CriteriaHandler`) | Base scenario instance, tracks current step + step states |
| `InstanceScenario` | class (extends `Scenario`) | Scenario asociado a un `InstanceMap` (con DB save) |
| `ScenarioMgr` | singleton | Registry de `ScenarioData`, `ScenarioPOI`, mapping (mapId, difficulty) → scenario A/H |
| `ScenarioStepState` | enum | INVALID=0, NOT_STARTED=1, IN_PROGRESS=2, DONE=3 |
| `ScenarioType` | enum | SCENARIO=0, CHALLENGE_MODE=1, SOLO=2, DUNGEON=10 |
| `ScenarioData` | struct | `Entry` (DB2 row) + `Steps: map<uint8, ScenarioStepEntry*>` |
| `ScenarioDBData` | struct | `MapID`, `DifficultyID`, `Scenario_A`, `Scenario_H` (per-faction routing) |
| `ScenarioPOI` | struct | `BlobIndex`, `MapID`, `UiMapID`, `Priority`, `Flags`, `WorldEffectID`, `PlayerConditionID`, `NavigationPlayerConditionID`, `Points: vec<ScenarioPOIPoint>` |
| `ScenarioPOIPoint` | struct | `X`, `Y`, `Z` (int32) |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ScenarioMgr::Instance()` | Singleton accessor | — |
| `ScenarioMgr::CreateInstanceScenario(InstanceMap*, TeamId)` | Lookup `_scenarioDBData[(mapId, diff)]`, devuelve scenario A o H según team | `_scenarioData` |
| `ScenarioMgr::LoadDBData()` | Lee tabla `scenarios` (mapId, difficulty, scenario_A, scenario_H) | `WorldDatabase` |
| `ScenarioMgr::LoadDB2Data()` | Carga `Scenario.db2` + `ScenarioStep.db2` | `sScenarioStore`, `sScenarioStepStore` |
| `ScenarioMgr::LoadScenarioPOI()` | Carga `scenario_poi` + `scenario_poi_points` | `WorldDatabase` |
| `ScenarioMgr::GetScenarioPOIs(criteriaTreeID)` | Lookup POIs por criteria-tree | — |
| `Scenario::Reset()` | Resetea step a `GetFirstStep`, limpia step states | `SetStep`, `Reset` (CriteriaHandler) |
| `Scenario::SetStep(ScenarioStepEntry const*)` | Activa step, marca anteriores como DONE, broadcast `SMSG_SCENARIO_STATE` | `SendScenarioState` |
| `Scenario::CompleteStep(step)` | Step → DONE, advance to next o `CompleteScenario` | `SetStep`, `CompleteScenario` |
| `Scenario::CompleteScenario()` | Final step DONE → broadcast completion, hooks reward | — |
| `Scenario::OnPlayerEnter(Player*)` | Insert en `_players`, send state | `SendScenarioState` |
| `Scenario::OnPlayerExit(Player*)` | Remove de `_players` | — |
| `Scenario::IsComplete()` | Todos los steps en DONE | — |
| `Scenario::GetStepState(step)` | Lookup `_stepStates[step]` | — |
| `Scenario::SendScenarioState(Player*)` | `SMSG_SCENARIO_STATE` con criteria progress + bonus objectives | `BuildScenarioStateFor` |
| `Scenario::SendBootPlayer(Player*)` | `SMSG_SCENARIO_BOOT_PLAYER` | — |

---

## 5. Module dependencies

**Depends on:**
- `CriteriaHandler` (achievements/criteria infra)
- `Map` / `InstanceMap`
- DB2: `sScenarioStore`, `sScenarioStepStore`, `sCriteriaStore`
- WorldDatabase: `scenarios`, `scenario_poi`, `scenario_poi_points`
- `WorldPackets::Scenario::*` (ScenarioState, BootPlayer, POIs)
- `Player` — para criteria progress per-player

**Depended on by:**
- `InstanceMap::SetInstanceScenario` — instance binding
- `Player::SendInitialPacketsBeforeAddToMap` — manda scenario state al entrar
- `LFGMgr` (en clientes MoP+) — `LFG_TYPE_RANDOM` con scenario flag

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT mapId, difficulty, scenario_A, scenario_H FROM scenarios` | Routing per faction | world |
| `SELECT * FROM scenario_poi` | POI metadata | world |
| `SELECT * FROM scenario_poi_points` | POI vertices | world |

**DB2 stores leídos:**

| Store | What it loads | Read by |
|---|---|---|
| `sScenarioStore` | Scenario.db2 | `ScenarioMgr::LoadDB2Data` |
| `sScenarioStepStore` | ScenarioStep.db2 | `ScenarioMgr::LoadDB2Data` |

> **En 3.4.x estos DB2 NO existen en el cliente clásico**, las tablas SQL no se llenan, y el loader sale temprano con count=0.

---

## 7. Wire-protocol packets

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_SCENARIO_STATE` | server → client | `Scenario::SendScenarioState` (MoP+ only) |
| `SMSG_SCENARIO_PROGRESS_UPDATE` | server → client | `SendCriteriaUpdate` |
| `SMSG_SCENARIO_COMPLETED` | server → client | `CompleteScenario` |
| `SMSG_SCENARIO_BOOT_PLAYER` | server → client | `Scenario::SendBootPlayer` |
| `SMSG_SCENARIO_POIS` | server → client | Response a request POI |
| `CMSG_QUERY_SCENARIO_POI` | client → server | `HandleQueryScenarioPOI` |

> **Cliente WoLK 3.4.3 NO envía `CMSG_QUERY_SCENARIO_POI` ni decodifica los SMSG_SCENARIO_*.** Sus opcode IDs probablemente conflictúan con otros opcodes WoLK.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/scenarios` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *NINGUNO*. No existe ni stub ni placeholder.

**What's implemented:**
- Nada.

**What's missing vs C++:**
- Todo. **Pero no se debe implementar para 3.4.x.** El esfuerzo de portar este módulo (~970 líneas C++ + DB2 stores + criteria infra) no genera beneficio funcional para un cliente WoLK clásico.

**Suspicious / likely divergent:**
- N/A — el cliente nunca dispara este código.

**Tests existing:**
- 0 tests. No se requieren.

---

## 9. Migration sub-tasks (ESQUELETO ONLY)

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SCENARIOS.WBS.001** Cerrar la migracion auditada de `game/Scenarios/InstanceScenario.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/InstanceScenario.cpp`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCENARIOS.WBS.002** Cerrar la migracion auditada de `game/Scenarios/InstanceScenario.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/InstanceScenario.h`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCENARIOS.WBS.003** Cerrar la migracion auditada de `game/Scenarios/Scenario.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCENARIOS.WBS.004** Cerrar la migracion auditada de `game/Scenarios/Scenario.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.h`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCENARIOS.WBS.005** Cerrar la migracion auditada de `game/Scenarios/ScenarioMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SCENARIOS.WBS.006** Cerrar la migracion auditada de `game/Scenarios/ScenarioMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h`
  Rust target: `crates/wow-world/src/scenarios`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Complejidad: **L** (<1h), **M** (1-4h), **H** (4-12h).

> **Política recomendada:** ejecutar #SCN.1–#SCN.3 sólo cuando el equipo decida soportar un cliente MoP+ (5.x). Para 3.4.x, marcar el módulo como **WONT-IMPLEMENT** y dejar el directorio inexistente.

- [ ] **#SCN.1** Crear placeholder `crates/wow-world/src/scenarios/mod.rs` con un único item `pub const SCENARIOS_NOT_SUPPORTED_IN_3_4_3: bool = true;` y un comentario explicando por qué (L)
- [ ] **#SCN.2** Documentar en `crates/wow-world/src/lfg/mod.rs` que `LfgQueueType::Scenario=3` está **disabled** (L)
- [ ] **#SCN.3** Asegurar que los handlers de opcodes `CMSG_QUERY_SCENARIO_POI` (si llegan a llegar) responden con paquete vacío sin panic (L)
- [ ] **#SCN.4** *(futuro, sólo si se soporta cliente MoP+)* Definir `Scenario`, `InstanceScenario`, `ScenarioMgr`, todos los enums (`ScenarioStepState`, `ScenarioType`) y structs (M)
- [ ] **#SCN.5** *(futuro)* Loader `load_db_data()` desde tabla `scenarios` (M)
- [ ] **#SCN.6** *(futuro)* Loader `load_db2_data()` desde `Scenario.db2` + `ScenarioStep.db2` — **requiere cliente MoP+** (M)
- [ ] **#SCN.7** *(futuro)* Loader `load_scenario_poi()` desde `scenario_poi` + `scenario_poi_points` (M)
- [ ] **#SCN.8** *(futuro)* Implementar `Scenario::set_step`, `complete_step`, `complete_scenario` con state-machine (H)
- [ ] **#SCN.9** *(futuro)* Integración con `CriteriaHandler` para tracking de progreso por jugador (H)
- [ ] **#SCN.10** *(futuro)* Builders SMSG_SCENARIO_* y handler `handle_query_scenario_poi` (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SCENARIOS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 973 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SCENARIOS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 973 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SCENARIOS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 973 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SCENARIOS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 973 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: en startup con cliente 3.4.x, `ScenarioMgr::LoadDB2Data()` no loaded (count=0) sin error de log spam
- [ ] Test: `CMSG_QUERY_SCENARIO_POI` (si el cliente alguna vez lo envía) responde con vacío sin panic
- [ ] Test: `LfgQueueType::Scenario` no está expuesto en `system_info` enviado al cliente
- [ ] *(futuro)* Test: round-trip `Scenario.db2` parsing
- [ ] *(futuro)* Test: state machine NOT_STARTED → IN_PROGRESS → DONE → next step

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `product_out_of_scope_or_post_wolk` | Keep C++ coverage assigned; implementation may be stubbed/disabled only with explicit client behavior and tests/n/a recorded. | 6 files / 973 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h` | *N/A para WoLK 3.4.x* — placeholder en `crates/wow-world/src/scenarios/` (skeleton only) \| ❌ not started — **N/A para 3.4.x classic** |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SCENARIOS.DIV.001` | `crates/wow-world/src/scenarios` (`missing_declared_path`, 0 Rust lines) | 6 C++ files / 973 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/Scenario.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Scenarios/ScenarioMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Decisión arquitectónica:** Scenarios son **post-WoLK**. El cliente 3.4.3 no tiene UI para ellos, no envía opcodes, no parsea SMSG. Implementarlos sería trabajo muerto.
- TrinityCore master rama mainline tiene Scenarios completos para Shadowlands+. Este fork legacy heredó el código pero NUNCA lo activó (no hay registros en tablas SQL ni DBC del cliente).
- `enum LfgQueueType { ..., LFG_QUEUE_SCENARIO=3 }` existe en `LFG.h` pero ningún path de matchmaking lo usa para 3.4.x.
- Si en el futuro alguien quiere portar el server a un cliente MoP+ (5.4.8 por ejemplo), este módulo **será obligatorio** y la implementación es no-trivial (~3 días-persona): requiere `CriteriaHandler` portado, DB2 loaders, packet builders, instance binding, y POI rendering.
- `InstanceScenario` interactúa con `InstanceMap::SetInstanceScenario` — si en el futuro se implementa, hay que coordinar con `instance.md`.
- POI system es independiente del scenario en sí: los POIs son metadata para minimap/world map; en 3.4.x el equivalente es `quest_poi` (que **sí** se usa).
- **NO confundir Scenarios con LFR o con Challenge Mode dungeons** — son sistemas separados:
  - `LFG_TYPE_RANDOM` + `flag SEASONAL`: WoLK Random Heroic Daily ✓ activo
  - `LFG_QUEUE_LFR=2`: 25-man Looking For Raid ✗ MoP+
  - `LFG_QUEUE_SCENARIO=3`: 3-man Scenarios ✗ MoP+
  - `SCENARIO_TYPE_CHALLENGE_MODE=1`: Challenge Mode dungeons ✗ MoP+
- Recomendación: este doc se mantiene como **referencia histórica + esqueleto**. No abrir tarea de implementación hasta que haya un caso de uso confirmado.

---

## 12. C++ → Rust mapping (high-level, FUTURE-USE only)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Scenario : CriteriaHandler` | `struct Scenario` con campo `criteria_handler: CriteriaHandler` | Composition, no inheritance |
| `class InstanceScenario : Scenario` | `struct InstanceScenario { base: Scenario, instance_id: u32 }` | — |
| `class ScenarioMgr` (singleton) | `static SCENARIO_MGR: OnceCell<ScenarioMgr>` | — |
| `enum ScenarioStepState` | `#[repr(u8)] enum ScenarioStepState { Invalid=0, NotStarted=1, InProgress=2, Done=3 }` | — |
| `enum ScenarioType` | `#[repr(u8)] enum ScenarioType { Scenario=0, ChallengeMode=1, Solo=2, Dungeon=10 }` | — |
| `ScenarioData { Entry, Steps: map<u8, ScenarioStepEntry*> }` | `struct ScenarioData { entry: ScenarioEntry, steps: BTreeMap<u8, ScenarioStepEntry> }` | — |
| `ScenarioPOI` con `vector<ScenarioPOIPoint>` | `struct ScenarioPOI { ..., points: Vec<ScenarioPOIPoint> }` | — |
| `unordered_map<pair<uint32, uint8>, ScenarioDBData>` | `HashMap<(u32, u8), ScenarioDBData>` | — |
| `void OnPlayerEnter(Player*)` | `async fn on_player_enter(&self, player: &Player)` | — |
| `bool IsComplete() const` | `pub fn is_complete(&self) -> bool` | — |

---

## 13. Audit (2026-05-01)

**Status confirmed: ✅ n/a for WoLK 3.4.3 — zero impact.**

Scenarios are a **Mists of Pandaria (5.0, Sep 2012)** feature: 3-player adventure instances with their own criteria-tree state machine and POI metadata, entirely post-WoLK. The WotLK 3.4.3.54261 client ships **no `Scenario.db2` / `ScenarioStep.db2`** data files, has no UI for scenario state or POI rendering, never sends `CMSG_QUERY_SCENARIO_POI`, and never decodes `SMSG_SCENARIO_STATE` / `SMSG_SCENARIO_PROGRESS_UPDATE` / `SMSG_SCENARIO_COMPLETED` / `SMSG_SCENARIO_BOOT_PLAYER` / `SMSG_SCENARIO_POIS`. The 3.4.x SQL schema has no `scenarios` / `scenario_poi` / `scenario_poi_points` tables populated, and no `lfg_dungeon_template` row has `type==SCENARIO`. Verified zero presence in RustyCore: no `crates/wow-world/src/scenarios/`, no `Scenario` / `InstanceScenario` / `ScenarioMgr` symbols, no `LFG_QUEUE_SCENARIO` matchmaking path. The C++ module survives in `woltk-trinity-legacy` only as upstream-merged dead code (loaders return count=0 against an empty 3.4.x dataset).

**Residual cleanup:** none required for 3.4.3. Sub-tasks #SCN.1–#SCN.3 (placeholder `SCENARIOS_NOT_SUPPORTED_IN_3_4_3` const, LFG queue-type comment, defensive empty response if `CMSG_QUERY_SCENARIO_POI` ever arrives) are nice-to-have hardening but not blocking; #SCN.4–#SCN.10 are gated on a hypothetical future MoP+ client port and should remain out-of-scope. This doc stays as a historical reference and skeleton — do not open implementation work.

---

*Template version: 1.0 (2026-05-01).* **Status:** documento de referencia; implementación N/A para WoLK 3.4.x.
