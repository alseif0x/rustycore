# Migration: Movement / Spline (MoveSpline + Spline + MoveSplineFlag + MoveSplineInit)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Movement/Spline/MoveSpline.{h,cpp}` + `Spline.{h,cpp}` + `SplineImpl.h` + `MoveSplineFlag.h` + `MoveSplineInit.{h,cpp}` + `MoveSplineInitArgs.h` + `MovementUtil.cpp` + `SplineChain.h` + `MovementTypedefs.h`
> **Rust target crate(s):** `crates/wow-movement/src/spline.rs` now active; eventual split may mirror C++ files (`move_spline.rs`, `move_spline_flag.rs`, `move_spline_init.rs`, `movement_util.rs`)
> **Layer:** L5 sub-module (depends on `wow-math` / glam, `wow-packet` for `SMSG_ON_MONSTER_MOVE`, [`movement-pathgen.md`](movement-pathgen.md) for `PointsArray` source)
> **Status:** ⚠️ partial — `#A06.8h.3a` created the crate and first real `MoveSpline` core; not yet connected to Unit/MotionMaster/packets/pathgen
> **Audited vs C++:** ✅ complete 2026-05-01
> **Last updated:** 2026-05-11

> Sub-doc of [`movement.md`](movement.md). Cross-links: [`movement-generators.md`](movement-generators.md) (every generator drives a `MoveSpline` via `MoveSplineInit`), [`movement-pathgen.md`](movement-pathgen.md) (produces the `PointsArray` consumed by `MoveSplineInit::MovebyPath`), [`common-collision.md`](common-collision.md) (height clamps for fall/parabolic apex validation), [`ai-base.md`](ai-base.md) (AI is the indirect consumer via generators).

---

## 1. Purpose

Implements the **server-side spline math + serialization** that Trinity uses to move any Unit smoothly along a curve: linear segments, Catmull-Rom smooth paths, parabolic jumps/knockbacks, gravity-driven falls, cyclic patrol loops, and `Animation`/`FadeObject`/`TransportEnter|Exit` modifier flags. `MoveSpline` is the per-Unit live state (current curve + `time_passed` + `point_Idx` + flags); `MoveSplineInit` is the fluent builder; `Spline<int32>` is the underlying parametric evaluator with arc-length parametrization. The output is `SMSG_ON_MONSTER_MOVE` (and variants) sent on `Launch`.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Movement/Spline/MoveSpline.cpp` | 400 | `prefix` |
| `game/Movement/Spline/MoveSpline.h` | 155 | `prefix` |
| `game/Movement/Spline/MoveSplineFlag.h` | 141 | `prefix` |
| `game/Movement/Spline/MoveSplineInit.cpp` | 294 | `prefix` |
| `game/Movement/Spline/MoveSplineInit.h` | 220 | `prefix` |
| `game/Movement/Spline/MoveSplineInitArgs.h` | 94 | `prefix` |
| `game/Movement/Spline/MovementTypedefs.h` | 85 | `prefix` |
| `game/Movement/Spline/MovementUtil.cpp` | 212 | `prefix` |
| `game/Movement/Spline/Spline.cpp` | 312 | `prefix` |
| `game/Movement/Spline/Spline.h` | 217 | `prefix` |
| `game/Movement/Spline/SplineChain.h` | 50 | `prefix` |
| `game/Movement/Spline/SplineImpl.h` | 96 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/server/game/Movement/Spline/Spline.h` | 217 | `SplineBase` + templated `Spline<length_type>` with `evaluate_percent`, `evaluate_derivative`, `length(idx)`, mode dispatch table |
| `src/server/game/Movement/Spline/Spline.cpp` | 312 | `InitLengths`, `InitSpline`, segment lookup, mode-specific eval methods (Linear / CatmullRom / Bezier3) |
| `src/server/game/Movement/Spline/SplineImpl.h` | 96 | Inline impls of `computeIndex`, `evaluate`, length scan |
| `src/server/game/Movement/Spline/MoveSpline.h` | 155 | `MoveSpline` class — current spline state + `_updateState` + `ComputePosition` + `Result_*` enum |
| `src/server/game/Movement/Spline/MoveSpline.cpp` | 400 | `_updateState`, `ComputePosition`, `computeParabolicElevation`, `computeFallElevation`, `init_spline`, `ToString` |
| `src/server/game/Movement/Spline/MoveSplineFlag.h` | 141 | 32-bit flag bitfield (None / FallingSlow / Done / Falling / No_Spline / Flying / OrientationFixed / Catmullrom / Cyclic / Enter_Cycle / Frozen / TransportEnter|Exit / Backward / SmoothGroundPath / CanSwim / UncompressedPath / Animation / Parabolic / FadeObject / Steering / UnlimitedSpeed) |
| `src/server/game/Movement/Spline/MoveSplineInit.h` | 220 | `MoveSplineInit` builder + `TransportPathTransform` functor |
| `src/server/game/Movement/Spline/MoveSplineInit.cpp` | 294 | `Launch` (validate → commit → send `SMSG_ON_MONSTER_MOVE`), `Stop`, `MoveTo`/`MovebyPath`, transport coord transform |
| `src/server/game/Movement/Spline/MoveSplineInitArgs.h` | 94 | POD `MoveSplineInitArgs` consumed by `MoveSpline::Initialize`; `FacingInfo`, `SpellEffectExtraData`, `AnimTierTransition` |
| `src/server/game/Movement/Spline/MovementUtil.cpp` | 212 | `gravity` constant, `computeFallTime`, `computeFallElevation`, `JumpVelocity` helpers |
| `src/server/game/Movement/Spline/MovementTypedefs.h` | 85 | `Vector3` import, `MonsterMoveType` enum, `index_type` typedef |
| `src/server/game/Movement/Spline/SplineChain.h` | 50 | `SplineChainLink` (POD points + duration + msToNext + velocity) + `SplineChainResumeInfo` |

