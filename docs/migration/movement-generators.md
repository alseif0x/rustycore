# Migration: Movement / Generators (MotionMaster + 13 generators)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Movement/MotionMaster.{h,cpp}` + `src/server/game/Movement/MovementGenerator.{h,cpp}` + `src/server/game/Movement/MovementGenerators/*.{h,cpp}` + `src/server/game/Movement/AbstractFollower.{h,cpp}` + `src/server/game/Movement/MovementDefines.{h,cpp}`
> **Rust target crate(s):** future `crates/wow-movement/src/motion_master.rs` + `crates/wow-movement/src/generators/`
> **Layer:** L5 sub-module (depends on Spline L5, PathGen L5, Entities L4, AI L6)
> **Status:** ⚠️ partial — structural `MotionSubsystem` exists in `wow-entities`; first represented `PointMovementGenerator` state bridge exists, but executable generators are not fully ported
> **Audited vs C++:** ✅ complete 2026-05-01
> **Last updated:** 2026-05-11
> **Contrast rule:** primary source is `/home/server/woltk-trinity-legacy`; if that fork is incomplete or suspect, use `/home/server/archived/woltk-trinity-core` only to validate 3.3.5 TrinityCore logic, and document the fallback explicitly.

> Sub-doc of [`movement.md`](movement.md). Cross-links: [`movement-spline.md`](movement-spline.md) (consumed by every generator that drives a `MoveSplineInit`), [`movement-pathgen.md`](movement-pathgen.md) (called by chase/follow/point/waypoint to compute walkable paths), [`common-collision.md`](common-collision.md) (height/LOS clamps used by random/wander), [`ai-base.md`](ai-base.md) (CreatureAI is the primary caller of `MotionMaster::Move*`).

---

## 1. Purpose

Drive **what a Unit decides to do next**: idle, wander, patrol a waypoint path, chase a hostile target, follow a friendly target, flee in fear, return home after evade, fly a taxi, hop along a script-defined spline chain, anchor to a formation leader, or run a one-shot custom spline. `MotionMaster` is the per-Unit stack of `MovementGenerator` instances split into priority slots; the top-of-stack generator owns the Unit's motion until it expires or is interrupted. Every generator ultimately produces a `MoveSpline` (see [`movement-spline.md`](movement-spline.md)) and emits the matching client packet via `MoveSplineInit::Launch`.

