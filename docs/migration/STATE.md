# RustyCore — Honest Current State (single source of truth)

## Integration status on 2026-11-21

At `dd0844474` on `3.4.3`, the death lifecycle (#43) and logout admission/countdown (#54)
are integrated through [#1484](https://github.com/alseif0x/rustycore/pull/1484) and
[#1485](https://github.com/alseif0x/rustycore/pull/1485).
Instance enter/leave and reconnect/logout integrity remains open in [#45](https://github.com/alseif0x/rustycore/issues/45).
Use the [master index](https://github.com/alseif0x/rustycore/issues/49) for the remaining dependencies.

Subsequent merges cover character enumeration customizations and initial equipment (#1488, #1489),
quest-giver replies and the accept dialogue (#1491, #1493), inspect serialization (#1496) and
auction packet layouts and house selection (#1494). The [inspect](../audits/inspect-result-54261.md)
and [auction](../audits/auction-packets-54261.md) audits record the client build and limits of
those results. Auction transactions, the full quest lifecycle and real-client creature acceptance
have separate open issues (#39, #41, #1342).

## Instance enter/leave and reconnect (#45)

The issue branch implements manual reset, group difficulty changes and
occupied-instance expiry against the 3.4.3 C++ reference
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. In `Map.cpp`,
`InstanceMap::Reset` reports the occupied map, retains its players,
and schedules unload only on a successful empty reset. A normal reset
leaves persisted character locks intact. `InstanceMap::Update` processes
the original expiry even on an extended save. The extension prompt arms
the canonical Player's pending bind for a non-warning lock. See the
dated issue-branch record at `7eb4fccca:docs/migration/STATE.md` for the
source paths, positive and negative tests, packet bytes, timings and
limits of each comparison.

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
