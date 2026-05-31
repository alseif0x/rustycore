# C++ to Rust Porting Methodology

This document is the operating method for the RustyCore C++ -> Rust port.
It is intentionally strict. The goal is not to make Rust compile with similar
names; the goal is observable parity with the legacy C++ implementation, with
no hidden gaps.

Primary C++ source of truth:

- `/home/server/woltk-trinity-legacy`

Secondary logic reference, only when legacy is incomplete or suspicious:

- archived TrinityCore 3.3.5, for logic comparison only
- do not copy code from it
- record when it was used and why the legacy source was insufficient

## Non-negotiable Rules

1. Do not trust existing Rust code.
   Existing Rust may be useful context, but every behavior must be contrasted
   against C++ before it is accepted.

2. Do not trust previous AI work.
   A commit, test, roadmap checkbox, or comment is not proof. C++ is proof.

3. Do not mark roadmap tasks as done for scaffolding.
   Adding fields, enums, snapshots, queues, or placeholder hooks is not a
   complete port unless the runtime behavior is actually wired and tested.

4. Every closed item needs a C++ anchor.
   A task is only closeable when the exact C++ file/function/line range has
   been reviewed and the Rust behavior is either equivalent or the divergence
   is explicitly documented.

5. Do not port suspected C++ bugs blindly.
   The legacy C++ implementation is the behavioral baseline, but it is not
   assumed infallible. If contrast reveals a likely C++ bug, undefined edge,
   or internally inconsistent behavior, stop and record it as a legacy
   divergence candidate with exact anchors, observed impact, proposed Rust/C++
   correction, and tests. Do not silently "fix" it in Rust, and do not silently
   copy it into Rust as if it were validated behavior.

6. Prefer small, mergeable, behavior-complete slices.
   A good slice is: C++ branch -> Rust implementation -> positive and negative
   tests -> roadmap update -> checks.

7. Runtime source of truth must move toward canonical entities.
   If a canonical `Unit`, `Player`, `Creature`, `Map`, or subsystem exists,
   prefer it over legacy `WorldSession` mirrors. Legacy mirrors can be used only
   as a documented fallback while migration is incomplete.

8. A fallback must never override a known canonical empty state.
   If the canonical player exists and `Player.unit().attacking()` is `None`,
   do not resurrect combat from `WorldSession::combat_target`. Fallbacks are
   only for "canonical entity does not exist yet", not "canonical says none".

9. Tests must cover C++ early returns.
   Porting only success paths creates false confidence. Every C++ guard that
   returns early needs a negative test whenever it is representable.

10. Keep the roadmap honest.
   The roadmap is the source of truth for what is done, partial, blocked, and
   still missing. It must not be used as a progress scoreboard.

11. Do not start the server unless explicitly requested.
    Development verification normally means unit/integration checks. Manual
    client/server testing is a separate step.

## Required Work Order For Each Task

### 1. Identify the exact C++ behavior

Before editing Rust, locate the C++ implementation.

Record:

- C++ file path
- function or method name
- relevant line range
- branch or condition being ported
- side effects, state writes, packet sends, callbacks, and early returns

Example:

```text
C++ anchor:
/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp
Unit::DoMeleeAttackIfReady, lines 2085-2156

Relevant rules:
- return if !UNIT_STATE_MELEE_ATTACKING
- return if UNIT_STATE_CHARGING
- base attack error sends SetAttackSwingError
- reset base timer only after attack attempt branch
```

If C++ calls into another function, inspect that function too. Do not stop at
the top-level handler if the real behavior is in `Unit`, `Player`, `Map`,
`Vehicle`, `MotionMaster`, `ThreatManager`, etc.

### 2. Classify the slice

Classify the task before implementation:

- `wire`: packet layout or serialization only
- `state`: entity fields, flags, timers, counters, or snapshots
- `runtime`: live behavior with real state transitions
- `side_effect`: packet sends, AI callbacks, threat, aura removal, movement,
  spell interruption, combat refs
- `bridge`: temporary glue from `WorldSession` or legacy maps to canonical
  entities
- `scaffold`: data shape prepared for later runtime wiring

Only `runtime`, `wire`, `state`, and `side_effect` can usually be marked done.
`scaffold` should be documented as partial unless it has complete behavior
behind it.

### 3. Map C++ state to Rust state

Make an explicit mapping:

```text
C++                         Rust
Unit::m_attacking           Unit::subsystems.combat.attacking_guid
Unit::m_attackTimer[]       Unit::attack_timer[]
Player::m_swingErrorMsg     WorldSession::player_swing_error_msg_like_cpp
VehicleSeatEntry::Flags     player_vehicle_seat_flags_like_cpp
```

For each mapped state, answer:

