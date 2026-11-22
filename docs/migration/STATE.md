# RustyCore — Honest Current State (single source of truth)

## Integration status on 2026-11-22

At `dd0844474` on `3.4.3`, the death lifecycle (#43) and logout admission/countdown (#54)
are integrated through [#1484](https://github.com/alseif0x/rustycore/pull/1484) and
[#1485](https://github.com/alseif0x/rustycore/pull/1485).
Instance enter/leave and reconnect/logout integrity [#45](https://github.com/alseif0x/rustycore/issues/45)
closed after [#1512](https://github.com/alseif0x/rustycore/pull/1512) merged at
`ad58236ad` and the [branch audit](https://github.com/alseif0x/rustycore/actions/runs/45020876206) passed.
The next stability work exercises those instance scenarios with multiple clients over time
under [#46](https://github.com/alseif0x/rustycore/issues/46). Save and recovery under load
remain with [#47](https://github.com/alseif0x/rustycore/issues/47).
Use the [master index](https://github.com/alseif0x/rustycore/issues/49) for the remaining dependencies.

Subsequent merges cover character enumeration customizations and initial equipment (#1488, #1489),
quest-giver replies and the accept dialogue (#1491, #1493), inspect serialization (#1496) and
auction packet layouts and house selection (#1494). The [inspect](../audits/inspect-result-54261.md)
and [auction](../audits/auction-packets-54261.md) audits record the client build and limits of
those results. Auction transactions, the full quest lifecycle and real-client creature acceptance
have separate open issues (#39, #41, #1342).

## Two-client instance stability (#46), 2026-11-22

On aarch64 the debug server from clean candidate `a7cf2210b` passed the
[instance workload](../../tools/wow-test-bot/RUSTYCORE_SMOKE.md#two-hour-instance-stability-workload-46).
The solo actor used Hearthstone, disconnected and re-entered the same save.
Qadeathcast and Qadeathfall then completed 31 group cycles in 7,247.159 seconds
on the same server process, with zero restarts. Both clients observed the
occupied reset. The leader reset the empty instance after the member left,
and MariaDB showed the group rows absent after the server processed the leave.
All 32 case reports matched their recorded hashes.

The four registry command counters each had a delta of zero across the
workload: ordinary sends rejected because the queue was full, the receiver
was disconnected or the registration was stale, and reliable sends retained
for retry on a full queue. These are attempt counts on the two instrumented
command paths. Across 498 process-memory samples, RSS rose from 909.2 MiB at
the first group sample to 951.6 MiB at the last. The last 30 minutes spanned
120 samples and a rise of 552 KiB. Longer-term memory growth and other queues
remain outside this measurement.

The run used `RUSTYCORE_DEATH_SCENARIO=instance-stability-soak` with
`tools/qa-death-runtime.sh` and `tools/qa/instance_scenario.py`. The outer
report verified the original binary, configuration and disposable fixture
were restored. The local report is
`/tmp/rustycore46-soak-a7cf2210b/death-runtime-report.json`
(SHA-256 `6b942b05f4baa8a73386422c230ecfe8831acf836d5e29cfc1dc9d83ef18a122`).
The candidate binary SHA-256 is
`4bba8849e993361d835f967244d1c30a99c68af2b6757d2d1de36fbaa86b5af1`.
The two-hour run repeats the #45 contracts against C++ `a5f8da2eb`,
including `WorldSession::HandleMoveWorldportAck`, `Group::ResetInstances`
and `InstanceMap::Reset`. The #45 section records the paired packet comparisons.

The local final at `c3fb81e04` ran 936 Rust tests (one additional test
ignored) and 266 runtime guard checks. It took 8m43s; the campaign including
the first attempt and its 13-line map-loop budget correction took 20m58s.
The first live attempt restored the fixture after journalctl returned an
INFO message as bytes before any bot logged in. The runner correction at
`a7cf2210b` passed 45 Python instance tests, a replay of that actual journal
entry and the final profile against `c3fb81e04`. The instance lane now has
multi-hour evidence. Creature visibility and relocation (#1342/#1354), loot
recovery (#1339/#1341) and the #47 save-under-load campaign retain their
separate acceptance.

### Creature relocation and money recovery on the #46 branch

On 2026-11-22 the local candidate based on `d17febe40` restored the
3.4.3 move-list phase for creature cell changes. Same-cell movement updates
immediately. A blocked relocation tries the spawn grid, clears combat and
motion, and queues removal when that grid is unavailable. Stationary players
at the origin and destination receive visibility updates from the physical
move. The source anchors at `a5f8da2eb` are `Unit::UpdatePosition`,
`Map::CreatureRelocation`, `MoveAllCreaturesInMoveList`,
`CreatureRespawnRelocation` and `ChaseMovementGenerator::Update`.
At `52933ae7d` the stationary-observer, blocked-respawn and player-owned
grid tests passed. A Session aggro fixture still sampled its owner before
the queued move. At `fbe495202` both owner-range cases passed with
`cargo test --locked -p wow-world --lib creature_aggro_creature_owned_creature`.
The resulting test binary passed all 4,206 active cases (one ignored).
The earlier six-library campaign spent 11m16s in its library step and
failed the old aggro fixture. It exceeded the ten-minute warm budget.

The Characters adapter at `32ea0410e` passed
`cargo test --locked -p wow-database --test loot_money_restart -- --exact money_claims_survive_process_restart --ignored --nocapture`
using a disposable MariaDB socket. Separate claim and recovery processes
verified stored-Item money, shared payouts with a capped recipient, a
rolled-back transaction and unrelated spending. A fresh process recovered
the spent chest using realm, map, instance, persistent spawn and loot scope.
It refused a stale personal grant and a stale deadline, and credited the
next generation once after the stored deadline elapsed. The expanded case
at `c7c9d94ce` forced the respawn-row write to fail, verified that the
source deadline rolled back too, and recovered both records in a third
process after a successful commit with no map acknowledgement. The parent
test took 3.18s after compilation. The shared QA database was untouched.
QA spawn 9106001 uses consumable chest template 2846 (loot 2278, group
loot enabled). The 3.4.3 C++ anchors are `GameObjectData.h:99–112,127`,
`GameObject.cpp:1609–1651,3711–3721` and `LootHandler.cpp:142–213`.
The four real-client creature scenarios (#1342) and a world-server
restart with the actual QA chest are still pending.

## Instance enter/leave and reconnect (#45)

[PR #1511](https://github.com/alseif0x/rustycore/pull/1511) integrated manual
reset, group difficulty changes and occupied-instance expiry on `3.4.3`
at `98f1f7ded`. The implementation follows the 3.4.3 C++ reference
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. In `Map.cpp`,
`InstanceMap::Reset` reports the occupied map, retains its players,
and schedules unload only on a successful empty reset. A normal reset
leaves persisted character locks intact. `InstanceMap::Update` processes
the original expiry even on an extended save. The extension prompt arms
the canonical Player's pending bind for a non-warning lock. See the
dated issue-branch record at `7eb4fccca:docs/migration/STATE.md` for the
source paths, positive and negative tests, packet bytes, timings and
limits of each comparison.

The [post-merge review](https://github.com/alseif0x/rustycore/pull/1511#discussion_r4655374017)
found that two groups selecting the same loaded raid could both list it among
their owned instances. In the C++ reference, `InstanceMap::TrySetOwningGroup`
(`Map.cpp:2993`) links only the first owner. That link calls
`GroupInstanceReference::targetObjectBuildLink` (`GroupInstanceReference.cpp:21`).
`Group::ResetInstances` (`Group.cpp:1354`) walks the resulting references.
The follow-up checks the map's owning group and writes the group's reset
reference under the same map-manager guard. A second group can still select
the instance through its leader's lock. Only the owner lists the instance
for reset.

On November 22 the two-session regression reproduced the second group's
unexpected `[(631, 1)]` on the integrated code. At `5d87fc3ee`, both groups
selected the same ID-bound map twice. The owner's reset list contained
exactly `[(631, 1)]` and the second group's list stayed empty.
`./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`
passed all 10 steps on aarch64, including 4,206 library tests with one
ignored. The verified final took 3m09s. Including the earlier run and its
test-line budget correction, acceptance took 10m52s. The local manifest is
`target/qa/issue45-group-owner/final-5d87fc3ee.json`.

The separate live `instance-group-reset` scenario used the debug server from
the same commit. The owner received the occupied reset failure, the member
remained inside, and the empty reset succeeded after the member left. The
guarded run took 8m30s and restored the service, configuration and fixture.
An initial attempt timed out during server startup and the fixture was
restored before retrying. The live report is
`/tmp/rustycore45-group-owner-5d87fc3ee-r2/death-runtime-report.json`
(SHA-256 `fadd3f19c97cafb7ab82a1caf5eee45a83b62190d4fbba349b81f664ad80ee67`).

Solo and group reset had paired action captures on the earlier issue
candidate. On November 21 the production MariaDB adapter at `af6919ee0`
also demonstrated a new member lock surviving a real database restart.
The Rust debug executable from `0362df4e3` (SHA-256
`79887a11fdb0d3ffc2118ddfbcb031fb3037503525ddbd0d74cf73bbf4920a66`)
and the pinned C++ R6 executable each passed three ICC epochs. All 11
selected expiry/decision packets and both post-restart leave packets
matched in order, direction, connection and body. Declining left the
member unbound. Acceptance saved its lock, and the leader renewed after
60 seconds. Both characters reloaded the same save. The occupied reset
and subsequent group removal preserved those locks. Both campaigns
restored the server, configuration and disposable fixture.

The local Rust and C++ reports are in
`/tmp/rustycore45-icc-bind-rust-0362df4e3-r6/` and
`/tmp/rustycore45-icc-bind-cpp-9e2c716fc-r5/`. The paired comparison
is `target/qa/issue45-restart/icc-expiry-leave-r6.txt` (SHA-256
`44d288458c6c149a0fcc5146147ee36486b0213cc0069ecf3c8212ec8947e3fa`).
The local component suites at `cbf831f0f` passed 5,933 tests with three
ignored. The initial compile exceeded the ten-minute local target.
A separate open-world login `WorldServerInfo` difficulty-field difference
is recorded at `7eb4fccca`.

### Difficulty change and party recovery after restart

On November 21 the same Rust and C++ binaries each passed two difficulty
epochs on aarch64. Changing to difficulty 2 kept the occupant inside.
The raw change request and both difficulty replies matched in order,
body and connection. Each server restarted, loaded the group from MariaDB
and admitted both characters to the same instance. Each character's
first realm `PartyUpdate` matched the native 89-byte body, including the
leader, both members, the recipient index, the online and offline slots,
the update sequence and the difficulty settings. The native layout and
roster are defined in `PartyPackets.cpp:446-530` and
`Group.cpp:836-909`. Both servers accepted the subsequent one-byte
leave request, removed the group rows and restored the fixture.

The Rust and native reports are in
`/tmp/rustycore45-difficulty-{rust,cpp}-0362df4e3/` (SHA-256
`0c5a64abe32de80e1def1653d8a7336fcfc792ee9fb7e1714ba77831377b2726`
and `543d4374ebc24e53441e88d907d49c834252e93ced0644418c2fcb1a4b800cfe`
respectively). From fixture preparation to final report, Rust took
12m52s and C++ took 6m58s. The paired result is
`target/qa/issue45-restart/difficulty-comparison.json`. The strict
change and leave windows had zero missing, extra, body or route
differences. The earlier solo and group reset captures remain recorded
at `7eb4fccca`; their raw files were unavailable for replay on November 21.
For empty map 33 the historical capture shows the successful reset response.
The current map-manager test observes destruction on the next update. The
production lifetime test also verifies that a failed destroy keeps an occupied
instance ID allocated. The live ICC comparison covers reset with an attached lock.

### Local acceptance on November 21

The aarch64 host tested clean code revision `411dde392eb1a6dcb2a0862c216a10be3c0fbcd8`
against `3.4.3` at `229d28970e74faf1cd4e54db1e1346d59c3a6f9b`. With two Cargo
jobs and one test thread, `./tools/validation-v2 final --base origin/3.4.3
--architecture --timings --logs --require-changes --keep-going` passed all 23 steps
in 611.8 seconds. The 14 affected library suites passed 8,392 tests with 12
ignored. The handler-contract checker passed all 465 tests. The verified local
manifest is `target/qa/issue45-restart/terminal-final-411dde392.json`, SHA-256
`d9ea1680bcb73964c8bf5f43b0d639fadd1e42760c60fce46e50344ce6891cbe`.

The three production integration targets (`production_handler_registry_contract`,
`production_player_lifetime` and `production_login_player_owner`) passed 56 tests.
The standalone `tools/wow-test-bot` Cargo suite passed 267. The three Python
suites `test_instance_scenario`, `test_death_fixture` and `test_death_scenario`
passed all 87 distinct tests, including a follow-up of the optional DDL check
against the pinned C++ checkout. The ordinary campaign ran from 21:29:43 to
21:44:09 UTC (14m26s), plus 0.004s for that DDL test. It exceeded the ten-minute
budget. The contract suite took 4m30s and compiling the production tests took
3m48s. Two earlier attempts took 17m34s and 20m26s and exposed the missing
map-command classification, registrar export grouping and bot formatting.

The separate `target/arch-tools/release/session-ownership-check check` passed
in 7m22s. It matched 43 session commands, 784 direct registry rows, 8,206
production and 2,681 fixture persistence accesses, and 1,095 semantic groups.
The four added production accesses belong to the read-only instance script
template loader. The local log is `target/qa/issue45-restart/terminal-exhaustive-411dde392.log`.

## Earlier evidence

The [archived checkpoint at this revision](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/STATE.md) contains the dated C++ references, capture and QA records, commands, tested SHAs and limits from earlier deliveries. Use the [port plan](PORT_PLAN.md) and the open owner issue when deciding what to implement next.

## 5. Definition of "done" going forward (replaces `represented-complete`)

A capability is **done** only when: (1) it runs in the **live runtime**, (2) its wire output
is **capture-clean vs a C++ capture** of the same action — byte/opcode exact unless a narrowly
reviewed comparator omits only an intrinsically runtime-allocated identifier or canonicalizes
only proven unordered C++ collection order, while retaining every stable value/count and failing
malformed input — (3) it has been **exercised on a running server/client**, and (4) C++ refs +
the validating capture/test are cited.
`represented-complete` is no longer a closure state — it means "logic drafted, not yet live".

This gameplay-capability definition does not require a fresh capture for every behavior-preserving
structural commit. Refactors follow the proportional gates in AGENTS.md and their explicit issue
acceptance: preserve bytes, metadata, connection and order with focused evidence; obtain the
required action-specific capture/live evidence when the change or acceptance calls for it.
Report a bounded proof as bounded, and distinguish code tests, historical golden regression,
fresh runtime evidence and full functional parity. Architecture completion requires the complete
technical gate: #584 core, the #583 product (including the mandatory Rust/Wasm/C mixed delivery
with optional operator activation), and the independent #153 audit. #133 is already closed
administratively and is not a future gate. Do not substitute a favorable field count or test
total. Neither architecture closure nor playable M6 closes the full Part-2 parity ledgers.
