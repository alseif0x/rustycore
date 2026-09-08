# Represented spell-acquisition application boundary — #587

## Current live result and dependency — 2026-09-08

At QA-driver commit `f13fd26c`, the derived C++ trainer acquisition/repeated-buy/
logout/fresh-login scenario **passes**; Rust's identical fixture **fails before
purchase**, because the stationary client never receives the trainer CREATE_OBJECT.
This supersedes the earlier preparation-only status below. #587 remains open;
neither paired trainer parity nor ordinary-effect live acceptance is established.

The private fixture is existing TESTBOT1, account 8 / character 14, race 10,
class 3, level 20, beside entry 16673 / SQL spawn 57663 on map 530. Trainer 7,
menu 6652, signed wire option -1702912 offers spell 6197 for 1,140 copper.
The original level-3 snapshot first caused expected level-dependent skill maxima
to change during login; its failed preservation check was retained. After that
normalization, only the purchased 6197 row was removed and money restored to
1,000,000 in the disposable, offline fixture. Both accepted C++ and failing Rust
runs started from this same normalized auth/characters snapshot:
`normalized-trainer.sql`, SHA-256
`ff2737c5802a50c81805ee7fce965dffc3d626726d27823196489dd19042daae`.
No preservation assertion was weakened.

C++ receipt: observed live trainer counter 130 (not SQL spawn 57663), learning
on instance, repeated purchase rejected, saved money 998,860, spell active and
enabled after confirmed logout, retained after fresh authentication, and all six
existing preservation projections pass. Intermediate DB money remained 1,000,000
until normal SaveToDB; it was not required to change during the action.
Private command driver: `run_plan.py cpp-trainer-normalized`, which invokes the
maintained `run_spell_acquisition_relog.sh` with disabled provisioning and pinned
private socket/loopback endpoints. Artifacts under
`/tmp/rustycore-587-runtime.MYPSPW/`:

- `cpp-trainer-normalized-report.json`: SHA-256
  `1e46c54e8635afb3c0c24701ec22dd6c82e4f389ed559113a0a479ef1090fc7e`.
- `cpp-trainer-normalized-report.json.acquisition.json`: SHA-256
  `35a5685d46751b201d371789b26898c164c4c39877592aa6aa0b3ed537c5cc61`.
- `cpp-trainer-normalized.pkt`: SHA-256
  `347766bbd37bc8d8bcebaa289863967af0fab463e9fc4d5af8d453eabf31c488`.
- `rust-trainer-report.json.first.json`, `rust-trainer-dump/`: failed admission
  evidence, not a successful acquisition capture. World executable SHA-256 is
  `baba7e609c345cf6fc6088fcd7471643149ca36273344ea41077d69efbb5880b`;
  the core source identity recorded below is unchanged.

The reference is the already disclosed four-file C++ derivation from #585,
source `8fe8fbf57b84f880651662a5cc503ebd1bde3e33`, executable SHA-256
`1901259f3b83377029d05e046c0495c1a37f9030e603a862c656f733a93ff9d5`.
It is not an unmodified or infallible reference. Original world/BNet services
were never replaced or stopped. All #587 isolated world/BNet/database units
were stopped afterward; both original services were verified active. The private
database retains disposable QA state. No push, PR or merge was performed.

### Approved visibility dependency #588

Fresh Rust evidence records the active-mover ACK and 215 loaded creature records,
but no trainer publication. `session/mod.rs::apply_move_init_active_mover_complete_like_cpp`
intentionally omits the deferred notify. This predates #587 (`b53204122`). C++
`Player.cpp:23045,23322` and `MovementHandler.cpp:808` gate visibility before ACK
and queue its notification afterward; the fresh reference capture confirms NPC
publication. The old Rust comment's blanket suppression rationale is insufficient.

Astra specialist (`gpt-6-astra`, high) independently inspected the current owners,
without edits or validation. `ManagedMap::update` already runs relocation notifies
after move-list drains, but its plans are evidence-only, use empty prior client
membership and have no production publication consumer. Calling a full refresh
directly from the ACK would bypass the deferred phase.

