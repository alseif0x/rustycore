# Represented session finalization — #585

Candidate `5f5e225f` passed bounded normal, disconnect and pending-transfer paired
runtime QA on the #585 branch, based on integrated `59f5bced`.
The implementation block preceded validation, as requested by the user.
Candidate `b8895373` subsequently passed final validation after a test-only repair
and reviewed test-fixture policy entry. Fresh scoped comparisons pass, but full
logout/portal windows remain divergent. The bounded scope disposition below resolves
that review without claiming full action parity; publication and issue closure remain pending.

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

LogoutComplete uses the same serialized packet and, after the capture-driven correction,
the realm connection required by C++ Opcodes.cpp:1665, with an
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

## Implementation and acceptance evidence (chronological)

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
correction and a new installed-candidate run were still outstanding at that checkpoint;
the following entries record their results.

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
installed-candidate retry was outstanding at that checkpoint and subsequently passed below.

Separate catalog limitation: the production area-trigger destination query joins
raw world_safe_locs.Facing, whereas C++ ObjectMgr.cpp:7032 converts degrees to radians.
Rust's separate WorldSafeLocsStore already does that conversion, but the currently
composed direct destination loader bypasses it. This pre-existing catalog-unit
defect is not proof against finalization of a valid radian destination, and is not
silently repaired by normalization. It remains outside #585's finalization contract;
do not label the portal's orientation as full C++ teleport parity. The normalizing
setter still must handle any finite radian input consistently with canonical relocation.

### Pending portal and final candidate accepted locally — 2026-09-07

On aarch64, candidate `bf884aecee151551503238fae761f52141b07556` passed the
guarded pending-transfer scenario after the distinct routing (`8805ab51`) and
normalization (`bf884aec`) repairs. The installed release binary SHA-256 was
`3662aabb0746694030e72af07d6e1cfbb51233dbb2cf99000c9db8cf3de855aa`;
the bot SHA-256 was
`62ea6ce7581783af5d7047b6d16345112fbb34e42789a39a2c9177cfd7fd671c`.
The clean source checkout was at the same candidate. Command:

```bash
python3 tools/wow-test-bot/run_session_transfer_qa.py \
  --allow-position-fixture \
  --source /tmp/rustycore-585-runtime.XxNlUp/source \
  --world-exec /home/server/rustycore/target/validation-v2/cargo/209fefad83026767/release/world-server
```

The command exited 0. Private runtime evidence is
`/tmp/rustycore-session-transfer-585.l2mpgaxb/runtime.json` (`passed-restored`,
bot status 0); its recovery journal records `admitted=true`, `restored=true`.
Bot evidence is `/tmp/rustycore-login-qa.812yZO/bot.json` and the accompanying
first/second reports. TransferPending and NewWorld arrived on realm, SuspendToken
on instance; the bot withheld WorldPortResponse and closed the transports.
The first phase confirmed disconnect, character/account offline and destination
save at map 369, (67.7607, 2490.98, -4.29649), without LogoutComplete. A fresh
login followed by normal logout confirmed the same saved destination and the six
tracked save projections/known and favorite spells; the aggregate reports
`login_disconnect_relog_verified=true`. This is not a TCP RST or crash-recovery test.

The wrapper verified both the original character location and the serving original
executable after restoration; installed SHA-256 is
`c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`.
No bnet restart, account creation/deletion or world portal-row mutation occurred.

The same candidate passed `validation-v2 final --base origin/3.4.3` with
`CARGO_INCREMENTAL=0`, the declared PROTOC, two Cargo jobs and a 1200-second
per-command timeout. Verified-green manifest:
`target/validation-v2/manifests/20260907T103421.255301Z-2-final.json`.
All 14 planned commands passed, including reverse-dependent compilation, the
affected library suites and QA-bot compilation. The wow-world library has 3,784
passing tests and one ignored. Production-linked `production_login_player_owner`
also passed in dev and release (34 tests per profile), using the same shared target
and `cargo test --offline --locked -p wow-world --test production_login_player_owner`
(with `--release` for the release run). Tracked files matched HEAD after validation.
The truthful dirty flag is solely the unchanged unrelated LFG document identified
in the earlier candidate evidence, not a modified source or fixture.

