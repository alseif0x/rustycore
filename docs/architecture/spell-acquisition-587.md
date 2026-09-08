# Represented spell-acquisition application boundary — #587

## Current acceptance — 2026-09-08

The represented application boundary, its #588 visibility prerequisite and the
bounded trainer visual codec repair are implemented locally. Both paired trainer
and controlled ordinary-effect scenarios now **PASS** acquisition, normal logout,
fresh authentication and all six retained persistence projections. Combined final
validation **PASS** at `b48a1cfb47888cc1a63581816bc57136d74c8602`: 14 commands,
5,824 Rust tests passed/two ignored, plus 20 physical-policy tests, zero failures.
The manifest `/tmp/rustycore-587-combined-final-manifest.json` was independently
verified green; log `/tmp/rustycore-587-combined-final.log`. Duration 1255.967s,
one Cargo job, aarch64, peak child RSS 4,893,940KiB. Its truthful `dirty:true`
records only unrelated `docs/architecture/lfg-343-audit.md`, preserved at SHA-256
`1a9155fbc06617201dc65ace2170e5885532e55c9dcb389e95a0a31b1de285dc`.
Tracked files still matched the tested HEAD when final finished. Subsequent
evidence wording is a documentation delta, not a relabeling of the tested SHA.
Local scoped acceptance is complete; publication/merge retain their separate gate.

Runtime source: `3a6346300ac912209b45258870315dfcc76d9c65`; Rust world executable
SHA-256 `57267b35b7a8ad686e277301d7bcc91e93fdf602d3efcf819c546c7d72d0e866`,
bot `3e382c6022a31d854812020d538c71c114cc158e6addad2f6c7a78a896d9f1bb`.
Release build: `/tmp/rustycore-587-corrected-release.log`, 12m07, aarch64, one
Cargo job. The C++ derivation retains its source/binary identity documented below.
The later generator/reference-document changes do not alter either executable.

Artifacts under `/tmp/rustycore-587-runtime.MYPSPW/`:

`final-acquisition-qa-evidence.json` consolidates executable identities, all four
verified paired reports and their file/capture hashes.

- `cpp-trainer-candidate` / `rust-trainer-candidate`: observed NPC purchase6197,
  one fee1140, repeated purchase rejected, saved/relogged money998860 and direct
  acquired spell retained. Their `.acquisition.json` and ordinary paired reports
  pass. `trainer-candidate-typed-comparison.json` verifies exact visual fields,
  connection, one learning after visuals, rejection and requests against each
  independently observed live NPC identity. The repaired player visual and
  LearnedSpells match bytes exactly. Full-window comparison still fails:
  two matches, four dynamic-NPC body differences, one missing/one extra money
  UPDATE, no connection mismatch (`/tmp/rustycore-587-trainer-candidate-window-diff.json`).
- `cpp-cast-direct` / `rust-cast-direct`: source30798 known and target6197 absent
  at login; explicit CMSG_CAST_SPELL reaches learning, saves active6197 and
  retains it at fresh login with money1000000 unchanged. The same six-family
  checker passes without an exception. CMSG_CAST_SPELL and LearnedSpells match
  exactly in the action window; the isolated LearnedSpells and LogoutComplete
  comparisons also pass (`/tmp/rustycore-587-cast-direct-{learned,logout}-diff.json`).
- The full cast window remains divergent: two matches, one SpellGo mismatch,
  missing SpellPrepare and SpellStart, no connection mismatch
  (`/tmp/rustycore-587-cast-direct-window-diff.json`). The inherited player cast
  pipeline (`handlers/spell.rs:756`, unchanged from the integration base, and
  `session/mod.rs:70847`, pre-existing metadata publication) differs in cast IDs,
  visual ID, flags and target representation. Those broader casting contracts
  remain #584/gameplay work; this acquisition boundary does not establish full
  player-spell protocol parity or stock30798 acceptance.

The accepted synthetic fixture is **version2**: the existing SpellMisc bit-clear
plus SpellEffect705389's TriggerSpell674→6197. Talent classification, target
selection, effect36 and all other bits remain unchanged. `cast-direct-data/fixture.json`
records all three locales, IDs/parent relation, bit145/width20 and changed bytes.
SpellEffect enUS/ruRU output SHA-256 is
`c5e3eb1cc6378674e29e0696fa0fb85df84ac4ec7ba9e8d98992cf55b6913661`; esES is
`50c34e2634ae7b8126d667fdec182fe8395e5456b7c9206fedde749d1de6d0ba`.
Generation with Python `-O` and four negative cases pass; evidence is
`/tmp/rustycore-587-direct-fixture-generation.json` and
`/tmp/rustycore-587-direct-fixture-negative.json`. Effective SQL/hotfix/dependency
override counts are all zero (`cast-direct-effective-metadata.json`); those
unchanged world/hotfix inputs were shared by both runtimes.