(`SMSG_ON_MONSTER_MOVE` packet writer lives in `src/server/game/Server/Packets/MovementPackets.{h,cpp}` — covered in [`movement.md`](movement.md) §7. Spline subsystem produces the data; packet module serializes it.)

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Movement::SplineBase` | abstract class | Common state for templated `Spline<T>`; holds `points`, `index_lo/hi`, `m_mode`, `cyclic`, `initialOrientation`; dispatch tables for Linear/CatmullRom/Bezier3 init/eval/seglen |
| `Movement::Spline<length_type>` | template class | Adds `lengths[]` array (cumulative arc-length per segment) + `length()` accessors; `length_type` is `int32` for `MoveSpline::MySpline` |
| `Movement::SplineBase::EvaluationMode` | enum | `ModeLinear` / `ModeCatmullrom` / `ModeBezier3_Unused` / `UninitializedMode` / `ModesEnd` |
| `Movement::MoveSpline` | class | Per-Unit current spline state: `spline`, `facing`, `m_Id`, `splineflags`, `time_passed`, `vertical_acceleration`, `effect_start_time`, `point_Idx`, `point_Idx_offset`, `velocity`, `spell_effect_extra`, `anim_tier`, `onTransport`, `splineIsFacingOnly` |
| `Movement::MoveSpline::UpdateResult` | enum | `Result_None=0x01` / `Result_Arrived=0x02` / `Result_NextCycle=0x04` / `Result_NextSegment=0x08` |
| `Movement::MoveSplineFlag` | bitfield class | 32-bit packed flags (see §2 file or list below) |
| `Movement::MoveSplineFlag::eFlags` | enum | `None=0x0`, `FallingSlow=0x10`, `Done=0x20`, `Falling=0x40`, `No_Spline=0x80`, `Flying=0x200`, `OrientationFixed=0x400`, `Catmullrom=0x800`, `Cyclic=0x1000`, `Enter_Cycle=0x2000`, `Frozen=0x4000`, `TransportEnter=0x8000`, `TransportExit=0x10000`, `Backward=0x80000`, `SmoothGroundPath=0x100000`, `CanSwim=0x200000`, `UncompressedPath=0x400000`, `Animation=0x2000000`, `Parabolic=0x4000000`, `FadeObject=0x8000000`, `Steering=0x10000000`, `UnlimitedSpeed=0x20000000` (+ several `Unknown_*` reserved bits) |
| `Movement::MoveSplineInit` | builder class | Fluent API to build a spline movement on a `Unit*`: `MoveTo`/`MovebyPath`/`SetFly`/`SetWalk(b)`/`SetCyclic`/`SetSmooth`/`SetUncompressed`/`SetParabolic`/`SetParabolicVerticalAcceleration`/`SetFall`/`SetTransportEnter|Exit`/`SetBackward`/`SetOrientationFixed(b)`/`SetUnlimitedSpeed`/`SetVelocity`/`SetFacing`/`SetAnimation`/`SetSpellEffectExtraData`/`SetFirstPointId`/`DisableTransportPathTransformations`/`Launch`/`Stop` |
| `Movement::MoveSplineInitArgs` | POD struct | `path: PointsArray`, `facing: FacingInfo`, `flags: MoveSplineFlag`, `path_Idx_offset`, `velocity`, `parabolic_amplitude`, `vertical_acceleration`, `effect_start_time_percent`, `effect_start_time`, `splineId`, `initialOrientation`, `spellEffectExtra`, `animTier`, `walk`, `HasVelocity`, `TransformForTransport` |
| `Movement::FacingInfo` | struct | `{f.x, f.y, f.z}` + `target: ObjectGuid` + `angle` + `type: MonsterMoveType` |
| `Movement::MonsterMoveType` | enum | `MONSTER_MOVE_NORMAL`, `MONSTER_MOVE_FACING_SPOT`, `MONSTER_MOVE_FACING_TARGET`, `MONSTER_MOVE_FACING_ANGLE`, `MONSTER_MOVE_STOP` |
| `Movement::SpellEffectExtraData` | struct | `Target: ObjectGuid`, `SpellVisualId`, `ProgressCurveId`, `ParabolicCurveId` |
| `Movement::AnimTierTransition` | struct | `TierTransitionId` + `AnimTier` |
| `Movement::Location` | struct | `Vector3` + `orientation` (return type of `ComputePosition`) |
| `Movement::PointsArray` | typedef | `std::vector<G3D::Vector3>` |
| `Movement::TransportPathTransform` | functor | Converts global Vector3 ↔ transport-local based on owner's transport |
| `SplineChainLink` | struct | `{Points: PointsArray, ExpectedDuration, TimeToNext, Velocity}` |
| `SplineChainResumeInfo` | struct | `{PointID, Chain*, IsWalkMode, SplineIndex, PointIndex, TimeToNext}` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Spline<T>::init_spline(Vector3 const*, count, mode)` | Pick init method by `mode` | `InitLinear` / `InitCatmullRom` / `InitBezier3` |
| `Spline<T>::init_cyclic_spline(...)` | Init with last point looping to first | as above + cyclic flag |
| `Spline<T>::initLengths()` | Compute cumulative `lengths[i]` from segment seg-len method | `SegLengthLinear` / `SegLengthCatmullRom` / `SegLengthBezier3` |
| `Spline<T>::initLengths(SegLengthInitializer&)` | Custom seg-length functor (used by `MoveSpline::init_spline` to integrate velocity to time) | functor |
| `Spline<T>::evaluate_percent(idx, u, &out)` | Evaluate position at segment `idx` and percent `u ∈ [0,1]` | dispatch table |
| `Spline<T>::evaluate_derivative(idx, u, &out)` | Tangent at `(idx, u)` | dispatch table |
| `Spline<T>::evaluate_percent(time, &out)` | Whole-spline `t ∈ [0, length()]` evaluation; computes segment + percent internally | `computeIndex`, `evaluate_percent` |
| `Spline<T>::computeIndex(t, &outIdx, &outU)` | Find segment that contains arc-length `t` via `lengths[]` binary search | — |
| `Spline<T>::length()` / `length(idx)` | Total / cumulative arc length | — |
| `Spline<T>::first()` / `last()` / `getPoint(idx)` / `getPoints()` | Index/data accessors | — |
| `Spline<T>::isCyclic()` / `getInitialOrientation()` | Read state | — |
| `MoveSpline::Initialize(MoveSplineInitArgs const&)` | Begin spline with parsed args; resets `time_passed=0`, `point_Idx=0` | `init_spline`, `Spline::initLengths` |
| `MoveSpline::init_spline(MoveSplineInitArgs const&)` | Internal — choose init method (linear/catmullrom), set flags, compute initial orientation | `Spline::init_spline`, `Spline::initLengths` |
| `MoveSpline::updateState(int32 difftime, UpdateHandler&)` | Loop over `_updateState` until `difftime <= 0`, calling handler with `Result_*` | `_updateState` |
| `MoveSpline::_updateState(int32& ms_time_diff)` | Advance `time_passed`; emit `Result_Arrived` / `NextSegment` / `NextCycle`; manage `point_Idx` | — |
| `MoveSpline::ComputePosition()` / `ComputePosition(int32 time_offset)` | Interpolate position at current/given time; apply parabolic / fall elevation | `Spline::evaluate_percent`, `computeParabolicElevation`, `computeFallElevation` |
| `MoveSpline::computePosition(time_point, point_index)` | Internal exact evaluation | `Spline::evaluate_percent` |
| `MoveSpline::computeParabolicElevation(t, &el)` | Add parabolic Z offset based on `parabolic_amplitude` + `vertical_acceleration` + `effect_start_time` | math |
| `MoveSpline::computeFallElevation(t, &el)` | Gravity-based Z drop: `el -= 0.5 * g * t²` | `Movement::computeFallElevation` |
| `MoveSpline::Duration()` | `spline.length()` (last cumulative length, in ms when scaled by velocity) | — |
| `MoveSpline::FinalDestination()` / `CurrentDestination()` | Last/next point of underlying `Spline::points` | — |
| `MoveSpline::Finalized()` / `isCyclic()` / `isFalling()` | Flag accessors | — |
| `MoveSpline::_Finalize()` | Snap unit to `FinalDestination`, set `splineflags.done` | — |
| `MoveSpline::_Interrupt()` | Sets `splineflags.done` without snap (used on `MoveSplineInit::Stop`) | — |
| `MoveSpline::ToString()` | Human-readable debug dump | flag stringification |
| `MoveSpline::HasStarted()` | `time_passed > 0` | — |
| `MoveSpline::GetAnimation()` | Return `anim_tier->AnimTier` if set | — |
| `MoveSplineFlag::raw()` / `hasFlag(f)` / `hasAllFlags(f)` | Read 32-bit pack | bitwise |
| `MoveSplineFlag::EnableAnimation/EnableParabolic/EnableFlying/EnableFalling/EnableCatmullRom/EnableTransportEnter/EnableTransportExit` | Set with sibling-flag clearing | bitwise |
| `MoveSplineFlag::isSmooth()` / `isLinear()` | Check `Catmullrom` bit | bitwise |
| `MoveSplineInit::MoveSplineInit(Unit*)` | Construct builder bound to a Unit | reads `Unit::movespline` |
| `MoveSplineInit::Launch()` | Validate args → commit `MoveSpline::Initialize` → send `SMSG_ON_MONSTER_MOVE` → return spline ID | `MoveSplineInitArgs::Validate`, packet send |
| `MoveSplineInit::Stop()` | Build minimal stop spline; send `SMSG_ON_MONSTER_MOVE` with `Done` flag | packet send |
| `MoveSplineInit::MoveTo(dest, generatePath, forceDest)` | Append straight-line OR call PathGenerator + load result | `PathGenerator::CalculatePath` |
| `MoveSplineInit::MovebyPath(path, pointId)` | Take an explicit path array | — |
| `MoveSplineInit::SetSmooth/SetUncompressed/SetFly/SetWalk(b)/SetCyclic/SetFall/SetTransportEnter/SetTransportExit/SetBackward/SetOrientationFixed(b)/SetUnlimitedSpeed/SetVelocity(f)/SetFirstPointId(id)` | Flag/state mutators | builder pattern |
| `MoveSplineInit::SetParabolic(amplitude, time_shift)` | Configure parabolic motion | bitfield setter |
| `MoveSplineInit::SetParabolicVerticalAcceleration(va, time_shift)` | Configure parabolic via vertical acceleration | bitfield setter |
| `MoveSplineInit::SetAnimation(AnimTier, transitionId, transitionStartTime)` | Schedule animation transition | bitfield setter |
| `MoveSplineInit::SetFacing(angle|point|target)` | Final-orientation spec | sets `FacingInfo` |
| `MoveSplineInit::SetSpellEffectExtraData(...)` | Attach spell visual extras | sets `args.spellEffectExtra` |
| `MoveSplineInit::DisableTransportPathTransformations()` | Skip transport coord conversion | sets `args.TransformForTransport=false` |
| `MoveSplineInitArgs::Validate(Unit*)` | Returns true if path lengths reasonable + flags coherent | `_checkPathLengths` |
| `Movement::computeFallTime(path_length, isSafe)` | Time to fall N yards under gravity | math |
| `Movement::computeFallElevation(t_passed, isSafe, start_velocity)` | Z drop at time `t` | math |
| `Movement::JumpVelocity(speedXY, speedZ, gravity)` | Solve initial velocity for jump arc | math |
| `TransportPathTransform::operator()(Vector3 in)` | Global ↔ transport-local conversion | `Unit::GetTransport`, transport relative pos |