Remaining acceptance is the applicable fresh C++/Rust action capture comparison,
not another implementation of the coordinator. The documented reference executable
`/home/server/trinity-legacy-install/bin/worldserver` is absent, as is the temporary
C++ executable named by the historical creature-spell evidence. Current clean legacy
source is `a5f8da2e`; old artifacts cannot be relabelled as a capture of this candidate.
The recording wrappers explicitly target PM2 while this deployment uses systemd
(`crates/capture-diff/README.md`, Recording a capture). A replacement reference build
and a bounded, restorative capture harness are needed before claiming that evidence.
Do not run the old service-swap scripts unchanged, waive the gate, or infer full portal
orientation parity from the successful Rust lifecycle scenario. Publication/review
and issue closure remain separate from this local evidence.

### Reference build and isolated database prepared — 2026-09-07

The pristine legacy `a5f8da2e` build failed in five LFGList.cpp diagnostics that
use nonexistent `LOG_DEBUG`. A private clone at
`/tmp/rustycore-585-cpp-reference.3xdDma/source` replaces those calls with the
existing fmt-style TC_LOG_DEBUG (`0f205830`). Release linking then exposed a
missing emitted Spell::SearchTargets specialization called by SpellAuras.cpp;
`1f2053f2ba4cebf5ae15fc07bdb8895602d1481f` explicitly instantiates the existing
template body, without altering its search logic. The original legacy repository
is unchanged and clean. No LFG implementation or gameplay repair is claimed.

`cmake --build /tmp/rustycore-585-cpp-reference.3xdDma/derived-build --target worldserver --parallel 2`
passed with GCC 13.3, Release, static scripts, tools/tests disabled. Executable:
`/tmp/rustycore-585-cpp-reference.3xdDma/derived-build/src/server/worldserver/worldserver`,
SHA-256 `1c26893b072226d5301f5b0d48cfdfd70e95de34e76d6c1ce5f50e160b630ac3`.
`--version` reports revision prefix `1f2053f2ba4c`; do not describe it as pristine
`a5f8da2e` or reuse the historical capture's different patched identity.
The complete base-to-derived patch is retained privately at
`/tmp/rustycore-585-cpp-reference.3xdDma/build-compatibility.patch`, SHA-256
`a01ec25064181bbac6a370e32c4de49990df1fa6dffddb0e5977d22eee62e0f4`.

Source inspection also showed that C++ startup mutates more than TESTBOT1:
Main.cpp:664-696 resets online/battleground state and updates world version;
World.cpp:2343/2444 invokes old-mail and old-character cleanup. The reference must
therefore run on isolated data, not through the existing position-only live guard.
A single-transaction, skip-lock-tables export of auth/characters/world/hotfixes
was imported into a separate MariaDB under the mode-0700 directory
`/tmp/rustycore-585-capture-db.unfTnR`. Its verified datadir is that directory's
`data/`, socket is `mysql.sock`, and `@@skip_networking=1`. The copy contains
41/118/242/444 tables respectively and offline character 14/account 8.
Private snapshot SHA-256:
`4be282882cbafa9b5ca592c45deea506bb21eef7b832d0bce1bda8560be75439`.
The temporary database was cleanly shut down after verification; no reference
world server was started. Both original world-server and bnet-server remain active.
The dump contains private data: do not print, publish or commit it.

Next: configure and prove isolated reference startup, then the bounded paired
logout/disconnect/transfer capture with exact source/build/fixture provenance.
The historical DB2 specialization patch remains a separate known derivation,
not applied to this new build and not assumed unnecessary before startup evidence.
These preparation results neither satisfy the capture gate nor reopen Rust's
already-green code validation.

### Fresh normal-logout comparison: divergence reproduced — 2026-09-07

