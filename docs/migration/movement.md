# Migration: Movement

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Movement/` (+ `src/server/game/Server/Packets/MovementPackets.{h,cpp}`)
> **Rust target crate(s):** `crates/wow-packet/` (packets), `crates/wow-world/` (handlers + per-session state), `crates/wow-recastdetour/` (Detour FFI scaffold), `crates/wow-movement/`
> **Layer:** L5 (depende de Maps L3 + Entities L4 + DataStores L1)
> **Status:** ⚠️ partial — parsing/broadcasts de movimiento y primer núcleo `MoveSpline` aislado; aún sin Unit/MotionMaster/pathfinding/generators conectados.
> **Audited vs C++:** ✅ audited 2026-05-01 (engine missing entirely — see §13)
> **Last updated:** 2026-05-11

---

## 1. Purpose

Encapsula toda la lógica de **traslación** de unidades en el mundo: parsing de packets de movimiento del cliente (player), generación server-side de splines para creatures (MoveSpline), pathfinding navegable (Detour vía MMaps) y orquestación de comportamientos AI (MotionMaster + MovementGenerators: idle, random, waypoint, follow, chase, flee, point, flight, formation, home, spline-chain). Resuelve sincronía cliente-servidor (anti-cheat de velocidad/posición), interpolación suave en transports, jumps parabólicos y caídas. Todo Update de Unit pasa por aquí.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Movement/AbstractFollower.cpp` | 31 | `prefix` |
| `game/Movement/AbstractFollower.h` | 36 | `prefix` |
| `game/Movement/MotionMaster.cpp` | 1376 | `prefix` |
| `game/Movement/MotionMaster.h` | 246 | `prefix` |
| `game/Movement/MovementDefines.cpp` | 48 | `prefix` |
| `game/Movement/MovementDefines.h` | 142 | `prefix` |
| `game/Movement/MovementGenerator.cpp` | 61 | `prefix` |
| `game/Movement/MovementGenerator.h` | 154 | `prefix` |
| `game/Movement/enuminfo_MovementDefines.cpp` | 115 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Movement/MotionMaster.h` | 246 | Stack de MovementGenerators por slot (default/active/controlled), API pública (Move*) |
| `src/server/game/Movement/MotionMaster.cpp` | 1376 | Implementación: Update, Add/Remove, MoveTo/MoveChase/MoveFollow/MoveJump/etc. |
| `src/server/game/Movement/MovementGenerator.h` | 154 | Interfaz abstracta + flags + factory holders (Idle/Random/Waypoint) |
| `src/server/game/Movement/MovementGenerator.cpp` | 61 | Destructor + GetDebugInfo |
| `src/server/game/Movement/MovementDefines.h` | 142 | Enums: MovementGeneratorType, MovementSlot, MovementGeneratorMode/Priority, RotateDirection, ChaseRange/Angle |
| `src/server/game/Movement/MovementDefines.cpp` | 48 | ChaseAngle helpers |
| `src/server/game/Movement/PathGenerator.h` | 150 | Detour-based pathfinder per WorldObject (pathPolyRefs, NavMesh, filter) |
| `src/server/game/Movement/PathGenerator.cpp` | 1045 | BuildPolyPath, BuildPointPath, FindSmoothPath, FixupCorridor, NormalizePath |
| `src/server/game/Movement/AbstractFollower.{h,cpp}` | 36+31 | Common helper: store target GUID, detect target moves |
| `src/server/game/Movement/Spline/MoveSpline.h` | 155 | MoveSpline class — current spline + time_passed + flags + ComputePosition |
| `src/server/game/Movement/Spline/MoveSpline.cpp` | 400 | _updateState, ComputePosition, parabolic/fall elevation, ToString |
| `src/server/game/Movement/Spline/MoveSplineFlag.h` | 141 | Bitfield flags (Done, Falling, Flying, Cyclic, CatmullRom, Parabolic, etc.) |
| `src/server/game/Movement/Spline/MoveSplineInit.h` | 220 | Builder for spline movement: SetFly/SetWalk/SetCyclic/SetParabolic/MoveTo/MovebyPath |
| `src/server/game/Movement/Spline/MoveSplineInit.cpp` | 294 | Launch (validates+commits), Stop, transport transform |
| `src/server/game/Movement/Spline/MoveSplineInitArgs.h` | 94 | POD args fed to MoveSpline::Initialize |
| `src/server/game/Movement/Spline/Spline.h` | 217 | Generic templated spline (linear / catmullrom / bezier) |
| `src/server/game/Movement/Spline/Spline.cpp` | 312 | InitLengths, InitSpline interpolation, segment lookup |
| `src/server/game/Movement/Spline/SplineImpl.h` | 96 | Inline impls of computeIndex/evaluate |
| `src/server/game/Movement/Spline/MovementUtil.cpp` | 212 | gravity/JumpVelocity/computeFallTime/computeFallElevation |
| `src/server/game/Movement/Spline/SplineChain.h` | 50 | SplineChainLink + SplineChainResumeInfo POD |
| `src/server/game/Movement/Waypoints/WaypointDefines.h` | 86 | WaypointNode, WaypointPath, WaypointPathType, MoveType |
| `src/server/game/Movement/Waypoints/WaypointManager.h` | 73 | Singleton: load `waypoint_path` + `waypoint_path_node` |
| `src/server/game/Movement/Waypoints/WaypointManager.cpp` | 321 | LoadPaths, GetPath, ReloadPath |
| `src/server/game/Movement/MovementGenerators/IdleMovementGenerator.h` | 81 | Idle/Distract/Rotate/AssistanceDistract |
| `src/server/game/Movement/MovementGenerators/RandomMovementGenerator.h` | 58 | Random wander within radius |
| `src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.h` | 93 | Path traversal con pauses, scripts, formation |
| `src/server/game/Movement/MovementGenerators/FollowMovementGenerator.h` | 62 | Tail target with offset+angle |
| `src/server/game/Movement/MovementGenerators/ChaseMovementGenerator.h` | 59 | Aggressive follow with ChaseRange/ChaseAngle |
| `src/server/game/Movement/MovementGenerators/FleeingMovementGenerator.h` | 67 | Run away from a feared source |
| `src/server/game/Movement/MovementGenerators/ConfusedMovementGenerator.h` | 49 | Random short hops (CC effect) |
| `src/server/game/Movement/MovementGenerators/HomeMovementGenerator.h` | 41 | Return-to-home after evade |
| `src/server/game/Movement/MovementGenerators/PointMovementGenerator.h` | 75 | Move to single (x,y,z) point |
| `src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.h` | 75 | Taxi flight; integrates `TaxiPathNodes` |
| `src/server/game/Movement/MovementGenerators/FormationMovementGenerator.h` | 57 | Anchor offset to leader |
| `src/server/game/Movement/MovementGenerators/SplineChainMovementGenerator.h` | 61 | Chained MoveSplines with pauses |
| `src/server/game/Movement/MovementGenerators/GenericMovementGenerator.h` | 55 | One-shot custom MoveSplineInit wrapper |
| `src/server/game/Movement/MovementGenerators/PathMovementBase.h` | 43 | Shared path index/iter helpers |
| `src/server/game/Server/Packets/MovementPackets.h` | 728 | Packet structs (CommonMovement, MonsterMove, MoveUpdate, ClientPlayerMovement, MoveSetXxxSpeed, MoveTeleport, FlightSplineSync, etc.) |
| `src/server/game/Server/Packets/MovementPackets.cpp` | 1097 | Read/Write for all of the above |
| `src/server/game/Handlers/MovementHandler.cpp` | ~1200 | WorldSession::HandleMovement* dispatch + anti-cheat hooks |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Movement::MoveSpline` | class | Current spline state of a Unit (path, time_passed, flags, point_Idx) |
| `Movement::MoveSplineFlag` | bitfield struct | Flags Done/Falling/Flying/Cyclic/CatmullRom/Parabolic/UncompressedPath/etc. |
| `Movement::MoveSplineInit` | class (builder) | Fluent builder: MoveTo, MovebyPath, SetFly, SetCyclic, SetParabolic, Launch |
| `Movement::MoveSplineInitArgs` | struct | POD packed args (path, flags, walk, velocity, parabolic_amplitude, animTier) |
| `Movement::Spline<T>` | template class | Templated curve evaluator (linear/catmullrom/bezier) |
| `Movement::Location` | struct | Vector3 + orientation |
| `Movement::FacingInfo` | struct | Facing target spec (angle / point / GUID) |
| `MovementGenerator` | abstract class | Interfaz: Initialize/Reset/Update/Deactivate/Finalize |
| `MovementGeneratorMedium<T,D>` | template | CRTP for typed Update dispatch |
| `MotionMaster` | class | Stack-of-generators per slot (DEFAULT/ACTIVE/CONTROLLED) for one Unit |
| `MotionMasterDelayedActionType` | enum | Defer ops while update in progress |
| `MovementGeneratorType` | enum | IDLE/RANDOM/WAYPOINT/CONFUSED/CHASE/HOME/FLIGHT/POINT/FLEEING/DISTRACT/EFFECT/SPLINE_CHAIN/FORMATION/etc. |
| `MovementSlot` | enum | DEFAULT/ACTIVE/CONTROLLED |
| `MovementGeneratorPriority` | enum | NORMAL/AUTHORITATIVE/HIGHEST |
| `MovementGeneratorFlags` | bitfield | INITIALIZED/INTERRUPTED/PAUSED/FINALIZED/PERSIST_ON_DEATH |
| `IdleMovementGenerator` / `DistractMovementGenerator` / `RotateMovementGenerator` / `AssistanceDistractMovementGenerator` | class | Idle variants |
| `RandomMovementGenerator<Creature>` | class | Wander within radius |
| `WaypointMovementGenerator<Creature>` | class | Traverse `waypoint_path` rows, fire AI hooks |
| `FollowMovementGenerator` | class | Stick to target at offset+angle |
| `ChaseMovementGenerator` | class | Combat follow w/ ChaseRange + ChaseAngle |
| `FleeingMovementGenerator` | class | Move away from a fearer |
| `ConfusedMovementGenerator` | class | CC-induced random wander |
| `PointMovementGenerator` | class | Move to a fixed point, fire MovementInform on arrival |
| `HomeMovementGenerator<Creature>` | class | Drive evade-return-to-home |
| `FlightPathMovementGenerator` | class | Taxi flight along TaxiPathNodes |
| `FormationMovementGenerator` | class | Anchor offset to formation leader |
| `SplineChainMovementGenerator` | class | Chain of MoveSplineInit launches with pauses |
| `GenericMovementGenerator` | class | Generic custom spline-based one-shot |
| `PathGenerator` | class | Detour pathfinder: BuildPolyPath, FindSmoothPath, NormalizePath, ShortenPathUntilDist |
| `PathType` | enum | BLANK/NORMAL/SHORTCUT/INCOMPLETE/NOPATH/NOT_USING_PATH/SHORT/FARFROMPOLY |
| `WaypointNode` | struct | id, x, y, z, orientation, delay, moveType, eventId, eventChance |
| `WaypointPath` | struct | id, name, MoveType, pathType, repeat, pathDirection, vector<WaypointNode> |
| `WaypointPathType` | enum | LOOP/ONCE/ROUNDTRIP |
| `WaypointMoveType` | enum | WALK/RUN/LAND/TAKEOFF |
| `WaypointManager` | singleton | Loaders for `waypoint_path*` SQL |
| `MovementInfo` (in `Object.h`) | struct | flags, time, pos, jump, transport, swim/fall extra (consumed by every CMSG_MOVE_*) |
| `WorldPackets::Movement::ClientPlayerMovement` | packet | Generic movement CMSG (carries MovementInfo) |
| `WorldPackets::Movement::MonsterMove` | packet | SMSG_ON_MONSTER_MOVE (server moves a creature) |
| `WorldPackets::Movement::MoveUpdate` | packet | SMSG_MOVE_UPDATE broadcast |
| `WorldPackets::Movement::MoveTeleport` | packet | SMSG_MOVE_TELEPORT |
| `WorldPackets::Movement::FlightSplineSync` | packet | SMSG_FLIGHT_SPLINE_SYNC progress |
| `AbstractFollower` | helper | GUID-based follower base for chase/follow generators |
| `TransportPathTransform` | functor | Map global ↔ transport-local coords |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `MotionMaster::Initialize()` / `InitializeDefault()` | Bootstrap default IdleMovementGenerator on slot DEFAULT | `MovementGenerator::Initialize` |
| `MotionMaster::Update(uint32 diff)` | Tick top generator; pop+finalize on completion; resolve delayed actions | `MovementGenerator::Update`, `Finalize` |
| `MotionMaster::Add(MovementGenerator*, MovementSlot)` | Push generator on slot; defer if updating | `MovementGenerator::Initialize` |
| `MotionMaster::Remove(type, slot)` / `Clear(slot)` | Pop+Finalize matching | `MovementGenerator::Finalize` |
| `MotionMaster::MoveIdle()` / `MoveTargetedHome()` | Convenience wrappers | `Add(new IdleMovementGenerator)` etc. |
| `MotionMaster::MoveRandom(radius, duration)` | Push RandomMovementGenerator | `Add` |
| `MotionMaster::MoveFollow(target, dist, ChaseAngle, duration, slot)` | Push FollowMovementGenerator | `Add` |
| `MotionMaster::MoveChase(target, ChaseRange, ChaseAngle)` | Push ChaseMovementGenerator | `Add` |
| `MotionMaster::MovePoint(id, x, y, z, generatePath, finalOrient)` | Push PointMovementGenerator with MoveSplineInit | `MoveSplineInit::Launch` |
| `MotionMaster::MoveJump(x, y, z, speedXY, speedZ, id)` | Parabolic jump to point | `MoveSplineInit::SetParabolic`+`Launch` |
| `MotionMaster::MoveFall(id)` | Drop straight down using gravity | `MoveSplineInit::SetFall`+`Launch` |
| `MotionMaster::MoveCharge(x, y, z, speed, id)` | Fast linear charge | `MoveSplineInit::Launch` |
| `MotionMaster::MoveFleeing(enemy, time)` | Push FleeingMovementGenerator | `Add` |
| `MotionMaster::MoveConfused()` | CC wander | `Add` |
| `MotionMaster::MoveTaxiFlight(pathId, pathnode)` | Push FlightPathMovementGenerator | `Add` |
| `MotionMaster::MoveAlongSplineChain(pointId, chainId, walk)` | Push SplineChainMovementGenerator | `Add` |
| `MotionMaster::MoveSplinePath(PointsArray*)` | Custom path | `Add(new GenericMovementGenerator)` |
| `MoveSpline::Initialize(MoveSplineInitArgs const&)` | Begin spline with parsed args | `init_spline`, `Spline::initLengths` |
| `MoveSpline::updateState(int32 difftime)` | Advance time_passed; emit Result_Arrived/NextSegment/NextCycle | `_updateState`, `ComputePosition` |
| `MoveSpline::ComputePosition()` / `ComputePosition(int32 t)` | Interpolate position for time | `Spline::evaluate`, `computeParabolicElevation` |
| `MoveSpline::_Finalize()` | Snap unit to FinalDestination | — |
| `MoveSplineInit::Launch()` | Validate+commit MoveSpline + send SMSG_ON_MONSTER_MOVE | `MoveSpline::Initialize`, packet send |
| `MoveSplineInit::Stop()` | Send stop spline packet | packet send |
| `MoveSplineInit::MoveTo(Vector3, generatePath, forceDest)` | Build PathGenerator path or straight-line | `PathGenerator::CalculatePath` |
| `MoveSplineInit::SetParabolic` / `SetFall` / `SetFly` / `SetCyclic` / `SetSmooth` / `SetWalk(bool)` / `SetVelocity(f)` | Configure flags/velocity | bitfield mutators |
| `PathGenerator::CalculatePath(x, y, z, forceDest)` | Build polypath + smooth path | `BuildPolyPath`, `FindSmoothPath`, `BuildShortcut` |
| `PathGenerator::ShortenPathUntilDist(point, dist)` | Trim final segment | — |
| `WaypointManager::LoadPaths()` | Query SQL `waypoint_path` + `waypoint_path_node` | DBWorldDatabase |
| `WorldSession::HandleMovementOpcodes(WorldPacket&)` | Parse MovementInfo + anti-cheat + Player::SetPosition + broadcast SMSG_MOVE_UPDATE | `MovementInfo::Read`, `Player::Relocate`, `SendMessageToSet` |
| `Movement::computeFallTime(path_length, isSafe)` | Util: time to fall N yards | math |

