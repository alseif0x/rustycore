# Migration: anticheat (cross-cutting reference)

> **C++ canonical path:** scattered — primarily `src/server/game/Entities/Player/Player.cpp`, `src/server/game/Handlers/MovementHandler.cpp`, `src/server/game/Server/WorldSession.cpp`
> **Rust target crate(s):** `crates/wow-world/` (handler-level), `crates/wow-anticheat/` (portable movement sanitizer core)
> **Layer:** L8 (game-rule policy / enforcement)
> **Status:** 🔧 partial — movement sanitizer core exists, handler/runtime integration still represented
> **Audited vs C++:** ⚠️ partial (this document is the audit)
> **Last updated:** 2026-05-01

> Scope note: **inline / heuristic** anticheat only. Warden (binary memory scanning, signed module checksums, MPQ verification) is a separate subsystem and lives in its own migration doc (`warden.md`, TBD). Everything below operates strictly on already-decoded server packets and server-side world state.

---

## 1. Purpose

Catch the most common third-party clients and packet-spoofing tricks that would otherwise let a player move faster than allowed, fly without a flying spell, walk through walls, teleport across a map, fall without taking damage, or DOS the server with packet floods. The C++ implementation is **not** a single module — it's ~30 separate checks woven into the movement, combat, and session paths. Each one is a small invariant: "this combination of `MovementFlags` cannot legally coexist", "this speed delta exceeds the player's known speed rate", "this opcode arrives at higher rate than CPU budget allows".

---

## 2. C++ canonical files

Paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Anticheat-relevant content |
|---|---:|---|
| `src/server/game/Entities/Player/Player.cpp` | 39000 | `Player::ValidateMovementInfo` (line 28435 — 86 lines) — the primary inline movement-flag sanitizer. `Player::HandleFall`, `Player::SetFallInformation`, `Player::SetCanFly`. |
| `src/server/game/Handlers/MovementHandler.cpp` | 816 | `WorldSession::HandleMovementOpcodes` (307), `HandleForceSpeedChangeAck` (470 — speed-mismatch kick), `HandleSetActiveMoverOpcode` (543 — wrong-mover trace), `HandleMoveTimeSkippedOpcode` (721), every Ack handler calls `ValidateMovementInfo`. |
| `src/server/game/Server/WorldSession.cpp` | 2400 | `DosProtection::EvaluateOpcode` (1259), `DosProtection::GetMaxPacketCounterAllowed` (1313 — ~190 cases), `KickPlayer`, ban escalation (1303 `BAN_ACCOUNT` / `BAN_IP`). |
| `src/server/game/Server/Protocol/Opcodes.cpp` | 2280 | Per-opcode `STATUS_LOGGEDIN` gate is a coarse anticheat (drops opcodes that arrive before the right session phase). |
| `src/server/game/Movement/MovementInfo.h` | 200 | `MovementInfo` flags + `RemoveMovementFlag` used by validator. |
| `src/server/worldserver/worldserver.conf.dist` | 4500 | `MaxOverspeedPings = 2` (line 296), `PacketSpoof.Policy = 1` (4314), `PacketSpoof.BanMode = 0` (4324), `PacketSpoof.BanDuration = 86400` (4333). |
| `sql/base/auth_database.sql` (`account_banned`, `ip_banned`) | — | Persistence for ban escalation issued by AntiDOS. |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Player::ValidateMovementInfo(MovementInfo*)` | method | The flag-sanitizer. Mutates the incoming `MovementInfo` in place, stripping illegal flag combinations. |
| `WorldSession::DosProtection` | nested class | Per-session opcode rate limiter. Holds `_PacketThrottlingMap` and `_policy`. |
| `PacketCounter` | struct | `{ time_t lastReceiveTime; uint32 amountCounter; }` — one slot per opcode. |
| `DosProtection::Policy` | enum | `POLICY_LOG=0`, `POLICY_KICK=1`, `POLICY_BAN=2`. |
| `BanMode` | enum | `BAN_ACCOUNT=0`, `BAN_CHARACTER=1` (folded to ACCOUNT), `BAN_IP=2`. |
| `MovementInfo` | struct | Wire-format movement state — `flags`, `flags2`, `position`, `time`, `transport`, `swimming`, `fall`, `pitch`, `splineElevation`, `stepUpStartElevation`. |
| `m_forced_speed_changes[MAX_MOVE_TYPE]` | `Player` field | Per-speed-type counter of expected `*_SPEED_CHANGE_ACK` packets — used to filter race-condition false positives. |

`MAX_MOVE_TYPE` = 9 (Walk, Run, RunBack, Swim, SwimBack, TurnRate, Flight, FlightBack, PitchRate).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Player::ValidateMovementInfo(MovementInfo*)` | Strip impossible flag combinations on every movement packet. Below table enumerates each rule. | `MovementInfo::HasMovementFlag`, `MovementInfo::RemoveMovementFlag`, `Unit::HasAuraType` |
| `WorldSession::HandleForceSpeedChangeAck(MovementSpeedAck&)` | Compare ack speed to server-known speed. Mismatch >0.01: if client > server, log + correct; if client < server, **kick**. | `Player::GetSpeed`, `Player::SetSpeedRate`, `KickPlayer` |
| `DosProtection::EvaluateOpcode(WorldPacket&, time_t)` | Per-tick rate cap per opcode; on overflow: log/kick/ban. | `KickPlayer`, `World::BanAccount` |
| `DosProtection::GetMaxPacketCounterAllowed(uint16)` | Hard-coded packets-per-second cap per opcode. ~25 default-1; ~190 explicit. | — |
| `Player::HandleFall(MovementInfo const&)` | Compute fall-damage. Server authoritative — client cannot avoid by lying. | `Player::EnvironmentalDamage` |
| `WorldSession::HandleMovementOpcode(OpcodeClient, MovementInfo&)` | Master inbound-movement dispatch. GUID match check, `ValidateMovementInfo`, transport-cast, broadcast. | `Player::ValidateMovementInfo`, `Map::PlayerRelocation` |

