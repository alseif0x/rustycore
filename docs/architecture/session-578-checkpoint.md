# Session convergence checkpoint — updated 2026-09-06

Issue #578 remains open. This is an exact inventory reconciliation, not the terminal #153
audit, a full C++ parity approval, or a live-client acceptance report.

## C1 Login-save boundary audit — `8c5aa68b`, 2026-09-06

The next finalization boundary cannot be accepted by moving the five collection
helpers out of Session or batching only those five calls. Exact legacy evidence in
`src/server/game/Entities/Player/Player.cpp:19312-19322,19662-19688` shows full
`SaveToDB` builds separate Character and Login transactions. The Login transaction
contains toys, battle pets, heirlooms, mounts, item appearances, transmog illusions,
then last-character delete/insert, in that order. This is not logout-only. Character
commit is submitted before Login commit; this is not distributed atomicity or proof
of asynchronous completion order.

Current Rust evidence at the audited SHA:

- `handlers/character/account.rs:1685-1904` prepares and saves collections through
  five independent calls, logging and discarding their persistence outcomes.
  `session/lifecycle/logout.rs:62` and `handlers/character/world_entry.rs:146` call
  them after character save, starting with mounts rather than C++'s toys.
- `wow-database/src/player_lifecycle_adapter.rs:1473-1570` builds and commits a
  transaction per collection. Its `Database::commit_transaction` path calls
  `SqlTransaction::commit`, which erases `SqlTransactionCommitError` classification;
  the collection adapter maps every returned error to `Failed`, unlike the full
  character-save adapter's explicit `Unknown`. A returned collection failure is
  therefore not proof of rollback.
- Repository-wide Rust search for `DEL_BNET_LAST_PLAYER_CHARACTERS` and
  `INS_BNET_LAST_PLAYER_CHARACTERS` finds only enum/SQL definitions in
  `wow-database/src/statements/login.rs`, not a caller of those statement variants.
  This search does not rule out equivalent raw SQL or prove whole-port coverage.
- `session/mod.rs::account_item_appearance_save_plan_like_cpp` advances favorite
  dirty state before persistence. C++ `CollectionMgr.cpp:516-558` also advances it
  while appending statements, before COMMIT. Retaining dirty state until confirmed
  receipt would be intentional durability hardening, not a mechanical parity move.
- Existing `battle_pet_account` authority and `battle_pet_purchase` recovery must
  be traced before including battle pets in another save coordinator. The existing
  purchase saga is not permission to create another account owner or replay grants.

**Decision for the next implementation:** model the complete Login-side save
operation and its relationships to existing account owners before extracting its
application boundary. Keep Character and Login results distinct; do not claim a
five-collection batch restores full-save parity. Compare integration with existing
durable account operations before changing their transaction/recovery contracts.
Do not add a universal Session context, account mirror or per-table service just
to split files. Unknown-COMMIT, cancellation, concurrent account mutations and
version/incarnation-bound acknowledgements remain explicit implementation work.

This audit corrects contradictory C++ attribution in the collection DTO, port and
adapter comments; it changes no executable behavior and closes no C0–C4 acceptance.
The preceding committed cleanup validation manifest
`target/validation-v2/manifests/20260906T221208.238092Z-3-quick.json` was verified green
at `8c5aa68b`. No publication, live runtime or database operation was performed.

Validation of this comment/document-only delta above that SHA: `cargo fmt --all
-- --check`, `git diff --check`, ownership `check --syntax-only`, architecture
`check`/`self-test`, and all five `persistence_policy` tests pass. `validation-v2
quick --base HEAD` compiles the affected persistence/database test targets; manifest
`target/validation-v2/manifests/20260906T222040.230319Z-3-quick.json` verifies green.
No logical/physical ceiling or persistence inventory was changed.

## C1 canonical retirement never reacquires by GUID — above `8cb125c1`, 2026-09-06

The preceding cleanup risk is now reproduced through the production wow-world library.
Both `production_repeated_cleanup_preserves_replacement_of_stale_owner` and
`production_repeated_cleanup_preserves_replacement_after_successful_retirement` fail
on `8cb125c1` with the new tests: 0 passed / 2 failed, log
`/tmp/rustycore-cleanup-incarnation-before.log`. In the first case the second cleanup
deletes the replacement; in the second case the first cleanup after replacement does.
The old implementation consumed its handle, then a later call fell back to GUID/map
search and removed a different incarnation while leaving its manager ownership index
inconsistent. These are controlled repeated public cleanup calls, not proof that a
current network scenario automatically schedules that repetition.

Intentional lifecycle repair in `session/lifecycle/cleanup.rs`: retire only the exact
Session-held PlayerHandle, clear it only after retirement returns the owned Player,
and retain it on failure. No handle means no authority to remove a map resident.
The unsafe GUID/map search is deleted, not moved behind cfg(test), and no new field,
mirror, lock, public method or automatic retry is added. A retained stale handle
continues to fail against a replacement; successful retirement followed by another
cleanup cannot reacquire it. C++ `Server/WorldSession.cpp:660-672` removes the exact
`_player` pointer, including its detached-map caveat, then clears that pointer; it
does not authorize deleting a newly found Player with the same GUID.

Alternatives rejected: retaining a stale token after every successful retirement would
contradict the existing cleared-handle contract, while a second "already cleaned" flag
would duplicate lifecycle state. The actual production construction/adoption paths
(`ensure_canonical_player_owner_exists_like_cpp`,
`install_detached_canonical_player_from_session_like_cpp`, and canonical transfer
admission in Session) already acquire an incarnation handle. Cleanup must not acquire
new ownership. Three older library fixtures inserted raw map Players without giving
their Session a handle; each now uses the existing explicit fixture-adoption helper.
All removal, detached-owner, unrelated-object and inventory assertions are preserved.

The small production integration module additionally injects a missing backing Player,
retains its exact Box locally, attempts cleanup, restores that same value, and repeats
cleanup. It checks both backing-value retirement and removal of the ownership index.
This proves retained retirement identity under controlled failure, not a supported
production transfer protocol, DB durability or a complete cleanup recovery policy.
Directory unregister still checks the control-channel identity; login-claim release
still checks its Arc identity. Visibility refresh ordering and inventory cleanup are
unchanged. Broad cancellation/partial-finalization results remain a separate open C1
obligation: a failed canonical retirement still does not classify the outer unit cleanup.

Reviewed metrics: cleanup.rs falls from 112 to 89 physical lines; Session logical
production/test/total becomes 82,797 / 107,493 / 190,290 (-23/+3/-20).
The existing Session test root grows by three explicit fixture-adoption lines to 96,722;
its temporary C4 ceiling retains the named family splits. New scenarios live in the
100-line private production integration module. No persistence inventory is regenerated
and no legacy physical ceiling is retired by this bounded repair.

Local aarch64 validation, with pinned PROTOC and offline/locked Cargo:
`cargo test -p wow-world --lib --test production_login_player_owner` passes 3,776
library tests (one ignored) and 27 production-linked integration tests, log
`/tmp/rustycore-cleanup-incarnation-final-tests.log`. Syntax-only ownership stays at
282 production + 433 fixture fields, 3,699 associated items and 590 registry rows.
Architecture check/self-test and five preserved persistence-policy tests pass.
Quick passes with verified manifest
`target/validation-v2/manifests/20260906T220301.557014Z-3-quick.json`; this evidence
paragraph was completed afterward. Migration checks cover 998 source files and retain
101 legacy ceilings; they do not close C4. The same production integration target
also passes all 27 tests under `--release`, log
`/tmp/rustycore-cleanup-incarnation-release.log`, including both formerly failing
replacement cases and the retained-token failure case. The mutated production
retirement path therefore executes with and without debug assertions.

No publication, runtime restart or DB operation is performed. C0 cross-clock admission,
the complete C1 finalization report/recovery and real lifecycle QA, C2/C3/C4 retirement
and full before/after-add/logout parity remain open.

## C1 disconnect attempt reaches supervision — above `ab753fe5`, 2026-09-06

The disconnect wrapper previously discarded `PlayerSaveOutcomeLikeCpp`, and the
shutdown helper accepted only unit futures. This cut propagates a public, narrow
`DisconnectSaveAttemptLikeCpp`: NoPlayer, NativeCompletionUnavailable or the actual
character-save classification. It is deliberately **not** a whole-finalization receipt.
The existing outcome type is reexported, not mirrored or reconstructed from Session
state. Applied remains Applied if its old incarnation cannot receive the local receipt;
Quarantined includes pre-existing reconciliation failure, not just a newly submitted
unknown COMMIT. Collection/offline methods still classify their own failures locally.

The production factory consumes the report in both normal and shutdown branches. Its
bounded helper now returns `Option<F::Output>`: Some preserves the exact operation
result, None marks the existing timeout failure. A completed future is no longer a
boolean that can be mistaken for a successful save. The existing independent cleanup
attempt after timeout remains; timeout does not prove rollback or worker quiescence.
Only the already established native-completion gate refuses normal cleanup. Returned
non-Applied character outcomes are explicitly logged before existing retirement, with
no automatic replay, no new global-fatal policy and no invented durable-recovery claim.
The former unconditional “Finished disconnect save” log is replaced by an attempt log
with character classification and an explicit independent-durability boundary.

**Decision from the caller review:** do not generalize the native-failure fatal gate to
all Deferred/Unavailable results. The latter combines missing/replaced owner, invalid
projection and absent lifecycle port (`persistence/deferred.rs`, `prepared.rs`, and
`persistence.rs`). These do not provide the cause or phase evidence needed for one
safe recovery/retention policy. Nor does stopping the process preserve unsaved RAM.
The preliminary generalized gate was removed before acceptance; this cut preserves
existing cleanup behavior and makes its incomplete contract visible rather than
pretending a report alone repairs it. The next C1 boundary must distinguish admission
causes and legitimate lifecycle exits, then coordinate collection/offline outcomes,
cancellation and exact-incarnation cleanup for the complete operation.
In particular, `cleanup.rs::unregister_canonical_player_from_map_like_cpp` consumes
the handle before retirement and retains a GUID/map fallback when no handle exists.
Before allowing repeated cleanup, test an old session against a replacement on the
same map across **two** cleanup calls; single-call handle checks are not idempotence
proof. This is a source-review risk to reproduce, not a tested defect closure.

C++ anchors: `Server/WorldSession.cpp:544-551` completes far transfers before logout;
`:633` submits save, `:672-676` retires Player and sends LogoutComplete.
`Entities/Player/Player.cpp:19312-19322` submits character/login transactions separately;
`:19324-19333` defers far-transfer saves. Those call sequences do not supply a confirmed
Rust async finalization receipt. This is a supervision/result-contract change, not a
claim of full C++ logout parity or a hidden behavioral repair to known rollback.

Controlled production-library regressions assert Applied/Failed/Quarantined reports,
confirmed old-incarnation results, submitted cancellation without replay, unavailable
source projection, incomplete native work before any persistence, and NoPlayer after
explicit logout. A small private factory module tests all seven returned variants and
independent cleanup after timeout. Neither fixture is a live socket/DB/restart scenario.
Reviewed growth: Session +19 production lines (82,820 / 107,490 / 190,310 logical
production/test/total); server +62 production and +73 fixture lines
(28,980 / 27,094 / 56,074). The private factory module is 114 lines and the production
logout integration module 278 lines, with a 537-line save fixture parent. Only the
existing main_tests.rs physical ceiling grows by two lines for the changed Option
assertions; its named C4 split remains. There are 997 physical files and 101 legacy
ceilings. Syntax remains 282 + 433 Session fields, 3,699 associated items and 590
registry rows. Reviewed exact deltas are the disconnect method return type, generic
shutdown helper signature/body and both factory body fingerprints; no persistence
inventory row, registry metadata, clock or state field changes.

Local aarch64 evidence (pinned PROTOC, offline/locked Cargo, existing validation cache):
`cargo test -p wow-world --lib --test production_login_player_owner` passes 3,776 library
tests (one ignored) and 24 production-linked integration tests in
`/tmp/rustycore-disconnect-report-final-tests.log`. Syntax-only ownership, architecture
check/self-test and five preserved persistence-policy tests pass. The initial server
binary test target compiled but contains zero tests; it is **not** supervision test
evidence. The actual tests are in `world-server --lib shutdown_`.
That target passes all ten selected tests (559 filtered) on the final source in
`/tmp/rustycore-disconnect-report-server-lib-final.log`. Quick compiles the affected
production/test targets and passes, with verified manifest
`target/validation-v2/manifests/20260906T214151.364190Z-3-quick.json`. This evidence
paragraph was completed afterward; no live durability or full finalization is inferred.

**Still open:** full finalization and recovery, C0 cross-clock admission, complete C++
before/after-add and logout semantics, C2/C3/C4 retirement, and authorized real DB/restart/
relogin/action-specific captures. No publication or runtime action is part of this cut.

## C1 explicit logout preserves save quarantine — above `c61903fc`, 2026-09-06

The next caller review reproduced an actual production-linked failure: an explicit
logout's full save classified COMMIT as Unknown and kicked the session, but the handler
continued through owner retirement and reset Disconnecting to Authed. The regression
`production_explicit_logout_does_not_restore_authed_after_unknown_commit` fails on
`c61903fc` with tests (0 passed / 1 failed, Authed instead of Disconnecting), log
`/tmp/rustycore-logout-quarantine-before.log`. This uses the compiled production library
and a controlled persistence port, not MariaDB or a complete login/client scenario.

The intentional async-lifecycle repair in `handlers/character/world_entry.rs` now:

- refuses to restart explicit logout on an already Disconnecting session;
- sends the existing LogoutResponse, then completes represented native far-transfer work
  before the logout flag/save, following `Handlers/MiscHandler.cpp:238-275` and
  `Server/WorldSession.cpp:544-551`; Terminal recovery returns to disconnect finalization
  without an extra explicit-handler source save;
- consumes the full-save outcome and preserves Disconnecting after its await; Deferred,
  Unavailable or quarantine cannot fall through to owner retirement, claim release,
  LogoutComplete or Authed. The owner remains for the existing disconnect path;
- preserves the existing known-rollback logout policy rather than adding a retry loop or
  claiming that returning to character selection proves a committed save. C++ submits
  SaveToDB before retiring Player and sending LogoutComplete (`WorldSession.cpp:633,
  :672-676`), but that sequence does not supply a Rust confirmed-COMMIT receipt.

The 190-line private production integration module `save/logout.rs` adds six tests:
Unknown, submitted cancellation/no replay, unavailable source projection, normal applied/
known-rollback behavior, pending transfer completed before save/retirement, and Terminal
recovery with exactly one source save left to disconnect. The transfer cases reuse the
existing fixture and call the production adapter directly; they do not claim its LoggedIn-
only network registration admits LogoutRequest during Transfer. Assertions cover actual
Player retention/retirement, GUID/state, transaction counts, saved destination/resurrection
health and absence/presence of LogoutComplete. The existing loot-release test now explicitly
installs its controlled persistence port and retains its exact packet-order assertions.

Reviewed Character logical growth is +26 production / +1 fixture line, to
20,685 / 13,131 / 33,816. Temporary physical ceilings become 2,840 for world_entry.rs
and 12,634 for the existing Character test root, retaining their named C1/C4 splits.
The integration fixture parent is 526 lines. There is no new production field, method,
clock, retry policy, opcode metadata or persistence inventory row. Session syntax remains
282 + 433 fields, 3,699 associated items and 590 exact registry rows. The 101 legacy
physical ceilings still require retirement; migration checks do not close C4.

Local aarch64 validation with pinned PROTOC and the existing Cargo validation cache:
`cargo test --offline --locked -p wow-world --lib --test production_login_player_owner`
passes 3,776 library tests (one ignored) and 22 controlled integration tests, log
`/tmp/rustycore-logout-quarantine-final-tests.log`. Syntax-only ownership, architecture
check/self-test, five preserved persistence-policy tests and diff checks pass.
Quick/format checks pass with verified manifest
`target/validation-v2/manifests/20260906T212034.629873Z-3-quick.json`; this evidence
paragraph was completed afterward. No fresh capture or live durability is claimed.

**Still open:** the factory-facing disconnect wrapper returns unit and its cleanup/shutdown
policy does not yet consume a typed finalization report. Cancellation after the character
save, collection/offline writes and partial retirement still need the whole operation's
completion contract. Known-rollback behavior is preserved, not upgraded to durable recovery.
The instant-only logout admission also remains incomplete against MiscHandler.cpp; its
old unsupported provenance comment is replaced by that exact source and an explicit limit.
Full before/after-add semantics, C0 cross-clock exclusion, C2/C3/C4 and real DB/restart/relogin/
action-specific captures remain open. No runtime or publication action was performed.

## C1 retained deferred save and registered ACK — above `bc3bdd2f`, 2026-09-06

Intentional async lifecycle repair implementing the transfer admission/resumption part
of the review below, not a behavior-preserving file split. Exact C++ anchors remain
`Player.cpp:19324-19333` (timer reset and DELAYED_SAVE_PLAYER), `:1494-1503`
(resurrection before delayed save), and `WorldSession.cpp:544-551` (complete far entry
before disconnect save). Rust's retained-until-confirmed-COMMIT policy remains stronger
than C++'s synchronous delayed-operation flag clearing; this does not invent a new
transaction or infer legacy durability from that flag.

- A private 85-line `wow-entities/src/player/deferred_save.rs` owns pending intent and
  a checked revision on the same Player. No Session mirror, timer, lock or DB dependency.
  Repeated requests coalesce into the next admitted save. The existing full-save receipt
  captures the revision; a matching confirmed receipt clears it, an older receipt does
  not clear newer intent, and the existing PlayerHandle gate protects a replacement.
  Revision overflow fails closed. Terminal/source-save remains the approved exception;
  unresolved native post-add work still defers even with Terminal set.
- The 67-line private `session/lifecycle/persistence/deferred.rs` adapter checks current
  Session GUID/handle and canonical access. Full-save orchestration now returns Applied,
  Deferred, Unavailable, Failed or Quarantined. Transfer deferral happens before durable
  waits/submission, and is checked again if preparation fails after those awaits. Existing
  item/money admission, reconciliation, commit cancellation and dirty-state fences remain.
  Confirmed application is not relabelled failure if the old owner can no longer receive
  its local acknowledgement.
- Worldport's actual registry thunk borrows the existing process-owned item generator.
  After represented native resurrection/completion, it drains the retained save before
  final stat publication and LoggedIn. A known rollback keeps dirty intent for the next
  scheduled save without disconnecting solely for that rollback, preserving existing
  SaveToDB failure behavior. Unknown/unavailable/still-deferred results reject readiness;
  an awaited save cannot overwrite Disconnecting with LoggedIn. An already-due Session
  periodic flag is transferred to native intent when post-add begins, so it participates
  in this phase rather than waiting behind the next packet-drain tail. No opcode, packet
  bytes, registry metadata, output queue or runtime clock is replaced.
- Private 289-line `lifecycle_persistence/deferred_transfer.rs` tests execute the **registered
  WorldPortResponse thunk** with canonical Player and controlled persistence. They cover
  direct coalesced requests and due periodic save, saved resurrection health, known failure,
  unknown COMMIT, rollback retry only at the next due autosave, submitted cancellation
  without replay, pre-submit cancellation without
  quarantine, and a newer intent surviving an older transaction's confirmed receipt.
  Existing production-linked save tests now retain native intent across saturated output,
  cancellation/unknown, replacement incarnation, retained post-add, and pre-ACK requested/
  homebind/Terminal-source save. The latter tests seed intent through the real Player
  operation; they do not claim that seed itself is a production direct-save caller.

Reviewed physical growth is limited to three Player-root declaration/initialization lines
and one Session-root outcome reexport. Both temporary C4 ceilings retain their split exits;
no new legacy exception is introduced. Logical Session is 82,801 production / 107,490 tests /
190,291 total (+95/+326), Player is 11,374 / 10,184 / 21,558 (+46/+45). Syntax remains
282 production + 433 fixture Session fields, with 3,699 exact associated items (+2 private
adapter methods) and unchanged 590 registry rows. No persistence inventory row is closed
or regenerated, and passing these migration ratchets does not retire the 101 legacy ceilings.

Local aarch64 validation on this working cut, using pinned PROTOC and the existing
validation Cargo cache: `cargo test --offline --locked -p wow-world --lib` passes
3,776 tests (one ignored); `cargo test --offline --locked -p wow-entities --lib` passes
723 tests; `cargo test --offline --locked -p wow-world --test production_login_player_owner`
passes 16 controlled integration tests. `cargo check --offline --locked -p world-server`,
syntax-only ownership, architecture check/self-test, five preserved persistence-policy
tests and format/diff checks pass. Full library evidence after the rollback-policy review:
`/tmp/rustycore-deferred-save-lib-final.log`; native/integration evidence:
`/tmp/rustycore-deferred-save-entities.log`, `/tmp/rustycore-deferred-save-integration.log`.
Verified quick manifest: `target/validation-v2/manifests/20260906T205057.846445Z-3-quick.json`.
The documentation evidence paragraph was completed afterward; no live durability or
action-specific capture acceptance is inferred from these controlled tests.

**Remaining C1 caller/finalization boundary:** the periodic processor, trainer pre-save,
explicit logout and disconnect wrapper still retain their existing outer flow. The trainer
continues its existing post-await revalidation. Explicit logout and the factory-facing
disconnect wrapper do not yet return a typed finalization result to authorize retirement;
their unit return or completion log is not durable-save proof. Complete that outcome/
cleanup contract next, including known rollback, unavailable projection/port, unknown
COMMIT and shutdown timeout, without turning a confirmed old-incarnation transaction into
a retry against its replacement. No repeated every-tick save retry is introduced here.
Cross-clock admission/exclusion, complete before/after-add gameplay/protocol parity, C2/C3/C4
retirements and live DB/restart/relogin/capture acceptance remain open. No publication or
runtime action was performed; this is not C0–C4 completion.

## C1 deferred-save admission review — `ae7a01b7`, 2026-09-06

This is a bounded implementation-prerequisite review, not deferred-save implementation
or a new approval gate. The native disconnect completion already committed at this SHA
does not retain a direct full-save request rejected during transfer.

Exact source contrast:

- C++ `Entities/Player/Player.cpp:19324-19333` resets the autosave timer and schedules
  `DELAYED_SAVE_PLAYER` when far teleport is pending. `ProcessDelayedOperations`
  (`:1494-1503`) executes resurrection before that save; `:1535-1536` clears delayed
  operations afterward. That synchronous flag clearing is not a Rust COMMIT receipt.
- Rust `session/mod.rs::process_pending_periodic_player_save_with_generator_like_cpp`
  preserves an already-due periodic flag while a destination remains pending or Session
  is not LoggedIn. This existing behavior must not be described as losing every autosave.
- In contrast, `session/lifecycle/persistence.rs::save_current_player_to_db_with_generator_like_cpp`
  resets that timer/flag before waiting for durable work and preparing the snapshot.
  `persistence/prepared.rs` rejects nonterminal far transfer and retained post-add work.
  No canonical delayed-save request replaces the cleared periodic flag. The periodic
  processor is also after the whole packet-drain loop in `session/driver/mod.rs`, not
  at C++'s delayed-operation point in the worldport handler.

A temporary private regression above this exact SHA reused
`transfer_completion::tests::save_fixture`, set the interval to 100 ms, advanced it
100 ms and verified the periodic flag was true; it then set canonical `far_pending`
and destination `(1, Position::new(7, 8, 9, 0.5))`, awaited
`save_current_player_to_db_like_cpp`, and asserted the flag remained true. It fails
at that last assertion: **0 passed / 1 failed**, local aarch64, log
`/tmp/rustycore-deferred-save-loss-reproduction.log`. Command:
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/home/server/rustycore/target/validation-v2/cargo/209fefad83026767 cargo test --offline --locked -p wow-world --lib direct_save_during_far_transfer_preserves_an_already_due_save_request`.
The temporary failing test was removed after observation; production code is unchanged.
The final correction should test retained canonical intent, not require the current
Session boolean to remain its owner. This proves the direct-call behavior, not that
an actual client can currently invoke every direct caller while Transfer is admitted.

### Required implementation contract inside C1

1. Keep the deferred request on the same canonical Player incarnation, separate from
   the periodic timer and from native post-add completion. Coalesce requests without
   erasing a newer request through a late receipt; review a revision-bound intent and
   the existing incarnation-bound acknowledgement together. Missing/stale ownership
   is not a successful deferral. Preserve the approved Terminal/source-save exception.
2. Give full-save orchestration an explicit outcome: confirmed application, deferred,
   not admitted/unavailable, known failure and unknown/quarantined must not all be `()`.
   Preserve existing money/item admission, reconciliation and cancellation fences.
   Dropping a submitted save future does not permit replay; cancellation before submission
   must not discard the retained request. Do not acknowledge intent before confirmed
   application or blindly retry known failure on every Session tick.
   A confirmed DB application whose old owner can no longer be acknowledged remains a
   confirmed application, not a failed transaction to retry against the replacement.
3. Drain at the worldport delayed-operation phase, after native delayed resurrection
   and before handler completion admits subsequent packets. Pass the already process-owned
   item generator through the registered capability, not a new Session-owned generator.
   After an awaited save, never overwrite Disconnecting/quarantine with LoggedIn.
   Disconnect completion must save the completed owner without replaying the ACK or
   depending on output capacity. Complete the corresponding pending-ACK, post-add,
   recovery and Terminal cases together; do not install a flag with no consumer.
4. Audit **all four** current production full-save callers: periodic processor,
   disconnect adapter, trainer pre-save and explicit logout in
   `handlers/character/world_entry.rs`. The trainer revalidates after its awaits;
   preserve that behavior. Explicit logout and the disconnect wrapper currently proceed
   toward offline/cleanup after the unit-returning save. Establish admission and outcome
   handling rather than claiming a durable save from their completion log. The explicit
   logout path does not call the new native completion helper; verify actual dispatch
   eligibility before claiming a reachable transfer bug or changing its behavior.
5. Exercise the real registered ACK and production-linked save boundary: no transaction
   while deferred, one retained request across detach/attach, resurrection reflected in
   the saved projection, confirmed acknowledgement, newer intent and replacement
   isolation, pre-submit cancellation, submitted cancellation/unknown COMMIT without
   replay, known rollback, closed/full output and disconnect/homebind/Terminal fallback.
   Existing dirty-state save tests are regression evidence, not coverage of the new intent.

This contract refines the already approved C1 operation; it adds no issue, publication
authority or production clock. Cross-clock admission/exclusion remains C0 work, and
real MariaDB/restart/relogin and action-specific captures remain separate acceptance.
C0–C4 are still open. No runtime, database, deployment or publication was performed.

Controls on unchanged production code `ae7a01b7` pass: the focused existing periodic
deferral test (1/1, `/tmp/rustycore-deferred-save-periodic-control.log`) and the
production-linked `production_save_rollback_unknown_and_cancellation_keep_native_dirty_state`
test (1/1, `/tmp/rustycore-deferred-save-fence-control.log`). The latter uses a controlled
port, not MariaDB. Five preserved persistence-policy tests pass. Documentation-only
quick and manifest verification pass; no full-suite rerun or new parity base is claimed.

## C0/C3 callback backpressure and worker failure — above `b35bba96`, 2026-09-06

The first callback integration still reached blocking `WorldSession::send_packet`.
A production-linked saturated-channel regression reproduces this on `b35bba96`:
the ready callback blocks, then the test explicitly drains the rendezvous and joins
the blocking worker before failing (`rename-saturation-before` log, 0 passed/1 failed).
It does not leave an abandoned thread or rely on an async timeout interrupting a
synchronous send. C++ `WorldSocket.cpp:526-535` enqueues into `_bufferQueue`;
`WorldSession.cpp:215-270` routes to that socket, rather than waiting for capacity
in the Rust bounded channel. Actual Rust session channels are bounded to 256 at
`wow-network/src/world_socket.rs::create_session_channels` and the instance accept path.

The corrected callback path owns `SendFut<'static, Vec<u8>>` values from the existing
channel and polls them once to reserve their FIFO positions. It retains pending futures,
not just an application outbox that later packets could overtake. The **entire ready
batch** is registered in order, including its tail when the head is pending. Accepted
futures are never repolled, only the accepted prefix is retired, and a blocked result
does not resubmit a query/commit. No unbounded replacement socket channel, transport task,
new lock or additional simulation clock is introduced. Ready DB results and read admission
wait behind retained delivery work; this is not a complete callback-resource bound.

This relies on the locked flume 0.11.1 implementation, inspected at `async.rs:94-104,
165-175,207-237` (owned send construction, cancellation and waiter registration) and
`lib.rs:444-459,535-543,590-603` (FIFO sender hooks and receiver promotion). Regression
tests preserve the required observable order across dependency updates. Dropping a pending
send at Session retirement cancels its remaining waiter; it cannot retract bytes already
accepted by the channel/socket or prove they were never delivered.

The 255-line callback coordinator now lives wholly in private `session/driver/callbacks.rs`,
absorbing the former 150-line application callback file. The application module retains
only owned read/commit stages, without transport. The same private Session field remains;
its type path is narrowed to `driver::RenameCallbacks`. The presenter constructs the owned
future once, retaining identical opcode/GUID/name/result serialization. A disconnected
sink retires Session. A lost/panicked DB worker closes read admission and retires Session
without treating its unknown completion as an ordinary DB rejection; submitted commits
remain supervised and finalization reports failure. Ordinary typed DB failures retain
their existing error response and are not conflated with worker loss.

Reviewed syntax delta: that field type path, the one crate-local presenter signature and
the matching struct bridge fingerprint change. The previous cut's final driver-wiring
test also adds one cfg(test) constructor trace, now explicitly reconciled; it was added
after that cut's syntax check. No unrelated sorting, field membership, factory body or
persistence snapshot is regenerated. Logical Session growth is +227 production lines
(driver module +226, reexport +1, including the relocated coordinator), Character +10;
there is no physical-root growth, new legacy ceiling or terminal exception. Tests remain
in the 668-line production integration target; 101 legacy ceilings still remain.

This closes the demonstrated **ready-callback** blocking/order defect, not all Session
output or C0/C3. Immediate rename admission-error responses and general send paths still
use the existing synchronous API. Global phase/barrier integration must address them,
callback resource bounds, control/shutdown progress, blocking ticks and the first complete
Player lifetime/save vertical. No real DB, client, capture, deployment or crash-recovery
acceptance is claimed by these controlled tests.

Validation on this working cut above `b35bba96` (aarch64, pinned PROTOC and existing
validation Cargo cache):

- `cargo test --offline --locked -p wow-world --test production_character_rename`:
  twelve PASS, none ignored. New cases cover the reproduced saturated callback,
  closed sink, active-worker loss/read cancellation, a later ordinary packet and
  an entire two-response batch ahead of that packet. Batch assertions compare exact
  serialized bytes, not only opcode counts. Controlled blocking producers are joined.
- `cargo test --offline --locked -p wow-world --lib character_administration::tests`:
  four PASS on the final batch implementation, including the real driver-wiring test.
- `session-ownership-check check --syntax-only`: PASS with unchanged 283 production/
  432 fixture fields, 3,677 items and 590 exact registry rows, including the reconciled
  fixture constructor. Architecture `check` and `self-test` PASS: 977 physical files,
  101 legacy ceilings and the reviewed logical counts above.
- Standalone checker `cargo test --offline --locked --release --lib persistence_policy`
  with its own manifest: five PASS; snapshot and reviewed persistence-policy semantics
  remain consistent without regeneration.
- Bounded quick PASS, verified manifest
  `target/validation-v2/manifests/20260906T143145.050078Z-3-quick.json`; final format/diff
  checks PASS. This is local dirty-tree evidence, not publication or runtime acceptance.

## C0/C3 production rename callbacks — above `ab1cdab3`, 2026-09-06

Intentional execution correction, not a behavior-preserving file move: the production
rename handler now submits the owned read and returns. A private Session callback owner
retains read/commit handles. The production driver invokes its recorded ready-only pass
after packet dispatch and before periodic save. Only that Session pass consumes prepared
reads to submit commits and presents confirmed outcomes; workers hold only owned inputs
and SQLx-free ports, never Session, Player, a map guard or packet transport.

C++ anchors re-read for this cut: `CharacterHandler.cpp:1550-1610`,
`AsyncCallbackProcessor.h:40-51`, `QueryCallback.cpp:205-224`,
`DatabaseWorkerPool.cpp:302-326` and `WorldSession.cpp:488-510`, under the legacy source
tree. Reads are inspected in registration order while pending reads are skipped. Commit
results already submitted are inspected before admitting new reads; new commits cannot
publish in their admission pass merely because a worker runs quickly. The previous Rust
result-before-response fence remains, unlike C++'s fire-and-forget commit publication.
No transaction statement, error byte, opcode registration or name/account gate changes.

Production `session_factory::create_session` closes read admission and drains submitted
commits before disconnect save and unregistering. Read workers cannot submit a commit,
even if cancellation races a ready candidate. The drain awaits handles by mutable reference
and retains them if its waiter is cancelled, so a resumed drain cannot lose/replay the write.
Retirement publishes no response. Normal disconnect awaits completion; shutdown uses the
existing bounded finalizer and marks the process unsuccessful on timeout or worker failure.
A fatal timeout is **not quiescence or proof of rollback**: a submitted worker/DB operation
may still run. No claim of crash recovery or real unknown-COMMIT classification is made.

This is the first production rename callback connection, not complete phase coordination.
Independent Session/map clocks, World/Map phase tails, first real Player lifecycle/save
integration, callback resource/backpressure acceptance, saturated sinks, blocking-tick
cancellation and full shutdown/capture/durability acceptance remain C0–C4 work. This queue
follows C++'s dynamic callback collection; it introduces no invented protocol rejection
threshold. Its eventual resource bounds must preserve admission/FIFO semantics before
global phase acceptance. Runtime installation, live tests and fresh captures are not run.

Reviewed ownership delta: one private `character_rename_callbacks` field belongs to the
existing persistence/lifecycle family (283 production + 432 fixture fields). Four narrow
Session methods are added; registry metadata remains the exact same 590 rows. The factory
and WorldSession surface fingerprints change, while their bridge evidence is unchanged.
Only those semantic syntax deltas are applied; an unrelated baseline sorting difference is
left alone. Physical Session root grows by its field/initializer, 76,745 → 76,747, with that
explicitly reviewed observation/ceiling and the same C4 exit. Logical changes are Session
+36 production/+1 test, Character +3 production, composition +24 production. The callback
implementation lives in small private modules; no blanket monolith exception is introduced.

Validation on the working cut above `ab1cdab3`, aarch64, with the pinned PROTOC and
existing local validation Cargo cache:

- `cargo check --offline --locked -p world-server`: PASS, real production composition.
- `cargo test --offline --locked -p wow-world --test production_character_rename`:
  seven production-library cases PASS. They cover two-session progress, pending read/commit,
  read retirement before/after readiness, retained handles after drain cancellation,
  query rejection, confirmed failure/success publication and failed-worker retirement.
- `cargo test --offline --locked -p wow-world --lib session::tests::driver`: ten PASS;
  `--lib character_rename`: three admission cases PASS.
- `cargo test --offline --locked -p wow-world --lib character_administration::tests`:
  four PASS, including the later wiring regression that drives actual
  `process_pending_with_catalogs_like_cpp`, not the standalone callback adapter. Removing
  callback execution from the driver would leave that test without its expected response.
- `session-ownership-check check --syntax-only`: PASS, 283 production/432 fixture fields,
  50 impl owners/3,677 items, 590 exact registry rows. Architecture `check` and `self-test`
  PASS: 978 physical files, 101 legacy ceilings and the reviewed logical deltas above.
- Standalone checker `cargo test --offline --locked --release --lib persistence_policy`
  (its own manifest): five snapshot/policy consistency tests PASS. No exhaustive snapshot
  or semantic persistence-policy regeneration is performed.
- Bounded quick PASS, verified manifest
  `target/validation-v2/manifests/20260906T135732.595086Z-3-quick.json`. This ran before
  the final cfg(test)-only driver-wiring regression; that later addition is compiled and
  exercised by the four-test command above. Final format/diff checks pass. This is not
  clean-HEAD publication validation, live-client evidence or full C0/C3 acceptance.

## C4 persistence-parser test decomposition — above `0038d265`, 2026-09-06

### Subsequent canonical inventory records — above `ef8cb6c3`

The private 548-line `persistence_access/records.rs` owns schema version 3,
record identity, accumulation/transaction associations, baseline comparison and JSON
serialization. The parser retains traversal and classification and calls the accumulator
through narrow parent-visible operations. Record identity and backing maps remain private;
the crate-local record/baseline/compare/render paths are preserved through root reexports.
No state mirror, grammar change, new persistence access or gameplay change is introduced.