---

## 5. Module dependencies

**Depends on:**
- `wow-math` (glam) — `Vector3` operations, `f32` math.
- `Entities/Unit` — `Unit*` owns `MoveSpline` (lives at `Unit::movespline`); `Unit::Relocate`, `Unit::IsTransported`, `Unit::GetTransport`.
- [`movement-pathgen.md`](movement-pathgen.md) — `MoveSplineInit::MoveTo(dest, generatePath=true, ...)` calls `PathGenerator::CalculatePath` to build the underlying `PointsArray`.
- [`movement-generators.md`](movement-generators.md) — every generator that emits motion uses `MoveSplineInit::Launch`.
- `wow-packet` (`MovementPackets`) — the `SMSG_ON_MONSTER_MOVE` writer reads `MoveSpline` state directly (friend access in C++).
- `Spell System` — `MoveSplineInit::SetSpellEffectExtraData` for charge/jump spell visuals; `Movement::SpellEffectExtraData` is consumed by `SMSG_ON_MONSTER_MOVE`.
- `Transports` — `TransportPathTransform` uses `Unit::GetTransport()` and the transport's global position to convert coords.
- [`common-collision.md`](common-collision.md) — fall validation + apex Z clamp.

**Depended on by:**
- All [`movement-generators.md`](movement-generators.md) generators (every `MoveTo` / `MovebyPath` / `SetParabolic` / `SetFall`).
- `MotionMaster::MoveJump` / `MoveCharge` / `MoveKnockbackFrom` / `MoveFall` (build a `MoveSplineInit` and `Launch`).
- `wow-packet` `SMSG_ON_MONSTER_MOVE` writer (reads MoveSpline directly via friend).
- `Player` knockback handlers, taxi interpolation, fall recovery.