---

## 5. Module dependencies

**Depends on:**
- `Maps` — `Map::GetHeight`, `VMapManager` para checks de altura/visión.
- `Grids` — `Map::PlayerRelocation` y notificadores de visibilidad al moverse.
- `Entities/Object` — `MovementInfo`, `Position`, `WorldObject::Relocate`.
- `Entities/Unit` — owner de MotionMaster; recibe MoveSplineInit::Launch.
- `Entities/Creature` — RandomMovement/Waypoint/Home/Flight generators son `<Creature>`-typed.
- `Entities/Player` — handlers CMSG_MOVE_*, taxi flights, transports.
- `DataStores` (DBC: `MapDifficulty`, `LiquidType`, `TaxiPathNode`, `TaxiPath`).
- `Detour` (3rdparty) vía `MMapMgr` / `MMapManager` para PathGenerator.
- `Spell System` — knockbacks, charges, parabolic spells generan MoveSplineInit con SetParabolic.
- `Combat` — ChaseMovementGenerator se activa al entrar combate; FleeingMovementGenerator desde fear.
- `AI` (CreatureAI/SmartAI) — driver: emite `MovePoint`, `MoveChase`, `MoveFollow`.
- `Transports` — `TransportBase`, `TransportPathTransform` convierte coords global ↔ transport-local.

