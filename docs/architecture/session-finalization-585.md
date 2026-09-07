# Represented session finalization — #585

Implementation is in final validation on the #585 branch, based on integrated
`59f5bced`. The implementation block preceded validation, as requested by the user.
This is not issue closure or runtime acceptance.

## Scope and owners

#578/#579 are closed. This delivery is a child of coordination issue #584, under
#133. It does not complete all C1/C0–C4, select the next core family, introduce hecs,
or open #583. Full Login-side SaveToDB composition and general account recovery
remain outside this delivery.

- `wow-world/src/finalization.rs` owns the finite obligation order and decision
  ledger. It receives no Session, transport, SQL pool or mutable Player.
- `session/lifecycle/finalization.rs` adapts those admitted steps to existing
  capabilities. It retains the operation in the task-owned Session across awaits.
- `session/lifecycle/account.rs` projects existing account/offline operations and
  returns their outcome instead of discarding it. Transactions are not regrouped.
- `world-server/src/session_factory/finalization.rs` handles whole-operation
  completion and fail-stop retention; `session_factory.rs` wires the actual task.
- Canonical Player retirement still belongs to MapManager and uses the exact
  existing PlayerHandle, never a GUID-based replacement lookup.

## Included routes and order

Existing character-selection logout: native transfer completion, loot settlement,
buyback, Character save, mounts/toys/heirlooms/appearances/illusions, character
offline, publication-directory removal and exact retirement, LogoutComplete,
character-account offline, claim/inventory release and realm-channel restoration.
Character-selection admission is reopened only after the included writes finish.

Disconnect/shutdown: the same persistence prefix, character-account and Login-account
offline, directory removal/exact retirement, then claim/inventory release. Account-only
disconnect still attempts Login-account offline; it is not a Character-save success.

Timed logout is admitted by the existing timer and executed by the same coordinator
through supervision. It publishes completion after save/retirement, then finishes
account offline obligations and disconnects. It does not implement missing combat,
falling or duel admission rules for CMSG_LOGOUT_REQUEST.

Autosave and trainer pre-save retain the existing Character-save operation. They
do not acquire finalization/account-wide semantics.

## Explicit behavior changes, not mechanical parity claims

The pre-existing paths continued after some known failures and hid offline/account
outcomes. This delivery retains and escalates any required failed, unavailable,
deferred, uncertain or interrupted obligation. Neither a confirmed Character save
nor an awaited future is whole-finalization success. Known rollback is reported
separately from uncertain completion; neither is automatically retried here.

The supervisor closes its existing admission gate, requests session stop and sets
terminal error status. The existing session task keeps the Session, character claim,
operation ledger and active registration until process teardown; no detached retry
task, new global store or second Player owner is introduced. Shutdown may therefore
report non-drained sessions and exit unsuccessfully. Runtime destruction is not
proof that the DB rolled back. General durable restart recovery remains outside scope.

Each step becomes InFlight before its await. Dropping a future leaves that evidence
in its owner; disconnect cannot replay an interrupted explicit logout. Account
collection requests are retained on the operation before submission, including the
appearance plan whose existing builder clears favorite dirty markers. They are
released only after successful classification, not on uncertainty. This is retained
recovery input, not an implementation of replay/reconciliation.

Autocommit offline writes can lose their acknowledgement too. The current execute
API does not prove submission stage on an error, so those errors are conservatively
Unknown, rather than inventing a definitely-rolled-back result. No statements change.

LogoutComplete uses the same serialized packet and current connection, with an
asynchronous channel send. Applied for that obligation means channel acceptance,
not client receipt. Timed logout no longer publishes success before persistence.

## Source anchors

Legacy root: `/home/server/woltk-trinity-legacy`.

- `src/server/game/Server/WorldSession.cpp:544-551`: pending transfers before logout.
- `WorldSession.cpp:618-632`: buyback clearing and SaveToDB before Player destruction.
- `WorldSession.cpp:660-685`: exact Player destruction, LogoutComplete, account-offline.
- `src/server/game/Entities/Player/Player.cpp:19312-19322` and `:19662-19688`:
  Character/Login transactions and Login participants; **not** proof that Rust's
  separately represented account saves form that complete C++ operation.
- Existing boundaries and preceding evidence remain in `session-578-checkpoint.md`
  and `docs/migration/player-lifecycle-persistence-contract.md`; their historical
  open-#578 status does not reopen that predecessor.

## Acceptance still to execute

