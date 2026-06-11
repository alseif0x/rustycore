# Migration: OutdoorPvP

> **C++ canonical path:** `src/server/game/OutdoorPvP/` + `src/server/scripts/OutdoorPvP/`
> **Rust target crate(s):** `crates/wow-pvp/` (nuevo módulo `outdoor/`), wiring en `crates/wow-world/`
> **Layer:** L7
> **Status:** ❌ not started
> **Audited vs C++:** ✅ audited 2026-05-01 (❌ confirmed — 5 zones in this fork)
> **Last updated:** 2026-05-01

---

## 1. Purpose

OutdoorPvP gestiona los *world PvP zones* clásicos pre-Wintergrasp: zonas con objetivos capturables (torres, bases, flags) que disparan buffs faccionales y world-state UI. En WoLK 3.4.x el manager carga *templates* desde `outdoorpvp_template`, instancia un `OutdoorPvP` derivado por mapa/zona via `ScriptMgr::CreateOutdoorPvP`, y delega ticks de objetivos al `OPvPCapturePoint` correspondiente. Los handlers de control-zone (`OutdoorPvPControlZoneHandler`) procesan eventos `GAMEOBJECT_TYPE_CONTROL_ZONE` (capture/contested/neutral/progress).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/OutdoorPvP/OutdoorPvP.cpp` | 255 | `prefix` |
| `game/OutdoorPvP/OutdoorPvP.h` | 232 | `prefix` |
| `game/OutdoorPvP/OutdoorPvPMgr.cpp` | 224 | `prefix` |
| `game/OutdoorPvP/OutdoorPvPMgr.h` | 112 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/OutdoorPvP/OutdoorPvP.h` | 232 | `OutdoorPvP`, `OPvPCapturePoint`, `OutdoorPvPControlZoneHandler`, enums (`OutdoorPvPTypes`, `ObjectiveStates`) |
| `src/server/game/OutdoorPvP/OutdoorPvP.cpp` | 255 | Lógica base (player enter/leave, broadcast, team buff, control-zone event dispatch) |
| `src/server/game/OutdoorPvP/OutdoorPvPMgr.h` | 112 | Singleton manager, mapa Map*+zoneId → OutdoorPvP* |
| `src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp` | 224 | `InitOutdoorPvP` (carga `outdoorpvp_template`), `CreateOutdoorPvPForMap`, `Update(diff)`, hook DefenseMessage |
| `src/server/scripts/OutdoorPvP/OutdoorPvPHP.{cpp,h}` | ~600 | Hellfire Peninsula (3 torres: Broken Hill, Overlook, Stadium; buffs 32071/32049) |
| `src/server/scripts/OutdoorPvP/OutdoorPvPNA.{cpp,h}` | ~700 | Nagrand / Halaa (control point + guards; buff `NA_CAPTURE_BUFF=33795` *Strength of the Halaani*) |
| `src/server/scripts/OutdoorPvP/OutdoorPvPSI.{cpp,h}` | ~250 | Silithus / silithyst flag-carrier; buff `SI_CENARION_FAVOR=30754` |
| `src/server/scripts/OutdoorPvP/OutdoorPvPTF.{cpp,h}` | ~500 | Terokkar Forest (5 spirit towers, *Blessing of Auchindoun*) |
| `src/server/scripts/OutdoorPvP/OutdoorPvPZM.{cpp,h}` | ~700 | Zangarmarsh (3 beacons + graveyard; buff `ZM_CAPTURE_BUFF=33779` *Twin Spire Blessing*) |
| `src/server/scripts/OutdoorPvP/OutdoorPvPScriptLoader.cpp` | ~30 | Registro de las 5 zonas via `sScriptMgr->RegisterOutdoorPvPScript` |