The two complete source blocks match their `ef8cb6c3` originals byte-for-byte after
removing only the introduced `pub(super)` visibility. The schema constant is unchanged.
Consequently identity ordering, occurrence counts, transaction/rollback handling,
validation diagnostics and serialization retain their original bodies. Existing full-pipeline
regressions, including snapshot/policy consistency, still exercise this implementation.
All 334 standalone checker library tests pass, none ignored, on this working cut
(aarch64), using `cargo test --offline --locked --release --manifest-path
tools/architecture/handler-contract-check/Cargo.toml --lib`.

The root shrinks from 12,879 to 12,345 lines and its exact physical ceiling is tightened;
the historical observed count and C4 exit remain. The remaining AST traversal/provenance
root is still oversized; all 101 legacy ceilings remain migration debt. This does not
close C4, C0/C3 production scheduling or the complete Player lifecycle vertical.

Architecture `check` and `self-test` pass: 975 physical files, 101 legacy ceilings,
175 cohesion reviews and unchanged logical-owner counts. A narrow unused-import allowance
on the record reexport preserves its existing crate-local path even in non-test builds;
it does not suppress diagnostics on the parser or records implementation.
Final bounded quick validation passes with verified manifest
`target/validation-v2/manifests/20260906T133415.989370Z-3-quick.json` on the working
cut above `ef8cb6c3`; format/diff checks and the final physical scan pass. No exhaustive
inventory regeneration or runtime/DB acceptance is claimed for this mechanical move.

### Subsequent production SQL-text boundary — above `7bd208a5`

`persistence_access/sql_text.rs` now owns pinned literal/compile-time-string interpretation,
SQL quoting/comments and conservative advisory-lock classification. It is a private
471-line module with five parent-visible operations; symbol resolution stays a supplied
callback, not access to ModuleSymbols, the source graph or mutable flow state. Its only
parent helpers normalize identifiers/tokens/paths. No new crate, state, lock or public API.

This is a mechanical production boundary extraction. Comparing the moved block against
the original after removing only the five `pub(super)` qualifiers proves byte equality;
no function body, function signature parameter, constant or order is changed. All original
callers use the same named functions through private imports. The existing SQL-string,
SQL-classification, macro and query regressions continue to exercise the complete inventory
pipeline; they are not replaced by isolated helper tests.

The parser root decreases again from 13,336 to 12,879 lines, with its exact ceiling
tightened accordingly. The remaining root and all 101 legacy entries still need their
stated production splits/acceptance; this module does not complete C4. Full standalone
checker `cargo test --offline --locked --release --manifest-path
tools/architecture/handler-contract-check/Cargo.toml --lib` passes 334 tests, none ignored,
on the working cut above `7bd208a5` (aarch64). Bounded quick validation passes with verified
manifest `target/validation-v2/manifests/20260906T132208.087707Z-3-quick.json`; format/diff
checks pass. Snapshot/policy consistency tests remain in the full suite. No grammar or
concrete persistence source changes require another exhaustive inventory for this move;
the preceding grammar repair's comparison is not replaced with a regenerated baseline.
Architecture `check` also passes: 974 physical files, 101 legacy ceilings and unchanged
logical-owner counts. Only the reviewed parser-root ceiling is tightened.

### Test relocation evidence

Mechanical test relocation: `handler-contract-check/src/persistence_access.rs` shrinks
from 21,030 to 13,336 physical lines. All 226 tests from its former inline test module
move into 16 private responsibility modules under `persistence_access/tests/`: aliases,
cross-source and dependency contracts, generic/trait/callable resolution, type/tuple
flow, control/deferred effects, conservative escapes, macros, SQL strings/classification,
query execution and baseline/cfg rules. Files range from 184 to 943 lines; their shared
root is 45 lines and retains only the original three fixture helpers plus module mounts.
No replacement oversized test file, public API, production helper or source mirror is added.

Before moving, the complete source and exact test list were checked; extraction stops
if either input is truncated or membership differs. Every original test body is copied
with its embedded fixture strings intact, then formatted with rustfmt. The zero-context
diff changes only the old test module, starting at line 13,336: production parser code
above its `#[cfg(test)]` attribute is byte-unchanged. Existing sibling parser test modules
remain untouched. This is not a new parser behavior or persistence interpretation.

`cargo test --offline --locked --release --manifest-path
tools/architecture/handler-contract-check/Cargo.toml --lib` passes **334 tests**, none
ignored, on the working cut above `0038d265` (aarch64). Before/after `-- --list` output
has identical membership and multiplicity after removing only the introduced private
module qualifier from the moved tests. The complete suite includes the five compiler
path-oracle cases and preserved persistence-policy/snapshot consistency tests.
Bounded quick validation passes with verified manifest
`target/validation-v2/manifests/20260906T131648.290936Z-3-quick.json`; format/diff checks pass.
Architecture `check` and `self-test` pass with 973 physical files, the tightened ceiling
and unchanged logical-owner counts; the usual 175 cohesion reviews remain migration debt.

The physical policy tightens only this root's ceiling to 13,336, preserving its historical
observed size and #578 C4 retirement. Remaining production AST traversal, statement/workflow
classification and provenance still need real decomposition. The 101 legacy entries are
not closed by moving tests, and terminal physical acceptance remains open. No fresh
exhaustive inventory is needed for this test-only move: the production parser is unchanged,
and the preceding grammar repair's exhaustive comparison and snapshot/policy remain intact.

## C4 explicit module-path fidelity — above `bca3885f`, 2026-09-06

The FIFO test cut below exposed a real source-audit defect, not a prohibition in
Rust. For `#[path = "mounted/renamed.rs"] mod logical;`, an implicit `mod child;`
inside the mounted file loads `mounted/child.rs`. Both audit walkers instead appended
the mounted filename stem, selecting `mounted/renamed/child.rs`. If that other file
exists, the audit can silently inspect the wrong source and omit the compiled child;
otherwise it rejects a valid decomposition. The ordinary `mod ordinary;` rule still
uses `ordinary/child.rs` and must not be changed globally.

The correction is limited to explicit file mounts in `ownership.rs` and
`registrations.rs`. Child directories belong to logical mounts, not physical source
identity: an ordinary and an explicit mount of the same file can have different children.
The graph retains both contexts and keys traversal by source, child directory and
context. It no longer rejects those valid mounts merely for having different child
directories. Existing package confinement, cycle detection, cfg ancestry, duplicate
logical ownership and unsupported inline/conditional path grammar remain enforced.
This is a checker repair; no server scheduling, packets, persistence operation or
gameplay state is changed.

The new private `registrations/path_tests.rs` compiles each fixture with
`rustc 1.98.0 (88d9e12ae 2026-08-18)`, edition 2024, metadata only. Decoy files contain
`compile_error!`, so compiler success independently establishes which paths were not
selected. Both source-graph membership and registration traversal must then agree.
Cases cover an explicit renamed file, explicit `mod.rs`, inline and ordinary descendants,
an ordinary file, and ordinary/explicit aliases of one physical file. The initial four
cases on the old walkers give **2 PASS / 2 FAIL**, with both failures selecting decoys
that rustc did not compile. After repair all five cases pass. These compiler observations
complement the [Rust Reference module-path rules](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute);
they do not claim that the checker accepts every Rust module grammar.

Focused/full checker evidence on the working change above `bca3885f` (aarch64):
`cargo test --offline --locked --release --manifest-path
tools/architecture/handler-contract-check/Cargo.toml --lib registrations::path_tests`
passes all five cases; the same command with `--lib` and no filter passes **334 tests**,
none ignored, including the repository registry contract and preserved persistence
policy/snapshot tests. The existing 42 Session path mounts are unchanged. Source sizes
shrink from 1,973 to 1,959 lines in ownership and from 1,452 to 1,451 in registrations;
the new tests occupy 177 lines. No physical or logical ceiling is increased.

Architecture `check` passes (956 physical files, 101 retained legacy ceilings and
unchanged logical counts). Bounded `validation-v2 quick --base HEAD` passes with
verified manifest `target/validation-v2/manifests/20260906T125506.332071Z-3-quick.json`;
format/diff checks pass. This is dirty-tree iteration evidence, not publication final.

Because this changes the source grammar consumed by the persistence inventory, the
exhaustive `session-ownership-check check` comparison was required for this cut.
It completes with PASS: 7,773 production + 2,311 fixture persistence rows (10,084 total),
1,027 semantic groups, 65 bridge rows, 48 generated inputs, 590 registry rows and the
unchanged Session field/item/install/command surfaces. No snapshot or policy regeneration
was needed. Their SHA-256 values remain respectively
`27009495e4d54fd339c93a21a7bef48a365b8e7a2f86110fe4b506884efc3d4e` and
`98baa1e37e3d4320109eae698d796af2a20e41a69dd10672c54b210e2956ef1f`.
Exact command: `cargo run --offline --locked --release --manifest-path
tools/architecture/handler-contract-check/Cargo.toml --bin session-ownership-check -- check`;
local result log `/tmp/rustycore-578-path-inventory.log`. This proves inventory preservation
on the reviewed working source above `bca3885f`, not macro/terminal or runtime acceptance.

## C0 FIFO queue conservation — 2026-09-06, above `bdae6204`

Exact C++ inspection adds an important constraint to the earlier filter contract:
`WorldSession.h:1920` declares `LockedQueue<WorldPacket*> _recvQueue`, and
`src/common/Threading/LockedQueue.h:82-98` checks **only the front packet**. If its
filter rejects it, `next` returns false without consuming or scanning ahead. A future
world/map phase adapter must stop at that head, not search for a later eligible opcode.
`WorldSession.cpp:354-490` consumes one selected packet and readds its separate
status-deferred list after the pass. Processing-place rejection and status requeue
are distinct operations; neither is implemented by relabeling the current driver.

The production Session queue now uses its existing VecDeque import and selects one
front packet immediately before the sole registered dispatcher call. The old
`drain(..).collect()` moved all pending packets into the async pass; cancelling that
future could destroy packets whose handlers had not been entered. The selected packet
remains consumed on cancellation because the handler may already have effects; unselected
packets remain with Session in FIFO order. Destroying Session still destroys its queue.
This is explicit cancellation behavior hardening, not durable delivery or rollback.

Both primary and realm ingestion now account for retained pending packets against
the existing shared 100-packet bound. With an empty pending queue the existing order,
budget, bytes, status gate and handler calls are unchanged. At capacity, surplus stays
in the transport channel. No new queue, clock, task, lock, mirror, handler registration
or persistence operation is added. This keeps the current Rust intake budget; it does
not claim equivalence to the C++ `processedPackets > 100` per-update stopping condition
or solve the broader intake/AntiDOS/status-requeue ordering differences.

Three focused tests in the existing driver module poll the actual async driver and
dispatcher using a per-session test thunk (not an inventory registration): cancellation
after the first handler entry retains exact bytes of the next two packets, a resumed
pass does not replay partial effects and preserves order, an unpolled dropped pass
consumes nothing, and retained packets share the next ingestion capacity. No timer,
real database or production scheduling is simulated by those assertions.

Reviewed guard delta: `pending_packets` changes only Vec to VecDeque in the exact
field ledger. The full Session bridge surface fingerprint changes by that five-character
type spelling; its legacy/canonical evidence fingerprint and multiplicity are unchanged.
The logical owner grows by 7 production and 115 test lines for this in-scope C0 safety
contract; its exact ceiling and latest review are updated manually, retaining #578
retirement. The existing driver tests remain one 290-line responsibility module; no
physical monolith ceiling is increased. No persistence snapshot/policy is regenerated.

An initial external test child exposed a checker/Rust path-resolution discrepancy:
for the existing `#[path]`-mounted driver test file, rustc resolves its implicit child
beside the mounted file while the source-graph collector expected the filename-derived
subdirectory. The 115-line probe suite stays inline in its cohesive 290-line driver
test module; the same tests execute, without a new mount or ignored checker error.
The compiler-backed C4 repair above addresses that child-resolution discrepancy;
the original probe remains inline and no test is removed or silently remounted.

Validation on the working source above `bdae6204` (aarch64, reused validation-v2
Cargo target cache, not a fresh build):

- `cargo test --offline --locked -p wow-world --lib session::tests::driver`:
  10 PASS, no ignored tests; `cargo test --offline --locked -p wow-world --lib`:
  3,746 PASS, 1 pre-existing ignored. Final suite includes the inline probe tests.
- `session-ownership-check check --syntax-only`: PASS, unchanged 590 registry rows.
- `check_architecture.py check`: PASS, 954 physical source files, 101 legacy ceilings.
- Standalone checker `cargo test --offline --locked --release --lib persistence_policy`:
  5 PASS, including reviewed persistence semantics and snapshot/policy consistency.
- `validation-v2 quick --base HEAD`: PASS and verified manifest
  `target/validation-v2/manifests/20260906T120958.598002Z-4172-quick.json`.
  Dirty bounded iteration is not a clean-HEAD publication final or live QA.

Phase-filter activation, current-owner error handling at dispatch, actual world/map
coordination, queue-head handoff and production timing/capture acceptance remain open.

## C0/C1 checked canonical residence — 2026-09-06, above `590b93f0`

The next cut adds `MapManager::checked_player_residence_like_cpp` in a private
`manager/player_owner/resolution.rs` module. It resolves the ownership index,
generation, expected container, Player GUID, IsInWorld and current map binding under
the caller's existing manager borrow. Missing owner, replaced generation, missing
map/Player and inconsistent identity/residence have distinct errors. This is a
point-in-time observation, not an execution lease or a borrow across await.

The existing `player_residence_like_cpp` now projects this checked result to Option.
This intentionally tightens invalid-state admission: previously a surviving index
alone returned Some(Active/Detached) even if its backing Player/map was absent.
Existing signatures and valid settled lifecycle results are preserved. The failure
behavior change is explicit, not claimed as pure mechanical movement. Existing
Option callers remain transitional; new phase admission must preserve the checked
errors rather than interpret every None as a session without a Player.

Affected current callers are the coherent save preparation in
`wow-world/src/session/lifecycle/persistence/prepared.rs`, save-header capture, owner
existence/attach/detach/position resolution and current-map queries in Session.
No second Player, new lock, runtime loop, packet registry change, SQL operation or
default state is introduced. Actual lifecycle mutation methods still use their
existing generation and transition guards; this cut does not globally remove broad
mutable Map/Player escapes or discover duplicate records in unrelated map containers.

C++ anchors: `Server/WorldSession.cpp:64-108` reads the real Player and IsInWorld;
`Maps/Map.cpp:427-462` binds/adds the active Player, and `:907-934` removes it from
world/grid. Rust's settled active/detached ownership contract requires the backing
container and world binding to agree. The check is not asserted inside C++'s
intermediate leave-map callbacks, nor proof that Rust implements all those callbacks.

The new negative fixtures deliberately corrupt private index/container state or use
existing broad mutable access; they reproduce unsafe query results, not live Player
loss. A separate `wow-map/tests/player_residence.rs` target exercises real public
production-library install, failed attach, transfer, retirement and replacement,
retaining the same Player's money. It does not use a fake owner or cfg(test) storage.
Evidence on the working tree above `590b93f0` (aarch64):

- `cargo test --offline --locked -p wow-map --lib`: 712 PASS, 1 pre-existing ignored;
  includes 10 new resolution tests. Root 685 lines, private resolver 79, private
  tests 201, production-linked integration 62; no physical ceiling is increased.
- `cargo test --offline --locked -p wow-map --test player_residence`, in dev and
  with `--release`: 1 PASS each, no ignored cases.
- `cargo test --offline --locked -p wow-world --test production_login_player_owner`:
  6 PASS, including coherent save, later mutations, replacement and rollback/unknown/
  cancellation fixtures. This does not establish real DB durability.
- `cargo test --offline --locked -p wow-world --lib`: 3,743 PASS, 1 pre-existing
  ignored. This run reuses the validation-v2 Cargo target cache; it is not a fresh build.
- `PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo check --offline --locked
  -p world-server`: PASS with the same reused cache; existing warnings remain.
- `check_architecture.py check`: PASS (954 source files, 101 legacy ceilings);
  syntax-only Session ownership PASS with unchanged 590 registry rows.
- `validation-v2 quick --base HEAD`: PASS, verified manifest
  `target/validation-v2/manifests/20260906T115021.478444Z-23-quick.json`.
  This is bounded dirty iteration above `590b93f0`, not full-PR publication evidence.

C0 phase coordination and the complete C1 lifecycle/durability acceptance remain open.
For that next integration, the current `MapUpdateVisitPlan::session_update_players`
field is only constructed in `map/mod.rs:3544-3608` and asserted in `map_tests.rs`;
no production consumer dispatches those sessions. `ManagedMap`'s actual update path
in `manager.rs:499-602` starts dynamic-tree/represented object-family updates without
consuming that plan. Wiring a phase to the plan alone would therefore miss the live
driver. The production coordination cut must join the actual Session runner and
ManagedMap update path, not claim execution from the visit-plan fixture.

## C0 packet-filter contract and integration cut — 2026-09-06

Source audit at `36d0ccbf`; the following working change starts the executable C0
contract, **not production phase enforcement or C0 acceptance**. The live #578 body
was also read on this date; its historical "conformance has not run" statement is
superseded by the completed finite gate recorded below, not a reason to repeat it.

Exact C++ references under `src/server/game/`:

- `Server/WorldSession.cpp:64-108`: `MapSessionFilter::Process` and
  `WorldSessionFilter::Process`. Inplace passes both filters; ThreadUnsafe only the
  world filter; ThreadSafe the map filter only for an existing in-world Player,
  otherwise the world filter. Missing and detached Player are distinct observations,
  even though these two filters give them the same eligibility.
- `Server/WorldSocket.cpp:472-499`: the default registered-opcode path queues the
  packet. Inplace is not a general instruction to invoke it on the socket thread.
- `Server/WorldSession.cpp:339-475`: queue filtering precedes the independent status
  gate and invocation. LoggedIn without a Player can be requeued when not recently
  logged out. Dropping a packet because it belongs to the other phase is not equivalent.
- `World/World.cpp:2704,2748,3394-3418`: world-session update precedes MapManager update.
  `Maps/Map.cpp:666-718` runs map sessions before respawns and Player/object updates;
  `Maps/MapManager.cpp:287-318` joins map updates before DelayedUpdate.

Current production call-path evidence:

- `world-server/src/session_factory.rs:170-208` independently runs the synchronous
  Session pass and then awaits `process_pending_with_catalogs_like_cpp`, with its own
  elapsed diff and idle sleep. Calling this pass a map phase would not create a barrier.
- `wow-world/src/session/driver/mod.rs:280-294` drains every pending packet and invokes
  the same registered dispatcher, after Session gameplay ticks. `dispatch.rs` checks
  SessionStatus but never reads PacketProcessing.
- `world-server/src/runtime/map.rs:1606-1695` independently spawns the canonical map
  interval and acquires its manager guard for the synchronous update. Moving the async
  Session dispatcher under that guard would violate the no-I/O/await-under-map-lock
  contract, not repair the missing phase order.
- `wow-map/src/manager/player_owner.rs:163-186,338-345` exposes generation-filtered
  residence/Player queries as Option. The production phase adapter must not collapse
  stale or inconsistent resolution into Missing, or substitute SessionState for the
  canonical Player's actual in-world state.

Implementation: `wow-handler/src/processing.rs` owns the pure filter vocabulary and
`PacketProcessing::allows_phase`; its private tests exhaust all 18 phase/class/residence
combinations and eligibility changes through attach/detach/reattach/retirement observations.
The enum documentation now matches the exact C++ filters. No new task, lock, queue,
registry entry, state mirror, SQL operation or handler invocation is introduced. Existing
packet bytes, status handling, order and current runtime behavior are unchanged.

Focused evidence on the working change above `36d0ccbf` (aarch64):

- `cargo test --offline --locked -p wow-handler`: 2 tests PASS, no ignored tests.
- Same command with `--release`: 2 tests PASS, no ignored tests.
- `python3 tools/architecture/check_architecture.py check`: PASS; 951 physical
  source files, 101 remaining legacy ceilings, unchanged logical ownership totals.
- `session-ownership-check check --syntax-only`: PASS; 590 exact registry rows.
- Checker `cargo test --offline --locked --release --lib persistence_policy` with
  its standalone manifest: 5 tests PASS, including preserved reviewed semantics and
  checked snapshot/policy consistency. No persistence inventory/policy is regenerated.
- `validation-v2 quick --base origin/3.4.3`: PASS and verified manifest
  `target/validation-v2/manifests/20260906T113510.650238Z-3-quick.json`; dirty iteration
  evidence, not a clean-HEAD publication final. Format and diff checks also pass.

The active PORT_PLAN and ownership guide also retire their stale "conformance has not
run" statements in favor of the existing V2 evidence; no acceptance gate is removed.

Remaining integration/deletion conditions: use the sole registry's metadata during real
phase selection; resolve canonical incarnation/residence with explicit errors; retain
ineligible queued packets and re-evaluate after transitions; consume Inplace exactly once;
separate existing status/requeue discrepancies from structural movement. Implement the
world/map coordination and cancellation/backpressure/barrier contract alongside the first
real operation, then test the production call path before replicating it. The filter is
not a second dispatcher or evidence that an independent Session task runs in Map::Update.
No scheduler, storage, durability or live/capture acceptance is claimed by these unit tests.

## Scoped callable provenance guard — 2026-09-06

The local C4 change above published `9cd1da41` repairs the callable import/reexport
limitation recorded in the adapter cut below. Two adversarial fixtures first failed:
crate-restricted statement-builder returns lost value/argument provenance, and a private
ancestor import chain lost the returned pool. The incomplete boundary was the AST inventory;
this repair does not alter the server operation.

Callable collection and resolution now live in a private 402-line module with a separate
570-line responsibility test module. The main parser shrinks from 21,248 to 21,030 lines;
its non-growth ceiling is tightened to that size. It remains oversized C4 debt, not a
terminal exception. There are still 101 initial oversized files. No server code, packet,
SQL plan, transaction, runtime owner, public server API or lifecycle contract changes.

Public provider reexport resolution retains its existing algorithm. Crate-local aliases
are resolved only **after** dependency caches are assembled, so restricted/private names
do not become exports in another package. Fixed-point resolution follows named and immediate
glob imports, preserves explicit/local function shadowing, honors cfg context and bounds
cycles without inventing recursive `tests::tests` paths. Return information, generic inputs
and mutable-argument effects follow the same local aliases. This is provenance for the
bounded AST grammar, not an implementation of Rust privacy/type checking.

A further adversarial case rejected the initial candidate: an unresolved
`F: FnMut() -> Fut`, `Fut: Future<Output = Result<T, E>>` chain could silently lose a
callback's actual returned pool. Unresolved generic output slots now conservatively retain
known argument provenance. Explicitly substituted outputs keep their precision; declared
concrete String returns do not acquire a pool merely because an input contains one. Higher-order
where-clause inference is **not solved**: false positives can remain where the grammar
cannot separate captures from results. Concrete pool-bearing containers, unknown calls,
mutable outputs and returned callback pools remain covered by adversarial fixtures.

The aarch64 locked/offline release checker suite passes **329 tests**, including all
315 pre-existing tests and 14 new cases. The differential statement test preserves consumer
targets, operations, symbols and counts across direct versus reexported builders.
Architecture check/physical budgets, formatting and diff checks pass. Routed quick validation
is verified green at the recorded dirty iteration above `9cd1da41`:
`target/validation-v2/manifests/20260906T105240.399402Z-3-quick.json`.

The first 10,062-row inventory was **not accepted or installed**: it predates the conservative
generic-output safeguard. The final exhaustive run yields **10,084 rows**, versus 10,116
at `9cd1da41`: 10,063 existing rows remain identical, including occurrence counts; 53 false
concrete-pool aliases/escapes are removed and 21 rows added. Every changed row was reviewed:

| Reviewed inventory delta | Rows | Current source evidence |
| --- | ---: | --- |
| Task/mailbox/stop results incorrectly treated as concrete pools | -19 | `world-server/lib.rs:310,322,346,436,495`; `runtime/map.rs:1606`; `runtime/delivery.rs:1606`. Mailboxes hold commands; returned handles/stop flags are not DB handles. Actual worker/port operations remain inventoried. |
| Listener-count summary incorrectly treated as four DB kinds | -12 | `world-server/shutdown.rs:37,182`: the result owns only a `usize` listener count. |
| Owned disable catalog and derived mmap configuration incorrectly treated as WorldDatabase | -21 | `world-server/bootstrap/config.rs:255,267`; `condition_disable_catalog.rs:53`; `wow-data/disable_mgr.rs:74,124`: owned flags/ID sets and configuration, not a retained pool. |
| Locale String incorrectly treated as LoginDatabase | -1 | `world-server/session_factory.rs:474`: concrete owned String return. |
| Actual cleanup transaction passed to commit | +1 | `world-server/lib.rs:191`, consumed by `app.rs:291`: six prepared cleanup statements, returned as `SqlTransaction`. |
| Unresolved generic integer results retained conservatively | +17 | `wow-database/{battle_pet_selection,game_tele,item_random_enchantment}_catalog_adapter.rs:13–35` and `player_creation_catalog_adapter.rs:56`. Numeric DTO fields are not new DB handles; this is documented over-reporting, not gameplay work. |
| Generic async lock fixture provenance retained conservatively | +3 | `wow-database/transaction.rs:778,1604`, test-only source class. |

The two retry-result `Applied` rows in group/stored-item loot money are deliberately retained;
the rejected first candidate dropped them. No previously recorded direct SQL operation is
removed or reclassified. The final policy generated from the reviewed snapshot compares
**byte-identically** to the existing policy; workflow annotations, ordering, connection,
unknown-COMMIT and retirement contracts are unchanged. No policy ceiling or exception is widened.

Installed snapshot SHA-256: `27009495e4d54fd339c93a21a7bef48a365b8e7a2f86110fe4b506884efc3d4e`;
unchanged policy SHA-256: `98baa1e37e3d4320109eae698d796af2a20e41a69dd10672c54b210e2956ef1f`.
The snapshot was installed with a reviewed patch and compares byte-for-byte to the generated
artifact. Full delta: `/tmp/rustycore-578-scoped-callable-conservative-review.json`.
After installation, all five persistence-policy consistency/rejection tests pass; post-install
quick validation is verified green:
`target/validation-v2/manifests/20260906T111125.449900Z-3-quick.json`.
The adapter's currency builder and explicit test imports are deliberately not moved again
in this guard repair. This improves the checked grammar; it does not close C4 or prove a
complete Rust type system, production storage migration or live durability.

Publication already completed separately at clean `9cd1da41abb6721be91155707492ef80dc787710`:
20/20 final commands, 7,185 Rust tests passed, four ignored, no failures; manifest verification
is green (`854a7ac38d23de054b79`). PR #579 remains draft. That publication does not include
this local guard repair, close #578/#133, or authorize a merge/runtime restart.

## MariaDB lifecycle adapter decomposition — 2026-09-06

The structural cut above `1e6b7c40` preserves the existing
`MariaDbPlayerLifecycleAdapterLikeCpp`, its three pool handles and its single lifecycle-port
implementation. The root shrinks from 4,957 to 1,608 lines. Seven private modules own economy
statement plans, login writes, login reads/decoding, transport reads, collection reads,
full-save steps/bindings and full-save ordering. The largest extracted production file is
693 lines. Four test modules separate economy/writes, login reads, save-step mapping and
save ordering; their largest file is 612 lines, with narrow shared fixtures in `tests.rs`.
The exact legacy physical-policy row is retired; **101 initial oversized files remain**.

The remaining root exceeds the 1,000-line review signal but not the 2,000-line limit.
Cohesion review keeps the one existing trait implementation and its transaction/outcome
branches together for this mechanical cut. This is not a new terminal exception or closure
of the broad lifecycle capability: C2 must still address that real operation boundary.
No pool, lock, trait, async gate, queue, mutable owner or transaction boundary is added.
The public adapter/tutorial builder paths are preserved, the latter through a scoped module
reexport. The shared crate-visible currency builder keeps its original root definition and
body: the existing analyzer loses caller provenance through a restricted callable reexport.
Private moved builders are visible only to the parent adapter and its descendants.

Before editing, C++ `Entities/Player/Player.cpp:19323` (`SaveToDB`, including deferred far
save) and `Handlers/CharacterHandler.cpp:88` onward (`LoginQueryHolder` statement setup)
were contrasted with the current adapter and the
[bounded lifecycle contract](../migration/player-lifecycle-persistence-contract.md).
Existing sequential-load, partial-save and separate account-collection transaction differences
remain unresolved; relocation does not approve them as C++ parity.

All 1,850 helper-source lines are accounted for exactly once (1,813 relocated; the shared
37-line currency builder retained). Independent rustfmt-normalized
comparisons preserve every extracted builder body and the complete original adapter/trait
implementation; the four moved test bodies also match, except the fixture's relative path
which still names the same unchanged JSON file. All 25 prior test identities execute and pass
on aarch64; only their responsibility submodule prefixes change. No statement, binding,
transaction ordering, await, commit classification or packet path changes.

The first exhaustive scan exposed two distinct issues instead of supplying an acceptable
replacement baseline. Its checked inventory dates from `05ca65ce`, before `11c34e6b` retired
five on-demand adapters and introduced startup query catalogs/mail hydration, and before
subsequent process-resource borrowing and nullable-LFG test changes. The current code audit
reconciles those historical paths separately: 27 annotations for absent adapters are retired;
six startup `world_query_catalog_adapter` workflows are annotated from actual source and
`app.rs:2290`/`world_query_catalog::load_like_cpp`. The ObjectMgr C++ loaders at
255/349, 6143/6189 and 7461/7552/7770 establish the startup responsibility. Their seven SELECTs
are independent reads, not a transactional snapshot. This reconciles inventory, not wider
packet or database parity; unrelated historical policies/retirement issues are not retargeted.