Current Rust status:
- `crates/wow-entities/src/unit_subsystems.rs` contains a structural `MotionSubsystem` with Trinity generator ids, slots, priorities, base-unit-state accounting, `move_point`, `move_charge`, `move_follow`, stop-on-death, and represented spline progress.
- `#A06.8h.3e.1` ports the first C++ state side effects for represented point/spline motion: `move_point` stores `POINT_MOTION_TYPE` with `UNIT_STATE_ROAMING`, and `WorldCreature` marks `UNIT_STATE_ROAMING_MOVE` while its real `wow_movement::MoveSpline` is active.
- `#A06.8h.3e.2` ports the pure jump speed/parabola helpers used by `MotionMaster::MoveJump` and `MotionMaster::CalculateJumpSpeeds`; callers still need to supply real Unit speeds and movement type.
- `#A06.8h.3e.3` ports represented `GenericMovementGenerator` lifecycle state: constructor fields, base unit state, duration/cyclic update rules, deactivation/finalization, inform payload and arrival-spell metadata.
- `#A06.8h.3e.4` ports represented `MotionMaster::LaunchMoveSpline`, `MoveJump` and `MoveJumpWithGravity` wrapper semantics: invalid generator type rejection, generic effect movement creation, highest jump priority, `UNIT_STATE_JUMPING`, arrival-spell metadata and gravity-jump persist-on-death.
- `#A06.8h.3e.5` ports represented `MoveKnockbackFrom` and `MoveFall` wrapper semantics: player/speed/height/root guards, generic effect movement creation, highest priority, knockback persist-on-death and player fall-info branch.
- `#A06.8h.3e.6` ports represented `PointMovementGenerator` lifecycle semantics: initialization/blocked movement/interruption/speed-update/finalize flags, `UNIT_STATE_ROAMING_MOVE` actions, and `EVENT_CHARGE_PREPATH` informing scripts as `EVENT_CHARGE`.
- `#A06.8h.3e.7` connects direct non-pathgen point movement to the represented `WorldCreature` runtime: initialize a point generator, mark `UNIT_STATE_ROAMING_MOVE`, launch the existing real `MoveSplineInit` bridge for direct destinations, preserve the C++ `EVENT_CHARGE_PREPATH` no-launch branch, and record `MovementInform(POINT,id)` on canonical creature AI state during finalize.
- `#A06.8h.3e.8` ports represented assistance movement semantics: `MoveSeekAssistance` stop/react-passive side effects, `EVENT_ASSIST_MOVE`, `AssistanceMovementGenerator::Finalize` side-effect plan, `CreatureFamilyAssistanceDelay=1500`, and `AssistanceDistractMovementGenerator` return-to-aggressive finalization.
- `#A06.8h.3e.9` ports represented idle/rotate/distract semantics: idle default priority/flags and stop action, rotate constructor/update/finalize rules, and distract constructor/init/update/finalize rules.
- `#A06.8h.3e.10` connects represented creature distract/rotate to real facing-only `MoveSplineInit` launches (`MoveTo(current position)` + `SetFacing(angle)`), preserves C++'s stationary facing spline shape, mutates real `UnitData::StandState` to stand on distract initialize, applies distract final home orientation, and records rotate `MovementInform(ROTATE,id)` on canonical creature AI state.
- `#A06.8h.3e.13` adds a represented `MotionMaster::Update` driver over `MotionSubsystem`: C++ stall flags, `UPDATE` guard, top initialize/reset/update, natural pop, and end-of-tick delayed action drain.
- `#A06.8h.3e.14` ports pure `MovementDefines` chase helpers into `wow-movement`: `ChaseRange`, `ChaseAngle`, C++ contact-distance tolerances and angle wrapping.
- `#A06.8h.3e.15` ports pure jump/charge helper data into `wow-movement`: `JumpArrivalCastArgs`, `JumpChargeParams`, and a tagged replacement for the C++ speed/move-time union.
- `#A06.8h.3e.16` adds the first runtime skeleton modules in `wow-movement`: `generator.rs` with C++ type/mode/priority/slot/flag values and a `MovementGenerator` trait, plus `motion_master.rs` with flags, delayed action ids and FIFO closure queue.
- `#A06.8h.3e.17` ports runtime `IdleMovementGenerator` into `wow-movement`; owner `StopMoving` is represented as an observable call counter until the trait receives a real `Unit` context.
- `#A06.8h.3e.18` ports runtime `RotateMovementGenerator` into `wow-movement`: exact `RotateDirection` ids, constructor/lifecycle flags, `UNIT_STATE_ROTATING`, C++ facing-angle math, duration/inform flag behavior, transport-path-transform output, and represented rotate `MovementInform`.
- `#A06.8h.3e.19` ports runtime `DistractMovementGenerator` and `AssistanceDistractMovementGenerator` into `wow-movement`: exact priorities, `UNIT_STATE_DISTRACTED`, stand-up/facing-spline initialization, strict `diff > timer` completion, home-orientation return, and assistance return-to-aggressive action.
- `#A06.8h.3e.20` adds the first runtime `MotionMaster` in `wow-movement`: default generator + active generators ordered by priority, delayed add/remove/clear/filter actions, top initialize/reset/update/pop flow, and represented `BaseUnitState` ref counting.
- `#A06.8h.3e.21` ports runtime `GenericMovementGenerator` into `wow-movement`: executable `FnOnce(&mut MoveSplineInit)` launch against `MoveSpline`, C++ duration/cyclic/finalized update rules, no-resume deactivation behavior, and represented arrival spell + movement inform output.
- `#A06.8h.3e.22` ports the direct runtime shape of `PointMovementGenerator` and `AssistanceMovementGenerator` into `wow-movement`: constructor flags/base state, `EVENT_CHARGE_PREPATH`, no-move/casting interruption, `UNIT_STATE_ROAMING_MOVE`, direct `MoveSplineInit` launch with speed/facing/final orientation/spell extras/close-enough, relaunch actions, point inform and assistance side-effect plan.
- `#A06.8h.3e.23` ports `AbstractFollower` and represented runtime `FollowMovementGenerator` into `wow-movement`: follower target add/remove events, constructor/init/reset/update/deactivate/finalize flags, duration/check timers, `PositionOkay`, angle selection, follow move state, target-counter inform and pet-speed side-effect counters.
- `#A06.8h.3e.24` ports represented runtime `ChaseMovementGenerator` into `wow-movement`: range-check timer, min/max/angle/LOS `PositionOkay`, mutual chase handling, lost-target/no-move/casting stops, chase move state, cannot-reach plan, walk-mode selection and target-counter inform.
- `#A06.8h.3e.25` ports represented runtime `FleeingMovementGenerator` and `TimedFleeingMovementGenerator` into `wow-movement`: highest priority fleeing state, `UNIT_FLAG_FLEEING`, quiet-distance destination math, LOS/path retry timers, path length limit, fleeing move state, random travel delay bounds, speed-update relaunch, Player/Creature finalization differences and timed-flee inform output.
- `#A06.8h.3e.26` ports represented runtime `ConfusedMovementGenerator` into `wow-movement`: highest priority confused state, `UNIT_FLAG_CONFUSED`, initial reference capture, stop-on-initialize, C++ short-hop random destination math, LOS/path retry timers, path length limit, walk launch plan, random travel delay bounds, speed-update relaunch and Player/Creature finalization differences.
- `#A06.8h.3e.27` ports represented runtime `HomeMovementGenerator<Creature>` into `wow-movement`: normal priority roaming state, no-search-assistance reset, root/stunned/distracted interruption, erasable-state cleanup preserving evade until finalize, run-to-home launch plan with home facing, update/finalize inform gating and represented `JustReachedHome` side effects.
- `#A06.8h.3e.28` ports represented runtime `RandomMovementGenerator<Creature>` into `wow-movement`: pause/resume flags, optional duration, initial reference capture, owner fallback wander distance, 2..10 step cycles, random destination shape, LOS/path retry timers, `FARFROMPOLY` allowance, walk/run selection, 4..10s rest pauses, formation signal and represented `MovementInform(RANDOM,0)`.
- `#A06.8h.3e.29` ports represented runtime `PathMovementBase`/`WaypointMovementGenerator<Creature>` into `wow-movement`: path/current-node state, DB/path constructors, pause/resume guard timers, reset-position lookup, C++ delayed initialization quirk, arrival hooks, waypoint AI inform payloads, repeat and backtracking node selection, path-end wait/random behavior, movement launch options, path-ended finalization and represented current-waypoint updates.
- `#A06.8h.3e.30` ports represented runtime `FormationMovementGenerator` into `wow-movement`: `AbstractFollower` target tracking, follow-formation base state, 1200ms relaunch interval, 1.65s leader-spline prediction, waypoint angle flip, predicted-spline stop, formation-move state, arrival facing/inform and finalize/deactivate cleanup.
- `#A06.8h.3e.31` ports represented runtime `FlightPathMovementGenerator` into `wow-movement`: default/highest/in-flight state, taxi/control flags, fly/smooth/uncompressed/walk launch at 32.0, map-end/teleport segmentation, C++ path-shortening, discount costs, taxi-destination switches, departure/arrival events, end-grid preload, teleport resume/skip and active finalize cleanup.
- `#A06.8h.3e.32` ports represented runtime `SplineChainMovementGenerator` into `wow-movement`: chain links, resume info, partial resume, invalid point clamp, `MovebyPath`/`MoveTo` launch selection, duration-adjusted `_msToNext`, update sequencing, resume-info extraction and finalize inform.
- `#A06.8h.3e.33` ports the runtime `MotionMaster::PropagateSpeedChange` and `StopOnDeath` hooks into `wow-movement`: speed changes notify only the current generator like C++, persist-on-death current generators suppress death cleanup, and non-persisting current generators produce clear/default-idle/stop-moving actions.
- Still missing: generalized executable generator behavior against a real `Unit`, pathgen-backed destinations, real SmartAI/script dispatch for `MovementInform`, real `CallAssistance` map/AI effects, SQL-backed spline-chain loading, and full runtime `MotionMaster` ownership outside the represented subsystem.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Movement/MovementGenerators/ChaseMovementGenerator.cpp` | 260 | `prefix` |
| `game/Movement/MovementGenerators/ChaseMovementGenerator.h` | 59 | `prefix` |
| `game/Movement/MovementGenerators/ConfusedMovementGenerator.cpp` | 177 | `prefix` |
| `game/Movement/MovementGenerators/ConfusedMovementGenerator.h` | 49 | `prefix` |
| `game/Movement/MovementGenerators/FleeingMovementGenerator.cpp` | 282 | `prefix` |
| `game/Movement/MovementGenerators/FleeingMovementGenerator.h` | 67 | `prefix` |
| `game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp` | 338 | `prefix` |
| `game/Movement/MovementGenerators/FlightPathMovementGenerator.h` | 75 | `prefix` |
| `game/Movement/MovementGenerators/FollowMovementGenerator.cpp` | 215 | `prefix` |
| `game/Movement/MovementGenerators/FollowMovementGenerator.h` | 62 | `prefix` |
| `game/Movement/MovementGenerators/FormationMovementGenerator.cpp` | 222 | `prefix` |
| `game/Movement/MovementGenerators/FormationMovementGenerator.h` | 57 | `prefix` |
| `game/Movement/MovementGenerators/GenericMovementGenerator.cpp` | 99 | `prefix` |
| `game/Movement/MovementGenerators/GenericMovementGenerator.h` | 55 | `prefix` |
| `game/Movement/MovementGenerators/HomeMovementGenerator.cpp` | 158 | `prefix` |
| `game/Movement/MovementGenerators/HomeMovementGenerator.h` | 41 | `prefix` |
| `game/Movement/MovementGenerators/IdleMovementGenerator.cpp` | 231 | `prefix` |
| `game/Movement/MovementGenerators/IdleMovementGenerator.h` | 81 | `prefix` |
| `game/Movement/MovementGenerators/PathMovementBase.h` | 43 | `prefix` |
| `game/Movement/MovementGenerators/PointMovementGenerator.cpp` | 227 | `prefix` |
| `game/Movement/MovementGenerators/PointMovementGenerator.h` | 75 | `prefix` |
| `game/Movement/MovementGenerators/RandomMovementGenerator.cpp` | 263 | `prefix` |
| `game/Movement/MovementGenerators/RandomMovementGenerator.h` | 58 | `prefix` |
| `game/Movement/MovementGenerators/SplineChainMovementGenerator.cpp` | 238 | `prefix` |
| `game/Movement/MovementGenerators/SplineChainMovementGenerator.h` | 61 | `prefix` |
| `game/Movement/MovementGenerators/WaypointMovementGenerator.cpp` | 469 | `prefix` |
| `game/Movement/MovementGenerators/WaypointMovementGenerator.h` | 93 | `prefix` |
| `game/Movement/Waypoints/WaypointDefines.h` | 86 | `prefix` |
| `game/Movement/Waypoints/WaypointManager.cpp` | 321 | `prefix` |
| `game/Movement/Waypoints/WaypointManager.h` | 73 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Movement/MotionMaster.h` | 246 | Per-Unit slot stack: `_defaultGenerator` + `_generators` multiset, delayed-action queue, public `Move*()` API |
| `src/server/game/Movement/MotionMaster.cpp` | 1376 | Implementation of all `Move*` factories + `Update`/`Add`/`Remove`/`Clear` + `ResolveDelayedActions` |
| `src/server/game/Movement/MovementGenerator.h` | 154 | Abstract base + `MovementGeneratorMedium<T,D>` CRTP + `FactoryHolder` registries (Idle/Random/Waypoint) |
| `src/server/game/Movement/MovementGenerator.cpp` | 61 | Destructor + `GetDebugInfo` |
| `src/server/game/Movement/MovementDefines.h` | 142 | Enums: `MovementGeneratorType` (0..18), `MovementSlot` (DEFAULT/ACTIVE), `MovementGeneratorMode/Priority/Flags`, `RotateDirection`, `ChaseRange`, `ChaseAngle`, `JumpArrivalCastArgs`, `JumpChargeParams` |
| `src/server/game/Movement/MovementDefines.cpp` | 48 | `ChaseAngle` constructor + bound helpers |
| `src/server/game/Movement/AbstractFollower.h` | 36 | Common helper: store target ObjectGuid, detect target moves between updates |
| `src/server/game/Movement/AbstractFollower.cpp` | 31 | Implementation |
| `src/server/game/Movement/MovementGenerators/IdleMovementGenerator.h` | 81 | `IdleMovementGenerator` + `RotateMovementGenerator` + `DistractMovementGenerator` + `AssistanceDistractMovementGenerator` |
| `src/server/game/Movement/MovementGenerators/IdleMovementGenerator.cpp` | 231 | Impls — Idle stops + clears unit state, Rotate ticks orientation, Distract pauses for N ms |
| `src/server/game/Movement/MovementGenerators/RandomMovementGenerator.h` | 58 | `RandomMovementGenerator<Creature>` — wander within radius |
| `src/server/game/Movement/MovementGenerators/RandomMovementGenerator.cpp` | 263 | `_setRandomLocation`, water/oxygen check, delay between hops |
| `src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.h` | 93 | `WaypointMovementGenerator<Creature>` — traverse `waypoint_path`/`script_waypoint` rows |
| `src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp` | 469 | `OnArrived`, `StartMove`, `Pause`/`Resume`, formation hooks, AI script callbacks |
| `src/server/game/Movement/MovementGenerators/ChaseMovementGenerator.h` | 59 | Combat follow with `ChaseRange` + `ChaseAngle` |
| `src/server/game/Movement/MovementGenerators/ChaseMovementGenerator.cpp` | 260 | Reposition logic, predicted target offset, melee leeway |
| `src/server/game/Movement/MovementGenerators/FollowMovementGenerator.h` | 62 | Tail target with offset+angle (non-combat) |
| `src/server/game/Movement/MovementGenerators/FollowMovementGenerator.cpp` | 215 | Pet/companion follow |
| `src/server/game/Movement/MovementGenerators/FleeingMovementGenerator.h` | 67 | Run away from a feared source |
| `src/server/game/Movement/MovementGenerators/FleeingMovementGenerator.cpp` | 282 | Random reachable point in opposite direction, repeat |
| `src/server/game/Movement/MovementGenerators/ConfusedMovementGenerator.h` | 49 | Random short hops (CC effect) |
| `src/server/game/Movement/MovementGenerators/ConfusedMovementGenerator.cpp` | 177 | Bounded short-radius wander; pathfinder validated |
| `src/server/game/Movement/MovementGenerators/HomeMovementGenerator.h` | 41 | Return-to-home after evade |
| `src/server/game/Movement/MovementGenerators/HomeMovementGenerator.cpp` | 158 | Move toward `Creature::GetHomePosition` then despawn-or-restore |
| `src/server/game/Movement/MovementGenerators/PointMovementGenerator.h` | 75 | Move to a fixed `(x,y,z)`, fire `MovementInform(POINT_MOTION_TYPE, id)` on arrival |
| `src/server/game/Movement/MovementGenerators/PointMovementGenerator.cpp` | 227 | Pathing + final orientation handling |
| `src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.h` | 75 | Taxi flight; integrates `TaxiPathNodes` |
| `src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp` | 338 | Build node-spline; per-node FlightSplineSync, money deduct, transition map |
| `src/server/game/Movement/MovementGenerators/FormationMovementGenerator.h` | 57 | Anchor offset to formation leader |
| `src/server/game/Movement/MovementGenerators/FormationMovementGenerator.cpp` | 222 | Track leader displacement, recompute slot pos |
| `src/server/game/Movement/MovementGenerators/SplineChainMovementGenerator.h` | 61 | Chained `MoveSplineInit` launches with pauses |
| `src/server/game/Movement/MovementGenerators/SplineChainMovementGenerator.cpp` | 238 | Resume from `SplineChainResumeInfo`, per-link `TimeToNext` |
| `src/server/game/Movement/MovementGenerators/GenericMovementGenerator.h` | 55 | One-shot custom `MoveSplineInit` wrapper |
| `src/server/game/Movement/MovementGenerators/GenericMovementGenerator.cpp` | 99 | Captured initializer lambda + duration |
| `src/server/game/Movement/MovementGenerators/PathMovementBase.h` | 43 | Shared path index/iter helpers |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `MotionMaster` | class | Stack-of-generators per slot for one Unit; entry point for AI/scripts |
| `MotionMaster::DelayedAction` | nested class | Captured `(action, validator, type)` deferred while `MOTIONMASTER_FLAG_UPDATE` set |
| `MotionMasterFlags` | enum | NONE / UPDATE / STATIC_INITIALIZATION_PENDING / INITIALIZATION_PENDING / INITIALIZING |
| `MotionMasterDelayedActionType` | enum | CLEAR / CLEAR_SLOT / CLEAR_MODE / CLEAR_PRIORITY / ADD / REMOVE / REMOVE_TYPE / INITIALIZE |
| `MovementGenerator` | abstract class | Interface: `Initialize/Reset/Update/Deactivate/Finalize/GetMovementGeneratorType/UnitSpeedChanged/Pause/Resume/GetResetPosition` |
| `MovementGeneratorMedium<T,D>` | CRTP template | Typed Update/Reset dispatch (avoids cast in concrete generators) |
| `MovementGeneratorFactory<T>` | template | Registry factory for type-tagged construction |
| `MovementGeneratorCreator` / `IdleMovementFactory` / `RandomMovementFactory` / `WaypointMovementFactory` | factory | Slot registration into `sMovementGeneratorRegistry` |
| `MovementGeneratorType` | enum (uint8) | IDLE=0, RANDOM=1, WAYPOINT=2, MAX_DB=3, CONFUSED=4, CHASE=5, HOME=6, FLIGHT=7, POINT=8, FLEEING=9, DISTRACT=10, ASSISTANCE=11, ASSISTANCE_DISTRACT=12, TIMED_FLEEING=13, FOLLOW=14, ROTATE=15, EFFECT=16, SPLINE_CHAIN=17, FORMATION=18 |
| `MovementSlot` | enum | DEFAULT (idle/random/waypoint) / ACTIVE (chase/flee/point/etc.) |
| `MovementGeneratorMode` | enum | DEFAULT / OVERRIDE |
| `MovementGeneratorPriority` | enum | NONE / NORMAL / HIGHEST |
| `MovementGeneratorFlags` | bitfield (uint16) | INITIALIZATION_PENDING / INITIALIZED / SPEED_UPDATE_PENDING / INTERRUPTED / PAUSED / TIMED_PAUSED / DEACTIVATED / INFORM_ENABLED / FINALIZED / PERSIST_ON_DEATH |
| `MovementGeneratorComparator` | functor | Stable ordering of `_generators` multiset (priority desc, then insertion) |
| `MovementGeneratorDeleter` | functor | unique_ptr deleter — runs destructor with knowledge of ABI boundary |
| `MovementGeneratorInformation` | POD | Debug snapshot `{Type, TargetGUID, TargetName}` |
| `RotateDirection` | enum | LEFT / RIGHT |
| `ChaseRange` | struct | `{MinRange, MinTolerance, MaxRange, MaxTolerance}` |
| `ChaseAngle` | struct | `{RelativeAngle, Tolerance}` + `IsAngleOkay`, `UpperBound`, `LowerBound` |
| `JumpArrivalCastArgs` | struct | `{SpellId, Target}` — cast-on-jump-end |
| `JumpChargeParams` | struct | Speed/MoveTimeInSec union + `JumpGravity` + `Spell/Progress/ParabolicCurveId` |
| `IdleMovementGenerator` | class | Ground-zero generator; never moves; clears unit move state |
| `RotateMovementGenerator` | class | Rotates orientation in place over `_duration` ms |
| `DistractMovementGenerator` | class | Hold facing for `_timer` ms (e.g. quest interaction) |
| `AssistanceDistractMovementGenerator` | class | Like Distract but reset to `Idle` on Finalize |
| `RandomMovementGenerator<Creature>` | template class | Wander within `_wanderDistance` radius; per-hop delay |
| `WaypointMovementGenerator<Creature>` | template class | Traverse `WaypointPath::Nodes`; per-node delay/script/event |
| `FollowMovementGenerator` | class | Owner tails target at `(distance, angle)` non-combat |
| `ChaseMovementGenerator` | class | Combat follow; reposition only when outside `ChaseRange.MaxRange` |
| `FleeingMovementGenerator` | class | Pick reachable point opposite from feared source; repeat |
| `TimedFleeingMovementGenerator` | class | FleeingMG with finite duration |
| `ConfusedMovementGenerator` | class | Short-radius wander (CC effect) |
| `HomeMovementGenerator<Creature>` | template class | Move to `Creature::GetHomePosition`; restore default state on arrival |
| `PointMovementGenerator` | class | Single `(x,y,z)` MoveSpline; fire `MovementInform(POINT_MOTION_TYPE, id)` |
| `FlightPathMovementGenerator` | class | Taxi flight along `TaxiPathNode` rows |
| `FormationMovementGenerator` | class | Anchor at `(range, angle)` from a formation leader |
| `SplineChainMovementGenerator` | class | Chain of `MoveSplineInit` launches with `TimeToNext` pauses |
| `GenericMovementGenerator` | class | Run a captured `std::function<void(MoveSplineInit&)>` once |
| `PathMovementBase<T,P>` | template | Shared path-index iterator helpers (waypoint, splinechain) |
| `AbstractFollower` | helper class | Stores target ObjectGuid + dirty bit on target relocate |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `MotionMaster::Initialize()` / `InitializeDefault()` | Bootstrap default IdleMovementGenerator on slot DEFAULT | `MovementGenerator::Initialize` |
| `MotionMaster::AddToWorld()` | Resolve `STATIC_INITIALIZATION_PENDING` after Unit added to map | `DirectInitialize` |
| `MotionMaster::Update(uint32 diff)` | Tick top generator; pop+finalize on completion; resolve delayed actions | `MovementGenerator::Update`, `Finalize`, `ResolveDelayedActions` |
| `MotionMaster::Add(MovementGenerator*, slot)` | Push generator on slot; defer if `MOTIONMASTER_FLAG_UPDATE` set | `DirectAdd` |
| `MotionMaster::Remove(generator|type, slot)` / `Clear(...)` | Pop+Finalize matching | `Delete`, `MovementGenerator::Finalize` |
| `MotionMaster::PropagateSpeedChange()` | Notify all live generators of speed change | `MovementGenerator::UnitSpeedChanged` |
| `MotionMaster::GetDestination(x,y,z)` | Read final destination of top spline | `MoveSpline::FinalDestination` |
| `MotionMaster::StopOnDeath()` | Pop non-`PERSIST_ON_DEATH` generators | `Clear` |
| `MotionMaster::MoveIdle()` / `MoveTargetedHome()` | Convenience wrappers | `Add(new IdleMovementGenerator)` etc. |
| `MotionMaster::MoveRandom(radius, duration)` | Push `RandomMovementGenerator` on DEFAULT | `Add` |
| `MotionMaster::MoveFollow(target, dist, ChaseAngle, duration, slot)` | Push `FollowMovementGenerator` | `Add` |
| `MotionMaster::MoveChase(target, ChaseRange, ChaseAngle)` | Push `ChaseMovementGenerator` | `Add` |
| `MotionMaster::MovePoint(id, pos, generatePath, finalOrient, speed, mode, closeEnoughDistance)` | Push `PointMovementGenerator` | `MoveSplineInit::MoveTo`, `Launch` |
| `MotionMaster::MoveCloserAndStop(id, target, distance)` | 2D approach until within distance, then stop | `MovePoint` variant |
| `MotionMaster::MoveLand` / `MoveTakeoff` | Push generator with landing/takeoff anim | `MoveSplineInit::SetAnimation` |
| `MotionMaster::MoveCharge(x,y,z,speed,id,...)` / `MoveCharge(PathGenerator&,...)` | Fast linear charge with pre-computed path | `MoveSplineInit::Launch` |
| `MotionMaster::MoveKnockbackFrom(origin, speedXY, speedZ, ...)` | Spell knockback parabolic | `MoveSplineInit::SetParabolic` |
| `MotionMaster::MoveJump` / `MoveJumpTo` / `MoveJumpWithGravity` | Parabolic jump variants (with optional `JumpArrivalCastArgs`) | `MoveSplineInit::SetParabolicVerticalAcceleration` |
| `MotionMaster::MoveCirclePath(x,y,z,radius,clockwise,stepCount)` | Circular orbit | `MoveSplineInit::MovebyPath` |
| `MotionMaster::MoveSmoothPath(pointId, points, size, walk, fly)` | Custom smooth path | `MoveSplineInit::MovebyPath` |
| `MotionMaster::MoveAlongSplineChain(pointId, chainId|chain, walk)` | Push `SplineChainMovementGenerator` | `Add` |
| `MotionMaster::ResumeSplineChain(SplineChainResumeInfo)` | Continue interrupted chain | `Add` |
| `MotionMaster::MoveFall(id)` | Drop straight down using gravity | `MoveSplineInit::SetFall` |
| `MotionMaster::MoveSeekAssistance(x,y,z)` / `MoveSeekAssistanceDistract(timer)` | Combat-assistance flow | `Add` |
| `MotionMaster::MoveTaxiFlight(path, pathnode)` | Push `FlightPathMovementGenerator` | `Add` |
| `MotionMaster::MoveDistract(time, orientation)` | Push `DistractMovementGenerator` | `Add` |
| `MotionMaster::MovePath(pathId|path, repeatable, duration, ...)` | Push `WaypointMovementGenerator` | `Add` |
| `MotionMaster::MoveRotate(id, time, dir)` | Push `RotateMovementGenerator` | `Add` |
| `MotionMaster::MoveFormation(leader, range, angle, p1, p2)` | Push `FormationMovementGenerator` | `Add` |
| `MotionMaster::MoveFleeing(enemy, time)` | Push `FleeingMovementGenerator` (or `TimedFleeingMovementGenerator` if `time>0`) | `Add` |
| `MotionMaster::MoveConfused()` | Push `ConfusedMovementGenerator` | `Add` |
| `MotionMaster::LaunchMoveSpline(initializer, id, priority, type)` | Run captured lambda once, push `GenericMovementGenerator` | `Add` |
| `MotionMaster::CalculateJumpSpeeds(dist, moveType, mult, minH, maxH, &outXY, &outZ)` | Solve parabolic for given distance + height bounds | math |
| `MotionMaster::ResolveDelayedActions()` | Drain `_delayedActions` deque (FIFO) | `DelayedAction::Resolve` |
| `MovementGenerator::Initialize(Unit*)` | Begin generator (called once when pushed) | concrete `DoInitialize` |
| `MovementGenerator::Reset(Unit*)` | Re-enter when previous top finalized | concrete `DoReset` |
| `MovementGenerator::Update(Unit*, diff) -> bool` | Tick; return false to finalize | concrete `DoUpdate` |
| `MovementGenerator::Deactivate(Unit*)` | Called when displaced by higher-priority generator | concrete `DoDeactivate` |
| `MovementGenerator::Finalize(Unit*, active, movementInform)` | Final cleanup; optionally fire `MovementInform` AI hook | concrete `DoFinalize` |
| `MovementGenerator::UnitSpeedChanged()` | Propagate Unit speed to live spline | varies |
| `MovementGenerator::Pause(timer)` / `Resume(timer)` | Pause generator (waypoint, follow) | sets `MOVEMENTGENERATOR_FLAG_PAUSED` |
| `MovementGenerator::GetResetPosition(Unit*, &x,&y,&z)` | Used by Evade flow to compute reset point | varies |
| `AbstractFollower::SetTarget(Unit*)` / `GetTarget()` | Track followed unit by GUID | — |