### 4.1 `ValidateMovementInfo` rule table (the heart of inline anticheat)

| # | Detected condition | Mask removed | Comment |
|---:|---|---|---|
| 1 | `ROOT` set, but unit not on a fixed-position vehicle | `ROOT` | client cannot self-root |
| 2 | `ROOT && (FORWARD\|BACKWARD\|LEFT\|RIGHT\|...)` | `MASK_MOVING` | "this must be a packet spoofing attempt" — would freeze peers |
| 3 | `HOVER` without `SPELL_AURA_HOVER` | `HOVER` | aura-required flag |
| 4 | `ASCENDING && DESCENDING` | both | mutually exclusive |
| 5 | `LEFT && RIGHT` | both | mutually exclusive |
| 6 | `STRAFE_LEFT && STRAFE_RIGHT` | both | mutually exclusive |
| 7 | `PITCH_UP && PITCH_DOWN` | both | mutually exclusive |
| 8 | `FORWARD && BACKWARD` | both | mutually exclusive |
| 9 | `WATERWALKING` without `SPELL_AURA_WATER_WALK` (and not ghost) | `WATERWALKING` | aura-required |
| 10 | `FALLING_SLOW` without `SPELL_AURA_FEATHER_FALL` | `FALLING_SLOW` | aura-required |
| 11 | `(FLYING\|CAN_FLY)` and account is plain `SEC_PLAYER` and no fly aura | `FLYING\|CAN_FLY` | classic fly-hack |
| 12 | `(DISABLE_GRAVITY\|CAN_FLY) && FALLING` | `FALLING` | gravity off ⇒ no falling |
| 13 | `SPLINE_ELEVATION` flag set with `stepUpStartElevation == 0.0f` | `SPLINE_ELEVATION` | flag/value mismatch |
| 14 | inverse: `stepUpStartElevation != 0.0f` but flag absent | (force-add `SPLINE_ELEVATION`) | client-side validator order |

**Order nuance:** with the current represented model there is no fixed-position vehicle runtime, so
Rust mirrors the common C++ path where rule 1 removes `ROOT` first. In that path `ROOT | FORWARD`
becomes `FORWARD`, not `ROOT`. Rule 2 only removes moving flags when `ROOT` survives rule 1, which
requires a fixed-position vehicle context that remains pending with Vehicle runtime.

### 4.2 Speed-change kick path

`HandleForceSpeedChangeAck` (line 470):

