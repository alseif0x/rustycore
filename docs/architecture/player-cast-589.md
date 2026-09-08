# Represented Player cast-request lifecycle — #589

Status, 2026-09-08: **implemented and locally accepted**, delivered as one
macro on `589-archcore-complete-represented-player-cast-request-lifecycle`.
The earlier "paused, uncommitted" state is superseded: the inherited working
tree is committed, the six sub-issues #590–#595 are executed, and the evidence
below was actually run at the recorded SHA. Earlier #587/#588 acceptance does
not validate these changed inputs; this branch carries all three deliveries.

Sub-issue map: #590 functional contract, identity and consumers; #591 payload
and publication; #592 ownership and physical organization; #593 acceptance bot;
#594 integral regression and paired QA; #595 evidence, publication and closure.

## Ownership and consumers

`wow-world::player_cast` coordinates normal request admission and preparation.
Its private Session adapters resolve canonical state, catalogs, power, wire
payload and packet publication. Pending state stays in Player gameplay state;
active execution and cooldowns stay in Unit spell execution. The existing
Session driver completes active casts before admitting pending casts. There is
no new timer, task or mutable state mirror in production.

Normal immediate and queued requests use the same preparation path, retain the
client request ID until SpellPrepare maps it to a map-allocated server Cast GUID,
and stamp active preparation with the canonical residence revision. Instance-local
Map allocation is shared with represented creature casts. Stale or detached Player
handles cannot allocate; a prepared cast cannot execute after residence reentry,
and that fence now covers any residence-stamped prepared cast rather than only
client requests, so a timed toy cast is fenced too.

Toy, binder, both spell-click consumers, first-login create-mode spells,
item-obtain spells and self-resurrection all adapt to fallible canonical GUID
allocation. The last three previously reached the shared executor with an EMPTY
cast id and a default metadata; they now allocate from the admitted Map's shared
`HighGuid::Cast` sequence through `execute_server_triggered_spell_like_cpp` and
carry explicit metadata for their own C++ trigger contract. Normal client
defaults are not imposed on any of them. Loot, movement/stance, channel and
transfer interruption reach the same active state owner, and loot now also
reaches the canonical Unit current-spell slots.

Start/Go and interruption frames use the existing bounded durable directory rail
for observers, after recipient visibility and map selection. Session delivery
checks connection generation through the directory, current map and shared client
visibility storage. No packet delivery occurs while the canonical map guard is held.

## Intentional behavior repairs

These are #589 repairs, not claims of behavior-preserving movement:

- Immediate casts now prepare and start before Go; queued requests revalidate
  knowledge, override, checks and power before preparation completes.
- Normal server Cast IDs, empty OriginalCastID, effective server visual, Start/Go
  flags and power sections replace the former client-derived metadata.
- Preparation starts the represented GCD once; launch does not restart it.
  Active cancellation clears that preparation GCD. A late power failure retains it.
- A `TRIGGERED_FULL_MASK` server cast now carries `TRIGGERED_IGNORE_GCD` and no
  longer starts the player's global cooldown at launch. `TRIGGERED_NONE` server
  casts still start it, matching `Spell::prepare`.
- Cancellation uses SpellFailure then SpellFailedOther before Interrupted result.
  Late launch rejection uses its specific CastFailed before those interruption
  frames. Mismatched CancelCast still cancels pending requests when a cast is active.
- Every abandoned request reports. C++ `Player::CancelPendingCastRequest` exists
  precisely so the cast button cannot stay highlighted; a lost spell, cooldown
  owner, visual, allocation or residence install previously published nothing and
  now sends `SPELL_FAILED_DONT_REPORT`, with the client id before construction and
  the consumed server id after the SpellPrepare mapping.
- Power consumption rechecks resources under the same canonical guard as the
  debit, and cannot report success after losing the owner before deduction.
- `CanExecutePendingSpellCastRequest` also cancels when the casting unit is not in
  world or is no longer `GetUnitBeingMoved()`.
- Loot interruption reaches the canonical Unit slots, not only the represented
  cast execution state.

## Publication contract and represented value limits

`SendSpellStart` and `SendSpellGo` flags are assembled in
`session/player_cast/wire.rs` and the optional sections are filled exactly where
the flag selected them. Start carries `HAS_TRAJECTORY`, conditional
`POWER_LEFT_SELF` and, while the cast is timed, `IMMUNITY`. Go carries
`UNKNOWN_9`, conditional `POWER_LEFT_SELF`, `PROJECTILE`, the Death Knight
`NO_GCD | RUNE_LIST` pair, `RUNE_LIST` for `SPELL_EFFECT_ACTIVATE_RUNE`,
`ADJUST_MISSILE` for a trajectory request and `NO_GCD` when the spell has no
`StartRecoveryTime`. `CAST_FLAG_PENDING` stays triggered-only. Start samples
power before the debit, Go after it. The presence bits are serialized from the
sections, never inferred from the flags.