**Depended on by:**
- `Combat` — chase target.
- `AI/SmartAI/Scripts` — todos los scripts de boss llaman MotionMaster.
- `Spells` — efectos de movimiento (charge, blink, knockback).
- `Battlegrounds/Instances` — moves teleport y spline events.
- `Pets/Vehicles` — follow/chase del owner; vehicle relocation.
- `Quests` — escort waypoints (`script_waypoint`).
- `World/MapManager` — Update llama Unit::Update → MotionMaster::Update.

---

## 6. SQL / DB queries (if any)

Solo el módulo Waypoints emite queries directas (vía `WaypointManager`). El resto del sistema consume datos a través de Creature/Spawn loaders.

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM waypoint_path` | Carga metadata de paths (nombre, MoveType, pathType, direction) | world |
| `SELECT * FROM waypoint_path_node` | Carga nodos (id, x, y, z, orientation, delay, action, eventId) | world |
| `SELECT * FROM waypoint_data` (legacy) | Backwards-compat path nodes | world |
| `SELECT * FROM creature_addon` (campo `path_id`) | Asocia creature spawn con waypoint path | world |
| `SELECT * FROM creature_template_addon` (campo `path_id`) | Asocia template default a path | world |
| `SELECT * FROM creature_movement_override` | Overrides MovementType / ground / swim / flight per spawn | world |
| `SELECT * FROM script_waypoint` | Escort/quest scripted waypoints | world |
| `SELECT * FROM creature_formations` | Formation leader/follower offsets | world |
| `SELECT * FROM spline_chain` (link, points) | Static spline chain definitions | world |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `TaxiPathStore` | `TaxiPath.dbc` | FlightPathMovementGenerator |
| `TaxiPathNodeStore` | `TaxiPathNode.dbc` | FlightPathMovementGenerator |
| `LiquidTypeStore` | `LiquidType.dbc` | MoveSplineInit (water flags) |
| `MapDifficultyStore` | `MapDifficulty.dbc` | Speed/distance scalers |

---

## 7. Wire-protocol packets (if any)

(Subset; opcodes de WoW 3.4.3 — ver `Opcodes.h` para IDs hexadecimales.)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_MOVE_START_FORWARD` (0x39E4) | C→S | `WorldSession::HandleMovementOpcodes` → MovementInfo |
| `CMSG_MOVE_START_BACKWARD` (0x39E5) | C→S | id. |
| `CMSG_MOVE_STOP` (0x39E6) | C→S | id. |
| `CMSG_MOVE_START_STRAFE_LEFT/RIGHT` (0x39E7/0x39E8) | C→S | id. |
| `CMSG_MOVE_STOP_STRAFE` (0x39E9) | C→S | id. |
| `CMSG_MOVE_JUMP` (0x39EA) | C→S | id. (jump info: fall_time, z_speed, sin/cos angle) |
| `CMSG_MOVE_START_TURN_LEFT/RIGHT` (0x39EC/0x39ED) | C→S | id. |
| `CMSG_MOVE_STOP_TURN` (0x39EE) | C→S | id. |
| `CMSG_MOVE_START_PITCH_UP/DOWN` (0x39EF/0x39F0) | C→S | id. (flying) |
| `CMSG_MOVE_STOP_PITCH` (0x39F1) | C→S | id. |
| `CMSG_MOVE_SET_RUN_MODE` / `CMSG_MOVE_SET_WALK_MODE` (0x39F2/0x39F3) | C→S | id. |
| `CMSG_MOVE_FALL_LAND` (0x39FB) | C→S | id. (calc fall damage) |
| `CMSG_MOVE_START_SWIM` / `CMSG_MOVE_STOP_SWIM` (0x39FC/0x39FD) | C→S | id. |
| `CMSG_MOVE_HEARTBEAT` (0x3A10) | C→S | id. |
| `CMSG_MOVE_SET_FACING` / `CMSG_MOVE_SET_FACING_HEARTBEAT` (0x3A09/0x3A5F) | C→S | id. |
| `CMSG_MOVE_SET_PITCH` (0x3A0A) | C→S | id. |
| `CMSG_MOVE_FORCE_*_SPEED_CHANGE_ACK` (0x3A0B–0x3A0E, 0x3A21–0x3A2E…) | C→S | acks de speed change |
| `CMSG_MOVE_FORCE_ROOT_ACK` / `CMSG_MOVE_FORCE_UNROOT_ACK` (0x3A0E/0x3A0F) | C→S | acks |
| `CMSG_MOVE_KNOCK_BACK_ACK` (0x3A12) | C→S | spell knockback ack |
| `CMSG_MOVE_HOVER_ACK` (0x3A13) | C→S | ack |
| `CMSG_MOVE_FALL_RESET` (0x3A19) | C→S | fall counter reset |
| `CMSG_MOVE_UPDATE_FALL_SPEED` (0x3A1A) | C→S | id. |
| `CMSG_MOVE_FEATHER_FALL_ACK` (0x3A1C) | C→S | ack slow-fall |
| `CMSG_MOVE_SPLINE_DONE` (0x3A18) | C→S | ack server-driven spline complete |
| `CMSG_MOVE_TELEPORT_ACK` (0x39FA) | C→S | ack to SMSG_MOVE_TELEPORT |
| `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` (0x3A46) | C→S | ack active mover ready |
| `CMSG_MOVE_SET_FLY` (0x3A28) / `CMSG_MOVE_SET_CAN_FLY_ACK` (0x3A27) | C→S | |
| `CMSG_MOVE_START_ASCEND` / `CMSG_MOVE_STOP_ASCEND` (0x3A29/0x3A2A) | C→S | |
| `CMSG_MOVE_START_DESCEND` (0x3A30) | C→S | |
| `CMSG_TIME_SYNC_RESPONSE` (0x3A3D) | C→S | reply a SMSG_TIME_SYNC_REQUEST (anti-cheat clock) |
| `CMSG_SET_ACTIVE_MOVER` (0x3A3C) | C→S | who client is moving |
| `CMSG_MOVE_REMOVE_MOVEMENT_FORCES` (0x3A17) | C→S | drop wind forces |
| `SMSG_MOVE_UPDATE` (0x2DE0) | S→C | broadcast posición a observadores |
| `SMSG_MOVE_UPDATE_*_SPEED` (0x2DD6–0x2DDC) | S→C | broadcast speed changes |
| `SMSG_MOVE_UPDATE_KNOCK_BACK` (0x2DE2) | S→C | knockback resolution |
| `SMSG_MOVE_SET_*_SPEED` (0x2DF0–0x2DF8) | S→C | force-set speed |
| `SMSG_MOVE_SPLINE_SET_*_SPEED` (0x2DE7–0x2DEF) | S→C | set speed during spline |
| `SMSG_MOVE_TELEPORT` (0x2E04) | S→C | force teleport (intra-map) |
| `SMSG_NEW_WORLD` / `SMSG_TRANSFER_PENDING` | S→C | inter-map teleport flow |
| `SMSG_ON_MONSTER_MOVE` | S→C | mover NPC con spline (puntos packed o uncompressed) |
| `SMSG_FLIGHT_SPLINE_SYNC` (0x2E2B) | S→C | progreso de taxi flight |
| `SMSG_MOVE_SPLINE_DISABLE/ENABLE_GRAVITY/COLLISION` (0x2E1B–0x2E1E) | S→C | spline-time toggles |
| `SMSG_MOVE_SET_ACTIVE_MOVER` (0x2DD5) | S→C | server tells client which unit it controls |
| `SMSG_MOVE_SET_HOVERING` / `SMSG_MOVE_SET_LAND_WALK` / `SMSG_MOVE_SET_FEATHER_FALL` (0x2E01/0x2DFE/0x2DFF) | S→C | flag toggles |
| `SMSG_BIND_POINT_UPDATE` (0x257D) | S→C | update hearthstone bind |
| `SMSG_TIME_SYNC_REQUEST` | S→C | anti-cheat clock probe |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-recastdetour` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |
| `crates/wow-movement` | `crate_dir` | 2 | active | `exists_active` | crate exists; first spline runtime core added in `#A06.8h.3a` |
| `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |
| `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `crates/wow-recastdetour/src/lib.rs` | `file` | 1 | 0 | `exists_empty` | file exists but has 0 lines |
| `crates/wow-world/src/handlers/misc.rs` | `file` | 1 | 661 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-packet/src/packets/movement.rs` — 461 líneas — cubre ~5–8% del C++ (sólo MovementInfo R/W, MoveUpdate, MonsterMove single-segment, SetActiveMover, MoveInitActiveMoverComplete).
- `crates/wow-world/src/handlers/movement.rs` — 204 líneas — 28 opcodes CMSG_MOVE_* registrados; un único `handle_movement` que parsea + valida GUID + actualiza `player_position` + broadcast.
- `crates/wow-recastdetour/src/lib.rs` — 0 líneas (solo `Cargo.toml`) — placeholder, sin FFI bindings reales aún.
- `crates/wow-movement/src/spline.rs` — núcleo inicial de `MoveSpline` contrastado contra C++: flags, args, fall/parabolic math, CatmullRom-compatible storage, duración, update/finalize.
- `crates/wow-world/src/handlers/misc.rs` — TimeSyncResponse handler (anti-cheat clock skeleton).
- No hay MotionMaster, no hay MovementGenerator trait, no hay PathGenerator, no hay WaypointManager; el `MoveSpline` real existe como núcleo aislado y aún no sustituye al shell runtime de Unit.

**What's implemented:**
- Parsing completo de `MovementInfo` (incluyendo bits de transport, fall, inertia, adv_flying, jump direction).
- Serialización de `MovementInfo` para SMSG_MOVE_UPDATE (broadcast a otros sesion-holders mismo map_id).
- `SMSG_ON_MONSTER_MOVE` DTO C++-like para destino, tolerancia, flags, elapsed/moveTime/fade/mode/transport/seat, counts bit-packed, face payload, puntos y packed deltas; todavía no está alimentado por un `MoveSpline` real conectado a Unit.
- `crates/wow-movement` con primer núcleo `MoveSpline`: `MoveSplineFlag`, `MoveSplineInitArgs::Validate`, fall/parabolic math, almacenamiento lineal/CatmullRom-compatible, duración, `ComputePosition`, `updateState`, cyclic wrap y `Finalize`.
- Validación anti-cheat trivial (GUID match + posición finita).
- Update de `player_position` en `WorldSession`; sync con `PlayerBroadcastInfo` registry.
- Hook a `update_visibility()` y `check_area_triggers()` en el handler.
- Stub `SetActiveMover` y `MoveInitActiveMoverComplete` (solo logging).

**What's missing vs C++:**
- **MoveSpline conectado a Unit** — el núcleo aislado existe, pero no está integrado en `Unit::Update`, `MoveSplineState`, packet mapper productivo ni transport motion; faltan extras, `MoveSplineInit::Launch/Stop` y lifecycle completo.
- **MotionMaster + MovementGenerator stack** — creature movement no tiene driver; no hay slots, no hay Update() per-tick.
- **PathGenerator (Detour)** — el crate `wow-recastdetour` está vacío. Sin pathfinding real, sin NavMesh load, sin smooth path.
- **WaypointManager** — sin loader de `waypoint_path*`.
- **Generators**: Idle/Random/Waypoint/Follow/Chase/Flee/Confused/Home/Point/FlightPath/Formation/SplineChain/Generic — ninguno existe.
- **Speed change pipeline** — sin SMSG_MOVE_SET_RUN_SPEED/etc., sin SMSG_MOVE_FORCE_*; cliente nunca recibe ajustes.
- **Teleport flow** — SMSG_MOVE_TELEPORT, SMSG_NEW_WORLD, SMSG_TRANSFER_PENDING no implementados (existe un stub básico para login pero no inter-map).
- **Anti-cheat** — sólo GUID match. Falta: speed-cap por flag, time-sync drift, height delta, fall-time validation, root/stun ignored, fly-while-not-allowed.
- **Transport relocation** — el packet parser lee transport info pero el server no actualiza posición relativa; transports inexistentes.
- **Knockback** — sin SMSG_MOVE_UPDATE_KNOCK_BACK, sin SetParabolic.
- **Charge / Jump server-driven** — sólo el cliente reporta jumps; server no inicia jumps.
- **Taxi flight** — sin FlightPathMovementGenerator, sin TaxiPath DBC integration.
- **Confused / Fleeing / Stunned movement** — sin generators de CC.
- **Spline chain** — sin soporte para encadenado boss-script.
- **Formations** — sin formation leader/follower offsets.
- **Time-sync** — el ack se procesa pero no se mantiene drift histórico.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- El `MovementInfo::write()` en Rust solo pone `has_spline=false`, lo que hará que clientes ignoren info de spline si añadimos creatures con spline simultáneo (probable mismatch al implementar spline).
- El parser asume `flush_bits` después del bloque inicial de bits; contrastar el flush con `MovementPackets.cpp` y `ByteBuffer.h` canónicos; este registro no demuestra esa rama.
- `MoveUpdate` reusa `MovementInfo` write directamente — falta el extra player GUID prefix que el cliente espera para broadcasts de otros (revisar contra MovementPackets.cpp `MoveUpdate::Write`).
- Falta detección de `MovementFlag::DISABLE_GRAVITY`, `WATERWALKING`, `HOVER` en server-side state — asumimos cliente honesto.
- La validación de posición sólo filtra NaN/Inf. C++ TrinityCore corre `MovementAnticheat` con tolerancias de speed/distance/jump-arc.

**Tests existing:**
- 0 tests específicos de movement en `crates/wow-packet/src/packets/movement.rs` (no hay `#[cfg(test)]`).
- 0 tests de handler.
- Hay tests de packet encoding genéricos en `wow-packet` pero no cubren MovementInfo round-trip.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MOVEMENT.WBS.001** Cerrar la migracion auditada de `game/Movement/AbstractFollower.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/AbstractFollower.cpp`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.002** Cerrar la migracion auditada de `game/Movement/AbstractFollower.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/AbstractFollower.h`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.003** Partir y cerrar la migracion auditada de `game/Movement/MotionMaster.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1376 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.004** Cerrar la migracion auditada de `game/Movement/MotionMaster.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.005** Cerrar la migracion auditada de `game/Movement/MovementDefines.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementDefines.cpp`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.006** Cerrar la migracion auditada de `game/Movement/MovementDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementDefines.h`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.007** Cerrar la migracion auditada de `game/Movement/MovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.cpp`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.008** Cerrar la migracion auditada de `game/Movement/MovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT.WBS.009** Cerrar la migracion auditada de `game/Movement/enuminfo_MovementDefines.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/enuminfo_MovementDefines.cpp`
  Rust target: `crates/wow-packet`, `crates/wow-world`, `crates/wow-recastdetour`, `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera para referencia desde `MIGRATION_ROADMAP.md` §5.