---

## 5. Module dependencies

**Depends on:**
- [`movement-spline.md`](movement-spline.md) — every generator that emits motion calls `MoveSplineInit::Launch` (defined in `Movement/Spline/MoveSplineInit.h`).
- [`movement-pathgen.md`](movement-pathgen.md) — `Chase`, `Follow`, `Point`, `Waypoint`, `Confused`, `Fleeing` all instantiate `PathGenerator` to compute walkable polylines.
- `Entities/Unit` — `Unit*` owns `MotionMaster`; receives `Unit::SetUnitState`, `ClearUnitState`.
- `Entities/Creature` — `Random/Waypoint/Home<Creature>` are typed templates; `Creature::GetHomePosition`, `Creature::SetHomePosition`, `Creature::IsAIEnabled`.
- `AI` ([`ai-base.md`](ai-base.md)) — `CreatureAI::MovementInform(type, id)` is the callback fired on Finalize when `MOVEMENTGENERATOR_FLAG_INFORM_ENABLED`.
- `Combat` — `ChaseMovementGenerator` activates on `Unit::EngageWithTarget`; `FleeingMovementGenerator` from `Spell::EffectInterruptCast` (fear).
- `Maps` — `Unit::Relocate` writes back to grid; visibility recompute in `Unit::Update`.
- `DataStores` — `TaxiPathStore`, `TaxiPathNodeStore` for `FlightPathMovementGenerator`.
- `Waypoints` — `WaypointManager` SQL loader (sub-module of movement; covered separately if split further).
- `SmartAI/Scripts` — emit `MovePoint`, `MoveChase`, `MoveAlongSplineChain` from boss scripts.
- `Detour` (3rdparty via [`movement-pathgen.md`](movement-pathgen.md)) — indirect through `PathGenerator`.
- [`common-collision.md`](common-collision.md) — wander/random pick must clamp to `Map::GetHeight` and pass LOS via `VMapManager`.