1. Look up expected `move_type` from opcode (9-case switch).
2. If `_player->m_forced_speed_changes[move_type] > 0`, decrement and skip last-only ack-filter (handles run/mount one-ack quirk).
3. If `!GetTransport()` and `|GetSpeed(move_type) − packet.Speed| > 0.01`:
   - server-side speed > ack: log + call `SetSpeedRate(GetSpeedRate())`.
   - server-side speed < ack: **`KickPlayer("Incorrect speed")`** with debug log including account id.

**Legacy nuance / possible C++ bug:** in this tree, `Unit::SetSpeedRate` returns early when the
rate is unchanged (`Unit.cpp:8464-8465`). Because the correction path passes the current
`GetSpeedRate(move_type)`, the "force-resync" comment does not actually emit a movement speed packet.
Rust mirrors the observable behavior by recording the correction without sending a packet; fixing this
would be a deliberate behavioral change, not a pure port.

### 4.3 DosProtection rate limit

`EvaluateOpcode`:

```
allowed = GetMaxPacketCounterAllowed(opcode)
if allowed == 0: return true                       // no cap
if counter.lastReceiveTime != now: counter.reset(now)
if ++counter.amountCounter <= allowed: return true
log("AntiDOS: flooding opc 0x{:X}, count {}")
switch policy:
    LOG  → return true
    KICK → KickPlayer("AntiDOS"); return false
    BAN  → BanAccount(BAN_ACCOUNT|BAN_IP, name|ip, BanDuration, "DOS"); KickPlayer; return false
```

Sample of explicit per-opcode caps (from the ~190-case switch):

| Opcode | Max/sec |
|---|---:|
| `CMSG_PLAYER_LOGIN` | 0 (unlimited) |
| `CMSG_LOGOUT_REQUEST` | 0 (unlimited) |
| `CMSG_QUERY_PLAYER_NAMES` | 0 (unlimited) |
| `CMSG_QUERY_TIME` | 0 (unlimited) |
| `CMSG_ATTACK_STOP` | 0 (unlimited) |
| `CMSG_MOVE_TIME_SKIPPED` | 0 (unlimited) |
| `CMSG_BANKER_ACTIVATE` | 0 (unlimited) |
| `CMSG_OPT_OUT_OF_LOOT` | 0 (unlimited) |
| `CMSG_MOVE_HEARTBEAT` | 200 |
| `CMSG_HOTFIX_REQUEST` | 1 |
| (default for unlisted) | 100 |

`MaxOverspeedPings = 2` is a separate socket ping counter in this C++ tree: `WorldSocket::HandlePing` increments `_OverSpeedPings` when consecutive `CMSG_PING` packets arrive faster than 27 seconds and closes the socket after `> maxAllowed`, unless RBAC grants `RBAC_PERM_SKIP_CHECK_OVERSPEED_PING`.

---

## 5. Module dependencies

**Depends on:**
- `MovementInfo` / `MovementFlags` enum — for flag membership tests.
- `Unit::HasAuraType` — fly/hover/waterwalk aura presence.
- `RBAC` / `AccountTypes` — `SEC_PLAYER` exemption (GMs skip fly check).
- `Vehicle::GetVehicleInfo` — root exemption for fixed-position vehicles.
- `World::BanAccount` — escalation persistence.
- `KickPlayer` — disconnect path on policy violation.

**Depended on by:**
- `MovementHandler` — every movement opcode calls `ValidateMovementInfo`.
- `WorldSession::Update` / `WorldSocket` — every inbound packet runs through `DosProtection::EvaluateOpcode` before dispatch.
- `BattlegroundQueue` (passively — ban affects queue eligibility).

---

## 6. SQL / DB queries (if any)

| Statement / Source | Purpose | DB |
|---|---|---|
| `INSERT INTO account_banned (id, bandate, unbandate, bannedby, banreason, active, unbanned)` | persisted by `BanMgr::BanAccount` when policy = BAN | auth |
| `INSERT INTO ip_banned (ip, bandate, unbandate, bannedby, banreason)` | for `BAN_IP` mode | auth |
| `UPDATE account SET locked = 1 WHERE id = ?` | account lock on severe DOS escalation | auth |

No DBC/DB2 store reads from anticheat — all rules are hard-coded.

---

## 7. Wire-protocol packets (if any)

Anticheat is reactive — it does not originate opcodes, it inspects them. Touched (read-only) opcodes:

| Opcode | Direction | Inspected by | Action on violation |
|---|---|---|---|
| `CMSG_MOVE_*` (60+ variants) | C→S | `HandleMovementOpcode` → `ValidateMovementInfo` | Strip flags (silent) |
| `CMSG_MOVE_FORCE_*_SPEED_CHANGE_ACK` (9) | C→S | `HandleForceSpeedChangeAck` | Record correction/no-resync in this legacy tree, or KickPlayer |
| `CMSG_MOVE_TIME_SKIPPED` | C→S | `HandleMoveTimeSkippedOpcode` | Log only |
| `CMSG_MOVE_KNOCK_BACK_ACK` | C→S | `HandleMoveKnockBackAck` | Validate + relay `MoveUpdateKnockBack` |
| `CMSG_MOVE_SET_COLLISION_HEIGHT_ACK` | C→S | `HandleSetCollisionHeightAck` | Validate |
| **all** CMSG | C→S | `DosProtection::EvaluateOpcode` | Log/Kick/Ban |
| `SMSG_FORCE_*_SPEED_CHANGE` (9) | S→C | emitted by productive `Unit::SetSpeedRate`; not emitted by this legacy correction branch because the rate is unchanged | — |
| `SMSG_MOVE_KNOCK_BACK` | S→C | emitted on knockback | — |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-world/src/handlers/movement.rs` — N=~700 — has **one** anticheat-shaped check at line 99-104:
  ```rust
  // Validate: position must be finite (anti-cheat sanity check).
  let pos = &info.info.position;
  if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite() {
      warn!(account = self.account_id, "Invalid movement position: {pos:?}");
      return;
  }
  ```
  Plus a GUID-mismatch reject (lines 86-97) which is a related sanity check.
- (No other files contain anticheat logic. `grep -rni "anticheat\|speed_check\|fly_hack" crates/` returns 0 hits — the repo has zero structured anticheat surface.)

**What's implemented:**
- NaN/Inf position reject.
- GUID-mismatch reject ("movement from a guid that isn't this session's character" — equivalent to part of `HandleMovementOpcode` line 333 trace).

**What's missing vs C++:**
- All 14 `ValidateMovementInfo` rules.
- Speed-mismatch kick (`HandleForceSpeedChangeAck`).
- Fall-damage server-authoritative computation.
- DOS / packet-rate protection (`DosProtection::EvaluateOpcode` + the per-opcode threshold table).
- Ban / kick policy escalation (no `world.policy` config consumed).
- `MaxOverspeedPings` socket-ping counter (`WorldSocket::HandlePing`), now represented in RustyCore for normal sockets; RBAC-exact bypass remains pending with RBAC.
- Mover-mismatch warning (`HandleSetActiveMoverOpcode`).
- Move-time-skipped sanity logging.

**Suspicious / likely divergent:**
- No session-level "this player is being moved by who" tracking → cannot implement mover-mismatch check (rule analogue at `MovementHandler.cpp:543`).

**Tests existing:** represented AntiDOS unit tests cover `PacketSpoof.Policy` log/kick/ban staging behavior, C++ `HotfixRequest` limit `1/sec`, unlimited `PlayerLogin`, account-ban staging, and IP-ban staging from the real socket address. Movement anticheat also covers GUID mismatch and invalid-coordinate rejection without state mutation or broadcast.

---

## 9. Migration sub-tasks

- [x] **#AC.1** Create `crates/wow-anticheat/` skeleton crate with `pub fn validate_movement_info(&mut MovementInfo, &PlayerState) -> ValidationResult`. The crate is workspace-owned, independent from `wow-world`, and has C++-ordered rule evidence for the portable `Player::ValidateMovementInfo` state. (M)
- [ ] **#AC.2** Port all 14 `ValidateMovementInfo` rules with one unit test per rule. `wow-anticheat` now owns the portable rule order/API, and `wow-world` delegates its represented sanitizer to that crate while preserving fixed-position vehicle `ROOT` ordering, aura/security exceptions, returned removed flags, and broadcast integration. Remaining exactness is full Unit/Aura/Vehicle runtime context and rule-level trace mapping from `ValidationResult`. (H)
- [x] **#AC.3** Implement represented speed ACK tracker in `crates/wow-world/src/session.rs`: `forced_speed_changes_like_cpp: [u8; 9]`, represented movement speed rates per move-type, C++ `playerBaseMoveSpeed * rate` expected speed, mismatch threshold `0.01`, transport bypass and event audit. (M)
- [x] **#AC.4** Wire `HandleForceSpeedChangeAck` / `HandleMoveSetModMovementForceMagnitudeAck`: Rust validates movement ACKs, decrements the C++ counters, skips pending forced changes, records correction when client speed is lower, kicks on client speed/magnitude above server truth, and mirrors the legacy correction path's no-packet behavior caused by `SetSpeedRate(GetSpeedRate())` returning early. Productive `Unit::SetSpeedRate` packet emission remains under Unit runtime, not this represented ACK slice. (M)
- [x] **#AC.5a** Implement represented `DosProtection` in `WorldSession`: per-session opcode counters keyed by `ClientOpcodes`, evaluated before packets enter `pending_packets`; `Policy=0` logs/allows, `Policy=1` kicks, `Policy=2` kicks and stages a C++-style ban plan. (M)
- [x] **#AC.5b** Finish `Policy=2` ban persistence using a real `World::BanAccount` equivalent for `auth.account_banned` / `auth.ip_banned`. Account/IP rows are now persisted from `WorldSession::process_pending()` using `UPD_ACCOUNT_NOT_BANNED` + `INS_ACCOUNT_BANNED` or `INS_IP_BANNED`, with `BAN_IP` sourced from the real `AccountInfo.client_address` set by the accept loop. Rust also mirrors the online-session eviction shape: account bans target that account id directly; IP bans query affected accounts with `SEL_ACCOUNT_BY_IP`; matching active sessions receive `KickLikeCpp` through `PlayerRegistry`. (M)
- [x] **#AC.6** Complete the per-opcode rate-limit table from `WorldSession.cpp:1313-1500` exhaustively. Rust now mirrors the C++ limit groups (`0`, `200`, `50`, `MAX_QUEST_LOG_SIZE`, `20`, `10`, `3`, `PLAYER_SLOTS_COUNT`, `1`, default `100`) and has a table-level regression test. (M)
- [x] **#AC.7a** Plumb `PacketSpoof.Policy` / `PacketSpoof.BanMode` / `PacketSpoof.BanDuration` from `WorldServer.conf` → `wow-config` → `WorldSession`. (L)
- [x] **#AC.8** Implement ban/kick escalation: `KickPolicy::Ban` writes to `auth.account_banned` or `auth.ip_banned`, drops the offending connection, and queues C++-style kicks for other online sessions matched by the affected account/IP set. (M)
- [x] **#AC.9** Server-authoritative fall-damage: track previous Z, on land compute `fall_distance`, apply environmental damage if > 13 yards. Mirrors `Player::HandleFall` with represented safe-fall/feather/fly/god/GM/immunity guards and update/log packets. (M)
- [x] **#AC.10** Move broadcast in `movement.rs:127` to **after** validation strip; peers now receive the sanitized `MovementInfo` flags, matching C++ `ValidateMovementInfo` before `SendMessageToSet`. (L — but high-impact correctness fix)
- [x] **#AC.11** Add structured trace event `anticheat.violation { rule, account, character, opcode, severity }` for represented movement anticheat decisions: movement GUID/coordinate rejects, `ValidateMovementInfo` flag strips, speed ACK correction/kick and movement-force magnitude kick. (L)
- [ ] **#AC.12** Add `wow-anticheat` integration test that replays a known-bad fly-hack capture and asserts the strip + kick path. (M)

---

## 10. Regression tests to write

- [x] Test: `MOVEMENTFLAG_ROOT | MOVEMENTFLAG_FORWARD` without fixed-position vehicle → after `validate_movement_info`, `ROOT` is stripped first and `FORWARD` remains, matching actual C++ rule order.
- [x] Test: fixed-position vehicle context preserves `MOVEMENTFLAG_ROOT`, but strips `MOVEMENTFLAG_MASK_MOVING` when `ROOT` is combined with movement flags.
- [x] Test: `MOVEMENTFLAG_FLYING` from non-GM with no fly aura → flag stripped.
- [x] Test: `MOVEMENTFLAG_HOVER` with hover aura present → not stripped.
- [x] Test: each C++ incompatible pair is stripped independently: `ASCENDING|DESCENDING`, `LEFT|RIGHT`, `STRAFE_LEFT|STRAFE_RIGHT`, `PITCH_UP|PITCH_DOWN`, `FORWARD|BACKWARD`.
- [x] Test: `LEFT | RIGHT` simultaneously → both stripped.
- [x] Test: `MOVEMENTFLAG_WATER_WALK` is preserved for represented `SPELL_AURA_GHOST`, matching the C++ ghost exception.
- [x] Test: `MOVEMENTFLAG_FLYING | MOVEMENTFLAG_CAN_FLY` is preserved for GM/security-bypassed movement, matching `GetSession()->GetSecurity() != SEC_PLAYER`.
- [x] Test: `MOVEMENTFLAG_FLYING | MOVEMENTFLAG_CAN_FLY` is preserved for represented `SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED`.
- [x] Test: standalone `wow-anticheat::validate_movement_info` preserves the C++ rule order and mutation-only behavior independently of `WorldSession`.
- [x] Test: `WorldSession::sanitize_movement_info_flags_represented_like_cpp` delegates through `wow-anticheat` without changing the represented movement tests.
- [x] Test: speed ack 8.0 vs server 7.0 with no transport → kick fires; ban table not touched (kick policy).
- [x] Test: speed ack 6.0 vs server 7.0 → correction is recorded, no kick, and no packet is emitted because legacy `SetSpeedRate(GetSpeedRate())` returns early on unchanged rate.
- [x] Test: repeated `CMSG_PLAYER_LOGIN` packets in one second remain allowed because C++ puts it in the zero-limit group (`maxPacketCounterAllowed == 0`); do **not** add a kick expectation here.
- [x] Test: NaN x in position → reject without panic, do not advance `player_position`, do not broadcast.
- [x] Test: GUID mismatch → reject without state mutation, do not broadcast.
- [x] Test: SPLINE_ELEVATION flag with elevation == 0 → flag stripped; non-zero elevation without flag → flag added.
- [ ] Test: golden — feed a 60-second movement capture, assert exactly N flag-strip events occur (regression-fence the rule set).

---

## 11. Notes / gotchas

- `ValidateMovementInfo` **mutates** the incoming `MovementInfo` and returns `void`. There is no "reject this packet" path — it always returns a sanitized version, and the player keeps moving with the cleaned-up flags. That choice is deliberate (latency tolerance — peers were getting kicked for legitimate desync). When porting, do **not** convert this to a fail-closed reject path.
- `WorldSession::HandleMovementOpcode` calls `ValidateMovementInfo` before GUID/position rejection in C++ (`MovementHandler.cpp:328-337`); Rust's represented handler now preserves that order before rejecting mismatched GUIDs or invalid coordinates.
- The fly check (rule 11) is bypassed for `SEC_PLAYER == false` accounts. RustyCore must consult `RBAC::GetSecurity()` analogue, not just an `is_gm` flag.
- The speed-mismatch kick (rule in `HandleForceSpeedChangeAck`) trips even with **client-side latency spikes**. TC accepts that false-positive rate; our doc must not advertise it as a perfect detector.
- `m_forced_speed_changes` exists specifically to absorb the mount/run double-ack quirk — without it, every mount toggle kicks the player. Port faithfully.
- `DosProtection::EvaluateOpcode` runs in the network thread, before `STATUS_LOGGEDIN` is enforced. Caps for unauthenticated opcodes (`CMSG_AUTH_SESSION` etc.) are critical — set them to **1/sec**.
- The default (unlisted) cap is **0**, which means "no cap" — the explicit list is the entire policy. Do not invert this in Rust to fail-closed; that would break dozens of legitimate high-rate opcodes (movement, spell cast).
- `MaxOverspeedPings = 2` (worldserver.conf line 296) is active in this C++ tree through `WorldSocket::HandlePing`, not the speed-ACK handler. RustyCore now mirrors the counter in `wow-network::WorldSocket`; the exact RBAC bypass remains a follow-up.
- Wallhack / position-jump detection (a "you teleported 50 yards in 1 tick" check) does **not** exist in TC inline anticheat. The closest analogue is the dynamic visibility window — unrealistic positions simply move out of range. Don't promise a wallhack detector that isn't in the upstream.
- Warden does the actual binary-tampering / module-injection detection. Anything in this doc is **complementary**, not a replacement.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `Player::ValidateMovementInfo(MovementInfo*)` | `wow_anticheat::validate_movement_info(&mut MovementInfo, &PlayerState) -> ValidationOutcome` | Mutate in place; return enum {`Clean`, `Stripped(Vec<Rule>)`}. |
| `WorldSession::DosProtection` | `wow_anticheat::DosProtection { throttle: DashMap<ClientOpcodes, PacketCounter>, policy: KickPolicy }` | One per `WorldSession`. |
| `PacketCounter` | `struct PacketCounter { last_seen: Instant, amount: u32 }` | Unchanged. |
| `DosProtection::Policy` | `enum KickPolicy { Log, Kick, Ban }` | 1:1. |
| `BanMode` | `enum BanMode { Account, Ip }` | Drop `BAN_CHARACTER` (folded). |
| `m_forced_speed_changes[MAX_MOVE_TYPE]` | `[u32; 9]` field on session | 1:1. |
| `MAX_MOVE_TYPE` | `pub const MAX_MOVE_TYPE: usize = 9;` | Constant. |
| `KickPlayer(reason: &str)` | `WorldSession::kick(reason: KickReason)` (existing) | Reason becomes structured enum. |
| `World::BanAccount(...)` | `wow_database::ban_account(...)` writing to `auth.account_banned` | Already async-ready in DB layer. |

---

## 13. §13 Audit (cross-cutting reference docs)

| Claim | Verified against | Verdict |
|---|---|---|
| `ValidateMovementInfo` lives at Player.cpp:28435 | `grep -n "void Player::ValidateMovementInfo" Player.cpp` → 28435 | ✅ |
| 14 rules in the validator | counted `REMOVE_VIOLATING_FLAGS` macros + the inverse SPLINE_ELEVATION block, lines 28456-28518 | ✅ |
| `DosProtection::EvaluateOpcode` at WorldSession.cpp:1259 | `grep -n DosProtection::EvaluateOpcode` → 1259 | ✅ |
| `GetMaxPacketCounterAllowed` switch cases | `grep -nE "case CMSG_" WorldSession.cpp` between 1313-1500 → ~190 cases | ✅ approximate |
| Conf keys `PacketSpoof.Policy/BanMode/BanDuration` at lines 4308/4317/4327 | `grep -n "PacketSpoof" worldserver.conf.dist` → 4308, 4317, 4327, 4314, 4324, 4333 | ✅ |
| `MaxOverspeedPings = 2` at worldserver.conf line 296 | `grep -n "MaxOverspeedPings" worldserver.conf.dist` → 296, 297, 302 | ✅ |
| `HandleForceSpeedChangeAck` kick path at line 470-541 | `grep -n "HandleForceSpeedChangeAck" MovementHandler.cpp` → 470 | ✅ |
| Rust state: 1 finite-position check, 0 anticheat surface | `grep -rni "anticheat" crates/` returns zero structural matches; `movement.rs:99-104` is the only check | ✅ |
| All CMSG_MOVE_* opcodes channel through `ValidateMovementInfo` | every `Handle*` in MovementHandler.cpp calls `ValidateMovementInfo` (12 call sites grep) | ✅ |
| `m_forced_speed_changes[MAX_MOVE_TYPE]` exists | `grep -n m_forced_speed_changes Player.h` (search confirms field) | ✅ |

**Open audit items:**
- The exact count of switch cases in `GetMaxPacketCounterAllowed` (~190) is approximate; an exhaustive table is deferred to #AC.6 implementation.
- "No wallhack detector exists in TC" claim verified by absence of any `Wallhack` / `LineOfSight` server-side movement reject in MovementHandler.cpp (only `Map::IsInLineOfSight` for spell casts, not movement). ✅ but admitted as a soft claim.
- "Warden is separate" — confirmed via `grep -rln Warden src/server/` showing a separate `WardenMgr` / `WardenWin` / `WardenMac` subsystem under `src/server/game/Warden/`. Out of scope for this doc.

**Result:** ⚠️ partial — primary call sites, line numbers, rule counts, and conf keys verified; full per-opcode DOS-cap table is enumerated only by sampling, not exhaustively.

---

*Template version 1.0. Last updated 2026-05-01.*