Isolated C++ startup reproduced two further prerequisites, corrected only in its
private clone: MySQLConnection.cpp mutated the shared host from `.` to `localhost`
on the first socket connection, losing the private socket on subsequent opens;
`c547aba6` uses a connection-local hostname. DB2 loading then reproduced the exact
ChrSpecialization OrderIndex assertion covered by the existing creature-spell
reference patch; that patch is reused unchanged in `8fe8fbf5`.
Current derived reference HEAD is `8fe8fbf57b84f880651662a5cc503ebd1bde3e33`,
worldserver SHA-256 `1901259f3b83377029d05e046c0495c1a37f9030e603a862c656f733a93ff9d5`.
The full four-file derivation is retained at
`/tmp/rustycore-585-cpp-reference.3xdDma/runtime-reference.patch`, SHA-256
`444c4b63186407d3385da6534c568ee48e8d7136d1bd9a300fb49def20a9b65b`.
This is a derived reference, not unmodified C++.

Both worlds used localhost:18085/18086 and the private database socket; an independent
BNet process used localhost:18081/11119 with the copied auth schema. Original services
were not restarted or replaced. The initial 180-second startup probe expired before
the bot connected; its connection-refused result is not a server behavior failure.

The next C++ login/logout failed the bot's unchanged-row check: factions 910, 946,
978, 1119 and 1126 changed flags from 0 to 2 (AtWar), standing still 0. C++
ReputationMgr.cpp:776-778 applies AtWar for hostile rank during loading. The resulting
private auth/characters state was exported as `normalized-session.sql` under the
private database directory, SHA-256
`e81b72e1f9da9471f1db4e43b35f53d3b7751a6f30704f321e5dc54020e6a3dc`.
The C++ repeat then passed login, LogoutComplete and the six save projections.
The same snapshot was restored for the Rust side, where the bot failed because
those five flags returned to 0. Do not weaken that assertion or label Rust's run
passing. The precise reputation cause remains to be established; the reputation
manager source is unchanged from integrated `59f5bced`, but that alone is not a
before/after runtime reproduction. No reputation repair is included here.

Evidence directory: `/tmp/rustycore-585-cpp-reference.3xdDma`.
C++ report `cpp-stable.json`, packet log `cpp-normal-stable.pkt` (SHA-256
`1e0484edbc89895f4297603f668de3d7fa870d8a1a3a2557628c295a20be2ffe`);
Rust report/log `rust-normal.json`/`rust-normal.log`, raw dump `rust-normal/`.
Rust executable is the previously identified `bf884aec` release candidate;
bot executable SHA-256 is `e28908e4024ce126db635116a1153befbe917cbc4dec612c791ced669c914211`.

`target/debug/capture-diff diff --cpp <cpp-normal-stable.pkt> --rust <rust-normal/> --from-opcode c2s:0x34D6 --until-opcode s2c:0x2684 --direction both --strict`
exited 1: one matched packet, one value difference, one route difference and sixteen
missing packets. Full result is `normal-unfiltered-diff.txt`. The proposed time-sync
filter was independently rejected because the two sides have different counts;
no asymmetric filtering or accepted baseline was installed. LogoutResponse differs
in its Instant bit; the pre-#585 handler already unconditionally used instant_ok.
C++ also has stand/root, group/aura/combat/object effects absent from this Rust window.
Those differences are not silently converted into completed #585 functionality.

The directly in-scope LogoutComplete route is corrected in the Session finalization
adapter from the active instance sender to `realm_route_tx().send_async`. Packet
bytes, obligation ordering, bounded backpressure and failure classification remain.
A private test exercises a saturated realm with an empty instance, successful exact
publication, then a closed realm without instance fallback. The reviewed logical
delta is one production separator and 60 test lines, with no field/API/clock growth.
The follow-up correction needs its validation and a new installed capture; the earlier
green final run and this failed paired run cannot be relabelled as testing that fix.

Both private world/BNet processes were stopped and the temporary MariaDB shut down
cleanly after the comparison. Both original services were verified active. No fresh
paired disconnect/transfer acceptance or whole-issue completion is claimed.

Local correction evidence above `c07e9c39`: the new realm/backpressure/failure test
passed. The first complete wow-world run had 3,784 passes, one ignored and one
failure in the unchanged `adjust_client_movement_time_uses_clock_delta_or_cpp_fallback`:
its `assert_ne!(adjusted, 1000)` coincided with the valid fallback clock value 1000.
The unchanged-code repeat passed 3,785 tests, zero failures, one ignored. Both logs
are retained as `logout-route-world-tests.log` and `logout-route-world-retry.log`
in the evidence directory; the initial failure is not relabelled green. The reviewed
hotspot ratchet passes. This does not yet validate a rebuilt/installed closing SHA.