**Depended on by:**
- All `CreatureAI` / `SmartAI` / boss scripts (call `MotionMaster::Move*`).
- `Player` (taxi flight, charge spells, fall recovery).
- `Pets/Vehicles` (follow/chase owner).
- `Battlegrounds/Instances` — boss orchestrators emit `MoveAlongSplineChain` on phase transitions.
- `Spell System` — knockback, charge, blink push generators on victim/caster.

---

## 6. SQL / DB queries (if any)

`MotionMaster` and the generators do not emit SQL directly. Two sources feed them:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM waypoint_path` (via `WaypointManager`) | Patrol path metadata | world |
| `SELECT * FROM waypoint_path_node` (via `WaypointManager`) | Per-node pos+delay+event | world |
| `SELECT * FROM script_waypoint` | Escort/quest scripted paths | world |
| `SELECT * FROM creature_formations` | `FormationMovementGenerator` leader/follower offsets | world |
| `SELECT * FROM script_spline_chain_meta`, `script_spline_chain_waypoints` | `SplineChainMovementGenerator` static chains | world |
| `SELECT * FROM creature_addon` (`path_id`) | Spawn-level waypoint path attachment | world |
| `SELECT * FROM creature_template_addon` (`path_id`) | Template-level default path | world |
| `SELECT * FROM creature_movement_override` | Override default `MovementType` per spawn | world |

DBC/DB2 stores consumed:

| Store | What it loads | Read by |
|---|---|---|
| `TaxiPathStore` | `TaxiPath.dbc` | `FlightPathMovementGenerator` |
| `TaxiPathNodeStore` | `TaxiPathNode.dbc` | `FlightPathMovementGenerator` |
| `MapStore` (transition map column) | `Map.dbc` | `FlightPathMovementGenerator` (cross-continent flights) |

---

## 7. Wire-protocol packets (if any)

Generators do not own packet writers directly; every motion is serialized through `MoveSplineInit::Launch` (see [`movement-spline.md`](movement-spline.md)). Only `FlightPathMovementGenerator` emits a per-node sync packet:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_FLIGHT_SPLINE_SYNC` (0x2E2B) | S→C | `FlightPathMovementGenerator::DoEventIfAny` after each node |
| `SMSG_ON_MONSTER_MOVE` | S→C | indirectly via every generator's `MoveSplineInit::Launch` |
| `SMSG_MOVE_TELEPORT` | S→C | `HomeMovementGenerator` short-circuit if forced teleport home |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-movement/src/motion_master.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-movement/src/generators` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-movement` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-ai/src` | `module_dir` | 1 | 346 | `exists_active` | directory exists |
| `crates/wow-world/src/handlers/movement.rs` | `file` | 1 | 204 | `exists_active` | file exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-ai` | `crate_dir` | 1 | 346 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-movement/` exists with `defines.rs`, `generator.rs`, `motion_master.rs`, `spline.rs`, and `generators/{idle,rotate,distract,generic,point,follow,chase}.rs`.
- `wow-entities::MotionSubsystem` carries a represented `MotionMaster` state/update bridge for currently ported slices.
- There is still no owner-backed runtime `MotionMaster` slot stack with real `Unit` context and full per-tick generator update for every concrete generator.

**What's implemented:**
- Runtime `MovementGeneratorType`, slot/mode/priority/flags, `MovementGenerator` trait, `MotionMasterFlags`, delayed action ids/queue, initial `MotionMaster` stack/update/filter core, `ChaseRange`, `ChaseAngle`, `RotateDirection`, `MovementWalkRunSpeedSelectionMode`, jump/charge parameter shapes, `IdleMovementGenerator`, `RotateMovementGenerator`, `DistractMovementGenerator`, `AssistanceDistractMovementGenerator`, `GenericMovementGenerator`, `PointMovementGenerator`, `AssistanceMovementGenerator`, `AbstractFollower`, `FollowMovementGenerator`, `ChaseMovementGenerator`, `FleeingMovementGenerator`, `TimedFleeingMovementGenerator`, `ConfusedMovementGenerator`, `HomeMovementGenerator`, `RandomMovementGenerator`, `WaypointMovementGenerator`, `FormationMovementGenerator`, `FlightPathMovementGenerator`, and `SplineChainMovementGenerator`.
- Represented bridges in `wow-entities`/`wow-world` for Point/Generic/Idle/Rotate/Distract slices, direct creature point movement, facing-only rotate/distract splines, and canonical represented AI movement inform recording.
- A legacy `wow_ai::wander::pick_wander_destination` linear-tween remains; it is **not** a generator and must be replaced by real generator pushes.
- `crates/wow-world/src/handlers/movement.rs` parses client `CMSG_MOVE_*` opcodes and broadcasts movement updates, but it still does not drive the full owner-backed creature `MotionMaster`.

**What's missing vs C++:**
- **Most concrete runtime generators remain missing owner backing** — Effect remains absent; SplineChain, Flight, Formation, Waypoint, Random, Chase, Point, Fleeing, Confused, Home, Distract (+ Assistance, AssistanceDistract), Follow and Generic have represented runtime modules but still need real owner `Unit`, pathgen/DB2/SQL where applicable and AI/script dispatch.
- **Owner-backed runtime `MotionMaster` is still incomplete.** `wow-movement::MotionMaster` has boxed generator storage, priority ordering, delayed actions and top update/pop behavior, but still lacks real `Unit` owner context, default factory selection, owner finalize callbacks, public `Move*` API parity and map tick wiring.
- **`AbstractFollower` helper is represented only.** Target add/remove tracking exists in `wow-movement`; real owner/target `Unit` lookup and dirty tracking from actual map relocation still need owner-backed integration.
- **`MovementInform` AI callback** is only represented for selected bridges; real SmartAI/script dispatch is not wired.
- **`PropagateSpeedChange`** — `wow-movement::MotionMaster` now matches C++ by notifying only the current generator; real owner speed mutation still needs to call this hook from the owner-backed runtime.
- **Waypoint loading** — no `WaypointManager`, no SQL reader for `waypoint_path*`, no `script_waypoint`, no `creature_formations`, no `script_spline_chain_meta`.
- **Taxi flight** — represented `FlightPathMovementGenerator` exists, but there is still no `PlayerTaxi`, no `TaxiPath.dbc`/`TaxiPathNode.dbc` consumer, no real taxi handlers and no `SMSG_FLIGHT_SPLINE_SYNC` writer/caller.
- **Charge / jump / knockback / fall** — no parabolic emitters; `CMSG_MOVE_FALL_LAND` is parsed but no fall damage compute.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The `wow-ai::wander` linear-tween bypasses `MoveSplineInit::Launch`; clients only see the result via periodic position snapshots, not as a real spline. Once a real `MotionMaster` lands, this wander loop must be replaced by `RandomMovementGenerator` or it will fight the slot stack.
- There is no place in `wow-world` `WorldSession::tick_*` paths that calls a per-Unit `Update(diff)`. Adding `MotionMaster` requires plumbing a tick into the creature update loop in `WorldSession::tick_combat_sync` (or moving to `MapManager`-driven ticks once that migration completes).
- The legacy two-place creature storage (`WorldSession.creatures` HashMap vs `MapManager`) means a `MotionMaster` will need to live on `WorldCreature` inside `MapManager`, not on the per-session copy — wiring on the wrong side will leak motion across phasing or duplicate updates.

**Tests existing:**
- Runtime generators now have focused unit coverage in `wow-movement` for represented C++ constructor/lifecycle/update/finalize behavior; owner-backed integration tests are still missing.
- Runtime `MotionMaster` has unit coverage for priority ordering, delayed actions and top update/pop; owner-backed integration tests are still missing.
- `ChaseRange`/`ChaseAngle` have unit coverage for C++ contact-distance and angle-wrap semantics.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MOVEMENT_GENERATORS.WBS.001** Cerrar la migracion auditada de `game/Movement/MovementGenerators/ChaseMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/ChaseMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.002** Cerrar la migracion auditada de `game/Movement/MovementGenerators/ChaseMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/ChaseMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.003** Cerrar la migracion auditada de `game/Movement/MovementGenerators/ConfusedMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/ConfusedMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.004** Cerrar la migracion auditada de `game/Movement/MovementGenerators/ConfusedMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/ConfusedMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.005** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FleeingMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FleeingMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.006** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FleeingMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FleeingMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.007** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.008** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FlightPathMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.009** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FollowMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FollowMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.010** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FollowMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FollowMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.011** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FormationMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FormationMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.012** Cerrar la migracion auditada de `game/Movement/MovementGenerators/FormationMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FormationMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.013** Cerrar la migracion auditada de `game/Movement/MovementGenerators/GenericMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/GenericMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.014** Cerrar la migracion auditada de `game/Movement/MovementGenerators/GenericMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/GenericMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.015** Cerrar la migracion auditada de `game/Movement/MovementGenerators/HomeMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/HomeMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.016** Cerrar la migracion auditada de `game/Movement/MovementGenerators/HomeMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/HomeMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.017** Cerrar la migracion auditada de `game/Movement/MovementGenerators/IdleMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/IdleMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.018** Cerrar la migracion auditada de `game/Movement/MovementGenerators/IdleMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/IdleMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.019** Cerrar la migracion auditada de `game/Movement/MovementGenerators/PathMovementBase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/PathMovementBase.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.020** Cerrar la migracion auditada de `game/Movement/MovementGenerators/PointMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/PointMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.021** Cerrar la migracion auditada de `game/Movement/MovementGenerators/PointMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/PointMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.022** Cerrar la migracion auditada de `game/Movement/MovementGenerators/RandomMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/RandomMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.023** Cerrar la migracion auditada de `game/Movement/MovementGenerators/RandomMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/RandomMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.024** Cerrar la migracion auditada de `game/Movement/MovementGenerators/SplineChainMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/SplineChainMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.025** Cerrar la migracion auditada de `game/Movement/MovementGenerators/SplineChainMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/SplineChainMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.026** Cerrar la migracion auditada de `game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.027** Cerrar la migracion auditada de `game/Movement/MovementGenerators/WaypointMovementGenerator.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.028** Cerrar la migracion auditada de `game/Movement/Waypoints/WaypointDefines.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointDefines.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.029** Cerrar la migracion auditada de `game/Movement/Waypoints/WaypointManager.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_GENERATORS.WBS.030** Cerrar la migracion auditada de `game/Movement/Waypoints/WaypointManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.h`
  Rust target: `crates/wow-movement`, `crates/wow-ai/src`, `crates/wow-constants`, `crates/wow-ai`, `crates/wow-world`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#MOVE-GEN.1** Create `crates/wow-movement/` with module skeleton: `motion_master.rs`, `generator.rs`, `defines.rs`, `generators/{idle,random,waypoint,chase,follow,...}.rs`. `wow-movement`, `defines.rs`, `generator.rs`, `motion_master.rs` and `generators/idle.rs` exist; remaining concrete generator modules are pending. (L)
- [x] **#MOVE-GEN.2** Port `MovementGeneratorType` enum (0..18, identical values). Runtime enum exists in `wow-movement`; represented bridge equivalent exists as `wow_entities::MovementGeneratorKind`. (L)
- [x] **#MOVE-GEN.3** Port `MovementSlot` (DEFAULT/ACTIVE), `MovementGeneratorMode`, `MovementGeneratorPriority`. Runtime values exist in `wow-movement`; represented values exist in `wow-entities`. (L)
- [x] **#MOVE-GEN.4** Port `MovementGeneratorFlags` bitflags (`bitflags!` crate already in workspace). Runtime `bitflags!` exists in `wow-movement`; represented constants exist in `wow-entities`. (L)
- [x] **#MOVE-GEN.5** Define `trait MovementGenerator { fn initialize, reset, update, deactivate, finalize, kind, ... }`. Owner `Unit` integration remains pending for `#MOVE-GEN.8/#MOVE-GEN.25`. (M)
- [x] **#MOVE-GEN.6** Port `ChaseRange` + `ChaseAngle` structs with `is_angle_okay`, `upper_bound`, `lower_bound`. (L)
- [x] **#MOVE-GEN.7** Port `MotionMasterFlags` + `MotionMasterDelayedActionType` + `DelayedAction` struct. Flags, enum IDs, FIFO validator resolution, and represented payload execution are in `wow-entities`; generalized `MotionMaster::Update` remains in `#MOVE-GEN.8/#MOVE-GEN.9`. (L)
- [ ] **#MOVE-GEN.8** Implement `MotionMaster` core: stack per slot (`Vec<Box<dyn MovementGenerator>>`), `update(diff)` with delayed-action draining. Initial runtime core exists in `wow-movement`: default + active generator storage, priority ordering, top init/reset/update/pop and delayed-action draining. Remaining work: real owner `Unit` context, default factory selection, owner finalize callbacks, full current-generator info and map tick wiring. (H)
- [ ] **#MOVE-GEN.9** Wire `MotionMaster::add/remove/clear` + flag-gated deferral. Runtime `add`, `remove_kind`, `clear`, `clear_slot`, `clear_mode`, `clear_priority` and delayed FIFO execution exist in `wow-movement`; remaining work is pointer-identity remove, full C++ public API parity and owner-integrated validation. (M)
- [x] **#MOVE-GEN.10** `IdleMovementGenerator` + `RotateMovementGenerator` + `DistractMovementGenerator` + `AssistanceDistractMovementGenerator`. Runtime structs exist in `wow-movement` with C++ constructor/lifecycle/update/finalize semantics represented: idle stop, rotate facing math/duration/inform, distract stand-up/facing/timer/home orientation, and assistance return-to-aggressive. Runtime integration with real owner `Unit`, generalized `MotionMaster` execution and real SmartAI/script dispatch remain tracked in `#MOVE-GEN.8`, `#MOVE-GEN.24` and `#MOVE-GEN.25`. (M)
- [ ] **#MOVE-GEN.11** `PointMovementGenerator` (single-target move + `MovementInform(POINT, id)` callback). Runtime direct branch now exists in `wow-movement`: constructor/flags/base-state, `EVENT_CHARGE_PREPATH`, no-move/casting interruption, `UNIT_STATE_ROAMING_MOVE`, direct `MoveSplineInit` launch with speed/facing/final orientation/spell extras/close-enough, relaunch action, point inform mapping and assistance side-effect plan. Represented creature bridge still exists in `wow-world`. Remaining work: real pathgen branch, real owner `Unit` wiring, `SignalFormationMovement`, real SmartAI/script dispatch, real `CallAssistance`, and generalized map-owned `MotionMaster::Update`. (M)
- [ ] **#MOVE-GEN.12** `RandomMovementGenerator` (wander radius + delay + water/oxygen check). Runtime represented shape exists in `wow-movement`: constructor/flags/base-state, pause/resume, optional duration completion, reference capture, owner fallback wander distance, 2..10 step cycles, random destination math, LOS/path retry timers, `FARFROMPOLY` allowed, walk/run selection, rest pauses, formation signal and represented `MovementInform(RANDOM,0)`. Remaining work: real `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` state mutation, real `MoveSplineInit`, real formation signaling and AI dispatch. (H)
- [ ] **#MOVE-GEN.13** `AbstractFollower` helper + `FollowMovementGenerator`. Runtime represented shape exists in `wow-movement`: target add/remove events, constructor/init/reset/update/deactivate/finalize flags, duration/check timers, `PositionOkay`, angle selection, `UNIT_STATE_FOLLOW_MOVE`, target-counter inform and pet-speed side-effect counters. Remaining work: real owner/target `Unit` lookup, `PathGenerator`, `GetNearPoint`, hover Z update, pet owner checks, real `MoveSplineInit` path launch and AI dispatch. (M)
- [ ] **#MOVE-GEN.14** `ChaseMovementGenerator` (combat reposition + `ChaseRange`/`ChaseAngle`). Runtime represented shape exists in `wow-movement`: constructor/init/reset/update/deactivate/finalize flags, range-check timer, `PositionOkay` min/max/angle/LOS, mutual chase, lost-target/no-move/casting stops, `UNIT_STATE_CHASE_MOVE`, cannot-reach plan, walk-mode selection and target-counter inform. Remaining work: real owner/target `Unit`, `PathGenerator`, `GetNearPoint`, `ShortenPathUntilDist`, terrain/LOS/accessibility checks and actual `MoveSplineInit` path launch. (H)
- [ ] **#MOVE-GEN.15** `FleeingMovementGenerator` + `TimedFleeingMovementGenerator`. Runtime represented shape exists in `wow-movement`: constructor/flags/base-state, `UNIT_FLAG_FLEEING`, quiet-distance destination math, LOS/path retry timers, path length limit, `UNIT_STATE_FLEEING_MOVE`, random delay bounds, speed-update relaunch, specialized Player/Creature finalizers and timed-flee `MovementInform(TIMED_FLEEING,0)`. Remaining work: real `ObjectAccessor` target lookup, `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` flag/state mutation, real `MoveSplineInit` path launch and AI dispatch. (M)
- [ ] **#MOVE-GEN.16** `ConfusedMovementGenerator` (CC short hops). Runtime represented shape exists in `wow-movement`: constructor/flags/base-state, `UNIT_FLAG_CONFUSED`, reference capture, stop-on-initialize/reset, C++ short-hop random destination math, LOS/path retry timers, path length limit, walk launch plan, random delay bounds, speed-update relaunch and Player/Creature finalizers. Remaining work: real `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` flag/state mutation, real `MoveSplineInit` path launch and AI dispatch. (M)
- [ ] **#MOVE-GEN.17** `HomeMovementGenerator` + Evade integration. Runtime represented shape exists in `wow-movement`: constructor/flags/base-state, no-search-assistance reset, root/stunned/distracted interruption, clear `UNIT_STATE_ALL_ERASABLE & ~UNIT_STATE_EVADE`, add `UNIT_STATE_ROAMING_MOVE`, run-to-home launch plan with facing, update/finalize inform gating and represented `JustReachedHome` side effects. Remaining work: owner `Unit` state mutation, `UpdateAllowedPositionZ`, real `MoveSplineInit`, `VehicleKit::Reset`, creature addon/spawn health reload and real AI dispatch. (M)
- [ ] **#MOVE-GEN.18** `WaypointManager` SQL loader + `WaypointMovementGenerator`. Runtime represented `WaypointMovementGenerator` exists in `wow-movement`: path/current-node state, pause/resume, reset-position lookup, delayed initialization, arrival hooks, waypoint AI inform payloads, repeat/backtracking selection, path-end wait/random behavior, launch options and path-ended finalization. Remaining work: `WaypointManager` SQL loader, DB path lookup, real owner `Unit`, real `MoveSplineInit`, `PathGenerator`, transport/home-position mutation, real MotionMaster `MoveRandom` at path ends and AI dispatch. (XL — split: loader L, generator H)
- [ ] **#MOVE-GEN.19** `FormationMovementGenerator` + `creature_formations` loader. Runtime represented shape exists in `wow-movement`: `AbstractFollower`, follow-formation base state, no-move/casting stops, 1200ms leader-position relaunch, 1.65s leader-spline prediction, waypoint angle flip, predicted-spline stop, formation-move state, arrival facing/inform and finalize/deactivate cleanup. Remaining work: `creature_formations` loader, real `CreatureGroup` leader/current-waypoint lookup, owner `Unit`, real `MoveSplineInit`, formation signal integration and AI dispatch. (H)
- [ ] **#MOVE-GEN.20** `FlightPathMovementGenerator` + `TaxiPath.dbc`/`TaxiPathNode.dbc` integration + `SMSG_FLIGHT_SPLINE_SYNC`. Runtime represented generator exists in `wow-movement`: constructor/flags/base-state, reset launch plan, `GetPathAtMapEnd`, path-shortening by 40y/map/teleport/stop-delay, route-segment cost switches, departure/arrival events, preload-end-grid, teleport resume/skip and active finalize cleanup. Remaining work: `PlayerTaxi`, DB2 `TaxiPath`/`TaxiPathNode` stores, `ObjectMgr::GetTaxiPath`, real owner `Player`, real `MoveSplineInit`, taxi packet handlers, final `CleanupAfterTaxiFlight` wiring and `SMSG_FLIGHT_SPLINE_SYNC`. (XL)
- [ ] **#MOVE-GEN.21** `SplineChainMovementGenerator` + `script_spline_chain_*` loader + `SplineChainResumeInfo`. Runtime represented generator exists in `wow-movement`: constructor/resume constructors, `SplineChainLink`, `SplineChainResumeInfo`, partial resume, invalid point clamp, `MovebyPath` vs `MoveTo` selection, optional velocity, walk mode, duration-adjusted `_msToNext`, update sequencing, final-spline completion, `GetResumeInfo`, deactivate/finalize cleanup and represented `MovementInform(SPLINE_CHAIN,id)`. Remaining work: SQL loader for `script_spline_chain_meta`/`script_spline_chain_waypoints`, real `SystemMgr`, owner `Unit`, real `MoveSplineInit`, real `MotionMaster::MoveAlongSplineChain`/`ResumeSplineChain` and script integration. (H)
- [x] **#MOVE-GEN.22** `GenericMovementGenerator` (lambda capture for one-shot custom splines). Runtime struct exists in `wow-movement`: captures `FnOnce(&mut MoveSplineInit)`, launches against `MoveSpline`, stores duration, preserves cyclic/finalized update rules, no-resume deactivation behavior and represented arrival spell/inform output. Real `Unit` owner lookup for `CastSpell` and real `CreatureAI::MovementInform` dispatch remain tracked by Spell/AI/Unit integration tasks, not by the generator shape. (M)
- [ ] **#MOVE-GEN.23** `MotionMaster::MoveJump` / `MoveCharge` / `MoveKnockbackFrom` / `MoveFall` (parabolic helpers — depend on movement-spline). Pure helper slice is done: Rust has C++-matched jump max-height and `CalculateJumpSpeeds` math. Represented wrapper slice is also done: `LaunchMoveSpline`, `MoveJump`, `MoveJumpWithGravity`, `MoveKnockbackFrom` and `MoveFall` create/guard generator state like C++, and `PointMovementGenerator` now covers charge/prepath lifecycle/inform mapping. Remaining work is executable `MoveSplineInit`, real Unit speed selection, `MoveJumpTo`, spell effect extra data, pathgen charge/knockback raycast and fall height/hover lookup. (H)
- [ ] **#MOVE-GEN.24** Wire `Unit::MovementInform(type, id)` AI callback (cross-link with [`ai-base.md`](ai-base.md)). (M)
- [ ] **#MOVE-GEN.25** Wire per-Unit `MotionMaster::update(diff)` into `MapManager` creature tick. (H)
- [ ] **#MOVE-GEN.26** Replace `wow_ai::wander` linear-tween with `RandomMovementGenerator` push at spawn time. (M)
- [x] **#MOVE-GEN.27** `MotionMaster::PropagateSpeedChange` and `StopOnDeath` hooks. C++ audit corrected the old plan: `PropagateSpeedChange` notifies only `GetCurrentMovementGenerator()`, not every live generator or a listener registry. Runtime `wow-movement::MotionMaster` now exposes current-generator speed propagation plus `StopOnDeath` persist/clear/idle/stop-moving action semantics. Remaining owner-backed caller wiring stays under `#MOVE-GEN.8/#MOVE-GEN.25`. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MOVEMENT_GENERATORS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 30 files / 4535 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp`. Rust target: `crates/wow-ai`, `crates/wow-constants`, `crates/wow-world`. | `cargo test -p wow-ai && cargo test -p wow-constants && cargo test -p wow-world` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MOVEMENT_GENERATORS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 30 files / 4535 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp`. Rust target: `crates/wow-ai`, `crates/wow-constants`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MOVEMENT_GENERATORS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 30 files / 4535 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp`. Rust target: `crates/wow-ai`, `crates/wow-constants`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MOVEMENT_GENERATORS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 30 files / 4535 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp`. Rust target: `crates/wow-ai`, `crates/wow-constants`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `MotionMaster::initialize_default` pushes `IdleMovementGenerator` into slot DEFAULT.
- [ ] Test: After `MovePoint` + arrival, top of stack is back to IDLE on slot DEFAULT.
- [ ] Test: `Add` during `update_in_progress` defers to `_delayedActions` and fires after Update.
- [ ] Test: `Clear(slot=ACTIVE)` while a chase is running fires `Finalize(active=true)` then re-enters `Reset` on whatever was below.
- [x] Test: `RandomMovementGenerator` destination math stays within `wander_distance`, preserves walk/run rules, step/rest timers, LOS/path retries including C++ `FARFROMPOLY` allowance, pause/resume and duration completion.
- [ ] Test: `ChaseMovementGenerator` with `ChaseRange::MaxRange=5` only repositions if target moves > MaxRange away.
- [ ] Test: `ChaseAngle::is_angle_okay` returns true within `±tolerance`, false outside.
- [x] Test: `WaypointMovementGenerator` with 3 nodes covers initial delayed start, node arrival delay, `MovementInform(WAYPOINT,node_id)`, current-waypoint update, repeat/backtracking selection, path-end random/wait behavior and path-ended finalization.
- [x] Test: `FleeingMovementGenerator` destination math with feared source at (0,0,0) moves away in the too-close branch and preserves C++ quiet-distance branch constants.
- [x] Test: `HomeMovementGenerator` finalize clears roaming/evade and gates represented home-arrival side effects on `INFORM_ENABLED`/`movementInform`, including vehicle reset and swim flag behavior.
- [x] Test: `FlightPathMovementGenerator` covers constructor state, map-end/teleport segmentation, C++ path-shortening, route-switch costs, launch plan, departure/arrival event alternation, teleport resume/skip and finalize cleanup.
- [ ] Test: `SMSG_FLIGHT_SPLINE_SYNC` writer/caller for taxi flight once real `Player`/`MoveSpline` runtime is wired.
- [x] Test: `SplineChainMovementGenerator::ResumeFrom(SplineChainResumeInfo)` continues from `(SplineIndex, PointIndex, TimeToNext)`, clamps invalid point indexes, adjusts `_msToNext` when launch duration differs, sequences next splines and emits finalize inform.
- [x] Test: `FormationMovementGenerator` covers stationary/moving leader shape, 1.65s prediction, angle flip, periodic relaunch, arrival inform and finalize/deactivate cleanup.
- [x] Test: `MotionMaster::PropagateSpeedChange` calls `MovementGenerator::UnitSpeedChanged` on the current generator only, matching `MotionMaster.cpp`.
- [x] Test: `StopOnDeath` preserves a current generator with `MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH`, and otherwise clears, moves idle when in-world, and requests owner `StopMoving`.
- [ ] Test: Generator pushed with `MOTION_PRIORITY_HIGHEST` displaces a `NORMAL` priority active even if same type.
- [ ] Test: `AbstractFollower::dirty_bit` flips when target relocates between updates.
- [x] Test: `ConfusedMovementGenerator` selected destinations preserve the C++ `4*frand-2` distance and `2*pi` angle shape, including negative-distance hops around the captured reference.
- [ ] Test: `RotateMovementGenerator` with `direction=LEFT` produces monotonically increasing orientation modulo 2π.
- [ ] Test: `IdleMovementGenerator::initialize` calls `Unit::ClearUnitState(UNIT_STATE_MOVING)`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 30 files / 4535 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp` | future `crates/wow-movement/src/motion_master.rs` + `crates/wow-movement/src/generators/` \| ❌ not started — 0 of 13 generators ported, no `MotionMaster`, no trait |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MOVEMENT_GENERATORS.DIV.001` | `crates/wow-movement/src/motion_master.rs` (`missing_declared_path`, 0 Rust lines) | 30 C++ files / 4535 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#MOVEMENT_GENERATORS.DIV.002` | `crates/wow-movement/src/generators` (`missing_declared_path`, 0 Rust lines) | 30 C++ files / 4535 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#MOVEMENT_GENERATORS.DIV.003` | `crates/wow-movement` (`missing_declared_path`, 0 Rust lines) | 30 C++ files / 4535 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/WaypointMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/MovementGenerators/FlightPathMovementGenerator.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Waypoints/WaypointManager.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Delayed actions are not optional.** Calling `Add/Remove/Clear` from inside `MovementGenerator::Update` causes iterator invalidation in C++ unless `MOTIONMASTER_FLAG_UPDATE` is set and the call is queued. Rust safety alone will not save us — the same logic must defer mutation to a `VecDeque<DelayedAction>` drained at end of tick.
- **Slot is not priority.** Slot is *which class of motion* (default/active); priority is a comparator within the same slot. Removing from the wrong slot freezes a creature with no top-of-stack.
- **`MOTION_SLOT_DEFAULT` always autofills with `IdleMovementGenerator`** when emptied. Forgetting this leaves the unit motionless after every `MoveTargetedHome` arrival.
- **`MovementInform` callback fires only when `MOVEMENTGENERATOR_FLAG_INFORM_ENABLED` AND `Finalize(active=true, movementInform=true)`**. Forgetting to set the flag means boss scripts never get notified of waypoint arrivals and stall.
- **`ChaseMovementGenerator` overshoot** — without `PathGenerator::ShortenPathUntilDist`, mobs "stick" to the player. Always shorten chase paths to `ChaseRange.MaxRange - ChaseRange.MaxTolerance`.
- **`FleeingMovementGenerator`** picks a *random reachable* point opposite the feared source. If pathfinding fails repeatedly (e.g. cornered), it falls back to short-radius confused-style hops; this fallback is necessary, not optional.
- **`HomeMovementGenerator`** is the *only* generator that integrates with the Evade flow. Skipping it makes Evade leave creatures stranded.
- **`FlightPathMovementGenerator`** transitions between maps during cross-continent flights (e.g. Stormwind → Theramore). The `Map::dbc` `MapId` column drives the transition, not an in-engine teleport.
- **`SplineChainMovementGenerator`** must persist `SplineChainResumeInfo` on creature respawn — boss scripts expect to resume mid-chain after a wipe.
- **Formation drift** — leader teleport (e.g. taxi) without explicit `FormationMovementGenerator::leader_teleported()` hook desyncs every follower. Hook it.
- **`UnitSpeedChanged`** must propagate even mid-spline — a Bloodlust during chase changes melee leeway. Do not no-op this for live generators.
- **Generators are NOT thread-safe**. They mutate `Unit*` and read MoveSpline. The per-Unit lock model in Rust must be a write-lock for the whole `MotionMaster::update`, not interior mutability.
- **`PathMovementBase`** is intentionally a CRTP base (waypoint + spline_chain) to share index iteration. In Rust this becomes a small `PathCursor` struct embedded in both, not a trait.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class MotionMaster` | `struct MotionMaster { owner: ObjectGuid, default: Box<dyn MovementGenerator>, slots: [Vec<Box<dyn MovementGenerator>>; 2], delayed: VecDeque<DelayedAction>, flags: MotionMasterFlags, base_unit_states: HashMap<u32, *const dyn MovementGenerator> }` | Stack-per-slot |
| `class MovementGenerator` (abstract) | `wow_movement::MovementGenerator` | Trait surface exists; owner `Unit` integration is still pending |
| `MovementGeneratorMedium<T,D>` (CRTP) | (drop entirely) | Trait + concrete impls suffice; no CRTP needed |
| `FactoryHolder<MovementGenerator,Unit,MovementGeneratorType>` | `inventory::submit!` registration → fn pointer table indexed by `MovementGeneratorType` | Match the existing handler-dispatch pattern |
| `MovementGeneratorType` (enum uint8) | `wow_movement::MovementGeneratorType` | Identical numeric values |
| `MovementSlot` | `wow_movement::MovementSlot` | — |
| `MovementGeneratorFlags` | `wow_movement::MovementGeneratorFlags` | Identical bit positions |
| `MotionMasterFlags` | `wow_movement::MotionMasterFlags` | Identical bit positions |
| `DelayedAction` | `wow_movement::DelayedAction<M>` | Boxed closure + validator skeleton |
| `std::deque<DelayedAction>` | `VecDeque<DelayedAction>` | — |
| `std::multiset<MovementGenerator*, Comparator>` | `Vec<Box<dyn MovementGenerator>>` kept sorted by `(priority desc, insertion_idx)` | Multiset is overkill |
| `unique_ptr<MovementGenerator, Deleter>` | `Box<dyn MovementGenerator>` | — |
| `ChaseRange` / `ChaseAngle` | `struct ChaseRange { min: f32, min_tol: f32, max: f32, max_tol: f32 }` / `struct ChaseAngle { relative: f32, tolerance: f32 }` | POD |
| `JumpArrivalCastArgs` | `wow_movement::JumpArrivalCastArgs { spell_id: u32, target: ObjectGuid }` | Ported in `#A06.8h.3e.15` |
| `JumpChargeParams` (union) | `wow_movement::JumpChargeParams { spec: JumpChargeSpec, jump_gravity, spell_visual_id, progress_curve_id, parabolic_curve_id }` | Tagged enum replaces union; ported in `#A06.8h.3e.15` |
| `IdleMovementGenerator` | `wow_movement::IdleMovementGenerator` | Runtime skeleton ported; owner `StopMoving` represented until `Unit` context lands |
| `RandomMovementGenerator<Creature>` | `struct RandomMovementGenerator { wander_distance: f32, duration: Option<Duration>, next_hop_at: Instant, current_path: Option<Vec<Vec3>> }` | Drop the `<Creature>` — runtime check |
| `WaypointMovementGenerator<Creature>` | `struct WaypointMovementGenerator { path: Arc<WaypointPath>, current_node: usize, paused_until: Option<Instant>, repeatable: bool, ... }` | Path shared via `Arc` |
| `ChaseMovementGenerator` | `struct ChaseMovementGenerator { target: ObjectGuid, range: ChaseRange, angle: ChaseAngle, follower: AbstractFollower, recompute_at: Instant }` | — |
| `FollowMovementGenerator` | `struct FollowMovementGenerator { target: ObjectGuid, dist: f32, angle: ChaseAngle, follower: AbstractFollower, duration: Option<Duration> }` | — |
| `FleeingMovementGenerator` | `struct FleeingMovementGenerator { fearer: ObjectGuid, until: Option<Instant>, current_path: Option<Vec<Vec3>> }` | — |
| `ConfusedMovementGenerator` | `struct ConfusedMovementGenerator { until: Option<Instant>, next_hop_at: Instant }` | — |
| `HomeMovementGenerator<Creature>` | `struct HomeMovementGenerator { home: Position, arrived: bool }` | — |
| `PointMovementGenerator` | `struct PointMovementGenerator { id: u32, dest: Position, generate_path: bool, final_orient: Option<f32>, speed: Option<f32>, mode: SpeedSelectionMode, close_enough: Option<f32> }` | — |
| `FlightPathMovementGenerator` | `struct FlightPathMovementGenerator { path_nodes: Arc<[TaxiPathNode]>, current_node: usize, money_per_node: u32, transition_map: Option<u32> }` | — |
| `FormationMovementGenerator` | `struct FormationMovementGenerator { leader: ObjectGuid, range: f32, angle: f32, point1: u32, point2: u32 }` | — |
| `SplineChainMovementGenerator` | `struct SplineChainMovementGenerator { chain: Arc<[SplineChainLink]>, current_link: usize, current_point: usize, walk: bool, time_to_next: Duration, point_id: u32 }` | Resume info inline |
| `GenericMovementGenerator` | `struct GenericMovementGenerator { initializer: Box<dyn FnOnce(&mut MoveSplineInit) + Send>, id: u32, kind: MovementGeneratorType }` | Captured lambda |
| `AbstractFollower` | `struct AbstractFollower { target: ObjectGuid, dirty: bool }` | — |
| `RotateDirection` | `enum RotateDirection { Left, Right }` | — |
| `Unit::MovementInform(type, id)` callback | `wow_ai::CreatureAi::movement_inform(&mut self, ty: MovementGeneratorType, id: u32)` | Hook inside [`ai-base.md`](ai-base.md) |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked `/home/server/woltk-trinity-legacy/src/server/game/Movement/MotionMaster.{h,cpp}` (246 + 1376 lines), `MovementGenerator.{h,cpp}` (154 + 61), `MovementDefines.{h,cpp}` (142 + 48), `AbstractFollower.{h,cpp}` (36 + 31), and all 13 generator pairs under `MovementGenerators/` (3134 lines total of `.cpp`) against the Rust workspace at `/home/server/rustycore/crates/`.