Approved separate #584 macro #588: **complete deferred player visibility publication**.
The existing canonical map phase selects owned notification intents; after map
guards are released, world-server routes them to the matching live Session/Player
incarnation and residence. Session retains its single client-membership ledger and
ordered publication. Consumers are wow-entities notification state, wow-map's
existing phase, world-server's tick/delivery bridge and wow-world's directory and
visibility adapter. No new scheduler, clock, Player mirror or map-lock delivery.
Admission must cover seer/detection, reciprocal visibility and initial visible-unit
packets within the represented scope; empty-ledger plans are not a faithful substitute.

Acceptance: stationary login publishes no NPC before ACK and eligible objects after
the map phase without movement; repeated ACK gives no duplicate CREATE; inactive or
nonexpired grids retain work; mailbox saturation, logout, transfer and stale
incarnations cannot lose or misapply delivery. Exercise the actual production
map-loop/directory/Session path, then repeat paired captures and #587 trainer QA.
The bridge is implemented in `328b721f`, with 97 focused tests passing before its
commit and committed-candidate final validation passing (nine commands; 5,094
tests passed, two ignored, zero failed). Its owning
[checkpoint](deferred-visibility-588-checkpoint.md) records the exact represented
scope and retained visibility limits. #587 is rebased onto that dependency;
backup branch `backup/587-before-588-da2c2949` preserves all original commits.
Combined runtime acceptance remains pending. The order remains required #584
core → #583 → #153 → #133.

The combined architecture check passes at rebased code `5654f6ce` with the local
documentation reconciliation (`/tmp/rustycore-587-combined-architecture.log`).
Session root is 76,107 physical lines. Combined logical attribution is 83,278
production / 108,260 test / 191,538 total lines; ceilings are tightened to those
measured values, retaining the remaining #584 C2/C4 responsibilities. Earlier
standalone measurements below retain their original candidate identities.
The reconstructed root checker also passes syntax ownership: 54 impl owners,
3,725 exact associated items, unchanged 283 production/433 fixture fields,
39 commands and 592 registry rows. Log
`/tmp/rustycore-587-combined-ownership-2.log`. The earlier log without `-2`
used a cached checker bound to the #588 checkout and is not combined evidence.

### Controlled ordinary-effect fixture

The exploratory stored-source fixture 30798 → 674 was rejected by the driver:
the target was already known at login, before any cast. Besides dependent learning,
the source has `SPELL_ATTR1_CAST_WHEN_LEARNED` (`0x80000000`), which C++ AddSpell
casts even while loading. The temporary DB2 inspection and failed `cpp-cast`
report are retained privately. This is not a successful ordinary-effect action;
stock 30798 is unsuitable for this explicit-cast scenario.

The approved controlled fixture preserves Talent 1690 and EffectLearnSpell
705389 (30798 → 674), clearing only SpellMisc 336029 Attributes[1]
CAST_WHEN_LEARNED. This is paired conformance with synthetic metadata, not stock
30798 gameplay or a production behavior change. The integrated generator
`tools/wow-test-bot/prepare_spell_acquisition_data.py` creates a new private Data
overlay and verifies stock files remain unchanged. On 2026-09-08 it passed with
Python `-O` for enUS, esES and ruRU, changing exactly one byte per SpellMisc;
existing output and source-tree destinations were also verified rejected.
The manifest is `/tmp/rustycore-587-runtime.MYPSPW/cast-data/fixture.json`.
Effective SQL hotfix verification and both served cast/save/relogin runs remain
pending. Their preflight must observe 30798 active and 674 absent after login.

Current QA update, 2026-09-08: publication `final` passed at `9cf85e51`
(`/tmp/rustycore-587-final-manifest.json`, verified green): 13 commands,
3,793 wow-world tests passed, zero failed, one ignored. The manifest's dirty
status includes the unrelated LFG document. Later bot changes at `1c87ae95`
passed their five affected tests; that later SHA is not the final manifest's SHA.