Complejidad: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, splitear).

- [ ] **#MOVE.1** Round-trip test de `MovementInfo::read/write` con vector de bytes capturado de cliente real (todos los bits combos: transport+fall+inertia+adv_flying). (M)
- [ ] **#MOVE.2** Crear crate `wow-movement` con módulos `spline`, `motion_master`, `generators`, `path` (esqueletos). (L)
- [ ] **#MOVE.3** Portar `MoveSplineFlag` (bitfield 32 bits) idéntico a C++ con tests de bit positions. (M)
- [ ] **#MOVE.4** Portar `Spline<T>` (linear + catmullrom) con parametric length lookup; fuzz contra coordenadas de C++. (H)
- [ ] **#MOVE.5** Portar `MoveSpline` (state machine: time_passed/point_Idx/Result_*) — minimal sin parabolic/fall. (H)
- [ ] **#MOVE.6** Añadir parabolic + fall elevation (`computeFallTime`, `computeFallElevation`). (M)
- [ ] **#MOVE.7** Implementar `MoveSplineInit` builder API (`MoveTo`, `SetWalk`, `SetFly`, `SetCyclic`, `SetParabolic`, `SetVelocity`, `Launch`). (M)
- [ ] **#MOVE.8** Trait `MovementGenerator` (Initialize/Reset/Update/Deactivate/Finalize) + flags. (M)
- [ ] **#MOVE.9** `MotionMaster` con stack por slot (Default/Active/Controlled), Update per-tick, delayed actions. (H)
- [ ] **#MOVE.10** Generators básicos: `IdleMovementGenerator`, `PointMovementGenerator`. (M)
- [ ] **#MOVE.11** `RandomMovementGenerator` con wander radius + delays. (M)
- [ ] **#MOVE.12** `FollowMovementGenerator` + `ChaseMovementGenerator` con `ChaseRange`/`ChaseAngle`. (H)
- [ ] **#MOVE.13** `FleeingMovementGenerator` + `ConfusedMovementGenerator`. (M)
- [ ] **#MOVE.14** `HomeMovementGenerator` ligado al evade-flow del AI. (M)
- [ ] **#MOVE.15** `WaypointMovementGenerator` + `WaypointManager` (loader SQL `waypoint_path*`). (H)
- [ ] **#MOVE.16** `FlightPathMovementGenerator` + integración `TaxiPath.dbc`. (H)
- [ ] **#MOVE.17** `FormationMovementGenerator` + tabla `creature_formations`. (M)
- [ ] **#MOVE.18** `SplineChainMovementGenerator` + tabla `spline_chain`. (M)
- [ ] **#MOVE.19** Fleshen out `wow-recastdetour`: linkear `recastnavigation` C library; expose dtNavMesh + dtNavMeshQuery. (XL)
- [ ] **#MOVE.20** Portar `PathGenerator::CalculatePath` (BuildPolyPath + FindSmoothPath + NormalizePath + AddFarFromPolyFlags + ShortenPathUntilDist). (XL)
- [ ] **#MOVE.21** MMaps loader: cargar tiles `mmaps/{mapid:03}{tx:02}{ty:02}.mmtile` desde disk. (H)
- [ ] **#MOVE.22** Server-side anti-cheat: speed-cap por flag, fly-flag check, root/stun checks, time-sync drift. (H)
- [ ] **#MOVE.23** SMSG_MOVE_TELEPORT + flow CMSG_MOVE_TELEPORT_ACK. (M)
- [ ] **#MOVE.24** SMSG_MOVE_SET_*_SPEED + SMSG_MOVE_SPLINE_SET_*_SPEED + acks (run/walk/swim/fly/turn/pitch). (H)
- [ ] **#MOVE.25** Knockback flow: SMSG_MOVE_UPDATE_KNOCK_BACK + ack. (M)
- [ ] **#MOVE.26** SMSG_ON_MONSTER_MOVE completo: facing types (None/Spot/Target/Angle), packed deltas, parabolic extra, anim transition, jump extra, fade time. (H)
- [ ] **#MOVE.27** Integrar visibility recompute server-side al `MoveSpline::ComputePosition` durante traversal de creature (no sólo al fin). (M)
- [ ] **#MOVE.28** Transport relocation: TransportPathTransform + sync de `transport_offset` en cada CMSG_MOVE_* y broadcast. (H)
- [ ] **#MOVE.29** Fall damage compute desde `CMSG_MOVE_FALL_LAND` (Movement::computeFallDamage equivalent). (M)
- [ ] **#MOVE.30** Time-sync: SMSG_TIME_SYNC_REQUEST cada 10s + drift tracking. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MOVEMENT.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 9 files / 2209 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h`. Rust target: `crates/wow-packet`, `crates/wow-recastdetour`, `crates/wow-world`. | `cargo test -p wow-packet && cargo test -p wow-recastdetour && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MOVEMENT.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 9 files / 2209 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h`. Rust target: `crates/wow-packet`, `crates/wow-recastdetour`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MOVEMENT.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 9 files / 2209 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h`. Rust target: `crates/wow-packet`, `crates/wow-recastdetour`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MOVEMENT.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 9 files / 2209 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h`. Rust target: `crates/wow-packet`, `crates/wow-recastdetour`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `MovementInfo` round-trip — escribir + leer una `MovementInfo` con todos los flags (transport+fall+inertia+adv_flying) y comparar campo a campo.
- [ ] Test: `MoveSplineFlag` bit positions match C++ (Done=0x100, Cyclic=0x800, UncompressedPath=0x400, Falling=0x2000, Flying=…). Hardcodear hex y verificar.
- [ ] Test: `Spline::evaluate(t=0)` == primer punto, `evaluate(t=length)` == último punto, monotonicidad de `length(idx)`.
- [ ] Test: `MoveSpline::updateState` con duración 1000ms emite `Result_Arrived` después de exactamente 1000ms acumulados.
- [ ] Test: Catmullrom con 4 puntos colineales reproduce línea recta (tolerancia 1e-3).
- [ ] Test: Parabolic — para amplitude=10, time_shift=0, half-time da elevation=10 (gravity-free formula).
- [ ] Test: Fall — `computeFallTime(yards=20, false)` ≈ valor del C++ (`MovementUtil::computeFallTime`).
- [ ] Test: `MotionMaster::MoveIdle` después de `MovePoint` + arrival → vuelve a IDLE en slot DEFAULT.
- [ ] Test: `RandomMovementGenerator` no se sale de `wanderDistance` (radius check).
- [ ] Test: `ChaseMovementGenerator` mantiene `ChaseRange` ± tolerance al moverse el target.
- [ ] Test: `WaypointManager::LoadPaths` parsea row con MoveType=WALK + delay=2000 + eventId.
- [ ] Test: `PathGenerator::CalculatePath` en map sin mmaps → `PATHFIND_SHORTCUT` + path = [start, end].
- [ ] Test: `PathGenerator::CalculatePath` con mmaps cargado → `PATHFIND_NORMAL` + path no cruza obstáculos (snapshot test).
- [ ] Test: Anti-cheat — CMSG_MOVE_HEARTBEAT con velocidad > maxRunSpeed×1.1 es rechazado/log-warn.
- [ ] Test: Anti-cheat — CMSG_MOVE_SET_FLY sin aura de fly válido → kick o ignore.
- [ ] Test: SMSG_MOVE_UPDATE round-trip con un cliente mock (decodifica posición correctamente).
- [ ] Test: SMSG_ON_MONSTER_MOVE con cyclic flag — cliente reproduce loop infinito (visual smoke-test manual con captura).
- [ ] Test: Transport — CMSG_MOVE_HEARTBEAT con `has_transport=true` actualiza el offset, no la posición global.
- [ ] Test: Time-sync — drift > 250ms produce kick (matching C++ behavior).
- [ ] Test: Speed change — SMSG_MOVE_SET_RUN_SPEED + ack pendiente bloquea segundo cambio hasta ack recibido.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 9 files / 2209 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h` | `crates/wow-packet/` (packets), `crates/wow-world/` (handlers + per-session state), `crates/wow-recastdetour/` (Detour FFI scaffold), future `crates/wow-movement/` \| ⚠️ partial — solo parsing CMSG_MOVE_*, broadcast SMSG_MOVE_UPDATE y posición server-side. Sin spline real, sin pathfinding, sin generators. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MOVEMENT.DIV.001` | `crates/wow-recastdetour` (`exists_empty`, 0 Rust lines) | 9 C++ files / 2209 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |
| `#MOVEMENT.DIV.002` | `crates/wow-movement` (`missing_declared_path`, 0 Rust lines) | 9 C++ files / 2209 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#MOVEMENT.DIV.003` | `crates/wow-recastdetour/src/lib.rs` (`exists_empty`, 0 Rust lines) | 9 C++ files / 2209 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.h`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerator.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. file exists but has 0 lines |