- Is Rust state canonical or a temporary mirror?
- Who writes it?
- Who reads it?
- Does it survive sync/refresh/relogin?
- Is there a test preventing accidental overwrite?

If Rust has both canonical and legacy mirrors, document which one wins and when.

### 4. Implement conservatively

Implementation rules:

- Prefer local patterns already in the repository.
- Keep changes scoped to the C++ behavior being ported.
- Do not introduce broad abstractions just to make a slice look elegant.
- Do not change unrelated warnings, formatting, or docs.
- Do not silently change public packet layout or DB assumptions.
- Use typed flags and structured parsers where available.
- Use `apply_patch` for manual edits.

When a C++ dependency does not exist yet in Rust, use one of these patterns:

- Add a clearly named represented field with `_like_cpp`.
- Gate behavior behind `*_represented` booleans if the runtime does not always
  know the value.
- Keep default behavior conservative so existing flows are not changed by fake
  data.
- Add a roadmap note saying what real runtime still needs to feed the snapshot.

Do not fake full behavior by setting defaults that make tests pass.

### 5. Add tests before declaring closure

For every C++ branch ported, add tests for:

- success path
- at least one negative/early return path
- state mutation
- side effects, if any
- fallback behavior, if migration uses temporary legacy mirrors

For C++ early returns, a test should assert what does not happen:

- no damage
- no timer reset, if C++ would not reset
- no packet
- no threat
- no attacker registration
- no target set
- no state transition

Example of a required negative test:

```text
C++: Unit::DoMeleeAttackIfReady returns if !UNIT_STATE_MELEE_ATTACKING.
Rust test must create a canonical player with a victim but without
MELEE_ATTACKING and assert:
- creature HP unchanged
- player attack timer unchanged except where C++ would update it
- no AttackerStateUpdate packet
```

### 6. Update the roadmap precisely

Roadmap status language:

- `[x]` only when the behavior is complete for the stated scope
- `represented` when data shape exists but runtime is not fully wired
- `partial` when some C++ branches are implemented and others remain
- `blocked` when a missing subsystem prevents correct implementation
- `TODO` when a known gap is intentionally deferred

Bad roadmap entry:

```text
[x] IsValidAttackTarget complete
```

Better roadmap entry:

```text
[x] IsValidAttackTarget represented flags: unattackable, taxi, GM, immunity.
[ ] IsValidAttackTarget runtime: visibility, faction, reputation, duel,
    sanctuary, PvP, traps, affecting player resolution.
```

## Audit Method For Work Done By Another Agent

When reviewing commits from another agent, use this sequence.

### 1. Establish the base

Find the last trusted commit:

```bash
git log --oneline --decorate --graph -20
git reflog --date=iso -20
git status --short --branch
```

Then inspect all later work:

```bash
git diff --stat <trusted-base>..HEAD
git diff --name-status <trusted-base>..HEAD
git log --reverse --format='%h %an <%ae> %ad %s' --date=iso <trusted-base>..HEAD
```

### 2. Read roadmap changes first

Roadmap changes reveal what the agent believes is done.

```bash
git diff <trusted-base>..HEAD -- docs/MIGRATION_ROADMAP.md
```

Treat every new `[x]` as a claim that must be proven against C++.

### 3. Inspect risky modules by behavior, not by commit message

Search for the behavior names and C++ terms:

```bash
rg -n "UnitAttackContextLikeCpp|IsValidAttackTarget|tick_combat_sync|DoMeleeAttackIfReady|AttackStop|Vehicle|Threat|Aura|Motion" crates docs
```

Then open focused ranges with `sed` or `nl -ba`.

### 4. Contrast with C++ line by line

For each claimed behavior:

- Open Rust implementation.
- Open C++ implementation.
- Compare branch order.
- Compare early returns.
- Compare side effects.
- Compare packet sends.
- Compare state writes and cleanup.

Branch order matters. C++ often intentionally checks duel before sanctuary,
visibility before dead target, or attack errors before timer reset. Preserve
order unless there is a documented reason.

### 5. Check for test quality

Tests are insufficient if they:

- manually set impossible Rust states
- skip the C++ guard being claimed
- only test success paths
- assert behavior that contradicts C++
- use session mirrors when canonical state should be source of truth
- pass because defaults hide missing runtime data

Every bug found in audit should get a regression test before fixing.

### 6. Run focused checks

Run tests closest to the modified behavior:

```bash
cargo test -p wow-entities <filter>
cargo test -p wow-world <filter>
cargo fmt --check
git diff --check
cargo check -p world-server
```

Do not rely on a broad `cargo check` alone. It only proves compilation.

## C++ Contrast Checklist

Use this checklist before closing any behavior.

### Function Shape

