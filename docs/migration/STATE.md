# RustyCore — Honest Current State (single source of truth)

**#43 current summary — 2026-10-29, branch `43-spirit-healer`:**
Local closeout preparation now includes native capture collection under the
existing systemd swap/restore guard (`645499a92`), observed NPC and in-memory
character selection (`603e911c5`), disposable fixture preparation/recovery
(`56a9a802a`) and packet-driven action sequencing (`8a5f498a0`). The latter's
focused coding feedback passed 42 cases before subsequent formatting; it is
not a committed-candidate final verdict. Affected acceptance at `1c0a2418c`
passed all 25 Python fixture/scenario tests, including the pinned C++
character-schema check and ASCII-case identity regressions, plus all 195
runtime-guard fake checks (22.328 seconds combined command time).
Logs and exact command timings are retained under
`/tmp/rustycore43-repair-evidence-56qnmiik/`.
No new fixture rows, live captures or real service operations were performed.

The first final campaign at `d795b7496` did not pass. Its manifest is
`target/validation-v2/manifests/20261028T212816.755377Z-1468914-final.json`.
Architecture reported seven unreviewed logical-owner count deltas, an unresolved
test import provenance and one unclassified construction-time policy setter.
The full workspace/all-targets check then reached its 900-second timeout while
checking the world-server test target, without a compiler error being reported.
The complete attempt, including the guard suite, took 1,048.717 seconds on this
aarch64 host with one Cargo job; the ordinary 600-second target was not met.
Library execution, production integration and candidate builds had not started.
The next candidate explicitly binds the test crate alias, classifies the
two-field configuration installer in the existing construction API and records
only the seven reviewed #43 owner deltas with their #584 retirement conditions
intact. These repairs still require renewed final evidence; no failed manifest
is relabelled as passing.

The follow-up final at `e91fab1cc` also failed. Its manifest is
`target/validation-v2/manifests/20261028T214951.143007Z-1477475-final.json`.
All 17 independently selected `wow-world-*` crate test-target checks passed,
but the workspace check found two inaccessible calls to the unchanged shared
corpse-driver test helper. Architecture also found the undeclared Application
`rand` dependency and stale exact syntax inventories. After collecting the
independent checks, the parent interrupted library compilation to repair these
known findings; no library test execution or final pass is claimed. The runner
recorded 829.778 seconds and exit 130; the ordinary 600-second target was not met.

The current repair moves that helper unchanged into its common test parent.
It declares `rand` only as the algorithm library already used by native SelfRes
selection and transform display selection (`Player::InitializeSelfResurrectionSpells`,
`ObjectMgr::ChooseDisplayId` and `AuraEffect::HandleAuraTransform` at the pinned
C++ revision). The syntax review changes nine exact inventory sections:
50 added/21 retired WorldSession item surfaces, one Core admission impl and
six added/one retired Core item surfaces, the catalog-bearing factory/helper
fingerprints, one AuraUpdate command/payload/derive, and 40 added/two retired
directory-access rows. WorldSession fields, impl owners, SessionResources,
Core fields/bindings and bridge rows are unchanged. The reviewed methods borrow
existing canonical owners and immutable catalogs; phase/residence/generation
checks and the established publication rails remain authoritative. This is an
inventory reconciliation, not completion of #584 or new parity evidence.
The fixture preflight also confirmed the exact designated account IDs but found
lowercase DB email storage. Plan and restore now normalize ASCII email case
while retaining exact ID/email bindings and offline/ownership checks. These
repairs remain unvalidated until the renewed committed-candidate acceptance.

The exhaustive repair inventory at clean `1c0a2418c` completed in 986.790
seconds on this aarch64 host, recorded separately from ordinary warm acceptance:
`/tmp/rustycore43-persistence-reconcile-udlawsx_/timing.json`. It collected
10,864 exact rows (8,185 production and 2,679 test-fixture), adding 741 and
retiring 217 stale identities. Review confines production changes to 15
database-adapter sources plus existing app startup composition, and test changes
to five adapter fixture sources. No new direct persistence edge enters
Application or a World domain. The annotations now cover all 1,090 production
workflows exactly; the derived policy has 1,093 groups, with 44 added adapter
groups, eight retired helper groups and six reviewed existing-group deltas.
Character save and each pet-save request retain their existing transaction
owners, statement order and definite-rollback/unknown-COMMIT outcomes. Pure
bindings and decoders acquire no connection. The by-spell lifecycle-aura deletes
are described with their real player/spell keys; obsolete claims that these
particular deletes preserve foreign-caster rows are removed. Unrelated owners,
issue-linked retirement conditions and the generated-input count are retained.
Rendering that policy from the saved snapshot passed; exhaustive comparison
and final acceptance on the resulting committed candidate remain required.
Read-only fixture planning also passed at `1c0a2418c`, proposing five new actors
with GUIDs 20–24 on accounts 9/10/12 while preserving their original offline
characters (`/tmp/rustycore43-readonly-plan-ajtjl7tx/`). No proposed rows were
created. Final Rust suites, production-linked tests, candidate artifacts and
authorized paired captures/DB/restart/relogin acceptance remain outstanding.

The final campaign at clean `0200e83d4` also failed. Its manifest is
`target/validation-v2/manifests/issue43-0200e83d4-20261028T223011-final.json`;
the runner took 1,137.730 seconds (22:30:11–22:49:09 UTC), exceeding the
ordinary 600-second target. Syntax ownership and all 17 selected standalone
domain test-target checks passed. Architecture found one unclassified existing
`SendPlayerAuraUpdate` command, and the workspace check reached the compiler's
query-depth limit in `production_login_player_owner`. Seventeen library suites
passed; Server, Data, Entities and World did not. The World process was stopped
only after identifying its blocked synchronous receive in
`scenarios_world_entities_1/durable_runtime_rail.rs:55`; the exact process,
signal and reason are recorded in
`/tmp/rustycore43-final-20261028-3x6ujald/world-test-termination.json`.
No complete World-suite verdict, production integration, candidate build or
final pass is claimed. Replaying the 183 already-failed World identities against
that same binary reproduced them in 0.37 seconds; its exact arguments, binary
hash and diagnostics are under
`/tmp/rustycore43-world-failure-diagnostic-66x8v912/`.

The current repair classifies that command under the existing Map delivery
owner and raises only the affected integration test crate's query-depth limit.
Three foundation test fixtures now reflect the pinned C++ contracts: GCD remains
separate from recovery time (`SpellInfo.cpp:1261–1265`), zero-duration cooldowns
are not inserted even on hold (`SpellHistory.cpp:406–428,503–513`), and durability
mutation is explicitly marked persistent before save acknowledgement
(`Player.cpp:4610–4616`, `Item.cpp:843` onward). The worker reported passing
focused feedback for those three cases; a separate retained log was not located,
so the final suites remain their required acceptance evidence. Server's empty
tick and completed-tick clock cases passed in the 23:01:55–23:05:52 UTC feedback
run; its capability test then exposed an obsolete ban on the now-required
serverside spell catalog. After correcting that single test contract, its
23:06:46–23:07:08 UTC focused rerun passed. Logs are
`/tmp/rustycore43-server3-20261028-230155.log` and
`/tmp/rustycore43-20261028-230646-scenario4.log`. These runs used dirty trees
based on `0200e83d4`, not a committed final candidate. World failures remain
under review. In particular, 90 World diagnostics reject missing exact aura
SpellInfo keys; fixture repair must preserve the production catalog guard and
the negative cases that deliberately omit a key. These are repair findings,
not new accepted evidence or additional gameplay scope.

Two demonstrated production defects are now repaired locally, still without
renewed acceptance. Release no longer sends an immediate HealthUpdate(1):
`Player::BuildPlayerRepop` uses SetHealth (`Player.cpp:4295`), whose native
field update is distinct from ModifyHealth's nonlethal-damage packet
(`Unit.cpp:9214`, `8115–8154`). The existing field publisher retains that
health change. Sickness save capture now preserves persisted rows if no exact
selected-spell key exists at any difficulty, including an empty application
set; it does not require a difficulty-zero key. Active applications retain
their existing difficulty, attributes and indexed-effect checks. The durable
mailbox fixture now tests a committed nonlethal presentation with bounded
packet reads; the lethal replay fixture expects only its attacker-state packet.
Native death integration owns the death body and durability penalty. The
previous blocking receive therefore cannot conceal another failed suite run.
These four source/test edits have been reviewed against the pinned reference;
their affected tests and final acceptance have not yet been rerun.

The positive Aura-fixture repair now covers 47 World test sources based on
`413d6ce05`, fingerprint
`ce77e0328324080057e3945097ca6add9b9aff8535f2132d625d165667a599ba`.
It seeds explicit exact keys through the existing canonical loader fixture
and retains the supplied indexed effects, attributes and difficulty inputs.
Movement/removal fixtures also retain their actual effect payloads; a key alone
cannot validate an active effect mask. The synthetic presence-only spell uses
an explicit DUMMY effect. Shared sickness and Ghost/Wisp builders preserve
their selected and additional spell keys. The production catalog guards and
deliberate missing-key constructors are unchanged. Static review preserved all
47 files' function identities and assertion counts; one assertion now reads
the canonical Aura owner. The reviewed patch and input hashes are retained in
`/tmp/rustycore43-catalog-review-g3n0frj_/`. This is unvalidated fixture repair,
not a passing rerun of the 90 original missing-key diagnostics. Required forced
visibility and remaining consumer/fixture repairs precede the next acceptance.

The forced-visibility repair now follows `Player::UpdateObjectVisibility(true)`
at the pinned C++ revision (`Player.h:2461`, `Player.cpp:23322–23343`): nearby
sessions are notified before the current Player's forced diff, while an
out-of-world Player is a no-op. `KillPlayer` awaits it after reclaim delay;
resurrection awaits it before obtain spells (`Player.cpp:4437,4373–4376`).
The existing generation/residence fences remain checked across that await.
Three new, unrun tests cover stationary death, real Ghost removal on
resurrection and an out-of-world no-op, with visibility caches already at the
current positions. Native health/power fields retain Map-tail publication.
This reviewed implementation is unvalidated and does not establish capture parity.

The next reviewed, unvalidated fixture repair covers 25 World test sources at
`eac4704dd`; its patch, source hashes and preserved function identities are in
`/tmp/rustycore43-consumer-fixture-review-zc79ry20/`. Quest fields are observed at
Map-tail publication (`Player.cpp:14756`, `Map.cpp:777,1929`); native death packet
prefixes precede the environmental log (`Player.cpp:621–681`). Player handles,
reciprocal combat references, known-empty faction/inventory data and physical
item counts now satisfy the operations being tested. Full inventory saves and
partial durability saves are asserted disjoint. Unsupported mixed Aura casts
must reject without packets or a partial application. No tests were executed
for this repair. The remaining loot-money/Aura fixtures and the real Stuck
death/recovery consumer still precede completed-delivery acceptance.

That Stuck consumer is now implemented locally. One boxed World callback delegates
the cross-domain operation to Application: native SpellHistory selects KillSelf,
dead-without-timer calls the real graveyard operation, and home teleport retains
`TELE_TO_SPELL` (`SpellEffects.cpp:3263–3303`, `SpellHistory.cpp:670–692`,
`Player.cpp:1426`). KillSelf admits death before changing native-width health;
intentional teleports retain the Player incarnation without requiring its former
residence. The temporary FULLMASK Hearthstone cast adds no cooldown
(`Spell.cpp:4195–4203,8034–8038`, `SpellHistory.cpp:252–257`). The existing
immutable graveyard catalog is now required inside the same World capability
bundle; all eight outer bundles remain mandatory. The direct Map dependency
names the existing generation/residence types and adds no mutable owner.
The reviewed 18-path patch and source hashes are under
`/tmp/rustycore43-stuck-review-wuci2kua/`. Application-only coding feedback first
failed on a private visibility-helper export (00:24:28–00:24:39 UTC, exit 101).
After repair, `cargo check -p wow-world-application --locked` passed at dirty
`e13430771` (00:27:54–00:28:28 UTC; log
`/tmp/rustycore43-stuck-app-check-20261029-002754.log`). That feedback did not
check World/Server or execute the branch regressions. It is not final acceptance.

The remaining Aura/interrupt/mount and loot-money fixture repair now covers 13
World test sources at `d440e0c02`. Static review retains their function identities;
the patch and hashes are in `/tmp/rustycore43-aura-fixture-review-jpjwlbkh/`.
Inert active Auras use APPLY_AURA/DUMMY rows, and far teleport checks the same
three applications and their applied-index projections. Negative cancellation
and unknown-metadata cases remain. The worker started an unplanned World
check/focused-test wrapper at 00:46:59 UTC despite the scheduling instruction;
it was interrupted with exit 130. No individual test result was retained, and
no acceptance pass is claimed (`/tmp/rustycore-aura-interrupt-loot-feedback-20261029.log`).

Read-only inspection through unchanged production DB2 readers confirms that
installed spell 7355 allows casting while dead and has a 1,500 ms cooldown;
8690 uses category 1176 with a 1,800,000 ms category cooldown. The three input
file hashes, reader-library hash and command results are retained under
`/tmp/rustycore43-stuck-data-c2miecxc/`. No local scalar override or matching
hotfix-removal record was found for those selected rows. Fresh action captures
remain required; this data inspection is not runtime acceptance.

The 2026-10-29 candidate review records only six measured logical-owner deltas,
the two added/two retired callback identities, and startup composition's one
additional graveyard-catalog line (5,709 lines). Existing owners and #584
retirement conditions are preserved. All 47 Rust formatting changes are exact
rustfmt transformations of `580414a07`; their source comparison, measurements
and reviewed policy patches are retained under
`/tmp/rustycore43-candidate-reconcile-5t9qvhyx/`. This is implementation review,
not acceptance. No new architecture scope or runtime authority is implied.

The existing death QA runner now includes two unvalidated Stuck cases, bringing
the bounded set to 12 scenarios and 18 epochs. Only Qadeathcast gains known
spells 7355/8690. Normal Hearthstone supplies the death case's cooldown; both
logout/relogin epochs require its persisted category 1176 row. Dead repop must
reach graveyard 106 within 0.25 yards, while home Stuck requires a completed
teleport ACK and no Hearthstone cooldown. No fixture, capture or runtime action
has been executed. Hermetic tests and the full committed-candidate campaign
remain required before any accepted result is recorded.

Acceptance at `10a1da1eb` stopped before any heavy check: one of 34 Python tests
raised a missing `tempfile` import (0.226 seconds for the attempt). The log is
`/tmp/rustycore43-final-20261029-ynb9_t0o/qa-python.log`. The import repair and
renewed candidate acceptance remain required; no Rust or runtime result is claimed.

At `39dc59c78`, all 34 Python tests and 195 runtime-guard fake checks passed.
Architecture policy and exact Session syntax passed (70.623 and 55.988 seconds).
The census then reported stale rows/ceilings; workspace test compilation found
two u32/i32 mismatches in the home Stuck fixture. The parent stopped that owned
runner before another compilation of the known failure. The attempt took
287.701 seconds and ended 130; its manifest is
`target/validation-v2/manifests/20261029T011021.172635Z-1574992-final.json` and
logs/termination identity are in `/tmp/rustycore43-final-20261029-hh8e6i2n/`.
It is not a final pass. The repair uses checked protocol-ID conversions and
records the reviewed census as 4,120 tests (same one ignored): four added cases,
one existing Stuck rename, and 87 derived owner/blocker rows. The only class
change is one W-to-H pet test after fake Spell host setters were removed.
The exact census review is under the candidate-reconcile directory above;
no test coverage was deleted and #584's outstanding boundaries are retained.

The next attempt at `50d97fb6e` passed architecture, exact Session syntax and
the repaired census, then found a missing production graveyard setter while
checking Server. The parent stopped the known-failing candidate after 280.744
seconds (exit 130); logs are in `/tmp/rustycore43-final-20261029-mefn75xl/` and
the manifest is
`target/validation-v2/manifests/20261029T011633.592037Z-1577058-final.json`.
The repair promotes the existing three-line catalog setter, removes its
four-line test shim, and updates only that method's cfg/source classification
and the exact logical counts. The census has identical identities/classes and
272,491 unit-test lines. This closes a production composition omission, not a
new gameplay operation. Renewed Rust execution evidence is still required.

The full final campaign at clean `04efcb2da` ended 101 after 1,335.240
seconds (01:23:17–01:45:33 UTC), exceeding the ordinary 600-second target.
Manifest:
`target/validation-v2/manifests/20261029T012317.975844Z-1579088-final.json`.
Architecture, Session syntax, census, workspace/all-targets compilation, all
17 standalone domain checks, SQL composition and the bot compile check passed.
Twenty of 21 library suites passed; World completed with 4,074 passing,
45 failing and one ignored case. The checker suite passed 463 cases and failed
its repository source guard on three local dirty-bit macros and a private
`inventory` module name. Exact failure sections are retained in
`/tmp/rustycore43-final-20261029-yhm_ktm6/world-failures.json` and
`all-failures.json`. Bot execution, production-linked tests and candidate builds
did not start. The unchanged Python/guard evidence remains at `39dc59c78`.
These failures require repair and renewed candidate acceptance; no runtime,
publication or parity completion is claimed.

The source-guard repair expands exactly 13 Unit, two active-field and 19
active-section dirty-bit macros, retaining their comparisons, order, marks
and `active_changed` writes. The existing array macro is unchanged. A static
token comparison confirms equivalence; this is not execution evidence.
The private save helper is now named `item_capture`, with the same physical
`inventory.rs` source and two callers. No persistence statement or owner changed.
This repair remains unvalidated; exact policy/count reconciliation is deferred
until the remaining World fixtures are repaired together.

The seven-case visibility repair now gives the Ghost source and map peer
native cell membership, retains exact Player handles and clears the replaced
bootstrap identity. Exit cleanup reuses its registry's MapManager. SelfRes
checks completed forced scanning rather than the non-forced notification bit;
the Ghost discovery ledger excludes self. Stationary recipient obligations,
both peer GUID assertions, real Ghost removal and deferred native VALUES remain
asserted. Anchors: `Player.cpp:23322–23343,23029`, `Map.cpp:427,450,777,1929`.
These five test-source edits are unvalidated; no production behavior changed.

The closeout physical review retains four unchanged death-driver tests in a
private child and moves four unchanged obtain-item readers out of storage.
That checkpoint retained 4,116 World tests (one ignored) and the same classifications. The
reviewed policy records exact composition/state-definition and test-adapter
deltas with their existing #584 C4 split exits; it does not declare that
architecture programme complete. Candidate formatting and module wiring refine
the World test-line count to 270,837, including the explicit provenance import,
without adding or removing tests. This earlier count excludes the unvalidated
acceptance-fixture repairs above; final formatting and physical review remain due.

The macro remains open and has not passed final acceptance. Changes remain
local to `43-spirit-healer`; this candidate has not been published or deployed.
Native death now fences canonical save capture, Map retirement and same-GUID
replacement while its Player-owned operation is Ready or InFlight. Application
cleanup checks that fence before unpublishing the Player or releasing the login
claim; logout checks it before admitting each finalization step. The existing
deferred-save intent retains save requests across those phases, including a
newer request that must survive an older save acknowledgement. These
guards are committed at `7b09213ca`. The subsequent native-death integration
is checkpointed locally at `8eb73d12a` (**UNVALIDATED**). Its focused coding
feedback preceded that commit; no committed-candidate final campaign or live
acceptance has run.
Application now admits lethal damage before writing zero health and runs the
ordered Player death body, including SelfRes initialization and no-death quest
failure. Fall, void, environmental spell damage, PowerBurn and creature
spell-click use that operation. Nonlethal damage preserves native-width health
and sends HealthUpdate; lethal health uses the existing Map field publisher.
The Map combat integration now awaits a required death continuation on the
existing Session phase rail before proceeding to the next swing or tick tail.
Admission binds the exact Player, registration, residence, Map incarnation,
tick and operation identity. A failed or revoked native operation retains its
Player-owned fence; the producer does not run another tick over that obligation.
The new Map composition has focused production-linked feedback. The
2026-10-27 21:00:50–21:06:09 UTC sweep passed `cargo check -p world-server`
and 15 focused cases on this aarch64 host with one Cargo job: Creature's
`native_combat` (2), World's `native_death_rail_` (5), `native_damage_` (3)
and `native_death_auras_` (2), and Server's `native_map_combat_` (3).
The Server cases execute the actual producer and Session consumer, including
body failure and a pre-existing unresolved operation. The sweep used the dirty
tree based at `7b09213ca`, fingerprint
`fd2f50945f9a54ce963d2d5fcf97b84c49e2e9aa0f8ee6048e8edf22c9b4b9ce`;
its commands, exits, timings and logs are retained under
`/tmp/rustycore43-native-feedback-20261027/`. This is coding feedback for
that tree, not committed-candidate final acceptance or live evidence.
Primary creature melee now also awaits its nonlethal/absorbed-hit presentation
on that rail before the next swing, so an earlier hit cannot remain in the
ordinary mailbox behind a later native death. The native initializer and
selected Aura cleanup still do not claim unimplemented script callbacks.

Focused coding feedback before the latest rail/Aura changes passed Application
compilation, three `native_damage_` World cases (including lethal fall followed
by real release/corpse creation), the Inventory retirement-during-commit World
case, six Quest failure cases, and the affected Inventory filters. These were
dirty-tree checks, not evidence for the later changes or a committed final
candidate. Final acceptance, action captures and authorized live DB/relogin
evidence remain outstanding.

**Earlier slice context:** The following summaries describe their own
checkpoints. Statements about a missing death caller at those checkpoints are
superseded by the native integration checkpoint above; they are not another task list.
The working release path now invokes Application's pet-removal operation.
Its absent-Pet branch resolves only the current Map, retains stale GUIDs,
returns temporary-pet reagents once, and resets the stable selection. The live
route now submits the native state and pet-row transactions separately before
unlinking the Pet and queueing Map removal. Its current admission requires
known-empty loaded and native aura state, empty combat/cast participants,
no summon spell, no group and no secondary controlled minion. The remaining
live-Pet cases still stop release before corpse creation; complete live-Pet
support is not claimed. Its save projection has six passing focused tests,
and all five handler cases pass after correcting their fixture setup. The
owner's minion GUID now has one native `UnitData::Summon` authority, projected
to VALUES and self/observer CREATE. Focused native-field, bridge, byte,
resummon and observer consumer tests pass. This is coding feedback, not final
acceptance or general Pet lifecycle parity.
Earlier focused coding feedback
passed three absent-Pet cases and 75 `release_` cases, including preservation
of an active Pet when removal cannot complete. The cancellation fence now
starts before the first potentially mutating reagent callback, after read-only
admission; Application compilation and 16 `pet_re` cases passed after that
correction. Native lethal-death integration, final acceptance
and authorized live DB/relogin evidence also remain outstanding.
Recent checkpoints add canonical stats, reconciliation of all selected aura
keys, selected load and refresh, signed durations, and an expiration executor
driven by the admitted diff. 9i3C2 connects that executor, selected publication,
and exact snapshots to PlayerRecovery. Its focused coding checks passed;
this does not establish final acceptance or complete AuraUpdate metadata.
The Map recovery pass now connects regeneration, KillPlayer, periodic save,
deathTimer, and delayed teleport through the existing Session rail. Both its
World consumer and Server coordinator have focused coding feedback.
9i3C3c connects the scalar wire projection to pending deltas and initial full
updates, including rejection in login/worldport callers when preparation fails.
9i3C3d3b connects creation flags for fresh selected auras and preserves refresh
provenance when creation-only facts are unavailable. 9i3C3d3c connects canonical
foreign-caster reaction to that constructor and has focused consumer feedback.
9i3C3d4a retains complete indexed effect rows and shares their resolver from Data.
9i3C3d4b connects selected save effects and attributes to that resolver and
uses each application's eligibility flags. 9i3C3d4c connects selected execution
effects, sickness stat readers, and selected initial effect snapshots. 9i3C3d4d
connects selected completion polarity and exact initial-threat attributes, with
required metadata checked before cast preparation. 9i3C3d4e connects selected
load effects, whole-spell offline polarity, full-width charge normalization,
and the shared fresh constructor. 9i3C3d4f adds effective condition-row presence
and its startup/session composition. 9i3C3d4g adds exact visual metadata,
complete effect/visual file loading, override-potential queries, and server
catalog composition. 9i3C3d4h connects those visuals to fresh selected Aura
construction and loading, including canonical preworld owner context and
preservation of signed wire bits. 9i3C3d4i connects selected self-cast visuals
before execution preparation. Subsequent charge and effective scalar work is
recorded below; ordinary client cast preparation remains pending.
9i3C3d4j1 prepares complete/effective loading and SQL-free row types for the
four scalar catalogs.
9i3C3d4j2 normalizes source difficulty before regular/server effect indexing
and preserves canonical server effect slots and duplicate-parent semantics.
9i3C3d4j3 adds the four scalar Hotfix SQL methods and checked native decoding
for all 23 core catalog projections. 9i3C3d4j4 connects those four scalar stores
to their existing startup positions and immutable owners.
9i3C3d4k adds checked key/server/proc/Difficulty inputs and effective PPM loading.
9i3C3d4l1 normalizes canonical server effect base points and preserves full-width
native proc-family metadata.
9i3C3d4l2a shares the existing AuraOptions and regular-family readers from Data,
retaining the World aliases and their previous behavior.
9i3C3d4l2b prepares exact regular/server proc sources and connects the fallible
builder to startup; its focused coding feedback is green. 9i3C3d4m2b connects
its selected fresh and refresh charge consumers.
9i3C3d4m1 prepares selected charge calculation and modifier admission as pure
readers; 9i3C3d4m2b connects their selected consumers.
9i3C3d4m2a shares the existing key-difficulty reader from Data and prepares
loader-backed canonical keys in selected World fixtures. Its initially
deferred compilation and identity test are covered by 9i3C3d4m2b feedback.
9i3C3d4m2b connects fresh and existing-aura charge consumers, canonical key
admission, the bounded nonzero-only selected FULL projection, and its cooldown
correction. Production compilation and 50 distinct World cases have focused
coding feedback. Original refresh policy, duration and Ghost/Wisp effect
properties are corrected by 9i3C3d4m3, with production compilation, 54 World
cases and four Core duration cases. The selected stat-view integration now
uses the Aura's actual difficulty key and indexed effect rows; its focused
coding feedback and remaining boundary are recorded below. None of this
establishes final acceptance.
The ordered SelfRes field now has one native Player owner and both CREATE and
VALUES consumers, with production compilation and 19 distinct focused cases.
Death-time initialization and the ordinary SelfRes cast contract remain open.
The native initializer participant is now prepared with 39 distinct focused
cases across Spell, Data and shared fixtures. Its required Application death
caller and the real gameplay producers remain open; this is not production
SelfRes integration or final acceptance.
Ghost/Wisp now use original Aura keys in loaded effects, transform ownership,
removal and full-save capture. The selected loader no longer requires a
legacy difficulty-zero payload. The scoped checkpoint records 54 passing
World cases, including a genuine nonzero-only save/load roundtrip; this is
coding feedback, not final macro or live acceptance.
The existing Player Unit pass now maintains its native SpellHistory using a
single frozen system-clock observation. The scoped history checkpoint records
94 distinct cases; this does not complete SelfRes or final macro acceptance.
The integration adds neither another clock nor another Player authority.
Native resurrection errors now stop dependent reclaim, spirit-healer and
login work, and the actual Character login consumer aborts on that error.
The scoped checkpoint below records production compilation and 31 distinct
World cases. A subsequent recovery integration retains the native operation
through corpse deletion, save acknowledgement, login and transfer completion,
and prevents retirement while recovery is unresolved. Its scoped feedback
covers 77 distinct World cases, including 33 new recovery cases, and the
Server composition compiles. Normal Player casts now connect pre-GO costs,
native cooldowns and physical-item consumption to the existing save owners;
45 distinct focused cases and Server compilation provide coding feedback.
Common resurrection obtain calls now use live inventory and checked normal
cast outcomes. The selected sickness constructor/refresh now retains its
physical cast-item provenance and passes the restored real regression.
The affected healer, recovery, resurrection, corpse-login and repop groups
now have green scoped feedback covering 148 distinct identities. The stack
abort was corrected in a test fixture. This is coding evidence, not final
macro acceptance. The sickness full-save path now resolves each active Aura's
actual SpellInfo key without a difficulty-zero prerequisite; its 35-case
save/load group has green scoped feedback. The SelfRes effect now uses the
same Player-owned native recovery continuation, including corpse deletion,
save acknowledgement and cancellation retention. Its focused feedback covers
37 Spell and 11 World cases. The shared CheckCast path now rejects ordinary
dead/ghost casters before cooldowns and launch, with 39 scoped cases passing.
Native recovery fields now use the existing Map publication owner and native
visibility notification; its affected groups have green scoped feedback
covering 165 distinct identities. Four registered SelfRes composition cases
now prove the normal cost/cooldown, native revival and conditional full-save
path locally with a genuine nonzero-only key. Its remaining admission/timing
cases, native death, QA and final acceptance remain open.
The native SetDrunkValue participant now serves the existing EffectInebriate
consumer, and Player update-field changes now enter the existing Map queue.
Its scoped feedback covers 26 relevant cases plus one matched duration
regression. The native-death caller is still absent. Normal casts now clear
exhausted cast-item provenance at the Source phase and propagate unavailable
consumption; seven distinct cases provide scoped feedback for that repair.
The cast dispatcher now reports unimplemented effects and unsupported Player
Aura grouping before START/GO instead of silently succeeding. Its scoped
feedback covers 33 distinct cases. This is a failure-contract repair, not a
port of those effects or proof of full parity for every known handler.

**Closure direction reset — 2026-10-27, reviewed at `e26551b0d`:**
The user requested an overengineering review, then authorized continuing to
issue closure. The 92 local commits since `origin/43-spirit-healer` are not
completion evidence. Stop selecting work from an open-ended dependency list;
each additional change must address a concrete failure in the acceptance rows
below. Preserve useful existing work and its regression coverage. Do not require
completion of the general spell, quest, pet, battleground or architecture
programmes to finish this issue.

| Issue #43 criterion | Current evidence and remaining closure blocker |
| --- | --- |
| Lethal damage and native death | The working Application body and five Session damage routes now use Player-owned native operations. The working Map composition awaits the same body on the existing rail before its tail, with pre-Kill creature melee presentation and native durability applied once. Fifteen earlier focused cases cover the selected composition, ordering/failure boundaries and Aura cleanup. The awaited split/share correction has three passing production-linked Server scenarios; connection routing now has separate-channel coding feedback. Exercise the complete death/release/recovery cycle and final candidate. This row is not accepted yet. |
| Release, ghost, corpse and graveyard | Release invokes pet removal for absent/temporarily unsummoned pets and now has a bounded quiescent live-Pet save/removal continuation. Exercise the complete in-scope path from an actual lethal producer, including ghost restrictions and corpse publication. Nonempty Pet Aura, combat, summon-spell, group and secondary-minion continuations remain explicit Pet-domain boundaries; the issue does not require completing the general Pet programme. A demonstrated lifecycle failure in a required scenario must still be repaired. Helper tests alone do not close this row. |
| Reclaim, healer and accepted resurrection | Production recovery continuations exist. Verify the registered paths, penalties, corpse cleanup and pending/transfer recovery together on the final candidate. |
| Persistence and failure recovery | The dirty quest-status full-save projection and commit acknowledgement have focused passing feedback, including concurrent native objective changes and failed/unknown commits. The working native death body now invokes ordered FailQuest and quest-item destruction. Save/retirement fences and these callbacks still need final-candidate and complete-cycle evidence; mocked outcomes do not close runtime persistence acceptance. |
| Local acceptance | Define one committed candidate after implementation, run final plus missing scope-specific integration evidence once, repair actual findings, and retain the real SHA and complete timings. |
| Live acceptance | Required action captures and real DB/restart/relogin evidence remain outstanding. Prepare the exact target and scenarios before requesting the separate runtime authority; no deployment, restart or data mutation is inferred from local implementation authority. |

Read-only runtime preflight on 2026-10-27 at 22:03 UTC used
`./tools/qa-runtime.sh snapshot` (exit 0). The serving target was systemd
`world-server`, PID `3094551`, executable
`/home/server/rustycore/target/deploy/live/world-server`, SHA-256
`290353718a1c534b348d03cb082fe203cf2b7ab0e179ef34d8ef4a8933c80fa4`,
with no configured packet dump. This identifies the existing build for the
later authorized swap/restore; it is not execution evidence for this candidate.

Read-only fixture preflight at 22:59 UTC used a read-only transaction against
the configured reference schemas. Characters 15/16/18 remained offline;
their levels were 80/80/3. None of the four proposed `Qadeath*` names in
`tools/wow-test-bot/RUSTYCORE_SMOKE.md` existed. The listed healer and Kobold
spawn rows were still present. No account, character, item or runtime state
was changed. Recheck those conditions before any authorized preparation;
these reads do not establish a playable fixture or action acceptance.

The 2026-10-28 read-only name recheck also found no `Qadeathcast` collision;
characters 15/16/18 remained offline at levels 80/80/3. The prepared campaign
now uses five new disposable characters, including a Human Paladin caster,
instead of adding a spell or changing the existing character 15. No fixture
rows were allocated. The current-host capture decision reuses the systemd
runtime guard and the existing comparator's raw-pair mode; it retains both
native server captures and the client's action trace, without inventing PM2
provenance or expanding the general capture-import programme.

The reference runtime is available at
`/home/server/woltk-trinity-legacy/build-r6/src/server/worldserver/worldserver`.
Its SHA-256 is
`80f143841b5a7b443022480500760ce336dc83052f05ddd7e3b7be6a6605b02a`;
the read-only `--version` probe exited 0 and reported `a5f8da2ebf54+`.
This is the derived R6 build recorded in
[the reference preparation](../architecture/captures/1263-r6-paired-2026-10-10.md),
not an unmodified upstream executable. Current checksums of all three local
patches match that record: the LFG logging macro correction, explicit
`Spell::SearchTargets` instantiations, and the specialization-index startup
guard. Retain that derivation in later capture provenance. Its existing
specialization/loot limits remain; these death scenarios must not establish
loot or specialization parity from this build. No reference service was started.

The native integration preserves the target C++ contracts at source
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Player.cpp:1158–1200` (Player death order), `14928–14997` (quest failure),
`15626–15672` (quest SpellArea updates), `11784–12035` (quest item destruction),
`Unit.cpp:8527–8585` (Unit death order), `3432–3524,3583–3602,4308–4328`
(applied and owned Aura removal), `1015–1022,8115–8154,9214–9243`
(nonlethal HealthUpdate versus lethal SetHealth), `10632–10650` (PvP death
and durability), `1958–2015` (split damage), `833–856` (shared damage),
and `Object.cpp:2254–2260` (controlling Player lookup).
The scope is the #43 lifecycle, not full Unit::Kill/proc/script, Pet, vehicle,
Battleground or Aura parity. Unsupported vehicle/secondary-controlled and
health/power-trigger Aura participants stop the native operation explicitly.
At the start of this review, the Map route rejected split/share damage before
its old scalar writers, also disabling existing nonlethal secondary damage.
The working correction removes that rejection and uses shared per-entry
calculations from the synchronous compatibility loop and the admitted async
future. Each secondary Player death is awaited before the next Aura entry or
primary health mutation; the current application and amount are reread per
entry. Absorb callbacks precede split, each split death precedes its own
non-melee log, and the primary AttackerState packet precedes shared damage.
Both secondary calls retain the reference's `durabilityLoss = false`.
Five new focused Creature cases pass, covering preserved nonlethal damage,
admission before zero health, source identity and same-GUID record replacement.
Three multi-session Server scenarios now pass: both secondary split deaths
finish before their logs and the primary hit, a failed secondary death fences
the next Aura and primary mutation, and lethal self-share runs one death body
after AttackerStateUpdate. The passing commands explicitly select the Server
library; `TEST-15R.log`, `TEST-16.log` and `TEST-17.log` record real exit 0,
from 23:02:46 through 23:06:55 UTC. This is dirty-tree coding feedback, not
final acceptance or general split/share script parity. Those logs and the
earlier failed attempts are retained under
`/tmp/rustycore43-secondary-native-feedback/`.

The bot's existing scripted runner now has typed death/recovery requests,
raw gameplay evidence with authentication-body redaction, exact packet/route
expectations and distinct logout/disconnect outcomes. It is checkpointed at
`befc96261` (**UNVALIDATED**). Its 27 focused tests passed on the preceding
dirty tree. The initial standalone dependency preparation cost 8m 01s for
`cargo check` and 6m 47s for the focused test build; these are coding/bootstrap
costs, not the ordinary warm final campaign. Logs are under
`/tmp/rustycore43-death-driver-feedback/`.
The subsequent client controller now retains the authenticated login burst,
initializes its actual mover/position, and owns root, water-walk, run/swim speed,
near-teleport and suspend/worldport acknowledgements. Worldport response and
renewed visibility initialization use the same instance socket to preserve
their order. Root clears incompatible movement/fall flags; unsupported
transport/vehicle and other unhandled controls fail explicitly. Initial control,
read, known-spell decode and auth-handoff failures retain their observed evidence
while closing both sockets. The repaired dirty tree passed 34 focused bot cases
(204 filtered), exit 0, at 18:56:25–18:56:35 UTC on 2026-10-28. The scoped patch
fingerprint was `33bfb13b2d4255259d6a7e8cffad4e0597944a0c891beaf12ffc8a3feb5f36fd`;
logs are under `/tmp/rustycore43-movement-feedback/`. The parent then formatted
the owned files. This is coding feedback, not committed-candidate final evidence.
The subsequent observed-target and in-memory character selection passed 40
focused bot cases (204 filtered), exit 0; the final coding-feedback command
ended at 19:50:17 UTC on 2026-10-28. The log
`/tmp/rustycore43-target-selection-feedback` retains the initial test-local
shadowing error, repair and actual repeated command results; they are not
additional distinct test identities. The tracked patch fingerprint was
`636ccd8b67cf4f53d87148ec9837b497f495eda7291ef7091d47238366a4d23d`, and
the new target resolver's SHA-256 was
`120e8edd97366d62ca8ddf3a75e73d6f0624b2c40ec2453cfcee524113568752`.
It resolves only from observed object updates after the current map barrier
and records the chosen GUID's source event. Character overrides happen before
failure-report identity capture, without rewriting credentials. The preceding
private test split retained all seven movement cases; its 535-line moved body
was unchanged, with one surrounding blank line removed. These remain coding
and source-review evidence. Fixture/scenario integration, final and live
lifecycle acceptance remain outstanding.

The current-host native capture collector is now implemented in
`tools/qa-runtime.sh capture`, reusing the existing systemd identity,
stop/swap/restore and exclusive-lock guard. It records native Rust dumps or
C++ PKT, candidate/runner/bot hashes, listener identities, selected-config
digests and restoration outcomes. Its private file helper journals and fsyncs
recovery before configuration mutation; drift or a missing armed journal holds
the service stopped. Fake-service cases are written but have not run, and no
syntax, final, service, database-write or live capture acceptance ran for this
unit. Collection does not establish lifecycle acceptance. The reference source
HEAD, three local patch paths and R6 executable SHA were rechecked unchanged
on 2026-10-28; the reference was not started.

An additional read-only installed-data probe found male models 1/3/7
(display IDs 49/51/55) for Human/Orc/Night Elf; all select sickness spell
15007. Items 2121 and 17030 have no ItemEffect rows. This probe reused the
existing reader artifact for unchanged race/model/effect loaders, not the
subsequently corrected class-mask field. Its compile/run exited 0 in 3.438 s;
the result is `/tmp/rustycore43-fixture-reader-preflight/result.json`. The
selected Kobold's actual World data has one SmartAI AGGRO/TALK row with 30%
chance, not an empty script set. Retain that fact in action-capture analysis;
no NPC row has been changed and none of these reads proves a live fixture.

Final-acceptance preparation reconciles the World test ledger to the real suite:
4,094 mounted `unit_tests` cases and 22 source cases, with the existing ignored
case preserved. The identity delta adds 363 and retires 12 renamed/replaced
cases; all added cases classify as composition tests. Nine existing cases now
classify as domain-movable after their actual forwarding changes. The reviewed
ceiling records exactly 270,614 physical `unit_tests` lines with no headroom.
This metadata update does not claim the suite has passed final acceptance.

The selected ItemSparse class-mask correction now reads target field 46,
not field 48 (`RandomSelect`). A portable sparse WDC4 fixture passed through
the production loader with distinct neighboring fields, positive and negative
signed masks, durability, flags and stack size. The same focused test's
explicit opt-in data probe read class mask `-1` for installed items 2121 and
17030; the Mage bit 128 is permitted. Item 2121 has durability 40 and reagent
17030 has stack size 10. The one test passed with 846 filtered; its log is
`/tmp/rustycore43-item-class-feedback/focused-test.log`. This fixes selected
fixture admission metadata; it does not prove runtime equipment, creation or
durability persistence. No fixture or runtime mutation has been performed.

Cohesion review: `crates/wow-world-creature/src/admitted_creature_execution/native_combat.rs`
has 1,368 lines, including its tests. It owns one admitted combat future and
the identity checks needed across its callbacks; split/share arithmetic already
lives in the shared per-entry helpers. It remains below the 2,000-line closeout
ceiling. The existing private test module is the next independent navigation
boundary if this file grows; this review does not require another runtime owner
or a restructuring of the ordered operation.

The Server scenario exposed a publication boundary: ordinary mailbox delivery
allowed a later priority-rail melee stage to overtake split logs. The working
Server publisher now queues this required pure packet output at its actual
combat call point, including the observer attack prelude. It retains canonical
receiver identity, residence and native source visibility selection, with no
Map guard during delivery. Player-source combat logs keep their direct self
copy separately from observer selection. It reuses the existing connection
writers and does not drain arbitrary Session commands. A separate source
review also found that the affected native
death paths use instance-channel helpers for several realm-channel packets.
Pinned `Opcodes.cpp` assigns HealthUpdate (`1564`), PowerUpdate (`1911`),
StandStateUpdate (`2131`), DurabilityDamageDeath (`1362`) and
EnvironmentalDamageLog (`1373`) to realm; AttackerStateUpdate (`1087`),
SpellAbsorbLog (`2102`), SpellMissLog (`2123`) and SpellNonMeleeDamageLog
(`2124`) use instance. The working owners now use those connections. World
`native_damage_` passed four cases after the owner correction, including
exact nonlethal health bytes and a real native Creature-kill body with one
realm wear message normally and none in battleground context. The absorption
case verifies instance combat-log observer commands and realm power output;
the queued-melee case verifies presentation does not apply another death
penalty. Inventory retains its shared wear calculation, called by Application;
the uncalled queued-melee wear publisher and its host hook were retired.
The legacy test is now named
`apply_creature_melee_damage_command_defers_lethal_durability_to_native_body_like_cpp`;
the native Creature-kill test carries the positive policy coverage.
The Server's disconnected-observer case also passed: closing a selected
observer's packet receiver does not cancel the primary death. Pure packet
queue failures skip that receiver, following C++ `WorldSession::SendPacket`;
required native body/phase failures still retain their operation fence.
The seven focused cases and failed attempts are recorded under
`/tmp/rustycore43-socket-routing-feedback/`. This is dirty-tree coding feedback,
not final or live connection acceptance. The combined-sender Server fixture
proves call/enqueue order, not network order across the two sockets.

The dirty-save stabilization and native death integration now have focused
coding feedback. Completion still requires the complete recovery acceptance,
including prepared action captures and real DB/restart/relogin evidence.
Before the subsequent integration, the 2026-10-27 caller check found
`begin_native_death_operation`, `claim_native_death_operation` and
`complete_native_death_operation` only in their owner and tests. The working
Application and Map callers above supersede that finding.
`has_unresolved_native_death_operation` protects production save capture,
logout and retirement; those fences alone do not complete the lifecycle.
Fence coding feedback on the dirty `d603dbce6` base, aarch64, 2026-10-27:
`cargo check -p wow-map`, `-p wow-world-core`, `-p wow-world-lifecycle` and
`-p wow-entities` each exited 0. Focused tests passed:
`cargo test -p wow-map native_death` (3),
`cargo test -p wow-world-core full_save_capture` (2),
`cargo test -p wow-entities native_death` (1), and
`cargo test -p wow-world native_death_fences_logout_and_cleanup_until_the_operation_completes --lib`
(1). The World case exercises the real save entry, lower unregister, logout
step admission, cleanup and retained login claim in Ready/InFlight, then
permits cleanup after Complete. An initial Lifecycle test fixture could not
compile with that crate's test dependencies; its assertion moved into the
World integration case. The first World build exposed a wrong HighGuid import,
which was corrected before the passing run. All commands used the declared
PROTOC, one job and this checkout's `target/`. This is seven distinct focused
tests, not committed-candidate final acceptance, native death execution or live
DB/capture evidence. Inspection of Server's existing
`session_factory/finalization.rs:126–144` confirms unresolved finalization feeds
its task-owned fail-stop retention; no additional retention task was added.
The pet-save persistence participant now accepts the two independent native
transactions and preserves their statement order and exact bindings. The
parent reviewed `Pet.cpp:452–546,1129–1176,1288–1347`,
`SpellHistory.cpp:182–218`, and `CharacterDatabase.cpp:664–686` at the pinned
target SHA. On the dirty tree based at `4ba72acbe`, aarch64, the worker ran
`cargo check -p wow-database` (exit 0) and
`cargo test -p wow-database pet_save --lib` (2 passed, exit 0 after repairing
a missing test import). Both used the declared PROTOC, one job and this
checkout's target directory. Subsequent changes only format those files and
add license headers. This is coding feedback; no live transaction or final
acceptance ran. At that checkpoint the production RemovePet caller, canonical
aura save capture, reagent refund and failure/cancellation continuation were
unfinished. The current integration boundary is recorded above.
Production pet resummoning now retains the loaded aura's cast difficulty and
indexed base amounts in its existing canonical loaded state, including their
removal lifetime. On the dirty tree based at `ebcb57e15`,
`cargo check -p wow-world-lifecycle` passed and the World
`resummon_pet_temporary_unsummoned_loads_represented_stable_pet_like_cpp` test
passed (1 test; 5m 14s compilation). The same one-job environment applied.
This establishes data retention, not full Pet aura eligibility/application or
production save capture. The metadata is not a second aura owner.
The next local draft composes reagent grants from canonical inventory, quest,
obtain-spell and collection operations, with ordinary Player full-save ownership.
It reuses `Player::send_new_item_plan`, checks new-slot/GUID collisions before
insertion, and derives stack changes under one canonical Player mutation.
A bound-quest preflight which updates no objective now preserves loaded quest
authority, so a subsequent reagent is not rejected solely by that no-op.
The draft still rejects bag placement and equipment, container, timed or charged
item creation. Those are explicit unfinished boundaries, not successful refunds;
the production RemovePet caller and its cancellation fence were absent at
that checkpoint and are now connected for the absent-Pet branch described above.
The empty PetSpells packet matches the pinned default writer's 65-byte body and
opcode `0x2c22`; `cargo test -p wow-packet clear_pet_spells --lib` passed one
case on the dirty tree based at `df4339f3e`. Application compilation also passed
before the subsequent notification/merge refinements. The final focused coding
run, `cargo test -p wow-world pet_reagent --lib`, compiled those refinements and
passed both cases: new item followed by merge with retained quest authority,
and a maximum-count rejection with unchanged storage and no error/push packet.
The preceding post-addition filter selected only the positive case; the final
filter selected both. All runs used aarch64, one job, the declared PROTOC and
this checkout's target directory. These are coding checks, not live captures
or final acceptance. The current absent-Pet removal path now calls the refund
helper. At that checkpoint the clear-PetSpells packet still awaited a live-Pet
caller; the current bounded continuation now sends it after both saves.
The release integration and its current-Map lookup follow
`Player.cpp:20839–20962`; refund suppression reads the native battleground
instance (`Player.h:2335–2337`), not the older represented type-only helper.
On the dirty tree based at `faafebe89`, aarch64, the worker ran
`cargo check -p wow-map` and `cargo test -p wow-map pet_combat --lib`
(4 passed), followed by `cargo check -p wow-world-application`,
`cargo test -p wow-world pet_removal --lib` (3 passed) and
`cargo test -p wow-world release_ --lib` (75 passed), all with exit 0.
An initial World compilation failed on a fixture tuple type mismatch before
running tests; the corrected runs above are the successful evidence.
The worker repeated one Application check after losing its prior command
handle; only the recovered exit-0 result is credited. These commands used
the declared PROTOC, one job and this checkout's target directory.
At that checkpoint the Map participant had no production caller. The current
quiescent live-Pet branch calls it only after proving its combat participants
empty. It can unlink indexed attack, combat and threat references and return
owned effects; native combat-exit
aura/engagement callbacks, the master's pet-combat flag and AI continuations
are not implemented by that helper. It is not complete `CombatStop` evidence.
After the refund-fence correction, `cargo check -p wow-world-application`
and `cargo test -p wow-world pet_re --lib` passed (exit 0; 16 tests, including
all six removal/reagent cases) on the same dirty base and host. Missing
ItemEffect metadata now fails without changing pet markers, inventory or the
persistence tracker. A prior successful grant retains the enclosing fence
through subsequent grants and marker updates. A pending-addon-read
cancellation case was not executed; the available fixture resolves immediately.
Map and release suites were not duplicated after this scoped correction.
No final campaign, live capture, DB/restart/relogin run or publication is
established by these coding checks.
On the dirty tree based at `c6e8c6147`, aarch64, the next worker feedback ran
`cargo check -p wow-world-application` (exit 0),
`cargo test -p wow-world-application prepare_pet_save` (6 passed, exit 0),
and `cargo test -p wow-world resummon_pet_temporary_unsummoned_loads_represented_stable_pet_like_cpp --lib`
(1 passed, exit 0). The latter covers restored Hunter rename eligibility from
`Pet.cpp:298`, so a subsequent save preserves `WasRenamed`. The first
`cargo test -p wow-world live_pet_release --lib` run failed all five cases
before handler entry: the fixture set its Map after adding the object to the
world. After correcting that order and connecting the native Summon field,
the same filter passed all five cases (exit 0). `cargo check -p wow-world`
also passed. The native minion setter, Core bridge and packet CREATE/VALUES
filters each passed one case; their byte assertions pin CREATE offset 46 and
the VALUES mask plus populated/empty packed GUID. Exact successful commands:
`cargo test -p wow-entities minion_guid --lib`,
`cargo test -p wow-world-core bridges_canonical_unit_minion_guid_and_empty_clear --lib`,
and `cargo test -p wow-packet player_create_and_unit_values_publish_summon_and_empty_clear_like_cpp --lib`.
The resummon fixture initially failed after adding native-owner assertions:
its setup lacked the adopted handle/map catalog, then still seeded the
fixture-only temporary marker and left canonical health at zero. The corrected
setup installs the canonical owner, temporary marker and living vitals;
`cargo test -p wow-world resummon_pet_temporary_unsummoned_loads_represented_stable_pet_like_cpp --lib`
then passed one case (exit 0), including rename flags and native control links.
`cargo test -p wow-world player_visibility_create_uses_live_stats_and_customizations_like_cpp --lib`
also passed one case (exit 0), covering the non-owner CREATE consumer. These
runs used the declared PROTOC, one job and this checkout's absolute `target/`.
They ran on the dirty `c6e8c6147` base; no later commit is relabelled as tested.
The new save projection follows `Pet.cpp:452–562,1129–1176` and
`SpellHistory.cpp:182–218`. The removal continuation awaits the two transactions
separately, revalidates the canonical Player and captured Pet after each wait,
and retains the Pet plus the existing persistence fence on failure, unknown
commit, cancellation or intervening mutation. These are implementation and
mocked-port contracts, not real DB/relogin evidence. The required owner field
is `UnitData::Summon` (`Unit.h:1139–1140`, `UpdateFields.h:210`); the repair
replaces the old control-only field with that native authority and connects
both CREATE and VALUES consumers. Observer snapshots capture it under the same
canonical Player read as their other presentation facts. The existing loader
now attaches its native control links after inserting the Pet; this does not
complete general `SetMinion`, Pet AI or Pet aura loading.
Any newly discovered prerequisite must name the failing scenario and the
smallest repair. A generic domain defect is not automatically a #43 blocker.
In particular, the reagent-return composition reuses native inventory, quest,
obtain-spell and collection operations; it does not start a general achievement
engine. `StoreNewItem`'s `ObtainAnyItem`/`AcquireItem` criterion emission remains
the progression responsibility in PORT_PLAN L18, as the existing D-C8 vendor
acceptance already records. Preserve that gap explicitly: neither reagent
return nor #43 acceptance may claim achievement parity. Actual item ownership,
refund quantity, callbacks, persistence and notifications remain in scope.

Coding feedback on 2026-10-27, dirty working tree based at `e26551b0d`, aarch64:
`cargo test -p wow-world-core session::player_save_owner::quest_status --lib`
passed 4 cases; Database `player::lifecycle_adapter::tests::save_order` passed 6;
World `lifecycle_persistence::save_interleaving` passed 13 after repairing two
old durability fixtures to supply ItemEffectStore and inspect full Item rows.
`cargo test -p wow-entities quest --lib` passed all 62 matches, including the
native quest-log, dirty-status and save-ack cases. Creature
`player_melee_application::death_tests` passed 3 cases, including PvP policy;
the two affected World PowerBurn/spell-click cases passed lethal and nonlethal
paths. Scalar death anchors are pinned Unit.cpp:10457,10611,10635; these tests
do not establish the complete Player death operation. The initial Server
configuration command mistakenly selected `--bin world-server` and ran zero
tests; it establishes compilation only. The corrected command,
`cargo test -p world-server --lib quest_status_cleaner_policy_keeps_only_source_proven_modes`,
passed its one test after fixing the missing import in the inherited test.
All Cargo runs used one job, the local
`target/` and the declared PROTOC. This is coding feedback, not final acceptance,
a live action capture, publication or issue completion.

Focused feedback below does not replace these requirements. Dated checkpoints
and source comparisons remain below; the initial World timer decisions are
historical context for the Map cut selected later.

Native-death caller review at `1d9946d0e`, 2026-10-26: the shared damage method
is on HubMut and already has immutable catalog access. Its five current caller
paths coexist with direct Map-owned melee writers. In particular,
world-creature/player_melee_application.rs:161–187 subtracts Player-victim health
without setting JUST_DIED; player_tick.rs:489 consumes that path. Creature
melee, share, and split set only the bare Unit death state. Unit::set_health
does not promote zero health into death, and a later stats synchronization
must not substitute CORPSE for the missing transition. Source Unit.cpp:1016,
10457–10614,8527–8585 and Player.cpp:1158–1200 require the native operation at
the admitted mutation point. Include all these writers in the same integration,
with owned publication after the Map guard; no Session mailbox cleanup after
P3 or parallel Player owner. This is source/caller inspection, not executed
death acceptance or a report that this participant is implemented.

Read-only participant refinement at `31e8f06d0`: the stats bridge already has
an actor-borrowed input, `PlayerStatFacts::from_player`, a pure
`wow_world_inventory::plan_player_stats`, the existing `PlayerStatCalculator`
function type, and Core's synchronous `apply_player_stat_computation`.
They can support the native death operation without looking up or cloning a
second Player. Do not call the Session owner-locking wrapper while Map is
already borrowed. The complete choreography will belong to Application,
which already composes Spell, Inventory, Quest and Lifecycle. Core will retain
only the shared request/result contract needed by the lower callers; moving
the pure stats planner or adding an Inventory dependency to WorldEntities is
not selected. These available functions do not by themselves make aura
removal complete.
The death transport remains a required Application operation at each existing
damage admission. Session callers reach it through required methods on the
existing movement and spell-click host contracts (the spell-effect host already
inherits the latter). Shared request/result types belong to Core. The original
synchronous borrowed-manager candidate is superseded: source review at
`d63c12ce0` found that `FailQuest` calls `SetQuestStatus` and `SendQuestUpdate`
(Player.cpp:14942,15557–15672), which can execute FULL casts from effective
SpellArea rows before the next quest slot is read. The Source loader does not
limit those rows to aura-only spells. The existing asynchronous executor
re-resolves the canonical manager, so calling it under the proposed held Map
guard would relock that manager. A frozen batch or post-death mailbox would
also lose the required live phase order.

Selected replacement at `1e8a06cc2`, 2026-10-27: preserve one Player-owned
Application death continuation and suspend the admitted combat cursor at its
required death step. Release the Map guard, deliver the already produced
combat packet prefix, and drive that same operation through the existing
Session phase rail before resuming the same admitted cursor. Do not consume
damage, attack RNG or a phase twice. This is a dedicated required phase step,
not a command-mailbox callback or another PlayerRecovery timer pass. Reuse the
rail's claim/revoke permit and exact Player incarnation, registration, Map
incarnation and tick admission. A started unresolved operation blocks the
remaining tick and retirement; revocation before start does not erase native
work already admitted to the Player. Disconnect finalization must complete
ready work or retain that owner under the existing fail-stop contract before
logout save and retirement. Read each live quest slot only after its preceding
callbacks finish. No Player clone, second clock/task, packet delivery or await
under a Map guard is selected. Exact cursor/phase signatures and complete
participants still need implementation and acceptance before enabling lethal
callers. Server currently runs all admitted combat phases under its canonical
guard (`runtime/canonical_creature_runtime.rs:117–180` and
`runtime/map/update_loop.rs:373–427`); this replacement is not installed.
Implementation simplification selected in the closure review at `e26551b0d`:
retain the admitted combat call as an awaited Rust future, using its local
variables for the current swing, calculated damage and already-consumed RNG.
Do not add a second handwritten combat-cursor state machine. Scope each Map
guard to synchronous mutation/read sections and await the required native
death operation between those sections, before the same call proceeds. The
existing Player operation identity and Session permit still fence cancellation
and retirement; an abandoned future is not completion evidence. This is an
implementation decision, not a claim that the async combat path is installed.
The current `runtime/map_tick.rs:252–300` callback runs before MapManager's
resume, while the same admitted plan remains AwaitingSessions. The selected
cursor can therefore release guards and retain that plan without adding a
second clock or a new Map coordination state. Multiple native deaths within
one tick require operation-ID admission, rather than reusing the once-per-tick
PlayerRecovery authority check. The asynchronous Application body can reuse
existing native Inventory/stat/Aura owners after releasing the outer Map
guard; a new borrowed durability engine is not required merely by the earlier
synchronous candidate. The native penalty must still run at its Source phase,
before the tick resumes, independently of presentation-command admission.
The later selected stat-view integration resolves actual SpellInfo keys and
indexed effects in `canonical_access/player_stats/aura_view.rs`; the dated
checkpoint below records that bounded fix. It does not complete generic aura
or native-death participants. No runtime callback has been installed here.

Durability participant review at `04583b7d3`, 2026-10-26: the current
WorldEntities combat command first admits Session state, Player/map identity
and health revision, then calls Inventory's
`publish_creature_melee_death_durability_loss_like_cpp`. Despite its name,
that function mutates item durability before sending the death-loss message.
Consequently, rejecting a presentation command can also skip the native
penalty. The complete death operation must apply that penalty at the admitted
Map mutation and return owned publication; it must not depend on Session
delivery or call the owner-locking inventory/modifier accessors under that
Map guard. Native Player inventory update and item-modifier action methods
already exist, as do the borrowed stat planner and synchronous application
described above. Their existence does not establish a complete borrowed
durability participant or its persistence/publication integration.
Pinned source Unit.cpp:10632–10652 sets PvP death before the penalty and
selects PvE or the configured PvP branch. World.cpp:761 loads
`DurabilityLoss.InPvP`; no corresponding Rust consumer was found in the
inspected Server, Core or World domains. Player.cpp:4522–4562,4590–4620
requires equipment-slot order, the aura multiplier, removal of broken-item
mods before writing zero durability, and `ITEM_CHANGED`. The fall caller
has a distinct source order and message percentage at Player.cpp:655–673;
preserve it instead of applying the creature-kill message formula to all
deaths. This is source/caller inspection only. No durability change, new
acceptance result or runtime authority is claimed here.

Pet participant review at `ce61305e3`, 2026-10-27: pinned
Player.cpp:20839–20962 resolves a live typed Pet before removing it; a
missing Pet does not clear a stale GUID. Reagent return also covers a
temporarily unsummoned Pet and precedes the no-Pet return. The live path
stops combat, saves, resets the stable slot, unlinks the minion, queues
deferred removal, then publishes controlled-Pet removal. Pet.cpp:452–546
saves auras before cleanup in its first transaction and the character_pet
row in a second transaction. Existing Entity save plans and Database
statements have no complete runtime executor in the inspected paths.
Core's talent-oriented represented removal skips those operations and
removes the object immediately; it is not a faithful death participant.
This bounded integration must not become a requirement to complete the
whole Pet product, nor may missing loaded-aura metadata justify replacing
persisted pet auras with an empty set. No Pet mutation or acceptance was
performed by this review.

NoDeath quest failure is also an operation, not just a status assignment.
Pinned Player.cpp:14928–14989 selects quest-log slots, accepts incomplete
quests or the source's specific completed timed case, updates status/log and
timer, publishes the appropriate failure, and destroys quest-bound objective
and quest-drop items. Existing Player quest status and inventory destruction
planners are available; no complete native FailQuest consumer was found.
Source Player.cpp:1182–1191 orders SelfRes initialization before that failure
and criteria updates, with Unit aura removal afterward. Preserve that order
and the inventory/aura consequences in the native integration. The existing
inebriation owner is PlayerData::Inebriation with a dirty writer; reuse that
canonical field rather than adding a second drunkenness state.
These are integration findings, not additional accepted gameplay evidence.

The required native FailQuest persistence must also retain a dirty quest
status until its actual save acknowledgement. Current
`player_save_owner.rs` explicitly omits the QuestStatus group because there
is no coherent dirty-set seam; PlayerCharacterSaveRequestLikeCpp has no
quest-status group. Existing immediate quest writes do not establish this
Source death-time participant's later `_SaveQuestStatus` behavior. Preserve
per-quest writes and unrelated rows; this finding does not justify replacing
the whole quest table or claiming that the current full save deletes it.

Further NoDeath ordering review at `c7829b4f9`, 2026-10-26: on the pinned
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` source, `FailQuest`
(`Player.cpp:14928–14989`) calls `SetQuestStatus` before setting the quest-log
failure flag, clearing the timer and sending the failure packet. That setter
(`:15557–15573`) invokes both quest-status script callbacks and
`SendQuestUpdate` (`:15624–15678`). The latter evaluates quest-bound spell-area
requirements, resolves conflicting cast/remove sets, casts missing eligible
auras before removing rejected ones, and refreshes visible gameobjects or
spell-click eligibility. These consequences occur before failed-quest item
destruction and before Unit death-aura removal. A native implementation must
preserve this operation and order; assigning FAILED and deleting items alone
does not implement it. The inspected Player, Unit and SpellHandler source
files have no local changes at that pin. No native death integration or new
acceptance run is claimed by this review.
The current Spell `aura_session/sources.rs` predicate only proves whether an
unrepresented autocast could exist; Quest's completion record still marks
spell-area execution and status scripts as unrepresented. Neither is an
executor for this transition. In particular, the source's ordered `if` /
`else if` in SendQuestUpdate checks fit for AUTOREMOVE, then selects AUTOCAST
unless IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE is set. Preserve that exact
branching, including an AUTOCAST-only row, rather than substituting the
conservative absence predicate as the execution rule.

Quest-log authority refinement at `740b21234`, 2026-10-26: pinned
Player.cpp:15983–15995 sets and removes individual `QuestLog::StateFlags`
bits; it does not recompute them from `QuestStatusData::Status`.
`CompleteQuest` (:14496–14505) sets COMPLETE, while `FailQuest`
(:14928–14959) subsequently adds FAIL. The permitted completed timed quest
with no objectives therefore retains both bits during that live transition.
The current Rust `quest_log_create_entries_like_cpp` derives exactly one of
those bits from status, and `PlayerQuestStatusRecord` has no independent
quest-log flag authority. That projection cannot establish this death
participant's parity. Preserve the distinct native field and its writers
when implementing failure; do not repair this by inventing a status value or
discarding COMPLETE. Reconnect the existing CREATE and VALUES readers to
that same authority. Source `_LoadQuestStatus` (:18680–18706) initializes the
log from durable status on login, and `_LoadQuestStatusObjectives`
(:18709–18748) then restores objective flag bits; live flag retention must
not accidentally become a new database persistence contract. This is
read-only source and consumer review, not an implemented quest-log change
or executed death acceptance.

Quest-log lifetime refinement at `b18321dad`, 2026-10-27: pinned
`RemoveActiveQuest` (Player.cpp:15575–15593) erases status/objective entries
and may call SendQuestUpdate, but does not clear the QuestLog update fields.
Callers clear their slot separately. `SetQuestSlot` (:15964–15974) resets
QuestID, flags, end time and objective counters independently. Keeping the
new live flags inside PlayerQuestStatusRecord would therefore give them the
wrong lifetime. The native QuestLog slot must own its actual update-field
values and masks; this is semantically required state rather than another
mutable wire projection of status. Source UpdateFields.h:350–359 declares
int64 EndTime, int32 QuestID, uint32 StateFlags and 24 uint16 objective values,
with 29 change bits per row. SetQuestSlotEndTime casts to uint32 before its
int64 assignment (:16000), which is separate from the field's wire width.
The required integration must connect coherent live writers, CREATE and
Map-owned VALUES to that authority, while preserving durable quest-status
and objective data as distinct Source responsibilities. The existing status
record's slot is not the authority for live UF occupancy. This finding
selects the required owner; it adds no native QuestLog implementation, SQL
column, acceptance evidence or gameplay completion.

The same Source also keeps QuestStatusData::Slot and Timer
(QuestDef.h:826–833), with Slot initially MAX_QUEST_LOG_SIZE and Timer zero.
Preserve those actual status fields: a separate QuestLog owner does not
justify deleting or renaming Slot as though it were an accidental mirror.
Status-slot attribution and live UF occupancy have different readers and
lifetimes. `_LoadQuestStatus` (Player.cpp:18673–18695) assigns the status slot
and writes the UF slot separately; live GetQuestSlot/first-free queries must
read UF QuestIDs, while status/objective operations retain their Source slot
contract. Likewise, durable endTime is not a substitute for native Timer.
The new UF must preserve nested row masks and receiver filtering: QuestLog
is PartyMember-visible (UpdateFields.cpp:1791–1797), and
Player::GetUpdateFieldFlagsFor (:3610–3617) uses IsInSameRaidWith, which is
true for self even without a group (:2139–2142). The current VALUES consumer
passes only OWNER for self; new native QuestLog delivery must include the
required PartyMember flag and the actual group-registry authority for other
receivers, without treating a stale Player group snapshot as membership proof.
These are required integration contracts, not new implementation evidence.

Native QuestLog local implementation, 2026-10-27, working tree based on
`ce61305e3` (**UNVALIDATED**): PlayerData now owns the 25 independent rows and
their nested masks; status removal retains them until an explicit slot clear.
CREATE and Map-owned VALUES read those rows. The current group registry and
the source Player's current group determine PartyMember delivery, including
self without a group and current instance groups. Live accept, shared accept,
completion, objective updates, reward, abandon and login-load consumers now
use that owner. The directory's existing objective-count projection reads
the canonical status records instead of an unmaintained parallel collection;
this does not establish complete native objective-reader parity. This does
not implement NoDeath failure,
native Timer, quest dirty-set persistence or the Application death participant.

The worker's final World `quest` coding-feedback run for this unit was
2026-10-27 09:45:20.703–09:46:54.656 UTC: 459 passed, zero failed, exit 0.
The separate directory projection case passed at 09:47:04.446–09:47:04.705
UTC, exit 0, giving 460 distinct World cases. Earlier runs failed, including
450/458 and 457/459; the later repaired run supersedes those results without
removing their logs. Review corrected the unmaintained projection, incomplete
native test actors and status-slot objective writers; a mismatch case now
distinguishes those writers from live QuestID lookup readers. Positive native
field assertions and actual Map publication tests remain in the suite.

Coding-feedback commands used `CARGO_BUILD_JOBS=1`,
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc` and
`CARGO_TARGET_DIR=/home/server/rustycore/target` on this aarch64 host:

- `cargo check -p wow-world`: exit 0, 09:39:36.534–09:40:02.201 UTC.
- `cargo test -p wow-world quest --lib`: 459 passed, as timed above.
- `cargo test -p wow-world player_registry_publishes_loot_condition_state_like_cpp --lib`:
  one passed, as timed above.
- `cargo test -p wow-entities player::quest_log --lib`: four passed,
  08:34:12–08:34:20 UTC; the original metadata's count of five included the
  result header and must not be used as a test identity.
- `cargo test -p wow-world-core group_owner::tests --lib`: two passed,
  07:57:14–07:58:30 UTC.
- `cargo test -p wow-world-visibility object_updates::tests --lib`: three
  passed, 07:58:36–07:58:56 UTC.

Logs, commands, environment and test identities are under
`/tmp/rustycore-native-quest-log-logs/` (runs 05, 06, 11, 24, 27 and 28).
These are focused coding checks of the dirty working tree, not a final
campaign or evidence at a later commit. The macro remains UNVALIDATED;
no final acceptance, fresh captures, runtime QA, deployment or durable
DB/restart/relogin evidence is established by this unit.

Parent integration review matched all 54 Rust inputs to the worker's
`final-rust-input-sha256.txt` without mismatches. The native implementation,
new World scenarios and shared native fixtures occupy separate leaves.
`crates/wow-world/unit_tests/handlers/quest_tests.rs` grows 978→1019 lines;
its remaining shared catalog and packet builders are the next natural split
if that root grows again. This meets the ordinary cohesion review required
above 1000 lines; it is not a terminal large-file exemption.
`crates/wow-entities/src/player/mod.rs` grows 4777→4785 for the private module
mount, two immutable re-exports, row/mask declarations and defaults, immutable
update snapshot member and reset call. The operations and tests live in
`player/quest_log.rs` and `player_tests/quest_log.rs`. Keep the data-shape and
constructor wiring beside the existing PlayerData declarations for this
unit: splitting only this array would obscure that layout. The parent reviews
the bounded ceiling as 4785 through #43 final acceptance, superseding the
earlier 4777 ceiling for this eight-line delta only. That acceptance must
revisit the PlayerData/default-construction split under the existing
distribution retirement owner; further growth requires a fresh review.
No physical-policy baseline was regenerated.

Native quest-save source refinement, 2026-10-27, at
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` (the inspected Source files remain
clean): `_SaveQuestStatus` (Player.cpp:20138–20205) visits the dirty map,
reads EndTime and objectives through the status slot, and replaces objective
rows for that quest only. Its flag getter, `GetQuestSlotObjectiveData`
(:15929–15948), literally evaluates `StateFlags & objective.StorageIndex`;
the flag writer (:16469–16476) uses `256 << StorageIndex`. That asymmetry is
a suspect pinned-source behavior. The bounded native save must retain the
literal getter result, with index-zero and COMPLETE/FAIL interaction cases;
a gameplay repair needs separate version/capture evidence and an explicit
contract. This review establishes no fresh action capture or runtime result.

The effective `IsTurnIn` gate is `!CONFIG_QUEST_IGNORE_AUTO_COMPLETE` and
the turn-in quest type (QuestDef.cpp:505–508). The existing config registry
already loads `Quests.IgnoreAutoComplete`, `CleanCharacterDB` and
`PersistentCharacterCleanFlags`; production consumers still need the narrow
resolved policy. World.cpp:170 initializes cleaning flags to zero.
CharacterDatabaseCleaner.cpp:30–62 leaves them there when cleaning is
disabled; its enabled branch reads `PersistentCharacterCleanFlags` from
`world_variable`, performs cleanup, masks the flags by configured persistent
flags and publishes the result. An enabled-cleaner execution is not established
in Rust. Preserve that uncertainty when an actual dirty NONE-status row needs
the keep-abandoned decision. Non-NONE saves and explicit per-quest deletions
do not depend on that flag. No database cleaner execution or live SQL is
authorized or claimed by this source review.

The bounded death integration does not require a new quest-script extension
API: at the pinned Source, ScriptMgr.cpp:2092–2095,2329–2335 dispatches the
two status callbacks, whose default bodies (:3002,3201) are empty. A search
of `src/server/scripts` for `OnQuestStatusChange` finds the command-side
invocations in `Commands/cs_quest.cpp:142–143`, but no overriding script body.
This proves that those callbacks have no gameplay body in that checkout;
it does not make `SendQuestUpdate` inert. Its loaded spell-area rows, casts,
removals and visibility refresh remain required participants. New native or
Wasm extension hooks retain their separate #583 contract. This is source
review, not implemented death or acceptance evidence.

Selected resurrection consumer review at `740b21234`, 2026-10-26: the
earlier sparse-key expiration case covers sickness, not both ghost auras.
Application's `apply_loaded_ghost_wisp_aura` and
`prepare_ghost_wisp_removal` still require the legacy difficulty-zero
`SpellStore::get` payload. Those readers must use the active Aura's original
difficulty, actual SpellInfo key and indexed effects before claiming
nonzero-only Ghost/Wisp load and removal. The existing loader records a
selected-effect restoration error as incomplete aura authority; that is a
failure boundary, not successful restoration.
Also, `resurrect_player_like_cpp` currently returns unit after logging a
native error. Its reclaim and spirit-healer consumers then continue to bones
or durability loss without a success result. The request continuation already
holds `ResurrectPlayerInFlight` on that error; extend truthful failure
propagation to the other consumers and the login caller. Preserve source
`LoadCorpse`'s bones-before-revive order (Player.cpp:18157–18180), rather than
inventing rollback for an earlier completed phase. Source resurrection order
remains Player.cpp:4320–4403 and NPCHandler.cpp:347–370 at the pinned C++ SHA.
These were inspected consumer gaps at this review point. The later Ghost/Wisp
checkpoint records its focused repair; the later native-error checkpoint
records the dependent consumer repair.
This review itself claimed no new test or runtime acceptance.
The failure signal also exists for corpse conversion:
`spawn_corpse_bones_like_cpp` returns false for unavailable/unconfirmed deletion
or an unapplied Player save, but direct reclaim, healer and login callers
currently discard it. Request resurrection already retains the single native
continuation through `BonesInFlight` and save admission; the handler disconnects
on an unresolved result. Core save capture and `Player::defer_save_if_transfer_pending_like_cpp`
both read that same continuation. Extend that owner to the remaining recovery
operations when completing their persistence/cancellation contract. A native
error return alone does not establish confirmed corpse deletion, a safe retry
after unknown COMMIT, or complete recovery acceptance. Preserve source
Player.cpp:4509–4515 and its existing deletion-before-save order; do not add a
second pending-deletion flag or turn an empty Map after conversion into proof
that an uncertain database deletion succeeded.
The selected native-error repair is to return the existing native error from
`resurrect_player_like_cpp`, stop reclaim before bones and healer before
durability/bones/teleport, and return failure from login corpse restoration
before clearing AT_LOGIN_RESURRECT. The existing Character login consumer
must then abort its partially completed login instead of publishing completion.
Keep ordinary triggered-cast failure semantics distinct: source
ResurrectPlayer ignores the sickness CastSpell return, and login continues
after its ghost-cast attempts. Do not turn those cast results into an invented
rollback of an already completed Alive transition. The native-error checkpoint
below records this implementation; the separate corpse-conversion fence above
remains open.
The concrete Ghost/Wisp regression run on `59f28568d`'s working candidate
exposed two further metadata consumers. The first suite returned 46 passed
and five new cases failed, exit 101 (2026-10-26, 15:18:52–15:23:34 UTC;
`/tmp/rustycore-ghost-original-metadata-logs/cargo-test.log`). Four failures
reached Wisp's transform owner, which still required legacy difficulty-zero
effect/polarity data. The conflicting-difficulty fixture also incorrectly
invalidated difficulty three as well as zero; correct that fixture error
without weakening the real metadata case. A narrow canonical Wisp transform
participant will reuse exact whole-spell polarity and the existing native
transform mutation. Source SpellAuraEffects.cpp:1944 resolves the current
transform at the new Aura's stored cast difficulty, rather than the target's
current Map difficulty. Keep that distinction in the implementation and tests.
The actual loader path has another unconditional legacy `get` gate at
`aura_session/aura_state.rs:220`, before its selected metadata resolver. Remove
that gate for selected auras using the already fallible actual-key resolver;
keep it for the existing nonselected legacy path. Source Player.cpp:18085–18098
requires the actual SpellInfo and preserves the nonzero raw-difficulty
validity check. A real nonzero-only persisted-row roundtrip must exercise the
corrected entry and callbacks. The earlier component checks do not prove this
whole load path. The repairs had not passed their focused rerun at this review
point; the later integration checkpoint records the completed coding feedback.
Cohesion review for this working repair: `wow-world-application/src/aura_removal/ghost_wisp.rs`
grows from 986 to 1042 lines. It still owns the selected Ghost/Wisp fresh,
loaded and removal effects with their display-restoration decisions. The new
canonical transform query/mutation lives separately in Spell; the Application
file only wires the participant. The next natural structural cut is the
existing display/model-selection group, not another mutable aura owner.
Recheck the actual size and cohesion at #43 completed-delivery acceptance;
no numeric baseline or blanket legacy exception was changed by this review.

Selected stat-view decision at `c88cf6e87`, 2026-10-26: the application already
selects sickness's stat cause from exact effects, but Inventory's pure planner
then calls PlayerStatAuraView, whose effect queries and stat multipliers still
use the legacy difficulty-zero payload. Correct that borrowed view before
claiming nonzero-only sickness penalties complete. Resolve an active Aura's
stored raw difficulty to its actual SpellInfo key and then its indexed effect
rows; retain the canonical represented amount for each active effect. All
stat multiplier, weapon-damage percentage and per-school damage percentage
queries must use the same selection. Preserve group rules and existing
arithmetic; no new Player, amount cache, mutable Aura mirror or general
modifier engine is required.

The view will validate required immutable reads at construction so its
existing Option-returning owner adapters can reject missing keys or unresolved
difficulty/effects before applying stats. G/W and every configured race
sickness spell require canonical keys, even if a legacy payload exists.
Other auras with canonical keys use the same exact component reader;
keyless legacy consumers retain their existing bounded behavior without
acquiring a parity claim. Borrow Difficulty and race metadata from existing
catalogs; catalog unavailability is not proof that a custom spell is absent.
The selected sickness cast already requires the race catalog before admission.
Keep these facts borrowed and repeat only invariant-checked reads, rather than
adding a persistent cache. The source anchors are Unit.cpp:4818–4875,
9033–9073 and SpellAuraEffects.cpp:3566–3604,4525–4548 at the pinned target.
The actual caller chain reaches Inventory stat_planning/projection.rs,
equipment.rs and spell_bonus.rs, then the existing native effective-stat
snapshot used by damage. Its implementation and scoped coding evidence are
recorded in the selected stat-view checkpoint below; this is not a rewrite
of all generic aura queries or final macro acceptance.

Native SpellHistory prerequisite review at `09600e731`, 2026-10-26: Player
already owns the live history at `Unit.subsystems.spells.history`; the Session
cooldown adapter projects that same owner. No second Player history is needed.
At that base, the existing native `SpellHistory::update` had no discovered
production Player caller. Pinned `Unit.cpp:418–432,2836–2838` runs maintenance in
`_UpdateSpells`, before later Unit work. `SpellHistory.cpp:220–244` removes
cooldowns with end strictly before the captured system-clock time and elapsed
charges with end at or before it. The existing Player Unit pass is the required
integration point, before selected Aura lifetimes, through its current phase
rail and admission. The implementation and scoped coding evidence are now
recorded in the native history checkpoint below; live acceptance is pending.

The coordinated pass will carry one immutable epoch-nanosecond observation
captured beside the producer's existing pre-World-pass Instant, without adding
a scheduler, clock owner or mutable cache. `GameTime.cpp:103–107` captures the
full system-clock time point. Flooring that observation to milliseconds loses
the strict cooldown boundary when now is an aligned deadline plus one
nanosecond. Existing native deadlines remain epoch milliseconds and compare
after lossless u128 scaling; the legacy millisecond update entry delegates to
the same algorithm. The Creature logical clock and movement's wrapping
timestamp must not be substituted. Standalone execution retains its existing
Unit pass and uses the same system-clock domain. Admission facts must be
rechecked under the guard that mutates history. This also matters for the
pending SelfRes initializer: Source `SpellHistory::HasCooldown`
(`SpellHistory.cpp:670–693`) tests spell/category membership and an applicable
ignore-cooldown aura, rather than expiring entries inside that query. The
initializer must not silently replace that predicate with the legacy native
remaining-time helper or treat a missing load as an empty history.

Source refinement during that implementation: an on-hold cooldown has a month
deadline (`SpellHistory.cpp:35,424–428`), not a permanent exemption from
`Update`. Its elapsed deadline still expires in the source's comparison loop.
The native updater's old unconditional on-hold exemption is therefore part of
the correction, with representative future-held and elapsed-held tests. This
is an intentional behavior repair in the first production Player integration,
not a move-only refactor or a claim that the earlier focused run tested it.

**#43, contexto y evidencia previos — corte 2026-10-25, 06:47 UTC:**
movimiento/timer, consumidor de resurrección, sus rutas de transferencia/logout
y productor desde hechizos están escritos y tienen feedback focalizado;
**no hay aceptación final de este candidato**. El cálculo básico del productor
tiene contraste de datos registrado abajo; Ghost/Wisp tiene feedback focalizado
tras reparar compilación, restauración de display y publicación de su visual.
El recast de la misma aura/caster también tiene reparación y feedback focalizado.
Las consultas de visibilidad de criaturas y Players tienen feedback focalizado;
la publicación delta/removal a observadores tiene feedback focalizado, y 9c4G
conecta los pendientes permanentes al tick Map con feedback de composición.
Faltan la integración finita de sickness,
participantes de limpieza al morir/resucitar, fallos de las otras operaciones
del ciclo y capturas/QA viva. El usuario autoriza
continuar los pendientes y la aceptación, no solo conservar el feedback anterior.
La revisión previa a la campaña detectó brechas en las llamadas de movimiento
de muerte/release/resurrección/login y en las respuestas de resurrección inmediata
y diferida. Los checkpoints `d69c0860e` y `89645b0e6` contienen sus reparaciones,
todavía sin aceptación final. La limpieza al morir sigue sin conectar todos
los participantes existentes. Anclas del mismo C++
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player.cpp:1158–1198`,
`4262–4441`, `24897–24937`, `Unit.cpp:12718–12757` y
`CharacterHandler.cpp:1227–1241`. Se completarán las rutas afectadas antes de
la campaña; no se presenta la inspección como prueba ejecutada.
La inspección del único caller de `kill_player_like_cpp` detecta además que
solo se ejecutaba al recibir RepopRequest y que `death_timer_active` no tenía
cuenta atrás. Decisión de integración: reutilizar el pase existente que llama
a `tick_player_regeneration_like_cpp` en `session/driver/mod.rs`, con KillPlayer
después de regeneración y antes del timer de guardado, y la expiración antes
del teleport diferido. C++: `Player.cpp:1053–1054,1101–1111`. El Player canónico
conservará los milisegundos restantes, sustituyendo el booleano de producción;
las consultas de actividad se derivarán de `remaining > 0`. No se crea otro
reloj, tarea ni propietario. Se preserva la cuenta en mapas instanceables y
con PREVENT_RESURRECTION; la expiración pone cero antes de BuildPlayerRepop y
RepopAtGraveyard. Es una reparación de comportamiento, no un refactor ni prueba
de que se haya completado la migración global de fases de runtime de #584.
El worker reanudado ejecutó esta auditoría con Astra (informado por el runtime),
sin nuevas ediciones ni Cargo; se cerró y se abrió un worker `luna_worker`
explícito para implementar la unidad de movimiento. No se atribuye esa revisión
a Luna. El padre conserva integración, políticas y aceptación.
Plan de evidencia: candidato local revisado, una ejecución `final --architecture
--timings --logs --base origin/3.4.3`, los objetivos de integración que no
ejecute ese perfil y revisión exhaustiva del delta de persistencia requerido
por los nuevos accesos SQL. El inventario exhaustivo tiene coste separado;
se registrarán duración total, reparaciones y cualquier incumplimiento de 600 s.
La comprobación de solo lectura `./tools/qa-runtime.sh snapshot` identifica
el proceso 3094551, binario `target/deploy/live/world-server`, SHA-256
`290353718a1c534b348d03cb082fe203cf2b7ab0e179ef34d8ef4a8933c80fa4`,
cero reinicios y sin packet dump activo. No es el candidato nuevo ni prueba viva
de #43. Capturas de la acción y DB/reinicio/relogin conservan su aceptación
propia y autorización de runtime; no se ha publicado, desplegado ni cerrado #43.

Visibilidad de criaturas (9c2b): la consulta productiva usa el Player canónico
con generación/residencia activa, las celdas del viewpoint y el cadáver del
jugador resuelto una sola vez bajo el guard. Conserva las diferencias entre
distancia desde el cuerpo al cadáver y distancia normal desde farsight, y
la admisión del vehículo antes del bypass GM. Anclas C++ del mismo `a5f8da2`:
`Object.cpp:1517–1627`, `Player.cpp:23337`, `CellImpl.h:62–66` y
`Unit.cpp:11335`. No se amplía la consulta a todo el mapa ni se implementa
el cinematic flyby ausente. Feedback del worker sobre `e113e657e` más el delta
local, aarch64/jobs=1, no aceptación del candidato: siete casos pasaron en
`/tmp/rustycore-43-9c2b-creature-visibility-indexed-fixture-20261024T205050Z.log`;
la regresión `visible_creatures_skip_not_in_world_canonical_objects_like_cpp`
pasó en `/tmp/rustycore-43-9c2b-existing-creature-query-20261024T205130Z.log`;
`cargo check -p wow-world-entities --no-default-features` terminó exit 0 en
`/tmp/rustycore-43-9c2b-entities-no-default-20261024T205138Z.log`.
El primer filtro falló sus siete casos por fixtures sin inserción en celdas;
se reparó la preparación mediante el helper indexado existente, sin debilitar
el predicado. Siguen pendientes la consulta de Players/grupos y la publicación
delta/removal de Ghost/Wisp a observadores.
9c2c conecta la consulta de Players a sus celdas/viewpoint canónicos y al
mismo predicado, con GroupMode firmado desde la configuración productiva:
0/desconocido exige subgrupo, 1 grupo/raid, 2 equipo canónico y 3 desactiva
esa excepción (`Player.cpp:2121–2143`, `World.cpp:1167`). El snapshot HOME
del grupo requiere membresía actual; no se introduce un owner de grupos BG.
La proyección CREATE del directorio conserva presencia, registro, handle y
revisión de residencia, incluso al salir y volver al mismo mapa, y se ejecuta
fuera del guard de selección. Se actualizaron solo las dos filas de inventario
de CONFIG_GROUP_VISIBILITY/Visibility.GroupMode como represented_partial.
Feedback del worker sobre base `bdfa32edb` más el delta local, aarch64/jobs=1:
`cargo test -p wow-world --lib scenarios_visibility_2 -- --nocapture`,
28 passed/exit 0 en
`/tmp/rustycore-43-9c2c-world-tests-repaired-20261024T211129Z-3599528.log`.
La primera ejecución terminó exit 101 por un nombre de método inexistente
en el fixture del cadáver; se corrigió usando el insertor existente.
`cargo check -p wow-world-entities --no-default-features` y
`cargo check -p world-server` terminaron exit 0, logs
`/tmp/rustycore-43-9c2c-entities-check-20261024T210933Z-3595883.log` y
`/tmp/rustycore-43-9c2c-world-server-check-20261024T211347Z-3600372.log`.
No es aceptación final ni evidencia de runtime del commit resultante.
La revisión de esa publicación fija la selección de destinatarios en el
momento de emitir el aura: `Player.cpp:6141–6149` y
`GridNotifiersImpl.h:38–103` / `GridNotifiers.h:157–194` seleccionan fase,
distancia, shared vision y HaveAtClient antes de SendPacket. El precedente
Rust de `SendPlayerSpellIfVisibleLikeCpp` ya conserva esa decisión en la
cola duradera existente. La entrega de Ghost/Wisp debe conservarla también,
con protección de encarnación/residencia del receptor, sin recalcular
HaveAtClient ni descartar una actualización comprometida porque el emisor
haya salido después. El handle/residencia del emisor se comprueba al publicar.
Esta decisión está implementada en 9c3 con el feedback descrito abajo;
no constituye aceptación final.
La cola necesita conservar además el prefijo de auras ya emitidas cuando
combina refrescos de visibilidad: `mailbox/durable.rs` y `mailbox/pump.rs`
insertan esos refrescos antes del primer paquete marcado como gated. Aplicar
esa regla sin adaptación a un AuraUpdate comprometido podría emitir un
DESTROY antes de su actualización. 9c3 conservará ese prefijo en ambos puntos
de mezcla, manteniendo el comportamiento anterior cuando no haya esas auras,
con pruebas del orden en cola. No se añade otra cola ni se promete atomicidad
global entre productores/socket. La entrega propia de un Player canónico
Detached se conserva sin fanout espacial; Detached no equivale a ausente.

9c3 selecciona los observadores en celdas WORLD y conserva shared vision de
Player/Creature, caster de DynamicObject y la ruta de Player en vehículo.
La entrega usa la cola duradera existente con barreras de handle, GUID,
residencia activa, estado LoggedIn, asignación de visibilidad y opcode.
Feedback sobre `4f7fb9628` más el delta local, aarch64/jobs=1:
`cargo check -p wow-world` terminó exit 0 en
`/tmp/rustycore-43-9c3-cargo-check-wow-world-20261024T214030Z.log`;
los filtros `scenarios_spell_state_27::ghost_wisp` y `deferred_visibility`
pasaron 13 casos cada uno, exit 0, en
`/tmp/rustycore-43-9c3-cargo-test-world-ghost-wisp-20261024T214031Z.log` y
`/tmp/rustycore-43-9c3-cargo-test-world-deferred-visibility-20261024T214311Z.log`.
El log `/tmp/rustycore-43-9c3-cargo-test-wow-map-aura-observers-20261024T214323Z.log`
registra 2 passed para `selected_aura_observers`; el worker no recuperó el
código de salida del proceso y no se presenta como exit 0 verificado.
El intento inicial de World terminó exit 101 por un re-export faltante;
se reparó antes de los resultados anteriores. La ruta inicial completa
`send_initial_player_auras_like_cpp` sigue llamada por
`wow-world-character/src/login_session/initial_packets.rs` y no se modificó;
la afirmación del handoff de que no existía emisor productivo era incorrecta.
Faltan la integración de la fase de actualización seleccionada y su evidencia,
la aceptación del candidato comprometido, capturas y QA viva.

Decisión de persistencia seleccionada (9d, pendiente de implementación):
reutilizar DTOs y pasos SQL de familia de aura, conservando el campo de
sickness y su comportamiento. Añadir una captura opcional de las dos familias
8326/20584 del propio caster y item vacío en la transacción completa existente.
Una familia conocida y vacía limpia sus filas; autoridad/catálogos insuficientes
producen None y conservan la DB. No se amplía al borrado de todas las auras.
Las auras permanentes nuevas usan -1/-1 en SQL; Ghost tiene recalculateMask 7
y Wisp 3, conservando metadata cargada. El loader recalcula los efectos
seleccionados 95/31/58 desde baseAmount y dieSides con stack, y conserva el
amount guardado de Transform. Anclas del C++ `a5f8da2`:
`Player.cpp:19811–19865`, `SpellAuras.cpp:1134–1172,1218` y
`SpellAuraEffects.cpp:619–626,679`. La prueba requerida une productor real,
captura, binds de SavePlan y carga; no sustituye DB/reinicio/relogin reales.
9d0 prepara esa unión con DTOs `PlayerLifecycleAura*` y el módulo privado
`save_steps/lifecycle_auras.rs`, sin cambiar el campo resurrection_sickness,
los binds SQL ni el orden transaccional. Feedback del worker sobre
`e92832cac` más el delta local, aarch64/jobs=1: `cargo check -p wow-database`
terminó exit 0, log `/tmp/rustycore-9d0-check-wow-database.XFfVvk`;
`cargo test -p wow-database lifecycle_aura_family -- --nocapture` pasó dos
casos, exit 0, log `/tmp/rustycore-9d0-test-wow-database.NBRdxq`.
9d1 implementa la captura del caster propio/item vacío y reutiliza esos pasos
en la transacción completa de PlayerSave. El loader recalcula los bits 7/3,
conserva Transform y los metadatos cargados y permite una nueva captura.
Feedback sobre `7dc335021` más el delta local, aarch64/jobs=1:
`cargo test -p wow-world ghost_wisp_persistence -- --nocapture` pasó cinco
casos, exit 0, log `/tmp/rustycore-9d1-world-ghost-wisp.log`; los dos binarios
de integración del mismo comando seleccionaron cero tests y no son evidencia
de integración. El filtro DB `ghost_wisp_lifecycle_aura_families` pasó un caso,
exit 0, log `/tmp/rustycore-9d1-database-aura-families.log`;
`cargo check -p wow-world-core` terminó exit 0, log
`/tmp/rustycore-9d1-core-check.log`. No es aceptación final ni durabilidad viva.
La revisión de la operación completa añade un límite que debe cerrarse antes
de aceptar ese guardado: `ResurrectPlayer` retira Ghost/Wisp de todos los
casters, mientras una captura restringida al propio caster conservaría filas
ajenas y podría reaplicarlas al relog. `Aura::CanBeSaved` no excluye por sí
solo todo caster ajeno: las condiciones de single target/área son explícitas
(`SpellAuras.cpp:1142–1155`, `SpellInfo.cpp:1861`). Los datos instalados tienen
Attributes[5]=0 y target self. El paso 9d1 conserva el corte de implementación
ya asignado; el cierre de la familia seleccionada debe reconciliar también
esas claves válidas y las aplicaciones retiradas, sin borrar otras auras.
No se aceptará el roundtrip del caster propio como prueba de ese límite.
9d2 cierra ese límite de implementación: la captura incluye caster/item y
una instantánea conocida reescribe exclusivamente 8326/20584 por guid+spell,
insertando las aplicaciones válidas restantes con su clave completa.
La enfermedad conserva su clave de item vacío y sus consultas anteriores.
El cambio no escribe familias ajenas a esos dos IDs; None conserva los datos.
Feedback sobre `95115552d` más el delta local, aarch64/jobs=1, 2026-10-25:
el filtro World `ghost_wisp_persistence` pasó siete casos, exit 0, log
`/tmp/rustycore-9d2-wow-world-ghost-wisp-persistence-20261025T0004Z.log`;
el filtro DB `lifecycle_aura` pasó tres casos, exit 0, log final
`/tmp/rustycore-9d2-wow-database-lifecycle-aura-final-20261025T000615Z.log`.
Los checks de World y Database terminaron exit 0, logs
`/tmp/rustycore-9d2-wow-world-check-20261025T000235Z.log` y
`/tmp/rustycore-9d2-wow-database-check-final-20261025T000614Z.log`.
Se retiró una variable sin uso del test DB antes de repetir su filtro.
La prueba World recorre la resurrección nativa y después la captura/carga;
la prueba DB comprueba los binds de SavePlan por separado. No se ha probado
todavía su unión mediante DB real, reinicio y relog. La elegibilidad de los
dos spells conserva la comprobación de LiquidType de SpellMgr.cpp:3329
como evidencia de datos pendiente para la campaña, sin declararla probada.

Antes de 9c4G, el pase de Session
llamaba `tick_auras` cada cuatro ticks antes de los pases MapSession;
el C++ consume `m_visibleAurasToUpdate` en `Unit::_UpdateSpells`
(`Unit.cpp:2872–2875`), después de MapSession y dentro del Player update
(`Map.cpp:669–717`). `MapUpdatePlan.player_update_guids` es una selección,
no un consumidor ejecutado. La reparación debe reutilizar el tick admitido,
consumir solo Ghost/Wisp una vez bajo la admisión del MapTickPlan, conservar
salidas efímeras y entregarlas fuera del guard antes del refresco de visibilidad.
No añade reloj ni completa la convergencia general de #584.
El corte revisado usa `canonical_map_tick_resume_with_creature_owner_like_cpp`
después de respawns y antes del callback opcional de criaturas: el plan Map
es consumible y no Clone/Copy (`manager/state_2.rs:969`), y el guard conserva
época e incarnación durante la selección. No debe depender de la lease de
criaturas, porque su rechazo o la ausencia de ese runtime no anula el update
de los Players. Se retirará el consumo de Session desde la primera admisión
World coordinada; el modo sin coordinador conserva un único fallback. 9c4G
implementa ese corte para Ghost/Wisp; su aceptación final sigue pendiente.
La inspección del consumidor coordinado confirma que el primer pase World
marca `map_phase_coordinated_like_cpp` antes de `update`; no hace falta otro
watermark ni esperar al primer pase Map para retirar el drenaje de Session.
El worker tiene asignada esa conexión Ghost/Wisp y una prueba de composición
world-server que recorre el resume real sin runtime opcional de criaturas.
El feedback focalizado de esa conexión se registra a continuación.
El target correcto es `world-server --lib`: main.rs solo delega en
`world_server::run`, y lib.rs incluye runtime y main_tests. La indicación
previa del padre de usar `--bin world-server` fue incorrecta: el filtro
ejecutó cero tests tras 9m24s de compilación y no aporta evidencia de fase.
Se registra como coste de feedback, fuera de la campaña final aún no iniciada.
La campaña seleccionará también el target real de static_data_overlay.
9c4G, sobre `1d2c7c691` más el delta local, aarch64/jobs=1: los checks de
wow-map, wow-world-core, wow-world-spell, wow-world y world-server terminaron
exit 0 (`/tmp/9c4g-check-*.log`). El filtro World
`coordinated_world_tick_leaves_selected_delta_for_map_owned_delivery` pasó
un caso, exit 0 (`/tmp/9c4g-test-wow-world-coordinated-aura-rerun2.log`).
Sus intentos anteriores fallaron por el sender de entrada descartado por el
fixture: update pasaba a Disconnecting y el receptor rechazaba correctamente
la entrega. Se conserva el canal y una aserción LoggedIn, sin rebajar el gate;
el primer archivo de fallo fue sobrescrito en una repetición, y su salida
permanece en el transcript. `selected_publication` de Spell pasó tres casos,
exit 0 (`/tmp/9c4g-test-wow-world-spell-selected-final.log`): todos los Players,
consumo único y rechazo de época/incarnación preservando pending.
El resume real con Creature=None pasó un caso, exit 0, en world-server --lib
(`/tmp/9c4g-test-world-server-lib-map-tick-aura-rerun1.log`). Su primer intento
lib terminó exit 101 al compilar tres fixtures de nuestro `09451333d` que
usaban un constructor default inexistente. Se corrigieron a from_entries([])
sin cambiar aserciones; static_data_overlay::tests pasó seis casos, exit 0
(`/tmp/9c4g-test-world-server-lib-static-data-overlay.log`). La entrega ocurre
fuera de los guards y antes de salidas Creature/visibilidad/VALUES; no se
presenta ese feedback como campaña final ni como aceptación de sickness.
La revisión del caller de visibilidad de resurrección confirma otro residuo
de la operación: finish_resurrection_visibility llama al adaptador de
movimiento, cuyo update_visibility_with_catalogs vuelve sin actualizar si el
Player se ha movido menos de 50 yardas. C++ llama UpdateObjectVisibility()
con forced=true por defecto (Player.h:2461, Player.cpp:23322–23337 y
4373); actualiza a los demás y al propio Player. El setter Ghost Rust solo
cambia flags/máscaras de detección, sin forzar ese refresh. KillPlayer también
llama UpdateObjectVisibility en Player.cpp:4437, ausente en el helper actual.
La reparación del ciclo usará la entrada forzada existente para esos puntos,
con binding comprobado tras await; no se desactiva el umbral normal de
movimiento ni se presenta la consulta aislada de 9c2 como prueba del caller.
Inspección del productor actual, sin nueva prueba ejecutada: update_loop.rs
espera el pase World quiescente antes de admitir Map, y
map_session_pass.rs ejecuta las sesiones de cada mapa en serie. Un permiso
iniciado sin resultado retiene la barrera; el timeout no autoriza el tail.
Por ello no se abre otro lock/owner para una supuesta publicación Map de
bones durante el await de esas operaciones admitidas. LoadCorpse después de
map-add también coincide con CharacterHandler.cpp:1202–1241. Los stamps de
identidad/residencia y las continuaciones tras cancelación siguen siendo
necesarios; esta inspección no prueba DB/reinicio ni completa #584.
La revisión del orden detecta también un paquete extra de release:
`player_release.rs::publish_ghost_side_effects` emite HealthUpdate(1), pero
`BuildPlayerRepop` usa SetHealth(1), que solo ensucia los update fields
(`Player.cpp:4297`, `Unit.cpp:9214`). El emisor C++ de HealthUpdate está en
ModifyHealth para daño no mortal (`Unit.cpp:8127–8154`); DealDamage llama
Kill para el mortal y ModifyHealth solo en la otra rama (`Unit.cpp:1016–1022`).
Los callers Rust de caída/under-map también necesitan ese contraste. La
conexión de HealthUpdate es REALM (`Opcodes.cpp:1564`), conservando su layout
GUID/int64. Se revisará junto al orden de VALUES de resurrección y la fase
seleccionada; todavía no se ha reparado ni capturado esa diferencia.
Contraste adicional del mismo registro (05:07 UTC): EnvironmentalDamageLog
y DurabilityDamageDeath también usan REALM (`Opcodes.cpp:1373,1362`). No se
debe deducir Instance del hecho de que se produzcan en una simulación Map.
RuntimeEvent solo transporta bytes/distribución; la conexión debe llegar al
emisor reutilizando los rails realm existentes, con identidad/residencia
congeladas para los nuevos outputs Player y sin hacer I/O bajo el guard.
El daño ambiental publica su log después de la limpieza mortal síncrona;
la penalización específica de caída viene después (`Player.cpp:621–675`).
Sigue pendiente esa integración y su prueba de conexión/orden; no se toma la
inspección del registro como captura ni como cambio ya implementado.

Contraste pendiente del sanador: `Aura::SetDuration` solo marca actualización
(`SpellAuras.cpp:897–907`), mientras el adaptador Rust de duración publica
inmediatamente y solo al propio Player. El recast de sickness debe renovar
amounts, basepoints, duración máxima y cargas sobre la misma aplicación,
conservando slot/procedencia (`Unit.cpp:3224–3283`,
`SpellAuras.cpp:933–953,1021–1096`). El ajuste corto por nivel sí selecciona
el caster propio (`Player.cpp:4397`); no se presume un defecto de caster
extranjero. La publicación inmediata de estadísticas de Inventory también
necesita un corte seleccionado para que no adelante al AuraUpdate diferido.
La proyección `PlayerEffectiveCombatStatsLikeCpp` no ensucia campos de red
y el Player VALUES actual no la transporta: suprimir el envío sintético por
sí solo perdería estadísticas. El diseño común debe reutilizar las fórmulas
y las consultas puras existentes, aplicar el resultado al Player canónico
antes del snapshot Map y entregar fuera del guard, sin Session ficticia,
clonar/reemplazar Player ni una segunda autoridad mutable. Sigue pendiente
el corte exacto de hechos/catálogos y la implementación; no se declara una
convergencia general de estadísticas ni de #584.
Decisión del integrador tras inspección de consumers (02:10 UTC): primero
extraer PlayerStatAuraView y sus cuatro catálogos prestados en Core; los
adapters actuales conservan API/fallback y delegan las fórmulas a esa vista.
Después, hechos mínimos de Player/armas/bonus y un planner Inventory puro,
con aplicación sobre el mismo &mut Player. No se convierte el acceso de
Session en un enum universal. Para publicación se reutilizarán las máscaras
Unit/ActivePlayer ya existentes: values_update captura el snapshot derivado
inmutable y el bridge copia sus campos faltantes con las mismas máscaras,
sin otra autoridad ni otra máscara mutable paralela. Los vitales siguen en
Unit y no se sobrescriben desde el snapshot derivado. La visibilidad por
owner conserva UpdateFields.cpp:850–874 y el filtro actual de observadores.
9h1 extrae la vista y sus cuatro catálogos prestados, y mantiene los adapters
actuales con los mismos fallbacks y la conversión i32→f32→i32 de resistencias.
`cargo check -p wow-world-core` terminó exit 0, aarch64/jobs=1, sobre
`e6bd4342c` más el delta local (`/tmp/rustycore-43-9h1-core-check-final-20261025.log`).
No se ejecutaron tests de esta extracción ni otra campaña. El planner común,
su aplicación al Player y la publicación derivada siguen sin implementar;
esta extracción no representa avance de gameplay ni aceptación productiva.
Tras ese check se eliminaron tres getters nuevos sin consumer, sin cambiar
consultas; no se etiqueta el check anterior como prueba de un SHA posterior.
9h2A añade PlayerStatFacts/PlayerStatWeaponFacts y PlayerStatCatalogs, con
entrada directa desde &Player o refs explícitas, sin owner lookup ni lock.
La selección de arma y encantamientos conserva el comportamiento existente;
los constructores de fixture son cfg(test/test-fixtures). El último
`cargo check -p wow-world-core --features test-fixtures` sobre `24a1b4968`
más delta terminó exit 0, aarch64/jobs=1
(`/tmp/rustycore-9h2a-core-check-refs-20261025a.log`). No se ejecutaron tests
ni se conectó aún el planner de Inventory; sigue siendo preparación estructural.
9h2B conecta un único planner puro Inventory a los consumidores de stats y
login. Core presta hechos/catálogos al callback de cálculo y recibe un
resultado inmutable con gear/projection/effective; no hay Session ni lock
dentro del planner. Se preservan la API de proyección explícita de creación,
su None sin Player canónico, los fallbacks de fixture solo sin handle y las
diferencias legacy de fórmula ya señaladas. No cambia todavía el setter de
salud ni la publicación sintética de estadísticas. El filtro
`cargo test -p wow-world --lib scenarios_player_items_12` pasó 18 casos,
exit 0, sobre `c1c67cdf7` más delta local, aarch64/jobs=1
(`/tmp/rustycore-9h2b-scenarios_player_items_12-run3.log`); compiló también
los consumidores Core/Inventory/Character/World, sin ejecutar todos sus tests.
Los dos intentos anteriores terminaron exit 101 por mutabilidad del array
fixture y una vida de referencia sin nombrar; los logs run1/run2 se conservan.
Se revisaron además seis archivos con formato incidental del mismo macro.
No hay check adicional, campaña final ni evidencia de runtime de esta unidad.
El contraste del aplicador pendiente añade dos límites de fuente (03:06 UTC,
sin nueva prueba): en el C++ objetivo, UpdateStats para INTELLECT no llama
UpdateMaxPower, mientras UpdateAllStats sí lo hace para todos los poderes
(`Entities/Unit/StatSystem.cpp:105–152,199–230,315–338`). AzerothCore sí
actualiza el máximo de maná en ese primer camino; se conserva la operación
del objetivo 3.4.3. Por tanto, el efecto 80 de sickness no debe aplicar sin
distinción todos los máximos del planner de equipo. El resultado puro común
no autoriza por sí solo esas mutaciones. Además, UpdateManaRegen solo escribe
PowerRegenFlatModifier y PowerRegenInterruptedFlatModifier
(`StatSystem.cpp:799–833`). La etiqueta legacy mana_regen_mp5 no demuestra un
productor de ModPowerRegen: el campo figura para expansion < 2 en
`UpdateFields.h:319`, sin escritor de gameplay encontrado. El nuevo transporte
opt-in no ensuciará ese campo a partir de MP5. El brief inicial del integrador
incluía ese mapeo erróneo; se corrigió antes de su integración y aceptación.
9h2C añade el setter opt-in replace_effective_combat_stats_for_values y una
proyección inmutable en PlayerValuesUpdate. Usa las máscaras Unit/ActivePlayer
existentes y encola también el cambio exclusivo de ActivePlayerData. El bridge
completa las secciones ya presentes, conserva los vitales de Unit y congela
el índice de maná durante el seal Map; no escribe ModPowerRegen ni LocalFlags.
El setter legacy no cambia, para evitar duplicar sus publicaciones antes de
migrar los consumidores seleccionados. Feedback sobre `95cbf02a5` más delta,
aarch64/jobs=1: `cargo test -p wow-entities --lib derived_stat` pasó tres casos,
Core uno, Map uno y Visibility uno, con el mismo filtro y objetivo `--lib`.
Logs finales `/tmp/rustycore-9h2c-wow-entities-derived-stat-r2.log`,
`/tmp/rustycore-9h2c-wow-world-core-derived-stat-r2.log`,
`/tmp/rustycore-9h2c-wow-map-derived-stat-r1.log` y
`/tmp/rustycore-9h2c-wow-world-visibility-derived-stat-r2.log`, exit 0.
Core ejercita Map → captura/clear → bridge/codificación y conserva los bytes
tras mutar el Player; Visibility conserva Health público y retira los campos
privados del observador. Los primeros intentos Entities fallaron por tipos
mezclados en una macro y un fixture fuera de world; Visibility falló por Health
sin cambio efectivo. Se corrigieron esos fixtures/código y se conservaron los
logs fallidos. El worker también hizo checks separados de Entities y Core,
exit 0, antes del feedback focalizado; no son una campaña adicional ni prueba
del macro completo. La aplicación seleccionada de stats y sickness sigue
pendiente; no se habilita una publicación global ni se declara aceptación final.
9h2D conecta la aplicación pura del cálculo al Player canónico y opta por
VALUES solo para sickness de ChrRaces, Players no vivos o con continuación
de resurrección. Las otras auras Alive conservan su publicación legacy.
Las causas distinguen AllStats, stats seleccionados y daño: stamina puede
recalcular salud, pero solo AllStats modifica el slot de maná ya existente;
no cambia display power, índices ni BaseMana. La habilidad 137 conserva el
porcentaje antes del clamp. Unit::set_health fuerza 1 para Player DEAD,
sin convertir Alive con salud cero en otro estado; no cambia Creature DEAD.
Anclas del mismo C++: StatSystem.cpp:105–152,199–230, Unit.cpp:9214–9268
y SpellAuraEffects.cpp:3566–3602,3656–3705. El nuevo camino no repara todavía
el sync legacy que cambia implícitamente DeathState ni la operación completa.
La retirada de ese sync debe migrar conjuntamente login y Hearth. En el C++
objetivo, LoadFromDB establece DEAD después de cargar auras si Ghost está
activo (`Player.cpp:17720–17722`), y después savedHealth==0 establece CORPSE,
antes de limpieza por muerte y UpdateAllStats (`17776–17790`). La secuencia
Rust llama al sync de salud en login_sequence.rs:515 sin esas decisiones
explícitas; restore_player_corpse_after_login decide más tarde a partir de
Unit::is_alive. No se puede retirar el cambio implícito y conservar ese
caller sin migrar. El nuevo contrato deriva Ghost de la aura cargada, no de
flags antiguos de SQL, y conserva la precedencia del cero guardado. Hearth
debe usar BuildPlayerRepop → ResurrectPlayer(1) → homebind, tras los gates de
vuelo/área (`BattleGroundHandler.cpp:703–725`), sin añadir gate dead-only ni
bones explícitos. Los callers de stats/level-up/scaling deberán publicar los
vitales realmente aplicados, incluidos CORPSE=0 y DEAD=1. Sigue siendo una
decisión previa a implementación y a la prueba real de relog.
Feedback del worker sobre `85ee6e345` más delta, aarch64/jobs=1:
`cargo test -p wow-entities --lib lifecycle_stat` pasó dos casos y Core cuatro
(`/tmp/rustycore-43-9h2d-entities-lifecycle-stat-20261025T0330Z.log`,
`/tmp/rustycore-43-9h2d-core-lifecycle-stat-20261025T0330Z.log`). World pasó
tres tras corregir la selección de publicación
(`/tmp/rustycore-43-9h2d-world-lifecycle-stat-r3-20261025T0400Z.log`);
el r2 verde precedía esa corrección y no prueba el resultado final. El primer
intento World falló por dos tipos en fixtures. Un check World adicional
terminó exit 0 antes de la corrección; tampoco se presenta como aceptación.
El feedback suplementario `cargo test -p wow-world --lib spirit_healer`
pasó 15 casos, incluidos propietario obsoleto, Map seal/bridge, retirada y
duración por nivel/raza, en
`/tmp/rustycore-43-9h2d-stale-fixture-world-spirit-healer-r5-20261025T0403Z.log`.
Los r2/r3/r4 de ese prefijo fallaron 13/15: el setup sustituía SpellStore
y perdía Ghost/Wisp necesarios para NativeRes. Se conservaron esas entradas
y atributos, y se añadieron Stats/ChrClasses con la clave canónica. Los
intentos fallidos se conservan; el caso sin catálogo lo elimina explícitamente.
Se movieron helpers privados del fixture a spirit_healer/setup.rs sin cambiar
las identidades de tests. No hay campaña final, capturas ni QA viva. Si falla
el planner, el error se propaga y stats quedan intactos, pero la aura ya puede
estar instalada/publicada: el preflight y la valla de la operación completa
siguen pendientes, junto con refresh, persistencia all-key y duración finita.
9h3A implementa la persistencia all-key de sickness seleccionada por ChrRaces:
el snapshot por spell contiene todas sus familias caster/item; None conserva
SQL y una lista completa vacía autoriza retirar solo ese spell. El plan usa
los borrados BySpell ya existentes, una vez antes de inserts con las claves
reales, y no cambia la transacción ni sus resultados de COMMIT. La captura
valida los tres tipos 80/79/159 y su subconjunto activo, conserva importes/base,
recalc y procedencia de item, e incluye caster ajeno e item finito. La exclusión
de item permanente se aplica según la convención canónica existente
duration_total=0; C++ representa permanente como -1. No se declara una solución
general a la representación de duraciones ni a CustomCannotSave, cargas
dependientes de AuraOptions o single-target runtime. Todavía se conserva
elapsed en la captura: countdown y snapshot cambiarán conjuntamente con el
pase seleccionado, sin habilitar ahora otro dueño de duración.
Feedback sobre `d413567d4` más delta, aarch64/jobs=1:
`cargo test -p wow-database --lib lifecycle_aura` pasó tres casos, exit 0,
(`/tmp/rustycore-9h3a-db-lifecycle-aura-20261025T0414Z.log`), y
`cargo test -p wow-world --lib sickness` pasó 15, exit 0,
(`/tmp/rustycore-9h3a-world-sickness-20261025T0419Z-r3.log`). El primer World
falló por un helper que usaba un shim privado; r2 pasó 14/15 y descubrió un
fixture expirado sin snapshot de efectos. Se corrigió su preparación y se
conservó la validación de datos incompletos. Los logs previos 0415Z y
0417Z-r2 se conservan. Después se aplicó rustfmt solo a las ocho rutas Rust
propias, sin otro Cargo; el feedback anterior no se relabela como aceptación
del commit posterior. Se revisó el wrapping incidental de un export vecino.
Los helpers privados de save_snapshot_owner se extrajeron con mount explícito;
dos tests renombrados distinguen all-key y duración permanente malformada,
sin retirar casos. Carga TryCreate, refresh y clock seleccionados siguen
pendientes; estos tests no prueban DB/relogin/restart reales ni aceptación final.
9h3B aplica a sickness la misma secuencia seleccionada de _LoadAuras que
Ghost/Wisp: conserva la clave SQL original para relacionar efectos, usa el
subconjunto activo 111, normaliza cargas con AuraOptions y asigna CastID antes
de la ausencia por máscara vacía. Cada fila crea un candidato nuevo; la
retirada de anteriores del mismo spell precede CanBeSaved y SetLoadedState.
La validación de item usa la duración base: un item con base finita puede
cargar aunque el máximo guardado sea permanente, y el snapshot posterior lo
omite según el máximo restaurado. Wisp conserva su regla particular de recalc
para transform. La duración cero no representable aún retiene autoridad
incompleta; no se presenta esa retención como paridad de la fila C++.
Feedback sobre `55a5b307b` más delta, aarch64/jobs=1:
`cargo test -p wow-world --lib sickness` pasó 22 casos, exit 0,
(`/tmp/rustycore-9h3b-sickness-r2.log`), y el filtro `ghost_wisp` pasó 39,
exit 0 (`/tmp/rustycore-9h3b-ghost-wisp-r1.log`). El primer sickness pasó 21/22;
una aserción nueva confundía filas instaladas (dos) con la aplicación final
tras reemplazo (una). Se corrigió el fixture, no el contador productivo,
y se conservó `/tmp/rustycore-9h3b-sickness-r1.log`. El worker ejecutó además
un `cargo check -p wow-world`, exit 0, 40,79 s
(`/tmp/rustycore-9h3b-check-r1.log`). Corrección del integrador a las 04:56:
el brief excluía ese check, pero .codex/agents/luna-worker.toml lo exige en
developer_instructions junto con los tests focalizados. No se atribuye al worker
incumplimiento de esa instrucción superior; se conserva su coste como feedback
de implementación y se adapta la coordinación a la ruta efectiva.
Los siete casos nuevos viven en sickness_loading/selected_restore.rs; los
filtros anteriores ejecutaron casos reales y el formato precede al r2 verde.
No se cambia countdown, refresh ni orden Map, y no hay aceptación final.
9h3C migra los mismos dos campos de duración de AuraApplication a i32,
sin otra autoridad paralela. El adapter comprobado conserva la convención
de los callers legacy; selected Load retiene ahora los valores SQL firmados,
incluido máximo cero. Se retira su hold de representación de 9h3B: el loader
puede tener autoridad completa aunque el snapshot de guardado todavía
conserve un hold para ese borde. La captura sigue usando elapsed para las
duraciones positivas hasta conectar conjuntamente countdown/snapshot exacto.
SetDuration(0) permanece literal cero y no se convierte en permanente.
AuraDataInfo usa Optional<i32>, como SpellPackets.h:170–171; el transporte
conserva los 32 bits y ambos campos si el máximo se muestra, incluso remaining
cero o -1. El barrido legacy conserva su gate de máximo finito. Se revisaron
sus consumidores y fixtures, incluidos refresh Ghost/Wisp y WorldServer;
ese target WorldServer todavía no se ha compilado para este delta.
Feedback sobre `519cec5e8` más delta, aarch64/jobs=1: Entities
`canonical_duration` pasó dos casos; World `aura_duration` uno, `sickness` 22
y `ghost_wisp` 39, todos exit 0. Logs finales
`/tmp/rustycore-9h3c-entities-canonical-duration-r1.log`,
`/tmp/rustycore-9h3c-world-aura-duration-r2.log`,
`/tmp/rustycore-9h3c-world-sickness-r1.log`,
`/tmp/rustycore-9h3c-world-ghost-wisp-r1.log`.
El check World r1 falló por adaptar dos veces Taunt, y r2 terminó exit 0;
el primer test World falló por comparar el valor firmado con bits u32 en un
fixture. Se corrigieron los tipos, conservando los logs fallidos.
La revisión detectó que Taunt permanente, fuera de las tres familias,
cambiaba sus dos optional de FFFFFFFF a ausentes. Se conserva expresamente
su metadata legacy en el adapter, sin declarar que eso sea paridad C++.
Un test real de Taunt ejercita ahora ambos subcasos, finito y permanente,
con el mismo nombre anterior: exit 0 en
`/tmp/rustycore-9h3c-taunt-world-test-r4.log`. R1 tuvo un error de tipos al
confundir AuraRef de Creature con AuraApplication; r2 repitió ese supuesto
como aserción y falló; r3 pasó solo el caso permanente, antes de restaurar
también la cobertura finita en r4. Check previo del código productivo: exit 0,
8,5 s (`/tmp/rustycore-9h3c-taunt-world-check-r1.log`); después solo cambió
ese test. No se repitieron los cuatro filtros disjuntos por esa corrección
de adapter. Todo sigue sin aceptación final ni prueba viva del countdown.

9h3D añade el refresh compartido de las tres familias, la aplicación de Stats
tras refrescar sickness y su preflight antes de cambiar o retirar el Aura.
El refresh recalcula todos los amounts desde sus bases aunque el bit de
recálculo esté apagado; conserva esos bits y Transform los apaga. Esto es
distinto de RecalculateAmount durante Load, que sí respeta el bit apagado
(`SpellAuras.cpp:1021–1040`, `SpellAuraEffects.cpp:658–680`). La retirada
acepta también las máscaras activas parciales válidas del loader.
Feedback del worker sobre `476b00394` más delta, aarch64/jobs=1:
World `sickness` pasó 25 casos y `ghost_wisp` 39, exit 0, logs
`/tmp/rustycore-9h3d-sickness-r2-20261025.log` y
`/tmp/rustycore-9h3d-ghost-wisp-r2-20261025.log`; check World r4 exit 0.
El primer check falló por un import; el primer sickness tuvo tres
expectativas incorrectas. Se conservaron los logs previos. Una aserción
amplia contra cualquier UpdateObject confundía la publicación propia de
ResurrectPlayer con Stats; la prueba causal de Stats conserva su prohibición
de publicación inmediata.
La revisión posterior de `SpellAuras.cpp:1078–1094` detecta un error en el
contrato del integrador: al pasar de tres stacks cargados a uno, la condición
`stackAmount >= GetStackAmount()` es falsa. Se recalculan amounts, pero
RefreshTimers y SetCharges no se ejecutan. La llamada posterior explícita a
SetDuration de sickness cambia solo remaining. Se corrigen código y pruebas
antes de incorporar este delta; los 64 verdes anteriores no prueban esa
corrección. El ejecutor seleccionado por diff (9h3E) se escribe por separado
y aún no tiene caller productivo: barrido legacy y snapshot con elapsed
permanecen hasta conectar conjuntamente el pase Map de recuperación.
La misma corrección respeta que Player.cpp:4396–4403 solo llama SetDuration
para los niveles con acortamiento; sin esa petición no se fuerza remaining
al máximo para obtener una publicación. En el borde aritmético del nuevo
ejecutor se conserva también la resta uint32 y conversión a int32 de
SpellAuras.cpp:818 antes del clamp de negativos. No se cambia silenciosamente
ese comportamiento por una resta i64: con remaining=5 y diff=UINT32_MAX la
fuente obtiene 6, mientras un overrun normal expira en cero. El primer
contrato del integrador proponía i64 y se corrigió durante la revisión.
Corrección D y ejecutor E terminados localmente sobre `476b00394` más delta:
check World exit 0 (`/tmp/rustycore-9h3e-check-final-20261025.log`);
`cargo test -p wow-world --lib sickness`, 27 passed, exit 0
(`/tmp/rustycore-9h3e-sickness-r3-20261025.log`);
`--lib ghost_wisp`, 42 passed, exit 0
(`/tmp/rustycore-9h3e-ghost-wisp-r2-20261025.log`);
`--lib selected_lifetime`, 4 passed, exit 0
(`/tmp/rustycore-9h3e-selected-lifetime-20261025.log`). Los cuatro últimos
casos ya pertenecen a los otros dos filtros: 69 identidades distintas,
no 73. Se ejecutaron secuencialmente, aarch64/jobs=1, con target local y
PROTOC declarado. Los intentos anteriores conservan dos checks fallidos por
referencia aritmética y exports; sickness primero falló al compilar una
fixture y luego tuvo 19 passed/8 failed por faltar AuraOptions antes de
release. Se añadió el catálogo vacío conocido a esa preparación, sin dar
por conocido un catálogo ausente en producción. Ghost/Wisp tuvo 41 passed
y un fallo en la expectativa de applied_at del stack 3; se corrigió según
el predicado de refresh de la fuente. No hubo aceptación final ni QA viva.
El ejecutor E usa el dueño canónico en mundo, valida el orden de spell y
aplicación, modifica remaining y retira por los participantes existentes;
el host todavía no representa AURA_REMOVE_BY_EXPIRE. No tiene caller
productivo en este checkpoint. La publicación seleccionada aún requiere
resolver su metadata pendiente: nivel capturado del caster, aplicaciones
por stacks/cargas, CastUnit no Unit y flags de caster extranjero. En
particular, sickness conserva el literal 0x15 al crear y debe derivar
NOCASTER de la identidad real, incluida la procedencia original ahora
resuelta; no se acepta ese literal desde los tests verdes anteriores.
Consulta adicional de solo lectura a las 06:19: `world.spell_group` no
contiene filas con spell_id 15007, 8326 o 20584. Esto acota el contraste de
CanStackWith para esos datos, no prueba cualquier overlay futuro.
9i1 extrae sin cambios de comportamiento el cuerpo LoggedIn del driver a
`driver/player_recovery.rs`, conservando su guard, traza, orden y awaits.
Sobre `ae26ef615` más ese delta: check World exit 0; `--lib map_phase_pass`
pasó 18 casos y `--lib death_timer` cuatro, exit 0, logs
`/tmp/rustycore-9i1-check-20261025.log`,
`/tmp/rustycore-9i1-map-phase-pass-20261025.log` y
`/tmp/rustycore-9i1-death-timer-20261025.log`. Es preparación estructural
para el corte admitido; no activa el ejecutor seleccionado ni demuestra
aceptación del nuevo orden productivo.
9i2 separa el prelude de respawns y su continuación en un DTO propietario
de plan y resumen. El wrapper existente sigue llamando ambos inmediatamente:
no hay aún una espera de recuperación ni cambia el orden de DB/publicación.
Sobre `70c89baac` más delta, `cargo test -p world-server --lib map_tick`
pasó seis casos, exit 0, con salida real retenida en
`/tmp/rustycore-9i2-map-tick-rc-20261025.log`. El primer run también mostró
seis verdes, pero su pipeline tee no retenía el exit de Cargo y no se usó
como evidencia numérica. El check previo a renombrar los helpers termina
en Finished dev (4 min 22 s), sin exit numérico retenido ni prueba del
source renombrado; se conserva ese límite. El filtro adicional death_timer
pasó uno, exit 0, ya incluido entre los seis; no añade otra identidad.
Logs `/tmp/rustycore-9i2-check-20261025.log`,
`/tmp/rustycore-9i2-map-tick-20261025.log` y
`/tmp/rustycore-9i2-death-timer-20261025.log`. Todo es feedback de
implementación, sin campaña final, runtime ni escrituras de DB.
9i3A mueve la validación de identidad Map a Core, conserva sus criterios
previos y añade PlayerRecoveryAdmission con epochs separados. El guard de
autoridad rechaza coordinadores retirados y epochs anteriores entre pases,
permitiendo el mismo epoch una vez por kind. La nueva validación exige
LoggedIn, registro/handle actuales, residencia e incarnación activas y el
plan Map todavía admitido. No hay aún un request Recovery ni otro reloj.
Sobre `f78641ac0` más delta, check World exit 0 y
`cargo test -p wow-world --lib map_phase_pass` pasó 25 casos, exit 0;
logs `/tmp/rustycore-9i3a-world-check-20261025.log` y
`/tmp/rustycore-9i3a-map-phase-pass-20261025.log`. Incluye los 18 casos
previos y siete nuevos; no es aceptación del candidato completo.

9i3B connects PlayerRecovery requests to the production coordinator after the
respawn prelude and before Creature/P3, including ticks without a Creature
executor. Selection preserves Player reference order and intersects the
objects reached by the current plan; C++ calls Player::Update in that order
(`Map.cpp:704–708`). This does not resolve the remaining #584 interleaving.
The producer submits prelude DB intents once under the existing ordering gate
before releasing it to await Session. Creature inputs are captured after
Recovery. Coordinated timers leave World; their consumer shares its body with
standalone mode, saves before advancing deathTimer, and checks the binding
after awaits.
The same atomic permit retains a CompletedUnavailable terminal state: the
pass has stopped executing, but this tick's tail must be abandoned even if
the reply is lost. Running or Interrupted permits retain the barrier across
ticks; a reply alone is not completion proof. A terminal reply with invalid
lineage also abandons the tick without pretending effects are still running.
On `a66cf242a` plus the local delta, the World filters `recovery_timer` and
`death_timer` each passed four cases, exit 0. Server `map_session_pass` passed
17 cases, including prelude DB submission without replay; the exact
post-respawn participant filter passed one more, both exit 0. These four
filters cover 26 distinct test identities. Server check exited 0. Final
focused-feedback logs:
`/tmp/rustycore-9i3b-check-world-server-r3.log`,
`/tmp/rustycore-9i3b-test-world-recovery-timer-r3.log`,
`/tmp/rustycore-9i3b-test-world-death-timer-r2.log`,
`/tmp/rustycore-9i3b-test-world-server-map-session-pass-r2.log` and
`/tmp/rustycore-9i3b-test-world-server-post-respawn-participants.log`.
Environment: aarch64, jobs=1, checkout-local target, and local PROTOC. Failed
attempts remain in their logs: summary mutability, transport peers dropped by
the fixture, and incorrect module paths/types in Server tests. The initial
World `player_recovery` filter passed seven 9i3A admission cases but did not
reach the new driver; it is not counted as evidence for that driver.
An existing test was corrected: a terminal permit with a reply for another
epoch proves quiescence but blocks the tail. Its coverage is retained under
`a_wrong_tick_terminal_reply_abandons_this_tick_without_live_effects`.
The controlled Save case exercises Applied and Unknown: deathTimer does not
advance during the await; Unknown also preserves a teleport intent added
during that await. Neither final acceptance nor live QA has been run.

9i3C1 prepares selected AuraUpdates for one canonical Player. It resolves
sickness from the current ChrRaces row, prepares every selected frame before
clearing its pending slot, and preserves unrelated pending entries. Missing
catalogs are unavailable when classification is required; a current Player
with no pending entries needs no race lookup. The prepared publisher checks
GUID, handle, residence, and active map identity again, drops the manager
guard, sends directly to self, and then hands frozen observer candidates to
the existing durable rail. Valid detached sources retain their existing
self-only publication behavior. The global Ghost/Wisp collector and legacy
expiry remain active until the next atomic countdown/snapshot integration.
On code base `d494db074`, with documentation-only HEAD `891a734e5` plus the
local delta, Spell check exited 0, Spell `selected_publication` passed eight
cases, and Core `selected_aura` passed four, both exit 0. The eight Spell
cases include three existing global-collector cases and five new cases.
All commands ran sequentially on aarch64 with jobs=1 and the checkout target;
logs are `/tmp/rustycore-9i3c1-check-spell-r1.log`,
`/tmp/rustycore-9i3c1-test-spell-selected-publication-r1.log`, and
`/tmp/rustycore-9i3c1-test-core-selected-aura-r1.log`. No retry, final campaign,
runtime operation, or DB mutation was performed for this unit. The resumed
worker reported `gpt-6.1-sol`, with effort not exposed; C1 is not attributed
to Luna. The next implementation worker must use a fresh fixed Luna role.

9i3C2 activates the selected Unit step in the existing PlayerRecovery pass,
before regeneration, KillPlayer, periodic save, deathTimer, and delayed
teleport. It uses the admitted Map diff and the same body in standalone mode.
The canDelayTeleport window closes on success and failure. Selected-step
failure abandons this tick's tail through the existing permit; a retired
binding remains PlayerLeft. C++ anchors at
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are Player.cpp:935–942,
1053–1066,1101–1111 and Unit.cpp:2854–2877.

Selected countdowns leave the legacy Player wall-clock sweep, while unrelated
Player expiry and the Creature sweep retain their cadence. Pending additions,
refreshes, and SetDuration changes publish once in slot order at the Unit point,
directly to self and through the existing observer rail. Automatic release
creates Ghost/Wisp after that point, so they wait for the next Recovery. The
global post-Creature collector, summary field, and delivery call are removed.
The general aura application and threat-hydration consumers now preserve the
complete selected sickness effect snapshot; an unknown race lookup must not
rebuild it as known absence. Sickness removal publishes before its effect/stat
handlers. Fresh and refreshed selected auras do not send an immediate duplicate.

Both selected PlayerSaveOwner captures now retain exact signed maximum and
remaining durations, including zero on a still-present application. They do
not subtract Instant or infer expired absence: Unit removal owns that change.
Source: Player::_SaveAuras, Player.cpp:19811–19865. A stale applied_at value
therefore cannot change the DTO. Selected refresh and SetDuration preserve
the application timestamp and mark the existing pending slot.

On base `8b4fba80c` plus the reviewed local delta, all commands below exited 0,
sequentially on aarch64 with jobs=1, checkout-local target, and local PROTOC:

- World check: `/tmp/rustycore-9i3c2-check-wow-world-c2-final2.log`.
- World selected_recovery, excluding ghost_wisp and sickness: 2 cases,
  `/tmp/rustycore-9i3c2-test-selected-recovery-c2-final.log`.
- World sickness: 28 cases,
  `/tmp/rustycore-9i3c2-test-sickness-c2-final4.log`.
- World ghost_wisp: 45 cases,
  `/tmp/rustycore-9i3c2-test-ghost-wisp-c2-recheck.log`.
- World aura_duration, excluding sickness: 2 cases,
  `/tmp/rustycore-9i3c2-test-aura-duration-c2-final.log`.
- World save_snapshot_owner, excluding sickness: 12 cases,
  `/tmp/rustycore-9i3c2-test-save-snapshot-owner-c2-corpse-recheck.log`.
- Spell selected_publication: 5 cases,
  `/tmp/rustycore-9i3c2-test-spell-selected-publication-c2-final.log`.
- Server selected_recovery: 1 real root/Session case,
  `/tmp/rustycore-9i3c2-test-server-selected-recovery-c2-final.log`.

These filters cover 95 distinct test identities; retries add no identities.
The Server build took 6m01s of coding feedback, not a final warm acceptance
campaign. The root case observes publication before the map tail and a second
Recovery without a duplicate frame. The periodic-save case records the actual
DTO before deathTimer advances. The generic sickness case also runs the real
hydration consumer before Recovery and then checks canonical stats/P3.

Retired global-collector coverage is mapped to per-Player independence and
current binding checks in Spell, epoch/registration/residence rejection in
player_recovery_admission, and the real Server root case without a Creature
executor. The old global-only restriction that excluded sickness is intentionally
retired. Prelude/continuation rejection coverage remains in map_tick_tests;
the root's next epoch also proves continuation without the former global flush.

Failed attempts included an unqualified debug macro, fixture imports/types,
stale immediate-publication and duration expectations, missing race metadata,
and map destruction before detaching its Player. The snapshot fixture now sets
Corpse before SetHealth(0); the production setter correctly keeps Dead at one HP
and was not changed. Available failed runs remain in the named C2 log family;
the original Spell compiler-error log was overwritten by a later green run and
is not claimed as retained. A fresh fixed luna_worker role was successfully
spawned; the worker reported GPT-6 without exposing a variant or effort setting.
No final campaign, capture, DB mutation, runtime swap, deployment, or whole-#584
closure is established. Scalar metadata and the other macro requirements remain.

9i3C3a adds a private selected_wire projection without changing production
callers or the legacy builder yet. It derives duration presence, original caster
level or item level, stack/charge applications, raw-caster wire projection, and
sparse effect-indexed Points from explicit canonical facts. It rejects incomplete
identity, flag, mask, or effect coverage before producing AuraInfo. No Session,
catalog, clock, pending-set, packet DTO, or publication owner is added.
On `d5d15a018` plus the local delta, Spell check and the six selected_wire
serialization cases passed, exit 0. Logs:
`/tmp/rustycore-9i3c3a-cargo-check-run2.log` and
`/tmp/rustycore-9i3c3a-cargo-test-selected_wire-run2.log`.
The first focused run passed one and failed five because the test decoder did
not consume the CastUnit presence bit and sparse fixture amounts occupied the
wrong tuple column; its log remains at
`/tmp/rustycore-9i3c3a-cargo-test-selected_wire-run1.log`.
The two new files were formatted. The temporary dead-code warnings are explicit:
the next integration must capture caster level at creation and connect delta and
initial full publication. Constructor flags/friendly-caster resolution and final
acceptance remain open. The fresh fixed worker role reported GPT-6, with its
variant and effort not exposed. These are focused aarch64/jobs=1 results only.

9i3C3b captures optional caster_level in the existing Aura cast provenance.
Legacy unrepresented origins retain None. Fresh selected casts and selected
login loads capture the actual resolved Unit level, or the known SpellLevel
when no Unit resolves; refresh preserves the old origin. This adds no SQL field
or second mutable level authority. The private resolver borrows MapManager,
accepts the detached self fast path, and does not require the lookup owner to be
in world. Foreign Players must be in world on the same physical map; Creature
and Pet lookup uses that map's canonical container. Inconsistent backing state
is unavailable, distinct from a known absent caster. Missing level catalogs
remain unknown; known empty data yields Source's zero default. Difficulty
fallback and highest RecordID selection follow SpellMgr.cpp:2558,2610–2688.
Other source anchors are SpellAuras.cpp:398–410,463–468 and
ObjectAccessor.cpp:206–239 at the same pinned C++ SHA.

Caller review corrected an initial parent assumption: sickness's visible caster
may be self while execution.rs preserves a resolved foreign original Aura caster.
That existing path is retained and captures the foreign level. The existing
Ghost/Wisp self-caster restriction is unchanged. Tests cover the real sickness
execution with original caster level 44, a refreshed Aura retaining level 11
after its target becomes level 12, and new load-time origin capture. The latter
healer refresh legitimately changes its requested duration from 60,000 to
120,000 ms (Player.cpp:4381–4404); this does not replace the Aura's captured level.

On `375eba176` plus the reviewed local delta, World check exited 0. Spell
selected_caster passed ten cases, World sickness thirty, and World ghost_wisp
forty-five, all exit 0: 85 distinct identities, with retries counted once.
Logs are `/tmp/rustycore-9i3c3b-check-wow-world-attempt2.log`,
`/tmp/rustycore-9i3c3b-test-selected-caster-attempt4.log`,
`/tmp/rustycore-9i3c3b-test-sickness-attempt4.log`, and
`/tmp/rustycore-9i3c3b-test-ghost-wisp-attempt1.log`.
All ran sequentially on aarch64 with jobs=1, local PROTOC, and the checkout target.
Earlier attempts retain visibility/type and fixture errors plus the stale
one-minute post-level-change expectation in their separate attempt logs.
The first worker turn stopped without edits because its generic GPT-6 self-label
did not expose a variant. The parent supplied the successful enforced fixed-role
spawn trace (gpt-6-luna/max); the same worker then continued, with no model/API
fallback. Self-exposed effort remained unavailable. Selected wire production
integration, constructor flags, and final acceptance remain unexecuted.

9i3C3c connects selected_wire to the existing per-Player pending collector and
the initial full AuraUpdate. A borrowed four-store catalog view resolves exact
attributes and AuraOptions through the known difficulty chain. Known missing
Difficulty rows stop fallback; an unavailable required store or an unknown
SpellInfo key holds preparation. A known exact SpellInfo key without Misc
retains Source's zero attributes. SpellStore now exposes an exact attributes
reader and hydrates attributes in ascending RecordID order, correcting the
previous HashMap-order overwrite. Source anchors at the pinned SHA are
SpellMgr.cpp:2558–2562,2610–2688, SpellInfo.cpp:1167–1195, and
SpellInfo.h:331–345. This does not repair every other DB2 contributor.

Delta preparation freezes the application, original level/visual/cast identity,
effect snapshot, and persistence metadata under the existing Manager owner.
Every frame is prepared before any selected pending slot is cleared. Initial
publication validates the same selected application facts and builds the whole
full frame before sending it. Missing canonical backing never uses a handleless
test fixture as a replacement. Legacy entries retain the old scalar builder.
Initial full publication preserves pending deltas, following
Player.cpp:24015–24035; the next admitted Unit update may legitimately send them.
No new Player authority, packet DTO, persistence field, lock, or clock is added.

Post-add initialization reports failure to its existing login and worldport
callers. Login invokes the existing partial-login retirement and kick before
corpse restoration or LoggedIn publication. Worldport retains its post-add and
accepted-resurrection fences and stops before native completion/deferred save.
The valid ACK still advances AwaitingTeleport to TeleportCompleted before that
publication point; the new failure test must preserve that legitimate progress,
not freeze the earlier phase. Its real handler test checks retained continuation
and deferred save, unchanged pre-native health/death state, and no pet resummon,
scaling, or LoggedIn transition. The separate login case exercises post-add
rejection plus the existing abort policy; it is not full-login execution evidence.

On `515acbb19` plus the reviewed local delta, focused aarch64/jobs=1 coding
feedback passed: World check, Data selected_misc_attributes (one active case),
Spell selected_publication (twelve), World sickness (thirty-five, including
the five selected_initial cases), ghost_wisp (forty-five), selected_recovery
excluding those two families (two), initial_aura_failure (two), and Server
`--lib selected_recovery` (one): 98 distinct active test identities. Successful
logs are `/tmp/rustycore-9i3c3c-04-check-wow-world.log`,
`-07-test-wow-data-selected-misc.log`,
`-08-test-wow-world-spell-selected-publication.log`,
`-12-test-wow-world-sickness.log`, `-14-test-wow-world-ghost-wisp.log`,
`-16-test-wow-world-selected-recovery.log`,
`-19-test-wow-world-initial-aura-failure.log`, and
`-20-test-world-server-selected-recovery.log`, all with the same
`/tmp/rustycore-9i3c3c` prefix. The Server test executes the real root callback
before the Map tail, including a coordinator without a Creature owner. Its
6 min 12 s compilation is coding feedback cost, not the ordinary final campaign.
The separate selected_initial command was not repeated because sickness already
executed those identities. A transient inline sorting-only test was retired;
the retained Data test checks the actual attributes selected by the loader.

Failed attempts remain under the same prefix: 01/02 (visibility and syntax),
06/09 (test imports/private helper access), 10/11 (incomplete fixture creation
facts, stack/charge setup, lost catalogs, and missing level-20 stat data),
13/15 (missing race/options/origin metadata), and 17 (the stale pre-ACK phase
expectation). Repairs preserve the Source operation: no production metadata or
stat fallback was added to make fixtures pass. The fresh enforced luna_worker
role has a successful spawn trace for gpt-6-luna/max; the worker self-reported
GPT-6 without exposing variant or effort. Its handoff shortened several log
names incorrectly; the paths above were checked against the actual files.
The known-key/no-Misc wire branch has no dedicated test in this checkpoint;
the Data exact-key reader assertion is separate and is not claimed as that test.
No final campaign, packet capture, live runtime/DB operation, restart/relogin,
publication, or issue completion is established by this checkpoint. Constructor
flags and all remaining macro requirements above still apply.

9i3C3d1 provides one Core faction-reaction rule over transient catalog references
and distinct borrowed source/target reputation owners. Existing NPC capabilities
use that rule through one synchronous canonical-owner callback; the selected
Spell constructor is not connected yet. The old NPC reputation clone is retired.
The controlled-player reputation branch now reads the source affecting Player's
contested flag; the lower faction rule retains the target flag. Known missing
templates remain distinct from unknown catalogs, and known empty friendship
reactions use ordinary reputation thresholds. Trainer-price logic and the separate
pre-existing Hub reputation helper are unchanged. Source anchors remain
Object.cpp:2709–2864 and ReputationMgr.cpp:75–83,181–198 at the pinned SHA.

Review corrected an intermediate adapter regression: the lower API must invoke
the lower rule directly, preserving both-template admission and contested-before-
forced ordering. Encoding it as an upper-reaction request changed that order.
A World regression for contested plus forced rank and a missing target template
is authored but not executed. Opcode registration, packets, persistence, runtime
ownership, and clocks are unchanged. Source/target reputation is not copied or
stored as another mutable authority.

On `40afdb711` plus the reviewed delta, sequential aarch64/jobs=1 Core feedback
passed: `cargo check -p wow-world-core` and `cargo test -p wow-world-core --lib
faction_reaction` (seven cases). Logs:
`/tmp/rustycore-9i3c3d1-check-wow-world-core-review-fix.log` and
`/tmp/rustycore-9i3c3d1-test-faction-reaction-review-fix.log`, both exit 0.
Earlier green runs preceded the adapter correction and are not counted twice.
World adapter regressions and downstream Quest/Spell checks remain deferred to
the single completed-delivery acceptance. The fresh fixed-role spawn succeeded;
the worker self-reported GPT-6 with effort unavailable. This is an explicit
behavior repair and shared rule boundary, not a move-only refactor or final
acceptance. Selected metadata/polarity, constructor connection, and the remaining
macro blocks above are still open.

9i3C3d2a retains the four raw inputs needed by selected effect polarity inside
the existing SpellEffectInfo, as optional SpellEffectPolarityMetadata. DB2
conversion supplies Some with exact attribute bits, RealPointsPerLevel,
Coefficient, and Variance; legacy/synthetic effects retain None. An actual
all-zero row is therefore distinguishable from unavailable metadata. No second
store, cache, runtime authority, packet field, or SQL schema is added. The only
production conversion into this model is the existing DB2 converter; server-side
effect conversion uses a separate existing model. Source anchors are
SpellInfo.cpp:430–444,496–623,4518–4541 at the same pinned SHA.

On `bf140daa2` plus the reviewed four-file delta, sequential aarch64/jobs=1 Data
check and `--lib selected_effect_polarity_metadata` passed, exit 0, one consumer
case. Logs are `/tmp/rustycore-9i3c3d2a-check.VgCDA5.log` and
`/tmp/rustycore-9i3c3d2a-test.Hj9U9m.log`. The case reads converted base and exact
difficulty rows and preserves existing amounts/targets. The fixed worker role
spawn succeeded; the worker exposed no actual model/effort telemetry. Downstream
builds, classifier/constructor activation, and final acceptance remain unexecuted.

9i3C3d2b adds private pure selected-polarity and application-flag rules, without
activating production callers. The classifier preserves pre-existing negativity,
Source passive/debuff/buff/harmful/family/mechanic precedence, the seven selected
aura types, and Transform's later-effect postpass. Whole-spell positivity is
separate from the active application mask. Optional family/mechanic inputs and
unknown effect metadata are required only when their Source branch needs them.
Known enemy target 6 is backed by SpellInfo.cpp:249 and SharedDefines.h:2634;
this is not the complete implicit-target engine.

Stable signed CalcValue intervals preserve Source's int32-to-float-to-rounded
base conversion. Required ambiguous, nonfinite, or out-of-range facts remain
unavailable; no per-application polarity roll or new negativity cache is added.
Transform checks implicit holes only within the source effect-vector bound,
using their default targets. The complete _isPositiveTarget helper returns true
for inactive effects (SpellInfo.cpp:4508–4514); an intermediate parent inference
from the AURA_UNIQUE caller alone was corrected before accepting the rule.
The flags builder implements self/nonfriendly ALL versus friendly ANY, uniform
active-mask decisions without an unnecessary relationship lookup, and the exact
NoCaster/cancelable/passive/scalable bits. Duration remains a wire projection.

On `3cfd23d8e` plus the reviewed local delta, aarch64/jobs=1 Spell check and
`--lib selected_flags` passed, exit 0, fifteen cases. Logs are
`/tmp/rustycore-9i3c3d2b-check-retry2.log` and
`/tmp/rustycore-9i3c3d2b-test-retry1.log`. Earlier check/test failures are retained
in `-check.log` (postpass borrow) and `-test.log` (fixture GUID helper type), with
the same prefix. The two new files were formatted; their unsuppressed unused
private API warnings remain until integration. The fixed worker spawn succeeded;
the worker reported GPT-6 without variant/effort telemetry. No existing producer,
load, or publication caller uses these rules yet. Metadata resolver corrections,
canonical caster connection, full acceptance, and the rest of #43 remain open.

9i3C3d2c connects the wire attributes/options adapter and caster-level fallback
to one private selected catalog resolver. Exact rows decide before consulting
the difficulty graph. A needed fallback follows actual known Difficulty rows,
including zero; a known missing row stops, an unknown required store holds,
and a needed cycle is rejected. A concrete metadata store with no row for this
spell proves its Source default without requiring an irrelevant graph. Options,
Levels, and raw Misc select the highest RecordID. Wire zero attributes require
a reachable regular SpellInfo key when no Misc attributes resolve. Raw cast
difficulty and existing canonical caster admission remain unchanged. Source:
SpellMgr.cpp:675–693,2614–2687 at the same pinned SHA.

On `fb241ddee` plus the reviewed seven-file delta, sequential aarch64/jobs=1
Spell check and focused tests passed: `--lib selected_catalog` ran thirteen
cases and `--lib selected_caster` ran ten, all exit 0. Logs are
`/tmp/rustycore-9i3c3d2c-cargo-check-rerun.log`,
`/tmp/rustycore-9i3c3d2c-test-selected-catalog-rerun.log`, and
`/tmp/rustycore-9i3c3d2c-test-selected-caster.log`. The first catalog test
compilation failed on a missing fixture import and ran zero cases; its log
without `-rerun` is retained. Key-chain traversal uses explicit pure lookup
inputs in tests; the positive known-key/no-Misc wire branch still has no
dedicated consumer test. The unused raw Misc reader is reserved for the next
duration consumer. The fixed worker role ran successfully; self-reported
variant/effort telemetry was unavailable. This is focused coding feedback,
not candidate final acceptance. Constructor activation, remaining cast/load
metadata consumers, and all four macro blocks above remain open.

9i3C3d3a resolves effective attributes, family, and spell mechanic into the
existing pure polarity classifier. The new Data reader distinguishes an exact
category value of zero from a missing row. ClassOptions is available at Source
difficulty zero and selects the highest RecordID; Categories follows its own
field chain. Known missing fields use actual Source defaults. Unknown family
or mechanic facts are required only if classification reaches their branches.
The pinned spell-61882 effect-2 negativity preset is preserved. No constructor,
load, wire consumer, or general spell-fix engine is activated by this adapter.
Anchors: SpellMgr.cpp:2540–2544,2614–2687,4766–4769 and
SpellInfo.cpp:1240–1258,4517–5000 at the pinned SHA.

On `a069f9a36` plus the reviewed seven-code-file delta, sequential aarch64/jobs=1
Data and Spell checks passed. Focused tests ran one `selected_spell_mechanic`,
eight `selected_metadata`, and thirteen `selected_catalog` cases, all exit 0.
Logs under `/tmp/rustycore-9i3c3d3a-` are `data-check.log`,
`data-test-selected_spell_mechanic.log`, `spell-check-retry1.log`,
`spell-test-selected_metadata.log`, and `spell-test-selected_catalog.log`.
The initial `spell-check.log` retains an import/type failure, corrected before
the green feedback. Tests exercise the actual seven effect types across all
three selected families, including unavailable or irrelevant cyclic inputs.
The regular-key/no-Misc positive consumer remains deferred. The worker's fixed
role spawn succeeded; self-reported variant/effort telemetry was unavailable.
This is coding feedback, not final acceptance. Actual creation, refresh, load,
charge, and canonical friendly-caster consumers remain work in this macro.

9i3C3d3b activates selected metadata and flags in the actual Ghost/Wisp and
race-selected sickness producers. The existing Spell host and Application CX
carry an owned creation Result; a matching full-mask refresh retains its
original flags, captured level, and provenance without consuming that result.
Fresh application requires success before replacement or insertion. Canonical
stock self-cast base flags are 0x11; finite sickness projects 0x15 on the wire.
NeedSend and application consult the same precise selected Misc resolver.
The current legacy charge calculation is retained for its separate correction.
Mixed foreign polarity remains unavailable until the canonical reaction
adapter is connected. Load deliberately retains its old Wisp 0x101 path and
is not claimed as constructor parity by the fresh-cast regressions.

On `79346ee82` plus the reviewed local delta, sequential aarch64/jobs=1 World
check and focused feedback passed, exit 0: selected_creation (3), ghost_wisp
(45), sickness (37), and represented_faction_reaction_adapter_preserves_lower_rule_order
(1). There are 84 distinct passing identities; sickness repeats two of the
new creation cases. Logs under `/tmp/rustycore-9i3c3d3b-` are
`check-wow-world-final.log`, `test-selected_creation-final.log`,
`test-ghost_wisp-corrected.log`, `test-sickness-corrected.log`, and
`test-represented_faction_reaction_adapter_preserves_lower_rule_order.log`.
Their filename suffixes do not denote the final acceptance profile.

Review caught and removed an unsupported SpellStore clone in a fixture and a
premature loader expectation change. The C3d1 World regression had placed the
contested-guard flag in the helper's faction-group argument; the actual flags
field is now assigned and that previously deferred regression ran successfully.
Initial compiler failures remain in the earlier logs, including redundant
filters attempted before the shared libtest error was repaired. The corrected
creation run first passed two cases and failed publication because its packet
peer was closed; the final run keeps that peer alive and retains the wire
assertion. An incoherent manual partial-mask fixture was removed rather than
claimed as evidence. An unrelated generic fixture delta was retired during
integration. The fixed worker role spawn succeeded; the worker self-reported
GPT-6 without exact variant/effort telemetry. Source SHA and affected reference
files were rechecked unchanged. No final campaign, live/DB action, capture,
publication, deployment, or #43 completion is established by this checkpoint.

9i3C3d3c connects the canonical foreign-caster reaction to selected construction.
Player, Creature, and Pet reads remain borrowed under the existing manager
guard; source and target affecting Players supply their own reputation facts.
Level and relationship preparation share that guard and return owned values.
The shared Core algorithm now accepts a lazy raid fact at its original branch;
its existing API supplies the existing known boolean. Forced reaction and duel
can therefore decide before an unavailable later raid fact. No new Session,
Player clone, directory lookup, lock, or mutable reaction authority is added.

Self level and non-Unit fallback do not require map assignment. A known
out-of-world foreign Player is absent before map-consistency checks; an
inconsistent in-world body holds. Pet control selects a nonempty charmer before
checking the owner branch, including an otherwise conflicting unused Pet owner.
The inherited raid wrapper compares current groups, helpful-raid Creature flags,
or raw Creature faction-template values as appropriate. These are explicit
source-order and lookup corrections, not a move-only change. Source anchors at
the pinned SHA are Object.cpp:2230–2275,2709–2864; ObjectAccessor.cpp:206–239;
Unit.cpp:11401–11419; Unit.h:834,1160–1165; and Player.cpp:2139.

On `b28bf8038` plus the reviewed delta, sequential aarch64/jobs=1 worker feedback
passed Core and Spell checks and 31 focused cases: Core faction_reaction (8),
Spell selected_caster (12), Spell selected_reaction (10), and World
selected_caster_reaction (1). The World case runs the actual selected producer,
Session recovery, and AuraUpdate serializer: friendly group base/wire flags
0x102/0x106, hostile duel 0x10/0x14, and captured caster level 44. It also
refreshes after relationship data becomes unavailable and retains the original
application flags and provenance. This is not a Server-coordinator or live test.
Logs under `/tmp/rustycore-9i3c3d3c-` are `core-check.log`,
`core-faction-reaction.log`, `spell-check-final.log`,
`spell-selected-caster-retry2.log`, `spell-selected-reaction-final2.log`, and
`world-selected-caster-reaction-final.log`, all exit 0. Filename suffixes do not
denote the final acceptance profile. Earlier compilation and fixture failures
required repairs; the first World execution failed its retained wire assertion.
The corrected World fixture preserves Ghost/Wisp metadata alongside sickness
and keeps its SpellStore and raw Misc attributes consistent. Those repairs
were made together; no isolated cause for the missing packet was established.
The original failed Spell check log was overwritten, so it is not retained
evidence. The failed Spell test compilation and World retry2 logs remain.
The fixed worker-role spawn succeeded; exact variant/effort self-telemetry was
unavailable. Load, indexed metadata, charge consumers, and the other macro
requirements remain open. No final campaign, capture, runtime/DB mutation,
publication, deployment, or issue completion is established by this checkpoint.

9i3C3d4a moves the shared exact-first difficulty traversal and attribute reader
to Data, retaining Spell's existing private names as aliases. The existing
effect collection now keeps the highest surviving RecordID per difficulty and
slot, including explicit inert rows. The indexed resolver inherits each slot
independently and returns borrowed rows in index order. It retains needed-cycle
and unknown-catalog errors, actual zero-difficulty fallback, and malformed
dynamic-vector rejection. Invalid raw indexes retain their bits and are rejected
by exact resolution instead of becoming missing effects or index zero.
The borrowed legacy projection exposes its active prefix and continues its
previous fallback for inert-only buckets. No additional mutable catalog is added.
Source: DB2Store.h:83–87; DBStorageIterator.h:35–55;
SpellMgr.cpp:2500–2508,2638–2640; and SpellInfo.cpp:1144–1163 at the pinned SHA.

On `ca4973b37` plus the reviewed local delta, the worker reported sequential
aarch64/jobs=1 checks for Data and Spell and 26 passing focused cases, all
exit 0: Data effect_resolution (11),
effective_effect_store_is_the_only_difficulty_zero_effect_authority_like_cpp (1),
selected_effect_polarity_metadata_preserves_raw_db2_inputs_and_unknown_legacy_state
(1), and Spell selected_catalog (13). The surviving-lower-ID regression starts
from the effective input after removal; it is not a live hotfix mutation test.
The command/result record is
`/tmp/rustycore-43-C3d4a-feedback-20261025T163021Z.summary.log`;
it is a manually recorded summary, not captured raw command output. The worker
reported no failed or repeated green commands. Parent review corrected the
initial raw-index filtering and the exact-reader fixture's storage-order
expectation before feedback. The fixed worker-role spawn succeeded, while
precise runtime variant/effort self-telemetry was unavailable. These are coding
checks on the local delta, not final acceptance. Core save and Spell execution,
load, and completion consumers remain pending. No live/DB/capture, publication,
deployment, or #43 completion is established by this checkpoint.

9i3C3d4b connects Core's selected Ghost/Wisp and race-selected sickness save
adapters to Data's exact indexed effects and attribute resolver. Each
application's passive/channelled attributes now skip it before effect-shape
validation, preserving Aura::CanBeSaved's order. Global base passive/channelled
flags no longer reject an active application at another difficulty. Complete
aura authority and known selected-base catalog/shape guards still protect empty
deletion. Saved difficulty, signed duration, charges, family identity, and exact
snapshot amounts retain their existing authority. This intentionally corrects
metadata selection and eligibility; it is not a move-only refactor or general
CanBeSaved completion. Source at the pinned SHA: SpellAuras.cpp:1134–1174
and Player.cpp:19811–19865.

Worker feedback on `4f43bfeb7` plus the reviewed local delta passed sequentially
on aarch64/jobs=1: `cargo check -p wow-world-core` and
`cargo test -p wow-world save_snapshot_owner -- --nocapture` (21 library cases).
That suite includes five new indexed consumer cases. After adding final Wisp
assertions to the existing grouped case, `cargo test -p wow-world
selected_aura_indexed -- --nocapture` passed those five again; the other sixteen
cases were not rerun after that test-only change. The filtered binary targets
ran zero cases and are not counted. Logs are
`/tmp/rustycore-c3d4b-cargo-check-wow-world-core-20261025.log`,
`/tmp/rustycore-c3d4b-cargo-test-wow-world-save-snapshot-owner-20261025.log`,
and `/tmp/rustycore-c3d4b-cargo-test-wow-world-selected-aura-indexed-v2-20261025.log`,
all exit 0. The initial focused log without `-v2` also remains; compilation
cost was 4m56s there and 1m19s for the changed test, not final campaign timing.
Parent review renamed one test from unknown to missing fallback: its absent
Difficulty row is known absence, not an unknown store. Feedback preceded this
name-only correction. The fixed worker-role spawn succeeded; precise runtime
variant/effort self-telemetry was unavailable. Creation/load/completion consumers,
charges, native lifecycle work, and macro acceptance remain pending. No final
campaign, capture, live/DB mutation, publication, or deployment is established.

9i3C3d4c resolves selected execution effects before cast preparation and SpellGo,
and completion keeps that owned vector instead of replacing it with a legacy
whole-difficulty vector. Sickness application, refresh, removal, and stat-cause
selection use exact selected effects and attributes. The selected initial aura
snapshot also uses indexed rows; unrelated snapshot callers explicitly retain
their legacy path. Inert rows are not activated by their aura-type field.
Missing required selected attributes remain unavailable. No mutable catalog,
Session method, or second aura authority is added.

Refresh keeps its original Aura difficulty and effect properties, replacing
base points and recalculating through those original properties. Source anchors
at the pinned SHA are Unit.cpp:3224–3280, SpellMgr.cpp:2500–2508,2638–2640,
SpellInfo.cpp:1144–1163, and SpellAuras.cpp:496–510. An explicit inert row reaches
the existing unsupported sickness-shape error after SpellGo and creates no
sickness Aura; that regression proves no active-base fallback, not a general
partial-metadata aura implementation. Ghost/Wisp-specific execution regressions
remain deferred; the actual new consumer tests exercise sickness.

On `af58258f6` plus the reviewed delta, sequential aarch64/jobs=1 worker feedback
passed Spell check and nine distinct World library cases: selected_execution_effects
(5) and selected_creation (4). The new cases cover actual canonical Map difficulty
4→3→0, actual 0→3 fallback, pre-SpellGo rejection of needed unknown/cyclic or
malformed metadata, the inert-row hold, and cross-difficulty refresh through
Session recovery and AuraUpdate. After the last test-only change, the focused
refresh case passed again with new-row DieSides=1 and original DieSides=0, so
using the wrong calculation properties would change the asserted amount.
Other unchanged passing cases were not rerun after that fixture strengthening.
Logs under `/tmp/rustycore-43-c3d4c-` are
`wow-world-spell-check-20261026-004543.log`,
`wow-world-selected-execution-effects-20261026-005135.log`,
`wow-world-selected-creation-20261026-005053.log`, and
`wow-world-refresh-original-properties-20261026-005305.log`, all exit 0.
Earlier compile and failing consumer runs remain under the same prefix.

The first World compilation used a nonexistent fixture spawn-mode method.
The initial consumer run then passed one and failed three; it exposed real
base-only stat/snapshot readers and an unsupported-shape fixture expectation.
Repairs preserved the canonical snapshot consistency checks. Parent review also
corrected missing-row versus unknown-store test inputs and added a real zero
fallback scenario. The fixed worker-role spawn succeeded; exact variant/effort
self-telemetry was unavailable. Relevant C++ files and SHA were rechecked clean.
These are coding checks on the local delta, not a final campaign. Load/visual,
charges, completion polarity/admission, native lifecycle, casts, QA, and the
remaining macro requirements are not completed by this checkpoint. No runtime,
DB, capture, publication, deployment, or #43 completion is established.

9i3C3d4d prepares a transient selected cast-threat decision before cast
preparation and SpellGo. Empty or unavailable required selected effects are
rejected there. Exact attributes can exclude initial threat without demanding
unneeded polarity facts; otherwise the shared complete-spell polarity and
attributes are carried through completion. The shared initial-threat algorithm
uses those attributes for NO_HELPFUL_THREAT and unconditionally honors
NO_HARMFUL_THREAT for both polarities. Amount calculation remains after effects.
There is no new mutable authority, Session method, queue, or lock.

Source anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
SpellInfo.cpp:1721–1724,4518–4541 and Spell.cpp:5558–5625,8067–8070.
The selected direct lifecycle casts have no triggering Aura. This checkpoint
does not establish the additional triggering-Aura polarity rule for general
Aura-triggered casts, or replace unrelated legacy metadata consumers.

On `f3b69e930` plus the reviewed local delta, sequential aarch64/jobs=1 worker
feedback passed `cargo check -p wow-world-spell` and eight distinct World library
cases: `selected_cast_threat` (6), `legacy_initial_threat_no_harmful` (1), and
`selected_caster_reaction_uses_actual_foreign_group_and_duel` (1). The six new
consumer cases cover DEBUFF versus positive effect amounts, a positive assisting
counterfactual, exact 0→3 NO_HELPFUL_THREAT fallback, actual first creation then
matching refresh after class metadata becomes unavailable, a required-polarity
hold with a known nonzero threat entry, and missing requested difficulty despite
known base effects. The shared gate case exercises both polarities. The existing
foreign-caster case additionally retains its refresh after group/faction data
loss; it is not the class-metadata-loss case.

Raw exit-0 feedback is retained at:

- `/tmp/rustycore-43-c3d4d-feedback-20261026T011148Z/spell-check.stdout.log`
  and `spell-check.stderr.log` (Spell check).
- `/tmp/rustycore-43-c3d4-polarity-proof-20261026T/1792977934450565938/`
  (final six-case consumer run, `stdout.log`, `stderr.log`, `status.txt`).
- `/tmp/rustycore-43-c3d4-focused-20261026T/1792977602206539795/`
  (`case-2.stdout.log`, `case-2.stderr.log`, `status.txt`, shared gate case).
- `/tmp/rustycore-43-c3d4-mixed-caster-20261026T/1792978159001387499/`
  (existing foreign-caster case, `stdout.log`, `stderr.log`, `status.txt`).

The initial compilation failed on fixture/API errors, and the first consumer
execution passed five and failed one. Those runs are retained separately under
the same feedback prefixes. Subsequent fixture changes strengthened actual
creation, source-valid refresh flags, nonzero threat, and exact fallback proofs;
their repeated executions are counted once. The assigned fixed worker-role spawn
succeeded; exact model variant/effort self-telemetry was unavailable. The parent
reviewed the code and final fixtures without another Cargo run. This is a local
UNVALIDATED checkpoint with coding feedback, not candidate final acceptance.
Load/visual/charge consumers, native lifecycle, normal casts, QA, and the macro's
remaining acceptance are still pending. No live/DB mutation, capture, publication,
deployment, or issue completion is established.

9i3C3d4e connects the selected persisted-aura loader to indexed effects and exact
metadata. The private `selected_load` module owns the bounded family shape,
metadata, recalculation, and save-eligibility rules. Each component follows its
own difficulty chain, including an actual Difficulty(0) fallback. The Aura keeps
its saved raw difficulty and joins effect rows using the original SQL mask.
Inactive saved effect rows remain ignored after mask normalization.

Offline expiration uses complete-spell polarity and resolved EXPIRES_OFFLINE
attributes before asking for live caster facts. Load charge normalization tests
the full uint32 ProcCharges value, then truncates only when filling an empty
saved count. A nonzero saved count remains unchanged when full ProcCharges is
nonzero, including 256; signed -1 converts to 255 when filling an empty count.
This is the load contract, not fresh/refresh CalcMaxCharges or caster modifiers.

Execution and loading now share private Aura constructor preparation through
the existing SessionHubHostLikeCpp bound. Load passes saved difficulty, resolves
flags/level/relation before conflict removal, and still removes conflicts before
CanBeSaved. Missing required constructor facts preserve the previous Aura. The
legacy active-effect positivity helper is retired. Source DEBUFF Wisp loads with
base flags 0x11; duration is a publication flag, so the existing finite sickness
packet remains 0x15. Source anchors at the unchanged pinned SHA are
Player.cpp:18035–18154, SpellAuras.cpp:110–161,398–410,463–468,1134–1174,
SpellInfo.cpp:4518–4541, and SpellMgr.cpp:2500–2508,2638–2640. Relevant files and
the reference HEAD were rechecked clean on 2026-10-26.

On `b25bed7da` plus the reviewed local delta, sequential aarch64/jobs=1 worker
feedback passed `cargo check -p wow-world-spell`,
`cargo test -p wow-world --lib sickness_loading` (29 cases, including eight new
consumer cases), and `cargo test -p wow-world --lib selected_creation` (4 cases).
The 33 distinct cases cover indexed 4→3→0 and 0→3 load consumers, missing requested
difficulty/cycles/malformed metadata, whole-spell DEBUFF expiration with positive
effect metadata and an unavailable caster, full-width charges, pre-removal holds,
conflict-removal order, Wisp provenance, canonical friendly foreign-caster mixed
polarity, and the preserved execution constructor/refresh consumer. Existing
mask/CastId and initial wire assertions remain exercised by these suites.

Raw logs and exit files are under
`/tmp/rustycore-43-selected-load-20261026-59iniw/`:
`cargo-check-wow-world-spell-final2.{log,exit}`,
`cargo-test-wow-world-sickness-loading-final.{log,exit}`, and
`cargo-test-wow-world-selected-creation.{log,exit}`, all exit 0. Initial Spell
compilation failed on extraction visibility; initial World test compilation
failed on test-helper visibility. The first actual load suite passed 28 and
failed one stale base-duration-flag assertion. The corrected assertion retains
the actual packet duration-bit proof. Parent review also removed an overstrict
inactive saved-row guard and strengthened the expiration counterfactual before
the final run. Repeated runs are counted once. The parent reviewed the final
consumer diff without a duplicate Cargo campaign.

Physical review: `aura_state.rs` is 764 lines, selected load rules 252, and
shared constructor preparation 103. The existing `sickness_loading.rs` root is
1,022 lines and retains its shared fixture setup and earlier integration cases;
the eight new consumer cases live in a separate 801-line child. Further growth
of the root should extract its existing fixture setup rather than append another
scenario group. No production visibility was widened for tests; three existing
test helpers use parent-module visibility for their concrete sibling consumer.
The fixed worker-role spawn succeeded; runtime model/effort self-telemetry was
unavailable.

The shared constructor and load metadata are implemented with focused coding
feedback; this remains a local UNVALIDATED checkpoint. Visual selection still
uses the previous loader helper and has not acquired caster/override/condition
parity. Fresh/refresh charges, remaining native lifecycle operations, normal
casts, QA, and macro acceptance remain pending. The preserved narrow family
contract is not a general aura engine. No runtime, DB, capture, publication,
deployment, or #43 completion is established by these checks.

9i3C3d4f implements the effective condition-record presence required by selected
visual lookup. Data loads both complete WDC4 ID sets before SQL, retains each
actual table hash, unions official/custom IDs, then applies final table-scoped
removals. Unresolved WDC4 copy sources withhold complete presence instead of
silently creating absence. The existing file-only ID loader retains its previous
behavior. A final immutable catalog exposes player/unit presence, including ID
zero; an explicit effective-ID constructor supports already-composed inputs.

One SQLx-free condition-presence port supplies both ID sets. Its MariaDB adapter
uses the two ID-only queries in official/custom order and the established
native-width uint32 round-trip conversion. For example, Int32(-1) retains its
bits while Int64(-1) cannot round-trip and is rejected. Missing/NULL/non-integer
IDs do not become zero. Startup propagates file/query/decode failures before
publishing an Arc. The existing Player catalog capability installs it through
one free World composition function; there is no new WorldSession method,
general mutable-catalog accessor, or gameplay authority. Typed PlayerCondition
payloads remain file-only and are not relabelled SQL-effective predicates.

Source anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are DB2Store.cpp:127–134,
DB2DatabaseLoader.cpp:28–32, DB2Stores.cpp:555, HotfixDatabase.cpp:1192 onward and
:1853–1857, SpellInfo.cpp:4446–4464, Field.cpp:62–67, and
FieldValueConverters.h:62–70. The unchanged DB2Stores anchor was verified against
the pinned version despite its unrelated local change described below.

On `0f29c68f9` plus the reviewed local delta, sequential aarch64/jobs=1 worker
coding feedback passed `cargo check -p world-server --lib` and eleven distinct
cases: Data `condition_presence` (3), Data `db2_id_store` (2), Database
`condition_presence` (2), Server `condition_presence` (3), and the explicitly
executed ignored Data installed-files diagnostic (1). The normal Data presence
filter reported that diagnostic as ignored; its separate execution is the sole
basis for counting it as passed. No initial compilation or test failures occurred.
The first check took 4m35s; a post-edit check took 7.43s. Server test compilation
took 6m28s. These are coding costs, not a timed ordinary final-acceptance campaign.

Raw exit-0 logs use prefix `/tmp/rustycore-43-condition-presence-`:
`cargo-check-world-server-post-edit.log`,
`cargo-test-wow-data-condition-presence.log`,
`cargo-test-wow-data-db2-id-store.log`, `cargo-test-wow-database.log`,
`cargo-test-world-server.log`, and `installed-files.log`. The earlier successful
check is retained as `cargo-check-world-server.log`. The fixed worker-role spawn
succeeded; runtime model/effort self-telemetry was unavailable. Its initial
read-only handoff exposed the private WorldSession catalog boundary. The parent
settled the narrow free composition adapter and resumed implementation without
widening that field or requesting a new approval.

With `RUSTYCORE_CONDITION_DATA_DIR=/home/server/woltk-server-core/Data` and locale
enUS, the diagnostic loaded 123 PlayerCondition IDs and 482 UnitCondition IDs;
neither file contained ID zero. Post-diagnostic SHA-256 identities were:

- `PlayerCondition.db2`:
  `bd415ca301849bbfcde4ee1de765088d76d435cb1f2656b3c21332871e989e67`.
- `UnitCondition.db2`:
  `95e3a382588a6934ca33c134eec31bb777046f273ac7b855c844da2dda15b7e2`.

Those are file-phase facts, not effective SQL/removal dataset proof. The actual
visual consumer, required predicates/active overrides, and scoped capture/live
evidence remain pending. New SQL also retains its required completed-delivery
persistence-inventory review; the parent did not run another coding campaign.

Physical review: the new Data catalog is 82 lines, SQL adapter 143, and Server
loader 29. `world-server/src/app.rs` remains the existing startup coordinator at
5,664 lines after twelve lines of bounded composition wiring; the new loader's
logic is private outside it. Temporary ceiling 5,664, owner #43 integration,
expires at the #43 final architecture review: splitting unrelated startup
dependencies during this catalog repair would broaden the current operation.
That review must resolve or explicitly renew the file-specific exception before
macro closeout. No architecture baseline was regenerated here.

This is another local UNVALIDATED checkpoint with focused feedback. No selected
visual parity, final acceptance, runtime/DB mutation, capture, publication,
deployment, or issue completion is established by it.

9i3C3d4g adds the Data visual relation resolver and selector. Resolution inherits
the first nonempty whole vector through the exact difficulty chain, including
a real zero fallback, and orders rows by descending caster-player-condition ID
then descending RecordID. Selection uses effective condition-row presence,
including ID zero. Caster-incompatible constraints skip a row; a needed unknown
predicate errors before a lower candidate rather than silently choosing it.
Known absence of all visual rows returns zero without irrelevant catalog facts.

SpellStore retains potential override targets from canonical full-domain raw
effect slots before its existing i32-spell/u8-difficulty projection. Highest
RecordID wins, including explicit inert winners. Queries also inspect retained
canonical projected rows and dynamic/corrected payloads. That prevents a false
absence when a second hydration retains a nonbase slot. A legacy primary
override without explicit effect metadata remains conservative potential.
The server-side query uses accepted SpellInfo effect payloads, not orphan rows.
Persistent-area and all Source unit-owned Aura carriers are included. These
are absence-supporting metadata queries, not active-Aura authority.

Effective SpellEffect and SpellXSpellVisual loaders now use one checked reader
pass and reject unresolved WDC4 copy sources before SQL overlays/removals.
Their existing file-only load methods retain legacy behavior. The already-loaded
server-side Arc is installed through the existing Spell capability and a narrow
free composition adapter. No WorldSession method, broad mutable accessor, new
gameplay owner, or general predicate/override engine is added.

Source anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
SpellMgr.cpp:2498–2508,2594–2606,2614–2688, SpellInfo.cpp:461–492,4446–4464,
Unit.cpp:13548–13563, and SpellAuraEffects.h:56–57. The existing server-effect
admission uses regular SpellInfo keys while parent spell rows use SpellName
(SpellMgr.cpp:2753–2760,2859–2866). The added regression preserves that
asymmetry; there is no new production admission change in this checkpoint.

On `40b6d47d4` plus the reviewed delta, sequential aarch64/jobs=1 worker feedback
passed Data `visual_metadata` (10), existing `effect_resolution` (11), and the
explicit ignored `visual_file_diagnostic` (1): 22 distinct cases. They cover
exact vector inheritance/order, condition presence and caster kind, necessary
predicate errors, full-domain and dynamic override potential, inert winners,
accepted versus orphan server effects, retained nonbase hydration, and complete
loader failure before overlays. The first test compilation missed a test import;
the next run passed nine and failed one server fixture. Both were corrected
before the final focused run. Repeated cases are counted once.

Raw logs use prefix `/tmp/rustycore-43-visual-metadata-`:
`focused-tests-rerun2.log`, `effect-resolution.log`, and `file-diagnostic.log`,
with corresponding exit files recording zero. The initial World Server check
printed successful completion in 4m10s but lost its final shell status. Its
session 37028 and exec cell 77 were authoritatively unavailable; it is not
counted as retained exit evidence. One approved warm recovery
`cargo check -p world-server --lib` then exited zero in 0.24s, recorded in
`world-server-check-recovery.{stdout.log,stderr.log,exit}`. The parent reviewed
the actual diff and did not launch a duplicate campaign. The fixed worker-role
spawn succeeded; exact runtime variant/effort self-telemetry was unavailable.

With `RUSTYCORE_VISUAL_DATA_DIR=/home/server/woltk-server-core/Data`, enUS,
the complete FILE diagnostic loaded 69,504 SpellEffect and 33,467
SpellXSpellVisual rows. It found no Aura 403 rows in the file, and file-only
override potential was false for 8326, 20584, and 15007. Ghost's visual relation
was RecordID 241720 and sickness's 245039, both difficulty zero with zero caster
condition IDs. Wisp had no visual rows. The stored SpellVisualIDs (886 and 69)
are distinct from those relation RecordIDs; Source returns the relation ID.
Post-diagnostic SHA-256 identities were:

- `SpellEffect.db2`:
  `c786e3f82d69f6a2675cdcc7a0a6c48a2684044fb23344bd488425d982b525f8`.
- `SpellXSpellVisual.db2`:
  `893b2141d6250684968cccd76fb365b548177327f2fad508131fe46050276e86`.

SQL additions/removals and accepted server-side rows were not part of that
diagnostic. It does not prove the effective runtime dataset or actual cast/Aura
visual parity. Those consumers and required evidence remain pending, along with
the separately identified scalar/charge and lifecycle work. Physical review:
the new Data metadata module is 133 lines and its main test child 509;
spell_db2/state_2.rs is 902 after sharing the row decoder. App wiring adds one
line to the existing startup coordinator, updating its temporary reviewed
ceiling from 5,664 to 5,665 with the same #43 final-architecture review expiry.
No architecture baseline was regenerated. This remains a local UNVALIDATED
checkpoint, not final acceptance or authorization for runtime/publication.

9i3C3d4h connects the selected Aura constructor and persisted load to exact visual
selection. It resolves the raw caster through the canonical Player/assigned Map,
keeps physical Player versus Creature/Pet condition semantics, and checks both
regular and accepted server-side override potential for a resolved caster.
Needed unknown predicates or active override ordering remain unavailable.
NoCaster skips Unit override checks; known empty visual rows yield zero.
Flags, caster level, and visual leave one existing MapManager guard as owned
values, without packet delivery or await under that guard. The Aura independently
selects its visual instead of copying the supplied cast visual, retaining every
uint32 bit in its signed provenance and AuraUpdate field. Matching refresh keeps
the original provenance even when fresh-constructor facts are unavailable.
Selected load prepares these facts before conflict removal; offline expiry
still precedes caster/visual requirements. The legacy caster-free visual helper
now belongs only to the nonselected load branch.

Canonical detached targets with an assigned existing Map can resolve a foreign
caster before admission. An active target outside the world retains its existing
positive lookup case when its canonical Map member and assignment agree.
Explicit residence diagnostics distinguish these cases from inconsistent bodies;
there is no full-map scan or change to cast-identity admission. Source anchors
at the pinned reference are SpellAuras.cpp:110–161,398–410,463–468,
ObjectAccessor.cpp:206–213,236–238, Object.h:604, Unit.cpp:13548–13563,
SpellInfo.cpp:4446–4464 and Spell.cpp:9283–9286. Ghost/Wisp fixture expectations
now retain Source's DEBUFF precedence, including an inactive Ghost effect;
this corrects stale tests rather than introducing another polarity rule.

On `8d0fe9b4a` plus the reviewed delta, sequential aarch64/jobs=1 worker coding
feedback passed `cargo check -p wow-world-spell` and 121 distinct cases:
Spell library filters `selected_caster` (14), `selected_visual` (8), and World
library filters `sickness_loading` (32), `selected_creation` (5),
`selected_execution_effects` (5), `selected_cast_threat` (6), `ghost_wisp` (46),
and the affected corpse fixture's `reclaim_corpse` (5).
Tests used `cargo test -p <crate> --lib <filter>`, the checkout-local target,
and the configured local PROTOC. New coverage includes raw-difficulty fallback,
priority, real condition ID zero, known absence versus unknown catalogs,
override holds before removal, detached foreign casters, constructor independence,
high visual bits in actual AuraUpdate, and refresh provenance.

Raw logs and actual exit files use `/tmp/rustycore-43-aura-visual-` with final
suffixes `cargo-check-wow-world-spell-repair-3`,
`test-wow-world-spell-selected-caster-repair-3`,
`test-wow-world-spell-selected-visual-final-1`,
`test-wow-world-sickness-loading-final-3`,
`test-wow-world-selected-creation-1`,
`test-wow-world-selected-execution-effects-1`,
`test-wow-world-selected-cast-threat-1`, `test-wow-world-ghost-wisp-final-2`,
and `test-wow-world-reclaim-corpse-1`.
All record exit zero. Earlier logs retain the owner-context and group-fixture
failures, two fixture type errors, a missing visual fixture, and stale Ghost/Wisp
expectations. These coding/repair costs are not a final acceptance campaign.
The parent reviewed the actual diff and retained the original preworld positive
test. Worker-role spawn evidence is retained; exact runtime variant/effort
self-telemetry remains unavailable.

Physical review: the private visual consumer is 88 lines, its Spell tests 414,
and new World test children 162 and 117. The existing load-test root is 1,043
after bounded catalog-fixture/registration changes; the new scenarios remain
in children. Cast visual preparation, effective scalar/charge inputs, native
lifecycle work, normal casts, QA and macro acceptance remain required. This is
a local UNVALIDATED checkpoint, with no runtime/capture or publication claim.

9i3C3d4i connects the selected self-cast execution boundary to the same visual
resolver. It replaces a supplied visual before preparation, power consumption,
or publication. Native server-triggered allocation consumes the existing Map
CastID before visual resolution, with no second allocation. Wisp now consults
effective rows instead of returning a constant zero; known empty rows still
allow the existing client-log predicate to suppress SpellGo. The sickness
visual no longer uses the creature-only helper. Its existing cast-flag
preparation is separate and retains its current behavior for the later normal
cast work. The Aura constructor remains independent, so refresh can publish a
new SpellGo visual while preserving the existing Aura's original provenance.

The visual helper resolves the canonical active Map and its difficulty under
one existing MapManager guard. Review caught and removed a draft nested-lock
call to Core's difficulty getter before it was executed. No guard crosses
packet delivery or await. This reuses the existing cast-admission gate without
changing the Aura/load preworld cases. Source at the pinned reference remains
Spell.cpp:580–583,8072–8077,9283–9286, SpellInfo.cpp:4446–4464,
Unit.cpp:13548–13563 and SpellAuras.cpp:463–468. The separate ScriptVisualID
field is already ignored by the 3.4.3 wire; this change does not alter that layout.

On `1d9946d0e` plus the reviewed delta, sequential aarch64/jobs=1 coding
feedback passed `cargo check -p wow-world-spell` and World library filters
`selected_cast_visual` (2), `selected_visual` (7), `selected_creation` (4),
`selected_execution_effects` (5), `selected_cast_threat` (6), and `ghost_wisp`
(46). One case is shared by two filters: the union is 69 distinct passing
cases, not 70. Commands used `cargo test -p wow-world --lib <filter>`, the local
PROTOC and checkout-local target. They cover actual SpellGo relation IDs and
high bits, difficulty/vector priority, Wisp's empty/nonempty visual rows,
missing or override-potential metadata before power/publication, one native
CastID allocation on rejection, and refresh's independent cast/Aura visuals.

Retained raw logs and actual zero exit files use `/tmp/rustycore-c3d4i-`:
`01-check-wow-world-spell`, `04-world-selected_cast_visual`,
`05-world-selected_visual`, `06-world-selected_creation`,
`08-world-selected_execution_effects`, `09-world-selected_cast_threat`, and
`10-world-ghost_wisp`. Earlier attempts `02`, `03`, and `07` retain their 101
exits: a test assumed an unsupported SpellInfo Default, a fallback fixture
lacked required class facts, and an effect-negative case needed an exact
visual row to reach its intended failure. The parent reviewed the actual diff
and exits; no final campaign ran. The worker reported GPT-6 with unavailable
effort telemetry, alongside the retained fixed-role spawn evidence.

Physical review: the private cast helper is 48 lines and the two affected
visual test children are 472 and 111. Ordinary client SpellStart visual
preparation, foreign-caster lifetime, active override source/order, effective
scalar/charge inputs, native lifecycle, QA, captures, and final acceptance
remain required. This is another local UNVALIDATED checkpoint.

9i3C3d4j1 adds complete/effective loaders for AuraOptions, ClassOptions, Labels,
and Levels. Each shares its typed decoder with the unchanged legacy file loader,
requires all declared WDC4 rows before applying overlays, replaces RecordIDs
in supplied official/custom order, and applies final removals using the file's
table hash and full RecordID bits. Four SQL-free persistence DTOs retain the
source getter widths, including signed class masks before later Data bit
conversion. No required port method, SQL adapter, or startup call changes here;
the four production stores still read files alone until that integration.

Source anchors remain the four DB2LoadInfo definitions and HotfixDatabase
queries recorded below, plus DB2Store.cpp:127–134 and
DB2DatabaseLoader.cpp:28–32 for official/custom order. On `9d15f0308` plus the
reviewed delta, sequential aarch64/jobs=1 coding feedback passed
`cargo check -p wow-data` (7.22s), `cargo check -p wow-persistence` (2.24s),
`cargo test -p wow-data --lib scalar_catalog_loading` (5 cases), and the
affected existing `visual_metadata_loaders_require_complete_copy_resolution_before_overlays`
case (1). The six distinct cases exercise actual public loaders with modeled
fixed-record/empty files, overlay replacement, additions, scoped removals,
high-bit and signed payloads, and missing/malformed/unresolved-copy failure.
They do not establish installed file or SQL dataset contents.

Logs and actual exit files are under `/tmp/rustycore-c3d4j1-logs/`: final
`01-cargo-check-wow-data`, `02-cargo-check-wow-persistence`,
`08-cargo-test-scalar-catalog-loading`, and
`09-cargo-test-visual-metadata-regression` all record zero. `03` retains the
initial fixture expectation failure (101); intermediate successes and the
subsequently removed unused fixture wrapper warning are retained. The commands
used local PROTOC, the checkout-local target and one Cargo job. The parent
reviewed the diff; exact worker effort telemetry was unavailable.

Physical review: the private scalar loader module is 124 lines, its test child
528, and the shared test fixture child 211. Existing lookup/charge consumers
are unchanged. SQL failure propagation, source difficulty normalization,
remaining scalar composition, charges, native lifecycle and macro acceptance
remain required. This is a local UNVALIDATED checkpoint, not final acceptance.

9i3C3d4j2 implements the target's uint8 Difficulty conversion at regular effect,
hit-mechanic, server-effect and server-parent indexing boundaries. Aliased
regular rows compete for the same slot by surviving RecordID, including inert
winners. Server effects validate the normalized difficulty and join their
normalized parent; their accepted vector applies the last input row per index
and retains blank slots. A duplicate parent keeps its first name/effects while
later input rows update scalars. Raw orphan effects still contribute no visual
override potential. The existing uint8 regular key authority is unchanged.

Source anchors at the pinned target are DBCEnums.h:833,
SpellMgr.cpp:2508,2732,2772,2859,2870–2946 and
SpellInfo.cpp:398–405,1337–1353. The explicit defensive negative-index contract
recorded below is implemented with the existing range-error outcome before
vector construction; it does not emulate or claim parity for undefined legacy
input. The radius-warning fixture now uses a valid effect index so it isolates
that existing behavior.

On `71676ce68` plus the reviewed delta, aarch64/jobs=1 coding feedback passed
`cargo check -p wow-data` (3.32s) and Data library filters
`difficulty_normalization` (5), `effect_resolution` (11), `visual_metadata`
(10), and `serverside_spell` (14): 40 distinct cases. The new cases cover
regular 256/0 slot collisions, signed -1/255, mechanics and inert visual
winners, server 256/0 and 999/231 joins, validation after conversion, ordered
replacement, blank slots, first-name/last-scalar parents, orphans and invalid
negative indices. They are modeled catalog evidence, not installed SQL or
runtime data evidence.

Raw logs and actual exit files are in `/tmp/rustycore-c3d4j2-logs/`: final `01`,
`02`, `03`, `04`, and `06` record zero; `05` retains a temporary fixture-length
expectation failure (101). Commands used the local PROTOC, checkout-local
target and one job, with `cargo test -p wow-data --lib <filter>`. The parent
reviewed the actual diff and retained raw results; worker effort telemetry was
unavailable. Physical review: corrections.rs is 772 lines, and the new private
test child is 345. SQL decoder failure propagation, four scalar startup
consumers, proc/charge preparation, native lifecycle, normal casts, QA and final
acceptance remain required. This remains a local UNVALIDATED checkpoint.

9i3C3d4j3 extends the existing core Hotfix persistence port with required
AuraOptions, ClassOptions, Label and Levels row methods. Their adapter uses
the target projections and reads official rows before custom rows. All 19
previous core projections now also decode through the existing native Field
getters. A query or conversion error fails the whole load; accepted earlier
rows cannot become a partial successful result. SQL NULL still becomes
numeric zero or an empty string. Error reasons retain table, row and field
context. The existing documented rejection of text/DECIMAL numeric columns
remains stricter than C++; no actual schema mismatch was observed or repaired.

At target `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, source anchors are
HotfixDatabase.cpp:1412–1416,1443–1446,1504–1506,1513–1516,
DB2LoadInfo.h:4655,4759,4984,5009, DB2Stores.cpp:555 and
FieldValueConverters.h:62–70 / Field.cpp:62–76. Getter widths follow the
existing DTOs except SpellEffect's four masks, which use GetInt32 before
preserving their bits in uint32 fields. ClassOptions masks remain signed
until composition. Levels retains signed int16 fields; no key authority or
Difficulty enum width changes.

On `c96f3c64c` plus this delta, aarch64/jobs=1 coding feedback passed
`cargo check -p wow-database` (8.12s) and database library filters
`core_db2_hotfix_adapter` (8) and `field_like_cpp` (4): 12 distinct cases.
The adapter cases distinguish signed getters using int64 -1 and unsigned
wide values, preserve high bits, reject narrow-field overflow, check visual
condition positions, retain NULL defaults, and reject an entire malformed
batch with its diagnostic context. They also pin the four SQL projections.
These are modeled native-row tests, not real SQL dataset or runtime evidence.

Raw logs and actual exit files are in `/tmp/rustycore-c3d4j3-logs/`: `01`,
`03`, and `04` record zero. `02` retains a test-only missing-import compile
failure (101); the import was fixed before the successful retry. Commands used
the local PROTOC, checkout-local target and one job. The parent reviewed the
diff and raw results without running another campaign. Physical review:
the adapter root is 736 lines, its private decoder 396, and decoder tests 286.
The four startup consumers, key/server/proc SQL guards, exact proc/charge
preparation, native lifecycle, normal casts, QA and final acceptance remain
required. This is a local UNVALIDATED checkpoint, not final acceptance.

9i3C3d4j4 connects AuraOptions, ClassOptions, Label and Levels to the existing
core-spell composition. A private child awaits the four existing port methods,
converts their payloads, and calls the complete/effective Data loaders. Signed
ClassOptions masks retain every bit in the existing uint32 Data fields; signed
Levels and full-width AuraOptions payloads remain intact. The same four Arc
owners are installed at their previous startup positions. Only construction of
the existing Core adapter moves immediately before SpellLevels, so that its
first new consumer can use it. SQL errors still stop composition through the
existing Failed-to-Err boundary. No second catalog, port or runtime owner is
introduced.

The mapping retains DB2LoadInfo.h:4655–4673,4759–4775,4984–4994,5009–5024
at the pinned target; ordered overlays and final removals use the Data loading
contract reviewed in j1 and the SQL contract reviewed in j3. On `4217cb194`
plus this delta, aarch64/jobs=1 coding feedback passed
`cargo check -p world-server --lib` and library filters `selected_scalars`
(1 case covering all four payloads) and
`empty_and_failure_remain_distinct_before_publication` (2 cases: core and
key composition). Three distinct cases passed. No Data suite was rerun.

Raw logs and actual exit files are in `/tmp/rustycore-c3d4j4-logs/`, `01`–`03`,
all zero. Cargo reported 4m24s for check, 6m43s for the test-profile build, and
0.24s for the second filter. Those coding commands alone total approximately
667 seconds; they are not the completed-candidate acceptance campaign and do
not demonstrate its 600-second target. The parent reviewed the actual diff
and raw results without another Cargo run. The worker-role execution reported
GPT-6 with no exact effort telemetry.

Physical review: the new private composition child is 120 lines and its tests
84. The existing core composition root is 1,119 lines after seven declaration
and re-export lines; production mapping stays in the cohesive child. App grows
5,665 to 5,689 (+24) for four asynchronous capability calls at required startup
positions, with no gameplay logic moved into it. The temporary reviewed app
ceiling is 5,689, owned by #43 integration and expiring at the same #43 final
architecture review. The tracked physical policy has not been regenerated or
claimed passing. Key/server/proc and Difficulty input guards, effective PPM,
exact proc/charge preparation, native lifecycle, normal casts, QA and final
acceptance remain required. This remains a local UNVALIDATED checkpoint.

9i3C3d4k replaces width-erasing key and Difficulty SQL reads and the selected
server/proc defaulting decoders with the existing native Field getters. The
twenty key contributors retain their order, with PowerDifficulty read after
Power, and official rows before custom rows. Seven signed SpellID contributors
use GetInt32 before preserving the bits in uint32 keys. SpellEffect difficulty
uses GetInt32 before the uint8 enum conversion; the nine byte-difficulty
contributors and PowerDifficulty use checked GetUInt8. No key authority or DTO
difficulty is widened. Row failures preserve table/row/field context and fail
the whole load rather than exposing earlier accepted rows.

Server effect Aura/target fields use GetInt16 before widening to the existing
DTOs; their masks retain signed getters, and SQL base points remain raw float.
The server parent's full uint32 proc/family fields remain intact. Proc SQL
retains its narrower uint16 family and uint8 charges. This is decoder work,
not the pending server effect quantization or joint proc-source preparation.
Seven acquisition Option hotfix source families and nine other World catalogs
retain their prior behavior; this unit does not claim them repaired.

Difficulty composition now requires complete FILE copy resolution before
querying its existing overlay port. Its legacy file-only load mode remains
available. PPM adds one SQL-free DTO and required method on the existing Core
port, checked official/custom SQL, complete/effective Data loading with final
removals, and the existing WorldServer Arc at the same startup position. Its
legacy and complete loaders share one typed decoder. PPM modifiers are not
needed for the ProcBasePPM presence predicate and are outside this operation.

Source anchors at the pinned target are DB2LoadInfo.h's contributor definitions,
Difficulty :1534–1555 and PPM :5109–5118; HotfixDatabase.cpp:519–521,1542;
SpellMgr.cpp:1500–1548,2727–2766,2852–2946; and
SpellInfo.cpp:1213–1216. The lookup/presence dependency remains
SpellMgr.cpp:490–509,1873–1888 and SpellAuras.cpp:965–976. Native getter
strictness for text/DECIMAL numeric columns remains the existing documented
departure from C++; no actual SQL schema mismatch was demonstrated.

The actual parent-verified base is `3c41e8b80` plus the reviewed delta. The
worker's handoff named an older base; it does not replace Git identity. On
aarch64/jobs=1, coding feedback passed the WorldServer library check and 39
distinct cases: database `spell::` (23), database `hotfix::difficulty_adapter`
(3), Data `scalar_catalog_loading` (8), WorldServer selected scalars (2), and
WorldServer `hotfix::difficulty` (3). The key getter tests freeze independent
seven-signed/nine-byte matrices. They distinguish native widths, enum folding,
NULL defaults, overflow and whole-load failure. Other cases cover server
payloads, fractional raw SQL base points, proc limits, complete PPM replacement
and removals, incomplete copies, and composition.

Raw logs and exit files are `/tmp/rustycore-c3d4k-logs/01`–`09` with their full
command names. Final `01/03/04/06/07/08/09` exit zero. `02` retains a missing
column diagnostic failure (22/23); `05` retains a test-only trait-method compile
failure. A private field-context helper repairs the first; removing the
redundant test assertion repairs the second. Database `08` repeats the same
23 cases after narrowing the two server decoders to private visibility; it
adds no distinct cases. The initial WorldServer check (4m17s) predates those
private diagnostic/visibility changes; subsequent database and WorldServer
tests compile them. It is not relabeled as final committed-candidate evidence.
Reported completed commands alone total over 654 seconds, excluding the failed
Data compile; this is coding feedback, not proof of the ordinary acceptance
campaign's 600-second target. No duplicate parent campaign was run.

The parent inspected the actual diff, raw outcomes and all twenty code-file
hashes. The new server decoder/test children are 219/113 lines; key and World
adapters are 624/666, Data loading tests 636, and World scalar composition/tests
144/97. The existing Core composition root remains 1,119. App grows 5,689 to
5,695 (+6) for the required asynchronous PPM call; temporary reviewed ceiling
5,695 retains #43 integration ownership and the same final architecture review
expiry. No architecture baseline was regenerated. Exact joint proc preparation,
charge consumers, native lifecycle, normal casts, QA and final acceptance remain
required. These modeled fixtures are not real SQL/runtime/capture evidence.
This remains a local UNVALIDATED checkpoint.

9i3C3d4l1 makes canonical server-effect base points int32, retaining raw float
in the input row and SQL DTO. Fallible row conversion truncates valid fractions
toward zero before the accepted effect vector is built; blank slots retain
integer zero. Nonfinite values and values outside [-2147483648,2147483648)
return the original row with BasePointsOutOfRange. This is an explicit defensive
valid-input boundary for legacy undefined float-to-int conversions, not claimed
parity for those invalid inputs. Radius warnings retain the raw row. Existing
difficulty normalization, input order and duplicate-parent rules are unchanged.

Native ProcEntry, ProcEventSpellInfo, ProcSource and ImplicitProcSource family
fields and matching arguments are now uint32. SQL proc rows and their DTO remain
uint16; entry construction widens their values without changing them. Regular
ClassOptions class-set fields remain uint8. Source anchors at the pinned target
are SpellMgr.cpp:2755, DB2Structure.h:3514, SpellInfo.cpp:407–435,
SpellInfo.h:409,514, SpellMgr.h:317 and SpellMgr.cpp:1533. The parent reviewed
those scalar boundaries and the actual Rust callers; no SQL or WorldServer
consumer change is needed by this unit.

On `21ae4503d` plus the reviewed delta, aarch64/jobs=1 coding feedback passed
`cargo check -p wow-data` and library filters `serverside_effect_base_points`
(3), `serverside_spell_effect_store_` (3), `spell_family_name_` (2), and
`spell_proc_source_builds_from_loaded_spell_and_db2_stores_like_cpp` (1).
These are nine distinct cases, including fractional signs, representable
boundaries, NaN/infinity rejection, raw diagnostics, dense blank slots,
full-family generated matching and unchanged SQL-family widening.

Raw logs, real exit files, timings and the final patch are in
`/tmp/rustycore-c3d4l1-logs/`. The five `*-candidate` check/filter logs exit
zero; their Cargo-reported durations are 0.11s, 15.72s, 0.11s, 0.13s and 0.10s.
The initial `test-serverside-effect-base-points` compile failure (101) is retained:
a test moved its captured source from an FnMut closure, corrected before the
passing candidate. Earlier successful working iterations are retained separately
and add no distinct cases or acceptance verdict. The parent reviewed the diff,
outcomes and all five code-file hashes without another Cargo run. Worker model
and effort telemetry were unavailable. Physical review: corrections.rs is 792
lines, catalog.rs 862, state_2.rs 699, the adjusted alias fixture 345, and spell_6
tests 875. Exact joint proc preparation, charge consumers and the other macro
blocks remain required. This is a local UNVALIDATED checkpoint, not final or
runtime acceptance.

9i3C3d4l2a moves exactly two existing scalar readers into Data's private
resolution module and exposes them through the existing spell exports.
World retains its selected_aura_options_entry and selected_spell_family names
as crate-private aliases. The parent reviewed identical bodies and argument /
result types apart from the equivalent error alias and cross-crate visibility.
The regular-family helper retains uint16 because its ClassSet input is uint8;
native proc metadata still widens that value into uint32. Mechanic, level and
raw-Misc readers remain in World. This structural move precedes the separate
semantic proc-source correction; existing source anchors and behavior contracts
remain unchanged. On base `4b3e10f4c`, no Cargo or validation was run under the
explicit level-1 deferral. The files are 198 and 109 lines respectively. The
next completed implementation will compile its consumers and exercise affected
reader cases. No new trait, context, crate or mutable authority is introduced.
This is a local UNVALIDATED structural checkpoint, not accepted evidence.

9i3C3d4l2b replaces the payload-only/difficulty-zero proc projection with
the existing exact regular SpellInfo keys and accepted server-side sources.
One temporary source map supplies explicit SQL/rank lookup through the known
difficulty-zero fallback and implicit generation at every actual key. A second
temporary key lookup records only calls the row loader can reach, respecting
non-first all-ranks rejection and duplicate-entry stops. It holds no runtime
authority. Known total absence avoids irrelevant graph cycles; required cycles
and producer overlap fail construction before the canonical proc Arc is
installed. Errors retain the affected exact key or SQL/rank lookup context.

Regular sources use the shared effective AuraOptions/family readers and the
SpellStore's canonical attributes and indexed effects, without a separate raw
Misc snapshot. Key-only SpellInfos retain source defaults. Effect vectors are
dense through the highest resolved slot, with zero holes and explicit inert
fallback stoppers. The two relevant effect/attribute hydration sites preserve
uint32 SpellID key bits through signed storage. TriggerSpell also preserves
its signed-input bits when projected into uint32. Other legacy high-ID
hydration sites are not claimed repaired. Accepted server sources retain their
full-width proc/family fields, canonical int32 base points, and Attr3 word.
Negative raw server effect types are rejected: C++ assigns GetInt32 into the
unsigned Effect field before its upper-bound check. Source anchors at the
pinned target are SpellMgr.cpp:490–509,1500–1634,1730–1888,2500–2665,2735,
2786–2792 and SpellInfo.cpp:407–435,1148–1164,1337–1353. This is metadata and
entry-presence preparation, not general CalcValue/scaling/proc-runtime parity.

On base `9d8b22194` plus the reviewed delta, aarch64/jobs=1 coding feedback
passed `cargo check -p world-server --lib` and 33 distinct cases: Data
`proc_source_preparation` (13), the existing loaded-source and invalid-server
effect cases (one each), four existing SQL/rank/implicit regressions, and
WorldSpell `selected_catalog` (13, including five publication-reader cases),
and its separate family-alias case (one). These exercise the l2a consumers.
Full command logs, real exits
and timings remain in `/tmp/rustycore-c3d4l2b-logs/`. The initial successful
server check (224.93 seconds) preceded the shared-resolver correction. Its
rerun retained a missing-macro-import failure (101), then passed in 34.59
seconds. Data retained two fixture type errors (101), then a 12/13 run whose
incorrect expected family mask was repaired from SpellMgr.cpp:1811–1815:
implicit masks come from effect SpellClassMask. The final 13/13 run passed in
9.18 seconds; the WorldSpell filter passed in 81.53 seconds. These are coding
feedback, not a completed-candidate campaign or proof of its 600-second target.
No positive WorldServer proc-composition test was found or claimed.
The worker reported generic GPT-6 identity without effort telemetry; the exact
runtime variant/effort is unavailable.

The parent reviewed the actual diff, callers, source anchors, raw outcomes and
all thirteen code-file hashes without a duplicate Cargo run. The new private
preparation/test children are 264/760 lines; catalog.rs is 897 and the adjusted
existing proc tests are 907. App grows 5,695 to 5,696 (+1) for the two joint
source arguments replacing raw Misc at the same startup call. Temporary
reviewed ceiling 5,696 retains #43 integration ownership and expires at the
same final architecture review; no baseline was regenerated. The charge
consumers and remaining macro blocks are still required. This is a local
UNVALIDATED checkpoint, not final, SQL, runtime or capture acceptance.

9i3C3d4m1 adds pure selected max-charge calculation and a separate fresh
self-caster modifier-admission proof in WorldSpell. The caller must supply
the actual resolved SpellInfo difficulty, not raw Aura/map difficulty.
AuraOptions preserves its full signed-input uint32 bits; the canonical proc
entry at that key or its guarded fallback overrides the value even when zero.
Only the final result converts to uint8. Required stores remain unavailable
when absent; known empty effective catalogs avoid irrelevant graph traversal.
Fresh admission uses exact Attr3 IGNORE_CASTER_MODIFIERS or known zero family
flags plus an empty inherited label list, including LabelID zero. A known
finite chain ending before the base row keeps source defaults; an unavailable
graph or required cycle remains an error. ClassOptions signed keys preserve
high-bit uint32 identities. Possible class-mask/label modifiers remain a
preparation error until their canonical runtime owner exists. Source anchors
are SpellAuras.cpp:965–976,1088–1090, SpellMgr.cpp:490–509,2555–2556,2649–2650,
SpellInfo.cpp:1784–1817, and Object.cpp:2274 at the pinned target.

On base `4f3841423` plus the reviewed three-file delta, aarch64/jobs=1 worker
feedback ran `cargo test -p wow-world-spell --lib selected_charges`: the final
run passed all ten cases, exit zero, in 5.188 seconds. Its exact command,
timestamps, exit and raw output are in
`/tmp/rustycore-c3d4m1-logs/focused-selected-charges-final-fixture-rerun.{txt,log}`.
The first attempt retains a moved FnMut test capture failure (101). After an
earlier 10/10 pass, parent review strengthened the generated-zero fixture so
the exact zero and nonzero fallback coexist, and added an unreachable nonzero
class-row case. That correction retains a test-only name-shadowing failure
(101) before the final green run. Earlier green iterations add no distinct
cases. The parent reviewed all code, corrected known-absence/high-ID rules,
and verified all three hashes without another Cargo run. Production and test
children are 150/547 lines. The worker reported generic GPT-6 without effort
telemetry; the exact runtime variant/effort is unavailable.

These helpers have no production caller yet; their module exports and unused
warnings do not establish application integration. Root export, actual-key
resolution, fresh creation and existing-aura refresh consumers, their fixtures,
and the recorded original-refresh findings still need implementation. Saved
load normalization remains unchanged. No workspace check, final campaign or
runtime/SQL/capture evidence was run. This is a local UNVALIDATED checkpoint.

9i3C3d4m2a moves the existing selected key-difficulty reader to Data as
resolve_spell_info_key_difficulty, retaining World's previous name as an
alias. The parent reviewed the same argument/result types and body, apart
from equivalent shared resolver/error names. The exact-key authority and
GetSpellInfo fallback contract remain unchanged; existing source anchors
continue to apply. The next App consumer will use the same reader instead
of inventing another key cache or treating hydrated payload as existence.

Selected World fixtures now prepare keys through the existing production
seed loader. A private test helper supplies complete empty WDC4 contributors,
ordered Hotfix input batches, effective SpellNames and explicit keys; it
does not fabricate a payload or expose a new Data setter. The new identity
case asserts nonzero-only and high-bit keys, absent difficulty zero and no
hydrated payload. It has not been executed. Positive release/sickness/mixed
fixtures preserve their keys while replacing payload, and effect fixtures
normalize difficulty before contributing keys. Parent inspection caught and
removed a borrowed iterator across the moved effect vector, without a Cargo
search loop. The unrelated inert-aura test retains its original store setup.

Base is `dbc933d37` plus the reviewed ten-code-file delta. No Cargo, metadata,
formatting or validation command was run under the explicit level-1 deferral.
The parent reviewed actual diffs, consumers and all ten file hashes; the new
fixture is 155 lines, Data resolution 220 and the World alias module 90.
The existing corpse/lifecycle fixture files are 1,887/1,046 lines. Their
cohesion review retains the current selected lifecycle groups; release-cast
fixtures and provenance/stat subgroups are the next natural splits if needed
at the macro's final physical review. No baseline was regenerated. The worker
reported generic GPT-6 identity with no effort telemetry. Root exports,
production charge admission/application and completed-consumer feedback remain
pending; this is a local UNVALIDATED preparatory checkpoint.

Independent read-only QA review also pins the bot's returned bytes to the
logical decrypted/uncompressed capture-diff model. The operation guide records
that client arrival order is not server cross-socket enqueue order; the bot
trace does not replace required server captures and provenance. Reference
WorldSocket.cpp:526–532 was clean relative to the pinned target in that file's
scope. No bot, server, SQL, capture or restart operation was executed.

9i3C3d4m2b wires the selected charge readers into fresh creation and existing
timer refresh. Fresh preparation returns flags, provenance and charges as one
fallible result; a matching refresh still ignores fresh-only failures. Refresh
charges resolve the stored raw difficulty to the existing SpellInfo key and
use CalcMaxCharges without a caster. Non-timer refresh preserves charges
without querying the proc catalog. The Core refresh witness now checks raw
difficulty before mutation. Shared persisted-load construction is unchanged.
The modifier-absence proof is caster-independent, preserving the supported
foreign-original-caster sickness operation. Immutable catalog references flow
through the existing application context and both trainer constructors.

The actual-key requirement precedes execution publication. Positive sparse
fixtures retain genuinely nonzero-only keys; the transient selected FULL
projection and explicit completion cooldown gate implement the bounded
contract recorded below. They do not complete ordinary sparse cast metadata
or the general FULL-mask engine. Source remains the pinned target
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, with anchors recorded in that
contract and the charge decision. The parent reviewed the actual application,
Core, spell, World and fixture diffs and their consumers. The execution file
is 1,178 lines; its selected projection is a separate 220-line module. New
charge tests are 789 lines. No physical baseline or ownership ceiling changed.

Focused worker feedback used base HEAD
`e005b5fc897706b3b91518d4e52ddd7a11f963b5` plus the reviewed uncommitted
sources, on this aarch64 host, sequentially with PROTOC set, one Cargo job
and this checkout's target directory. Final commands and raw logs under
`/tmp/rustycore-c3d4m2b-logs/` are:

- `cargo check -p world-server --lib`: exit 0, 47.85 seconds,
  `world-server-check-completion-final.log`, 10:23:05–10:23:53 UTC.
- `cargo test -p wow-world --lib spirit_healer`: exit 0, 48 passed,
  `world-spirit-healer-cooldown-final.log`, 10:24:04–10:25:57 UTC.
- `cargo test -p wow-world --lib selected_charge_fixture_preserves_exact_canonical_spell_info_keys_without_payloads`:
  exit 0, one passed,
  `world-selected-charge-fixture-final.log`, completed 10:26:08 UTC.
- `cargo test -p wow-world --lib repop_request_creates_saved_corpse_and_releases_to_graveyard_like_cpp`:
  exit 0, one passed,
  `world-corpse-final.log`, completed 10:26:19 UTC.

These are 50 distinct World cases, not 50 new cases or a final campaign.
Earlier wrapper/compile failures, the ten initial integration findings and
the later foreign-caster fixture failure remain in the same log directory.
Repairs include real nonzero keys and projection, generated-proc flags,
preserving the proc store while dropping only ClassOptions, and attaching the
foreign Player to the owner's actual map instance. The final healer run also
covers Base0 and sparse selected FULL cooldown behavior, preserving existing
history and emitting no CooldownEvent. Coding and repair time is not evidence
for the 600-second ordinary acceptance budget. The worker reports generic
GPT-6 with effort telemetry unavailable. No parent Cargo, final campaign,
runtime, SQL or capture operation was performed. This remains UNVALIDATED at
macro level; original refresh findings and the other three blocks stay open.

9i3C3d4m3 corrects the original selected Aura refresh findings. The pure
selected_aura_refresh_policy reader resolves the stored raw difficulty to
the original SpellInfo key before reading its stack policy, attributes and
duration. Both application consumers use those old facts and the old actual
key for charges and effects. Ghost/Wisp now copies only incoming base points
into the original effect rows, retaining their DieSides, aura type and misc
data. The slot, raw difficulty, provenance, indexed amounts and recalculation
mask remain canonical. Fresh-only stack and duration checks follow the
matching refresh branch. Existing no-timer application refreshes do not read
unused duration or proc catalogs; the complete cast still retains its own
admission requirements.

Core computes the refreshed lifetime under the canonical mutation borrow,
using its current remaining duration before changing any state. Attr13's
extension uses the source's f32 percentage and truncation, with a checked
addition; overflow leaves the Aura and effect metadata unchanged. A normal
refresh accepts signed remaining values such as -1 and restores the old base
duration. Invalid duration INT_MIN is rejected instead of saturating its
absolute value. The selected sickness removal guard now resolves the Aura's
actual key instead of requiring a legacy difficulty-zero payload. A positive
nonzero-only expiry and a keyless negative removal case cover that consumer.

Source is pinned target `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Unit.cpp:3244–3278, SpellAuras.cpp:869–895,933–952,1021–1094,
SpellInfo.cpp:3905–3910, common/Utilities/Util.h:72–75 and
Miscellaneous/SharedDefines.h:932. No new modifier engine, cumulative stacking
implementation, stored SpellInfo mirror, Player owner, lock or clock was
introduced. The existing general MaxAuraStacks and effect modifier boundaries
remain; this checkpoint does not retire them or establish capture parity.

Worker coding feedback on this aarch64 host used HEAD `31e8f06d0` plus the
reviewed uncommitted source delta, PROTOC set, one Cargo job and this checkout's
target directory. Raw logs are in `/tmp/rustycore-c3d4m3-logs/`:

- `cargo check -p world-server --lib`: exit 0, 3 minutes 51 seconds,
  `01-world-server-check.log`, 10:55:14–10:59:05 UTC.
- `cargo test -p wow-world --lib spirit_healer`: exit 0, 54 passed,
  `02-world-spirit-healer-tests.log`, 10:59:14–11:02:37 UTC.
- `cargo test -p wow-world-core --lib selected_duration_tests`: exit 0,
  four passed, `04-world-core-selected-duration-tests.log`,
  11:03:37 UTC. The preceding `selected_refresh_duration` filter ran zero
  tests in `03-world-core-selected-refresh-duration.log` and proves nothing.

These are 58 distinct focused cases, including six new World cases and four
new Core cases. The unit's 10:55:14–11:03:37 feedback interval is not a final
campaign or evidence that the complete ordinary acceptance fits 600 seconds.
Parent review corrected arithmetic oracles, signed-remaining handling and
INT_MIN admission before the feedback, made effect-property fixtures
deterministic, and kept the keyless fixture's fabricated base payload strictly
inside its negative case. The policy reader, Core implementation, mirrored
duration tests and new World refresh tests are 104/534/39/740 lines;
Ghost/Wisp is 986 lines. Physical baselines remain unchanged. No parent Cargo,
runtime, SQL, capture or restart operation was executed. This is an UNVALIDATED
local macro checkpoint; native death, recovery, normal casts, QA and final
acceptance remain open.

The worker reported generic GPT-6 with effort telemetry unavailable. Its
recursive rustfmt invocation also touched earlier #43 modules outside M3.
After the feedback and the worker's ownership confirmation, the parent removed
those formatter-only file deltas, retaining the sibling charge-test helper
visibility changes and formatting in semantic M3 files. The source behavior
and new test bodies are unchanged; the earlier feedback is not relabelled as
a final run at the subsequent commit. The parent reviewed the actual M3 diffs
and retained hashes for the policy, Core, application and test files.

Selected stat-view integration, 2026-10-26, base `c7829b4f9`: the borrowed
`PlayerStatAuraView` now resolves an existing Aura's stored raw difficulty to
its actual SpellInfo key and indexed effect rows. Its four effect-query
delegates and stat multipliers share active-mask selection and prefer the
canonical represented amount for each effect index. The existing no-caster
CalcValue fallback remains for the fresh calculation before selected metadata
initialization; this is not a new general modifier engine or amount authority.
Original effect properties remain selected across a refresh from another map
difficulty. Group rules, arithmetic and traversal policy are preserved.

Construction is fallible through the four existing Option-returning adapters.
G/W and every nonzero sickness spell configured by any race require real keys,
including a sickness spell carried by a Player of another race. Other known
canonical keys use the exact reader too. An unresolved key, graph or active
effect rejects the calculation before native stats change. A missing race
catalog is unknown, so a keyless application cannot silently enter the legacy
path; an authoritative race catalog can prove it is an ordinary keyless aura.
No production setter, mutable Aura mirror, Player clone, lock or clock is added.

The new mirrored World test module exercises a true difficulty-three-only
fresh cast, original properties with incoming amounts on refresh, expiry,
represented amounts distinct from base points, required keyless rejection,
unresolvable difficulty, fixed G/W keys, missing race authority and ordinary
same-effect grouping. Fresh/refresh/expiry assertions read the resulting
native stats immediately, without a later forced stat update repairing the
operation. Five existing positive legacy fixtures now install an authoritative
empty race catalog; their expected health, amount and selector behavior is
unchanged, and the generic session fixture still has no fabricated authority.

Source anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
`Unit.cpp:4818–4875,9033–9073` and
`SpellAuraEffects.cpp:3566–3604,4525–4548`. The test oracles also retain
`SpellInfo.cpp:495–524` (DieSides zero adds nothing) and source f32 operation
order, rather than replacing `1 - 60 / 100` with the differently rounded
literal `0.4`. The inspected source files have no local changes at this pin.

Worker coding feedback used `c7829b4f94e110f5ffbebf039f6bccaf694d4bc2`
plus the reviewed source delta on this aarch64 host, with PROTOC set, one
Cargo job and `/home/server/rustycore/target`. Raw commands, UTC times and
real exits are retained in `/tmp/rustycore-selected-stat-view-logs/`:

- `cargo check -p world-server --lib`: exit 0, 39.77 seconds,
  `cargo-check-world-server-lib.log`, 12:19:42–12:20:21 UTC.
- `cargo test -p wow-world --lib selected_stats`: exit 0, seven passed,
  `cargo-test-wow-world-selected-stats-attempt6.log`, 12:32:46–12:34:03 UTC.
- `cargo test -p wow-world --lib total_stat_percentage`: exit 0, five passed,
  `cargo-test-wow-world-total-stat-percentage-attempt2.log`,
  12:35:55–12:37:06 UTC.
- `cargo test -p wow-world --lib spell_group_stack_rule`: exit 0, two passed,
  `cargo-test-wow-world-spell-group-stack-rule.log`, 12:37:12–12:37:13 UTC.
- The focused native Map VALUES sickness case passed at 12:39:44 UTC in
  `cargo-test-wow-world-spirit-healer-lifecycle-stat.log`; it is also covered
  by the final Spirit Healer filter below and is not counted twice.
- After shared-fixture repair, `cargo check -p wow-world --lib`: exit 0,
  `cargo-check-wow-world-after-ghost-fixture-preservation.log`,
  12:47:52–12:48:57 UTC.
- `cargo test -p wow-world --lib spirit_healer`: exit 0, 61 passed,
  `cargo-test-wow-world-spirit-healer-after-ghost-fixture-preservation.log`,
  12:49:08–12:50:34 UTC. This includes the seven new cases.

The union is 68 distinct World cases, seven new. Earlier failures are retained:
test tuple arity; an incomplete fixture reset leaving stale same-slot effect
snapshots; DieSides/f32 oracle errors; missing race authority in five legacy
fixtures; and 19 failures in the broader Spirit Healer filter. The last group
came from preserving released Ghost/Wisp keys while dropping their original
payload/effect data when rebuilding the selected fixture's store. That fixture
now restores only its original G/W payloads and attributes under already
loader-proven zero keys. The positive nonzero-only sickness fixture still has
no zero key or legacy sickness payload. Production rejection was not weakened.

The Core view and new World tests are 518 and 574 lines. The existing Data
character-progression file is 1040 lines after its nine-line predicate; the
predicate belongs to its existing ChrRacesStore implementation and adds no
new owner. The repaired shared World fixture is 802 lines. No physical
baseline was regenerated. The worker-owned patch hash is
`7333ec7fb9c4f4ed9b0ee610a99b72bad80fb6e32d87e5e6e086d93c85413232`;
the view and new test hashes are respectively
`43d82fc0e27f5faebe58bfed92fe8823e257bf6e4c7499f02949ddd3c7d5287f`
and `5de9d5a57256973988cab809bb5421727fcf2613bf62458f226bb56d4a1da493`.
The parent reviewed the actual production, fixture and test changes. The
worker reports that effective model/effort metadata is unavailable; the
configured route is not claimed as observed execution telemetry.

The 12:19:42–12:50:34 interval is 1852 seconds of coding, repair and focused
feedback, not a final campaign or proof of the 600-second ordinary acceptance
target. No parent Cargo, runtime, SQL, capture, restart or publication was
performed. This is an UNVALIDATED local macro checkpoint, not final acceptance
at a later commit. Native death, recovery, normal casts, QA and the complete
candidate acceptance remain open.

Ghost/Wisp original-metadata integration, 2026-10-26, base `59f28568d`:
Application's loaded-effect and removal readers resolve each Aura's stored
raw difficulty to its actual canonical SpellInfo key and then its indexed
effects. The narrow Spell Wisp transform participant reuses whole-spell
polarity, resolves the current transform at the incoming Aura's cast difficulty,
and invokes the existing native transform operation. Difficulty errors remain
errors; they do not become evidence that the current transform is absent.
A positive Wisp does not overwrite a negative current transform or its display.
Class-options input is borrowed through the existing Application catalogs and
all three concrete constructors; generic transform behavior remains bounded
by its existing consumers.

The selected loader now uses its fallible selected metadata instead of requiring
`SpellStore::get` at difficulty zero before that resolver can run. Nonselected
legacy admission and all raw-difficulty, effect, charge, caster and duration
checks remain in place. Full-save Ghost/Wisp capture also no longer requires
zero-difficulty payloads: canonical family keys establish known families,
known-empty families need no unrelated zero-difficulty attributes/effects,
and each active Aura resolves its own actual key before eligibility and effect
capture. Transaction, item/caster key, tombstone and publication semantics
were not replaced. The existing indexed-save fixture now supplies real canonical
keys rather than treating payload hydration as key authority.

The tests exercise native capture, persisted-row conversion and the actual
loader with both selected spells having only difficulty-three keys and no
legacy payload. They verify complete aura authority, flags, visibility, speeds
and Wisp display without repairing those values after loading. Additional cases
cover conflicting zero-difficulty metadata, current-transform polarity at the
incoming cast difficulty, retained negative-transform display, malformed graphs,
missing active effects, missing keys and known-empty families.

Source anchors at target `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Player.cpp:18034–18140,19811–19851; SpellMgr.cpp:675–693;
SpellAuras.cpp:1134–1195; SpellAuraEffects.cpp:1718–1742,1935–1951,
3159–3174,3210–3218; Unit.cpp:10038–10100,11763–11870.

Sequential worker coding feedback used the checkout's `target`, local PROTOC
and one Cargo job. Raw commands, UTC times and actual exits are retained in
`/tmp/rustycore-ghost-original-metadata-logs`:

- Final production check: `cargo check -p wow-world-application --lib`,
  exit 0, 16:11:32–16:11:52.
- Final World feedback: `cargo test -p wow-world --lib ghost_wisp`,
  54 passed, exit 0, 16:16:55–16:18:20. This is 46 existing and eight new cases.
- The first World run failed with 46 passed/five failed, exit 101,
  15:18:52–15:23:34. After the transform and loader changes, the next run
  failed with 50 passed/three failed, exit 101, 15:59:18–16:01:32.
  Native capture then exposed the remaining save entry gate. Its repair
  produced 53 passed/one existing fixture failed, exit 101, 16:11:58–16:14:14;
  that fixture gained canonical keys before the final passing run.

The parent reviewed the production changes, actual callers, fixtures, test
cases and raw results, and verified all 14 implementation file hashes in
`owned-files-20261026T1618.sha256`, manifest SHA-256
`c5d6604e304a6ea25edaf2a3496936f23e12abe9660d7f3fed5ec1e18a98cfe8`.
This identifies the tested dirty working inputs at the stated base, not final
acceptance at a later commit. Effective worker model/effort telemetry remains
unavailable. The 15:18:24–16:18:20 feedback/repair interval is 3596 seconds of
coding feedback, not the completed-delivery campaign or proof of its 600-second
ordinary performance target. The Application file's 986→1042-line cohesion
review is recorded above; the new test leaf is 822 lines. No numeric baseline
was regenerated.

This is an UNVALIDATED local macro checkpoint. Native-error propagation,
remaining recovery continuations, the sibling sickness full-save entry gate,
native-death integration, normal casts, QA and complete candidate acceptance
remain open. The sickness save gate was inspected but was not changed by this
Ghost/Wisp unit. No parent Cargo, runtime, SQL, restart, capture or publication
was performed.

Recovery failure, receipt and retirement integration, 2026-10-26,
base `1e2b1799315662eb0b1ad6cea328d4b932e845d1`:
the existing Player resurrection continuation now owns complete reclaim,
spirit-healer, login-corpse and forced-graveyard recovery operations. Private
WorldEntities modules separate their phase execution without creating another
Player, pending-deletion authority, clock, task or mutable save mirror.
An absent corpse is not confirmation of an earlier uncertain deletion: failed
or cancelled deletion retains its nonreplayable phase. A genuine no-corpse
entry preserves the source's no-delete/no-save behavior. Login retains
triggerSave=false, fresh post-deletion alive/AT_LOGIN_RESURRECT reads, the
default applySickness=false, and its subsequent ghost restoration. Forced
recovery waits for confirmed deletion/save before graveyard selection,
countdown stop and teleport, rereading the current map and position.

The incarnation-bound native save receipt promotes only its captured, matching
SavingPlayer continuation to PlayerSaveApplied. It does not clear remaining
post-save work. The admitted PlayerRecovery rail resumes confirmed work before
Unit timers, rechecks admission after awaits and preserves Unavailable when an
unresolved phase disconnects the session. Failed, unknown or cancelled
pre-ACK saves cannot be retried as confirmed work. Spirit-healer durability
and its cached corpse graveyard survive deferred saves without reapplication.
A returned healer TeleportTo hands ACK ownership to the native transfer;
cancellation before that return retains the in-flight operation. A registered
worldport ACK leaves a newly admitted far transfer for its own ACK, while
disconnect finalization finishes successive native transfers before saving.
Application and lower lifecycle cleanup retain the exact Player incarnation
and login claim while recovery remains unresolved. The existing Server
finalization retention path remains the owner of an unresolved session.

Source anchors at pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
Player.cpp:4262–4403,4509–4515,4743–4795,18157–18180,
Player.h:2117, NPCHandler.cpp:347–370, MiscHandler.cpp:416–445,
CharacterHandler.cpp:1227–1243, MovementHandler.cpp:234 and
WorldSession.cpp:547,602. The retained failure phases describe the explicit
async recovery contract; local controlled ports do not prove real database
durability or restart recovery. Existing repop tests now use complete native
fixtures and distinguish ResurrectPlayer's exact cleared DeathReleaseLoc from
a graveyard-destination packet. The reclaim no-replay test inspects the real
full-save health snapshot before worldport's final stat recalculation.

Scoped coding feedback used PROTOC=/home/ubuntu/.local/protoc/bin/protoc,
CARGO_BUILD_JOBS=1 and CARGO_TARGET_DIR=/home/server/rustycore/target:

- `cargo check -p wow-world-application --lib`: exit 0, 18:46:49–18:47:21 UTC.
- `cargo test -p wow-world --lib recovery_fences`: 33 passed, exit 0,
  19:20:04–19:21:22; repeated after the repop fixture repairs at
  19:25:47–19:25:48, again 33 passed, exit 0.
- `cargo test -p wow-world --lib resurrection`: 25 passed, exit 0, 19:21:30.
- `cargo test -p wow-world --lib corpse_login`: 13 passed, exit 0, 19:21:34.
- `cargo test -p wow-world --lib repop`: 18 passed, exit 0, 19:24:30–19:25:43.
- `cargo check -p world-server --bin world-server`: exit 0, 19:25:57–19:30:33.

The four filters cover 77 distinct World test identities, including 33 new
recovery cases. Logs and the deduplicated identities are retained in
`/tmp/rustycore-recovery-fences-logs`. Initial compilation and fixture failures
remain recorded. Run 08 ended with Cargo exit 101 after SIGTERM of the owned
stalled test: 31 cases had passed, one failed and one was blocked. The parent
stack trace proved a bounded fixture packet-channel send in CancelCombat,
not a persistence or map-lock wait. The test now captures activation output
and concurrently collects ACK output; production transport is unchanged.

The parent reviewed the production consumers, failure contracts, fixtures and
raw results, and verified all 77 hashes in `final-owned-files.sha256`, digest
`260c9493ac665ffa120551a9559af59cf36ffbf3810f8afb17143d1c1064acb6`.
Those hashes include 55 changed Rust inputs and 22 restored/unchanged files;
the 55-input parent snapshot has digest
`062fba4f72da8d09caca741f6acc2100ed94df5510c0f8a05a9651a3a218f86f`.
They identify tested working inputs at the stated base, not final acceptance
at a later commit. Effective worker effort telemetry remains unavailable.
The 18:35:40–19:30:33 feedback/repair interval is 3293 seconds of coding
feedback, not the completed-delivery campaign or proof of its 600-second
ordinary performance target.

This is an UNVALIDATED local macro checkpoint. The sibling sickness full-save
entry gate, native-death integration, normal SelfRes/obtain casts, QA and
complete candidate acceptance remain open. No runtime installation, SQL,
restart, fresh capture, publication or issue closure was performed.

Native dead-caster CheckCast, 2026-10-27, base
`c7a30f42a27ff9057bfe5cb4941b33c25dddeaab` plus 11 Rust working inputs:
the shared gate reads the actual typed Unit caster, including Vehicle GUIDs,
and Unit::is_alive rather than the convenience health predicate. Native
Alive/zero-health casts during resurrection are admitted. Source disabled and
busy checks retain priority; death precedes cooldowns, GCD, power and effects
in client/server preparation and is rechecked before launch. Passive,
AllowCastWhileDead and the currently represented direct FULL-mask triggers
remain exemptions. Attribute reads retain actual SpellInfo key selection,
loaded-empty Misc defaults and unavailable-source errors. Signed internal
SpellID storage preserves the Source uint32 bits.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors are
Spell.cpp:3453–3467,5643–5644,8031–8034; SharedDefines.h:437,454;
SpellInfo.h:331 and SpellInfo.cpp:1144–1195. Existing nonzero trigger-flag
writers are direct corpse/login casts; this does not establish an AuraEffect
trigger-origin executor. The current normal-client payload path is also not
upgraded into a general nonzero-only SpellInfo engine by this gate. Complete
registered SelfRes integration must still prove its actual server-normal
nonzero-only source without a synthetic base payload.

Sequential coding feedback used PROTOC=/home/ubuntu/.local/protoc/bin/protoc,
CARGO_BUILD_JOBS=1 and CARGO_TARGET_DIR=/home/server/rustycore/target:

| Cargo command | UTC interval | Result |
| --- | --- | --- |
| `check -p wow-world-spell` | 02:44:24–02:44:26 | exit 0 |
| `check -p wow-world` | 02:51:59 | exit 0 |
| `test -p wow-world-spell caster` | 02:44:48–02:44:56 | 23 passed |
| `test -p wow-world dead_caster` | 02:52:05–02:52:36 | 5 passed |
| `test -p wow-world self_resurrect` | 02:53:10–02:54:05 | 11 passed |

The parent verified 39 distinct target-qualified library identities; zero-test
integration targets are not credited. The Source EffectThreat dead-caster
regression now supplies AllowCastWhileDead and its selected key before
exercising the effect's own alive guard (SpellEffects.cpp:2864–2880).
Existing SelfRes cancellation, corpse-save and fail-stop tests remain green.
Logs in `/tmp/rustycore-dead-cast-admission-logs` preserve the mutable-metadata
compile error, test compilation failures, the 3/2 and 4/1 World results, and
the 10/1 SelfRes result before repair. In particular, the Creature fixture's
map/NPC-flag argument order was corrected; typed caster lookup was retained.
The 02:18:41–02:54:05 implementation/feedback interval is 2124 seconds, not
the final campaign or proof of its ordinary 600-second performance target.

All 11 inputs match `changed-inputs.json`, digest
`73275c71adaee934d45bf0c8af850432cb8e478cef33ea4d61c15883adcb8fc2`;
the tracked Rust diff digest is
`8bf0fa75cbfa438284e1bec6b134b4aea7f0e194017b3c38f147716cf92041bd`.
These identify working inputs at the stated base with parent-owned State text
also dirty, not a test of the later commit. The worker declared GPT-6 from
runtime instructions; specific model/effort telemetry is unavailable.
The new private test leaf has 568 lines. The pre-existing cast-lifecycle
parent grows only by its three-line mount, 1052→1055; its reviewed ceiling is
1055 through #43 acceptance, when its identity/publication split must be
reviewed before further growth. This remains an UNVALIDATED macro checkpoint.
Registered SelfRes, native publication/death, QA, final and authorized live
evidence remain required.

Native recovery publication review at `c7a30f42a`, 2026-10-27: the scoped
packet vectors above are not Source cadence evidence. Pinned
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` Player.cpp:4320–4403,23322–23343
calls default, deferred UpdateObjectVisibility; Unit.cpp:9214–9231 marks
health fields. Map.cpp:777,1929–1948 owns SendObjectUpdates. Rust already has
the corresponding native visibility notification and
`wow-map/src/map/send_object_updates.rs` owner, with post-guard Server fanout
in `runtime/map/update_loop.rs:495–505`. Selected sickness stat application
already uses the native VALUES path; its unrelated legacy path is separate.
The remaining mismatch is concrete: native resurrection, request-value
application and SelfRes-value application send immediate self VALUES, and
resurrection also invokes the position-threshold visibility scanner directly.
Those snapshots neither consume native masks nor replace the later Map flush.
In SelfRes they expose the intermediate Alive/zero-health state before obtain
and the caller's restored values. The selected correction is to retain these
native dirty fields for the existing Map publication owner and request native
VISIBILITY_CHANGED notification, then test that actual phase and its typed
snapshot. The correction below installs that ownership; fresh paired action
captures and final acceptance remain required.

Native recovery publication correction, 2026-10-27, base
`4ca41413441f66c5a4c1b898c659addd618bbd6e` plus nine Rust working inputs:
native resurrection, request-data restoration and SelfRes restoration now
retain dirty native fields for Map::SendObjectUpdates. Resurrection queues
VISIBILITY_CHANGED on the actual in-world Object instead of invoking the
position-threshold scanner. Existing immediate movement, aura, corpse and
spell packets and the operation's ownership/save fences remain in place.
The source anchors and existing post-guard Server consumer are those above.
Tests drive the real Map owner and World consumer, check final health/powers
and the cleared SelfRes field using native power indices, and verify one-time
mask consumption and delivery. The out-of-world SelfRes effect remains a
no-op without a visibility notification; the one-shot helper does not gain
permission to complete without an active residence.

Sequential coding feedback on the aarch64 host used
PROTOC=/home/ubuntu/.local/protoc/bin/protoc, CARGO_BUILD_JOBS=1 and
CARGO_TARGET_DIR=/home/server/rustycore/target:

| Cargo command | UTC interval | Result |
| --- | --- | --- |
| `check -p wow-world-entities` | 03:13:11–03:13:13 | exit 0 |
| `check -p wow-world` | 03:13:18–03:13:35 | exit 0 |
| `test -p wow-world resurrect` | 03:30:52–03:32:26 | 55 library + 2 production integration cases passed |
| `test -p wow-world recovery_fences` | 03:24:13 | 35 passed |
| `test -p wow-world reclaim` | 03:24:20 | 16 passed |
| `test -p wow-world spirit_healer` | 03:32:55–03:34:26 | 83 passed |
| `test -p wow-world corpse_login` | 03:24:45 | 13 passed |

The parent verified 165 distinct successful target-qualified identities and
all nine input hashes. Logs in
`/tmp/rustycore-native-recovery-publication-logs` preserve the initial test
compilation error, the 53/3 and 53/2 resurrection failures, and the 79/4 healer
result. Repairs declare the request fixture's known-empty inventory, configure
its native powers, and restore the visual/threat fixtures' Player to Alive
before ordinary sickness casts. They do not bypass CheckCast. An invalid new
one-shot out-of-world success test was removed; the actual effect's no-op
case remains. The shared setup stays in the existing 724-line fixture file;
the 1251-line healer parent is unchanged. The legacy corpse test file grows
2004→2012 only for inventory setup and failure diagnostics; its bounded ceiling
is 2012 through #43 acceptance, with a split review before further growth.

`changed-inputs.json` has digest
`3f97ba8c5fa5e6a7b346b58fddfa17c5837c565584e0c50e1a939eae54dabc29`;
the tracked Rust diff digest is
`6688b08d2ae13ddbdcca8e0fcc40b3a59f285b8664c3213c78163f7640e14e45`.
These identify tested working inputs at the stated base, not a test of the
later commit. Runtime declares GPT-6; specific model/effort telemetry is
unavailable. The 03:13:11–03:34:26 feedback/repair interval is 1275 seconds,
with implementation before it; this is not the completed-delivery campaign
or evidence for its ordinary 600-second performance target. This remains an
UNVALIDATED macro checkpoint. Registered SelfRes composition, native death,
QA, final acceptance and authorized live/capture/durability evidence remain
open. No push, installation, SQL or restart was performed.

Cast dispatch and grouping availability, 2026-10-27, base
`7583d94c8034dfa8055d4f88519cc5f10085a27e` plus 13 Rust working inputs:
one private Spell classifier identifies the actual direct, post-GO, Aura and
Source-inert routes. Unknown effects and unsupported generic Player Aura
groups now report unavailable. The normal prepare path checks this after
ordinary busy/death/cooldown/precondition/cost rejection and before native
cooldown/START publication or active-cast installation. This includes timed
casts, which previously could return Started before their skipped effects
became visible. Execution also checks availability before power debit and
GO, and retains explicit drift guards at direct and special-Aura fallthroughs.
The legacy primary fallback no longer logs unknown work and returns success.
Actual post-GO handlers and selected sickness/Ghost/Wisp constructors remain
recognized by the shared route facts. No additional task or mutable owner
was introduced.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors are the
SpellEffects.cpp:88–403 dispatch table, :406–414 NULL/Unused handlers,
:1389–1396 CreateItem and its DoCreateItem operation. Type 24 is a real
mutation, not a Source-inert effect; this change does not implement it.
The positive carried-item case uses the Source-disabled AddComboPoints body
(:3164–3181), whose mutation is commented out in this target. DUMMY remains
script-driven and is not treated as an unconditional no-op. Existing
ordinary gameplay rejection stays distinct from unavailable implementation.

On the aarch64 host, sequential one-job coding feedback used the same PROTOC
and checkout-local target environment above. `cargo check -p wow-world`
passed at 06:36:48.077–06:37:17.043 UTC (28.966 seconds). The final
`cargo test -p wow-world-spell execution_support --lib` passed three cases,
including real AuraType constants and a native Player GUID. The affected
`player_cast::lifecycle::tests` filter passed 18, including instant and timed
unimplemented-route rejection before installation and publication. World's
`registered_self_res` filter covers ten distinct cases: its seven existing
cases passed unchanged; the three new OnPickup cases passed after fixture
repair in `registered_self_res::onpickup --lib`. The selected sickness and
Ghost/Wisp filters each passed one. Parent review recomputed 33 distinct
passing identities from the logs; reruns are not added to that count.

The real registered SelfRes case has exact difficulty-three outer and inner
spell keys, a physical OnPickup item with loaded level 50, separate cooldown
categories and a valid nonzero CreateItem output item. An unsupported timed
obtain cast retains native Alive/zero-health recovery at ObtainSpellsInFlight,
without inner START/GO, corpse deletion or save. A separate fresh Alive/zero
fixture calls the real obtain pipeline and checks the specific unavailable
effect reason. A Source-inert obtain cast instead completes native revival,
bones and one full-save request. These are controlled persistence-port cases,
not real database/restart evidence.

Logs and machine-readable argv/UTC/monotonic-time/exit records are under
`/tmp/rustycore-cast-execution-availability-logs`. The first World run and
first OnPickup rerun exited 101. Repairs replaced the legacy HP-sensitive
alive facade with native Unit-state assertions, read all four SpellGo GUIDs,
and distinguished the outer SelfRes START from the forbidden inner START.
The final OnPickup run passed three at 06:45:56.560–06:47:19.649 UTC
(83.089 seconds); only the changed Spell test leaf was then rerun, passing
three at 06:47:26.390–06:47:32.327 (5.938 seconds). No production input
changed after the green check. This coding and fixture-repair interval is
separate from the unstarted final campaign and its 600-second ordinary target.

Parent review matched all 13 Rust hashes to `coding-feedback-inputs.json`,
digest `e5182b811fe2d084058affe5bc6f1ffc71c21523833eef1ddd9fd0475673f39b`.
The tracked Rust diff digest is
`2816108af4ac815d18d3fc3a815412e6da95c03cb994a1b8c234b0022034861f`;
the input record includes all three new leaves. The production classifier
is 333 lines and the new World test leaf 589; runtime turn-context telemetry
confirms Luna max, including the same open worker's feedback turn. This is
an UNVALIDATED macro checkpoint at the working base and inputs, not evidence
for its later commit. It proves bounded dispatch/grouping availability and
error propagation, not full parity of known effect handlers, all Aura
execution, scripting or new gameplay-effect support. Native death, remaining
SelfRes admission/timing cases, QA and authorized live/capture/DB/relogin
acceptance remain open. No push, installation, SQL or restart was performed.

Normal cast-item lifetime and failure propagation, 2026-10-27, base
`b18321dad32f4915f58f62f5bf7023e86c6200a6` plus six Rust working inputs:
Application's existing TakeCastItem owner now returns updated execution
metadata or an explicit unavailable error. Its Spell host and World adapter
retain that distinction; both executor sites assign the returned metadata
and propagate errors. Consuming an expendable item clears only the logical
cast-item GUID and entry, including when one unit remains in the physical
stack. The cached item level and all other metadata remain unchanged.
No-item, ignored-item and non-expendable use preserve the logical pointer.
This corrects the upstream provenance gap recorded by the preceding native
drunkenness checkpoint without adding per-effect item-existence guesses.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` Source anchors are
Spell.cpp:3842–3854,4025,5310–5380. GO precedes TakeCastItem; delayed
nonchannel spells take it before effects and immediate spells afterward.
The Source clears its logical item pointer after charge exhaustion even when
DestroyItemCount leaves a surviving stack. TakeReagents' earlier clear was
already represented and was preserved. These tests establish execution-phase
ordering, not an implementation of projectile travel timing.

One-job aarch64 feedback used the same PROTOC and checkout-local target
environment as above. `cargo check -p wow-world` passed at
05:35:38.763–05:36:23.426 UTC, instrumented 44.662 seconds. Four existing
registered SelfRes cases and one new real-Application owner case passed in
the focused `registered_self_res --lib` runs. The new owner case checks a
surviving stack, unchanged cached metadata, ordinary no-item/ignore and
non-expendable paths, and missing physical item/entry errors.

The two executor cases initially exposed incomplete fixture item catalogs,
then missing prepared residence stamps. An interim seven-case pass after
removing GO assertions is superseded for those two cases and earns no final
credit for them. Parent review required actual canonical cast IDs and
residence stamps and restored GO assertions. The final
`cargo test -p wow-world normal_cast_execution --lib` passed both corrected
cases at 05:43:51.983–05:45:08.026 UTC, instrumented 76.043 seconds.
They execute through the real World host with normal-cast metadata, distinct
physical reagents and cast items, selected difficulty-three hit-delay data
and Realm threshold packets. They are executor cases, not two additional
registered-opcode cases. Combined evidence covers seven distinct cases
across runs; only the two changed cases were rerun after that last fixture
repair, and no production inputs changed after the check.

Per-command argv, UTC bounds, monotonic durations, exits and retained failure
logs are in `/tmp/rustycore-cast-item-lifetime-logs`. Parent review verified
all six final Rust file hashes. The tracked Rust diff digest is
`fd66468b537857db91ef81fac01d7b80ef92e2a1b7cff5d8b71ed2e7184e895b`;
the new test leaf digest is
`84d5b3bb1e3f058354fa0387ed1dfcad415d458df5d5e868805fffa5a45586bc`.
The shared test fixture is 596 lines and its new private leaf is 469 lines.
Runtime turn-context telemetry confirms the fresh worker used Luna max.
This remains an UNVALIDATED macro checkpoint at the stated working base and
inputs, not evidence for its later commit. Implementation and repair time
are separate from the unstarted final acceptance campaign and its ordinary
600-second target. Unknown mutating-effect execution, native death, remaining
SelfRes admission/timing cases, QA and authorized runtime/capture/DB/relogin
evidence remain open. No push, installation, SQL or restart was performed.

Native SetDrunkValue and Player field publication, 2026-10-27, base
`3f354d8f02999d03c0bd35b61a6b9ffb80c36e46` plus 21 Rust working inputs:
the private Entity participant reuses PlayerData::Inebriation, applies the
Source detection flags/value and sobering-counter reset, and requests native
in-world visibility notification. Application reads actual indexed fake-Aura
amounts and mutates the same Player under one MapManager guard. The exact
integer query preserves first-rank groups, absolute-magnitude selection,
newest-registration-first ties and unavailable/known-empty distinctions.
An unavailable amount or undefined integer arithmetic stops before mutation.
The existing EffectInebriate host now invokes this participant even when its
stored value is unchanged. Threshold packets use the existing Realm connection,
self first and then frozen visible observers, outside the Map guard. Existing
Instance AuraUpdate delivery retains its durable command rail.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors:
Player.cpp:863–910; Unit.cpp:3567–3579,4818–4843,4904;
SpellMgr.cpp:400–432; SpellEffects.cpp:3464–3482;
MiscPackets.cpp:686–693, MiscPackets.h:846–858; and
Opcodes.cpp:1328 (`CONNECTION_TYPE_REALM`). The byte vector is derived from
these sources; it is not an action capture. ItemID retains all uint32 bits.

The real Map publication test exposed an additional defect: Player's five
canonical dirty helpers marked their masks without queueing the native Object.
Map::SendObjectUpdates only collects queued objects, so an isolated PlayerData
or ActivePlayerData change could remain unpublished. Those helpers now use
the same Object queue primitive as Unit. Source Object.h:270–276,295–323 and
Object.cpp:791–806 establish the changed-value/dynamic-field queue operation,
its in-world guard and mask clearing. No new publisher, state mirror or scan
was added. The World case now verifies the final Inebriation field and mask,
one Map-owned snapshot, one consumer send and no repeated send.

On the aarch64 host, one-job coding feedback used
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc` and
`CARGO_TARGET_DIR=/home/server/rustycore/target`:
`cargo check -p wow-entities` passed; the final
`cargo test -p wow-entities player::inebriation::tests` passed six cases;
`cargo test -p wow-world-core fake_inebriation` passed six;
`cargo test -p wow-world-core selected_aura` passed six publication cases
and one unrelated duration regression; the packet threshold filter passed
one; and `cargo test -p wow-world scenarios_spell_state_12::inebriate`
passed seven. Its two integration targets matched zero cases and receive no
credit. The earlier World production check preceded the dirty-helper repair;
the final World test compiled that final production path. Logs retain the
initial shadowed helper, missing canonical metadata, wrong GUID integer type
and missing Map-queue failures, followed by their affected passing runs.
Parent review then found that the nonzero-key case checked only catalog
existence. Its corrected body now constructs a difficulty-three Aura, resolves
its actual indexed 304 effect and reads amount 17 despite base points 99,
with neither a zero key nor a legacy payload. Only
`cargo test -p wow-world-core genuine_nonzero_difficulty_key --lib` was rerun
for that changed case: one passed, exit 0, at
05:20:24.997–05:20:45.267 UTC (instrumented 20.269 seconds). The other five
Core cases and all other Rust inputs were unchanged.

Evidence is in `/tmp/rustycore-native-drunk-transition-logs`. The final World
log interval is 05:11:28.933–05:13:41.558 UTC; the final Entity test log interval
is 05:15:25.899–05:15:32.003 UTC. These are log birth/last-write observations,
not independently instrumented command start/end times. Parent review matched
all 21 modified/new Rust paths to `changed-rust-sha256.txt`, digest
`7d66370da2ac1db9a8f1430d3b0787612ba4a264a3051faa995eeb5b5fb732ba`.
The tracked Rust diff digest is
`8ba06f9ea2594331ae2ae08da2cc20511548185872f319fe09d8d7c08240bc27`;
untracked leaves are covered by the per-path manifest. The legacy Player root
grows 4753→4777 for one native field/mount/default and the five queue calls.
This reviewed bounded ceiling is 4777 through #43 acceptance; review further
growth before editing, and retain the existing distribution retirement owner.
No physical-policy baseline was regenerated. Runtime turn-context telemetry
confirms Luna max for the implementation and main feedback. Reopening that
worker for the last test correction unexpectedly selected Sol high; its
05:19:17 turn context records that route change. The parent disclosed it and
closed the worker; future units use a fresh assigned-role worker.

This remains an UNVALIDATED macro checkpoint at the working base and inputs,
not evidence for the later commit. Drunken Vomit, foreign-player effect
transport and the sobering tick executor are not implemented here. Native
death invocation remains open. A separate upstream normal-cast gap also
remains: Source Spell.cpp:3842–3854,5310–5380 clears m_CastItem/GUID/entry when
an expendable item exhausts its charge, even if its stack survives. The Rust
TakeCastItem adapter returns an ignored boolean and does not update execution
metadata after pre-effect consumption. This checkpoint therefore does not
prove ItemID provenance for that path. Correct it in the existing cast-item
owner; do not reinterpret physical-item existence as Source pointer lifetime.
The approximately 04:49–05:20 coding-feedback period and earlier implementation are separate
from the unstarted final campaign and its 600-second ordinary-run target.
No push, installation, SQL, restart, capture or live acceptance was performed.

Canonical native-death operation ownership, 2026-10-27, base
`04486d4e4836694101e34223496d946a02bfc9aa` plus four Rust working inputs:
the Player now privately retains one operation context, a checked local ID
and Ready/InFlight/Complete state. Begin and claim read native IsAlive and
full-width nonzero health. Duplicate or wrong-ID calls cannot replace or
replay owned work; completion requires a claim, and retirement requires exact
completion. ID exhaustion cannot wrap. IDs must be bound to the actual Player
handle in the future transport; this owner does not establish that binding.
The existing resurrection continuation, SelfRes list and vitals are preserved.
These Rust phases preserve a future asynchronous operation; they are not
claimed to exist in synchronous C++ Unit::Kill. Source anchors remain
Unit.cpp:10457–10652,8527–8585 and Player.cpp:1158–1200 at the pinned SHA.

On the aarch64 host, sequential `cargo check -p wow-entities`
(04:07:49–04:07:54 UTC) and `cargo test -p wow-entities death_operation`
(04:07:57–04:08:09 UTC) passed, with ten distinct owner cases. They used the
same PROTOC, one-job and checkout-local target environment above. Logs in
`/tmp/rustycore-native-death-owner-logs` retain the test-only DeathState import
failure before repair. Parent review verified all eight final recorded input
hashes, including the four changed Rust files, against the final block of
`inputs.log`, whose digest is
`7580966b074103e21a3bc79a4d6bb00b09bbea49fd3249287eb77f36d410ac89`.
The private production/test leaves are 191/240 lines. The legacy Player root
grows only 4746→4753 for its mount, exports, field and default; its bounded
ceiling is 4753 through #43 acceptance, with a split review before further
growth. No policy baseline was regenerated.

This is an UNVALIDATED owner-only checkpoint at the stated working base, not
a test of the later commit or of real async cancellation. Application death
execution, required Session phase delivery, retained Map cursor, finalization
fences and lethal callers remain unconnected. No new death behavior is
enabled. Runtime declares GPT-6; specific model/effort telemetry is unavailable.
The 04:06:06–04:08:09 feedback interval is 123 seconds, with implementation
before it; it is not the final campaign or its ordinary performance result.

Native-death callback reentrance review at `7583d94c8`, 2026-10-27:
Player::setDeathState runs NoDeath quest callbacks before its final
Unit::setDeathState call (Player.cpp:1187–1198). SendQuestUpdate can issue
FULL casts from effective SpellArea rows. A Source-valid nested InstaKill
can therefore still see an alive, positive-health Player: its guard checks
IsAlive and then invokes Unit::Kill (SpellEffects.cpp:444–465), whose entry
only rejects zero health (Unit.cpp:10457–10461). A blanket refusal of every
InFlight death invocation would lose this synchronous Source call chain.
The required Application body must distinguish duplicate external admission
from a nested call made by its own awaited callback chain. Keep one claimed
canonical root operation and its existing claim/revoke fence; execute nested
Source frames inside that body, with their own call arguments and Source
order, without replacing the root context, claiming another root or starting
another task. Verify the exact same-Session/Player-handle ownership proof
and cancellation fence when implementing this route. This does not authorize
replaying an interrupted InFlight operation. No nested-death executor or
regression evidence is implemented by this read-only review.

Registered SelfRes normal-cast composition, 2026-10-27, base
`b11b70973930b3ce2ddbd0ec3a8f73a838b7b732` plus two Rust test inputs:
four cases dispatch CMSG_SELF_RES through its actual registration and thunk.
The synthetic spell 61043 exists only at actual Map difficulty 3: neither a
difficulty-zero existence key nor a legacy payload is installed. Its 20-percent
effect and ten-second cooldown are fixture data, not evidence for the real
21169 catalog. Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors are
SpellHandler.cpp:416–429, SpellEffects.cpp:3757–3791,
SpellInfo.cpp:495–525 and Player.cpp:4509–4515; the normal cast-cost source
contract is retained from the earlier pre-GO checkpoint.

The positive case consumes the physical reagent before native revival and
captures its Removed row, restored health/mana and active cooldown in the same
full-save request. A controlled save ACK retains the native item tombstone
until acknowledgement, then retires it and completes the same resurrection
operation. Missing reagent and active cooldown produce Reagents and NotReady
respectively, without GO/effects/save, and consume the listed request. Without
a corpse, revival succeeds and costs/history remain dirty without an immediate
save. This is controlled-port evidence, not real database durability.

`cargo test -p wow-world registered_self_res --lib`, started at 03:57:32 UTC,
passed all four distinct cases with exit 0, using the same PROTOC, one-job and
checkout-local target environment above. Its exact end UTC was not recorded.
The earlier `cargo check -p wow-world` at 03:47:13 passed before final test
edits; it does not compile cfg(test) and is not credited for those edits.
Logs in `/tmp/rustycore-registered-selfres-pipeline-logs` retain fixture compile
errors, the confirmed recursive-lock run stopped only through its own session
(exit 130), and the initial 0/4 result. Parent review removed a legacy payload
seed, fixed the fixture lock lifetime and retained its packet peer. Known-empty
visual/serverside catalogs and explicit DB2 base points 20 then exercise the
actual path; no production guard or arithmetic was weakened.

Both inputs match `final-input-sha256.txt`, digest
`ccd9b997501a13937c444f4db501bda1cf91e9404913b1e504bd665b2fcf8263`.
The new 574-line leaf has digest
`a0a1ee27995569ad5fbea6ca2be2942b24c2889bac739eda513a9c28e56e197c`;
its parent grows only 606→609 for the private mount. These identify tested
working inputs, not the later commit. Runtime declares GPT-6; specific
model/effort telemetry is unavailable. This remains an UNVALIDATED macro
checkpoint, not the final campaign or its 600-second performance result.
Silent admission, duplicate/timed/nested cast cases, native death, integrated
QA and final/live acceptance remain open. No production code, runtime,
database or publication changed in this checkpoint.

Native SelfRes effect and recovery ownership, 2026-10-27, base
`1e8a06cc21da8dd31c27b5520f7bfe66fb8edb33` plus 17 Rust working inputs:
the effect's direct revival is replaced by the existing Player-owned recovery
operation. It verifies the actual caster and native IsInWorld, freezes health
and mana before resurrection, then performs native revival, visibility and
obtain spells before applying the current native setters, including Focus=0.
Corpse conversion and its conditional full save use the existing delete/ACK
fences. No corpse means no immediate Player save. Unavailable work disconnects
and retains the exact operation; the timed cast driver suppresses its synthetic
NotKnown error after that fail-stop.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors are
SpellEffects.cpp:3757–3791, Unit.h:766, Util.h:72–75, Player.h:2117 and
Player.cpp:4320–4403,4509–4515. Percentage calculation uses Source f32 and
full u64 health before u32 narrowing; flat mana preserves the signed bit
conversion. Undefined Source integer-negation/float-cast inputs are reported
as unavailable before revival, rather than assigned invented saturation.
The registry's permissive in-world fallback is not the caster's native
IsInWorld. Alive with zero health during ResurrectPlayer(0.0) is also distinct
from the legacy convenience vitals predicate; the failure test reads Unit state.

Sequential coding feedback used PROTOC=/home/ubuntu/.local/protoc/bin/protoc,
CARGO_BUILD_JOBS=1 and CARGO_TARGET_DIR=/home/server/rustycore/target:

| Cargo command | UTC interval | Result |
| --- | --- | --- |
| `check -p wow-world-spell` | 02:01:54–02:01:55 | exit 0 |
| `test -p wow-world-spell self_resurrection --lib` | 02:02:03–02:02:11 | 37 passed |
| `check -p wow-world` | 02:02:19–02:02:31 | exit 0 |
| `test -p wow-world self_resurrect --lib` | 02:10:03–02:11:37 | 11 passed |

The parent verified 48 distinct target-qualified identities. The World cases
include actual effect execution, a real map corpse and captured Player-save
DTO, unavailable-owner/native-phase failures, and cancellation at a controlled
corpse-delete persistence future. After cancellation, admitted recovery retains
the same Player at RecoveryCorpseBonesInFlight with one delete and no save.
This is not cancellation during obtain or real database durability evidence.
Logs in `/tmp/rustycore-native-selfres-effect-logs` preserve the initial four
fixture compilation errors and the 3/6 failed World run, then 9, 10 and 11
passing runs. The earlier Entities check passed, but its original exact UTC
times are unavailable; it was not rerun to manufacture metadata. File names
containing final do not establish a validation-v2 final campaign. The
01:31:16–02:11:37 implementation/feedback interval is 2421 seconds, not
the ordinary completed-delivery campaign or its 600-second performance target.

The parent reviewed all 17 changed Rust inputs and verified
`parent-reviewed-inputs.sha256`, digest
`2a36ce635f9ebce3678a6418f1f1235e5bb7dfc285e2a61c420723e619a655b2`;
the tracked Rust diff digest is
`f8616c5c734d5c806aaf71fb499bdd563c858b8155debdb28d1686ff2533e74a`.
These identify working inputs at the stated base; parent-owned State text
was also dirty. They do not relabel that run as testing a later commit.
The three original spell_self_resurrect_* tests moved under the private
scenarios_spell_state_13::self_resurrection module. The parent/child files
are now 709/533 lines, and the recovery fixture is 606 lines. Effective worker
model/effort telemetry is unavailable.

This remains an UNVALIDATED local macro checkpoint. Packet vectors establish
the tested Rust participant order, not Source enqueue cadence or byte parity.
Registered CMSG_SELF_RES, native CheckCast, production death participants,
generic obtain execution boundaries, QA, final and authorized live evidence
remain required. The separate death-transport decision above is not installed.

Sickness original-key full-save capture, 2026-10-27, base
`54fb1f2e13edf4886656b6b964b07b6134fa220b` plus five Rust working inputs:
Core no longer requires a race-wide difficulty-zero payload, attributes or
effects before inspecting actual sickness applications. Each active Aura's
SpellInfo key supplies its component reads; the saved difficulty remains the
original cast difficulty. Complete empty families remain capturable, while
unavailable source metadata for an active Aura holds that save group.
Existing passive, channel, foreign single-target and effect-mask policies
remain in place. No payload-only fallback substitutes for exact key authority.

Pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` Player.cpp:18085–18130,
19811–19866 and SpellAuras.cpp:463–465 distinguish the actual SpellInfo from
the original cast difficulty and persist the active Aura's native metadata.
The new loader-backed fixture has only a real difficulty-3 key, explicitly
no key or payload at zero. Typed load, full Player-save DTO capture and reload
retain item GUID/entry/signed level, durations, charges and effect/base amounts.
Existing fixtures now seed explicit source keys. The missing-attributes
negative contains an active Aura; empty authoritative state is not an error.

Sequential coding feedback used PROTOC=/home/ubuntu/.local/protoc/bin/protoc,
CARGO_BUILD_JOBS=1 and CARGO_TARGET_DIR=/home/server/rustycore/target:
`cargo check -p wow-world-core` exited 0 at 01:26:29–01:26:31 UTC;
`cargo test -p wow-world sickness_loading` exited 0 at 01:26:36–01:28:56,
with 35 distinct library cases, including all three new cases. Zero-test
targets are not credited. Logs in
`/tmp/rustycore-sickness-original-save-key-logs` preserve three failed test
iterations: private fixture helpers, incomplete key fixtures (21/14), and
a fixture borrow/move error. The 01:17:32–01:28:56 feedback/repair interval
is 684 seconds of coding feedback, not a final campaign or its 600-second
performance target.

The parent reviewed the complete diff and verified all five changed-input
hashes. `changed-inputs.sha256` digest is
`476927c7a4f8438dae1f5e2c1d011187cb455de8c80d25c4482f2f798559844b`;
the candidate diff digest is
`3aa72813c6c5a8744f7ec93548d0dedc743d24f3cbcf5c70021bececa0af7d76`.
These identify working inputs at the stated base, not testing at a later
commit. Effective worker model/effort telemetry is unavailable. This is an
UNVALIDATED local macro checkpoint. Complete SelfRes, native death, QA,
final acceptance and authorized live evidence remain open.

Obtain ordering and recovery fixture contract, 2026-10-27, base
`ad78f38716f850e77fa0c2be24386f551d3feb34` plus 14 working Rust inputs:
the physical item's entry is now read without calculating level. LEGACY,
known empty pickup lists and already-present auras do not demand cast-level
facts; those are read only immediately before an actual normal cast.
Player.cpp:8054–8079 and Spell.cpp:3416–3421 at pinned
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` establish that order. A real
World test deliberately supplies no item-level rows for no-pickup/LEGACY
items. Missing catalogs still hold when the operation actually needs them.

Recovery fixtures now provide known canonical inventory topology and empty
currency loads through existing load-complete APIs. Top-level and bag-child
rows populate the actual inventory positions. Deliberately incomplete inputs
remain separate. The dungeon case verifies active residence before/after its
accepted ACK. The healer test verifies captured item identities, entries,
slots, pre-ACK Changed/New states and durability 0/75/75, then acknowledges
their Unchanged states; untouched bank rows remain 100 and are not dirty save
rows. Item.cpp:383–579 ends SaveToDB with ITEM_UNCHANGED. No production
persistence or residence fence was weakened. Boxing the large test-owned
WorldSession in the refresh fixture removes the reproduced isolated stack
overflow without a global RUST_MIN_STACK override or runtime ownership change.

Sequential feedback used PROTOC, one Cargo job and the checkout target:

| Cargo command | UTC interval | Result |
| --- | --- | --- |
| `check -p wow-world` | 00:58:29–00:58:43 | exit 0 |
| `test -p wow-world spirit_healer` | 01:03:28–01:05:03 | 83 passed |
| `test -p wow-world recovery_fences` | 01:06:14–01:07:42 | 33 passed |
| `test -p wow-world resurrection` | 01:07:47 | 26 library + 2 integration passed |
| `test -p wow-world corpse_login` | 01:07:51 | 13 passed |
| `test -p wow-world repop` | 01:07:54–01:07:55 | 18 passed |

The parent deduplicated 148 target-qualified identities, including the two
production-linked disconnect/near-resurrection integration cases. Zero-test
targets are not credited. Logs in
`/tmp/rustycore-recovery-fixture-contract-logs` preserve the initial isolated
stack abort, both failed healer fixture attempts, the 82/1 healer result and
the 31/2 recovery result. The initial pandemic log's stale b545 header remains
with an explicit correction; its actual run followed the ad78 commit, whose
Rust inputs match the preceding dirty origin candidate. New receipts use the
actual HEAD. File names containing final do not mean validation-v2 final.
The 00:38:22–01:07:55 diagnostic/repair interval is 1773 seconds, not a final
campaign or evidence of the ordinary 600-second target.

All 14 changed inputs match `changed-input-sha256.txt`, digest
`fba60ae6afea6ee6113082203267d1e8b2144e6dd607dc33b5027de08770dac9`;
the tracked diff digest is
`62a00ac2a4abb76c299531792c3714d18dcfdf2aab6013ce5c66d18a008fc161`.
These describe working inputs at ad78, not testing at a later commit. The
worker reported GPT-6; specific model/effort remain unavailable. This remains
an UNVALIDATED local macro checkpoint. The original-key sickness save gate,
complete SelfRes, native death, QA, final and authorized live evidence remain.

Selected sickness cast-item origin, 2026-10-27, base
`b545669132cb191ca5abaecf3dcaaa52dfb729df` plus 14 Rust working inputs:
the existing SpellCastMetadata now reaches the sickness constructor. Fresh
AuraPersistenceMetadata retains physical GUID, entry and level, including
explicit retained level with an empty GUID. The existing ephemeral
SelectedAuraRefresh carries those inputs; its canonical operation updates
all three only when GUID changes. Same-GUID refresh retains old ID/level;
ordinary no-item refresh clears them after a physical GUID. Ghost/Wisp
callers preserve their empty defaults. No second canonical item owner or
post-cast stamping pass was introduced.

Source anchors at pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Object.cpp:2898–2940; Spell.cpp:3416–3421; SpellAuras.cpp:464–465,1218;
Unit.cpp:3265–3272; Player.cpp:19811–19866. The parent reviewed the actual
constructor, refresh and direct signature consumers. All 14 input hashes
match `exact-changed-input-sha256.txt` under
`/tmp/rustycore-aura-cast-item-origin-logs`, manifest digest
`d542d5a1e85ff4aeb32efa0d0495b0b5bef52f9f2f5c0b6803155cdfe14cd402`.
The tracked diff digest is
`fb0da73daf9517e453633014e698c00944ed5967665be602fd068902dda707c8`.

Sequential feedback used the same PROTOC, one Cargo job and checkout target
as the preceding checkpoint. `cargo check -p wow-world` exited 0 at
00:23:27–00:23:51 UTC. The restored real reclaim/obtain regression passed
at 00:24:01–00:26:26; `cargo test -p wow-world sickness_cast_item_origin`
passed two cases at 00:26:30–00:26:31. They exercise constructor and refresh
with same/new/empty GUIDs and retained level. Zero-test targets are not proof.

The broader current candidate is red: `spirit_healer` exited 101 after
44 passes and 15 reported failures, then a stack overflow in the pandemic
refresh test aborted its process (00:26:37–00:26:38). `recovery_fences`
passed 15/failed 18; `resurrection` passed 19/failed 7; `corpse_login` passed
11/failed 2; `repop` passed 16/failed 2 (00:27:12–00:27:29). These overlapping
counts are not a completion total. Their current causes require grouped
review: many stop at the new obtain phase with incomplete inventory fixtures,
and the remaining save/residence and stack failures are not yet classified.
Baseline status was not rerun or assumed. Full stdout/stderr, UTC and exit
receipts remain in the named log directory. The 242-second interval is
coding feedback, not final acceptance or proof of its performance target.
The worker reported the GPT-6 family; specific model/effort are unavailable.

This remains an UNVALIDATED local checkpoint. Fix the affected regression
inputs and any demonstrated production findings, then rerun affected evidence.
The sibling sickness original-key save gate, complete SelfRes, native death,
QA, final acceptance and authorized live evidence remain open.

Live common resurrection obtain integration, 2026-10-26/27, base
`d63c12ce0eb8e6136225f6a43c4e9eba06ce831d` plus 20 working Rust inputs:
backpack end is read once; each equipped bag is resolved when reached, and
each contained position, HasAura and physical item's current entry/level is
read at its Source phase. Player incarnation and residence are checked
between effects and across awaits. Canonical reclaim, healer, login and
request continuations retain ObtainSpellsInFlight on error without replay or
dependent bones/save. The public one-shot resurrection wrapper currently has
only test/helper callers and is not cancellation-safe production evidence.

Normal preparation distinguishes known absent SpellInfo from unavailable
catalogs/ownership. Ordinary rejection continues the obtain scan; unavailable
preparation or pre-GO authority holds it. Pre-GO failure keeps its specific
frames and propagates the error without another generic NotKnown frame.
Anchors at pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
Player.cpp:4320–4403,8034–8079; Object.cpp:2898–2940; Spell.cpp:3416–3421.

Worker feedback used `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`,
`CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=/home/server/rustycore/target`.
`cargo check -p wow-entities`, `-p wow-world-spell`, `-p wow-world-entities`
and `-p wow-world` exited 0. Focused `cargo test -p wow-entities cast_all_obtain`
passed 1 case; `cargo test -p wow-world-spell normal_prepare` passed 4;
`cargo test -p wow-world-spell normal_pre_go` passed 1. Exact commands, base,
UTC and exits remain in `/tmp/rustycore-common-resurrection-obtain-logs`.
The first check started at 2026-10-26 23:40:08 UTC. Root test compilation first
failed; obtain repairs 1–4 retained 4,3,3,3 failing cases respectively. The
fixtures were corrected for canonical topology, hydrated exact-key payloads,
native history and complete authority, rather than weakening admission.

Repair5 of `cargo test -p wow-world obtain` at 2026-10-27 00:03:59–00:05:25
exited 101: 5 passed and the real Aura item-GUID assertion failed (EMPTY
instead of the physical item). Repairs6/7 exited 0 with 6/7 cases while that
assertion was absent. The parent required its restoration, including GUID,
entry and level checks. The current candidate was not rerun and those green
runs do not validate it. Repair7 includes one unrelated currency-objective
case, which is not obtain/resurrection coverage. The grouped recovery,
resurrection, corpse-login and repop regressions were deferred until the
constructor fix. Literal cancellation after an obtain effect was not
exercised; the real partial-authority/no-replay case is a different scenario.
Some `.log` files contain receipt headers only; matching `.output` files
retain Cargo diagnostics and test identities. All failed runs remain intact.
The feedback/repair interval ended at 2026-10-27 00:11:12 UTC (1864 seconds),
not a final campaign or evidence for its 600-second ordinary target.

The parent verified all 20 changed Rust inputs; the snapshot
`parent-owned-inputs.sha256`, digest
`c39fe55bd3547776cef15d3a7fab932fbf016e73aa0484340dd065acfba36c6a`,
identifies the restored current candidate,
not a candidate proven green. Effective model/effort telemetry is unavailable.
This is an UNVALIDATED intermediate checkpoint. The next required correction
must thread cast-item input through Aura construction and refresh into the
existing AuraPersistenceMetadata authority (SpellAuras.cpp:464–465;
Unit.cpp:3265–3272). Refresh changes item ID/level only when GUID changes;
adding duplicate canonical item fields or stamping matching auras after the
cast is not selected. Common obtain acceptance, sickness's original-key save
gate, SelfRes, native death, QA and final/live acceptance remain open.

Normal Player cast pre-GO participants, 2026-10-26, base
`4ef33343d954860996c0f86751bbde3384db0bf8` plus working inputs:
Application owns resource/cooldown choreography before GO; Spell retains pure
policy and Entities retains canonical inventory, currency and SpellHistory.
Physical-item charges are consumed at the Source hit-delay/channel phase,
with live facts read again there. Consumed New and persisted items remain
Removed until the existing save/delete acknowledgement permits retirement.
The same full-save DTO and transaction carry costs, currencies and charges.
Native cooldown timestamps use the persisted Unix clock domain. Default
StartCooldown publication is distinct from SendCooldownEvent. Cast-item GUID
and actor CasterUnit remain distinct, and normal OriginalCastId is empty.

NoReagentCostMask has one native owner, owner update projection and aura-256
apply/remove/load consumers. Item destruction removes obtain spell families
and the first OnLooted spell for that physical item. ItemEffect iteration and
first-match cooldown lookup follow ascending DB2 record ID, not legacy slot
or HashMap order. Normal and selected FULL trigger contracts remain separate.

Source anchors at pinned `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Spell.cpp:3819–3842,5310–5380,5504–5556,7306–7376,8857;
SpellHistory.cpp:279,504–525,529,670–689 and SpellHistory.h:49;
Player.cpp:8503–8520,11712,24706–24724;
SpellAuraEffects.cpp:4628–4643; SpellDefines.h:479;
DB2Stores.cpp:1193–1194, DB2Store.h:86, DBStorageIterator.h:49–58 and
ObjectMgr.cpp:3331–3333. SpellInfo.h:389–390 and SpellInfo.cpp:1144–1195,1726
establish known absent Misc defaults, separately from unavailable catalogs.
Production owners include Application `player_spell_costs`, Spell
`player_cast/normal_costs` and execution hooks, Entities item/save owners,
Persistence DTOs, Database save plans and Server acquisition composition.

Sequential worker coding feedback used `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`,
`CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=/home/server/rustycore/target`:

| Cargo command | Log prefix | UTC interval | Result |
| --- | --- | --- | --- |
| `check -p wow-entities --lib` | 02 | 22:30:08–22:30:15 | exit 0 |
| `test -p wow-entities normal_` | 04 | 22:30:55–22:31:16 | 6 relevant + 1 unrelated, exit 0 |
| `test -p wow-entities consumed_new_item_survives_inflight_insert_until_delete_acknowledges` | 05 | 22:31:21–22:31:22 | 1 passed |
| `test -p wow-data category_tests` | 10 | 22:33:51–22:34:47 | 11 passed |
| `test -p wow-data payload_tests` | 11 | 22:35:20–22:35:21 | 6 passed |
| `check -p wow-data --lib` | 27 | 22:57:42–22:57:46 | exit 0 |
| `test -p wow-data item_effect_views_preserve_record_order_for_first_matching_spell` | 28 | 22:57:50–22:58:35 | 1 passed |
| `check -p wow-world-spell --lib` | 30 | 22:59:25–22:59:27 | exit 0 |
| `test -p wow-world-spell normal_` | 31 | 22:59:31–23:00:20 | 14 passed |
| `test -p wow-world-spell item_spell_removal_selects_only_the_matching_spell_and_physical_item` | 32 | 23:00:28 | 1 passed |
| `check -p wow-world-application --lib` | 33 | 23:00:40–23:00:50 | exit 0 |
| `test -p wow-world-application item_removal_` | 34 | 23:00:53–23:01:13 | 3 passed |
| `check -p wow-world --lib` | 35 | 23:01:24–23:01:27 | exit 0 |
| `test -p wow-world server_normal_` | 36 | 23:01:31–23:05:46 | 2 passed |
| `check -p world-server --bin world-server` | 37 | 23:08:43–23:13:28 | exit 0 |

Logs remain in `/tmp/rustycore-normal-resurrection-casts-logs`. The parent
verified 45 distinct relevant identities: Entities 7, Data 18, Spell 15,
Application 3, World 2. Zero-test targets and the unrelated creature case are
not credited. Failed iterations 01,03,06,08,09,12,14,15,16,18,20,21,23,24,26,29
retain exit 101 and their actual failures; repeated passes are not added.
The 22:29:16–23:13:28 coding/repair interval is 2652 seconds, not final
acceptance or evidence of the ordinary 600-second campaign target.

The parent verified all 114 changed inputs (102 tracked, 12 new), without
uncovered or unchanged paths. `owned-inputs.sha256` digest is
`e3194ac05f34b2a61e7f4c55a0e2cfb91c7cbbc3f99db922826a8c5771e9d060`;
the tracked binary diff digest is
`c613324452ea12389e920d74af30c24891782f7b6d5420bfabfdce2f496818cf`.
These identify working inputs at the stated base, not testing of a later
commit. Effective model/effort telemetry is unavailable.

This is an UNVALIDATED local macro checkpoint. Full common resurrection
obtain integration, SelfRes execution, native-death participants, the sibling
sickness save gate, QA and final acceptance remain open. No real DB durability,
runtime, fresh capture, publication or issue completion is established.

Native resurrection error consumers, 2026-10-26, base `04583b7d3`:
`resurrect_player_like_cpp` now returns the error already produced by the
native phase. Reclaim returns false before bones on that error, and the
spirit healer returns before durability loss, graveyard lookup, bones or
teleport. Login corpse restoration returns an error before clearing
AT_LOGIN_RESURRECT or restoring ghost auras; the actual Character login
sequence invokes its existing partial-login abort and returns false.
Missing canonical ownership is also reported by corpse restoration.
The successful paths retain their previous order. Ordinary sickness and
login ghost-cast failure remain ignored as in the source; no rollback of an
already completed Alive transition is introduced. Login preserves the
source's earlier bones-before-revive phase even when native revival fails.
Request resurrection retains its existing nonreplayable native continuation.

Source anchors are Player.cpp:4320–4403,18157–18180,
NPCHandler.cpp:347–370, MiscHandler.cpp:416–445 and
CharacterHandler.cpp:1227–1243 at pinned
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. These source functions assume
available native authority; this change propagates the port's existing
explicit failure rather than claiming that C++ returns the same Result.
The parent rechecked the affected Player, Unit, NPC and World source files;
those files have no local changes at that pin.

Focused coding feedback on this aarch64 host used
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc`, `CARGO_BUILD_JOBS=1` and
`CARGO_TARGET_DIR=/home/server/rustycore/target`, sequentially:

- `cargo check -p wow-world-character --lib`: exit 0,
  16:34:16–16:34:24 UTC; the final production check also returned 0 at
  16:46:36. This includes the actual Character login consumer.
- `cargo test -p wow-world --lib resurrection`: exit 0,
  17:01:09–17:02:33 UTC, 24 passed and no failures.
- `cargo test -p wow-world --lib corpse_login`: exit 0,
  17:02:48–17:02:49 UTC, seven passed and no failures.

The two filters cover 31 distinct cases, including seven new native-outcome
cases. They exercise failure and success through real canonical Player,
corpse and inventory state, the login flag, and the existing request replay
and save fences. The actual complete Character login failure sequence was
compiled and reviewed, not exercised through a full login fixture or client.

Raw evidence is retained in
`/tmp/rustycore-resurrection-native-outcome-logs`. The first resurrection run
and its rerun exited 101 after bounded SIGTERM of their verified test child;
they were not passing runs. The first also exposed two new fixture mistakes:
an Orc Ghost fixture incorrectly expected Wisp, and the healer's graveyard
precondition resolved the default instead of the intended distinct grave.
Those fixture corrections did not change production behavior. The remaining
existing failed-bones case blocked synchronously before corpse deletion.
The parent's detached GDB trace in `parent-gdb-stack.log` identifies
`flume::Sender::send` from `SessionCore::send_packet`, reached through
`resurrection_request::set_request_values`. Its unconsumed instance test
channel had capacity 16. The test now captures its completed repop/ACK
packet burst before handling resurrection, retaining every original failure,
disconnect, owner-retention, duplicate-response and save-fence assertion.
Production transport and persistence were unchanged. Temporary diagnostic
markers and the diagnostic timeout/early return were removed before the
passing run. GDB and its missing libraries were extracted under `/tmp` for
this test inspection; no system package installation or live server action
was performed.

The parent reviewed the final production diff, callers and cases, and verified
all nine implementation hashes in `sha256-manifest.txt`, manifest SHA-256
`53af6724063da29757f82397c29d63106055090960567b3ed22f8c5e73ac47e0`.
This binds the tested dirty inputs at the stated base; it does not relabel
them as final evidence at a later committed SHA. The configured worker role
was `luna_worker`; reported model telemetry identifies only GPT-6, with effort
and effective role unavailable. No effective Luna claim is made. The 16:34:16–17:02:49
feedback/repair interval is 1713 seconds, not a final acceptance campaign or
proof of its 600-second ordinary performance target. The new test leaf has
384 lines. This remains an UNVALIDATED local macro checkpoint. Unconfirmed
corpse deletion/save, cancellation and transfer continuations, native death,
normal casts, remaining selected metadata, QA and full final/live acceptance
remain open. No runtime, SQL, restart, client capture, publication or merge
was authorized or performed by this unit.

Native SelfRes initializer participant, 2026-10-26, base `740b21234`:
`wow-world-spell::initialize_self_resurrection_spells` borrows the canonical
Player, checks immutable inputs before clear or RNG, and rebuilds the existing
ordered native field. It scans active indexed DUMMY effects, preserves the
Soulstone/Nether else-if and every eligible ten-percent roll, then appends
3026, 23700 and 21169 in source order. Identical roll probabilities and constant
assignments permit counting every Nether roll without another ordering owner.
Zero active masks contribute nothing; unknown aura authority, missing active
rows and non-aura active bits reject before list or dirty-mask mutation.
`HasSpell(20608)` reads the native row, accepts inactive rows and rejects
disabled/Removed rows; incomplete absence remains unknown.

Regular and server keys share exact-before-fallback lookup at each difficulty.
Original Aura difficulty and current-map Reincarnation difficulty stay distinct.
Regular effects/family use the existing Data readers; server effects, category
and full-width family use their canonical payload. The cooldown predicate
checks Aura 383 first, including generic-family and same-family zero-mask
wildcards and all 128 mask bits. A matching ignore aura makes Reincarnation
eligible. Otherwise it reads direct/category membership without expiration
arithmetic; the existing Unit pass owns that maintenance. Unused catalogs do
not become prerequisites for an unrelated candidate.

Data now distinguishes a completely hydrated empty SpellCategories component
from an unhydrated SpellStore. Only the complete typed hydrator establishes
that provenance; an individual hit-metadata insertion does not. The category
index producer preserves high unsigned spell IDs instead of dropping them at
an `i32::try_from` conversion. Exact zero stops fallback and the highest record
ID still wins. The session-free canonical key fixture moved to the existing
dev-only test-support crate, with the old World alias retained. Its existing
test identity moved from `wow-world:selected_spell_fixtures` to
`wow-world-test-support:spell_catalog::selected_keys`, with the same function
name, test configuration and non-ignored state.

The source is target `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Player.cpp:24757–24780,3764–3771; SpellHistory.cpp:670–693,1000–1043;
SpellInfo.cpp:461–492,1770–1782; SpellMgr.cpp:675–693,2850–2880,2938–2939;
SpellAuras.cpp:69–99,163–185 and Unit.cpp:3567–3580,4671–4679.
`CooldownAuraSpellId` defaults to zero (SpellInfo.h:371); the inspected pinned
server source has no assignment to activate that branch. This is a bounded
base-server finding, not a new external-module contract.

Worker coding feedback on this dirty working candidate, aarch64, used
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc`, `CARGO_BUILD_JOBS=1` and
`CARGO_TARGET_DIR=/home/server/rustycore/target`, sequentially. Raw commands,
UTC times and real exits are in `/tmp/rustycore-self-res-initializer-logs`:

- `cargo check -p wow-world-spell --lib`: exit 0, 14:51:15–14:51:18.
- `cargo test -p wow-world-spell --lib self_resurrection`: 32 passed,
  exit 0, 14:53:21–14:53:34.
- `cargo test -p wow-data --lib category_`: six passed, exit 0,
  14:53:54–14:54:49, including five new category-reader cases.
- `cargo test -p wow-world-test-support --lib selected_charge_fixture_preserves_exact_canonical_spell_info_keys_without_payloads`:
  one passed, exit 0, 14:55:01–14:55:38.

These are 39 distinct cases, 37 new and two existing. The first production
check failed with an Option-closure typing error (exit 101, 14:49:51–14:50:36);
the first test build failed on imports without executing tests (exit 101,
14:51:29–14:52:58). Those logs are retained. Parent source review also corrected
the initial ignore-cooldown inverse and its test oracles, unknown-empty aura
admission, and the missing persistent-area effect kind before green feedback.
The 14:49:51–14:55:38 feedback/repair interval is 347 seconds, not a final
campaign or evidence that the ordinary 600-second macro acceptance target is met.
The parent reviewed the actual production changes, consumers, moved fixture,
test cases and raw results. The 20 implementation files predate their passing
checks; their reviewed content manifest is
`/tmp/rustycore-self-res-initializer-logs/parent-reviewed-files.sha256`, SHA-256
`d6d3349945f3a5a07033902025ea19ce6eb7177511b14a5bee549aa58fc73075`.
This identifies the working inputs, not final acceptance at a later commit.
Effective worker model/effort telemetry is unavailable; the configured role
is not reported as observed model execution.

This is an UNVALIDATED local macro checkpoint. No native-death caller was
enabled: Application still owns the complete preamble before NoDeath failure
and Unit aura removal. Actual Soulstone/Nether producers, normal SelfRes cast
preparation, physical-item/reagent handling, the other recovery consumers and
complete candidate acceptance remain open. The existing persisted-aura marker
does not prove every upstream producer. No runtime, SQL, restart, capture or
publication was performed.

Native Player SpellHistory integration, 2026-10-26, base `09600e731`: the
existing coordinated producer captures one Unix-nanosecond observation beside
its pre-World-pass Instant and forwards it unchanged through every
PlayerRecoveryAdmission in that tick. Standalone execution samples the same
system-clock domain once for its existing Unit pass. The native Spell helper
checks handle, active residence, map incarnation, GUID and world binding under
the manager guard that reaches the canonical Player. It updates the existing
Unit SpellHistory before selected Aura lifetimes and releases the guard before
Aura work, delivery or await. There is no new scheduler, clock owner, mutable
history mirror or config callback.

The existing millisecond update entry delegates to one precise algorithm.
Native u64 millisecond deadlines scale losslessly to u128 nanoseconds, preserving
source cooldown/category strict-before and charge at-or-before comparisons.
An elapsed held deadline expires too; the source's +MONTH representation is
not a permanent exemption. The loaded-row adapter now installs its nonzero
category index with direct assignment semantics, including a shorter replacement
deadline, and reset clears both cooldown maps. This is an intentional source
behavior correction, not structural movement.

Seven new World cases exercise actual admitted recovery, loaded-row projection
and reset, exact and fractional timestamp boundaries, a completed history update
before later Aura refusal, stale handle/map/residence, and replay with a later
timestamp that would otherwise expire another entry. One new Server case
checks identical full-precision forwarding to two participants; one Entity case
covers strict equality, a one-nanosecond advance, charges and elapsed +MONTH.
The later Aura refusal remains an unavailable overall recovery outcome; it is
not reported as a completed whole operation merely because history advanced.

Pinned source `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` anchors are
Unit.cpp:418–432,2836–2845; SpellHistory.cpp:153–164,220–244,424–428 and
SpellHistory.h:49,185; GameTime.cpp:103–107; Duration.h:42. The initializer's
remaining dependency is Player.cpp:24757–24780. The inspected history/time
source files have no local changes at this pin.

Worker feedback used the base SHA plus the reviewed delta on aarch64, PROTOC
set, one Cargo job and `/home/server/rustycore/target`. Full commands, source
hashes, UTC times, exits and identities are in
`/tmp/rustycore-player-history-unit-logs/`. Corrected-candidate evidence:

- `cargo check -p world-server --lib`: exit 0, 13:41:24–13:42:56 UTC,
  `04-world-server-check.log`.
- `cargo test -p wow-entities --lib spell_history`: exit 0, six passed,
  13:52:36–13:52:46 UTC, `10-wow-entities-spell-history-test.log`.
- `cargo test -p world-server --lib recovery`: exit 0, eleven passed,
  13:48:35–13:51:01 UTC, `08-world-server-recovery-test.log`.
- `cargo test -p wow-world --lib recovery`: exit 0, 33 passed,
  13:59:32–14:00:46 UTC, `11-wow-world-recovery-fixture-rerun.log`.
- `cargo test -p wow-world --lib ghost_wisp`: exit 0, 46 passed,
  14:02:38–14:03:36 UTC, `12-wow-world-ghost-wisp-test.log`.

The identity union is 94, with two World recovery cases shared between filters;
there are nine new cases. Earlier millisecond greens are superseded, not
precision evidence. The interrupted narrow World command retained exit 130
before any test ran when the requested filter changed. The earlier corrected
recovery run retained exit 101, 30 passed and three failed, exposing older
fixtures without sickness attributes or complete G/W key/catalog inputs.
Their positive inputs were repaired without changing production gates; the
later full G/W run also repaired its positive mask fixture. Deliberately empty
negative catalogs remain. The first broad Entity run preceded a test-only
correction to use the source +MONTH deadline; its repeated cases are not counted
again.

The native helper, World test leaf, Entity test leaf and Server forwarding leaf
are 49/376/69/38 lines; the existing Server fixture root is 862 lines. Physical
baselines are unchanged. The parent reviewed the source and fixture deltas and
verified all 21 file hashes against the worker content manifest, whose hash is
`8006586521161c7ca121ee041495d460b04e7b9243ed05ef224d4a152a2f00ba`.
The helper hash is
`339af8b19211e0c82e6bd922aaf4ea74755682f2b8c30edbd8be19906fbe67d7`;
the precise native algorithm file hash is
`77775a15d908178e38d73deffd90537fff01e83516f5a90da8e05217ca0e485f`.
Effective worker model/effort telemetry was unavailable and is not inferred.

The 13:21:20–14:03:36 feedback/repair interval is 2536 seconds, with earlier
implementation time separate. It is not a final campaign or evidence that the
complete ordinary acceptance meets 600 seconds. No parent Cargo, runtime, DB,
capture, restart or publication occurred. This remains an UNVALIDATED macro
checkpoint. The native death choreography, SelfRes initialization and ordinary
casts, recovery failures/durability, QA and full candidate acceptance remain open.

SelfRes field integration, 2026-10-26, base `c88cf6e87`: the live BTreeSet has
been removed from PlayerResurrectionState. Player::active_data now owns the
ordered i32 vector and its nested update mask. Named Player operations append
duplicates, remove the first matching value while marking shifted indices,
and clear the field even when already empty. Publication resets nested mask
bits without making the next append resend old values. Signed IDs and mask
boundaries above 32 entries are retained. Handle-less test fixtures use the
same ordered semantics; production and stale-handle paths do not fall back to
them. No second live list or stored snapshot was introduced.

Membership, removal, both represented revival consumers, and resurrection
snapshots use the new owner. The VALUES bridge copies the native vector and
mask. The owner CREATE serializer writes its real size and ordered payload;
both login_sequence and worldport self_create read that same native list.
The non-owner path does not expose it. The production-linked login case
seeds the canonical list through the existing fake persistence fixture,
observes the actual CREATE packet, and verifies the getter did not reinitialize
the list. This fixture performs no database integration or runtime operation.

Pinned target source `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
Player.h:2738–2745; Player.cpp:1196–1198; UpdateField.h:143–205,806–905;
UpdateField.cpp:21–41; UpdateFields.h:637; and
UpdateFields.cpp:3042,3096–3098,3153–3160,3327–3329,3476–3480.
The VALUES oracle preserves little-endian scalar fields and MSB-first packed
mask/size bits, followed by selected little-endian i32 values. The parent
checked that byte derivation against C++, rather than treating an earlier
incorrect test expectation or the Rust writer as proof.

Focused worker feedback ran sequentially on aarch64 with PROTOC set,
CARGO_BUILD_JOBS=1 and this checkout's absolute target directory. Logs and
command/start/exit/end metadata are in `/tmp/rustycore-self-res-list-logs/`:

- `cargo check -p world-server --lib`: final exit 0, 47 seconds,
  11:38:18–11:39:05 UTC, `world-server-check-reexport.{log,meta}`. The preceding
  corrected check also passed before the required field-bit re-export.
- `cargo test -p wow-entities self_res`: exit 0, three passed,
  `wow-entities-self-res.{log,meta}`, 11:29:36–11:29:56 UTC.
- `cargo test -p wow-packet self_res`: final exit 0, four passed,
  `wow-packet-self-res-mounted.{log,meta}`, 11:37:45–11:37:54 UTC. The rerun
  verifies the three new cases after their private module relocation.
- `cargo test -p wow-world-core self_res`: final exit 0, two passed,
  `wow-world-core-self-res-final.{log,meta}`, 11:32:32–11:32:58 UTC.
- `cargo test -p wow-world self_res`: exit 0, eight passed,
  `wow-world-self-res.{log,meta}`, 11:33:04–11:37:02 UTC. This includes the real
  worldport CREATE consumer and retains the original XP assertions.
- `cargo test -p wow-world --test production_login_player_owner production_login_create_projects_ordered_self_res_spells`:
  final exit 0, one passed, `wow-world-production-login-self-res-final.{log,meta}`,
  11:45:05–11:45:14 UTC. Zero-test integration targets in the broader filtered
  command are not additional evidence.
- The exact World library case
  `session::tests::scenarios_misc_5::canonical_player_resurrection_follows_active_detached_and_stale_ownership_like_cpp`
  passed once, exit 0, 11:56:39–11:57:53 UTC,
  `wow-world-canonical-resurrection-ownership.{log,meta}`. This restores the
  native list assertions removed from the old lifecycle-state snapshot and
  checks active, detached and stale/replacement ownership directly.

These are 19 distinct cases, including nine new cases. The earlier import
failure, Core test re-export failure, incorrect packet oracle and skipped
login-fixture seed remain in their raw failure logs. The coding/repair
feedback interval, including final review and its ownership correction, was
11:25:23–11:57:53 UTC (1,950 seconds); this is not the
complete ordinary final campaign or evidence that it meets the 600-second
target. The initial worker reports generic GPT-6 with effort telemetry
unavailable. Its resumed ownership correction instead reports gpt-6.1-sol;
the tool's fixed luna_worker route and that report disagree, so the effective
variant/effort is not claimed. The parent reviewed the correction and its raw
one-case result independently, then closed that worker.
No parent Cargo, workspace/architecture/final campaign, runtime, SQL, capture
or restart operation was run.

The new Entity and Packet test modules are 79/81 lines; the native operations
remain in the 220-line resurrection module. The bridge fixture is 956 lines,
and the login integration fixture/root are 339/533. The Player root grew seven
field/default/mask lines to 4,737, with its coherent methods kept outside it.
Review ceiling 4,737 expires at #43 final physical review; the next natural
cut is its ActivePlayerData value and mask responsibility under #584. No
physical baseline was regenerated. The parent reviewed the actual consumers,
diffs and retained file hashes. This is an UNVALIDATED local macro checkpoint;
ordinary cast admission/attempt timing, death-time Soulstone/Twisting Nether/
Reincarnation initialization, native death and the other #43 blocks remain
required. The field work alone is not complete SelfRes behavior.

La duración finita seleccionada requiere el mismo dueño: C++ descuenta
`diff` en `Aura::Update` y retira expiradas antes de publicar pendientes
(`SpellAuras.cpp:778–825`, `Unit.cpp:2836–2877`). El barrido Rust actual usa
Instant cada 200 ms y los snapshots de guardado descuentan ese tiempo de
pared. Antes de aceptar sickness, su countdown/expiry y su guardado deben
usar una única duración canónica del tick admitido, sin doble descuento ni
publicar estadísticas un pase después de la expiración. La reparación
seleccionada conserva fuera de alcance los relojes de las demás auras.
Decisión de representación previa a ese countdown: AuraApplication conserva
dos u32 con 0/0 como permanente; eso no distingue los valores C++ -1/-1 de
SetDuration(0) o del máximo cero cargado. Se migrarán esos mismos dos campos
a i32, con adaptación explícita de los callers legacy, sin otra duración
seleccionada paralela. Los consumidores no seleccionados conservarán su
comportamiento existente en ese adapter. Anclas: SpellAuras.h:176–177 separa
IsExpired (remaining==0, sin drop event) de IsPermanent (max==-1);
SpellAuras.cpp:818 descuenta diff solo con remaining>0. Un máximo permanente
no exime un remaining cero de expirar. _SaveAuras guarda los valores reales
y no filtra IsExpired: el pase Unit retira antes del save periódico, pero
otro punto de guardado puede observar cero legítimamente. Por tanto, la
captura seleccionada debe dejar de descontar Instant y de convertir una
aura todavía presente en ausencia por sí sola. Esta decisión no está
aceptada en la campaña final; 9h3C implementa la representación y retira el
hold del loader, conservando el del snapshot hasta su siguiente reparación.
El transporte debe conservar ambos valores cuando
el flag de duración lo exige, incluso remaining cero o -1.
Decisión del integrador para esa reparación (2026-10-25, antes de habilitarla):
reutilizar el rail y los permisos de Session en un pase interno de recuperación
admitido por Map, después de respawns y antes de Creature/P3. No se añade otro
reloj ni una fase de opcodes: World/Map conservan su registro. El pase usará el
diff del plan, expirará las auras seleccionadas antes de regeneración y
publicará sus pendientes en el punto Unit, antes de KillPlayer y deathTimer.
Así, Ghost creado por automatic release después de ese punto esperará al
siguiente pase, como en C++. El barrido legacy de las demás auras conserva su
cadencia y se excluirán de él las familias con duración canónica seleccionada.
La alternativa de mantenerlo en World conserva el orden/diff incorrectos; un
segundo ejecutor nativo de todos esos participantes duplicaría dueños existentes.
El nuevo corte debe soltar todos los guards antes de solicitar el pase y volver
a comprobar generación, residencia, encarnación, epoch y permiso. Respawns
ya aplicados conservarán sus intenciones DB: se enviarán una vez, bajo el gate
de orden existente, antes del await; los intents de visibilidad siguen en Map
hasta el tail legítimo. Cancelar el pase no autoriza perder ni repetir esos
efectos. Los tests de composición deben probar esa frontera, también sin lease
Creature. La inspección detectó además que SaveTimer solo marca un pendiente:
el consumidor actual lo guarda después de deathTimer/dispatch, mientras C++
guarda antes (`Player.cpp:1055–1066,1101`). El pase seleccionado consumirá ese
save en su punto, reutilizando el owner/fences actuales y comprobando el binding
tras await; no continuará recovery después de quarantine/desconexión. Las
fronteras aún incompletas de melee/intercalado por Player de #584 permanecen
explícitas. Esta decisión no es implementación ni aceptación de ese pase.

La carga de efectos seleccionados tiene una corrección en curso (9g):
`Aura::TryCreate` descarta máscara vacía y aplica solo el subconjunto activo
(`SpellAuras.cpp:384–394,494–502`). La restauración Rust aplica Ghost y la
selección de transform de Wisp sin consultar esos bits, y su retirada exige
siempre los tres efectos. Se debe admitir el subconjunto válido de 8326/20584,
sin activar 95/56 ausentes. La frontera canónica rechaza bits desconocidos;
el loader debe seguir la normalización previa de C++ descrita a continuación.
El worker ha escrito la reparación; todavía no hay aceptación del candidato.
El contraste completo de `_LoadAuras` añade una corrección al contrato anterior:
`TryCreate` intersecta la máscara SQL con los efectos soportados, pero los
amounts/basepoints se buscan por la clave SQL original. Un subconjunto vacío
no instala aura. Además, `_LoadAuras` llama `CanBeSaved` ANTES de
`SetLoadedState` (`Player.cpp:18117–18145`, `SpellAuras.cpp:398–441,479–480`).
Por tanto, un Ghost/Wisp con item GUID no vacío se considera permanente por
su duración base y se descarta al cargar, aunque la fila guarde duración finita.
La captura finita con item de 9d2 sigue siendo válida para Save, pero su test
de roundtrip no demuestra fidelidad de Load y debe corregirse expresamente.
La revisión de la operación completa corrige también la premisa de 9b2 de
conservar otro caster al crear una aplicación nueva del mismo Ghost/Wisp:
`Unit::_AddAura` ejecuta `_RemoveNoStackAurasDueToAura` antes de la aplicación
(`Unit.cpp:3285–3289,3547–3564`); `Aura::CanStackWith` rechaza el mismo spell
salvo sus excepciones (`SpellAuras.cpp:1593–1731`). Los datos seleccionados
no tienen DOT_STACKING_RULE (attr3 0x80), ni son auras periódicas/multislot.
Un refresh de la aplicación existente sigue siendo una ruta distinta que
conserva identidad/procedencia. La corrección debe cubrir nueva aplicación,
mask distinta y carga de varias filas, preservando orden de retirada antes
de los efectos. En Load, la construcción y esa retirada preceden a
CanBeSaved: una fila ligada a item puede retirar la anterior y luego ser
descartada ella misma. No se atribuye fidelidad a los tests anteriores que
esperaban conservar ambos casters. La reparación local 9g sustituye esas
expectativas; no convierte su feedback en aceptación final.
La consulta SQL de solo lectura a las 01:18 UTC no encuentra membresía de
8326/20584 en world.spell_group (287 filas globales; diez stack rules).
No se usa ese resultado para declarar implementado el motor general de
acumulación ni sus reglas para otros spells/datasets.
La revisión del primer filtro verde de 9g (37 casos) rechazó un supuesto
incorrecto del propio contrato: con Wisp transform y recalculateMask bit 2
activo, SetLoadedState activa CanBeRecalculated y llama RecalculateAmount
(`SpellAuras.cpp:1239–1255`, `SpellAuraEffects.h:74–77`). CalculateAmount
calcula desde baseAmount, desactiva ese indicador para TRANSFORM y devuelve
la cantidad nueva (`SpellAuraEffects.cpp:658–680,718–724`). Solo con el bit
inactivo se conserva el amount SQL. Se corrigen loader y pruebas para ambos
casos y para la recaptura sin ese bit; aquel verde no prueba este contrato.
El diagnóstico ampliado de datos instalados (solo DB2, sin overlays SQL)
identifica 21/21 ChrRaces con sickness 15007 y 26 LiquidType sin referencias
a 8326/20584. ItemEffect tiene 24 713 filas y 86 con trigger 5 y SpellID > 0;
85 tienen Item/ItemSparse y no llevan LEGACY, incluidas 39 con cargas no cero.
Esto no prueba que estén en un inventario ni que sus spells estén portados,
pero impide omitir CastItem/consumo alegando ausencia de datos alcanzables.
RequiredLevel no se expone en el reader actual; no se inventa ese filtro.
Log acotado: `/tmp/9g-data-ghost-wisp-load-boundary-items.log` (un diagnóstico
real, no cero tests). Hashes de tabla: ChrRaces 0x53F1783C, LiquidType
0x6613BED3, ItemEffect 0x4002A5B1, Item 0x50238EC2, ItemSparse 0x919BE54E.
Sickness 15007 tiene efectos 80/79/159, basepoints -76/-76/-1 y dieSides 1;
su duración 427 guarda -600000/60000/600000 (GetDuration toma el absoluto).
Ghost, Wisp y sickness carecen de fila SpellAuraOptions en estos datos.
El primer diagnóstico usó un filtro OR demasiado amplio; se corrigió a
trigger 5 AND SpellID > 0 antes de usar estos conteos. No son capturas vivas.
Selected AuraUpdate metadata remains unfinished. The source contract reviewed
on 2026-10-25 is `SpellAuras.cpp:249–289` at the pinned C++ SHA above:
Applications is the actual stack count when IsUsingStacks (catalog StackAmount
or runtime stack greater than one), otherwise the canonical charge count.
CastLevel is captured once by the Aura constructor from its resolved caster,
or SpellLevel when none resolves (`:398–410,463–468`); target level changes and
refreshes must not replace it. Attr11 SCALES_WITH_ITEM_LEVEL selects the cast
item level converted to uint16 instead. Preserve this origin in existing Aura
provenance, with no second mutable charge, stack, or wire-count authority.
The existing Core spell catalog already supplies SpellLevelsStore.

The wire Duration flag and optional pair are derived from current maximum
duration greater than zero and absence of Attr5 DO_NOT_DISPLAY_DURATION
(`SharedDefines.h:626`, bit `0x400`), preserving exact signed remaining values.
Scalable Points uses effect indexes, including zero-filled holes: active mask
`0b101` produces three entries, not two. The selected effect types do not
produce EstimatedPoints (`SpellAuraEffects.cpp:727–751`); this is not a claim
about the complete aura engine. A non-Unit raw caster projects an empty CastUnit
on the wire while its original GUID remains canonical provenance.

The flag contract is `_InitFlags` (`SpellAuras.cpp:110–161`), including its
any-positive branch for a resolved friendly foreign caster. Self, missing, and
hostile casters use the all-positive branch. These base flags are established
at application creation, not recomputed from later friendship at publication.
Caster resolution must preserve
the self-owner fast path before world admission. A foreign Player must be in
world and on the owner's map (`ObjectAccessor.cpp:206–239`); do not substitute
the target's reputation for that caster's owner or acquire the Session directory
under a map guard. Delta and initial full updates must share the selected
projection. The checkpoints above distinguish the implemented consumers from
the remaining canonical friendly-caster and load work.

The existing installed-data diagnostic provides a concrete constructor check:
SpellMisc records 320590/329372/325284 for 8326/20584/15007 all contain
Attr0 AURA_IS_DEBUFF (`0x04000000`). They are not passive. Source therefore
marks every effect negative before its per-aura-type or target checks
(SpellInfo.cpp:4518–4541; SharedDefines.h:437,457). In particular, a selected
partial mask does not make stock Ghost or sickness positive, and stock Wisp
does not become cancelable from its speed effects. Both friendship branches
produce negative flags for this known uniform polarity. Preserve the general
mixed-polarity constructor contract; do not pretend an unresolved caster is
hostile to obtain the uniform result. Evidence is the existing file-data log
`/tmp/9g-data-ghost-wisp-load-boundary-items.log:248–250`, not a new effective
SQL-overlay run or a client capture. Fresh construction is repaired by
9i3C3d3b, and mixed foreign-caster construction is connected by 9i3C3d3c.
The load constructor remains pending.

Constructor reaction decision, 2026-10-25: reuse the existing Core faction
reaction algorithm through a transient borrowed input, rather than copying it
into Spell or constructing a Session for the foreign caster. Each affecting
Player supplies its own canonical reputation, race, and class. Existing NPC,
quest, and spell-click adapters remain consumers of that one algorithm.
Self, absent-caster, and already-decided reaction branches must not require
unrelated faction catalogs. A catalog needed by an undecided branch must be
known; a known missing row follows Source, whereas an absent store remains
unavailable for selected construction. No Player or reputation clone, storage
mirror, directory lookup under the Map guard, or new mutable owner is needed.

The source comparison found a specific existing defect: the controlled-player
reputation branch tests the source affecting Player's contested-PvP flag,
while the previous Rust input used the target flag. The lower faction branch
correctly tests the target flag. 9i3C3d1 repaired these separately named facts
in the shared algorithm; this is an intentional behavior correction,
not a move-only refactor. Anchors at the same pinned SHA are Object.cpp:
2709–2864 and ReputationMgr.cpp:75–83,181–198. Friendship thresholds are needed
only when the reached faction has a nonzero FriendshipRepID; a known store
without reactions uses ordinary thresholds. Do not interpret an unavailable
friendship store as that known absence.

Owner resolution follows Object.cpp:2230–2275: direct resolved charmer/owner
for the shared-owner comparison, and the bounded GetAffectingPlayer chain for
reputation. It is not an arbitrary recursive ancestor search. The raid check
first calls inherited Unit::IsInRaidWith (Unit.cpp:11401–11419): compare the
affecting Players' direct resolved charmer/owner-or-self Units, then their
shared identity, Player group relation, helpful-raid Creature static flag,
or Creature faction equality, in that order. Its Player comparison uses
IsInSameRaidWith and current Group identity (Player.cpp:2139), not OriginalGroup.
Calling only the Player helper would omit the represented control branches.
The final Creature/Creature comparison uses raw FactionTemplate values:
Unit.h:834 defines GetFaction as that uint32 field, not the faction joined
through FactionTemplateEntry. It requires no faction catalog. Preserve the
stored field's bits when forming reaction inputs. Canonical owner identity
validation precedes self lookup, but self level and non-Unit fallback must not
require map facts; known out-of-world foreign Players are absent before any
map-assignment comparison (ObjectAccessor.cpp:233–239).

The adapter must also preserve forced-reaction and duel decisions before the
inherited raid lookup. The selected control graph can be unavailable at that
later lookup. Keep one Core algorithm and provide a narrow lazy raid-fact
callback at its existing raid branch; the legacy entry supplies its existing
known boolean. A needed unknown raid fact holds, while an earlier forced,
same-owner, or duel decision does not request it. 9i3C3d3c connects this callback
inside the current macro, without a second reaction algorithm or authority.
The selected effect polarity remains a separate SpellInfo decision from
friendly-caster application flags. The connected canonical adapter and its
checkpointed fresh consumer feedback do not complete the
general faction, summon, or aura engines.

Selected metadata follow-up, 2026-10-25: keep cast difficulty distinct from the
resolved SpellInfo key. Spell.cpp:7916–7919 returns the caster Map difficulty,
AuraCreateInfo retains it (SpellAuras.cpp:59–60,463), and Player.cpp:19839 saves
GetCastDifficulty; the loader passes the saved difficulty back at :18122.
Do not rewrite that field to the difficulty of an inherited metadata row.
SpellMgr.cpp:675–692 resolves the key separately, while :2610–2688 fills each
missing metadata component along its own chain. Its loop does not special-case
zero: a known Difficulty(0) overlay may provide a real fallback. Selected
lookups must preserve that case, distinguish a known absent row from an unknown
store, and reject a needed cycle instead of hanging or inventing a base row.
A known metadata store without any row for the spell can still prove its
Source default independently of the chain. The selected readers corrected in
9i3C3d2c/9i3C3d3a follow this contract; the general legacy helpers retain their
separate boundaries. Indexed inheritance is available from 9i3C3d4a, while
its remaining execution/load consumers are identified below.

Effect-field follow-up: execution now replaces selected effects before preparing
the cast; 9i3C3d4e also replaces the selected loader's whole legacy effect vector.
Source fills each effect index independently along its difficulty chain
(SpellMgr.cpp:2500–2508,2638–2640; SpellInfo.cpp:1144–1163). 9i3C3d4a repairs
the former Data loss of explicit Effect=0 rows and duplicate slot selection.
It retains complete indexed rows and a shared resolver in the existing catalog,
with a separate active legacy projection. 9i3C3d4b connects the save adapters
and 9i3C3d4c connects execution and sickness application/stat/snapshot readers.
9i3C3d4e connects loading to indexed effects and exact selected metadata; visual
and remaining charge consumers still require completion before macro acceptance.

9i3C3d4d replaces the selected completion consumer's legacy represented positivity
helper with the shared whole-spell decision. That helper read effect types and
amounts but ignored attributes such as AURA_IS_DEBUFF, so positive effect amounts
could disagree with Source polarity despite correct indexed selection. Required
selected effects, attributes, and polarity are now prepared before cast
preparation and SpellGo. Source no-initial-aggro attributes bypass unnecessary
polarity requirements, preserving matching refresh when creation-only facts are
unavailable. Completion retains the prepared decision and samples threat amounts
after effects, at the existing threat boundary (Spell.cpp:5558–5625).
Spell::IsPositive also checks the triggering aura when one exists
(Spell.cpp:8067–8070). The direct lifecycle casts have no such trigger; their
whole-spell decision must not be advertised as general triggered-spell parity.
HasInitialAggro unconditionally excludes NO_HARMFUL_THREAT, in addition to
NO_THREAT and NO_INITIAL_THREAT (SpellInfo.cpp:1721–1724). The shared threat
algorithm now preserves that unconditional gate for both selected and legacy
callers; its positive and negative regressions passed in worker coding feedback.

Resolver ownership decision, implemented by 9i3C3d4a/b: shared spell difficulty
traversal, exact attributes, and indexed effect resolution belong to Data;
Spell retains private aliases and Core save adapters consume Data directly.
Core must not depend on Spell for catalog facts. The existing per-difficulty
collection retains inert winning rows, with an active prefix for its borrowed
legacy projection.
An inert-only bucket must continue the legacy getter's existing fallback,
because that bucket was previously absent after filtering. The new exact
resolver instead retains the inert slot as an explicit inheritance override.
Highest-RecordID slot selection is an explicit common metadata correction,
not a behavior-preserving move. No additional mutable effect catalog is needed.
Dynamic base payloads may supply their existing explicit effect vector only
when no raw per-difficulty rows for that spell exist; they must not mask a
known missing or inert raw row. A malformed duplicate/index in a dynamic vector
must remain unavailable rather than becoming a newly accepted selected Aura.
Malformed raw effect indexes must likewise remain detectable by the exact
resolver, rather than being discarded into apparent absence or rewritten to zero.
Loading and remaining source-identity/polarity consumers still need integration
and evidence after the Data/save/execution cuts. The connected selected save adapters
derive passive/channelled eligibility from each application's resolved
attributes. A global base SpellInfo flag must not reject a valid application
at another difficulty. Existing complete-
authority and supported-family absence checks remain required for deletion.

Constructor integration cut, connected by 9i3C3d3b: Spell computes flags from selected
metadata and canonical caster facts; the existing application capability consumes
them. The fixed sickness `0x15` in Application is retired through the existing
host/CX call chain, without coupling Application to the classifier. The retained aura's original
flags and captured level remain authoritative on refresh. Source
Unit.cpp:3224–3280 and SpellAuras.cpp:341–383 refresh before creating a new Aura;
do not introduce new friendship/level lookups as a prerequisite to refreshing an
already matching application. 9i3C3d3c connects the canonical foreign relationship
through the same creation/refresh boundary.

Refresh/creation boundary decision: carry selected creation preparation as an
owned Result of computed flags and the existing AuraCastProvenance through the
current Spell host and Application capability. This result has no side effects,
canonical storage, or new lifetime authority. The existing same-caster/full-mask
refresh branch does not consume it and retains the old flags and provenance.
Only the fresh branch requires success, before replacement/removal or insertion.
An unavailable fresh caster or polarity fact must therefore reject fresh creation
without rejecting a valid refresh or removing its previous application. This
avoids a separate preflight query whose refresh decision could become stale
between canonical borrows. Application receives primitive computed flags and
existing provenance; it does not depend on Spell's classifier. Caster level
is captured under one borrowed manager guard. The connected foreign relationship
adapter captures its facts in that same guard; all guards must be released
before publication. The boundary is covered by 9i3C3d3b and its foreign
relationship consumer by 9i3C3d3c. 9i3C3d4e shares the fresh constructor with load
through Aura's private module, using the existing SessionHubHostLikeCpp bound
already inherited by both caller hosts. It receives raw cast difficulty
explicitly: current Map difficulty for execution, saved row difficulty for
loading. Load classifies whole-spell offline expiration before requesting live
caster facts, then prepares fresh flags and level before removing an existing
application. It still removes conflicts before CanBeSaved, as Source does.
This placement adds neither a new host trait nor a WorldSession method.
Load visual selection is also pending: Aura's constructor calls the resolved
Unit caster's visual method when available and uses the caster-free SpellInfo
method otherwise (SpellAuras.cpp:463–468). Self resolution still supplies that
caster before world admission. Unit.cpp:13548–13563 applies active visual
overrides first; SpellInfo.cpp:4446–4464 checks conditions only when their
catalog rows exist. The current loader always chooses its caster-free helper.
Precise visual inheritance and needed caster/condition facts must be resolved
before replacement; missing facts must not be relabelled as visual zero.
Visual absence proof must cover complete effective regular and server-side
effect metadata. Misc values are static SpellEffectInfo properties
(SpellAuraEffects.h:56–57), so a proven absence of any matching visual-override
effect can avoid an unrelated runtime-aura completeness requirement. If a
potential override exists, its active source and order still require resolution;
catalog potential alone is not proof that it is active. Condition-row presence
also needs an effective catalog, including Hotfix additions and removals.
An ID-only UnitCondition catalog is sufficient for known absence and for
caster-free rejection of existing conditions; it does not evaluate a needed
Unit condition. Neither condition ID zero nor an unavailable store by itself
proves that no constraint exists. These catalog/visual connections remain pending.

Condition-presence implementation decision, 2026-10-26: reuse Data's ID-store
model for a dedicated effective PlayerCondition/UnitCondition presence catalog,
combining file IDs, official/custom SQL additions, and final table-hash/record-ID
removals. The existing typed PlayerCondition payload is file-only and cannot
prove SQL-effective predicate values. A single condition-presence persistence
capability can supply both ID sets without introducing a predicate engine or
putting SQL in Data. Source DB2Store.cpp:127–134 loads official then custom;
DB2DatabaseLoader.cpp:28–32 inverts its `custom` argument for VerifiedBuild.
HotfixDatabase.cpp:1192 onward and :1853–1857 own the two table queries. Needed
file/query/decode failures remain unavailable. WDC4 `iter_records` includes only
copies whose source resolves in its direct-ID index; compare its observed count
to `total_count` before claiming complete ID absence. An unresolved copy must
withhold that proof rather than silently becoming a missing condition row.
9i3C3d4f implements that effective presence catalog and composes it at startup
and into sessions. Its free composition adapter installs only this immutable
capability, without exposing mutable catalogs or adding a WorldSession method.
Visual selection and predicate evaluation are separate pending consumers;
neither the catalog nor its coding feedback establishes macro acceptance.

Visual-consumer review, 2026-10-26: a complete static override-absence proof must
not inspect only SpellStore's narrowed i32-spell/u8-difficulty effect projection.
Its hydration skips rows outside those domains, while an otherwise valid
uint32 spell can carry an override targeting a selected lifecycle spell. Retain
potential override targets from the complete effective regular effect domain
before narrowing, and include existing dynamic payloads and the accepted
server-side SpellInfo effects. A Data-owned derived metadata index is sufficient;
it is not an active-Aura authority. The already-loaded server-side catalog needs
production session composition before its absence can be consumed. An available
potential override still requires its active source/order, not an assumption
that it is active. Source anchors remain Unit.cpp:13548–13563 and
SpellAuraEffects.h:56–57. The consumer and actual effective-data proof are pending.

The same review identifies an owner-context correction for that constructor
consumer. `selected_owner_map` currently requires the owner to be a physical Map
member. Source Aura::Create can resolve a foreign caster against a target whose
Map is already assigned before world admission: ObjectAccessor.cpp:206–213,
236–238 calls the target's GetMap, and Object.h:604 requires that pointer rather
than world/container membership. Accept the canonical detached target's valid
assigned Map while preserving handle, residence, map, and active-member
consistency checks. Self and non-Unit caster fast paths stay independent of Map
membership. Add actual detached-owner/foreign-caster tests before claiming this
load path complete; the earlier active-owner tests do not establish it.
The existing active-owner test also preserves foreign-caster lookup while the
requester is out of world but still has its canonical Map member and assignment.
Do not replace its positive expectation with an execution-admission rejection.
Both residence-query APIs apply world-admission checks; use their explicit
diagnostics with the borrowed canonical body and assigned Map to distinguish
that case from an inconsistent active key or a detached assigned target.
This metadata read needs neither a new residence API nor a scan of every Map.

The visual consumer includes execution as well as loading. Spell.cpp:583 obtains
the cast visual from the caster; Aura's constructor independently obtains its
visual at SpellAuras.cpp:465. A supplied cast visual is therefore not creation
proof for a fresh selected Aura. The current Ghost/Wisp overload also returns
zero for Wisp without consulting effective visual rows, and the sickness
overload uses the bounded creature helper. Connect the exact selected resolver
at the cast boundary and the shared fresh constructor, preserving refresh's
original Aura provenance. Preserve all uint32 visual-ID bits when projecting
into the signed wire field (Spell.cpp:9283–9286), rather than clamping to
INT32_MAX. Known absence of visual rows returns Source zero; unavailable needed
metadata must remain an error. These consumers remain pending.

Reference provenance for these metadata/visual reviews: the pinned HEAD remains
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, but the reference checkout is not wholly
clean. DB2Stores.cpp has a pre-existing specialization-bounds change after the
unchanged LoadFromDB anchor. Spell.cpp has pre-existing template instantiations
appended after the unchanged constructor/visual-conversion anchors. Those anchors
were checked against `git show` at the pinned SHA. A separate LFGList.cpp change
is outside this review. All three local changes are preserved. A future paired
runtime/build record must retain the actual reference candidate and dirty state;
these source reads are not evidence of a new reference build or capture.

Selected charge consumer review: the legacy AuraOptions helper clamps signed
ProcCharges into u8, whereas Source preserves its uint32 assignment and final
uint8 conversion. Fresh Aura::CalcMaxCharges also consults SpellProcEntry and
the resolved caster's applicable spell modifiers; refresh calls CalcMaxCharges
without a caster (SpellAuras.cpp:965–976,1090). Load normalization instead tests
the full SpellInfo ProcCharges value before converting an empty saved charge
count, and preserves an already nonzero saved count (Player.cpp:18108–18118).
The selected producer/load consumers must use those distinct contracts; changing
the general legacy helper is not required. Known absence must come from the
effective catalogs, not from an unavailable proc/modifier source. The existing
file diagnostic found no AuraOptions rows for 8326/20584/15007, but does not
prove absence of SQL overrides or matching runtime modifiers. 9i3C3d4e connects
the selected loader's full-width normalization, including preservation of
nonzero saved charges. Fresh and refresh CalcMaxCharges consumers remain
pending and are not claimed by scalar projection or metadata lookup tests.

Production-input review, 2026-10-26, identified file-only AuraOptions,
ClassOptions, Levels and Label calls in world-server/app.rs. Exact in-memory
lookup could not supply missing SQL payload overlays. j1/j3/j4 now connect
these four stores through the existing core-spell Hotfix port, official/custom
replacement and final removals, preserving their Data and immutable session
owners. Source HotfixDatabase.cpp:1412–1416,1443–1446,1504–1506,1513–1516 owns
their queries. This closes those composition gaps; it is not actual SQL data
or runtime evidence for selected polarity, fallback caster levels, stack/charge
limits or absence of eligible label modifiers.

The SQL decoder widths are fixed by DB2LoadInfo.h:4655–4673,4759–4775,
4984–4994,5009–5024 at the pinned reference SHA. AuraOptions keeps signed
32-bit recovery, charges, and proc masks; Levels keeps signed 16-bit base,
maximum, and spell levels. ClassOptions reads its four class masks through
signed 32-bit getters, then preserves those bits in the existing unsigned
Data fields. Its SpellID is also signed; Label fields are unsigned 32-bit.
Reuse the existing native-width Field getters and round-trip checks. Do not
silently drop malformed rows or substitute zero for a failed conversion.
The implementation must preserve C++ NULL-to-zero behavior separately from
conversion failure, and fail startup before installing a partial catalog.

Refresh charge calculation also belongs to the existing Aura's SpellInfo:
SpellAuras.cpp:1088–1090 calls CalcMaxCharges without a caster on that Aura.
Resolve its original difficulty and key, rather than using the new cast's
difficulty. Fresh creation can require caster-modifier eligibility facts that
this refresh does not need. Required scalar metadata must remain unavailable
when unknown; creation-only modifier failures must not reject an otherwise
valid matching refresh. The no-eligible-modifier proof requires effective
family flags and labels, or IGNORE_CASTER_MODIFIERS; zero family flags alone
do not exclude label modifiers (SpellInfo.cpp:1784–1813).

Charge lookup must distinguish the raw Aura cast difficulty from the resolved
SpellInfo key: GetSpellProcEntry starts at SpellInfo::Difficulty
(SpellMgr.cpp:490–509), then follows its chain. SQL proc rows are keyed using an
already resolved SpellInfo (SpellMgr.cpp:1511–1517,1632); they do not contribute
new SpellInfo keys. Fresh and refresh calculations consult that
entry, including an explicit zero; saved-charge normalization does not.
The existing canonical proc store can supply exact entries without using its
legacy unguarded fallback loop.

Selected charge integration decision, 2026-10-26: keep fresh-only charge
preparation outside the shared constructor used by persisted aura loading.
The application selects a matching full-mask aura before observing any fresh
creation failure. A timer-refreshing match reads charges from its existing
SpellInfo difficulty with no caster; a non-refreshing match does not request
charge metadata. Fresh self-casts may bypass modifier lookup only when Attr3
IGNORE_CASTER_MODIFIERS (`0x20000000`) applies, or effective family flags are
all zero and the effective label list is empty. Every label row counts,
including LabelID zero; labels are inherited as a list when the difficulty
chain reaches their difficulty-zero source. Missing effective catalogs are
not known absence. A known missing difficulty node instead terminates a
finite chain and preserves default family flags and an empty label list;
do not reinterpret that proven absence as an unavailable catalog. Signed
ClassOptions SpellID storage preserves uint32 identity bits. The selected
paths have no canonical SpellMod registry
from which to calculate an otherwise eligible modifier, so that case remains
a preparation error before conflicting auras are removed. This does not
retire the broader modifier owner or claim its implementation. Reuse immutable
catalog borrows and the existing canonical proc store; introduce no charge
cache, extra Player, operation lock or scheduling owner. The source anchors
are SpellAuras.cpp:965–976,1078–1094, SpellInfo.cpp:1784–1817,
SpellMgr.cpp:2555–2556,2649–2650, and Object.cpp:2274–2295 at the pinned target.
This records the next consumer contract, not executed consumer acceptance.

Consumer admission must preserve the already-supported sickness path with a
resolved foreign original caster. The no-eligible-modifier predicate is also
sufficient there: SpellInfo.cpp:1784–1817 returns false for every class-mask
or label modifier under those facts, independently of which caster owns the
modifier registry. Generalize the helper's self-caster naming/contract when
wiring that path; do not introduce a self-only rejection or claim a general
modifier engine. Separately, require the selected cast's actual SpellInfo key
before execution publication. Missing cast existence is an admission failure,
not a fresh-only constructor error that a matching refresh may ignore. Keep
fresh-only modifier failures separate from that all-cast key requirement.

The m2b feedback exposed a distinct production projection gap: a selected
nonzero-only SpellInfo can have exact effects/attributes and a canonical key
while the legacy base-payload getter is empty. Execution and sickness admission
must not reinterpret that as missing SpellInfo, or manufacture a difficulty-zero
key in a positive fixture. The settled correction permits a transient existing
SpellInfo carrier only in the selected server-triggered FULL path, after exact
key resolution, no difficulty-zero key, the supported three-effect Aura shape
and known non-channelled attributes. No carrier is installed in the catalog.
Under a complete exact key authority, absence of zero also excludes the
zero-only CastingRequirements contributor, so focus is zero. The selected FULL
direct path does not consume the otherwise unused cast-time or client
power-debit fields. The reader audit also found unconditional cooldown
bookkeeping/event publication in Rust completion: pass the existing explicit
selected-FULL decision to completion and skip that cooldown block, preserving
any existing cooldown. Spell.cpp:4195–4203,8036–8039 and
SpellHistory.cpp:252–255 return before handling cooldowns under the FULL
IGNORE_SPELL_AND_CATEGORY_CD flag. This is a required selected-path behavior
correction, not a structural move or general FULL-engine completion. Canonical
effects and the separate exact Aura metadata readers remain authoritative.
SpellDefines.h:244–261,446 and Spell.cpp:3540,3816,5977 establish the other
parts of that bounded contract. Ordinary client,
channelled and general sparse-payload cast preparation remain required in their
existing normal-cast owner; this is not general scalar/periodic-cost parity.
9i3C3d4m2b implements these repairs and records the affected green feedback
above. They are not final accepted production or runtime evidence.

The original selected refresh finding, corrected by 9i3C3d4m3, requires the
original SpellInfo's timer policy and duration, not just its proc difficulty. Unit.cpp:3244–3278
copies new base points into existing effects; SpellAuras.cpp:1021–1094 then
uses the existing Aura for stack policy, recalculation and timer refresh.
RefreshTimers at :933–952 calculates the original maximum without a caster
and applies Attr13 PERIODIC_REFRESH_EXTENDS_DURATION when set. At that review,
sickness refresh already preserved original effect properties, while its
policy/duration arguments came from the new cast; Ghost/Wisp also recalculated
from new effect properties. 9i3C3d4m3 corrects these consumers with focused
feedback above; it does not establish complete selected metadata acceptance. Preserve
raw cast difficulty on the canonical application, resolve its actual SpellInfo
key through the immutable catalogs, and validate the snapshot's difficulty
when committing the existing-slot mutation. No new stored SpellInfo mirror
is needed. Complete these findings within #43 after charge calculation and
its consumer wiring.

Its production preparation also needs correction before selected charge
acceptance. SpellProcStore::from_rows_and_stores currently projects every
SpellStore payload as difficulty zero and builds its explicit-row lookup by
spell ID alone. Source loads explicit SQL rows through GetSpellInfo(id,0),
which can return a different key through a real zero fallback, and generates
implicit rows for each actual SpellInfo key. Build those two views from the
existing key authority and exact component readers, with the known Difficulty
store; keep one canonical proc store. A missing entry in the current base-only
projection is not proof of Source absence. Implicit charges equal the selected
SpellInfo ProcCharges (SpellMgr.cpp:1871), but that equality does not repair an
explicit row assigned to the wrong key. This is metadata preparation for the
selected charge consumer, not a claim to have completed the general proc engine.

The equality of implicit Charges and SpellInfo ProcCharges is only an exact-key
fact, not permission to skip implicit-entry presence. GetSpellProcEntry follows
the difficulty fallback when the exact entry is absent (:490–509). A generated
zero-charge entry can stop that search; suppressing it can expose a fallback
entry with nonzero charges. Source's infinite-loop guard (:1873–1888) depends
on ProcBasePPM when charges are zero, so file-only PPM could not establish
effective entry presence. C3d4k now connects its complete FILE loading,
official/custom SQL overlays and final removals; the joint proc preparation and
charge lookup still need to consume that input correctly. HotfixDatabase.cpp:1542 and
DB2LoadInfo.h:5109–5118 specify uint32 ID, float BaseProcRate and uint8 Flags.
This is a dependency of the selected charge lookup, not a general proc-runtime
completion claim. No new parallel charge store is required.

The source views also include accepted server-side SpellInfo rows: explicit SQL
resolves against the joint map (SpellMgr.cpp:675–693,1511–1517), and implicit
generation traverses that same map (:1742–1752). Reuse the already-loaded
server-side catalog, including its direct full-width proc fields
(:2916–2921), rather than projecting regular spells alone. Preserve admitted
server keys; orphan effect rows remain excluded. Its
SQL float base points first enter the signed int32 SpellEffectEntry field
(:2755; DB2Structure.h:3504 onward), before SpellEffectInfo calculations.
Do not round that assignment or silently narrow server spell-family metadata:
SpellInfo.h:409 and SpellMgr.h:317 retain uint32 family names even though the
explicit proc SQL field itself uses GetUInt16 (:1533).

Further input review, 2026-10-26: all nineteen existing core Hotfix DTO numeric
widths match DB2LoadInfo; the four SpellEffect class masks intentionally keep
the signed getter's bits in unsigned Data fields. Their adapter substituted
defaults or dropped rows after failed SQL decoding; j3 replaces that behavior
with the existing native Field getters and whole-load failure. C3d4k applies
that boundary to the key adapter's consumed fields as well. Source NULL-to-zero
remains distinct from conversion failure. These input repairs do not establish
the actual effective dataset or complete selected proc/charge consumers.
FieldValueConverters.h:62–70 and Field.cpp:62–67 own this contract. No installed
SQL schema drift or affected runtime row was demonstrated by this source review.

The production Difficulty composition already applies SQL overlays and final
removals; it is not a file-only store. Its former adapter erased native width
before checking an i128 range, admitting int64 -1 as uint8 255 where Source's
round-trip rejects it and treating SQL NULL as failure rather than zero.
C3d4k now uses native getters for the consumed fields (projection columns
0/2/5/8/10; DB2LoadInfo.h:1534–1555, HotfixDatabase.cpp:519–521). Its production
FILE reader also rejects unresolved copy rows, preserving the current store,
port, startup position and overlay/removal order. No second graph is added.
This is modeled input-repair evidence, not a report of an affected installed
SQL row or actual fallback lifecycle acceptance.

Difficulty normalization has a separate source rule: DBCEnums.h:833 declares
`Difficulty : uint8`. Regular SpellEffect rows cast their int32 difficulty into
that enum before indexing (SpellMgr.cpp:2508); server effects and parent rows
cast their SQL uint32 values before validation/keying (:2732,2772,2859).
Do not widen the canonical key authority to raw uint32 difficulty or confuse
the raw getter width with the enum conversion. Existing regular key folding is
intentional. j2 now normalizes the regular payload and server-model boundaries
and preserves ordered row winners, so aliased effects join their admitted
parent. Its focused alias regressions are recorded above; orphan effects stay
excluded. This does not replace effective SQL inputs or actual dataset evidence
before accepting visual/proc absence.
Server SpellInfo construction also assigns effects by slot in query order,
with the last row winning and empty slots retained (SpellInfo.cpp:1337–1353).
Duplicate parent keys retain the first constructed name/effects while later
rows overwrite scalar fields (SpellMgr.cpp:2870 onward). Preserve that sequence
after normalization. Negative effect indices reach undefined vector access in
the legacy constructor; the selected valid-input contract rejects them with
the existing range error before slot construction. This explicit defensive
boundary does not claim parity for the legacy undefined-input case.

Zero family flags alone do not prove that caster modifiers are irrelevant.
SpellInfo.cpp:1784–1813 supports both class-mask and label modifiers.
Known IGNORE_CASTER_MODIFIERS, or known zero family flags together with known
empty resolved labels, can prove no applicable modifier without a second
runtime modifier store. SpellMgr.cpp:2555–2556,2649–2650 assigns labels at zero
and inherits them along the metadata chain. The current file diagnostic does
not establish label absence. Needed unknown facts remain unavailable; this is
a pending selected-consumer contract, not a general spell-modifier engine.

9g escrito y revisado sobre `a6da440dac37aa377c23c71fb084accff3fa1764`
más delta local: máscaras normalizadas con lookup SQL original, efectos
activos, CastId antes del skip vacío y sustitución del mismo spell antes de
CanBeSaved. Load usa TryCreate incluso con caster/máscara iguales; el refresh
de cast propio conserva identidad. Wisp recalc activo recalcula desde base
y limpia su bit; inactivo conserva el amount guardado. Cargas se normalizan
desde el catálogo explícito; los datos seleccionados tienen cero. La fila
de item finito se guarda pero no sobrevive a la carga permanente, como fuente.
No generaliza runtime SingleTarget, custom cannot-save, áreas ni stacking.
La preparación de fixtures quedó privada en setup.rs (raíz 851 líneas).
Feedback aarch64/jobs=1, no aceptación final: `cargo test -p wow-world
--lib ghost_wisp` terminó exit 0, 39 passed/0 failed, en
`/tmp/9g-test-wow-world-ghost-wisp-rerun6.log`. Incluye los dos caminos de
Wisp y rechazo de target de área antes de retirar la aplicación anterior.
El diagnóstico opt-in `wow-data::spell_db2::tests::resurrection_effect_data::
ghost_wisp_spell_data` con RUSTYCORE_RESURRECTION_DATA_DIR instalado terminó
exit 0, un caso real (`/tmp/9g-data-ghost-wisp-load-boundary-items.log`).
Los checks Core/Spell/World en `/tmp/9g-check-*-rerun2.log` terminaron exit 0
antes de la última reparación; no se atribuyen al delta posterior. El filtro
World posterior sí compiló esos consumidores. Se preservan los fallos previos:
primer intento de compilación y rerun1/rerun2 con fixtures/expectativas por
corregir; rerun3 verde rechazado en revisión; rerun4 exit 101 por tupla de
fixture y rerun5 exit 130 interrumpido antes de tests para corregir la
expectativa numérica (base -1 + dieSides 1 = 0). No hay campaña final ni prueba
durable en DB/reinicio/relogin de este candidato.
Inspección adicional, solo lectura (02:14 UTC): la consulta
`SELECT entry, attributes FROM world.spell_custom_attr WHERE entry IN
(8326,20584,15007)` dentro de START TRANSACTION READ ONLY / ROLLBACK no
devuelve filas. SpellMgr.cpp:3026–3043,3278–3280,3325–3329 deriva
CU_AURA_CANNOT_BE_SAVED de ciertos tipos, LeaveWorld y líquidos; los tipos
seleccionados y el diagnóstico DB2 anterior no activan esas ramas. Esto
acota los datos actuales, no implementa el motor general de custom flags ni
garantiza overlays futuros. En construcción, Unit.cpp:3294 fija SingleTarget
desde caster && SpellInfo::IsSingleTarget; SpellInfo.cpp:1861–1868 solo
consulta Attr5 LIMIT_N en esta referencia. Los cambios posteriores de una
aura por otros productores conservan su propio contrato.

Feedback de Ghost/Wisp a las 19:40 UTC, sobre `6117a9dc5` más el delta local
del worker (aarch64, un job; no aceptación del SHA ni campaña final):
`cargo test -p wow-world --lib scenarios_spell_state_27 -- --nocapture`
terminó con 12 passed en
`/tmp/rustycore-unit9b-test-wow-world-spell-state-27-20261024T194007Z-RxgHes.log`.
La primera ejecución conjunta tuvo 9 passed y 3 failed: la publicación perdía
la procedencia visual del aura, la consulta de Glyph of Stars no incluía su
aplicación runtime y el test de login confundía los dos paquetes de velocidad
con el paquete completo de auras. Se corrigieron y se repitió el grupo afectado.
El filtro `reclaim_corpse_in_range_resurrects_half_and_spawns_bones_like_cpp`
pasó un caso, log
`/tmp/rustycore-unit9b-test-wow-world-release-reclaim-20261024T193910Z-e1I5Uf.log`.
Los tres casos de selección/restauración de display de Application pasaron en
`/tmp/rustycore-unit9b-test-wow-world-application-ghost-wisp-20261024T193633Z-lMk7Ew.log`.
La revisión posterior detectó que el insertor genérico siempre elige un slot
libre: falta cerrar la renovación de Ghost/Wisp del mismo caster sin duplicar
la aplicación, conforme a `Unit.cpp:3224–3288` y `SpellAuras.cpp:1021–1096`.
La publicación delta/removal a observadores queda con la integración de
visibilidad; los tests de carga con recalc cero no prueban el guardado real:
la continuación debe cubrir Ghost recalc 7 y Wisp recalc 3, duración SQL -1,
en la transacción existente de PlayerSave y en su loader productivo.
El handoff de las 19:50 confirma exit 0 para esos filtros y los checks de
Application, Spell y World; el recast no se implementó en este checkpoint.
`ghost_wisp_spell_data` pasó un diagnóstico adicional en
`/tmp/rustycore-unit9b-ghost-wisp-data-20261024T194400Z-aOe5Fc.log`:
SpellAuraOptions (TableHash `0xF42FC065`) no selecciona filas para ambos
spells en dificultad 0; la consulta SQL de solo lectura por sus SpellIDs
en `hotfixes.spell_aura_options` devolvió COUNT 0 a las 19:45 UTC.
Esto respalda stack máximo por defecto, sin sustituir la selección de
catálogos del código. La renovación debe preservar slot/procedencia y no
reaplicar ghost/transform (`SpellAuraEffects.cpp:923–964,1718,1935`).
Checkpoint local de esos participantes: `09451333d`, marcado UNVALIDATED.
La revisión del consumidor encuentra además que
`handlers/entities/corpse/host.rs::remove_auras_due_to_spell_like_cpp`
descarta errores de eliminación y devuelve el número de slots intentados.
Con las validaciones de display de Wisp, `resurrect_player_native` podría
pasar a Alive conservando un aura cuya eliminación falló. La reparación de
los fences de resurrección debe propagar ese fallo antes de Alive,
visibilidad, bones y guardado, con un caso negativo de catálogo incompleto.
No se presenta el feedback positivo anterior como cobertura de ese fallo.
La inspección de `resurrection_request::process_after_teleport` añade que
`ResurrectPlayerInFlight` todavía permite volver a llamar al tramo nativo:
conservar ese nombre de fase no garantiza por sí solo no repetir efectos.
El cierre debe distinguir preparación de ejecución iniciada y probar que un
fallo parcial no repite publicaciones ni avanza a bones/guardado al reentrar.
La misma revisión fija el orden de retirada seleccionada: C++
`Unit::_UnapplyAura` llama `AuraApplication::_Remove` (AuraUpdate de retirada)
antes de deshacer los efectos (`Unit.cpp:3479–3487`, `SpellAuras.cpp:100–107`).
Application emitía Ghost/Wisp después de los cambios de velocidad; el arreglo
seleccionado publicará una sola retirada tras la eliminación inicial validada
y antes de sus efectos. No se extiende ese cambio a todo el motor de auras.
9e implementa ese cambio y propaga los errores de retirada. La consulta de
orden/completitud pertenece a AuraSubsystem; el adaptador World elimina los
slots confirmados. Las continuaciones de request y teleport-revive distinguen
Ready de InFlight y no repiten el tramo nativo iniciado tras un fallo.
Feedback sobre `41674dddb` más el delta local, aarch64/jobs=1, 2026-10-25:
`cargo test -p wow-world --lib ghost_wisp::resurrection` pasó cuatro casos,
exit 0, log `/tmp/rustycore-9e-world-ghost-wisp-resurrection-final.log`;
`resurrect_response` pasó otros cuatro, exit 0, log
`/tmp/rustycore-9e-world-resurrect-response.log`.
Los checks de wow-entities y wow-world-entities sin features por defecto,
Application y World terminaron exit 0; logs `/tmp/rustycore-9e-*-check*.log`.
El caso positivo nuevo falló dos veces por max_health=0 en el fixture;
se inicializó a 1.000 y se repitió todo el grupo afectado, incluyendo
aserciones de DeathState canónico en los casos de fallo. No se rebajó la
aserción positiva. Esta evidencia no cubre aún las operaciones completas
reclaim/healer/login ni DB/reinicio/relogin o aceptación final.
The pending SelfRes participant must retain one ordered list. C++ appends
Soulstone 3026, Twisting Nether 23700 and Reincarnation 21169 in that order
(`Player.cpp:24757–24780`); Add permits duplicates and Remove erases the first
match (`Player.h:2738–2745`). Move authority from the current BTreeSet to
Player's ActivePlayerData SelfResSpells dynamic field (bit 12, parent 0),
with named Player operations and its nested change mask. Do not retain a
second mutable list in PlayerResurrectionState. Clear marks the field even
when empty. UpdateField.h:186–205 shifts values on removal and marks shifted
indices; clear empties both values and nested mask. Initialization checks
HasSpell(20608) and cooldown 21169 before death removes auras. This remains
an initialization obligation. The SelfRes field integration checkpoint above
implements the ordered list contract and its wire consumers; it does not
complete death-time initialization or the ordinary cast operation.

The 2026-10-26 consumer review at `31e8f06d0` also identifies both wire paths:
entity_update_bridge/state_2.rs must project the native list and nested mask
for VALUES, and PlayerCreateData/state_2.rs must write its real size and
ordered payload for self CREATE. At that review CREATE hardcoded size zero. Connect
both login_sequence and worldport self_create to the same live list; never
initialize SelfRes merely because login loads a dead Player. Pinned
UpdateFields.cpp:3042,3096–3098,3327–3329,3476–3480 and
UpdateFields.h:637 establish the layouts. SpellHandler.cpp:416–429 first
requires membership, resolvable SpellInfo, and no preventing aura unless
Attr7 permits bypass. It then calls ordinary CastSpell and removes the first
match after the attempt, including a failed cast. Rust currently removes
only after success and omits that aura admission check. These remain required
consumer fixes alongside ordinary cast preparation and reagents; correcting
the list alone must not be reported as complete SelfRes behavior.

Renovación Ghost/Wisp, 20:08 UTC, sobre `09451333d` más el delta local:
se reutilizan slot, procedencia y aplicación canónica del mismo spell/caster,
incluso sin slots libres. Se recalculan los tres amounts, sus índices y los
basepoints de metadatos cargados; se preservan otras fuentes y no se reaplican
ghost/transform ni se emite velocidad cuando no cambia su valor. La lista de
efectos por tipo refleja la nueva prioridad de registro de C++
(`Unit.cpp:3567–3580`), sin usar otro reloj ni una copia mutable de autoridad.
Los tests nuevos viven en `scenarios_spell_state_27/ghost_wisp/recast.rs`.
`cargo check -p wow-world-application` pasó en 12.61 s, log
`/tmp/rustycore-unit9b2-check-wow-world-application-20261024T195959Z-K4bOWd.log`.
El grupo de hechizos ejecutó 13 passed y 1 failed en
`/tmp/rustycore-unit9b2-test-scenarios-spell-state-27-20261024T200320Z-EtJsDs.log`;
el fallo comparaba toda la aplicación de Wisp, incluidos los amounts que el
refresh debe renovar. Corregida esa expectativa, solo su caso afectado pasó
en `/tmp/rustycore-unit9b2-test-wisp-recast-20261024T200622Z-Kzesgz.log`
(1 passed; compilación 1m14s). No se presenta como un run único de 14 verdes.
El intento intermedio `200523Z-VGmTdK` fue interrumpido al acotar el filtro:
no aporta evidencia. Los fallos previos de inferencia y montaje del submódulo
también se repararon; ninguna de estas ejecuciones es aceptación final.

Máscaras de criaturas, 2026-10-24, 20:27 UTC: el delta sobre `2fbb6e587`
aplica `VISIBLE_TO_GHOSTS` antes del addon y después la prioridad de las
NPC flags efectivas de sanador/área o `GHOST_VISIBILITY`, tanto en creación
nativa como en registro compatible. Anclas C++ del mismo `a5f8da2e`:
`Creature.cpp:1104–1106,1148–1152`. La creación nativa recibe las flags de
la dificultad en `world-server/src/creature_loaded_grid.rs:524`; el
materializador compatible ahora conserva esa selección y su fallback.
Los tests cubren criaturas normales, visibles en ambos estados, sanadores,
flag extra y su prioridad; el caso de materialización recorre la función real
mediante un adaptador disponible solo con fixtures. No se amplía su API de
producción. Las consultas de visibilidad de Players/cadáver y la publicación
de auras a observadores siguen pendientes; estas máscaras no las sustituyen.
Feedback del worker (aarch64, un job, no aceptación final): check
`wow-world-entities --no-default-features`, exit 0, log
`/tmp/rustycore-unit9c1-check-wow-world-entities-20261024T202316Z.log`;
filtro `ghost_visibility` de wow-entities, 1 passed, log
`/tmp/rustycore-unit9c1-test-wow-entities-ghost-visibility-20261024T202323Z.log`;
el mismo filtro de `wow-world --lib`, 2 passed, log
`/tmp/rustycore-unit9c1-test-wow-world-ghost-visibility-20261024T202513Z.log`.
Ambos tests salieron 0. Los intentos `202232Z` y `202345Z` fallaron al
compilar fixtures (GUID y referencias) y se corrigieron. Tras el feedback
se restauró el `type_flags = 0x20` del fixture general: la tabla nueva
sobrescribe ese campo en cada caso; no se atribuye a la corrida un SHA posterior.

Predicado de visibilidad, 20:35 UTC, sobre `089a606ae` más el delta local:
`UnitVisibilityContext` aporta resultados efímeros de distancia, cadáver y
grupo; la distancia se comprueba antes de GM y las excepciones solo afectan
al filtro ghost. La de cadáver exige un Player observador y la de grupo
exige dos Players. Se retiró el campo mutable de grupo, cuyo único productor
era un test. Se conserva el orden de fase, privacidad, condiciones, despawn,
invisibilidad y sigilo (`Object.cpp:1517–1627`). Feedback focal del worker:
6 passed en `unit::tests::visibility_context` y 1 passed en el test existente
`can_see_or_detect_unit_like_cpp_applies_ghost_despawn_and_always_detectable_gates`,
ambos exit 0. Logs `/tmp/rustycore-9c2a-visibility-context-20261024TIfvuo5.log`
y `/tmp/rustycore-9c2a-existing-visibility-20261024TqnlmEM.log`.
En el checkpoint 9c2a todavía faltaba el cálculo desde Map/Player/cadáver.
9c2b lo conecta a la consulta productiva de criaturas, con el feedback descrito
arriba; 9c2c añade Player/grupos. Sigue pendiente la publicación a observadores
y no hay aceptación final de visibilidad.
La revisión de los participantes de muerte añade dos consumidores a cerrar:
`canonical_access/player_stats.rs::sync_canonical_player_health_like_cpp`
cambia el estado de muerte al recalcular salud, aunque `Unit.cpp:9214–9266`
lo conserva; esto puede adelantar JustDied a Corpse durante la retirada de
auras. El setter nativo tampoco aplica aún la salud 1 del Player en DEAD.
La reparación debe distinguir carga, estadísticas y resurrección, y conservar
el valor canónico al publicar. Además, el camino existente de
`login_session/lifecycle.rs::handle_hearth_and_resurrect` solo repone salud:
`BattleGroundHandler.cpp:703–725` exige BuildPlayerRepop, ResurrectPlayer y
después teleport. Se conectará ese consumidor existente, sin implementar
el administrador completo de Battlefield ni ampliar a todo Stuck.
Ese consumidor debe distinguir el retorno conocido de BuildPlayerRepop por
cadáver ya presente (`Player.cpp:4285`) de un resultado incierto de los awaits:
el C++ continúa con ResurrectPlayer después del primero. No se debe convertir
el booleano actual en una nueva denegación ni continuar tras SQL incierto.
La composición utilizará la continuación existente para toda la operación,
sin instalar propietarios anidados para release y resurrección.
La inicialización de autorresurrección también debe conservar el orden:
`Player.cpp:24757–24784` añade Soulstone, Twisting Nether y Reincarnation
en ese orden, y `Player.h:2738–2745` usa una lista dinámica con eliminación
del primer elemento coincidente. El `BTreeSet` canónico actual alteraría ese
orden al proyectarlo al cliente. El cierre de ese participante utilizará una
sola colección ordenada y la codificación dinámica SelfResSpells existente,
incluida su limpieza al volver a Alive; todavía no está implementado.
Para la limpieza nativa no basta con encolar trabajo de auras hasta el siguiente
pase de Session: el mapa podría capturar VALUES con salud cero y máximos u
otros efectos antiguos. El corte debe conservar la mutación canónica antes
de SendObjectUpdates y diferir solo la entrega fuera del guard. Inventory ya
construye `PlayerStatSystemInput` para un cálculo puro; se estudiará una sola
aplicación común sobre hechos inmutables de estadísticas y catálogos acotados,
usada tanto desde Session como desde el mapa. No se ha aprobado aún su corte
de funciones ni implementado otra copia del cálculo. Las listas canónicas
`AuraSubsystem.applied_auras/owned_auras` son Vec y conservan la inserción;
permiten el orden estable por SpellID de `Unit.cpp:3350,4308` sin otro contador.

Contraste de datos del productor, 2026-10-24: el diagnóstico opt-in mantenido
`wow-data::spell_db2::tests::resurrection_effect_data` leyó el DataDir instalado
`/home/server/woltk-server-core/Data`, locale enUS, con lectores productivos.
`SpellEffect.db2` SHA-256
`c786e3f82d69f6a2675cdcc7a0a6c48a2684044fb23344bd488425d982b525f8`:
50 filas (18: 11; 113: 39; 172: 0), sin términos no cero de nivel, recursos,
coeficiente o varianza. Las consultas SQL de solo lectura por los 50 SpellIDs
y por tipos 18/113/172 no encontraron reemplazos en `hotfixes.spell_effect`
ni `world.serverside_spell_effect`, ni estados de hotfix para sus RecordIDs
y TableHash `0xF04238A5`. Las 50 filas correspondientes de SpellMisc tampoco
tienen overlays ni estados de hotfix (TableHash `0xC603EE28`). Ninguna activa
IGNORE_CASTER_MODIFIERS: esto NO exime el cálculo de modificadores del caster
(`SpellInfo.cpp:1784–1816`). El motor general de modificadores y scripts de clase
sigue en #32/#34; esta evidencia solo delimita la aritmética básica con estos
datos, no declara paridad completa de `CalcValue` ni cubre otros datasets.
Sobre base `5a8d500f3` más el delta de tests, el worker ejecutó
`cargo test -p wow-data resurrection_effect_data -- --ignored --nocapture`
con `RUSTYCORE_RESURRECTION_DATA_DIR` (1 passed) y
`cargo test -p wow-world --lib scenarios_spell_state_27 -- --nocapture`
(7 passed, incluido efecto 113: base 31 + dado 1 → salud 32, maná 17).
Ambos salieron 0; también los checks de wow-data y wow-world. Logs:
`/tmp/rustycore-unit6-resurrection-effect-data-20261024T155438186674823.log`
y `/tmp/rustycore-unit6-spell-state-27-20261024T155512887112188.log`.
Es feedback de implementación en aarch64, no aceptación final del candidato.

Avance local, 14:41 UTC: el codec `ResurrectRequest` reproduce
`SpellPackets.h:825–839` y `SpellPackets.cpp:852–866` del mismo C++ fijado:
GUID empaquetado, realm virtual, pet number, spell ID y longitud UTF-8 de once
bits con flags de timer/sickness. `cargo test -p wow-packet resurrect_request`
ejecutó tres tests (exit 0, 42.34 s de Cargo); `cargo check -p wow-packet`
terminó con exit 0 (20.62 s). Es feedback del worker sobre los archivos locales
posteriores a `89645b0e6`, no evidencia de aceptación de ese SHA ni captura viva.
El productor desde hechizos sigue pendiente. La revisión de muerte identificó
además que el daño de spell-click usa directamente `Corpse`; deberá entrar por
la transición `JustDied` con los participantes existentes, en lugar de eludir
KillPlayer y su timer. No se da esa corrección por implementada.

Revisión del productor de ofertas, 15:04 UTC: `afa4681c8` conserva el codec
como checkpoint **UNVALIDATED**; la integración de efectos 18/113/172 está en
curso. `EffectResurrect`/`EffectResurrectNew` resuelven el dueño de un cadáver
con `ObjectAccessor::FindPlayer`, aunque ya esté en otro mapa; 172 requiere
un Player como unitTarget. La admisión se comprueba en el Player canónico y
el envío queda ligado a la generación del registro y al mismo PlayerHandle.
El locale pertenece a la identidad inmutable de la conexión; una consulta
acotada evita copiarlo en cada snapshot espacial. El nombre localizado usa
solo una traducción no vacía; sin ella conserva el nombre real de la criatura
(`Creature.cpp:3109–3118`), también para mascotas con nombre propio. Los
catálogos siguen siendo de proceso, sin nuevos campos de catálogo en Session.
El cálculo provisional del productor usa el helper existente de base/dados;
no prueba el `CalcValue` con caster de `SpellInfo.cpp:495–560`. El modelo DB2
conserva escalado/varianza/recursos que la conversión al efecto runtime omite.
La consulta de solo lectura a `hotfixes.spell_effect` y
`world.serverside_spell_effect` no encontró filas de esos tres efectos; esto
no sustituye inspeccionar las filas efectivas procedentes de DB2.

La revisión de participantes corrige una premisa de los comentarios actuales:
hay flags canónicos del Player/Object, identidad de reclutamiento y un helper
productivo de efectos de objetos adquiridos. Sobre `97d4d45c3` más el delta local
se conectaron limpiar OUT_OF_BOUNDS (0x4000), restaurar REFER_A_FRIEND (0x80)
cuando corresponde y limpiar la lista canónica de autorresurrección al pasar
de muerto a vivo (`Player.cpp:4320–4356,1158–1198`). Movimiento bajo el mapa y
RepopAtGraveyard escriben/limpian el mismo flag (`MovementHandler.cpp:437–467`,
`Player.cpp:4790`). Check de wow-world-entities: exit 0, 1 s; filtros de
wow-world `spirit_healer`: 11 passed, 114 s; `movement_under_map`: 2 passed,
1 s; `repop_request_creates_saved_corpse_and_releases_to_graveyard_like_cpp`:
1 passed, menos de 1 s. Logs `/tmp/rustycore-u7-*-20261024T161*.log`;
el primer wrapper falló con exit 127 antes de Cargo por ausencia de
`/usr/bin/time`; se corrigió el wrapper. Es feedback, no aceptación final.
La lista SelfResSpells carece aún de proyección productiva hacia el campo
dinámico del paquete: el test prueba el estado de admisión canónico, no la UI.
Falta `CastAllObtainSpells` sobre mochila y contenido de bolsas
(`Player.cpp:8034–8076`), además de los participantes de muerte pendientes.
El tratamiento de fallos de release/reclaim/healer/login reutilizará la
continuación canónica existente: un resultado SQL incierto o un await cancelado
no permite repetir efectos ni capturar un guardado completo. C++ encola
`Corpse::SaveToDB` (`Corpse.cpp:104–146`); la espera de confirmación del puerto
Rust y su retención ante incertidumbre son un contrato explícito de durabilidad
asíncrona, no una afirmación de que C++ espere ese mismo resultado.
Decisión para el siguiente bloque de release: `BuildPlayerRepop` devolverá un
resultado explícito, y sus callers de petición/timer no continuarán al
cementerio tras fallo. La continuación del Player conservará las fases de
auras/creación pendientes antes de suspenderse; cancelar no autoriza replay.
El cadáver recién construido y su solicitud SQL serán valores locales de esa
operación hasta confirmar el guardado, sin un guard de Map durante el await.
La publicación en Map y `_corpseLocation` quedará después de la confirmación,
con nueva comprobación de encarnación/residencia. Es una adaptación explícita
de la frontera asíncrona: C++ registra con `AddCorpse`, encola SaveToDB y llama
`AddToMap` sin esperar SQL (`Player.cpp:4288–4293,4493–4505`). No se atribuye a
C++ un commit-before-publication que no tiene. Un fallo/cancelación conserva
el bloqueo del guardado completo y la retención del dueño, sin reintentar una
escritura de resultado desconocido. El estado ghost completo es la frontera
de salida de BuildPlayerRepop; el teleport posterior conserva su propio
contrato. El bloque está escrito en `player_release.rs` sobre `8be267512`,
con feedback focalizado y pendiente de aceptación final. Reutiliza
la continuación del Player para release/creación, rechaza ACKs de teleport
para esas operaciones y conserva la identidad y revisión de residencia al
publicar. El helper independiente de creación conserva el guardado posterior
a huesos antes de construir el reemplazo. Los callers de petición y timer
solo continúan al cementerio cuando BuildPlayerRepop confirma el resultado.
El primer check encontró un préstamo temporal y el primer intento de compilar
tests un getter inexistente; ambos fueron reparados. La primera ejecución de
los siete casos nuevos produjo 1 passed/6 failed: el fixture carecía de
SpellInfo para Ghost y se detenía antes de SQL. Se completó ese fixture sin
debilitar el guard del cast. El caso de reemplazo requirió además preparar
la salud máxima después de crear/instalar el Player; se corrigió un getter
inexistente durante esa reparación. El resultado final del filtro
`cargo test -p wow-world --lib release_failure -- --nocapture` fue **8 passed**,
exit 0, en `/tmp/rustycore-unit8-release-failure-repair3-20261024T165552Z.log`.
Incluye identidades distintas para revisión de residencia y nueva encarnación,
cancelación antes de commit, ausencia/fallo del puerto, petición duplicada,
fallo de timer y precondiciones de huesos/cadáver. El check de
`wow-world-entities` pasó tras reparar el préstamo. Los filtros positivos de
release manual, timer de seis minutos y creación/guardado tras worldport dieron
1 passed cada uno, exit 0, logs `/tmp/rustycore-unit8-request-success-20261024T165800Z.log`,
`/tmp/rustycore-unit8-timer-success-20261024T165802Z.log` y
`/tmp/rustycore-unit8-standalone-corpse-success-20261024T165806Z.log`.
Todos son feedback del delta de implementación en aarch64, jobs=1, no evidencia
de aceptación final de un SHA posterior ni prueba de durabilidad con DB real.

La revisión previa de ghost encontró que el flag Player no conectaba las
máscaras Unit de visibilidad/detección. `09451333d` conecta ese participante de
`HandleAuraGhost` (`SpellAuraEffects.cpp:1718–1743`), con renovación posterior
en `2fbb6e587`; ambos siguen sin aceptación final. Están en implementación
las máscaras de criaturas visibles a ghosts/spirit healers
(`Creature.cpp:1104–1106,1148–1152`). La consulta de
visibilidad de otros Players filtra fase, sin el contrato ghost completo.
Se conservarán las excepciones de cadáver cercano y grupo de
`Object.cpp:1517–1595`, sin introducir estado mutable por observador.
La excepción de grupo debe usar `Player.cpp:2121–2143`:
`Visibility.GroupMode` (default 1 en `World.cpp:1167`) distingue subgrupo,
raid, equipo y ninguna excepción; el helper Rust actual de pertenencia a grupo
no demuestra por sí solo ese contrato.
La ruta genérica usaba 30 segundos y omitía parte de los grupos multiefecto.
La ruta seleccionada 8326/20584 ya aplica sus tres efectos permanentes y
conserva su procedencia, con el contraste DB2 y feedback registrados arriba;
faltan la publicación a observadores, persistencia y aceptación integrada.
Un retorno Ok del cast no demuestra por sí solo esos contratos. Esto no exige
portar todas las auras de #32, pero sí las necesarias para el ciclo de #43.
El diagnóstico opt-in `wow-data::spell_db2::tests::resurrection_effect_data::ghost_wisp_spell_data`
leyó los cinco DB2 instalados en enUS sobre `7c8e01858` más el delta de test:
1 passed, exit 0; log `/tmp/rustycore-unit9a-ghost-wisp-data-20261024T170231Z.log`.
8326 tiene efectos APPLY_AURA 95/31/58 (RecordIDs 686362–686364, índices 0–2,
base/dado -21/1, 49/1, 49/1); 20584 tiene 31/58/56 (697054–697056,
74/1, 74/1, -1/1), transform MiscValue=12861. Ambas usan dificultad 0,
target self y DurationIndex 21: duración base/máxima -1, permanente. Misc IDs
320590/329372; Ghost tiene XSpellVisual 241720 → visual 886 sin misil;
Wisp no tiene relación visual. No se presume paridad completa de modificadores
del caster a partir de estos valores base. Consultas SQL de solo lectura no
encontraron overlays de efectos/Misc por ambos SpellIDs, ni Duration 21,
XSpellVisual por ambos IDs/241720 o Visual 886; tampoco estados de hotfix por
los RecordIDs y sus TableHash F04238A5/C603EE28/B423FA7A/27B7A01A/F72496D9.
`world.serverside_spell_effect` tampoco tiene filas por ambos SpellIDs.
El template SQL 12861 existe y tiene un único modelo: DisplayID 1825,
escala 1, probabilidad 100. Esta lectura no ejecutó casts ni QA viva.
Decisión del bloque de implementación de auras: una aplicación por hechizo,
con todos sus efectos activos, duración permanente y procedencia real; se
reutilizarán los registros de efectos del AuraSubsystem y los consumidores de
movimiento existentes. La retirada de Ghost conservará sus máscaras mientras
quede otra aura de tipo 95. La restauración de display debe respetar
`Unit.cpp:10038–10098,11763–11870`, no forzar siempre el modelo nativo.
Falta confirmar con el lector productivo la ausencia de enlaces de modelos
de personalización: el header WDC4 de `ChrCustomizationDisplayInfo.db2`
declara cero registros (92 bytes), y la consulta SQL de su tabla de hotfixes
cuenta cero filas. La lectura del header no sustituye ese diagnóstico ni
prueba el soporte del selector para otros datasets. Las consultas de
visibilidad y máscaras de criaturas serán el bloque siguiente; no hay
activación de runtime de un estado intermedio.
La revisión de guardado encuentra además que el snapshot/adapter actual de
auras solo escribe la sickness seleccionada; aún no escribe 8326/20584.
El diagnóstico del worker de 18:12 UTC encontró cero filas SpellInterrupts
para ambos SpellIDs (TableHash 0x668FAE03); una consulta SQL de solo lectura
posterior tampoco encontró overlays por esos IDs. `Aura::CanBeSaved`
(`SpellAuras.cpp:1134`, `SpellMgr.cpp:3280`, exclusión LeaveWorld) permite
estas auras permanentes autocasteadas sin item. Su guardado y retirada deben integrarse en la
transacción existente con claves acotadas, preservando las demás auras aún no
portadas; cargar filas preparadas en un test no prueba ese productor de guardado.
Esto coexiste con el fallback explícito de `CharacterHandler.cpp:1227–1243`:
al entrar muerto, C++ recastea Wisp para Night Elf y Ghost solo cuando faltan.
El fallback ya tiene consumidor Rust en `corpse_login.rs`; no sustituye el
guardado ni permite duplicar las aplicaciones que sí se cargaron.
El feedback `cargo test -p wow-data resurrection_effect_data -- --ignored
--nocapture` ejecutó dos tests, exit 0, sobre `9f24f3832` más delta local;
log `/tmp/rustycore-unit9b-ghost-wisp-db2-20261024T181243Z.A3Esbf.log`.
El primer `cargo check -p wow-world-application` falló (exit 101) por dos
llamadas a un mutador de auras limitado a fixtures; log
`/tmp/rustycore-unit9b-check-wow-world-application-20261024T181340Z.ykKShG.log`.
La reparación usa operaciones canónicas acotadas de la cola existente; no
convierte el mutador genérico de fixtures en API de producción. Las pruebas
de aplicación y restauración siguen pendientes; no es aceptación final.
La revisión del delta detectó dos expectativas incorrectas antes de ejecutar
sus tests: `SpellInfo.cpp:4863` clasifica el efecto GHOST como negativo
(autocast: NOCASTER|NEGATIVE, no CANCELABLE), y `Unit.cpp:10038–10098`
restaura el display nativo si GetModelForForm devuelve cero, incluso con
Glyph of Stars. `SpellAuraEffects.cpp:2132` pasa IsMounted a RestoreDisplayId;
no permite sustituirlo por un booleano de fallback. Las correcciones pertenecen
al mismo bloque. La consulta de solo lectura de `world.spell_script_names`
por 8326/20584 y sus claves negativas no encontró scripts asociados.

La comprobación del productor sin features de fixtures detectó un defecto del
propio candidato #43 en `login_sequence.rs`: el código de LoadCorpse accedía a
`SessionCatalogs.graveyard_store`, disponible solo con fixtures. Los checks
anteriores con `--tests` no prueban esa composición de producción. La reparación
seleccionada es pasar `ResurrectionOperationCatalogs` a la restauración del
cadáver, que no utiliza cementerios, y retirar el catálogo vacío de fallback;
no ampliar la visibilidad de un campo de pruebas. Se debe repetir el check sin
fixtures y ejecutar las pruebas afectadas antes de aceptar la corrección.

Feedback del productor, 15:41 UTC: sobre `afa4681c8` más su delta local,
`cargo check -p wow-world-spell` y `cargo check -p wow-world
--no-default-features` terminaron con exit 0 (el segundo, 17.13 s).
`cargo test -p wow-data localized_name` pasó un test; `cargo test -p wow-world
--lib scenarios_spell_state_27 -- --nocapture` pasó los seis tests nuevos
(1 min 09 s de Cargo). `cargo check -p world-server --no-default-features`
pasó en 4 min 13 s. Las seis pruebas ejecutan el productor e incluyen dueño
de cadáver en otro mapa, estado JustDied, locale/pet/realm, trigger ausente o
aura ya presente, duplicados y encarnación obsoleta. El check sin fixtures
confirma la reparación de LoadCorpse; sus tests específicos no se ejecutaron
en esta unidad. El primer filtro `resurrect` obtuvo 25 passes existentes y
cinco fallos de preparación canónica; no seleccionaba el test de duplicados.
Se corrigieron los fixtures usando install/attach reales y se ejecutó el módulo
completo. Se conservan los logs de fallo y reparación separados bajo
`/tmp/rustycore-spirit-offer-*`; el run verde del módulo es
`wow-world-scenarios-27-tests-20261024T153316065477947.log` y el de producción
`world-server-no-default-check-20261024T153440584980239.log`. Esto sigue siendo
feedback de implementación, no campaña final ni prueba viva. La traza del worker
`01a16e58-4407-7612-b4cc-1f8c17babadc` confirma `gpt-6-luna`/`max`.

Avance local a las 12:46 UTC: movimiento y timer escritos, **sin compilar ni
ejecutar todavía sus pruebas**. Worker `01a16ddf-f5d4-7702-a6f2-dec78c3d3767`:
la traza efectiva confirma `gpt-6-luna`/`max` en sus tres turnos. Se reutilizan
el contador y la publicación de movimiento existentes; water-walk es
idempotente, ResurrectPlayer fuerza land-walk y los guards de logout/stun
conservan root cuando corresponde. Ghost login restablece water-walk después
de sus auras. El timer sustituye el booleano canónico, actualiza RELEASE_TIMER
y no avanza con el Player separado del mundo. Las pruebas nuevas llaman al
driver real con su entrada de paquetes viva; verifican límite, pausa, destino
pendiente y orden realm sin confundir el envío del teleport con su ACK.
La revisión corrigió esas expectativas antes de ejecutar evidencia. Continúa
la operación de resurrección por solicitud; ningún test escrito en este avance
se incluye en el recuento verde anterior.

Revisión de integración, 13:12 UTC: el checkpoint local `d69c0860e` conserva
reconexión, movimiento y timer como **UNVALIDATED**. La continuación de
`ResurrectUsingRequestData` se implementa sobre el mismo Player, antes del primer
await, y distingue retorno normal de TeleportTo, ACK admitido y efecto externo
interrumpido. C++ `Player.cpp:24897–24937` ignora el rechazo de TeleportTo:
no se exige llegar a las coordenadas solicitadas para resucitar cuando la llamada
retorna sin teleport pendiente. Una llamada cancelada antes de resolver ese paso
no puede darse por terminada solo por coincidencia de coordenadas. Los efectos
externos de resultado incierto se retienen; no se autoriza replay ni retiro del
Player como si hubieran terminado. `TELE_REVIVE_AT_TELEPORT` conserva su fase
anterior al envío del teleport cercano (`Player.cpp`, rama de TeleportTo), distinta
de la resurrección por solicitud posterior al ACK. El finalizador exterior recibe
las capacidades concretas existentes; no se almacena otro catálogo en Session.
Esta implementación y sus nuevos tests siguen sin ejecutar aceptación.
La revisión del consumidor far detectó un ciclo entre `post_add` y el guardado
de SpawnCorpseBones: el save esperaba acabar el teleport y el teleport esperaba
ese save. La transferencia debe confirmar zona/escalado/pet y limpiar su
`post_add` antes de continuar la resurrección, cuyo estado canónico sigue
bloqueando la captura incompleta. El guard compartido reside en la admisión y
captura del Player, no en cada consumidor de guardado; solo la fase posterior
al borrado confirmado del cadáver permite preparar su save. Un retorno fallido
no confirmado debe desconectar y conservar la continuación para el finalizador,
en lugar de permitir gameplay con una operación parcial. Son decisiones de la
revisión; sus regresiones todavía no constituyen evidencia ejecutada.

Feedback de implementación, 13:55 UTC: `cargo check -p wow-world --tests`
termina correctamente (intento r9, 40,68 s). Los logs
`/tmp/rustycore-unit3-cargo-test-20261024-*.log` muestran doce casos distintos
verdes: respuesta de resurrección (4), fallo al borrar bones (2), finalización
far con cadáver y cancelación durante save (2), timer/detached (3) y atribución
del aura al lanzador original (1, tras reparar su fixture). La regresión
TELE_REVIVE todavía falla por esperar únicamente CancelCombat/MoveTeleport;
la operación completa publica también DeathReleaseLoc, land-walk, unroot y
valores antes del teleport. Son resultados sobre el árbol de trabajo posterior
a `d69c0860e`, no aceptación final ni evidencia del candidato comprometido.
Quedan reparaciones y propagación de capacidades en logout instantáneo/timer.
La revisión de desconexión near fija otro contrato: C++ LogoutPlayer solo
fuerza transferencias far (`WorldSession.cpp:544–548,602`); el near ejecuta
operaciones diferidas únicamente al ACK (`MovementHandler.cpp:302`), y
SaveToDB conserva su destino pendiente (`Player.cpp:19488–19510`). Por ello,
al finalizar se abandona exclusivamente una solicitud aceptada en
AwaitingTeleport con near pendiente, todavía sin efectos de resurrección;
se conserva el muerto/cadáver y el destino para guardar. No se simula un ACK
ni se descartan fases inciertas de teleport, aura, bones o save. Esa reparación
y sus regresiones siguen pendientes de implementación/evidencia.
La revisión posterior del aura detecta que ese test verde de atribución todavía
usaba un GUID sin Unit: no prueba el contrato C++. `Spell.cpp:536–545,3198–3222`
y `ObjectAccessor.cpp:217–239` requieren resolver el lanzador original en el
mismo mapa y en mundo; si ya no está disponible, el aura usa el lanzador real.
Se corrige el resolver y se exige un caso positivo con dueño canónico real y
el negativo ausente/en otro mapa antes de aceptar esa evidencia.
La revisión de `process_represented_delayed_teleport_after_update_like_cpp`
rechaza quitar su guard IsAlive: `Player.cpp:1151–1155` lo exige precisamente
para no volver a teletransportar un ghost. El nuevo timer se había situado
dentro de un tramo Rust con `can_delay=true`; C++ cierra ese tramo tras
`Unit::Update` (`Player.cpp:935–942`), antes del timer de muerte. Corrección
acotada: cerrar el tramo Rust después del tick de auras, antes de los timers
de save/muerte, y conservar el guard vivo de la operación diferida. No se
presenta este ajuste como migración completa de las fases de #584.
Los dos casos `production_disconnect_near_resurrection_*` pasan en el objetivo
`production_login_player_owner` (log local
`/tmp/rustycore-unit3-cargo-test-20261024-near-finalization.log`): preservan
cadáver, salud del ghost y destino pendiente, y retienen el fence después de
un save Unknown sin segundo envío. El resto de reparaciones sigue en feedback.

Checkpoint local del consumidor, 14:34 UTC: worker detenido tras entregar, sin
Cargo activo; traza efectiva final `gpt-6-luna`/`max`. Se conserva el guard vivo y
se cierra `can_delay` antes del timer. Último check de `wow-world --tests`: exit 0,
22,69 s (`/tmp/rustycore-unit3-cargo-check-20261024-can-delay-alive.log`).
`cargo check -p world-server --tests`: exit 0, 7m37s
(`/tmp/rustycore-unit3-cargo-check-20261024-world-server-tests.log`); compila sus
targets de pruebas, **no los ejecuta**. Los filtros de expiración (1), `delayed_`
(5) y revive-at-teleport (1) pasan, además de los filtros previos de respuesta,
bones fallido, save far/cancelado, timer/detached, atribución original resuelta
y las dos pruebas de integración near. El test nuevo de ghost diferido se
renombró a `update_keeps_delayed_teleport_for_ghost_without_emitting` después de
ejecutar el mismo cuerpo con sufijo legado; no se relabela esa evidencia.
Logs de feedback: `/tmp/rustycore-unit3-cargo-test-20261024-*.log`; el de
revive-at-teleport contiene su última ejecución corregida. No se ejecutó
`validation-v2 final`, ni inventario exhaustivo, ni cliente/DB/runtime; el
checkpoint sigue **UNVALIDATED** para la aceptación completa. Estos tiempos
son de implementación y reparación, no una campaña ordinaria verde de 600 s.

La inspección encontró además que no hay productor de solicitudes integrado en
los efectos de hechizos: los callers del setter eran fixtures/tests. Completar
el consumidor no basta para probar esa ruta en producción. La unidad productora
debe conservar `SpellEffects.cpp:416–441,3599–3626,5328–5357`,
`Spell.cpp:5288–5310` y `Player.cpp:22705–22714`: Player muerto y en mundo,
sin solicitud previa, valores absolutos/porcentuales según efecto, aura opcional,
ubicación del lanzador y publicación `SMSG_RESURRECT_REQUEST` por realm.
La revisión de muerte también mantiene pendiente conectar los participantes
existentes de `Player::setDeathState` y la retirada de auras de
`Unit::setDeathState`; el helper de auras por sí solo no demuestra esa integración.
La guía del bot contiene la matriz de QA de #43 como preparación, sin declarar
un driver de muerte ni capturas ejecutadas. Permanecen reservados el cambio del
binario vivo, las escrituras de cuentas/personajes y DB/reinicio/relogin hasta
disponer del candidato y de autorización concreta para esos objetivos.
La inspección de procesos de solo lectura durante esta preparación encuentra
`bnet-server` y `world-server` Rust, sin un proceso `worldserver` C++ activo.
El directorio `/home/server/trinity-legacy-install/bin` no existe; se conservan
los nombres de configuración en `etc`, cuyo contenido no se ha volcado.
La búsqueda posterior localiza el ejecutable en
`/home/server/woltk-trinity-legacy/build-r6/src/server/worldserver/worldserver`.
Su SHA-256 actual es
`80f143841b5a7b443022480500760ce336dc83052f05ddd7e3b7be6a6605b02a`,
el mismo registrado en [la preparación R6](../architecture/captures/1263-r6-paired-2026-10-10.md).
El checkout sigue en `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` con los tres
parches locales de infraestructura allí documentados (LFGList, instanciaciones
de Spell y guard DB2); no se presenta como binario upstream sin modificaciones.
La evidencia R6 histórica no sustituye capturas de muerte nuevas. No se ha
arrancado, recompilado ni modificado ese runtime de referencia en esta preparación.

**#43, recuperación al reconectar — implementación local y feedback focalizado, 2026-10-24:**
continuación autorizada sobre `d78b90750`, rama `43-spirit-healer`. Alcance:
restaurar la ubicación personal del cadáver durante login y conservar enfermedad
de resurrección y durabilidad al guardar/reconectar. Todavía **sin nueva aceptación**.
La decisión de persistencia conserva el coordinador, captura canónica, transacción
Characters y tratamiento de COMMIT desconocido/cancelación existentes. Se añaden
participantes acotados: enfermedad seleccionada por ChrRaces y durabilidad de
objetos propios; no se sustituye toda `character_aura` con una representación
parcial ni se reconoce como guardado el resto del estado de Item. Una carga
incompleta no autoriza borrados. Anclas revisadas en el C++ objetivo
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player.cpp:19632–19646`
(inventario/auras en SaveToDB), `19811–19865` (_SaveAuras),
`19868–20000` (_SaveInventory), `18034–18145` (_LoadAuras, tiempo desconectado),
`SpellAuras.cpp:1134–1174,1218–1244` (CanBeSaved y clave de efectos).
La durabilidad actualiza filas existentes de item_instance, ligadas al dueño
mediante character_inventory; esta entrega no sustituye la creación ni el
guardado completo de Item. Se captura sin limpiar su estado general Changed/New;
la enfermedad expirada debe retirar únicamente sus propias filas, conservando
las auras ajenas. Participantes adicionales necesarios para la reconexión:
`Player.cpp:19422,19557` guarda GetHealth, incluido el 1 del fantasma; el snapshot
Rust activo lo convertía en 0 por `!is_alive`, causando el override a CORPSE
de `Player.cpp:17776–17778`. Se corrige esa captura, conservando el cero del
cadáver, y se añade `at_login` al mismo guardado para que la limpieza de
AT_LOGIN_RESURRECT no se repita al reconectar (`18157–18178,19543`). El tiempo
de penalización por muertes sucesivas pertenece al mismo Player
(`17676–17679,19545`), con el límite de carga de tres pasos de cinco minutos.
Se mantiene pendiente la prueba real DB/reinicio/relogin:
requiere autorización de runtime, y ninguna prueba local de planes SQL o futuros
simulados se presentará como evidencia de durabilidad real.
La recuperación personal ya está conectada en `wow-world-character` después de
los paquetes post-add y antes de LoggedIn: el puerto auxiliar lee
`SEL_CORPSE_LOCATION` por GUID; `wow-world-entities::corpse_login` consume la fila
local en el Player canónico, restaura ubicación/release timer y ejecuta las ramas
alive/AT_LOGIN_RESURRECT. La carga de cadáveres del mapa conserva su operación.
`wow-world-application::continue_login` restaura DEAD desde Ghost y después
CORPSE si la salud guardada es cero (`Player.cpp:17717–17723,17776–17778`).
Las auras 20584/8326 se recuperan independientemente según
`CharacterHandler.cpp:1227–1241`; se reconoce tanto la aplicación runtime como
el registro de aura del mismo Unit para no duplicar una ya cargada. Sigue fuera
de esta parte `SetWaterWalking(true)` y la aceptación completa de movimiento
fantasma. Las coordenadas incompletas/inválidas y mapas desconocidos no crean
ubicación; consulta vacía y fallo se distinguen en el adaptador del flujo.
Feedback de implementación del worker Luna/max (ruta efectiva verificada):
`cargo check -p wow-world-entities` y `cargo check -p wow-world --tests`, con
PROTOC local, un job y target de este checkout; compilación verde tras corregir
los hallazgos. La última comprobación de wow-world terminó el 2026-10-24
10:52:30 UTC (13,93 s). **En ese corte no se había ejecutado ningún test ni se
reclamaba aceptación nueva**. Este feedback corresponde solo al delta de
reconexión anterior a los participantes de guardado, aún no implementados en
ese corte; no cubre los cambios posteriores. El feedback posterior figura abajo.
Decisión de representación para la enfermedad: conservar en el subsistema de
auras del Unit los metadatos de persistencia que antes se descartaban al cargar
(GUID/identidad del objeto lanzador, máscara de recálculo, cargas y cantidades
base por efecto). Su vida queda ligada al slot de la aplicación, como la
procedencia de lanzamiento existente; sustituir, retirar o limpiar la aplicación
retira esos metadatos. No se duplican duración, stacks ni cantidades actuales.
Reconstruir siempre la cantidad base desde el catálogo perdería el valor leído
de DB tras cambios de datos; extender la escritura a todas las auras borraría
familias aún no representadas. Por ello la escritura queda acotada a la
enfermedad propia seleccionada por raza, sin objeto lanzador, y las otras filas
se conservan. La lectura conserva sus metadatos completos para ese límite.
La revisión de la carga identifica dos reparaciones necesarias para ese contrato:
el flujo pasaba cero como tiempo desconectado y trataba todas las auras como
positivas; además publicaba una actualización anticipada con remaining=max.
El flujo ahora descuenta el tiempo desconectado de la enfermedad negativa,
aplica sus efectos cargados antes del cálculo inicial de estadísticas y
publica su duración restante e identidad de lanzamiento en el AuraUpdate
completo post-add existente. Anclas: `Player.cpp:17640,17718,18034–18145,
23568–23635,24015–24035`; `SpellInfo.cpp:4725–4775` para los efectos negativos
79/80. La clave de carga conserva caster/item/spell/effectMask, sin convertir
un caster vacío en el jugador ni mezclar efectos de distintos objetos.
Una fila incompleta, GUID binario inválido o aplicación no admitida deja
incompleta la autoridad de persistencia; la expiración offline legítima sí
permite retirar la familia propia en el siguiente guardado.
`Aura::SetLoadedState` también recalcula los efectos cuyo bit está activo
(`SpellAuras.cpp:1239–1257`, `SpellAuraEffects.h:74–75` y
`SpellAuraEffects.cpp:659–725`). Para los efectos constantes 79/80/159 de la
enfermedad se usa la base persistida, DieSides y stacks; un bit desactivado
conserva amount. Esta parte no constituye un port completo de CalcValue,
escalado ni scripts de auras arbitrarias.
La aceptación pendiente de esta entrega debe incluir los escenarios de cadáver
personal, proyección de guardado y fallos/COMMIT desconocido, SQL y orden del
plan, carga/expiración/publicación del aura y consumidores de login. Los nuevos
accesos SQL requieren revisar el delta del inventario de persistencia en esa
campaña; no se ha regenerado su baseline ni ejecutado otra campaña final.
Feedback posterior de implementación: `cargo check -p wow-world --tests`
terminó con exit 0 el 2026-10-24 a las 11:55:19 UTC (inicio 11:53:40 UTC,
99 s), sobre el árbol de trabajo modificado en HEAD `d78b90750`, con los
participantes de guardado y la carga de auras ya escritos. Registro:
`target/validation-v2/evidence/43-reconnect-wow-world-check.{meta,stderr.log}`.
Este check no ejecuta tests ni constituye aceptación final. El feedback del
worker terminó con **44 identidades de test distintas verdes**: wow-world,
`sickness_loading::` (5), `save_snapshot_owner::` (15), `corpse_login::` (7)
y `save_interleaving::` (11); wow-database, `resurrection_sickness` (2),
`character_save_adapter_preserves_the_frozen_statement_order_like_cpp` (1)
y `full_save_death_metadata_and_item_durability_use_exact_owner_scoped_binds`
(1); wow-entities,
`aura_cast_provenance_is_slot_scoped_and_cleared_on_replacement_like_cpp` (1);
wow-data,
`represented_spell_positivity_marks_resurrection_sickness_modifiers_negative`
(1). Comandos: `cargo test -p <crate> --lib <filtro> -- --nocapture`, salvo
el primer filtro sickness_loading, ejecutado sin `--lib`: sus dos objetivos
de integración tuvieron cero casos ejecutados y **no aportan evidencia**.
Los registros exactos son `target/validation-v2/evidence/43-reconnect-*.meta`,
con stdout/stderr asociados. Se conservaron los primeros fallos: dos fixtures
de guardado y dos de cadáver; se corrigieron la salud máxima inicial y el uso
del oráculo antiguo, consultando el propietario real de guardado. Los reruns
`wow-world-save-snapshot-owner-retry1` y `wow-world-corpse-login-retry1` pasan
sin rebajar aserciones ni cambiar producción para acomodar las fixtures.
La prueba nueva `handlers/character_tests/sickness_loading.rs` incluye
carga→efectos canónicos→snapshot de guardado y paquete completo; expiración
genera Some(grupo vacío), y una clave malformada conserva None para esa familia.

Coste separado de desarrollo: ventana 11:53:40–12:11:30 UTC (1.070 s, incluidos
revisión/reparación y pausas); suma registrada de comandos 872,57 s, incluidas
compilaciones y primeros fallos. **Supera diez minutos** y no acredita el
objetivo de rendimiento de una futura aceptación final. Todo corresponde al
árbol modificado sobre `d78b90750`, con ajustes posteriores de fixtures; no se
ha creado un nuevo commit, manifiesto final ni evidencia de publicación.
Siguen pendientes la campaña final, su inventario de persistencia y la prueba
real de DB/reinicio/relogin. No hubo push, despliegue ni cambios de runtime;
**#43 permanece abierta**, incluidos el movimiento fantasma
(`SetWaterWalking(true)`) y la limpieza general de auras al morir
(`Player.cpp:17780`, `RemoveAllAurasOnDeath`).

**#43, sanador de espíritus — aceptación local verde, 2026-10-24:** continuación de
las partes integradas en #1482 (`bdfbe8e2a`, release/corpse/cementerio) y #1483
(`408d11c12`, reclaim). Rama `43-spirit-healer`, base `408d11c12`; código guardado
en el commit local `f4633801212a3ea6515978264e27526b4ddd0bf7`. Final verde:
5.809 tests pasan y se conserva un helper manual ignorado; **sin publicación,
captura nueva, QA viva ni despliegue; #43 no se cierra**. Contrato:
`NPCHandler.cpp:331–376` del C++ objetivo `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
admisión del NPC, retirada de muerte fingida, `ResurrectPlayer(0.5f, true)`,
`DurabilityLossAll(0.25f, true)`, selección del cementerio desde la ubicación del
cadáver antes de `SpawnCorpseBones`, y teleport solo si difiere del cementerio
del jugador. `Player.cpp:4320–4404` toma el hechizo de `ChrRaces.ResSicknessSpellID`
y el umbral de `Death.SicknessLevel`; los nueve primeros niveles afectados
ajustan el tiempo restante, conservando la duración máxima del aura
(`SpellAuras.cpp:897–906`).
La duración final viaja como dato efímero del lanzamiento para publicar una sola
actualización del aura, siguiendo la acumulación de `SetNeedClientUpdate`
(`SpellAuras.cpp:220–228`). La reactivación refresca la misma aplicación y
conserva su identidad de lanzamiento (`Unit.cpp:3224–3283`). La pérdida del
25 % selecciona equipo, mochila y bolsas llevadas, excluyendo banco y sus bolsas,
en el orden de `Player.cpp:4522–4544`; publica la durabilidad modificada y
recalcula estadísticas si el equipo se rompe.

La inspección de los DB2 locales de referencia (`Data/dbc/enUS`, sin overlays SQL)
encuentra en 15007 los efectos 80/79/159, índices 0/1/2: estadísticas base −75 %,
daño −75 % y ausencia de crédito PvP. `SpellMisc` 325284 usa `SpellDuration` 427:
duración −600000, máximo 600000; `SpellInfo::GetDuration` toma el valor absoluto.
Huellas SHA-256: SpellEffect `c786e3f82d69f6a2675cdcc7a0a6c48a2684044fb23344bd488425d982b525f8`,
SpellMisc `23a649b44401e99d123375dc7a0d7be1eee76287af02e7bd91c2a45636a089c2`,
SpellDuration `4b5c9c1795abe70cb788d5652c1dcad413bf535fd9423d49501309400ee3da53`.
`SpellXSpellVisual` contiene para 15007 la fila incondicional 245039 (visual 69),
que es la identidad visual transmitida por `SpellInfo::GetSpellXSpellVisualId`
(`SpellInfo.cpp:4446`). Su huella es
`893b2141d6250684968cccd76fb365b548177327f2fad508131fe46050276e86`.
No se encontraron filas 15007 en las secciones legibles de `SpellCooldowns`
(`4dd4678937c313c08952095e249308904099a06e825b714ed3268e6fc6600b69`) ni
`SpellPower` (`2e62ed5891722bcdb4f87d4bde8f78abb0814e80f2095bcc18cbe64ac41ed2e5`);
se omitieron sus siete y dos registros cifrados, respectivamente. El contrato
del lanzamiento interno de enfermedad conserva CastID propio, OriginalCastID
vacío y la visual efectiva; `Spell.cpp:4765–4814,8072–8077` gobierna flags y
admisión de SpellGo. No supone una reparación general de lanzamientos internos.
Son datos inspeccionados, no prueba de los overlays efectivos del servidor vivo.
Participantes necesarios de esta entrega: duración restante/expiración, aplicación
conjunta de los efectos de la enfermedad y su multiplicador de estadísticas base
(`SpellAuraEffects.cpp:3567–3601`, `Unit.cpp:9080–9094`). La aceptación local
se ejecutó con los cambios sin commit sobre `408d11c12`; después se guardaron
sin alteraciones en `f46338012`. El manifiesto conserva `dirty: true` y el HEAD
base real: no se presenta como una ejecución sobre el commit posterior.
Se escribieron y ejecutaron
escenarios en `crates/wow-world/unit_tests/handlers/entities/tests/spirit_healer.rs`
(admisión, enfermedad por nivel/raza/configuración, renovación, estadísticas,
durabilidad y cementerios) y `crates/wow-world/unit_tests/session/tests/aura_duration.rs`
(duración restante, identidad y expiración), además del caso de multiplicadores
en `crates/wow-data/src/player/stats.rs`. Pasan tanto los casos enfocados como las
suites completas seleccionadas por final. El soporte de pruebas del
NPC usa los catálogos de la sesión; los escenarios completan el ACK del teleport
de release y establecen LoggedIn para ejercitar el refresco de estadísticas.
**Aceptación final del candidato 3:** mismo comando y entorno de abajo, sin
`--keep-going`, aarch64/Rust 1.98.0. Manifiesto
`target/validation-v2/manifests/20261024T001007.529334Z-3206589-final.json`,
exit 0, verificado con `./tools/validation-v2 verify --manifest <ruta anterior>
--require-profile final`. Runner 00:10:07.528–00:26:42.333 UTC:
**994,804 s**; ventana completa con verificación y contraste de huellas hasta
00:27:22.428 UTC: **1.034,900 s**. **Objetivo de 600 s incumplido.**
El cierre de consumidores compiló en 719,221 s, incluyendo artefactos aún fríos
de `world-server`/SQL; el lote de tests compiló y ejecutó en 131,328 s.
No se repitió una campaña intacta solo para obtener un tiempo cálido menor.
Pasan arquitectura/fixtures/firmas, censo, formato y nueve bibliotecas:
`wow-data` 759, `wow-entities` 986, `wow-world` 3.764, `wow-world-application` 15,
`wow-world-core` 224, `wow-world-entities` 2, `wow-world-interaction` 1,
`wow-world-inventory` 33 y `wow-world-spell` 25. Única prueba ignorada:
`handler_contract_tests::print_world_handler_contract_snapshot`, helper manual
ya ignorado antes de esta entrega. La auditoría exhaustiva de persistencia no
forma parte de final y sigue sin ejecutarse; no se reclama esa evidencia.
Las huellas de los 44 archivos probados están en
`target/validation-v2/evidence/43-spirit-healer-candidate-3.json`; se verificaron
antes del commit, cuyo árbol es `a8098c58daac7c43b17886a1568087f049d42136`.
Las tres corridas final, incluidas las fallidas de abajo, suman 1.807,132 s;
los tiempos de reparación/feedback se conservan por separado. La aceptación
de publicación del candidato commiteado y la aceptación viva siguen pendientes.
El delta posterior es solo este checkpoint: `quick --base f46338012
--require-changes --logs`, manifiesto local
`target/validation-v2/manifests/20261024-43-spirit-healer-evidence-quick.json`;
reutiliza el final anterior para código idéntico, sin atribuirle otro HEAD.

Primera campaña en aarch64, árbol modificado sobre `408d11c12`:
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=1
CARGO_TARGET_DIR=/home/server/rustycore/target VALIDATION_V2_CARGO_JOBS=1
VALIDATION_V2_TIMEOUT_SECONDS=1800 ./tools/validation-v2 final --base origin/3.4.3
--architecture --timings --logs --require-changes --keep-going`,
23:26:21–23:36:56 UTC, **634,941 s**. Manifiesto local
`target/validation-v2/manifests/20261023T232621.284597Z-3184099-final.json`;
huellas del candidato en `target/validation-v2/evidence/43-spirit-healer-candidate-1.json`.
Fallaron el límite de crecimiento de Session/Player, el inventario de interfaces
y el import de `ItemUpdateState` desde la raíz de `wow-entities`. El integrador
interrumpió con SIGTERM la compilación de tests dependiente del mismo error
(exit 143); no se ejecutaron los tests. Formato, censo, archivos físicos,
ausencia de duplicados y método-índice pasaron. El comprobador de arquitectura
se compiló desde frío (289,578 s en su primer paso); también faltaban artefactos
del workspace. **Supera 600 s; no acredita el objetivo de rendimiento cálido.**
La reparación posterior se mide aparte. Censo revisado: seis tests nuevos de
composición, ninguna identidad/clase preexistente retirada o reclasificada.
Segunda campaña, mismas opciones sin `--keep-going`, candidato local 2:
23:41:45–23:44:43 UTC, **177,387 s**, exit 101, manifiesto
`target/validation-v2/manifests/20261023T234145.998066Z-3190700-final.json`.
Arquitectura, fixtures, firmas, censo y formato pasaron. La compilación detectó
dos argumentos `true` añadidos indebidamente a taunt y al aura de criatura al
adaptar el host de aura de jugador; tests todavía no ejecutados. Se conserva el
resultado fallido y se corrigen ambos consumidores. Reparación entre campañas:
23:36:56–23:41:45 UTC (aproximadamente 290 s, fuera de la medición de aceptación).
La secuencia de enfermedad y sus flags quedan en `wow-world-spell`; el adaptador
existente de World solo reenvía `send_update`. Crecimiento revisado y acotado:
Session +2 líneas de producción/+200 de tests; Player +34 de producción sobre
el almacén canónico, sin nuevo estado, lock, reloj o tarea. Inventario de firmas:
dos métodos de host añadidos y dos firmas ampliadas; ninguna otra sección cambia.
Feedback del implementador (secuencial, `CARGO_BUILD_JOBS=1`, mismo target/PROTOC,
sin campaña paralela del integrador): `cargo check -p wow-world --tests` tuvo dos
fallos (acceso privado del adaptador; acceso privado y orientación en el fixture)
y pasó tras corregirlos. `cargo test -p wow-world spirit_healer` pasó finalmente
**9/9**, y `cargo test -p wow-world aura_duration`, **1/1**. Sus ejecuciones filtradas
de targets de integración encontraron cero tests y no se cuentan como evidencia.
Logs locales bajo `target/validation-v2/evidence/`, prefijo `spirit-healer-wow-world-`:

| Log | Segundos de pared | Resultado |
| --- | ---: | --- |
| `check.log` | 138 | exit 101 |
| `check-2.log` | 52 | exit 101 |
| `check-3.log` | 49 | exit 0 |
| `test-spirit_healer.log` | 445 | 8 pasan, 1 falla; compilación inicial del ejecutable |
| `test-spirit_healer-2.log` | 73 | 9 pasan |
| `test-spirit_healer-3.log` | 77 | 4 pasan, 5 fallan en la precondición común del ACK |
| `test-spirit_healer-4.log` | 73 | 9 pasan; termina 2026-10-24 00:08:10 UTC |
| `test-aura_duration.log` | 1 | 1 pasa; termina 00:08:18 UTC |

Los 908 s de órdenes de feedback son coste de implementación/reparación, no una
campaña final verde. El fallo de cementerio era una aserción sobre
`far_destination` para un teleport local: ahora se comprueban selección 5/4,
`near_destination`, paquete/contador, ACK, posición final y flags pendientes.
El destino histórico se conserva tras ACK; su presencia no significa pendiente.
No se cambió producción para adaptar esa prueba. El archivo de escenarios mide
1.068 líneas: revisión de cohesión aceptada para este único flujo con sus fixtures;
si la continuación de #43 amplía sus fixtures, separar sus decodificadores y datos
de hechizo antes de añadir escenarios de relog. No requiere excepción de 2.000 líneas.
La issue completa sigue abierta: carga del cadáver al relog, participantes de
movimiento y aceptación DB/restart/relogin conservan sus pendientes.
Siguiente responsabilidad contrastada: `Player::LoadCorpse` (`Player.cpp:18157`)
y su llamada tras entrada en mundo (`CharacterHandler.cpp:1227`, antes de restaurar
auras de ghost/velocidad/water-walking). La carga actual de cadáveres del mapa en
`wow-world-character/src/login_session/login_sequence.rs:349` no restaura por sí
sola la ubicación de cadáver del Player, `AT_LOGIN_RESURRECT` y release timer.
Este es el siguiente contrato de relog que debe implementarse y probarse, sin
suponer que el objeto corpse esté en el mapa actual del jugador.
El guardado general representado (`PlayerCharacterSaveRequestLikeCpp` y
`wow-world-lifecycle/src/player_save.rs`) no captura auras ni durabilidad de los
objetos: marcar el objeto como modificado y publicar sus valores no demuestra
su supervivencia a logout/restart. El efecto 159 queda representado en el aura;
no se reclama implementación del sistema de crédito de honor. La agrupación y
el orden observables de AuraUpdate requieren captura específica de esta acción.

**#1263 F5: las 10 entradas diferidas — 2026-10-11 (rama `1263-f5-deferred`, PR sin integrar):** void storage (4), talent (2) y corpse (4) salen del inventario heredado hacia tres registradores de área explícitos (`wow_world_inventory::register_void_storage_handlers_like_cpp`, `wow_world_application::register_talent_handlers_like_cpp` y `register_corpse_handlers_like_cpp`) cuyos rasgos de host son los contratos de adaptador que faltaban: la persistencia de inventario/dinero, el commit/publicación del reset de talentos y las transiciones de muerte/resurrección/mapa se quedan en el host de World, que ya las poseía. Sin cambio de comportamiento; `world-handler-contract.tsv` idéntico. **F5 en 476/476** (0 legacy-direct + 423 builder + 53 macro); el puente heredado queda vacío y su retirada es un corte aparte. Con este corte, **los seis puntos del criterio de cierre de #1263 están entregados** cuando se integre: (1) #1338, (2) #1345, (3) #1347–#1353, (4) #1355/#1356, (5) este corte, (6) #1358. Seguimientos fuera de #1263: #1339–#1344, #1354, #1357, #1359, más #1360 (retirar el puente heredado vacío) y #1361 (paridad de los handlers de talentos), abiertos por este corte. Detalle y evidencia: entrada `F5: las 10 entradas diferidas` de `wow-world-distribution-plan.md`.

**#1263 F4 LootMoney dependencia (8) — 2026-10-11 (rama `1263-f4-lootmoney-dep8`, integrada como #1358):** el drenaje de objetivos de quest sale del shell: el dueño de producción es `QuestObjectiveProgressCx::drain_represented_quest_objective_progress_like_cpp` (`wow-world-application`), cuya finalización ejecuta ella misma el refresco `UpdateVisibleGameobjectsOrSpellClicks` de `SendQuestUpdate`; `session/quest/objective_drain.rs` solo presta los dueños y se borran los cuerpos de dinero/moneda/reputación del shell. Sin cambio de comportamiento. Con esto **las nueve dependencias de LootMoney están en sus dueños** (el hold «dependencia (8) sigue en el shell» de las entradas de cierre de abajo queda superado). Incluye dos seguimientos de #1356: test de fijación de `UnitFlags` vivos en el CREATE de visibilidad y robustez del test de captura C1 frente a la caché de interés de `tracing`. Detalle y evidencia: entrada homónima de `wow-world-distribution-plan.md`.

**#1263 F6-8E-2 — 2026-10-11 (rama `1263-f6-8e2`, integrada como #1356):** se borra `wow-world-core/src/map_manager/**` (el `MapManager`/`SharedMapManager` heredado, `MapInstance`, `Grid`) y la vista transitoria `WorldCreature`: la raíz de mutación presta la `Creature` canónica (`&mut Creature`) y la proyección `CreatureCreateData` se calcula bajo demanda. Las utilidades de producción viven en `wow_world_core::{terrain, pathfinder, creature_movement, creature_respawn, creature_projection, corpse_load, runtime_plan}`. El runtime canónico pierde los nombres `Legacy*` (`session::creature_runtime`). Seguimientos de #1355: el loot AE visita solo celdas cercanas y lee en sitio; las consultas del movedor controlado vuelven a `None` sin clave canónica del Player (semántica previa a E-1); notas de DynamicFlags/D-2 corregidas; procedencia del TSV de D3a documentada. Detalle y evidencia: entrada F6-8E-2 de `wow-world-distribution-plan.md`.

**#1263 F6-8E-1 — 2026-10-11 (rama `1263-f6-8e`, integrada como #1355):** se borra el runtime heredado de criaturas alrededor del almacén canónico: `RuntimeTickOwner`, los ticks de criatura y combate por sesión, el bucle global heredado y su entrega, las entradas heredadas de fase con sus rasgos de almacén, todos los espejos heredado↔canónico, los respaldos de lectura del almacén heredado y la decisión C1 tautológica. El bucle canónico del mapa es el único runtime de criaturas. Al portar los tests del tick de sesión se corrigieron tres divergencias de la fase de melé del jugador propiedad del mapa (rango/orientación como `getAutoAttackError`, `AttackStop` y fin de combate al matar, clave de mapa de las consultas del movedor) y los dos huecos de DynamicFlags forzados de la liberación de loot. **Queda para F6-8E-2** el borrado de `wow-world-core/src/map_manager/**` (el `MapManager` heredado, solo de tests, y `WorldCreature`, ahora vista transitoria de la criatura canónica). Sin despliegue ni QA live; **#1263 no se cierra**. Detalle en [wow-world-distribution-plan.md §8](../architecture/wow-world-distribution-plan.md) (F6-8E-1).

**Cierre del tramo F4→F6 de #1263 — 2026-10-09:** las **seis reparaciones firmadas** del contrato de reconciliación de autoridades de loot están integradas (R4a, R1a, R1b, R7 completo, R2+R3+R5) más las **capturas de runtime instrumentadas C1/C2** con su registro fechado. Los **cuatro remates estructurales de F4** están integrados, pero **F4 no está completo** (LootMoney estructural con la frontera 8/9 re-firmada por el revisor y la dependencia (8) aún en el shell, StorageMove con el crecimiento de R1 revisado, Save con la valla finalización-antes-de-captura convertida en valla probada, y refresh). De **F6-8** están entregadas las slices **A, B, C, D1 y D2**: el estado de runtime persistente, el RNG, el reloj y la autoridad de ciclo de vida/spawn son canónicos; el movimiento y la publicación de visibilidad leen el estado canónico; los consumidores de melé/aggro/hechizo y de loot deciden canónicamente; el motor completo de fases vive en propiedad canónica con continuidad probada contra los cuerpos pre-D1; y el **camino aislado de ejecución admitida** tiene valla de admisión previa a la mutación, test de exclusividad de tres dueños y publicación diferida, **con la propiedad de producción intacta**. La migración **no diferida de F5 está cerrada: 466/476** registros efectivos (413 literales + 53 por macro). **Holds declarados, no entregados:** la paridad de payout de F4 y la recuperación de claims tras reinicio (slice de comportamiento propia, #1339); la **dependencia (8) de LootMoney** (drenaje de objetivos de quest), que sigue en el shell y cuya retirada sigue debida dentro de #1263; la **paridad del paquete combinado C++ de refresh de F4**, pendiente de captura específica y reparación propia (#1340); los **10 registros diferidos de F5** (void storage 4, talent 2, corpse 4, con sus contratos de adaptador pendientes; desde #1334–#1336 son exactamente las «familias F5 restantes» de la frase del revisor); **F6-8 D3/E** (corte de producción, retirada del planificador y borrado del almacén heredado), cuya aceptación **terminal exige un cliente 3.4.3 real** (#1342); la **evidencia pareada de R6**, que sigue en hold: la corrida pareada del 2026-10-10 (fixture aprovisionado una vez y restaurado exacto) confirma el mismo `CMSG_ATTACK_SWING` admitido, `SMSG_ATTACK_START` y primer golpe idéntico en ambos lados, pero la reacción de combate de la criatura **diverge** justo después, la ventana de persecución no se capturó en ningún lado y el rechazo stale/ABA **no se ejercitó** ([1263-r6-paired-2026-10-10.md](../architecture/captures/1263-r6-paired-2026-10-10.md); divergencia en #1344, defecto de referencia C++ en #1343); y la reapertura de la cuarentena por COMMIT desconocido (#1341). **Salvedad de revisor:** la exención de crecimiento R1 de #1325 (`budget: 661`) y la re-firma 8/9 de #1324 las adjudicó, según la revisión independiente de #1337, el modelo de respaldo del coordinador (`minimax-m3`) actuando como revisor; **esperan firma independiente**. **Rendimiento (corregido 2026-10-10):** de R4a a #1337 hubo **44 campañas `final --architecture --timings`: 38 verdes y 6 fallidas**. Verdes: **dos excesos divulgados** — la cola F5 con **944,88 s** (caché frío tras un disco lleno; re-medición cálida 305,41 s) y F6-8D2 con **613,37 s** (re-medición cálida 136,29 s)—; las otras 36 entre **110,65 y 576,23 s** (no 112–479 s). La `final` de R1a (#1313) registra `dirty: true`. Las seis fallidas (dos de R4a #1312, #1324, #1325, #1329 interrumpida por SIGTERM, #1334), con su paso fallido y dónde se divulgaron, están en la tabla del cierre en [wow-world-distribution-plan.md §8.1](../architecture/wow-world-distribution-plan.md). **Criterio de cierre de #1263 (aprobado por el operador, 2026-10-10):** #1263 se cierra cuando se entreguen, cada uno como slice propia con evidencia y `final` verde: (1) la evidencia pareada de R6 restringida a combate, entregada como MISMATCH registrado en #1338 (R6 cerrada como slice de evidencia; la divergencia pasa a #1344); (2) esta corrección del cierre; (3) F6-8 D3, corte de producción al runtime canónico y retirada de la planificación heredada, aceptado con evidencia local; (4) F6-8 E, borrado de `MapManager`/`SharedMapManager`, ramas de compatibilidad y configuración obsoleta; (5) las 10 entradas diferidas de F5 con sus contratos de adaptador; (6) la dependencia (8) de LootMoney. Todo lo demás sale de #1263 como issues propias (#1339–#1344). **#1263 no se cierra** y no se reclama paridad de juego ni retirada del runtime legado.

**Cierre del tramo de reparaciones de #1263 — 2026-10-08:** las **seis reparaciones firmadas** del contrato de reconciliación de autoridades de loot están **integradas** en `3.4.3` como cortes propios, cada uno con PR con squash y su propia campaña `final` verde sobre el candidato commiteado con árbol limpio: **R4a** (contención del agotamiento), **R1a** (admisión de autoridad única), **R1b** (sellado de asignación y revinculación), **R7** (R7a contrato de mutación con puerta, R7b-1 límite de admisión, R7b-2a raíces de mutación restantes, R7b-2b puerta compartida en los dos espejos de tick con la ventana diferida medida), **R2** (dueño designado con `Found`/`Absent`/`Unavailable` explícitos y eliminación del colapso `Option` de producción, incluida la corrección de ausencia **ordenada por el revisor** que bloqueaba la aceptación) y **R3** (retirada de la reconciliación de doble almacén, su bucle de ocho rondas, sus CAS y su productor de lápida de conflicto) con **R5** (cierre: regresión de la regla de rechazo de alias ajeno/obsoleto y corrección de tres contratos obsoletos). La **cola F5** se remata con trainer (2) y la familia de macros de movimiento (54 = 53 macro + 1 directa) sobre registradores de área, con la **frontera declarada**: **410/476** registros efectivos migrados, **66 literales en 20 ficheros** todavía por el inventario heredado, 10 de ellos **diferidos deliberadamente** con su razón semántica. Se ejecutó la **campaña de capturas de runtime instrumentadas C1/C2** con registro fechado y esquema v1: C1 vivo **parcial** (mutación legítima admitida y aplicada 74 µs antes del paquete visible al cliente), rechazo **solo** con arnés inyectado y **etiquetado como inyectado**, C2 **solo login** (la ventana detached/far-transfer no fue producible y se declara no ejecutada). **Holds acotados declarados y aprobados por el operador:** F4 remates (LootMoney completo con sus nueve dependencias, StorageMove, Save, refresh), F6-8 (retirada del runtime legado y del `map_manager` legado), la evidencia pareada Rust/3.4.3 de **R6** y la reapertura/retirada de la cuarentena por COMMIT desconocido. **Rendimiento:** todas las campañas `final --architecture --timings` cálidas quedaron **≤600 s** (138–367 s en los cortes de reparación; 305–323 s en el de F5); una corrida de **caché frío** posterior a un incidente de disco lleno dio **944,88 s** y se registra como coste de arranque en frío, **no** como la corrida ordinaria. Sin despliegue, sin QA live nueva y sin reclamar paridad: **#1263 no se cierra**. Detalle por corte en [wow-world-distribution-plan.md §8.1](../architecture/wow-world-distribution-plan.md).

**Distribución wow-world, P4a aceptada — 2026-10-02:** #1263 continúa tras P3
integrada por #1264 en `1f8a7c800`. La rama `1263-f4a-p4a-world-core` mueve las dependencias
independientes del hub a `wow-world-core`, con consumidores, fixtures y scanner. Código
aceptado `d9c9e3637`: final/arquitectura verde (385,703 s), configuraciones e integración
verdes, 3.950 identidades conservadas, cero mensajes de warning nuevos e inventario completo
verde (9.939 registros, 1.034 grupos). El snapshot heredado se reconcilió después de revisar
190 altas/88 entradas obsoletas en seis fuentes sin cambios en P4a; no se reparó gameplay.
Campaña 11:55:50–15:19:37 UTC, 12.227,640 s de pared; incluso las órdenes
ordinarias del candidato definitivo suman 691,218 s sin su inventario de 784,958 s.
**Objetivo de 600 s incumplido.** SHAs, manifiestos, reparaciones y costes separados en
[wow-world-distribution-plan.md §8](../architecture/wow-world-distribution-plan.md).
La publicación/integración de la fase se registra en #1263; P4b/F4b/F5/F6 siguen pendientes. No reclama
paridad nueva, retirada de runtime legado ni cierre de #1263/#584; sin despliegue o QA live.

**Continuación de distribución wow-world — 2026-10-02:** #1263 conserva el trabajo
pendiente de #1241 bajo #584. Base integrada contrastada: `1f8a7c800` (PR #1264);
F0–F3 y F4a P1–P3 integradas. P3 tiene aceptación acotada verde en `002ff5e46`
(movimiento `6f0660ab0`), con publicación/integración posterior registrada en #1263;
P4a tiene la aceptación descrita arriba; P4b, F4b, F5 y F6 siguen pendientes. #1241 ya está
cerrada en GitHub, sin que eso pruebe cierre técnico. La revisión del plan y la
consulta de estado no revalidaron las fases anteriores ni reclaman nueva paridad.
Campaña P3 de código 09:30:05–10:20:07 UTC, 3.002 s de pared; final inicial 622,487 s,
final tras reparar compose 122,093 s. El objetivo completo de 600 s no se cumplió.
El alcance, secuencia y aceptación corregida viven en
[wow-world-distribution-plan.md](../architecture/wow-world-distribution-plan.md);
las entradas fechadas anteriores conservan sus propios límites de evidencia.

**Integration head — 2026-09-19:** the current integration head on `3.4.3` is
`a82f4b2f` (PR #1226, following PR #1225's #29 creature-victim damage-immunity
slice and PR #1224's school-absorb slice). The older #31
diagnosis chain is retained below as historical evidence; its next entry is
`8448bdb4`, the #31 player-aura canonical-sync diagnosis and its route
correction, following PR #1211, the #31 caster label damage-taken term and its label authority,
following PR #1209, the #31 damage-taken term stacking regression, following
PR #1207, the #31 caster spell-family damage-taken term, following
PR #1205, the #31 mechanic damage-taken term and its data seam,
following PR #1203, the #31 caster school-mask damage-taken term, following
PR #1201, the #31 cheat-death damage-taken term, following
PR #1199, the #31 victim school damage-taken term, following
PR #1197, the #31 drain/burn `SpellDamageBonusDone` pre-scaling, following
PR #1195, the #31 remaining execute-log list layouts, following
PR #1193, the #31 zero-pool drain log repair, following
PR #1191, the #31 zero-amplitude creature burn regression, following
PR #1189, the #31 creature power-type drain regression, following
PR #1187, the #31 drained-creature power publication, following
PR #1185, the #31 creature power burn through the damage path,
following PR #1183, the #31 creature-target power drain, following
PR #1181, the #31 durability rows in the spell execute log, following
PR #1179, the #31 spell execute log with take-power entries, following
PR #1177, the #31 flagged-power regen interrupt on energize, following
PR #1175, the #31 `EnergizeBySpell` casing and assisting threat, following
PR #1173, the #31 per-slot creature addon aura effects, following
PR #1171, the #31 creature-target aura application, following
PR #1169, the #29 melee
ignore-absorb term, following PR #1167, the #31 spell energize
log, following PR #1165, the #31 player-target
heal-absorb stage, following PR #1163, the #31 direct spell heal
combat log, following PR #1161, the #31 direct spell
damage combat log, following PR #1159, the #29 player-victim
melee mana-shield absorb stage, following PR #1157, the #29 player-victim
melee absorb log, following PR #1155, the #29 player-victim
melee school absorb with the C++ sub-damage wire block, following PR #1153, the
#584 session syntax-ownership baseline reconciliation and architecture-acceptance
continuation, following PR #1150, the #29
creature-victim immunity scenario, following PR #1148, the #29
melee physical-immunity gate, PR #1144, the #29
ExpectedStat table load, PR #1142, the #29
player-victim block band, PR #1140, the #29
creature-victim parry scenario, PR #1137, the #29
creature-victim evade scenario, PR #1131, the #29
creature-victim melee fixture determinism fix, PR #1129, the #29
creature-victim blocked-amount publication, PR #1127, the #29
creature-victim block-band scenario, PR #1125, the #29
creature-victim avoidance and crit bands, PR #1123, the #29
creature-victim miss band and outcome publication, PR #1121, the #29
melee scenario-suite split, PR #1119, the #29
creature-victim melee mitigation, PR #1117, the #29
player-victim melee damage-taken chain, PR #1115, the #29
creature-to-player armour mitigation, PR #1113, the #29
creature-to-player dodge/parry/crit bands, PR #1111, the #29
creature-to-player melee miss band, PR #1109, the #29
ignore-target-resist armour term, PR #1107, the #29 critical
original-damage correction, PR #1105, the #29
critical-damage-bonus aura, PR #1103, the #29 original-damage
publication, PR #1101, the #29 melee evade
outcome, PR #1099, the #29 controlled-victim
avoidance gate, PR #1097, the #29 ignore-dual-wield
hit-penalty aura, PR #1095, the #29 victim conditional
critical-chance auras, PR #1093, the #29 victim avoidance
auras, PR #1091, the #29 victim-side melee
damage-taken chain, PR #1089, the #29 melee block band,
PR #1087, the #29 melee attack
table, PR #1085, the #29 white-swing armour
mitigation, PR #1083, the #29 white-swing
damage roll, PR #1081, the #65 GameObject
respawn-save test fidelity fix, PR #1079, the #61 victim
aurastate/aura-mechanic melee bonuses, PR #1077, the #61 melee
creature-type damage bonus, PR #1075, the #61 cast-speed auras, PR #1073, the #61 white-swing auto-attack damage aura, PR #1071, the #61 displayed-power ownership, PR #1069, the #61 `HandleShapeshiftBoosts`, PR #1067, the #61 form-change item effect refresh, PR #1065, the #61 shapeshift form ownership and `CombatRoundTime`, PR #1063, the #61 attack-speed aura application, PR #1061, the #61 `CONFIG_STATS_LIMITS_*` caps, PR #1059, the #61 `BonusCoefficientFromAP` table term, PR #1057, the #61 `SpellHealingPctDone` completion, PR #1055, the #61 remaining `SpellDamagePctDone` terms, PR #1053, the #61 health-derived unit aura states, PR #1051, the #61 mechanic-based damage multipliers, PR #1049, the #61 versus-aurastate damage multiplier, following PR #1047, the creature missing-health heal scaling, PR #1045, the versus-creature-type damage multiplier, PR #1043, the missing-health healing scaling, PR #1041, the `SpellHealingBonusTaken`, PR #1039, the victim `ModHealing` term, PR #1037, the direct-heal spell-power bonus, PR #1035, the school damage percentage, PR #1033, the caster spell-power damage bonus, PR #1031, the weapon-enchantment damage term, PR #1029, the ranged weapon fit, PR #1027, the `Unit::UpdateDamageDoneMods` representation, PR #1025, the `UpdateDamagePctDoneMods` representation, PR #1023, the `VersatilityBonus` publication, PR #1021, the override percentage publication, PR #1019, the `ModTargetResistance`/spell-penetration publication, PR #1017, the `ModHealingDonePercent` publication, PR #1015, the `ModDamageDonePercent` publication, PR #1013, the narrow values-update negative spell field, PR #1011, the spell field wire publication, PR #1009, the spell damage/healing done producers, PR #1007, the override-attack-power-by-spell-power aura, PR #1005, the seven stale `wow-world --lib` expectations, PR #1003, the quest party fixture identity fix, PR #1001, the save-snapshot manager-lock re-entry fix, PR #999, the session reputation-closure lock re-entry deadlock fix, PR #997, the collection appearance `CanUseItem` template gates, PR #995, the collection appearance weapon-proficiency gate, PR #993, the #61 attack power aura producers, PR #991, the #61 school resistances, PR #989, the #61 critical-strike aura percentages, PR #987, the #61 avoidance aura percentages, PR #985, the #61 armor aura producers, PR #983, the #61 `Unit::m_transformSpell`/`IsPolymorphed` owner, PR #980, the #61 aura-backed per-attack expertise, PR #978, the #61 food/drink regeneration emote visual, PR #976, the #61 observer `SMSG_POWER_UPDATE` fan-out, PR #974, the #61 creature-kill durability loss, PR #972, the #61 durability-damage spell effects, PR #970, the #61 fall-death item durability loss, PR #968, the #61 C++ regeneration rates, PR #966, the #61 non-mana power-regeneration loop, PR #964, the #61 health-regeneration tick, PR #962, the #61 mana-regeneration docs sync, PR #960, PR #959/#958, docs-only PR #956, and PR #957/#955/#954/#953/#950/#948/#935/#933/#931/#929/#926/#925/#924/#923/#922/#921/#904/#902/#901/#899/#897/#895/#893/#891/#889/#887/#885/#876/#873/#871/#869/#866/#864/#862/#860/#859/#855/#854/#853, PR #851, PR #848, PR #846, PR #844 and PR #842). The entries below preserve
dated evidence and limits; they do not select an already integrated macro again.
The active architecture sequence is the remaining measured work in #584, followed
by the stateful module product #583 and the independent audit #153. #582 and
#587–#589 are closed in their bounded scopes; #486 and #524 remain open only for
the residual acceptance explicitly stated below.

**#29 creature white-swing split/share damage — 2026-09-19, split implementation
`5ec01cb4`, share implementation `b18fd23a`, unkillable implementation
`11f66869`, damage-threat implementation `3cba7f7f` and replay extraction
`027067eb`, aura-provenance implementation `39fdab43`, candidate in PR #1228:** the
map-owned creature melee path now executes the represented
`SPELL_AURA_SPLIT_DAMAGE_PCT` tail of `Unit::CalcAbsorbResist`
(`Unit.cpp:1958-2015`) after school and mana shields. It snapshots the primary
Player/Creature victim's active split effects, applies each percentage to the
current remainder, preserves the primary absorption before the secondary
immunity/`DealDamageMods` gates, and mutates the live Player/Creature caster
through canonical map authority. Missing, self, out-of-world and dead casters
are skipped; immune casters keep the primary split absorbed and emit the exact
3.4.3 `SpellMissLog` layout; in-flight/evading secondary targets take no health
damage and report the split as absorbed in `SpellNonMeleeDamageLog`. Secondary
Creature health/death revisions join the existing canonical-to-legacy CAS chain,
and a primary Creature already at its sparring threshold zeroes its remaining
wire damage at C++'s post-`DealDamageMods` point. Split logs precede the primary
`AttackerStateUpdate` on the owning delivery rail. Production-shaped regressions
cover Player and Creature primary victims, Player and Creature secondary
targets, missing caster, immunity, evade, sparring, health publication and
packet order. The private split module keeps the existing melee tick facade
below the 2,000-line hard threshold.

The same owner now also executes `SPELL_AURA_SHARE_DAMAGE_PCT` at
`Unit::DealDamage`'s post-hook point (`Unit.cpp:767-856`). Each live,
school-matching aura is revalidated by application identity, copies a percentage
of the same post-split and post-sparring `damageDone`, leaves the primary hit
unchanged, and applies the secondary call as `NODAMAGE`, so another share aura
on that target does not recurse. Player and Creature health/death remain
canonical-map writes; Creature transitions join the revision/incarnation CAS
chain. The primary attacker-state frame remains ahead of share mutations;
self-share Player health publications remain on the victim-session FIFO between
that frame and the final primary health, while Creature values updates retain
the same order in the map plan. Regressions cover Player/Creature targets,
split-then-share arithmetic, multiple same-base shares, self share, evade,
sparring-before-share, non-recursion, canonical health and packet order.

The primary and both secondary Creature damage paths now preserve
`CREATURE_STATIC_FLAG_UNKILLABLE` at the common health-commit boundary exactly
where target 3.4.3 `Unit::DealDamage` clamps `damageTaken` to `health - 1`
(`Unit.cpp:887-898`). A lethal hit from another Unit leaves one HP and no death
state, while self damage, non-lethal damage and creatures without the flag are
unchanged. Because the C++ clamp follows damage/log calculation, the primary
attacker-state and split non-melee log continue to expose the raw amount.
Regressions exercise the entity rule and the direct, split and share production
consumers, including canonical-to-legacy CAS publication. The nearby virtual
target/health multipliers remain 1.0 defaults and have no overrides anywhere in
the pinned target source (`Unit.cpp:746-758`, `767-800`; `Unit.h:776-777`), so
they require no speculative Rust seam.

The direct, split and share Creature nonlethal paths now settle damage threat
after the health transition at target C++'s `Unit::DealDamage` point
(`Unit.cpp:868-869`, `1015-1034`; `ThreatManager.cpp:353-465`, `662-690`).
The transition creates reciprocal combat/threat references and enters Creature
AI combat, applies `NO_THREAT`, `NO_HARMFUL_THREAT`, `NO_INITIAL_THREAT`,
`SpellThreat.pctMod` with chain fallback and the attacker's school-specific
`MOD_THREAT`, and reuses the represented `CanHaveThreatList` capability. Lethal
damage does not execute this nonlethal settlement. Sparring `damageDone == 0`
returns before threat, while an `UNKILLABLE` clamp from positive `damageDone`
to zero `damageTaken` still performs `AddThreat(0)` and establishes combat, as
in C++. Canonical-to-legacy replay preflights the whole contiguous victim chain
and verifies each threat attacker's spawn and shared authority before mutation,
so a same-GUID replacement cannot receive reciprocal state. Production-shaped
regressions cover direct/split/share accumulation, both modifier classes,
suppression and initial-admission attributes, lethal exclusion, zero-value
`UNKILLABLE` references, AI state and both canonical and legacy reciprocal
references. The behavior-preserving replay extraction leaves its owner, lock
order and public paths unchanged while reducing `creature_melee_tick.rs` from
2,135 to 1,863 lines; the new private `creature_melee_sync.rs` is 341 lines,
and the physical-source ratchet passes without a ceiling increase.

Cast-created Player and Creature auras now retain the C++ Aura base's
`CastGUID` and resolved `SpellXSpellVisualID` on the canonical Unit slot, and
slot replacement/removal clears that provenance before reuse. Restored Player
auras allocate a fresh map-owned Cast GUID and select the first unconditional
visual exactly as the no-caster `GetSpellXSpellVisualId()` path does. The split
consumer reads those fields into `SpellNonMeleeDamageLog` instead of emitting
empty/zero placeholders. This follows `Aura::Aura` (`SpellAuras.cpp:462-465`),
`SpellInfo::GetSpellXSpellVisualId` (`SpellInfo.cpp:4446-4463`), Player aura
load (`Player.cpp:18036-18122`) and the split log construction
(`Unit.cpp:2003-2012`). Regressions cover slot lifetime, active Player and
Creature cast application, restored Player selection that skips a
caster-conditioned visual, and exact Player/Creature split packet bytes.

The current continuation carries the same Aura-base identity through Creature
spawn addons instead of fabricating a Cast GUID from the visible slot at
publication time. Target 3.4.3 `Creature::LoadCreaturesAddon` calls
`Unit::AddAura`, whose normal-source Cast GUID comes from the owning Map and
whose Aura retains the selected `SpellXSpellVisualID`
(`Creature.cpp:2734-2798`, `Unit.cpp:11473-11517`,
`SpellAuras.cpp:344-389,462-465`, `SpellInfo.cpp:4446-4462`,
`Map.h:514-519`, `Map.cpp:2505-2512`). Rust now admits the exact Unit-owned
slot first and settles its provenance from the Map's shared `HighGuid::Cast`
sequence on initial loaded-grid creation, canonical `JustRespawned`, and the
pending-respawn compatibility path. Rejected, duplicate or stale applications
neither overwrite a live Aura nor consume a sequence value; the legacy mirror
receives the already-settled canonical identity without allocating another.
Taunt Aura creation likewise inherits its parent Spell Cast GUID/visual rather
than allocating a second identity, and `AuraUpdate` publishes the retained
canonical Cast GUID and visual.

The bounded addon visual resolver is data-proven rather than a guessed
`UnitCondition` port. Against target source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, the 910 effective addon spell IDs
intersect 450 `SpellXSpellVisual` rows; every matching row is difficulty 0,
unconditional for the caster, and unique per spell. The audited DB2 hashes are
`893b2141d6250684968cccd76fb365b548177327f2fad508131fe46050276e86`
(`SpellXSpellVisual.db2`) and
`95e3a382588a6934ca33c1346eec31bb777046f273ac7b855c844da2dda15b7e2`
(`UnitCondition.db2`). The four effective SQL visual overrides do not target an
addon spell. Difficulty-zero fallback therefore returns the relation row ID,
while missing, duplicate or caster-conditioned rows fail closed to zero;
viewer conditions are intentionally irrelevant because target
`GetSpellXSpellVisualId` evaluates only caster conditions in this path.

This is a bounded represented white-swing delivery, not closure of #29 or of
generic `Unit::DealDamage`: aura script split hooks, the remaining AI/script
hooks and criteria, damage/proc/fear/kill side effects of the recursive
`NODAMAGE` call, threat redirection, vehicle/private-object routing and Player
`SpellMod::Hate`, pet/guardian secondary targets, and generated cast/visual
provenance for pet/guardian and other non-cast aura producers, conditional or
override visual evaluation outside the audited addon domain, and
server-triggered Creature cast visual resolution remain explicit boundaries.
No fresh 3.4.3 client capture or authorized live runtime/DB-relogin acceptance
was performed for this candidate.

Candidate validation: both focused split scenarios pass (2/2), the byte-level
`SpellMissLog` regression passes (1/1), and the complete `world-server`,
`wow-data`, `wow-packet` and `wow-world` library command passes; `wow-world`
reports 4034 passed, 0 failed and 1 ignored after the share continuation. The
other complete suite counts are `world-server` 594, `wow-data` 754 and
`wow-packet` 755, all with zero failures. The exact locked affected-workspace
check (`cargo check --locked --tests --jobs 1 -p world-server -p wow-data -p
wow-packet -p wow-world`) passes on `b18fd23a` with Cargo reporting 8m49s. The
complete locked library command reports a 9m47s build before the suites. The
earlier exact locked production check
for the final affected workspace closure passes in 2m18s on the repaired warm
candidate; formatting/diff/JSON checks pass, the physical-file ratchet passes
without raising a ceiling, and the
syntax-only Session ownership check passes with its reviewed baseline delta.
`validation-v2 final --base origin/3.4.3 --architecture --timings` stops only
on the unchanged global hotspot ratchet in `session/mod.rs`,
`handlers/character/mod.rs`, `world-server/src/lib.rs`,
`handlers/quest/mod.rs` and `wow-entities/src/player/mod.rs` (manifest
`20260919T201800.039244Z-2-final.json`); the standard final likewise stops at
that unchanged ratchet (`20260919T201950.128060Z-2-final.json`). The standard
publication-candidate final on `26a11f9a` passes diff, physical-file,
whitespace, JSON and format checks before stopping only on the same global
hotspot ratchet (manifest `20260919T210735.650972Z-2-final.json`). The ordinary
campaign performance
target is not met: architecture acceptance (84.42s) plus the production check
(555s in the initial timed campaign) already totals at least 639.42s before the
library suites and command overhead. The faster repaired rerun does not relabel
that budget overrun or the red global gate as green.
The focused share scenarios pass (2/2, plus one unrelated name-filter match),
the victim-session self-share ordering regression passes (1/1), and the direct,
split and share unkillable regressions pass with mirrored legacy/canonical
fixtures. The exact locked affected command `cargo test --locked --lib --jobs 1
-p wow-entities -p wow-world` passes on `11f66869`; `wow-entities` reports 942
passed and `wow-world` reports 4035 passed and 1 ignored. Its warm execution
took 3.04s. This is functional local evidence for committed candidate
`11f66869`, not a green
global architecture gate, live acceptance or satisfaction of the ordinary
600-second performance target.
The damage-threat repair's exact sparring-zero and zero-value `UNKILLABLE`
regressions each pass 1/1, and the complete focused
`scenarios_world_entities_34` module passes 7/7 on `3cba7f7f`. On the exact
tree committed as `027067eb`, the locked production/test-target check for
`wow-world` and `world-server` passes in 8m32s, and the complete locked affected
library command passes (`wow-entities`: 942; `wow-world`: 4036 passed and 1
ignored; zero failures) after a 2m45s build. The committed-candidate standard
final passes diff, physical-file, whitespace, JSON and format checks before
stopping only on the unchanged global hotspot ratchet (manifest
`20260919T222233.238707Z-2-final.json`). This campaign took at least 12m10s
across its required checks, so the 600-second ordinary performance target is
not met; the correctness evidence is green apart from that known global gate.
The aura-provenance implementation's six focused regressions pass for slot
lifetime, active Player/Creature casts, restored Player selection and exact
Player/Creature split-log bytes. On the tree committed as `39fdab43`, the
locked affected library command passes with zero failures (`wow-world`: 4039
passed and 1 ignored), and the locked `world-server`/`wow-world` production and
test-target check passes in 8m51s. Formatting and diff checks pass after the
comment-only candidate delta. The reviewed syntax-ownership baseline passes
with 225 production and 429 fixture fields, 69 impl owners / 3935 exact items
and 630 exact direct-registry rows. This continuation was part of the already
over-budget campaign and therefore does not satisfy the ordinary 600-second
performance target; publication-final and its manifest are recorded with the
documentation candidate below. The committed-candidate standard final passes
diff, physical-file, whitespace, JSON and format checks before stopping only
on the same unchanged five-file global hotspot ratchet (manifest
`20260919T232529.121135Z-2-final.json`).

The Creature-addon continuation's final local suite results are zero failures:
`world-server` 597, `wow-data` 754, `wow-entities` 946, `wow-map` 752,
`wow-packet` 755, and `wow-world` 4041 passed with 1 ignored. Two strengthened
provenance fixtures initially exposed invalid synthetic inputs: an addon
Creature used a generic GUID without the Cast GUID's realm/map, and the taunt
test invoked the low-level executor with an empty parent Cast GUID. The repaired
fixtures now use a real Creature GUID and consume the owning Map's shared Cast
sequence; their focused reruns and complete affected suites pass. The exact
locked reverse-closure check for the thirteen affected packages and test targets
passes in 6m12s. Formatting/diff/physical-file checks pass without raising a
ceiling. The standard final passes those static gates before stopping on the
already-recorded global hotspot ratchet (manifest
`20260920T095833.199932Z-2-final.json`). The ordinary performance target is not
met: that 42.6s final prefix, the 372s closure check and the initial 12m50s
library build total at least 1,184.6s before repair reruns and command overhead.
This is green local correctness evidence apart from the known global ratchet;
it is not a fresh client capture or authorized live runtime/DB acceptance.

**#31 direct-damage fidelity correction — 2026-09-19, integrated as
`a22e9390` by PR #1220:** the
target C++ `Spell::EffectPowerDrain` path calls
`Unit::SpellDamageBonusTaken(..., SPELL_DIRECT_DAMAGE)`
(`SpellEffects.cpp:1082-1088`), and `Unit::SpellDamageBonusTaken` returns the
input unchanged for `DIRECT_DAMAGE` before reading any damage-taken aura
(`Unit.cpp:6775-6777`). The recent Rust drain/burn path had incorrectly folded
victim, mechanic, cheat-death and caster damage-taken terms into this direct
effect. The integrated correction removes that taken-side fold, keeps only the
represented `SpellDamageBonusDone` pre-scaling, and changes the regressions to
prove aura terms are ignored for direct power effects. Periodic-only DOT and
Sanctified Wrath terms remain boundaries of the non-direct damage pipeline; they
are not a reason to extend the power-drain path or the threat-aura carrier. The
older #31 taken-term entries below are historical evidence of the mistaken route
and are superseded by this correction.

**#29 creature-attacker melee done bonus — 2026-09-19, implementation on the
current branch:** the creature-attacker path now applies the represented C++
`Unit::MeleeDamageBonusDone` terms before the victim taken/mitigation chain
(`Unit.cpp:7558-7650`): `MOD_DAMAGE_DONE_CREATURE`, creature-type melee AP
conversion, victim `MELEE_ATTACK_POWER_ATTACKER_BONUS`, `MOD_DAMAGE_DONE_VERSUS`,
`MOD_AUTOATTACK_DAMAGE`, and the victim aura-state/mechanic multipliers. The
player-victim path obtains the creature type from the production `ChrRaces.db2`
store; the creature-victim path uses the canonical creature template store. The
new regression proves `(10 + 5) * 2 = 30` damage through the production-shaped
creature tick and canonical health publication. Scope deliberately leaves the
C++ immunity registry/interrupt contract, negative crushing-band policy,
remaining absorb/split/physical-resistance fan-out, shapeshift creature-type
override, and live DB/relogin acceptance for the next #29 slice; this entry does
not close the macro issue. Validation on the candidate: `cargo check -p
world-server` passed in 187s; the exact new regression passed 1/1 in 37s; the
`scenarios_world_entities_32`, `scenarios_world_entities_28` and
`scenarios_combat_1` suites passed in the focused run; `cargo fmt --all --
--check`, `git diff --check`, the physical-file scan and its 20 self-tests pass.
`validation-v2 final --base origin/3.4.3 --timings` reached the hotspot ratchet
and stopped on the pre-existing baseline drift in `session/mod.rs`,
`handlers/character/mod.rs`, `world-server/src/lib.rs`, `handlers/quest/mod.rs`
and `wow-entities/src/player/mod.rs` (manifest
`20260919T093737.215152Z-55267-final.json`); no changed-file failure was hidden
or relabeled.

**#29 melee absorb-log visible-set fanout — 2026-09-19, implementation on the
current branch:** the victim session now keeps the direct `SpellAbsorbLog` copy
and sends the identical serialized packet through the existing player-visible
realm rail. This matches C++ `WorldObject::SendCombatLogMessage`
(`Object.cpp:1785-1794`), which delivers to the victim and then to nearby
visible players. The production-shaped melee absorb scenario now registers a
nearby observer and asserts the observer receives `ServerOpcodes::SpellAbsorbLog`
with the victim as the source, while the victim still logs before the exhausted
shield's `AuraUpdate`. This closes the absorb-log fan-out residual; mana-shield
power ownership, creature-victim mutable shields, split damage, `IMMUNITY_DAMAGE`,
negative crushing policy and live DB/relogin acceptance remain separate #29
boundaries.

**#29 physical melee resistance proof — 2026-09-19, implementation on the
current branch:** C++ `Unit::CalcSpellResistedDamage` exits with zero resisted
damage for any non-magic school (`Unit.cpp:1688-1693`). The production-shaped
creature-to-player melee scenario now applies a +100,000 generic resistance
aura and still requires the physical white swing to deal its full 10 damage.
This records the non-magic early-out at the real melee consumer without adding a
second resistance stage; split damage, `IMMUNITY_DAMAGE`, negative crushing
policy and live DB/relogin acceptance remain separate #29 boundaries.

**#29 creature-victim school absorb — 2026-09-19, implementation on the
current branch:** the map-owned creature melee path now projects a creature's
canonical `AppliedAuraRef` amount table into the same C++
`CalcAbsorbResist` school-absorb rules used for player victims. Each swing
spends the mutable amount, publishes `SpellAbsorbLog` and an exhausted-aura
`AuraUpdate` through the map event rail before `AttackerStateUpdate`, and the
next swing sees the remainder rather than the spell base value. The production
shaped regression covers three 10-point spends from a 30-point creature shield,
ordered log/removal/attack events, and the full hit after removal. Mana shields,
split damage, `IMMUNITY_DAMAGE`, negative crushing policy and live DB/relogin
acceptance remain separate boundaries.
Validation on this candidate: the exact `scenarios_world_entities_33` regression
passed 1/1; the `scenarios_world_entities_32` and `_28` focused suites passed;
`PROTOC=/home/ubuntu/.local/protoc/bin/protoc CARGO_BUILD_JOBS=1 cargo check
-p world-server`, `cargo fmt --all -- --check`, `git diff --check`, the physical
ratchet, and `session-ownership-check check --syntax-only` all pass. The general
architecture check and its self-test still stop on the unchanged pre-existing
hotspot ratchet in `session/mod.rs`, `handlers/character/mod.rs`,
`world-server/src/lib.rs`, `handlers/quest/mod.rs` and
`wow-entities/src/player/mod.rs`; this delivery does not touch those files or
the runtime-ownership ledger.

**#29 damage-immunity registry — 2026-09-19, implementation on the current
branch:** C++ `Unit::IsImmunedToDamage` (`Unit.cpp:7318-7336`) accepts both the
school-immunity and `IMMUNITY_DAMAGE` masks when the complete normal school is
covered. The production creature melee path now checks both aura types for
player and creature victims, so a damage-immunity aura blocks the swing before
the hit table just like school immunity. Existing school-immunity behavior is
unchanged; richer spell-immunity registry data, split damage, negative crushing
policy and live DB/relogin acceptance remain separate.
Validation on this candidate: `scenarios_world_entities_34` passed 1/1;
`scenarios_world_entities_32`, `_28` and `_33` passed; the production
`world-server` check, formatting, diff check, physical ratchet and
`session-ownership-check check --syntax-only` pass. The general architecture
check remains red only on the known hotspot ratchet; this slice adds bounded
scenario coverage under the existing Session aggregate but changes no
production hotspot authority or runtime-ownership ledger.

**#29 negative crushing-band fidelity — 2026-09-19, integrated as PR #1226:**
the melee outcome inputs now carry the creature-control and
`CREATURE_FLAG_EXTRA_NO_CRUSHING_BLOWS` gates from C++
`Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2364-2378`). The target 3.4.3 source
computes the crushing band with the literal expression
`attackerLevel - victimLevel * 1000 - 1500` (`Unit.cpp:2371`), which is negative
for ordinary eligible levels; Rust preserves that result and therefore does not
invent crushing blows or silently fix the target bug. The outcome switch and
wire presentation still carry the exact 150%/`HITINFO_CRUSHING` branch if a
future target-data case produces a positive raw band. Rules coverage proves the
negative eligible case, both gates, the 150% arithmetic and packet flag. The
split-damage block remains open because it needs a second canonical damage
target plus `DealDamage` ordering, immunity, combat-log and proc ownership;
live DB/relogin acceptance also remains open.
Validation on the merged candidate: the complete `scenarios_combat_4` suite (16/16),
the complete `scenarios_combat_5` suite (6/6, including the crushing gates),
the `wow-packet` library suite (754/754), `cargo check -p world-server`,
format/diff checks, the physical-file ratchet and the syntax-only
session-ownership check pass. `validation-v2 final --base origin/3.4.3
--timings` reached the unchanged hotspot ratchet and stopped at manifest
`20260919T111216.939112Z-95761-final.json`; none of the changed files is one of
the reported hotspot owners. No runtime/DB completion claim is made.

**#31 negative damage-taken aura regression — 2026-09-18, implementation
`6f1a5c02`, integrated by PR #1213:** test-only pin of the amount path the
drain/burn pre-scaling reads, added while attempting the Sanctified Wrath bypass
term (`Unit.cpp:6840-6854`), whose fixture behaved as if the bypass did nothing.
`creature_negative_damage_taken_aura_registers_a_reduction_like_cpp` proves that a
creature aura with base `-50` and matching misc bit registers its amount
unchanged and folds to exactly `0.5` in
`total_aura_multiplier_by_misc_mask_like_cpp`, so the creature-aura amount path
and multiplier fold are correct for reductions. That isolates the failed bypass
attempt to the *caster-side* aura application, and a follow-up diagnostic resolved
it: `apply_aura` (through `apply_aura_with_effect_mask_like_cpp`) inserts only the
Player-visible `AuraApplication` and syncs *threat-relevant* effect data into the
canonical Unit aura subsystem (`sync_canonical_threat_relevant_aura_like_cpp`), so
a +50 `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` aura reads back as
`total_aura_modifier_like_cpp(269) == 0`. Every canonical-subsystem consumer
(`total_aura_modifier_like_cpp`, `total_aura_multiplier_by_misc_mask_like_cpp`,
`aura_school_mask_like_cpp`, `has_aura_type_like_cpp`) is therefore blind to
ordinary player auras applied through that path. Two routes exist and both are
larger than a slice: extend the canonical sync from threat-relevant to all applied
effects (broad consumer impact, per-consumer review) or read the bypass term from
the Player-visible list (which needs a `ModIgnoreTargetResist` variant on
`RepresentedAuraEffectLikeCpp`). A first idea — adding type 269 to
`sync_canonical_threat_relevant_aura_like_cpp` — was inspected and rejected: that
sync is built on the threat-snapshot machinery
(`canonical_threat_aura_snapshot_for_difficulty_like_cpp`,
`AuraThreatSnapshotLikeCpp`) and is not the right carrier for an unrelated effect
type. The real options remain the two above: a general player-aura effect
registration in the canonical subsystem (faithful, but every canonical consumer
must be reviewed and its regressions re-run) or a Player-visible reader for the
bypass term with a new `ModIgnoreTargetResist` variant. Evidence: `wow-world --lib` 4026/0/1, `cargo fmt --all --check` and
`git diff --check` clean, physical ratchet PASS with no ceiling moved, ownership
syntax PASS with no baseline delta (test-only); `validation-v2 quick` PASS 103.4 s
(manifest `20260918T075849.424866Z-3904210-quick.json`) and `final
--architecture` 87.8 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS
and the pre-existing hotspot ratchet as the only red; the runner campaign is
191.2 s, inside the 600 s ordinary budget. No production behavior, packet layout
or ownership surface changed.

**#31 caster label damage-taken term — 2026-09-18, implementation `c11833e6`,
integrated as `7a40dba2` by PR #1211:** C++ `Unit::SpellDamageBonusTaken`
(`Unit.cpp:6830-6835`) multiplies the amount by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL)` over
the auras the damaging caster applied whose `MiscValue` is a label the damaging
spell carries (`SpellInfo::HasLabel`, `SpellInfo.h:598`); the represented model
had neither the aura-type constant nor a label authority at the effect site.
`SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL = 507` joins the aura-type
constants with its `SpellAuraEffects.cpp:578` anchor,
`SpellLabelStore::has_label_like_cpp` scans the store's own entries for a
`(spell_id, label_id)` pair, and the existing `SpellLabel.db2` loader is now
loaded at startup and carried through `SessionSpellCatalogCapabilitiesLikeCpp` to
a new session catalog slot (`set_spell_label_store`/`spell_label_store`), so
`power_drain_pre_scaled_damage_like_cpp` folds the label term for player and
creature victims alongside the other five. Coverage:
`spell_power_drain_applies_the_caster_label_damage_taken_aura_like_cpp` applies a
+50% aura of type 507 with misc label 7 from the draining player against a spell
carrying label 7 through `SpellLabel.db2`, draining base 100 at amplitude 0.5 to
150 (pool 50) with a 75 caster share. Evidence at `c11833e6`: `wow-data --lib`
754/0, `wow-world --lib` 4025/0/1, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS with two recorded ceiling bumps (`app.rs`
5666 to 5674 and `spell_db2.rs` to 239, reasons appended), ownership syntax PASS
after a reviewed `print-baseline` delta of exactly the two new session methods;
`validation-v2 quick` PASS 561.4 s (manifest
`20260918T073832.458445Z-3894381-quick.json`), where `path-06` alone took 550.4 s
because adding a field to the composition root invalidated the whole
`world-server`/`wow-packet`/`wow-world` test check graph under
`CARGO_BUILD_JOBS=1`, and `final --architecture` 84.9 s, exit 1, 2 of 9 steps with
`session-syntax-acceptance` PASS and the pre-existing hotspot ratchet as the only
red — **the runner campaign is 646.3 s, over the 600 s ordinary budget**, the same
dependency-forced overrun class as the earlier `wow-packet` and
`wow-entities`/`wow-data` rebuilds, with no step skipped, split or relabelled.
Boundaries: the DOT term and Sanctified Wrath of `SpellDamageBonusTaken` remain
unrepresented, the represented mechanic mask is still `u32` while C++ uses
`uint64`, and the label store is loaded but only this term consumes it so far.

**#31 damage-taken term stacking regression — 2026-09-18, implementation
`66c6bf3f`, integrated as `529c4dd6` by PR #1209:** test-only proof of the
composition semantics of the five produced `SpellDamageBonusTaken` terms. C++
`Unit::SpellDamageBonusTaken` (`Unit.cpp:6775-6840`) accumulates every term into a
single `TakenTotalMod` and scales `pdamage` once, so two +50% terms yield `225`
from a base of `100` rather than `200`;
`spell_power_drain_stacks_damage_taken_terms_multiplicatively_like_cpp` gives a
creature both a +50% school damage-taken aura and a +50% mechanic damage-taken
aura matching the drain spell's mechanic, draining base 100 at amplitude 0.5 to
225 from a 400-point pool (175 left) with a 112 caster share (137 mana), which an
additive implementation would fail. Evidence: `wow-world --lib` 4024/0/1,
`cargo fmt --all --check` and `git diff --check` clean, physical ratchet PASS with
no ceiling moved, ownership syntax PASS with no baseline delta (test-only);
`validation-v2 quick` PASS 38.2 s (manifest
`20260918T070125.697106Z-3878083-quick.json`) and `final --architecture` 84.5 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 122.7 s, inside the 600 s
ordinary budget. No production behavior, packet layout or ownership surface
changed.

**#31 caster spell-family damage-taken term — 2026-09-18, implementation
`424efd37`, integrated as `06d7f251` by PR #1207:** C++
`Unit::SpellDamageBonusTaken` (`Unit.cpp:6823-6828`) multiplies the amount by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER)` over the auras
the damaging caster applied whose spell affects the damaging spell through
`AuraEffect::IsAffectingSpell` (same `SpellFamilyName`, intersecting
`SpellFamilyFlags`). The module-level
`drain_taken_spell_from_caster_multiplier_like_cpp` reads the typed auras, filters
by caster, resolves each aura spell's `SpellClassOptions` entry (family set and
mask) and compares it with the damaging spell's entry, then folds
`1 + amount / 100`; the victim lookup applies it for player and creature victims
after the school, cheat-death, caster school-mask and mechanic terms. Coverage:
`spell_power_drain_applies_the_caster_spell_damage_taken_aura_like_cpp` gives both
the aura spell and the drain spell `SpellClassOptions` family 3 with mask `[0x1]`
and applies the +50% aura from the draining player, draining base 100 at amplitude
0.5 to 150 (pool 50) with a 75 caster share. Evidence at `424efd37`: `wow-world
--lib` 4023/0/1, `cargo fmt --all --check` and `git diff --check` clean, physical
ratchet PASS with no ceiling moved, ownership syntax PASS with no baseline delta
(module-level helper); `validation-v2 quick` PASS 52.4 s (manifest
`20260918T065152.581064Z-3871080-quick.json`) and `final --architecture` 84.2 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 136.6 s, inside the 600 s
ordinary budget. Boundaries:
`SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL` needs spell labels and remains
unrepresented, as do the DOT term and Sanctified Wrath, and the represented
mechanic mask is still `u32` while C++ uses `uint64`.

**#31 mechanic damage-taken term — 2026-09-18, implementation `7c32137d`,
integrated as `e623b648` by PR #1205:** C++ `Unit::SpellDamageBonusTaken`
(`Unit.cpp:6783-6791`) multiplies the amount by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT)` over the
auras whose misc value intersects `SpellInfo::GetAllEffectsMechanicMask()`, and
that mask needs each effect's `Mechanic`, which the represented `SpellEffectInfo`
did not carry. `SpellEffectInfo` gains `effect_mechanic`, mapped from the
already-loaded `SpellEffectDb2Entry::effect_mechanic` in
`spell_effect_from_db2_like_cpp` — no new DB2 read or field index was invented —
and `SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT = 255` joins the aura-type
constants with its `SpellAuraEffects.cpp:326` anchor.
`power_drain_pre_scaled_damage_like_cpp` builds the mask from the spell's effects
(mechanic indices below 32, the represented `u32` bound) and folds the victim's
mechanic auras for both player and creature victims. Coverage:
`spell_power_drain_applies_the_mechanic_damage_taken_aura_like_cpp` gives the
drain spell `effect_mechanic = 5` and the victim a +50% aura of type 255 with misc
`1 << 5`, draining base 100 at amplitude 0.5 to 150 (pool 50) with a 75 caster
share. Evidence at `7c32137d`: `wow-data --lib` 754/0, `wow-world --lib` 4022/0/1,
`cargo fmt --all --check` and `git diff --check` clean, physical ratchet PASS with
no ceiling moved, ownership syntax PASS with no baseline delta; `validation-v2
quick` PASS 77.9 s (manifest `20260918T064131.850771Z-3863900-quick.json`) and
`final --architecture` 84.7 s, exit 1, 2 of 9 steps with
`session-syntax-acceptance` PASS and the pre-existing hotspot ratchet as the only
red; the runner campaign is 162.6 s, inside the 600 s ordinary budget. Boundaries:
the represented mechanic mask is `u32` while the C++ mask is `uint64`, so a
mechanic index of 32 or above cannot be represented, and
`SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER`,
`SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL`, the DOT term and Sanctified
Wrath remain unrepresented.

**#31 caster school-mask damage-taken term — 2026-09-18, implementation
`f832c5f0`, integrated as `02f0d08b` by PR #1203:** C++
`Unit::SpellDamageBonusTaken` (`Unit.cpp:6815-6822`) multiplies the amount by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER)` over the
auras the damaging caster applied whose misc value intersects the spell's school
mask. The module-level
`drain_taken_school_from_caster_multiplier_like_cpp` walks the victim's typed
auras of that type, keeps the ones whose `caster_guid` is the damaging caster and
whose misc value intersects the school mask, and folds `1 + amount / 100`; the
victim lookup applies it for both player and creature victims after the school and
cheat-death terms. Coverage:
`spell_power_drain_applies_the_caster_school_damage_taken_aura_like_cpp` gives a
creature a +50% caster-school aura applied by the draining player (misc bit 0) and
drains base 100 at amplitude 0.5, ending at 50 pool (`int32(100 * 1.5) = 150`
drained) and 100 caster mana (`int32(150 * 0.5) = 75`), while all existing
drain/burn scenarios and the other two produced terms keep passing. Evidence at
`f832c5f0`: `wow-world --lib` 4021/0/1, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS with no ceiling moved, ownership syntax PASS
with no baseline delta (module-level helper); `validation-v2 quick` PASS 41.2 s
(manifest `20260918T062935.408007Z-3854851-quick.json`) and `final --architecture`
86.5 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the
pre-existing hotspot ratchet as the only red; the runner campaign is 127.7 s,
inside the 600 s ordinary budget. Boundaries:
`SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER` and
`SPELL_AURA_MOD_DAMAGE_TAKEN_FROM_CASTER_BY_LABEL` need C++
`AuraEffect::IsAffectingSpell`/label relations and remain unrepresented, as do the
mechanic-mask (needs an `effect_mechanic` field and its aura-type constant), DOT
and Sanctified Wrath terms.

**#31 cheat-death damage-taken term — 2026-09-18, implementation `27a544be`,
integrated as `dd92f76b` by PR #1201:** C++ `Unit::SpellDamageBonusTaken`
(`Unit.cpp:6793-6795`) adds the amount of the aura on spell `45182`
(`GetAuraEffect(45182, EFFECT_0)`) as a percentage when that aura's misc value
intersects `SPELL_SCHOOL_MASK_NORMAL`, independently of the school
`MOD_DAMAGE_PERCENT_TAKEN` term. The module-level
`drain_taken_cheat_death_multiplier_like_cpp` reads the victim's aura subsystem
for spell `45182`, checks the normal-school misc bit and returns
`1 + amount / 100`; the victim lookup in
`power_drain_pre_scaled_damage_like_cpp` multiplies it with the school term for
both the player and creature victim paths. Coverage:
`spell_power_drain_applies_the_cheat_death_taken_term_like_cpp` gives a creature
a +20% `45182` aura through the creature-aura path with misc bit 0 (type
`SPELL_AURA_MOD_DAMAGE_DONE`, so the school term stays 1.0) and drains base 100 at
amplitude 0.5, ending at 80 pool (`int32(100 * 1.2) = 120` drained) and 85 caster
mana (`int32(120 * 0.5) = 60`). Evidence at `27a544be`: `wow-world --lib`
4020/0/1, `cargo fmt --all --check` and `git diff --check` clean, physical ratchet
PASS with no ceiling moved, ownership syntax PASS with no baseline delta
(module-level helper); `validation-v2 quick` PASS 42.6 s (manifest
`20260918T062033.025181Z-3847805-quick.json`) and `final --architecture` 84.6 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 127.2 s, inside the 600 s
ordinary budget. Boundaries: the mechanic-mask, caster-specific aura types, DOT
and Sanctified Wrath terms of `SpellDamageBonusTaken` remain unrepresented, and
the mechanic term additionally needs an `effect_mechanic` field on
`SpellEffectInfo` plus the `SPELL_AURA_MOD_MECHANIC_DAMAGE_TAKEN_PERCENT`
aura-type constant before it can be produced.

**#31 victim school damage-taken term — 2026-09-18, implementation `c3f86243`,
integrated as `eff6323c` by PR #1199:** C++ `Unit::SpellDamageBonusTaken`
(`Unit.cpp:6775-6820`) multiplies the pre-scaled amount by
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
spellProto->GetSchoolMask())`, skipped when the spell carries
`SPELL_ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS` (`SharedDefines.h:587`,
`0x0000_0100`); the produced effect was missing the victim's damage-taken auras,
so a +50% debuff on the victim did not grow the drained amount.
`SPELL_ATTR4_IGNORE_DAMAGE_TAKEN_MODIFIERS` joins the wow-data attribute
constants with its anchor, and `power_drain_pre_scaled_damage_like_cpp` applies
that multiplier from the victim's aura subsystem (player or creature) after the
`Done` scaling and the attribute gate. Coverage:
`spell_power_drain_applies_the_victim_school_damage_taken_aura_like_cpp` gives a
creature a +50% `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` aura through the
creature-aura path and drains base 100 at amplitude 0.5, ending at 50 pool
(`int32(100 * 1.5) = 150` drained) and 100 caster mana (`int32(150 * 0.5) = 75`),
while every existing drain/burn scenario keeps passing with the implicit 1.0
multiplier. Evidence at `c3f86243`: `wow-world --lib` 4019/0/1, `wow-data --lib`
754/0, `cargo fmt --all --check` and `git diff --check` clean, physical ratchet
PASS with one recorded ceiling bump (`wow-data/src/spell/mod.rs` 133 to 136,
reason appended), ownership syntax PASS after a reviewed `print-baseline` delta of
exactly the changed helper receiver; `validation-v2 quick` PASS 66.5 s (manifest
`20260918T061107.158168Z-3840558-quick.json`) and `final --architecture` 85.6 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 152.1 s, inside the 600 s
ordinary budget. Boundaries: the mechanic-mask, cheat-death (`45182`),
caster-specific aura types, DOT and Sanctified Wrath terms of
`SpellDamageBonusTaken` remain unrepresented — only the school
`MOD_DAMAGE_PERCENT_TAKEN` term and its attribute gate are produced — and the
helper keeps its documented `Done`-side limits.

**#31 drain/burn `SpellDamageBonusDone` pre-scaling — 2026-09-18, implementation
`482bc9f7`, integrated as `956d43cf` by PR #1197:** C++
`Spell::EffectPowerDrain`/`EffectPowerBurn` (`SpellEffects.cpp:1082-1088`) call
`Unit::SpellDamageBonusDone(..., SPELL_DIRECT_DAMAGE, ...)` on the effect amount
before draining it, so spell power and its coefficient grow the drained pool and
the caster's share; the represented path used the raw `effect_base_points`.
`power_drain_pre_scaled_damage_like_cpp` routes a non-negative base through the
existing `represented_spell_damage_bonus_done_like_cpp` (the `damage < 0` gate
runs first in C++, so a negative base keeps its value for the effect's own
refusal), and both the drain and burn call sites pass the pre-scaled amount, which
feeds the pool drain, the take-power row, the burn damage and the caster gain.
Coverage: `spell_power_drain_pre_scales_with_spell_damage_bonus_done_like_cpp`
runs spell power 100, `BonusCoefficient` 0.5, base 100 and amplitude 0.5 against a
200-point creature pool, ending at 50 pool and 100 caster mana
(`int32(100 + int32(100 * 0.5)) = 150` drained, `int32(150 * 0.5) = 75` share) —
without the scaling the assertions fail, so the test proves the integration — and
every existing drain/burn scenario (self-drain, mismatched power, empty pool, gate
refusal, zero-amplitude burn, creature burn) still passes unchanged. Evidence at
`482bc9f7`: `wow-world --lib` 4018/0/1, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS with no ceiling moved, ownership syntax PASS
after a reviewed `print-baseline` delta of exactly the new helper; `validation-v2
quick` PASS 62.3 s (manifest `20260918T055903.110763Z-3831043-quick.json`) and
`final --architecture` 84.4 s, exit 1, 2 of 9 steps with
`session-syntax-acceptance` PASS and the pre-existing hotspot ratchet as the only
red; the runner campaign is 146.7 s, inside the 600 s ordinary budget. Boundaries:
`SpellDamageBonusTaken` has no represented producer yet, so the taken half of
C++'s pre-scaling (`SpellEffects.cpp:1087`) stays unrepresented and recorded, and
the helper's own documented limits apply (family-scripted terms, one
`BonusCoefficient` per spell, creature casters keep the raw value).

**#31 remaining execute-log list layouts — 2026-09-18, implementation
`6dda6e3c`, integrated as `6e821bad` by PR #1195:** test-only byte-level coverage
for the execute-log row families that still have no producer.
`WorldPackets::CombatLog::SpellExecuteLog::Write` (`CombatLogPackets.cpp:90-155`)
serialises six families per effect; the existing writer test covered the
take-power and extra-attacks rows (plus the durability row indirectly through a
world scenario). The new
`spell_execute_log_writes_the_durability_generic_trade_and_feed_lists` puts all
four remaining families on one effect and decodes them field by field after the
six counts — `(Victim, int32 ItemID, int32 Amount)`, `(Victim)`, `int32 ItemID`,
`int32 ItemID` — plus the trailing log-data bit and no leftover bytes. Evidence:
`wow-packet --lib` 754/0, `cargo fmt --all --check` and `git diff --check` clean,
physical ratchet PASS with no ceiling moved, ownership syntax PASS with no
baseline delta (test-only); `validation-v2 quick` PASS 17.1 s (manifest
`20260918T054504.280356Z-3822240-quick.json`) and `final --architecture` 83.8 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 100.9 s, inside the 600 s
ordinary budget. No production behavior, packet layout or ownership surface
changed.

**#31 zero-pool drain log repair — 2026-09-18, implementation `54263163`,
integrated as `e2af51e0` by PR #1193:** C++ `Spell::EffectPowerDrain`
(`SpellEffects.cpp:1069-1102`) computes
`newDamage = -(unitTarget->ModifyPower(powerType, -damage))` and then, for any
target that passed the `IsAlive()`/`GetPowerType()` gate, calls
`ExecuteLogEffectTakeTargetPower(..., newDamage, ...)` and — when the caster is
not the target — `EnergizeBySpell(caster, spellInfo, gain, powerType)`
unconditionally (`:1090-1101`), so an empty pool still produces a take-power row
with `points 0` and a zeroed energize log. The represented path returned early
whenever the drained amount was zero, so a valid-but-empty target published
nothing. Both branches now distinguish a gate refusal (dead target, mismatched
`DisplayPower`, negative damage — still silent) from a valid zero-drain, which
logs the zeroed rows and still calls the energize path for a non-self caster.
Coverage: `spell_power_drain_on_an_empty_creature_pool_logs_zero_like_cpp` drains
a creature with 0 of 100 mana and matching `DisplayPower` for 15, asserting the
caster gains nothing and the cast publishes
`[SpellGo, SpellEnergizeLog, SpellExecuteLog, CooldownEvent]` with the decoded
energize `amount 0`/`over 0`, while the mismatch and negative-amount scenarios
still publish `[SpellGo, CooldownEvent]`. The new scenario lives in the new
terminal-limit-bounded `session/tests/scenarios_spell_state_26.rs` (split out of
`scenarios_spell_state_12.rs`, which had reached the 2,000-line test-file limit),
with the two inseparable module-registration lines and the recorded
`session_tests.rs` ceiling bump in the physical policy. Evidence at `54263163`:
`wow-world --lib` 4017/0/1, `cargo fmt --all --check` and `git diff --check`
clean, physical ratchet PASS (2235 files, one recorded ceiling bump), ownership
syntax PASS with no baseline delta; `validation-v2 quick` PASS 43.4 s (manifest
`20260918T053713.736131Z-3818986.six`-family run) and `final --architecture`
86.7 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the
pre-existing hotspot ratchet as the only red; the runner campaign is 130.1 s,
inside the 600 s ordinary budget. Boundaries: the player self-drain path still
publishes no caster gain (C++ skips it when caster == target) and keeps its
existing burn damage handling; drain/burn still skip C++'s
`SpellDamageBonusDone/Taken` pre-scaling (`SpellEffects.cpp:1082-1088`).

**#31 zero-amplitude creature burn regression — 2026-09-18, implementation
`4c0f67e9`, integrated as `413f379e` by PR #1191:** test-only coverage for the
`CalcValueMultiplier` semantics PR #1185 landed. C++
`Spell::EffectPowerBurn` (`SpellEffects.cpp:1157-1164`) scales the burned health
by the effect amplitude with no implicit 1.0 default, so an amplitude-0 burn still
drains the pool and still logs the take-power row while dealing no damage.
`spell_power_burn_on_a_creature_with_zero_amplitude_deals_no_damage_like_cpp`
burns a creature with 40 of 100 mana and 100 health for 15 at the default 0.0
amplitude, asserting the pool drops to 25, the health stays 100 and the decoded
execute-log row carries points 15 with amplitude 0.0. Evidence at `4c0f67e9`:
`wow-world --lib` 4016/0/1, `cargo fmt --all --check` and `git diff --check`
clean, physical ratchet PASS with no ceiling moved, ownership syntax PASS with no
baseline delta (test-only); `validation-v2 quick` PASS 32.9 s (manifest
`20260918T052241.497206Z-3806015-quick.json`) and `final --architecture` 86.8 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 119.7 s, inside the 600 s
ordinary budget. No production behavior, packet layout or ownership surface
changed.

**#31 creature power-type drain regression — 2026-09-18, implementation
`bae3875a`, integrated as `8afca1dd` by PR #1189:** test-only regression for the
gate PR #1183 landed: C++ `Spell::EffectPowerDrain` returns before touching the
pool when the target's `GetPowerType()` differs from the effect's power
(`SpellEffects.cpp:1078`). The new
`spell_power_drain_on_a_creature_requires_the_matching_power_type_like_cpp` drains
Mana from a creature whose `DisplayPower` is Energy, asserting the pool stays 40,
the caster restores nothing and the cast publishes only
`[SpellGo, CooldownEvent]` — no energize log and no execute log. Evidence at
`bae3875a`: `wow-world --lib` 4015/0/1, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS with no ceiling moved, ownership syntax PASS
with no baseline delta (test-only); `validation-v2 quick` PASS 29.2 s (manifest
`20260918T051420.466809Z-3799184-quick.json`) and `final --architecture` 85.6 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 114.8 s, inside the 600 s
ordinary budget. No production behavior, packet layout or ownership surface
changed.

**#31 drained-creature power publication — 2026-09-18, implementation
`e7943b27`, integrated as `f407cda6` by PR #1187:** C++ drains a creature's power
through `Unit::ModifyPower`, which writes the `UNIT_FIELD_POWER*` data field the
client receives over the normal values-update rail; the represented drain mutated
the canonical creature pool without publishing it, so the drained power never
reached the client. The creature drain branch now captures
`Unit::values_update()` from the same canonical mutation and publishes it through
`represented_unit_values_update_to_update_object_like_cpp` — the mechanism the
represented creature heal already uses for its health change — for a creature the
session can see. Coverage:
`spell_power_burn_on_a_creature_applies_the_scaled_damage_like_cpp` now also
asserts the cast's opcode list contains `UpdateObject` (the drained power field)
and `SpellExecuteLog` (the take-power row), with the pool and health assertions
unchanged, and the creature drain, player drain/burn, self-drain, mismatched-power
and negative-amount scenarios still pass. Evidence at `e7943b27`: `wow-world
--lib` 4014/0/1, `cargo fmt --all --check` and `git diff --check` clean, physical
ratchet PASS with no ceiling moved, `session-ownership-check --syntax-only` PASS
with no baseline delta (no `WorldSession` surface changed); `validation-v2 quick`
PASS 42.9 s (manifest `20260918T050307.904865Z-3791776-quick.json`) and `final
--architecture` 83.2 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS
and the pre-existing hotspot ratchet as the only red; the runner campaign is
126.1 s, inside the 600 s ordinary budget. Boundaries: the player target's own
power change keeps its existing representation (the energize log plus the session's
normal player values publication), and the publication reaches the session's own
client when it can see the creature, matching the represented creature-heal
pattern, so the observer broadcast of that values update remains the session-local
rail.

**#31 creature power burn through the damage path — 2026-09-18, implementation
`ee80e538`, integrated as `51c6ae5a` by PR #1185:** C++
`Spell::EffectPowerBurn` (`SpellEffects.cpp:1142-1165`) drains the target's power
and adds `int32(drained * SpellEffectInfo::CalcValueMultiplier)` to `m_damage`,
which the spell applies through the normal damage pipeline; PR #1183 delivered the
creature drain but left the creature burn a no-op because that damage needs the
represented creature damage path, which is async and needs the cast identity.
`apply_power_drain_effect_like_cpp` is now async and takes the item-GUID
generator plus `cast_id`/`spell_visual_id`, and a creature target with
`burn_damage` routes the scaled amount through `apply_damage_from_caster_like_cpp`
— the same path `SPELL_EFFECT_SCHOOL_DAMAGE` uses — so the damage lands with its
combat log, tap and kill-cascade behavior; the drain-on-creature and both
player-target branches are unchanged. Coverage:
`spell_power_burn_on_a_creature_applies_the_scaled_damage_like_cpp` burns a
creature with 40 of 100 mana and 100 health for 15 at amplitude 1.0, asserting the
pool drops to 25 and the health to 85, alongside the unchanged creature-drain and
player drain/burn scenarios. Evidence at `ee80e538`: `wow-world --lib` 4014/0/1,
`cargo fmt --all --check` and `git diff --check` clean, physical ratchet PASS with
no ceiling moved, `session-ownership-check --syntax-only` PASS after a reviewed
`print-baseline` delta of exactly the changed drain signature (225 production +
429 fixture fields, 3928 associated items); `validation-v2 quick` PASS 43.7 s
(manifest `20260918T044904.085914Z-3783747-quick.json`) and `final --architecture`
85.2 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the
pre-existing hotspot ratchet as the only red; the runner campaign is 128.9 s,
inside the 600 s ordinary budget. Boundaries: the creature's power change is still
not published to observers (no represented creature power update-field writer)
while the health damage uses the existing creature damage path, and drain/burn
still skip C++'s `SpellDamageBonusDone/Taken` pre-scaling
(`SpellEffects.cpp:1082-1088`).

**#31 creature-target power drain — 2026-09-18, implementation `5cce3a15`,
integrated as `c2405506` by PR #1183:** C++ `Spell::EffectPowerDrain`
(`SpellEffects.cpp:1069-1102`) accepts any living target whose `GetPowerType()`
matches the effect (`:1078`), drains its pool and, when `unitCaster != unitTarget`,
restores `drained * SpellEffectInfo::CalcValueMultiplier` through
`Unit::EnergizeBySpell` (`:1094-1100`); the represented path only accepted the
canonical player, so a player draining a creature did nothing. The drain effect
now takes a creature target, gating on the creature being alive with a matching
`DisplayPower`, drains the canonical creature pool through
`mutate_canonical_creature_by_guid_like_cpp`, records the
`ExecuteLogEffectTakeTargetPower` row and grants the caster's share through the
represented `EnergizeBySpell` path, which owns the energize log, assisting threat
and regen interrupt landed in the previous rounds; the player path keeps C++'s
no-gain-on-self-drain rule. Coverage:
`spell_power_drain_on_a_creature_restores_the_caster_share_like_cpp` drains 15 of
a creature's 40-point pool with amplitude 0.5, asserts the caster gains
`int32(15 * 0.5) = 7` and decodes the
`[SpellGo, SpellEnergizeLog, SpellExecuteLog, CooldownEvent]` sequence with the
take-power row (victim = creature, points 15, power type 0, amplitude 0.5); the
self-drain, mismatched-power, negative-amount and burn scenarios still pass
unchanged. Evidence at `5cce3a15`: `wow-world --lib` 4013/0/1, `cargo fmt --all
--check` and `git diff --check` clean, physical ratchet PASS with no ceiling
moved, `session-ownership-check --syntax-only` PASS with no baseline delta (no
`WorldSession` surface changed); `validation-v2 quick` PASS 41.0 s (manifest
`20260918T043131.994643Z-3769494-quick.json`) and `final --architecture` 83.4 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 124.4 s, inside the 600 s
ordinary budget. Boundaries: `EffectPowerBurn` on a creature stays a no-op
because C++ accumulates that damage into the spell damage pipeline, which the
represented chain applies to player victims only; the creature's power change is
not published to observers yet (no represented creature power update-field
writer), so the server-side pool, the caster share and the logs are what this
slice delivers; the creature target still skips C++'s
`SpellDamageBonusDone/Taken` pre-scaling (`SpellEffects.cpp:1082-1088`).

**#31 durability rows in the spell execute log — 2026-09-18, implementation
`d8688e97`, integrated as `2b7d2449` by PR #1181:** C++
`Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4314-4341`) calls
`Spell::ExecuteLogEffectDurabilityDamage` (`Spell.cpp:5108-5116`) with `(-1, -1)`
for the all-items branch (`SpellEffects.cpp:4328`) and
`(item->GetEntry(), slot)` for the equipped-slot branch
(`SpellEffects.cpp:4339`); `EffectDurabilityDamagePCT` has no execute-log call.
The represented durability effect now takes its effect id and fills the
`durability_damage_targets` list of the `SpellLogEffect` the PR #1179 writer
already serializes, recording the all-items row unconditionally and the item row
only after the represented loss found and changed the equipped item. Coverage:
`durability_damage_spell_effect_reduces_equipped_items_like_cpp` now asserts the
`[SpellGo, SpellExecuteLog, CooldownEvent]` sequence and decodes the `-1/-1` row,
and `durability_damage_spell_effect_logs_the_item_entry_and_slot_like_cpp` equips
entry 300 in the mainhand, drains 7 points and decodes the `(300, slot)` row
through the shared `durability_execute_log_row_like_cpp` decoder, which checks the
six effect counts before the row. Evidence at `d8688e97`: `wow-world --lib`
4012/0/1, `cargo fmt --all --check` and `git diff --check` clean, physical ratchet
PASS with no ceiling moved, `session-ownership-check --syntax-only` PASS after a
reviewed `print-baseline` delta (one changed signature plus the new recorder; 225
production + 429 fixture fields, 3928 associated items); `validation-v2 quick`
PASS 62.6 s (manifest `20260918T041851.085493Z-3761517-quick.json`) and `final
--architecture` 83.8 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS
and the pre-existing hotspot ratchet as the only red; the runner campaign is
146.4 s, inside the 600 s ordinary budget. Boundaries: the generic-victim,
trade-skill and feed-pet lists still have no producer, and the percent variant
emits no row because C++ emits none.

**#31 spell execute log with take-power entries — 2026-09-18, implementation
`1e1b3655`, integrated as `adefbfe9` by PR #1179:** C++
`Spell::FinishTargetProcessing` (`Spell.cpp:8493-8496`) calls
`Spell::SendSpellExecuteLog` (`Spell.cpp:5048-5060`), which publishes
`SMSG_SPELL_EXECUTE_LOG` with every effect's `_executeLogEffects` entry, but the
represented chain had no such packet. `wow-packet` gained `SpellExecuteLog` with
`SpellLogEffect` and its six row families (`Spell.h:165-213`), written exactly
like `CombatLogPackets.cpp:90-155` (caster, `int32(SpellID)`, effect count, per
effect its id and six list counts, then each list's rows, closing with the basic
packet's log-data bit). The session owns a per-cast accumulator
(`represented_spell_execute_log_effects_like_cpp`) cleared at cast start and
taken by `send_spell_execute_log_like_cpp` after the effect loop, mirroring the
C++ `_executeLogEffects` lifetime; `represented_spell_execute_log_effect_like_cpp`
reproduces `GetExecuteLogEffect`'s find-or-create. Producers:
`ExecuteLogEffectTakeTargetPower` (`Spell.cpp:5076-5086`) from `EffectPowerDrain`
(amplitude = `CalcValueMultiplier`, `SpellEffects.cpp:1101`) and
`EffectPowerBurn` (amplitude `0.0f`, drained power logged before the multiplier,
`SpellEffects.cpp:1160`), and `ExecuteLogEffectExtraAttacks`
(`Spell.cpp:5088-5095`) from `EffectAddExtraAttacks`. The same change repairs a
recorded placeholder with C++ evidence: `EffectPowerBurn` now scales the drained
power by `SpellEffectInfo::CalcValueMultiplier` (`SpellEffects.cpp:1157-1164`)
instead of the previous multiplier 1.0, with the effect amplitude carried through
the cast loop's effect tuple. Coverage:
`spell_execute_log_writes_cpp_effect_lists` (writer shape for two effects and both
list kinds); `spell_power_drain_publishes_take_target_power_execute_log_like_cpp`
(15 drained of 25 mana with amplitude 0.5 logs points 15, power type 0,
amplitude 0.5); `spell_power_burn_scales_damage_by_the_value_multiplier_like_cpp`
(amplitude 0.5 burns `int32(15 * 0.5) = 7` health while the logged amplitude stays
`0.0f`); and the extra-attacks scenario decodes the `ExtraAttacksTargets` row
(victim = the spell's `unitTarget`, 2 attacks), while the amplitude-1.0 burn keeps
its previous 15 damage and the mismatched/negative drain cases still publish no
execute log. Evidence at `1e1b3655`: `wow-packet --lib` 753/0, `wow-world --lib`
4011/0/1, `cargo fmt --all --check` and `git diff --check` clean, physical ratchet
PASS with one recorded ceiling bump (`session/mod.rs` 18992 to 18997, reason
appended to the entry's `split`), `session-ownership-check --syntax-only` PASS
after a reviewed `print-baseline` delta (the new field, three new methods and two
changed signatures; 225 production + 429 fixture fields, 3927 associated items);
`validation-v2 quick` PASS 139.7 s (manifest
`20260918T040425.959549Z-3753317-quick.json`) and `final --architecture` 87.2 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 226.9 s, inside the 600 s
ordinary budget. Boundaries: only the take-power and extra-attacks lists have
producers (the other four are written as empty counts, like C++'s absent
`Optional` vectors); `EffectPowerDrain`'s target and caster are still the canonical
player, so C++'s `unitCaster != unitTarget` `EnergizeBySpell` gain has no producer
and drain/burn still skip the `SpellDamageBonusDone/Taken` pre-scaling
(`SpellEffects.cpp:1082-1088`); the packet carries the basic variant like the
other represented combat logs.

**#31 flagged-power regen interrupt on energize — 2026-09-18, implementation
`1e89ffa8`, integrated as `2c6e8966` by PR #1177:** C++
`Unit::EnergizeBySpell` (`Unit.cpp:6578-6590`) first calls
`Player::InterruptPowerRegen` (`Player.cpp:1831-1840`) when the target power's DB2
`PowerTypeEntry` carries `PowerTypeFlags::UseRegenInterrupt`
(`DBCEnums.h:1796-1810`, `0x0002`), resetting `m_regenInterruptTimestamp` and
`m_powerFraction[powerIndex]` and publishing `SMSG_INTERRUPT_POWER_REGEN`. The
represented regen tick already consumed `use_regen_interrupt`, but nothing ever
set the interrupt, so a flagged power kept regenerating through an energize.
`PowerTypeStore::uses_regen_interrupt_like_cpp` now owns the flag rule (missing
entry false, like C++'s null `GetPowerTypeEntry`) and
`POWER_TYPE_FLAG_USE_REGEN_INTERRUPT_LIKE_CPP` is public with its anchor;
`regeneration.rs` reads that helper instead of its private copy of the constant,
so the gate and the trigger cannot drift. `wow-packet` gained the
`InterruptPowerRegen` writer matching `CombatPackets.cpp:117-122`
(`int32(PowerType)` only). The previously `#[cfg(test)]` `power_type_store` slot
is now a production read-only session handle (`set_power_type_store` /
`power_type_store_like_cpp`) installed from the composition root, with the
already-loaded `PowerTypeStore` attached to
`SessionPlayerCatalogCapabilitiesLikeCpp`, so the session reads the same
process-owned DB2 store the spawn and regeneration paths use;
`apply_energize_effect_like_cpp` interrupts before the power change and publishes
the packet before `SpellEnergizeLog`, matching C++ order. Coverage:
`interrupt_power_regen_writes_only_the_power_type_like_cpp` (writer shape),
`use_regen_interrupt_reads_the_db2_flag_like_cpp` (set, clear, missing entry) and
`spell_energize_interrupts_flagged_power_regen_like_cpp`, where the flagged power
produces `[SpellGo, InterruptPowerRegen, SpellEnergizeLog, CooldownEvent]`, the
unflagged one `[SpellGo, SpellEnergizeLog, CooldownEvent]`, and the power still
lands (25 to 75 mana). Evidence at `1e89ffa8`: `wow-data --lib` 754/0,
`wow-packet --lib` 752/0, `wow-world --lib` 4009/0/1, `cargo fmt --all --check`
and `git diff --check` clean, physical ratchet PASS with one recorded ceiling
bump (`world-server/src/app.rs` 5665 to 5666, reason appended to the entry's
`split`), `session-ownership-check --syntax-only` PASS after a reviewed
`print-baseline` delta (`set_power_type_store` moves from test fixture to
production, the `WorldSession` bridge surface fingerprint follows the field, and
`interrupt_player_power_regen_like_cpp` plus `power_type_store_like_cpp` appear;
224 production + 429 fixture fields, 3923 associated items); `validation-v2 quick`
PASS 93.3 s (manifest `20260918T034220.005229Z-3740769-quick.json`) and `final
--architecture` 84.5 s, exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS
and the pre-existing hotspot ratchet as the only red; the runner campaign is
177.8 s, inside the 600 s ordinary budget. Boundaries: the trigger runs only for
the represented self-energize target, so a creature caster or target still has no
energized-regen state to interrupt.

**#31 `EnergizeBySpell` casing and assisting threat — 2026-09-18,
implementation `fb3aabdb` (tested tree `6b72eb84`), integrated as `7c070d0e` by
PR #1175:** three pieces of C++ `Spell::EffectEnergize`
(`SpellEffects.cpp:1488-1530`) and `Unit::EnergizeBySpell` (`Unit.cpp:6578-6590`)
were outside the represented slice. The `EffectEnergize` switch runs before the
power change: Blood Fury (24571) subtracts `10 * max(0, min(30, level - 60))`,
Burst of Energy (24532) subtracts `4 * max(0, min(15, level - 60))`, and the
Runic Mana Injector (67490) adds `AddPct(damage, 25)` when the caster Player has
`SKILL_ENGINEERING` (`SharedDefines.h:5388`, `Player::HasSkill`,
`Player.cpp:5880-5887`); `energize_caster_scaled_amount_like_cpp` reproduces all
three and the `EffectEnergizePct` branch keeps its unscaled percentage.
`EnergizeBySpell` also forwards `damage / 2` through
`ThreatManager::ForwardThreatForAssistingMe(..., ignoreModifiers = true)` before
the log, so the forwarding body the heal path alone had became
`forward_assisting_threat_like_cpp` with `ignore_modifiers` mirroring the C++
parameter — heals keep `false` (`Spell.cpp:2883`, percentage and school-threat
multiplier applied) and `forward_heal_threat_like_cpp` is now a thin wrapper, so
heal behavior is unchanged. Cast-level `Spell::HandleThreatSpells`
(`Spell.cpp:5599-5601`) still applies on top, exactly as C++ runs both.
Coverage: level-80 Blood Fury turns 300 into 100 mana and Burst of Energy 100
into 40 energy; the Runic Mana Injector gives 100 without a skill record and 125
with `SKILL_ENGINEERING`; a creature threatening the energizing player ends with
10 existing + 8 cast-level + 25 forwarded (`50 / 2`, not the `pct_mod 2.0`-doubled
50), asserted on both the legacy facade and the canonical creature; the existing
`spell_self_heal_adds_half_effective_heal_threat_like_cpp` regression still
passes. Evidence at the identical tree of `6b72eb84`/`fb3aabdb`: `wow-entities
--lib` 941/0, `wow-world --lib` 4008/0/1, `cargo fmt --all --check` and
`git diff --check` clean, physical ratchet PASS (2234 files, no ceiling moved),
`session-ownership-check --syntax-only` PASS after a reviewed `print-baseline`
delta that adds exactly the two new methods (3921 exact associated items);
`validation-v2 quick` PASS 80.8 s (manifest
`20260918T030827.726019Z-3724047-quick.json`) and `final --architecture` 89.1 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the runner campaign is 169.9 s, inside the 600 s
ordinary budget. Landing note: the change was first pushed directly to `3.4.3`
by mistake as `6b72eb84`; it was reverted on the integration branch (`a59c08fb`)
and re-landed through the required branch/PR path, and the re-landed commit has
the byte-identical tree (`9c4b607f`), so the evidence applies to the merged
candidate. Boundaries: the `PowerTypeFlags::UseRegenInterrupt` branch
(`Player::InterruptPowerRegen` plus `SMSG_INTERRUPT_POWER_REGEN`) still needs the
DB2 `PowerType` entry at the effect site, because the trigger would require
threading that production store through the spell-effect chain — the regen gate
already consumes `use_regen_interrupt` from it; caster level and skill come from
the session Player, so a non-player caster keeps the unmodified amount; the
represented forwarding splits by eligible threatening owners where C++ first
divides by the number of spell targets, a pre-existing representation.

**#31 per-slot creature addon aura effects — 2026-09-18, implementation
`3082a93e`, integrated as `a6128cd0` by PR #1173:** two gaps in the represented
creature-aura model. First, C++ `Creature::LoadCreaturesAddon`
(`Creature.cpp:2777-2798`) applies each `auras` row through
`Unit::AddAura(spellId, this)`, which filters the spell's effects with
`Aura::BuildEffectMaskForOwner` (`SpellAuras.cpp:344-357`) and creates one
`AuraEffect` per surviving slot with its own type, amount and misc value; the
represented addon path created a bare mask-only application, so the canonical
aura consumers — the aggro tick's `SPELL_AURA_MOD_DETECT_RANGE` modifier,
`Unit::GetSchoolImmunityMask` through `aura_school_mask_like_cpp`, and
`has_aura_type_like_cpp` — could not see spawn-addon auras at all.
`CreatureAddonAuraApplicationLikeCpp` now carries a resolved `effects` list from
the data seam (`CreatureAddonStoreLikeCpp`, which already owns the
`SpellStore`), `effect_mask` is derived from that list so the two cannot drift,
and `AuraSubsystem::add_self_cast_addon_aura_application_with_effects_like_cpp`
registers it. Because the data seam resolves the effects once, the session
registry, the pending-respawn queue and the loaded-grid resolver all apply the
same data without a store at entity level. Second, both this path and the #1171
`Spell::EffectApplyAura` creature path registered every slot under one aggregate
`AppliedAuraRef`, so a two-slot aura answered `GetTotalAuraModifier` with
whichever amount was registered last; effect data now uses the per-slot
`AppliedAuraRef` (`1 << effect_index`) convention the pet-load
(`summoning.rs:451-459`) and threat-snapshot (`player/auras.rs`) paths already
establish, and the represented expiry removes every covered slot. The single
slot `SMSG_AURA_UPDATE` and the full aura update now share one `AuraDataInfo`
builder (`represented_creature_aura_info_like_cpp`), so the single-slot
publication no longer drops the application's `AFLAG` value and scalable points
that `AuraApplication::BuildUpdateData` always sends. Coverage:
`creature_addon_store_normalizes_auras_like_cpp` asserts the resolved per-slot
effect list and its derived mask in `wow-data`;
`creature_addon_aura_applications_register_effect_data_like_cpp` applies a
two-slot addon aura plus a school-immunity aura and asserts each slot's own
amount, the immunity misc value, `has_aura_type_like_cpp` and the visible slot
amounts; `registered_creature_addon_aura_effects_feed_canonical_consumers_like_cpp`
registers a world creature whose addon carries detect-range and school-immunity
auras and reads the consumers the aggro tick uses;
`spell_apply_aura_on_creature_keeps_each_effect_slot_amount_like_cpp` casts a
two-slot creature aura and asserts the per-slot masks and amounts plus the
expiry of both slots. Evidence at `a6128cd0`: `wow-entities --lib` 941/0,
`wow-data --lib` 753/0, `wow-world --lib` 4005/0/1, `cargo fmt --all --check`
and `git diff --check` clean, physical ratchet PASS (2234 files, no ceiling
moved), `session-ownership-check --syntax-only` PASS with no baseline delta (no
`WorldSession` surface changed); `validation-v2 quick` PASS (manifest
`20260918T023252.159860Z-3706760-quick.json`) and `final --architecture` 84.4 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red. **The ordinary warm campaign is 649.7 s, over
the 600 s budget**: changing the foundational `wow-entities`/`wow-data` crates
forced a full recompile of the dependent `world-server`/`wow-packet`/`wow-world`
check and test graph under `CARGO_BUILD_JOBS=1` (`path-04` alone 555.6 s), the
same dependency-forced overrun recorded for the earlier `wow-packet` rebuild. No
step was skipped, split or relabelled. Boundaries: a record whose data seam
resolved no unit-owned effect slot keeps the previous represented behavior (one
bare application per `auras` entry), while C++ `Unit::AddAura` creates nothing
there — production rows always carry the resolved applications and the
divergence is asserted as fixture behavior; addon effect amounts use
`SpellEffectInfo::calc_value_no_caster_like_cpp()`, the convention the creature
aura-effect projection already uses, rather than C++'s caster-evaluated amount;
aura recalculation on stat change, stack/refresh rules and periodic ticks remain
unrepresented.

**#31 creature-target aura application — 2026-09-18, implementation `8e062b6e`,
integrated as `b08ffff5` by PR #1171:** C++ `Spell::EffectApplyAura` applies the
aura to `unitTarget` (`SpellEffects.cpp:6150-6220`), but the represented
`SPELL_EFFECT_APPLY_AURA` chain always applied to the caster, so a debuff cast
at a creature landed on the player and a creature target could only ever hold
the addon auras created at spawn. `apply_creature_aura_like_cpp` now creates the
canonical creature's `AuraApplication` with each applied effect slot's
represented aura type, amount and misc value — so
`total_aura_modifier_like_cpp`, `aura_school_mask_like_cpp` and
`has_aura_type_like_cpp` see it, unlike the addon path — its
`LoadedAuraStateLikeCpp` duration and its visible slot; a second application of
the same spell by the same caster is refused like the spawn-addon path.
`publish_creature_aura_slot_update_like_cpp` sends the slot's
`SMSG_AURA_UPDATE` to the caster and to the creature's observers over the
existing creature visibility rail, and `tick_represented_creature_auras_like_cpp`,
driven by the session's existing aura sweep, removes the canonical application
and publishes the removal when the represented duration elapses. The
`SPELL_EFFECT_APPLY_AURA` loop routes a creature target to the generic
application once per spell; `SPELL_AURA_MOD_TAUNT` keeps flowing through the
represented taunt path that owns its `EffectTaunt` current-victim gate.
Coverage: a production-shaped scenario in `scenarios_world_entities_10.rs` casts
a `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` debuff at a creature and asserts the
canonical application and its registered amount, that the caster keeps none of
it, the `[SpellGo, AuraUpdate, CooldownEvent]` sequence, that a
`SPELL_AURA_MOD_DETECT_RANGE` aura feeds the creature's own aura-modifier
consumer (the aggro-range input), and that an elapsed duration removes both
auras with one removal packet each. Evidence at `b08ffff5`: `wow-world --lib`
4003/0/1, `wow-packet --lib` 751/0, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS (2234 files; one recorded ceiling bump for
the six-line session field with its reason),
`session-ownership-check --syntax-only` PASS with a reviewed `print-baseline`
delta (the new field, its four methods and the `WorldSession` bridge);
`validation-v2 quick` PASS in 130.4 s (manifest
`20260918T015451.857756Z-3687347-quick.json`) and `final --architecture` 85.9 s,
exit 1, 2 of 9 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the campaign is 216.3 s, inside the 600 s
ordinary budget. Boundaries: the represented model has no stack or refresh rule,
so an identical application is refused rather than refreshed, and the duration
is the represented 30-second convention rather than `SpellDuration.db2`; the
periodic effect amount is registered but nothing ticks it while no periodic tick
exists; an aura whose applying session logs out before its duration ends has no
other expiry owner; specially-cased player-only auras other than
`SPELL_AURA_MOD_TAUNT` now reach a creature target through the generic path.

**#29 melee ignore-absorb term — 2026-09-18, implementation `05568f2e`,
integrated as `e6b33d5c` by PR #1169:** C++ `Unit::CalcAbsorbResist` reduces how
much of a hit a school-absorb or mana shield may take when the attacker carries
`SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` and the shield's spell lacks
`SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE` (`Unit.cpp:1803-1832`); neither half was
represented. `wow-data` gained `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL = 194` and
`SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE = 0x0040_0000` (`SharedDefines.h:675`); the
school-absorb and mana-shield projections now carry `cannot_be_ignored`,
resolved through `SpellStore::has_attribute_for_difficulty_like_cpp` at the
aura's difficulty; `represented_melee_ignore_absorb_like_cpp` reproduces
`GetMaxPositiveAuraModifierByMiscMask` plus `RoundToInterval(0, 100)` and
`represented_melee_ignored_absorb_amount_like_cpp` reproduces
`CalculatePct(damage, auraAbsorbMod)`; both absorb loops subtract that amount
from the damage a shield without the attribute may take and restore it for the
next shield, exactly like C++'s `ModifyDamage(-x)`/`ModifyDamage(+x)` pair, and
the map-owned melee stage computes the modifier from the creature attacker's
aura effects. Coverage: a rules test for the maximum/clamp/`CalculatePct`
truncation and both loops with and without the attribute, plus a
production-shaped scenario in `scenarios_world_entities_32.rs` where a creature
attacker with a 50% modifier lands 5 of its 10-point hit into a 30-point shield
(absorbed 5, damage 5, `HITINFO_PARTIAL_ABSORB`) while a shield whose spell
carries `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE` absorbs the whole hit. Evidence at
`e6b33d5c`: `wow-world --lib` 4002/0/1, `wow-packet --lib` 751/0, `wow-data
--lib` 753/0, `cargo fmt --all --check` and `git diff --check` clean, physical
ratchet PASS (2234 files; one recorded ceiling bump for the three-line constant
in `wow-data/src/spell/mod.rs` with its reason),
`session-ownership-check --syntax-only` PASS with a reviewed `print-baseline`
delta (the melee tick bridge body); `validation-v2 quick` PASS in 52.8 s
(manifest `20260918T012740.730059Z-3660302-quick.json`) and `final
--architecture` 82.9 s, exit 1, 2 of 9 steps with `session-syntax-acceptance`
PASS and the pre-existing hotspot ratchet as the only red; the campaign is
135.7 s, inside the 600 s ordinary budget. Boundary: C++ can drive the temporary
damage negative when the ignoring amount exceeds what is left and then clamps
with a negative maximum; the represented loop floors that temporary damage at
zero instead of reproducing the artifact.

**#31 spell energize log — 2026-09-18, implementation `c1fc0309`, integrated as
`32e9a5d4` by PR #1167:** C++ `Unit::EnergizeBySpell` publishes
`SMSG_SPELL_ENERGIZE_LOG` for every spell power change
(`Unit.cpp:6566-6590`), but the represented energize path only set the power, so
a player's rage/energy/mana gains never reached the client. `wow-packet` gained
`SpellEnergizeLog` matching
`WorldPackets::CombatLog::SpellEnergizeLog::Write` (`CombatLogPackets.cpp:214-234`):
target and caster packed GUIDs, `int32(SpellID)`, `int32(Type)` (the `Powers`
value), `int32(Amount)` and `int32(OverEnergize)`, then the basic packet's
log-data bit. `apply_energize_effect_like_cpp` now takes the cast identity and
computes what C++ publishes — `Amount` is the delta `ModifyPower` actually
applied (the pool clamp included) and `OverEnergize` is the requested amount the
pool could not take — and its flat and percentage effect call sites pass the
spell and caster. Coverage: the flat and percentage energize scenarios assert the
log's position in the sent opcode sequence, the flat one decodes the packet
field by field, and a new scenario proves the clamp (with only 10 points of room
a 50-point energize publishes `amount 10`/`over_energize 40`). Evidence at
`32e9a5d4`: `wow-world --lib` 4000/0/1, `wow-packet --lib` 751/0, `cargo fmt
--all --check` and `git diff --check` clean, physical ratchet PASS (2234 files,
no ceiling moved), `session-ownership-check --syntax-only` PASS with a reviewed
`print-baseline` delta (the energize method's signature); `validation-v2 quick`
PASS in 104.8 s (manifest `20260918T010408.124942Z-3646754-quick.json`) and
`final --architecture` 82.8 s, exit 1, 2 of 8 steps with
`session-syntax-acceptance` PASS and the pre-existing hotspot ratchet as the only
red; the campaign is 187.7 s, inside the 600 s ordinary budget. Limits: the
power-drain/burn path stays silent because C++ publishes the target side through
the take-power log and restores no power on a self-drain; the C++ spell-id
special cases in `EffectEnergize` and `InterruptPowerRegen` remain outside this
slice; the log reaches the caster's session while C++ `SendCombatLogMessage` also
fans it out to the visible set.

**#31 player-target heal absorb — 2026-09-18, implementation `71286ca2`,
integrated as `06aa842e` by PR #1165:** C++ `Unit::CalcHealAbsorb`
(`Unit.cpp:2020-2084`) had no represented producer: a target's
`SPELL_AURA_SCHOOL_HEAL_ABSORB` shields absorbed nothing and the heal log always
reported a zero `Absorbed`. `session_rules::player_heal_absorb_shields_like_cpp`
projects each active heal-absorb effect (slot, effect index, amount) whose
`MiscValue` covers the heal's school mask, and
`represented_heal_absorb_like_cpp` reproduces C++'s loop: the amount is clamped
to the heal left, an amount-counting shield is depleted and removed at zero and
a negative amount is clamped to zero and never removed (no priority sort and no
ignore-absorb term exist in this C++ loop).
`WorldSession::apply_owned_player_heal_absorb_like_cpp` runs the stage for the
session's own player target before `DealHeal`, publishing one
`SMSG_SPELL_HEAL_ABSORB_LOG` per consuming shield through the new
`SpellHealAbsorbLog` writer (`WorldPackets::CombatLog::SpellHealAbsorbLog::Write`,
`CombatLogPackets.cpp:471-486`: target, absorb caster and healer packed GUIDs,
`AbsorbSpellID`, `AbsorbedSpellID`, `Absorbed`, `OriginalHeal`, content-tuning
presence bit), depleting the canonical `AuraEffect` amount and removing spent
shields through the aura transition that owns the publication; the heal then
receives the reduced amount and the heal log reports
`HealInfo::GetHeal()`/`GetOriginalHeal()`/`GetAbsorb()` exactly like
`HealInfo::AbsorbHeal`. The shield-amount depletion is now one shared
implementation (`session::combat::write_absorbed_shield_amount_like_cpp`) used
by both the map-owned melee absorb stage and this session-owned heal stage.
Coverage: the bounded `session/tests/scenarios_combat_5.rs` rules suite (no
shield, zero heal, partial/exact/spill consumption, negative infinite shield,
aura-order spending) and a production-shaped scenario in
`scenarios_spell_state_16.rs` where a 30-point shield absorbs a 20-point heal
whole and keeps 10, the absorb log decodes field by field, the heal log reports
`health 0`/`original 20`/`absorbed 20`, and the next heal spends the last 10 and
removes the shield. Evidence at `06aa842e`: `wow-world --lib` 3999/0/1,
`wow-packet --lib` 750/0, `cargo fmt --all --check` and `git diff --check`
clean, physical ratchet PASS (2234 files, no ceiling moved),
`session-ownership-check --syntax-only` PASS with a reviewed `print-baseline`
delta (the new heal-absorb method at 3915 exact associated items);
`validation-v2 quick` PASS in 117.8 s (manifest
`20260918T004911.084986Z-3632218-quick.json`) and `final --architecture` 85.3 s,
exit 1, 2 of 8 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the campaign is 203.0 s, inside the 600 s
ordinary budget. Limits: a creature heal target has no mutable represented aura
amount, so its heal absorb stays open exactly like the damage absorb for
creature victims; critical heals remain unrepresented and the combat-log
visible-set fan-out is still session-local.

**#31 direct spell heal combat log — 2026-09-18, implementation `5958c9a1`,
integrated as `8b0c0d49` by PR #1163:** C++ `Unit::HealBySpell` publishes
`SMSG_SPELL_HEAL_LOG` for every spell heal (`Unit.cpp:6538-6563`), but the
represented heal path applied the health and threat without any combat log, so a
player's heals never reached the client. `wow-packet` gained `SpellHealLog`
matching `WorldPackets::CombatLog::SpellHealLog::Write`
(`CombatLogPackets.cpp:180-212`): target and caster packed GUIDs,
`int32(SpellID)`, `int32(Health)`, `int32(OriginalHeal)`, `int32(OverHeal)`,
`int32(Absorbed)`, the empty supporter count and the bit tail (`Crit`, the
crit-roll-made and crit-roll-needed presence bits, the log-data bit and the
content-tuning presence bit). `WorldSession::publish_heal_spell_log_like_cpp`
(`session/spell_effects/effect_combat.rs`) publishes from both heal branches
after the committed heal — the owner-player branch before its health values
update, the creature branch after `forward_heal_threat_like_cpp` and before the
target's object update — mirroring C++'s
`CalcHealAbsorb -> DealHeal -> SendHealSpellLog` order; a heal without a
represented spell stays silent, like C++ with no `SpellInfo`. Coverage: the
packet wire test, the healed opcode sequence in the nine affected heal/leech
scenarios and a field-level decode in
`spell_heal_mechanical_effect_row_heals_player_like_cpp_without_type_gate`
asserting target, caster, spell id, requested heal, original heal, zero
over-heal, zero absorbed, empty supporters and the bit tail. Evidence at
`8b0c0d49`: `wow-world --lib` 3997/0/1, `wow-packet --lib` 749/0, `cargo fmt
--all --check` and `git diff --check` clean, physical ratchet PASS (2234 files,
no ceiling moved), `session-ownership-check --syntax-only` PASS with a reviewed
`print-baseline` delta (the new publication method at 3914 exact associated
items); `validation-v2 quick` PASS in 123.3 s (manifest
`20260918T003100.511665Z-3616696-quick.json`) and `final --architecture` 83.5 s,
exit 1, 2 of 8 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the campaign is 206.8 s, inside the 600 s
ordinary budget. Limits: heal absorb (`Unit::CalcHealAbsorb`) and critical heals
are not represented, so `Absorbed` is zero and `Crit` false with no crit-roll
floats; the log reaches the caster's session while C++
`SendCombatLogMessage` also fans it out to the visible set; creature-cast spell
effects remain unimplemented, so this covers player heals today.

**#31 direct spell damage combat log — 2026-09-18, implementation `8808bb91`,
integrated as `5424658a` by PR #1161:** C++ `Unit::DealSpellDamage` publishes
`SMSG_SPELL_NON_MELEE_DAMAGE_LOG` for every direct spell hit
(`Unit.cpp:1250-1260`, `Unit::SendSpellNonMeleeDamageLog` `Unit.cpp:5353-5380`),
but the represented direct-damage path applied damage, threat, kill, loot and
experience without any combat log, so a player's spell damage never reached the
client. `wow-packet` gained `SpellNonMeleeDamageLog` matching
`WorldPackets::CombatLog::SpellNonMeleeDamageLog::Write`
(`CombatLogPackets.cpp:92-124`): `Me`/caster/cast packed GUIDs,
`int32(SpellID)`, the `SpellCastVisual` id, damage, original damage, overkill,
`uint8(SchoolMask)`, absorbed, resisted, shield block, the empty
world-text-viewer and supporter counts and the bit tail (`Periodic`, seven
`Flags` bits, the false debug, log-data and content-tuning bits).
`apply_damage_from_caster_like_cpp` now takes the cast identity
(`SpellNonMeleeDamage::castId` plus the visual) from its cast-context caller and
sends the log right after the committed hit and before the kill cascade, reading
`preHitHealth` before the damage so `Overkill` matches C++; callers without a
represented cast context pass an empty GUID and no visual. Coverage: the packet
wire test decodes the exact order and tail, the direct-damage, health-leech and
instakill scenarios assert the log's position in the sent opcode sequence
(`[SpellGo, SpellNonMeleeDamageLog, UpdateObject, ..., CooldownEvent]`), and the
clicker-cast scenario decodes the packet and asserts target, caster, non-empty
cast identity, spell id, damage, overkill, school mask, zero absorb/resist/block
and the bit tail. Evidence at `5424658a`: `wow-world --lib` 3997/0/1,
`wow-packet --lib` 748/0, `world-server --lib` 594/0, `cargo fmt --all --check`
and `git diff --check` clean, physical ratchet PASS (2234 files, no ceiling
moved), `session-ownership-check --syntax-only` PASS with a reviewed
`print-baseline` delta (the damage-application method's signature);
`validation-v2 quick` PASS in 123.6 s (manifest
`20260918T001503.658454Z-3593488-quick.json`) and `final --architecture` 85.7 s,
exit 1, 2 of 8 steps with `session-syntax-acceptance` PASS and the pre-existing
hotspot ratchet as the only red; the campaign is 209.3 s, inside the 600 s
ordinary budget. Limits: a creature target has no represented spell absorb,
resist, block or critical stage, so those packet fields and `HitInfo` stay zero
and the spell-side `CalcAbsorbResist` remains #31 work; the log reaches the
caster's session while C++ `SendCombatLogMessage` also fans it out to the
visible set; creature-cast spell effects remain unimplemented, so this log
covers player direct damage today.

**#29 player-victim melee mana shield — 2026-09-17, implementation `e701fcd8`,
integrated as `0e53a402` by PR #1159:** a creature swing against a player now
also runs C++ `Unit::CalcAbsorbResist`'s mana-shield loop
(`Unit.cpp:1886-1930`), which had no represented producer: a player's
`SPELL_AURA_MANA_SHIELD` (Mage Mana Shield) neither drained mana nor absorbed.
`session_rules::player_mana_shields_like_cpp` projects the victim's active
mana-shield effects (slot, effect index, amount, amplitude) whose `MiscValue`
covers the school mask, and
`session_rules::represented_melee_mana_absorb_like_cpp` reproduces the loop: the
amount caps the damage, the drain is `amount * CalcValueMultiplier`, the
absorbed damage scales by `manaTaken / manaReduction`, a negative amount is
clamped to zero, an amount-counting shield is depleted and removed at zero, and
the loop stops once the damage is gone. `wow-data` gained
`SpellEffectInfo::Amplitude` (`SpellEffectEntry::EffectAmplitude`) with
`calc_value_multiplier_like_cpp()` (`SpellInfo.cpp:624-631`) and the
`SPELL_AURA_MANA_SHIELD = 97` constant. The map-owned absorb stage drains the
canonical player's mana with `Unit::set_power` in the same locked phase as the
health write, and the victim session publishes the resulting
`SMSG_POWER_UPDATE` (`Unit.cpp:9287-9312`) through the existing power owner,
beside the absorb log and the exhausted-shield removal it already owned; the
delivery command carries `mana_spent`, and the *total* absorbed amount (school
plus mana) decides `HITINFO_FULL_ABSORB`/`HITINFO_PARTIAL_ABSORB`, matching
C++'s single `DamageInfo` absorb. Coverage: the bounded
`session/tests/scenarios_combat_5.rs` rules suite (no shield, zero damage, full
absorb, mana-limited fraction, amplitude 2, amount cap, negative infinite
shield, dry mana) and a production-shaped scenario in
`scenarios_world_entities_32.rs` where a 30-point shield with one mana per damage
absorbs a 10-point hit and drains 10 mana, delivers the `PowerUpdate` and the
absorb log, then absorbs only the 3 points the last three mana can pay and
finally nothing with an empty pool. Evidence at `0e53a402`: `wow-world --lib`
3997/0/1, `wow-data --lib` 753/0, `wow-packet --lib` 747/0, `cargo check -p
world-server --all-targets` clean, `cargo fmt --all --check` and `git diff
--check` clean, physical ratchet PASS (2234 files, no ceiling moved),
`session-ownership-check --syntax-only` PASS with a reviewed `print-baseline`
delta (the command's `mana_spent`, the publication method's signature and the
tick bridge body); `validation-v2 quick` PASS in 529.2 s (manifest
`20260917T233742.699772Z-3566940-quick.json`) of which 519.0 s is the
check-profile recompilation, and `final --architecture` 84.5 s, exit 1, 2 of 8
steps with `session-syntax-acceptance` PASS and the pre-existing hotspot ratchet
as the only red; the pair is 613.7 s against the 600 s ordinary budget, a ~14 s
overrun reported rather than hidden. Limits: C++'s `absorbIgnoringDamage` term
and the spellmod half of `CalcValueMultiplier` stay unrepresented, a zero drain
resolves to no absorb instead of C++'s `0 / 0` float division, and creature
victims still have no mutable represented shield pool.

**#29 player-victim melee absorb log — 2026-09-17, implementation `ffa1097b`,
integrated as `9d8a1be6` by PR #1157:** the melee absorb stage now also publishes
C++ `Unit::CalcAbsorbResist`'s per-shield `SMSG_SPELL_ABSORB_LOG`
(`Unit.cpp:1876-1889`), which the previous slice left open. `wow-packet` gained
`SpellAbsorbLog` matching `WorldPackets::CombatLog::SpellAbsorbLog::Write`
(`CombatLogPackets.cpp:376-397`): attacker and victim packed GUIDs,
`int32(AbsorbedSpellID)` (zero for a white swing, which carries no spell),
`int32(AbsorbSpellID)`, the absorb aura's caster GUID, `int32(Absorbed)`,
`int32(OriginalDamage)`, the empty supporter count and the two flushed false bits
(`Unk` plus the basic packet's log-data bit). The delivery command now carries
every shield the map spent (`CreatureMeleeAbsorbConsumptionLikeCpp { slot,
consumed, removed }`) instead of only the exhausted slots, and
`WorldSession::publish_melee_absorb_consumption_like_cpp`
(`session/combat/damage.rs`, the represented absorb/resistance module) runs
C++'s per-shield order — read the shield's caster and spell while the aura
exists, send the log, then remove the aura spent to zero through the session's
own transition — before the attacker-state packet. Coverage: the packet's own
wire test decodes the exact field order and bit tail, and the production-shaped
scenario asserts the absorb log precedes the `AuraUpdate` removal and that the
consumption carries the spent amount. Evidence at `9d8a1be6`: `wow-world --lib`
3995/0/1, `wow-packet --lib` 747/0, `world-server --lib` 594/0, `cargo fmt
--all --check` and `git diff --check` clean, physical ratchet PASS (2234 files,
no ceiling moved; the loot handler stayed at its 2384-line ceiling by moving the
per-shield loop into the session module that owns aura publication),
`session-ownership-check --syntax-only` PASS with a reviewed `print-baseline`
delta (the consumption payload type and its derive, the command's
`exhausted_absorb_slots` -> `absorb_consumptions` swap, and the new publication
method at 3913 exact associated items). Budget, reported rather than hidden:
`validation-v2 quick` PASS in 542.5 s (manifest
`20260917T225619.282618Z-3548842-quick.json`) of which 531.3 s is the
`cargo check --locked --tests -p world-server -p wow-packet -p wow-world`
recompilation the `wow-packet` change forces at one Cargo job, and `final
--architecture` 86.6 s, exit 1, 2 of 8 steps with `session-syntax-acceptance`
PASS and the pre-existing hotspot ratchet as the only red; the pair is 629.1 s
against the 600 s ordinary budget, a ~29 s overrun caused by that check-profile
recompilation. Limits: the basic combat-log packet is delivered to the victim
session, while C++ `SendCombatLogMessage` also fans it out to the unit's visible
set, which no represented combat-log packet does yet;
`SPELL_AURA_MANA_SHIELD` (superseded 2026-09-17 by PR #1159, which resolves
the loop and its power publication), the `absorbIgnoringDamage`/`SPELL_ATTR6`
interaction, physical resist, the creature-victim absorb pool and live QA stay
open.

**#29 player-victim melee school absorb — 2026-09-17, implementation `a7bdac56`,
integrated as `045c565e` by PR #1155:** a creature swing against a player now
runs C++ `Unit::CalculateMeleeDamage`'s absorb tail (`Unit.cpp:1449-1466`) and
`Unit::CalcAbsorbResist` (`Unit.cpp:1789-1880`) instead of ignoring the victim's
`SPELL_AURA_SCHOOL_ABSORB` shields. `session_rules::player_absorb_shields_like_cpp`
projects each active absorb effect (slot, effect index, spell,
`SpellCategories.Category`, amount) whose `MiscValue` covers the school mask;
`session_rules::represented_melee_absorb_like_cpp` reproduces the loop —
`Trinity::AbsorbAuraOrderPred` order (Fel Blossom `28527`, Ice Barrier category
`471`, Sacrifice `7812`, other shields, Cauterize `86949` and Spirit of
Redemption `20711` last; `SpellAuraEffects.h:365-407`), negative amounts clamped
to zero, consumption clamped to the damage left and the zero-amount removal C++
performs. The map-owned tick runs the stage after the outcome switch and before
the health write, spends the canonical `AuraEffect` amount in the same locked
phase (so two attackers in one batch cannot spend the same pool twice), ORs
`HITINFO_FULL_ABSORB`/`HITINFO_PARTIAL_ABSORB` and carries the absorbed amount
plus the exhausted slots on `ApplyCreatureMeleeDamageLikeCppCommand`; the victim
session removes each exhausted shield through its own `remove_aura` transition
(`Unit.cpp:1856-1860`, before the attacker-state packet), which owns the removal
publication. `wow-packet`'s `AttackerStateUpdate` now emits C++ `SubDmg` for
every melee swing — presence byte, school mask, float and integer damage and,
with the bit set, the absorbed amount (`Unit.cpp:5473-5479`,
`CombatLogPackets.cpp:346-406`) — replacing the "no SubDmg" byte the target C++
never writes for `SendAttackStateUpdate(CalcDamageInfo*)`; the same change
restored the `#[test]` attribute `attacker_state_update_writes_custom_hit_info_like_cpp`
had lost (it had never run) and corrected the QA bot parser comment that claimed
Rust omitted the block. Coverage: the new bounded
`session/tests/scenarios_combat_5.rs` rules suite (no shield, zero damage,
partial/exact/spill consumption, negative infinite shield, priority order and
`AbsorbAuraOrderPred` ranks) and a production-shaped scenario in
`scenarios_world_entities_32.rs` where a 30-point shield absorbs 10 three times,
the third swing reports the exhausted slot, the delivered command removes the
aura and publishes an `AuraUpdate`, the next swing lands full damage and a
4-point shield leaves a partial absorb. Evidence at `045c565e`: `wow-world
--lib` 3995/0/1, `wow-packet --lib` 746/0, `world-server --lib` 594/0,
`wow-test-bot` packet-parser 6/0, `validation-v2 quick` PASS (62.98 s, manifest
`20260917T222914.597922Z-3528837-quick.json`), physical ratchet PASS (2234
files, two ceilings bumped with recorded reasons), `cargo fmt --all --check` and
`git diff --check` clean, `session-ownership-check --syntax-only` PASS with a
reviewed `print-baseline` delta of exactly the two new command fields; `final
--architecture` is 84.1 s, exit 1, 2 of 11 steps executed with
`session-syntax-acceptance` PASS and the pre-existing hotspot ratchet as the
only red. Limits: `SPELL_AURA_MANA_SHIELD` needs a power write the melee path
does not own (superseded 2026-09-17 by PR #1159); C++'s
`absorbIgnoringDamage` term (attacker
`SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` reduced by
`SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`) has no represented producer or spell
attribute projection; `SMSG_SPELL_ABSORB_LOG` is not sent (superseded
2026-09-17 by PR #1157, which publishes it for the melee path); physical melee
never resists (`Unit::CalcSpellResistedDamage` returns zero for a non-magic
school mask, `Unit.cpp:2058-2060`); creature victims have no mutable represented
absorb pool, so player→creature absorb stays open; no live DB/restart/relogin QA
ran.

**#584 session ownership baseline reconciliation — 2026-09-17, implementation
`0774178f`, integrated as `1f25ce5b` by PR #1153:** `session-ownership-check
check --syntax-only` had been red at every integration head since the policy was
last reconciled (`ef1a7b3d`, PR #950, 2026-09-15), and the mask was structural:
`validation-v2`'s `run_steps` stops at the first failure while the
`--architecture` plan runs `architecture-policy-acceptance` before
`session-syntax-acceptance`, so the open hotspot-ratchet breach aborted the
manifest before the ownership command ever ran. The policy was regenerated with
the reviewed generator (`print-baseline`) after a row-by-row review of the whole
delta: fields 649->652 (three private production fields), impl items 3807->3912
(105 session/handler methods plus seven retyped signatures), generated inputs
54->57 (three `#[must_use]`), direct-registry rows 626->630 and bridges 77->80
(two `cfg(test)` fixtures plus the #953
`run_legacy_creature_runtime_tick_with_input_and_deliver_once_like_cpp`
boundary). The delta is additive; the only retired rows are the nine registry
escapes of the #953 rename, re-recorded under the new name, so no previously
reviewed bridge, field or direct-registry escape was dropped and no new
unreviewed legacy/canonical dual-write entered the inventory. `run_steps` now
honours `continue_to_section_on_failure` (the `--architecture` policy step names
`session-syntax-acceptance`, the audit plan's policy step names
`session-persistence-ratchet`), so the first failure keeps its exit code and the
fail-fast budget while the independent verdict is still recorded. Evidence at
`1f25ce5b`: `check --syntax-only` PASS (222 production + 430 test-fixture
WorldSession fields; 69 impl owners / 3912 exact associated items; 8
SessionResources fields; 41 SessionCommand variants; 630 exact direct-registry
rows); `validation-v2 final --base origin/3.4.3 --architecture --timings` 81.2 s,
exit 1, with `architecture-policy-acceptance` failing on the pre-existing
hotspot ratchet (`crates/wow-world/src/session/mod.rs` 197853 -> 213571 total
lines) and `session-syntax-acceptance` passing, 2 of 7 planned steps executed;
`validation-v2 quick` PASS; `python3 tools/test_validation_v2.py` PASS; physical
ratchet PASS (2233 files). Limits: the hotspot ratchet stays red because its
growth is real accumulated LOC and bumping its baseline would hide the debt the
ratchet exists to expose; the exhaustive `session-ownership-check check`
persistence inventory was not completed inside the local window and remains the
audit profile's evidence. #584 is not closed by this slice.

**#29 melee chain verification — 2026-09-17, head `b0cb59c6`, no code change:**
the integration branch was re-verified after the nine #29 rounds merged in this
session (attack table, armour, taken, creature-victim bands, outcome
publication, blocked amount, fixture determinism). Three consecutive
`wow-world --lib` runs report 3990/0/1 each, so the creature-victim critical-band
flake fixed by PR #1131 stays gone under repetition; `world-server --lib`
594/0/0, `wow-packet --lib` 744/0, `wow-entities --lib` 940/0 and `wow-data
--lib` 753/0 pass; `cargo fmt --all --check`, `git diff --check` and the
physical ratchet pass. Remaining #29 melee boundaries are unchanged and each has
a recorded reason: the creature attacker's `MeleeDamageBonusDone` (no
represented creature-aura producer), the player-victim block band (needs the
`ExpectedStat` `ArmorConstant` consumer and mirrors the target build's
`CalculatePct(damage, fraction)` behaviour rather than the intuitive percentage),
the target build's negative crushing band, and live DB/restart/relogin QA. The
next unit should be the `ExpectedStat` consumer, whose store already exists in
`wow-data` with no consumer.

**#29 melee-suite acceptance snapshot — 2026-09-17, head `2e6c4054`, no code
change:** the accumulated #29 melee work (attack-table bands, armour, taken
chain, block reduction with the loaded `ExpectedStat` table, physical-immunity
gate and their production runtime scenarios) was accepted as one candidate.
Three consecutive `wow-world --lib` runs report 3992/0/1 each; `wow-data --lib`
753/0, `wow-packet --lib` 744/0, `wow-entities --lib` 940/0 and
`world-server --lib` 594/0 all pass; `cargo fmt --all --check` and
`git diff --check` pass, and the physical ratchet passes with the recorded
ceilings. `validation-v2 quick` passes (manifest
`20260917T211043.290594Z-3486868-quick.json`). This is an evidence snapshot of
the current candidate, not a completion claim: the four remaining #29 items are
listed in the entry below with the reason each is still open, and no live
DB/restart/relogin QA has been run.

**#29 creature-victim immunity scenario — 2026-09-17, implementation `65269938`,
integrated as `40b9e081` by PR #1150:** the immunity gate (PR #1148) resolves
`is_immune_to_damage` in both owners, but only the player-victim direction had a
production runtime scenario, so the creature-victim scan was compile-verified
only. The creature-victim bands scenario now applies a normal-school
`SPELL_AURA_SCHOOL_IMMUNITY` aura to the canonical victim and asserts the swing
commits no hit, deals no damage, and decodes the plan-event packet to require a
zero `HitInfo` (`HITINFO_NORMALSWING` is `0x0`) with `VICTIMSTATE_IS_IMMUNE`,
matching C++ `CalculateMeleeDamage`'s physical immunity return
(`Unit.cpp:1315-1324`). Both melee directions now have end-to-end immunity
coverage. Evidence: wow-world 3992/0/1 on two consecutive runs and world-server
594/0/0 pass; format, `git diff --check` and the physical ratchet pass without new
ceiling growth, and `validation-v2 quick` (manifest
`20260917T210537.585617Z-3464438-quick.json`) passes. No live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math.

**#29 melee physical-immunity gate — 2026-09-17, implementation `a269c051`,
integrated as `0036623f` by PR #1148:** C++ `Unit::CalculateMeleeDamage` opens
with a physical immunity check (`Unit.cpp:1315-1324`): when
`victim->IsImmunedToDamage(schoolMask)` holds, the swing ends before any damage
roll or band with `HITINFO_NORMALSWING` and `VICTIMSTATE_IS_IMMUNE` and zero
damage; the represented table had no immunity input, so a school-immune player
victim still took damage. `IsImmunedToDamage` (`Unit.cpp:7320-7336`) requires
`(GetSchoolImmunityMask() & schoolMask) == schoolMask`; for a white swing that is
`SPELL_AURA_SCHOOL_IMMUNITY` (39) covering `SPELL_SCHOOL_MASK_NORMAL` (`0x01`),
which both owners now scan for. `RepresentedMeleeOutcomeLikeCpp` gained `Immune`,
returned before every band and treated as an avoided outcome so no health is
committed; its damage arm returns zero and its presentation reproduces C++
exactly — `HITINFO_NORMALSWING` (`0x0`, so a main-hand immune swing publishes
`hitInfo == 0`) and `VICTIMSTATE_IS_IMMUNE`, without the
`HITINFO_AFFECTS_VICTIM` the later line would add. `wow-packet` gained the two
constants and the victim facts gained `is_immune_to_damage`; the creature-victim
branch resolves it from the same aura type. Boundary:
`GetDamageImmunityMask`'s `IMMUNITY_DAMAGE` map has no represented producer, so
only the school-immunity half is represented. Evidence: the pure table pins
`(0, 0, 100)` for the immune damage arm and `(0, VICTIM_STATE_IS_IMMUNE)` for the
presentation, and the production runtime scenario applies a normal-school
immunity aura to the player victim and asserts no committed hit, zero damage,
`hit_info == 0` and `VICTIM_STATE_IS_IMMUNE`. wow-world 3992/0/1 on two
consecutive runs, world-server 594/0/0 and wow-packet 744/0 pass; format,
`git diff --check` and the physical ratchet pass without new ceiling growth, and
`validation-v2 quick` (manifest
`20260917T205709.258475Z-3450946-quick.json`) passes. No live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math.

**#29 melee immunity gate discovery — 2026-09-17, no code change:** the claim
that the melee band table and its inputs are complete in both directions needs
one qualification. C++ `Unit::CalculateMeleeDamage` starts with a physical
immunity check (`Unit.cpp:1315-1324`): when
`victim->IsImmunedToDamage(SpellSchoolMask(DamageSchoolMask))` holds, the swing
returns `HITINFO_NORMALSWING` with `VICTIMSTATE_IS_IMMUNE` and zero damage before
any damage roll or band. The represented table has no immunity input at all, and
`wow-packet` has neither `HIT_INFO_NORMALSWING` (`0x0`) nor
`VICTIM_STATE_IS_IMMUNE` (`7`); `wow-entities` exposes only the vehicle immunity
structures, so no `IsImmunedToDamage` producer exists yet. Reachability differs
by direction: a player victim's school-immunity auras have a represented
producer, so creature→player immunity is reachable and currently deals damage;
a creature victim's immunities depend on the represented aura/immunity
registry, which has no producer today. The next melee-adjacent unit is therefore
the immunity gate: publish the two packet constants, resolve the victim's
normal-school immunity from its auras, add an `is_immune_to_damage` victim fact
and the early return in both owners, and cover it with a pure plus production
runtime scenario. The C++ semantics were read exactly this round and pin the
implementation: `Unit::IsImmunedToDamage(SpellSchoolMask)`
(`Unit.cpp:7320-7336`) returns true only when `(GetSchoolImmunityMask() &
schoolMask) == schoolMask` or `(GetDamageImmunityMask() & schoolMask) ==
schoolMask`, i.e. the victim must be immune to *every* school in the mask — for a
white swing that is `SPELL_SCHOOL_MASK_NORMAL` (`0x01`), whose source is a
`SPELL_AURA_SCHOOL_IMMUNITY` (39) effect whose `MiscValue` covers `0x01`;
`GetDamageImmunityMask`'s `IMMUNITY_DAMAGE` map has no represented producer and
is a boundary. The packet semantics are equally specific: the immune block sets
`HitInfo |= HITINFO_NORMALSWING` (which is `0x0`) and `TargetState =
VICTIMSTATE_IS_IMMUNE`, then returns *before* the `HITINFO_AFFECTS_VICTIM` line,
so a main-hand immune swing publishes `hitInfo == 0` with `VICTIMSTATE_IS_IMMUNE`
and zero damage — not the miss flags and not `AFFECTS_VICTIM`.

**#29 ExpectedStat table load — 2026-09-17, implementation `179e9b92`,
integrated as `7bab6a40` by PR #1144:** C++ `Player::GetBlockPercent`
(`Player.cpp:25288-25298`) reads
`DB2Manager::EvaluateExpectedStat(ExpectedStatType::ArmorConstant, attackerLevel,
-2, 0, CLASS_NONE)` (`DB2Stores.cpp:2103-2173`), but the represented block band
used C++'s empty-store fallback unconditionally because nothing loaded the table.
`ExpectedStatStore::armor_constant_like_cpp(level, expansion)` now reproduces the
target lookup: the `(lvl, expansion_id)` row, the `(lvl, -2)` fallback, and
`1.0f` when the level row is absent (`CLASS_NONE` applies no class modifier and
`contentTuningId` is unused in this fork).
`LegacyCreatureAggroConfigLikeCpp` gained an `expected_stat_store` handle, the
map-owned creature melee tick passes the resolved constant into
`player_block_percent_like_cpp`, and `world-server` loads `ExpectedStat.db2` once
at startup beside the other stores, tolerating a missing file through C++'s own
fallback. Evidence: the new production runtime scenario uses a 10,000-damage
swing so the blocked amount is visible on the wire — with no store the `1.0`
fallback gives `min(2000/2001, 0.85) = 0.85` and 85 blocked, while a level-80 row
with `ArmorConstant = 8000` gives `2000/10000 = 0.2` and 20 blocked, proving the
store value reaches the block arm end to end. wow-world 3992/0/1 on two
consecutive runs, world-server 594/0/0, wow-data 753/0 and wow-packet 744/0 pass;
format, `git diff --check` and the physical ratchet pass with recorded one-line
`session/mod.rs` and three-line `app.rs` ceiling growth, and `validation-v2
quick` (manifest `20260917T203033.611468Z-3426762-quick.json`) passes. No live
DB/restart/relogin QA. #29 remains open for the remaining spell/melee math.

**#29 player-victim block band — 2026-09-17, implementation `f21031f1`,
integrated as `7a808e4d` by PR #1142:** C++
`Unit::RollMeleeOutcomeAgainst` resolves a player victim's block chance from the
published `BlockPercentage` (`GetUnitBlockChance`'s player branch,
`Unit.cpp:2722-2751`) and, on a block, `CalculateMeleeDamage` computes
`Blocked = CalculatePct(damage, victim->GetBlockPercent(GetLevel()))`
(`Unit.cpp:1399-1407`); the represented table only knew the flat 30% creature
block, so a player victim could never block.
`melee_outcome_damage_like_cpp` gained `block_percent_like_cpp`, and
`player_block_percent_like_cpp` reproduces `Player::GetBlockPercent`
(`Player.cpp:25288-25298`): the published `ShieldBlock` over itself plus the
`ExpectedStatType::ArmorConstant` value, capped at `0.85`, with C++'s `0` guard
when both inputs are zero. The player-victim branch now takes `block_chance_pct`
from the published `BlockPercentage` (gated by the same facing/controlled gate
as parry) and the runtime passes the player's fraction; the session path and the
creature-victim branch keep the flat creature `30.0`. Boundaries: the runtime
does not load `ExpectedStat.db2`, so the armour constant uses C++'s own
empty-store `EvaluateExpectedStat` fallback (`1.0f`), and the target build's
`CalculatePct(base, pct) = base * pct / 100` means a player's returned fraction
blocks at most `0.85%` of the damage — that quirk is mirrored, not repaired.
Evidence: the pure table test pins the player-victim band order (dodge
[250, 2100), parry [2100, 3550), block [3550, 4550), crit [4550, 5750), hit
[5750, 10000)) plus the facing gate, a new
`player_block_percent_matches_get_block_percent_like_cpp` covers the fraction,
the `0.85` cap and the zero guard, and the production runtime scenario adds a
player-victim block stage (`BlockPercentage = 100`, `ShieldBlock = 2000`,
`HITINFO_BLOCK` on the wire with the truncated block). wow-world 3991/0/1 on
three consecutive runs, world-server 594/0/0, wow-packet 744/0 and wow-entities
940/0 pass; format, `git diff --check` and the physical ratchet pass with
`session/mod.rs` at a recorded 18,983 one-line ceiling growth, and
`validation-v2 quick` (manifest
`20260917T195857.595065Z-3396615-quick.json`) passes. The melee band table now
has no unresolved victim band in either direction. No live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math.

**#29 creature-victim parry scenario — 2026-09-17, implementation `292e8ca4`,
integrated as `2884a185` by PR #1140:** PR #1125 made the victim creature's
whole avoidance table reachable, but the production runtime scenario only
exercised miss, dodge, evade, block and crit, so the parry band had no runtime
coverage. The scenario now sets the canonical victim's
`CreatureAvoidanceLikeCpp::parry_pct` to 100 and asserts the swing commits no
hit, deals no damage, and decodes `victimState` sequentially from the plan-event
packet (after `hitInfo`, both packed guids, damage, original, over and the
sub-damage flag) to require `VICTIMSTATE_PARRY` on the wire, matching C++
`CalculateMeleeDamage`'s parry arm (`Unit.cpp:1380-1390`). With this, every
creature-victim band (miss, dodge, parry, evade, block, crit) has production
runtime coverage; the player-victim bands already did. Evidence: `wow-world
--lib` 3990/0/1 on three consecutive runs, `world-server --lib` 594/0/0 and
`wow-packet --lib` 744/0; format, `git diff --check` and the physical ratchet
pass, and `validation-v2 quick` (manifest
`20260917T194253.489532Z-3348541-quick.json`) passes.

**#29 creature-victim evade scenario — 2026-09-17, implementation `772895c2`,
integrated as `4c750401` by PR #1137:** C++ `Unit::RollMeleeOutcomeAgainst`
returns `MELEE_HIT_EVADE` before every band when the creature victim is evading
(`Unit.cpp:2274-2275`), and `CalculateMeleeDamage`'s evade arm publishes
`HITINFO_MISS | HITINFO_SWINGNOHITSOUND` with `VICTIMSTATE_EVADES` and zero
damage (`Unit.cpp:1345-1355`). PR #1125 made that gate reachable from the
runtime because it started resolving the victim creature's `is_evading_attacks`
fact, but only the pure table test covered evade. The production runtime
scenario now sets the canonical victim's evade mode and asserts the swing
commits no hit, deals no damage and reaches the wire as
`HITINFO_MISS | HITINFO_SWINGNOHITSOUND`, then clears the mode so the block and
critical stages continue. Evidence: `wow-world --lib` 3990/0/1 on three
consecutive runs, `world-server --lib` 594/0/0 and `wow-packet --lib` 744/0;
format, `git diff --check` and the physical ratchet pass, and
`validation-v2 quick` (manifest `20260917T193631.487323Z-3327648-quick.json`)
passes. The docs-only PRs #1133–#1136 between #1131 and this entry did not move
the implementation head.

**#29 player-victim block band preparation — 2026-09-17, no code change:** the
next melee unit is the player-victim block band, and its exact C++ contract was
traced this round so it can be implemented without re-deriving it:
`Player::GetBlockPercent(attackerLevel)` (`Player.cpp:25288-25298`) reads
`ActivePlayerData::ShieldBlock` and
`DB2Manager::EvaluateExpectedStat(ExpectedStatType::ArmorConstant,
attackerLevel, -2, 0, CLASS_NONE)`, returns `0` when both are zero and
`std::min(blockArmor / (blockArmor + armorConstant), 0.85f)` otherwise — a
**fraction**, not a percentage. `Unit::CalculateMeleeDamage`'s
`MELEE_HIT_BLOCK` arm (`Unit.cpp:1399-1407`) then computes
`Blocked = CalculatePct(Damage, GetBlockPercent(GetLevel()))`, and
`CalculatePct(base, pct)` is `base * pct / 100` (`Util.h:72-75`), so the target
build blocks at most `0.85%` of the damage: that quirk must be reproduced
faithfully, and repairing it is a legacy bug fix requiring an explicit contract
rather than a silent change. `DB2Manager::EvaluateExpectedStat`
(`DB2Stores.cpp:2103-2173`) looks up `_expectedStatsByLevel[(level, expansion)]`
with a fallback to `(level, -2)`, returns `1.0f` when the level row is absent,
applies a class mod only for the four player classes (irrelevant for
`CLASS_NONE`), reads `ExpectedStatEntry::ArmorConstant` for this stat, and leaves
`contentTuningId` unused; the represented `ExpectedStatStore` already exists in
`wow-data` with no consumer. Remaining wiring: a `block_reduction_fraction`
victim fact (flat `30.0` for creature victims, `GetBlockPercent` for a player
victim), the `ExpectedStatStore` in the runtime config, and the player-victim
branch's `block_chance_pct` set from the published `BlockPercentage` (today
`0.0`); `PlayerEffectiveCombatStatsLikeCpp` already publishes `shield_block`.
The store itself was inspected this round: `ExpectedStatStore` is generated by
`db2_store!`/`impl_from_entries!` in `wow-data/src/misc_generated.rs` and exposes
`get(id)`/`len()`, while C++'s lookup is by `(level, expansion)` — the entry
carries `lvl` and `expansion_id` — so the consumer needs a scan or a small
`(lvl, expansion_id)` index helper; tests can build the store with
`ExpectedStatStore::from_entries([...])`. The `ArmorConstant` input can be
resolved either by adding the store to the runtime config (one field, with the
`session/mod.rs` ceiling already at its recorded limit) or by publishing a
`block_reduction_fraction` in the player stat projection, which keeps the
map-owned runtime free of a new store handle; the projection route is the
smaller ownership change and should be preferred.

**#29 creature-victim melee fixture determinism — 2026-09-17, implementation
`50bef0de`, integrated as `32914a56` by PR #1131:** the full `wow-world --lib`
suite failed intermittently (about two runs in fourteen):
`legacy_creature_melee_tick_once_mitigates_creature_victim_like_cpp` expected 12
damage and occasionally saw 24, and
`legacy_creature_melee_tick_once_resolves_creature_victim_bands_like_cpp`
expected 8 and occasionally saw 16 — the doubling of an unplanned critical hit.
The creature-victim branch only started rolling the victim's critical band in
PR #1125, and those two fixtures kept the attacker's flat 5% critical. Both now
set `CREATURE_FLAG_EXTRA_NO_CRIT` on the attacker, exactly like the other
exact-damage melee fixtures; the bands scenario still covers the critical arm
because its dedicated stage supplies a `+100`
`SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` victim aura. The blocked-amount
entry above called this a pre-existing flaky case; it was not — PR #1125
introduced it in these two fixtures, and this change corrects the record.
Evidence: twelve consecutive full `wow-world --lib` runs pass (3990/0/1 each)
after the change, against the reproduced `12 vs 24`
(`scenarios_world_entities_28.rs:1198`) and `8 vs 16` (`:1424`) failures before
it; world-server 594/0/0 and wow-packet 744/0 pass; format, `git diff --check`
and `validation-v2 quick` (manifest
`20260917T192526.288811Z-3288980-quick.json`) pass.

**#29 creature-victim blocked-amount publication — 2026-09-17, implementation
`da3cf981`, integrated as `0ab5ad7b` by PR #1129:** round 75 left the bridge
packet's `blocked` field at zero with an open question — a fixed-offset decode
read zeros even with `HITINFO_BLOCK` set. The packet writer already serializes
`int32(BlockAmount)` plus the trailing `float Unk` when the flag is set (its own
test reads `blocked == 30` back), so the fixed-offset arithmetic was simply
wrong. `apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp` now
takes the blocked amount in its `outcome_presentation` tuple
(`(HitInfo, TargetState, Blocked)`), the creature-victim branch supplies the
value `melee_outcome_damage_like_cpp` computed beside the dealt damage, and the
packet writes it; the scenario decodes the round info sequentially — the same
order the writer's own test uses — and asserts the victim's flat 30% creature
block reports `blocked == 2` beside the 8-to-6 damage reduction and
`HITINFO_BLOCK`. Evidence: wow-world 3990/0/1, world-server 594/0/0 and
wow-packet 744/0 pass; format, `git diff --check` and the physical ratchet pass,
and `validation-v2 quick` (manifest
`20260917T191915.218057Z-3177586-quick.json`) passes. One full-suite run in that
round reported a single failure; the determinism entry above records that it was
**not** pre-existing — the creature-victim critical band activated by PR #1125
made two exact-damage fixtures non-deterministic, and PR #1131 fixed them.

**#29 creature-victim block-band scenario — 2026-09-17, implementation
`77c7d3a8`, integrated as `19df40a5` by PR #1127:** the creature-victim
avoidance unit (PR #1125) made the victim creature's flat 30% block band
reachable (`Unit.h:947`, `melee_outcome_damage_like_cpp`'s block arm), but no
scenario exercised it. The production runtime scenario now sets the victim's
`CreatureAvoidanceLikeCpp::block_pct` to 100 and asserts that the mitigated 8
damage is reduced by `CalculatePct(8, 30) = 2` to 6, with `HITINFO_BLOCK` on the
wire; the `melee_outcome_damage_like_cpp` tuple is also named at the call site
instead of accessed by `.0`. Evidence: wow-world 3990/0/1 and world-server
594/0/0 pass, plus format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T191337.050229Z-3152855-quick.json`).
Explicitly not claimed: the bridge packet's `blocked` field stays `0`; a decode
attempt did not show the appended `int32(BlockAmount)` in this plan-event
packet, so publishing the producer's blocked amount there needs its own
investigation first (the existing writer test covers the session path's blocked
byte).

**#29 creature-victim avoidance and crit bands — 2026-09-17, implementation
`505b9fbe`, integrated as `96708a82` by PR #1125:** C++
`Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2360`) reads the victim's
`GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance` creature branch
(its `CreatureBaseStats`-seeded avoidance plus the `MOD_*_PERCENT` aura sums and
the victim-level bonus), `GetUnitCriticalChanceAgainst` and the facing/controlled
gates for every victim, but the represented creature-victim branch only passed
the miss terms, so a creature victim could never dodge, parry, block or be
critically hit. The branch now resolves the full victim facts from the canonical
creature — `CreatureAvoidanceLikeCpp`, `is_totem_unit_type_like_cpp`,
`is_evading_attacks_like_cpp`, the percentage and attacker-side aura sums, the
health-conditioned and for-caster critical sums, the facing and controlled gates
— together with the full creature-attacker facts (base `5.0` critical unless
`CREATURE_FLAG_EXTRA_NO_CRIT`, `MOD_WEAPON_CRIT_PERCENT`/`MOD_CRIT_PCT`,
`MOD_AUTOATTACK_CRIT_CHANCE`, `MOD_EXPERTISE / 4`, combat-result and
enemy-dodge sums, crit-damage multiplier). `melee_outcome_inputs_like_cpp` and
`melee_outcome_damage_like_cpp` were already complete for creature victims, so
this is the runtime resolution only; the evade short-circuit and the flat 30%
creature block now take effect too. Evidence: the runtime scenario drives a
landed mitigated 8-damage hit, then a 100% `CreatureAvoidanceLikeCpp` dodge with
unchanged health and `canonical_creature_hits == 0`, then a `+100`
`SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` critical for 16, then a `-200`
hit-chance miss with `HITINFO_MISS` on the wire. wow-world 3990/0/1, wow-data
753/0, wow-packet 744/0, wow-entities 940/0 and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass without new ceiling
growth, and `validation-v2 quick` (manifest
`20260917T190001.364083Z-3142500-quick.json`) passes. No live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math; the creature attacker's
`MeleeDamageBonusDone`, the player-victim block band and the target build's
negative crushing term remain the melee boundaries.

**#29 creature-victim miss band and outcome publication — 2026-09-17,
implementation `242aa446`, integrated as `865dd228` by PR #1123:** C++
`Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) rolls the attack table
for every victim and `CalculateMeleeDamage` publishes `HitInfo`/`TargetState`
through `AttackerStateUpdate`, but the represented creature-victim branch still
committed a normal hit for every swing, so a creature (or player pet) swinging at
another creature could never miss. The branch now rolls the miss band from the
victim creature's `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum over the flat
5.0 (`Unit.cpp:11652-11685`) and threads the resolved `(HitInfo, TargetState)`
into `apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp`, whose
packet now carries the caller's presentation instead of a hardcoded normal hit;
an avoided swing commits zero damage and no legacy mirror sync.
`melee_outcomes_unrepresented` now means "the represented outcome table did not
run", so a table-resolved creature-victim swing no longer increments it.
Evidence: the runtime scenario drives a `+5` victim aura to the landed,
mitigated 8-damage hit with `HITINFO_AFFECTS_VICTIM` on the wire and then a
`-200` aura to a miss with unchanged health, `canonical_creature_hits == 0` and
`HITINFO_MISS` on the wire. wow-world 3990/0/1, wow-data 753/0, wow-packet
744/0, wow-entities 940/0 and world-server 594/0/0 pass; format,
`git diff --check` and the physical ratchet pass without new ceiling growth, and
`validation-v2 quick` (manifest `20260917T185049.290927Z-3131478-quick.json`)
passes. Boundary: the creature victim's own dodge/parry/block/crit runtime facts
(its `CreatureAvoidanceLikeCpp`, aura percentages and level bonus) are the next
slice — `melee_outcome_inputs_like_cpp` already implements them for a creature
victim, only the runtime resolution remains. No live DB/restart/relogin QA. #29
remains open for the remaining spell/melee math.

**#29 melee scenario-suite split — 2026-09-17, implementation `d444f94d`,
integrated as `a2d86694` by PR #1121:** `session/tests/scenarios_world_entities_28.rs`
had reached 1,977 of the 2,000-line test-file budget, and the physical policy
requires a split before the next melee scenario can land there. The three
map-owned player-victim runtime scenarios (miss/dodge/parry/crit bands, armour
mitigation and the damage-taken chain) moved unchanged into the new bounded
`session/tests/scenarios_world_entities_32.rs`, leaving the ten legacy
creature-tick scenarios in the parent file: 1,197 and 787 lines respectively.
This is a structural move only — no assertion, fixture or production line
changed, and the parent module still owns the shared fixtures through
`use super::*`. The new module is registered in `session_tests.rs`, whose
recorded 5,796-line ceiling grew by the two inseparable registration lines to
5,798 with the explanation appended to its physical-policy entry. Evidence:
`wow-world --lib` stays at 3,989 passing tests (the moved scenarios are still
registered and executed), plus wow-data 753/0, wow-packet 744/0, wow-entities
940/0 and world-server 594/0/0; format, `git diff --check` and the physical
ratchet pass, and `validation-v2 quick` (manifest
`20260917T183826.035128Z-3119618-quick.json`) passes. This is module
maintenance, not gameplay progress; the next melee unit (the creature-victim
outcome table and its packet presentation) can now land in the split suite.

**#29 creature-victim melee mitigation — 2026-09-17, implementation `3800be67`,
integrated as `3d6d7958` by PR #1119:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1326-1343`) runs the victim's `MeleeDamageBonusTaken` and
`CalcArmorReducedDamage` before the outcome switch for every attacker, but the
represented creature tick applied `roll_damage()` unmitigated when the victim
was another creature (including player pets), so pet and creature-vs-creature
damage ignored the victim's armour entirely. The creature-victim branch of the
map-owned creature melee tick now resolves the victim creature's canonical level
and `combat_log_stats_like_cpp().armor`, its applied-aura
`MeleeDamageBonusTaken` chain, and the attacker's normal-school
`SPELL_AURA_MOD_TARGET_RESISTANCE`/`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` sums,
then applies the shared taken and armour arithmetic before the compatibility
bridge commits the health change; `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` stays
zero on this branch because a creature victim's bypass aura has no represented
producer, and the branch's outcome table and packet presentation are unchanged.
Evidence: the production runtime scenario drives a level-80 creature attacker
against a 5,000-armour level-80 creature victim, asserting the committed health
drops by 8, then 12 once the victim carries a `+5` flat
`SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` aura, then 15 once the attacker's
normal-school `-5,000` `SPELL_AURA_MOD_TARGET_RESISTANCE` cancels the armour.
wow-world 3989/0/1, wow-data 753/0, wow-packet 744/0, wow-entities 940/0 and
world-server 594/0/0 pass; format, `git diff --check` and the physical ratchet
pass without new ceiling growth, and `validation-v2 quick` (manifest
`20260917T183231.918763Z-3109908-quick.json`) passes.
`session/tests/scenarios_world_entities_28.rs` reaches 1,977 of the 2,000-line
test budget, so the next melee scenario there must split the file first.
Boundaries: the creature-victim outcome table and its packet presentation, the
creature attacker's `MeleeDamageBonusDone`, the player-victim block band and the
target build's negative crushing term remain unrepresented. No live
DB/restart/relogin QA. #29 remains open for the remaining spell/melee math.

**#29 player-victim melee damage-taken chain — 2026-09-17, implementation
`908a6927`, integrated as `316b10b2` by PR #1117:** C++
`Unit::CalculateMeleeDamage` (`Unit.cpp:1326-1343`) runs the victim's
`MeleeDamageBonusTaken` before `CalcArmorReducedDamage`, but the represented
creature bridge only had the armour step, so a player's damage-taken auras never
applied to creature melee. The map-owned creature melee tick now resolves the
victim player's whole active aura list once and feeds it through the shared
`melee_damage_taken_flat_pct_like_cpp`/`melee_damage_taken_apply_like_cpp`
chain — the flat `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` benefit, the school-masked
`SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` and
`SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` multipliers and the
`SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` term restricted to the attacker —
before the armour reduction, with the attacker's
`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` pairs feeding C++'s Sanctified Wrath
bypass (`Unit.cpp:7670-7778`). `player_aura_effects_all_like_cpp` is the
unfiltered projection the chain needs, and
`PlayerAuraEffectLikeCpp::as_applied_like_cpp` re-shapes it into the shared
creature-aura form, so both victim kinds keep one arithmetic implementation.
Evidence: the production runtime scenario drives 5,000 armour to 8 damage, a
`+5` flat taken aura to 12, a fire-school `+100 %` percent aura to no change,
the normal-school row to 23, and then, with the total modifier below one, a
`-50 %` taken aura plus a creature `+50` ignore-resist aura to 10 (the bypass
shrinks the reduction to `0.75` while the same aura halves the armour); the
existing pure taken tests pin the bypass arithmetic. wow-world 3988/0/1,
wow-data 753/0, wow-packet 744/0, wow-entities 940/0 and world-server 594/0/0
pass; format, `git diff --check` and the physical ratchet pass without new
ceiling growth, and `validation-v2 quick` (manifest
`20260917T182258.318808Z-3099266-quick.json`) passes. Boundaries: the attacker's
`MeleeDamageBonusDone` for a creature, the player-victim block band and the
target build's negative crushing term remain unrepresented; without a spell
store the runtime keeps the pre-table bridge. No live DB/restart/relogin QA. #29
remains open for the remaining spell/melee math.

**#29 creature-to-player armour mitigation — 2026-09-17, implementation
`fc8cadf8`, integrated as `e26133e5` by PR #1115:** C++
`Unit::CalculateMeleeDamage` (`Unit.cpp:1326-1343`) runs
`CalcArmorReducedDamage` before the outcome switch, but the represented
creature bridge applied its rolled damage to a player victim unmitigated, so a
5,000-armour player took the same hit as an unarmoured one. The map-owned
creature melee tick now resolves the player victim's published `GetArmor()` and
the attacker's normal-school `SPELL_AURA_MOD_TARGET_RESISTANCE` and
`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` sums, then applies
`armor_reduced_damage_like_cpp` to the pre-outcome damage; a creature attacker
has no `CR_ARMOR_PENETRATION` rating, so that term stays zero. The victim's
`SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` sum for effects the attacker cast is the
one new victim-side input: C++ applies
`CalculatePct(armor, 100 - min(sum, 100))` before the target-resistance sum
(`Unit.cpp:1631-1637`), which `armor_reduced_damage_like_cpp` now reproduces
through a new `bypass_armor_pct_by_caster` parameter;
`RepresentedArmorMitigationLikeCpp` carries the field and the session path (a
player attacking a creature) passes zero because a creature victim's bypass
aura has no represented producer. The pure armour test is now table-driven so
the added cases stay inside the test-file budget. Evidence: the armour table
pins 15 cases including the bypass order (`911` with a `-1,000`
target-resistance sum versus `884` if the bypass applied after it), the
`min(sum, 100)` clamp and a negative amount; the production runtime scenario
drives 5,000 armour to 8 damage, a foreign-caster bypass to no change, one
attacker-cast 50% bypass to 9 and two to the full 10. wow-world 3987/0/1,
wow-data 753/0, wow-packet 744/0, wow-entities 940/0 and world-server 594/0/0
pass; format, `git diff --check` and the physical ratchet pass with
`session/mod.rs` at a recorded 18,982 one-line ceiling growth for the new
argument, and `validation-v2 quick` (manifest
`20260917T180725.086754Z-3086301-quick.json`) passes. Boundaries: the player
victim's `MeleeDamageBonusDone`/`MeleeDamageBonusTaken` chain, its block band
and the target build's negative crushing term remain unrepresented; without a
spell store the runtime keeps the pre-table bridge. No live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math.

**#29 creature-to-player dodge/parry/crit bands — 2026-09-17, implementation
`8eb3420f`, integrated as `37433879` by PR #1113:** C++
`Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2360`) resolves a player
victim's avoidance from the published `DodgePercentage`/`ParryPercentage` and
its sitting-target critical; the represented creature bridge only had the miss
band, so a player could never dodge, parry or be critically hit by a creature.
`RepresentedMeleeVictimFactsLikeCpp` gained `is_stand_state`,
`RepresentedMeleeOutcomeInputsLikeCpp` an `always_crits` flag, and the
player-victim branch of `melee_outcome_inputs_like_cpp` now gates
`canDodge`/`canParryOrBlock` on `victim->HasInArc(M_PI, attacker)` and
`UNIT_STATE_CONTROLLED`, applies the attacker's expertise and dodge reductions,
and builds the critical band from `GetUnitCriticalChanceAgainst`;
`melee_outcome_like_cpp` returns `Crit` before the avoidance bands when the
sitting rule holds. The map-owned creature melee tick resolves the full
attacker/victim facts from the spell store: the creature's `5.0` base critical
(skipped while `CREATURE_FLAG_EXTRA_NO_CRIT` is set), its
`MOD_WEAPON_CRIT_PERCENT`/`MOD_CRIT_PCT`, `MOD_AUTOATTACK_CRIT_CHANCE`,
`MOD_EXPERTISE / 4` and combat-result/enemy-dodge sums, and the victim's
attacker-melee hit/crit, crit-vs-target-health (`!HealthBelowPct(MiscValueB)`)
and crit-for-caster (caster == attacker) sums;
`player_aura_effects_full_by_spell_aura_type_like_cpp` is the new shared
projection that also exposes `MiscValueB` and the caster, with the existing
`(MiscValue, amount)` and caster wrappers delegating to it. Evidence: the inputs
test pins the player victim's `18.5` dodge / `14.5` parry / `12` crit bands with
their `(250, 2100, 3550, 4750)` edges, the facing/controlled gates, the sitting
crit and the zero-critical counter-case; the production runtime scenario drives
the victim's published stats and stand state to a dodge, a parry, a sitting crit
(20 damage, `HITINFO_CRITICALHIT`, `original_damage 20`) and a `+100`
crit-vs-health-aura crit. wow-world 3986/0/1, wow-packet 744/0, wow-entities
940/0 and world-server 594/0/0 pass; format, `git diff --check` and the physical
ratchet pass with `session/mod.rs` and `handlers/loot/handlers.rs` at their
recorded ceilings, and `validation-v2 quick` (manifest
`20260917T175105.344443Z-3063865-quick.json`) passes. Boundaries: a player
victim's block band and armour mitigation stay out (the blocked damage needs C++
`Player::GetBlockPercent`'s DB2 `ExpectedStatType::ArmorConstant` table), the
target build's crushing term evaluates to a negative band, and without a spell
store the runtime keeps the pre-table bridge. No live DB/restart/relogin QA. #29
remains open for the remaining spell/melee math.

**#29 creature-to-player melee miss band — 2026-09-17, implementation
`51041a70`, integrated as `d3a24246` by PR #1111:** C++
`Unit::RollMeleeOutcomeAgainst` (`Unit.cpp:2272-2310`) always rolls the attack
table, but the represented creature bridge applied `roll_damage()` to every
player victim, so a creature swing could never miss. C++
`MeleeSpellMissChance` (`Unit.cpp:11652-11685`) starts from the victim's flat
`GetUnitMissChance()` of `5.0`, subtracts the creature's zero
`m_modMeleeHitChance` (`Unit.cpp:360`), its `SPELL_AURA_MOD_HIT_CHANCE` sum and
the victim's `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum, and the miss arm
publishes `HITINFO_MISS`/`VICTIMSTATE_INTACT` with zero dealt damage and no
`DealMeleeDamage` call. `RepresentedMeleeVictimFactsLikeCpp` gained `is_player`
and `melee_outcome_inputs_like_cpp` resolves the miss band for a player victim
instead of returning `NONE`; the map-owned creature melee tick now takes the
runtime config (the spell store both aura sums need), rolls the band for player
victims, and publishes the resolved
`HitInfo`/`TargetState`/`OriginalDamage` through
`ApplyCreatureMeleeDamageLikeCppCommand`, which the session handler delivers
without a `HealthUpdate` for an avoided swing. An avoided swing removes the
attacking-interrupt auras and rearms the timer exactly where
`Unit::AttackerStateUpdate` does (`Unit.cpp:2172-2173`). Evidence: the inputs
test pins a player victim's `2.5` miss band with zero dodge/parry/block/crit and
the `249`/`250` band edge, and the new production runtime scenario drives a `+5`
victim aura to a landed 10-damage hit and a `-100` aura to a zero-damage
`HITINFO_MISS`/`VICTIMSTATE_INTACT` command with `original_damage 10` and
unchanged health. wow-world 3986/0/1, wow-packet 744/0, wow-entities 940/0 and
world-server 594/0/0 pass; format, `git diff --check` and the physical ratchet
pass with `session/mod.rs` and `handlers/loot/handlers.rs` back at their
recorded ceilings, and `validation-v2 quick` (manifest
`20260917T170034.802907Z-3042128-quick.json`) passes. Boundaries: a player
victim's dodge/parry/block/crit bands, its armour mitigation and its
`MeleeDamageBonusTaken` terms remain unrepresented (the block damage reduction
additionally needs the DB2 `ExpectedStatType::ArmorConstant` table, and the
target build's crushing term evaluates to a negative band, so a represented
crushing outcome stays absent); without a spell store the runtime keeps the
pre-table bridge. No live DB/restart/relogin QA. #29 remains open for the
remaining spell/melee math.

**#29 ignore-target-resist armour term — 2026-09-17, implementation `73140a7c`,
integrated as `042084ae` by PR #1109:** C++ `Unit::CalcArmorReducedDamage`
(`Unit.cpp:1646-1651`) shrinks the victim's armour with
`std::floor(AddPct(armor, -amount))` for every attacker
`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` effect whose `MiscValue` covers the
attacker's school; the represented armour step resolved only the
`SPELL_AURA_MOD_TARGET_RESISTANCE` sum and the CR_ARMOR_PENETRATION rating, and
its doc comment claimed the aura had no represented producer even though the
same attacker aura already feeds the `MeleeDamageBonusTaken` Sanctified Wrath
bypass. The term now runs after the target-resistance sum and before the rating
penetration, in C++ order. `armor_reduced_damage_like_cpp` gained the parameter,
`RepresentedArmorMitigationLikeCpp` the field, and both owners (the session path
and the map-owned `GlobalLegacy` runtime) resolve the normal-school sum from the
attacker's auras. Boundary: C++ truncates each effect separately while the owner
sums the amounts first, so two concurrent effects differ by that per-step
truncation, and `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` stays unrepresented.
Evidence: the armour test pins 860 for a 50% sum over 5,000 armour, 1,000 for
100% and 671 for a negative amount; the session scenario walks a non-normal
ignore row (no change), the normal 50% row (753 → 860), a non-normal
target-resistance row (still 860) and the normal target-resistance row that
cancels the armour (1,000); and the production-owner runtime scenario
`map_owned_player_melee_applies_attacker_ignore_target_resist_like_cpp` drives
the map-owned tick to the same 860. wow-world 3985/0/1, wow-entities 940/0 and
world-server 594/0/0 pass; format, `git diff --check` and the physical ratchet
pass with `session/mod.rs` exactly at its recorded 18,981 ceiling, and
`validation-v2 quick` (manifest `20260917T161620.682511Z-3011389-quick.json`)
passes. No live DB/restart/relogin QA. #29 remains open for the remaining
spell/melee math.

**#29 critical original-damage correction — 2026-09-17, implementation `87b8132f`,
integrated as `e533de81` by PR #1107:** C++ `Unit::CalculateMeleeDamage` assigns
`damageInfo->OriginalDamage` inside the outcome switch (`Unit.cpp:1343-1440`),
not once before it: the evade, miss, dodge, parry, glancing and block arms keep
the post-armour value, while the critical arm assigns it *after* doubling and
after the `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` multiplier (`Unit.cpp:1362-1375`),
so a critical swing serializes the doubled value as its original too. The shared
swing captured `original_damage` before the switch, so every represented crit
published a pre-crit original the C++ never writes, and the original-damage
publication entry's blanket "post-armour value the outcome switch then scales"
comment over-generalized the assignment.
`melee_outcome_damage_like_cpp` now mirrors the switch's `(Damage, Blocked,
OriginalDamage)` triple per arm and
`represented_white_swing_damage_like_cpp` consumes it instead of capturing the
value itself; the function's stale doc comment, which still claimed the crit
multiplier was unrepresented after PR #1105 added it, is corrected at the same
time. Evidence: the damage-switch test pins `(200, 0, 200)` and `(400, 0, 400)`
for crits, `(90, 0, 100)` for a glancing reduction and `(70, 30, 100)` for a
block, and the forced-crit session scenario asserts `original_damage == damage`
(14/14, then 28/28 after the `+100 %` aura) beside the existing missed-swing
`7`; wow-world 3984/0/1, wow-packet 744/0, wow-entities 940/0 and world-server
594/0/0 pass; format, `git diff --check` and the physical ratchet pass without
ceiling growth (`session/mod.rs` shrank to 18980), and `validation-v2 quick`
(manifest `20260917T160321.888716Z-2997013-quick.json`) passes. No live
DB/restart/relogin QA. #29 remains open for the remaining spell/melee math.

**#29 critical-damage-bonus aura — 2026-09-17, implementation `16151cba`,
integrated as `d31af699` by PR #1105:** C++ `Unit::CalculateMeleeDamage`'s
`MELEE_HIT_CRIT` branch (`Unit.cpp:1362-1375`) doubles the post-armour damage
and then multiplies it by
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_CRIT_DAMAGE_BONUS, schoolMask)`;
the represented crit only doubled, which was the documented boundary of the
attack-table unit. The aura constant was added to the data crate,
`RepresentedMeleeAttackerFactsLikeCpp` gained
`crit_damage_multiplier` (the normal-school multiplier, `1.0` with no aura),
`melee_outcome_damage_like_cpp` gained the matching parameter and scales its
crit arm by it, and both owners (session path and map-owned `GlobalLegacy`
runtime) resolve the multiplier as the product of `1 + amount/100` over the
attacker's `SPELL_AURA_MOD_CRIT_DAMAGE_BONUS` auras whose misc covers the
normal school. Evidence: the damage-switch case pins `200` at multiplier `1.0`
and `400` at `2.0`, and the forced-crit session scenario
`white_swing_applies_victim_critical_chance_auras_like_cpp` extends from 14 to
28 damage once a `+100 %` aura is applied; wow-packet 744/0, wow-data 753/0,
wow-entities 940/0, wow-world 3984/0/1 and world-server 594/0/0 pass; format,
`git diff --check` and the physical ratchet pass (one recorded one-line
`session/mod.rs` ceiling growth), and `validation-v2 quick` (manifest
`20260917T155306.337559Z-2985789-quick.json`) passes. Remaining boundary: the
spell-school-specific multiplier for a non-normal school is not represented
because the white swing is always normal school, and no live DB/restart/relogin
QA. #29 remains open for the remaining spell/melee math.

**#29 original-damage publication — 2026-09-17, implementation `3d16476e`,
integrated as `53240ffd` by PR #1103:** C++ `AttackerStateUpdate::Write`
(`CombatLogPackets.cpp:350-356`) serializes `int32(Damage)` and
`int32(OriginalDamage)`, the post-armour value the outcome switch then scales
(`Unit.cpp:1343-1440`); the represented packet wrote the dealt damage twice, so a
missed, dodged, parried, glancing or blocked swing lost the pre-outcome value the
client uses for its combat text. `RepresentedMeleeSwingLikeCpp` and the mailbox
DTO gained `original_damage`, the shared swing captures it before the outcome
switch, and every packet construction publishes it (the creature-owned and
loot-roll packets keep `damage` as the original). Evidence: the block byte test
asserts `damage 70` against `original damage 100`, and the forced-miss session
scenario asserts `original_damage == 7` on a zero-damage swing; wow-packet 744/0,
wow-world 3984/0/1 and world-server 594/0/0 pass; format, `git diff --check` and
the physical ratchet pass (two recorded 1-4 line ceiling growths), and
`validation-v2 quick` (manifest `20260917T154339.509166Z-2972717-quick.json`)
passes. No live DB/restart/relogin QA. #29 remains open for the remaining
spell/melee math.

**#29 melee evade outcome — 2026-09-17, implementation `10cf9234`, integrated as
`d6980e2a` by PR #1101:** C++ `Unit::RollMeleeOutcomeAgainst`
(`Unit.cpp:2274-2275`) returns `MELEE_HIT_EVADE` before any band for a creature
victim whose `IsEvadingAttacks()` holds, and `Unit::CalculateMeleeDamage`'s evade
branch (`Unit.cpp:1345-1355`) publishes
`HITINFO_MISS | HITINFO_SWINGNOHITSOUND` with `VICTIMSTATE_EVADES` and zero
damage; the represented table had no evade outcome.
`RepresentedMeleeOutcomeLikeCpp` gained `Evade`, the outcome inputs and victim
facts an `is_evading_attacks` flag resolved from the creature in both owners, and
`melee_outcome_like_cpp` returns it before rolling; `wow-packet` gained
`HIT_INFO_SWING_NO_HIT_SOUND` and `VICTIM_STATE_EVADES`. Evidence: the band test
asserts an evading victim short-circuits at rolls 0 and 9999, the damage and
presentation tests pin `(0, 0)` and the evade flags, and
`white_swing_publishes_an_evade_like_cpp` drives the session owner from a normal
7 damage hit to zero damage with `VICTIMSTATE_EVADES` once
`set_in_evade_mode_like_cpp(true)` is applied; wow-packet 744/0, wow-data 753/0,
wow-entities 940/0, wow-world 3984/0/1 and world-server 594/0/0 pass; format,
`git diff --check` and the physical ratchet pass without new ceiling growth, and
`validation-v2 quick` (manifest `20260917T153350.701096Z-2963268-quick.json`)
passes. No live DB/restart/relogin QA. #29 remains open for the remaining
spell/melee math.

**#29 controlled-victim avoidance gate — 2026-09-17, implementation `f15252bd`,
integrated as `d64a4aa9` by PR #1099:** C++ `Unit::RollMeleeOutcomeAgainst`
(`Unit.cpp:2296-2304`) clears `canDodge` and `canParryOrBlock` when the victim is
casting a non-melee spell or carries `UNIT_STATE_CONTROLLED`; the represented
table resolved neither gate. `RepresentedMeleeVictimFactsLikeCpp` gained
`is_controlled`, resolved from the victim's unit state in both owners, and
`melee_outcome_inputs_like_cpp` clears `can_dodge`/`can_parry` for a controlled
victim. Boundary: the represented creature runtime does not track its current
spell slots, so C++ `IsNonMeleeSpellCast` stays unrepresented. Evidence: the pure
fact test pins both gates false for a controlled victim, and
`white_swing_gates_avoidance_on_the_controlled_state_like_cpp` drives the session
owner with a `+100%` dodge aura — the swing is dodged (`VICTIM_STATE_DODGE`)
while the victim is free and lands normally (7 damage,
`HIT_INFO_AFFECTS_VICTIM`) once `UNIT_STATE_CONTROLLED` is set; wow-packet 744/0,
wow-data 753/0, wow-entities 940/0, wow-world 3983/0/1 and world-server 594/0/0
pass; format, `git diff --check` and the physical ratchet pass without new
ceiling growth, and `validation-v2 quick` (manifest
`20260917T152301.868398Z-2950950-quick.json`) passes. No live DB/restart/relogin
QA. This closes the victim-side attack-table term list; #29 remains open for the
remaining spell/melee math (melee haste/cooldown consumers, ranged auto-attack,
proc/scripted terms).

**#29 ignore-dual-wield hit-penalty aura — 2026-09-17, implementation `2b93a741`,
integrated as `fbd008ac` by PR #1097:** C++ `Unit::MeleeSpellMissChance`
(`Unit.cpp:10427-10460`) adds its `19%` dual-wield miss penalty only while
`!HasAuraType(SPELL_AURA_IGNORE_DUAL_WIELD_HIT_PENALTY)` (458) holds; the
represented rule always added it. `RepresentedMeleeAttackerFactsLikeCpp` gained
`ignores_dual_wield_hit_penalty`, resolved from the attacker's live auras in both
owners (the aura's amount is irrelevant, matching `HasAuraType`), and
`melee_outcome_inputs_like_cpp` gates the penalty. Evidence: the pure fact test
pins miss 0 while the flag is set against 24 without it, and
`melee_attack_table_reads_the_dual_wield_penalty_aura_like_cpp` drives the session
owner with the plain `7.5` melee hit chance, asserting the flag is clear and the
dual-wield miss band is `16.5`, then applies the aura and asserts the band is
`0`; wow-packet 744/0, wow-data 753/0, wow-entities 940/0, wow-world 3983/0/1 and
world-server 594/0/0 pass; format, `git diff --check` and the physical ratchet
pass without new ceiling growth, and `validation-v2 quick` (manifest
`20260917T151423.438114Z-2939239-quick.json`) passes. Limit: the map-owned
runtime resolves the same flag through the shared rule, but this unit adds no
dedicated runtime scenario for it; no live DB/restart/relogin QA.

**#29 victim conditional critical-chance auras — 2026-09-17, implementation
`accffc17`, integrated as `d3583610` by PR #1095:** C++
`Unit::GetUnitCriticalChanceTaken` (`Unit.cpp:2424-2452`) adds the victim's
`SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` (183) while
`!HealthBelowPct(MiscValueB)` holds and its
`SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER` (306) while the attacker is the caster,
on top of the melee-crit sums delivered by PR #1093.
`AppliedAuraEffectLikeCpp` now carries the effect's `MiscValueB`, and
`RepresentedMeleeVictimFactsLikeCpp` gained
`crit_chance_vs_target_health_pct` and `crit_chance_for_caster_pct`, folded into
the crit band by `melee_outcome_inputs_like_cpp`; both owners apply the C++
predicates from the creature's health percentage and the attacker GUID. Boundary:
the `SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER_PET` branch needs a `TempSummon`
attacker, which the represented player swing is not. Evidence: the fact test pins
crit 25 from the two conditional sums,
`white_swing_applies_victim_critical_chance_auras_like_cpp` proves a foreign
caster's aura is ignored and the attacker's own applies (7 damage normal, 14 with
`HITINFO_CRITICALHIT` from the 100% target-health aura), and the production owner
`map_owned_player_melee_applies_victim_critical_chance_auras_like_cpp` holds the
map-owned tick to the same crit; wow-packet 744/0, wow-data 753/0, wow-entities
940/0, wow-world 3982/0/1 and world-server 594/0/0 pass; format,
`git diff --check` and the physical ratchet pass without new ceiling growth, and
`validation-v2 quick` (manifest `20260917T150133.551794Z-2926042-quick.json`)
passes. `validation-v2 final` stops only at the pre-existing `hotspot-ratchet`
baseline failure (manifest `20260917T150216.515054Z-2926250-final.json`). No live
DB/restart/relogin QA. The victim-side attack-table terms are now complete apart
from the casting/control avoidance gate; #29 remains open for the remaining
spell/melee math.

**#29 victim avoidance auras — 2026-09-17, implementation `28622fb0`, integrated
as `4cbacecc` by PR #1093:** C++
`Unit::GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance`
(`Unit.cpp:2313-2360`) add the victim's `SPELL_AURA_MOD_DODGE_PERCENT` (49),
`SPELL_AURA_MOD_PARRY_PERCENT` (47) and `SPELL_AURA_MOD_BLOCK_PERCENT` (51);
`MeleeSpellMissChance` subtracts the victim's
`SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` (184);
`GetUnitCriticalChanceTaken` adds `SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE`
(187) plus `SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE` (197); and
`GetUnitDodgeChance` adds the attacker's `SPELL_AURA_MOD_COMBAT_RESULT_CHANCE`
(248) `VICTIMSTATE_DODGE` row and `SPELL_AURA_MOD_ENEMY_DODGE` (251). Those were
the attack table's documented "no represented producer" boundary, which round
57's `creature_aura_effects_like_cpp` now retires:
`RepresentedMeleeAttackerFactsLikeCpp` gained `dodge_reduction_pct`,
`RepresentedMeleeVictimFactsLikeCpp` the five victim aura sums, and
`melee_outcome_inputs_like_cpp` folds them into the same bands. Both owners
resolve them — the session from the target creature's applied auras plus the
attacker's aura sums, the map-owned `GlobalLegacy` runtime from its creature
snapshot and the attacker's auras. Boundary: the victim-side conditional crit
modifiers (`MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH`,
`MOD_CRIT_CHANCE_FOR_CASTER`/`_PET`) and the casting/control avoidance gate still
have no represented producer. Evidence: the fact test pins
`MOD_DODGE_PERCENT` +100, `MOD_PARRY_PERCENT` -4, `MOD_BLOCK_PERCENT` +2, a -1
attacker dodge reduction and the hit/crit aura sums (dodge 101.5, parry 1.5,
block 5, crit 15); `white_swing_applies_victim_avoidance_auras_like_cpp` forces a
guaranteed dodge in the session owner and
`map_owned_player_melee_applies_victim_avoidance_auras_like_cpp` holds the
map-owned tick to the same outcome with unchanged creature health; wow-data
753/0, wow-world 3980/0/1 (two consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass without new ceiling
growth, and `validation-v2 quick` (manifest
`20260917T145024.092238Z-2913639-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T145121.294308Z-2913958-final.json`). No live DB/restart/relogin QA. #29
remains open for the remaining spell/melee math.

**#29 victim-side melee damage-taken chain — 2026-09-17, implementation
`45b7571c`, integrated as `9bcb8bef` by PR #1091:** C++
`Unit::CalculateMeleeDamage` (`Unit.cpp:1326-1334`) runs the victim's
`Unit::MeleeDamageBonusTaken` (`Unit.cpp:1687-1759`) between the done bonus and
the armour reduction; the represented white swing skipped the whole victim-side
chain because no rule could read a creature's applied auras.
`session_rules::creature_aura_effects_like_cpp` now resolves every active effect
of a creature's `AppliedAuraRef` list at the caller's difficulty (the player
projection's counterpart, and the first creature-aura-effect consumer beyond the
mechanic mask), and `session_rules::melee_damage_taken_flat_pct_like_cpp`
assembles `MeleeDamageBonusTaken` for a white swing: the normal-school
`SPELL_AURA_MOD_DAMAGE_TAKEN` (14) and `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` (125)
flat sums, the `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` (87),
`SPELL_AURA_MOD_MELEE_DAMAGE_FROM_CASTER` (343) and
`SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` (126) multipliers, and the Sanctified
Wrath bypass from the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269);
`melee_damage_taken_apply_like_cpp` applies the C++ tail, including the early
return when the negative flat benefit absorbs the hit. Both owners resolve the
inputs — the session from the target creature's applied auras plus the attacker's
ignore-resist sum, the map-owned `GlobalLegacy` runtime from its creature
snapshot and the attacker's auras. Boundary: the fixed cheat-death aura (45182),
the ranged variants and every `spellProto` branch cannot apply to a represented
white swing, the versatility term is commented out in the 3.4.3 source itself,
and a canonical-player victim keeps `NONE` like the other creature-only victim
terms. Evidence: `creature_aura_effects_resolve_the_applied_mask_like_cpp` pins
the effect-mask projection, `melee_damage_taken_matches_cpp_like_cpp` pins the
flat sums, the school/caster/pct multipliers, the negative-flat clamp and the
ignore-resist bypass,
`white_swing_applies_victim_melee_damage_taken_like_cpp` drives the session owner
100 -> 50 (flat) -> 25 (pct) -> 50 (bypass), and the production owner
`map_owned_player_melee_applies_victim_melee_damage_taken_like_cpp` holds the
map-owned tick to the flat half; wow-packet 744/0, wow-data 753/0, wow-entities
940/0, wow-world 3978/0/1 (three consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass (one recorded ten-line
ceiling growth), and `validation-v2 quick` (manifest
`20260917T143743.670941Z-2898243-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T143842.381692Z-2898605-final.json`). No live DB/restart/relogin QA. The
represented melee attacker and victim chains of `CalculateMeleeDamage` are now
complete for creature victims; #29 remains open for the remaining spell/melee
math (melee haste/cooldown consumers, ranged auto-attack, proc/scripted terms).

**#29 melee block band — 2026-09-17, implementation `91c3df70`, integrated as
`ede81837` by PR #1089:** C++ `Unit::CalculateMeleeDamage`'s `MELEE_HIT_BLOCK`
branch (`Unit.cpp:1399-1407`) keeps `VICTIMSTATE_HIT`, sets `HITINFO_BLOCK`,
subtracts `CalculatePct(damage, GetBlockPercent(attackerLevel))` and doubles the
blocked amount when the block is critical; `AttackerStateUpdate::Write`
(`CombatLogPackets.cpp:373-397`) then appends the blocked amount and the trailing
`float Unk`. The represented table skipped the band because neither wire field
was ported. `session_rules/rules_4.rs` now rolls `MELEE_HIT_BLOCK` between
GLANCING and CRIT from the victim's `GetUnitBlockChance` base plus the
victim-level bonus (no expertise reduction, exactly like C++), and the outcome
switch returns `(damage, blocked)` with `CalculatePct(damage, 30)` for the flat
creature block percent (`Unit.h:947`); `wow-packet` gained `blocked: i32` on
`AttackerStateUpdate`, the conditional `int32(blocked)`/`float(Unk)` write and
`HIT_INFO_BLOCK`; and `RepresentedMeleeSwingLikeCpp`, the mailbox DTO and the
`PlayerMeleeCreatureHitLikeCpp` presentation vector carry the blocked amount to
the publishing session in both owners. Boundary: `IsBlockCritical`'s victim aura
sum and `Player::GetBlockPercent`'s shield-block formula have no represented
producer, so the flat 30% creature block stays. Evidence: the band-edge test
covers the block boundaries, the damage switch pins `(70, 30)` and `(5, 2)`, the
presentation pins `HITINFO_BLOCK | HITINFO_AFFECTS_VICTIM` with
`VICTIMSTATE_HIT`, the packet byte test parses the blocked amount and the
trailing float, and the new `white_swing_publishes_a_block_like_cpp` (session
owner) and `map_owned_player_melee_publishes_a_block_like_cpp` (production
map-owned tick) force a 100% block band and hold the 70/30 split plus the
creature's health loss. wow-packet 744/0, wow-data 753/0, wow-entities 940/0,
wow-world 3974/0/1 (three consecutive runs) and world-server 594/0/0 pass;
format, `git diff --check` and the physical ratchet pass (three recorded ceiling
growths of 1-2 lines), and `validation-v2 quick` (manifest
`20260917T141411.755780Z-2873193-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T141516.534662Z-2873575-final.json`). No live DB/restart/relogin QA. #29
remains open for the `MeleeDamageBonusTaken` victim chain and the remaining
spell/melee math.

**#29 melee attack table — 2026-09-17, implementation `e478a2b7`, integrated as
`08e27329` by PR #1087:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1341-1343`) rolls `Unit::RollMeleeOutcomeAgainst`
(`Unit.cpp:2272-2378`) after mitigation and applies the outcome switch
(`1345-1440`); the represented white swing always landed as a normal hit, so
nothing could miss, be dodged, parried, glance or crit and the published
`SMSG_ATTACKERSTATEUPDATE` always carried the plain hit flags. The new
`session_rules/rules_4.rs` owns the represented table — the band order
`MISS > DODGE > PARRY > GLANCING > CRIT > HIT` in C++'s 1/10000 units with its
`int32(chance * 100.0f)` truncation and gated bands, the outcome damage switch
(avoids to zero, glancing `1 - min(leveldiff, 3) * 0.1`, crit double) and the
`HitInfo`/`TargetState` presentation — while `melee_outcome_inputs_like_cpp`
assembles every chance from attacker/victim facts so both owners share one rule.
`Creature` gained `CreatureAvoidanceLikeCpp` (its C++
`GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance` bases), seeded by
`apply_lifecycle_record` from the template and its `NO_PARRY`/`NO_BLOCK` flags,
and `PlayerEffectiveCombatStatsLikeCpp` gained `melee_hit_chance_pct`
(`m_modMeleeHitChance`), so the map runtime resolves the miss band without a
combat-ratings table. `take_canonical_player_attack_swings_like_cpp` draws the
roll per landed swing and returns `RepresentedMeleeSwingLikeCpp { damage,
hit_info, victim_state }`; both owners resolve the facts, the avoid path deals no
damage, no tap and no threat, and the mailbox DTO carries the outcome to the
publishing session. `wow-packet` gained the correctly named 3.4.3 constants
(`HIT_INFO_AFFECTS_VICTIM`, `HIT_INFO_OFFHAND`, `HIT_INFO_MISS`,
`HIT_INFO_CRITICAL_HIT`, `HIT_INFO_GLANCING`, `VICTIM_STATE_INTACT`,
`VICTIM_STATE_DODGE`, `VICTIM_STATE_PARRY`), replacing the misnamed
`HIT_INFO_NORMAL_SWING` (`C++ HITINFO_NORMALSWING` is zero) and the stale
`victim_state` doc. Boundaries: the block band stayed out of the roll because the
represented `AttackerStateUpdate` did not yet port the conditional
`blocked`/`unk` fields C++ appends for `HITINFO_BLOCK` (delivered by the entry
above); victim
`MOD_DODGE_PERCENT`/`MOD_PARRY_PERCENT` auras,
`CREATURE_FLAG_EXTRA_NO_CRUSHING_BLOWS`, the critical-damage-bonus aura and the
casting/control avoidance gate have no represented producer; a canonical-player
victim keeps the pre-table behaviour. Evidence:
`melee_attack_table_matches_roll_melee_outcome_against_like_cpp` pins every band
edge plus the gated and truncated cases,
`melee_attack_table_outcome_effects_match_calculate_melee_damage_like_cpp` pins
the damage/presentation switch (including `HITINFO_OFFHAND`),
`melee_attack_table_inputs_resolve_cpp_chances_like_cpp` pins the fact assembly
(dual-wield `+19`, `+1.5` per level, glancing at `+4` levels, totem, player
victim), `white_swing_publishes_the_attack_table_outcome_like_cpp` drives the
session owner (400 landed swings with both hits and avoids, then a `-200%`
hit-chance aura forcing `HIT_INFO_MISS`/`VICTIM_STATE_INTACT` at zero damage) and
`map_owned_player_melee_publishes_the_attack_table_outcome_like_cpp` holds the
production map-owned tick to the same miss; fixtures isolate the table through
`CreatureAvoidanceLikeCpp` (zero for a directly constructed creature) plus a
trained hit chance, so the existing exact-damage regressions stay deterministic.
wow-packet 742/0, wow-entities 940/0, wow-world 3972/0/1 and world-server 594/0/0
pass; format, `git diff --check` and the physical ratchet pass (two recorded
ceiling growths; the melee-math tests live in the bounded
`session/tests/scenarios_combat_4.rs`), and `validation-v2 quick` (manifest
`20260917T135143.177788Z-2830540-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T135303.990925Z-2831019-final.json`). No live DB/restart/relogin QA. #29
remains open for the `MeleeDamageBonusTaken` victim chain and the remaining
spell/melee math.

**#29 white-swing armour mitigation — 2026-09-17, implementation `a739c8a4`,
integrated as `9cc0ea20` by PR #1085:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1326-1339`) runs the rolled and bonus-adjusted damage through
`Unit::CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) before the hit table; the
represented white swing skipped armour, so every auto-attack landed at full value
against armoured victims. `session_rules::armor_reduced_damage_like_cpp`
implements the formula — the victim's `GetArmor()` (a creature's
`CreatureBaseStats::GenerateArmor`), the attacker's live
`CR_ARMOR_PENETRATION` percentage with C++'s `maxArmorPen` cap, the attacker's
`SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sum covering
`SPELL_SCHOOL_MASK_NORMAL`, the level-59 extension and the 75% clamp — while
`GetArmorMultiplierForTarget` is `1.0` for every 3.4.3 unit.
`PlayerEffectiveCombatStatsLikeCpp` gained `armor_penetration_pct`, written once
by the stat projection as `GetRatingBonusValue(CR_ARMOR_PENETRATION)` clamped to
100, so both swing owners read one canonical value and the map runtime needs no
combat-ratings table; `RepresentedArmorMitigationLikeCpp` carries the resolved
inputs into the shared `take_canonical_player_attack_swings_like_cpp`, with the
session resolving them in `represented_melee_armor_mitigation_like_cpp` and the
map-owned `GlobalLegacy` runtime resolving the creature's armour/level plus the
attacker's snapshot and aura sum. Boundary:
`SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` (345) and
`SPELL_AURA_MOD_IGNORE_TARGET_RESIST` (269) have no represented producer, the
`SpellModOp::TargetResistance` adjustment cannot apply to an auto-attack, a
canonical-player victim contributes no armour in either owner (that snapshot
belongs to the victim's session), the represented Player has no
`SetMeleeDamageSchool` override so the swing is physical, and the final
`max(1.0).round()` conversion is unchanged. Evidence:
`armor_reduction_matches_calc_armor_reduced_damage_like_cpp` pins the formula
(armour 0/5,000/10,000,000, 25%/100% penetration, a cancelled armour value and
the sub-60 cap), `white_swing_applies_victim_armor_mitigation_like_cpp` holds the
session owner to 753/1,000 damage and proves a non-normal
`MOD_TARGET_RESISTANCE` row is ignored while the normal row cancels the armour,
and `map_owned_player_melee_applies_victim_armor_mitigation_like_cpp` holds the
production map-owned tick to the same 753/1,000; wow-world --lib is 3967/0/1 and
world-server --lib 594/0/0; format, `git diff --check` and the physical ratchet
pass, and `validation-v2 quick` (manifest
`20260917T124035.688694Z-2724130-quick.json`) passes. `validation-v2 final` stops
only at the pre-existing `hotspot-ratchet` baseline failure (manifest
`20260917T124051.127247Z-2724225-final.json`). No live DB/restart/relogin QA. #29
remains open for the `RollMeleeOutcomeAgainst` hit table and the
`MeleeDamageBonusTaken` victim chain (both delivered by the entries above) and the
remaining spell/melee math.

**#29 white-swing damage roll — 2026-09-17, implementation `cdc6ea1a`, integrated
as `eae76cf2` by PR #1083:** C++ `Unit::CalculateMeleeDamage`
(`Unit.cpp:1326-1334`) rolls `Unit::CalculateDamage` and passes the rolled value
into `MeleeDamageBonusDone`; `CalculateDamage` (`Unit.cpp:2384-2435`) clamps both
`UnitData` bounds at zero, orders them, truncates to `uint32` and returns
`urand(min, max)`. The represented white swing instead always used the lower
bound, so every auto-attack dealt the weapon's minimum damage. The new
receiver-free rule `session_rules::white_swing_roll_like_cpp` implements that
sequence and `represented_white_swing_damage_like_cpp` calls it for the base and
offhand swings, so the session owner and the map-owned `GlobalLegacy` owner share
one implementation; the draw sits inside the landed-swing branch, so a timer that
is not ready consumes nothing. The draw mirrors C++'s process-global
`RandomEngine::Instance()` through the established `wow_core::urand_like_cpp`
wrapper (the mapping `docs/migration/common.md` records for `urand`); the
creature-owned RNG stream is a separate later design for creature runtime
authority and is untouched. Boundary: the rest of `CalculateMeleeDamage` (the
`RollMeleeOutcomeAgainst` hit table and the `CalcArmorReducedDamage` mitigation,
both delivered by the entries above), the `MeleeDamageBonusTaken` victim chain
and the final `max(1.0).round()` conversion remain as they were. Evidence:
`white_swing_roll_bounds_follow_calculate_damage_like_cpp` pins the
clamp/order/truncate bounds (zero and negative ranges, inverted fractional
`9.9`/`5.2` -> `[5, 9]`), `white_swing_damage_rolls_the_published_range_like_cpp`
lands 400 swings from a `[5, 9]` weapon inside the range and requires both bounds
plus at least four distinct values, and every equal-range exact-damage white-swing
regression stays exact; wow-world --lib is 3964/0/1 and world-server --lib
594/0/0; format, `git diff --check` and the physical ratchet pass, and
`validation-v2 quick` (manifest `20260917T120229.664585Z-2698392-quick.json`)
passes. `validation-v2 final` stops only at the pre-existing `hotspot-ratchet`
baseline failure (manifest `20260917T120326.090074Z-2698686-final.json`). No live
DB/restart/relogin QA. #29 remains open for the hit table, mitigation, the
victim-side chain and the remaining spell/melee math.

**#65 GameObject respawn-save test fidelity — 2026-09-17, implementation
`192da738`, integrated as `22423d96` by PR #1081:** the three `scenarios_9`
GameObject respawn-save regressions that this document carried as pre-existing
failures for many rounds are stale tests, not a production defect. C++
`Map::Update` reaches an object only through a player's nearby cells or
`m_activeNonPlayers` (`Map.cpp:701-767`), and the represented production
`MapObjectUpdateSelectionLikeCpp::NearbyCells` plan uses that same source set,
while the tests inserted the GameObject through the test helper's
`already_in_world` shortcut, which skips both the cell insert and
`Map::AddToMap`'s `AddToActive` tail (`Map.cpp:555-570`), leaving the map with no
visitor source at all; they were written against the pre-selection whole-store
visitor. The new local fixture
`insert_active_live_gameobject_for_spawn_like_cpp` marks the object active and
inserts it through `add_map_object_record_to_map_like_cpp`, so `Map::AddToMap`
inserts it into its cell and `AddToActive` puts it in the active non-player set
`Map::Update` visits every tick; the replaced-timer scenario additionally keeps a
non-active sibling with the identical deactivated state as a negative control, so
exactly one save is still required. No production code changed. Evidence:
`world-server --lib` is now 594/0/0 (previously 591 passed with these three
failures), format, `git diff --check` and the physical ratchet pass, and
`validation-v2 quick` (manifest `20260917T114619.975721Z-2683096-quick.json`)
passes; `validation-v2 final` still stops only at the pre-existing
`hotspot-ratchet` baseline failure (manifest
`20260917T114814.937686Z-2683594-final.json`), which this tests-only change does
not introduce. This supersedes the "three pre-existing `scenarios_9` failures"
statements in the older dated entries below.

**#61 victim aurastate/aura-mechanic melee bonuses — 2026-09-17, implementation
`1d33f078`, integrated as `44525aae` by PR #1079:** C++
`Unit::MeleeDamageBonusDone` (`Unit.cpp:7631-7648`) multiplies `DoneTotalMod` by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) when
`pVictim->HasAuraState(miscValue)` holds, and by
`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC` (249) when
`pVictim->HasAuraWithMechanic(1 << miscValue)` holds; the represented white swing
applied neither in either owner. `session_rules::melee_damage_bonus_done_like_cpp`
(renamed from `..._creature_type_like_cpp`) now takes the victim's aura-state and
mechanic masks and folds both multipliers, and its early return on an empty
creature-type mask is gone because these two terms apply without a creature type.
`Unit::HasAuraWithMechanic` now has a single receiver-free implementation in
`session_rules` (`aura_application_mechanic_mask_like_cpp` for a canonical
Player's `AuraApplication` map, `applied_aura_mechanic_mask_like_cpp` for a
creature's `AppliedAuraRef` list), replacing the session-local copy the
spell-damage path used; the session target helper delegates to it. The session
owner reads `represented_target_aura_state_mask_like_cpp` (already shared with the
spell path) and `represented_target_mechanic_mask_like_cpp`; the map-owned
`GlobalLegacy` production phase derives the creature victim's aurastate mask from
its aura subsystem plus `WorldCreature::health_aura_state_like_cpp` and the
mechanic mask from its applied auras at the map's own difficulty, collected during
the canonical collect phase so the execute phase keeps the established
canonical-then-legacy lock order. Boundary: a canonical-player victim resolved
through the runtime's non-creature branch still contributes no masks (that state
is owned by the victim's session), and the victim
`SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`/`RANGED_...` term still has no
represented creature-aura producer. Evidence:
`white_swing_applies_victim_aurastate_and_mechanic_melee_bonus_like_cpp` holds the
session owner to `pct` 1.0/2.0/4.0 and a 28-damage swing from a 7-damage weapon,
`map_owned_player_melee_applies_victim_aurastate_and_mechanic_bonus_like_cpp`
holds the map-owned production tick to 5/10/20 damage, and wow-world --lib is
3962/0/1; format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T112226.278946Z-2668271-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT term, the
blocked victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains
(creature aura producers are absent), the ranged/offhand attack-type selection for
the AP term, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 melee creature-type damage bonus — 2026-09-17, implementation `db3c2db6`,
integrated as `6d9b91bc` by PR #1077:** C++ `Unit::MeleeDamageBonusDone`
(`Unit.cpp:7558-7650`) adds the flat `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` (59)
benefit, converts the attacker's `SPELL_AURA_MOD_MELEE_ATTACK_POWER_VERSUS` (102)
sum with `GetAPMultiplier`, and multiplies by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` (168), all keyed on the victim's
`GetCreatureTypeMask`; the represented white swing applied none of them in either
owner. `session_rules::melee_damage_bonus_done_creature_type_like_cpp` is the
receiver-free rule both owners call (built on the shared aura projection from PR
#1073, so they cannot drift), `RepresentedMeleeDamageBonusLikeCpp` carries the
`(DoneFlatBenefit, DoneTotalMod)` pair into the shared swing function which now
computes `max((rolled + flat) * pct * autoattack_pct, 1.0)`, the session resolves
the victim from the canonical `attacking()` target with the existing represented
target helpers, and the map-owned runtime gained the immutable
`creature_template_lifecycle_store` on `LegacyCreatureAggroConfigLikeCpp`
(populated in `app.rs` beside the spell stores) with the config now threaded into
`run_legacy_player_melee_tick_once_like_cpp`. Boundary: the victim's
`SPELL_AURA_MELEE_ATTACK_POWER_ATTACKER_BONUS`/`RANGED_...` term has no
represented creature-aura producer; the versus-aurastate (303) and
target-aura-mechanic (249) multipliers were the next unit, delivered by the entry
above. Evidence: the new
scenario registers a type-7 creature template and asserts `flat = 5` from the
creature-type aura alone, `pct = 2.0` after adding the versus aura, and a
24-damage swing from 7 (`(7 + 5) * 2`); wow-data --lib 753/0, wow-entities --lib
940/0 and wow-world --lib 3960/0/1 pass, as do format, `git diff --check`, the
physical ratchet (two recorded ceiling growths) and `validation-v2 quick`
(manifest `20260917T105857.497150Z-2661037-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 cast-speed auras — 2026-09-17, implementation `fa7abbfd`, integrated as
`a7b1faf0` by PR #1075:** C++ `Spell::prepare` stores `m_casttime` after
`Unit::ApplyCastTimePercentMod` (`Unit.cpp:10229-10252`), driven by
`AuraEffect::HandleModCastingSpeed` (`SpellAuraEffects.cpp:4272-4315`) and
`HandleModCombatSpeedPct` (`4330-4351`). The represented cast state copied the
raw DB2 `CastTime`, so haste auras never shortened a cast.
`represented_cast_speed_multiplier_like_cpp` folds
`SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK` (65), `SPELL_AURA_HASTE_SPELLS` (216),
`SPELL_AURA_MELEE_SLOW` (193) and `SPELL_AURA_MOD_SPEED_SLOW_ALL` (252) with the
C++ conversion (positive `100/(100+amount)`, negative `(100-amount)/100`), and a
total of `1000` is C++'s `SetInstantCast`, represented as a zero multiplier. The
`player_cast::Runtime` seam gained `cast_time_multiplier` (default `1.0`) and
`prepare` applies it once when building `SpellCastState`, so the stored state,
the `timed` gate and the published `SMSG_SPELL_START` agree; the session
implements it from its canonical auras. A mislabeled constant found while wiring
this was fixed: the file declared both `SPELL_AURA_HASTE_SPELLS = 73` and
`SPELL_AURA_MOD_POWER_COST_SCHOOL = 73`, while C++ has
`MOD_POWER_COST_SCHOOL = 73` and `HASTE_SPELLS = 216`, so the duplicate was
removed and its only (test) consumer updated. Boundary: the spell-group
de-duplication and the `ModHasteRegen` cooldown-recovery consumer remain
unrepresented. Evidence: the new scenario prepares a 1500 ms cast with and
without a +50% aura and asserts the stored cast time is 1500 then 1000, with the
multiplier reading `100/150`; wow-data --lib 753/0, wow-entities --lib 940/0 and
wow-world --lib 3959/0/1 pass, as do format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T095808.364454Z-2630347-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the remaining `MeleeDamageBonusDone` terms (which need the
victim's creature type and mechanic masks, so the map-owned runtime needs the
creature-template store plumbed into `LegacyCreatureAggroConfigLikeCpp`), the
Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 white-swing auto-attack damage aura — 2026-09-17, implementation
`02f1affc`, integrated as `ccab6018` by PR #1073:** C++
`Unit::MeleeDamageBonusDone` (`Unit.cpp:7620-7627`, tail `7666-7667`) adds every
active `SPELL_AURA_MOD_AUTOATTACK_DAMAGE` percentage to `DoneTotalMod`, which
multiplies the swung damage; the represented white swing ignored it in both the
session and the production map-owned runtime. New receiver-free
`session_rules/rules_3.rs` hosts the aura-effect projection
`player_aura_effects_by_spell_aura_type_like_cpp` (deterministic ascending slot
order) and `represented_autoattack_damage_multiplier_like_cpp`; the session's
`resolved_aura_effects_by_spell_aura_type_like_cpp` now delegates to it, so both
owners resolve the canonical Player auras with one implementation instead of two
drifting copies. `Unit::mod_autoattack_damage_pct_like_cpp` carries the product
on the canonical Player and the owning session recomputes it inside the existing
aura-mutation sync (the same place that writes the attack-time multipliers), so
the map-owned swing path only reads it and there is one writer per transition;
`represented_white_swing_damage_like_cpp` applies it in the shared swing function
both owners call. Boundary: the remaining `MeleeDamageBonusDone` terms (flat
creature-type damage, `MOD_MELEE_ATTACK_POWER_VERSUS` with `GetAPMultiplier`,
versus-creature-type, versus-aurastate and target-aura-mechanic multipliers) need
the victim's creature type and mechanic masks and were the next unit, delivered by
the melee creature-type and victim aurastate/aura-mechanic entries above. Evidence:
the new scenario asserts a 7-damage white swing, 14 once a +100%
`SPELL_AURA_MOD_AUTOATTACK_DAMAGE` aura is applied (with the canonical multiplier
reading 2.0) and 7 again after removal; wow-data --lib 753/0, wow-entities --lib
940/0 and wow-world --lib 3958/0/1 pass, as do format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T094729.144549Z-2619377-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the remaining `MeleeDamageBonusDone` terms, the Shadow Bite
per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 displayed-power ownership and the shapeshift Unit field — 2026-09-17,
implementation `b48b7204`, integrated as `dd9b138b` by PR #1071:** C++
`Unit::CalculateDisplayPowerType` (`Unit.cpp:5550-5600`) selects
`UNIT_FIELD_DISPLAYPOWER` from the active form, then an active
`SPELL_AURA_MOD_POWER_DISPLAY`, then `ChrClasses`; `Unit::UpdateDisplayPower`
(`Unit.cpp:5600-5603`) is reached from `Player::InitDataForForm`
(`Player.cpp:22090`) and `AuraEffect::HandleAuraModPowerDisplay`
(`SpellAuraEffects.cpp:4027-4039`). The represented runtime never recomputed it,
so a druid in cat form kept the mana bar.
`Unit::calculate_display_power_type_like_cpp` and
`represented_power_type_from_u8_like_cpp` implement the selection;
`sync_represented_display_power_like_cpp` writes the canonical field and
publishes the changed values delta from the form-change sync and from any aura
mutation carrying `SPELL_AURA_MOD_POWER_DISPLAY`. Wiring it exposed a
second-authority defect: the session only wrote the transitional `Player`
gameplay-state projection and never `Unit::data.shapeshift_form` (the published
`UNIT_FIELD_BYTES_2` byte C++ `Unit::SetShapeshiftForm` writes), so the reader
never saw the canonical value. The setter now writes the Unit field and keeps the
projection in sync, the reader prefers the Unit field with the projection as
fallback, and `Unit::set/get_shapeshift_form_id_like_cpp` expose the raw byte so
ids the represented enum does not name (`FORM_GHOUL`, epic flight) still select
their power type. Boundary: the vehicle and hunter/warlock pet branches of
`CalculateDisplayPowerType`, and the skill-boost/collision-height half of
`HandleAuraModShapeshift`, remain unrepresented. Evidence: one scenario asserts
the mana default, `POWER_ENERGY` in cat form with a published UPDATE_OBJECT, the
mana restore on removal and `POWER_RAGE` in bear form; another asserts a
`SPELL_AURA_MOD_POWER_DISPLAY` aura selects `POWER_RUNIC_POWER` outside a form;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3957/0/1 pass,
as do format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T092920.288970Z-2602350-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura
producers are absent), the ranged/offhand attack-type selection for the AP term,
the offhand-damage aura scale, the melee auto-attack `MeleeDamageBonusDone`
chain (the production swing owner is the global legacy runtime, so it needs the
shared Player+SpellStore resolver and the creature-template plumbing), the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 `HandleShapeshiftBoosts` — 2026-09-17, implementation `9cb30cd8`,
integrated as `74f9efac` by PR #1069:** C++
`AuraEffect::HandleShapeshiftBoosts` (`SpellAuraEffects.cpp:1325-1464`) owns the
form's bonus spells and the stance-gated self-aura sweep; the represented form
change installed only the attack times, so a druid entered cat or bear form
without the form's passive boosts and kept auras a shapeshift should drop. New
`session/spell_state/shapeshift.rs` implements both branches:
`apply_represented_shapeshift_boosts_like_cpp` casts the form's hardcoded boost
ids (including the glyph-gated Shadowform and Spectral Wolf choices, and
reproducing C++'s absent `FORM_DIRE_BEAR_FORM` entry) plus every known passive or
`SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG` spell whose `Stances`
mask admits the form; `remove_represented_shapeshift_boosts_like_cpp` drops the
removed form's owned boost auras and sweeps every self-cast aura whose `Stances`
the resulting form no longer admits, reproducing `Aura::IsRemovedOnShapeLost`.
`sync_represented_shapeshift_form_ownership_like_cpp` now returns the mutation
direction and resulting form so the sync runs the matching branch before the item
refresh and the stat republish. Boundary: the skill-boost, `UpdateDisplayPower`
and collision-height half of `HandleAuraModShapeshift` remains unrepresented.
Evidence: one scenario asserts the cat-form boost spell 3025 appears on entry and
is dropped by `RemoveOwnedAura` on exit, another asserts a known passive with a
cat-form `Stances` mask is applied on entry and swept when the form is lost;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3955/0/1 pass,
as do format, `git diff --check`, the physical ratchet (one new bounded
`spell_state/shapeshift.rs`) and `validation-v2 quick` (manifest
`20260917T091150.444546Z-2580017-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 form-change item effect refresh — 2026-09-17, implementation
`5c427e5c`, integrated as `14b07630` by PR #1067:** C++
`Player::InitDataForForm` (`Player.cpp:22093-22094`) calls
`Player::UpdateEquipSpellsAtFormChange` on every shapeshift apply and removal:
each equipped item runs `ApplyItemEquipSpell(item, false, true)` and
`(item, true, true)` so spells whose `Stances` no longer fit are dropped and the
now-fitting ones are applied, and the item-set auras replay under the new form.
The represented `apply_represented_item_set_aura_refresh_events_like_cpp`
planner existed but nothing outside tests ever called it, so item-set and equip
spells gated by a form were inert.
`refresh_represented_item_effects_at_form_change_like_cpp` now removes the
equipped items' spell auras the new form rejects (matching on the item caster
GUID), re-applies each item's equip spells through the existing form-aware
`apply_initial_item_equip_auras_like_cpp`, and replays the item-set auras with
`form_change = true`; `sync_represented_shapeshift_form_like_cpp` runs it
between the attack-time update and the stat republish. Boundary:
`HandleShapeshiftBoosts` (the form's bonus spells) and the form display/
power-type switch remain unrepresented. Evidence: two new scenarios equip an
item carrying a cat-form-gated `ItemEffect` equip spell and a two-piece set
bonus gated the same way, then assert both auras appear when the form aura is
applied and disappear when it is removed; wow-data --lib 753/0, wow-entities
--lib 940/0 and wow-world --lib 3953/0/1 pass, as do format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T085935.668738Z-2566293-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, `HandleShapeshiftBoosts`, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 shapeshift form ownership and `CombatRoundTime` — 2026-09-17,
implementation `eddc62de`, integrated as `08f0bd3c` by PR #1065:** C++
`AuraEffect::HandleAuraModShapeshift` (`SpellAuraEffects.cpp:1838-1866`) owns
`Unit::SetShapeshiftForm`, and `Player::InitDataForForm`
(`Player.cpp:22076-22098`) installs the form's
`SpellShapeshiftFormEntry::CombatRoundTime`, consumed by
`Player::CalculateMinMaxDamage` (`StatSystem.cpp:461-467`) and
`Player::_ApplyWeaponDamage` (`Player.cpp:8018-8020`). The represented form
mirror had no producer, so a feral form never changed the attack time or the
weapon damage. `sync_represented_shapeshift_form_ownership_like_cpp` now sets
the form from an applied `SPELL_AURA_MOD_SHAPESHIFT` effect and clears it on
removal only when no other active aura still applies one;
`represented_shapeshift_combat_round_time_like_cpp` and
`apply_represented_shapeshift_base_attack_time_like_cpp` mirror
`InitDataForForm`/`SetRegularAttackTime` (writing only an attack whose equipped
weapon declares a delay) and `sync_represented_shapeshift_form_like_cpp` runs
both plus the stat republish; `effective_weapon_damage_ranges_like_cpp` applies
the `CombatRoundTime / 1000 / GetAPMultiplier` weapon rescale; and
`represented_can_use_attack_type_like_cpp` (extracted from the enchantment gate)
returns `None` for an unavailable canonical owner so the enchantment path stays
fail-closed while `_ApplyWeaponDamage` treats an unknown unit as unflagged.
Boundary: `HandleShapeshiftBoosts` (the form's bonus spells), the form display
and power-type switch, and `UpdateEquipSpellsAtFormChange` remain
unrepresented. Evidence: the new scenario asserts no round time before the aura,
the resolved 1000 ms form after it, main/off-hand base attack times at the form
value with ranged at 2000, and the form clearing on removal; the pure-function
test asserts the feral rescale and the zero-round-time branch; wow-data --lib
753/0, wow-entities --lib 940/0 and wow-world --lib 3951/0/1 pass, as do format,
`git diff --check`, the physical ratchet (two recorded one-line call-site
growths on `items.rs` and `world_entry.rs`) and `validation-v2 quick` (manifest
`20260917T084828.044088Z-2556349-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune regeneration and
live DB/restart/relogin QA.

**#61 attack-speed aura application — 2026-09-17, implementation
`dc8dac1b`, integrated as `920587ff` by PR #1063:** C++
`Unit::ApplyAttackTimePercentMod` (`Unit.cpp:10201-10227`) maintains
`m_modAttackSpeedPct` from the attack-speed aura handlers
(`SpellAuraEffects.cpp:4353-4393`), and the represented Unit owned the field
with no writer, so every swing used the unhasted base attack time.
`Unit::apply_attack_time_multipliers_like_cpp` installs the per-attack
multipliers while preserving the remaining fraction of the current swing;
`represented_attack_speed_multipliers_like_cpp` reproduces the handler mapping
(9 main hand; 138/217/319 main+off; 140 ranged; 192/342 all three; 193/252 all
three) and the exact C++ amount conversion (positive uses
`100 / (100 + amount)`, negative uses `(100 - amount) / 100`, which is not the
inverse); and `sync_represented_attack_speed_like_cpp` rewrites the canonical
Player values after every aura apply and removal, so the session that owns the
auras stays the only writer while the map-owned swing path only reads. Boundary:
the C++ `GetHighestExclusiveSameEffectSpellGroupValue` de-duplication of
`MOD_MELEE_HASTE`/`MELEE_SLOW` needs spell-group tables the represented session
does not load, so every active effect multiplies, and
`ApplyCastTimePercentMod` has no represented consumer yet. Evidence: the new
scenario asserts the `[1.0; 3]` sentinel, `100/130` on main and off hand from a
+30 `SPELL_AURA_MOD_MELEE_HASTE`, the swing-timer reset using the hasted
duration, the `1.3` factor from a -30 `SPELL_AURA_MOD_SPEED_SLOW_ALL` on all
three attacks, and the restore after each removal; wow-data --lib 753/0,
wow-entities --lib 940/0 and wow-world --lib 3950/0/1 pass, as do format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T082722.684280Z-2531109-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the shapeshift `CombatRoundTime` weapon-damage path,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate
powers, rune regeneration and live DB/restart/relogin QA.

**#61 `CONFIG_STATS_LIMITS_*` caps — 2026-09-17, implementation
`5eef7a77`, integrated as `604381f9` by PR #1061:** C++
`Player::UpdateBlockPercentage`, `UpdateDodgePercentage`,
`UpdateParryPercentage` and `UpdateCritPercentage` cap their published
percentages with `value > limit ? limit : value` while
`CONFIG_STATS_LIMITS_ENABLE` is set (`StatSystem.cpp:496-497`, `506-507`,
`675-676`, `712-713`; `World.cpp:1664-1668` defaults `false`/`95.0`), which the
represented projection ignored. `wow_data::StatsLimitsLikeCpp` now carries the
five values with `clamp_block`/`clamp_dodge`/`clamp_parry`/`clamp_crit`
helpers; `world-server` resolves the already-registered
`CONFIG_STATS_LIMITS_*` world-config keys and hands the immutable value to the
session through the existing `SessionRuntimePolicyCapabilitiesLikeCpp` beside
the durability rate; and `player_stat_system_projection_like_cpp` applies the
caps once, so the login create snapshot and the canonical effective-stats
snapshot cannot diverge. Boundary: the caps cover exactly the six published
fields C++ writes; the represented model still has no per-caster avoidance aura
state or diminishing-returns difference. Evidence: the three new/extended
tests cover the enabled/disabled, boundary (`>`, not `>=`) and negative cases,
the world-config file keys and defaults, and a login projection whose
parry/block/crit read 5.0 by default and 1.0 once the limits are enabled;
wow-data --lib 753/0, wow-entities --lib 940/0 and wow-world --lib 3949/0/1 pass,
and `world-server --lib` reports 591 passed with the three `scenarios_9`
gameobject-compatibility failures that reproduce unchanged on the untouched base
(verified with the change stashed), so they are pre-existing and outside this
unit; they were later resolved as stale test setups by the #65 GameObject
respawn-save test fidelity entry above. Format, `git diff --check`, the physical ratchet (the one-line composition
wiring is recorded on `crates/world-server/src/app.rs`, 5658 -> 5659) and
`validation-v2 quick` (manifest `20260917T080332.494026Z-2517083-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT
term, the blocked victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken`
chains (creature aura producers are absent), the ranged/offhand attack-type
selection for the AP term, the offhand-damage aura scale, the player-killer
(PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `BonusCoefficientFromAP` table term — 2026-09-17, implementation
`de597571`, integrated as `a00f9efe` by PR #1059:** the C++ "Check for table
values" block of `Unit::SpellDamageBonusDone`/`SpellHealingBonusDone`
(`Unit.cpp:6633-6649`, `7132-7140`) adds
`int32(stack * BonusCoefficientFromAP * APbonus)` to the flat done benefit. The
DB2 field was already loaded into `SpellEffectDb2Entry` but dropped by the
runtime `SpellEffectInfo`, so represented direct damage and heals ignored it.
`wow_data::SpellEffectInfo` now carries `effect_bonus_coefficient_from_ap`
(`DB2Structure.h:3525`, `SpellInfo.cpp:436`), populated by
`spell_effect_from_db2_like_cpp`; the new
`represented_spell_bonus_coefficient_from_ap_like_cpp` resolves
`Unit::GetTotalAttackPowerValue(BASE_ATTACK)` through the canonical Player
accessor, and both bonus helpers add the term.
`represented_spell_has_attribute_like_cpp` centralises the difficulty-resolved
attribute read now shared by the three attribute gates, and
`SpellDamageBonusDone`'s `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` early-out also
skips the flat advertised benefit, matching the C++ order
(`Unit.cpp:6607-6612`). Boundary: `stack` is always one, the
`SpellModOp::BonusCoefficient` adjustment is not represented, and the attack
type is always `BASE_ATTACK` because the represented `SpellInfo` carries no
`SpellFamilyName`/`EquippedItemSubClassMask`, so the C++
`RANGED_ATTACK`/`OFF_ATTACK` selection and the victim's
`SPELL_AURA_*_ATTACK_POWER_ATTACKER_BONUS` term remain unrepresented. Evidence:
three new scenarios in `session/tests/scenarios_spell_state_25.rs` assert
`int32(3.0 * 100)` extra damage with 100 attack power, `int32(2.0 * 100)` extra
healing, and that an attr3 spell skips the flat `BonusCoefficient` benefit while
the un-gated one keeps it; wow-data --lib 752/0, wow-entities --lib 940/0 and
wow-world --lib 3949/0/1 pass, as do format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T072428.572336Z-2496250-quick.json`). No live DB/restart/relogin QA.
#61 stays open for the Shadow Bite per-DoT term, the blocked victim-side
`SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura producers
are absent), the ranged/offhand attack-type selection for the AP term, the
offhand-damage aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP`
branch and `SetPvPDeath`, alternate powers, rune regeneration and live
DB/restart/relogin QA.

**#61 `SpellHealingPctDone` completion — 2026-09-17, implementation
`1eab52dc`, integrated as `003e1bb9` by PR #1057:** the represented direct heal
applied only the published `ModHealingDonePercent` and the missing-health aura,
so C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7185-7229`) was incomplete in two
ways. `represented_healing_pct_done_gated_like_cpp` now returns `1.0f` for
`SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` (`SharedDefines.h:571`) and
`SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` (`SharedDefines.h:680`) while the flat
`SpellBaseHealingBonusDone` benefit stays in place, exactly as C++ returns from
`SpellHealingPctDone` alone; the un-gated chain multiplies by
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) for every active victim aura
state before the missing-health term. `represented_aura_state_bit_like_cpp`
fails closed on an out-of-range aura state instead of shifting out of range, and
the damage path uses the same helper. Boundary: the `SPELLFAMILY_POTION`
early-out (the represented `SpellInfo` has no family name), the 354
`IsAffectingSpell` family/flag gate, the totem redirect and the scripted handlers
remain unrepresented. Evidence: the two new scenarios in
`session/tests/scenarios_spell_state_25.rs` assert a 100 heal becomes 200 against
a defensive-aurastate creature, and that a coefficient-0.5 heal with 100 spell
power heals 225 without the attribute and 150 with
`SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` (proving the flat benefit survives the
gate); wow-data --lib 752/0, wow-entities --lib 940/0 and wow-world --lib
3946/0/1 pass, as do format, `git diff --check`, the physical ratchet (the
three-line `SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` constant is recorded on
`crates/wow-data/src/spell/mod.rs`, 127 -> 130) and `validation-v2 quick`
(manifest `20260917T071254.960525Z-2480943-quick.json`). No live DB/restart/
relogin QA. #61 stays open for the Shadow Bite per-DoT term, the blocked
victim-side `SpellDamageBonusTaken`/`MeleeDamageBonusTaken` chains (creature aura
producers are absent), the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 remaining `SpellDamagePctDone` terms — 2026-09-17, implementation
`04dd26c3`, integrated as `150664e5` by PR #1055:** the last three pieces of
C++ `Unit::SpellDamagePctDone` (`Unit.cpp:6683-6772`) were missing from the
represented direct-damage path. `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS`
(`SharedDefines.h:571`) and `SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS`
(`SharedDefines.h:682`) now return `1.0f` before any done term is read; Mage Ice
Lance (228598) multiplies by three on a victim carrying `AURA_STATE_FROZEN`
(`Unit.cpp:6750-6755`); and Warlock Drain Soul (198590) doubles while the caster
carries `AURA_STATE_WOUNDED_20_PERCENT` (`Unit.cpp:6762-6765`). Both scripted
terms read the represented `Unit::HasAuraState` union added by PR #1053;
`AURA_STATE_FROZEN` (=4, `SharedDefines.h:2510`) is added. Boundary: the Warlock
Shadow Bite per-DoT term, the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` shortcut of
`Unit::HasAuraState`, and the `SpellFamilyName` switch guard (the represented
`SpellInfo` has no family name, so the globally unique spell id keys the branch)
remain unrepresented. Evidence: the three new scenarios in
`session/tests/scenarios_spell_state_25.rs` assert the versus-aurastate +100%
aura applies to a plain spell (200 damage) but not to an attr6- or attr3-gated
one (100 each), Ice Lance deals 100 unfrozen and 300 frozen, and Drain Soul deals
100 from a healthy caster and 200 from a wounded one; wow-data --lib 752/0,
wow-entities --lib 940/0 and wow-world --lib 3944/0/1 pass, as do format,
`git diff --check`, the physical ratchet (the six-line inline `attributes`
addition is recorded on `crates/wow-data/src/spell/mod.rs`, 121 -> 127) and
`validation-v2 quick` (manifest `20260917T070107.585649Z-2470766-quick.json`).
No live DB/restart/relogin QA. #61 stays open for the Shadow Bite per-DoT term,
`SpellDamageBonusTaken` and the remaining victim-side gates, the offhand-damage
aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 health-derived unit aura states — 2026-09-17, implementation
`4555be48`, integrated as `2db5424d` by PR #1053:** C++ `Unit::Update`
(`Unit.cpp:469-476`) maintains the health-derived bits of
`Unit::m_unitData->AuraState` (WOUNDED_20/25/35, WOUND_HEALTH_20_80,
HEALTHY_75, WOUND_HEALTH_35_80) for every alive Unit, and
`Unit::HasAuraState` reads that union. The represented runtime split the union:
the damage path read only the aura subsystem's aura-driven bits, so
`SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303) never matched a health state,
and `represented_has_aura_state_like_cpp` was a hardcoded `false` stub that
denied every `Spell::CheckCast` caster-aura-state restriction.
`represented_player_aura_state_mask_like_cpp` now ORs the canonical Player's
aura-driven mask with `WorldCreature::health_aura_state_like_cpp` over the
resolved vitals, `represented_unit_aura_state_mask_like_cpp` extends the union to
a world creature, `represented_target_aura_state_mask_like_cpp` feeds the damage
path the same union, and `represented_has_aura_state_like_cpp` answers the cast
restriction with checked shifting (a positive state outside the 32-bit mask
denies rather than panics). `AURA_STATE_WOUNDED_20_PERCENT` (=2,
`SharedDefines.h:2508`) is added. Boundary: the per-caster aura-state branch of
`Unit::HasAuraState` (`PER_CASTER_AURA_STATE_MASK`, RAID_ENCOUNTER_2 and
ROGUE_POISONED) and the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` shortcut remain
unrepresented; a unit that cannot be resolved still yields mask zero. Evidence:
the three new scenarios in the bounded `session/tests/scenarios_spell_state_25.rs`
(creature below 20% health receives the aura-303 multiplier while a healthy one
does not; the player mask is empty at 50% health and carries WOUNDED_20 at 10%
while HEALTHY_75 clears; the login passive cast gate follows the health-derived
caster aura state) pass, as do wow-entities --lib 940/0 and wow-world --lib
3941/0/1, format, `git diff --check`, the physical ratchet (after
scenarios_spell_state_11.rs reached the 2,000-line test-file budget and the split
registration needed the recorded session_tests.rs ceiling move 5766 -> 5768) and
`validation-v2 quick` (manifest `20260917T065115.813498Z-2459846-quick.json`).
No live DB/restart/relogin QA. #61 stays open for the family-scripted and
attribute-gated damage terms, `SpellDamageBonusTaken` and the remaining
victim-side gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 mechanic-based damage multipliers — 2026-09-17, implementation
`f190dd97`, integrated as `d23bfd17` by PR #1051:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6734-6746`) still omitted the two mechanic terms of the represented
direct-damage path. `represented_target_mechanic_mask_like_cpp` now mirrors
`Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`), unioning each applied aura's
`SpellInfo::Mechanic` and the mechanics of its applied `IsEffect()` slots, for
the canonical player's visible applications (using the application's stored
difficulty) and for world-creature `AppliedAuraRef`s (which retain none, so the
current map difficulty is used). `represented_spell_damage_pct_done_like_cpp`
multiplies the damage by every caster aura-249 effect whose misc mechanic the
victim carries and applies the summed aura-276 percentage for the cast effect's
`SpellEffectInfo::Mechanic`, falling back to `SpellInfo::Mechanic`, resolved
through `SpellHitMetadataLikeCpp`; the bonus helper now receives the C++ effect
index for that lookup. Boundary: the family-scripted terms and the
`SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` /
`SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS` early-outs remain unrepresented.
Evidence: the two new session tests assert the matching and non-matching mechanic
cases for auras 249 and 276 (`spell_school_damage_applies_damage_done_for_mechanic_like_cpp`,
`spell_school_damage_applies_damage_percent_done_by_target_aura_mechanic_like_cpp`);
wow-data --lib 752/0 and wow-world --lib 3938/0/1; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T062936.129884Z-2436096-quick.json`) pass. `validation-v2 final` was
also run on this candidate: it fails only at `check_architecture.py
hotspot-ratchet`, which already fails identically on the untouched base
`35535748` (`crates/wow-world/src/session/mod.rs` +5591 total lines against the
stored baseline, `handlers/character/mod.rs` +1550, and others); the drift is
pre-existing repository debt outside this unit's scope and was not re-baselined.
No live DB/restart/relogin QA. #61 stays open for the family-scripted and
attribute-gated damage terms, `SpellDamageBonusTaken` and the remaining
victim-side gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 versus-aurastate damage multiplier — 2026-09-17, implementation
`cf8190d5`, integrated as `93f20092` by PR #1049:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6728-6734`) multiplies spell damage by
`GetTotalAuraMultiplier(SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE)` for every
active effect whose misc aura state the victim carries, which the represented
damage path omitted. `represented_target_aura_state_mask_like_cpp` reads the
target's unit aura state mask (the canonical player's or the world creature's),
and `represented_spell_damage_pct_done_like_cpp` multiplies its school maximum by
each matching aura-303 effect. Boundary: the target-aura-mechanic (249) and
`MOD_DAMAGE_DONE_FOR_MECHANIC` terms (resolved next by the 2026-09-17
mechanic-based multipliers entry) plus the family scripts remained
unrepresented, and a target whose aura state cannot be resolved keeps only the
previously applied multipliers. Evidence: the new session test sets the
defensive aura state on a registered creature, asserts an unrelated-state aura
(misc 2) leaves 100 damage and the matching aura (misc 1, +100%) doubles it to
200; wow-data --lib 752/0 and wow-world --lib 3936/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T061932.575648Z-2427345-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the mechanic/family damage terms, `SpellDamageBonusTaken`
and the remaining victim-side gates, the offhand-damage aura scale, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 creature missing-health heal scaling — 2026-09-17, implementation
`5011ad4a`, integrated as `3afbd42b` by PR #1047:** the missing-health healing
term added in PR #1043 only applied when the victim was the session player, so
heals landing on creatures ignored
`SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH` (354).
`represented_target_health_pct_like_cpp` now resolves the target's health
percentage from the canonical player vitals or the world creature's runtime
health (`WorldCreature::current_hp`/`max_hp`), and
`represented_spell_healing_bonus_done_like_cpp` applies the scaling for any
resolvable target. Boundary: the aura's `AuraEffect::IsAffectingSpell`
family/flag gate is still not represented, and a target that cannot be resolved
keeps the unscaled percentage. Evidence: the new session test damages a
registered creature to 500/1000 health, applies aura 354 with amount 100 and
casts a 100 base heal, asserting 150 healing (`int32(100 * 1.5)`) and 650
remaining health; the player-victim heal scenario stays green; wow-data --lib
752/0 and wow-world --lib 3935/0/1; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T061323.951908Z-2419707-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining aurastate/mechanic and victim-side terms,
the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 versus-creature-type damage multiplier — 2026-09-17, implementation
`7f2aa52c`, integrated as `5b0c9954` by PR #1045:** `Unit::SpellDamagePctDone`
(`Unit.cpp:6724-6726`) multiplies spell damage by
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE_VERSUS, victim->
GetCreatureTypeMask())`, which the represented damage path omitted.
`represented_target_creature_type_mask_like_cpp` implements C++
`Unit::GetCreatureTypeMask` (`Unit.cpp:8796-8800`) from the world creature's
template `creature_type` through the already-present
`CreatureTemplateLifecycleStoreLikeCpp` (players and unknown templates return 0),
and `represented_spell_damage_pct_done_like_cpp` multiplies its school maximum by
every active aura-168 effect whose misc intersects the victim's type mask.
`CreatureTemplateLifecycleRecordLikeCpp` derives `Default` so fixtures can build
a template with only the fields they exercise. Boundary: the aurastate (303),
target-aura-mechanic (249) and `MOD_DAMAGE_DONE_FOR_MECHANIC` terms plus the
family scripts remained unrepresented; the first two were resolved by the
versus-aurastate and mechanic-based multiplier entries above, leaving the family
scripts, and a target whose creature type is unavailable keeps only the school
percentage. Evidence: the new session test
registers a creature whose template type is 7 (bit 6), applies a versus aura for
an unrelated type (bit 1) and asserts 100 damage, then the matching aura (+100%)
and asserts 200 damage; wow-data --lib 752/0 and wow-world --lib 3934/0/1;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T060718.732178Z-2412155-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the remaining aurastate/mechanic and
victim-side terms, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 missing-health healing scaling — 2026-09-17, implementation `c4453097`,
integrated as `1043da08` by PR #1043:** `Unit::SpellHealingPctDone`
(`Unit.cpp:7204-7229`) scales healing done by the target's missing health for
`SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH` (354), which the
represented heal path omitted.
`represented_spell_healing_bonus_done_like_cpp` now applies
`AddPct(DoneTotalMod, CalculatePct(amount, healthPctDiff))` for every active
aura-354 effect when the victim is the session player, using
`resolved_player_vitals_like_cpp` for the missing-health percentage. Boundary:
the aura's `AuraEffect::IsAffectingSpell` family/flag gate is not represented, so
the term applies to any represented heal the aura owner casts; creature victims
keep it unapplied because their health and auras are not represented. Evidence:
the heal scenario removes the taken auras, applies aura 354 with amount 100 and
sets the player to 500/1000 health; the fourth cast asserts 382 healing
(`int32((100 + int32(140 * 0.5)) * (1.5 * 1.5))`) taking the player to 882;
wow-data --lib 752/0 and wow-world --lib 3933/0/1; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T055914.037802Z-2404133-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining caster/victim modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `SpellHealingBonusTaken` — 2026-09-17, implementation `b5a2f7a7`,
integrated as `490b1f92` by PR #1041:** `Unit::SpellHealingBonusTaken`
(`Unit.cpp:7231-7239`) multiplies healing a unit receives by the most positive
and most negative active `SPELL_AURA_MOD_HEALING_PCT` amounts, which the
represented heal path omitted.
`represented_spell_healing_bonus_taken_like_cpp`
(`session/spell_effects/effect_combat.rs`) mirrors
`GetMaxNegativeAuraModifier`/`GetMaxPositiveAuraModifier` and applies each with
`AddPct`, truncating the product at zero; `apply_heal_from_caster_like_cpp`
applies it to spell-driven heals when the target is the session player, the only
represented aura owner. Boundary: creature targets keep the raw amount because
their auras are not represented, the Nourish druid case is not modelled, and
non-spell heals are unchanged. Evidence: the heal scenario applies
`SPELL_AURA_MOD_HEALING_PCT` -50 and +25 after the second cast and asserts the
third heal is `int32(255 * (1 - 0.5) * (1 + 0.25)) = 159`, taking the player from
580 to 739 health; wow-data --lib 752/0 and wow-world --lib 3933/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T055228.342758Z-2396282-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the remaining caster/victim modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 victim `ModHealing` term — 2026-09-17, implementation `2d158052`,
integrated as `5924714b` by PR #1039:** `Unit::SpellHealingBonusDone`
(`Unit.cpp:7123-7124`) also adds the victim's
`GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_HEALING, schoolMask)` to the
advertised benefit, which the direct-heal path of PR #1037 omitted.
`represented_spell_healing_bonus_done_like_cpp` now takes the `target_guid` and
adds the `SPELL_AURA_MOD_HEALING` (115) sum whose misc intersects the spell's
`SpellMisc.SchoolMask` when the victim is the session player, the only
represented aura owner; the `SPELL_EFFECT_HEAL` branch passes the target guid
through. Boundary: creature victims keep the term unapplied because their auras
are not represented, and the `BonusCoefficientFromAP` table, the periodic-leech
suppression, the spell-mod coefficient adjustment and the scripted handlers
remain open. Evidence: the heal scenario applies a holy
`SPELL_AURA_MOD_HEALING` (+40) after the first cast and asserts the second heal
is `int32((100 + int32(140 * 0.5)) * 1.5) = 255`, taking the player from 325 to
580 health; wow-data --lib 752/0 and wow-world --lib 3933/0/1; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T054507.143715Z-2388072-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the remaining victim/caster modifier gates, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 direct-heal spell-power bonus — 2026-09-17, implementation `f7416510`,
integrated as `1a19dbcd` by PR #1037:** `Unit::SpellHealingBonusDone`
(`Unit.cpp:7100-7183`) was not represented, so `SPELL_EFFECT_HEAL` applied the
raw `EffectBasePoints`. `represented_spell_healing_bonus_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) now implements the player-caster
`int32(max(float(healamount + int32(SpellBaseHealingBonusDone(schoolMask) *
BonusCoefficient)) * DoneTotalMod, 0.0f))` with `DoneTotalMod` read from the
published `mod_healing_done_percent`, and
`represented_spell_base_healing_bonus_done_like_cpp` mirrors
`Unit::SpellBaseHealingBonusDone` (`Unit.cpp:7282-7315`): the
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise the
`SPELL_AURA_MOD_HEALING_DONE` sum whose misc is zero or intersects the school
mask, plus the published `GetBaseSpellPowerBonus()`, the mana-class intellect
term (represented by the base-mana row) and the
`SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` percentages. The
`SPELL_EFFECT_HEAL` branch in `execution.rs` routes the base through the helper
using the same `SpellMisc.SchoolMask` resolution as the damage path. Boundaries:
the victim `SPELL_AURA_MOD_HEALING` term, the `BonusCoefficientFromAP` table, the
periodic-leech suppression, the spell-mod coefficient adjustment and the scripted
handlers remain unrepresented; creature casters keep the raw value and a missing
`SpellMisc` row or canonical snapshot fails closed. Evidence: the new session
test installs a canonical player with `spell_power = 100`,
`mod_healing_done_percent = 1.5`, a holy `SpellMisc` row and a heal spell with
`effect_bonus_coefficient = 0.5` and 100 base healing, casts it at itself and
asserts 325 health (`(100 + 50) * 1.5`); wow-data --lib 752/0 and wow-world --lib
3933/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T053806.484070Z-2379570-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the versus/mechanic and
victim-modifier gates, the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 school damage percentage — 2026-09-17, implementation `32572914`,
integrated as `d94a13d4` by PR #1035:** `Unit::SpellDamagePctDone`'s player
branch was missing from the school-damage path, so the
`ActivePlayerData::ModDamageDonePercent` values published by PR #1015 did not
affect damage. `represented_spell_damage_pct_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) implements
`maxModDamagePercentSchool` (`Unit.cpp:6683-6772`): the highest published
`mod_damage_done_percent` among the spell's `SpellMisc.SchoolMask` schools, and
`represented_spell_damage_bonus_done_like_cpp` now multiplies
`(pdamage + DoneTotal)` by it before the final
`int32(max(float(...), 0.0f))` truncation. Boundaries: the versus-creature-type
(168), versus-aurastate (303), target-aura-mechanic (249) and
`MOD_DAMAGE_DONE_FOR_MECHANIC` multipliers plus the spell-family scripted cases
remain unrepresented. Evidence: the new session test installs a canonical player
with holy `mod_damage_done_percent = 1.5` and fire `2.5`, a holy|fire `SpellMisc`
row and a 100 base-damage spell, casts it at a 1000 HP creature and asserts 750
HP remaining (`100 * max(1.5, 2.5)`); the flat-term test stays green; wow-data
--lib 752/0 and wow-world --lib 3932/0/1; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T053142.856354Z-2372583-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the versus/mechanic damage multipliers, the
offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 caster spell-power damage bonus — 2026-09-17, implementation `067a6f91`,
integrated as `ec20d472` by PR #1033:** `Unit::SpellDamageBonusDone`
(`Unit.cpp:6623-6680`) was not represented, so `SPELL_EFFECT_SCHOOL_DAMAGE`
applied the raw `EffectBasePoints` and ignored the caster's spell power and
damage coefficient. `represented_spell_damage_bonus_done_like_cpp`
(`session/spell_effects/effect_combat.rs`) now implements the player-caster
`SPELL_DIRECT_DAMAGE` flat term `int32(max((pdamage +
int32(SpellBaseDamageBonusDone(schoolMask) * BonusCoefficient)) * DoneTotalMod,
0))` with `DoneTotalMod` currently 1.0, and
`represented_spell_base_damage_bonus_done_like_cpp` mirrors
`Unit::SpellBaseDamageBonusDone` (`Unit.cpp:6860-6890`): the
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise
`GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, schoolMask)` plus
the published `GetBaseSpellPowerBonus()` plus the
`SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` terms read from the canonical
snapshot's stats. The school mask comes from the session's existing
`SpellMiscStore` (`SpellMisc.SchoolMask`), so no data-model change was needed,
and the school-damage branch in `execution.rs` routes the base through the
helper before `apply_damage_from_caster_like_cpp`. Boundaries:
`SpellDamagePctDone` (the school `max ModDamageDonePercent` and the
versus/aurastate/mechanic multipliers) is not modelled yet, the represented model
stores one `BonusCoefficient` per spell rather than per `SpellEffectInfo`,
creature casters keep the raw value, and a missing `SpellMisc` row or canonical
snapshot fails closed to the raw value. Evidence: the new session test installs a
canonical player with `spell_power = 100`, a holy `SpellMisc` row and a spell
with `effect_bonus_coefficient = 0.5` and 100 base damage, casts it at a 1000 HP
creature and asserts 850 HP remaining (`(100 + 50) * 1.0`); wow-data --lib 752/0
and wow-world --lib 3931/0/1; format, `git diff --check`, the physical ratchet
and `validation-v2 quick` (manifest
`20260917T052557.573032Z-2365779-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for `SpellDamagePctDone`, the offhand-damage aura scale, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 weapon-enchantment damage term — 2026-09-17, implementation `54c1f8ec`,
integrated as `1e4bb9d4` by PR #1031:** `Player::UpdateDamageDoneMods`
(`Player.cpp:4991-5015`) adds `ITEM_ENCHANTMENT_TYPE_DAMAGE` (2)
`EffectScalingPoints` for the attack's enchanted weapon, plus the same scaled by
the weapon delay for `ITEM_ENCHANTMENT_TYPE_TOTEM` (6) on shamans. The Rust
`SpellItemEnchantmentEntry` never loaded `EffectScalingPoints`, so enchanted
weapon damage was missing. The entry now carries
`effect_scaling_points: [f32; 3]` read from DB2 field 3
(`DB2Structure.h:3573`); `Eq` is no longer derived because the field is `f32`,
and the twenty struct literals across the data/world fixtures gained the field.
`represented_weapon_enchant_damage_like_cpp`
(`session/player_items/equipment_slots.rs`) resolves the attack's usable weapon,
walks every enchantment slot and sums the damage/totem points;
`represented_weapon_damage_flat_like_cpp` now returns `[f32; 3]` (aura sum plus
the enchantment term), and `PlayerSpellBonusInputLikeCpp.weapon_damage_flat`,
`PlayerStatSystemProjectionLikeCpp` and `PlayerEffectiveCombatStatsLikeCpp` move
to `f32`, matching the C++ `SetStatFlatModifier` float storage. Evidence: the new
session test runs the same enchanted sword fixture for a shaman (class 7,
permanent enchant `effect = [DAMAGE, TOTEM]`,
`effect_scaling_points = [25.0, 10.0]`, 2.0 s weapon delay) and a warrior
(class 1), asserting `weapon_damage_flat == [45.0, 0.0, 0.0]` and `[25.0, 0.0,
0.0]`; the wow-data flat/percentage test moves to the `f32` values; wow-data
--lib 752/0 and wow-world --lib 3930/0/1; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T051430.821411Z-2355689-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 ranged weapon fit — 2026-09-17, implementation `9dd5c131`, integrated as
`616d0163` by PR #1029:** the two preceding entries recorded that the
weapon-fit producers excluded ranged weapon requirements because the
represented `GetWeaponForAttack` helper covered the melee slots only.
`Player::GetWeaponForAttack(RANGED_ATTACK, true)` (`Player.cpp:9243-9270`) reads
`EQUIPMENT_SLOT_RANGED`, so bow/gun/crossbow restricted auras were wrongly
rejected for the ranged attack. `represented_usable_weapon_item_id_like_cpp`
now resolves the ranged slot; the callers that mirror
`Player::UpdateExpertise` (`StatSystem.cpp:759-786`) still skip `RANGED_ATTACK`
themselves, because C++ returns early there, while
`UpdateWeaponDependentCritAuras`, `UpdateDamageDoneMods` and
`UpdateDamagePctDoneMods` now fit against the equipped ranged weapon. The
helper's doc comment is corrected accordingly: the ranged early return belongs to
`UpdateExpertise`, not to `GetWeaponForAttack`. Evidence: the new session test
equips a bow, applies a bow-restricted `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` (+5)
and `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (+100), and asserts `ranged_crit_pct ==
10`, `crit_pct == 5`, `offhand_crit_pct == 5` and `weapon_damage_pct == [1.0,
0.5, 2.0]`; wow-world --lib 3929/0/1; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T044535.708128Z-2334728-quick.json`) pass. No live DB/restart/relogin
QA. #61 stays open for the weapon-enchantment damage term, the offhand-damage
aura scale, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin
QA.

**#61 `Unit::UpdateDamageDoneMods` representation — 2026-09-17,
implementation `7c0232da`, integrated as `cca7331a` by PR #1027:**
`Unit::UpdateDamageDoneMods` (`Unit.cpp:8997-9027`) sets the
`UNIT_MOD_DAMAGE_*` `TOTAL_VALUE` from the physical
`SPELL_AURA_MOD_DAMAGE_DONE` (13) sum filtered by
`Player::CheckAttackFitToAuraRequirement`, and `Unit::CalculateMinMaxDamage`
adds it inside the total percentage. RustyCore never modelled the flat term, so
melee damage bonuses from that aura were missing from every represented weapon
range. `represented_weapon_damage_flat_like_cpp`
(`session/player_items/equipment_slots.rs`) reuses the represented weapon-fit
helper also used by the crit, expertise and damage-percentage producers;
`PlayerSpellBonusInputLikeCpp.weapon_damage_flat` (default `[0; 3]`) flows
through `PlayerStatSystemProjectionLikeCpp` into
`effective_weapon_damage_ranges_like_cpp` as
`((weapon + ap) + flat) * pct` (C++ `((weapon + baseValue) * basePct +
totalValue) * totalPct`), and `PlayerEffectiveCombatStatsLikeCpp` publishes the
per-attack values. Boundary: the weapon-enchantment
`ITEM_ENCHANTMENT_TYPE_DAMAGE`/`TOTEM` term that
`Player::UpdateDamageDoneMods` (`Player.cpp:4965-5015`) adds for an enchanted
weapon is not represented and remains a separate gate. Evidence: the wow-data
test pins the flat-before-percentage arithmetic and the offhand factor; the
session test extends the weapon-damage scenario with an item-neutral +20 physical
aura, asserting `weapon_damage_flat == [20, 20, 20]` and a mainhand range growth
of `20 * 3.0`; wow-data --lib 752/0, wow-entities --lib 940/0 and wow-world --lib
3928/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T043815.435806Z-2327221-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the offhand-damage aura scale, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 `Unit::UpdateDamagePctDoneMods` representation — 2026-09-17,
implementation `720bcb0a`, integrated as `8a55e6c5` by PR #1025:** the
`UNIT_MOD_DAMAGE_*` `TOTAL_PCT` that `Unit::CalculateMinMaxDamage` applies to
both weapon range bounds was never represented, so the offhand 50% penalty and
the physical damage-percentage auras were missing. `represented_weapon_damage_pct_like_cpp`
(`session/player_items/equipment_slots.rs`) now applies the C++ base factor
(mainhand and ranged 1.0, offhand 0.5) times every active
`SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (79) effect covering
`SPELL_SCHOOL_MASK_NORMAL` and fitting the attack's weapon through
`Player::CheckAttackFitToAuraRequirement` (`Player.cpp:8145-8156`);
`resolved_aura_effects_with_spell_and_misc_like_cpp`
(`session/spell_state/aura.rs`) exposes the owning spell id the fit needs.
`PlayerSpellBonusInputLikeCpp.weapon_damage_pct` (default `[1.0, 0.5, 1.0]`)
flows through `PlayerStatSystemProjectionLikeCpp` into
`effective_weapon_damage_ranges_like_cpp` and is published on
`PlayerEffectiveCombatStatsLikeCpp`. **Deliberate departure, recorded:** the
source multiplies the offhand factor by
`GetTotalAuraModifier(SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT, ...)`, a raw sum that is
0 when no such aura is active and therefore zeroes offhand damage on every
equip/login; RustyCore keeps the evident intent (0.5 base times the physical
multiplier) and does not apply that term, leaving
`SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT` as a separate gate that needs capture
evidence for its scale. Ranged weapon requirements are excluded rather than
resolved because the represented `GetWeaponForAttack` helper covers the melee
slots only. Evidence: the wow-data test pins the projection copy; the session
test equips a sword mainhand and a dagger offhand and asserts `[1.0, 0.5, 1.0]`
with the offhand range at half the mainhand one, `[1.5, 0.75, 1.5]` after an
item-neutral +50% aura and `[3.0, 0.75, 1.5]` after a sword-restricted +100%
aura; wow-data --lib 751/0, wow-entities --lib 940/0 and wow-world --lib
3928/0/1; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest `20260917T042933.856352Z-2316020-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the offhand-damage aura
scale, ranged weapon requirements, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `VersatilityBonus` publication — 2026-09-17, implementation `a065cd97`,
integrated as `3bff1bf7` by PR #1023:** the create block and the narrow values
update wrote `ActivePlayerData::VersatilityBonus` (bit 56) as a constant 0.0.
`AuraEffect::HandleModVersatilityByPct` (`SpellAuraEffects.cpp:3797-3808`) now
supplies it: the sum of every active `SPELL_AURA_MOD_VERSATILITY` (471) amount,
stored through `Player::SetVersatilityBonus` (`Player.h:2753`) with the
`SetUpdateFieldStatValue` zero clamp. The same handler's
`UpdateHealingDonePercentMod` versatility term is commented out in C++ and stays
absent, and `UpdateVersatilityDamageDone` writes no field, so only the bonus is
produced. `represented_spell_bonus_like_cpp` sums the aura amounts,
`PlayerSpellBonusInputLikeCpp` carries `versatility_bonus_aura`,
`PlayerStatSystemProjectionLikeCpp` exposes the clamped `versatility_bonus`, and
`PlayerEffectiveCombatStatsLikeCpp` publishes it; `PlayerCombatStats`,
`PlayerCreateData` and `PlayerStatChanges` carry it so both writers emit the
real value (bit 56, already masked). Evidence: the wow-data test pins the sum,
the zero clamp and the default; the extended session test applies aura 471 with
amount 200 and asserts 200.0; `scenarios_spell_state_22` asserts the 0.0 default;
the `scenarios_3` byte test locates parent-38 field 16 (bit 56) and reads the
value back; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib
3927/0/1 and wow-packet --lib 742/0; format, `git diff --check`, the physical
ratchet (three explained fixture ceilings: `update_tests/mod.rs` 216→218,
`items.rs` 3902→3903 and `world_entry.rs` 2780→2781) and `validation-v2 quick`
(manifest `20260917T041731.698070Z-2298918-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the melee damage percentage consumer,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 override percentage field publication — 2026-09-17, implementation
`204958b6`, integrated as `b75fb666` by PR #1021:** the create block and the
narrow values update wrote `OverrideSpellPowerByAPPercent` and
`OverrideAPBySpellPowerPercent` as an unconditional `-1.0` that has no C++ or
capture source in this tree. C++ accumulates the aura amounts into
zero-initialized fields: `AuraEffect::HandleOverrideSpellPowerByAttackPower`
(`SpellAuraEffects.cpp:3770-3781`) for
`SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` (366) and
`HandleOverrideAttackPowerBySpellPower` (`3783-3796`) for
`SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (404). The projection now exposes
`override_spell_power_by_ap_percent` and `override_ap_by_spell_power_percent`
(0.0 when absent), `PlayerEffectiveCombatStatsLikeCpp` publishes them, and
`PlayerCombatStats`/`PlayerCreateData`/`PlayerStatChanges` carry them so both
writers emit the accumulated values (bits 65-66, already masked). The
attack-power branch keeps the C++ `HasAuraType` presence rule, now distinct from
the published value. Evidence: the wow-data test pins the defaults and the 366
value; the session test asserts 0.0 before any override aura and 50.0 after the
404 aura, and a dedicated level-80 warrior test drives 366 to
`int32(CalculatePct(220, 50) + 0.5) = 110` on both flat spell fields with attack
power unchanged; the `scenarios_3` byte test locates parent-38 fields 25-26
(bits 65-66) and reads both floats; `scenarios_spell_state_22` pins the 0.0
defaults; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib
3927/0/1 and wow-packet --lib 742/0; format, `git diff --check`, the physical
ratchet (three explained fixture ceilings: `update_tests/mod.rs` 212→216,
`items.rs` 3900→3902 and `world_entry.rs` 2778→2780) and `validation-v2 quick`
(manifest `20260917T040804.079374Z-2286951-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the melee damage percentage consumer,
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModTargetResistance` and item spell penetration publication —
2026-09-17, implementation `2715525d`, integrated as `5fd96c75` by PR #1019:**
the create block and the narrow values update wrote both target resistance
fields as constants. `AuraEffect::HandleModTargetResistance`
(`SpellAuraEffects.cpp:3507-3530`) now feeds them: an effect covering
`SPELL_SCHOOL_MASK_NORMAL` adds to `ModTargetPhysicalResistance`, and one
covering the whole `SPELL_SCHOOL_MASK_SPELL` (`SharedDefines.h:340-343`) adds to
`ModTargetResistance`; `Player::ApplySpellPenetrationBonus`
(`StatSystem.cpp:231-235`) subtracts the item/enchant
`ITEM_MOD_SPELL_PENETRATION`. `represented_spell_bonus_like_cpp` sums the two
aura sets and passes the `spell_penetration_bonus` accumulator;
`PlayerSpellBonusInputLikeCpp` carries `target_resistance_aura`,
`item_spell_penetration` and `target_physical_resistance_aura`;
`PlayerStatSystemProjectionLikeCpp` exposes `mod_target_resistance`
(`aura − item`) and `mod_target_physical_resistance`; and
`PlayerEffectiveCombatStatsLikeCpp` publishes both. `PlayerCombatStats`,
`PlayerCreateData` and `PlayerStatChanges` carry the fields so the create block
and the narrow values update write them instead of the hardcoded zeros (bits
67-68, already masked). Boundary: `ModHealingPercent`,
`ModPeriodicHealingDonePercent` and `ModSpellPowerPercent` keep their 1.0 create
value because C++ has no writer for them, and the
`OverrideSpellPowerByAPPercent`/`OverrideAPBySpellPowerPercent` create sentinels
are unchanged and remain a separate contract decision. Evidence: the wow-data
test pins the projection arithmetic; the extended session test applies aura 123
with the magic mask (20) and armor mask (30) plus an item penetration bonus (15)
and asserts 5/30; `scenarios_spell_state_22` asserts the item path leaves both at
zero; the `scenarios_3` byte test locates parent-38 fields 27 and 28 (bits 67-68)
and reads back both values; wow-data --lib 751/0, wow-entities --lib 940/0,
wow-world --lib 3926/0/1 and wow-packet --lib 742/0; format, `git diff --check`,
the physical ratchet (two explained fixture ceilings: `update_tests/mod.rs`
208→212 and `world_entry.rs` 2776→2778) and `validation-v2 quick` (manifest
`20260917T035358.228914Z-2263464-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the override percentage contract, the melee damage percentage
consumer, the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and
`SetPvPDeath`, alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModHealingDonePercent` publication — 2026-09-17, implementation
`cbbb3a49`, integrated as `98a45d75` by PR #1017:** `Player::UpdateHealingDonePercentMod`
(`StatSystem.cpp:588-599`) is now produced: the value starts at 1.0, multiplies
`1 + amount/100` over every active `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136)
effect and is clamped at zero, matching the re-run from
`AuraEffect::HandleModHealingDonePct` (`SpellAuraEffects.cpp:3645-3654`).
`represented_spell_bonus_like_cpp` computes it, `PlayerSpellBonusInputLikeCpp`
carries `healing_done_percent` with a manual `Default` of 1.0,
`PlayerStatSystemProjectionLikeCpp` exposes `mod_healing_done_percent` and
`PlayerEffectiveCombatStatsLikeCpp` publishes it. `PlayerCombatStats` and
`PlayerCreateData` carry `mod_healing_done_pct` so the create block writes the
real multiplier instead of 1.0, and `PlayerStatChanges`'s existing field is fed
from the projection for the narrow values update (bit 61, already emitted).
Boundary: `ModHealingPercent`, `ModPeriodicHealingDonePercent` and
`ModSpellPowerPercent` have no C++ writer and keep their 1.0 create value, while
`ModTargetResistance`/`ModTargetPhysicalResistance` (aura 123 plus the item
spell-penetration term) remain hardcoded and are the next gate. Evidence: the
wow-data test pins the projection copy; the extended session test drives aura 136
twice on a level-80 priest to `(1+0.5)*(1+1.0) = 3.0`; `scenarios_spell_state_22`
pins the 1.0 create value; the `scenarios_3` byte test locates parent-38 field 21
(bit 61) and asserts the emitted multiplier; wow-data --lib 751/0, wow-entities
--lib 940/0, wow-world --lib 3926/0/1 and wow-packet --lib 742/0; format,
`git diff --check`, the physical ratchet (two explained fixture ceilings:
`update_tests/mod.rs` 207→208 and `world_entry.rs` 2775→2776) and
`validation-v2 quick` (manifest `20260917T034615.374132Z-2251853-quick.json`)
pass. No live DB/restart/relogin QA. #61 stays open for the target
resistance/spell-penetration fields, the melee damage percentage consumer, the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 `ModDamageDonePercent` publication — 2026-09-17, implementation
`b11a04cb`, integrated as `c3645a79` by PR #1015:** the last per-school
`ActivePlayerData` spell field is now produced and published.
`AuraEffect::HandleModDamagePercentDone` (`SpellAuraEffects.cpp:4525-4548`) sets
`ModDamageDonePercent[i]` to
`GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, 1 << i)` —
the product of `1 + amount/100` over the effects intersecting that school, with
the 1.0 create value kept when none does. `represented_spell_bonus_like_cpp`
computes that array, `PlayerSpellBonusInputLikeCpp` carries it with a manual
`Default` that starts at 1.0, and `PlayerEffectiveCombatStatsLikeCpp` publishes
`mod_damage_done_percent`. `PlayerCombatStats`, `PlayerCreateData` and
`PlayerStatChanges` carry it so the create block and the narrow values update
emit the real multiplier; the narrow writer sets the `ModDamageDonePercent` bits
(291-297 → block 9 bits 3-9) and writes the value after `ModDamageDoneNeg`,
completing the `SpellCrit/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct` interleave.
Boundary: the physical-school `Unit::UpdateAllDamagePctDoneMods()`
(`Unit.cpp:9074-9078`) melee damage percentage remains unrepresented and open.
Evidence: the wow-data test pins the projection copy; the extended session test
drives aura 79 on a level-80 priest to holy `(1+0.5)*(1+1.0) = 3.0`, fire 1.25
and 1.0 elsewhere; `scenarios_spell_state_22` pins the 1.0 create value; the
`scenarios_3` byte test pins block 9 `0x000003FF` and every school's percent
value; wow-data --lib 751/0, wow-entities --lib 940/0, wow-world --lib 3926/0/1
and wow-packet --lib 742/0; format, `git diff --check`, the physical ratchet (two
explained fixture ceilings: `update_tests/mod.rs` 205→207 and `world_entry.rs`
2774→2775) and `validation-v2 quick` (manifest
`20260917T033705.720788Z-2239874-quick.json`) pass. No live DB/restart/relogin QA.
#61 stays open for the melee
damage percentage consumer, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 narrow values-update negative spell field — 2026-09-17, implementation
`a7c53527`, integrated as `a584faf0` by PR #1013:** the boundary recorded by the
wire-publication entry above is closed. `PlayerStatChanges` gains
`mod_damage_done_neg: [i32; 7]`, the last C++ `ActivePlayerData` spell field the
narrow path was missing; the equipment projection (`items.rs`) and the shared
`session/mod.rs` fixture populate it. The narrow writer (`state_5.rs`) now sets
the `ModDamageDoneNeg` mask bits (284-290 → block 8 bits 28-31 and block 9 bits
0-2) and emits each school's negative value after the positive one, matching the
`SpellCrit/ModDmgDonePos/ModDmgDoneNeg/ModDmgDonePct` interleave from
`Player::UpdateSpellDamageAndHealingBonus` (`StatSystem.cpp:171-197`).
`ModDamageDonePercent` (291-297) stays outside this runtime writer and remains
open. Evidence: the `scenarios_3` byte test pins group 0 with block 9, block 8
`0xFFFFE000`, block 9 `0x00000007` and reads back every school's negative value;
`scenarios_spell_state_22` asserts the item path leaves the negative array zero;
wow-packet --lib 742/0 and wow-world --lib 3926/0/1; format, `git diff --check`,
the physical ratchet (one explained fixture ceiling: `update_tests/mod.rs`
204→205) and `validation-v2 quick` (manifest
`20260917T032401.703353Z-2226427-quick.json`) pass. No live DB/restart/relogin QA.
#61 stayed open for `ModDamageDonePercent`, delivered by the entry above, and
remains open for the melee damage percentage consumer, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 spell damage and healing field wire publication — 2026-09-17,
implementation `9847962e`, integrated as `2307fb68` by PR #1011:** the packet
adapters no longer send the item `spell_power` scalar to every magic school and
to healing. `PlayerCombatStats`, `PlayerCreateData` and `PlayerStatChanges`
replace `spell_power` with the C++ field set — `mod_damage_done_pos[7]`,
`mod_damage_done_neg[7]` (create data) and `mod_healing_done_pos` — so each
struct has one authority per value. The create block (`state_2.rs`,
`StatSystem.cpp:171-197`) writes the per-school positive and negative entries plus
the healing field from those arrays, and the narrow values update
(`state_5.rs`) writes `ModDamageDonePos[i]` and `ModHealingDonePos` from the same
arrays; the wire layout is unchanged because the scalar occupied exactly those
slots. The login `PlayerCombatStats` snapshot (`world_entry.rs`) and the equip
`PlayerStatChanges` (`items.rs`) copy the projection fields, and the
`session/mod.rs` fixture helper states that it has no aura/stat producers.
Boundary: the narrow values-update path still leaves the `ModDamageDoneNeg` bits
(284-290) unset, exactly as before this unit; the create block publishes the
negative entries and the generic `state_4.rs` writer already supports them once
their mask bits are set. Evidence: `scenarios_spell_state_22` asserts the item
spell power reaches `mod_healing_done_pos` and every magic school; wow-packet
--lib 742/0 and wow-world --lib 3926/0/1; format, `git diff --check`, the
physical ratchet (two explained fixture ceilings: `update_tests/mod.rs` 201→204
and `world_entry.rs` 2772→2774) and `validation-v2 quick` (manifest
`20260917T031859.286082Z-2219004-quick.json`) pass. No live DB/restart/relogin QA.
#61 stayed open for the narrow values-update negative bits, delivered by the
narrow values-update entry above, and remains open for
`ModDamageDonePercent`, the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers,
rune regeneration and live DB/restart/relogin QA.

**#61 spell damage and healing done producers — 2026-09-17, implementation
`92f0f550`, integrated as `bd449980` by PR #1009:** `Player::UpdateSpellDamageAndHealingBonus`
(`StatSystem.cpp:171-197`) is now produced. `PlayerSpellBonusInputLikeCpp` carries
the C++ terms: `GetBaseSpellPowerBonus()`, the `SPELL_AURA_MOD_DAMAGE_DONE` (13)
per-school net sum and its negative `ModDamageDoneNeg` part,
`SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` (174, school mask plus stat) and
`SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` (175, stat),
`SPELL_AURA_MOD_HEALING_DONE` (135), and
`ActivePlayerData::OverrideSpellPowerByAPPercent` (366).
`calculate_player_stat_system_like_cpp` computes the fields in C++
`UpdateAllStats` order — spell fields first, then `UpdateAttackPowerAndDamage`,
then the 366 pass that replaces both bonuses with
`int32(CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct) + 0.5f)` — so the
404 branch now derives `min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX])` from
the real fields instead of the item scalar, and `SetUpdateFieldStatValue` clamps
both published fields at zero. The healing path adds `max(0, Intellect)` when the
class base-mana row is nonzero (`GetPowerIndex(POWER_MANA) != MAX_POWERS`), and
`Player::ApplySpellPowerBonus`'s early return while 366 is present is represented
by zeroing `GetBaseSpellPowerBonus()`. `PlayerEffectiveCombatStatsLikeCpp`
publishes `mod_damage_done_pos`, `mod_damage_done_neg` and
`mod_healing_done_pos`. Boundary: the create-block and values-update adapters
published only the scalar `spell_power` at that point; the per-school arrays
are published by the spell-field wire entry above. Evidence: a pure test pins the per-school
positive/negative split, the mana intellect term and the 366 override, and a
session test drives 13/135/174/175 on a level-80 priest to holy 150, fire 120
with `ModDamageDoneNeg[fire] = -50` and healing 187, then 404 to both attack mods
50; `wow-data --lib` 751/0, `wow-entities --lib` 940/0, `wow-world --lib`
3926/0/1, format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T030821.891532Z-2209220-quick.json`) pass. No live
DB/restart/relogin QA. #61 stayed open for the packet publication of those fields,
delivered by the spell-field wire entry above, and remains open for the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 override attack power by spell power — 2026-09-17, implementation
`8ee397c5`, integrated as `12efaf8f` by PR #1007:** the last unrepresented branch
of `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:333-403`) is applied:
while `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (`SpellAuraDefines.h:499`) is
active, C++ skips the strength/agility/level base and sets `BASE_VALUE` for both
the melee and the ranged unit mod to `CalculatePct(float(minSpellPower),
OverrideAPBySpellPowerPercent)` (`StatSystem.cpp:341-379`), where `minSpellPower`
is `min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX])`.
`PlayerStatSystemInputLikeCpp` now carries
`attack_power_override_by_spell_power: Option<(i32, f32)>`; the pure stat system
replaces both bases when it is `Some`, preserving the `int32(base_attPower)`
truncation, and otherwise keeps the previous arithmetic. The producer sums every
active effect amount, mirroring the `ApplyModUpdateFieldValue` accumulator in
`AuraEffect::HandleOverrideAttackPowerBySpellPower`
(`SpellAuraEffects.cpp:3785-3796`); C++ tests `HasAuraType`, so an active effect
whose summed percent is zero still overrides. The minimum spell power is the
represented item spell power that `Player::ApplySpellPowerBonus`
(`StatSystem.cpp:153-168`) publishes to `ModHealingDonePos` and the magic
`ModDamageDonePos` entries; the `SPELL_AURA_MOD_DAMAGE_DONE`/`MOD_HEALING_DONE`
producers that would widen those fields are implemented by the spell
damage/healing done entry above. The override
reaches the existing consumers unchanged: `Unit::GetTotalAttackPowerValue`
(threat) and `effective_weapon_damage_ranges_like_cpp`
(`Unit::CalculateMinMaxDamage`). Evidence: a pure stat-system test pins the
replacement, truncation and zero-percent presence rule, an end-to-end session
test moves the baseline 220/-10 to 200/200 with 1,000 spell power and two
stacking 15+5 percent effects; `wow-data --lib` 750/0, `wow-world --lib`
3925/0/1, format, `git diff --check`, the physical ratchet and `validation-v2
quick` (manifest `20260917T025325.633908Z-2197184-quick.json`) pass. No live
DB/restart/relogin QA. #61 stays open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration, the packet publication of the spell fields and live
DB/restart/relogin QA.

**Seven stale `wow-world --lib` expectations — 2026-09-17, implementation
`2534b97e`, integrated as `343be766` by PR #1005 (test harness, not gameplay
progress):** the seven scenarios left red by PR #1003 were each reproduced on
clean base `6b46be23` before correction; none was a production regression and no
production code changed. (1) `login_load_adds_default_void_item_appearance_like_cpp`
installed a canonical Player with no learned proficiencies and then asserted
`CollectionMgr::CanAddAppearance` (`CollectionMgr.cpp:649-726`), which requires
`Player::GetWeaponProficiency() & (1 << subclass)`; a real character learns its
class proficiency spells on first login (`CharacterHandler.cpp:1284-1288` casting
`playercreateinfo_cast_spell`), whose `SPELL_EFFECT_PROFICIENCY` runs
`Player::AddWeaponProficiency` (`SpellEffects.cpp:1785-1804`, `Player.h:1433`), so
the fixture now seeds the one-handed-sword mask on the Player it installs.
(2) `repeated_login_attachment_preserves_loaded_reputation_for_final_save`
installed the canonical Player without the loaded identity, so the second
`send_login_sequence` ensure read a race change and reinitialized the manager;
the fixture now publishes the loaded race/class/level first, matching
`Player::LoadFromDB` (`CharacterHandler.cpp:1070`). (3) The logout save-snapshot
scenario still expected the stale session level 10 while `Player::SaveToDB` reads
canonical `Unit::GetLevel` (the snapshot itself already expected 42).
(4) The durable creature rail drain now starts with the `Unit::Kill`
creature-killer durability loss (`Unit.cpp:10639-10648`) before the victim health
presentation, and the fall-land lethal scenario adds
`Player::EnvironmentalDamage`'s durability loss (`Player.cpp:667-669`). (5) The
two `scenarios_world_entities_16` quest-giver queries set the level before
canonical adoption, which overwrote it. Evidence: `wow-world --lib` moves from
3917 passed/7 failed to **3924 passed/0 failed/1 ignored**; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T024356.370256Z-2188249-quick.json`) pass. This restores the suite as
usable evidence; it does not add gameplay parity and no live DB/restart/relogin
QA.

**Represented quest-share party fixture identity — 2026-09-17, implementation
`05d0d5e4`, integrated as `1bc1f9c3` by PR #1003 (test harness, not gameplay progress):** `install_represented_party`
registered the receiver and only then adopted the synthetic canonical Player,
whose identity had never been set. Since the registry movement publication
snapshots `player_level_like_cpp()` (the canonical owner's level, C++
`Unit::GetLevel`), every later `sync_player_registry_state_like_cpp()` overwrote
the registry level with 0 and the represented `SatisfyQuestMinLevel` gate rejected
the receiver with `QUEST_PUSH_REASON_LOW_LEVEL` (22) before the intended
prerequisite, reputation, level or expansion branches ran; 21
`push_quest_to_party` scenarios were red for that artificial reason. The fixture
now re-applies the loaded identity after adoption, matching
`Player::LoadFromDB`, which builds the canonical Player with its identity before
the session registers; production behavior is unchanged. Evidence:
`wow-world --lib` moves from 3896 passed/28 failed to **3917 passed/7 failed**,
`push_quest_to_party` 49/49 green; format, `git diff --check`, the physical
ratchet and `validation-v2 quick` (manifest
`20260917T022350.040983Z-2176206-quick.json`, 37.9 s) pass. The 7 remaining
`wow-world --lib` failures — `reputation_retention`,
`movement_fall_land_lethal`, `void_storage` login appearance,
`scenarios_persistence_2` logout snapshot, two `scenarios_world_entities_16`
quest-giver queries and `scenarios_world_entities_1` durable creature rail —
each need their own reproduction and C++ contrast and remained an open defect track,
resolved by the seven-stale-expectations entry above. No live DB/restart/relogin QA.

**Save-snapshot manager-lock re-entry — 2026-09-17, implementation `f55d9ef3`, integrated as `ac592c5f` by PR #1001:**
`fixture_player_save_to_db_snapshot_like_cpp` held the canonical map-manager lock
for its `do_for_all_maps` scan and resolved `self.player_level_like_cpp()` inside
that closure; the accessor re-enters the same non-reentrant lock, so both
`save_snapshot_owner` scenarios hung forever (they also hung on the clean base
`b3f36c1b`). The level is now resolved before the lock, with the snapshot fields
unchanged and no production path affected (`current_player_save_to_db_snapshot_like_cpp`'s
header helper only reads the borrowed Player). This was the last known hang:
**`wow-world --lib` now runs to completion with no skips — 3896 passed, 28 failed,
1 ignored, 0 filtered.** The 28 failures are pre-existing (their filters fail
identically on the clean base, re-checked here): 21 `handlers::quest::tests`
`push_quest_to_party` fan-out cases, two `scenarios_world_entities_16`
quest-giver queries, `reputation_retention`, `movement_fall_land_lethal`,
`void_storage` login appearance, `scenarios_persistence_2` logout snapshot and
`scenarios_world_entities_1` durable creature rail; they form a separate defect
track and are not claimed fixed. Focused: `save_snapshot_owner` (5). Format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260917T021722.275999Z-2169415-quick.json`, 38.3 s) pass. No live
DB/restart/relogin QA.

**Session reputation-closure lock re-entry deadlock — 2026-09-17, implementation
`299fe975`, integrated as `3c3f2ce4` by PR #999:** six call sites ran `self.player_race_like_cpp()`/
`self.player_class_like_cpp()` (and the friendship store) inside
`with_reputation_mgr_like_cpp`/`mutate_reputation_mgr_like_cpp`, whose closure
runs while the canonical map-manager `Mutex` is held; those session accessors take
the same non-reentrant lock, so any session with a canonical Player deadlocked.
The identity is now resolved before the closure in
`session/progression/reputation.rs`, `session/quest/objectives.rs`,
`session/world_entities/creature_kill.rs`, `handlers/quest/eligibility.rs` and
the new item-admission helper, with the same values passed to the same manager
calls and no other behavior change. This removes the reputation-family hangs: the
previously stuck
`scenarios_player_items_8::repair_item_handler_requires_repair_npc_and_repairs_single_item_like_cpp`
(1) and the `scenarios_progression` reputation-objective tests (13) now pass, and
`wow-world --lib` (skipping two unrelated pre-existing `save_snapshot_owner`
hangs) completes with 3894 passed and 28 failed whose filters fail identically on
the clean base `b3f36c1b`, so no regression is introduced. The same unit
completes two `CollectionMgr::CanAddAppearance` gates that C++
`Player::CanUseItem(ItemTemplate const*)` applies: the `483`/`55884`
learning-effect pair and the required-faction reputation rank, sharing the new
`represented_item_reputation_rank_like_cpp` and
`represented_item_effect_spell_ids_like_cpp` helpers with the login inventory
admission. Focused: `can_add_item_appearance` (5). Format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T021045.633991Z-2163059-quick.json`, 62.2 s) pass. The two
`save_snapshot_owner` tests still hang on both the clean base and this head and
need their own investigation; the 28 pre-existing `wow-world --lib` failures
(quest party fan-out, fall-lethal values, void-storage appearance, logout
snapshot, quest-giver query, durable creature rail) are unchanged by this unit and
remain a separate defect track. No live DB/restart/relogin QA.

**Collection appearance `CanUseItem` template gates — 2026-09-17, implementation
`a2c8c3bb`, integrated as `a8ba2c3c` by PR #997:** `can_add_item_appearance_represented_like_cpp` now runs the
`Player::CanUseItem(ItemTemplate const*)` template admission
(`Player.cpp:11069-11125`) that C++ `CollectionMgr::CanAddAppearance` applies
before its own branches: the `ITEM_FLAG2_INTERNAL_ITEM` and Faction
Horde/Alliance flags, the allowable race mask, the required level, the required
skill and rank against the canonical skill value, and the required ability
against the known spells. The transmog-specific gates (source type,
`NoSourceForItemVisual`, quality/artifact, item class/subclass/inventory, learned
weapon proficiency and duplicate) are unchanged. The holiday, reputation, the
483/55884 learning-effect pair and the artifact specialization gates remain
separate slices, documented in place. Focused coverage: a new scenario matrix
asserts a plain usable weapon passes while the internal flag, an opposite-faction
item, an above-level item, an unknown required ability and a race-restricted item
are rejected, and a known required ability passes; `scenarios_player_items_5`
(15), `scenarios_player_items_6` (13), `transmog` (15), `collection` (27),
`heirloom` (9), `scenarios_player_items_1` (53) and `persistence::` (105) stay
green; format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T012737.474242Z-2142222-quick.json`, 39.5 s) pass. This is the
collection/transmog vertical, not the F1 equipment-stat vertical of #61. The full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. No live DB/restart/relogin QA.

**Collection appearance weapon-proficiency gate — 2026-09-17, implementation
`4ba42678`, integrated as `978d8ee9` by PR #995:** the represented `CollectionMgr::CanAddAppearance`
(`CollectionMgr.cpp:649-726`) now reads the learned `Player::GetWeaponProficiency`
mask (`Player.h:1432`) instead of the class default
`SetProficiency::default_weapons`, matching the C++ `ITEM_CLASS_WEAPON` branch and
the `!item || !GetPlayer()` guards. `Player::weapon_proficiency_like_cpp` and
`represented_player_weapon_proficiency_like_cpp` are the new read path; the
collection scenarios install the canonical Player that `_owner->GetPlayer()`
requires, seed the mask through the new `grant_learned_weapon_proficiency_like_cpp`
fixture, and drive the already-collected case through the canonical collection
state. A new scenario proves the mask decides admission: a warrior with only Mace
learned collects a mace appearance and is rejected for a sword even though the
warrior class default includes swords. The physical policy records the reviewed
`session_tests.rs` fixture delta (5762→5766). Focused coverage:
`scenarios_player_items_5` (14), `scenarios_player_items_6` (13), `transmog`
(15), `collection` (27) and `heirloom` (9) stay green; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T011753.474734Z-2137458-quick.json`, 69.7 s) pass. This is the
collection/transmog vertical, not the F1 equipment-stat vertical of #61; the
armor-proficiency half of `CanUseItem`, the broader `CanUseItem` class/race/level
gates and the `ItemSpecStats` fallback remain separate gates. The full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. No live DB/restart/relogin QA.

**#61 attack power aura producers — 2026-09-17, implementation `408725a5`, integrated as `9c7d8ff8` by PR #993:** the
attack power route now runs C++ `Player::UpdateAttackPowerAndDamage`
(`StatSystem.cpp:333-403`): `SPELL_AURA_MOD_ATTACK_POWER` (99) and
`SPELL_AURA_MOD_RANGED_ATTACK_POWER` (124) supply the flat `TOTAL_VALUE`
published as `AttackPowerModPos`/`RangedAttackPowerModPos`,
`SPELL_AURA_MOD_ATTACK_POWER_PCT` (166) and
`SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT` (167) supply `TOTAL_PCT` published as
the `TOTAL_PCT - 1.0` multiplier, and the ranged producers keep the
`CLASSMASK_WAND_USERS` skip (`SharedDefines.h:190`). `Unit::GetTotalAttackPowerValue`
now applies its zero clamp before the multiplier, replacing the previous
unclamped `total_attack_power` (the damage-range consumer therefore uses the
aura-aware total). The projection returns both multipliers, the canonical
effective stats and `PlayerStatChanges` carry them instead of a hardcoded zero,
and the create block writes `AttackPowerMultiplier`/`RangedAttackPowerMultiplier`
from `PlayerCombatStats`/`PlayerCreateData`. The two new aura constants carry
their `SpellAuraDefines.h` anchors and the physical policy records the reviewed
one-line `world_entry.rs` (2771→2772) and two-line fixture
(`update_tests/mod.rs` 199→201) deltas. Focused coverage: a pure stat-system test
pins the flat/percentage/multiplier arithmetic and the clamp, an end-to-end
session test asserts the melee and ranged producers plus
`GetTotalAttackPowerValue` (405), a second asserts the wand-user skip, and a
packet test pins the create-block multipliers; `wow-data stat_system` (8),
`wow-packet --lib` (742), `scenarios_player_items_12` (12),
`scenarios_player_items_1` (53), `persistence::` (105),
`handlers::character::tests::login` (11), `worldport` (9),
`scenarios_world_entities_24` (11) and `scenarios_spell_state_22` (9) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T010221.921007Z-2129573-quick.json`, 76.3 s) pass.
`SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` is implemented by the
override-attack-power entry above (it has no `SPELL_AURA_MOD_ATTACK_POWER_OF_ARMOR`
counterpart in this 3.4.3 source), and the same-effect stack-rule question
remains, the `GetTotalAuraMultiplier` same-effect stack-rule
grouping is not applied (consistent with the older aura helpers), the full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 school resistances (`Unit::UpdateResistances`) — 2026-09-17, implementation
`67d23891`, integrated as `0300850f` by PR #991:** the six magic school resistances are now represented and published.
`represented_school_resistances_like_cpp` resolves each school as the item
`BASE_VALUE` (`gear.resistances[school]`) scaled by the
`SPELL_AURA_MOD_BASE_RESISTANCE_PCT` `BASE_PCT`, plus the flat
`SPELL_AURA_MOD_RESISTANCE`/`MOD_BASE_RESISTANCE` `TOTAL_VALUE`, then the
`SPELL_AURA_MOD_RESISTANCE_PCT` `TOTAL_PCT`, with the C++ `int32(value)`
truncation (`Unit.cpp:9148-9163`); the mask-parameterized aura helpers are shared
with the `Player::UpdateArmor` physical route from PR #985. The canonical
effective stats now carry the aura-aware schools and the login create path
threads them into `PlayerCombatStats`/`PlayerCreateData.school_resistances[6]`,
so `PlayerCreateData::write_create` writes the seven `UnitData::Resistances`
values that were previously a hardcoded zero. The packet layout is unchanged and
no new mutable state, mirror, lock or clock appears; the physical policy records
the reviewed one-line fixture delta (`update_tests/mod.rs` 198→199). Focused
coverage: a packet test pins the seven create values in order and an end-to-end
session test asserts holy/fire item+aura resistances, the per-school percentage
isolation and the removal path; `wow-packet --lib` (741),
`scenarios_player_items_12` (10), `scenarios_player_items_1` (51),
`persistence::` (105), `handlers::character::tests::login` (11), `worldport` (9)
and `scenarios_world_entities_24` (11) stay green; format, `git diff --check`,
the physical ratchet and `validation-v2 quick` (manifest
`20260917T005300.823629Z-2123777-quick.json`, 69.5 s) pass. The
`BASE_PCT_EXCLUDE_CREATE` modifier has no represented producer and is implicitly
1.0 (C++ default 100.0), post-login resistance *deltas* remain unrepresented
because no resistance delta writer exists yet, the `GetTotalAuraMultiplier`
same-effect stack-rule grouping is not applied (consistent with the older aura
helpers), the full `wow-world --lib` suite stays unusable on this host because
several unrelated pre-existing async tests hang, and `validation-v2 final` stops
at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and was
not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration, class-talent bonuses and live DB/restart/relogin QA.

**#61 critical-strike aura percentages — 2026-09-17, implementation
`7412dac5`, integrated as `81ee68b9` by PR #989:** the represented critical-strike projection now consumes the C++
aura producers. `Player::UpdateWeaponDependentCritAuras`
(`Player.cpp:8079-8107`) resolves `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` with the
`CheckAttackFitToAuraRequirement` filter (`Player.cpp:8145-8156`, reusing the
represented weapon admitted by `IsItemFitToSpellRequirements`) plus the
unfiltered `SPELL_AURA_MOD_CRIT_PCT` (290) sum into
`PlayerStatSystemInputLikeCpp.crit_mainhand_aura_pct`/`crit_offhand_aura_pct`/
`crit_ranged_aura_pct`; `Player::UpdateCritPercentage`
(`StatSystem.cpp:502-538`) then applies `5% + FLAT_MOD + rating` per group, so
the offhand no longer shares the mainhand value and the per-attack weapon
requirement can select different auras. `Player::UpdateSpellCritChance`
(`StatSystem.cpp:718-731`) adds `SPELL_AURA_MOD_SPELL_CRIT_CHANCE` (57) plus
`MOD_CRIT_PCT` before the spell rating for all seven schools. The new
`represented_weapon_crit_aura_modifier_like_cpp` reuses the expertise unit's
weapon/item-fit admission, so no second ownership path appears; the four
published critical fields already flow through the create block and the
post-login stat update, so no packet, mirror, lock or clock change is needed.
The `SPELL_AURA_MOD_CRIT_PCT` constant carries its `SpellAuraDefines.h` anchor.
Focused coverage: a pure stat-system test pins the per-group base plus aura sums,
and an end-to-end session test asserts that the sword-only aura is rejected
unarmed, applies to the mainhand after equipping the sword, and never leaks to
the offhand/ranged or spell groups; `wow-data stat_system` (7),
`scenarios_player_items_12` (9), `scenarios_player_items_1` (50),
`persistence::` (105), `handlers::character::tests::login` (11),
`scenarios_spell_state_22` (9) and `scenarios_world_entities_24` (11) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260917T004453.285261Z-2117991-quick.json`, 63.9 s) pass. The
`GetTotalAuraModifier` same-effect stack-rule grouping is not applied (consistent
with the older aura modifier helpers), class-talent critical bonuses and the
school (1-6) resistance publication remain separate gates, the full
`wow-world --lib` suite stays unusable on this host because several unrelated
pre-existing async tests hang, and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 avoidance aura percentages — 2026-09-17, implementation `255c888f`, integrated as `25d6c0b9` by PR #987:** the
represented avoidance projection now consumes the C++ aura producers of
`Player::UpdateBlockPercentage` (`StatSystem.cpp:483-499`),
`Player::UpdateParryPercentage` (`659-679`) and `Player::UpdateDodgePercentage`
(`700-717`): the flat `GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT)` /
`MOD_PARRY_PERCENT` / `MOD_DODGE_PERCENT` sums feed
`PlayerStatSystemInputLikeCpp.spell_block_pct`/`spell_parry_pct`/
`spell_dodge_pct`, added after the 5% base and on the non-diminishing side before
the class diminishing-returns formula, with the class parry caps honoured (a
zero cap keeps parry at zero). The session resolves them from the canonical
visible applications through the shared `GetTotalAuraModifier` helper;
`block_pct`/`dodge_pct`/`parry_pct` were already published through the create
block and the post-login stat update, so no packet, mirror, lock or clock change
is needed. The two new aura type constants carry their `SpellAuraDefines.h`
anchors. Focused coverage: a pure stat-system test pins the aura terms plus the
zero-cap class branch, and an end-to-end session test applies the three avoidance
auras and the removal path; `wow-data stat_system` (6),
`scenarios_player_items_12` (8), `scenarios_player_items_1` (49),
`persistence::` (105), `handlers::character::tests::login` (11) and
`scenarios_spell_state_22` (9) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T003635.249782Z-2114404-quick.json`, 84.2 s) pass. The
`GetTotalAuraModifier` same-effect stack-rule grouping is not applied (consistent
with the older aura modifier helpers), melee/ranged/spell crit aura producers,
the school (1-6) resistance publication and `GetDodgeFromAgility` (empty in this
3.4.3 fork) remain separate gates, the full `wow-world --lib` suite stays
unusable on this host because several unrelated pre-existing async tests hang,
and `validation-v2 final` stops at the pre-existing runtime hotspot LOC ratchet,
which keeps its drift and was not regenerated. #61 remains open for the
player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
alternate powers, rune regeneration and live DB/restart/relogin QA.

**#61 armor aura producers (`Player::UpdateArmor`) — 2026-09-17, implementation
`6b7334a0`, integrated as `487cd1d3` by PR #985:** the pure stat system now applies every C++ `Player::UpdateArmor`
producer (`StatSystem.cpp:251-276`): `SPELL_AURA_MOD_BASE_RESISTANCE_PCT` (142)
supplies the `BASE_PCT` factor scaling the item `BASE_VALUE`,
`SPELL_AURA_MOD_RESISTANCE` (22) plus `SPELL_AURA_MOD_BASE_RESISTANCE` (83) with
the normal school mask supply the flat `TOTAL_VALUE`,
`SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` (182) adds `CalculatePct` of its
`MiscValueB` stat before the multipliers, `SPELL_AURA_MOD_RESISTANCE_PCT` (101)
supplies `TOTAL_PCT` and `SPELL_AURA_MOD_BONUS_ARMOR_PCT` (466) the final
multiplier, with the C++ `int32(value)` truncation. The session resolves those
inputs from the canonical visible applications through the new
`resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp` (which keeps
`MiscValueB` available) and feeds the existing
`PlayerStatSystemInputLikeCpp`; `armor` is already published through the create
block and the post-login stat update, so no packet or mirror change is needed and
no second writer appears. The five aura type constants carry their
`SpellAuraDefines.h` anchors. Focused coverage: a pure stat-system test pins the
producer order (`((100*1.5)+24+40+6)*1.25*1.1` → 302) and an end-to-end session
test applies the normal-mask flat aura, a fire-mask aura, the percentage pair and
the of-stat aura, asserting each step plus the removal path; `wow-data
stat_system` (5), `scenarios_player_items_12` (7), `scenarios_player_items_1`
(48), `persistence::` (105), `scenarios_spell_state_22` (9) and
`scenarios_world_entities_24` (11) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T002634.532691Z-2110432-quick.json`, 113.8 s) pass. The same-effect
stack-rule grouping of `GetTotalAuraMultiplier` is not applied to the
`MOD_BONUS_ARMOR_PCT` product (consistent with the older mana/stat aura
multiplier helpers), the school (1-6) resistance `BASE_VALUE`/`TOTAL_VALUE`
publication remains a separate gate because no resistance delta writer exists
yet, and the full `wow-world --lib` suite stays unusable on this host because
several unrelated pre-existing async tests hang; `validation-v2 final` stops at
the pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 `Unit::m_transformSpell` and `IsPolymorphed` — 2026-09-17, implementation
`e8994e95`, integrated as `72c1fd9b` by PR #983:** the canonical Unit aura subsystem now owns C++
`Unit::m_transformSpell` (`AuraSubsystem::transform_spell_like_cpp`) with the two
writers from `AuraEffect::HandleAuraTransform`: the apply rule overwrites when
there is no current transform spell info, when the new spell is not positive, or
when the current transform spell is positive (`SpellAuraEffects.cpp:1944-1951`),
and the remove rule clears only the aura that owns the current transform
(`SpellAuraEffects.cpp:2129-2131`). The session aura insert/remove funnels are the
only production writers and the character-identity bulk clear drops a stale
transform. `Unit::IsPolymorphed` (`Unit.cpp:9993-10004`) is now resolved from the
MAGE `SpellClassOptions` family (`SPELLFAMILY_MAGE`, family flag `0x1000000`)
plus effect 0 applying `SPELL_AURA_MOD_CONFUSE` (`SpellInfo.cpp:2665-2671`), which
replaces the hardcoded `false` in the C++ `Player::RegenerateHealth` gate
(`Player.cpp:1857-1859`); `Unit::IsInDisallowedMountForm`
(`Unit.cpp:8813-8820`) reads the canonical transform spell instead of scanning
visible auras. `SpellClassOptionsStore`, already loaded at startup, is attached
to the existing `SessionSpellCatalogCapabilitiesLikeCpp` bundle, so no new
mutable state, lock or clock appears; the physical policy records the reviewed
one-line `app.rs` composition delta (5657→5658). Focused coverage: a new
wow-entities precedence test for the transform write rules and an end-to-end
regeneration test where a represented Polymorph (118) bypasses the in-combat gate
with `GetMaxHealth() / 3.0` and removal restores suppression; the mount suite
(14), `scenarios_spell_state_11` (12), `persistence::` (105) and
`wow-entities unit_subsystems` (60) stay green; format, `git diff --check`, the
physical ratchet and `validation-v2 quick` (manifest
`20260917T000637.522552Z-2103079-quick.json`, 495.0 s, of which 485.2 s is the
full workspace rebuild) pass. Handle-less test fixtures do not persist the
derived transform field; acceptance cases install a canonical Player owner, as
production does. The full `wow-world --lib` suite remains unusable on this host
because several unrelated pre-existing async tests hang, and `validation-v2 final`
stops at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and
was not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and live DB/restart/relogin QA.

**#61 aura-backed per-attack expertise (`Player::UpdateExpertise`) — 2026-09-16,
implementation `a3345bc9`, integrated as `a9a2f2aa` by PR #980:** the character stat projection now derives
`MainhandExpertise`/`OffhandExpertise` like C++
`Player::UpdateExpertise` (`StatSystem.cpp:759-786`): the combat-rating bonus is
truncated to `int32`, each attack adds the `SPELL_AURA_MOD_EXPERTISE` sum whose
spell is fit for that attack's weapon, and the result is clamped at zero.
`represented_expertise_aura_modifier_like_cpp` mirrors
`GetTotalAuraModifier(SPELL_AURA_MOD_EXPERTISE, predicate)` including the
`SPELL_GROUP_STACK_RULE_EXCLUSIVE_SAME_EFFECT` grouping, resolves the weapon
through `Player::GetWeaponForAttack(attack, true)` (`Player.cpp:9243-9270`,
equipped and not broken) and filters each aura through
`SpellInfo::IsItemFitToSpellRequirements` (`SpellInfo.cpp:1757-1768`) against the
represented `SpellEquippedItems` row, so the two weapons can select different
auras. Two adjacent C++ corrections land with the same owner: the rating bonus
is truncated like `int32(GetRatingBonusValue(CR_EXPERTISE))`, and
`ActivePlayerData::RangedExpertise`/`CombatRatingExpertise` keep their zero
create value because C++ never writes them (`UpdateFields.cpp:2889-2893`,
`UpdateFields.h:655-658`). A new
`resolved_aura_effect_amounts_by_spell_like_cpp` exposes the owning spell id
next to each aura amount for the filter predicate. No new `WorldSession` field,
lock or clock. Focused coverage: a new end-to-end test equips a mainhand axe and
applies item-neutral, axe-fit, sword-only and armor-only expertise auras,
asserting the per-attack filtering and the removal path; the existing
equipment-projection test asserts the zero `RangedExpertise`/
`CombatRatingExpertise` create values; `wow-world --lib persistence::` (104),
`scenarios_player_items_1` (47) and `scenarios_player_items_12` (6) stay green;
format, `git diff --check`, the physical ratchet and `validation-v2 quick`
(manifest `20260916T233915.554585Z-2091574-quick.json`, 57.6 s) pass. The full
`wow-world --lib` suite is not usable on this host because several unrelated
pre-existing async tests hang (for example
`scenarios_player_items_8::repair_item_handler_requires_repair_npc_and_repairs_single_item_like_cpp`,
reproduced on unmodified `3.4.3`), and `validation-v2 final` stops at the
pre-existing runtime hotspot LOC ratchet, which keeps its drift and was not
regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, alternate powers, rune
regeneration and `IsPolymorphed`/`m_transformSpell` (blocked on representing
the spell-family flags that classify `SPELL_SPECIFIC_MAGE_POLYMORPH`), plus live
DB/restart/relogin QA.

**#61 food/drink regeneration emote visual — 2026-09-16, implementation
`73c67a9a`, integrated as `c2608254` by PR #978:** the regeneration tick now completes the tail of C++
`Player::RegenerateAll` (`Player.cpp:1609-1678`). `m_foodEmoteTimerCount`
accumulates `m_regenTimer` beside the two-second health window and stays
independent from it — it is never reset when the aura applies, matching the C++
comment and sniff behaviour. Crossing 5000ms publishes `SPELL_VISUAL_KIT_FOOD`
(406, `SharedDefines.h:397`) when an active `SPELL_AURA_MOD_REGEN` spell carries
`SpellAuraInterruptFlags::Standing`, otherwise `SPELL_VISUAL_KIT_DRINK` (438,
`SharedDefines.h:398`) for a Standing `SPELL_AURA_MOD_POWER_REGEN`, then
subtracts one five-second window; food wins over drink when both apply. The
accumulator lives on the canonical `UnitPowerRegenStateLikeCpp`, the session tick
keeps the single writer, and the kit selection reads the canonical visible
applications plus the difficulty-resolved interrupt word used by the StandState
interrupt path. The packet is `Unit::SendPlaySpellVisualKit`
(`Unit.cpp:11566-11574`) with `KitType = 0`/`Duration = 0`, delivered as one realm
copy for the owner plus the existing position-based visibility fan-out, the same
`SendMessageToSet(packet, true)` split as `SMSG_POWER_UPDATE`. No new
`WorldSession` field, lock or clock. Focused coverage: a new wow-entities
five-second-window timer test and three end-to-end regeneration tests (food
preferred over drink, drink-only, and suppression when no Standing flag is
present); `wow-world --lib persistence::` (104), the pre-existing regeneration
suite (7) and `wow-entities --lib unit::ops_3` (19) stay green; format,
`git diff --check`, the physical ratchet and `validation-v2 quick` (manifest
`20260916T221447.629046Z-2067328-quick.json`, 74.9 s) pass. `validation-v2 final`
stops at the pre-existing runtime hotspot LOC ratchet, which keeps its drift and
was not regenerated. #61 remains open for the player-killer (PvP)
`CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`, the remaining
`RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, and live DB/restart/relogin QA.

**#61 observer `SMSG_POWER_UPDATE` fan-out — 2026-09-16, implementation
`06ef4fd2`, integrated as `93532597` by PR #976:** the owner power publication now mirrors C++
`Unit::SetPower`'s `SendMessageToSet(packet, true)` (`Unit.cpp:9287-9312`). The
owner session still receives its own `SMSG_POWER_UPDATE`; the same serialized
bytes are queued for the nearby observers through the existing position-based
realm visibility rail via
`broadcast_player_packet_to_visible_set_realm_like_cpp`, which excludes the
source so each session gets exactly one packet. No new `WorldSession` field,
lock, clock or public API. Focused coverage: a new player-publication test
registers a visible observer and asserts the realm-visible command carries the
identical bytes, while the cast-lifecycle (13), regeneration (7) and durability
(14) suites stay green; format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest
`20260916T220112.859778Z-2062121-quick.json`, 72.7 s) pass. #61 remains open for
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
the remaining `RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, aura-backed expertise and live
DB/restart/relogin QA.

**#61 creature-kill durability loss (`Unit::Kill`) — 2026-09-16, implementation
`7d33d742`, integrated as `1dd3ba5b` by PR #974:** the creature-melee victim handler now runs the C++ `Unit::Kill`
player-victim durability branch (`Unit.cpp:10639-10648`). `over_damage >= 0` is
the represented kill signal (the map already committed the lethal swing), and the
PvE condition `durabilityLoss && !player && !victim->InBattleground()` skips a
battleground victim. `apply_represented_durability_loss_on_death_like_cpp` calls
`Player::DurabilityLossAll(baseLoss, false)` and returns the
`SMSG_DURABILITY_DAMAGE_DEATH` percent C++ derives as
`baseLoss - baseLoss * GetTotalAuraMultiplier(MOD_DURABILITY_LOSS)`; the C++
`uint32` truncation (0 with no aura) is reproduced rather than repaired. The loss
message is published before the melee result presentation, matching `Unit::Kill`
running inside `DealMeleeDamage`. Focused coverage: two new creature-melee
command tests (lethal publishes the loss, battleground skips it) plus the four
existing command regressions; the durability and durability-spell suites stay
green (`wow-world --lib durability`: 14 passed); format, `git diff --check`, the
physical ratchet plus its 20-test suite and `validation-v2 quick` (manifest
`20260916T214822.691938Z-2057566-quick.json`, 77.2 s) pass. #61 remains open for
the player-killer (PvP) `CONFIG_DURABILITY_LOSS_IN_PVP` branch and `SetPvPDeath`,
the remaining `RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 durability-damage spell effects — 2026-09-16, implementation `5759b474`,
integrated as `93d91726` by PR #972:**
the represented direct spell-effect dispatch now handles
`SPELL_EFFECT_DURABILITY_DAMAGE` (111) and
`SPELL_EFFECT_DURABILITY_DAMAGE_PCT` (115) for the represented player target
(`SpellEffects.cpp:4316-4373`). Effect 111 calls
`Player::DurabilityPointsLossAll(damage, misc < -1)` when `MiscValue < 0` and
`Player::DurabilityPointsLoss` on the `INVENTORY_SLOT_BAG_0` slot otherwise;
effect 115 calls `Player::DurabilityLossAll(damage / 100, misc < -1)` /
`Player::DurabilityLoss`. Both reuse the integrated broken-item mod-removal
ordering and the `SPELL_AURA_PREVENT_DURABILITY_LOSS` gate, and only the
represented player target is processed like C++'s `TYPEID_PLAYER` guard. Focused
coverage: two new end-to-end `execute_spell` tests plus the existing
break/mod-removal and fall-producer regressions (`cargo test -p wow-world --lib
durability`: 12 passed); format, `git diff --check`, the physical ratchet and
`validation-v2 quick` (manifest
`20260916T213346.613171Z-2052499-quick.json`, 150.2 s) pass. #61 remains open for
the general `Unit::Kill` PvE/PvP producer, the remaining `RatesForPower` entries,
alternate powers, rune regeneration, `IsPolymorphed`/`m_transformSpell`, observer
`SendMessageToSet` parity and live DB/restart/relogin QA.

**#61 fall-death item durability loss (`Player::EnvironmentalDamage` →
`DurabilityPointsLoss`) — 2026-09-16, implementation `c8059a85`, integrated as
`68bfce47` by PR #970:** the
represented durability chain now runs on a lethal fall.
`apply_represented_durability_loss_all_like_cpp` walks the equipment slots (and,
when `inventory`, the backpack and bag contents) exactly like
`Player::DurabilityLossAll` (`Player.cpp:4522-4544`), applies the
`SPELL_AURA_MOD_DURABILITY_LOSS` multiplier and the `max(1, ...)` floor per
item, and honours `SPELL_AURA_PREVENT_DURABILITY_LOSS`. It reproduces
`Player::DurabilityPointsLoss` (`Player.cpp:4590-4620`): reaching 0 durability
removes the equipped item's mods *before* the durability write so the
represented `_ApplyItemMods` gate still sees the item as unbroken, and a repair
restores them. The fall-to-death branch of `Player::EnvironmentalDamage`
(`Player.cpp:655-670`) now publishes `SMSG_DURABILITY_DAMAGE_DEATH` through the
new `DurabilityDamageDeath` packet (`MiscPackets.cpp:481-486`).
`DurabilityLoss.OnDeath`/`RATE_DURABILITY_LOSS_ON_DEATH` (`World.cpp:710-721`)
is a registry row loaded by the composition root and installed through
`SessionRuntimePolicyCapabilitiesLikeCpp`; C++'s out-of-range 0.0 behaviour is
reproduced rather than repaired. Focused coverage: the wow-packet layout test,
the wow-config registry count/key test (353 rows, 50 Float), the world-server
`durability_loss_on_death_rate_uses_cpp_world_config_key_like_cpp` config test,
the `durability_points_loss_breaks_and_removes_equipped_item_mods_like_cpp`
break/mod-removal test and the `fall_death_applies_item_durability_loss_message_like_cpp`
producer test; format, `git diff --check`, the physical ratchet plus its 20-test
suite and `validation-v2 quick` (manifest
`20260916T211452.247049Z-2046069-quick.json`, 553.7 s) pass. The physical policy
records the reviewed one-line `app.rs` composition delta. The runtime hotspot and
Session syntax-ownership ratchets keep their pre-existing drift and were not
regenerated. #61 remains open for the general `Unit::Kill` PvE/PvP producer,
`SPELL_EFFECT_DURABILITY_DAMAGE`/`DURABILITY_DAMAGE_PCT`, the remaining
`RatesForPower` entries, alternate powers, rune regeneration,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 C++ regeneration rates — 2026-09-16, implementation `bf3794f0`,
integrated as `fbed40ea` by PR #968:** the
regeneration tick no longer hardcodes `rate: 1.0`/`rate_health: 1.0`; it
consumes the C++ `World::setRegenRate` values (`World.cpp:615-623`).
`PlayerRegenerationRatesLikeCpp` (`session_policy.rs`) carries `RATE_HEALTH`,
`RATE_POWER_MANA`, `RATE_POWER_RAGE_LOSS`, `RATE_POWER_FOCUS`,
`RATE_POWER_ENERGY` and `RATE_POWER_RUNIC_POWER_LOSS`, maps them through the
C++ `RatesForPower` table (`Player.cpp:1681-1747`) and defaults every field to
1.0 like `sWorld->getRate`. The six keys are now rows of
`cpp-world-config-registry.tsv`; the composition root resolves them from the
already-loaded `WorldConfigSet` and attaches the immutable value to
`SessionHandlerCatalogsLikeCpp`, so no new `WorldSession` field or second
authority appears. `Rate.Health` scales the `RegenerateHealth` spirit component
and the per-power rate scales `Regenerate` before the aura producers, exactly as
C++. The physical policy records the reviewed 3-line composition delta for
`app.rs`. Focused coverage: the wow-config registry count/key test (352 rows, 49
Float), the world-server `player_regeneration_rates_use_cpp_world_config_keys`
config test and the `regeneration_rates_scale_mana_and_health_like_cpp` tick
test; format, `git diff --check`, the physical ratchet plus its 20-test unit
suite and `validation-v2 quick` (manifest
`20260916T204201.787414Z-2033943-quick.json`, 523.7 s) pass. The runtime hotspot
and Session syntax-ownership ratchets keep their pre-existing drift and were not
regenerated. #61 remains open for the remaining `RatesForPower` entries of
unrepresented powers (soul shards, lunar power, combo points, ...), alternate
powers beyond the represented primary, `UpdateAllRunesRegen`/rune cooldowns,
`IsPolymorphed`/`m_transformSpell`, observer `SendMessageToSet` parity and live
DB/restart/relogin QA.

**#61 non-mana power-regeneration loop — 2026-09-16, implementation `2387c04b`,
integrated as `4206ab00` by PR #966:**
the session tick now walks the complete C++ `RegenerateAll` power loop
(`Player.cpp:1614`): it iterates `POWER_MANA..MAX_POWERS`, skips powers without a
represented index, and applies one `Player::Regenerate` per power, so a
rage/energy/focus/runic-power primary is regenerated or decayed from its DB2
`PowerTypeEntry` instead of being ignored. `Unit::regenerate_power_like_cpp` now
applies the non-mana aura producers from `Player.cpp:1745-1747`:
`GetTotalAuraMultiplierByMiscValue(SPELL_AURA_MOD_POWER_REGEN_PERCENT, power)`
and the flat `SPELL_AURA_MOD_POWER_REGEN` modifier scaled by `m_regenTimerCount`
for every power except energy, which uses `m_regenTimer`; mana keeps reading the
published `UpdateManaRegen` fields and skips those producers exactly like C++. The
tick resolves the per-power DB2 entry, the `SPELL_AURA_PREVENT_REGENERATE_POWER`
value check and both misc-value aura families once, then publishes one
`SMSG_POWER_UPDATE` per changed power on the two-second boundary. Focused
coverage: 4 new `wow-entities` tests (percent/flat aura, energy timer selection,
accumulated window for other powers, mana exclusion) and 2 production-shaped
`wow-world` tick tests (rage decay, energy throttle); format, `git diff --check`,
the physical-source ratchet and `cargo check -p world-server` pass. The runtime
hotspot and Session syntax-ownership ratchets keep their pre-existing drift and
were not regenerated. #61 remains open for `RatesForPower`/`sWorld->getRate`
config overrides, alternate powers beyond the represented primary,
`UpdateAllRunesRegen`/rune cooldowns, `IsPolymorphed`/`m_transformSpell`, the
observer `SendMessageToSet` `SetPower` parity, productive wear-to-broken and live
DB/restart/relogin QA.

**#61 health-regeneration tick (`Player::RegenerateAll` → `RegenerateHealth`) —
2026-09-16, implementation `fbd8755e`, integrated as `381038dd` by PR #964:** the
session-owned tick now runs the
complete C++ `Player::Update → RegenerateAll` step. It accumulates
`m_regenTimer`/`m_regenTimerCount` for every living in-world player regardless
of mana prevention or power representation, regenerates the represented primary
power, and runs the two-second `RegenerateHealth` branch on the canonical
`Unit`. `Unit::regenerate_health_like_cpp` reproduces the C++ formula
(`Player.cpp:1842-1882`): `OCTRegenHPPerSpirit` with the 50-Spirit split and the
`OCTRegenHP.txt`/`RegenHPPerSpt.txt` level/class ratios
(`Player.cpp:5162-5180`), the `RATE_HEALTH` under-15 level multiplier, the
out-of-combat `SPELL_AURA_MOD_HEALTH_REGEN_PERCENT` multiplier and
`SPELL_AURA_MOD_REGEN * 0.4` flat term, the in-combat
`SPELL_AURA_MOD_REGEN_DURING_COMBAT` `ApplyPct`, the non-stand `1.5x` factor,
`SPELL_AURA_MOD_HEALTH_REGEN_IN_COMBAT`, `m_baseHealthRegen / 2.5`, the `int32`
truncation and the `Unit::ModifyHealth` clamp (`Unit.cpp:8115-8155`). Positive
regen marks `UnitData::Health`, which the canonical map's `SendObjectUpdates`
publishes; C++ sends no explicit packet for a positive delta. `wow-data` now
owns the moved `RegenMPPerSpt` family plus the new `RegenHPPerSpt` and
`OCTRegenHP` tables in `game_tables/regen.rs` behind `RegenGameTablesLikeCpp`
(`GameTables.h:124-165,302-330`, `GameTables.cpp:128,130`); the composition root
loads the bundle once and installs it on the Session, replacing the single MP
table field. Focused coverage: 7 `wow-entities` formula tests, 9 `wow-data`
regen-table tests and 2 production-shaped `wow-world` tick tests (window gating
+ in-combat suppression); format, `git diff --check` and `cargo check -p
world-server` pass. The architecture syntax ownership baseline remains the
pre-existing stale set from #958/#962 and was not regenerated; this slice
renames the already-unreviewed mana-regen Session field/setter and adds the
health tick method, so `session-ownership-check --syntax-only` stays red on the
same drift. (Superseded 2026-09-17 by PR #1153: the reviewed baseline delta was
reconciled from `print-baseline` and `session-ownership-check check
--syntax-only` passes at `1f25ce5b`.) #61 remains open for `IsPolymorphed`/`m_transformSpell`, alternate
powers, `Rate.Health`/`Rate.Mana` config overrides, observer
`SendMessageToSet` packet-type parity, productive wear-to-broken, aura-backed
expertise and live DB/restart/relogin QA.

**#61 mana-regeneration tick and `SMSG_POWER_UPDATE` publication — 2026-09-16,
implementation `5bc59ddb`, integrated as `d74381ec` by PR #962:** the canonical
Player snapshot now drives the C++
`Player::Update → RegenerateAll → Regenerate(POWER_MANA)` chain instead of only
publishing the regen fields. A session-owned tick (the same `Player::Update`
boundary already used for `DoMeleeAttackIfReady`) runs with the canonical
world/map diff, accumulates `m_regenTimer`/`m_regenTimerCount`, and consumes the
published `PlayerEffectiveCombatStatsLikeCpp::mana_regen`/`mana_regen_combat`,
the DB2 `PowerTypeEntry` (`RegenPeace`, `RegenCombat`, `MinPower`, `CenterPower`,
`Flags`, `RegenInterruptTimeMS`) and the five-second MP5 rule
(`_regenMP5InterruptStartTime`). The operation reproduces the `m_powerFraction`
carry, the min/max clamp, the `m_regenTimerCount >= 2000 || forcesSetPower`
publication boundary, the throttled `ClearChanged` write (the suppressed setter
does not queue a packet) and the new `PowerUpdate`
(`SMSG_POWER_UPDATE`, `CombatPackets.cpp:104-116`) on the boundary.
`Spell::TakePower` (`Spell.cpp:5444-5445`) now arms the five-second rule, and
`SPELL_AURA_PREVENT_REGENERATE_POWER` (`SpellAuraDefines.h:389`) is resolved from
the canonical visible auras. The regen runtime state lives on the canonical
`Unit` (C++ keeps the timers on `Player` and the MP5 mark on `Unit`) so the
reviewed `player/mod.rs` physical ceiling is preserved. Focused coverage: 6
`wow-entities` regeneration tests, the `wow-packet` `PowerUpdate` layout test and
3 production-shaped `wow-world` tests (throttle/publish, interrupted rate,
mana-cost MP5 producer); format, `git diff --check` and the affected
entity/packet/world checks pass. The architecture physical-source ratchet passes;
the runtime hotspot ratchet keeps its pre-existing `session/mod.rs`,
`character/mod.rs`, `world-server/lib.rs` and `player/mod.rs` growth and was not
regenerated. #61 remains open for health regeneration, non-mana powers, the
`Rate.Mana` config override, observer `SendMessageToSet` packet-type parity,
productive wear-to-broken and live DB/restart/relogin QA.

**#61 canonical equipment contribution closure — 2026-09-15, implementation
`a43f51cf`:** the effective-stat projection no longer sums equipped inventory rows
alongside the canonical `PlayerItemBonusStateLikeCpp` accumulator. Login now seeds the
accumulator through the same non-broken-item gate as `_ApplyAllItemMods`, and all represented
equipment transitions publish the complete Player-owned effective snapshot consumed by
combat. The focused regression covers equip, unequip, break, repair and login equivalence;
affected inventory suites pass 7/7, 5/5, 10/10 and 16/16, and `cargo check -p world-server`
passes. The architecture check remains blocked by the pre-existing `session/mod.rs`
ratchet drift and was not masked by baseline regeneration. #61 remains open for the
unimplemented wear-to-broken producer, aura-backed and complete combat formulas, and live
capture/DB/relogin proof.

**#61 canonical equipment expertise publication — 2026-09-15, implementation
`48378050`:** the complete Player snapshot now derives the equipment/rating portion
of mainhand, offhand and aggregate expertise from the canonical expertise rating and
the level-specific `CombatRatings` multiplier. The VALUES adapter consumes that
snapshot, retaining its raw calculation only for handle-less test/early-login
fixtures. This follows `Player::GetRatingMultiplier` / `GetRatingBonusValue`
(`Player.cpp:5189-5209`) and `Player::UpdateExpertise`
(`StatSystem.cpp:759-783`); aura expertise remains a separate producer gate.
The focused canonical equipment regression and item suites pass (1, 7 and 5 tests),
format/diff checks pass and `cargo check -p world-server` passes in 7m01s. The
architecture ratchet still has the previously recorded `session/mod.rs` drift;
no baseline was regenerated. #61 remains open for aura-backed expertise, full
combat/regen/penetration formulas, wear-to-broken production and capture/DB/relogin
acceptance.

**#61 C++ mana-regeneration table projection — 2026-09-15, implementation
`9004f5cf`:** startup now loads `gt/RegenMPPerSpt.txt` once through the world-server
composition root and installs the immutable table in each `WorldSession`. The canonical
Player effective-stat snapshot computes spirit regeneration as
`sqrt(Intellect) * Spirit * ratio[level,class]`, matching `Player::OCTRegenMPPerSpirit`
(`Player.cpp:5182-5190`) and the spirit branch of `Player::UpdateManaRegen`
(`StatSystem.cpp:799-827`); the packet adapter consumes that snapshot instead of keeping
the former hard-coded class coefficients and extra constant. The table parser, class-column
mapping and level-80 fixture are covered by 3 `wow-data` tests; the focused character
stat/persistence set passes 6/6, `cargo check -p world-server` passes in 3m02s, and format/
diff checks pass. Aura percentage modifiers, stat-derived MP5, full regen tick publication,
wear-to-broken production and live capture/DB/relogin acceptance remain explicit #61 gates.
The architecture physical-source check passes; its runtime hotspot ratchet still reports
the pre-existing drift in `session/mod.rs`, character handlers, `world-server/lib.rs` and
`wow-entities/player/mod.rs`, so no baseline was regenerated.

**#61 aura-backed mana-regeneration percentage projection — 2026-09-15, implementation
`48218460`, integrated by PR #959 (`4f7ce25b`):** the canonical Player stat publisher now applies the
two C++ percentage producers from `Player::UpdateManaRegen` (`StatSystem.cpp:809-812`):
`SPELL_AURA_MOD_POWER_REGEN_PERCENT` and `SPELL_AURA_MOD_MANA_REGEN_PCT`, both filtered
to `POWER_MANA`. The producer resolves canonical visible aura applications against
immutable `SpellInfo` effect metadata and multiplies each active effect as
`1 + amount / 100`, preserving loaded applications whose represented-effect shortcut
is absent. The focused aura regression and the six existing stat-update regressions
pass; the affected `world-server`, `wow-data` and `wow-world` test-aware check also
passes. Flat `MOD_POWER_REGEN`, stat-derived MP5, complete tick/publication semantics,
wear-to-broken production and live capture/DB/relogin evidence remain explicit #61
gates. Same-effect C++ stack-policy details stay outside this bounded producer until
the aura application contract carries them explicitly. The architecture physical-source
check passes; its runtime hotspot ratchet still reports the pre-existing growth in the
Session, character-handler, world-server and Player roots, so no baseline was regenerated.

**#61 flat and interrupted mana-regeneration aura projection — 2026-09-15, implementation
`bfe49b2b`, integrated by PR #960 (`96b8ffe3`):** the same Player-owned producer now completes the
remaining local `UpdateManaRegen` arithmetic. It adds `MOD_POWER_REGEN` to the flat
MP5-equivalent rate, adds each `MOD_MANA_REGEN_FROM_STAT` effect as
`stat * amount / 500`, and applies the capped `MOD_MANA_REGEN_INTERRUPT` percentage
to the spirit component for the interrupted-combat field (`StatSystem.cpp:815-826`).
The canonical aura resolver sums modifiers by MiscValue and retains the same immutable
SpellInfo/visible-application authority used by the percentage path. The combined aura
regression and six existing stat-update regressions pass; the affected test-aware
world-server/data/world check, format and diff checks pass. The old `ModPowerRegen`
packet field remains zero because this C++ path writes `PowerRegenFlatModifier` and
`PowerRegenInterruptedFlatModifier`; full tick/publication, wear-to-broken production
and live capture/DB/relogin evidence remain #61 gates. The physical-source architecture
check passes; the pre-existing runtime hotspot ratchet remains unaltered.

**Fresh Creature runtime audit — 2026-09-15:** the current boundary and next
macro are recorded in
[`creature-runtime-audit.md`](../architecture/creature-runtime-audit.md).
`WorldCreature` remains the mutable runtime/AI owner while
`wow-map::Map::entity_world` supplies visibility and target reads. Production
uses `GlobalLegacy` and the canonical `ExternalRuntime` no-op to avoid a second
timer writer; cloned sync fences protect stale health/loot/incarnation writes,
but the canonical `runtime_update_plan` still has no production effect
consumer. The structural **C3.1 — one map-owned Creature runtime outcome
boundary** is implemented by `e2ca3df9`: typed tick input/outcome, admitted
ObjectUpdater Creature stamps, authority/incarnation validation and
production-linked phase-order, death/respawn and stale-incarnation tests. The
full C++ `Map::Update`/`Creature::Update` operation is therefore not converged;
the next behavior vertical must consume this envelope. No mass owner migration
or timer-only plan consumer is selected.

**#63 movement audit — 2026-09-14:** the current code and TrinityCore anchors show
that ordinary force-speed ACKs are already implemented in
`session/movement/speed.rs` with the C++ forced-change counter, transport exemption,
correction and kick branches (`MovementHandler.cpp:468-546`), and knockback ACK
admission/publication is integrated by PR #866 (`MovementHandler.cpp:548-559`). They
are no longer pending implementation items. The open boundary is complete
vehicle/transport seat-offset admission, runtime branches whose mover or consumer is
not represented, exact packet-order captures and live client/server/DB QA.

**Player `m_seer` visibility projection integrated — 2026-09-14, PR #923, merge
`5f6b1ad83d6ea6adf81368a36384593808632737`:** TrinityCore initializes
`Player::m_seer` to self (`Player.cpp:298-300`, `Player.h:2417-2425`) and mutates
it through `SetViewpoint` (`Player.cpp:25338-25395`), while map and visibility
read that pointer (`Map.cpp:716-718`, `GridNotifiers.cpp:95-222`). Rust production
now derives the seer GUID from the canonical map-owned Player's
`ActivePlayerData::FarsightObject`; empty means the Player itself. Deferred
visibility, movement, aggro and GameObject/DynamicObject consumers use the
projection, and the former Session field is a `cfg(test)` fixture only.
`last_observed_farsight_object_like_cpp` is a receiver-local publication fence for
the explicit FAR_SIGHT clear VALUES packet, not gameplay authority. Focused
FAR_SIGHT (14), GameObject despawn (22), DynamicObject VALUES (15), package,
ownership syntax, architecture and 20 self-tests pass. Full captures,
DB/relogin durability and live QA remain open gameplay/runtime gates under
#41/#63/#584; #584 still owns other C0-C4 and runtime work.

**Canonical Pet visibility CREATE integrated — 2026-09-14, #584 / PR #925, merge
`a1f66c33903ee76b6bb0675177d33e3b283806ed`:** the canonical map already places
Pets in the Creature cell family, matching TrinityCore's generic
`Map::AddToMap`/`UpdateObjectVisibilityOnCreate` path (`Map.cpp:530-610`) and
`Pet::AddToWorld` (`Pet.cpp:69-88`). The Session visibility query now reads
Creature/Pet records through `with_creature_or_pet_like_cpp`, so an in-world Pet
passes the existing map, phase, range and detection gates and reaches the common
Creature CREATE snapshot. The focused regression
`visible_creatures_skip_not_in_world_canonical_objects_like_cpp`
passes, as do the affected package check and formatting/diff checks. This closes
the projection gap only; Pet AI/movement, summon ownership/persistence,
corpse/transport lifecycle, exact captures and live DB/restart/relogin QA remain
explicit #584/#63 gates. Directed Pet DESTROY is covered by the integrated PR #927
slice recorded below.

**Directed object DESTROY integrated — 2026-09-14, #584 / PR #929, merge `028185d8`:**
TrinityCore removes ordinary units through `Map::RemoveFromMap` while the source is
still attached, so `WorldObject::DestroyForNearbyPlayers` can walk nearby Players
(`Map.cpp:934-951`, `Object.cpp:3617-3648`); `Pet::RemoveFromWorld` follows that
Unit path (`Pet.cpp:94-101`). For Corpses, `Corpse::RemoveFromWorld` delegates to
the same world-object removal path (`Corpse.cpp:56`, `Object.cpp:1023-1029`). Rust
now captures one generic object recipient list before canonical erasure and sends
one directed DESTROY command for Creature, Pet or Corpse, retaining map-incarnation
and `HaveAtClient` fences. The `wow-map` visibility suite (47 tests), `wow-world`
deferred-visibility suite (12 tests), mailbox suite (16 tests), package check and
architecture checks pass. Pet AI, summon ownership, corpse reclaim/persistence/loot,
vehicle/transport lifecycle and live QA remain separate gates.

**P4 loaded-grid test split integrated — 2026-09-14, #584 / PR #931, merge `b2545d50`:**
`crates/world-server/src/creature_loaded_grid.rs` now keeps 797 production lines and
mounts `creature_loaded_grid_tests/mod.rs`; its builder/resolver children retain all
28 regressions under explicit `cfg(test)`. This is a physical navigability closure:
no runtime owner, dependency, packet, persistence or behavior changed. Focused
world-server tests pass 28/28; format/diff and architecture check/self-test pass.
The global physical terminal remains open for other #584 entries.

**P4 game-event runtime split integrated — 2026-09-15, #584 / PR #933, merge `ef30bb3e`:**
`crates/world-server/src/runtime/game_events.rs` is now a compact facade over seven
responsibility modules (unspawn, grid, spawn, bootstrap, scheduler, live update and
consume). The complete 181-test game-event suite remains green; `cargo check`,
format/diff and architecture check/self-test pass. No runtime owner, dependency,
packet, persistence or behavior changed. The global physical terminal and remaining
C0–C4/runtime/capture/DB/relogin/live-QA gates remain open.

**P4 spawn catalog navigability integrated — 2026-09-15, #584 / PR #935, merge
`c922fd60d71018a0502954a6ddffe151401e48bf`:** the post-#933 audit split the
5,125-line `world-server/src/spawn_store_loader.rs` by responsibility. Startup
composition, DB ownership and spawn conversion remain in the 3,427-line facade;
GameEventMgr catalog models and WorldStateMgr startup/index rules now live in
`spawn_store_loader/game_event_catalog.rs` (1,355 lines) and
`world_state_catalog.rs` (345 lines), with the existing public paths reexported.
The published #933 runtime child modules are crate-visible to their parent reexports,
restoring the integration compile contract. The focused game-event suite passes
181/181; compile, format/diff and architecture check/self-test pass. The full local
profile still has three pre-existing `scenarios_9` GameObject respawn-save failures
(587 passed, 3 failed). Follow-up PR #936 tightens the two physical policy rows to the
exact scanner counts at the current head. This is physical organization only; remaining #584 C0–C4,
runtime, capture, DB/relogin and live-QA gates stay open.

**P4 game-event runtime/cache split integrated — 2026-09-15, #584 / PR #938, merge
`c5967026730b8c95802839f3d48ef083cae4dfac`:** the post-#935 audit extracted the
complete game-event state-transition and cache family from
`spawn_store_loader.rs` into private `spawn_store_loader/game_event_runtime.rs`.
The implementation remains an inherent impl on `CanonicalSpawnMetadataLikeCpp`,
so its canonical ownership and public method paths are unchanged. The current
facade is 2,627 lines and the child is 800; the aggregate hotspot remains at the
validated 3,427-line ceiling. The 181 game-event tests pass; compile,
format/diff, architecture check and self-test pass. The full local profile still
reports the three pre-existing `scenarios_9` GameObject respawn-save failures
(587 passed, 3 failed). No packet, SQL, clock, lock, persistence order or runtime
behavior changed; the remaining loader families and #584 C0–C4/runtime/capture/
DB-relogin/live-QA gates stay open.

**P4 GameEventMgr startup-loader split integrated — 2026-09-15, #584 / PR #940,
merge `2959e24565ea786fa507105eb28a519f52b1fa7e`:** the post-#938 audit moved
the complete GameEventMgr startup loading/validation family into private
`spawn_store_loader/game_event_loader.rs`. Conditions, prerequisites, pools,
spawn GUIDs, quest relations, NPC flags/vendors and model/equipment validation
remain the same operations and order; only their physical module changed. The
composition facade is 1,990 lines and the loader child 637; with the 800-line
runtime child, the measured spawn-loader aggregate remains at the exact 3,427-line
ceiling. All 181 game-event tests pass, as do compile, format/diff, architecture
check/self-test and final structural gates. The full local profile still has the
three pre-existing `scenarios_9` GameObject respawn-save failures (587 passed, 3
failed). Parent-private fixture helper paths remain available through `pub(super)`;
no public API, owner, startup behavior, packet, SQL, clock, lock or persistence
order changed. Remaining loader families and #584 C0–C4/runtime/capture/DB-relogin/
live-QA gates remain open.

**P4 pool and spawn-group startup-loader split integrated — 2026-09-15, #584 / PR #942,
merge `77ca1c64fdca504172bf47c07b43a3a6a9d2aaf8`:** the post-#940 audit moved
PoolMgr templates and member loading, relation/map/final validation, autospawn
candidates and spawn-group template construction to private
`spawn_store_loader/pool_loader.rs`. The composition facade is 1,652 lines and
the child 338; the measured aggregate remains exactly the 3,427-line ceiling.
The public spawn-group helper path and parent-private fixture access remain
unchanged. Focused pool tests pass 29/29, with architecture, compile,
format/diff and structural gates passing; the full profile retains the same
three pre-existing `scenarios_9` GameObject respawn-save failures (587 passed,
3 failed). No startup order, owner, packet, SQL, clock, lock, persistence or
runtime behavior changed; remaining loader families and #584 C0–C4/runtime/
capture/DB-relogin/live-QA gates remain open.

**P4 object-spawn startup-loader split integrated — 2026-09-15, #584 / PR #944,
merge `e0a8fae08aa0f2d220330495eb8303125969bd18`:** the post-#942 audit moved
Creature, GameObject and AreaTrigger spawn-row loading, conversion/validation,
runtime-row capture and linked-respawn admission to private
`spawn_store_loader/spawn_object_loader.rs`. The composition facade is 999
lines and the child 653; with the existing pool and GameEvent children, the
measured aggregate remains exactly the 3,427-line ceiling. Existing startup
calls and parent-private fixture paths remain unchanged. Focused spawn-loader
tests pass 135/135, with architecture, compile, format/diff and structural
gates passing; the full profile retains the same three pre-existing
`scenarios_9` GameObject respawn-save failures (587 passed, 3 failed). No
startup order, owner, packet, SQL, clock, lock, persistence or runtime behavior
changed; the remaining addon loader family and other #584 gates remain open.

**P4 creature movement metadata split integrated — 2026-09-15, #584 / PR #946,
merge `9698a656b2a3dad320a6f4a3d737d7698a326f79`:** the post-#944 audit moved
waypoint path/report construction, coordinate and delay normalization, default
waypoint lookup, creature formation validation and their persistence loaders to
private `spawn_store_loader/creature_movement_loader.rs`. The composition facade
is 784 lines and the child 215; with the existing pool, object and GameEvent
children, the measured aggregate remains exactly the 3,427-line ceiling. Public
waypoint/formation types and helper paths remain available through explicit
re-exports and startup order is unchanged. Focused spawn-loader tests pass
135/135, with architecture, compile, format/diff and structural gates passing;
the full profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No gameplay, packet, SQL, clock,
lock, persistence or runtime behavior changed; the remaining addon loader family
and residual composition readers require a fresh audit.

**P4 spawn-loader residual adapter closure integrated — 2026-09-15, #584 / PR
#948, merge `861fbc701a6e556f4988ad149e90faa23b3c53f1`:** the post-#946 audit
moved the linked-respawn row conversion into `spawn_object_loader.rs`, the
GameEvent world prefix/suffix adapters into `game_event_loader.rs`, and the
spawn-group member persistence adapter into `pool_loader.rs`. The composition
facade is 730 lines; the object, pool, GameEvent loader, movement and runtime
children are 661, 358, 663, 215 and 800 lines, with the measured aggregate still
exactly 3,427 lines. Existing scoped imports preserve startup order, public paths
and parent-private fixtures. Focused spawn-loader tests pass 135/135; architecture
check/self-test, compile, format/diff and final physical/hotspot gates pass. The
full profile retains the same three pre-existing `scenarios_9` GameObject
respawn-save failures (587 passed, 3 failed). No gameplay, packet, SQL, clock,
lock, persistence or runtime behavior changed. PR #950 reconciles the stale
Session ownership anchor and exact syntax, registry and bridge baselines after
#929/#933; the checker-only suite passes 364/364, syntax-only ownership passes,
and architecture check/self-test remain green. The next step is a fresh audit
of the remaining #584 C0–C4/runtime/capture/DB/relogin/live-QA work.

**Transport C0/C3 lifecycle integrated — 2026-09-14, PR #901, merge
`bf460aa7a8ccec0269eea1771a094ef12f0c6109` (implementation `82b2d8d9`):** the bounded
CREATE/DESTROY and phase-visibility slice is integrated into `3.4.3`. `wow-map` marks same-phase Players
through the Transport map-reference walk (`Map.cpp:574-610,1853-1915`) on add/remove;
`wow-world` snapshots typed in-world transports into owned `GameObjectCreateData`,
creates `CreateTransport`/OUT-OF-RANGE blocks during the existing deferred visibility
refresh, and publishes `m_visibleTransports` atomically with the packet. The map guard
is released before Session delivery, and the separate Transport VALUES membership
rail remains intact. Focused map visibility and entity-bridge regressions pass, as do
the `wow-world` package check and formatting/diff checks on the merged implementation. The slice
does not claim `Transport::TeleportPassengersAndHideTransport` map relocation,
passenger seat/offset admission, AI/scripts, taxi routing, exact C++ captures, or live
DB/restart/relogin QA; those remain explicit #63/#584 gates. No legacy Creature writer
migration is implied.

**P2 Player identity owner — integrated PR #904, merge `4ad36d420a0f56297390262f3667be1b5fc4ae6f`,
2026-09-14:** the #584 macro moves name, race, class, level and gender
authority to the canonical `wow_entities::Player`/`Unit`/`WorldObject`. The change
follows TrinityCore `Player::LoadFromDB` (`Player.cpp:17060-17089`, `17247-17283`)
and `Unit::GetLevel/GetRace/GetClass/GetGender` (`Unit.h:733-745`). Session retains
only a login bootstrap DTO until owner installation; the old identity fields are
fixture-only under `cfg(test)`, stale handles fail closed, and module/registry/
character/group/chat consumers resolve through the canonical owner. `Player::m_swingErrorMsg`
remains a separate melee residual. The candidate passes the two crate checks, 919
entity tests, login 17/17, persistence 25/25, identity 23/23, formatting/diff and
architecture check plus 20 self-tests. Save/reload durability, captures and live
DB/relogin QA remain open gates; the next #584 implementation must follow a fresh
C0–C4 responsibility and consumer audit rather than reselecting this slice.

**P2 Player-owned mount presentation transition — 2026-09-14, #584 / PR #897, integration `90e58c7358d03340fb9ce10461a14dd33b94ceed` (implementation `bc58334f`):** the remaining production mount-presentation write now uses a named `Player` transition that applies `MountDisplayID` and `UNIT_FLAG_MOUNT` together, following TrinityCore `Unit::Mount` / `Unit::Dismount` (`Entities/Unit/Unit.cpp:7822-7865`). Session retains aura, collision, vehicle-kit and packet side effects; its broad unit-presentation closure is test-only for detached scale fixtures. The Player owner regression, mount spell-state scenarios, package, formatting/diff and architecture checks pass. Full mount gameplay, persistence, captures and live QA remain separate gates under #63/#584.

**P2 Player-owned RestMgr transitions — 2026-09-14, #584 / PR #895, integration `94c21521a6d215f8f9fb086ddbef3267ee08c10f` (implementation `ea18126e`):** the remaining production Player rest-flag, deferred-publication and rest-clock writes now use named transitions on `wow-entities::Player` over its Player-owned `PlayerRestState`. The boundary follows `RestMgr::SetRestFlag` / `RemoveRestFlag` (`RestMgr.cpp:95-122`), `RestMgr::_restTime` (`RestMgr.h:86`) and `Player::SetRestState` (`Player.h:2652`). Session retains zone/catalog resolution, packet publication and application ordering; its generic PlayerRestState mutator is now a detached-fixture seam under `cfg(test)` only. The Player owner regression, rest-owner scenarios, chat, area-trigger, zone, package, formatting/diff and architecture checks pass. Quest objective progress, durable persistence, captures and live QA remain separate gameplay gates under #41/#584.

**P2 Player-owned mount VehicleKit operations — 2026-09-14, #584 / PR #878,
integration `d8cb0594093c25ae618ab2603896fd3e2f7de26d` (implementation
`ceb58c9a`):** the remaining production Session mutation surface for a Player's
mount vehicle kit now routes through named `Player` operations in
`crates/wow-entities/src/player/vehicle.rs`. This follows TrinityCore's
`Unit::CreateVehicleKit`, `Unit::RemoveVehicleKit` and `Unit::GetVehicleKit`
(`src/server/game/Entities/Unit/Unit.cpp:11304-11323`): install, snapshot,
clear, uninstall/removal and ejectable-passenger removal are owner transitions.
Session retains vehicle-template admission, aura/packet/presentation effects and
the detached fixture fallback under `cfg(test)`; no second production authority,
lock or clock is added. The owner lifecycle test, three represented-eject tests,
both package checks, formatting/diff checks and the architecture ratchet pass.
This is a structural owner closure only: complete vehicle seat/offset admission,
passenger lifecycle, CREATE/DESTROY, Pet/corpse/Transport publication, captures,
DB/restart/relogin and live QA remain separate #584/#63 gates.

**P2 Player-owned aura mutation operations — 2026-09-14, #584 / PR #881,
integration `2a916c1c429456085e9274a60fcf57138ce12b43` (implementation
`3e1bdd9d`):** the remaining production Session mutation surface for Player aura
state now routes through named operations on `Player` over its Unit-owned
`AuraSubsystem`: visible-aura insert/remove, persisted aura-authority completion,
spell-hit authority tombstone/reset, and threat-aura install/apply/remove. This
matches TrinityCore's Unit-owned aura maps and transitions (`Unit.h:620-640,
1226-1260,1825-1844`; `Unit.cpp:680-690`). Session retains packet/catalog
adaptation and the handle-less generic mutator is `cfg(test)` only. The focused
canonical/detached/replacement aura-authority regression, both package checks,
formatting/diff checks and architecture ratchet pass. Full aura gameplay, exact
packet captures, durable DB/restart/relogin and live QA remain separate #584 gates.

**P2 Player-owned trade transitions — 2026-09-14, #584 / PR #883, integration
`0f79ca837a7417876986cf1803f715a8512b7503` (implementation `50e98f43`):** the represented `TradeData` state now has a private
`wow-entities::Player` owner module with named open/clear, state-index,
acceptance, gold, item-slot and trade-spell transitions. This follows
`Player::m_trade` (`Player.h:2998`), creation in `TradeHandler.cpp:694-695`,
cleanup in `Player.cpp:12864-12879` and `TradeData.cpp:58-150`. Session retains
money/inventory/catalog admission, packet encoding and partner mailbox delivery;
its broad trade mutator is available only to handle-less `cfg(test)` fixtures.
The focused Player owner regressions and the 15-test social scenario suite pass,
as do the world check, architecture ratchet and formatting/diff checks. This is
an ownership closure: full trade settlement durability, captures and live QA stay
with the gameplay acceptance boundary.

**P2 Player-owned guild membership transitions — 2026-09-14, #584 / PR #885,
integration `19dea8e078f5b7bec827532cefb8f46911e332e7` (implementation
`f0d32675`):** the represented guild membership and pending invitation state now
have named operations on `wow-entities::Player`,
following `Player::SetInGuild` (`Player.cpp:7216`), `SetGuildIdInvited` and
`SetGuildRank` (`Player.h:1939,1943`). Session remains the GuildMgr/cache, protocol
and application adapter; its whole-state guild mutator is retained only for
handle-less `cfg(test)` fixtures. The composite guild install is retired. Owner,
social scenario, guild handler, package, architecture and formatting/diff checks
pass. This is an ownership closure, not guild-manager/database, packet-capture or
live-QA acceptance.

**P2 Player-owned Battleground transitions — 2026-09-14, #584 / PR #887,
integration `72f6a3fa87d00f9319c1cfa626f7a10345fc9654` (implementation
`4bd82511`):** represented Battleground type/map, status, queue-slot and arena-team
invitation transitions now resolve named operations on `wow-entities::Player`. The
owner follows `Player::m_bgData`/`BGData` (`Player.h:976,2821`),
`InBattleground`/`GetBattlegroundTypeId` (`Player.h:2335-2338`), the represented
Battleground id write (`Player.cpp:24258-24262`) and
`SetArenaTeamIdInvited` (`Player.h:1956`). Session keeps queue admission,
matchmaking/lifecycle coordination, packets and application effects; its
whole-state Battleground mutator is retained only for handle-less `cfg(test)`
fixtures. Owner tests, the 11-test canonical ownership scenario, the 23-test PVP
handler suite, both package checks, formatting/diff checks and the architecture
ratchet pass. Functional queue/matchmaking/lifecycle, persistence, captures and
live QA remain separate #584 gameplay gates.

**P2 Player-owned persistent capability transitions — 2026-09-14, #584 / PR #889,
integration `e37570c4e1e9feee04aadac6d485f5d1f314ced1` (implementation
`4066e261`):** the character-loaded at-login flags and weapon/armor proficiency
masks now have named transitions on `wow-entities::Player` in
`player/persistent_capabilities.rs`. The operations follow
`Player::SetAtLoginFlag` (`Player.h:2474`) and
`Player::AddWeaponProficiency`/`AddArmorProficiency` (`Player.h:1433-1434`),
including idempotent mask OR and removal reporting. Session retains persistence
projection and packet/application orchestration; its whole-state mutator is
fixture-only under `cfg(test)`. Owner tests, the 25-test persistence scenario,
spell-state scenarios, package checks, formatting/diff checks and the architecture
ratchet pass. Durable save/reload, captures and live QA remain separate gameplay
gates; #584 stays open.

**P2 Player-owned taxi flight transitions — 2026-09-14, #584 / PR #891,
integration `faa5964bbdb360adc12e58279ab17b76c76b66b9` (implementation
`a2a32586`):** post-teleport route advancement and taxi-flight cleanup now use
named transitions on `wow-entities::Player` over its `PlayerTaxi` state. The
boundary follows `PlayerTaxi::NextTaxiDestination` (`PlayerTaxi.h:74`) and
`Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`); Session retains map
admission, spline completion and packet/application effects, while the generic
handle-less taxi mutator is fixture-only under `cfg(test)`. The Player owner
regression, 14 taxi tests, 11 canonical-access scenarios, 13 movement scenarios,
package checks, formatting/diff checks and architecture ratchet pass. Route
creation/node admission, teleport ordering, persistence, captures and live QA
remain separate #584 gameplay gates.

**P2 Player-owned world-local transitions — 2026-09-14, #584 / PR #893,
integration `cd054d5a3b19f9f75bfef726de1dc99368b3227b` (implementation
`89c66ada`):** all production zone/area, terrain-authority, PvP-hostility/timer
and outdoors writes now use named transitions on `wow-entities::Player` over
`PlayerWorldLocalState`. The boundary follows `Player::UpdateZone`,
`Player::UpdateArea`, `Player::UpdatePvPState`, `Player::UpdateContestedPvP`
and `WorldObject::IsOutdoors`; Session retains terrain/catalog resolution,
rest/aura/packet effects and application ordering, while the generic world-local
mutator is fixture-only under `cfg(test)`. The Player owner test, 11 owner tests,
chat/zone, world-state and outdoors/spell-state scenarios, package checks,
formatting/diff checks and architecture ratchet/self-test pass. Full terrain
admission, aura/quest/rest side effects, persistence, captures and live QA remain
separate #584 gameplay gates.

**F1 movement knockback-ACK slice — 2026-09-14, #63 / PR #866, merge
`0079daa81c4955e38031009a24b17b8dbabc7d9b`:** `HandleMoveKnockBackAck` now
admits a status whose GUID is the active `_player->m_unitMovedByMe`, after the
Player-owned validation, matching `MovementHandler.cpp:548-559`. The accepted
status remains written to the Player-owned movement-info state and the
`MoveUpdateKnockBack` publication keeps the Player as its source, as in C++.
A controlled-mover regression proves acceptance and Player-source publication;
the focused test, all 50 movement-handler tests, architecture checks,
`world-server` check and `validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T052813.807323Z-281858-quick.json`.
#63 remains open for ordinary speed ACKs, remaining transport/death/BG/taxi
branches, broader mover coverage, exact captures and live QA.

**F1 movement force-ACK slice — 2026-09-14, #63 / PR #864, merge
`2d375ef164a292f10f265a73e2086785161e95f8`:** `HandleMoveApplyMovementForceAck`,
`HandleMoveRemoveMovementForceAck` and `HandleMoveSetModMovementForceMagnitudeAck`
now validate against the active `m_unitMovedByMe`, read the expected magnitude from
that Unit (including controlled Creature/Pet map state), adjust accepted client time
and publish from the mover's position/GUID. This matches
`MovementHandler.cpp:581-663`, including `mover->SendMessageToSet`; wrong-GUID
ACKs are rejected without publication. The focused controlled-mover regression, 50
movement-handler tests, architecture checks, `world-server` check and
`validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T050653.422270Z-262358-quick.json`.
#63 remains open for ordinary speed ACKs, knockback, remaining transport/death/BG/
taxi branches, broader mover coverage, captures and live QA.

**F1 movement MoveTimeSkipped controlled-mover slice — 2026-09-14, #63 / PR #862, merge
`28762f166d39d69e844b99978bbd72e4085a2739`:** `HandleMoveTimeSkippedOpcode` now
validates the packet against the active `m_unitMovedByMe`, advances the Player or
represented controlled Creature/Pet's own uint32 movement clock, and publishes
`MoveSkipTime` from that mover's position/GUID. This matches
`MovementHandler.cpp:721-739`, including `mover->SendMessageToSet`; a focused
regression proves the controlled mover clock and observer routing. The focused
slice and movement suite pass (2 and 49 tests), architecture checks and
`world-server` compilation pass, and `validation-v2 quick` passes at
`target/validation-v2/manifests/20260914T043604.661090Z-236189-quick.json`.
#63 remains open for the remaining transport seat/offset admission, other movement
ACK/force/knockback/taxi/death/BG branches, broader mover coverage and live
client/server/DB capture QA.

**F1 movement stale-mover transport guard — 2026-09-14, #63 / PR #860, merge
`704dc4cb55652bd8a74974146a843dc3edc4a506`:** the stale transport distance
check now resolves the active mover position for Player, legacy controlled
Creature/Pet and the canonical Creature/Pet fallback before comparing against
`SIZE_OF_GRIDS`, matching `MovementHandler.cpp:345-350` for every represented
`Unit`. A focused controlled-mover regression rejects a large stale packet
without relocation or a second `MoveUpdate`; final validation passes with 3,881
tests and zero failures. #63 remains open for the remaining transport seat/
offset admission, death/BG/taxi branches, ACK/order and live client/server/DB
capture QA.

**F1 movement vehicle-turning slice — 2026-09-14, #63 / PR #859, merge
`95444be06bd7213529a0656d37c4aaeaa3410489`:** a Player passenger in a seat
with `VEHICLE_SEAT_FLAG_ALLOW_TURNING` now updates only facing, removes turning
interrupt auras and returns before relocation or `MoveUpdate` publication,
matching `MovementHandler.cpp:408-421`. Pre-return fall/landing/flight side
effects stay before the branch, and the focused regression proves unchanged
coordinates and no broadcast. Final validation passes with 3,881 tests and
zero failures. #63 remains open for complete transport offset/seat admission,
other mover kinds, death/BG/taxi branches, ACK/order and live capture QA.

**F1 movement transport membership slice — 2026-09-14, #63 / PR #855, merge
`10528d454f8e2b1504e6ed87ee5c1eb0a0d38524`:** the accepted Player movement path now reconciles the canonical Map-owned Transport passenger set before side effects and publication. A transport switch removes the previous passenger first; a valid in-world target is added; a missing or not-in-world target resets transport state. Focused attach/switch/detach/missing-target regressions and the 47-test movement suite pass; `validation-v2 final` passes with 3,880 `wow-world` tests and 0 failures. #63 remains open for vehicle seat/turning, complete offset validation, non-Creature/death/BG/taxi branches, ACK/order and live client/server/DB capture QA.

**F1 movement admission slice — 2026-09-14, #63 / PR #853, merge
`7c3add2fd5a1df791fcc793295028de553f9346a` (implementation `3af90ec2`):**
`HandleMovementOpcode` now rejects player movement while near or far teleport is
pending and rejects controlled-mover movement until its canonical MoveSpline is
finalized. The guards run before emote, position, state or packet-publication side
effects, matching `MovementHandler.cpp` and `Player.h::IsBeingTeleported`; the
active legacy `MapManager` spline authority remains used while that runtime is
live, with the canonical map fallback and fail-closed unknown state. Focused
positive/negative regressions, the full `wow-world` library suite (3,878 passed,
0 failed, 1 ignored), architecture checks and `validation-v2 final` passed at the
integration candidate. #63 remains open for transport passenger enter/exit and
reset, vehicle turning, non-creature movers, death/BG/taxi branches,
acknowledgement/order, and live client/server/DB capture evidence. The deferred
visibility bridge for `MoveInitActiveMoverComplete` is already integrated under
#588 and is not duplicated here.

**F1 equipment/stat projection — 2026-09-13, #61 / PR #851, merge
`b26ce713b844f1146a7b2952aade4dd532f1a16e` (implementation `fb33111c`):** the
character stat projection now lives in a private `handlers/character/stats.rs`
module and publishes one runtime-only `PlayerEffectiveCombatStatsLikeCpp` snapshot
owned by the canonical `Player`. Login and equipment recalculation publish the
represented item/stat projection; packet VALUES publication keeps its existing
adapter split. The bounded projection includes represented item stats, AP/ranged AP,
health/mana, ratings, spell power, armor, static school resistances and the Player
item-bonus fields for resistance, regen, penetration and shield. `PlayerGameplayState::is_empty`
now treats preallocated all-empty Void Storage capacity as empty. The final profile
passed at this merge SHA (`target/validation-v2/manifests/20260913T232737.961399Z-4127376-final.json`):
architecture/format checks, one-job workspace test compilation and 4,778
`wow-entities`/`wow-world` library tests passed (one ignored). #61 remains open for
production combat/melee/spell consumers, exact AP/damage/aura/regen/expertise/
penetration behavior, reversible equipment lifecycle and capture/live DB/relogin
acceptance where observable; this projection does not close the issue.

**F1 canonical AP threat consumer — 2026-09-14, #61 / PR #857, merge
`fd302c350ce56cae4841c04165e50c23ae0b01e8`:** initial spell threat now reads
the Player-owned effective attack-power snapshot, preserving C++ modifier sum,
non-negative clamp and multiplier order (`Spell.cpp:5558-5575`,
`Unit.cpp:9165-9180`). The snapshot retains AP and ranged-AP multipliers;
the focused harmful-spell and Player-owner tests pass. Aura-backed producers,
the remaining combat/stat consumers and live parity remain open in #61.

**F1 canonical weapon-range consumer — 2026-09-14, #61 / PR #858, merge
`cda7f8a0dcd2129c45d83982eabdb1ae7c3a6f14`:** the melee pass now snapshots
Player-owned base/offhand weapon ranges before the mutable Unit timer update;
item/AP/delay inputs come from one C++-shaped `wow-data` projection. The focused
world/entity/data tests and final validation pass at
`target/validation-v2/manifests/20260914T024623.613803Z-127420-final.json`.
Exact aura-backed damage modifiers, complete weapon admission, full spell/melee
parity and live capture/QA remain open in #61.

**F1 exact offhand admission — 2026-09-14, #61 / PR #869, merge
`abd396a0afcb247b411acbaf0d05d8713b747daf`:** the melee timer and swing path now
require a canonical Player offhand slot with a weapon inventory type and a
non-broken Item object, and reject the offhand branch in feral forms. This follows
`Unit::haveOffhandWeapon` (`Unit.cpp:496`), `Unit::DoMeleeAttackIfReady`
(`Unit.cpp:2140`), `Unit::IsInFeralForm` (`Unit.cpp:8807-8812`) and
`Player::GetWeaponForAttack` (`Player.cpp:9243-9270`). Focused owner admission/
feral regressions, all 26 `combat_tick_` world tests, production `world-server`
check and architecture checks pass at the pre-merge candidate. Aura-backed damage
modifiers, captures and live combat parity remain open in #61.

**Latest bounded data delivery — 2026-09-13, PR #848, merge
`179fd5d40491e4ded2a8c25b3261263330855cd5` (implementation `95274da1`):** the
effective Trait catalog now composes the C++ locale projections for
`TraitDefinition` and `TraitCurrencySource` after their effective base stores. The
adapter uses the exact locale SQL statements, official-then-custom precedence and
the same fail-before-publication field validation as the base hotfix set. Locale
entries are retained in the immutable Player catalog capability group for future
packet/runtime consumers. Focused locale composition and production batch tests pass;
this does not claim a live MariaDB startup/restart/relogin run, cross-store failure
coverage beyond the exercised batches, a production consumer for
`TraitDefinitionEffectPoints`, or later spending, mutation and starter-build
behavior. #524 remains open for those explicit gates.

**Previous bounded data delivery — 2026-09-13, PR #846, merge
`93fa95a9f4c803ff04c68253910766738f3b31be` (implementation `57116f75`):** the
effective Trait catalog now composes all 24 C++-projected base hotfix tables,
including `SpecSetMember`, in official-then-custom order before publication. Each
WDC4-backed store retains its table hash and applies final hash-scoped
`RecordRemoved` tombstones; malformed rows fail before the effective catalog is
published. The exact SQL projections are anchored to `HotfixDatabase.cpp:1409,
1690-1803`; focused adapter, statement and overlay tests pass. This delivery does
not claim locale-specific overlays (`trait_definition_locale` and
`trait_currency_source_locale`), a production consumer for
`TraitDefinitionEffectPoints`, complete cross-store/startup DB/restart/relogin
evidence, or later spending, mutation and starter-build behavior; #524 remains open
for those gates.

**Latest bounded architecture delivery — 2026-09-13, PR #844, merge
`6f42782fedb1eb77d7896fd139c195fbfbb9c43b` (implementation `77e2c4b2`):** the
fixed `Player::_voidStorageItems` state and its clear/load/mark/free-slot/lookup/
add/delete/swap transitions now have one canonical owner in `wow-entities::Player`.
Session keeps item-template admission, persistence orchestration, packet encoding and
detached test fixtures. The C++ anchors are `Player.cpp:18334`, `20002` and
`28025-28098`; no second production authority, lock or clock was added. The focused
owner invariants and all 29 existing Void Storage world tests pass. This slice does
not claim durable DB/restart/relogin evidence or close #584.

**Previous bounded architecture delivery — 2026-09-13, PR #839, merge
`cc0559980a4232ab5743affaaa2babfedffdfcf3` (implementation `ecc67603`):** a fresh
audit of the integrated head selected and delivered the remaining P2 item-modifier
owner closure. The previous generic
`mutate_player_item_modifier_runtime_like_cpp` Session surface is retired from
production; named Player operations now own item-set transitions, level caps, bonus
reset and resolved enchantment actions. Consumers and fixtures were migrated without
changing catalog, aura, packet or publication ordering. The canonical detached/stale
ownership regression, the Player owner regression and the existing item-threat
regression pass; `cargo check -p wow-world` with one job plus format/diff and the
ownership-policy check pass. `validation-v2 final` then passed on 2026-09-13; the
manifest is `target/validation-v2/manifests/20260913T165720.613650Z-3851477-final.json`
and the `wow-entities`/`wow-world` suite completed 3875 tests with zero failures
(one ignored). The delivery is integrated; #584 remains open for its remaining
boundaries. This evidence does not claim #61 effective statistics, TraitMgr consumers
(#524), Creature migration, live QA or DB durability.

The latest bounded functional delivery is #524's **semantic TraitMgr configuration
validation and deterministic login fallback**, integrated by PR #842 as
`995cd77fb48566b972741520986bc41d106470ea` (implementation `add6650a`). The immutable
projection now evaluates C++-aligned conditions, costs, parent/rank rules and granted
entries before publishing Player state, using canonical Player facts. PR #846 now
composes the effective base Trait/`SpecSetMember` SQL hotfix set with WDC4-hash-scoped
removals and fail-before-publication semantics. PR #848 now adds the two locale
overlays (`trait_definition_locale` and `trait_currency_source_locale`) with the
same official/custom composition and immutable capability retention. The issue
remains open for explicit cross-store fail-before-publication coverage,
startup/live DB/relogin evidence, production consumers for locale/effect-point data,
and later spending, mutation persistence and starter-build application. The
legacy Creature writer remains deferred until all AI, combat, movement, script,
persistence and visibility consumers have one owner.

**#486 target-account identity correction — merged in PR #807, integration
`86a0eb97` (2026-09-13):**
`CMSG_QUERY_PLAYER_NAMES` now reads a startup-warmed `CharacterCache` projection
(`CharacterCache.cpp:69+`) containing the target game-account, race, gender, class,
level and delete state, plus the login `account.battlenet_account` relation. The
handler no longer constructs account GUIDs from the querying session. It preserves
the C++ cache-presence gate and overlays the connected target's PlayerDirectory
identity when available. Character create/delete/rename/customize commits update the
same projection after their database result, so no second mutable Player owner or
packet-time full `CHAR_SEL_CHARACTER` query exists. The three focused query cases
(offline/mixed result, missing/failure ordering and connected override) and the two
cache projection tests pass; `cargo check --locked -p wow-database -p wow-persistence
-p wow-world -p world-server`, formatting/diff checks and architecture check/self-test
pass on aarch64 with one Cargo job. Declined-name fields and live undelete/barber
cache updates remain outside the currently represented Rust administration surface;
the implementation is integrated, while the issue remains open until the required
action-specific capture/live acceptance is completed.

**Integrated delivery — 2026-09-12, #787 / PR #792, integration SHA
`d14a9a67194e8013241e5dc837d9e72589aabac0`:** World/Map phase coordination is
implemented and locally accepted on `787-p3-map-driven-session-pass` at `76369bda`.
The correction retains the World
completion acknowledgement through task-owned finalization, session destruction,
BattlePet attachment release and registration retirement, preserves the shutdown
handover, and keeps interrupted effects behind the cross-step barrier.

The clean final gate passed at manifest
`20260912T225625.408381Z-3135200-final.json` (`status: passed`, 1,107.844 s,
one Cargo job): `wow-map` 735, `wow-world` 3,867 and `world-server` 585 tests,
with zero failures, plus the architecture and ownership checks. Guarded live
save/relogin QA also passed with `outcome: passed-restored`, `bot_status: 0` and
`login_save_relog_verified: true` for `TESTBOT1@bot.local`; the original live
SHA `c2a3b461…` is serving again. Two earlier attempts exceeded the guard's
startup timeout because the unoptimized debug candidate needs about 205 s to
load the full data set; they were restored cleanly and are not functional
failures. [The session checkpoint](../architecture/session-578-checkpoint.md#787-resumption-finalization-is-inside-the-world-completion-boundary--2026-09-12)
owns the contract and evidence. This is the selected #584 delivery; older
next-issue instructions below are dated history.

**Architecture plan reconciliation — 2026-09-11, #748, base `5d8c079a`:**
All 46 initially open issues were reviewed and their bodies synchronized with the
master plan. #42 was consolidated into #43, and #58/#59 into #41, retaining their
acceptance before closing them as superseded (`not_planned`). The resulting inventory
is 43 open program issues plus planning delivery #748; no functionality was marked
implemented by these administrative closures. #748 records this delivery's local
validation and publication status separately from historical gameplay evidence.

`PORT_PLAN.md` and GitHub #49 remain the general direction; the architecture documents
below hold the technical ownership and acceptance detail. #133 was closed on 2026-09-09.
#578/#585/#587/#588/#589/#716/#718/#722/#737 are integrated and closed in their bounded
scopes. Do not wait for or reopen #133, and do not use older checkpoint ordering as a
current instruction.

The technical gate remains **#584 core → #583 native/Wasm product → #153 independent
audit**. #584 retains unfinished C0–C4 core work; #583 owns the preserved M0–M4
native/Wasm product and does not block unrelated gameplay macros, while production module
integration waits for required #584 work. The Rust/Wasm/C mixed product is mandatory even
though operator activation is optional. No new micro-issues are planned:
each macro includes its consumers and validation, with file-specific exceptions allowed by
the module policy.

**Persistence ratchet reconciled — 2026-09-13, #584 follow-up:** the first remote check
after P3.1 found a valid out-of-line module declaration unsupported by the inventory
parser and stale snapshot/workflow paths left behind by the #695/#703/#718 file moves.
The parser now mounts such declarations as child source boundaries, with a focused
regression. A fresh inventory of the current `3.4.3` tree records 9,837 exact rows
(7,619 production and 2,218 fixtures), 1,024 production workflows and 1,027 semantic
groups. The reconciliation preserves 986 reviewed workflow contracts under their new
paths, explicitly annotates 38 current identities (including all six #718 quest-reward
adapter identities), and removes 40 obsolete pre-split identities. The full
`session-ownership-check check`, the checked snapshot/policy consistency test and
architecture `check --self-test` pass. This is architecture evidence maintenance only;
it does not claim gameplay progress or close any core gate.

**P3.2 map VALUES publication is implemented — 2026-09-13, #584, `0290ba79`:** the canonical
`Map::SendObjectUpdates` producer now snapshots and clears Player, Unit, GameObject/
Transport, Corpse, AreaTrigger, SceneObject and Conversation values. The world-server
map tick converts those snapshots to owned `UpdateObject` packets and delivers them
after releasing map, metadata and persistence guards, filtering current sessions by
map, instance, in-world state and committed visibility. DynamicObject retains its
existing dedicated session consumer, so no duplicate writer was introduced. The
focused map and world-server publication tests pass. Architecture `check` and
`self-test`, formatting, diff checks and validation-v2 `quick` pass; the manifest is
`target/validation-v2/manifests/20260913T032906.489145Z-3320295-quick.json`. This is
a bounded runtime publication delivery, not client capture/live QA or completion of #584.

**LFG decoder slice integrated — 2026-09-13, #582 / PR #797, integration SHA
`21686375d1fae81876b91f662069a25781ef10b2`:** six C++-anchored client packet
decoders are present in `wow-packet` and issue #582 is closed. The ten focused
decoder cases, the full 738-test `wow-packet` library suite, formatting/diff checks
and validation-v2 quick passed on the aarch64 host. This is wire decoding only:
there is no production handler registration, queue, matchmaking, teleport, reward,
two-socket live QA or full LFG parity claim. The detailed boundary and C++ anchors
are in [the LFG audit](../architecture/lfg-343-audit.md).

**P3.3 respawn/condition phase order — 2026-09-13, #584 / PR #798, integration SHA
`b912c95e33073090a1ef9ea0e09a2b260856389e` (implementation `c7daa069`):**
the canonical map tick now runs `ProcessRespawns` and
`UpdateSpawnGroupConditions` after the admitted session pass and before object
visitation, matching `Map.cpp:682-693` within the split-tick design. The phase is
restricted to the `MapTickParticipantLikeCpp` key/incarnation list so a map
recreated during the released session phase cannot receive its predecessor's work;
the `MapManager.cpp:287-318` delayed-update barrier and existing asynchronous DB
queue/persistence fence remain unchanged. A focused replacement-incarnation
regression plus the six existing spawn-condition tests, world-server check and
format/diff checks pass. Validation-v2 `quick` passed with one Cargo job in 7m03s;
manifest: `target/validation-v2/manifests/20260913T042148.632508Z-3355103-quick.json`.
The legacy Creature writer, nearby-cell visitation and other unrepresented
`Map::Update` phases remain outside this bounded delivery. The integrated delivery
has no live client, capture or DB/restart/relogin QA evidence.

**P3.4 delivered under #584 — nearby-cell ObjectUpdater production wiring, 2026-09-13:**
the production map tick now consumes one incarnation-scoped, deduplicated nearby plan
from in-world Players, viewpoints, represented far combat/aura/summon references and
active non-Players before selected `ObjectUpdater` family consumers. This follows the
C++ source order and visitor boundary (`Map.cpp:695-754`,
`GridNotifiers.cpp:258-264`): Players and Corpses stay out of the visitor set and
transports retain their separate full loop. The selection is implemented in
`wow-map/src/map/object_update_selection.rs`, wired through
`ManagedMap::update_after_sessions...`, and selected by
`world-server/src/runtime/map_tick.rs`; direct callers retain the whole-store seam.
Focused positive/negative tests prove nearby inclusion, out-of-cell exclusion and
split-tick consumption; architecture, formatting, `wow-map` tests and
`world-server` composition checks pass. Creature/Pet source activation uses the
canonical per-source `m_SightDistance` for inactive Creatures/Pets, while active
objects and Players use the map visibility range, matching
`WorldObject::GetGridActivationRange` (`Entities/Object/Object.cpp:1433-1450`).
Unsupported or missing source records fail closed. Creature AI/combat, scripts,
transport passengers, relocation fanout and live client/DB parity remain separate
#584 boundaries.

**P3.5 activation range — 2026-09-13, #584, implementation `7214fb68`:** the
production nearby-cell selection now resolves each center's C++ activation radius
from its canonical map record. Inactive Creatures/Pets use their existing
`sight_distance` field; active objects and Players retain `Map::GetVisibilityRange`.
The focused regression covers both branches, and the existing P3.4 nearby-selection
regression remains green. This is a finite phase-fidelity correction; it does not
claim the still-unmodeled Player cinematic activation override or any live client,
capture, DB/restart/relogin or Creature AI/combat acceptance.

**P3.6 Player cinematic activation radius — 2026-09-13, #584 / PR #804, integration
SHA `6157a0916899bb341b3f4a271ea69333a7cf61f6`:** the nearby-cell
selector now matches `WorldObject::GetGridActivationRange` for an active Player
cinematic: after the represented camera cursor is selected, it uses
`max(DEFAULT_VISIBILITY_INSTANCE, Map::GetVisibilityRange)`; beginning a sequence
before its first camera and ending it retain the map range. This follows
`Entities/Object/Object.cpp:1433-1450` and `Entities/Player/CinematicMgr.h:38-45`.
The implementation reads the canonical Player-owned cinematic state and does not
create a second owner or clock. The current Rust state has no FlyByCamera store, so
camera-row lookup and cinematic movement remain outside this selector. The focused
positive/negative regression is `grid_activation_range_uses_instance_distance_for_active_player_cinematic_like_cpp`;
the existing inactive Creature/active-object regression remains green. No live
client, capture or DB/restart/relogin evidence is claimed, and Creature AI/combat,
scripts, fanout and relocation notification effects remain separate #584 work.
The integration candidate passed `VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2
quick --base origin/3.4.3` with one Cargo job in 11.519 seconds; manifest:
`target/validation-v2/manifests/20260913T061658.691366Z-3417098-quick.json`.

**P3.7 Creature relocation visibility fanout — 2026-09-13, #584 / PR #814,
integration `a3e970635c2df891f284fa6ac0b083b4c2659473` (implementation
`a130d9da`):** the canonical map now reuses the same Player source construction for
`ObjectUpdater` and `ProcessRelocationNotifies`, including viewpoints, far PvE
combat creatures, aura casters and summons. Relocation marking resolves each center
through `grid_activation_range_for_guid_like_cpp`, preserving the C++ source-specific
radius. `MapManager` consumes `CreatureRelocationVisibilityPlan.player_visibility_updates`
and coalesces one `PlayerVisibilityRefreshIntentLikeCpp` per affected Player before
the existing deferred Session rail. Residence revision, incarnation, viewpoint,
backpressure and delivery outside the map guard remain enforced by the existing
owner/Session path. The focused manager, map and relocation suites plus
`validation-v2 quick` passed on the aarch64 host in 36.36 seconds with one Cargo job
(`target/validation-v2/manifests/20260913T094251.016189Z-3544489-quick.json`). The
architecture check's physical ratchet passes, while the unchanged hotspot ceilings
still report their pre-existing drift; no baseline was regenerated. This slice does
not claim directed creature CREATE/DESTROY packets, AI/combat, scripts, FlyByCamera,
live client capture or DB/restart/relogin evidence.

**P3.8 map object lifecycle visibility intents — 2026-09-13, #584 / PR #818, integration
`6ef133437fc26e4e27182be084e50246ef48d375` (implementation `162c1a9d`):** the
canonical `Map::AddToMap` and `Map::RemoveFromMap` paths now mark nearby in-world
Players with `ObjectNotifyFlags::VISIBILITY_CHANGED` while the source is still
attached. This supplies the missing recipient selection for the existing deferred
visibility rail: the map tick consumes the flag, `MapManager` coalesces a
residence/incarnation-checked intent and the Session recomputes its real client
ledger after map guards are released. The helper covers already-in-world, Creature,
GameObject and generic map-object admission paths and removal before record erasure;
it sorts/deduplicates recipients, excludes a Player from its own walk and never
delivers a packet or awaits under the map mutation.

The focused add/remove map tests pass and prove the canonical Player flag mutation
and source removal. This closes the unmarked-recipient gap only: exact directed
CREATE/DESTROY bytes, transport-specific fanout, client capture and live
DB/restart/relogin acceptance remain separate #584 gates. The outcome telemetry
still names the direct synchronous C++ visibility call as a runtime gap because the
Rust equivalent is intentionally deferred through the existing phase boundary.

**P3.9 directed Creature DESTROY — 2026-09-13, #584 / PR #820, integration
`62c1369f4e49200b6f6d7605b0bc7de5caeace7c` (implementation `8ab62574`):** an ordinary
Creature removed from an active map now produces a typed directed-destroy result
while it is still attached. The canonical map captures nearby in-world Players,
excludes the direct charmer and binds the result to the map incarnation; the
world loop publishes it after releasing map guards through the durable Session
mailbox. The Session validates login, map/instance, incarnation and its
`client_visible_guids_like_cpp` ledger before atomically removing the GUID and
emitting one `SMSG_UPDATE_OBJECT` destroy. Durable ordering keeps this destroy
ahead of a coalesced visibility refresh. The focused map test, two positive/negative
Session tests, mailbox fence test, `cargo check` for `wow-map`/`world-server`,
formatting and diff checks pass with one Cargo job. CREATE, Pet, corpse/transport,
capture and live DB/restart/relogin acceptance remain separate gates; #584 stays
open for its remaining measured macrodeliverables.

**P3.10 Player/Unit VALUES fanout — 2026-09-14, #584 / PR #871 + #873, integration
`bd5b13d4b885f1887b95c654e2ebdb13f6c70c49` (implementation `30157c1f`, correction
`5586997d`):** the
Session now consumes the Player and Unit snapshots produced by
`Map::SendObjectUpdates` after map guards are released. Unit field mutations enqueue
their canonical in-world object, self updates retain owner/active-player fields, and
observers receive receiver-filtered Player/Unit values only when map, instance,
incarnation, phase, range and `HaveAtClient` fences hold. Creature/Pet updates use
the same committed visibility path; packet bytes are built before delivery and no
map lock crosses a send. The generic map rail excludes Player/Unit so the filtered
Session path is the sole publisher for those families, and the Session rechecks
the admitted `MapKey` before publishing after the map lock is released. Three
`wow-world` fanout regressions, 27 `wow-entities`
unit regressions, the production `world-server` check, architecture checks and
`validation-v2 quick` pass at
`target/validation-v2/manifests/20260914T072651.033982Z-371881-quick.json`.
Shared-raid field flags, exact packet captures, complete CREATE/Pet/corpse/transport
coverage and live DB/restart/relogin acceptance remain separate #584 gates; #584
stays open and the next macro still requires a fresh responsibility audit.

**Transport VALUES visibility projection — 2026-09-14, #584 / PR #876, integration
`ed92d14f173ee2be2332b242576eb603aaff58f4` (implementation `0c8e0f69`):** the
map `Transport` VALUES route now uses the C++ `Player::m_visibleTransports`
membership separately from ordinary `m_clientGUIDs`. The Session publishes that
generation-safe set through `PlayerSessionRegistrationLikeCpp` and
`PlayerRuntimeRecipient`; the world-server delivery adapter selects it only for
MO transport GUIDs, while all other object families retain ordinary committed
visibility. The Session command consumer applies the same gate, so a stale or
undeliverable Transport update is dropped before packet publication. The focused
world-server fanout regression proves visible and non-visible recipients, and the
wow-world regression proves absent, present and cleared Session membership. The
world-server check, focused tests, formatting/diff checks and full architecture
check pass. This closes only Transport VALUES fanout; Transport CREATE/DESTROY,
passenger lifecycle, exact captures and live DB/restart/relogin evidence remain
separate #584/#63 gates.

**P2 item-bonus writer retirement — 2026-09-13, #584 / PR #816, integration
`db1250767090a5c951dae96ad6c2a2d5b24873ff` (implementation `948b7ea9`, ledger
ratchet `d5d21a60`):** the last generic `with_bonuses_mut_like_cpp` closure no
longer exposes `&mut PlayerItemBonusStateLikeCpp` from Session. The state-only
resolved-effect tail of TrinityCore `Player::_ApplyItemBonuses`
(`Player.cpp:7688-7975`) and `Player::ApplyEnchantment`
(`Player.cpp:13058-13389`) is now
`PlayerItemModifierRuntimeStateLikeCpp::apply_enchantment_effect_action_like_cpp`
in `wow-entities`. Catalog lookup, item admission, spell casts, aura publication
and packets remain in `wow-world`; unsupported/deferred actions remain explicit
and are not falsely claimed as applied by the owner. The Player hotspot ceiling was
updated to the reviewed live aggregate (`15468` production, `13122` test lines);
the physical ratchet passes and the four unrelated logical hotspot drifts remain
unchanged. Evidence: 14 `wow-entities` modifier tests, 16 world-entity tests, 129
player-item tests, `cargo check --locked --tests -p wow-world`, formatting/diff
checks and validation-v2 quick in 8.53 s with one Cargo job
(`target/validation-v2/manifests/20260913T100604.990747Z-3562620-quick.json`).
No live runtime, capture, DB/restart/relogin or full item-stat parity claim is made.

**#524 skill catalog and TraitMgr projection — 2026-09-13, sequence implementation
`0192bac3` plus hotspot follow-up `de4e114e`, TraitTree overlay `9cca8f2b`, final-removal delivery `68fe676d`, generic TraitMgr index `eee72efd`, combat/class index `4ee7391e` and persisted-entry validation `c09a1f42`, integration `4e3ad8f0`:** the production bootstrap now keeps the C++ table stages independently observable: `SkillLine` → WDC4/official/custom `SkillLineAbility` → WDC4/official/custom `SkillRaceClassInfo` → WDC4 `TraitTree` plus WDC4/official/custom `SkillLineXTraitTree` projection. Official-then-custom order and fail-before-publication behavior remain explicit. The link store retains its WDC4 table hash and applies final table-scoped `RecordRemoved` statuses before projection. PR #830 adds the immutable `TraitSystemID -> TraitTreeID` projection and Generic config validation; PR #832 adds the C++ `ChrSpecialization -> ClassID -> _skillLinesByClass` combat projection and moves TraitMgr construction after effective `SkillRaceClassInfo` composition; PR #834 rejects missing or over-ranked persisted node entries at the existing Session authority boundary. Focused `wow-data`, `wow-world` and `world-server` regressions plus the physical architecture check pass. This remains a bounded startup correction, not complete TraitMgr parity: node topology, cost/condition/loadout consumers, final cross-store orchestration and startup/live evidence remain open.

**#524 TraitMgr node-graph projection — PR #836, merge `0fca1020`, implementation `d9770755`:**
the catalog owner now loads the C++ node, group, edge, cost, condition and loadout DB2
relations once, builds deterministic immutable indexes for tree nodes, node entries,
groups, parent edges, costs, conditions and specialization loadouts, and returns the
single `TraitNodeEntryStore` capability used by the world bootstrap. When the graph is
available, the existing Player trait-config authority rejects a persisted node/entry pair
that is not linked to one of the config's resolved trees. Focused `wow-data` and
`wow-world` regressions, `world-server` check, formatting/diff and physical-source checks
pass. This is still a bounded projection: the index does not yet implement currency
ownership, condition evaluation, spending, starter-build application, complete
cross-store failure orchestration or live startup/DB/relogin evidence; #524 remains open.

**Group state application is integrated — 2026-09-11, #743 / PR #750,
`77df8194` (implementation `9e6767bb`):**
`GroupRegistry` stays the single authority and every state-bearing group command now
either reaches its member or records a delivery obligation that the member converges
on in a dedicated driver phase. C++ never needs this fence because
`Group::RemoveMember` (`Group.cpp:550`) and `Group::Disband` (`Group.cpp:713`) call
`Player::SetGroup(nullptr)` (`Player.cpp:23440`) inside the operation itself. The
delivered contract, its exact code targets and its retained boundary are in
[the refactor completion plan](../architecture/refactor-completion-plan.md#entrega-local-aceptada--9e6767bb).
Evidence on this aarch64 development host with one Cargo job: `cargo test -p wow-world
--lib` 3,841 passed, `cargo test -p wow-social --lib` 80 passed, the three
production-linked `wow-world` integration targets passed, and `session-ownership-check
check --syntax-only`, `check_architecture.py check`/`self-test`, `cargo fmt --all --
--check` and `git diff --check` passed. The reviewed inventory deltas are exactly the
new surface and the `handlers/loot/handlers.rs` → `handlers/group/commands.rs`
relocation. **No live runtime, capture or DB/restart/relogin evidence exists for this
delivery**; queue saturation is exercised on the real bounded channel in the session
composition, not on a running server.

**Reputation encapsulation is integrated — 2026-09-11, #735 / PR #751,
`9ec0e855` (implementation `c68f8e16`):** the canonical Player
owns `PlayerReputationStateLikeCpp`, the equivalent of C++ `Player::m_reputationMgr`
(`Player.h:3116`), together with the state-changing invariants that need no catalog.
`ReputationMgrLikeCpp` became a borrow of that state, so the per-operation reconstruction
and the whole-aggregate write-back through `gameplay_state_mut()` are removed from the tree
rather than renamed; catalog resolution, rank/spillover rules and packet construction stayed
in `wow-world`, and the shared value types moved to `wow-constants` with a
`wow_data::reputation` re-export so no forbidden dependency was added. Every recorded
consumer migrated: load/hydration, login publication, reputation flags, quest rewards, spell
effects, kill rewards, the save projection and the save acknowledgement, with `need_send`
and `need_save` still independent. The delivered contract is in
[the refactor completion plan](../architecture/refactor-completion-plan.md). **No live
runtime, capture or DB/restart/relogin evidence exists for this delivery.**

**Talent encapsulation is locally accepted — 2026-09-11, #752:** the canonical Player
owns `PlayerTalentRuntimeState` with private fields and the named transitions C++
performs on `_talents`/`_specializationInfo`, each bounding its group index and glyph
slot. The session's generic talent-runtime closure is private to its owner module:
persistence load and the talent-reset COMMIT call named transitions instead. The
catalog, spell/aura and packet work stayed in `wow-world`. C++'s per-row talent
`State` remains unrepresented in Rust; that is a pre-existing boundary this delivery
records rather than changes. **No live runtime, capture or DB/restart/relogin
evidence exists for this delivery.**

**Spell-runtime encapsulation is locally accepted — 2026-09-11, #754:** the canonical
Player owns `PlayerSpellRuntimeState` with its fields closed to the Player module and
the transitions C++ performs on `m_spells` and `m_overrideSpells`. The owner now
enforces the rules callers wrote by hand, and the session's generic closure is scoped
to `session::spell_state`. The completeness flags keep their authoritative-empty versus
unhydrated meaning. **No live runtime, capture or DB/restart/relogin evidence exists
for this delivery.**

**Quest-state encapsulation is locally accepted — 2026-09-12, #756:** the canonical
Player owns `PlayerQuestGameplayState` with its fields closed to the Player module and
the transitions C++ performs on `m_QuestStatus`, `m_RewardedQuests`, `m_DFQuests` and
`m_seasonalquests`. Fourteen single-transition callers use named session operations; the
five catalog-driven objective walks and the quest-slot compaction keep the borrow as a
recorded projection whose exit condition is #41's objective-progress contract. **No live
runtime, capture or DB/restart/relogin evidence exists for this delivery.**

The remaining P2 operations — taxi, collections, item equipment sets, cast state,
difficulty, trait configs and the two canonical access helpers — then lead into P3
runtime/lifetime/private-hecs and P4 semantic/physical work under #584. Item-modifier
state ownership is delivered; remaining item work is limited to its catalog/effect
consumers and any separately reproduced gameplay gap.

Finite hecs V2 conformance passed within its recorded laboratory limits. Production `hecs`
and Wasmtime are not installed in this base. The dated six-clock trace and the 31 oversized
file measurement are evidence boundaries, not new exhaustive measurements. The historical
363-test analyzer result is not a new test run.

**Quest reward is integrated — 2026-09-11, #718:** P1 established one durable character
transaction for the represented reward participants. C++ `Player::RewardQuest`
(`Player.cpp:14625`) applies grants in memory and reaches the database through its closing
`SaveToDB(false)` (`Player.cpp:14867`); the Rust contract and departures are in the
[quest reward operation contract](../architecture/quest-reward-operation-contract.md).
Implementation is `78276ddf57463fc0f568c1c4bcf84d619af68cad`, integrated with its recorded
local evidence. Evidence on aarch64, Rust 1.98.0 and one Cargo job: `cargo test -p wow-world --lib quest`
427 passed; `./tools/validation-v2 final --base origin/3.4.3` passed at that SHA with a clean
tree and a manifest verified green, covering the 363 analyzer tests, the exact ownership
surface, the physical and hotspot ratchets and the server build. The three moved ceilings
and the reviewed ownership surface each carry their measurement. No real database write,
restart or relogin QA was performed; live durability and the unimplemented participants
remain open port work.

The records dated before this reconciliation are retained below as historical evidence.
Their former next-step, issue-state and test-count language does not override this block.

**Historical #716 delivery record — 2026-09-10, reviewed integration `aff42a51`:**
the user approved starting the architecture repair program. #716 restores exact
ownership-provenance acceptance after the mechanical module passes. At that base,
the general architecture check and self-tests pass; the syntax-only Session check
fails on two relocated fixture scopes and one still-live canonical/legacy bridge
whose parent import is lost by the analyzer. Physical terminal acceptance also fails:
31 files exceed 2,000 lines, while migration ceilings still pass. These results do not
reopen the integrated, closed #587/#588/#589 scoped gameplay deliveries.
Implementation and acceptance of #716 are tracked in the
[architecture plan](../architecture/modularity-and-ecs-plan.md#architecture-program-state--2026-09-11).
**#716 is locally accepted**, with implementation commit
`6ae62d73a9f910ebd664d82e331520b836a73c77`: 363 analyzer library tests, syntax-only
ownership, architecture check/self-test and validation-v2 quick pass. The 65 earlier
bridge records are preserved after the reviewed relocation; seven existing dual
references are now inventoried, giving 72 exact records. Test commands ran on the
working candidate at `aff42a51` and its verified V2 manifest records `dirty: true`;
the tested code/policies were committed unchanged. The
[technical refactor plan](../architecture/refactor-completion-plan.md#7-registro-historico-conservado)
records commands, source identity and the remaining boundaries. No push, merge,
server build, deployment, live QA or exhaustive persistence rescan was performed
for this tooling delivery. Physical terminal acceptance still fails at the same
31 files. The former ordering text is historical; current #584 sequencing is #743,
then #735 without a hard dependency, followed by remaining P2/P3/P4 work.

**Second physical pass closed — 2026-09-10, integration `a4a9073e`:** fourteen
deliveries between #685 and #711 finished the physical track outside the curated
hotspots. #685 and #687 moved the last inline `mod tests` blocks out of oversized
production files; #689, #691, #693, #701, #705, #707, #709 and #711 divided 27 files by
item family, by method group or by protocol family; #695, #697, #699 and #703 grouped
flat sibling module families into directories, by shared prefix and then by module kind,
taking the `wow-persistence` root from 53 entries to 26, `wow-database` from 64 to 40 and
`world-server` from 46 to 25. The physical ratchet's cohesion reviews fell from 111 to
63. Every delivery proved its item or method multiset identical before and after, kept
each crate's test count unchanged, and passed
`./tools/validation-v2 final --base origin/3.4.3` at the committed SHA.

That pass retained four files outside the hotspots above the 1,000-line review
budget: `session_tests.rs` (5,756) and its 197 shared helpers, plus
`statements/character/identities.rs` (1,789), `statement_def.rs` (1,627) and
`player/lifecycle_adapter.rs` (1,609). Forty-four more sat inside the eight curated
hotspot aggregates. #716 corrects the earlier claim that a trait impl or match must
change its type to delegate to private modules; those shapes and fixture sharing do
not establish terminal exceptions. This corrects
the earlier claim that relocation was exhausted for "the `map_manager` pair": #705 divided
its 2,069-line `impl WorldCreature` into seven submodules and #711 divided the 2,191-line
root into five, so both now sit at 130 and 255 lines. The measured limits behind the rest
are recorded in
[the module-design guide](../architecture/module-design-guidelines.md#what-a-physical-division-cannot-reach),
including #713's measurement that dividing the loot and quest fixture roots costs their
hotspot aggregates +63 and +43 test lines at the cheapest wiring available. What remains
includes both semantic extraction and justified private decomposition under #584
C0-C4. This dated record predates #133's closure and does not close the technical #584 gate.

**Historical physical decomposition pass — 2026-09-09, integration `df94f231`:**
#589 and its sub-issues #590-#595 are merged. Sixteen deliveries between #634 and
#664, following the Session-root separations of #603-#632, then completed the
physical half of the module-design policy: 184,295 lines of oversized roots
reduced to 4,972 across the QA bot, the architecture checker, `wow-entities`,
`wow-data`, `wow-packet`, `wow-loot`, `wow-map`, `wow-database`, `wow-world`,
`capture-diff`, `wow-instances`, `wow-ai`, `wow-network` and `wow-recastdetour`. Every delivery proved its top-level item
surface identical before and after, kept its crate's test count unchanged, and
passed `./tools/validation-v2 final --base origin/3.4.3` at the committed SHA;
sixty-seven of the hundred reviewed ceilings are now at or below 2,000 lines.

At that revision, 116,693 lines remained above 2,000 lines in 33 files: 29 sat
inside the eight curated runtime-ownership hotspots or the `map_manager` pair,
one was a vendored C++ translation unit and three were capture fixture shell
scripts with their own runtime-authorization gates. The earlier conclusion that
all relocation was exhausted is superseded by the 2026-09-10 review above. The
hotspot ratchet measures the module aggregate, so splitting a file inside an
owner at its baseline makes the aggregate grow; and splitting a bridge
function's context removed a row from the 65-row bridge inventory while the
build and all 3,822 `wow-world` tests stayed green (#662, reverted). That count
loss did not prove retirement or make file separation invalid; #716 repairs the
parent-import provenance gap. What remains includes semantic extraction and
justified physical decomposition under #584 C0-C4, with the per-owner entry
conditions recorded in
[the modularity plan](../architecture/modularity-and-ecs-plan.md#historical-physical-decomposition-pass--2026-09-09).
This dated record closes neither the technical #584 gate nor any current gameplay macro.

**Historical delivery — 2026-09-08, integration `8c47af95`:** #586 is merged and
#585 closed. #587's represented spell-acquisition boundary is implemented locally
on its own branch (original `43c4e801`, subsequent QA/evidence through `c88603ee`).
The branch is now rebased onto #588 commit `328b721f`; paired trainer and controlled
ordinary-effect acquisition/save/relogin pass on runtime source `3a634630`.
Combined final at `b48a1cfb` passes 14 commands, 5,824 Rust tests (two ignored) and
20 physical-policy tests, with zero failures. Local scoped acceptance is complete;
publication is pending. The earlier stationary-login failure remains recorded.
The user approved **#588 — deferred player visibility publication** as that
concrete prerequisite. It connects the existing map notify phase to retained
Session delivery, with exact incarnation/residence admission and the actual
client visibility ledger. No new clock or whole-map rewrite is selected.
Implementation and acceptance are tracked in
[the #588 checkpoint](../architecture/deferred-visibility-588-checkpoint.md).
#588 and #587 have local scoped acceptance. Their content is carried in full by
the #589 branch, which stacks on both, so a single PR integrates the three
deliveries; the user authorised that integration on 2026-09-08.
#589, the represented Player cast-request lifecycle, is **implemented and
locally accepted**, described in
[the existing architecture plan](../architecture/modularity-and-ecs-plan.md#historical-delivered-design--589-2026-09-08)
and recorded in [its checkpoint](../architecture/player-cast-589.md), which owns
the executed evidence, the represented value limits of the publication payload
and the deviations deliberately left open. Its six sub-issues #590-#595 cover the
functional contract and consumers, the 3.4.3 payload and publication, ownership
and physical organization, the acceptance bot, the integral regression campaign,
and closure. The earlier "paused, uncommitted working tree" state is superseded.
The current technical gate is #584 core → #583 → #153; #133's tracker closure is
already recorded above. #589 does not complete all spell gameplay.
The dated pre-merge account below remains historical
evidence, not an instruction to reopen #585.

Derived C++ and Rust trainer purchase/rejection/save/relogin pass, as does the
explicitly synthetic30798→6197 cast fixture with unchanged stock data and verified
effective SQL. Full trainer/cast windows retain documented order/metadata differences;
scoped learning and logout bytes match. Current #587 evidence:
[spell acquisition](../architecture/spell-acquisition-587.md).
#585's scoped acceptance and retained parity limits
are recorded below and in its checkpoint; do not reopen it from pre-merge status text.
The following dated candidate narrative is historical, not an instruction to repeat
completed work or a new whole-port completion/parity claim.

Local #585 candidate `ccf5f84d` passed final validation on 2026-09-07. After renewed
explicit authorization, guarded normal save/relogin QA passed and the original
world-server executable was restored and serving. The earlier permission blocker
is resolved. The subsequent orderly transport-disconnect/save/relogin scenario
also passed with QA tooling `55ec9a8b`, and the original server was restored.
Candidate `bf884aec` subsequently passed final validation and guarded pending-transfer
disconnect/save/relogin QA, after two separately committed fixes for realm routing
and retained-destination normalization. Original character location and server binary
were restored and verified. Applicable fresh C++ capture comparison remains outstanding.
A temporary derived C++ executable has now been exercised on an isolated database copy;
its four explicit reference corrections are documented in the checkpoint.
Candidate `07698639` passed final validation and 34 release production-integration tests.
Its installed LogoutComplete now matches C++ in bytes and realm connection in a fresh
strict one-packet comparison. The full paired normal-logout capture remains divergent:
instant-logout response/order and missing side effects. The same starting fixture exposes
a reputation flag mismatch; neither failed comparison is waived. The old capture wrappers
target PM2, not the current systemd deployment. These bounded passes are not full parity
or issue closure; details and private evidence identities are in the checkpoint below.
Subsequent final QA corrected compressed faction arrays and repeated map-entry reputation
initialization; installed `48b3729b` preserves the five affected faction rows. Its next
failure exposed persisted bows being rejected by a new-acquisition ambiguity policy.
That bounded load correction and fresh-C++-driven portal reason/orientation corrections
are implemented and installed candidate `5f5e225f` passed all three paired live
scenarios (normal logout, EOF disconnect and pending-transfer disconnect, each with
relogin and six retained persistence projections). Release production integration
passed 34 tests. Scoped LogoutComplete, InitializeFactions, TransferPending and
NewWorld comparisons match C++; full logout/portal windows remain divergent.
Final validation first exposed a concurrent ambient-trace test; the test-only
isolation repair passed 358 database tests and 20 concurrent trace-suite repetitions.
Candidate `b8895373` then passed final validation; the reviewed exhaustive ownership
check also passed with all 10,106 persistence references and 1,029 semantic groups.
This is bounded local acceptance evidence, not publication, full action parity or
closure of #585/#584.
The subsequent bounded scope review classifies inherited admission and broader
combat/aura/group/object cleanup differences as retained #584 work, not additional
#585 implementation prerequisites. Included-path blockers were corrected and tested.
The local delivery is ready for publication/review with those limits disclosed;
no full-action parity, push, merge or next implementation family is claimed.
PR #586 is published. Its two cross-socket ordering findings are corrected. Immediate
and delayed transfer use the existing realm writer fence; LogoutComplete uses the
instance writer fence. The existing session update becomes async without a new
owner/task. Renewed final validation at `37ef2a93` passed 6,162 tests with zero
failures. Release executable `5e48162a` passed paired normal logout/relogin and
pending-portal disconnect/relogin on the isolated database; the four scoped
logout/transfer packet comparisons passed. The original runtime stayed intact.
The user authorized integration without another review request; #585 remains open
until PR #586 merges. Further #584 work and the next-family selection remain separate.
Fresh C++ EOF and pending-transfer bot scenarios passed; C++ then crashed on stopping
after the portal scenario. That reference shutdown is not recorded as successful.
Current #585 contract, evidence and remaining boundaries:
[session finalization](../architecture/session-finalization-585.md).

**Historical capability-audit base:** 2026-08-09 · `3.4.3` @ `42977e9a`, including issue #26's
bounded creature-spell P1 wire/lifecycle acceptance and login faction hydration.
**Architecture/plan review:** 2026-09-05 · local #578 branch @ `93e4002a`.
The latter is not a new whole-port parity audit or a deployment claim. Undated subsystem tables,
counts and source locations below belong to the historical audit unless a later bounded note
explicitly updates them; recheck them against current code before selecting implementation work.

The guarded C++ and Rust `15691` evidence was recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`; strict diff is CLEAN at 2/2 packets and
`verify-required creature-spell-casting` is CLEAN.

This document replaces the drifting status snapshots in `_INDEX.md` (2026-05-01, "5–15%"),
the `MIGRATION_ROADMAP.md` §3 inherited table (which tells you not to trust it), and the
old append-log, now referenced through `current-session-handoff.md`'s Git-history pointer.
Its historical capability matrix is **grounded in
the named code audit down to subsystem/subdependency level**, not in what prior docs or the
inventory TSV claim. Architecture decisions: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md).
Forward plan: [PORT_PLAN.md](PORT_PLAN.md). Bugs found in already-shipped code:
[EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). Source-verification issues #50–#64
and index #65 retain traceability under plan ledger L26, not authority for their original
diagnoses or finding counts. Recheck selected residuals against current Rust and exact C++
sources; preserve only independently supported behavior and evidence in the
[C++ findings](../audits/cpp-parity-findings.md).

Repository refactors are governed by
[`docs/architecture/ownership-and-boundaries.md`](../architecture/ownership-and-boundaries.md):
one mutable owner per concept, private modules before crates, explicit mirror retirement, and
executable Cargo/handler-contract guardrails.

The approved [module design guidelines](../architecture/module-design-guidelines.md) now require
both semantic boundaries and physical source/test navigability. Remaining monolith decomposition
belongs to #584 C0–C4; #583 applies the same policy to its own product, including Rust/Wasm/C.
The physical ratchet implemented above `8f5caedc` now covers repository source/tests/tooling,
with 103 initial legacy non-growth ceilings and an independent terminal mode. The first Rust
split above `d3f5c20c` reduces the persistence facade from 4,513 to 544 lines, preserving root
public contracts in private operation modules and retiring its legacy ceiling (102 remain).
The following adapter split above `1e6b7c40` reduces its 4,957-line root to 1,608 lines,
with private statement/row modules and responsibility-specific tests (101 legacy ceilings
remain). Its reviewed exhaustive inventory and matching policy are reconciled in the owning
checkpoint, including stale pre-catalog records and preservation of currency caller provenance.
These are physical decompositions, not closure of broad lifecycle capability cohesion. The existing
logical-owner guards remain. Migration PASS is not terminal acceptance: the legacy files
still need their stated splits or concrete bounded exceptions before #578 closes.

The following local C4 guard repair above published `9cd1da41` preserves scoped callable
reexport/import-chain provenance without exporting private aliases across packages. Unresolved
generic outputs conservatively retain known argument provenance instead of silently losing pools;
329 checker tests pass. Its reviewed exhaustive snapshot contains 10,084 rows: 53 explained
false aliases removed, 21 references added (20 deliberately conservative), with no direct SQL
operation removed and a byte-identical policy. Exact deltas and limitations live in the
owning checkpoint. The parser root's ceiling shrinks from 21,248 to 21,030 lines, with small
private provenance/test modules; this is not terminal physical or semantic closure.

The subsequent C4 repair above `bca3885f` corrects implicit child resolution beneath
explicit `#[path]` file mounts in both source walkers. Five compiler-backed fixtures
and the full 334-test checker suite pass. The exhaustive inventory comparison also
passes with the same 10,084 persistence rows and byte-identical snapshot/policy;
no ceiling or exception is widened. This closes the bounded module-path defect,
not C4 or production C0/C3 phase coordination; details live in the owning checkpoint.

The C0 cut above `36d0ccbf` adds the exact C++ world/map packet-filter contract to
`wow-handler`, tested across all processing classes and Player residences in dev/release.
It corrects the misleading Inplace/socket-thread description, without changing dispatch.
The current independent Session and map loops still lack the required phase coordination;
the integration cut and queue/incarnation/barrier obligations are recorded in the owning
checkpoint. A passing pure filter test is not production scheduling or C0 acceptance.

The C0/C1 residence cut above `590b93f0` checks canonical index, generation, backing
Player identity/container and world/map binding together. Existing residence queries
now fail closed on inconsistent state; the checked API distinguishes those errors
from missing and replaced owners. Public production-library lifecycle and Session
login/save regressions exercise the change. This is invalid-state admission hardening,
not a new scheduler, storage migration or complete lifecycle/durability acceptance;
exact tests and remaining boundaries live in the Session checkpoint.

The following C0 queue cut above `bdae6204` preserves unselected packets in one
bounded Session FIFO when a handler future is cancelled, without replaying the
selected handler's partial effects. C++ LockedQueue filters only the head and stops
when it is ineligible; this constrains the pending world/map integration. Current
normal dispatch order and registry remain unchanged; phase filtering and coordination
are still not enabled. The checkpoint records focused tests and the reviewed small
logical-owner growth, not a physical monolith waiver or C0 completion.

The subsequent C0/C3 rename cut above `ab1cdab3` connects owned read/commit stages
to the production Session driver: the handler submits and returns, Session admits ready
reads and presents confirmed commit results, and composition drains submitted commits
before disconnect save. Workers cannot mutate Session/Player or send packets. Cancellation
of the drain retains its handles; worker failure or a shutdown timeout is not clean
quiescence or rollback proof. This changes asynchronous scheduling while preserving Rust's
commit-result-before-response fence. Global World/Map phase coordination, the first complete
Player lifecycle/save vertical, resource/backpressure and live/durability acceptance remain
open. Exact validation and source anchors live in the owning Session checkpoint.

The following cut above `b35bba96` reproduces and removes blocking capacity waits
from ready rename callbacks. Session now retains owned channel-send futures for the
entire ready batch, preserving FIFO against later packets without repeating commits.
Callback coordination/transport lifetime moves into the private Session driver; read/
commit application stages stay transport-free. Worker loss stops read admission and
retires Session rather than pretending completion was a normal DB rejection. General
synchronous sends and immediate admission-error responses remain explicit C0/C3 work;
this is bounded callback evidence, not whole-runtime backpressure or durability acceptance.

## Historical architecture and execution checkpoint — 2026-09-05

The former approved implementation unit was **#578 with draft PR #579**, under #133. Internal
commits/checkpoints did not create micro-issues, micro-PRs or a new approval gate. The current
contract-led plan, exact inventories, acceptance evidence and remaining boundaries now live in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md). #153 verifies the
complete result; it is not an implementation owner for already-known cuts.

The [explicit reanalysis cadence](../architecture/modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication)
is conformance before production storage migration, then review of the first real C1/C2 vertical
with C0 execution evidence before replicating its design. C4 checks the complete #578 balance
before #583 production integration; #153 audits both merged macros. After architecture, review
each selected gameplay macro just in time and perform the fresh whole-port planning pass at
#47/M6.2. No checkpoint introduced another routine approval, issue or PR. The tracker closure
of #133 is recorded in the current block above; the technical gate remains #584 → #583 → #153.

At the reviewed local HEAD, canonical `wow_entities::Player` owns the migrated gameplay families
and `wow_map::MapManager` coordinates its generation-checked active/detached lifetime. The former
whole-Player Session write-back and ObjectAccessor Player-copy paths are retired. This is real
ownership progress, not proof that Session is already a thin shell: gameplay orchestration,
catalog/service retention, broad mutable Map access and runtime bridges were then #578 work;
the remaining technical ownership now belongs to #584.
SQLx isolation under #169 is closed; terminal capability cohesion and the complete Session/runtime
boundary still need evidence. Closing #252/#297/#378 proved their stated directory, transport and
classification cuts; those dated results do not alter the current #133 tracker closure.

The next cuts are selected by complete operation contracts and their deletion conditions, not
field counts. Distinguish implementation, production-path integration and parity evidence. Use
focused checks during a cut, bounded integration/failure tests at the affected owner boundary,
and the exhaustive/final stack at terminal acceptance. Retain required live/capture evidence and
explicit publication/deployment approvals; a green fixture suite does not replace them.

After the architecture deliverable, re-audit the next port macro against current Rust and exact
C++ anchors before implementing its residual work. For example, #26 closed a bounded wire/lifecycle
slice, not general creature spell execution; #30's original claim of no power deduction is stale
(`handlers/spell.rs` already checks and deducts canonical Player power and tests rejection without
deduction). Existing #30–#35 and the full-parity ledgers retain their broader contracts. Do not
restart completed work from an old issue diagnosis or silently narrow a milestone to one capture.

The latest [modularity and ECS plan](../architecture/modularity-and-ecs-plan.md), reviewed above
laboratory HEAD `ee9a0128`, **selects private selective `hecs` now**, retaining cohesive domain
aggregates. This is a design choice, not an installed backend or proof it beats every alternative.
The finite independent-state/third-module checkpoint precedes production migration,
not another open-ended backend selection. Its two-module pre-freeze stage passes
at `118171c1`; the post-freeze independent third module also passes all four producer/lifecycle
tests at `c67acbfd`, without host/ABI/oracle edits. The 320-sample aarch64 campaign also passes
all preregistered gates: [result, costs and retained evidence](../architecture/modularity-conformance-results.md).
This completes finite pre-migration conformance, not production acceptance or a 10 ms frame
budget; the next checkpoint is the first real C1/C2 vertical with C0 admission/phase evidence.
The first production C1 cut now captures one coherent full-save projection and acknowledges
only its saved incarnation/row values, retaining changes made during pending I/O. The old
group-wide ACK is test-only. Production-linked controlled-persistence tests cover late change,
replacement, rollback, Unknown and cancellation; they do not establish real DB/relogin or
scheduler parity. All C0–C4 acceptance remains open to the extent recorded in the
[checkpoint](../architecture/session-578-checkpoint.md), including far-transfer save semantics.
An authorized live run on 2026-09-05 now adds bounded **real normal-save/relogin** evidence
for runtime `68fb338b` with QA tooling `04d54074`: two confirmed MariaDB save transactions,
two fresh logins, 13 skill/207 reputation rows retained, and identical 42-known-spell packets.
Persisted spell/favorite/equipment/transmog tables were empty, so their nonempty mutation
branches are not claimed live-proven. The original world executable was restored and verified
serving; BNet was not restarted. No crash, injected failure, scheduler/transfer parity,
publication or macro completion is implied. Exact hashes, scope and reports are in the checkpoint.
The C1 lifetime cut rejects occupied-map destruction/bulk unload and preserves the old
incarnation when replacement cannot allocate a generation. Controlled production-linked map
tests reproduce the old failure; automatic evacuation and complete shutdown QA remain open.
Map occupancy now comes solely from canonical Players: the manual count field/setter/fallback
is retired, and instance-full/GM fixtures use real occupants with unchanged packet assertions.

Native Rust is the default for first-party/custom extensions; Wasmtime/Core Wasm is the selected
operator-optional executor of shared hooks/state/lifecycle contracts. **Scope expansion:** #583
now delivers that bounded adapter and Rust/C guest evidence as well as external stateful modules,
composition and durable operator lifecycle after #231/#578. #153 audits both macros before #133
closes, including Wasm acceptance. The bounded delivery no longer waits for M6; the wider #99
ecosystem retains a fresh planning gate. Current login-message modules do not prove this product.

The completed [V1 laboratory](../architecture/modularity-lab-results.md) supplies 34 contract
checks and 120 corrected-campaign samples on aarch64, all within its pre-registered budgets.
It demonstrates the modeled contracts/costs, not arbitrary module state, a non-Rust guest, real
save durability or production integration. Its first campaign is retained as superseded after
three test/adapter defects were corrected. The separate V2 result above adds independent-module
evidence; neither experiment installs a production ECS/Wasm dependency, deploys code, advances
gameplay completion or establishes a whole-port capability-audit base.

Database migration boundary (issue #256): the daemon-owned permissive `DbUpdater` has been
retired. The `rustycore-db` composition binary is the sole schema migration authority, using a
source-controlled immutable SHA-256 manifest, per-database/component chains, advisory locks and a
durable incomplete marker that makes no false MariaDB DDL rollback claim. `world-server` validates
auth/characters/world/hotfixes and `bnet-server` validates auth through bounded read-only queries
before runtime writes or listeners; neither scans SQL paths, creates schemas, invokes a SQL client
or downloads artifacts. Exact legacy hashes or explicit schema fingerprints provide the
TDB343.24081 transition without reapplying already-materialized RustyCore DDL. Baseline artifact
acquisition remains #255 and the terminal persistence audit remains #153.

Trainer architecture note (issues #157/#158/#159, later dispatched by #142): list and the buy
adapter share one immutable offer decision. Normal trainer teaching revalidates that decision
under the exclusive money owner, commits effective money plus the exact #164 spell/skill result in
one Character DB transaction, attributes unknown COMMIT outcomes with a durable 128-bit operation
token, installs runtime state, and then publishes money, visual kits 179/362 and acquisition actions
in C++ success order. Non-packet acquisition effects install immediately after commit so a later
cross-socket fence failure cannot discard them; a valid cast fully suppressed by immunity or
rejected by the dynamic spell-disable gate still pays and emits both trainer visuals like C++,
while channeled wrappers remain outside the reduced projection. A process-wide pre-ConnectTo
character claim rejects a second live session for the same GUID and is released on a failed
instance handoff or late login packet-ordering fence, preserving C++'s single `Player*` save
authority; normal logout retains that claim through the account-wide offline write and old Player
identity teardown. Effective equipped-item and target-restriction duplicates follow C++'s deterministic
highest-record-ID assignment; ordinary pending spell/skill
changes are saved before trainer preparation instead of making the trainer unavailable until the
next autosave. Trainer failures and visuals use the Realm connection; creature visual fanout
retains the already validated canonical-or-legacy source position. Castable wrappers
require both a startup audit of effective/world-table blockers and a fresh player effect-mask proof;
the startup audit intentionally omits shapeshift metadata because C++ trainer wrappers use
`TRIGGERED_FULL_MASK`, including `TRIGGERED_IGNORE_SHAPESHIFT`;
the active proof rejects unsupported pet-aura hooks before mutation, replays definite self-target
and aura-spell cast failures after the C++-ordered fee/visuals, and resolves retained immunity auras
from their creation difficulty instead of the player's current map difficulty;
active auras now match covered `EffectAura`/`EffectAttributes` and negative aura-link immunity to the exact wrapper
effect/spell while startup excludes unsupported mechanic/state shapes; full C++ immunity-map parity
remains deferred until canonical Unit ownership.
Aura restrictions, equipped-item restrictions and craft reagents compose DB2,
official/custom hotfix overlays and final removals. Craft startup authority rejects a craft when
its created item or any positive reagent
effective sparse item template is absent, matching
`SpellMgr::IsSpellValid`.
Deterministic player `EffectLearnSpell` retains its distinct immediate-runtime/deferred-save timing.
Issue #142 later activated the `TrainerBuySpell` dispatcher arm and reconciled the
PartyUninvite/Vehicle registrations to exact equality with zero drift exceptions.

Battle-pet trainer purchase note (issue #161): a confirmed battle-pet species is now a purchasable
offer product (`Trainer.cpp:127-146` resolves `IsCastable()` before the `AddPet` branch, so only
direct-learn trainer spells reach it) and the list renders it available because C++ `GetSpellState`
has no cap gate. The purchase itself closes the legacy crash window — C++ charged in memory and
committed Character DB first and Login DB second at the next `Player::SaveToDB`
(`Player.cpp:19336-19344`), and `BattlePetMgr::SaveToDB` cleared `SaveInfo` at statement-append
time (`BattlePetMgr.cpp:377`), so a crash between commits kept the charge and silently lost the
pet — with a durable saga keyed by a 128-bit request key shared with the #160 Login DB receipt:
guarded charge + pending command in one Character DB transaction, exactly one pet through the #160
account owner (fence, journal lease and per-species capacity rechecked inside it), success packets
queued only after pet durability and recorded afterward by a durable `published` marker,
exactly-once refund for terminal failures with absolute durable-money reconciliation, and bounded login recovery that
converges interrupted commands without background tasks. Publication keeps the C++ battle-pet
order (money update, `SMSG_BATTLE_PET_UPDATES` petAdded, dependent runtime learn +
`SMSG_LEARNED_SPELLS`, trainer visual kits suppressed, silent cap). Pet, charge and refund are
exactly-once; packet enqueue attempts are recoverable and may repeat because enqueue has no client
ACK and cannot be atomic with the marker, while actual network delivery remains best-effort. A
crash may cause a recovery re-send without consuming the sole durable recovery signal first;
admission-time capacity/journal-lock failures return a structured result
while the wire stays silent like C++. Full design, transition table and fault matrix:
[battlepets.md](battlepets.md) (2026-08-03, #161). #142 activated the dispatcher arm.

### Fidelity policy for proven legacy defects

The legacy C++ server is the behavioral baseline, not an instruction to reproduce undefined
behavior or a demonstrated logic bug. An intentional Rust deviation is acceptable only when the
C++ behavior and defect are both pinned to exact source, the replacement is the smallest bounded
repair, focused tests distinguish it from both the legacy failure and a speculative rewrite, and
the deviation is recorded in the owning migration item. Client-visible changes additionally need
the corresponding C++/Rust capture decision recorded and, when deliberately different, a
re-pinned golden approved as a compatibility change. Suspicious literals, cleanup opportunities
and merely plausible optimizations do not meet that bar.

---

## 0. Historical capability audit: the "represented" pattern

The historical audit found many **`represented_*_like_cpp` paths** where a handler decoded
and validated a request but recorded intent without the required live mutation. This explained
why represented breadth was not playable parity. It is not a current rule that every function
with that suffix is inert: later paths directly mutate canonical owners. Trace the selected
operation from admission through mutation, persistence and publication before diagnosing it.

- Where the mutation path *was* wired, the feature genuinely **WORKS** (melee combat,
  bounded creature aggro/threat and spell-wire publication, inventory move/equip/destroy,
  loot, quest accept/turn-in, vendor, trainer, groups — durable state paths persist to DB).
- Where only the represented layer exists, the feature **looks handled but does nothing**
  observable (mail, auction, trade, taxi, resurrection, hearthstone bind, GO-use/portals,
  most spell effects and creature AI families beyond the bounded aggro/threat/melee/template-
  spell slices).

This is why the old "98% represented" metric and "bags don't open" coexist without
contradiction: ~98% of logic is *represented*, a much smaller fraction is *live*. The plan's
job is to convert represented→live for the playable path, then for everything.

Initial bridge convention: [represented-live-bridge.md](represented-live-bridge.md) documents
the handler → represented intent → live application boundary. The first converted example is
client stand-state change because it has one canonical owner and a deterministic, capturable
realm response. Applied outcomes emit bounded telemetry only after the canonical mutation
succeeds; client-controlled intent history is retained only in tests.

Live evidence (2026-07-12): the stand-state bot passed a Sit request with distinct realm and
instance sockets, and the bounded C++/Rust capture matched four packets exactly:
`CMSG_STAND_STATE_CHANGE 0x318C` on connection 1, `SMSG_STAND_STATE_UPDATE 0x271C` on
connection 0, `SMSG_UPDATE_OBJECT 0x27CB` on connection 1, and the `CMSG_PING 0x3768`
fence on connection 1. Strict capture-diff reported CLEAN after symmetrically excluding only
ambient `s2c:0x2DD4` creature movement. The slice remains represented-partial for the
documented full `Spell::cancel`, original cast-difficulty metadata, cross-Unit aura-application
lifecycle, and canonical `Map::SendObjectUpdates` ownership gaps. Missing masks now resolve from
an effective table composed in C++ load order from `SpellInterrupts.db2`, official/custom SQL
overlays by DB2 record ID, world `serverside_spell` masks, and the interrupt-mask subset of
`LoadSpellInfoCorrections`; this does not claim full server-side `SpellInfo` or correction parity.

Creature-movement evidence (2026-07-24, issues #21–#24): M2.1's production implementation had already
landed after the issue was opened. The global legacy tick launches random/waypoint `MoveSpline`s,
serializes `SMSG_ON_MONSTER_MOVE`, and fans them to nearby sessions through the final
`HaveAtClient` gate; PR #77 installed/restarted that runtime and manually verified visible
creature movement in the client. The issue closeout pins a real 117-byte C++ compressed-waypoint
packet from the accredited capture artifact
`a25f2c2bbf60de6cda7e32f305d732733017e711eb474dd5dbf6e007690143a8`, and Rust reproduces
it byte-for-byte; the complete 717-test packet suite is clean with that regression.

Issue #24's guarded live Detour capture is now strict-clean on three isolated packets
(heartbeat, one compressed chase spline, ping fence). The capture proved C++ falls from the
elevated fixture to the lower `.map` plane when static VMap height is unavailable. Rust preserves
the elevation only because Detour itself returned a connected elevated polygon corridor; it does
not lift a lower route from equal endpoint heights, which cannot distinguish disconnected
platforms. Identity, options, flags and transport fields remain strict. The same capture exposed
and fixed a separate lifecycle omission: Rust now mirrors
`Creature::AtEngage`/home-finalize by temporarily adding `UNIT_FLAG_CAN_SWIM` when the movement
template permits water, so `MoveSplineInit` publishes the same `CAN_SWIM` flag as C++ and restores
the out-of-combat flag afterward. Player chase snapshots are keyed by map, instance and GUID, so a
live target that teleports elsewhere cannot feed foreign coordinates to the chaser's Detour map.
M2.2 adds one persistent `wow_movement::MotionMaster` to each legacy `WorldCreature` and advances
it once from the globally owned creature frame, after spline position advancement as in
`Unit::Update`. Random and waypoint execution is now gated by the selected stack entry; active
combat chase has normal priority, interrupts an in-flight wander spline with the existing
C++-shape stop packet in the same global aggro frame, but remains below a represented
highest-priority point/charge generator. The represented source lifecycle is advanced in that
same frame and popped finalizers are applied before resynchronizing, so finite spline/timer
generators release their selector proxy and expose chase/default naturally. Combat reset also
exposes the default generator again. This is a bounded runtime bridge: the owner-dependent
random/waypoint work still runs in the existing concrete generators after stack selection, and
real chase target pathing remains with M2.5.

M2.3 re-audited the existing world startup loader against `WaypointManager::LoadPaths`: the
issue's “no waypoint path loader” diagnosis was stale. The current DB loads 7,698 parent paths
and 142,185 ordered nodes, and 5,419 waypoint spawns resolve a nonzero addon path. The remaining
live defect was cadence: the spline advanced against elapsed wall time while random/waypoint
timers received a fixed configured 10 ms, so scheduler delay could finalize the spline before
the generator timer and stretch patrol re-arming in proportion to runtime lag. The single existing Tokio owner now measures and
clamps the real elapsed frame `diff`, matching the C++ `World::Update(diff)` →
`MapManager::Update(diff)` contract, without adding tasks or holding a lock across an await.
Positive, negative and long-horizon regressions prove random re-arming and multi-node waypoint
progress. The installed release (`3c210cdc…`) passed a connected bot login/stand smoke: the bot
received two `SMSG_ON_MONSTER_MOVE` packets and the server published 627 movement packets over
327 visible-work ticks in the observed window. This closes the bounded live wander/patrol item,
not Detour path exactness, formation/transport behavior, SmartAI callbacks or chase/threat.

M2.4's opening diagnosis — "navmesh loaded but `find_path()` never invoked" — was **stale**.
`wow-recastdetour` is a real vendored Detour build with `findPath`/`findStraightPath`/
`moveAlongSurface`/`raycast` plus a ported `FindSmoothPath`/`FixupCorridor`/`GetSteerTarget`, the
runtime pathfinder thread is created whenever `CONFIG_ENABLE_MMAPS` is on (C++ default `true`),
and both live generators already resolved corridors and launched them through
`MoveSplineInit::MovebyPath`. The real defects were in **what the query returned and when it was
even attempted**, all four contrasted against `PathGenerator.cpp`:

1. `PathGenerator::CreateFilter`/`UpdateFilter` (`PathGenerator.cpp:648-698`) derives the Detour
   filter from the owner; both live call sites passed a hardcoded ground-only creature filter, so
   amphibious owners could never cross `NAV_WATER`/`NAV_MAGMA_SLIME` polygons and combat/evade
   owners never got `NAV_GROUND_STEEP`. The filter is now sampled per creature from
   `CanWalk()`/`CanEnterWater()`/`IsInCombat()`/`IsInEvadeMode()`.
2. `CalculatePath` answers a missing navmesh/tile with `BuildShortcut()` and
   `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` (`PathGenerator.cpp:79-86`), which
   `RandomMovementGenerator::SetRandomLocation` happily launches. RustyCore mapped that same case
   to a path *failure*, so wander on unmeshed terrain retried every 100 ms forever instead of
   moving. Only a genuine Detour error stays a failure now.
3. `BuildPolyPath` calls `BuildPointPath` exactly once (`PathGenerator.cpp:287`, `:527`). The Rust
   corridor builder also ran a full `findStraightPath` pass whose result was then discarded — one
   wasted Detour query per request, and its `PATHFIND_SHORTCUT`/`PATHFIND_SHORT` bits leaked into
   the surviving smooth path, which made the callers reject a perfectly good navmesh route. The
   point path is now built once, in the mode `_useStraightPath`/`_useRaycast` selects, and the
   possibly clamped `endPoint` is threaded through while `GetEndPosition()` stays the requested
   destination for the `_forceDestination` comparisons.
4. `CalculatePath` requires `HaveTile(start)` **and** `HaveTile(dest)`; C++ satisfies both because
   `TerrainInfo::LoadMMap` loads each grid's `.mmtile` as the grid loads. RustyCore demand-loads
   from the path request and only consulted the start position, so any destination one tile over
   reported "no navmesh" while the mesh sat on disk. Both endpoints are demand-loaded now.

The three shortcut/failure `PathType` values are also bit-exact again: C++ `BuildShortcut()`
*assigns* `PATHFIND_SHORTCUT` before the caller ORs onto it, so the no-poly and empty-`findPath`
branches are plain `PATHFIND_NOPATH` and the point-path failures are exactly
`SHORTCUT|NOPATH` / `SHORTCUT|SHORT`. A deterministic navmesh fixture (a walkable ring around an
unwalkable centre cell) proves the live waypoint tick walks *around* the hole with real
intermediate points; the same test degrades to the four-point straight line when pathfinding is
disabled, so it is a genuine regression guard.

Four further C++ branches were then closed in the same slice:

5. **The mesh-hole exceptions.** `BuildPolyPath` grants `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
   when the owner `CanFly()` (`PathGenerator.cpp:180,198-202`), and the far-from-poly branch also
   shortcuts for a flying owner or one that `IsFalling()` towards a lower destination — the charge
   case (`:221-240`). Those owner facts are now threaded into the Detour layer. The `CanSwim()`
   halves still need `Map::GetLiquidStatus`; treating "no liquid data" as "not submerged" can only
   withhold a shortcut C++ would grant, never invent one.
6. **`UNIT_STATE_IGNORE_PATHFINDING`** is set from `CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING`
   (0x20000000) as `Creature::Create` does (`Creature.cpp:1154-1155`), through both the create
   lifecycle and the runtime `flags_extra` seam the legacy registration path uses.
7. **Corridor reuse.** The previous `_pathPolyRefs` now travel in the path request, so
   `BuildPolyPath`'s subpath and 80%-prefix-plus-suffix branches (`:291-413`) are reachable, and
   `GetPathPolyByPosition` (`:94-123`) is ported so `GetPolyByLocation` consults that corridor
   before paying for `findNearestPoly` — which also changes the `distToStartPoly` feeding the
   7.0-yard test. Random and chase keep their corridor for the generator's lifetime like C++;
   waypoint and home get a fresh one per call because `MoveSplineInit::MoveTo` constructs a new
   `PathGenerator`. Two edge decisions are explicit: the C++ lookup really uses the full 3D
   `dtVdistSqr` and its literal squared `< 3.0f` threshold (effective radius `sqrt(3)`), so Rust
   does not replace it with the reviewer's 2D metric or a speculative `< 9.0`. A failed
   80%-suffix keeps the complete valid prefix: with no suffix there is no overlap polygon to
   subtract, although C++ unconditionally computes `prefix + 0 - 1`. Rust clamps the point path
   to that retained corridor boundary, recalculates a singleton prefix instead of reproducing
   C++'s zero-length tail underflow, and clamps a successful singleton partial corridor to its
   reachable polygon rather than appending a straight segment across a disconnected gap.
   `BuildPointPath` branches that call C++ `BuildShortcut()` now also clear the retained polygon
   corridor, so a later update cannot reuse path state that `PathGenerator::Clear()` destroyed.
8. **Chase and home now path.** `ChaseMovementGenerator::Update`
   (`ChaseMovementGenerator.cpp:94-240`) and `HomeMovementGenerator::SetTargetLocation`
   (`HomeMovementGenerator.cpp:60-82`) were already faithful Rust ports with **no caller**. Chase
   drives the live tick with the victim's facts snapshotted by the tick driver (players from the
   registry, creature victims from the map, both before the mutable borrow) because the creature
   step has no object accessor; it applies the C++ destination choice, `forceDest = CanFly()`,
   `ShortenPathUntilDist` against the victim, `SetFacing(target)` and the chase walk template.
   Rust deliberately stores the computed move-toward/move-away direction: C++ compares
   `moveToward` with `_movingTowards` but never assigns the field, leaving move-away range checks
   on the wrong bound and able to stop immediately. The direction is committed only after the
   corresponding spline launches; a direction flip discards the old `PathGenerator` corridor
   before querying, while a failed query cannot publish movement that never began. Home
   replaces a *teleport*: the old `Returning` arm assigned `move_target` onto the creature position
   with no spline and no packet. Both have around-obstacle tests against the real navmesh fixture
   that fail when pathfinding is disabled.

The connected gate is now represented by the fail-closed
`detour-chase-around-obstacle` capture flow. It pins a generated MMap tile, disposable
character/spawn identity, exact heartbeat → compressed `SMSG_ON_MONSTER_MOVE` → ping window,
full MonsterMove decoding (normalizing only the process-global spline ID), source/binary
revision provenance, and guarded private-DataDir/database restoration. Its reviewed C++/Rust
pair is strict-CLEAN across all three selected packets with an empty divergence baseline.

Still absent, and explicitly **not** claimed: **point/charge, fleeing and confused** generators are
complete, unit-tested ports with no live trigger — nothing sets `UNIT_STATE_FLEEING`/`CONFUSED`
(there are no fear/confuse aura handlers) and no live caller installs a Point generator, so wiring
tick arms for them would only be exercisable from tests. Also open: mutual chase (needs the
victim's `MotionMaster`), `IsWithinLOS` and `ShortenPathUntilDist`'s LOS test (VMap stub), the
`CanSwim()` mesh-hole halves and `Map::IsUnderWater` (liquid), `Map::GetForceEnabled/Disabled
NavMeshFilterFlags`, `_useRaycast`/`_useStraightPath` (no live caller),
`MovePositionToFirstCollision` + `IsWithinLOS` gating the wander roll, VMAP/liquid-aware
`NormalizePath`, off-mesh links/transports/formation, the reached-home addon/sparring-health reloads,
and C++'s per-instance pathfinder concurrency — every map still serializes
through one pathfinder thread.

M2.5 / issue #25 replaces spawn-frozen threat with the live C++ ownership path. Nonlethal hostile
spell damage now engages the creature and adds effective-damage threat; direct healing forwards
half of effective healing, divided across eligible threatening creatures. `EffectTaunt` matches
the caster to the available highest threat and `SPELL_AURA_MOD_TAUNT` gives the newest active
taunt priority until its DB2 duration expires, restoring an older still-active taunt afterward.
The global creature tick reselects at C++'s 110% melee / 130% ranged thresholds, broadcasts
`SMSG_ATTACK_STOP` on evade, clears threat/tap state, blocks re-aggro while returning, and restores
spawn health only when home movement finalizes. Threat references now also preserve C++'s distinct
`Suppressed` state for targets immune to the attacker's melee school, confused, or held by a
damage-breakable stun; they remain in the threat list but cannot be selected until online again,
and clearing the aura alone does not expire suppression: only new threat from that target or
C++'s explicit `TauntUpdate` reevaluation can reactivate it. An active taunt explicitly bypasses
suppression. `CallAssistance` is once per engagement, delayed by the configured family-assistance
delay, stored on the caller with assistant GUIDs like C++ `AssistDelayEvent`, re-resolved and
restricted by C++ `CanAssistTo` gates when due, and cannot chain from an assistant. Focused
positive/negative regressions also pin ordered, lossless delivery of committed creature combat
events; its per-session backlog is bounded and disconnects a stalled/desynchronized consumer
instead of dropping events or growing without limit. The complete `wow-world`
3119/0 and `wow-entities` 667/0 library suites are clean. A guarded live
`detour-chase-around-obstacle` recapture from `4535a25a` proves the attack-accepted → target
acquisition → chase slice byte/opcode-clean against the retained C++ golden (3/3 packets, no
value/routing/missing/extra differences); the fixture also restored its character, respawn, world
DB, and private DataDir snapshot exactly. That wire window does not exercise heal, taunt,
assistance, or evade; those branches remain covered by focused C++-anchored regressions rather
than dedicated live captures.

M2.6 / issue #26 closes the bounded stock `CombatAI`/`TurretAI` template-spell publication
slice. The globally owned creature frame reads the hydrated template slots and active-difficulty
spell metadata, schedules only the represented instant target shapes with the C++ raw cooldown
rules, and commits an atomic adjacent `SMSG_SPELL_START`/`SMSG_SPELL_GO` pair before the
same-frame melee phase. The final P1 hardening removes GO's unconditional-hit assumption.
It permits publication only for the bounded C++ resolution of a physical
`DmgClass=MELEE` Creature spell whose sole target is a Player attacked from behind, whose spell
and represented effect mechanics are all zero, and whose complete Creature/Player source
authority proves every omitted spell-hit source hit-inert. Completeness does not require every
external source to be empty: persistence/login sources may be nonempty when every exact effect is neutral
to this bounded result, but the reduced runtime's canonical local aura
application/modifier/visible containers still must be empty until their full C++ semantics are
owned. Player authority fails closed across persistence and login/zone reconciliation, map/area
ancestry, guild, skills, active/rewarded/auto-push quests, glyphs, active-specialization traits,
pets and battle-pet slots, FFA/PvP/war-mode state, SpellArea/outdoor/battlefield sources, and
script, legacy/all-rank and SpellLinked hooks. A valid SpellLinked hook blocks its candidate, as
does the absolute trigger ID retained from a rejected loader row.

A Creature-owned uniform roll in `0..=9_999` resolves base `MISS` below `500` (5%) and `HIT`
otherwise. C++ causal order is preserved locally: cast before repeat schedule and hit roll before
the cooldown draw; `NO_ATTACK_MISS` consumes exactly one hit roll before forcing `HIT`. The due
EventMap slot is cleared before cast, so a blocked post-cast schedule cannot create an immediate
retry loop. After publishing an accepted HIT, Rust tombstones before scheduling: C++'s launch
phase next consumes an unconditional critical roll and possible effect-value draws outside this
wire slice. MISS does not enter those target-effect draws and may retain authority for its repeat
delay. Spell, melee and movement randomness share a fail-closed Creature tombstone. A valid
melee swing that reaches the still-unrepresented C++ damage/outcome/proc calculation sets it and
publishes no invented damage or wire. C++ has one process-global RNG whereas Rust has one RNG per
Creature; P1 therefore claims equal distributions and local causal draw order, not the exact
global stream or cross-Creature interleaving. Missing or unsupported metadata, authority, power,
aura, projectile, target, visual, difficulty and optional-payload shapes fail closed with neither
START nor GO. Already-performed deterministic event/reset work remains, while a tombstone blocks
subsequent random-dependent work. This is a **wire/lifecycle subset only**: GO's resolved hit/miss
topology does not apply spell effects, damage, health, aura, or power mutation, and it is not
evidence of the full `Spell` pipeline or M3 combat math.

The first Rust live attempt exposed a real login bridge defect rather than a casting mismatch:
`player_faction_template_like_cpp` remained unset, the player registry published faction template
`0`, and the creature-hostility gate correctly rejected that unrepresented identity before
`AttackStart`. The final HEAD derives the player's faction template from `ChrRacesStore`
when the loaded identity is installed and mirrors it into the canonical `Player`; the regression
covers identity → registry → canonical player without manually seeding a faction. The accredited C++ source
derivation starts at base HEAD `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, applies the reviewed
one-file patch SHA-256
`ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262`, and yields patched HEAD
`8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c`. That patch only corrects the
`ChrSpecialization` index-container bound needed to load the installed DB2 dataset; it does not
touch creature AI, spell selection, casting, or packet serialization.

The final fixture guard uses contract `creature-spell-casting-shell-fixture-v2`. It verifies the
installed stock `AIName=SmartAI` and difficulty-0 `StaticFlags1=0`, CAS-switches the capture
window to `CombatAI` and `0x00100000` (`CREATURE_STATIC_FLAG_NO_MELEE`), then CAS-restores the
exact `SmartAI`/`0` pair and verifies cleanup. Suppressing auto-melee prevents that unrelated path
from consuming melee damage RNG before CombatAI's due spell without disabling its EventMap cast.
The final live authority also loads effective `ChrSpecialization` hotfix rows and corrects
external-ID `AreaTable` WDC4 offsets, so Shattrath area `3697` resolves to map `530` and Terokkar
zone `3519` like C++. OutdoorPvPTF source spell `33377` is admitted only after its effective XP
and outgoing-damage auras plus exact runtime-hook authority prove it hit-inert; the four
Auchindoun dungeon IDs remain fail-closed without C++'s `(Map*, zone)` registration authority.

Both C++ and Rust were recaptured from clean harness HEAD
`42977e9accb24fc3921af075f4122e1f0180f4a2`. The final guarded Cabal Interrogator `22378` /
Eviscerate `15691` import contains exactly the adjacent START/GO pair on both sides with no
accepted divergence. It records one observed **HIT** branch and does not prove that `15691`
always hits. Its current review identities are:

- C++ RAW PKT:
  `b52cc8ba962160be63286e72eb7611c6282b0cdc3a1cee0082fc6d6d7bf2c7b9`;
- Rust RAW tree:
  `9aee309d9ffb2e2e1e5a33167c228ccaa8d1634d917efd026e1a525f2a5db94a`;
- C++ / Rust capture manifests:
  `d40e3615b3337a26a3c4d4e380dc665c23719133ec0b3c7a05febdfd640e849d` /
  `3c942209db52f9f36b3d661477f0cad766e7e2ee49cf1fc97d68a74d996f0da0`;
- filtered C++ PKT:
  `a6b32206e3277e455e25f6aa8e491606aa5cd9449e2bf24245ea9dd5db79d932`;
- normalized Rust tree:
  `cc8d53b06c2727c95990eda80fd095a1a7e390da0af16981429d07176e0c003b`;
- `capture-lineage.json` file:
  `f443539e7857ac27dfb2029012f1e889d92ed27a224f89f7a6247f9510f0479d`.

`diff creature-spell-casting --strict` reports 2 matched packets with zero value, routing,
missing, or extra differences. `verify-required creature-spell-casting` is CLEAN with exact
topology/order and correlated payload semantics.

---

## 1. Historical progress picture (three axes, not one number)

Historical capability snapshot at the audit base above, not measured percentages for #578 or
current HEAD. The current architecture checkpoint reports acceptance boundaries separately.

| Axis | Estimate | Meaning |
|---|---:|---|
| **A. Breadth represented** | ~98% of R8 rows touched | Logic exists/contrasted in the represented model. **Retired as a headline** — rewards breadth over working features. |
| **B. Live-playable core** | **partial, holed, buggy** | Core loop (login→move→melee→loot→quest→vendor→group) is live. The scoped D-C1…D-C9 CRIT integrity track is closed, but combat math and multiple HIGH/MED gameplay/runtime gaps remain. Spells and creature AI have bounded live subsets; most spell effects/AI families, world-interaction and death remain represented-only or absent. |
| **C. Full 1:1 parity** | **low** | Long tail absent: ~108 spell effects, ~255 aura types, content scripts (0/294k LOC), mail/AH/calendar live, BG/arena/instances, ~215 stat/data stores. |

Part 1 of the plan drives **B** to complete; Part 2 drives **C** to complete.

---

## 2. Historical capability matrix and bounded additions

This matrix records the historical subsystem audit and its explicitly dated additions. Its old
paths and statuses are discovery pointers, not proof that the same gap still exists at HEAD.

**WORKS** = mutates live state + persists/broadcasts like C++ · **WORKS⚠** = live + persists
**but has correctness/integrity bugs** (see [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)) ·
**PARTIAL** = subset only · **STUB / REPRESENTED-ONLY** = validates/records intent, no
observable mutation · **ABSENT**.

> ⚠ The scoped D-C1…D-C9 integrity defects are closed, but the adversarial audit still lists
> substantial HIGH/MED gaps in [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md). "WORKS"
> means the named path is live; it is not a claim of full 1:1 gameplay parity.

### Core gameplay loop — live, with remaining non-CRIT gaps
| Capability | Status | Evidence / defects |
|---|---|---|
| Auth/BNet SRP6 + world-enter handshake | WORKS | recent `fix(bnet)` commits; played live |
| Player base/stat projection | **PARTIAL (live)** | issue #60 replaces the incorrect `player_levelstats` path with C++-anchored `player_racestats` + `player_classlevelstats`, `GtBaseMP`, `CreateHealth=0` and a shared create/login/equipment/level-up StatSystem projection. Login seeds passive parry/block capability before projection and defers its saved-health clamp until persisted stat auras and represented item/enchantment modifiers are active; covered total-stat aura recalculations retain those item bonuses and emit no pre-CreateObject VALUES delta. The recorded paired C++/Rust login captures match the scoped max-health/mana, five primary stats, armor, base mana, AP and damage fields, including the 3% total-stat racial passive. That evidence does not establish complete login `UpdateObject` or wider unit-mod/aura/item-stat parity, which remain open. |
| Starting skills and skill-rewarded login spells | **WORKS (scoped issue #62)** | `SkillRaceClassInfo` now follows C++ `Availability`/`MinLevel`; default skill rank/max/step follows language, level, mono, tier, always-max and DK rules; loaded rows are normalized like `_LoadSkills`; and the live no-DB-spell login path applies `LearnSkillRewardedSpells` with real spell levels, quest fallback, Riding, masks and actual skill values. Correct WDC4 inline IDs restore Common-compressed `SkillLineAbility` fields. A live Blood Elf Hunter C++/Rust pair yields the same exact 43-spell set under the reviewed unordered-map comparator; bit/count/list/favorites integrity remains strict. Wider skill gain/update/discovery/unlearn runtime remains in L18. |
| Player movement + broadcast to nearby | WORKS⚠ | `movement.rs:310`; trust-client position (D-H10), creature destroy deferred (D-H15), async CREATE race (D-H14) |
| Melee combat (deals damage→death→loot) | **PARTIAL** | `session.rs:47635`; **no damage formula / hit table / armor mitigation** (D-H1, D-H2) — numbers are wrong |
| Global creature runtime (aggro/melee/move→packets) | WORKS (default on) | `world-server/src/main.rs:12682+` |
| Inventory equip/swap/move/destroy (+DB) | WORKS | D-C1/C2 relog metadata and D-C4 atomic swaps are closed; issue #52 adds C++ position/bank/unequip/store/equip gates, container-aware move/merge/swap/destroy, realm-routed errors, and paired installed C++/Rust invalid-gate plus occupied-swap/relog evidence with a strict clean action capture |
| Loot items + money (+DB) | WORKS⚠ | D-C5/C6 concurrent-claim duplication is closed; quest-credit gap (D-H6) and process-abort recovery of detached post-COMMIT continuations remain separate boundaries |
| Quests accept/turn-in core rewards (+DB) | **PARTIAL** | `quest.rs:1359,5569`; kill/explore/item objectives may not auto-advance (D-H4/H5/H6) |
| Vendor buy/sell, Trainer learn, Groups (+DB) | WORKS⚠ | vendor D-C8 and group-capacity D-C9 atomicity are closed; finite-stock oversell (D-H11), trainer validation (D-H9), buyback/refund and wider group parity remain |
| Gossip menus + quest-giver status icons | WORKS | `handlers/quest.rs:1248`; gossip conditions evaluated |
| Item enchant/gem/socket, durability repair, binding | **PARTIAL** | D-C1/D-C2 relog now reloads and serializes the exact persisted 13-slot enchant/random-property state, including the paired exact create-block proof. Broader gem/socket/durability/binding runtime parity remains unproven and belongs to later parity work; issue #52 is the separate move/equip/store validation closure. |
| Bank / equipment-sets / void-storage persistence | **WORKS** | D-C3 is closed: PRs #103, #113 and #115 merged with required CI/review gates. Installed bank, equipment/transmog and void-storage relog QA passed; the committed equipment-set ACK and void-storage query captures are strict C++/Rust CLEAN. Issue #114's documented failure-only all-or-nothing divergence remains intentional and bounded to its stronger transaction contract. |
| UpdateFields / CREATE-block serialization | WORKS | `wow-packet/src/packets/update.rs`; issue #10 re-audited rows 1212–1220 and closed M1.4's bounded value gaps, including selected non-mana creature power and runtime GO ArtKit; canonical per-spawn ParentRotation remains in its documented architecture follow-up |
| Rested XP / offline rest state (XP slice) | **PARTIAL (live)** | issue #81: live accrual, consumption, DB persistence/relog and `SMSG_LOG_XP_GAIN` are capture-clean under the reviewed runtime-counter comparator; full `RestMgr` ownership and rest-area wire remain open |

#### 2026-07-18 bounded rested-XP evidence

The same guarded bot workflow passed against the primary C++ reference and
RustyCore. A DB-controlled 86,400-second offline interval produced wilderness
and resting bonuses of about `14.88` and `60.00`; a live Mana Wyrm kill
(`entry=15274`, map `530`) emitted realm-routed `SMSG_LOG_XP_GAIN` with
`Original=100`, `Reason=Kill`, `Amount=50`, and `GroupBonus=1.0`. XP persisted
`0 -> 100`, rest bonus persisted `300 -> 250`, a full relog observed the saved
state, the disposable fixture was restored, and the persisted 300-second
creature respawn row cleared naturally.

The committed one-packet C++/Rust flow is strict-CLEAN only under the narrowly
reviewed comparator that omits the nonzero lower 40-bit runtime counter of a
Creature Kill victim GUID. It still requires realm routing and exact high type,
realm, map, entry, subtype, server id, and every XP field; malformed or
zero-counter bodies fail. This does not prove a real tavern/city AreaTrigger
walk, nested `RestInfo` update-field wire, honor rest, full group/KillRewarder
fanout, or a per-Player `RestMgr`/`Player::Update` owner. The aggregate
`#PLAYER.12` therefore remains open.

### Engines — PARTIAL / STUB (the gameplay-quality gap)
| Capability | Status | Evidence |
|---|---|---|
| Spell cast → SPELL_START / SPELL_GO | **WORKS (bounded paths; issue #26 P1 verified)** | the represented player-cast handler publishes its existing START/GO path; issue #26 adds one map-owned creature path. P1 no longer presumes HIT: only a physical melee Creature spell against a rear-attacked Player, with zero mechanics and complete source authority proving omitted sources hit-inert, may publish the atomic pair; canonical local aura containers remain empty. Player persistence/login/zone/map/guild/skill/quest/glyph/trait/pet/FFA/SpellArea/hook evidence fails closed, including valid and rejected-trigger SpellLinked rows. The Creature roll maps `<500` to base `MISS` and the rest to `HIT`; local order is cast→schedule and hit→cooldown, with `NO_ATTACK_MISS` still consuming a hit roll. An accepted HIT publishes then tombstones before scheduling because subsequent launch/effect RNG is omitted; MISS may retain authority for its repeat delay. Spell/melee/movement share the tombstone. C++ global-vs-Rust per-Creature RNG parity is distribution/local-order only. Guard v2 recaptured C++ and Rust from clean `42977e9a`, temporarily switching stock `SmartAI`/`0` to `CombatAI`/`NO_MELEE` and restoring it; strict 2/2 and `verify-required` are CLEAN. This does not claim effect execution or the full Spell pipeline. |
| Spell effects dispatched | PARTIAL **~42/150** | `session.rs:48774-49383` |
| Spell cast cost/timing/cooldown | **PARTIAL** | SpellCastTimes/SpellCooldowns/SpellPower DB2 hydrate represented `SpellInfo`; the player path checks/deducts flat + mana-pct costs, while creature `CombatAI` uses raw `RecoveryTime`, floors missing/short values at C++'s 5-second AI default, and re-arms every repeat attempt in `[cooldown, 2×cooldown]`. Full C++ SpellHistory/modifiers and general cast lifecycle remain open. |
| Spell damage calc (coeff/crit/resist/absorb) | **ABSENT** | `SpellEffectDb2Entry` parsed but unused |
| Spell LOS/range/facing/reagent checks | **PARTIAL** | issue #26 revalidates the live canonical creature/victim, C++ min/max range with combat reaches and movement allowance, and map LOS immediately before publication; the final bounded P1 slice additionally requires that the Creature caster is behind its Player target. General player-cast range/facing/reagent coverage and real VMap-backed LOS remain incomplete; the shared VMap foundation is still a stub. |
| Aura apply + client update | PARTIAL | `session.rs:26431` |
| Aura periodic tick (DoT/HoT) + ~255 aura types | **STUB** (~5 types) | `session.rs:27810` expiry only; `unit_subsystems.rs:12-14` |
| Proc system | **ABSENT** | `SpellAuraOptionsEntry` fields unused |
| Channeled / missiles / ground-AOE / DynamicObject | **ABSENT** | no lifecycle state machine |
| Creature AI (threat gen / spell cast / waypoints / SmartAI / text) | **PARTIAL** | random wander, DB waypoint patrol, threat/taunt/assistance/evade, and bounded CombatAI/TurretAI template-spell START/GO publication are live; spell effects, other AI families, text and SmartAI interpretation remain open |
| Creature movement: spline broadcast (SMSG_MONSTER_MOVE) | **PARTIAL** | global legacy tick continuously launches and broadcasts random/waypoint splines on measured elapsed cadence; exact captured compressed-waypoint body is byte-clean |
| MotionMaster tick | **PARTIAL** | persistent per-creature stack is ticked once by the global frame and selects random/waypoint/chase priority; concrete owner-bound bodies and broader generator callbacks remain |
| Pathfinding (Detour navmesh query) | **PARTIAL** (random/waypoint/chase/home live) | real vendored Detour + ported `FindSmoothPath`; owner-derived filter, single `BuildPointPath`, both endpoint tiles demand-loaded, C++-exact no-navmesh shortcut, fly/falling mesh-hole exceptions, `IGNORE_PATHFINDING`, corridor reuse + `GetPathPolyByPosition`. Point/flee/confused have no live trigger; raycast/straight-path unreachable; mutual chase, VMAP LOS and liquid-aware `NormalizePath` absent |
| Canonical map tick (`wow_map::MapManager`) | **STUB** | `wow-map/src/manager.rs:520` no AI/combat side effects |

### World-interaction & death — REPRESENTED-ONLY (handlers record intent, no mutation)
| Capability | Status | Evidence |
|---|---|---|
| GameObject use (doors/chests/**portals**/quest objs) | REPRESENTED-ONLY | `handlers/misc.rs:6378`; portal/chest/transport fall to "not ported"; **#13** |
| Mail send/list/take/delete | **ABSENT** (no handler) | only `QueryNextMailTime` exists |
| Auction list/bid/buy/sell | REPRESENTED-ONLY | `handlers/misc.rs:1862` record-only |
| Player trade | REPRESENTED-ONLY | `handlers/misc.rs:1466` record-only |
| Taxi/flight path | REPRESENTED-ONLY | `handlers/misc.rs:117` no flight movement |
| Death/resurrection/corpse/graveyard | REPRESENTED-ONLY | `handlers/misc.rs:2443,2495`; "graveyard data not implemented" |
| Durability loss on death, ghost flags | **ABSENT** | no KillPlayer hook |
| Hearthstone bind / set home | **STUB** | `character.rs:8926` "TODO: set bind in DB" |
| Item-use → cast spell (`ItemEffect.db2`) | **ABSENT** | store not loaded; no `CMSG_USE_ITEM` executor |

### Simulation foundations — silent subdependencies (block correctness everywhere)
| Foundation | Status | Evidence / impact |
|---|---|---|
| Terrain height (GetHeightZ / ground correction) | **ABSENT** | creatures spawn at DB Z; no ground snap |
| VMap line-of-sight | **STUB** (returns `true`) | `world_object.rs:1551`; spells/pathing tunnel walls |
| Stat-formula GameTables (Gt* crit/dodge/HP) | **PARTIAL** | `CombatRatings.txt` now drives represented crit/dodge/parry/block rating scaling; HP/MP/regen/class stat tables still incomplete |
| `ChrClasses`, `ChrRaces`, `FactionTemplate`, `CharBaseInfo` stores | **PARTIAL (live)** | the first three stores are loaded and login now publishes the race-derived player faction to the registry/canonical Player, closing the faction-0 creature-hostility failure; full `CharBaseInfo` and wider create/stat parity remain open |
| `SpellPower`/`SpellCastTimes`/`SpellCooldowns` stores | **PARTIAL** | loaded into represented spell metadata for normal difficulty; full C++ modifier and runtime integration still incomplete |
| DBC/DB2 store coverage | **~110 / ~325 (34%)** | `cpp-db2-stores.tsv` |
| Player periodic save | **REPRESENTED-PARTIAL** | session timer now uses `CONFIG_INTERVAL_SAVE` / `PlayerSaveInterval` and queues represented `Player::SaveToDB`; installed runtime passed bot login/logout and action/travel/quest-objective preservation QA, while first-save randomization, capture diff, and manual live-client QA remain pending |
| Multi-statement save transactions | **REPRESENTED-PARTIAL** | represented character save now commits the Rust-covered `Player::SaveToDB` statement set in one `SqlTransaction`; full C++ save surface, login/account transaction coupling, capture diff, and manual live-client QA remain pending |
| Respawn DB persistence | **ABSENT** | in-memory queue; respawns lost on restart |
| Quest objective auto-credit (loot item / explore) | **ABSENT** | not hooked to loot/area-trigger |

### Absent systems (whole-domain)
Content **Scripts** (`wow-scripts` 40 ln vs ~294k LOC C++) · Battlegrounds/Arena/Battlefield/
OutdoorPvP · Instances lock/save/difficulty toggle · full Conditions eval · full Phasing
refresh · Weather · Warden · Calendar · Petitions · Pet/Totem AI. Empty crates:
`wow-combat`/`wow-spell`/`wow-achievement`/`wow-social`/`wow-pvp` (1 line each; `wow-ecs` was removed by #298).

---

## 3. Historical playable-blocker notes and bounded fixes

Issue #7's CUF login crash is fixed on its branch: Rust now matches the exact C++
post-add order, and a paired live capture with one non-empty profile pins the
four-packet sequence. The CUF and final phase-shift packets are byte-identical;
the broader M1 exit and manual-client UI validation remain open.

Issue #8 closes the compression-stream lifetime defect: the active C++ `0x400`
threshold now has one persistent deflate owner for the complete physical socket,
including across `WorldSocket::split_for_io`. Installed login QA decoded four
successive compressed packets through one persistent inflater before completing
the stand-state round trip. Original-client/manual UI validation remains part of
the broader M1 exit.

Issue #9 closes the first eleven login-burst audit rows. Seven live gaps are corrected:
global account-data and tutorial resends, battle-pet-lock placement, configured MOTD,
cross-socket packet ordering, contact-list publication, and PlayerCondition-filtered
account-mount partials. The other five rows were already fixed, request-driven, or
explicitly bounded after C++ contrast. An accredited 81-packet two-socket Rust capture
proves the corrected physical order; the installed C++ runtime could not complete the
same bot's second-socket login, so this is intentionally not described as a full
byte-clean login capture.

| # | Bug | Effect | Status |
|---|---|---|---|
| #13 | `CMSG_GAME_OBJ_USE` doesn't cast GO use-spell | **portals do nothing** | open |
| #9–#12 | 33 login-burst divergences (`world-load-audit.md`) | ordering/value parity | #9–#10 merged; #11 implemented and locally validated; #12 open |

---

## Historical architecture reality

At the 2026-09-05 reviewed local HEAD, the legacy creature runtime still supplies behavior while
canonical Map storage owns active Players and the staged `MapRuntime` applies typed transitions.
It is no longer accurate to call canonical Map an empty storage/runtime skeleton. Nor does that
make it the sole production simulation driver: shared manager synchronization, legacy/canonical
bridges and the remaining Session-driven work are explicit convergence boundaries.

The target remains a modular monolith with one mutable owner per concept, explicit C++ phases and
generation-checked Player identity. `MapRuntime` currently uses staged synchronous commands under
the existing manager synchronization; a dedicated task per map and a production ECS are not
claimed. See [the accepted entity-world ADR](adr-map-runtime-entity-world.md) and
[the current checkpoint](../architecture/session-578-checkpoint.md) for actual cuts and evidence.
Old LOC, warning and test counts elsewhere in this document must not be reused as current metrics.

---

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