The first isolated trainer run entered the derived C++ world successfully but
failed before purchase because no trainer CREATE_OBJECT was visible. Captures
and `Player.cpp:23045` / `MovementHandler.cpp:808` identify the missing client
MoveInitActiveMoverComplete ACK, already used by the bot's other NPC scenarios.
The acquisition driver now sends that ACK at LoginVerifyWorld and applies its
discovery gate to every normal login exit. Its five affected tests, executable
build, physical-file policy and diff hygiene pass. This is a QA-driver correction;
world-server Rust inputs and its previously built executable are unchanged.
The ACK recheck discovered the actual trainer and received its gossip menu, then
failed before purchase because the fixture sent SQL OptionID `0` instead of wire
GossipOptionID `-1702912`. The captured signed ID matches the existing menu 6652
row and C++ NPCPackets.cpp's signed field. The driver now permits signed IDs and
documents that distinction. Its five affected tests and build pass again; the
new executable hash is `98cfa383a394418c89c4951d9634cf4207a5290e5a628389802ba4f08b55bcb4`.
The failed run is not acquisition, save/relogin or packet-parity acceptance.
Private evidence is under `/tmp/rustycore-587-runtime.MYPSPW/`; original services
remain untouched. Live trainer and ordinary-effect acceptance remain pending.

Per the user's latest direction, C++ is a behavioral reference, not correctness
proof. Suspected defects require contrast with invariants, data and actual
captures; intentional behavior corrections remain explicit and separate from
this structural change. Neither a passing Rust test nor a C++ observation alone
settles a disputed behavior.

Authorization update, 2026-09-08: the user explicitly granted full autonomous
control to continue. The earlier pending-commit authorization is resolved; the
local candidate is being committed and prepared for the remaining scoped QA.
Earlier no-authorization statements below describe the evidence at that time,
not a continuing request to reconfirm. Runtime work still requires identifying
the concrete target and preserving unrelated data and services.

Committed locally: orchestration `1a89df32`; acquisition boundary `43c4e801`.
The unrelated LFG document remains untracked and untouched. The action driver is
now implemented in `tools/wow-test-bot/src/spell_acquisition.rs`, with normal
logout orchestration retained in `login_save.rs` and the paired wrapper
`run_spell_acquisition_relog.sh`. It supplies trainer acquisition/repeated-purchase
rejection, explicit self-target learning casts and fresh-login verification; it
does not provision/clean fixtures or claim paired capture acceptance by itself.
Scope and typed plan inputs are in `tools/wow-test-bot/RUSTYCORE_SMOKE.md`.

Bot acceptance: first compile found a missing Clone derive for its report type,
corrected without changing scenario behavior. The full bot suite then passed 149
tests and rejected one permissive verify-plan decoder. Replacing the unit variant
with an empty struct variant made the strict unknown-field test pass on focused
recheck. The other 149 tests were not repeated for that correction. Report checks
pass positive plus six negative acquisition cases and positive plus eleven retained
login-save cases. Shell syntax, diff hygiene and physical migration policy pass.
Logs: `/tmp/rustycore-587-bot-tests.log`, `/tmp/rustycore-587-bot-tests-final.log`,
`/tmp/rustycore-587-bot-plan-recheck.log`. Live execution remains pending.

Selected and authorized on 2026-09-07 against integration `8c47af95`, after #585
closed through #586. Parent #584 retains all other core work and the gate before
#583; #153 remains the terminal audit. This is the owning #587 checkpoint.

## Contract

The responsibility is normal trainer acquisition and player-target EffectLearnSpell,
including the retained bounded fallback. The deterministic planner and canonical
Player remain authoritative. Application sequencing consumes operation-specific
capabilities; Session adapts catalogs, Player access, persistence and transport.
No new runtime owner, lock, task, state mirror or crate is introduced.