- [ ] Same entry condition?
- [ ] Same null/self checks?
- [ ] Same alive/in-world checks?
- [ ] Same mounted/vehicle checks?
- [ ] Same GM/admin checks?
- [ ] Same state flags?
- [ ] Same aura checks?
- [ ] Same spell attribute exceptions?
- [ ] Same branch order?
- [ ] Same return behavior?

### State Writes

- [ ] Same target/victim assignment?
- [ ] Same attacker set update?
- [ ] Same threat update?
- [ ] Same combat refs?
- [ ] Same unit flags?
- [ ] Same timers?
- [ ] Same cooldowns?
- [ ] Same movement state?
- [ ] Same charm/control state?
- [ ] Same faction/reputation state?

### Side Effects

- [ ] Same packet sent?
- [ ] Same packet suppression rules?
- [ ] Same AI callback?
- [ ] Same aura removal?
- [ ] Same spell interruption?
- [ ] Same movement stop/start?
- [ ] Same death cleanup?
- [ ] Same pet/control callback?
- [ ] Same script hook?
- [ ] Same map/cell update?

### Runtime Ownership

- [ ] Which Rust entity owns this state?
- [ ] Is this canonical or legacy?
- [ ] Can a sync overwrite it?
- [ ] Does logout/map transfer affect it?
- [ ] Does it need DB persistence?
- [ ] Does it need packet broadcast?

## Canonical vs Legacy Policy

RustyCore currently contains both canonical entity state and older session/map
mirrors. The migration must continuously reduce legacy dependency.

Preferred order:

1. Canonical `wow_entities` model.
2. Canonical `wow_map` map-owned entity.
3. Map manager bridge if still required.
4. `WorldSession` represented mirror.
5. Legacy fields only as explicit fallback.

Rules:

- Canonical state wins over mirrors.
- Session mirrors may cache, but must not invent canonical behavior.
- If canonical state exists and says "none", do not fallback to a stale mirror.
- Sync functions must preserve live canonical state such as attacking target,
  timers, flags, player flags, current spells, auras, threat, and control state.
- If a sync currently preserves only part of that state, document the missing
  fields and add tests before relying on it.

## Roadmap Closure Levels

Use these closure levels in notes and reviews.

### L0: Not Started

No meaningful Rust shape exists.

### L1: Constants/Wire Shape

Enums, flags, packet layout, DTOs, or parser shape exists.
This is not runtime behavior.

### L2: Represented State

Rust has fields/snapshots for the C++ data and tests for direct helper behavior.
Runtime may still not feed the values.

### L3: Runtime Hooked

Real runtime code feeds the state and produces observable behavior for a
specific flow.

### L4: Integrated Behavior

The behavior is connected through canonical entities, packets, map ownership,
and side effects for the target flow.

### L5: Full Parity For Scope

All C++ branches for the declared scope are covered, including negative cases,
side effects, and cleanup. The roadmap may mark this exact scope `[x]`.

Do not call a task "done" unless it has reached L5 for its stated scope.
Partial scopes can be marked done only if the remaining scope is explicitly
split out.

## Example: Correcting Combat Tick Work

The C++ anchor:

```text
/home/server/woltk-trinity-legacy/src/server/game/Entities/Unit/Unit.cpp
Unit::DoMeleeAttackIfReady, lines 2085-2156
```

C++ starts with:

```text
if (!HasUnitState(UNIT_STATE_MELEE_ATTACKING))
    return;

if (HasUnitState(UNIT_STATE_CHARGING))
    return;
```

Correct Rust requirements:

- `tick_combat_sync` must not deal melee damage unless the canonical `Unit`
  has `MELEE_ATTACKING`.
- `CHARGING` must return without resetting attack timer.
- `NotInRange` and `BadFacing` set the relevant attack timer to 100 ms and
  send/suppress `AttackSwingError` like `Player::SetAttackSwingError`.
- If a base attack has `CURRENT_MELEE_SPELL`, cast/finish represented spell
  instead of white damage.
- If `AttackerStateUpdate` returns early due to `PACIFIED`,
  `CANNOT_AUTOATTACK`, disabled attacking aura, dead victim, or LOS, timer reset
  must follow C++ behavior from the caller.

Required tests:

- player has victim but no `MELEE_ATTACKING`: no damage, no packet
- charging player: no damage, no timer reset
- out of range: no damage, base timer 100, one error packet
- repeated same error: no duplicate packet
- bad facing: no damage, base timer 100, error reason 1
- current melee spell: no white damage, spell consumed, timer reset
- PACIFIED: no damage, timer reset after attempted attack branch
- target dead: no damage and cleanup matches C++

## Example: Correcting IsValidAttackTarget Work

