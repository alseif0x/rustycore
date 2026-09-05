# Session convergence checkpoint — updated 2026-09-05

Issue #578 remains open. This is an exact inventory reconciliation, not the terminal #153
audit, a full C++ parity approval, or a live-client acceptance report.

Initial reviewed source: `74daf3f9` plus the active-Player relocation and borrowed grid-capability
slice committed with this checkpoint. The prior runtime family membership was last edited
at `9a29e195`; the prior syntax snapshot was last edited at `26f72455`. Neither described the
current source. The historical persistence snapshot is deliberately unchanged: ordinary
iteration uses `session-ownership-check check --syntax-only`, not an exhaustive persistence scan.

## Exact membership and remaining work

After the 2026-09-05 talent-tab capability slice, the AST has **717 WorldSession fields:
285 production and 432 test fixtures**. The runtime
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

- 134 production catalog/configuration/service fields still reside on Session. Required
  construction is not enough: the owning vertical must consume the narrow capability.
- The map/runtime family still has 20 production fields, including both map-manager handles,
  creature scheduling/delivery state and GameObject state. Keep one clock per responsibility;
  remove Session map selection/gameplay and the remaining legacy/canonical bridges incrementally.
- Inventory/loot/economy has 15 remaining production members, spells/progression 16,
  movement/combat seven, social three, and the unresolved residual 18. The exact field lists
  remain executable ledger data; their inclusion does not endorse their current owner.
- Handler and external Session impl bodies still coordinate gameplay. Moving data to Player
  does not itself complete the decode/adapt/encode boundary.
- Public mutable Map access and final runtime-owned grid materialization remain open.
  The generation-checked lifetime coordinator still uses an outer manager mutex, not an actor
  handoff. Full persistence/bridge inventories and live acceptance remain terminal gates.

`SessionResources` has eight required aggregate fields (`core`, `inventory`, `player`, `spells`,
`world`, `progression`, `runtime`, `realm`), rather than 273 flat fields with 216 optional slots.
Their immediate capability types contain respectively 5, 30, 22, 34, 29, 21, 20 and six members:
**167 first-level members, plus further nested handler/persistence bundles**. Glyph and
talent-tab catalogs are required members of the process-owned PlayerBootstrap catalog,
borrowed by login/learning instead of installed on Session. The hotfix
dependency now lives in the nested, process-owned handler capabilities instead of the
Player catalog bundle and is borrowed by its consumers. The constructor
aggregate stays in world-server, not wow-network. Its `install_into_session_like_cpp` methods
still install many catalogs on Session, so eight fields are not evidence of final convergence.

## C++ contrast for this slice

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

### User-prioritized next front: automatic Dungeon Finder

The user approved this order: repair and verify current login first, then audit and scope LFG
as its own issue/branch. This does not close or silently defer #578's remaining ownership work.
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

The next scope should cover roles, join/leave, matching, proposals, group creation, teleport,
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

The next regression reproduces this difference without a live DB: initial map selection followed
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