---

## 6. SQL / DB queries (if any)

Spline subsystem itself emits **no SQL**. It consumes data prepared by callers. Two indirect SQL dependencies:

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM script_spline_chain_meta`, `script_spline_chain_waypoints` | `SplineChainLink` arrays driving `MoveSplineInit::MovebyPath` | world (loaded by `SplineChainMovementGenerator`) |
| `SELECT * FROM waypoint_path_node` | Per-node positions used by `MoveSplineInit::MovebyPath` | world (via `WaypointMovementGenerator`) |

DBC/DB2 stores indirectly relevant:

| Store | What it loads | Read by |
|---|---|---|
| `LiquidTypeStore` (`LiquidType.dbc`) | Used by `MoveSplineInit::MoveTo` to detect water for `CanSwim` flag | indirect |
| `MapDifficultyStore` (`MapDifficulty.dbc`) | Velocity scaling | indirect |

---

## 7. Wire-protocol packets (if any)

`MoveSplineInit::Launch()` is the canonical emitter; the writer lives in `MovementPackets.cpp` but reads `MoveSpline` directly (friend declared on `class CommonMovement` and `class MonsterMove`).

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `SMSG_ON_MONSTER_MOVE` | S→C | `MoveSplineInit::Launch` (full path), `Stop` (Done flag) |
| `SMSG_FLIGHT_SPLINE_SYNC` (0x2E2B) | S→C | `FlightPathMovementGenerator::DoEventIfAny` (uses spline `time_passed`) |
| `SMSG_MOVE_SPLINE_SET_RUN_SPEED` / `MOVE_SPLINE_SET_WALK_SPEED` / `MOVE_SPLINE_SET_FLY_SPEED` etc. (0x2DE7..0x2DEF) | S→C | mid-spline speed change broadcast |
| `SMSG_MOVE_SPLINE_DISABLE/ENABLE_GRAVITY/COLLISION` (0x2E1B..0x2E1E) | S→C | mid-spline toggle of `splineflags.unknown_*` |

The actual byte layout (compressed packed deltas vs `UncompressedPath`, parabolic extra block, anim transition extra, jump extra, fade time) is defined in `MovementPackets.cpp` and is covered in [`movement.md`](movement.md) §7.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-movement/src/spline` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-movement` | `crate_dir` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-packet/src/packets/movement.rs` | `file` | 1 | 461 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-movement/src/spline.rs`: first active runtime core for `MoveSplineFlag`, `MonsterMoveType`, `FacingInfo`, `SpellEffectExtraData`, `MoveSplineInitArgs`, fall/parabolic math, CatmullRom-compatible point storage, duration arrays, `MoveSpline::initialize`, `compute_position`, `update_state`, cyclic wrap, `finalize`, and a portable `MoveSplineInit` builder core.
- `crates/wow-movement/src/lib.rs`: exports the movement spline API.

**What's implemented:**
- `#A06.8h.3a`: `MoveSplineFlag` bit values and conflict mutators are ported from `MoveSplineFlag.h`; `computeFallTime`/`computeFallElevation` are ported from `MovementUtil.cpp`; `MoveSplineInitArgs::Validate` and `_checkPathLengths` are ported from `MoveSpline.cpp`; `MoveSpline` initialization, duration, parabolic elevation, falling elevation, linear/CatmullRom-compatible storage, update state and finalization are ported as an isolated Rust core with unit tests.
- `#A06.8h.3d.1-#A06.8h.3d.3`: the portable `MoveSplineInit` slice now covers `MoveTo`/`MovebyPath`, first-point correction on launch, selected/custom velocity handling, movement flags, `Stop` with `Done`/tolerance 2, and the C++ setter side effects for first point id, transport enter/exit, parabolic, parabolic vertical acceleration, animation, facing spot/angle/target-with-precomputed-angle, spell effect extra data, and disabling transport path transformations.
- A C++-like `SMSG_ON_MONSTER_MOVE` DTO/writer exists in `crates/wow-packet/src/packets/movement.rs`, including face modes, points and packed deltas. It is **not yet driven by the new `MoveSpline` runtime**.
- `MovementInfo::read/write` parses the *client-side* movement bits (transport, fall, inertia, adv_flying), but the server never produces a server-driven spline.

**What's missing vs C++:**
- **Unit integration** — the active Rust world still uses the old `MoveSplineState` shell in `wow-entities`; the new `wow-movement::MoveSpline` is not yet owned/ticked by Unit.
- **`MoveSpline` completion** — first core exists, and `Enter_Cycle` path rewrite preserving the previous duration plus `AnimTierTransition` modeling are now ported. DB2 curve application for spell parabolic/progress curves and broader fixtures are still pending.
- **`MoveSplineInit`** — portable builder core exists, but it is not a full C++ `Unit*` builder yet. Missing pieces: constructor bound to real `Unit`, `TransportPathTransform`, pathgen-backed `MoveTo(generatePath=true)`, `CanSwim`/movement-mode selection from Unit, `SetFacing(Unit const*)` angle calculation from owner/target positions, generic Unit broadcast, and all MotionMaster/generator callers.
- **`TransportPathTransform`** — no global ↔ transport-local conversion. Transport offsets in `MovementInfo` are read but never re-applied to a server-driven path (because no server-driven path exists).
- **Packet mapping from real `MoveSpline`** — `SMSG_ON_MONSTER_MOVE` DTO exists, but `MovementPackets.cpp::InitializeSplineData` equivalent is not connected to the new runtime; optional filter/spell/jump/anim blocks are not emitted from runtime state.
- **`AnimTierTransition` + Spline extras** — extra blocks of the packet are not fully modeled/emitted.
- **`SplineChainLink` + `SplineChainResumeInfo`** — POD does not exist; cannot drive boss script chains.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Bit positions in any future `MoveSplineFlag` port must match C++ exactly (`Done=0x20`, `Cyclic=0x1000`, `UncompressedPath=0x400000`, `Falling=0x40`, `Flying=0x200`, `Catmullrom=0x800`, `Parabolic=0x4000000`). Wire format breaks immediately if any bit is moved.
- Catmull-Rom evaluation in C++ uses a specific Hermite-basis form (`SegLengthCatmullRom` integrates over `stepsPerSegment=3`). A naive 4-point uniform Catmull-Rom in Rust will produce visually different paths than what the client expects.
- The C++ `Spline` is templated on `length_type` (`int32` for `MoveSpline`); arc-length is stored as integer milliseconds after dividing by velocity. Rust port must keep this `i32` quantization or `time_passed` arithmetic drifts.
- `MoveSpline::FinalDestination()` returns `Vector3::zero()` if `!Initialized()`. Calling code relies on this. In Rust prefer `Option<Vec3>` to avoid silent zero-bug.
- WotLK 3.4.3 has a `Steering` flag (0x10000000) marked `Mask_Unused` in TC master but used by some flying mounts in Classic. Confirm against client behavior before treating as no-op.