The C++ anchor:

```text
/home/server/woltk-trinity-legacy/src/server/game/Entities/Object/Object.cpp
WorldObject::IsValidAttackTarget, around lines 2993-3145
```

Split into explicit subscopes:

1. Basic rejects:
   self, unattackable, GM, dead, untargetable, uninteractible, player uber.

2. Unit flags and immunity:
   `NON_ATTACKABLE`, `ON_TAXI`, `NOT_ATTACKABLE_1`,
   `IMMUNE_TO_PC`, `IMMUNE_TO_NPC`, `PLAYER_CONTROLLED`.

3. Visibility:
   `CanSeeOrDetect`, spell attributes, area-affecting behavior.

4. Creature-vs-creature:
   hostile in either direction.

5. Trap/gameobject owner:
   GO trap owner as unit or ownerless trap behavior.

6. Friendly reject:
   `IsFriendlyTo(target) || target->IsFriendlyTo(this)`.

7. Affecting player resolution:
   `GetAffectingPlayer`, pets, mounted owner immunity.

8. Reputation:
   contested guard, forced rank, faction state, `AtWar`.

9. Duel:
   duel opponent and `DUEL_STATE_IN_PROGRESS`.

10. Sanctuary:
    both player controlled and sanctuary flags.

11. Final PvP:
    target PvP, FFA PvP, `UNIT_BYTE2_FLAG_UNK1`.

Each subscope can be marked done only when either runtime is wired or the
roadmap clearly says it is represented-only.

## Commit Hygiene

A good commit contains:

- focused implementation
- tests for the same behavior
- roadmap update for the same behavior
- no unrelated formatting churn
- no unrelated warning cleanup
- no server process changes unless requested

Commit message should name the behavior, not the internal refactor.

Good:

```text
Respect C++ melee attacking guard in combat tick
```

Bad:

```text
Update session combat stuff
```

Before commit:

```bash
cargo fmt --check
git diff --check
cargo test -p <crate> <focused-filter>
cargo check -p world-server
git status --short
```

## When To Ask For Manual Client Testing

Manual client testing is useful only after a server-visible slice is coherent.

Ask for testing when:

- login/realm/character flow changed
- packet layout changed
- combat packets changed
- movement packets changed
- DB/config/server process changed
- client-visible state changed

Do not ask for manual testing just because unit tests passed.

When reporting a testable point, include:

- what to test
- what should happen
- what should not happen
- whether C++ server must be stopped
- whether Rust server was started
- estimated progress percentage

## Progress Percentage Policy

Progress percentage is an engineering estimate, not a promise.

Increase percentage only when:

- runtime behavior is completed, not just represented
- major gaps are actually closed
- tests cover both positive and negative C++ paths
- roadmap has fewer real unknowns

Decrease percentage when:

- audit finds fake closure
- roadmap split reveals more real work
- runtime gap was hidden by snapshots
- tests validate behavior that contradicts C++

Example:

- Adding 20 snapshot fields for PvP does not move progress much.
- Wiring real `IsValidAttackTarget` with faction/reputation/duel/PvP and tests
  does move progress.

## Review Template

Use this format when reviewing another agent's work.

```text
Base reviewed:
- trusted commit: <hash>
- reviewed range: <hash>..<hash>

Claims reviewed:
- <roadmap id>: <claim>

C++ anchors:
- <path>:<line> <function>

Findings:
1. Severity: <bug/risk/gap>
   Rust: <path:line>
   C++: <path:line>
   Issue: <specific mismatch>
   Required fix: <specific action>

Checks run:
- <command>: pass/fail

Verdict:
- accept / accept with TODO / do not accept

Progress estimate:
- <percent> and reason
```

## Immediate Known Lessons From Recent Audit

Recent audit after work by another agent found these concrete lessons:

1. Tests can pass while validating non-C++ behavior.
   Example: setting `Unit.attacking()` manually without `MELEE_ATTACKING` and
   expecting melee damage contradicts `Unit::DoMeleeAttackIfReady`.

2. `Option<T>` can hide state source ambiguity.
   `None` can mean "canonical player not found" or "canonical player found with
   no victim". Those cases must be represented differently.

3. Roadmap checkboxes can overstate scaffolding.
   `IsValidAttackTarget` snapshots are useful, but the full runtime remains open
   until real visibility/faction/reputation/duel/sanctuary/PvP data is wired.

4. Broad commits are harder to trust.
   A large first commit that changes validation, combat tick, packets, tests,
   and roadmap at once should be audited by C++ behavior area, not by commit
   message.

## Final Principle

The port is complete only when the Rust server no longer needs the C++ server as
a behavioral reference for the covered scope. Until then, every change must make
the gap smaller, visible, and tested.