**MotionMaster: runtime core exists, owner integration incomplete.** `wow-entities::MotionSubsystem` carries the represented bridge, and `wow-movement::MotionMaster` now has boxed executable `MovementGenerator` storage, default/active priority ordering, delayed actions, update/init/reset/pop behavior, represented base-unit-state ref counts, current-generator speed propagation, and C++ death-stop action semantics. Still missing: owner `Unit`, real finalize callbacks, default factory selection, map tick wiring, owner calls into `PropagateSpeedChange`, and full public `Move*` API parity.

**Generators: partially represented, not 13/13 runtime-complete.**
- IdleMovementGenerator + Rotate + Distract + AssistanceDistract: runtime structs now exist in `wow-movement`; owner-specific effects remain represented until the owner-backed `MotionMaster` is wired. Creature rotate/distract bridge can launch facing-only `MoveSplineInit`; real generic SmartAI/script dispatch is still pending.
- RandomMovementGenerator: runtime represented shape exists, including pause/resume, optional duration, reference/wander distance setup, 2..10 step cycles, random destination math, LOS/path retries, `FARFROMPOLY` allowance, walk/run selection, rest pauses, formation signal and represented `MovementInform(RANDOM,0)`. Real `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` mutation, actual path launch, formation signaling and AI dispatch remain pending. The legacy `wow_ai::wander` linear-tween is still present and must be replaced by generator pushes.
- WaypointMovementGenerator: runtime represented shape exists, including current-node state, pause/resume, reset position, delayed init quirk, arrival hooks, waypoint started/reached/path-ended payloads, repeat/backtracking, path-end wait/random behavior and launch options. `WaypointManager`, SQL loader for `waypoint_path`/`waypoint_path_node`, real owner `Unit`, real path/spline launch, transport/home-position mutation and AI dispatch remain pending.
- ConfusedMovementGenerator: runtime represented shape exists, including confused flag/state, captured reference, stop-on-initialize, short-hop random destination branches, LOS/path retry timers, path length limit, walking launch plan, speed-update relaunch and Player/Creature finalization differences. Real `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` mutation, actual path launch and AI dispatch remain pending.
- ChaseMovementGenerator: runtime represented shape exists, including `ChaseRange`/`ChaseAngle`, `PositionOkay`, mutual chase, range checks and cannot-reach plan. Real owner/target `Unit`, `PathGenerator`, `GetNearPoint`, `ShortenPathUntilDist`, terrain/LOS/accessibility checks and actual path launch remain pending.
- HomeMovementGenerator: runtime represented shape exists, including no-search-assistance reset, root/stunned/distracted interruption, erasable-state cleanup while preserving evade until finalize, home-facing run launch plan, update finish on interrupted/finalized spline and represented `JustReachedHome` side effects. Real owner `Unit`, `UpdateAllowedPositionZ`, actual `MoveSplineInit`, VehicleKit and AI dispatch remain pending.
- FlightPathMovementGenerator: runtime represented shape exists, including highest-priority in-flight state, taxi/control flags, spline launch options, path-end by map/teleport, path-shortening, route-switch costs, events, end-grid preload, teleport resume/skip and finalize cleanup. Real `PlayerTaxi`, DB2 `TaxiPath`/`TaxiPathNode`, `ObjectMgr::GetTaxiPath`, owner `Player`, actual `MoveSplineInit`, taxi handlers and `SMSG_FLIGHT_SPLINE_SYNC` remain pending.
- PointMovementGenerator: runtime direct branch exists in `wow-movement`, including close-enough/facing/spell extras and assistance plan; represented creature bridge exists in `wow-world`, including canonical represented `MovementInform(POINT,id)`. Real pathgen, owner Unit wiring, formation signal, real `CallAssistance` and real SmartAI/script dispatch are pending.
- FleeingMovementGenerator + TimedFleeingMovementGenerator: runtime represented shape exists, including fleeing flag/state, quiet-distance random destination branches, LOS/path retry timers, path length limit, move-state launch plan, speed-update relaunch, Player/Creature finalization differences and timed-flee inform. Real `ObjectAccessor`, `MovePositionToFirstCollision`, `PathGenerator`, owner `Unit` mutation, actual path launch and AI dispatch remain pending.
- FollowMovementGenerator: runtime represented shape exists, including `AbstractFollower`, timers, `PositionOkay`, angle selection and inform planning. Real owner/target `Unit`, `PathGenerator`, `GetNearPoint`, pet checks and actual path launch remain pending.
- FormationMovementGenerator: runtime represented shape exists, including `AbstractFollower`, 1200ms relaunch interval, 1.65s leader-spline prediction, waypoint angle flip, predicted-spline stop, formation-move state, arrival facing/inform and finalize/deactivate cleanup. Real `CreatureGroup`, `creature_formations` loader, owner `Unit`, actual `MoveSplineInit`, formation signal integration and AI dispatch remain pending.
- SplineChainMovementGenerator: runtime represented shape exists, including link/resume structs, partial resume, invalid point clamp, path launch shape, duration-adjusted `_msToNext`, final-spline completion, resume-info extraction and finalize inform. SQL loader `script_spline_chain_*`, real `SystemMgr`, owner `Unit`, actual `MoveSplineInit`, `MotionMaster` API and script integration remain pending.
- GenericMovementGenerator: runtime struct exists with executable `FnOnce(&mut MoveSplineInit)` launch against `MoveSpline`, duration/cyclic/finalized update rules and represented arrival spell/inform output. Real `CastSpell` and `CreatureAI::MovementInform` dispatch are pending in Spell/AI/Unit integration.