> **Eastern Plaguelands (EP):** *no existe* en este fork legacy. Sólo HP/NA/SI/TF/ZM están registrados (ver `enum OutdoorPvPTypes`: HP=1, NA=2, TF=3, ZM=4, SI=5).
> **Wintergrasp (WG):** *no está en OutdoorPvP/*, vive en `src/server/game/Battlefield/` + `src/server/scripts/Northrend/zone_wintergrasp.cpp`. Documentar en `battlefield.md`.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `OutdoorPvP` | class (extends `ZoneScript`) | Base por-zona/mapa. Mantiene `m_capturePoints`, `m_players[2]`, `ControlZoneHandlers` |
| `OPvPCapturePoint` | class (abstract) | Objetivo individual; tiene `m_team`, `m_State`, `m_OldState`. Métodos virtuales `Update`, `ChangeState`, `HandleCustomSpell`, `HandleOpenGo`, `HandleDropFlag` |
| `OutdoorPvPControlZoneHandler` | class | Wrapper para procesar eventos GO `CONTROL_ZONE`. Override `HandleProgress/Neutral/Capture/Contested EventAlliance/Horde` |
| `OutdoorPvPMgr` | singleton | Registry: `m_OutdoorPvPByMap`, `m_OutdoorPvPMap[(Map*, zoneId)]`, ticking 1Hz |
| `OutdoorPvPTypes` | enum | `HP=1, NA=2, TF=3, ZM=4, SI=5, MAX_OUTDOORPVP_TYPES=6` |
| `ObjectiveStates` | enum | NEUTRAL / ALLIANCE / HORDE / *_CHALLENGE variantes (7 estados) |
| `go_type`, `creature_type` | struct | POD para spawn data hardcoded de los scripts de zona |
| `OutdoorPvPHP/NA/SI/TF/ZM` | class | Implementaciones concretas (zone-specific tower counters, world-states, eventos GO) |
| `HP/NA/TF/ZM ControlZoneHandler` | class | Per-tower handlers con flagGuid, artkit, kill-credit, worldstate ids |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `OutdoorPvPMgr::InitOutdoorPvP()` | Carga `outdoorpvp_template`, mapea typeId → ScriptId | `WorldDatabase`, `sObjectMgr->GetScriptId` |
| `OutdoorPvPMgr::CreateOutdoorPvPForMap(Map*)` | Por cada typeId con `m_OutdoorMapIds[i]==map->GetId()`, llama `sScriptMgr->CreateOutdoorPvP(...)` y `SetupOutdoorPvP()` | `ScriptMgr` |
| `OutdoorPvPMgr::Update(uint32 diff)` | Acumula diff; cada `OUTDOORPVP_OBJECTIVE_UPDATE_INTERVAL=1000ms` llama `pvp->Update(timer)` | — |
| `OutdoorPvPMgr::HandlePlayerEnterZone/LeaveZone` | Lookup `(player->GetMap(), zoneId)`, delega al `OutdoorPvP` correspondiente | `OutdoorPvP::HandlePlayer*Zone` |
| `OutdoorPvPMgr::HandleCustomSpell/OpenGo/DropFlag` | Routea via `player->GetOutdoorPvP()` | — |
| `OutdoorPvPMgr::GetDefenseMessage(zoneId, id, locale)` | Devuelve texto localizado desde `BroadcastTextEntry` | `sBroadcastTextStore`, `DB2Manager::GetBroadcastTextValue` |
| `OutdoorPvP::HandlePlayerEnterZone(Player*, zone)` | Inserta GUID en `m_players[teamId]` | — |
| `OutdoorPvP::HandlePlayerLeaveZone(...)` | `SendRemoveWorldStates` + erase | `SendRemoveWorldStates` |
| `OutdoorPvP::Update(diff)` | Tick de cada `OPvPCapturePoint` | `OPvPCapturePoint::Update` |
| `OutdoorPvP::HandleKill(Player*, Unit*)` | Dispara `HandleKillImpl` para cada miembro de grupo a *reward distance* y outdoorpvp-active | `Group::GetFirstMember`, `Player::IsAtGroupRewardDistance` |
| `OutdoorPvP::TeamApplyBuff(team, spellId1, spellId2)` | Aplica buff a team y remueve al opuesto | `TeamCastSpell` |
| `OutdoorPvP::SendDefenseMessage(zoneId, id)` | `BroadcastWorker<DefenseMessageBuilder>` localizado | `Trinity::LocalizedDo`, `WorldPackets::Chat::DefenseMessage` |
| `OutdoorPvP::ProcessEvent(target, eventId, invoker)` | Si `invoker` es GO `CONTROL_ZONE`, llama el handler correspondiente del `ControlZoneHandlers[goEntry]` | `GameObjectTemplate::controlZone` |
| `OutdoorPvP::GetWorldState/SetWorldState(int32, int32)` | Read/write via `sWorldStateMgr` con scope al `Map*` | `WorldStateMgr` |
| `OPvPCapturePoint::HandleCustomSpell` | Si `player->IsOutdoorPvPActive()` true, default true | — |
| `OPvPCapturePoint::ChangeState()` (puro virtual) | Implementado en cada zona para mover entre `OBJECTIVESTATE_*` | — |

---

## 5. Module dependencies

**Depends on:**
- `Map` / `MapManager` — `OutdoorPvP::m_map`, ticking driven from map update
- `ZoneScript` — base class (Map call dispatch)
- `WorldStateMgr` — para UI per-map worldstates (`sWorldStateMgr->SetValue/GetValue`)
- `ScriptMgr` — `CreateOutdoorPvP(scriptId, map)`, registro de scripts de zona
- `ObjectMgr` — `GetScriptId` para resolver `outdoorpvp_template.ScriptName`
- `Group` — reward distance check en `HandleKill`
- `Player` — `GetOutdoorPvP()`, `GetTeamId`, `IsOutdoorPvPActive`, `CastSpell`, `RemoveAura`
- `GameObject` (`GAMEOBJECT_TYPE_CONTROL_ZONE`) + `GameObjectTemplate::controlZone` (event ids)
- `BroadcastText` (DB2) — defense messages localizados
- `DisableMgr` (`DISABLE_TYPE_OUTDOORPVP`)
- `WorldPackets::Chat::DefenseMessage` — wire packet represented in `wow-packet`; localized send path still pending here

**Depended on by:**
- `Player` — `Player::GetOutdoorPvP()` cachea handler activo
- `World::SetInitialWorldSettings` — llama `sOutdoorPvPMgr->InitOutdoorPvP()` al boot
- `MapInstanced` / `Map::Initialize` — llama `CreateOutdoorPvPForMap`
- `MapMgr::Update` (cadena) — invoca `sOutdoorPvPMgr->Update(diff)`
- `WorldSession` — drop flag / open GO routing si la zona es OutdoorPvP

---

## 6. SQL / DB queries

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT TypeId, ScriptName FROM outdoorpvp_template` | Mapea typeId 1..5 a ScriptId del module | world |
| `pool_outdoorpvp` (referencia indirecta via PoolMgr) | Pools de spawns gobernados por estado de zona (no consultado directo desde OutdoorPvP*) | world |
| `gameobject_template.controlZone.*` (Event ids) | Read-side; alimenta `OutdoorPvP::ProcessEvent` | world |
| `broadcast_text` (DB2) | Defense messages localizados | hotfixes |

**DB2 stores leídos:**

| Store | What it loads | Read by |
|---|---|---|
| `sBroadcastTextStore` | BroadcastText.db2 | `OutdoorPvPMgr::GetDefenseMessage` |
| `sAreaTableStore` (indirecto via `HandlePlayerEnterZone`) | Area/zone metadata | Map / Player |

---

## 7. Wire-protocol packets

OutdoorPvP no tiene opcodes propios; piggybacks sobre los existentes:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_DEFENSE_MESSAGE` | server → client | `OutdoorPvP::SendDefenseMessage` (via `WorldPackets::Chat::DefenseMessage`; wire represented, localized send path pending) |
| `SMSG_INIT_WORLD_STATES` / `SMSG_UPDATE_WORLD_STATE` | server → client | `WorldStateMgr` (per-map scope), disparado por `SetWorldState` |
| `SMSG_PLAY_SOUND` (en algunos scripts de zona) | server → client | NA/HP/ZM en captura |
| `CMSG_AREATRIGGER` | client → server | `OutdoorPvP::HandleAreaTrigger` (SI silithyst flag delivery) |
| `CMSG_GAMEOBJ_USE` | client → server | `OutdoorPvP::HandleOpenGo` |
| `CMSG_CAST_SPELL` (custom IDs) | client → server | `OutdoorPvP::HandleCustomSpell` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-pvp` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- *NINGUNO*. No existe módulo `outdoor` ni en `crates/wow-pvp/` ni en `crates/wow-world/`.

**What's implemented:**
- Nada. Ni el manager singleton, ni el trait `OutdoorPvP`, ni los 5 scripts de zona, ni el dispatch desde Map.

**What's missing vs C++:**
- 100%. Esto incluye: tabla `outdoorpvp_template` loader, registro en `ScriptMgr`, ticking 1Hz acoplado a Map, hook de player enter/leave zone, hook control-zone GO events, defense-message broadcast, world-state writes per map, las 5 implementaciones zonales (HP/NA/SI/TF/ZM), y la cadena reward (kill credit, buffs, áreatrigger silithyst).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `Player::GetOutdoorPvP()` no existe en Rust → cualquier dispatch desde sesión (drop flag, open GO en zonas OPvP) está dead code o silenciosamente ignorado.
- `WorldStateMgr` Rust (si existe) probablemente no soporta scope `Map*`; OutdoorPvP necesita worldstates *per-map-instance* (no globales).
- DefenseMessages requieren `BroadcastText.db2` localizado — verificar que el pipeline de hotfixes esté cargando esa store.

**Tests existing:**
- 0 tests.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#OUTDOORPVP.WBS.001** Cerrar la migracion auditada de `game/OutdoorPvP/OutdoorPvP.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#OUTDOORPVP.WBS.002** Cerrar la migracion auditada de `game/OutdoorPvP/OutdoorPvP.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`
  Rust target: `crates/wow-pvp`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#OUTDOORPVP.WBS.003** Cerrar la migracion auditada de `game/OutdoorPvP/OutdoorPvPMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp`
  Rust target: `crates/wow-pvp`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#OUTDOORPVP.WBS.004** Cerrar la migracion auditada de `game/OutdoorPvP/OutdoorPvPMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.h`
  Rust target: `crates/wow-pvp`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Complejidad: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#OPVP.1** Crear módulo `crates/wow-pvp/src/outdoor/mod.rs` con enums `OutdoorPvpType` (HP/NA/TF/ZM/SI), `ObjectiveState` (7 variantes), `OutdoorPvpEvent` (control-zone events) (L)
- [ ] **#OPVP.2** Definir trait `OutdoorPvp` con métodos `setup`, `update(diff)`, `handle_player_enter_zone`, `handle_player_leave_zone`, `handle_player_resurrects`, `handle_kill`, `handle_drop_flag`, `handle_custom_spell`, `handle_open_go`, `handle_area_trigger`, `process_event`, `send_remove_world_states` (M)
- [ ] **#OPVP.3** Definir trait `OPvPCapturePoint` con `update(diff)`, `change_state`, `change_team`, `handle_custom_spell`, `handle_open_go`, `handle_drop_flag` y campos `team`, `old_state`, `state` (M)
- [ ] **#OPVP.4** Estructura `OutdoorPvpControlZoneHandler` con vtable async para los 8 eventos (Progress/Contested/Neutral/Capture × Alliance/Horde) (M)
- [ ] **#OPVP.5** Implementar `OutdoorPvpMgr` singleton (`OnceCell` o `Arc<RwLock<...>>`) con `outdoor_pvp_by_map: HashMap<MapId, Vec<Box<dyn OutdoorPvp>>>` y `outdoor_pvp_map: HashMap<(MapId, ZoneId), Weak<...>>` (H)
- [ ] **#OPVP.6** Loader `init_outdoor_pvp()` desde `outdoorpvp_template` (`SELECT TypeId, ScriptName`) + integración con `wow-script::ScriptId` registry (M)
- [ ] **#OPVP.7** `create_outdoor_pvp_for_map(map)` — instanciar handlers para los map-ids hardcoded `[0=unused, 530, 530, 530, 530, 1]` (HP/NA/TF/ZM en Outland 530; SI en Kalimdor 1) (M)
- [ ] **#OPVP.8** `destroy_outdoor_pvp_for_map(map)` — cleanup en map unload (L)
- [ ] **#OPVP.9** Tick driver: en `MapManager::update` invocar `sOutdoorPvpMgr.update(diff)` con interval 1000ms acumulado (L)
- [ ] **#OPVP.10** Hook `Player::update_zone` → `handle_player_enter_zone` / `handle_player_leave_zone` (vía `Session::set_zone`) (M)
- [ ] **#OPVP.11** Cachear `Player::outdoor_pvp` (Weak handle) para acceso rápido en handlers de session (drop flag, open GO) (L)
- [ ] **#OPVP.12** `team_apply_buff(team, spell_id1, spell_id2)` y `team_cast_spell(team, spell_id_signed)` integrado con `wow-spell` (M)
- [ ] **#OPVP.13** `process_event(target, event_id, invoker)` — pattern match para GO `ControlZone` y dispatch a handler por entry (M)
- [ ] **#OPVP.14** Implementar `OutdoorPvpHP` (Hellfire Peninsula): 3 torres (`HP_GO_ENTRY_TOWER_W/N/S = 182173/4/5`), counters Alliance/Horde, world-states 0x9ba/0x9b9/0x9ae/0x9ac, buffs 32071/32049, kill rewards 32155/32158 (H)
- [ ] **#OPVP.15** Implementar `OutdoorPvpNA` (Nagrand / Halaa): control point + guard waves + flight spells + Wyvern states + Halaa events + buff 33795 (H)
- [ ] **#OPVP.16** Implementar `OutdoorPvpSI` (Silithus): silithyst flag-carrier (spell 29519), tracker areatrigger, gather counters worldstates 2313/2314/2317, Cenarion Favor 30754 (H)
- [ ] **#OPVP.17** Implementar `OutdoorPvpTF` (Terokkar Forest): 5 spirit towers, contestation timer, Blessing of Auchindoun buff (H)
- [ ] **#OPVP.18** Implementar `OutdoorPvpZM` (Zangarmarsh): 3 beacons + graveyard capture point + Twin Spire Blessing 33779 (H)
- [ ] **#OPVP.19** Defense message system: cargar `BroadcastText.db2`, `get_defense_message(zone, id, locale) -> String`, opcode `SMSG_DEFENSE_MESSAGE` builder (M)
- [ ] **#OPVP.20** Integrar `WorldStateMgr` per-map scope: `set_world_state(id, value, map)` debe broadcast `SMSG_UPDATE_WORLD_STATE` solo a players en ese mapa (M)
- [ ] **#OPVP.21** Reward chain: `handle_kill` recorre group con `is_at_group_reward_distance`, llama `handle_kill_impl` por zona (M)
- [ ] **#OPVP.22** `handle_area_trigger` routing desde `Player::area_trigger` handler (necesario para SI) (L)
- [ ] **#OPVP.23** `handle_drop_flag(spell_id)` desde `Spell` cancel/dispel pipeline (L)
- [ ] **#OPVP.24** `handle_open_go(go)` desde `GameObject::use_door_or_button` / `GameObjectUse` opcode (L)
- [ ] **#OPVP.25** `is_outdoor_pvp_active(player)` — flag dinámico basado en zone + buff state (L)
- [ ] **#OPVP.26** Disable check: `DisableMgr.is_disabled_for(OutdoorPvp, type_id)` antes de instanciar (L)
- [ ] **#OPVP.27** Pool integration: si `pool_outdoorpvp` existe, vincular spawn-pool toggling con cambios de estado (M)
- [ ] **#OPVP.28** Persistencia (tower state) — verificar si TC persiste estados; en WoLK 3.4.x los outdoor PvP **no persisten** entre restarts (reset al boot) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#OUTDOORPVP.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 4 files / 823 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp`. Rust target: `crates/wow-pvp`, `crates/wow-world`. | `cargo test -p wow-pvp && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#OUTDOORPVP.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 4 files / 823 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp`. Rust target: `crates/wow-pvp`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#OUTDOORPVP.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 4 files / 823 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp`. Rust target: `crates/wow-pvp`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#OUTDOORPVP.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 4 files / 823 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp`. Rust target: `crates/wow-pvp`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: cargar `outdoorpvp_template` con 5 entradas → `m_OutdoorPvPDatas[1..=5]` poblado, count=5
- [ ] Test: `CreateOutdoorPvPForMap(map=530)` instancia exactamente 4 handlers (HP/NA/TF/ZM) y NO el de Silithus
- [ ] Test: `CreateOutdoorPvPForMap(map=1)` instancia exactamente 1 handler (SI)
- [ ] Test: `Update(500ms)` no dispara tick; `Update(1100ms)` dispara exactamente 1 tick por handler
- [ ] Test: `HandlePlayerEnterZone` con `(map, zoneId)` desconocido es no-op
- [ ] Test: `HandlePlayerEnterZone` doble llamada → segunda es no-op (`HasPlayer` true)
- [ ] Test: `TeamApplyBuff(ALLIANCE, S1, S2)` aplica S1 a alliance y remueve S2 a horde
- [ ] Test: HP — capturar las 3 torres por Alliance → `m_AllianceTowersControlled==3`, world-state `HP_UI_TOWER_COUNT_A==3`, buff alliance aplicado
- [ ] Test: HP — torre cambia de neutral→horde-progress→horde dispara `HandleProgressEventHorde` → `HandleCaptureEventHorde` exactamente una vez cada uno
- [ ] Test: SI — entregar silithyst flag (areatrigger) incrementa `SI_GATHERED_*`, alcanza `SI_SILITHYST_MAX=200` → buff Cenarion Favor a la facción ganadora
- [ ] Test: defense message localizado para locale enUS y esES devuelve strings distintas
- [ ] Test: round-trip `process_event(go=control_zone, event=ContestedEventAlliance)` invoca `HandleContestedEventAlliance` del handler con entry GO correcto
- [ ] Test: parity vs C++: secuencia HP completa de 10 minutos genera idéntico timeline de world-state changes y SMSG_DEFENSE_MESSAGE

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 4 files / 823 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp` | `crates/wow-pvp/` (nuevo módulo `outdoor/`), wiring en `crates/wow-world/` \| ❌ not started |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#OUTDOORPVP.DIV.001` | `crates/wow-pvp` (`exists_empty`, 0 Rust lines) | 4 C++ files / 823 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvP.h`, `/home/server/woltk-trinity-legacy/src/server/game/OutdoorPvP/OutdoorPvPMgr.cpp` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **ZONA EP (Eastern Plaguelands) NO EXISTE en este fork legacy.** El template original de TC tiene 7 zonas; este snapshot WoLK sólo registra HP/NA/TF/ZM/SI (`OutdoorPvPTypes::MAX_OUTDOORPVP_TYPES = 6` con índice 0 reservado).
- **Wintergrasp NO está aquí.** Vive en `Battlefield/` (zona 4197, lake battle 30 min) — documentar en `battlefield.md`. El field `m_OutdoorMapIds[5] = 1` es para Silithus en Kalimdor; `[1..=4] = 530` son los 4 de Outland.
- World-states en OutdoorPvP usan **scope `Map*`**, NO global. Si Rust hace los worldstates globales, alliance ganando HP en server 1 mostrará buff en server-instance 2 (BUG potencial).
- `OPvPCapturePoint::HandleCustomSpell` por defecto retorna `true` si player es OutdoorPvPActive — esto bloquea fall-through a otros handlers; cuidado con re-implementar el orden.
- En C++ `OutdoorPvP::HandleKill` aplica reward sólo si `IsOutdoorPvPActive()` (flag dinámico) **o** si la víctima es UNIT (creature kills siempre cuentan). Player kills sólo cuentan si activo.
- `OUTDOORPVP_OBJECTIVE_UPDATE_INTERVAL = 1000ms` es el tick GLOBAL del manager, no por-handler. Cada handler recibe `diff = m_UpdateTimer` acumulado (puede ser >1000 si el world tick se atrasa).
- `OutdoorPvP` extiende `ZoneScript` — esto es una jerarquía Map-side. Cuando se migre `ZoneScript` (battlefields, BG, instances), el trait `OutdoorPvp` debería componerse, no heredar.
- `ProcessEvent` lee `gameobject->GetGOInfo()->controlZone.{Capture,Contested,Neutral,Progress}Event{Alliance,Horde}` — esto son **8 event-ids por GO** definidos en `gameobject_template`. Sin esos campos cargados, todos los handlers son silent no-ops.
- En 3.4.x los outdoor PvP states **no persisten** en DB; reset completo al startup (vs Wintergrasp que sí persiste).
- DefenseMessage usa `Trinity::PacketSenderOwning<DefenseMessage>` con `LocalizedDo` — patrón que requiere localizer per-receiver. Es perf hotspot durante captures masivos en HP/ZM.

---

## 12. C++ → Rust mapping

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class OutdoorPvP : ZoneScript` | `trait OutdoorPvp: ZoneScript` (en `crates/wow-pvp/src/outdoor/mod.rs`) | Sin herencia; trait + struct concreto por zona |
| `class OPvPCapturePoint` | `trait OPvPCapturePoint` | Async fns para `update`, `change_state`, etc. |
| `OutdoorPvPControlZoneHandler` | `trait ControlZoneHandler` o `struct` con 8 closures | Per-tower state en HP/NA/ZM |
| `class OutdoorPvPMgr` (singleton) | `static OUTDOOR_PVP_MGR: OnceCell<OutdoorPvpMgr>` | Inicialización en `World::set_initial_world_settings` |
| `std::unordered_map<std::pair<Map*, uint32>, OutdoorPvP*>` | `DashMap<(MapId, ZoneId), Arc<dyn OutdoorPvp>>` | Concurrencia desde múltiples mapas |
| `std::unordered_map<Map*, std::vector<std::unique_ptr<OutdoorPvP>>>` | `DashMap<MapId, Vec<Arc<dyn OutdoorPvp>>>` | Owner por mapa |
| `OPvPCapturePointMap = std::map<lowGuid, unique_ptr<...>>` | `BTreeMap<u64, Box<dyn OPvPCapturePoint>>` | Spawn-id keyed |
| `m_players[2]` (`GuidSet[2]`) | `[HashSet<Guid>; 2]` indexado por `TeamId as usize` | — |
| `enum ObjectiveStates` (7 valores) | `enum ObjectiveState { Neutral, Alliance, Horde, NeutralAllianceChallenge, NeutralHordeChallenge, AllianceHordeChallenge, HordeAllianceChallenge }` | — |
| `enum OutdoorPvPTypes` | `#[repr(u8)] enum OutdoorPvpType { Hp=1, Na=2, Tf=3, Zm=4, Si=5 }` | `MAX = 6` |
| `void Update(uint32 diff)` | `async fn update(&self, diff: Duration)` | — |
| `int32 GetWorldState/SetWorldState` | `world_state(id) -> i32` / `set_world_state(id, val)` | Scope al `Map` |
| `BroadcastPacket(WorldPacket const*)` | `broadcast_packet(&packet)` iterando `m_players[2]` | Acceso `ObjectAccessor::FindPlayer` → Rust `Map::find_player` |
| `Trinity::LocalizedDo<DefenseMessageBuilder>` | Iter `players.zone == zoneId` y construir packet por locale | Sin templates; closure per-locale |
| `sScriptMgr->CreateOutdoorPvP(scriptId, map)` | `wow_script::Registry::create_outdoor_pvp(script_id, map)` | Factory dispatch |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

❌ confirmado. Auditado contra `/home/server/rustycore/crates/`.

**Hallazgos clave:**
- No existe módulo `outdoor` en `crates/wow-pvp/` (recordatorio: `wow-pvp/src/lib.rs` está vacío). No existe `crates/wow-world/src/outdoor/`.
- Búsqueda de `OutdoorPvp`, `OutdoorPvpMgr`, `OPvPCapturePoint`, `outdoorpvp_template`, `Halaa`, `Silithyst`, `Hellfire`, `Terokkar`, `Zangarmarsh` en todo el workspace: **0 resultados**.
- Búsqueda de los buff IDs distintivos (`32071`, `32049`, `33795`, `30754`, `33779` — Strength of the Halaani, Cenarion Favor, Twin Spire Blessing, Hellfire Towers, Blessing of Auchindoun): **0 resultados**.
- Tabla `outdoorpvp_template` no aparece en `crates/wow-database/src/`. Cero queries SQL relacionadas.
- 0 handlers para `CMSG_AREATRIGGER` con routing OutdoorPvP (Silithus). 0 handlers `HandleOpenGo` con routing OutdoorPvP.

**Verificación zone count (claim del doc: 5 zonas, NOT 7):**
- ✅ confirmado revisando el doc original §2: HP, NA, TF, ZM, SI son las 5 zonas con scripts en este fork legacy WoLK 3.4.3. Eastern Plaguelands (EP) **no** tiene script en `OutdoorPvP/` en este fork. Wintergrasp vive en `Battlefield/` (separado, no cuenta como OutdoorPvP). El enum C++ es `OutdoorPvPTypes::MAX = 6` con índice 0 reservado → 5 entradas reales (HP=1, NA=2, TF=3, ZM=4, SI=5).
- Map distribution confirmada: HP/NA/TF/ZM en map 530 (Outland), SI en map 1 (Kalimdor). Total 4 handlers en map 530, 1 handler en map 1.

**Riesgo de UI hang silencioso:**
- 🟢 **Bajo riesgo**. Los OutdoorPvP no tienen opcodes propios — piggybacks sobre `CMSG_AREATRIGGER` (ya hay handler genérico en `wow-world`), `CMSG_GAMEOBJ_USE` (idem), `CMSG_CAST_SPELL` (idem). En zonas OutdoorPvP el client no se queda esperando packets server-only — la única UI afectada son los `SMSG_DEFENSE_MESSAGE` (zone chat) y los `SMSG_UPDATE_WORLD_STATE` (HUD widgets de torres/buffs). Sin esos, el HUD muestra valores stale/iniciales pero no se cuelga.
- ⚠️ Riesgo MUY menor: en HP/NA/ZM, los control-zone GameObjects son interactuables — al "click" sobre una capture flag, el cliente envía `CMSG_GAMEOBJ_USE`. Sin routing OutdoorPvP, el GO solo dispara su default `use` (animación) sin progresar la captura. No hay hang, pero el feedback visual del progress bar nunca avanza. Inofensivo en single-player; visible solo si hay raid PvP organizada en Outland — improbable a este nivel del proyecto.

**Acción:** dejar `❌ not started`. Migrar después de WorldStateMgr (per-map scope). El orden recomendado: implementar SI primero (más simple — flag carrier + areatrigger, 1 zona, sin towers), luego HP (3 torres simétricas), luego ZM/TF/NA. Auditar particularmente §11 nota sobre "world-states scope `Map*`, NO global" — bug latente si se cablea un singleton sin map binding.