Development evidence on the uncommitted implementation above `1e1a4c41`
(aarch64, 2026-09-07; not evidence at a closing SHA): the production integration
target `production_login_player_owner` passed 34 tests; the world-server library
filter `shutdown_` passed 10; the wow-world library filter `finalization` passed 10.
The complete wow-world library suite then reported 3,775 passed, seven failed and
one ignored. After correcting old early-publication expectations, unit fixtures
without canonical ownership and a logout fixture whose persistence port deliberately
fails, the complete suite passed 3,782 tests, zero failures, one ignored. The four
affected character-consumer tests also passed after their private module extraction.
The production integration target passed all 34 tests in release. The wow-database
library passed 358 tests with two ignored. Ignored tests are not passing evidence.

Architecture check, architecture self-test and the syntax-only ownership check pass.
The reviewed Session associated-item delta is 14 old records removed and 19 new
records added; the field set adds only the finalization ledger. The two existing
bridge fingerprints change with the Session/factory bodies; no bridge is retired
or newly introduced. The physical ratchet has 100 legacy ceilings after retiring
the account handler entry; vendor/world-entry/test-root ceilings tighten after
bounded moves. Their remaining responsibilities stay in #584. The pre-squash
ownership anchor now names integrated `59f5bced`; historical measurements retain
their original SHA. Committed-candidate final validation and live QA remain outstanding.

The completed exhaustive scan preserves 10,106 accesses and 1,029 groups. Its only
six changed records are async-capture/pin/return fingerprints for the same offline
adapter, changing Failed to Unknown; statements, databases, counts and direct SQL
operations are unchanged. The policy changes only that workflow's connection,
ordering and uncertainty contract. All five `persistence_policy` tests pass,
including checked-snapshot consistency. This behavior correction is committed
separately as `a278cc98`; the finalization boundary follows in the same delivery.

The user authorized temporary world-server installation/restart for existing
TESTBOT1@bot.local login/logout/save/relogin with restoration of the original build,
normal auth/character writes only, no account creation, deletion or bnet restart.
This authorization does not claim execution, transfer/capture coverage or a pass.

Remaining acceptance: perform the normal save/relogin QA and the separately applicable
action-specific transfer/logout/disconnect capture/runtime scenarios; record exact
candidate and restored identities; prepare the single #585 PR. Normal save/relogin
does not waive the action-specific gate. No live or full issue acceptance is claimed.

### Committed candidate — 2026-09-07

`ccf5f84df6b277b16e50f683a94a8cb3fdf7b275` passed
`CARGO_INCREMENTAL=0 PROTOC=/home/ubuntu/.local/protoc/bin/protoc VALIDATION_V2_CARGO_JOBS=2 VALIDATION_V2_TIMEOUT_SECONDS=1200 ./tools/validation-v2 final --base origin/3.4.3`.
The verified-green manifest is
`target/validation-v2/manifests/20260907T014920.415561Z-3-final.json`.
It includes affected/reverse-consumer compilation and library suites: world-server
569 passed; wow-database 358 passed/two ignored; wow-world 3,782 passed/one ignored.
Tracked files matched HEAD after the run. Its truthful dirty flag includes only the
unrelated, untracked `docs/architecture/lfg-343-audit.md`, retained unchanged with SHA-256
`1a9155fbc06617201dc65ace2170e5885532e55c9dcb389e95a0a31b1de285dc`;
it is not a Rust build input and is not part of #585's commit.

The release executable embeds this candidate commit and has SHA-256
`6cbc844d66f84d50cb4003c2101dbca19403830238eb7feda021ac39cc48930f`.
The current bot was rebuilt successfully, SHA-256
`ce591f69ae084e3b2e5065964b3a25528401cf513c037cd7faf20a1cb8271290`.
A clean checkout of the same candidate at
`/tmp/rustycore-585-runtime.XxNlUp/source` supplies the runtime guard's source identity;
the existing build is reused, not rebuilt in that checkout. Guarded dry-run passed.

Runtime QA did **not** start: the execution permission reviewer rejected the actual
service-swap command twice, including reconsideration with the user's affirmative
reply to the scoped authorization question. No service stop/copy/start or bot login
was executed by those rejected calls. The original live identity observed before
the attempt was SHA-256
`c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`,
PID 1234826, zero automatic restarts and no packet dump. A runtime gate remains
outstanding; final-profile success does not waive it. Push, merge and #585 closure
have not occurred.
The read-only snapshot after rejection confirmed the identical PID, executable
hash, restart count and absent packet dump; no restoration was necessary.