**Trait + enum surface.** The first boxed runtime `MovementGenerator` trait and runtime enum/flag surfaces now exist in `wow-movement`. Represented enum/flag surfaces still exist in `wow-entities`; pure `RotateDirection`, `ChaseRange`, `ChaseAngle`, `JumpArrivalCastArgs` and `JumpChargeParams` exist in `wow-movement`. Remaining structural gap: add the rest of the concrete runtime generator modules and move represented behavior behind a real runtime `MotionMaster` API without duplicating or regressing the tested bridge behavior.

**AI callback surface.** `Unit::MovementInform(type, id)` (the AI hook every generator calls on Finalize when `MOVEMENTGENERATOR_FLAG_INFORM_ENABLED`) has no Rust counterpart in `wow-ai`. Boss scripts that depend on waypoint/point arrival callbacks have nothing to attach to.

**Worst divergence.** This sub-doc is the single largest greenfield in the engine layer alongside [`movement-spline.md`](movement-spline.md) and [`movement-pathgen.md`](movement-pathgen.md): 13 concrete generator classes plus the `MotionMaster` orchestrator must be ported before any AI script in the codebase can request `MoveChase`, `MovePoint`, `MoveAlongSplineChain`, `MoveTaxiFlight`, `MoveCharge`, `MoveJump`, `MoveFleeing`, `MoveConfused`, `MoveHome`, `MovePath`, or `MoveFormation`. The current `wow_ai::wander` linear-tween is a non-substitute — it lacks lifecycle, slots, priority, deferral, AI callbacks, and pathfinding. Estimated XL across §9 tasks #MOVE-GEN.1 → #MOVE-GEN.27.