### Realm publication recapture and final validation — 2026-09-07

Correction candidate `07698639cde82d983b64562ace8b4a2a6c87782a` passed
`./tools/validation-v2 final --base origin/3.4.3` (all 14 commands), manifest
`target/validation-v2/manifests/20260907T141018.250049Z-2726418-final.json`.
The manifest truthfully records dirty=true: the sole untracked file was the unrelated
`docs/architecture/lfg-343-audit.md`, with no build/test influence; tracked files were
unchanged throughout validation. Release production integration also passed all 34 tests.
These are aarch64 results, not hosted x86_64 evidence.

The rebuilt release executable SHA-256 is
`2fd79300b0ea5d0274578470752dbe620c7386a94fbf6eb3bc7c2b3d152d79f0`.
It was actually started on the isolated database restored from the same normalized
snapshot, with the independent BNet and pinned bot described above. Fresh raw capture
`rust-route-fixed/` and bot report/log `rust-route-fixed.json`/`rust-route-fixed.log`
are under the existing private evidence directory. The bot still failed the unchanged
reputation-row assertion; this is not a whole-scenario pass.

`target/debug/capture-diff diff --cpp <cpp-normal-stable.pkt> --rust <rust-route-fixed/> --from-opcode s2c:0x2684 --until-opcode s2c:0x2684 --direction s2c --strict`
passed: one matched LogoutComplete, zero value/route/missing/extra differences.
Output `logout-publication-diff.txt` SHA-256:
`18475e76b91212876e0f7f8114fd90250cd236339d70c7b54cec8f4f49f7afd7`.
Raw-file hash manifest `rust-route-fixed-files.sha256` SHA-256:
`73278699b972b13d750304a4c86bd6db57667e1b02d84cc54730e5e018b3a3b8`.
Capture-diff executable SHA-256:
`49f5d22c9ae53b14b6bc278de421332b5bd3a167f4f6eca13d6e1100293f6cfa`.

The complete CMSG_LOGOUT_REQUEST to SMSG_LOGOUT_COMPLETE comparison still exits 1:
two matched, one value difference, zero route differences and sixteen missing packets
(`normal-after-route-diff.txt`). The narrow successful comparison proves only the
publication correction; it does not replace the full failed comparison. No ignore rule
or golden was changed. Paired disconnect/transfer evidence remains outstanding.
The isolated world/BNet processes were stopped, the private database shut down, and
both original services verified active without replacing their executables.

Read-only investigation also found InitializeFactions differences at reputation indices
37 and 93 (C++ flags 16, Rust 18). These two wire differences have not been connected
to the five saved faction rows above; neither their cause nor a pre-#585 runtime
reproduction is established. Keep reputation diagnosis separate from acceptance of
the publication fix and do not silently expand this delivery into gameplay repairs.

### Bounded faction-catalog correction during final QA

Further read-only inspection reproduced an input decoding defect: FactionStore used
`get_array_i32` for fields 14/15, which reads raw record bits even for PalletArray.
For faction 910 the loaded base was 84936240, while the compression-aware reader
returns -42000. The five affected factions have distinct reputation indices, ruling
out an index alias for these rows. Exact C++ anchors are DB2Metadata.h:3567-3591
(signed int32[4] ReputationBase/Max) and DB2FileLoader.cpp:847-858
(`RecordGetVarInt` PalletArray lookup followed by payload-bit copy).

The bounded correction changes only FactionStore's two arrays to use the existing
compression-aware `get_array_element(..., 32) as i32`. The generic WDC4 helper and
its other consumers remain unchanged; this is an intentional catalog correction,
not structural movement. It is needed to investigate the persisted-reputation
guarantee exercised by #585's live QA, not an all-catalog repair campaign.
`crates/wow-data/tests/faction_reputation_arrays.rs` exercises the actual loader on
synthetic palette/uncompressed records, distinct indices, zero, negative and signed
boundary values. A separately ignored host-data test checks the five real faction
bases; it must be explicitly run and must not silently pass when data is absent.
Both tests passed with `cargo test -p wow-data --test faction_reputation_arrays -- --include-ignored`
using the existing validation target and CARGO_INCREMENTAL=0. The installed scenario
and final profile must be rerun for this code delta; the green `07698639` evidence
does not validate it or establish that all reputation discrepancies are resolved.