Version1 (30798→674) proves C++ learning and first-save skill118=1/1, but fails
the unchanged relog preservation check: stock SkillRaceClassInfo132 flags0x92
and SkillLine118 category6 legitimately normalize it to100/100 at level20.
Rust's load path represents that same rule. This is not a malformed fixture
value or repaired gameplay bug. Its failed reports remain diagnostic evidence,
and the stable-skill persistence mode is not claimed live-accepted from it.

All isolated world/BNet/database services are stopped after QA; original services
remain active and were never replaced. No real-stack restart/crash durability,
full-window parity, issue closure, push or merge is claimed. The earlier above-cap
Defense restriction remains explicitly retained below. Required #584 core still
precedes #583, then #153 and #133; no subsequent family is selected.

## Diagnostic sequence and dependency

At combined code `3ab2e3c3`, release executable SHA-256
`eaf0419412fdaf3d800ab3efc23a345d50f4f1a92675564889012c7c381f74a1`,
the stationary bot now observes the trainer CREATE and reaches gossip/list without
movement. The first purchase was rejected with the old fixture. Temporary
diagnostic instrumentation confirmed complete spell/skill authority and all 43
login roots `CoveredWithoutNode`; it has been removed. Its executable SHA-256 was
`19cded852e0781537372cad6d698318c12f1d881d8978bb782e64064be6f6408`,
patch SHA-256 `e613f398845c23e2c639a830a7b1dca24f513516c027df157c5d2b0c616e5808`.

That fixture retained Defense95 rank300/max100 after setting level20. C++
`Player::_LoadSkills:25767` and `UpdateSkillsForLevel:5604` preserve rank when
reducing the maximum; Defense's flags lack ALWAYS_MAX_VALUE. This can also follow
`.character level`, not only direct fixture SQL. Rust loads it equivalently but
the pre-existing acquisition planner rejects `skill_value_above_maximum`
(`planner/mod.rs:228`, originating in `f32aa90b4`). The refactor preserves that
restriction; ordinary-fixture acceptance does not establish parity for above-cap
skills after level reduction. This retained gameplay boundary must not disappear
from later core/parity work.

For the ordinary paired scenario, the authorized private fixture now changes
only Defense95 from300/100 to100/100 after restoring the same immutable snapshot.
`ordinary-fixture.json` records that delta. Derived C++ purchase/rejection/save/
relogin passes (`cpp-trainer-valid-report.json.acquisition.json`). The next Rust
attempt exposed a bot phase-continuity defect: initial login had already observed
the trainer UPDATE_OBJECT, but the subsequent drain required another one. The
bot correction now retains instance publication across both phases in a private
`login_stream` module (91 production/225 test lines); six focused tests pass in
`/tmp/rustycore-587-login-stream-tests.log`. The root shrinks to20,952 lines and
its physical ceiling is tightened. Renewed paired trainer acceptance now passes
with that bot (`bb1953fe`, executable SHA-256
`1cea4c812655cf7b1eecb88e54f1048de7d04058cb341d21ccddd048af037681`):
`cpp-trainer-fixed` and `rust-trainer-fixed` each buy6197 once, reject the repeated
purchase, save money998860 and retain the acquired spell through fresh login and
all six preservation projections. Artifacts are under the same private runtime
directory. LearnedSpells and LogoutComplete independently match byte-for-byte
and by connection (`/tmp/rustycore-587-trainer-learned-diff.json` and
`/tmp/rustycore-587-trainer-logout-diff.json`).

The complete buy→repeat-rejection window fails strict comparison: one match,
five body mismatches, one missing/one extra UPDATE_OBJECT, no connection mismatch
(`/tmp/rustycore-587-trainer-window-diff.json`). Three body differences are the
different live NPC counters in requests/rejection. The two visual packets expose
an inherited codec defect (`d7a224e85`): PlaySpellVisualKit writes a raw GUID,
whereas SpellPackets.cpp:780 and the fresh C++ capture use PackedGuid. Its bounded
repair is separate from the application refactor and requires renewed acceptance.
C++ emits visuals→learning→money UPDATE; the represented Rust money fence emits
money UPDATE→visuals→learning. This preserved contract is not a C++ full-window
ordering pass. Both publish exactly one learning packet and charge once.

