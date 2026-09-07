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

Remaining acceptance: perform the separately applicable
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

### Authorized normal save/relogin — 2026-09-07

The user renewed explicit runtime authorization after the rejection above. The
execution control accepted the same bounded procedure; the earlier permission
blocker is resolved. On the already validated `ccf5f84d` candidate, run:

```bash
BNET_HOST=127.0.0.1 BNET_PORT=8081 \
QA_GIT_DIR=/tmp/rustycore-585-runtime.XxNlUp/source \
QA_SMOKE=/home/server/rustycore/tools/wow-test-bot/run_login_save_relog.sh \
./tools/qa-runtime.sh --allow-runtime-qa \
  --world-exec /home/server/rustycore/target/validation-v2/cargo/209fefad83026767/release/world-server \
  --report /tmp/rustycore-585-runtime.XxNlUp/runtime.json login
```

Exit 0; runtime report `outcome=passed-restored`, `bot_status=0`. The private report
`/tmp/rustycore-login-qa.QpBnwv/bot.json` has
`login_save_relog_verified=true` and successful authentication, character enumeration,
world entry and drained login streams. The maintained wrapper's two fresh
authentications verified normal logout/save and matching saved projections.
Candidate PID was 2492153; executable SHA-256 remained
`6cbc844d66f84d50cb4003c2101dbca19403830238eb7feda021ac39cc48930f`.
The guard restored the original executable with SHA-256
`c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`
and confirmed it was serving before returning success.

Only existing TESTBOT1@bot.local and normal auth/character writes were used;
no account provisioning, fixture deletion or bnet-server restart was performed.
This is bounded normal save/relogin evidence, not transfer/disconnect fault-injection,
fresh packet-capture parity or unknown-COMMIT recovery proof. Those action-specific
acceptance gates remain outstanding. #585 remains open; no push or merge occurred.

### Authorized transport EOF/save/relogin — 2026-09-07

QA tooling commit `55ec9a8b` adds a bounded disconnect mode in the private
`tools/wow-test-bot/src/login_save.rs` scenario, extracting termination from the
main bot dispatcher. Existing normal logout still requires LogoutComplete.
The new mode sends FIN on both authenticated transports without LogoutRequest,
then requires a newer offline Character save and Login account offline. A second
fresh authentication verifies the same six saved projections and known/favorite
spell packets, and finishes with confirmed normal logout. No fixture SQL is written.

Finished-tool validation (aarch64): `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo test
--offline --locked --manifest-path tools/wow-test-bot/Cargo.toml` passed all 144
tests. The corresponding `cargo build` succeeded. The normal report acceptance
script passed its positive and 11 negative cases; the disconnect script passed
its positive and 12 negative cases. Bot formatting, diff checks and the physical
ratchet passed (1,006 files, 100 legacy ceilings). These are tool-specific results,
not a new whole-branch final manifest.

The clean runtime checkout was advanced to `55ec9a8b`. Its delta from core candidate
`ccf5f84d` contains documentation and bot tooling only; the already validated server
executable was reused with unchanged SHA-256
`6cbc844d66f84d50cb4003c2101dbca19403830238eb7feda021ac39cc48930f`.
The rebuilt bot SHA-256 is
`f2a882e29cd98a8f2d4f35c9f8e661b58892ab345058887531a5c104c13df24b`.

```bash
BNET_HOST=127.0.0.1 BNET_PORT=8081 \
QA_GIT_DIR=/tmp/rustycore-585-runtime.XxNlUp/source \
QA_SMOKE=/home/server/rustycore/tools/wow-test-bot/run_login_disconnect_relog.sh \
./tools/qa-runtime.sh --allow-runtime-qa \
  --world-exec /home/server/rustycore/target/validation-v2/cargo/209fefad83026767/release/world-server \
  --report /tmp/rustycore-585-runtime.XxNlUp/disconnect-runtime.json login
```

Exit 0, `outcome=passed-restored`, `bot_status=0`; candidate PID 2502495.
Private `/tmp/rustycore-login-qa.ovG9vK/bot.json` records
`login_disconnect_relog_verified=true`. Its first-phase report confirms
`disconnect_confirmed=true`, `logout_confirmed=false`,
`login_account_offline=true`, and a new offline save. The second phase confirms
normal logout and matching saved state. The guard restored original SHA-256
`c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`
and confirmed it serving before returning. No account provisioning, SQL fixture
setup/deletion or bnet restart occurred.