The trainer adapter preserves NPC/provenance admission, feign-death ordering,
dirty-state save before admission, admission rechecks after the money fence, and
the existing battle-pet saga routing. The acquisition operation prepares and
preflights before the existing fee/acquisition commit. It retains the exclusion
through runtime installation, money publication, instance writer fence, realm
visuals, realm writer fence, skill fields and learning actions. Its completion
retains the guard while the handler publishes a rejection or quarantines the
session, then releases it before draining quest progress. Cancellation still
uses the existing money commit fence; unknown COMMIT is not rollback.

EffectLearnSpell chooses the validated plan or bounded fallback synchronously.
Both mutate canonical Player dirty spell/skill state and use ordinary SaveToDB;
neither forces a character transaction during a cast. Fallback preserves complete
spell-map admission, ranked-insertion rejection, disabled dependency recursion,
trait/override removal, favorite/dependent bits, temporary-row replacement and
ordered publication. The existing shallow fallback's restricted coverage is not
expanded into missing gameplay here.

## Source and consumer boundary

C++ under `/home/server/woltk-trinity-legacy/src/server/game`:

- `Handlers/NPCHandler.cpp:132`, `HandleTrainerBuySpellOpcode`: admission and routing.
- `Entities/Creature/Trainer.cpp:79`, `Trainer::TeachSpell`: fee, visuals, cast/learn.
- `Entities/Player/Player.cpp:2741`, `AddSpell`, and `:3192`, `LearnSpell`: spell state,
  disabled/rank/dependency handling and learning publication.
- `Spells/SpellEffects.cpp:2025`, `EffectLearnSpell`: player hit target and learning.
- `Entities/Player/Player.cpp:20399`, `_SaveSpells`, and `:20348`, `_SaveSkills`:
  ordinary dirty-state save; `:5635`, `SetSkill`: skill transitions.

Rust implementation targets:

- `wow-world/src/spell_acquisition`: existing planning/preparation, generic runtime
  application, trainer purchase, effect/fallback decisions and runtime adaptation.
- `wow-world/src/session/{trainer_acquisition,effect_learning}.rs`: existing
  persistence/transport and canonical Player adaptations; effect dispatcher remains
  a caller. The generic application does not import WorldSession or packet types.
- `wow-world/src/handlers/trainer.rs`: admission, decoded input, saga routing and
  result handling; its existing tests are split by scenario under `trainer/tests`.
- `wow-entities` Player remains the mutation owner; `wow-persistence` requests,
  `wow-database` transaction/reconciliation adapters and `world-server` injected
  catalogs/ports retain their APIs. Login/save readers remain consumers of the
  same Player spell/skill dirty state. No cross-crate API change is planned.

Full talents, combat, pet/item/mount learning, unimplemented passive/criteria
effects, map scheduling/storage and SDK work are outside this represented cut.
These exclusions do not establish full gameplay parity or remove #584 debt.

## Acceptance and evidence

Implementation and test integration are complete locally; live acceptance is pending.
Parent is the sole validation executor. Astra high settled the bounded
runtime contract; Luna owns common application/runtime adaptation and tests;
parent owns trainer/effect consumers, fallback, checkpoint and policy integration.

Physical retirement at the local candidate: trainer handler 3693 → 846 lines;
application 3588 → 1458; Session root 76584 → 76131; Session tests 96722 → 95703.
New files remain below 2000 lines. The 1801-line application scenario suite and
1018-line effect suite preserve their cohesive fixtures/scenarios; they are not new
global fixture containers. The Session roots retain other #584 responsibilities;
their reductions are not global physical closeout. Four existing ceilings tighten.

Executed on aarch64, base `8c47af95` plus local uncommitted code:

- `cargo check -p world-server` with `CARGO_BUILD_JOBS=1` and configured PROTOC:
  PASS (4m29s), with existing warnings; no runtime installed.