Four values degrade because their subsystem is unported, each the way C++ also
degrades for a caster with nothing to report. They are limits of value, not of
wire structure:

| Section | Represented value | C++ input not ported |
| --- | --- | --- |
| `RemainingRunes` | `Start`/`Count` zero, empty cooldowns | `m_runesState`, `Player::GetRunesState`; the C++ cooldown loop is itself commented out |
| `AmmoDisplayID` | zero, `AmmoInventoryType` absent | `GetSpellCastDataAmmo` thrown-weapon and "Requires No Ammo" branches; C++ also discards the inventory type it computes |
| `MissileTrajectory` | request pitch, zero `TravelTime` | `Spell::m_delayMoment` missile simulation |
| `Immunities.Value` | zero | `SpellInfo::GetMechanicImmunityMask` |

An earlier iteration of this work rejected projectile, rune, heal-prediction and
trajectory casts outright. That was withdrawn: refusing every Death Knight and
every ranged-slot spell is a functional regression C++ does not have, and the
sections are structurally representable.

A spell visual gated by a `CasterUnitConditionID` is still rejected explicitly,
because visual zero is not a valid silent substitute for an unevaluated
condition. The rejection now reports instead of dropping the request.

## Deviations deliberately left open

Recorded rather than silently closed, with the reason each is out of this
contract:

- `CanRequestSpellCast` consults one GCD value and the single active execution
  slot. C++ iterates `CURRENT_MELEE_SPELL` and `CURRENT_GENERIC_SPELL`. Two cast
  representations still coexist and their convergence belongs to #584, not here.
- `CanExecutePendingSpellCastRequest` blocks on any non-zero remaining cast time,
  where C++ allows immediate execution behind a channeled spell. Channels are
  explicitly outside #589.
- An instant cast re-runs `CheckCast` and takes power at launch, where C++ passes
  `skipCheck` to `_cast` for `willCastDirectly`. Both calls occur in the same tick;
  the recheck also resolves the spell focus object, so skipping it would change
  target selection. Left as a recorded ordering deviation.
- `record_cast_character_spell_cooldown_like_cpp` persists
  `recovery_time_ms.max(cooldown_ms)`, so a spell with only a `StartRecoveryTime`
  writes a persisted cooldown row C++ would not. Pre-existing, not introduced
  here; the runtime cooldown check short-circuits on `recovery_time_ms == 0`.

The full effects engine, SpellHistory, projectile simulation, channels and
autorepeat, pets and vehicles, and the complete target-selection rules remain
port work under #30 and #584.

## Executed local acceptance