<!-- REFINE.023:END known-divergences -->

- **Anti-cheat es crítico**: clientes hackeados envían CMSG_MOVE_* con velocidades infladas o teleports. C++ tiene `MovementAnticheat` que valida cada packet contra el *último* recibido (delta-time vs delta-position). En Rust hoy no existe — implementar antes de exponer servidor a Internet.
- **Spline interpolation tiene 3 modos**: linear (default), catmullrom (smooth), bezier (raro, casi sólo cinematics). C++ marca el modo en `MoveSplineFlag::CatmullRom`. La diferencia entre linear y catmullrom es visible: clientes verán "snapping" si el server reporta linear pero el AI script esperaba smooth.
- **Jump validation**: `CMSG_MOVE_JUMP` lleva `z_speed` (vertical) y opcional `xy_speed + sin_angle + cos_angle` (horizontal direction). Fall damage se calcula desde `CMSG_MOVE_FALL_LAND.fall_time`; un cliente puede mentir el fall_time para evitar daño — C++ lo verifica contra apex calculado desde el jump z_speed previo.
- **Transport relocation**: la posición global del player es `transport.global_pos + offset_in_local`. Si el server no actualiza `offset_in_local` desde packets, la visibilidad/aggro estará off por toda la longitud del transport. Bug histórico de muchos servers.
- **MMaps tile size = 533.333 yards** (1/3 del map grid de 1600 yards). Si Pathfinding cruza tiles no cargados, `PathGenerator` devuelve `PATHFIND_INCOMPLETE`. Recargar tiles dinámicamente en server.
- **`MovementGeneratorMode`** distingue `DEFAULT` (persistente, idle) de `ACTIVE` (chase/flee/etc., temporal). Removerlo con la slot equivocada hace que el creature se quede congelado.
- **`MOTIONMASTER_FLAG_UPDATE`** previene mutación durante Update — todas las llamadas `Add/Remove/Clear` durante Update se difieren a un `_delayedActions` queue. Olvidarse de eso causa iterator invalidation crashes.
- **Cyclic splines** (`MoveSplineFlag::Cyclic`) nunca emiten `Result_Arrived`. Si AI espera arrival callback en cyclic → bug.
- **`SetUnlimitedSpeed`** desactiva el cap interno de spline (50.0f para flying, 28.0f ground). Fuera de cinematics, dejarlo sin set.
- **3.4.3 specifics**: WoLK Classic 3.4.3 introdujo `MovementFlags3` (que TrinityCore master no tiene). Adv flying / impulse / inertia son bits añadidos. Verificar contra `wow-constants/movement.rs` MovementFlag3 cuando se vea `has_inertia`/`has_adv_flying`.
- **`MoveSplineInit::Launch` envía SMSG_ON_MONSTER_MOVE inline** — mover esto fuera del Init en Rust (separar building from sending) para poder hacer broadcast batched.
- **`SplineChainMovementGenerator`**: encadena varios MoveSplines con pauses; usado por scripts de boss (Lich King platform tour, etc.). Mantener el `chainId/pointId` consistente para reanudar tras crash.
- **`PathGenerator::ShortenPathUntilDist`** es usado por chase/follow para no hacer overshoot del target — sin esto, los creatures se "pegan" al jugador.
- **Detour filter**: TrinityCore configura `dtQueryFilter` con `NavTerrain` mask (ground/water/magma/slime). Si copiamos sin esto, los creatures atravesarían lava.
- **`time_passed` overflow**: `int32` da ~24 días de spline duration. Cyclic splines necesitan reset al cumplir un ciclo (C++ lo hace en `_updateState`).

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Movement::MoveSpline` | `struct MoveSpline` (en `crates/wow-movement/src/spline/move_spline.rs`) | Owned by Unit; sin herencia de Spline (composición) |
| `class Movement::Spline<int32>` | `struct Spline { mode: SplineMode, points: Vec<Vector3>, lengths: Vec<i32> }` | Genérico → enum SplineMode (Linear/CatmullRom/Bezier) |
| `Movement::MoveSplineFlag` | `bitflags! struct MoveSplineFlag: u32` | Mapeado bit-a-bit; tests con valores hex de C++ |
| `class Movement::MoveSplineInit` | `struct MoveSplineInit<'a> { args: MoveSplineInitArgs, unit: &'a mut Unit }` | Builder consumible: `Launch(self) -> i32` |
| `class MovementGenerator` (abstract) | `trait MovementGenerator` (Initialize/Reset/Update/Deactivate/Finalize) | Sin CRTP; trait objects en MotionMaster |
| `MovementGeneratorMedium<T,D>` | (no necesario — trait + impls concretos) | CRTP innecesario en Rust |
| `class MotionMaster` | `struct MotionMaster { slots: [Vec<Box<dyn MovementGenerator>>; 3], flags: MotionFlags, delayed: VecDeque<DelayedAction> }` | Stack per slot |
| `MovementGeneratorType` | `#[repr(u8)] enum MovementGeneratorType { Idle, Random, Waypoint, … }` | Mismo orden que C++ |
| `MovementSlot` | `enum MovementSlot { Default, Active, Controlled }` | — |
| `class PathGenerator` | `struct PathGenerator<'a> { source: &'a WorldObject, mesh: &'a NavMesh, … }` | Recompute on demand; reuse buffer |
| `dtNavMesh*` / `dtNavMeshQuery*` | `Pin<Box<DetourNavMesh>>` (FFI), `DetourNavMeshQuery` wrapping `*mut dtNavMeshQuery` | Vía `wow-recastdetour` cxx-bridge |
| `class WaypointManager` (singleton) | `static WAYPOINT_MGR: OnceCell<WaypointManager>` con `HashMap<u32, WaypointPath>` | DashMap si concurrente |
| `struct WaypointNode/WaypointPath` | `struct WaypointNode { id, x, y, z, orientation, delay, move_type, event_id }` / `WaypointPath { id, name, move_type, kind, repeat, dir, nodes: Vec<WaypointNode> }` | POD-like |
| `MovementInfo` | `struct MovementInfo { ... }` (ya existe en `crates/wow-packet/src/packets/movement.rs`) | Ya cubierto |
| `WorldPackets::Movement::MonsterMove` | `struct MonsterMove { … }` (ya existe) | Falta extender a packed deltas + facing types |
| `WorldPackets::Movement::MoveTeleport` | TODO `struct MoveTeleport` | Implementar |
| `class TransportPathTransform` | `fn transform_for_transport(owner: &Unit, point: Vector3) -> Vector3` | Plain function |
| `Optional<SpellEffectExtraData>` | `Option<SpellEffectExtraData>` | — |
| `std::deque<DelayedAction>` | `std::collections::VecDeque<DelayedAction>` | — |
| `ASSERT(Initialized())` | `debug_assert!(self.initialized())` | — |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-11.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked C++ canonical sources at `/home/server/woltk-trinity-legacy/src/server/game/Movement/` (MotionMaster + 13 generators + Spline subsystem + PathGenerator + Waypoints + MovementPackets) against the Rust workspace at `/home/server/rustycore/crates/`.