**Tests existing:**
- 0 tests for `MoveSplineFlag` (does not exist).
- 0 tests for `Spline` (does not exist).
- 0 tests for `MoveSpline::updateState` (does not exist).
- 0 tests for `computeFallTime` / `computeParabolicElevation`.
- 0 round-trip test for `SMSG_ON_MONSTER_MOVE` decode by a mock client.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#MOVEMENT_SPLINE.WBS.001** Cerrar la migracion auditada de `game/Movement/Spline/MoveSpline.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.002** Cerrar la migracion auditada de `game/Movement/Spline/MoveSpline.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.003** Cerrar la migracion auditada de `game/Movement/Spline/MoveSplineFlag.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineFlag.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.004** Cerrar la migracion auditada de `game/Movement/Spline/MoveSplineInit.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.005** Cerrar la migracion auditada de `game/Movement/Spline/MoveSplineInit.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.006** Cerrar la migracion auditada de `game/Movement/Spline/MoveSplineInitArgs.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInitArgs.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.007** Cerrar la migracion auditada de `game/Movement/Spline/MovementTypedefs.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MovementTypedefs.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.008** Cerrar la migracion auditada de `game/Movement/Spline/MovementUtil.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MovementUtil.cpp`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.009** Cerrar la migracion auditada de `game/Movement/Spline/Spline.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.010** Cerrar la migracion auditada de `game/Movement/Spline/Spline.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.011** Cerrar la migracion auditada de `game/Movement/Spline/SplineChain.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/SplineChain.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#MOVEMENT_SPLINE.WBS.012** Cerrar la migracion auditada de `game/Movement/Spline/SplineImpl.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/SplineImpl.h`
  Rust target: `crates/wow-movement`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md` §5. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [x] **#MOVE-SPL.1** Create `crates/wow-movement` skeleton and active `spline.rs` module. (L)
- [x] **#MOVE-SPL.2** Port `MoveSplineFlag` as `bitflags! struct MoveSplineFlag: u32` with all 32 named bits + `Mask_No_Monster_Move` + `Mask_Unused` + the `EnableAnimation/EnableParabolic/EnableFlying/EnableFalling/EnableCatmullRom/EnableTransportEnter|Exit` setters that clear sibling flags. Hex-value tests. (M)
- [x] **#MOVE-SPL.3** Port Location semantics onto `wow_core::Position` (`x/y/z/orientation`) for the first runtime core. (L)
- [x] **#MOVE-SPL.4** Port full `Movement::FacingInfo` + `MonsterMoveType` enum + `SpellEffectExtraData` + `AnimTierTransition` as runtime data structs; packet-extra mapping completed in `#A06.8h.3c.1`. (L)
- [x] **#MOVE-SPL.5** Port `MoveSplineInitArgs` as POD; `Validate(unit)` + `_checkPathLengths` without Unit logging side effects. (M)
- [x] **#MOVE-SPL.6** Port `SplineBase` core needed by `MoveSpline`: `points`, `index_lo/hi`, smooth/linear mode, cyclic, `initial_orientation`, `steps_per_segment=3`. (M)
- [x] **#MOVE-SPL.7** Port linear evaluator: `eval_percent_linear`, `eval_derivative_linear`, `seg_length_linear`. C++ still stores via CatmullRom-style virtual points; Rust mirrors that storage. (M)
- [x] **#MOVE-SPL.8** Port Catmull-Rom evaluator: `eval_percent_catmullrom`, `eval_derivative_catmullrom`, `seg_length_catmullrom`, `init_catmullrom` with virtual endpoints. (H)
- [x] **#MOVE-SPL.9** Port `Spline<i32>` arc-length wrapper shape: `lengths: Vec<i32>`, `init_lengths`, `length(idx)`, segment durations and `compute_index(t)` / percent evaluation rules from `SplineImpl.h`. (H)
- [x] **#MOVE-SPL.10** Port `MoveSpline` core: state struct + `Initialize(args)` + `init_spline` + `_updateState(&mut diff)` + `UpdateResult` enum. (H)
- [x] **#MOVE-SPL.11** Port `MoveSpline::ComputePosition(time_offset)` + `computeParabolicElevation` + `computeFallElevation`. (M)
- [x] **#MOVE-SPL.12** Port `Movement::computeFallTime` + `computeFallElevation` (free fns, gravity = 19.291105f). (M)
- [ ] **#MOVE-SPL.13** Port `MoveSpline::_Finalize` + `_Interrupt` + `ToString` (debug). `_Finalize` and `_Interrupt` semantics exist; `ToString` debug parity is still pending. (L)
- [ ] **#MOVE-SPL.14** Port `MoveSplineInit` builder: constructor, all setters (`SetFly/SetWalk/SetCyclic/SetSmooth/SetUncompressed/SetFall/SetTransportEnter|Exit/SetBackward/SetOrientationFixed/SetUnlimitedSpeed/SetVelocity/SetFirstPointId/DisableTransportPathTransformations`), `MoveTo/MovebyPath`, `SetParabolic/SetParabolicVerticalAcceleration`, `SetAnimation`, `SetFacing` (4 overloads), `SetSpellEffectExtraData`. Portable setter slice is done in `wow_movement::MoveSplineInit` for every setter that does not require live `Unit`/transport context; remaining work is the Unit-bound constructor, `SetFacing(Unit const*)` angle calculation, transport transforms, pathgen-backed `MoveTo`, and caller integration. (H)
- [ ] **#MOVE-SPL.15** Port `MoveSplineInit::Launch` (validate → commit `MoveSpline::Initialize` → send packet → return spline ID) + `Stop`. First core slice is done in `wow_movement::MoveSplineInit`: builder shape, `MoveTo/MovebyPath`, initial point correction, `enter_cycle`, movement flag forward/backward/root handling, selected/custom velocity with C++ speed clamp, `MoveSpline::Initialize`, returned duration, and `Stop` reinitialization with `MoveSplineFlag::DONE` plus tolerance 2. Remaining work: full Unit-owned launch/broadcast, transport transform, speed selection from real Unit movement mode, and all callers. (H)
- [ ] **#MOVE-SPL.16** Port `TransportPathTransform` functor (global ↔ transport-local). (M)
- [ ] **#MOVE-SPL.17** Port `SplineChainLink` + `SplineChainResumeInfo` POD. (L)
- [x] **#MOVE-SPL.18** Extend `crates/wow-packet/src/packets/movement.rs` `SMSG_ON_MONSTER_MOVE` writer to emit: facing types (`MONSTER_MOVE_FACING_SPOT/TARGET/ANGLE`), packed deltas vs `UncompressedPath`, `Cyclic`/`Enter_Cycle` packet flags, `SplineFilter`, spell visual extra data, jump extra data, anim-transition extra and fade-time. Covered by `#A06.8h.3c.1`; runtime Unit hookup remains under `#MOVE-SPL.20`. (XL — split per feature)
- [ ] **#MOVE-SPL.19** Round-trip test: serialize a `MoveSpline` via writer + decode bytes against a captured packet from a real client session. (M)
- [ ] **#MOVE-SPL.20** Hook `MoveSpline::updateState(diff)` into the per-Unit tick (consumed by `MotionMaster::Update`); produce `Unit::Relocate` calls each segment. First partial bridge is done for represented creature wander only: `WorldCreature` stores an active `MoveSpline`, advances it in the world tick, synchronizes represented `MoveSplineState`, and broadcasts via `MovementMonsterSpline::from_move_spline`. Full `Unit::UpdateSplineMovement` parity remains open. (H)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#MOVEMENT_SPLINE.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 12 files / 2276 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#MOVEMENT_SPLINE.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 12 files / 2276 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#MOVEMENT_SPLINE.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 12 files / 2276 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#MOVEMENT_SPLINE.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 12 files / 2276 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `MoveSplineFlag` bit positions match C++ hex (`Done=0x20`, `Falling=0x40`, `Flying=0x200`, `Catmullrom=0x800`, `Cyclic=0x1000`, `Enter_Cycle=0x2000`, `Frozen=0x4000`, `TransportEnter=0x8000`, `TransportExit=0x10000`, `Backward=0x80000`, `SmoothGroundPath=0x100000`, `CanSwim=0x200000`, `UncompressedPath=0x400000`, `Animation=0x2000000`, `Parabolic=0x4000000`, `FadeObject=0x8000000`, `Steering=0x10000000`, `UnlimitedSpeed=0x20000000`).
- [ ] Test: `MoveSplineFlag::EnableParabolic` clears `Falling`, `Animation`, `FallingSlow`, `FadeObject` and sets `Parabolic`.
- [ ] Test: `MoveSplineFlag::EnableFalling` clears `Parabolic`, `Animation`, `Flying` and sets `Falling`.
- [ ] Test: `Spline::evaluate_percent(t=0)` returns first point exactly; `evaluate_percent(t=length())` returns last point.
- [ ] Test: `Spline::length()` is monotonic non-decreasing over `lengths[i]`.
- [ ] Test: Catmull-Rom with 4 collinear points reproduces a straight line (tolerance 1e-3).
- [ ] Test: Catmull-Rom with 4 known points produces the same `evaluate_percent(0.5)` as the C++ reference (port a captured `(input, expected_output)` table).
- [ ] Test: `Spline::compute_index(t)` returns `(idx, u)` such that `lengths[idx] <= t * total <= lengths[idx+1]`.
- [ ] Test: `MoveSpline::updateState` with duration 1000ms ticks 100ms × 10 emits exactly one `Result_Arrived` at t=1000ms.
- [ ] Test: `MoveSpline::updateState` with `Cyclic` flag emits `Result_NextCycle` at end of last segment, never `Result_Arrived`.
- [ ] Test: `MoveSpline::updateState` with multi-segment path emits `Result_NextSegment` at each segment boundary.
- [ ] Test: `MoveSpline::ComputePosition` with `Parabolic` flag at half-time returns Z = base_z + amplitude (gravity-free `effect_start_time_percent=0`).
- [ ] Test: `MoveSpline::ComputePosition` with `Falling` flag drops `0.5*g*t²` from start altitude.
- [ ] Test: `Movement::computeFallTime(yards=20, isSafe=false)` matches C++ reference value within 1e-3.
- [ ] Test: `MoveSplineInit::MoveTo(dest, generatePath=false)` produces a 2-point straight-line path (`[start, dest]`).
- [ ] Test: `MoveSplineInit::SetCyclic` then `Launch` produces a `MoveSpline` with `splineflags.cyclic == true` and `Duration() > 0`.
- [ ] Test: `MoveSplineInit::Stop` produces a packet with only the `Done` flag and no point array.
- [ ] Test: `TransportPathTransform`: with owner on a transport at global (100,0,0), input local (5,0,0) → output (105,0,0).
- [ ] Test: `SMSG_ON_MONSTER_MOVE` round-trip: write a `MoveSpline` with parabolic + facing-target + 4 catmullrom points; decode bytes and verify all fields match.
- [ ] Test: `MoveSplineInitArgs::Validate` rejects path with > 100 points or NaN coordinates.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 12 files / 2276 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp` | future `crates/wow-movement/src/spline/` (`spline.rs`, `move_spline.rs`, `move_spline_flag.rs`, `move_spline_init.rs`, `movement_util.rs`) \| ❌ not started — 0 spline classes, no flag bitfield, no parabolic/fall math |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#MOVEMENT_SPLINE.DIV.001` | `crates/wow-movement/src/spline` (`missing_declared_path`, 0 Rust lines) | 12 C++ files / 2276 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#MOVEMENT_SPLINE.DIV.002` | `crates/wow-movement` (`missing_declared_path`, 0 Rust lines) | 12 C++ files / 2276 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSpline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/Spline.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/MoveSplineInit.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **`Catmullrom` interpolation requires phantom endpoints.** C++ inserts a duplicate of the first and last point as control points (`InitCatmullRom` extends `points` array by 2). Forgetting this gives wrong tangents at boundaries — visible client snap.
- **Arc-length integer quantization.** `Spline<int32>::lengths` stores cumulative arc-length as `int32` milliseconds after dividing by velocity. `time_passed` is also `int32`. Mixing `f32` arc-length with `i32` time causes drift across long cyclic patrols.
- **`steps_per_segment = 3` is canonical.** C++ default. Lower (1) loses precision; higher (20 — client default) costs more CPU. Don't change without spec.
- **`Frozen` flag never arrives.** A `MoveSpline` with `Frozen` set will never emit `Result_Arrived` — `_updateState` early-returns. Used by GM commands and stuck-mob scripts.
- **`Enter_Cycle` flag erases the first vertex after one cycle completes.** This is a wire-protocol quirk: cyclic paths starting at the unit's spawn point want the first cycle to include the spawn, but subsequent cycles to skip it. Replicate exactly.
- **`Mask_No_Monster_Move = Done`** — the `Done` flag must NEVER appear in an outgoing `SMSG_ON_MONSTER_MOVE` (it would tell the client the move is already finished). Strip it on serialization.
- **`Mask_Unused`** lists the bits that should never be set in any Wotlk Classic context (`No_Spline | Enter_Cycle | Frozen | Unknown_0x8 | Unknown_0x100 | ...`). `MoveSplineInitArgs::Validate` checks no unused bit is set.
- **`EnableParabolic` is mutually exclusive with `Falling`, `Animation`, `FallingSlow`, `FadeObject`** — the setter clears them. Replicate the clearing in Rust.
- **`EnableFalling` clears `Parabolic`, `Animation`, `Flying`** — same deal.
- **Velocity ≠ speed.** `MoveSplineInitArgs::velocity` overrides the auto-selected speed only if `HasVelocity == true`. If `velocity > 50.0f` (flying) or `> 28.0f` (ground) and `UnlimitedSpeed` not set, validation should reject.
- **`onTransport` is a `MoveSpline` member, NOT a flag bit.** Easy mistake — it's used to apply `TransportPathTransform` on each `ComputePosition`. Don't conflate with `TransportEnter/TransportExit` flags (which are wire-only handshake bits).
- **`splineIsFacingOnly`** is also a `MoveSpline` member — true for spline that only changes facing without moving (used by some scripts). Skipping it makes facing-only changes broadcast as movement.
- **Parabolic vs JumpWithGravity.** `SetParabolic(amplitude, time_shift)` uses `parabolic_amplitude`; `SetParabolicVerticalAcceleration(va, time_shift)` uses `vertical_acceleration`. Mutually exclusive — only one is non-zero at any time.
- **`effect_start_time` vs `effect_start_time_percent`.** Animation transitions use absolute ms; parabolic uses fraction of total duration. Don't swap them.
- **Cyclic time wraparound.** `time_passed` is `int32`, ~24 days max. Cyclic splines must reset on cycle complete or arithmetic overflows (C++ does this in `_updateState` on `Result_NextCycle`).
- **Friend access.** C++ `WorldPackets::Movement::CommonMovement` and `MonsterMove` are friends of `MoveSpline` so they can read internal state. In Rust, expose a read-only accessor module instead.
- **Initial orientation.** Computed from `derivative` at `t=0` (tangent of first segment). If the spline is degenerate (length 0), fall back to `args.initialOrientation`. Skipping fallback yields NaN orientation.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Movement::SplineBase` | `struct SplineBase { points: Vec<Vec3>, index_lo: i32, index_hi: i32, mode: SplineMode, cyclic: bool, initial_orientation: f32, steps_per_segment: i32 }` | `Vec3` from glam |
| `enum SplineBase::EvaluationMode` | `#[repr(u8)] enum SplineMode { Linear, CatmullRom, Bezier3 }` | Drop `UninitializedMode` — `Option<Spline>` instead |
| `class Movement::Spline<int32>` | `struct Spline { base: SplineBase, lengths: Vec<i32> }` | Drop the template — int32 is the only instantiation |
| `class Movement::MoveSpline` | `struct MoveSpline { spline: Spline, facing: FacingInfo, id: u32, flags: MoveSplineFlag, time_passed: i32, vertical_acceleration: f32, initial_orientation: f32, effect_start_time: i32, point_idx: i32, point_idx_offset: i32, velocity: f32, spell_effect_extra: Option<SpellEffectExtraData>, anim_tier: Option<AnimTierTransition>, on_transport: bool, spline_is_facing_only: bool }` | — |
| `enum MoveSpline::UpdateResult` | `bitflags! struct UpdateResult: u8 { const NONE=0x01; const ARRIVED=0x02; const NEXT_CYCLE=0x04; const NEXT_SEGMENT=0x08; }` | Bitflags so multiple events per tick |
| `class Movement::MoveSplineFlag` (32-bit packed) | `bitflags! struct MoveSplineFlag: u32 { ... }` | All bits identical |
| `class Movement::MoveSplineInit` | `struct MoveSplineInit<'a> { args: MoveSplineInitArgs, unit: &'a mut Unit }` | Builder; `Launch` consumes self |
| `struct Movement::MoveSplineInitArgs` | `struct MoveSplineInitArgs { path: Vec<Vec3>, facing: FacingInfo, flags: MoveSplineFlag, path_idx_offset: i32, velocity: f32, parabolic_amplitude: f32, vertical_acceleration: f32, effect_start_time_percent: f32, effect_start_time: Duration, spline_id: u32, initial_orientation: f32, spell_effect_extra: Option<SpellEffectExtraData>, anim_tier: Option<AnimTierTransition>, walk: bool, has_velocity: bool, transform_for_transport: bool }` | POD |
| `struct Movement::FacingInfo` | `enum FacingInfo { Normal, Spot(Vec3), Target(ObjectGuid), Angle(f32), Stop }` | Tagged enum replaces union-ish struct; matches `MonsterMoveType` |
| `struct Movement::SpellEffectExtraData` | `struct SpellEffectExtraData { target: ObjectGuid, spell_visual_id: u32, progress_curve_id: u32, parabolic_curve_id: u32 }` | POD |
| `struct Movement::AnimTierTransition` | `struct AnimTierTransition { tier_transition_id: u32, anim_tier: AnimTier }` | POD |
| `struct Movement::Location` | `struct Location { pos: Vec3, orientation: f32 }` | — |
| `Movement::PointsArray` (vector<Vector3>) | `Vec<Vec3>` | — |
| `class Movement::TransportPathTransform` | `fn transform_for_transport(owner: &Unit, point: Vec3, direction: TransformDir) -> Vec3` | Pure function |
| `Movement::computeFallTime(path_length, isSafe)` | `pub fn compute_fall_time(yards: f32, is_safe: bool) -> Duration` | — |
| `Movement::computeFallElevation(t, isSafe, v0)` | `pub fn compute_fall_elevation(t: Duration, is_safe: bool, v0: f32) -> f32` | — |
| `gravity` constant | `pub const GRAVITY: f32 = 19.291105;` | Match C++ exactly |
| `SplineChainLink` | `struct SplineChainLink { points: Vec<Vec3>, expected_duration: Duration, time_to_next: Duration, velocity: f32 }` | POD |
| `SplineChainResumeInfo` | `struct SplineChainResumeInfo { point_id: u32, chain: Arc<[SplineChainLink]>, is_walk_mode: bool, spline_index: u8, point_index: u8, time_to_next: Duration }` | Use `Arc` instead of raw pointer |
| `MoveSpline::updateState<UpdateHandler>(diff, handler)` | `pub fn update_state<F: FnMut(UpdateResult)>(&mut self, diff: &mut i32, mut handler: F)` | Generic closure |
| `ASSERT(Initialized())` | `debug_assert!(self.initialized())` | — |
| `Vector3::zero()` | `Vec3::ZERO` | — |
| `friend class MonsterMove` | pub(crate) accessors module | Avoid `friend` — expose getters under `pub(crate)` for the packet writer |