At `04c4a914`, both loader tests passed, including the explicit host-data test;
the synthetic regression also failed as expected against the retained pre-fix library
(`faction-before-regression.log`, palette case). The real Faction.db2 SHA-256 is
`75ca205dd3a9e88eed6099c7ae77aa9f074184aea07dd9786443ee315b2b4d73`.
Release build passed; executable SHA-256
`c90771a53a89802cfd70c9ff7a86daf3e7207f90e1f60c0d669d025e64e8ef73`.
All 34 release production-integration tests passed. Installed isolated QA still failed
the same five saved rows (`rust-faction-fixed.log`/raw capture `rust-faction-fixed/`).
However, strict comparison restricted to InitializeFactions (s2c:0x2724) now passes
one matched packet with no differences (`faction-initial-diff.txt`). This proves the
catalog correction, not final save retention. The final profile was deliberately
interrupted with SIGINT after this failure exposed a further correction; manifest
`20260907T143919.566382Z-2752391-final.json` is interrupted, not passing evidence.
The isolated processes and database were stopped; originals were not replaced.

### Repeated map-entry identity initialization loses loaded reputation

The fresh packet contains flags 2 for all five affected faction indices, but the
subsequent saved rows contain 0. Inspection locates a second
`ensure_login_player_controller_like_cpp` in `handlers/character/session_state.rs`
inside `send_login_sequence`, after initial packets. Its existing-owner path invokes
`set_loaded_player_identity_like_cpp`, which unconditionally initialized reputation
again. C++ CharacterHandler.cpp:1070/1141/1176 loads, publishes and adds the same
Player; map admission does not repeat ReputationMgr::Initialize. The relevant Rust
call and unconditional initialization also exist at integrated `59f5bced`; this is
a source-confirmed inherited defect, not yet a baseline live reproduction.

The bounded correction initializes when race/class changes or canonical reputation
is empty, preserving already-loaded state for repeated identity/location/level updates.
It adds no mirror, lock, field or public API. The private character-consumer regression
`repeated_login_attachment_preserves_loaded_reputation_for_final_save` reproduces both
attachment calls with loaded hostile flags, nonzero standing and clean save state;
it also covers location-only updates and a real race change. Reviewed logical delta:
Session +7 production lines; character tests +82 lines including registration.
The focused regression passed (one test, 3,786 filtered out), as did the reviewed
hotspot ratchet and cargo fmt check. Full and installed acceptance remain to be
recorded at the new candidate; these focused results are not whole-issue closure.

At `48b3729b`, release build and all 34 production integration tests passed.
Executable SHA-256 was `1453e0dd42c011ffd17eaad44d83907753e6d2ec8cfc556fc0e99e2a97f57fa4`.
The installed normalized-fixture run (`rust-retention-fixed/`, matching bot log)
preserved flags 2 for all five faction rows, but failed the next unchanged-row
assertion: skill 45 disappeared. The original services were not replaced; the
isolated Rust world/BNet processes stopped normally. Final validation did not pass:
`20260907T145830.398534Z-2766232-final.json` rejected the seven-line physical Session
growth. That transition requires an explicit reviewed physical ceiling update,
not a regenerated baseline or a claim that the earlier final run covered it.

### Persisted skills are not new skill acquisition

The normalized C++ fixture has `(guid=14, skill=45, value=1, max=15, professionSlot=-1)`.
Rust loads 14 rows, logs `Skipping forbidden persisted skill` for 45, then saves 13.
The same warning exists in the earlier `rust-route-fixed-server.log`; it is not
introduced by the reputation-retention correction. Raw SkillRaceClassInfo.db2 has
matching records 126 (race mask 650, class mask 4, availability 1) and 127 (32767,
13, availability 0), both flags 128 and tier 0. The current general acquisition
lookup calls that overlap Indeterminate. Treating it as a forbidden persisted
skill loses an existing row.