**Historical empty-crate finding, updated 2026-05-11.** The original 2026-05-01 audit found no `crates/wow-movement/` crate. `#A06.8h.3a` now creates it and ports the first isolated `MoveSpline` core, but `wow-recastdetour` is still empty and the spline/generators/pathfinding subsystem remains incomplete until Unit/MotionMaster/pathgen are connected. The old finding is no longer "missing crate"; it is now "runtime not integrated".
- `crates/wow-packet/src/packets/movement.rs` (461 lines, parser + writer for `MovementInfo`).
- `crates/wow-world/src/handlers/movement.rs` (204 lines, single `handle_movement` for ~28 CMSG_MOVE_* opcodes).

**MovementGenerator subclass count.** C++ ships **13 concrete subclasses** under `Movement/MovementGenerators/` (Idle, Random, Waypoint, Confused, Chase, Home, Flight, Point, Fleeing, Formation, SplineChain, Generic, Follow) plus the `PathMovementBase` helper. **Rust ships 0 generators** — there is no `MovementGenerator` trait, no `MotionMaster` struct, no slot stack, no `Update` per-tick. Creature movement is driven by the legacy `WorldCreature` wandering inside `wow-ai` (a single `pick_wander_destination` linear-tween), which means there is no chase-on-aggro, no flee, no waypoint patrol, no formation, no taxi flight, no splined boss tour.