---

*Template version: 1.0 (2026-05-01).* Last updated: 2026-05-01.

---

## 13. Audit (2026-05-01)

**Scope.** Cross-checked `/home/server/woltk-trinity-legacy/src/server/game/Movement/Spline/` (12 files, 2276 lines: `MoveSpline.{h,cpp}` 155+400, `Spline.{h,cpp}` 217+312, `SplineImpl.h` 96, `MoveSplineFlag.h` 141, `MoveSplineInit.{h,cpp}` 220+294, `MoveSplineInitArgs.h` 94, `MovementUtil.cpp` 212, `SplineChain.h` 50, `MovementTypedefs.h` 85) against the Rust workspace at `/home/server/rustycore/crates/`.

**Spline subsystem: absent.** There is **no spline math anywhere in the workspace**. No `MoveSplineFlag` bitfield (so the 32 flag bits cannot be set, read, or serialized). No `SplineBase` / `Spline<T>` parametric evaluator (so no Catmull-Rom, no arc-length parametrization, no segment lookup). No `MoveSpline` state machine (so no `time_passed`, no `_updateState`, no `Result_Arrived` callbacks, no parabolic, no fall). No `MoveSplineInit` builder (so no fluent way to launch a spline movement). No `MoveSplineInitArgs` POD. No `computeFallTime` / `computeFallElevation` / gravity constant. No `TransportPathTransform`. No `SplineChainLink` / `SplineChainResumeInfo`.