- `cargo test -p wow-world --lib`: 3792 PASS, one ignored, one failed source-string
  test. Splitting the trainer suite exposed its self-matching obsolete thunk text.
  The corrected test inspects the actual inventory registration; the unchanged
  adjacent packet-dispatch test exercises that call.
- `cargo test -p wow-world --lib handlers::trainer::tests::failures`: all seven PASS
  after that test-only correction. The other passing tests were not repeated.
  Logs: `/tmp/rustycore-587-check.log`, `/tmp/rustycore-587-lib.log`,
  `/tmp/rustycore-587-trainer-recheck.log`. No later SHA is claimed as tested.
- `cargo test -p wow-world --test production_handler_registry_contract --test
  production_login_player_owner`: PASS, 1 exact registry + 34 production-linked
  Player/login/save/finalization scenarios. Log: `/tmp/rustycore-587-integration.log`.
  These are production-linked controlled-port scenarios, not a live DB claim.
- Exhaustive `session-ownership-check print-persistence-baseline`: all 10,106
  references match `persistence-access-snapshot.json` exactly. The snapshot and
  persistence policy are unchanged. `cargo test --release --locked --manifest-path
  tools/architecture/handler-contract-check/Cargo.toml persistence_policy --lib`:
  five PASS, including checked snapshot/policy consistency.
- Initial architecture acceptance found Session's aggregate test ceiling exceeded
  by four lines after extraction. Removed the two obsolete test-only Session
  facades and directed those scenarios to the application entry points; no ceiling
  increase. An intermediate patch command failed before replacing the callers,
  and its subsequent test build failed on those missing methods. This failed run
  is not counted as evidence; the complete caller correction is validated below.
  Neither correction changes production persistence accesses, so the exhaustive
  inventory above remains applicable.
  The first recheck still exceeded the aggregate by one line; the extracted suite
  now imports its application module consistently with the other scenario suites.
  The final compile checks that import-only adjustment; no assertion changed.
- `cargo test -p wow-world --lib session::tests::effect_learning_tests`: all 17
  PASS after retiring the test-only facades. Log:
  `/tmp/rustycore-587-effect-final.log`.
- Final `session-ownership-check check --syntax-only`: PASS, 54 impl owners,
  3,724 associated items, unchanged 283 production / 433 fixture fields and 590
  registry rows. Policy adopts only the reviewed three trait impls and method
  relocation/removal; no other baseline surface changes. Log:
  `/tmp/rustycore-587-ownership-final.log`.
- `python3 tools/architecture/check_architecture.py check`: PASS after reconciling
  the documented sequence with the selected issue. Dependencies, ownership,
  aggregate hotspot ceilings and physical migration policy pass. The earlier
  missing #587 sequence row was corrected in `ownership-and-boundaries.md`.
- `python3 tools/architecture/check_architecture.py self-test`: PASS, including
  20 physical-file tests and architecture/debt/runtime/hotspot fixtures.
- `cargo fmt --all -- --check` and `git diff --check`: PASS.
- `python3 tools/architecture/check_architecture.py physical-files --terminal`:
  FAIL on 98 retained oversized migrations. Trainer and application no longer
  fail; other Session, catalog, loader and tooling responsibilities remain under
  the core umbrella. This is not terminal physical acceptance of #584/#133.
  Logs: `/tmp/rustycore-587-architecture-accepted.log`,
  `/tmp/rustycore-587-architecture-self-test.log`, `/tmp/rustycore-587-terminal.log`.
- `VALIDATION_V2_CARGO_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc
  VALIDATION_V2_MANIFEST=/tmp/rustycore-587-quick-manifest.json ./tools/validation-v2
  quick --base origin/3.4.3`: PASS, six planned commands, including `cargo check
  --locked --tests --jobs 1 -p wow-world` and 20 physical-policy tests.
  `./tools/validation-v2 verify --manifest /tmp/rustycore-587-quick-manifest.json`:
  verified green. Manifest records `dirty: true` at integration `8c47af95`.
  Final evidence prose in this checkpoint and STATE is a documentation-only delta;
  it does not alter the tested Rust inputs. Log: `/tmp/rustycore-587-quick.log`.

