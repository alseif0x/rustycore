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