Every command below was run in `/home/server/rustycore-cast-589` on aarch64,
sequentially, with `CARGO_BUILD_JOBS=1`, `--locked --offline`, and
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc` for workspace builds. The
standalone QA bot uses its own manifest. No command below is reported from an
earlier run against different inputs.

| Command | Result |
| --- | --- |
| `cargo test -p wow-packet --lib` | 728 passed, 0 failed |
| `cargo test -p wow-entities --lib` | 725 passed, 0 failed |
| `cargo test -p wow-map --lib` | 727 passed, 0 failed, 1 ignored |
| `cargo test -p wow-world --lib` | 3822 passed, 0 failed |
| `cargo test -p world-server` | 570 passed, 0 failed (plus two empty targets) |
| `cargo test -p capture-diff` | 157 passed across 17 targets, 0 failed |
| `cargo test --manifest-path tools/wow-test-bot/Cargo.toml --bin wow-test-bot` | 179 passed, 0 failed |
| `session-ownership-check check --syntax-only` | PASS: 55 impl owners, 3761 exact associated items, 40 SessionCommand variants, 594 registry rows |
| `check_architecture.py check` | PASS: physical ratchet, dependencies, ownership, hotspot ratchet |
| `check_architecture.py self-test` | PASS: 20 fixtures |
| `cargo fmt --all -- --check` and `git diff --check` | clean |

The `wow-map`, `capture-diff` and QA-bot results were produced earlier in the
same campaign and are reused only because their inputs did not change
afterwards; every later edit was confined to `wow-packet`, `wow-world` and the
architecture policies, which were rerun in full. The ownership checker was
built from this checkout, not from a previously installed binary.

### Defects the campaign found

The packet, character and session suites had never been executed against the
inherited working tree; running them surfaced six real problems, all fixed
here rather than accommodated:

1. `spell_cast_data_writes_nonempty_fields_in_cpp_order` asserted the payload
   was fully consumed without reading the trailing byte that
   `SpellGo::Write` produces from `WriteLogDataBit` and `FlushBits`. The
   production writer was correct; the test now asserts the combat-log bit is
   clear and only then that the buffer is empty.
2. and 3. Both binder scenarios installed a canonical Player but never adopted
   its handle, so the cast identity allocator failed closed and the handler
   returned before the bind effect and before closing gossip. Adopting the
   handle then exposed two further fixture gaps that login provides in
   production: the canonical Player had no vitals, so the alive gate rejected
   the caster, and no faction template, so the interaction reaction check
   failed closed. The fixture now installs all three.
4. to 6. The three spell-click scenarios set a canonical map manager but never
   placed a canonical Player on it, so the same allocator failed closed and
   every planned cast was reported as failed. They now install and adopt a
   canonical Player the way login does.

None of these six required a production change; each was a fixture that
predated the migration to fallible canonical allocation. That migration is
what made the gaps observable, which is the point of the fence.

### Scenario coverage added

`session/tests/player_cast_lifecycle.rs` gains two scenarios:
`normal_start_and_go_carry_cpp_cast_flags_like_cpp` proves Start assembles
`HAS_TRAJECTORY | POWER_LEFT_SELF` while Go assembles
`UNKNOWN_9 | POWER_LEFT_SELF | NO_GCD` from the same cast and selects neither
the rune nor the ammo section for a plain mana spell, and
`reentry_denies_a_residence_stamped_server_triggered_cast_like_cpp` proves the
widened residence fence drops a server-triggered prepared cast after reentry.
`player_cast/tests.rs` gains
`unresolvable_visual_reports_a_cancellation_instead_of_dropping_the_request`.

## Publication validation

`./tools/validation-v2 final --base origin/3.4.3` **passed**: 14 of 14 commands
green, exit 0, 1968 seconds, one Cargo job, peak child RSS 4.9 GiB. The
manifest records head `215e481e`, base `8c47af95`, a clean tree
(`dirty: false`), 109 changed paths and Rust 1.98.0, and is stored at
`target/validation-v2/manifests/20260908T212452.975354Z-3593240-final.json`.
The profile covered diff hygiene, the physical-files scan and its 20 unit
tests, trailing-whitespace and JSON/Python/bash syntax gates, workspace and bot
formatting, the hotspot ratchet, the reverse-dependent `cargo check --tests`
closure over thirteen packages, the library suites for `world-server`,
`wow-entities`, `wow-map`, `wow-packet` and `wow-world`, and the standalone bot
manifest check.

## Live QA: not executed, and why

Paired caster/observer captures against a running server are **not** part of
this evidence, and nothing here should be read as if they were.

The maintained mechanism is `tools/qa-runtime.sh --allow-runtime-qa`, which
snapshots the live build, swaps in the candidate, restarts the `world-server`
systemd unit, runs the bot and restores the original. This host currently has
that unit active (`world-server` from `/home/server/rustycore/target/deploy/live`
against `/home/server/trinity-legacy-install/etc/worldserver.conf`, alongside
`bnet-server` and MariaDB). The standing instruction for this delivery permits
isolated QA but forbids modifying active original services or other people's
data, and the guard restarts exactly that active service. Running a genuinely
separate instance instead would need its own ports and its own copies of the
auth, characters, world and hotfixes schemas; reusing the live schemas would
write data that is not this delivery's to write.

There is a second, independent blocker, and it is the harder one. #594 requires
fresh **Rust and C++** captures taken under the same effective conditions, with
the provenance of the C++ binary recorded separately. No C++ TrinityCore server
binary exists on this host: `/home/server/trinity-legacy-install/` contains only
`worldserver.conf`, `bnetserver.conf` and certificates, which the Rust server
reuses, and the versioned reference tree at `/home/server/woltk-trinity-legacy`
is source-only with no build directory. A search for `worldserver`, `authserver`
or `bnetserver` executables under `/home/server`, `/opt` and `/usr/local` finds
none. Producing the C++ side would mean building TrinityCore 3.4.3 from source
with its dependencies, database and client data - a separate provisioning task,
not a step of this delivery.

So the paired matrix in #594 - instant and timed casts, the 400 ms queue
boundary, replacement, active and pending cancellation, late failure, shared
Player/creature identity, and a nearby observer's bytes, identities, visual,
flags, optional payload, targets, connections and ordering - remains
outstanding. The acceptance evaluator for it is complete and tested
(`tools/wow-test-bot`, 179 tests including 20 for the cast lifecycle, with a
documented caster/observer correlation procedure in `CAST_LIFECYCLE.md`), so
the work needed is the runtime authority, not more tooling.

The synthetic 30798 to 6197 fixture inherited from #587 remains synthetic and
covers none of that matrix. `observe_only` bot collection never reports a pass.

To close #594 later, two prerequisites must be supplied together: authority to
run the candidate against a server target that is not the active unit, and a
built C++ 3.4.3 reference whose binary provenance can be recorded. Neither is a
code change in this repository.