C++ Player.cpp:25723-25800 `_LoadSkills` does not read Availability or MinLevel;
its normalization reads the skill range/tier and subsequent level-update flags.
The bounded load contract now accepts matching candidates only when flags and
skill tier agree, while retaining rejection for malformed/missing sources or
disagreement in those consumed fields. Only conflict diagnostics may be bypassed
under that agreement. General acquisition lookup remains fail-closed and unchanged;
no arbitrary unordered candidate or default permission is introduced.

Mechanical commit `aec5fda0` moved the unchanged hydration method to private
`wow-data/src/skill/loaded.rs`; the separate repair changes its load-only lookup.
The two new private tests cover the actual bow overlap in both record orders,
continued acquisition rejection, forbidden class, conflicting flags/tier and
missing effective metadata. Existing range/step tests remain registered in skill.rs.
`cargo test -p wow-data --lib skill::` passed all 33 tests after the repair.
This is not a general redesign of how login handles genuinely indeterminate
metadata or full Login-side SaveToDB composition; that boundary remains under #584.

### Fresh EOF/portal references and portal contract correction

Using the same derived C++ executable and normalized private database, the orderly
EOF scenario passed with the original pinned bot: disconnect confirmed, character
and Login account offline, six projections unchanged. Evidence `cpp-disconnect.json`
SHA-256 `31bb8f287d17506c24385ec3c57f6abee1f5c513a595f776eda9a81381ad37b0`,
capture `cpp-disconnect.pkt` SHA-256
`2ffad3f438d01e806582d35e20d3edb99f755f57fbc9c630a9148ebb72b2a5eb`.

The private copy was restored, then only character 14/account 8's seven location
fields were set to the already-reviewed trigger 2173 fixture. The first portal run
failed the bot because its NewWorld check incorrectly expected reason 0. The C++
capture carries reason 16 at payload offset 28 and orientation pi at offset 16.
Exact anchors: Player.h:769-770 (`NEW_WORLD_NORMAL=16`, seamless=21),
MovementHandler.cpp:251-255, MovementPackets.cpp:696-702; ObjectMgr.cpp:7032 converts
WorldSafeLocs.Facing from degrees before constructing the destination.

The Rust ordinary far-transfer adapter now sends reason 16; packet documentation
is corrected. The production area-trigger composition converts the raw persistence
row's SQL Facing to radians, without changing that SQL/source DTO or introducing
a second conversion in the generic Position type. This closes the previously
identified destination-loader gap for this composed path. The bot now checks the
exact normal reason and pi orientation, with negative checks for the old wrong values.
The actual composition/order tests passed (2), the realm transfer test passed (1),
and the bot portal test passed (1). Reviewed logical increases: character tests +4,
world-server tests +14; production LOC is unchanged for this portal correction.

Rebuilt bot SHA-256 `dfdee794935eb9578d691c30030351502759e2fd83825de20cbb3dd82ecf6395`
then passed the fresh C++ pending-transfer scenario: no WorldPortResponse, saved
destination map 369, both offline marks and all six projections retained.
Report `cpp-pending-transfer-corrected.json` SHA-256
`b0e31d820166ca8732b985d1aa74d27f839ac5e00f91075f512da8c6510fb540`;
capture `cpp-pending-transfer-corrected-bot.pkt` SHA-256
`0ebf6c4f4db04907754841c21369fdebeee07c34d4e3642f40153d8e1e5b0fd1`.
Both portal-reference processes subsequently exited 139 when requested to stop;
the second crash occurred after the bot had confirmed the save. This is not a
successful reference shutdown, and its cause is not established. Packet/SQL
observations before that stop remain distinct from shutdown evidence. BNet stopped
normally, the private database was shut down, and original services were untouched.
The corrected Rust portal and persisted-skill paths were subsequently exercised
in the combined installed QA below; no acceptance assertion or capture filter was waived.

### Combined installed candidate and final acceptance findings

On aarch64, clean QA checkout `5f5e225f` produced world-server SHA-256
`c4a1b0cd14f94ff220a39caa56b0fafec9e8c849b6d23d7b0c71cfe93a6ab8fa`.
Release `production_login_player_owner` passed 34 tests. With the pinned corrected
bot above, all three two-pass scenarios passed the maintained save/disconnect
aggregation checks: normal logout/relogin, orderly EOF disconnect/relogin and
pending-transfer disconnect/relogin without WorldPortResponse. Each scenario began
from the same normalized private database; only the portal fixture's location was
changed. Six persistence projections survive, including 207 reputation rows and
14 skills. EOF and portal first-pass saved projections also equal their C++ reports.