The first controlled cast timed out on C++ CastFailed66/ITEM_GONE. The bot wrote
an extra ScriptVisualID that neither C++ CombatLogPacketsCommon.cpp:164 nor
Rust SpellCastVisual::read consumes. The corrected request passes five focused
bot tests (`/tmp/rustycore-587-cast-wire-tests.log`) and C++ subsequently publishes
LearnedSpells674 (`cpp-cast-wire-fixed.pkt`). Its direct character_spell-row check
then fails: skill118=1/1 is saved, while674 is dependent and has no direct row.
C++ EffectLearnSpell→LearnSpell(false)→SetSkill→LearnSkillRewardedSpells promotes
the target to dependent (Player.cpp:2812); _SaveSpells:20399 excludes it, and
_LoadSkills:25815 reconstructs it. The bot must explicitly verify that skill
root and fresh-login knowledge; this failed assertion is not yet relog acceptance.
The maintained driver now declares `persistence: {kind: "skill", id:118,
value:1, max:1}` for this case, propagates it unchanged to the verification
login, requires the exact saved root and no active direct target row, and records
`saved_spell` literally alongside `persistence_verified`. The default trainer
contract still requires a direct active spell row. No preservation check is waived.

Correction acceptance at `bb1953fe` plus the reviewed local diff: nine focused
bot tests pass (`/tmp/rustycore-587-derived-persistence-tests-3.log`), acquisition
report positive/negative and contract-propagation checks pass, and the unchanged
six-family report check passes its positive and eleven negative cases. The first
bot compile rejected a Rust2024 let-chain in the standalone Rust2021 tool; the
next run exposed serde's unit-variant handling of surplus fields. Nested syntax
and an empty struct variant correct those failures; neither failed run is green.
The fresh C++ visual golden passes in wow-packet
(`/tmp/rustycore-587-visual-codec-test.log`), and all39 trainer tests pass
(`/tmp/rustycore-587-visual-trainer-tests.log`). Runtime repetition remains pending.

The user-requested complementary AzerothCore source was inspected at
`a5e0e6b8f2bf878cb45cb1dc2251eb1448b9bbc3`. It corroborates skill-based
reconstruction (SpellMgr.cpp:1488, Player.cpp:5582,12231,14073), but its
EffectLearnSpell:2565 and learnSpell:3415 use a different temporary/spec-mask
representation. A reentrant reward returns when the spell is already active;
PlayerStorage.cpp:7885 can therefore persist the directly acquired row. This is
source-level contrast, not an executed AzerothCore scenario or a reason to
replace the captured Classic dependent-root contract. Version differences are
concrete: AzerothCore NPCPackets.cpp:48 reads GUID64+SpellID, whereas Classic
NPCPackets.cpp:245 reads packed GUID128+TrainerID+SpellID. The 3.3.5 wire layout
must not replace the target-version request.

### Earlier paired evidence before #588

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
Combined trainer runtime acceptance passes; controlled effect/save/relogin and
the final combined candidate remain pending. The order remains required #584
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

## Earlier structural acceptance and evidence

The initial structural slice completed implementation and test integration before
live acceptance. The following record retains its original SHA and evidence limits;
the current acceptance result is at the top of this checkpoint.
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

At that stage local acceptance was complete with the explicit terminal/live limits above and
below. No previously green functional suite was rerun merely for test relocation,
import cleanup or the final documentation-only evidence update. Publication final
then awaited an authorized committed candidate.

Live trainer/learn/save/relogin and fresh action captures require separately scoped
runtime authority. Existing regressions or mocks do not establish fresh capture
parity or live durability. The initial slice had no commit, push, merge or runtime
authority. Later scoped commit/runtime authorization and completed bot actions,
recorded above, supersede that preparation-only boundary; publication remains gated.
At that initial stage the integrated bot could inspect trainer lists and run save/relogin, but had
no trainer-purchase or this EffectLearnSpell action scenario. Fresh acceptance must
use a real client or first add the missing bot actions; a trainer-list smoke is not
acquisition QA. Do not assume the historical PM2 capture wrappers fit current systemd.

## Live acceptance protocol and initial preparation

Initial preparation on 2026-09-08, before the paired executions recorded above.
The executable preparation is read-only with respect to
runtime and databases; building it does not authorize installation. Before live
execution, record a committed candidate/source identity and executable hash.
`CARGO_BUILD_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo build
--release --locked -p world-server` completed successfully on aarch64 in 14m20s.
Artifact: `target/release/world-server`; SHA-256:
`baba7e609c345cf6fc6088fcd7471643149ca36273344ea41077d69efbb5880b`.
Log: `/tmp/rustycore-587-release.log`. The Rust source digest above was recomputed
and matched after that build. It had not yet been installed or run live at that point.
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