**Spline movement.** **Rust now has an isolated core, not a complete runtime.** `crates/wow-movement` ports `MoveSplineFlag`, `MoveSplineInitArgs`, fall/parabolic math and core `MoveSpline` state/update/position logic. `SMSG_ON_MONSTER_MOVE` also has a C++-like DTO. Missing pieces remain large: no `MoveSplineInit::Launch/Stop`, no Unit-owned live spline update/relocation, no MotionMaster/generators, no packet mapper from real spline extras, no transport transform and no pathgen.

**Speed packets / FallToGround / JumpExtraData.** All three are **missing on the send side**. Opcodes for `SMSG_MOVE_SET_RUN_SPEED`, `SMSG_MOVE_SPLINE_SET_RUN_SPEED`, `SMSG_MOVE_UPDATE_KNOCK_BACK`, `SMSG_MOVE_TELEPORT`, `SMSG_FLIGHT_SPLINE_SYNC` are listed in `crates/wow-constants/src/opcodes.rs` (verified — `MoveSplineSetRunSpeed = 0x2de7`, etc.) but **no writers and no callers** exist anywhere in the workspace. `CMSG_MOVE_FALL_LAND` is parsed via the generic `MovementInfo` reader but no fall-damage compute is wired (would be `Movement::computeFallDamage`). `CMSG_MOVE_JUMP` is likewise just deserialized into `MovementInfo` — there is no anti-cheat against jump apex/`z_speed` and no server-driven jump emission (`MoveSplineInit::SetParabolic`).

**Anti-cheat surface.** C++ runs `MovementAnticheat` on every CMSG with delta-time/delta-position/speed-cap/fly-flag/root-state checks. Rust validates only `GUID match + position is finite`. Time-sync ack is parsed but no drift table is kept. Transport offset is parsed but never re-broadcast — global position is taken at face value.

**Worst divergence.** **There is still no integrated server-side motion engine.** Every NPC visible to a player still depends on represented wander until the new `wow-movement` core is wired into Unit/MotionMaster/generators. The moment a designer needs chase, flee, return home, taxi route or scripted waypoint movement, there is still no product API to call. The gap is reduced from "no movement crate" to "movement crate foundation exists, runtime integration absent".