Private aggregate reports under `/tmp/rustycore-585-cpp-reference.3xdDma`:

| Report | SHA-256 |
| --- | --- |
| `rust-closing-normal.json` | `3c0b21a475b8bad60ab5a47b4f4bb8cf8d276636204d11d826653f80651f0837` |
| `rust-closing-disconnect.json` | `81eafa2ac2309312e0e63503cd62b037a59e6f7168dd57d33f391db891a971ad` |
| `rust-closing-portal.json` | `fe6a534adf68fb4ffb7a248ceabfe035b53f5a8ff2ba76422dd768ca4c2bbbd5` |

Strict one-packet comparisons match bytes and connection for LogoutComplete
(`0x2684`), InitializeFactions (`0x2724`), TransferPending (`0x25CD`) and NewWorld
(`0x2594`). These are deliberately scoped results, not whole-action parity.
The full normal window still reports 2 matched, 1 value difference, 16 missing
packets, zero routing differences and zero extras. The portal window, from
CMSG_AREA_TRIGGER (`0x31D6`) through SMSG_NEW_WORLD, reports 5 matched, zero value
differences, 1 CancelCombat routing difference, 2 missing packets (CancelCombat
and UpdateObject), zero extras. The full logs are `closing-full-logout-diff.txt`
and `closing-portal-diff.txt`. Their residual admission/combat/object side effects
are not implemented by claiming the represented finalization ledger complete;
acceptance must resolve their scope explicitly before issue closure.

All isolated Rust world/BNet processes stopped successfully after their scenarios;
the private MariaDB instance shut down normally. Original world/BNet processes
remained active and the original world executable retained SHA-256
`c2a3b461132553156cb341933afa832424479f7efcdb2d555c647381b528ae46`.
Reference C++ portal shutdown failures above remain distinct and unresolved.

Final manifest `20260907T152501.562479Z-2778496-final.json` at `5f5e225f`
failed: an ambient recording test received SEL_ENUM from concurrently running
`an_untraced_transaction_records_nothing`. That opt-out test did not acquire the
existing process-wide capture-test mutex. Its test-only correction acquires that
mutex and asserts the ambient recorder is absent, replacing the meaningless
assertion on a never-installed recorder. All 358 database library tests pass with
two ignored after the correction; production behavior and test line counts are
unchanged. A fresh final profile remains required, not a relabelled earlier pass.

The exhaustive ownership attempt also exposed one unregistered test-fixture
constructor: `crate::session::lifecycle::finalization::tests`, WorldSession::new,
10 arguments, cfg(test), count 1. The reviewed policy adds exactly that record,
without granting a production surface. Policy SHA-256
`3612c5388d799f1ec0ab0e70a7e16ee656e33f4a09f074ff673815fc8c1730bb` matches
the private reviewed copy supplied to the exhaustive check, which subsequently
passed. The first failed attempt did not compare persistence.

### Closing local validation — `b8895373`

`validation-v2 final --base origin/3.4.3` completed successfully, manifest
`20260907T155105.620844Z-2788490-final.json`, exit 0. Its complete library suites
passed: world-server 569, wow-data 724, wow-database 358 (2 ignored), wow-packet 724,
wow-world 3,786 (1 ignored). Ignored tests are not passing evidence. The corrected
transaction trace suite also passed 20 repetitions with eight test threads.
The installed production executable remains the tested `5f5e225f` build: the
subsequent delta is one cfg(test) repair, the reviewed constructor policy and docs,
not a claim of reinstalling `b8895373`.

The exhaustive command `session-ownership-check check --policy <reviewed-policy>`
passed using the byte-identical policy identified above: 283 production and 433
test-fixture Session fields, 51 impl owners, 3,704 associated items, 590 registry
rows, 7,777 production plus 2,329 test-fixture persistence rows, 1,029 semantic
groups and 65 bridge rows. The additional tracked-policy syntax-only check passed.
The reviewed policy was prepared while final validation ran at `5f5e225f`; the
later one-test isolation change does not affect this Session persistence inventory.
Normal semantic/physical ratchets pass. Global physical terminal acceptance still
reports the 100 inherited file ceilings owned by #584, not a completed C0–C4 program.