The initial test-module import chain also hid PreparedStatement value/argument/macro evidence;
tests now name concrete builder imports explicitly. Retaining the original currency definition
protects vendor/inventory transaction provenance instead of accepting four missing inventory
rows (including the vendor row's three occurrences). The follow-up C4 scoped-callable guard
recorded above addresses that prerequisite for its tested grammar; the explicit imports and
root currency definition remain unchanged. No analyzer weakening or blanket baseline drop
was part of this adapter cut.

The final exhaustive `session-ownership-check print-persistence-baseline` run completed on
the final source, with **10,116 access rows** versus the checked 10,255. The reviewed delta
accounts for historical sources (-174 rows), the prior Mail branch (+4), and explicit module
imports (+31). After source/module relocation, scoped visibility and two exact rustfmt-only
trailing-comma changes are normalized, no other body or consumer access disappears. In
particular, the vendor/inventory currency references are identical, including occurrence counts.
The first/intermediate inventories with missing references were **not installed**.

Snapshot SHA-256: `8203a54719416170fd3716f3dcefd521b61057267205c02e33a53765c20bb562`.
The policy is rendered from that same reviewed artifact and the preserved workflow annotations;
SHA-256: `98baa1e37e3d4320109eae698d796af2a20e41a69dd10672c54b210e2956ef1f`.
Both installed files compare byte-for-byte to those outputs. The retained workflow contracts
compare unchanged after the exact path reconciliation; no remaining retirement issue or
failure/unknown-COMMIT contract is relaxed. The five persistence-policy tests now pass,
including checked-snapshot/policy consistency and stale/overlapping/unowned rejection.
Local review artifact: `/tmp/rustycore-578-adapter-inventory-verified-review.log`.

aarch64 validation uses `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`, locked/offline Cargo
and two build jobs: `cargo test -p wow-database --lib` passes **354**, with the same two ignored;
`cargo test -p wow-world --test production_login_player_owner` passes **six**; and
`cargo check -p world-server --all-targets` passes with existing warnings. Architecture
check/self-test, syntax-only ownership, formatting and diff checks pass. Quick validation
for this cut uses `--base 1e6b7c40`; manifest before the final inventory installation:
`target/validation-v2/manifests/20260906T094333.362607Z-3-quick.json`.
The post-installation quick pass is
`target/validation-v2/manifests/20260906T095839.334914Z-3-quick.json`.
This is not the whole-PR final gate. No live durability, fresh capture, installation,
publication or macro completion is claimed by this structural cut.

## Persistence contract source decomposition — 2026-09-05

Above `d3f5c20c`, the former 4,513-line `wow-persistence/src/lib.rs` becomes a
544-line facade. Twenty private modules group the existing contracts by operation:
login, save, lifecycle, account collections, economy, battle pets, groups, social,
void storage, session administration/account and the smaller query/runtime capabilities.
The existing 252 root public declarations retain explicit root reexports. There are no
new crates, dependencies, runtime owners, port methods or shared contexts. The largest
new module is `player_login.rs` at 766 lines; the unchanged crate-root regression suite
lives in `tests.rs` at 671 lines. The reviewed physical-policy row for the facade is
retired, reducing the initial 103 legacy oversized files to **102**, without raising
another ceiling or granting an exception.

This is a behavior-preserving relocation, not terminal C2 capability cohesion:
`PlayerLifecyclePortLikeCpp` retains its existing breadth. `PersistenceOutcomeLikeCpp`
and its Applied/Failed/Unknown distinctions, transaction requests and adapters are unchanged.
The owning behavioral contract remains
[`player-lifecycle-persistence-contract.md`](../migration/player-lifecycle-persistence-contract.md).
C++ anchors inspected before the move include `Entities/Player/Player.cpp:19323`
(`SaveToDB`, including far-transfer deferral), `:19685` (pet/save continuation),
`:19692` (`SaveInventoryAndGoldToDB`), and `BattlePets/BattlePetMgr.cpp:240/:331`
(`LoadFromDB`/`SaveToDB`). Existing timing/transaction differences are not repaired
or approved by moving their declarations.

Mechanical comparison against `d3f5c20c` accounts for every original declaration/body
exactly once: all 20 extracted module bodies and the complete original test body match
after independent rustfmt normalization. On aarch64, the unchanged persistence suite
passes before and after (35 tests, identical test paths); all 25 lifecycle adapter tests
pass; all six `wow-world --test production_login_player_owner` integration tests pass
(including late mutation, replacement, rollback, Unknown and cancellation). These use a
controlled persistence port, not live MariaDB. `cargo check --offline --locked -p world-server --all-targets` passes with
existing warnings. Architecture check/self-test, the physical guard (935 source files,
102 legacy ceilings), and the syntax-only Session ownership ratchet pass without changes
to the eight logical-owner budgets. `validation-v2 quick --base d3f5c20c` passes; initial manifest:
`target/validation-v2/manifests/20260905T225133.566739Z-1278495-quick.json`.
No packet/SQL change, live runtime installation, fresh capture, publication or macro
completion is claimed. The remaining source and semantic cuts stay within #578 C0–C4.

## Physical source ratchet and checker separation — 2026-09-05

Above `8f5caedc`, #578 C4 implements the approved physical branch in the existing
architecture checker. This is tooling/guardrail work, not gameplay ownership, scheduler
or parity progress. `physical-files` measures repository source, integration tests, tools,
supported non-Rust languages and extensionless scripts; `check`/`self-test` also enforce it.
The prior logical-owner production/test/total checks and syntax/mount coverage remain intact.

Initial inventory: **914 source files**, 176 above the ordinary 1,000-line cohesion-review
signal, and **103 above 2,000 lines**. The exact paths, observed counts, non-growth ceilings,
concrete split targets and `578:C4` checkpoint are in `physical-file-policy.json`.
They are migration debt, not 103 justified terminal exceptions. The initial terminal-exception
and generated-source lists are empty; `misc_generated.rs` remains handwritten, and vendored
Detour code requires its stated upstream/provenance review rather than a blind source rewrite.

The default migration check passes only with the reviewed ceilings. New oversize files and
missing/renamed legacy paths fail; validated reductions can tighten/retire their exact rows.
`physical-files --terminal` **correctly fails on all 103 unresolved oversized files** at
introduction, so this tooling cannot make the unfinished macro appear physically complete.
Concrete exceptions require owner/responsibility/rationale, dated review/expiry and a named
checkpoint. Generated attribution checks pinned generator/inputs/output plus an exact matching,
hash-pinned reproduction record; it does not execute generators or certify a fabricated record.
The full contract and coverage exclusions live in the module-design guide.

`hotspot_metrics.py` mechanically receives 27 existing Rust scanner/size/ratchet functions;
AST comparison to `8f5caedc` finds **zero changed function bodies**. The common error/root
vocabulary is shared by two private modules, not imported back from the CLI. The main checker
shrinks **3,858 -> 3,106 physical lines**; its reviewed ceiling tightens to 3,106, with its
remaining policy/fixture decomposition still open C4 debt. New physical implementation/tests
are 218/236 lines; the existing metric module is 807. No root Rust owner ceiling changes.

Every nonempty `validation-v2 final` diff now runs the cheap physical branch, including
tooling-only changes, source deletion and generator inputs. Workspace Rust independently
retains the logical ratchet. Focused physical-policy and shared-scanner changes trigger their
own tests during quick iteration, without the exhaustive persistence inventory. The validation
planner tests pin this routing and preserve the existing final test-target contract.

aarch64 evidence: 20 physical adversarial unit tests pass, including production/tests/tooling
coverage, growth, path moves/rename to a different extension, reduction/tightening, terminal
failure, stale/expired policy, source symlinks, ignored build products and generated provenance
drift. Existing architecture self-test/check and validation-v2 contract tests pass. The
retained logical checks still report the same eight audited owner roots; no runtime/source owner
or packet/SQL contract is moved. No C++ fidelity change, live runtime action, publication or
closure of C0–C4 is implied. The next source split uses this ratchet; the remaining 103 paths
are not deferred to #583 or the #153 auditor.
`validation-v2 quick --base 8f5caedc` passes with manifest
`target/validation-v2/manifests/20260905T224117.829362Z-1270735-quick.json`.

## Authorized real save/relogin QA — 2026-09-05

The user explicitly authorized temporary installation/restart of the test `world-server`,
normal auth/character writes for existing `TESTBOT1@bot.local`, and restoration of the prior
executable. The bounded run **passed and restored**. It did not authorize fixture setup,
account provisioning, publication or merge; none occurred. `bnet-server` stayed on PID 4153913.

Runtime source is `68fb338b`; QA tooling is committed at `04d54074`. The later commit changes
only the integrated bot/docs, with no Cargo/runtime source delta. On aarch64,
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=2 cargo build --offline --locked
--release -p world-server --bin world-server` passed (existing warnings; 8m46s). Bot build and
all 143 tests passed; its row-retention test covers default additions versus removal/change.
The report verifier passes one positive and 11 negative checks; runtime restoration self-test
passes 69 checks. `validation-v2 quick --base 68fb338b` passed with manifest
`target/validation-v2/manifests/20260905T212436.394907Z-1231315-quick.json`.

The clean detached source checkout `/tmp/rustycore-578-runtime.l8Dzf9/source` is exactly
`04d54074`; the tracked runtime sources were compared to the build source before copying the
executables into that private evidence directory. This avoids moving or ignoring the unrelated
untracked LFG audit in the primary worktree. The unchanged runtime guard ran from that clean
checkout with `QA_LIVE_DIR=/home/server/rustycore/target/deploy/live`, the copied/hash-pinned
bot, the operator's private `QA_ENV_FILE`, and `QA_SMOKE` pointing to the new maintained
`tools/wow-test-bot/run_login_save_relog.sh`, using `--allow-runtime-qa ... login`.
The guard disabled provisioning, password generation and fixture
modes, checked both listening ports/PID, and retained its restore trap.

Evidence at 21:27:53 / 21:27:55 UTC:

- Exact account 8 / character 14 was verified offline and owned by the approved identity
  before authentication. Two fresh BNet/world authentications entered world and drained
  both login streams. Both normal logouts observed an empty `SMSG_LOGOUT_COMPLETE`, not
  socket-loss fallback, followed by the exact character's offline database row.
- `logout_time`: `1788568309 -> 1788643673 -> 1788643675`. The candidate's journal records
  two full-save COMMIT confirmations for GUID 14, each with 428 statements; no failed or
  Unknown full-save outcome was recorded during that process lifetime.
- All existing rows survived both saves. The six ordered database projections match before,
  between and after the sessions: **13 skill rows and 207 reputation rows**, with empty
  persisted spell/favorite/equipment-set/transmog-outfit tables. Both client login packets
  contain the same 42 known spells and zero favorites. Empty families are empty-state
  regression evidence, **not proof of real spell/equipment INSERT/UPDATE/DELETE**. Default
  known-spell hydration is not confused with nonempty `character_spell` durability.
- Candidate process 1233909 served SHA-256
  `3cc20114ca3c90508af2996e6d2d1a768e23541bc87de438e48ffbbe38c78378`.
  Bot SHA-256: `198755ea45dc03e22d9680d04ad3efab0502c41f4323374d92be1b3fa46baab9`.
- Original executable SHA-256
  `c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`
  was restored and verified serving on PID 1234826, both ports owned by that process,
  zero automatic restarts, no packet dump. The character remained offline. Normal
  authentication/logout writes are retained; no database rollback or fixture cleanup is claimed.

Outer report: `/tmp/rustycore-578-runtime.l8Dzf9/runtime-report.json` has
`outcome=passed-restored`, `bot_status=0`. Private phase reports/logs and the original executable
are retained under `/tmp/rustycore-login-qa.v7SWud`; `bot.json` reports
`login_save_relog_verified=true`. The paragraph above preserves sanitized results if temporary
artifacts expire; credentials and raw logs are not committed.

The reusable bot code is a private 175-line `src/login_save.rs` module plus a bounded two-pass
wrapper/report checker. It adds no SQL writes or gameplay fixtures. C++ anchors remain
`CharacterPackets.cpp:535 LogoutRequest::Read`, `WorldSession.cpp LogoutPlayer` (save before
the `:676` completion packet), and `Player.cpp SaveToDB/_SaveSpells/_SaveSkills/
_SaveEquipmentSets`. This is actual normal-save/relogin evidence, not a crash/restart-between-
saves durability test, injected DB-failure/concurrent-edit proof, transfer/evacuation QA,
scheduler parity, full character parity, or a fresh capture. Controlled tests retain those
in-flight acknowledgement branches. Broader #578 C0–C4 and PR #579 acceptance remain open.

## Save acknowledgement follows reputation key selection — 2026-09-05

Review above `ec3aa459` found a defensive edge in the first save cut: native reputation is
currently a vector, while the actual projection builds `FactionStateList` keyed by
`ReputationListID` (`ReputationMgr.h:63`; `ReputationMgr.cpp:792`). With duplicate keys,
`ReputationMgrLikeCpp::from_player_gameplay_state_like_cpp` selects the last row, but the
receipt had retained both native rows. A real save-pipeline test ran red: the single-row
request caused both native rows to be marked clean.

The receipt now applies the identical keyed selection. The omitted row stays dirty; it is
not silently declared durable. The ordinary native two-faction test uses distinct list keys,
as a valid C++ container does. This intentionally malformed-state reproduction is not evidence
of duplicate rows in a live database, nor a new normalization/repair policy for corrupt state.
No SQL projection, field narrowing, statement order, lifetime or concurrency policy changes.

Reviewed delta: native Player +9 production/+2 tests; Session +41 tests and no production/API
growth. The receipt/test modules remain 210/222 lines and the save-interleaving module 268.
aarch64 focused validation above `ec3aa459`, with explicit `PROTOC` and two build jobs:
six `wow-entities --lib player::save_ack` tests, 40 `wow-world --lib
session::tests::lifecycle_persistence` tests and six production `production_login_player_owner`
integration tests pass. Architecture check/self-test and syntax-only ownership check pass.
`./tools/validation-v2 quick --base ec3aa459` and its green-manifest verification pass:
`target/validation-v2/manifests/20260905T210159.182056Z-1209293-quick.json`.
All broader C0–C4/real-durability boundaries remain.

## Canonical map occupancy; manual count retired — 2026-09-05

Above `ca7034fa`, `ManagedMap::player_count`, non-GM admission count, destruction admission
and `MapManager::num_players_in_instances` now query the canonical typed Players. The mutable
`player_count` field, initialization, public setter and fallback are removed, not synchronized
to another count. No new occupancy mirror/index, clock or writer is introduced.

The production test `production_occupancy_follows_attach_detach_and_game_master_state` ran red
against the previous getter (zero reported for one attached Player), then green against the
canonical query. Total instance population includes GMs but excludes world maps; admission
excludes GM occupants; detach changes both counts immediately. Unadopted canonical Player
records also count and prevent map destruction, so an absent generation registration is not
permission to destroy their storage.

C++ anchors: `Maps/Map.h:353`, `Maps/Map.cpp:2648-2655` and
`Maps/MapManager.cpp:367-372`. The map library's two count-only fixtures retire in favor of
the production lifetime/count tests. Generic map visitor/initializer tests use an actual
mutable encounter flag rather than a fabricated player count. All three Session instance-full
and GM tests now install real Players through the canonical handle/attach API; their decision,
packet bytes and admitted/rejected Player assertions are retained in the 201-line
`session/tests/instance_occupancy.rs` instead of the Session test monolith.

Physical Session tests shrink 96,845 -> 96,627 lines; the complete logical Session test surface
shrinks by 17 lines including the new module, and its ceiling tightens accordingly. This is a
bounded responsibility retirement, not a claim that the remaining monoliths satisfy C4.
Production login/save tests also attempt map destruction while DB completion is pending and
require rejection before map progress/late-mutation/old-incarnation checks.

aarch64 validation above `ca7034fa`, with explicit `PROTOC` and two build jobs:

- `cargo test --offline --locked -p wow-map --lib --test production_player_lifetime -- --quiet`:
  702 library passes, zero failures, one ignored; six production integration passes.
  The two retired library fixtures are replaced by real-owner integration assertions, not lost coverage.
- `cargo test --offline --locked -p wow-map --release --test production_player_lifetime`:
  six passes. `-p wow-world --lib -- --quiet`: 3,742 passes, zero failures, one ignored;
  the focused `instance_occupancy` filter runs all three migrated tests successfully.
- `-p wow-world --test production_login_player_owner`: six passes including map destruction
  rejection during pending save. Architecture check/self-test and syntax-only ownership pass.
- Formatting/diff checks and `./tools/validation-v2 quick --base ca7034fa` pass; green local
  manifest verified at `target/validation-v2/manifests/20260905T205419.252407Z-1197983-quick.json`.

The current occupancy query scans canonical records;
no new map-frame performance bound, live instance-capacity capture or shutdown parity is claimed.
Far-transfer completion, evacuation delivery, mutable Map escapes and C0/C3 phase work remain open.

## Failed map/lifetime transitions preserve Player — 2026-09-05

The next C1 cut above `b3d899c3` exercises the production `wow-map` library, not a fabricated
player-count-only fixture. Two tests ran red: `destroy_map` returned success for an occupied
map, and `update` on an unload candidate made the current Player handle stop resolving.
No live map loss is claimed; these are deterministic API/runtime-update reproductions.

`manager/map_lifetime.rs` now owns removal admission. Destruction refuses real typed Players
even when the compatibility count is zero, keeps the instance ID reserved on rejection, and
only removes the empty map after actual detach. `unload_all` now returns a typed
`MapUnloadBlockedLikeCpp` with ordered occupied map keys, rejecting the entire unload before
any map is changed. Once active Players are drained, it removes map storage without retiring
their still-valid detached lifetimes. No production caller of this bulk-unload API was found;
the automatic update path does call the shared destruction helper and is covered explicitly.

C++ anchors: `Maps/MapManager.cpp:322-339`, `Maps/Map.cpp:1629-1643` and
`worldserver/Main.cpp:390-391` (KickAll/UpdateSessions teardown precedes the map cleanup at
`:345`). C++ requests homebind evacuation and refuses deletion while Players remain. Rust's
old `remove_all_players` only set a counter to zero; that fake evacuation is deleted. The
automatic evacuation request/delivery is still open C1/C3 work: refusal preserves the owner
but is not full evacuation or shutdown parity. No I/O or packet delivery is added under a lock.

An additional failure test reproduced generation exhaustion retiring the previous Player
before returning `GenerationExhausted`. Replacement now reserves the generation before
retirement. The rejection preserves both active and detached old incarnations. This is an
artificial allocator-boundary test, not a claim that normal play exhausts u64 generations;
failure may consume a generation but never reuses one. The C++ single-Player lifetime and
the approved Rust generation contract remain the ownership constraints.

Five production-linked cases cover explicit/automatic occupied destruction, unadopted typed
Players, atomic bulk-unload rejection followed by detached persistence, and instance-ID reuse.
The old count-only test now expects refusal until its declared occupants have actually been
cleared. aarch64 validation above `b3d899c3` (with explicit `PROTOC` and two build jobs):

- `cargo test --offline --locked -p wow-map --lib --test production_player_lifetime -- --quiet`:
  704 library passes, zero failures, one ignored; five production integration passes.
- `cargo test --offline --locked -p wow-map --release --test production_player_lifetime`:
  five passes; `-p wow-world --test production_login_player_owner`: six passes again against
  the changed map library, including pending-save and stale-incarnation confirmation.
- Architecture check/self-test, syntax-only ownership check, formatting and diff checks pass;
  no checked-in ceiling or syntax waiver changed for this map cut.
- `./tools/validation-v2 quick --base b3d899c3`: pass, green manifest verified at
  `target/validation-v2/manifests/20260905T204257.069964Z-1183102-quick.json`.

New private removal implementation / production tests / allocator failure tests are 61/146/35
physical lines. This does not close the larger manager/Player or physical-ratchet C4 work.
No new clock, writer, storage backend, baseline waiver, restart, deployment or DB action.

## Coherent full-save capture and acknowledgement — 2026-09-05

Implementation starts above `abb072a9`, inside #578 C1. This is an explicit deferred-save
correctness repair plus boundary extraction, not a claim that a mechanical split preserves
the old group-wide acknowledgement. No production storage backend or clock changes here.

The old path captured the header and row families through separate canonical reads, then
cleaned whole **current** groups after asynchronous COMMIT. The regression
`full_save_ack_does_not_clean_a_spell_added_after_capture` was run red against that path:
a spell added during the pending save became `Unchanged`, although absent from its request.
The repaired path passes that test and keeps the later row dirty.

- `session/lifecycle/persistence/prepared.rs` acquires one generation-checked Player read
  for header, complete SQLx-free request and single-use receipt, then releases the owner.
  `projection.rs` only projects the admitted Player and the still-session-owned account data.
- The request alone crosses the persistence await. Confirmed Applied consumes the receipt
  against the same handle/incarnation; rollback, Unknown and cancellation do not acknowledge.
  Existing money admission/mutation fences and uncertain-COMMIT quarantine remain in place.
- Native `Player::acknowledge_saved_projection_like_cpp` compares captured row values,
  preserves later mutations and rebases incremental spell/equipment INSERT/DELETE states.
  A later edit of a confirmed NEW row must become Changed, not retry a duplicate INSERT.
  Skills retain the adapter's complete replacement contract. Reputation delivery flags do
  not decide whether saved standing/flags are clean. This is a precise value projection,
  **not numerical row revisions, a generic concurrency SDK or exactly-once persistence**.
- The old builder/group-wide ACK is a 608-line `cfg(test)` oracle, absent from production.
  Production orchestration/preparation/projection are 274/199/457 lines; native ACK is 201,
  with a separate 220-line test file. The production-linked login fixture/save challenge are
  461/206 lines. No new file crosses the 1,000-line cohesion-review signal.
- Reviewed logical ceilings: Session +15 production/+853 tests (including the retained
  old oracle); Player +201 production/+222 tests. This is bounded implementation/evidence
  growth, not a physical monolith waiver. Syntax delta is five old production helpers made
  test-only and two narrow preparation/header methods added; Session fields and other
  ownership definitions remain unchanged. Re-review at C1/C4; legacy fixtures retire when
  their old-projection equivalence obligations have production-linked replacements.

C++ anchors: `Entities/Player/Player.cpp:19323` (`SaveToDB`), `:20348` (`_SaveSkills`),
`:20399` (`_SaveSpells`), `:26409` (`_SaveEquipmentSets`), and
`Reputation/ReputationMgr.cpp:792` (`SaveToDB`), under the legacy reference tree.
C++ consumes the rows visited during preparation; Rust retains the established #169 rule
that dirty-state acknowledgement waits for confirmed COMMIT. SQL statement decomposition
and transaction order in `wow-database/src/player_lifecycle_adapter.rs` are unchanged.
The diagnostic event now says `player.save.commit_confirmed`, not that all current dirty
state is clean; it does not claim a stale incarnation accepted the receipt.

Initial aarch64 evidence: six native ACK tests, 39 lifecycle unit tests, and the six-test
`production_login_player_owner` integration target (three original login tests plus three
save challenges). The production-linked save cases execute the real public disconnect-save
path against a controlled port, with a full eight-slot output queue, two real map updates
during pending I/O, replacement incarnation, late row, rollback, Unknown and cancellation.
Expanded aarch64 validation on this worktree above `abb072a9`:

- `PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=2 cargo test --offline --locked -p wow-world --lib`:
  3,742 passed, zero failed, one ignored.
- The same Cargo environment with `-p wow-entities --lib player::save_ack`: six passed;
  with `-p wow-database --lib player_lifecycle_adapter::tests`: 25 passed, including the
  frozen statement order and operation-to-statement mappings.
- `-p wow-world --test production_login_player_owner`: six passed in debug and release.
- `session-ownership-check check --syntax-only`, architecture `check` and `self-test`,
  `cargo fmt --all -- --check`, and `git diff --check`: pass.
- `./tools/validation-v2 quick --base abb072a9`: pass; retained local manifest
  `target/validation-v2/manifests/20260905T202802.177176Z-1166643-quick.json`.
  This intentionally routes the current cut, not all eight earlier unpushed commits or the
  final PR gate. The unrelated untracked LFG audit was only inspected by hygiene checks,
  remains byte-identical and is not staged. No live DB/runtime/capture test was run.

Still open: actual scheduler/phase admission C0, private-hecs integration, full login/relogin,
real MariaDB durability and restart QA, and general ordering against other durable writers.
At that cut the header retained Session map/level staging and a zero detached instance;
the owner-header correction below supersedes those three reads. Detached-health
projection and far teleport postponement (`Player.cpp:19327`) need their own faithful transition
cut rather than being silently folded into this extraction. Equipment type/identity changes
across the existing two-table adapter are not generalized by this receipt. No new concurrent
writer is enabled. These boundaries prevent full C1/C2 or macro acceptance from this result.

### C1 save-header authority correction — 2026-09-06, above `0ad78a77`

`session/lifecycle/persistence/prepared.rs` now reads map, instance and level from
the same generation-checked Player as the rest of the header. C++ `Player.cpp:19480-19514`
uses `GetLevel`, `GetMapId` and `GetInstanceId`; `Unit.h:733` casts the native level
to uint8. `Object.cpp:1814-1824` preserves map/instance when `ResetMap` clears the
binding, matching Rust `WorldObject::reset_map`. Detached ownership is not instance zero.
This is an intentional correction of old-Rust parity debt, not a structural-only move.

The new active/detached regression forces Session map/level to 1/11 while Player
holds 571/73, then checks both the prepared DTO and the actual controlled save port.
Before the fix it failed with `(1, 11)` instead of `(571, 73)`; the extended case
also retains instance 43 after detach. Production-linked disconnect-save tests change
only the canonical level after login hydration and inspect the actual submitted request.
Old snapshot equivalence assertions are narrowed where their fixture intentionally
still uses legacy staging; explicit C++ expectations replace that obsolete oracle.

Local aarch64 validation above `0ad78a77` (working cut, not a deployed build):

- `cargo test --offline --locked -p wow-world --lib`: 3,750 passed, zero failed,
  one ignored, after correcting the additional legacy logout-level expectation.
- `cargo test --offline --locked -p wow-world --test production_login_player_owner`:
  six passed, including actual production save submission, pending I/O, replacement,
  rollback, Unknown and cancellation. These use a controlled port, not MariaDB.
- Both Cargo commands use `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`,
  `CARGO_BUILD_JOBS=2` and the existing local validation dev target directory.
- Syntax-only ownership, architecture check/self-test, formatting and diff checks pass.
  Reviewed logical Session size is 82,127 production + 106,531 test = 188,658 lines
  (-2/+34); no physical ceiling, field, exception or persistence baseline is expanded.
- Logs: `/tmp/rustycore-578-save-owner-{before,all-final,integration,ownership,architecture-checked,self-test}.log`.
- `PROTOC=/home/ubuntu/.local/protoc/bin/protoc ./tools/validation-v2 quick --base HEAD`:
  pass; manifest `target/validation-v2/manifests/20260906T145042.693001Z-3-quick.json`.
- The subsequent explicit detach-success assertion also passes the focused
  `full_save_reads_native_map_and_level_despite_stale_session_staging` test;
  log `/tmp/rustycore-578-save-owner-detach-final.log`. No production code changed
  after the whole-library/integration/quick runs.

No Session mirror writer is added or retired globally, no new concurrent save execution
is enabled, and teleport-destination precedence, health projection, transaction order,
money/cancellation fences and revision-safe ACK remain unchanged. Actual phase admission,
far-transfer save postponement, full lifecycle and real durability QA remain C0/C1 work.

Next transition evidence: `Player.cpp:1494-1503` executes delayed save after delayed
resurrection, and `MovementHandler.cpp:234/302` runs delayed operations after successful
far/near transfer. Rust currently calls only delayed resurrection from
`handlers/misc/travel.rs` and the near-ACK path; no equivalent delayed-save flag exists
in `PlayerTeleportStateLikeCpp`. `WorldSession.cpp:544-551` finishes far transfers before
logout, while Rust `session/lifecycle/logout.rs` goes directly to save/finalization.
Therefore a save-entry early return alone is not an acceptable fix: it could suppress
the disconnect save without completing the transfer. Implement defer/resume and logout
completion together, including failed attach and cancellation, before claiming this gate.

### C1 teleport admission prerequisite — 2026-09-06, above `40aa1131`

The production `teleport_to_with_options` entry now validates the full u32 map ID
against its MapStore and all X/Y/Z/orientation components before any movement reset,
pet/combat effect, packet, detachment or pending-transfer mutation. It replaces the
old `new_map as u16 > 0xFFF` check, which neither proved catalog membership nor
validated coordinates. Exact anchors: `Player.cpp:1237-1244`,
`MapManager.h:90-97`, `MapManager.cpp:339-342`, `GridDefines.h:231-248`.
This is a parity correction needed by transfer/save work, not full transfer acceptance.

The production-linked regression reproduced packets being emitted for infinite Z
before the correction. It now checks near/far invalid destinations, nonfinite and
out-of-range components, missing/truncated map IDs, preserved native position and
existing teleport state. A positive test admits both signed coordinate limits and
finite orientations outside a normalized angle range, as C++ does. The lower-level
Map cell check deliberately remains two-dimensional (`Map.cpp:426-434`): do not
mistake its separate contract for the complete teleport admission gate.

Nine existing library scenarios lacked their destination catalog. They now supply
explicit maps through a small test-only catalog builder; no production/cfg(test)
validation bypass and no weaker assertions are introduced. New integration cases
live in `tests/production_login_player_owner/teleport_admission.rs` rather than the
legacy monoliths. Reviewed necessary physical growth: Session root +6 gate lines,
Session tests +7 catalog-install lines, character tests +1 catalog-install line;
their existing C4 split exits remain. No field, lock, clock or durable writer is added.

Local aarch64 evidence on this working cut above `40aa1131`:

- `cargo test --offline --locked -p wow-world --lib`: 3,750 passed, zero failed,
  one ignored, after supplying the nine missing fixture catalogs.
- `cargo test --offline --locked -p wow-world --test production_login_player_owner`:
  eight passed, including the two new production-library admission cases.
- Cargo uses `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`, `CARGO_BUILD_JOBS=2`
  and the existing validation dev target directory. Syntax ownership passes with
  unchanged fields, associated items and exact registry metadata.
- Logs: `/tmp/rustycore-578-teleport-admission-{before,lib-final,production-final,ownership}.log`.
- Architecture check/self-test, `cargo fmt --all -- --check`, `git diff --check`
  and `validation-v2 quick --base HEAD` pass. Verified quick manifest:
  `target/validation-v2/manifests/20260906T150418.986434Z-3-quick.json`.
  Logical Session is 82,133 production + 106,538 tests = 188,671 lines;
  Character is 20,617 + 12,812 = 33,429. The three explained physical ceiling
  updates are migration debt, not terminal exceptions or C4 acceptance.

The remaining ACK path still consumes its destination/clears the far semaphore before
checking map creation/attach success, and its caller ignores the ensure result. C++
`MovementHandler.cpp:50-134` validates the ACK destination, rechecks entry, restores
the old binding after failed AddPlayerToMap and requests homebind recovery; its delayed
operations run only after successful completion. This must be addressed with the
defer/resume/logout contract above. No runtime/capture/DB QA or C0-C4 closure is claimed.

### C1 worldport owner preservation — 2026-09-06, above `218a8d2a`

The still-active worldport regression now adopts the canonical handle before ACK.
On the old code it reproduced source coordinates `(1,2,3,0)` after a requested move
to `(11,22,33,1.5)`: Session's cross-map position setter correctly refuses to relocate
an active Player on another map, but the ACK then attached it using that old position.
`handlers/misc/travel.rs` now detaches the same owner before destination relocation,
as required by `MovementHandler.cpp:84-104`. A missing destination or failed owner
detachment no longer consumes the pending transfer or publishes LoggedIn success.

The existing Session detach operation is now crate-visible for this caller. Its raw
multi-map fallback previously discarded `RemoveFromMap`'s returned boxed Player;
it now adopts only a unique unowned legacy record and retains the checked handle.
A stale or GUID-mismatched handle never triggers fresh adoption. Already-detached
ownership is idempotent. C++ removes with `delete=false` (`Player.cpp:1453-1455`;
`Map.cpp:907-934`), not by reconstructing Player from Session after removal.

Three new tests in the 106-line `session/tests/player_detach.rs` check exact object
address/data retention, ambiguous-record rejection without partial deletion, and
stale-handle rejection without touching the replacement. Two old fixtures now adopt
before setting homebind/cast state. The preserved-spell test explicitly requires the
detached owner to exist; its old optional mutation closure could silently skip assertion.
The production-library test starts a far teleport and verifies that its retained Player
answers SuspendTokenResponse with NewWorld; it is not full worldport/live acceptance.

Local aarch64 working-cut evidence above `218a8d2a`:

- `cargo test --offline --locked -p wow-world --lib`: 3,753 passed, zero failed,
  one ignored. `--test production_login_player_owner`: nine passed.
- Both use the existing validation dev target, `CARGO_BUILD_JOBS=2` and
  `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`. Syntax-only ownership and the
  five standalone persistence-policy tests pass. Only the existing detach method's
  crate visibility/visible guard changes in the exact policy; no persistence rows change.
- Logs: `/tmp/rustycore-578-worldport-owner-{before,lib-final,production,ownership,policy}.log`.
- Root production shrinks 76,753 -> 76,751; legacy root tests grow five explained
  wiring/fixture/assertion lines. Logical Session is 82,131 + 106,649 = 188,780.
- Architecture check/self-test, formatting and diff checks pass. Quick validation
  (`PROTOC=... ./tools/validation-v2 quick --base HEAD`) and manifest verification
  pass: `target/validation-v2/manifests/20260906T152207.017814Z-3-quick.json`.

Still open: map creation/admission/attach are fused in the existing ensure helper;
the ACK still ignores its failure and lacks the full C++ create/bind/send/add/recovery
sequence. Homebind recovery, failed-attach publication, delayed-save resumption and
logout transfer completion must be completed together. No new clock, lock, asynchronous
save writer or map-guarded packet delivery is introduced. C0-C4 and live/capture/DB
acceptance remain open; the unrelated untracked LFG audit remains untouched.

### C1 map preparation/attachment seam — 2026-09-06, above `ef480679`

The 250-line map ensure operation moves from Session's root to the 276-line private
`session/lifecycle/map_entry.rs`. Its compatibility facade still reads the current
destination/position, performs preparation and attaches synchronously in the same order.
The extracted preparation body is byte-identical to the old body excluding its final
attachment block. C++ `MapManager.cpp:139-231` returns the selected map separately from
the caller's binding/add operation (`MovementHandler.cpp:84-134`). This extraction
does not claim that the existing Rust ACK already implements that complete sequence.

`prepare_canonical_map_entry_like_cpp` is Session-visible, not a public capability or
an asynchronous permit. It preserves admission packets, map creation, canonical owner
materialization, instance history and recent-instance/lock side effects. It is neither
a pure query nor a reservation; failed later attachment does not roll those effects back.
No field, lock, clock, packet metadata, durable writer or runtime path is added.

Two tests in the separate 69-line `session/tests/map_entry.rs` verify that preparing
new/existing destinations preserves an active or detached Player's handle, residence and
position, leaves the destination empty and releases its guard; an absent catalog entry
leaves the current owner untouched. Root production shrinks 76,751 -> 76,500 lines;
root tests grow only two registration lines to 96,641. Logical Session is
82,157 production + 106,720 tests = 188,877. These are reviewed transitional counts,
not ownership retirement or renewed terminal exceptions.

Local aarch64 working-cut evidence above `ef480679`:

- `cargo test --offline --locked -p wow-world --lib`: 3,755 passed, zero failed,
  one ignored; the focused `create_map` filter also passes all 22 tests.
- `cargo test --offline --locked -p wow-world --test production_login_player_owner`:
  nine passed. Existing validation dev target, `CARGO_BUILD_JOBS=2` and local PROTOC.
- Syntax-only ownership passes: one reviewed Session-visible method added, no changed
  fields or exact opcode registration rows. Five persistence-policy tests pass;
  no exhaustive inventory regeneration or snapshot changes are required by this move.
- Logs: `/tmp/rustycore-578-map-entry-{lib,focused,production,ownership,policy}.log`.
- `validation-v2 quick --base HEAD` passes; manifest:
  `target/validation-v2/manifests/20260906T153559.581434Z-3-quick.json`.
- Manifest verification, architecture check/self-test, formatting and diff checks
  pass. The physical inventory has 982 files and the same 101 legacy ceilings;
  no terminal exception is added. The self-test runs 20 fixtures.

Remaining: wire the separate phases into the full ACK create/bind/send/add/recovery
contract, then complete delayed-save resumption and logout transfer completion without
weakening persistence fences. C0-C4 and real client/DB/runtime acceptance remain open.

### C1 recovery contract review — 2026-09-06, `8c9a8042`

Source tracing identifies an unresolved legacy corner before wiring homebind recovery:

1. `Player.cpp:1453-1455` removes Player from the old map with `delete=false`.
   `Map.cpp:907-934` removes world/grid membership but retains the old map binding.
2. On destination creation/admission failure, `MovementHandler.cpp:90-100` invokes
   `TeleportTo(homebind)` before destination relocation or rebinding. Failed add
   similarly restores the old binding before homebind (`:124-134`).
3. If homebind has the old map ID, `Player.cpp:1303-1346` selects a near teleport
   without testing `IsInWorld()`. The near ACK (`MovementHandler.cpp:263-303`)
   calls `UpdatePosition`, not `AddPlayerToMap` or `AddToWorld`.
4. `Player.cpp:6122-6138` delegates to `Unit.cpp:12257-12300`, which calls
   `Map::PlayerRelocation` (`Map.cpp:1015-1044`). That path may change grid
   membership, but does not restore world membership. For a same-cell destination,
   even grid insertion is skipped. This is static source evidence of a potentially
   stranded Player, not a reproduced legacy server/client failure.

The current Rust `teleport_to_with_options` also chooses near solely by map ID.
Therefore naively adding `teleport_to(homebind)` to the failed ACK is not an adequate
recovery contract. Neither suppressing save during far transfer alone nor looping ACKs
at logout resolves this case; C++ logout only loops while the far semaphore remains set
(`WorldSession.cpp:544-551`), and the near path clears that semaphore.

**Explicit legacy-departure contract approved by the user on 2026-09-06:** recovery must
retain the same canonical Player incarnation and finish with validated active map/world
membership before reporting successful entry, regardless of whether homebind shares the
source map ID. Evaluate a full far-recovery handshake for a detached Player even on the
same map; do not enable it without positive/negative production-linked tests and the
required action-specific client capture. Keep ordinary active same-map teleports unchanged.
Preserve save fences and do not fabricate successful entry or durable completion on failure.
Invalid/unavailable homebind and interrupted recovery still require explicit terminal
outcomes in that implementation; no guessed fallback location or unbounded retry is approved.

Alternatives: reproducing the legacy near branch preserves source selection but fails the
membership invariant above; reattaching before an ordinary near handshake changes admission
and publication order and also needs an explicit contract. A real legacy capture may clarify
observable behavior, but does not itself authorize silently repairing a legacy defect.
The user's approval resolves the behavior-design pause. No runtime, publication, deployment or
database authority is added; this is not C1 acceptance or a new macro/issue.

Local implementation above `8c9a8042` corrects immediate and delayed near/far selection:
near requires active same-map residence, not merely a retained map ID. Detached entry
clears obsolete near state and suppresses seamless transfer. Active near/seamless paths
retain their existing packet expectations. The production-library reproduction first
hung in `combat_stop_like_cpp`: missing-map/player branches called finalization while
still holding the manager guard, and finalization reentered canonical state setters.
Both branches now drop that guard before finalization. C++ `Unit.cpp:5802-5818`
performs combat cancellation without this Rust-only mutex/reentrancy problem.

After the guard repair, the new integration test reproduced CancelCombat + MoveTeleport
on the old selector. The local correction instead produces CancelCombat + TransferPending
+ SuspendToken, followed by NewWorld on suspend ACK. It also rejects nonfinite recovery
coordinates without publishing a replacement transfer. The private map-entry test covers
immediate/delayed detached return, obsolete near state, suppressed seamless, exact handle
and object address retention, and reattachment through the existing canonical operation.
It deliberately does not substitute direct attachment for full ACK/client acceptance.

Seven legacy near/seamless unit fixtures now install and attach their canonical Player;
the hearth and resurrection fixtures also establish active membership before setting
state. Their old Session-only setup did not establish the premise of a near teleport.
The delayed-near combat assertion now reads native state rather than stale Session staging.
No production cfg(test) bypass, new field, lock, clock or persistence path is introduced.

Still required: failed-ACK dispatch into bounded homebind recovery, no-success publication
on failed add, terminal handling of invalid/unavailable homebind, interrupted ACK recovery,
delayed-save resumption and logout completion. No target build was installed/restarted;
the action-specific real-client capture gate remains open. This local prerequisite is
not a completed recovery operation or C0-C4 acceptance.

Local aarch64 validation on the working cut above `8c9a8042`:

- `cargo test --offline --locked -p wow-world --lib`: 3,756 passed, zero failed,
  one ignored. `--test production_login_player_owner`: ten passed. Both use
  the existing validation dev target, `CARGO_BUILD_JOBS=2` and local PROTOC.
- Syntax-only ownership passes with unchanged fields, associated items and exact
  registration rows. Five standalone persistence-policy tests pass; no persistence
  inventory rows or snapshot were regenerated.
- Architecture check/self-test, formatting and diff checks pass. Physical inventory:
  982 files, 101 legacy ceilings, no new terminal exception. The explained growth is
  Session root +21 lines, seven root fixture setups +42, private map-entry tests +66,
  shared hearth fixture +1 and resurrection fixture +1. Production-linked recovery
  coverage grows its existing small integration module by 68 lines.
- Logical Session: 82,178 production + 106,828 tests = 189,006; Character:
  20,617 + 12,813 = 33,430. These are transitional ceilings with retained C4 exits.
- `validation-v2 quick --base HEAD` passes; manifest:
  `target/validation-v2/manifests/20260906T155842.050525Z-3-quick.json`.
- Logs: `/tmp/rustycore-578-homebind-{before,unlocked,lib-verified,integration-final,ownership,policy,architecture-final,self-test,quick-final}.log`.
  `before` is the interrupted deadlock reproduction; `unlocked` records the expected
  failing near/far assertion, not passing acceptance. No real-client or DB result is claimed.

### C1 rejected-entry publication fence — 2026-09-06, above `a7ddabe8`

The production-linked missing-destination regression reproduced the old ACK continuing
into initialization (`unexpected auxiliary request`) after map admission failed.
`handlers/misc/travel.rs` now consumes the pending destination/far semaphore only after
`session/lifecycle/map_entry.rs::try_attach_worldport_destination_like_cpp` succeeds.
That synchronous operation validates coordinates and rejects map IDs that cannot fit the
existing Session map representation, selects/admit-prepares the map, detaches if needed,
attaches the same owner and only then updates Session destination bookkeeping.

Missing/rejected admission and failed attachment no longer write destination coordinates
onto the detached source Player or continue ResumeToken/initialization/LoggedIn publication.
Map creation and existing admission side effects are still impure and not rolled back.
The existing canonical runtime validates attachment before changing binding/coordinates
(`wow-map/src/map/runtime.rs:162-231`); its error path returns the same Box to the manager.
The new private regression checks missing catalog and duplicate destination GUID, exact
handle/object address, retained source map/position and released guards. Existing positive
immediate/delayed return tests now exercise this entry operation directly.

C++ `MovementHandler.cpp:90-134` does not proceed with successful entry after rejected
admission/add. This local fence does not yet implement its full bind/before-add/add order:
the existing successful Rust packet sequence is unchanged. Failure retains Transfer state
and the pending destination as an intermediate recoverable state, not a terminal policy or
an approved unbounded retry loop. Automatic bounded homebind recovery, terminal fallback,
interrupted publication, deferred save and logout transfer completion remain C1 work.

Local aarch64 working-cut evidence above `a7ddabe8`:

- `cargo test --offline --locked -p wow-world --lib`: 3,757 passed, zero failed,
  one ignored. `--test production_login_player_owner`: eleven passed, including the
  formerly failing ACK regression. Existing validation dev target, two build jobs, local PROTOC.
- Syntax ownership and five persistence-policy tests pass; one reviewed crate-visible
  operation is added, without fields, opcode registration changes or persistence rows.
- Architecture check/self-test, formatting and diff checks pass. No physical ceiling or
  exception changes. Private production map-entry module: 307 lines; tests: 172.
  Logical Session: 82,209 production + 106,865 tests = 189,074 (+31/+37).
- Quick validation passes, manifest
  `target/validation-v2/manifests/20260906T161035.034988Z-3-quick.json`.
- Logs: `/tmp/rustycore-578-entry-reject-{before,lib,production,ownership,policy,architecture,self-test,quick}.log`.

No runtime was installed/restarted and no capture, DB durability, complete recovery or
C0-C4 acceptance is claimed. The unrelated LFG audit remains untouched.

### Bounded terminal recovery — 2026-09-06, working tree above `cf5b0757`

The user's continuation approved the proposed terminal policy below, an intentional
legacy departure. C++ `WorldSession.cpp:544-551` loops while a
far transfer remains pending; `Player.cpp:19327-19333` defers saves in that state.
Combining those literally with repeated failed far-homebind recovery risks a retry loop
or a permanently deferred disconnect save. Current Rust instead goes straight to save
in `session/lifecycle/logout.rs`, without completing the transfer. Neither is acceptance.

Approved bounded contract: attempt the requested destination, then at most
one homebind recovery. If neither can admit the same Player, close gameplay admission
and disconnect without publishing entry success. If the retained incarnation and its
source map/instance/coordinates still form a coherent snapshot, persist current Player
progress at that retained source location using the existing full-save transaction and
all existing fences. This is a saved return location for relogin, not a claim of active
membership or an immediate successful return to the old map. Revalidate entry at login.

Never substitute a guessed location, save the rejected destination, infer a successful
COMMIT, or clear dirty revisions on Failed/Unknown. A missing/stale owner or an
indeterminate durable base keeps the existing no-overwrite/quarantine rule. Explicit
terminal recovery state must distinguish this save from an ordinary in-flight far save;
merely clearing the far flag to bypass deferral is not the implementation contract.

Implemented in `session/lifecycle/transfer_recovery.rs`, the travel handlers and the
Player-owned teleport state: one homebind attempt, ACK admission only after recovery
NewWorld is queued, then terminal disconnect on rejection. Repeated ACKs cannot restart
terminal recovery. Queue acceptance is not delivery proof or perfect empty-ACK correlation.
Successful attachment clears the recovery marker; terminal failure retains far-pending.
`persistence/prepared.rs` explicitly ignores the rejected pending destination only in
terminal state and validates the same retained owner's map, instance and finite position.
This fixes the prior save header's pending-destination precedence for this terminal case;
no Session mirror, new lock or transaction path is added.

Local aarch64 evidence: `cargo test --offline --locked -p wow-world --lib` passes
3,759 tests (one ignored); the two new map-entry tests cover bounded/missing/invalid
homebind, terminal replay rejection and incoherent-source save refusal. Existing travel
fixtures now adopt their installed canonical Player before setting transfer state.
`cargo test --offline --locked -p wow-entities --lib player::location` passes one test.
Production-linked `cargo test --offline --locked -p wow-world --test
production_login_player_owner` passes 11 tests and exercises rejected destination,
homebind NewWorld, second rejection and the existing disconnect full-save entrypoint
against a controlled port, distinguishing native origin from both rejected destinations.
This is not real DB durability, restart/relogin or client capture evidence.

Local structural validation also passes: Session syntax/exact-item policy, five
preserved persistence-policy tests, architecture check and self-test (20 fixtures),
format and diff checks. `PROTOC=/home/ubuntu/.local/protoc/bin/protoc
./tools/validation-v2 quick --base HEAD` passes above `cf5b0757`; verified manifest:
`target/validation-v2/manifests/20260906T172106.098493Z-3-quick.json`.
Reviewed logical Session growth is +109 production/+90 test lines, mostly private
modules; root files gain only six terminal guard lines and four fixture-literal lines
combined. All 101 existing physical migration ceilings still require C4 retirement.

Still open: ordinary in-flight far-save deferral and its resumption, nonterminal transfer
completion during logout, before-add publication order and cancellation after attachment.
Calling the network ACK handler during logout is not a substitute: it can publish packets
or await auxiliary reads. This slice does not close C1 or C0–C4 and does not authorize
runtime installation, DB mutations, capture sessions, push or merge.

### C1 worldport delayed-operation order — above `0e2fdb85`, 2026-09-06

The next deferred-save/logout review found an already integrated ordering defect:
`handlers/misc/travel.rs` resummoned pets and applied delayed resurrection immediately
after attachment, before destination self CREATE and after-add initialization. C++
`Handlers/MovementHandler.cpp:156-234` completes initialization/zone handling first,
then resummons the pet and calls `Player::ProcessDelayedOperations`; the latter applies
resurrection before deferred save (`Entities/Player/Player.cpp:1494-1503`). These anchors
are under `/home/server/woltk-trinity-legacy/src/server/game/`.

The existing canonical worldport regression now starts with distinct health values and
a pending resurrection. Against `0e2fdb85` plus the test, it fails because self CREATE
already reflects the resurrected health (`transfer-tail-before` log, one failed test).
The handler now runs those two existing operations after destination initialization and
zone resolution. The test checks pre-resurrection health bytes in self CREATE, their
replacement in the final canonical Player, consumed delayed work, unchanged exact opcode
sequence and source/destination ownership. Full library tests pass: 3,759, one ignored.
Commands (local aarch64, pinned PROTOC and the validation Cargo cache):
`cargo test --offline --locked -p wow-world --lib` and
`cargo test --offline --locked -p wow-world --test production_login_player_owner`
(11 integration tests pass). Session syntax/exact-item check, architecture check and
self-test (20 fixtures), formatting and diff checks pass without policy regeneration.
`validation-v2 quick --base HEAD` above `0e2fdb85` passes; verified manifest:
`target/validation-v2/manifests/20260906T172906.289367Z-3-quick.json`.
This is a controlled packet-construction regression, not a fresh client capture or a claim
that the partial health-only resurrection implementation equals full C++ resurrection.

Also corrected unsupported comments in travel and character initialization: Rust's full
before-add helper is currently login-only; C++ replays it on non-seamless worldport.
Client retention is not evidence to omit it. No new field, public API, lock, persistence
transaction, registry row, logical-owner ceiling or physical exception is introduced.
The changed travel production/test files are 827/537 lines; remaining legacy ceilings
and C0–C4 acceptance are unchanged.

Next integration boundary remains the complete transfer operation, not an early-return
save patch. Preserve pending work until its acknowledged phase and carry no map decision
across awaits as a reservation: current `prepare_canonical_map_entry_like_cpp` explicitly
does not reserve admission. Separate before-add preparation/publication from canonical
attachment, make disconnect completion independent of live packet delivery, then wire
ordinary far-save deferral and its resurrection-before-save resumption together. Merely
calling the current ACK during logout can block on delivery or perform auxiliary reads;
merely deferring save would skip existing disconnect persistence without a completion path.
Before-add parity, cancellation/recovery across initialization, full logout completion,
real durability and action-specific captures remain open.

### C1 worldport appearance authority — above `b769b860`, 2026-09-06

`send_player_self_create_for_teleport_like_cpp` incorrectly repeated the login
customization query instead of serializing the current Player. With a canonical choice
present and no auxiliary persistence result, the old handler omitted that choice from
self CREATE. The existing worldport test reproduces this on `b769b860` plus the new
assertion (`customization-before` log: one failure at the expected appearance assertion).

C++ loads customization rows once into Player at `Entities/Player/Player.cpp:17304-17318`.
`Entities/Object/Updates/UpdateFields.cpp:1620-1624,1777,1822-1825` serializes those owned
choices in CREATE. Paths are under the legacy `src/server/game` tree. Rust now projects
the existing native choices through `session/appearance.rs`, using the generation-checked
owner and returning `None` rather than guessed empty state for an unavailable incarnation.
The ten-line reader owns no state and releases its map guard before packet construction.
Login hydration and its persistence helper remain unchanged; no extra cache is added.

Coverage includes appearance bytes in the actual worldport self CREATE and a narrow
missing/authoritative-empty/active/detached/retired-owner test. The existing XP self-CREATE
fixture now adopts a real canonical Player before setup; production has no ownerless
appearance fallback. Logical Session growth: +11 production (reader and module declaration)
and +35 test lines in the small map-entry suite; root growth is only one declaration.
One exact read-only associated item is added to the reviewed policy. No existing field,
lock, registry row, transaction or physical exception is added.

Local aarch64 validation above `b769b860`, using pinned PROTOC and the existing Cargo
cache: `cargo test --offline --locked -p wow-world --lib` passes 3,760 tests (one ignored);
`cargo test --offline --locked -p wow-world --test production_login_player_owner` passes
11 controlled integration regressions. Session syntax/exact-item policy, five preserved
persistence-policy tests, architecture check/self-test, format and diff checks pass.
`validation-v2 quick --base HEAD` passes; verified manifest:
`target/validation-v2/manifests/20260906T173917.775025Z-3-quick.json`.
This is not real DB/client/relogin or action-specific capture evidence.

The parallel trait review found a different boundary: `PlayerSpellRuntimeState` retains
only `(type, specialization, combat flags)` by config ID and entry completeness flags.
It does not retain config names, local identifiers, skill/system IDs or complete entries
required by `TraitConfigCreateData`. The current worldport loader still queries them and
restarts trait authority hydration. Removing those queries by substituting empty data or
duplicating a packet cache would not complete ownership. Expand the canonical trait model
and migrate its actual readers/writers before retiring that loader. Whole-packet coherent
preparation, before-add ordering, transfer cancellation, logout completion and ordinary
far-save deferral/resumption remain open; this appearance repair does not close C1/C2.

### C1 trait retention and worldport projection — above `2fdbd962`, 2026-09-06

The preceding appearance review's missing full trait representation is now addressed
for loaded configuration retention/CREATE projection. The existing native config map
has richer values: the same raw header plus explicitly optional names, local/skill/system
IDs, complete entries/ranks and dynamic-field insertion order. There is no second map or
packet cache. Header-only states remain insufficient for CREATE. All existing transient
spell snapshots, fixture conversions, readers and reset/clear writers retain the same
value type, so unrelated spell operations cannot silently discard its full details.

`session/trait_configs.rs` retains the existing login loader's full parsed configurations
after its complete-header proof and projects them through the checked Player handle.
Duplicate IDs, mismatched headers or incomplete ownership are rejected before mutation;
empty and unavailable are distinct. CREATE preserves insertion order, not sorted map-ID
order. `send_player_self_create_for_teleport_like_cpp` no longer invokes either trait or
customization login queries, nor restarts trait authority hydration. Its success result
also prevents the outer ACK from continuing to LoggedIn when self CREATE cannot be built
or queued. This does not undo an already attached Player or prove wire delivery.

Re-read C++ anchors under legacy `src/server/game`: `Player.cpp:26635-26779` loads,
validates, supplies defaults and applies trait configs; `MovementHandler.cpp:120-162`
initializes the attached Player; `UpdateFields.cpp:2560-2586,3135-3137` serializes
owned configurations and their dynamic-field order.
**Still not implemented by this cut:** full `TraitMgr::IsValidEntry`, `ValidateConfig`,
granted-entry/default-config creation and active-config application parity. Preserving
all currently parsed fields is not proof those gameplay operations have been ported.
No guessed defaults or empty substitute replaces an incomplete configuration.

Evidence: the canonical login auxiliary test retains the complete parsed payload, and
the new owner test round-trips deliberately unsorted IDs, names, all entry fields and
header/detail metadata, rejects duplicate/mismatched input atomically, preserves detached
reads and invalidates on reset. Worldport's actual self CREATE contains a runtime trait
name and preserves its owner state. Header-incomplete self CREATE returns failure without
packets; a production-linked partial-login case disconnects instead of claiming entry
success, and its controlled port rejects any unexpected auxiliary read.

Local aarch64 tests with pinned PROTOC and the existing validation Cargo cache pass:
`cargo test --offline --locked -p wow-world --lib` (3,761; one ignored),
`cargo test --offline --locked -p wow-entities --lib` (721), and
`cargo test --offline --locked -p wow-world --test production_login_player_owner` (12).
These are local controlled tests, not real DB/client/relogin/capture acceptance.

Session syntax/exact-item policy passes with 3,686 items and unchanged fields/registry;
five preserved persistence-policy tests and architecture check/self-test (20 fixtures)
pass without inventory regeneration. Format and diff checks pass. Bounded
`validation-v2 quick --base HEAD` above `2fdbd962` passes; verified manifest:
`target/validation-v2/manifests/20260906T175615.351735Z-3-quick.json`.

Reviewed growth: Session +120 production/+76 test lines, Player +41/+3, Character +1/+0.
Native types are 35 lines; the private hydration/projection adapter is 115. Existing roots
grow only by wiring/type/reader adjustments (+5 Session, +2 Player, +1 character init)
and three formatting lines in the old Player test root. No new physical exception, field,
lock, clock, transaction or registry row. The exact policy changes are the fixture field
type and its bridge fingerprint, two read/hydration methods and self-CREATE's bool result.
Full before-add ordering, packet backpressure/cancellation, logout completion, ordinary
far-save deferral/resumption, C0/C3 coordination and all 101 legacy ceilings remain open.

### C1 disconnect completes pending far admission without client ACK — above `fbad212b`, 2026-09-06

The public production disconnect-save path previously returned after saving proposed
destination coordinates while the Player remained detached from that map. The new
`production_disconnect_completes_pending_far_transfer_before_save` fails on `fbad212b`
at its native destination-owner assertion: 0 passed / 1 failed,
`/tmp/rustycore-disconnect-pre-ack-before.log`. This controlled persistence-port evidence
is not a real database write or restart/relogin observation.

C++ `WorldSession.cpp:544-551` calls HandleMoveWorldportAck until pending far transfer is
finished before setting logout state. `MovementHandler.cpp:49-134` clears the semaphore,
checks/adopts the destination and falls back to homebind; its successful tail then runs
post-add/zone/pet/delayed work (`:153-234`). `Player.cpp:19324-19333` does not save a pending
far transfer. Existing Rust already retained the canonical Player and pending destination,
but disconnect completion only examined post-add progress and returned early before ACK.

The same completion entry now also processes pending far admission. It reuses the existing
map decision/admission/attachment functions with publication disabled; every rejection,
instance decision and native side effect remains shared with the unchanged publishing
network path. It never sends ResumeToken, self CREATE, TransferAborted or a new handshake
during disconnect. After successful admission it clears far/destination together, resets
the represented movement counter, updates the derived registry position and starts the
existing native post-add completion. It does not claim LoggedIn or client initialization.

Rejection gets at most one distinct homebind attempt. Failure retains the already-approved
Terminal/source-save policy, including the unresolved far flag and source location; it
does not save the rejected target or loop forever. Unavailable owner/destination or an
inconsistent active operation refuses completion under the existing factory gate. Normal
save projection now also rejects pending nonterminal far transfers. This is **not** the
missing delayed-autosave scheduling/receipt implementation; that remains C1 work.

Three production-linked scenarios cover requested destination, homebind fallback and both
destinations rejected, all with saturated output. Successful saves carry the post-resurrection
health and actual admitted map; terminal rejection saves source coordinates/health. The
co-located preparation test pins rejection before far completion and the terminal exception.
The Session logical delta is +107 production/+20 tests (82,706 / 107,164 / 189,870 total).
Physical files remain bounded: map entry 353 lines, transfer completion 225, production save
tests 451. No new field, clock, query or physical ceiling is added.

Syntax policy reviews three new internal methods and the recovery terminator's narrow
Session visibility. It also reconciles the co-located test constructor added late in
`fbad212b`: that cut's syntax check preceded the final fixture addition, and quick does not
run the ownership checker. The current exact syntax recheck includes that fixture; this is
not a new production constructor or persistence access. Full before/after-add native and
protocol parity, cross-clock admission/mutation barriers, delayed saves and live QA still
remain C0–C4 obligations; these bounded scenarios do not close the macro.

Local aarch64 validation above `fbad212b`: `cargo test --offline --locked -p wow-world
--lib` passes 3,770 tests (one ignored), and `--test production_login_player_owner` passes
16. Exact syntax ownership passes with 3,697 associated items, unchanged 282 production /
433 fixture Session fields and 590 registry rows. Architecture check/self-test and the
five persistence-policy tests pass; no inventory row is closed. Format/diff checks pass,
and quick manifest `target/validation-v2/manifests/20260906T201503.360086Z-3-quick.json`
verifies green. No release/live DB/capture/restart acceptance or publication is claimed.

### C1 retained native post-add completion — working tree above `e8a3fb1f`, 2026-09-06

The `cancelled_worldport_finishes_native_effects_before_disconnect_save` regression
first failed against `e8a3fb1f`. It polls the
real worldport handler until its typed initial-world-state read remains pending, drops
that future with a bounded timeout, closes output and invokes the real disconnect-save
entry point. An observation inside the read proves cancellation reached that phase;
no map guard crosses the pending await. Before save, both the far semaphore and pending
destination are already clear. After disconnect-save returns, canonical delayed
resurrection is still present. Command: `cargo test --offline --locked -p wow-world --lib
cancelled_worldport_finishes_native_effects_before_disconnect_save`, local aarch64,
0 passed / 1 failed; `/tmp/rustycore-worldport-cancel-save-before.log`. This proves missing
native completion before finalization returns, **not** that a real database committed an
incomplete snapshot. No runtime/database/capture operation was performed.

Current production cancellation is in `session_factory.rs::run_world_session_until_disconnect_like_cpp`:
the cancellation arm drops the entire update future. `process_pending_with_catalogs_like_cpp`
contains asynchronous phase work and drains multiple selected handlers, followed by rename
callbacks and periodic save. Keeping that entire future alive without an admission barrier
would permit additional work after shutdown. Conversely, merely refusing SaveToDB on a
new pending bit is insufficient: the factory currently proceeds from unit-returning save
to cleanup. Both alternatives must preserve owner retention, submitted-commit drains and
the bounded fatal-shutdown contract, not silently extend force cancellation indefinitely.

The implementation retains `post_add` in the canonical Player teleport state, independently
of the far semaphore. Its immutable map/position and three monotonic native phases cover
before zone, zone applied and scaling applied. Worldport starts this record after successful
self CREATE construction (the async wrapper itself does not suspend), before the nearby
creature/gameobject and shared visibility awaits. Shared after-add advances it around zone
and scaling. A private 139-line lifecycle module (98 production/41 test lines) completes only remaining represented native
effects and the pet/resurrection tail; it never replays the ACK or reads world states for
publication. Both normal completion and disconnect use it. Completion removes the record;
repeated completion cannot repeat the pet/resurrection tail.

Scaling now has an availability-aware, publication-selectable internal path. Existing callers
keep their boolean wrapper; recovery performs the same native scaling without sending its
stat packet into a possibly full channel. Wrong target, missing incarnation and unavailable
scaling fail closed with the record retained. The existing active/detached/replacement test
includes the progress field and rejects stale completion without changing the replacement.

Before ordinary save/cleanup, the factory completes retained native work after the existing
rename-commit drain. Failure requests the existing fatal exit and returns without ordinary
save/cleanup; this is not a successful logout or a durable recovery claim. The disconnect-save
entry also gates itself, and prepared save refuses an owner with outstanding post-add work.
No Session state mirror, scheduler, lock, asynchronous gate or database request is introduced.

C++ `WorldSession.cpp:544-551` finishes pending transfers
before setting logout state; `Player.cpp:1494-1503` orders delayed resurrection before save,
and `Player.cpp:19324-19333` defers saves while far transfer is pending. The Rust async
representation must preserve that operation ordering even after its early far-flag clear.
The cancellation regression now passes in the initial full library run (3,768 passed,
one ignored), with a retained-before-zone case, wrong-target rejection and no repeated pet
tail. A new production-linked save scenario fills the output queue, retains a real native
equipped item and checks that the controlled save receives post-resurrection health after
nonpublishing scaling. A co-located test also proves prepared save is available before
the operation, rejected while native work remains, and available after completion.

Reviewed logical deltas: Session +116 production/+48 tests, Character +11/+61, server
composition +10/+0, and logical Player +0/+2; the sibling native gameplay-state file grows
17 lines. Necessary physical changes are +10 Session-root lines, +7 existing exact-state
test lines and +11 shared-entry lines. The completion module is 139 lines; focused post-add
tests are 194 lines and the production save test module is 316. The three legacy ceilings
retain their C4 split exits; none is a terminal exception. Syntax policy reviews only four
new methods and the factory's changed body/bridge fingerprint; no registry row or field is
added, and the bridge evidence itself is unchanged.

Remaining boundary: this is the **represented post-add native tail**, not all C1. Transfers
cancelled before ACK/admission, rejection/self-CREATE-unavailable recovery, unimplemented C++
before/after-add gameplay and payloads, delayed autosave scheduling, submitted-save receipts
and full visibility/Map effects still need their operation contracts. Independent map/session
clocks still require C0 admission and mutation barriers; these sequential checked accesses
do not prove a cross-clock atomic operation. Terrain reads remain synchronous outside locks;
bounded asynchronous shutdown/transport and live durability/capture QA remain C0/C3 work.
No whole-transfer, macro, live database or restart acceptance follows from the local tests.

Local aarch64 working-tree validation above `e8a3fb1f`: `cargo test --offline --locked
-p wow-world --lib` passes 3,769 tests (one ignored); `-p wow-entities --lib` passes 721;
`-p wow-world --test production_login_player_owner` passes 13, including the saturated-output
save scenario. `cargo check --offline --locked -p world-server` passes. Syntax-only ownership
passes with unchanged 282 production/433 fixture fields, 3,694 associated items and 590
registry rows; architecture check/self-test, five persistence-policy checks and format/diff
checks pass. The quick run compiles all three changed packages' test targets and its manifest
`target/validation-v2/manifests/20260906T195108.956424Z-3-quick.json` verifies green.
These are local dev/test results, not release-mode or live acceptance. No publication occurs.

### C1/C3 early self CREATE delivery does not abort native effects — above `71383a93`, 2026-09-06

The actual worldport handler still returned before post-add processing when its self
CREATE sender reported false. That conflated unavailable canonical projection with closed
output. A controlled receiver closure **before ACK** reproduces an unconsumed native delayed
resurrection on `71383a93` (`/tmp/rustycore-worldport-early-output-before.log`); the previous
closure-during-world-state-read test did not exercise this earlier exit.

C++ `Map.cpp:427-463` calls SendInitSelf and then continues visibility/phase/map-entry
effects; its success result is not the socket send result. `WorldSession.cpp:250-255`
returns from SendPacket for an absent socket, while `MovementHandler.cpp:153-234`
continues the after-add/zone/pet/delayed-operation tail. The repair preserves this separation:
the private sender returns `None` for unavailable projection, `Some(false)` for rejected
delivery, and `Some(true)` for channel acceptance. Only unavailable projection takes the
existing early rejection path. Rejected delivery runs the represented remaining native
effects and fails final client readiness, even if later publication were accepted.

The existing self-CREATE test now pins all three outcomes and still compares exact prepared
and delivered bytes. The 133-line post-add test module covers successful login/worldport,
closure during the read, and closure before ACK; worldport cases also pin consumption of
the canonical delayed resurrection. Character's reviewed logical growth is 36 test lines
(20,648 production / 13,069 tests / 33,717 total); misc travel is 681 physical lines.
No new Player/Session state, lock, clock, query, packet representation or physical exception.

This repairs delivery-dependent loss of represented native work, not missing C++ gameplay
or cancellation resumption. In particular, a dropped worldport future can still interrupt
the visibility/world-state awaits; the far semaphore and destination have already cleared.
The factory still calls disconnect save then cleanup without a retained whole-transfer
stage. Completing that contract requires incarnation-bound remaining work before the save
admission/cleanup decision, not replaying the network ACK or merely postponing SaveToDB
while the factory discards the owner. C0–C4 remain open; no live durability or capture claim.

Local aarch64 evidence above `71383a93`: `cargo test --offline --locked -p wow-world
--lib` passes 3,766 tests (one ignored), and `--test production_login_player_owner` passes
12. The checker's `--lib persistence_policy` passes five; syntax-only ownership passes
with the same 282 production/433 fixture fields, 3,690 associated items and 590 registry
rows. Architecture check/self-test and format/diff checks pass. Quick manifest
`target/validation-v2/manifests/20260906T192245.785370Z-3-quick.json` verifies green.
No push, restart, live DB test or macro closure is claimed.

### C1/C3 completion separates server effects from publication — above `5b6a233d`, 2026-09-06

The controlled worldport scenario closes its receiver inside the initial-world-state
read, after attachment/self CREATE and before the post-add packet tail. Against
`5b6a233d` with the test, the actual handler ends LoggedIn instead of Disconnecting
(`/tmp/rustycore-worldport-completion-before.log`). This is a reproduced false readiness
transition, not evidence that the whole initialization can be replayed safely.

`send_stat_update` now reports unavailable projection or the existing channel-send result.
Worldport checks this final publication before LoggedIn and before logging terminal
acceptance. Missing Player identity/movement-state branches also disconnect rather than
declare readiness. The same packets and server effects run in their existing order;
post-add scaling, pet recovery and delayed resurrection are not skipped merely because
the output closes. Other stat-update callers retain their existing behavior. No new
state, clock, lock, queue, query or packet representation is introduced.

The reused 97-line post-add test module now has the two successful scaling cases plus
the closed-output case. The latter verifies that native destination scaling still occurs
while Session is not admitted as LoggedIn. C++ `WorldSession.cpp:215-270` can return from
SendPacket when the socket is absent without aborting the caller's gameplay operation;
`MovementHandler.cpp:153-234` continues after-add effects and delayed operations. These
anchors support separating domain completion from network availability, not aborting
all remaining server effects on the first failed packet.

The final fixture supplies stat catalogs and asserts projection availability, so its
closed-output case isolates delivery failure; the initial red reproduction lacked those
catalogs and establishes false readiness, not a rejected final packet. Two existing partial
worldport fixtures retain all native-state/packet assertions but now expect Disconnecting
when the final stat projection is unavailable. Logical Character grows by 19 test lines
only (20,648 production / 13,033 tests / 33,681 total); no physical ceiling is widened.

The completion contract for the remaining transfer integration has two independent axes:

- Server effects complete + presentation accepted: eligible for client-ready admission,
  subject to the full required packet/phase contract; acceptance is not client receipt.
- Server effects complete + presentation closed/failed: disconnect without LoggedIn;
  the coherent native state can enter the established save fences/receipt workflow.
- Server effects incomplete, regardless of presentation: retain the same incarnation and
  remaining operation; no saved/ready claim and no blind whole-ACK replay or owner discard.

This cut enforces the reproduced **terminal-send** gate only. The shared post-add helper
still returns unit, earlier senders have partial result propagation, and cancellation does
not yet retain a resumable whole-operation stage. A single boolean must not conflate those
two axes. Continue integrating explicit remaining server work with save admission and
factory cleanup; C0–C4 and live durability/capture/runtime acceptance remain open.

Local aarch64 validation above `5b6a233d`: `cargo test --offline --locked -p wow-world
--lib` passes 3,765 tests (one ignored); `--test production_login_player_owner` passes
12, and the standalone checker's `--lib persistence_policy` passes five. Syntax-only
ownership passes with unchanged 282 production/433 fixture fields and 3,690 associated
items (the stat helper's bool signature is reviewed in policy). Architecture check/self-test,
format/diff checks and quick pass; quick manifest
`target/validation-v2/manifests/20260906T191422.360574Z-3-quick.json` verifies green.
No capture, live restart/DB proof, publication or macro closure is claimed.

### C1 far destination joins the canonical teleport owner — above `160d1ce5`, 2026-09-06

Ownership migration under the safe-refactor skill, prerequisite to incarnation-bound
transfer progress. C++ `Player.h:2167,3098` and `Player.cpp:1315,1335,1383,1456` place
the teleport destination on Player. Rust's semaphores/recovery were already native,
but the far destination remained a separate production Session field. Adding progress
beside that split would permit destination and completion authority to diverge.

`PlayerTeleportStateLikeCpp::far_destination` now owns the existing optional map/position.
Private `session/lifecycle/transfer_destination.rs` (19 lines) uses generation-checked
Player access for reads/writes, including detached ownership. Immediate/delayed far
initiation, near-transfer clearing, suspend/ACK, homebind recovery, corpse checks and
save projections all migrate. The prepared save reads the destination directly from
the Player already captured under the manager guard, never through a locking accessor.
Missing/stale writers return failure rather than retaining a destination on Session.

The old member is private cfg(test) only, used by the existing ownerless fixture adapter
and fixture adoption. No production mirror, lock, clock, query or packet is added. The
former direct test assignments now use checked setters; existing exact-state tests
include the destination through active/detached ownership and replacement, and explicitly
reject stale reads/writes without changing the replacement. Save-header/recovery and
worldport packet tests exercise the migrated consumers. Near/far representations,
clear-after-attachment timing, terminal source-save exception and ordinary save semantics
are deliberately unchanged: this is not full C++ teleport/instance-location parity.

Reviewed syntax: 282 production/433 fixture Session fields (715 total), two crate-local
accessors, unchanged 590 registry rows/38 commands/8 resources. Only the affected field,
method entries and corresponding Session struct fingerprint change. Physical Session
root +8 lines, existing Session tests +27 (mostly accessor wrapping plus exact literals
and stale assertions); new bodies stay in the private lifecycle module. Logical Session
+29 production/+30 tests; Player's logical tree +2 tests and its sibling gameplay-state
file +1 production field. The family ledger now has six remaining production members.
The 101 legacy ceilings and their C4 split exits remain; no terminal exception is added.

Local aarch64 tests above `160d1ce5`, pinned PROTOC and validation Cargo cache:
`cargo test --offline --locked -p wow-world --lib` passes 3,764 (one ignored),
`-p wow-entities --lib` passes 721, and `-p wow-world --test production_login_player_owner`
passes 12 controlled production-linked cases. Syntax-only ownership, architecture
check/self-test, five preserved persistence-policy tests and format/diff checks pass.
`cargo check --offline --locked -p world-server` also passes; no server is installed or started.
Bounded quick passes with verified manifest
`target/validation-v2/manifests/20260906T185414.270148Z-3-quick.json`; only checkpoint and
ledger review prose was completed afterward. The exhaustive persistence snapshot is not regenerated.
No fresh capture, runtime installation, live DB or restart acceptance is claimed.

Next, bind progress to this same canonical transfer and distinguish attachment from
completed initialization before deferred-save/logout admission. Do not infer completion
from far=false, replay the whole ACK after cancellation, or discard the owner after a
deferred return. C0–C4 remain open; the migration does not establish live durability.

### C1 destination scaling phase and logout prerequisite audit — above `d70c88c3`, 2026-09-06

Intentional phase-order repair. C++ `Player.cpp:23648-23650` calls
UpdateItemLevelAreaBasedScaling after PhasingHandler::OnMapChange in the shared
SendInitialPacketsAfterAddToMap path. Rust instead called the represented helper only
from worldport, immediately after attachment and before ResumeToken/self CREATE; the
shared login post-add path omitted it. The two canonical-Player regressions fail against
`d70c88c3` with tests: worldport is already scaled during the initial-world-state read,
while direct login post-add remains unscaled at completion
(`/tmp/rustycore-post-add-scaling-before.log`).

The existing helper now runs in the shared post-add path after phase publication. Its
rules, owner access, remove/apply item modifiers and health-percentage restoration are
unchanged (`Player.cpp:28715-28729` is the C++ rule anchor). The redundant early worldport
call is removed; no new state, DB query, lock, packet representation or execution owner
is introduced. Existing aura-triggered scaling callers are not relocated. This establishes
the represented map-entry phase, not full item/stat or initialization parity.

Two tests in private `character_tests/post_add_scaling.rs` (78 lines) share the existing
typed lifecycle fixture. One invokes the real worldport handler, the other the shared
login post-add operation. The read callback inspects the same map-owned Player before
InitWorldStates without sending under its map guard; both assert final scaling. This is
a controlled phase observation, not an installed-client capture or full-login scenario.
Physical travel shrinks 679 -> 674; session_state grows 2,782 -> 2,785; the old test root
gains only two registrations lines (12,631 -> 12,633). Logical Character +3 production/
80 tests is explicitly reviewed; no new physical exception or syntax-policy item.

Local aarch64 evidence on the working cut above `d70c88c3`, using pinned PROTOC and
the validation Cargo cache: `cargo test --offline --locked -p wow-world --lib` passes
3,764 tests (one ignored); `--test production_login_player_owner` passes 12 controlled
production-linked cases. Syntax-only ownership, architecture check/self-test, five
standalone persistence-policy tests and format/diff checks pass. Bounded quick passes;
verified manifest `target/validation-v2/manifests/20260906T184211.729956Z-3-quick.json`.
Only this evidence paragraph was completed after the run. No fresh client/capture,
live DB, deployment or durability acceptance is inferred from the local tests.

The same current-source audit identifies the next C1 integration prerequisites, rather
than treating the completed helpers as a finished disconnect operation:

- `WorldSession.cpp:544-551` completes pending far transfer before the logout flag/save.
  Rust `session/lifecycle/logout.rs` sets logout and saves without that completion, while
  the network ACK rejects Disconnecting and depends on working transport/fences. Calling
  that ACK blindly from disconnect is not a correct integration.
- `handlers/misc/travel.rs` clears far/pending destination after attachment, before its
  awaited visibility and post-add effects. A cancelled, attached Player can therefore
  have far=false while Session is still Transfer. The far semaphore alone cannot prove
  initialization complete or safely drive save admission/reentry.
- `Player.cpp:19324-19333` schedules DELAYED_SAVE_PLAYER during far transfer. Rust's full
  save currently does not provide that deferred outcome; merely adding an early return
  would discard the disconnect save when the factory next cleans up the owner.
- The bounded Terminal recovery/source-save contract remains valid for its tested failure
  case, not a substitute for ordinary transfer completion. Full-save APIs return unit;
  factory completion/logging alone is not evidence of committed durability.

Continue with an explicit completion/admission contract for the **whole** transfer and its
non-network effects before enabling deferred-save/logout integration. Preserve the existing
incarnation-bound save receipt and commit-unknown fences. C0–C4, nonblocking output,
before-add completeness and real durability/runtime acceptance remain open.

### C1/C3 rest publication survives cancellation — above `bb0c6d7e`, 2026-09-06

Intentional Rust async-lifecycle repair, separate from the preceding structural extraction.
The controlled canonical-Player scenario cancels post-add exactly at a pending initial-world-
state read, after leaving faction-area rest, then reenters the same destination. Against
`bb0c6d7e` with the test, it fails: the last packet is PhaseShiftChange rather than the
required resting-field UpdateObject (`/tmp/rustycore-rest-cancel-before.log`). The first
city-rest fixture exposed the separate missing-zone-data early return and was corrected
to use the represented faction-area transition; no city-rest rule was changed to pass it.

The existing native Player rest marker now survives the world-state await and zone reentry.
After-add consumes it only after the existing sender reports channel acceptance. Both
normal zone-update branches likewise retain it on unavailable projection/closed output.
The sender returns its existing send result; serialization, connection and ordinary packet
order are unchanged. There is no await between projection/send/marker retirement, no new
state, lock, retry task or persistence query. Channel acceptance is not client receipt.

C++ anchors: `RestMgr.cpp:95-122` changes Player flags on zero/nonzero crossings;
`Object.cpp:3722-3728` builds updates before clearing the object mask;
`Map.cpp:1929-1948` collects updates and sends them in the map update phase. C++ does not
have this Rust database await/cancellation point. Retaining pending presentation is the
async adaptation; this does not claim C++ retains a marker until client acknowledgement.

The new 108-line private test uses the existing lifecycle fixture with queued owned
ready/pending futures, polls the actual after-add operation to the controlled read, and
drops it without timeout or abandoned task. It verifies the native marker after cancellation,
the delayed packet on same-zone reentry, retirement after acceptance, and retention with
a closed sink in the ordinary known-zone branch. Its empty temporary terrain directory
isolates the scenario from installed map data. Affected library tests pass: 3,762, one
ignored (`cargo test --offline --locked -p wow-world --lib`, local aarch64, pinned PROTOC
and existing validation Cargo cache).

`cargo test --offline --locked -p wow-world --test production_login_player_owner` passes
12 controlled integration cases. Syntax-only ownership, architecture check/self-test,
five standalone persistence-policy tests and format/diff checks pass. Bounded quick
passes with verified manifest
`target/validation-v2/manifests/20260906T183316.652618Z-3-quick.json` (working-tree evidence;
the checkpoint text was completed afterward, without changing the tested code).

Reviewed footprint: Session +5 production lines, Character +4 production/+121 tests
(108 new scenario lines and 13 shared-fixture/module lines). The three exact legacy file
ceilings rise by those local deltas; their C4 split exits remain, without new exceptions.
The only syntax contract change is the rest sender's boolean result; fields, owners and
590 registry entries remain unchanged. The persistence snapshot is not regenerated.

This closes the reproduced marker-loss path, not restart durability, whole-post-add replay
safety, global nonblocking output or full worldport/logout completion. Earlier visibility
and initialization effects can already have occurred when post-add is cancelled; blindly
replaying the entire ACK is not justified. Ordinary output remains synchronous; saturation,
phase-coordinated ownership and remaining direct rest senders still need their complete
C0/C3 treatment. No live DB/client/capture/restart acceptance is claimed.

### C1/C2 post-add zone application boundary — above `c936c826`, 2026-09-06

Behavior-preserving boundary extraction: the existing terrain resolution and represented
zone/rest application now live in private `handlers/character/entry_zone.rs` (80 lines).
The after-add adapter invokes the synchronous operation at exactly the previous position,
after visibility and before InitWorldStates/CUF/auras/phase/rest publication. It still owns
the canonical Player through existing scoped access; no state, lock, query, opcode or
execution owner is added. Terrain reads are file I/O outside Player/map guards, not a
pure calculation. False means the existing missing-seeded-state early exit, not a general
owner validation result; true is not proof of complete terrain authority.

C++ `Entities/Player/Player.cpp:7298-7438` anchors area/zone rest transitions and their
broader effects. The Rust packet-suppressed zone path and its transitive area, hostile-area
and native rest mutators were inspected: this represented subset does not publish. It is
not full C++ UpdateZone parity: phasing, PvP, auras, channels, scripts and other zone effects
still require complete treatment before logout reuse can claim the full operation.

The existing worldport rest scenario retains its destination and packet-order assertions.
Additional assertions directly apply the same transition with an open and closed sink,
read authority from the canonical Player, and retain the deferred rest update marker.
Truncated terrain preserves the seeded location with incomplete authority; a missing tile
instead uses the catalog fallback (zero in this fixture), also with incomplete authority.
These are distinct existing branches, not a new fallback policy.

Physical session_state shrinks 2,834 -> 2,778; character root 2,729 -> 2,728, both ceilings
tightened. Logical Character grows by 23 production lines for the private wrapper/imports
and scope documentation, with no logical-owner retirement claimed. Travel tests grow
631 -> 686. One exact impl and method are added to syntax policy; no fields or registrations
change and the exhaustive persistence snapshot is unchanged. The 101 legacy ceilings remain.

Local aarch64 evidence on this working cut above `c936c826`, pinned PROTOC and the
validation Cargo cache: `cargo test --offline --locked -p wow-world --lib` passes
3,761 tests (one ignored); `--test production_login_player_owner` passes 12 controlled
production-linked cases. Syntax-only ownership passes (3,688 exact items, unchanged
283 production/432 fixture fields and 590 registry rows). Architecture check/self-test,
five standalone `persistence_policy` tests, format and diff checks pass. No live DB,
restart, client/capture or durability acceptance is claimed.
Bounded `validation-v2 quick --base HEAD` passes; verified manifest:
`target/validation-v2/manifests/20260906T182156.268630Z-3-quick.json`.
This is working-tree evidence; only this evidence paragraph was added afterward.

At this earlier cut, the separate unresolved cancellation boundary was: after-add consumes the deferred rest flag
before awaiting world-state loading. Cancellation can lose that presentation intention;
zone reentry also resets the marker. This refactor deliberately does not change those
semantics. A repair needs cancellation/reentry/delivery-acceptance tests, not an untested
line move. The marker-loss repair and its bounded evidence are recorded immediately above;
full worldport/logout/deferred-save integration and C0–C4 remain open.

### C1/C2 self-CREATE construction/delivery boundary — above `a1610315`, 2026-09-06

Behavior-preserving adapter extraction under the safe-refactor skill. The complete
existing worldport self-CREATE construction moves from `handlers/misc/travel.rs` to
the private 178-line `worldport_create.rs`. The constructor takes `&self` and returns
an owned `Option<UpdateObject>` without I/O or async waiting. The old catalog-bearing
send entrypoint immediately invokes it and preserves the same packet, connection,
logging and success/failure result. Its currently unused trait catalog remains a
compatibility argument until the full ACK/catalog contract is narrowed; no new trait,
field, lock, persistence operation or opcode registration is introduced.

The moved construction body matches the previous body exactly after whitespace,
false-to-None early-return wrapping and removal of the unused catalog binding. The
XP self-CREATE test now compares every byte of the prepared packet against the actual
delivered packet, asserts construction sends nothing, then closes the receiver and
proves construction still succeeds while the delivery adapter returns false. Existing
appearance/trait/deferred-resurrection packet ordering and incomplete-CREATE tests remain.
Library tests pass: 3,761, one ignored (`cargo test --offline --locked -p wow-world --lib`,
local aarch64 with pinned PROTOC and the validation Cargo cache).

`cargo test --offline --locked -p wow-world --test production_login_player_owner` passes
12 controlled cases. Session syntax/exact-item policy, five preserved persistence-policy
tests, architecture check/self-test (20 fixtures), format and diff checks pass. Bounded
quick above `a1610315` passes; verified manifest:
`target/validation-v2/manifests/20260906T180601.402127Z-3-quick.json`.
The now-unused travel import is removed after that run; no serialization expression changes.

C++ `Maps/Map.cpp:1826-1851` separates building UpdateData from SendDirectMessage.
`Entities/Player/Player.cpp:3586-3608` includes inventory object CREATEs for the Player
itself. The old Rust comment implying their omission was justified by client retention
is corrected: item CREATEs, transports and placeholder combat stats remain parity gaps,
not changes hidden in this refactor. Their repair still requires exact packet evidence.

Physical files: travel 848 -> 679 lines; new builder 178; misc root 156 -> 157; existing
travel tests gain 25 lines. No physical/logical ceiling changes or new exceptions.
One exact private-parent-visible builder method is added to the ownership policy;
the production registry, send entrypoint and current execution owner are unchanged.

This is not a coherent whole-Player snapshot, a completed gameplay-owner migration or
logout completion. The builder still uses the established multiple scoped reads and
partial payload semantics. Do not retain its result across mutations/awaits as though it
were an incarnation/revision-bound reservation. Production still sends immediately through
the existing synchronous channel; backpressure, before-add order, complete initialization,
logout/deferred-save integration and C0–C4 terminal acceptance remain open.

## Independent extension checkpoint — 2026-09-05, `c67acbfd`

The post-freeze third module, `expedition` (ID 73), passes the real native, Rust-Wasm,
C-Wasm and mixed lifecycle challenge. Its contract-only crate adds a non-Clone variable
15–23-byte stampbook, sorted unique checkpoints, retained lifetime history and map-residence
contribution suspension/restoration. The C implementation independently encodes the same rules.
This custom behavior is not a C++ gameplay port, durable reward or production SDK.

- The original freeze remains unchanged: 53 existing files byte-identical, four permitted
  declarative dependency/registration changes and seven new extension files. No host, ABI,
  oracle, benchmark or supervisor implementation changed to accommodate this module.
- [Correctness evidence](evidence/modularity-conformance-v2-expedition-20260905.json.gz):
  51 host tests, 89 common/native-negative case executions and all four full lifecycle tests
  pass; `passed: true`, `decision_eligible: false` before costs. Canonical bytes, revisions,
  complete ordered traces/callback results and observables agree across actual producers.
- [Root semantic review](evidence/modularity-conformance-v2-expedition-20260905.review.json)
  binds all 64 current source files to the original freeze. The extension author did not
  author the core or initial modules but previously worked on the supervisor; this is not
  a blind external-consumer audit. Dependency edits were inspected, not merely hash-approved.
- Negative coverage includes malformed variable state, capacity/overflow, duplicate writes,
  detached admission, transfer failure, stale incarnation, reset, removal/reinstallation and
  unload. A calls=4 limit intentionally leaves an accepted stamp without its later contribution;
  retry does not duplicate history and detach/attach restores the derived contribution.
  This demonstrates explicit partial effects, not transaction rollback or durable recovery.
- The new module's native/Wasm strict Clippy checks pass. Strict driver Clippy reports existing
  `drop_non_drop` at `driver/src/bench/dispatch.rs:96` and `large_enum_variant` at
  `driver/src/harness.rs:61`; frozen code was not changed to silence them.

The preregistered 320-sample cost campaign now **passes** on aarch64, with
`decision_eligible: true` and no errors: see the [full result and retained raw evidence](modularity-conformance-results.md).
The original freeze and thresholds remain unchanged. Dense/10k median update batch p99 is
25.44 ms native, 34.77 ms Rust-Wasm, 32.74 ms C-Wasm and 29.97 ms mixed. These pass the
provisional laboratory bound, **not a hypothetical 10 ms whole-map frame**. The custom STAMP
operation is covered by lifecycle tests, not a dedicated timed workload.

The finite pre-migration gate is complete; next is the first production C1/C2 vertical with C0
admission/phase evidence before replication. No production migration is yet implemented or
accepted. All production C0–C4 obligations remain open as described below.

## Conformance implementation checkpoint — 2026-09-05, `118171c1`

The private V2 laboratory now has two independently defined modules running as native Rust,
Rust Core Wasm, C Core Wasm and mixed execution. This is **passing pre-freeze evidence**, not
the complete third-module/cost gate and not a production storage or SDK integration.

- Source: `tools/architecture/modularity-conformance/`, committed at `118171c1`.
- aarch64 validation: 51 host tests (27 core + 24 Wasm), 15 native driver/module tests,
  43 Python control tests and 89 functional case executions (20 common per mode + nine
  native-only negatives). Complete common oracles agree, including ordered callback results,
  canonical bytes, revisions, identity, residence and contributions.
- The retained [pre-freeze report](evidence/modularity-conformance-v2-prefreeze-20260905.json.gz)
  ran 19:09:12–19:09:31 UTC and reports `passed: true`, **`decision_eligible: false`**.
  Its [source freeze](evidence/modularity-conformance-v2-prefreeze-20260905.freeze.json)
  binds 57 laboratory files and the two baseline module IDs before third-module authoring.
  The report precedes the commit; its complete source hash set matches the committed freeze.
  Freeze SHA-256: `5aeadb7a4a889bdfc879f9c69898c85325ea555479204ec303b9a8880fbc9424`.
- Before freezing, review corrected opaque codec admission, canonical round trips, replay
  capability checks, failed-write revision allocation, cumulative validation fuel and hidden
  second-memory admission. A zero-capacity Wasm read regression was reproduced as `Invalid`
  instead of the expected capacity `Limit`, then fixed without weakening overlap rejection.
- All timing thresholds remain provisional laboratory gates. The protocol's pre-measurement
  completion added a 4 MiB artifact cap and increased command supervision from 120 to 240s
  to accommodate the already allowed batch/cold work; it did not change timing/RSS thresholds.
  At this pre-freeze checkpoint no V2 measurement campaign had run. Native-only compilation passes with two unused-code
  warnings for the intentionally unavailable opaque adapter; native execution is not sandboxed.

The next step at this historical checkpoint was the third independent state/lifecycle module,
using only its own code plus the permitted
dependency/declarative registration edits. Verify the unchanged freeze, review the new module and
exercise all four real compositions before the predeclared measurement campaign. A host fix
invalidates that challenge; do not rehash its existing third module into a supposed fresh proof.
Testable modularity and physical navigability remain distinct from a storage choice;
mock replay does not prove durable recovery.
All production C0–C4 obligations below remain open to their stated extent. No push, deployment,
restart, database mutation, hot reload or macro-final acceptance is claimed here.

## Approved remaining plan — 2026-09-05, reviewed code `93e4002a`

Keep **one macrodeliverable, issue #578 / PR #579**. The blocks below are internal
implementation/review checkpoints, not new issues, PRs, or approval requests per helper.
Preserve useful work already landed or committed. This plan supersedes field-by-field
selection and any instruction to postpone all integration evidence until the end of the
macro. It does not authorize a push, merge, deployment, restart, or unrelated gameplay fix.

Progress means an accepted transition contract, not fields moved, test counts, commits,
or an unsupported completion percentage. The 132 immutable catalog/configuration/service
dependencies below are not 132 mutable owners. Each completed operation must document
input/admission, decision, canonical owner, persistence when applicable, and ordered
publication; migrate all related consumers and retire the superseded fields, setters,
installation paths and bridges. Small internal commits remain useful; they do not define
the user-facing delivery size. Exact inventories remain acceptance evidence, not an
alternative to complete feature boundaries.

| Internal block | Exit contract and evidence | Current acceptance |
| --- | --- | --- |
| C0 — Execution contract | Name owner, admitted residence, writer, phase/clock, persistence boundary and publication order before enabling new writers. Define executable `PacketProcessing` and world/map phase expectations. Implement the relevant scheduling proof alongside C1/C2, not as an untested timing rewrite at the end. | Open; metadata equality alone does not apply its temporal semantics. |
| C1 — Player lifetime and persistence | Preserve one incarnation through login, active/detached state and transfers; replacement/retirement invalidate its old handle without affecting the new incarnation. Cover failed attach/unload and save/logout. Coherent save projection plus generation/revision-safe acknowledgement (or proven equivalent exclusion). Production-linked lifecycle and controlled-I/O interleaving tests. | Partial implementation; complete lifecycle/composition evidence remains open. |
| C2 — Complete gameplay operations | Finish represented Player families through narrow command/query/outcome APIs, every related reader/writer and capability consumer, then retire old Session access. Keep transaction and publication semantics explicit. Each family must also meet the physical source/test navigability policy. | Partial; catalog borrowing, native storage or file splitting alone do not close a vertical. |
| C3 — Runtime and delivery completion | Execute migrated work under the admitted owner/phase; preserve #28/#371 cuts, barriers, backpressure and shutdown. No packet delivery or I/O under owner locks. Remove remaining Session/legacy authority and whole-entity bridges. | Open; C0 obligations accompany each writer migration. |
| C4 — Boundary decisions and macro acceptance | Resolve the inherited #378 dispatch/kernel/transport decisions and remaining dependency/catalog exceptions; justify legitimate application-adapter edges by responsibility. Complete remaining physical core/adapter/composition/tooling and test decomposition with bounded file-specific exceptions; extend the existing physical ratchet. Retire bridges, run final inventories and clean-HEAD validation, then hand off evidence to #153. | Open; #153 verifies the result, it does not implement these known cuts. |

### Physical and semantic acceptance together

Read [module design and source navigability](module-design-guidelines.md), including its Rust
submodule skeleton. The usual 200–800 physical lines are a target, not a minimum; above 1,000
prompts routine cohesion review, not another approval. No handwritten production/test/fixture/tool
file above 2,000 physical lines is terminally accepted without a concrete justified file-specific
owner/exit exception. There is no permanent Session or aggregate exemption. Each completed family
must satisfy this and its semantic owner/API contract; a distributed God object is still debt.

C4 inventories and retires the remaining legacy files in #133 scope inside this macro, including
Session and its large tests, using per-file ceilings tightened after coherent validated reductions.
It extends the existing architecture checker to physical files, integration tests, integrated tools
and verifiable generated-source provenance, with negative fixtures for growth, move/rename escape,
oversized tests and stale/expired exceptions. Keep logical-owner coverage independently. The current
checker now enforces physical migration ceilings alongside the logical guards; its migration
PASS still does not establish terminal physical completion.

Safe same-owner mechanical source/test splits can precede or run alongside the selected-hecs
conformance experiment. That gate remains mandatory before production storage migration, not
before organizing files. Keep all C0–C4 obligations and PR #579; no helper issues or routine stops.

### C1: precise lifetime and save requirements

- A detached Player is a valid owner, not a missing Player. Persistent-state queries remain
  available; commands requiring an active map return `NotActive`. Missing, stale-generation
  and inconsistent residence must be distinguishable internally; never fabricate defaults.
- Attach/detach, failed transfer, map destruction/unload and generation retirement use the
  same lifetime authority. Review or restrict mutable Map escapes that can bypass it.
  The failed-transition cut above deletes fake counter-only evacuation and refuses occupied
  destruction/bulk unload, with real typed-Player and automatic-update reproductions. Actual
  evacuation delivery remains open; this is **not demonstrated live loss or complete shutdown
  parity**. C++ `Map.cpp:1629-1643` requests evacuation and `MapManager.cpp:322-339` refuses
  destruction while players remain. Mutable Map escapes still need their stated retirement.
- Prepare a coherent owned save DTO for the intended incarnation. Acknowledgement after
  `.await` may clear only the confirmed saved revisions, or must have an explicit equivalent
  exclusion proof. A late result must not affect a replacement incarnation or erase a newer
  mutation. Preserve existing money fences, cancellation, rollback, unknown-COMMIT and
  recovery semantics. The coherent-save cut above replaces fragmented production reads and
  group-wide ACK; its controlled interleaving reproduces incorrect dirty-state cleanup on the
  old path, not live database corruption. Full lifecycle/durable-writer acceptance remains open.
- Preserve C++ far-teleport deferred save (`Player.cpp:19327-19333`) and near-teleport
  destination persistence without relocating the runtime Player (`19480-19514`). Logout
  finishes pending far transfers before saving (`WorldSession.cpp:544-551`). Differences
  deliberately frozen by old-Rust equivalence tests need an explicit parity decision;
  such tests do not turn a known discrepancy into final C++ behavior.

### C0/C3: execution is distinct from storage

`session/dispatch.rs` currently gates SessionStatus and calls the registered thunk; the
registry's `PacketProcessing` values do not by themselves enforce execution phase.
C++ `WorldSession.cpp:64-108` filters by processing class and Player residence;
`Map.cpp:666-718` updates map sessions before respawns and Player/object updates;
`MapManager.cpp:287-318` imposes a barrier before `DelayedUpdate`.

Test these actual paths, not just enum labels. Converge one complete responsibility with
its consumers, preserving relevant absolute deadlines as well as elapsed diffs. A global
MapManager mutex is a transitional access mechanism, not the final gameplay API. A single
writer per responsibility does not require a Tokio task per map or a new worker pool.
Separate intentional observable timing corrections from behavior-preserving movement.

### C0/C3 integration constraints — 2026-09-06

Bounded source review at `b6faea6f`, using `sed` on the functions below and `rg`
for callers. This refines the next integration cut, not the acceptance scope or
runtime authority. These are source findings, not timing measurements, live QA or
a new whole-port audit. C++ paths are relative to the legacy tree's `src/`.

- `server/worldserver/Main.cpp:519-552` measures world diff and honors
  `MinWorldUpdateTime` (default 1 ms). `server/game/Maps/MapManager.cpp:287-318`
  accumulates that diff until its separate map interval passes, then joins map
  updates before delayed updates. Rust `world-server/src/session_factory.rs:170-208`
  measures a diff per Session task and sleeps 50 ms when idle. Neither that idle
  sleep nor the map interval is a substitute for the C++ world cadence.
- `server/game/Server/WorldSession.cpp:488-540` advances time sync in the map
  pass, processes ready query callbacks in both passes, and confines logout/socket
  retirement to the world pass. Calling the whole current Rust Session update twice
  would duplicate unrelated work and would not preserve these phase tails.
- `server/game/Handlers/CharacterHandler.cpp:1550-1561` submits the rename query
  and returns. `common/Utilities/AsyncCallbackProcessor.h:40-51` snapshots callbacks;
  `server/database/Database/QueryCallback.cpp:205-224` invokes a prepared callback
  only if ready, without waiting for the DB. Rust
  `wow-world/src/handlers/character/lifecycle.rs:253-304` instead awaits the rename
  candidate inside its mutable Session handler. A global barrier awaiting unchanged
  Rust handlers would promote this per-session wait into a server-wide stall.
  This proves a scheduling mismatch, not that every C++ DB operation is async.
- `server/game/Maps/MapReference.cpp:22-28` inserts Player references at the front;
  `server/game/Maps/Map.cpp:666-718` iterates them for map sessions before respawns
  and Player/object updates. A GUID-sorted visit plan is not that membership order.
  Transfers, unlink and new participants need explicit iteration semantics, not
  a cached `is_in_world` flag or a second mutable Player mirror.
- Rust `world-server/src/session_factory.rs:269-277,346-408` registers before async
  initialization and unregisters after disconnect persistence/cleanup. Registration
  therefore does not imply readiness to acknowledge a simulation grant.
- Rust `world-server/src/runtime/delivery.rs:1639-1699` awaits a `spawn_blocking`
  tick that owns cloned mutation handles. Dropping its async waiter does not prove
  the blocking work stopped. Cancellation is not a successful barrier acknowledgement.
- Rust `world-server/src/app.rs:5480-5620` closes registration, queues kicks, waits
  for Session flush acknowledgements, drains sessions and only then stops respawn
  producers/writer. A Session waiting for a grant from an already stopped scheduler
  cannot consume that flush. Terminal world/control execution must remain possible.
- `server/game/World/World.cpp:2748,2817-2823` updates maps before world game events.
  The Rust canonical loop also contains game-event orchestration; its entire body
  cannot be relabeled as a map phase without separating that responsibility.

Integration direction within the approved single-owner design:

1. Keep mutable Session ownership in its existing task and canonical Player/Map
   ownership in their authority. Compose bounded phase execution at startup, without
   `Arc<Mutex<WorldSession>>`, a map guard across handler futures or a generic bus.
   A mutex around autonomous loops excludes overlap but does not establish phase order.
2. Before placing dispatch behind a shared barrier, classify asynchronous boundaries
   against C++. Convert submit/ready-callback operations into owned requests and
   completions applied by the Session in its permitted callback pass. Preserve actual
   synchronous contracts, transaction fences, unknown-COMMIT recovery and publication
   order. A pending `&mut Session` future cannot be retained while reborrowing Session
   for another pass; detaching it is not an ownership solution either.
3. Distinguish registration, phase readiness and finalization. Identify admitted work
   by epoch and session incarnation; Player work additionally revalidates its canonical
   handle/residence at execution. Stale completion must neither mutate a replacement
   nor release its barrier. Session and Player identities are distinct; account for
   identity reuse/exhaustion before relying on them as acknowledgement fences.
4. Compose world and accumulated map diffs as independent gameplay diff producers
   retire; do not add a third autonomous simulation loop beside the existing two.
   Preserve serial world-session and within-map session execution. Cross-map parallel
   execution requires the map/delayed-update barrier. Retain #28/#371 guarantees.
5. Separate packet admission, ready callbacks, phase-specific Session tails and map
   simulation. Use registry metadata as the only opcode call source and retain FIFO
   head eligibility. Name lock order and out-of-lock delivery, including saturated
   sinks and pending persistence, for each integrated operation.
6. Keep shutdown control progress after simulation admission closes. Retain and join
   actual mutation work before declaring quiescence; failed/cancelled acknowledgement
   is not success. Preserve disconnect save/cleanup and producer-before-writer drain
   order without introducing unscheduled finalization mutations.

The next implementation must connect the handler continuation boundary and phase-specific
passes to real Session/composition paths, not add another isolated enum/demo scheduler.
Integrate a real Player lifecycle/save operation before replicating across families.
Rename is a required pending-callback regression, not a substitute for that C1/C2 vertical.
Production-linked tests must drive two sessions, a pending DB result, world/map transitions,
logout/transfer, stale completion, cancellation of an active blocking tick and a saturated
sink. Prove eligible work progresses while an asynchronous query is pending, completions
apply only in their permitted pass, and shutdown never acknowledges unfinished mutation.
Runtime/capture and real durability gates remain. This refinement adds no routine approval
gate or micro-issue and does not reduce C0–C4.

Documentation-only validation above `b6faea6f` (aarch64): standalone checker
`cargo test --offline --locked --release --lib persistence_policy` with its manifest
passes all five tests, including preserved workflow semantics and checked snapshot/policy
consistency. `validation-v2 quick --base HEAD` passes, with verified manifest
`target/validation-v2/manifests/20260906T122545.109184Z-2-quick.json`; `git diff --check`
passes. No source, policy or inventory baseline changes, runtime tests, publication or
deployment are part of this review. The earlier physical-checker status wording in
PORT_PLAN and this checkpoint is reconciled with the already implemented ratchet.

#### Production-linked rename continuation boundary — above `5dcc118e`

The next C0/C3 investigation follows the callback through its transaction, not just
the initial query. C++ `CharacterHandler.cpp:1563-1610` calls `CommitTransaction`
then publishes rename success/cache changes; `server/database/Database/DatabaseWorkerPool.cpp:302-326`
posts the transaction to the DB executor without waiting for its result. Rust
`wow-world/src/handlers/character/lifecycle.rs:317-331` awaits the mutation outcome
before publishing; `wow-database/src/character_administration_adapter.rs:234-260`
awaits the concrete transaction. Therefore moving only the initial query into a ready
callback would leave a second blocking boundary in that callback. Removing the await
would also remove an existing result-before-publication fence, not be a structural move.
The current two-way administration outcome is not proof of unknown-COMMIT recovery.

`wow-world/tests/production_character_rename.rs` moves the existing typed-port success
test out of a `cfg(test)` library module and adds controlled pending-query, query
cancellation and pending-commit success/failure cases. All use the real public Session
constructor and handler, without widening production visibility or installing gameplay
fixture state. The only removed library mount belongs to that moved test. The 322-line
integration file retains its original ordered request/flag/response assertions; no
monolith ceiling or logical-owner baseline changes. Controlled futures are manually
polled, with no sleeps or live DB. They establish the current Rust continuation fence,
not parity of C++ scheduling, actual dispatcher phase integration or DB cancellation.
In particular, cancelling the mocked read cannot prove that an already-submitted real
transaction rolls back. No production behavior changes in this test cut.

Keep both boundaries explicit in the pending integration: Session-owned ready query
continuations and commit-result/publication handling, with real cancellation/recovery
classification. Do not replace the C1/C2 Player vertical with this character-list test,
and do not label its current awaiting handler a completed asynchronous callback path.

Validation of this working cut above `5dcc118e` (aarch64):

- `cargo test --offline --locked -p wow-world --test production_character_rename`:
  four tests PASS, none ignored; the pending-commit test executes both outcomes.
- The same production integration target with `--release`: four tests PASS, none
  ignored. Both profiles compile the real library without its `cfg(test)` fixture paths.
- `session-ownership-check check --syntax-only`: PASS, unchanged 282 production/
  432 fixture fields, 50 impl owners/3,673 associated items and 590 registry rows.
- `python3 tools/architecture/check_architecture.py check`: PASS, 954 source files,
  101 legacy ceilings and unchanged logical-owner totals.
- Standalone checker `cargo test --offline --locked --release --lib persistence_policy`:
  all five policy/snapshot consistency tests PASS; no inventory regeneration.
- `validation-v2 quick --base HEAD`: PASS, verified manifest
  `target/validation-v2/manifests/20260906T123047.312680Z-3-quick.json`.
  This is a bounded dirty-tree iteration, not a clean-HEAD publication final.

#### Owned rename application boundary — above `a12df541`

Subsequent read-ready/write boundary above `e72a3ad6` (2026-09-06): the same production
`rename` operation now composes an owned `prepare_rename` future and an opaque single-use
`PreparedRename::commit` continuation. Reading or discarding a ready candidate cannot
call the mutation port. The continuation retains only the typed persistence port,
GUID/name, previous name and remaining at-login flags; it has no Clone, Session/Player
borrow, packet sink or field access for callers. Both futures are `Send + 'static`.
The existing handler still awaits their composition, so production scheduling and its
result-before-response fence are unchanged. This is a structural prerequisite, not
an enabled callback queue, phase coordinator, new worker or proof of SQL rollback.

The exact C++ contract was re-read at `CharacterHandler.cpp:1550-1610`,
`AsyncCallbackProcessor.h:40-51`, `QueryCallback.cpp:205-224` and
`DatabaseWorkerPool.cpp:302-326`: pending reads do not block callback passes, but a
retired callback must not start a transaction. Moving the entire former async body
into a detached worker would violate that boundary. The new staged implementation
makes that distinction explicit while retaining the current Rust commit-result fence.
Before enabling asynchronous production callbacks, Session must own admission of ready
reads and publication; already-submitted commits need actual supervised shutdown and
classified recovery, not an assumption that dropping a future rolls back a database.
The first complete Player lifecycle/save vertical and all C0–C4 gates remain required.

Validation on this working cut above `e72a3ad6` (aarch64): `cargo test --offline --locked
-p wow-world --lib character_administration::tests` passes three manually polled tests:
ready reads cannot commit without explicit continuation consumption, read retirement
before/after readiness cannot write, and discarding an unpolled commit cannot submit it.
`cargo test --offline --locked -p wow-world --test production_character_rename` passes all
five unchanged production-linked cases, including pending query/commit and all rejection
outcomes. Both use `PROTOC=/home/ubuntu/.local/protoc/bin/protoc` and the existing local
validation Cargo cache. `session-ownership-check check --syntax-only` and architecture
`check` pass with unchanged Session/registry/logical-owner counts, 976 physical files
and 101 remaining legacy ceilings. Production application/test files are 126/182 lines;
no ceiling, snapshot or policy is regenerated. Bounded quick passes with verified manifest
`target/validation-v2/manifests/20260906T133956.099202Z-3-quick.json`; format/diff checks pass.
These tests do not prove phase scheduling, asynchronous commit supervision or real durability.

Boundary extraction, preserving current Rust behavior: the real rename handler now
calls private `wow-world/src/character_administration.rs`. The operation owns a narrow
GUID/name request and an Arc to the existing SQLx-free administration port; its return
type explicitly guarantees `Future + Send + 'static`. No Session/Player borrow, catalog,
packet writer, concrete SQL or new lock/task is available inside that operation. Account/
name admission and result presentation remain in the handler. Query outcome, at-login
eligibility, clearing only the rename bit, awaited commit, failure classification and
the previous name for the success log belong to the application operation.

This removes the Session dependency from the persistence continuation, not the handler's
await or its current task clock. It is **not** yet submitted to a ready-callback queue;
do not detach the whole operation and thereby let a query cancelled with its Session
start a new transaction. C0/C3 still needs explicit query-ready/commit/publication phase
integration, cancellation and supervised finalization, including the C++ difference
recorded immediately above. All C1/C2 Player and macro gates remain.

The exact legacy references remain `CharacterHandler.cpp:1550-1610` and
`DatabaseWorkerPool.cpp:302-326`. No intentional scheduling or durability repair is mixed
into this extraction. The existing production rename integration tests retain their
query/commit cancellation and publication assertions; a fifth test covers NotFound,
query failure and missing rename eligibility without starting a transaction. Failure
responses assert the exact C++ error byte 25 and absence of a GUID. No opcode registration,
packet encoding, concrete statement or persistence inventory entry is changed.

Physical result: lifecycle handler 839 → 821 lines; private application file 77 lines;
production integration file 358 lines. Character logical-owner ceilings tighten from
20,622/12,811/33,433 to 20,604/12,811/33,415 production/test/total lines. No test moves or
new physical exception are hidden in that reduction; the application source is separately
counted by the physical inventory. This is one extracted operation, not closure of the
remaining create/delete/customize or Player lifecycle capabilities.

Validation above `a12df541` (aarch64): production integration target
`cargo test --offline --locked -p wow-world --test production_character_rename`
passes all five tests in both dev and `--release`; `cargo test --offline --locked
-p wow-world --lib character_rename` passes all three existing admission tests.
The standalone syntax-only ownership check passes with unchanged field/item/registry
sets. Architecture `check` and `self-test` pass, including the tightened character
ceiling; all five persistence-policy/snapshot tests pass without a baseline refresh.
Final bounded quick manifest
`target/validation-v2/manifests/20260906T124321.209765Z-4-quick.json` passes and verifies;
format/diff checks pass. This is not clean-HEAD publication, fresh capture or live DB
evidence. Only this reviewed application extraction and its tests are claimed complete.

### Proportional evidence inside the macro

The [plan's reanalysis checkpoints](modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication)
are the review cadence: finite conformance before storage migration; then the first production
C1/C2 vertical with C0 admission/phase evidence **before replicating it to other families**.
Review the complete C0–C4 balance at C4 before #583 production integration. #153 audits both
merged macros; #47/M6.2 triggers the later whole-port planning pass. These are internal evidence
reviews, not routine approval requests or reasons to stop after each helper.

1. During iteration: focused positive/negative tests, formatting/diff checks, routed quick
   validation and syntax-only ownership ratchets where affected.
2. At each affected contract checkpoint: focused adversarial local review and production-linked
   integration tests (library compiled **without** `cfg(test)`) in dev and release. The existing
   `production_login_player_owner` integration target catches production-only wiring failures,
   but its three bounded scenarios do not prove a complete login/save/logout cycle. Run it
   explicitly when relevant: ordinary `validation-v2 final` runs library suites, not this target.
3. For lifetime/persistence/execution: controlled persistence futures and explicit ticks;
   two sessions, generation replacement, mutation after snapshot, failed attach/unload,
   Applied/rollback/unknown/cancellation and a saturated delivery sink. Prove that old work
   cannot mutate/publish for a new incarnation or hold other sessions/ticks behind I/O locks.
4. Before macro publication: `validation-v2 final` on clean committed HEAD, focused integration
   evidence tied to that SHA and the issue's exhaustive ownership/persistence/bridge inventories.
   #153 then performs the terminal audit on merged integration HEAD. Do not rerun exhaustive
   persistence scans merely for each helper or issue-state metadata refresh. Metadata-only
   updates must still validate preserved persistence policy/workflow issue references and
   snapshot-policy consistency; syntax-only does not perform those persistence checks.
5. Capture-diff evidence is required for changed bytes, metadata, connection or observable order;
   distinguish retained regression evidence from fresh action-specific captures and recapture
   when applicable evidence is absent or explicit acceptance requires it.
   live QA is required for live lifecycle/runtime changes. Real MariaDB commit-loss/crash and
   relogin evidence must use an authorized runtime fixture; mocks cannot establish durable
   recovery. Pending runtime authorization is not a reason to stop safe code/tests/docs work,
   nor permission to claim live acceptance. Publication/deployment approvals remain separate.

Label evidence as **old-Rust equivalence**, **C++ contract**, **production integration**, or
**live/capture**. Record exact SHA, command, result, remaining boundary and host architecture
(development host aarch64; hosted runners x86_64). Keep local-first validation and the exact
`alseif0x` author-gated remote skips; no new remote approval gate or per-checkpoint PR.

### Plan ownership and next decision

#578 is an explicit prerequisite of #153. Closed #169/#574 and the bounded #378 delivery
remain closed; their inherited Session/catalog/kernel work is assigned here, not to the
terminal auditor. A legitimate packet/application adapter dependency may be retained with
a concrete classification decision; inventing traits or crates just to erase an exception
is not acceptance. Re-audit the next gameplay macro just in time against HEAD; do not
pre-granulate Part 2 or equate a historical issue closure with full gameplay parity.

The preserved historical persistence snapshot/policy still attributes 60 nonstable groups
to #153. This metadata is not current implementation ownership: C4 in #578 must reconcile
the actual annotations, contracts and removal work before terminal acceptance. Do not blindly
retarget or regenerate the historical inventory during this plan-only update to simulate that
semantic audit; verify its existing references remain valid while recording the required work.

The latest approved [modularity/ECS plan](modularity-and-ecs-plan.md) and revised
[`MapRuntime / EntityWorld` ADR](../migration/adr-map-runtime-entity-world.md) select private,
selective `hecs`, preserving cohesive domain aggregates and explicit owners. The next checkpoint
inside #578 is the plan's finite **conformance proof before production storage migration**, not
another indefinite backend selection. Freeze a private host/adapter contract after two independent
modules, then add a third module with a new state type without module-specific host/storage edits.
Exercise equivalent native Rust, Rust Core Wasm and C Core Wasm cases, mixed executors, lifecycle,
composition/conflicts, reentry and bounded failures. The current correctness/measurement status
is recorded at the top of this checkpoint; mock state replay is
not durable DB evidence. Reopen the choice only for a demonstrated backend-specific limitation,
not a generic ABI or implementation bug. No SDK-wide prerequisite or new spike issue is added.

After conformance, retain every C0–C4 obligation and completed Player contract; prove real-owner
lifetime/save, phase/publication and bridge retirement for each affected integration. #583 then
delivers the production external-module proof under #99: shared semantic hooks for native
first-party/custom modules and a bounded Wasm executor with Rust/C bindings, preserving all
stateful composition, durable progress/reward and operator lifecycle acceptance. **This expands
#133's closure requirements:** Wasm is optional for the operator, not optional for #583 acceptance.
The bounded delivery no longer waits for M6. #153 audits both complete macros; it does not inherit
their implementation work. #578 does not depend on #583 or a production SDK/Wasm executor.
Production storage is unchanged by this plan update, reviewed above laboratory HEAD `ee9a0128`;
the production-code inventory remains based on `93e4002a`.

The subsequent [controlled lab](modularity-lab-results.md) is complete on aarch64: 16 storage
and 18 native/Core-Wasm contract cases plus 120 corrected-campaign samples pass pre-registered
lab gates. A first campaign is explicitly superseded after adversarial review, not discarded
for its timings. Its bounded feasibility evidence informs the selection above; native remains the
default execution path. The lab does not close C0–C4 or the new conformance gate, prove independent
arbitrary module state, a C-language guest, save durability, real Map scheduling or external #583
lifecycle. No production dependency or runtime change follows from the architectural selection.

### Earlier synchronization evidence (before the modularity/ECS update)

Updated and read back GitHub bodies #133, #578, #153, #378, #30, #26, #49 and #99;
all issue open/closed states were preserved. The main architecture DAG now includes #578
after its completed #378/#574 inputs and before #153; live-state refresh corrected only
#169/#574 from open to closed. The runtime ledger now resolves #578 through that main DAG,
without duplicate external tracking. Existing field/variant membership and numeric ratchets
are unchanged; the refresh tool also normalizes JSON indentation.

Plan-only validation on aarch64: architecture check/self-test, syntax-only ownership check,
preserved persistence snapshot-policy test, bounded persistence issue-state/classification
check and live `refresh-issue-state --check` PASS. Persistence policy/workflows/snapshot,
production code, the ECS ADR, PR contents and runtime were not changed. This is planning and
metadata consistency evidence, not a new gameplay, exhaustive inventory or final macro pass.

## Historical checkpoint basis

The sections below retain bounded implementation snapshots and validation runs at their named
commits. Earlier “next” investigations and runtime service states are historical, not a second
execution queue or present deployment evidence. Use the approved remaining plan above to choose
work; recheck source/runtime state before relying on an old path, count or installed-build note.

Initial reviewed source: `74daf3f9` plus the active-Player relocation and borrowed grid-capability
slice committed with this checkpoint. The prior runtime family membership was last edited
at `9a29e195`; the prior syntax snapshot was last edited at `26f72455`. Neither described the
current source. The historical persistence snapshot is deliberately unchanged: ordinary
iteration uses `session-ownership-check check --syntax-only`, not an exhaustive persistence scan.

## Exact membership and remaining work

After the 2026-09-05 borrowed TraitNodeEntry dependency slice, the AST has **714 WorldSession fields:
282 production and 432 test fixtures**. The runtime
ledger previously assigned 726 identifiers and classified only 32 as test fixtures. Every
current `cfg(test)` identifier is now assigned exclusively to `test_only_fixtures`; production
members retain their semantic family. This classification does not prove that callers are thin
or that every fixture exercises production behavior.

The removed identifiers are `battle_pet_purchase_store_like_cpp`,
`gameobject_template_lifecycle_store`, `player_grid_load_resolver_like_cpp`, and the six
`represented_pet_{aura_effects,auras,declined_names,spell_charges,spell_cooldowns,spells}_like_cpp`
members. Three identifiers absent from the old runtime ledger are explicitly classified:

| Identifier | Classification and evidence |
|---|---|
| `gameobject_template_lifecycle_store_like_cpp` | Existing production immutable catalog, still installed by `SessionCoreCapabilitiesLikeCpp`; not a new state owner. |
| `object_mgr_catalogs_like_cpp` | Test-only injected catalog fixture. Production borrows the process catalog through dispatch capabilities. |
| `pet_load_query_holder_rows_like_cpp` | Production deferred Pet load staging, not the live Pet. C++ `Pet.cpp:157-203,386-408` defines six query results and resolves the current Player/Pet before applying them. |

The following are still open #578 work, not stable exceptions or work deferred to #153:

- 132 production catalog/configuration/service fields still reside on Session. Required
  construction is not enough: the owning vertical must consume the narrow capability.
- The map/runtime family still has 20 production fields, including both map-manager handles,
  creature scheduling/delivery state and GameObject state. Keep one clock per responsibility;
  remove Session map selection/gameplay and the remaining legacy/canonical bridges incrementally.
- Inventory/loot/economy has 15 remaining production members, spells/progression 15,
  movement/combat seven, social three, and the unresolved residual 18. The exact field lists
  remain executable ledger data; their inclusion does not endorse their current owner.
- Handler and external Session impl bodies still coordinate gameplay. Moving data to Player
  does not itself complete the decode/adapt/encode boundary.
- Public mutable Map access and final runtime-owned grid materialization remain open.
  The generation-checked lifetime coordinator still uses an outer manager mutex, not an actor
  handoff. Full persistence/bridge inventories and live acceptance remain terminal gates.

`SessionResources` has eight required aggregate fields (`core`, `inventory`, `player`, `spells`,
`world`, `progression`, `runtime`, `realm`), rather than 273 flat fields with 216 optional slots.
Their immediate capability types contain respectively 5, 30, 21, 34, 28, 21, 19 and six members:
**164 first-level members, plus further nested handler/persistence bundles**. Glyph,
talent-tab and trait-node-entry catalogs are required members of the process-owned PlayerBootstrap catalog,
borrowed by login/learning/teleport instead of installed on Session. The hotfix
dependency now lives in the nested, process-owned handler capabilities instead of the
Player catalog bundle and is borrowed by its consumers. The constructor
aggregate stays in world-server, not wow-network. Its `install_into_session_like_cpp` methods
still install many catalogs on Session, so eight fields are not evidence of final convergence.

## C++ contrast for this slice

### 2026-09-05 — Borrowed QuestInfo for all three questgiver query registrations

Boundary extraction on `43a81376`: single, visible-multiple and tracked queries
borrow the required process QuestInfo catalog through their existing registrations.
The collector and dialog calculation accept the dependency explicitly. The former
public handler signatures are cfg(test) adapters only; production dispatch never
falls back to the Session catalog. Startup shares the already-loaded immutable Arc,
with no new load, mutable owner, lookup service or per-request catalog clone.

C++ `QuestHandler.cpp:41-78,770-778`, `Player.cpp:16803-16834`,
`QuestDef.cpp:430-445` and `Opcodes.cpp:782-784` anchor lookup, selection and
admission. All three stay LoggedIn; multiple stays ThreadUnsafe, single/tracked
stay Inplace. Packet layouts, single-versus-multiple envelopes, GUID selection,
tracked count validation, relation/eligibility gates and send calls are unchanged.

The new registered-handler test covers three catalog states (empty, important,
covenant) across all three opcodes, deliberately installing conflicting Session
metadata. It checks decoded packet results, no extra packet, admission metadata
and no retained Arc. The first run exposed an incorrect test expectation that
multiple was Inplace; C++ and unchanged production both say ThreadUnsafe, so only
the test expectation was corrected. This test exercises the registry thunk, not
the outer driver's complete admission/lifecycle path.

No Session field is retired yet: the old adapter remains for GameObject quest
activation, and quest-list/gossip presentation still reads the Session field.
Both use the same startup-loaded Arc in production. Threading those consumers
through visibility/runtime and interaction entrypoints, then deleting the field,
setter and install clone together, remains required #578 work.

Remaining dynamic-flags call graph, audited in this worktree:
`update_visible_gameobjects_like_cpp`, `visible_gameobjects_from_canonical_map_like_cpp`,
`gameobject_create_data_from_canonical_like_cpp`, `handlers/loot/claims.rs` and
both visibility assembly paths in `handlers/character/visibility.rs` call
`represented_gameobject_dynamic_flags_for_player_like_cpp`. That helper reaches
the old dialog adapter through GameObject activation. These are real catalog
consumers even though they never name the QuestInfo field directly.

Exact syntax inventory: 282 production + 432 fixture fields, 49 impl owners,
3,671 associated items (+one explicit-catalog calculation and three test-only
handler adapters), 590 registry rows. SessionResources still has eight top-level /
164 immediate capability members; the nested handler bundle adds one required
QuestInfo dependency. Logical LOC: Session 81,844 + 105,504 = 187,348; character
20,622 + 12,811 = 33,433; quest 8,907 + 10,824 = 19,731; world-server
28,884 + 27,021 = 55,905. The classifier counts cfg(test) methods nested inside
production impls in production LOC; syntax inventory distinguishes them exactly.
No hotspot shrink, full catalog retirement or terminal #578/#133 acceptance is claimed.

Validation on aarch64: initial focused status suite 28/0; final full
`wow-world --lib` 3,737/0 (one ignored), including the corrected registered-handler
metadata test. `world-server` check, format/diff, syntax-only ownership and
architecture check/self-test PASS. The final full run supersedes the first
registered-handler test run with the incorrect processing expectation.
Validation-v2 quick PASS: `20260905T073112.646172Z-633673-quick.json`.
No fresh capture or live install/restart is claimed; packet/runtime behavior is
unchanged. Publication and terminal acceptance remain pending.

### 2026-09-05 — Pure quest dialog presentation boundary

Boundary extraction on `e478ac5d`: the private quest `dialog_status` module
classifies borrowed QuestInfo metadata and quest flags without Session, catalog
ownership, SQL, locks or packet publication. Six Session helpers become this
presentation value and one narrow catalog adapter; the important predicate
remains an adapter for quest-list and gossip presentation. Associated items fall
3,672 -> 3,667; all 714 fields (282 production), 49 impl owners and 590 registry
rows remain unchanged. No catalog field or resource-install clone is retired.

C++ `Player.cpp:15706-15784` and `QuestDef.cpp:438-445` anchor the classification.
Important wins over covenant/legendary/daily; future status deliberately has no
covenant branch. The moved branches preserve the old Rust results, missing
metadata fallback, eligibility gates and packet assembly. The separate repeatable
turn-in trivial-marker discrepancy is recorded in EXISTING-CODE-DEFECTS.md and
not repaired in this structural commit.

Two new tests exercise 80 metadata/legendary/daily/hidden-POI/trivial combinations
plus unrelated modifier/type/flag negatives. Existing handler tests still cover
the Session adapter. Quest logical LOC is 8,882 production + 10,752 tests =
19,634 (+25/+132); logical Session is unchanged at 81,838 + 105,503 = 187,341.
This is semantic separation, not a claim that the logical quest hotspot shrank.

Validation on aarch64: focused 2/0; full `wow-world --lib` 3,736/0 with one
ignored; `world-server` check, format/diff, syntax-only ownership, architecture
check/self-test and validation-v2 quick PASS (manifest
`20260905T072307.026594Z-626321-quick.json`). No fresh capture, live install,
restart or publication is claimed; packet layouts/routing and runtime lifecycle
are unchanged by this boundary extraction.

Remaining #578 work: thread the process-owned QuestInfo capability through
questgiver query/visibility, quest-list and gossip paths, then remove the Session
field, setter and installation together. This private boundary adds no crate,
trait, locator, persistent mirror or owner. Full #578/#133 acceptance stays open.

### 2026-09-05 — Catalog routing audit and cinematic characterization

Verdict on `a3b03e65`: small reference counts are not evidence of unused Session
catalogs. No production field is removed by this audit. The remaining 132 catalog
fields, 282 production Session fields and 3,672 associated items are unchanged.
This is diagnostic/test evidence, not an implementation or parity closeout.

Audited consumer boundaries:

| Catalog | Existing consumers and owner defect | Required retirement boundary |
| --- | --- | --- |
| QuestInfo | `handlers/quest/eligibility.rs:406-518` at the audited base drives important/covenant dialog statuses; Session quest-list/gossip builders also call the important predicate. C++ `QuestDef.cpp:438-445` reads global sQuestInfoStore. | Borrow one immutable quest metadata capability through every dialog and quest-list/gossip path; remove field, setter and resource installation together. |
| LFGDungeons DB2 | `represented_championing_faction_for_kill_like_cpp` reads map/difficulty target level. LFG system-info already borrows a different, derived LfgDungeonStoreLikeCpp. | Migrate kill-reputation consumers too; removing the DB2 field merely because LFG handlers use a process catalog would break championing. |
| TraitDefinition | Login trait loading, spell-acquisition adapter, recursive unlearning and base-grant fallback all read it. | One process-owned catalog, borrowed through all four paths; no second locator on Session and no partial login-only retirement. |
| CinematicSequences | `opening_cinematic_like_cpp` and GameObject camera use call `send_represented_cinematic_start_like_cpp`; the only setter callers are tests. | Separate missing-startup-wiring correction from field removal. Both verticals must borrow the same catalog; mutable camera state stays in Player. |

Cinematic evidence: C++ `DB2Stores.cpp:106,681` loads the global catalog;
`Player.cpp:6178-6185` sends TriggerCinematic then calls BeginCinematic when its
entry exists (`CinematicMgr.h:39`). Rust has no startup load/installation and
performs camera-state initialization only when its optional Session field exists.
The new canonical-Player test compares absent/present catalog: both emit one
packet, only the present catalog sets cinematic ID and camera IDs. This proves
that wiring the catalog is a behavioral correction, not a mechanical move.
The verified open defect is recorded in EXISTING-CODE-DEFECTS.md.

Other C++ anchors: `Player.cpp:6412-6422` gates championing on a non-raid
dungeon and the LFG target level; `DB2Stores.cpp:331,906` owns/loads
TraitDefinition and `Player.cpp:2824,3022,3411` uses it during add/remove.

Risk-ranked implementation sequence within #578: (1) close the complete QuestInfo
query vertical with positive/negative dialog and quest-list/gossip evidence; (2) converge shared
TraitDefinition consumption across loading/acquisition/removal, preserving commit
and publication order; (3) handle cinematic bootstrap correction in a distinct
behavior commit with present/missing DB2, opening/GameObject camera, stale/detached
owner and runtime QA evidence. The cinematic finding does not block unrelated
local refactoring, nor does it authorize an unapproved runtime restart.

Keep these APIs in existing private feature modules and wow-data catalogs: no new
crate, trait, task, channel, actor or mutable owner is justified. Every catalog
field/setter/install clone retires when its last production consumer accepts the
explicit narrow dependency; no known consumer is deferred to terminal #153.
Capture checks apply to changed bytes/routing/order; wiring camera state needs
runtime validation, not just a green unit test. Architecture counts remain exact;
logical Session is 81,838 production + 105,503 tests = 187,341 (+34 test lines).
Validation on aarch64: focused characterization 1/0, full `wow-world --lib`
3,734/0 (one ignored), format/diff, syntax-only ownership, architecture check/
self-test and validation-v2 quick PASS (manifest
`20260905T071016.564060Z-613082-quick.json`). The quick run checks workspace
targets; no production code was changed. No install, restart or publication.

### 2026-09-05 — Native known-spell commands

Ownership migration on `236bcba9`: native spell state owns known-ID replacement,
low-level grants and dependent metadata transitions. Replacement prunes dependent,
favorite and trait entries together under one owner, removing the duplicate ID
Vec and a separate trait-pruning owner access. Existing invalidation remains before
the command, account-mount learning remains after it and outside the guard. The
learn/dependent adapters preserve their previous phase/access ordering.

C++ `Player::AddSpell` (`Player.cpp:2741,2812-2819`) and `LearnSpell`
(`Player.cpp:3192-3200`) own PlayerSpellMap transitions. This moves existing reduced
Rust projection rules, not the full AddSpell closure: signed IDs, insertion order,
pre-existing duplicate IDs and unchanged dependent-row dirty state are preserved.
No SQL, packet, publication, source-proof or clock behavior changes. The login
callers remain in `handlers/character/world_entry.rs`; account-mount dependencies
still use the established catalog gate and sorted expansion.

Three old core algorithms remain cfg(test) oracles, sharing the unchanged mount
expansion path. 36 active/detached comparisons cover replace/grant/dependent
commands, complete/partial initial authority and empty/overlapping/duplicate/
signed input, with an account mount retained across replacement. Separate tests
cover stale/missing owners; native tests pin pruning, grant idempotence and the
complete/incomplete row branch of dependent marking. No mutable mirror added.

AST adds three fixture methods (3,672 associated items), no fields or registry
rows: 714 fields = 282 production + 432 fixtures, 590 registrations. Logical
ceilings: Session 81,838 + 105,469 = 187,307; Player 11,077 + 9,906 = 20,983.
Validation on aarch64: focused command tests 2/0; full `wow-world --lib`
3,733/0 (one ignored), including the account-mount fixture; `wow-entities --lib`
715/0. `world-server` check PASS (3m38s, existing warnings). Format/diff,
syntax-only ownership, architecture check/self-test and validation-v2 quick PASS
(manifest `20260905T065948.704121Z-595862-quick.json`).
No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Borrowed native spell queries

Boundary extraction on `c9eef8d6`: nine leaf queries and two row-completeness
predicates stop constructing the full represented spell-runtime snapshot. Ten
are production reads; the loaded-row predicate is test-only. A private spell-only
adapter resolves the existing generation-checked Player and borrows its native
state for one synchronous query. Results copy only the requested collection, or
return a scalar. The now-unused rows_loaded and override_spells fields of the
remaining represented snapshot are test-only, eliminating those production copies.
No state ownership, clock, writer, public API or resource changes.

C++ `Player::GetSpellMap` (`Player.h:1852-1853`) returns Player-owned storage;
`Player::HasSpell` (`Player.cpp:3764-3769`) reads it directly. This cut preserves
the Rust represented known-spell projection rather than replacing it with a new
C++ eligibility rule. Vector order, duplicate/signed IDs, map keys, raw rows,
source-proof gates and existing empty-result adapters remain unchanged. The
resolved owner query returns None on stale/missing ownership and never executes
the callback there. Session-field fallback exists only in cfg(test).

32 active/detached comparisons cover all combinations of loaded/complete row,
trait and override proofs against the former whole-snapshot route. Tests pin
single callback invocation under the owner guard, release after the query,
stale/missing results and replacement isolation. No packets, publication, catalog
lookups, persistence or await occur inside these callbacks. The private adapter
retires with the remaining Session spell-query facade; whole snapshots still used
by acquisition/trait-eligibility paths remain open #578 work, not stable exceptions.

AST: 3,669 associated items (one private production adapter), 714 fields =
282 production + 432 fixtures and 590 registrations unchanged. Logical Session
ceiling: 81,809 + 105,360 = 187,169; other owner ceilings unchanged.
Validation on aarch64: focused query tests 2/0; final full `wow-world --lib`
3,731/0 (one ignored); final `world-server` check PASS (1m54s, existing warnings).
Format/diff, syntax-only ownership, architecture check/self-test and final
validation-v2 quick PASS (manifest `20260905T065332.781659Z-583889-quick.json`).
The final compile and suite include the two removed production snapshot fields.
No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Native spell-save finalization

Ownership migration on `05f3235f`: `PlayerSpellRuntimeState` owns saved-row
retirement/normalization and rebuilding dependent, favorite and known-spell sets.
The former algorithm remains only as a cfg(test) oracle. This route already used
one owner access; this slice transfers its rules, not a new lock or clock cut.
Session retains committed-group admission and registry publication outside the
generation-checked owner. Failed/Unknown transaction branches remain untouched.

C++ `Player::_SaveSpells` (`Player.cpp:20399-20451`) removes Removed rows and
normalizes non-Temporary rows. Rust deliberately retains its established #169
post-confirmed-commit timing rather than C++'s statement-append-time cleanup.
SQL, rollback, unknown-COMMIT fencing, packets and registry publication order
are unchanged. Fallback grants are still cleared only by their separate committed
group; source completeness flags and overrides survive this command unchanged.

64 active/detached comparisons cover all five row states, disabled/dependent/
favorite flags, complete/partial authority and committed/uncommitted spell groups.
Replacement and missing-owner tests prove no stale mutation; native tests pin
temporary state, disabled projections, trait pruning and pending-grant retention.
AST adds one fixture (3,668 associated items), no fields or registrations:
714 fields = 282 production + 432 fixtures, 590 registry rows. Logical ceilings:
Session 81,758 + 105,209 = 186,967; Player 11,042 + 9,869 = 20,911.
Validation on aarch64: focused finalization tests 2/0; full `wow-world --lib`
3,729/0 (one ignored); full `wow-entities --lib` 714/0. `world-server` check
PASS (3m29s, existing warnings); format/diff, syntax-only ownership, architecture
check/self-test and validation-v2 quick PASS (manifest
`20260905T064441.272916Z-568020-quick.json`). No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Native loaded-spell reconciliation

Ownership migration on `5a8c83bd`: `PlayerSpellRuntimeState` now reconciles
pending fallback grants with loaded rows and installs the result under one
generation-checked owner access. The production route no longer clones the whole
spell runtime to read fallback rows before a separate write. No new mirror,
resource, clock, dependency, packet or persistence transaction is introduced.

C++ `Player::LearnSpell` (`Player.cpp:3192-3200`) selects active/favorite from the
existing PlayerSpellMap; `Player.h:175-192` owns persistence state. The existing
Rust fallback reconciliation is retained exactly, including New/Removed/Temporary
transitions and dependent promotion. This does not claim full AddSpell parity.
The login caller remains `handlers/character/world_entry.rs:2366`; input iteration,
positive/unique-ID validation and prior auxiliary invalidation remain outside the
owner. Invalid input still clears row authority without clearing pending grants.

The old route remains an independent cfg(test) oracle. 160 active/detached
comparisons cover all five states, active/disabled/dependent flags and complete
versus partial loads; additional cases cover empty input, duplicate/nonpositive
IDs, stale/missing owners and replacement protection. Native tests pin retained
fallback storage and unrelated state. Known-spell projection and fallback-map
retirement remain separate open work; this slice changes their access, not their
semantics. No gameplay publication occurs under the owner guard.

AST: 3,667 associated items (one new fixture), with 714 fields, 282 production /
432 fixtures and 590 registrations unchanged. Logical ceilings: Session
81,745 + 105,097 = 186,842; Player 11,006 + 9,830 = 20,836.
Validation on aarch64: focused reconciliation tests 2/0; full `wow-world --lib`
3,727/0 (one ignored); full `wow-entities --lib` 713/0. `world-server` check
PASS (2m04s, existing warnings); format/diff, syntax-only ownership, architecture
check/self-test and validation-v2 quick PASS (manifest
`20260905T063859.788549Z-560332-quick.json`). No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Native TraitConfig load lifecycle

Ownership migration on `720819fc`: PlayerSpellRuntimeState owns beginning and
completing the represented TraitConfig source load. Begin clears trait-spell IDs,
headers and all four completeness/empty flags. Complete validates unique positive
config IDs, installs raw header tuples and sets the header/entry proof. Invalid
input resets both source families. Known spells, overrides and unrelated state
remain untouched; valid completion does not discard pre-existing trait-spell IDs.

C++ `Player::_LoadTraits` (`Player.cpp:26635-26698`) owns entry/config construction
and TraitMgr validation. This cut preserves the narrower existing Rust source
proof, not full TraitMgr parity: type/spec/flags remain raw and negative metadata
is not newly rejected. SQL query order and the loader's malformed/failed-query
classification remain unchanged. Input iteration stays outside the owner lock;
the production caller is a pure projection over already loaded config rows.

Session delegates through its existing generation-checked spell-owner helper.
Begin still invalidates aura authority after reset; complete invalidates before
validation and again after an invalid reset, outside the owner lock. An unresolved
owner produces no source transition or publication. The two former algorithms
remain independent cfg(test) oracles. No new resource, task, clock, crate, trait
or mutable mirror; no packet/transaction/commit/retry change.

Forty-eight active/detached comparisons cover begin/no-begin, empty/nonempty
entry proof, valid/duplicate/nonpositive headers and raw metadata, with separate
stale/missing-owner replacement protection. Native tests pin reset, unrelated
state retention and an empty authoritative reload. AST adds two fixture oracles
(3,666 associated items); 714 fields, 282 production/432 fixtures, 590 registrations
and persistence evidence stay unchanged. Logical ceilings: Session
81,721 + 104,937 = 186,658; Player 10,969 + 9,786 = 20,755.

Validation on aarch64: focused lifecycle tests 2/0; full `wow-world --lib`
3,725/0 (one ignored); full `wow-entities --lib` 712/0; `world-server` check
PASS (2m10s, existing warnings). Format/diff checks, syntax-only ownership,
architecture check/self-test and validation-v2 quick PASS (manifest
`20260905T062938.925245Z-547138-quick.json`). No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Native trait-load and override transitions

Ownership migration on `34703ca8`: PlayerSpellRuntimeState now owns complete
trait-ID map validation/replacement and override add/remove rules. Session keeps
the prior aura-authority invalidation before trait loading and the signed-ID early
admission gate for override addition, then invokes native state operations under
the existing owner helper. No runtime state is cloned in the production path.

C++ `Player.h:191` owns PlayerSpell::TraitDefinitionId;
`Player.cpp:28581-28596` adds set members and removes empty override keys.
The port's existing positive-ID/duplicate/completeness rules are preserved,
including clearing the previous trait proof after malformed input and preserving
the override completeness flag. Caller iteration is materialized outside the
owner lock; the sole production trait-load caller passes an already owned Vec.
No callback/I/O is introduced inside the owner. No SQL, packet or timing changes.

Full source search proves complete override replacement and single-trait
assignment now have only fixture consumers, so both become cfg(test) instead of
introducing unused native production APIs. The old complete-trait loader is a
test oracle. Sixteen active/detached differential cases cover empty/valid/
duplicate/nonpositive trait input, unrelated state and override set behavior;
stale/missing owner coverage protects replacements. Tests also assert iterator
evaluation outside the lock. Native tests pin invalid proof clearing, duplicate
override collapse, last-member removal and pre-existing empty-key cleanup.

Fields stay 714 total/282 production/432 fixtures; AST has 3,664 associated items
(one new oracle and two existing methods reclassified). Registrations and
persistence evidence remain unchanged. Logical ceilings: Session
81,694 + 104,829 = 186,523; Player 10,937 + 9,755 = 20,692. No new state,
resource, task, crate, trait or dependency. Catalog/handler convergence remains
open under #578, not deferred as a stable exception to #153.

Validation on aarch64: world library 3,723 passed/zero failed/one ignored;
entities library 711 passed/zero failed. Focused tests, world-server check,
formatting/diff checks, syntax ownership, architecture check/self-test and quick
validation pass. Evidence:
`target/validation-v2/manifests/20260905T062205.758562Z-534761-quick.json`.
No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Prepare and apply acquisition on one Player owner

Ownership migration on `ee78492f`: `PreparedPlayerSpellAcquisitionLikeCpp` is a
single-use domain command value with private fields, not another mutable Player.
Its constructor validates spell rows, trait references, override pairs, keyed
skills, occupied slots and tombstones before Session invalidates aura authority.
One native Player operation then installs spells and skills under the same
generation-checked owner access. The registry publishes afterward, outside the
lock and before acquisition action packets, exactly at the existing boundary.

C++ `Player.cpp:2797-2835` owns AddSpell state/dependency/trait/favorite changes;
`5753-5766` owns SetSkill deletion-state semantics. This cut preserves the port's
validated prepared-result behavior, not full AddSpell/SetSkill parity. Existing
SQL/commit/unknown/retry and callback/action ordering in
`spell_acquisition/application.rs` are untouched. Preparation stays outside the
owner; known/dependent/favorite/removed projections and both state installations
execute inside it without await, SQL or publication. Fallback grants and loaded
TraitConfig evidence remain owned by the same Player and are not overwritten.

The previous two-access spell-then-skill install is now cfg(test); its exact-skill
writeback helper and tombstone predicate also have no remaining production
consumer and become cfg(test). No new Session field, resource, state mirror,
task, trait or crate. The public prepared type is required at the application-to-
domain boundary and is re-exported through the existing entities API; all fields
remain private. Incoming keyed skills are a transitional DTO, not a new owner.

Forty active/detached differential cases compare accepted/rejected inputs, full
spell/skill state, dirty masks and untouched fallback/trait source evidence.
Cases cover duplicate/nonpositive spells, missing/removed/duplicate traits,
invalid and duplicate overrides, malformed skills/keys/slots/tombstones,
disabled/temporary spells and empty authoritative results. Stale/missing-owner
tests protect replacements; native tests pin duplicate input rejection and both
families' final state. Existing full acquisition tests remain in the world suite.

AST adds one fixture oracle (3,663 associated items) and reclassifies two helpers;
714 fields, 282 production/432 fixtures, 590 registrations and persistence
evidence remain unchanged. Logical ceilings: Session 81,686 + 104,721 = 186,407;
Player 10,894 + 9,726 = 20,620. Test oracle lines remain counted by the logical
LOC classifier. The outer acquisition projection/catalog/transaction adapters
and broader handler convergence remain open under #578.

Validation on aarch64: world library 3,721 passed/zero failed/one ignored;
entities library 710 passed/zero failed. Focused owner comparisons, world-server
check, formatting/diff checks, syntax ownership, architecture check/self-test and
quick validation pass. Evidence:
`target/validation-v2/manifests/20260905T061136.455331Z-521137-quick.json`.
No install, restart, capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Player owns represented skill replacement

Ownership migration on `8bb57e11`: the load/mutation replacement adapter now only
converts input rows and calls one native Player command. Structural completeness,
retention of existing non-durable tombstones and addition of deleted rows execute
under that owner. Session no longer reads/clones existing tombstones before a
later owner access. The previous algorithm is cfg(test), including handleless
fixtures. No additional Session field, owner, lock, resource or runtime task.

C++ `Player.cpp:5753-5766` distinguishes deleted persisted skills from cleared
new skills; `_LoadSkills` at `25735-25755` classifies unusable rows while retaining
their slots. Existing Rust validation is preserved, not broadened into full C++
skill parity. A mismatched key or malformed Deleted row makes completeness false
but does not suppress row installation or its existing tombstone behavior. Input
iteration order is retained, including malformed duplicate record IDs. Zero ID
handling remains unchanged. Occupied-slot proof is reset exactly as before.

The command borrows a temporary keyed index of the incoming rows and transfers
the Player's tombstone set into its existing record-install primitive; it creates
no retained mirror. Session's prior aura-authority invalidation and both login
call sites (loaded/default skills), failure outcomes and publication order remain.
No SQL, packet layout, routing, commit/retry or clock changes. The prepared
acquisition-result validation/exact install remains open #578 work.

Differential tests cover 56 active/detached, loaded/complete and row-shape cases,
including key mismatch, duplicate record IDs, malformed deletion, reactivation,
cleared Unchanged/New rows and zero ID. Stale/missing-owner tests protect the
replacement Player; native tests pin tombstone retention and false completeness.
AST adds one fixture oracle (3,662 items); 714 fields, 282 production/432 fixtures,
590 registry rows and persistence evidence are unchanged. Logical ceilings:
Session 81,635 + 104,523 = 186,158; Player 10,758 + 9,648 = 20,406.

Validation on aarch64: world library 3,719 passed/zero failed/one ignored;
entities library 709 passed/zero failed. Focused differential tests, world-server
check, formatting/diff checks, syntax ownership, architecture check/self-test and
quick validation pass. Evidence:
`target/validation-v2/manifests/20260905T060156.372466Z-503845-quick.json`.
No install, restart, fresh capture or publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Player owns skill save and identity finalization

Ownership migration on `293babce`: save completion and identity cleanup mutate
skills directly under one generation-checked Player access. They no longer clone
the whole skill map/tombstones through Session and replace it afterward. Their
previous routes remain separate cfg(test) differential oracles. Save completion
still publishes the registry only after the owner command and outside its lock;
missing/stale owners return before publication. Identity cleanup keeps its old
call position and never creates a Player or modifies a replacement incarnation.

C++ `_SaveSkills` (`Player.cpp:20348-20399`) sets dirty entries unchanged after
appending their SQL. Rust deliberately retains #169's existing confirmed-COMMIT
timing: `save_current_player_to_db_with_generator_like_cpp` invokes completion
only for Applied, and only the `player_skills` committed-group flag selects it.
Failed/Unknown branches, transaction order and retry semantics are untouched.
Skill tombstones belong to the Player lifetime, not the authenticated Session.

The old u16-keyed conversion also removed wider IDs and selected the last
duplicate. A private in-place normalization preserves that behavior explicitly,
rather than silently repairing it in this refactor. The sole record replacement
writer sorts IDs; retain/reverse/dedup/reverse preserves the exact winner/order.
Save marks surviving rows unchanged, retains/adds deleted-slot tombstones and
derives completeness from the previous occupied-slot proof. Identity cleanup
clears tombstones, preserves load/completeness, and discards incomplete slot proof.
No packet, SQL, clock, task, crate, trait, resource or new state mirror is added.

Differential coverage compares 48 save/clear, active/detached, loaded/complete,
occupied-proof combinations with mixed dirty states, distinct duplicates and a
wide ID; it also pins dirty masks and no mutation without a committed skill group.
Separate tests reject stale/missing owners and protect replacement state. Native
coverage checks last-duplicate winner, removed wide ID, tombstones and retained
Vec storage. Full replacement/load semantics remain open #578 work.

AST adds only two fixture oracles (3,661 associated items); 714 fields, 282
production/432 fixtures and 590 registry rows are unchanged, as is persistence
evidence. Logical ceilings: Session 81,610 + 104,397 = 186,007; Player
10,710 + 9,606 = 20,316. Retained impl-level fixture lines remain included in
the LOC classifier, separately from the exact AST test classification.

Validation on aarch64: world library 3,717 passed/zero failed/one ignored;
entities library 708 passed/zero failed. Focused differential tests, world-server
check, formatting/diff checks, syntax ownership, architecture check/self-test and
quick validation pass. A missing type qualification found by the initial compile
was corrected before these successful suites. Quick evidence:
`target/validation-v2/manifests/20260905T055353.579092Z-490649-quick.json`.
No install, restart, fresh capture or remote publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Player owns occupied skill-slot authorization

Ownership migration on `332ef103`: one generation-checked Player command now
validates occupied skill slots and sets/clears their completeness proof. Session
retains the existing prior aura-authority invalidation and returns false for a
missing/stale owner; no lock survives the synchronous call. Its former repeated
read/clone/replace path is a cfg(test) oracle, not a production writeback bridge.
Both production callers (complete skill load and represented skill mutation)
retain their ordering and signatures. No SQL, packet, timer or registry change.

C++ `Player.cpp:5753-5766` retains the SkillLineID slot on deletion;
`Player.h:137` and `UpdateFields.h:428` define 256 slots. `_SaveSkills`
(`Player.cpp:20348-20399`) consumes Player-owned state. Rust's existing distinct
u16-ID projection is preserved, including duplicate collapse and exclusion of
wider IDs; this is not new validation or a claim of full skill-slot parity.
Deleted rows still count, incomplete/incorrect/over-cap requests clear the proof,
and skill records/tombstones are never cloned or republished by the new command.

Differential tests cover 200 active/detached, loaded/complete, empty/deleted,
duplicate/wide-ID and 0/1/2/256/257-slot cases. Stale/missing owner coverage pins
replacement state; native coverage pins invalid-proof clearing, unchanged rows,
tombstones and retained Vec storage. Complete implies loaded at the sole native
writer (`replace_skill_records_like_cpp`), verified by workspace source search.
The separate tombstone-clear helper still normalizes/filter-rebuilds records at
the identity boundary; it remains open rather than silently becoming a plain clear.

Fields remain 714/282 production/432 fixtures; associated items 3,658->3,659
with only the old route added as a test oracle. Persistence evidence and all
590 registry rows are unchanged. Measured logical ceilings: Session
81,582 + 104,269 = 185,851; Player 10,667 + 9,559 = 20,226. The LOC classifier
includes retained impl-level oracle lines; AST fixture classification is exact.
No new crate, trait, resource, state mirror or mutable owner is introduced.

Validation on aarch64: world library 3,715 passed/zero failed/one ignored;
entities library 707 passed/zero failed. Focused owner tests, world-server check,
formatting/diff checks, syntax ownership, architecture check/self-test and quick
validation pass. Evidence:
`target/validation-v2/manifests/20260905T054602.439641Z-481948-quick.json`.
No installation, restart, fresh capture or publication.
#578 and full #133/#153 acceptance remain open.

### 2026-09-05 — Borrow TraitNodeEntry from process-owned bootstrap

Boundary extraction on `3c617aeb`: the required startup-loaded TraitNodeEntry catalog
now lives in PlayerBootstrap and is borrowed narrowly by login and far-teleport
self-create trait loading. Its Session field/default/setter/getter and installed
Player capability member are deleted. No new container, trait, crate, task, lock,
state mirror or retained bootstrap locator is introduced. TraitDefinition still
has other Session consumers and remains explicit open work.

C++ `DB2Stores.cpp:335,910` owns/loads `sTraitNodeEntryStore` process-wide.
`Player.cpp:26635-26658` loads entries before configs;
`TraitMgr.cpp:543-557` checks node membership and rank. This slice preserves the
existing Rust filtering/authority rules, not full `IsValidEntry` parity: malformed
or missing catalog data still cannot authorize trait spells, while represented raw
configuration packet values remain unchanged. No extra SQL, load, packet, connection
or publication reorder is introduced. The teleport admission comment now cites
`MovementHandler.cpp:44-57`; no claim of full teleport parity is made.

Coverage: required borrowed catalog with positive/missing-node/missing-definition
cases for active and detached Player, clearing previous trait authority; stale and
missing owner must not mutate a replacement Player; existing raw-value, query-order,
malformed-row and failed-query regressions remain. Production composition still
fails on the same DB2 load error; empty defaults exist only in explicit test fixtures.

Exact AST delta: one production field and two accessors removed, four consumer
signatures gain a borrowed catalog, and the WorldSession structural fingerprint
changes. There are 3,658 associated items; registry metadata and persistence evidence
remain unchanged. Catalog family 133->132, total/production fields 715/283->714/282;
432 fixtures unchanged. Immediate capability members 165->164; required nested
PlayerBootstrap members 8->9. Logical ceilings: Session 81,569 + 104,171 = 185,740;
character 20,592 + 12,811 = 33,403; world-server 28,883 + 27,021 = 55,904.

Validation on aarch64: world library 3,713 passed/zero failed/one ignored, including
both new owner/catalog regressions; production-login integration three passed.
`cargo check -p world-server`, formatting/diff checks, syntax ownership,
architecture check/self-test and quick validation pass. Quick evidence:
`target/validation-v2/manifests/20260905T053444.315068Z-465679-quick.json`.
No installation, restart, fresh capture or remote publication was performed.
#578 and terminal #153 acceptance remain open.

### 2026-09-05 — Retire unused DungeonEncounter Session dependency

Architecture verdict on `a0804690`: the DungeonEncounter catalog has no Session
consumer. Workspace-wide source search finds only the field/default, setter/getter
and factory assignment, with no getter calls. Remove that dependency rather than
introducing an unused capability wrapper. C++ `DB2Stores.cpp:124,699` and
`DB2Stores.h:98` own/load `sDungeonEncounterStore` globally, not per WorldSession.
Rust's required startup load and error context in `world-server/src/app.rs` remain
unchanged; the DB2 parser/data crate is untouched. This does not implement missing
instance/encounter runtime behavior or claim full C++ parity.

The selected boundary cut deletes the Session field, default, public setter and
getter, the world capability field, install call and cloned factory assignment.
No runtime reader changes, packet/SQL changes, new state, task, module, trait,
crate or replacement service locator is introduced. A source-contract test pins
both retained startup load/error behavior and absent per-session retention; exact
syntax inventories and downstream compilation guard against hidden consumers.

The catalog family falls 134->133 and total/production Session fields fall
716/284->715/283, with 432 fixtures unchanged. Associated items fall 3,662->3,660.
The syntax diff removes only the field/two methods and updates the corresponding
WorldSession structural fingerprint; persistence evidence/counts are unchanged.
Current first-level bundle counts are 5/30/22/34/28/21/19/6 = 165. The ledger's
previous 170 summary and Player=25/Runtime=20 counts were already stale (the code
had 166, Player=22/Runtime=19); correcting those is documentation reconciliation,
not additional implementation credit. World drops 29->28 in this slice.

Logical ceilings: Session 81,577 + 104,010 = 185,587; world-server 28,885 +
27,021 = 55,906. Its production count drops three from live HEAD; the previous
28,896 ceiling contained eight spare lines. Thirteen test lines are added.
The next policy cuts must trace full consumers: configured maximum spans rested
XP/GiveXP/create projection, and quest-XP catalogs span manual/automatic rewards
and LFG reward queries. Neither may be replaced by a new generic Session locator.
Those cuts and the remaining 133 catalog/service fields stay open under #578;
terminal #153 and live acceptance are not satisfied by this retirement.

Validation on aarch64: world library 3,711 passed/zero failed/one ignored;
the world-server retention/startup contract test passes (one selected test).
`cargo check -p world-server`, formatting/diff checks, syntax ownership,
architecture check/self-test and quick validation pass. The only post-suite
source cleanup removed the orphan field comment; formatting, syntax ownership,
architecture and quick validation were repeated afterward. Final quick evidence:
`target/validation-v2/manifests/20260905T051755.514501Z-448346-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Player owns timed online/offline rest accumulation

Ownership migration based on `76755a8c`: native Player commands own elapsed-time
guards, NextLevelXP calculation, timer update and bonus addition under one
generation-checked owner guard. Session still selects borrowed rate inputs and
max-level/RaF policy; the existing 3% RNG gate remains outside the online command
and no random draw or clock is added. The old eligibility/per-second helpers are
test-only, with separate online/offline fixture oracles.

C++ `RestMgr.cpp:141-153,162-174` defines the ten-second timer boundary and
per-second XP formula; `Player.cpp:17892-17901` selects the offline rate. Rust's
#81 zero/future logout-time rejection, online checked subtraction, unchanged
timestamp no-op, configured-maximum guard and float operation order are preserved.
The timer projection still precedes bonus projection. Offline returns computed
extra rather than the capped balance delta, matching the previous Rust API.
Packet and persistence ordering are unchanged; rate/social ownership, RNG and
full Player tick convergence remain open, not deferred to #153.

Native differential coverage checks 48 online/offline, active/detached,
city/wilderness and temporal-boundary cases against the old route, including
state and dirty masks. A separate stale/missing-owner test verifies both commands
leave a replacement Player untouched. Entity coverage pins timestamp rejection,
ten-second boundary, no repeat, capped balance versus returned extra and max-level
timer advancement. No publication occurs inside the commands.

The reviewed syntax delta adds two fixture oracles (3,662 associated items) and
reclassifies two calculation helpers; fields/registrations are unchanged.
Logical ceilings: Session 81,592 + 104,010 = 185,602; Player 10,647 + 9,520 =
20,167. Full RestMgr parity, live acceptance and #578 remain open.

Validation on aarch64: world library 3,711 passed/zero failed/one ignored;
entities library 706 passed/zero failed. Compilation, formatting/diff checks,
syntax ownership, architecture check/self-test and quick validation pass.
Quick evidence:
`target/validation-v2/manifests/20260905T045827.298793Z-428352-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Player owns rest award and consumption

Ownership migration based on `b98903e8`: the native Player command reads the
available rest, caps awarded XP, computes represented percentage loss and calls
its normalization command inside one existing owner guard. Session retains the
empty-victim gate and borrows aura/max-level/RaF policy before the guard; no
policy or aura collection is newly retained. The former percentage and rest-set
Session helpers are now test-only, alongside the previous consumption oracle.

C++ `RestMgr.cpp:125-138` defines award/loss and unconditional normalization,
including zero integer awards. The existing Rust signed-integer percentage and
saturation differ from `Util.h:71-87`'s float term conversion; that discrepancy
is documented in `EXISTING-CODE-DEFECTS.md` and is not changed in this refactor.
Victim admission, GiveXP's mutually exclusive RaF branch, XP mutation, persistence
and LogXPGain/rest-field publication remain in their existing order outside the
guard. Full aura policy ownership and full GiveXP atomicity are not claimed.

Entity tests cover eight percentage/award cases including negative and extreme
modifiers plus zero-award state normalization. Native world tests compare the
old route on active/detached Player, verify empty-victim purity and no publication;
existing stale-owner and GiveXP tests remain active. The reviewed syntax delta
adds one fixture oracle (3,660 associated items) and reclassifies two helpers;
fields/registrations are unchanged. Logical ceilings: Session 81,535 + 103,897 =
185,432; Player 10,588 + 9,477 = 20,065. #578 and live acceptance remain open.

Validation on aarch64: world library 3,709 passed/zero failed/one ignored;
entities library 705 passed/zero failed. Compilation, formatting/diff checks,
syntax ownership, architecture check/self-test and quick validation pass.
Quick evidence:
`target/validation-v2/manifests/20260905T045041.146078Z-412415-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Player owns bonus normalization and addition

Ownership migration based on `132f3943`: native set/add bonus commands now read
Player's NextLevelXP and previous rest state, apply represented normalization and
cap, mutate/project once and return the nested change mask under one existing
owner guard. Addition reads its current bonus inside that guard instead of a
Session snapshot followed by a separate write. Session still obtains max-level
and RaF policy outside the guard; those catalog/social policy boundaries remain
open and this slice does not switch Session's staged level authority to Player.

C++ `RestMgr.cpp:33-80` owns cap/state selection and the combined old/new mask
decision; `RestMgr.h:44-47` defines states 1, 2 and 6. Existing Rust non-finite
input rejection, negative-input clamp, unavailable NextLevelXP guards and config
max-level semantics remain unchanged. The same native rest mutation still
projects flags/RestInfo, including fractional-only mutations whose returned wire
mask stays zero. No packet, SQL, clock or consumption-percentage rule changes.

The previous set algorithm is a test-only oracle and handleless fixture path;
its cap helper and constant are test-only too. A 48-case differential test covers
active/detached Player, zero/nonzero next XP, set/add and negative, fractional,
oversized, infinite and NaN inputs, asserting exact state, RestInfo and dirty
mask equality plus no publication. A domain test pins max-level reset, RaF
priority and fractional no-change masks. Existing stale-owner tests remain active.

The reviewed syntax delta adds one fixture oracle (3,659 associated items) and
reclassifies the cap helper; Session fields and registrations are unchanged.
Logical ceilings: Session 81,507 + 103,856 = 185,363; Player 10,565 + 9,446 =
20,011. Fixtures remain explicit debt, not production owners. Broader rested-XP
runtime parity and #578 acceptance remain open.

Validation on aarch64: world library 3,708 passed/zero failed/one ignored;
entities library 704 passed/zero failed. Compilation, formatting/diff checks,
syntax ownership, architecture check/self-test and quick validation pass.
Quick evidence:
`target/validation-v2/manifests/20260905T044325.277207Z-400861-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Player rest state owns flag transition rules

Ownership migration based on `986665be`: the Session set/remove flag adapters
now delegate to `PlayerRestState::set_flag_like_cpp` / `remove_flag_like_cpp`
inside the existing native mutation guard. The domain state owns mask changes,
first/last transition, rest time, trigger and deferred-publication bookkeeping.
The injected clock is called only on the first nonempty transition, at the same
point within the guard as before; no clock, state copy or dependency is added.

C++ `RestMgr.cpp:95-122` defines first/last rest transitions and `RestMgr.h:53-55`
defines tavern/city/faction masks. The represented Rust location-initialization,
tavern-trigger cleanup and deferred-publication rules are moved unchanged; in
particular Rust clears an absent tavern trigger while C++ RemoveRestFlag does not
clear that field. This refactor preserves that existing difference rather than
silently changing gameplay. All Session callers retain their order, C++ area/zone
gates, projection through Player and packet sends outside the guard.

Two new entity tests cover empty, repeated and overlapping flags, a changed
tavern trigger without a new start time, last removal, repeated removal, deferred
dirty bookkeeping and unrelated state preservation. Existing world tests cover
active/detached/stale native ownership and area/tavern packet behavior. Exact
Session syntax policy passes unchanged: no new fields, methods or registrations.
Reviewed logical ceilings: Session 81,485 + 103,805 = 185,290 (32 production
lines removed); Player 10,517 + 9,429 = 19,946 (49 production + 58 test lines
added). Full RestMgr parity and #578 acceptance remain open.

Validation on aarch64: world library 3,707 passed/zero failed/one ignored;
entities library 703 passed/zero failed. Compilation, formatting/diff checks,
syntax ownership, architecture check/self-test and quick validation pass.
Quick evidence:
`target/validation-v2/manifests/20260905T043723.122118Z-390134-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Rest load is one native Player command

Ownership migration based on `e663fcde`: production rest load now performs one
generation-checked Player access. `Player::load_xp_rest_bonus_like_cpp` resets
the six transient location fields and loads bonus/state using the existing native
mutation/field projection. It replaces the separate flag query, cloned reset and
bonus mutation; the old reset helper remains only for handleless test fixtures.

C++ anchors: `Player.cpp:348` constructs its RestMgr, `RestMgr.cpp:26-30`
initializes the location mask/time/trigger, and `Player.cpp:17693` loads the
persisted XP rest values before subsequent progression initialization. The Rust
login caller (`handlers/character/world_entry.rs`) still applies offline rest
immediately after this command. Loaded Player flags stay unchanged, including
the resting bit, until later location initialization. The old set/remove calls
re-applied that same loaded bit; they were not new location decisions. Invalid
persisted state normalization remains in the existing adapter, and bonus clamping
for RestInfo, unrelated honor/XP/logout fields and packet/SQL ordering are unchanged.

The new regression exercises 20 active/detached, loaded resting/nonresting and
valid/invalid persisted-state combinations. It verifies exact reset fields,
preservation of all unrelated rest state and Player flags, projected RestInfo,
and no publication. Stale/missing load rejection supplements existing replacement
owner protection. This does not complete rate/catalog ownership or broader RestMgr
runtime parity; live save/teleport acceptance and #578 remain open.

The syntax delta changes only the old reset helper to a test fixture; Session
fields and associated item identities stay unchanged. Reviewed logical ceilings:
Session 81,517 + 103,805 = 185,322; Player 10,468 + 9,371 = 19,839.
The method-level fixture branch accounts for Session's eight production-classified
lines; the 17 new Player lines hold the command, with no new state or dependency.

Validation on aarch64: world library 3,707 passed/zero failed/one ignored;
entities library 701 passed/zero failed. Compilation, formatting/diff checks,
syntax ownership, architecture check/self-test and quick validation pass.
Quick evidence:
`target/validation-v2/manifests/20260905T043145.249517Z-379684-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Rest mutation runs on the canonical Player

Ownership migration based on `44c445d0`: Session's rest mutation helper now
resolves its generation-checked Player once and delegates to
`Player::mutate_rest_state_like_cpp`. The native state is modified directly,
then existing RestInfo and Player-flag setters project it under that same guard.
The old Session rest replacement helper is now exclusively a test fixture and
differential oracle. No production rest snapshot/writeback remains in this helper.

C++ `RestMgr.cpp:65-80,95-122` owns rest values and their Player field updates.
All eleven Rust mutation call sites were inspected: callbacks synchronously
modify rest fields, optionally read the existing game clock, and neither re-enter
the owner, await, persist nor send packets. Existing threshold clamping, rest
state, initialization-gated flag normalization, unrelated flags and dirty masks
are preserved. Packets remain outside the guard; no SQL or tick owner changes.
This does not complete rate/catalog ownership, the load-reset helper or full
RestMgr behavior. Handleless fixtures retain their prior route; native stale and
missing owners do not execute the callback.

The new active/detached differential test compares the old projection with the
native command for initialized/uninitialized and empty/nonempty masks, checks
exact state, flags, RestInfo and active-data dirty masks, and proves one callback
under the existing lock with no publication. Negative coverage rejects stale and
missing owners even with populated Session fixtures. Full library suites pass:
3,706 world tests (one ignored) and 701 entity tests, zero failures, on aarch64.
The reviewed syntax delta changes only the replacement helper's classification
to `cfg(test)`; field counts and 3,658 associated item identities are unchanged.
Logical ceilings: Session 81,509 + 103,727 = 185,236; Player 10,451 + 9,371 =
19,822. The LOC classifier retains method-level fixture code in its production
count; these ceilings do not imply additional production gameplay authority.
Live acceptance remains pending and #578 remains open.

Compilation, formatting/diff checks, syntax ownership, architecture check/self-test
and quick validation pass. Quick evidence:
`target/validation-v2/manifests/20260905T042603.602733Z-369190-quick.json`.
No installation, restart, fresh capture or remote publication was performed.

### 2026-09-05 — Rest visibility and saved flags read one Player owner

Ownership read convergence based on `318bcdab`: each of
`resolved_visible_resting_like_cpp` and
`resolved_player_flags_for_rest_state_save_like_cpp` now borrows rest state and
Player flags together under one existing generation-checked owner guard. The
production queries no longer clone rest state or resolve the same Player twice.
No new Player copy, mutex, catalog, public API or Session field is introduced.

C++ `RestMgr.cpp:99-125` maintains the mask and resting flag on its owning
Player. Rust's existing load boundary is preserved: before location initialization,
use loaded flags; afterward derive the resting bit from the rest mask while
preserving unrelated flags. This is not a repair of the remaining RestMgr update
or clone/writeback mutation paths, nor a claim of full C++ rest parity. Save
transaction ordering, create/save consumers, publication and clocks are unchanged.
Handleless fallback remains test-only; stale or missing native owners return None.

Two new tests cover 24 active/detached combinations of initialized/uninitialized
state, empty/city/tavern mask and loaded flag, verify no state/flag mutation or
packet emission, and reject stale/missing owners despite populated fixtures.
The aarch64 world library suite passes 3,705 tests (zero failures, one ignored).
The exact syntax ownership policy passes unchanged. The logical Session ceiling
is reviewed at 81,503 production + 103,657 test lines = 185,160: eight production
lines for grouped queries/fixture branches and 94 test lines including registration.
Live save/teleport acceptance remains pending; #578 stays open.

Compilation (`cargo check -p wow-world`), formatting, diff check, architecture
check/self-test and quick validation pass. Quick evidence:
`target/validation-v2/manifests/20260905T042011.533248Z-358486-quick.json`.
No fresh capture, installation, restart or remote publication was performed.

### 2026-09-05 — Difficulty preferences mutate and save as one owner group

Ownership migration based on `39fb9f3a`: difficulty mutation now borrows the
three native Player fields under one generation-checked owner access rather
than mutating a copied tuple and writing all three fields back in a second
access. Full save reads the preference tuple once instead of resolving three
individual values separately. Full replacement remains for explicit login/group
hydration, where the caller intentionally supplies all three authoritative values.

C++ `Player.h:1965-1967` owns the setters on Player and
`Player.cpp:19488-19511` saves the three preferences from that same Player.
All Rust mutation callbacks were audited: they only update selected difficulty
fields synchronously, with no owner re-entry, await, SQL or publication. Group
membership/instance-entry checks and packet publication remain outside the
mutation in their original order. This does not move group authority or a clock.

Two new tests prove one callback under the owner guard for active/detached
Player, preservation of unselected preferences, exact save-header values,
released guard and no packet emission, plus stale/missing-owner rejection
without touching replacement preferences. No fields, public API, crate edges,
or full-save/teleport acceptance gates are retired by this bounded cut.

On aarch64, `wow-world --lib` passes 3,703/0 with one ignored and syntax-only
ownership passes without any baseline change: 284 production + 432 fixture
fields, 3,658 associated items and 590 registrations. Formatting and diff checks
pass. The reviewed logical measure is 81,495 production + 103,563 test lines;
growth is the explicit single-owner path and the two focused owner tests.
Architecture check/self-test and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T041129.470223Z-347557-quick.json`.
No capture, restart, push or terminal acceptance is claimed. The prior request
for guarded live save/teleport QA authorization remains unanswered.

### 2026-09-05 — Retire save-snapshot writeback into gameplay

Verdict: the save snapshot is persistence input, not a command to mutate Player.
This is an explicit behavior correction within #578's bridge-retirement scope,
based on `720b2519`, separate from the preceding behavior-preserving read cut.
The canonical Player remains the gameplay owner; the existing private lifecycle
adapter consumes its snapshot through the existing persistence port. No new
crate, trait, runtime task or mutable mirror is needed.

C++ `Player.cpp:19323-19348,19470-19514,19548-19565,19615-19692` selects persisted
fields/destination and appends save groups; it does not relocate Player or replay
level, XP, money and health through setters. On pre-fix Rust, a recording-port
regression proved that a failed full save moved a near-teleporting Player from
(1,2,3) to (11,22,33). The same bridge also replayed Session's staged level and
recalculated talent points before persistence. These are writes by a non-owner
save adapter, not required C++ save side effects.

`sync_session_from_save_to_db_snapshot_like_cpp` is deleted, including its test
call sites; there is no compatibility alias. Full save and tests call the pure
query. The character header consumes captured level/XP/money directly, alongside
captured position/health/powers, instead of rereading them after a writeback.
Admission fences, pending durable-work reconciliation, exclusive money locking,
transaction requests and Applied/Failed/Unknown classification are unchanged.
Post-commit dirty-group cleanup remains the only existing save publication path.

Risk-ranked continuation: verify the removed pre-transaction writes on every
commit outcome; then converge remaining save-group reads and staged identity
inputs without introducing a new snapshot owner. The full-save regression checks
unchanged live position, level and talent points for Applied/Failed/Unknown while
the request retains its save-only destination. A separate test proves header
capture does not reread or overwrite subsequently changed runtime values.
Full persistence inventory and live save/near-teleport QA remain acceptance gates;
this correction is not presented as capture-clean or manual-test-ready without
that evidence. Runtime interruption still requires explicit authorization.

On aarch64, the pre-fix full-save regression fails at the unintended relocation;
after correction `wow-world --lib` passes 3,701/0 with one ignored. Syntax-only
ownership and architecture check/self-test pass. The exact syntax delta removes
the writeback method only, leaving 3,658 associated items; fields, registrations
and the other bridge inventory rows are unchanged. Session measures 81,480
production + 103,475 test lines (18 production lines removed; 119 test lines
added). These local checks do not replace the pending live acceptance above.
Formatting, diff checks and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T040406.941418Z-338179-quick.json`.
No runtime install, capture, push or terminal acceptance is claimed.

### 2026-09-05 — Save projection reads one generation-checked Player owner

Ownership migration based on `b813d262`: the production save projection resolves
one PlayerHandle and residence under one manager guard. Powers, XP, money,
position, health and near-teleport state are read from that same Player. The
previous repeated owner reads followed by a GUID scan of every map are gone
from production; the former algorithm is a cfg(test)-only compatibility fixture
and differential oracle. Missing/stale handles and GUID mismatch fail closed.

C++ `Player.cpp:19323-19337,19480-19514,19557` defines the owner and save fields.
The move preserves current Session level/map staging, far-before-near destination
priority, destination instance zero, detached instance zero and residence-specific
health normalization. Their boundaries and the retained snapshot writeback are
documented in `EXISTING-CODE-DEFECTS.md`: this slice does not silently remove
recalculation/relocation side effects or claim full Player::SaveToDB parity.
The existing far-teleport save scheduling gate is unchanged. No SQL request,
transaction, publication, packet metadata or runtime clock is modified.

Four tests cover exact active/detached values, unchanged state/update fields,
pending-destination precedence without relocation, represented dead-health
projection, missing manager and stale replacement rejection. The same native
fixtures are compared against the previous projection as well as explicit values.

On aarch64, focused save-snapshot tests pass 13/0; `wow-world --lib` passes
3,699/0 with one ignored. Syntax-only ownership and architecture check/self-test
pass. The reviewed syntax delta adds only the private test fixture helper;
3,659 total associated items, 284 production + 432 fixture fields, 590 opcode
registrations and the bridge inventory are unchanged otherwise. The logical
classifier measures 81,498 production + 103,356 test lines, including the retained
cfg(test) oracle. This is read-path convergence, not retirement of the enclosing
save writeback or the full persistence snapshot inventory.
Formatting, diff checks and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T035454.402735Z-324754-quick.json`.
No fresh capture, live runtime action, push or terminal acceptance is claimed.

### 2026-09-05 — Talent reset pricing belongs to canonical Player talent state

Ownership migration based on `95cb0a34`: `PlayerTalentRuntimeState` now owns the
represented `GetNextResetTalentsCost` rule in Player's private progression module.
Session resolves the generation-checked owner once and borrows its reset history;
the former two whole talent snapshots and the Session pricing helper/constants
are gone. Individual reset cost/time getters also read scalars directly under
the owner instead of cloning talent/glyph groups. Only handle-less tests retain
their fixture path; stale/missing owners remain `None`.

C++ `Player.cpp:3472-3503` reads both reset-history fields from the same Player;
`Common.h:33` defines the 30-day month and `SharedDefines.h:259-264` the gold unit.
The rule keeps the existing first-use steps, monthly decay floor and cap. Rust's
saturating/narrowing differences for abnormal persisted values are explicitly
recorded in `EXISTING-CODE-DEFECTS.md`, preserved rather than silently corrected.
No persistence request, money guard, publication order, opcode or runtime clock
changes. The broader reset adapter and other Session catalogs remain #578 debt.

A native entity test covers fourteen fee/time boundaries and no state mutation.
Two Session tests cover active and detached state, unchanged talent groups,
scalar reads, guard release, missing manager and stale-generation rejection
without querying or mutating the replacement Player through the old session.

On aarch64, `wow-entities --lib` passes 701/0 and `wow-world --lib` passes
3,695/0 with one ignored. Syntax-only ownership and architecture check/self-test
pass. Reviewed syntax removes only the old Session pricing helper: 3,658
associated items; fields, registrations and bridges are unchanged. Session
measures 81,427 production + 103,196 test lines; Player measures 10,424 production
+ 9,371 test lines. The growth is the moved rule and the three owner/boundary tests.
Formatting, diff checks and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T034508.665422Z-311385-quick.json`.
No fresh capture, runtime install, push or terminal acceptance is claimed.

### 2026-09-05 — Talent reset borrows process-owned cost policy

Ownership migration based on `7de52e09`: `CONFIG_NO_RESET_TALENT_COST` is built
once in the required Progression capability. The ConfirmRespecWipe registration
passes only that bool, together with its existing item generator, through the
transaction adapter. Session's field, initializer and setter and the old runtime
resource installation are removed without a test-only mirror.

C++ `Player.cpp:3505-3524` (`ResetTalents`) reads the World configuration before
the money check. Its explicit `noCost` parameter is a separate policy: the
script-hook argument remains false and login's free-reset path is unchanged.
Rust retains cost planning before its exclusive money guard, the same persistence
request and Applied/Failed/Unknown handling, and publication only after COMMIT.
This move does not claim full C++ reset semantics or change any packet metadata.

The free-reset test now invokes the actual registry thunk with explicit process
policy, retaining byte-exact output/criteria assertions and checking metadata and
no retained Arc. A new recording-port test alternates free and paid policy on the
same session: each request has the expected cost/money, while runtime publication
remains separate. Existing rollback and unknown-COMMIT tests pass the paid policy
explicitly. Remaining catalogs and talent gameplay adapters stay open #578 work.

On aarch64, `wow-world --lib` passes 3,693/0 with one ignored and the production-
linked login regression passes 3/0. Syntax-only ownership and architecture
check/self-test pass. Reviewed syntax removes one field/setter, changes three
signatures and the Session fingerprint only: 284 production + 432 fixtures,
3,659 associated items, 590 registrations. The removed config was ledgered in
spells/progression (16 -> 15), not the separate 134-member catalog family.
Session measures 81,436 production + 103,118 test lines; world-server shrinks
three production lines. No packet, transaction or clock contract changes.
Formatting, diff checks and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T033628.948629Z-298213-quick.json`.
No fresh capture, runtime install, push or terminal acceptance is claimed.

### 2026-09-05 — Player owns the represented talent-point operation

Ownership migration based on `6e28ab96`: the count/reward/bounds/update operation
now lives in Player's existing private progression module, exposed as a domain
method to the adapter crate. Session supplies the unchanged level-derived base
and an immutable talent-validity predicate; it no longer counts talents or
chooses CharacterPoints in production. The standalone Session point setter has
no remaining production caller and is now test-only. No new stored catalog,
state copy, dependency, clock or persistence path is introduced.

The same C++ anchors and represented-policy boundaries as the preceding slice
apply. Two entity tests prove active-group-only predicate visits, unchanged
talent/reward state, exact update-mask equivalence with the existing setter,
no dirty field on an unchanged result, invalid-group handling, saturation and
signed-field clamping. The existing three canonical Session tests still cover
active/detached/stale/missing ownership around the domain operation.
The catalog and Session identity inputs remain explicit #578 debt; this is not
full InitTalentForLevel or issue completion.

On aarch64, `wow-entities --lib` passes 700/0 and `wow-world --lib` passes
3,692/0 with one ignored. Syntax-only ownership and architecture check/self-test
pass; the only syntax delta is the obsolete Session setter's test-only cfg.
The logical Session measure shrinks to 81,439 production + 103,093 test lines;
Player grows to 10,396 production + 9,336 test lines, reviewed for this operation
and its two tests. Fields, registrations, bridges and crate edges are unchanged.
Formatting, diff checks and `validation-v2 quick` pass (exit 0), manifest
`target/validation-v2/manifests/20260905T033008.525971Z-289045-quick.json`.
No fresh capture or live runtime action is required by this behavior-preserving
operation move; no push or terminal #578/#153 acceptance is claimed.

### 2026-09-05 — Talent points refresh uses one canonical Player access

Ownership migration based on `1b9731d2`: refresh now borrows the active talent
group, reads quest-awarded points and writes CharacterPoints under one
generation-checked Player mutation. It no longer clones the talent runtime or
re-resolves the owner between counting, reading rewards and publishing the update
field. The immutable talent/spell validation path cannot re-enter the owner;
no packet, SQL, await or runtime task runs under this guard. Existing handle-less
fixture helpers are test-only, and a stale/missing owner cannot use them.

C++ `Player.cpp:26356-26359` (`CalculateTalentsPoints`) and `28670-28679`
(`GetSpentTalentPointsCount`) put both inputs on Player; `2344-2362`
(`InitTalentForLevel`) writes CharacterPoints there. This slice preserves Rust's
represented catalog-validity filter, Session level/class inputs, absent-level-
catalog base zero, saturating subtraction and signed-field clamp. It does not
claim C++'s full removed-talent, reset, permission, tier or publication behavior.
Level/class ownership, catalog retirement and the enclosing gameplay adapters
remain #578 work. No Session field or bridge is retired by this slice.

Three new canonical-owner tests cover active/detached refresh, active-group
selection, invalid talent/spell exclusion, quest rewards, absent level catalog,
saturation/clamping, no packet emission, guard release, and stale-generation or
missing-manager rejection without touching the replacement Player.

On aarch64, the focused tests pass 3/3 and `wow-world --lib` passes 3,692 with
zero failures and one ignored. Syntax-only ownership, architecture check and
self-test, formatting and diff checks pass. The reviewed syntax delta classifies
three private helpers as test-only; fields, total associated items, registrations
and bridge inventory are unchanged. The logical Session measure is 81,452
production + 103,093 test lines. `validation-v2 quick` passes (exit 0), manifest
`target/validation-v2/manifests/20260905T032537.552346Z-283211-quick.json`.
No packet layout, metadata, connection or observable publication order changed;
no fresh capture, live runtime restart, push or terminal acceptance is claimed.

### 2026-09-05 — Talent login and learning borrow the same required tab catalog

Ownership migration based on `194f9d1b`: world-server bootstrap installs one shared
`TalentTabStore` in `PlayerBootstrapCatalogsLikeCpp`. Login and the LearnTalent
registration pass that exact catalog by reference through the existing load/learn
adapters. The unregistered LearnTalents adapter also requires it. Session's field,
getter and setter are removed with no test-only mirror or fallback construction.
Test setup helpers return explicit catalogs, retaining missing-tab and wrong-class
fixtures rather than silently replacing them with a universal valid catalog.

C++ `Player.cpp:26036-26058` (`LearnTalent`) resolves `sTalentTabStore` and its class
mask; `SkillHandler.cpp:29-33` publishes talents only after successful learning.
`Player.cpp:26623-26633` (`_LoadTalents`) and `2644-2692` (`AddTalent`) define the
loading/active-group spell side effects. Rust's additional tab/class gate during
login remains unchanged and is recorded in `EXISTING-CODE-DEFECTS.md`; this slice
does not approve it as full C++ parity. Point, prerequisite, rank, tier, spell,
override, aura-interruption, persistence and publication order stay unchanged.

New coverage invokes the actual registered thunk with empty then populated process
catalogs, checks admission metadata and byte-exact successful output, and proves no
extra retained Arc. A canonical-owner test covers active/detached row validation,
failed-load state preservation and missing-owner rejection. Existing talent and
respec tests keep their assertions while taking the catalog returned by fixtures.

Reviewed syntax delta: one field and two methods removed, five signatures gain a
required borrowed parameter, and the Session struct fingerprint shrinks; no opcode
or bridge row is retired. Totals: 285 production + 432 fixtures, 3,660 associated
items and 590 registrations. Session production shrinks eight lines; Session tests
grow 41, and character login grows one line for the explicit reference.
On aarch64, `wow-world --lib` passes 3,689 / zero failures / one ignored, and
syntax-only ownership plus architecture check/self-test pass. The production-linked
initial-login regression passes three tests (not full login/live acceptance).
Formatting, diff checks and `validation-v2 quick` pass (exit 0), including workspace
all-target and isolated bot checks; manifest
`target/validation-v2/manifests/20260905T031117.251391Z-260323-quick.json`.
No fresh capture, push or terminal acceptance is claimed.
No new dependency, clock, mutable state or runtime install
is introduced; gameplay adapters, remaining catalogs and terminal #578/#153 gates
are still open work, not stable exceptions.

### 2026-09-05 — Login borrows the process glyph catalog

Ownership migration based on `b4d407b9`: `PlayerBootstrapCatalogsLikeCpp` requires
the process-wide `GlyphPropertiesStore`, populated by world-server bootstrap.
`world_entry` passes that reference directly to its glyph-row adapter. The Session
field, getter and setter are deleted, including test storage; the corresponding
SessionPlayerCatalog capability field and installation call are gone. No new Cargo
edge, state mirror, clock, query, ordering or packet change is introduced.

C++ `Player.cpp:26573-26598` reads `sGlyphPropertiesStore` during `_LoadGlyphs`;
`Player.cpp:25477-25481` applies `SetGlyph`. The represented zero-ID clearing and
row-selected group differ from those C++ paths and are preserved, not approved as
parity; exact discrepancies are recorded in `EXISTING-CODE-DEFECTS.md`.
The adapter requires `&GlyphPropertiesStore`; its sole production caller borrows
the required bootstrap member. Legacy unit fixtures now supply explicit valid
catalog rows instead of relying on missing-catalog acceptance. There is no
absent-catalog construction or fallback Session lookup in this path.

Existing invalid group/slot/ID and talent-packet assertions are retained with explicit
test inputs. A new active/detached canonical-owner test varies the supplied catalog,
proves missing-ID rejection preserves prior state, retains represented zero clearing,
checks that Session retains no additional Arc and rejects a missing owner. The real
production-login integration fixture now supplies the required catalog explicitly.

Reviewed inventory delta: one production field and two methods removed; the load
adapter gains a borrowed argument and the struct surface fingerprint shrinks.
Totals are 286 production + 432 fixtures, 3,662 Session associated items and 590
registrations. Session production shrinks 10 lines; its test footprint grows 52.
Character login grows one line for the explicit reference. On aarch64, the final
required-catalog source passes `wow-world --lib` (3,687 / zero failures / one ignored)
and `production_login_player_owner` (three / zero failures). That integration
fixture stops at initial hydration/map selection; it does not prove full login or
live glyph persistence. Syntax-only ownership, architecture check/self-test and
diff checks pass. The final `validation-v2 quick` passes (exit 0), including
workspace all-target and isolated bot checks; manifest
`target/validation-v2/manifests/20260905T030336.592145Z-253993-quick.json`.
No fresh capture, runtime install/restart, push or terminal acceptance is claimed.
This does not close #578: the remaining catalogs and gameplay orchestration still
need to leave Session, and the glyph mutation adapter itself remains transitional.

### 2026-09-05 — Cast readiness and interruption are Unit-domain transitions

Boundary extraction based on `3ddf51d5`: `CastExecutionStateLikeCpp` now implements
retained-cast interruption, remaining cast/global-cooldown queries and ready-cast
consumption. Session's generation-checked adapters invoke those transitions; they
no longer implement the readiness condition or matching-cast cancellation rule.
The ready outcome owns the consumed cast and its late-power-failure rollback
metadata, so effect execution and packet publication remain outside the owner guard.

C++ anchors remain `Unit.cpp:3008-3035` (`InterruptSpell`), `Spell.cpp:4235-4252`
(`SPELL_STATE_PREPARING`) and `Player.cpp:29109-29120` (`CanRequestSpellCast`).
This preserves the existing represented Instant samples, cast-time comparison,
per-spell timestamp retention and queue cancellation ordering. No opcode, packet,
SQL, dependency, mutable owner, clock or Session signature changes. The domain API
accepts only values and returns an owned cast/boolean/duration; no packet, pool,
channel, guard or application context crosses into entities.

Four domain tests cover the exact readiness boundary, no mutation before readiness,
one-shot consumption, rollback metadata and full payload retention, zero-time casts,
matching/nonmatching/wildcard cancellation, and absent/expired timing queries.
Existing canonical active/detached/stale-owner and packet-facing spell tests remain
the adapter regression coverage. On aarch64, `wow-world --lib` passes 3,686 / zero
failures / one ignored and `wow-entities --lib` passes 698 / zero failures.
Syntax-only ownership passes with the unchanged exact baseline; architecture check
and self-test pass after tightening Session's production ceiling from 81,455 to
81,429 lines (102,893 test lines, 184,322 total). Formatting, diff checks and
`validation-v2 quick` pass (exit 0), including workspace all-target and isolated
bot checks; manifest
`target/validation-v2/manifests/20260905T024721.763116Z-226759-quick.json`.
No fresh capture, live install/restart, push or terminal acceptance is claimed.

No additional Session field is retired in this cut (287 production, 432 fixtures).
The remaining current-spell-reference policy, execution scheduler, other cast writes
and full SpellHistory convergence are still open. Initial catalog inspection also
confirmed the four runtime-script authority sets feed Player spell-hit/aura safety
through `spell_has_no_unrepresented_runtime_hooks_like_cpp`; their removal must
carry those consumers, not merely rename the Session service-locator fields.

### 2026-09-05 — Unit owns active cast execution and represented timestamps

Ownership migration based on `09ffc929`: canonical Unit's `SpellSubsystem::execution`
owns the retained active cast and the two represented last-cast timestamp stores.
Their former Session fields compile only for handle-less test fixtures; no production
mirror or whole-substate write-back remains. Packet handlers and existing Session
adapters resolve the generation-checked Player, then access its Unit synchronously.
No new Cargo edge, lock, task, clock or persistence field is introduced.

C++ anchors: `Unit.cpp:2932` (`SetCurrentCastSpell`) and `3008-3035`
(`InterruptSpell`), `Unit.h:1823` (`m_currentSpells`), `Spell.h:554,592-602,899`
(retained cast values and timer) and `Spell.cpp:4235-4252` (ready cast execution).
The existing Rust Instant-based represented policy is retained, not replaced with
C++ diff timers in this structural cut. Global/per-spell cooldown rules, inclusive
400ms queue admission and late-power-failure timestamp restoration remain unchanged.

Normal/ toy start, cancel, looting/teleport/stand/channel interruption, readiness,
cooldown queries and writes, and CastUnstuck's hearthstone timestamp use the owner.
Ready execution takes the cast and changes its timestamp under one owner access;
effects and publication happen after releasing the guard. Cancellation tests and
fixtures now use the same owner adapters. Missing ownership returns `None` separately
from a valid zero cooldown; the boolean cooldown gate fails closed. No packet layout,
recipient, connection, SQL or represented publication ordering is changed.

The retained execution record and existing `current_spells` references now share
Unit ownership, but their policies are not yet fully converged. This does not move
the Session driver into MapRuntime or make handlers fully decode/adapt/encode. Those
remain #578 work, along with the remaining catalogs/application state. The private/
crate-local access bridges are transitional adapters, not a new stable feature API;
retire them as cast execution moves behind the owning vertical/runtime outcomes.

Focused tests prove active/detached owner access once, released guards, timestamp
preservation across interruption, no fixture mirroring, readiness consumption once,
and no completion/cancellation/replacement/publication by a stale or missing owner.
Validation on aarch64: `wow-world --lib` passes 3,686 / zero failures / one ignored;
`wow-entities --lib` passes 694 / zero failures. Syntax-only ownership and architecture
check/self-test pass. The reviewed syntax delta moves three fields to test fixtures,
adds six access methods, and makes two timing queries explicitly optional; totals
are 287 production + 432 fixtures, 3,664 associated items and 590 registrations.
The first quick run detected formatting drift; `cargo fmt --all` corrected it.
The aggregate quick rerun passes (exit 0), including workspace all-target checks
and the isolated bot check: manifest
`target/validation-v2/manifests/20260905T023820.470904Z-218906-quick.json`.
Formatting and diff checks pass. No fresh capture, live install/restart, push or
terminal #578/#153 acceptance is claimed.

### 2026-09-05 — Player owns the pending cast request

Ownership migration based on `5e925671`: `PlayerGameplayState::pending_spell_cast`
holds the queued request. The former Session field is `cfg(test)` only, used solely
when no PlayerHandle is installed. Private query/mutation bridges resolve the
current generation for active and detached Players; unknown ownership is `None`,
distinct from a valid owner with an empty queue. No new mirror is synchronized.

C++ `Player.cpp:29078-29106` owns request replacement and cancellation;
`29109-29127` defines the 400ms admission window and begins pending execution.
Rust retains its represented deferred tick, cooldown/active-cast gates, validation,
cancel-before-replace publication, and removal-before-execution order. The tick
revalidates generation plus cast/spell/caster identity before taking the current
request; it executes the taken value rather than a previously cloned payload.
The guard is released before `CastFailed`, spell execution or any await. This does
not change SQL, packet routes, queue timing or the separate active-cast clock;
full C++ item/possession/override request policy remains outside this slice.

Focused coverage checks owner-lock/exactly-once access, active/detached replacement
and cancellation with byte-exact old/new cast IDs, empty cancellation, fixture
non-mirroring, stale-generation/missing-owner rejection and replacement preservation.
A canonical-owner tick test rejects an unknown spell once and leaves the queue empty.
The active cast and two represented cooldown timestamps still belong to Session;
their migration and the broader MapRuntime/application cut remain open #578 work.

Validation on aarch64: `wow-world --lib` passes 3,683 / zero failures / one ignored;
`wow-entities --lib` passes 694 / zero failures. Syntax-only ownership,
architecture check/self-test, formatting and diff checks pass. Reviewed inventory
delta: one field becomes test-only, two private access bridges are added, and the
Session bridge surface fingerprint changes with that cfg annotation; no bridge row
is closed. Totals are 290 production + 429 fixtures, 3,658 associated items and 590
registry entries. Logical Session production 81,313 -> 81,351 (+38), tests
102,621 -> 102,744 (+123). No new dependency edge or runtime clock.
The first quick run caught a test GUID argument type mismatch, corrected before
the successful `validation-v2 quick --base origin/3.4.3` run; manifest
`target/validation-v2/manifests/20260905T021732.877401Z-190684-quick.json` records the
worktree based on `5e925671`, not a clean post-commit final. No live install, fresh
capture, push or terminal architecture acceptance is claimed.

### 2026-09-05 — packet-independent retained cast data

Verdict: the cast ownership migration needs a downward dependency boundary first.
Based on `150d0b1f`, `wow-entities` now defines active/queued cast records and their
target, visual and metadata values in private `spell_cast.rs`. Production active,
queued and toy casts use these records; `wow-world::spell_cast_adapter` converts
at admission, deferred execution and failure publication. No packet implementation,
SQL, loader, channel or runtime task is added to entities. No Cargo edge is added.
This is a dependency-boundary change, **not yet a mutable-owner transfer**: the four
Session cast/queue/timing fields remain production debt in the exact ledger.

C++ evidence: `Spell.h:554,592-602,899` retains cast identity, visuals, targets and
timer; `Spell.cpp:133-171,174` converts request targets into cast-owned data and back
for publication; `SpellDefines.h:497-502` separates cast visual from its packet
conversion. `SpellCastRequest.h:33-43` retains the pending request, owned by Player
(`Player.h:3154`), while Unit owns current spells (`Unit.h:1823`). Rust deliberately
keeps packet decoding out of domain types rather than reproducing the C++ include
dependency. The reduced target record preserves existing Rust values exactly; it
does not claim full C++ target-resolution/transport/trajectory policy coverage.

Use a private entity module plus the existing public value boundary, not a new crate
or generic context. Existing Session type aliases preserve consumer paths temporarily;
delete them when the cast consumers have moved to their owning vertical. No second
mutable cast record is installed or synchronized. Two adapter tests cover all sixteen
optional-target combinations, default/absent targets, bidirectional value preservation,
wire-byte equality and retention of script-visual evidence that remains unserialized.
Metadata defaults, original-cast fallback and timestamps are moved unchanged.

Remaining risk-ordered implementation: (1) install the active record on Unit and the
queue on Player, redirect all writers/readers and reject stale owners; (2) move both
represented cooldown timestamp stores without changing their policy or power-failure
restore order; (3) converge current-spell references and move scheduling/application
effects behind MapRuntime outcomes. Freeze queue replacement/cancellation, interruption,
delay completion, target/visual bytes, publication connection/order and existing clock
ownership in each cut. No lock may span execution await or packet delivery. The old
fields are retired only with their last production consumer, not by relabeling them.

Validation on aarch64: `wow-world --lib` passes 3,680 / zero failures / one ignored;
`wow-entities --lib` passes 694 / zero failures; `cargo check -p wow-world --all-targets`,
syntax-only ownership, architecture check/self-test, formatting and diff checks pass.
The generated syntax inventory is byte-for-byte unchanged. Logical Session production
81,410 -> 81,313 (-97), tests unchanged at 102,621. New entity data and the private
packet adapter are each 118 physical lines; this is redistribution plus a dependency
boundary, not gameplay progress or owner retirement.
`validation-v2 quick --base origin/3.4.3` passes workspace/all-targets and isolated bot
checks; manifest `target/validation-v2/manifests/20260905T020247.924270Z-160062-quick.json`
records the worktree based on `150d0b1f`, not a clean post-commit final. No fresh capture,
live runtime install, push or terminal #578/#133 acceptance is claimed.

### 2026-09-05 — taxi mutations inside canonical Player ownership

Based on `e394af7d`, `mutate_player_taxi_state_like_cpp` now applies its callback
under one generation-checked owner access. The previous read-copy-write sequence
is confined to handle-less test fixtures; the whole-state Session replacement is
private and `cfg(test)` only. Two production callers update the flight node or
perform final cleanup; three test setters share the same helper. All callbacks
are value/container operations without additional locks, delivery or await.

C++ anchors: `PlayerTaxi.h:70-79` (owned route mutation),
`Player.cpp:22019-22024` (`CleanupAfterTaxiFlight`) and
`MovementHandler.cpp:667-722` (flight continuation before teleport; final cleanup
before fall information and honorless-target effects). The existing represented
Rust decisions and their ordering are preserved; this slice does not complete
flight-generator parity or relocate the Session movement handler/map coordinator.

New coverage checks exactly-once callback execution under the active/detached
owner lock, guard release, preservation of flags/mount state when changing a route,
and stale/missing-owner rejection without modifying the replacement Player.
Validation on aarch64: `wow-world --lib` passes 3,678 / zero failures / one ignored.
Syntax-only ownership, architecture check/self-test, formatting and diff checks pass.
`validation-v2 quick --base origin/3.4.3` passes; manifest
`target/validation-v2/manifests/20260905T015448.907973Z-148803-quick.json` records
the worktree based on `e394af7d`, not a clean post-commit final. The reviewed syntax
delta only makes the taxi replacement private/test-only. Logical Session production
81,406 -> 81,410 (+4), tests 102,556 -> 102,621 (+65); field, registry and bridge
totals stay unchanged. No runtime install, capture, push or terminal acceptance;
#578 remains open.

Next ownership investigation: `active_spell_cast`,
`represented_pending_spell_cast_request_like_cpp`, `last_spell_cast_time` and
`last_spell_cast_time_per_spell` remain production Session fields. C++ owns current
spells on Unit (`Unit.h:1823`) and the pending request on Player (`Player.h:3154`).
At this taxi checkpoint, Rust's active/pending records still contained packet-layer
target/visual metadata; the subsequent retained-cast boundary above removes that
dependency prerequisite. Moving fields alone would have introduced an upward edge. The
coherent cut must account for those adapters plus start, cancel, interrupt, delayed
completion, queue promotion and cooldown timing; it is not closed by the taxi slice.

### 2026-09-05 — native canonical spell-book mutation

Ownership-boundary correction based on `0de97d11`: production
`replace_player_spell_runtime_like_cpp` is retired. Ordinary mutations now borrow
`Player.gameplay_state().spells` once through the generation-checked owner; only
handle-less test fixtures use the old represented conversion. Read-only projections
remain adapter inputs, not mutable owners. Acquisition installs its validated fields
without replacing unrelated fallback/trait-config evidence. Save normalization changes
the current owner's rows, then publishes outside the guard. Fallback learning still
reconstructs its prepared row map after the low-level learn helper invalidates row
authority. Attempting a single-row insertion failed the existing disabled-rank closure
regression and was reverted; retiring that narrower bridge requires a separate coherent
cut through the invalidation/learning sequence.

C++ anchors: `Player::AddSpell` (`Player.cpp:2741` onward), `RemoveSpell` (`3236`
onward), `AddOverrideSpell`/`RemoveOverrideSpell` (`28581-28597`), and `_SaveSpells`
(`20399-20452`). The latter removes tombstones and normalizes non-temporary rows;
Rust preserves its existing post-COMMIT timing rather than changing SQL or transaction
semantics in this structural slice. Existing validation, known-spell vector order,
packet metadata/routing, skill installation order and publication remain unchanged.
Callbacks contain only container/value operations, with no I/O, nested owner lock or await.

Focused coverage proves native-state mutation once for active and detached Players,
guard release, unrelated-field preservation, save normalization, and rejection of a
stale incarnation or missing owner without invoking the callback or touching its replacement.
This does not make acquisition's separately ordered spell/skill steps atomic, migrate
cast clocks, remove the remaining Session catalogs, or close #578/#133.

Validation on aarch64: `wow-world --lib` passes 3,677 / zero failures / one ignored;
the expanded native-owner test also passes independently. Syntax-only ownership,
architecture check/self-test, formatting and diff checks pass. Reviewed syntax changes
are the native callback parameter and replacement-to-private-test-fixture transition;
field/registry totals are unchanged. Logical Session production is 81,419 -> 81,406
(-13), tests 102,427 -> 102,556 (+129, including fixture-only conversion).
`validation-v2 quick --base origin/3.4.3` passes; manifest
`target/validation-v2/manifests/20260905T015117.369456Z-141620-quick.json` records the
worktree based on `0de97d11`, not a clean post-commit final. No runtime install,
fresh capture, push or terminal acceptance is claimed for this slice.

### 2026-09-05 — mutate talents/glyphs inside their canonical Player

The follow-on ownership-boundary correction, based on `1245cb72`, retires production
`replace_player_talent_runtime_like_cpp`. The twelve callers of
`mutate_player_talent_runtime_like_cpp` now change `Player.gameplay_state().talents` under
one generation-checked owner access instead of snapshot -> callback -> second-access
replacement. The only remaining conversion back into handle-less Session fixtures is
`store_player_talent_fixture_like_cpp`, compiled exclusively for tests and incapable of
assigning canonical Player state. No new public Player or Session API is introduced.

C++ `Player::AddTalent` (`Player.cpp:2644-2695`) mutates the player's selected talent map;
`SetGlyph` (`25477-25481`) changes the player's glyph slot before update-field publication.
The Rust callers retain their existing validation, talent-group clamping, glyph/talent
load completeness, cost/time accounting, and downstream spell/point/packet effects.
Every callback was inspected: container operations and value changes only, with no SQL,
packet delivery, additional manager lock or await. The paid reset's multiple post-COMMIT
steps remain ordered separately; this does not claim whole-reset atomicity or full C++
talent policy parity. No cast timer, active-cast lifecycle or packet layout is changed.

Focused coverage checks lock ownership, exactly-once execution and guard release for active
and detached Players, preservation of unrelated talent/glyph/cost fields when marking load
completion, and no callback with a missing owner. The existing incarnation-replacement
test now asserts rejected mutation rather than calling the removed replacement API.
The spell-book read-copy-write path and cast lifecycle remain open #578 work.

Validation on aarch64: `wow-world --lib` passes 3,676 / zero failures / one ignored;
syntax-only ownership passes with unchanged field/associated-item/registry totals.
`validation-v2 quick --base origin/3.4.3` passes workspace/all-targets and isolated bot
checks; manifest `target/validation-v2/manifests/20260905T013851.058279Z-119418-quick.json`
records the worktree based on `1245cb72`, not a clean post-commit final. The reviewed
syntax delta removes the crate-visible replacement API and adds only a private fixture
method; generation re-sorts the unchanged SpellHistory fixture entry. Logical Session
production 81,417 -> 81,419 (+2), tests 102,370 -> 102,427 (+57). No field is reclassified
as a stable Session responsibility and no legacy/canonical bridge row is closed.
Architecture check and self-test, formatting and diff checks pass. No runtime install,
capture, push or terminal architecture acceptance is claimed for this slice.

### 2026-09-05 — mutate SpellHistory inside its canonical owner

Ownership-boundary correction on #578, based on `7802ed56`. Before moving the remaining
Session cast clocks, retire `replace_player_spell_history_like_cpp`: the old mutation helper
cloned the complete canonical history under a read access, changed the clone after releasing
the manager, and replaced the canonical history under a second access. That left an
interleaving window in which a different history mutation could be overwritten. This is a
source-proven window, not a claim of an observed live data-loss incident.

`mutate_player_spell_history_like_cpp` now resolves the generation and mutates the Unit's
existing history under exactly one canonical owner access. Its seven production callers
only clear/mark/insert cooldowns or charges, or restore a charge; none performs delivery,
database work, another manager acquisition or an await inside the closure. The guard is
released before the caller resumes. Stale/missing ownership returns `None` without invoking
the operation; active and detached residence use the same lifetime contract.

C++ anchors: `Unit.h:1417-1418,1945` owns one SpellHistory; `SpellHistory.cpp:147-175`
loads directly into that owner's containers, `554-571` changes cooldown entries in place,
and `852-861` restores a charge on the same owner before publication. This change preserves
the existing Rust duration/category/charge policy and its persistence and packet order; it
does not claim full SpellHistory policy parity or change any clock.

The historical handle-less fixture conversion is now explicitly test-only
`store_spell_history_fixture_like_cpp`; it cannot assign canonical history. Read-only
snapshots remain for queries, but production no longer writes one back through this path.
The syntax policy removes the production replacement and adds only the cfg(test) fixture
method. The field inventory stays 291 production + 428 fixtures, and the legacy/canonical
bridge scanner's 65 rows are unchanged: that scanner is not a count of every substate
write-back operation. The remaining cast timestamps/current cast and other substate
write-back paths remain #578 work.

Two focused tests pass on aarch64: active/detached callbacks run once with the manager
locked, the guard is available immediately afterward, and stale/missing owners never
invoke the callback or alter replacement history. The existing spell-family ownership
test now tests rejected mutation instead of the removed replacement API.
Reviewed logical LOC: Session production 81,412 -> 81,417 (+5 for the single-owner path),
tests 102,269 -> 102,370 (+101 for ownership regressions and fixture classification).
Validation on aarch64 passes: `wow-world --lib` 3,675 passed / zero failures / one ignored;
syntax-only ownership ratchet; architecture check and self-test; formatting/diff checks;
and `validation-v2 quick --base origin/3.4.3`, including workspace/all-targets and isolated
bot checks. Manifest `target/validation-v2/manifests/20260905T013345.023415Z-111320-quick.json`
records the implementation worktree based on `7802ed56`, not a clean post-commit final.
No restart, capture, push, new clock or terminal #133/#153 acceptance is claimed.

Follow-on source audit identifies equivalent production read-copy-write helpers in
`mutate_player_talent_runtime_like_cpp` and `mutate_player_spell_runtime_like_cpp`; they
remain open. The remaining cast clocks also interact with active-cast metadata that saves
and restores a previous timestamp on power failure, so their migration must preserve the
selected-character incarnation together with that lifecycle rather than moving isolated
timer fields and leaving a stale cast able to target a replacement.

### 2026-09-05 — borrowed hotfix delivery capability

Boundary extraction on #578, based on `13c984a6`: delete the production
`WorldSession.hotfix_blob_cache`, its setter and getter, with no test fixture mirror.
Bootstrap still builds/overlays the same immutable `HotfixBlobCache` before listeners start.
`SessionHandlerCatalogsLikeCpp.hotfixes` holds that required process-owned catalog; the
session factory and HotfixRequest registration each pass only `&HotfixBlobCache` to the
existing initialization/request consumer. Session retains no catalog or aggregate handle.

C++ anchors: `Handlers/HotfixHandler.cpp:61-135` borrows `sDB2Manager.GetHotfixData()` for
both advertisement and requests. `Server/WorldSession.cpp:1193-1206` places advertisement
after client cache version and before account data/tutorials. `Server/Protocol/Opcodes.cpp`
registers the request as `STATUS_AUTHED/PROCESS_THREADUNSAFE` at line 541 and routes
AvailableHotfixes/HotfixConnect over Realm at lines 1117/1566.

Frozen contract: identical startup data source and overlay order, locale selection, request
iteration order, empty/unknown response, raw-DB2 fail-closed behavior, SQL-blob payload,
opcode metadata and current primary-channel routing. No SQL, clock, lock, persistence or gameplay changes.
The pre-existing typed DB2 serializer gap and optional-data projection are not repaired or
claimed C++-complete by this structural slice. No new capture/live-runtime claim is made.

Tests cover one shared catalog across esES/enUS/deDE sessions, exact initialization opcode
order and advertisement bytes, real request dispatch metadata and response bytes, unknown
push IDs, locale misses, current primary-channel delivery, and no retained Arc. Existing raw DB2 and
SQL-blob tests now inject the capability directly; DBQueryBulk's raw-cache rejection test
uses actual dispatch with a populated catalog.

The initial Realm-only assertions exposed a pre-existing mismatch: HotfixConnect goes through
generic `send_packet`, hence the primary channel after ConnectTo, unlike C++'s Realm route.
The test now explicitly characterizes that existing defect without claiming parity. See
`docs/migration/EXISTING-CODE-DEFECTS.md`; correcting routing is a separate behavioral slice.
Initialization still runs before ConnectTo, when primary is Realm.

Reviewed syntax delta: one field and two accessors removed; two consumer signatures gain a
borrowed cache. Factory fingerprint gains only that argument. All 65 bridges remain; the
WorldSession surface fingerprint changes from the deleted field, not new bridge authority.
The generator also sorts the unchanged `represented_seer_kinds_like_cpp` entry.

Validation (aarch64 development host): `wow-world --lib` passes 3,673 tests, zero failures,
one ignored; this includes both new shared-catalog tests and the adapted raw/SQL-blob
regressions. The syntax-only ownership gate passes 291 production + 428 fixture fields,
49 impl owners / 3,656 associated items, and 590 direct-registry rows. Architecture check
and self-test pass. Reviewed logical LOC: Session production 81,427 -> 81,412, tests
102,131 -> 102,269 (+138 for capability/routing coverage); character-handler production
20,587 -> 20,588 (explicit borrowed signature), tests 12,803 -> 12,810 (direct injection
and dispatch coverage). No bridge or historical persistence inventory is closed.

`cargo check -p world-server`, format and diff checks also pass. `validation-v2 quick
--base origin/3.4.3` passes the full workspace/all-targets check and isolated bot check;
manifest `target/validation-v2/manifests/20260905T012145.604272Z-98232-quick.json`.
That manifest records the dirty implementation worktree based on `13c984a6`, not a clean
post-commit final gate. No push, server restart or fresh capture was performed. The prior
login runtime/final evidence below belongs to its stated revisions, not this new slice.

Active position changes must use the owning Map: `Unit::UpdatePosition`
(`src/server/game/Entities/Unit/Unit.cpp:12257-12284`) calls `Map::PlayerRelocation`
(`src/server/game/Maps/Map.cpp:1015-1040`), which updates cell/grid membership as well as position.
Both the movement setter and the same-map residence path now call the generation-checked
`MapManager::relocate_player_like_cpp`, then private `MapRuntime` and the existing map relocation
operation. Detached preparation still edits the same detached Player value; stale generations
cannot relocate a replacement. This corrects a stale cell index, not merely a method location.
It does not claim new coverage of vehicle passenger relocation or all C++ visibility effects.

The grid callback is a separate capability extraction: one required callback is built in `app.rs`
and borrowed by movement, embedded spell movement and login. The captured stores/managers and
call boundaries are unchanged; no new timer, queue, SQL statement or opcode registration is added.
C++ `Map::EnsureGridLoadedForActiveObject` / `AddPlayerToMap` (`Map.cpp:348-363,427-445`) anchors
the grid responsibility and the login grid gate remains before success publication.

The preceding syntax changes retain one Player value across residence (`MapManager::CreateMap`,
`MapManager.cpp:139-232`), replace external borrowed Creature access with closure-scoped queries,
and route represented creature combat through MapRuntime commands. C++ `Unit::Attack`
(`Unit.cpp:5645-5745`), `Unit::CombatStop` (`5802-5821`) and `CombatManager::SetInCombatWith`
(`CombatManager.cpp:187-228`) anchor reciprocal combat ownership before publication. These
anchors explain ownership and phase constraints; this inventory refresh does not certify all
earlier gameplay changes or complete AI/script callback parity.

## Reviewed syntax delta

Relative to the checked-in policy at `26f72455`:

- Remove the grid-resolver field, its setter and `ensure_player_grid_loaded_like_cpp`.
- Add `ensure_canonical_player_owner_exists_like_cpp`: adopt the current incarnation or create
  the one detached value, with revalidation under the manager lock, before map selection effects.
- Replace `is_represented_seer_kind_like_cpp` with `represented_seer_kinds_like_cpp`, retaining
  the same Player/Creature/Pet/DynamicObject kind set for the narrowed lookup API.
- Borrow the required grid capability through the six changed login/movement/spell/connection
  signatures. No registration metadata or dispatch admission rule changes.
- Retain all 65 discovered bridge rows. Eighteen fingerprints change for the reviewed
  Creature lookup/combat/residence paths, factory callback relocation, WorldSession declaration
  and corresponding world-server fixtures. No bridge is accepted as retired by renaming it.
- Registry accesses, SessionCommand vocabulary and generated-surface inputs are unchanged.

## Logical size reconciliation

These are exact logical-owner counts on the aarch64 development worktree, including reviewed
private descendants, not physical-file size or performance measurements. Both increases and
reductions are recorded. The ceilings continue to reject further unreviewed growth; this
checkpoint does not waive #578's semantic acceptance criteria.

| Logical owner | Production old → current | Tests old → current |
|---|---:|---:|
| Session | 73,339 → 81,427 | 97,325 → 102,131 |
| Map | 15,396 → 16,167 | 18,273 → 18,728 |
| Character handlers | 19,786 → 20,587 | 12,899 → 12,803 |
| Loot handlers | 13,415 → 13,939 | 16,383 → 16,478 |
| World-server crate | 29,273 → 28,896 | 26,605 → 27,008 |
| Player | 9,536 → 10,370 | 8,891 → 9,273 |
| Quest handlers | 8,325 → 8,857 | 10,591 → 10,620 |

Session/handler growth includes explicit unavailable-owner handling, scoped canonical queries,
capability parameters and fixture migration; it remains a large application monolith. Player
growth is private canonical substates and their tests. Map growth is private EntityWorld/runtime
boundaries, residence/command/relocation operations and tests, without an additional mutable
representation. World-server production shrinks while canonical runtime fixtures grow. The
unchanged Group hotspot ceiling is left untouched.

## Validation boundaries

### Login-stream follow-up — 2026-09-05

Bot/guard commit `10684ccb` closes the premature smoke-test disconnect described below.
Ordinary login-only QA now retains both sockets until an instance `SMSG_UPDATE_OBJECT` has
arrived and the streams have been quiet for one second (30-second absolute budget). It uses
cancellation-safe peeks before reading complete encrypted frames, rejects connection closure,
and responds to time sync. Anchors: C++ `Map.cpp:427-446,1826` (`AddPlayerToMap` / `SendInitSelf`)
and `MiscPackets.cpp:156-167` (time-sync request/response). It does not decode the self CREATE
or prove full world visibility/gameplay; `login_stream_drained` names this bounded evidence.
The runtime guard now requires that field, not just `player_login_verified`.

All 142 bot tests pass, including successful drain, realm closure, and missing object
publication; the 69 runtime-guard checks pass. Bot build and formatting/diff checks pass.
The installed optimized server remains code `d568f3aa`, SHA-256
`91663b7c21888f4de5e280ddd1a22c5f811e7ecca844eeed154ab65deee191ca`.
Guarded report: `/tmp/rustycore-578-drained-login-runtime.json`; private bot evidence:
`/tmp/rustycore-login-qa.crzSbP/bot.json`. All four auth/enumeration/login/drain flags are true.
Candidate PID 45080 logged `Login sequence complete` at **00:31:48.563330 UTC**; the following
`World::KickAll` at 00:31:49 is the guard's shutdown for restoration, not the previous
`login packet sequence failed` error. This validates one automated login on the local fixture,
not manual-client readiness, sustained gameplay, LFG, or fresh C++ capture parity.
The guard reports `passed-restored`; the deployed binary's original SHA-256 was independently
verified and both services are active. No push or merge was performed.
Final validation on code HEAD `10684ccb` also passed:
`target/validation-v2/manifests/20260905T003127.014359Z-45213-final.json`
(6,745 library tests, 315 contract-checker tests, and the isolated bot check). This evidence
predates only the documentation closeout commit; the 142 bot tests were run separately.

### Earlier Dungeon Finder scope note — not the current execution order

At that earlier checkpoint, the user requested login repair/verification followed by an LFG audit.
The later approved #133/#578/#583 plan above now governs execution; this retained scope note
does not start another issue, bypass prerequisites or silently defer #578's remaining work.
No LFG gameplay implementation or database repair is included in this checkpoint.

Preliminary evidence (not a completed subsystem audit):

- Rust registers information/status/blacklist handlers in `handlers/misc/lfg.rs`, but the
  production handler search does not find DFJoin/DFLeave/DFProposalResponse/DFSetRoles/DFTeleport.
  Its LFG-list status explicitly represents removed-from-queue while the manager is unported.
- C++ does contain automatic matching: `LFGHandler.cpp:31-104`, registrations at
  `Opcodes.cpp:425-430`, `LFGMgr.cpp:286,397,945,1052,1357,1472`, and
  `LFGQueue.cpp:288,358`. Existing code is not proof of complete client-3.4.3 behavior.
  Manual listings are a different surface: `LFGHandler.cpp:584-632` returns zero search results
  and an explicitly unimplemented application response, despite a partial `LFGListManager`.
- Local Hotfix rows 256/258 are named Random Lich King Heroic/Normal with build 12340; the
  other 97 rows have build 52237. Provenance remains unknown. The downloaded
  [Wago LFGDungeons export for 3.4.3.54261](https://wago.tools/db2/LFGDungeons/csv?build=3.4.3.54261)
  (SHA-256 `fe615884df9b32a1a281d94499509dd4f80da61160156e31d8f39523815e1d47`)
  has empty descriptions for Random Lich King Dungeon/Heroic at IDs 261/262. IDs 256/258 instead
  mean Halls of Reflection/Random Classic Dungeon. Do not infer missing descriptions or rewrite
  these local IDs without auditing references and the effective local DB2/Hotfix overlay.
- Issues #550/#552 closed loader/capability extraction only, not the gameplay system.

When LFG is selected through the current plan, its scope should cover roles, join/leave, matching, proposals, group creation, teleport,
completion/reward and cancellation/disconnect/retry cases. First audit C++ gaps, data integrity,
current Player/Group/Map dependencies and packet-capture availability. Missing C++ behavior
requires exact-build client/capture evidence. Manual LFG List remains a separate scope, and no
queue owner or new runtime clock is chosen here.

### Runtime follow-up and production-only construction regression

On `d9f1e5ee`, final validation passed:
`target/validation-v2/manifests/20260904T234919.430148Z-4193605-final.json` (6,745 library
tests and 315 checker tests). The two production-linked login tests and 157 capture-diff
regressions passed separately. The release build completed on aarch64 in 8m13s.

Installed candidate `8281cd5aebdedd7ae792493d8da356937fff0791b3ed416855025a7993a9c1fc`
passed initial mail hydration but stopped at `canonical Player currency owner unavailable during
login`, after map selection. Guarded QA reported `failed-restored` in
`/tmp/rustycore-578-login-owner-runtime.json`; private evidence is
`/tmp/rustycore-login-qa.8sJ1lB`. The original executable was restored and serving.

The subsequent regression reproduced this difference without a live DB: initial map selection followed
by collection reads and interleaved map ticks passes in dev but fails in release. Root cause:
`Map::insert_map_object_record` put its actual insertion inside `debug_assert!`, so optimized
builds erased the mutation. Moving the insertion into an unconditional statement adds exactly
one production Map line (16,167 -> 16,168; tests stay 18,728; total 34,895 -> 34,896). This is the
only hotspot-ceiling adjustment; field/bridge/syntax policy is not refreshed. It is a behavior
repair of the staged storage change, not completion of #578.

Post-fix evidence on `d568f3aa` (2026-09-05, aarch64):

- Final validation passed: `target/validation-v2/manifests/20260905T001226.991927Z-24382-final.json`
  (6,745 library tests and 315 checker tests). All three production-linked login regressions
  pass in both dev and release. The optimized world-server build completed in 12m54s.
- Installed candidate SHA-256
  `91663b7c21888f4de5e280ddd1a22c5f811e7ecca844eeed154ab65deee191ca`
  returned bot status zero; `/tmp/rustycore-578-map-insertion-runtime.json` records the guarded
  result. Private bot evidence is `/tmp/rustycore-login-qa.TFANN8/bot.json`: authentication,
  character enumeration and `player_login_verified` are true. Candidate PID 38971 reached
  aura hydration and the later "continuing login" phase at 00:24:44 UTC, beyond both repaired
  mail/currency-owner failures.
- This is **bounded login verification, not full world-entry acceptance**. The bot's ordinary
  login loop exits on `SMSG_LOGIN_VERIFY_WORLD` (`main.rs:5704-5733`) and closes the sockets;
  the candidate subsequently reports connection reset/broken pipe and "login packet sequence
  failed". Extend the maintained bot's completion criterion before claiming stable world entry
  or starting LFG runtime acceptance. No new client packet layout is inferred from this run.
- The guard restored the original executable, SHA-256
  `c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`;
  world-server and bnet-server are active. No manual-client readiness or fresh C++ capture-diff
  is claimed, and no LFG gameplay or local LFG row changes were made.

The local final gate passed on `e1daed4c` and again on `fbd762c6`; the latter manifest is
`target/validation-v2/manifests/20260904T230645.707038Z-3-final.json` (6,745 library tests
passed). These are historical evidence, not validation of subsequent changes.

Guarded login QA exposed three independent boundaries:

- Local DB schemas were already materialized, but the official migration history was absent.
  The official `rustycore-db` transition-import path adopted the four existing auth/characters
  migrations without replaying their DDL. All four databases then validated compatible.
  Before adoption, full auth/characters and schema-only world/hotfixes backups were saved under
  private `/tmp/rustycore-578-db-backup.Ay81P8`. These contain sensitive runtime data and must
  never be committed. No LFG rows were edited.
- SQL NULL LFG descriptions exposed the separate loader repair in `fbd762c6`.
- Candidate `64a95e7eb6572577498776d09bd39b692a695c9ef93d6716e14dba68265ad028`
  authenticated, enumerated characters and linked the instance socket, then kicked during
  mail hydration because initial Player construction required its own canonical inventory.
  The old build was restored; this run did **not** pass login QA.

The construction fix limits Session-to-Player equipment hydration to old unit fixtures.
Production starts with the new Player's empty equipment and uses the existing later inventory
load; it adds no fallback for unresolved active/stale owners. C++ anchors and the failure are
recorded in `EXISTING-CODE-DEFECTS.md`. The new integration test compiles wow-world without
`cfg(test)`, reaches PetStable only after successful mail/scalar hydration, and rejects a missing
manager before that point. The positive case fails on the original code and passes with the fix;
the negative case passes in both. Architecture check/self-test and the syntax-only ratchet pass
without baseline changes. Subsequent final and bounded installed-login evidence is recorded
above; complete world-entry acceptance remains pending.

Focused current/stale/detached Player tests and movement/login/spell checks are recorded in
`docs/migration/adr-map-runtime-entity-world.md`. The added same-map residence regression also
passes after changing the destination across a cell boundary. The full world suite was repeated
after that correction: 3,671 passed, zero failed, one ignored. The map suite (703 passed, zero
failed, one ignored) and quick validation had already passed. The release world-server build
also passes. Architecture check/self-test and the syntax-only Session ratchet pass with the
reviewed ledger. Final validation and live QA must be reported separately, not inferred from
a refreshed baseline.