C++ `WorldSession.cpp:507-536` supplies the closed-socket expiry/removal reference;
this pass does not claim identical timing, TCP RST, pending-transfer behavior,
fresh capture parity or crash/unknown-COMMIT recovery. Pending-transfer lifecycle
QA remains separate. The bot's existing auto-teleport is LFG-specific and must not
be used as a substitute. The represented area-trigger route in
`handlers/misc/travel.rs:386` validates proximity before teleporting, matching
`MiscHandler.cpp:478-503`; exercising it needs an explicitly scoped portal scenario,
not an arbitrary trigger packet from the current character position.

### Pending portal QA: routing defect reproduced — 2026-09-07

The user explicitly authorized temporary relocation/restoration of TESTBOT1 and
the development/runtime tests needed for this delivery. Tooling `2c43d68a` adds
the recoverable position-only fixture and an actual AreaTrigger/SuspendToken/
NewWorld handshake, withholding WorldPortResponse. The production DB2 reader
locates trigger 2173 at map 0, (-8346.46, 514.031, 96.5989), radius 10; existing
safe location 3650 targets map 369 at (67.7607, 2490.98, -4.29649).
No portal/world rows are changed. The positional wrapper records original
map/instance/zone/XYZ/orientation before mutation and restores only those fields
with the world service stopped. All 145 bot tests and four hermetic fixture
recovery tests passed; this is tooling evidence, not server acceptance.

The real run on unchanged core `ccf5f84d` failed before its intended cutoff:
`misrouted/duplicate transfer-pending`. Private evidence is
`/tmp/rustycore-login-qa.gy2BIF`; runtime report and recovery journal are under
`/tmp/rustycore-session-transfer-585.vbftsly3`. The runtime guard reported failure
and restored the original executable; the positional wrapper separately verified
the original location and serving original executable, then exited nonzero.
The journal records `restored=true`. This failed run is not transfer acceptance.

Current source confirms the route defect: both represented TransferPending
emission sites call instance-default `send_packet`, as does the NewWorld response.
Exact C++ `Opcodes.cpp:2173/1811` register these on realm; `:2150` registers
SuspendToken on instance. The bounded correction changes those two packet routes,
preserves the existing NewWorld queue-failure kick, and adds a private two-channel
regression. This is an intentional protocol repair, not structural refactoring.
The character logical-test ceiling grows by exactly 51 lines for that regression;
production ownership and physical root ceilings are unchanged. Validation of the
correction and a new installed-candidate run remain outstanding.

Routing correction `8805ab51` passed its two-channel regression, the complete
wow-world library (3,783 passed, one ignored), formatting, architecture and current
syntax-only ownership checks. Its release build succeeded; executable SHA-256
`ffe50b9a2dcc2e98a279e3b8a6164f9caf807b3dd2cabcfcb1985d63dd813682`.
The next real run reached NewWorld but failed the 90-second offline/save check.
Candidate PID 2553366 reported NativeTransfer=Unavailable, every later obligation
NotAttempted, RetainAndEscalate; the supervisor did not label this a successful save.
Private evidence: `/tmp/rustycore-login-qa.O3sIpW`; runtime/journal:
`/tmp/rustycore-session-transfer-585.yzobe_mc`. Both original executable and original
character location were restored and verified; the run exited nonzero.

The pending destination setter retained raw orientation, while canonical
WorldObject relocation normalizes it. A request such as orientation 180 therefore
cannot satisfy the exact-position comparison at native post-add admission.
C++ Player.cpp:1456 creates WorldLocation, whose Position constructor normalizes
orientation (Position.h:29). The bounded correction reuses Rust's WorldLocation
normalization when retaining the far destination. Its regression covers positive,
negative and full-turn inputs through native completion and save preparation.
This does not change map authority, introduce a position mirror or weaken the
post-add equality guard. The normalization regression and full wow-world library
passed locally (3,784 passed, one ignored); formatting/diff checks pass. The fresh
installed-candidate retry remains outstanding.

Separate catalog limitation: the production area-trigger destination query joins
raw world_safe_locs.Facing, whereas C++ ObjectMgr.cpp:7032 converts degrees to radians.
Rust's separate WorldSafeLocsStore already does that conversion, but the currently
composed direct destination loader bypasses it. This pre-existing catalog-unit
defect is not proof against finalization of a valid radian destination, and is not
silently repaired by normalization. It remains outside #585's finalization contract;
do not label the portal's orientation as full C++ teleport parity. The normalizing
setter still must handle any finite radian input consistently with canonical relocation.