Tracked files matched the committed candidate at final completion. The manifest
truthfully retains dirty status for unrelated untracked `docs/architecture/lfg-343-audit.md`
(SHA-256 `1a9155fbc06617201dc65ace2170e5885532e55c9dcb389e95a0a31b1de285dc`),
which has no build/test influence and was neither edited nor staged.

Residual review boundary: `59f5bced` already sends `LogoutResponse::instant_ok()`
in `handlers/character/world_entry.rs:104` and sends CancelCombat through the
active packet route in `session/mod.rs:13071`. C++ MiscHandler.cpp:238-291 instead
selects timed admission and stand/root state; Opcodes.cpp:1209 places CancelCombat
on realm, through Player.cpp:20620-20623. These are verified inherited gaps, not
new finalization regressions. Full normal-window group/aura/object cleanup and the
additional portal cleanup packet remain unproven beyond the scoped results above.
They are not silently implemented, waived or bulk-closed here. Review the bounded
capture contract and remaining responsibility assignment before merging; do not
interpret these local passes as full C++ action parity or a new whole-port audit.

### Residual scope disposition — reviewed above `2096fd17`

The #585 issue explicitly covers the currently represented obligations and says not
to turn unrelated pre-existing gaps into prerequisites. Applying that existing scope
does not authorize dropping a failing included obligation. The review disposition is:

| Finding | Disposition and retained responsibility |
| --- | --- |
| LogoutComplete route, retained transfer destination, faction/skill retention and normal NewWorld payload | Included-path blockers, corrected in the separate behavior commits above; installed scoped captures and paired persistence/relogin scenarios pass. |
| Instant-only logout versus C++ timed admission, stand/root and timer-window traffic | Inherited admission gap, not implementation of the new outcome coordinator. Retain under #584's C0/C1 lifecycle work; this delivery does not claim complete logout request/cancel parity. |
| CancelCombat route and additional portal combat/object cleanup | Inherited transfer/publication gap. Retain under #584's C1/C3 boundaries until assigned to an analyzed implementation family; no automatic second family is selected. |
| Normal-window group/aura/object cleanup | Full gameplay cleanup composition is not among the represented obligations. Retain under #584 C1 and the corresponding gameplay owners; individual packet causes remain to be characterized there. |
| Derived C++ portal process exit 139 on shutdown | Reference-runtime defect of unknown cause. Do not claim successful C++ shutdown or use the captured save as proof of shutdown correctness. The isolated Rust shutdowns passed. |

Evidence for the boundary, not merely the age of the code: the baseline explicit
logout path in `handlers/character/world_entry.rs` already performs directory removal,
visibility notification and exact canonical retirement without the C++ group/aura/
combat cleanup sequence. The new `FinalizationStep::Retirement` preserves those
three calls and makes retirement failure explicit. `wow-map/src/manager/player_owner.rs`
has no diff against `59f5bced`. The production body of `transfer_completion.rs` is
unchanged; its delta adds a destination regression test. The Session combat helper
and its CancelCombat emission are unchanged; Session's production delta contains
the ledger, TransferPending route correction and hydration guard, not deletion of
combat or aura cleanup. C++ Player.cpp:1391 and :1454 perform combat stop and source
map removal during far-transfer preparation; WorldSession.cpp:637-672 contains the
broader group/social/destruction sequence. This comparison does not assign an exact
cause to every missing UpdateObject/AuraUpdate packet.

Consequently the broad divergent windows remain negative full-parity evidence,
but do not require implementing those excluded families to accept #585's bounded
outcome/retirement contract. No capture filter, golden, inventory row or issue
acceptance checkbox is changed to manufacture parity. A later reproduction showing
loss of an included authority, failed-save misclassification or replacement retirement
would reopen this acceptance regardless of whether its cause predates #585.

The local delivery is ready for the single PR's publication/review with these limits
disclosed. No additional implementation is selected here. Full Login-side save,
general crash recovery and the remaining #584 C0–C4 requirements stay open; #583
does not become unblocked. This disposition creates no push or merge authorization.