Final local Rust candidate identity (21 changed/untracked wow-world `.rs` paths,
sorted; SHA-256 over each path + NUL + file bytes + NUL):
`04786cc4f5c28ee54dc35ea22da77fb03c2144e5d545df7cd1c9d0da9bcaabb9`.
The path manifest is `/tmp/rustycore-587-source-identity.txt`. HEAD remains
`8c47af95`; this identifies a dirty candidate, not a committed/tested new SHA.

Local acceptance is complete with the explicit terminal/live limits above and
below. No previously green functional suite was rerun merely for test relocation,
import cleanup or the final documentation-only evidence update. Publication final
waits for an authorized committed candidate.

Live trainer/learn/save/relogin and fresh action captures require separately scoped
runtime authority. Existing regressions or mocks do not establish fresh capture
parity or live durability. No commit, push, merge or runtime operation is authorized.
The current integrated bot can inspect trainer lists and run save/relogin, but has
no trainer-purchase or this EffectLearnSpell action scenario. Fresh acceptance must
use a real client or first add the missing bot actions; a trainer-list smoke is not
acquisition QA. Do not assume the historical PM2 capture wrappers fit current systemd.

## Pending live acceptance protocol

Rechecked on 2026-09-08. The executable preparation is read-only with respect to
runtime and databases; building it does not authorize installation. Before live
execution, record a committed candidate/source identity and executable hash.
`CARGO_BUILD_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo build
--release --locked -p world-server` completed successfully on aarch64 in 14m20s.
Artifact: `target/release/world-server`; SHA-256:
`baba7e609c345cf6fc6088fcd7471643149ca36273344ea41077d69efbb5880b`.
Log: `/tmp/rustycore-587-release.log`. The Rust source digest above was recomputed
and matched after the build. This executable has not been installed or run live.
`tools/qa-runtime.sh:302` rejects a dirty source checkout, even for its dry run.
Use an isolated checkout of the authorized committed candidate; preserve the
unrelated LFG document and orchestration work in the original checkout. Do not
misrepresent the integration SHA as the source of the modified executable.

The smallest live scope uses a disposable character with recorded initial money,
spells, favorites and skills. Discover a valid trainer offer and a represented
player-target learning spell from the actual fixture/catalogs; unit-test IDs are
not evidence that either is available live. Do not provision accounts, relocate
characters, seed spells or rewrite money without explicit fixture authority.

| Scenario | Required observation |
|---|---|
| Normal trainer purchase | Actual NPC admission and valid offer; one fee deduction; expected canonical spell/skill acquisition; instance money publication before realm visuals and realm visuals before instance learning publication. |
| Rejected or repeated purchase | Applicable rejection with no extra fee or duplicate learning; preserve unrelated character state. |
| Ordinary player-target EffectLearnSpell | Actual spell effect reaches the included path; canonical dirty spell/skill change without forcing a trainer-style transaction. |
| Logout and fresh login after each acquisition | Confirmed logout, persisted acquisition and unchanged unrelated projections; fresh known/favorite-spell packets agree with saved state. |
| Paired C++/Rust action captures | Same initial fixture and action; compare affected bytes, connection and observable ordering, including both transports. Record broader divergences separately rather than claiming full-window parity. |

The existing `run_login_save_relog.sh` can corroborate retention **after** an
acquisition, but cannot perform the two acquisition actions. A real client is an
acceptable action driver; automated execution first needs those bot actions.
Cancellation/unknown-COMMIT remains controlled-test evidence unless a separately
authorized failure scenario is actually run. No crash experiment is implied.

Any authorized service swap must restore and verify the original executable and
serving state, retain structured scenario/capture reports, and leave bnet untouched.
Source preparation, runtime execution, fixture writes, publication and merge are
distinct authority scopes; none is inferred from a passing build or this protocol.