**Packet writer is a stub.** `crates/wow-packet/src/packets/movement.rs` does emit a `SMSG_ON_MONSTER_MOVE` byte stream, but it is a single-segment straight-line writer: `[start_pos, dest]` with no flag bitfield, no facing-types (`MONSTER_MOVE_FACING_SPOT/TARGET/ANGLE`), no compressed packed deltas, no parabolic extra block, no fall extra, no anim transition extra, no fade-time. Used today only by ad-hoc test harnesses; not driven by any generator (because no generator exists, see [`movement-generators.md`](movement-generators.md)).

**Math: missing.** `MovementUtil.cpp` (212 lines) defines gravity and the fall-time integration that the entire jump/fall/parabolic flow depends on. Rust ships **0 lines** of equivalent math. `CMSG_MOVE_FALL_LAND` is parsed by `MovementInfo::read` but the server cannot validate the client-reported `fall_time` against an expected apex (because there is nothing to expect from). `CMSG_MOVE_JUMP` is similarly parsed but unverified — a hacked client can lie about `z_speed`, `xy_speed`, `sin_angle`, `cos_angle` with no consequence.

**Flags: critical correctness risk.** `MoveSplineFlag` defines 32 named bits with very specific hex values (`Done=0x20`, `Falling=0x40`, `Flying=0x200`, `Catmullrom=0x800`, `Cyclic=0x1000`, `Parabolic=0x4000000`, ...). These are wire-format positions read by the WoLK 3.4.3 client — getting any one wrong silently breaks creature movement visually (snap, glide, never-arrive). Rust currently has no place to encode these positions, and no test fixture will catch errors before a real client connects.

**Worst divergence.** This is the single largest pure-math greenfield in the engine. A `MoveSpline` is what every animated NPC, taxi flight, boss tour, fear flee, charge spell, jump cast, knockback, and falling player visually relies on. Without it: creatures cannot follow curves (they can only snap or linear-tween), bosses cannot do scripted spline tours, taxi flights have nothing to interpolate along, fear cannot pick a curved escape path, charge spells cannot show a parabolic visual, and `CMSG_MOVE_FALL_LAND` cannot be validated. All of this is gated on porting Spline + MoveSpline + MoveSplineFlag + MoveSplineInit before [`movement-generators.md`](movement-generators.md) sub-tasks can land. Estimated XL across §9 tasks #MOVE-SPL.1 → #MOVE-SPL.20.
