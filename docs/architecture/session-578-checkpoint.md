# Session convergence checkpoint — updated 2026-09-05

Issue #578 remains open. This is an exact inventory reconciliation, not the terminal #153
audit, a full C++ parity approval, or a live-client acceptance report.

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
The header intentionally retains its previous Session map/level staging and detached-health
projection; far teleport postponement (`Player.cpp:19327`) needs its own faithful transition
cut rather than being silently folded into this extraction. Equipment type/identity changes
across the existing two-table adapter are not generalized by this receipt. No new concurrent
writer is enabled. These boundaries prevent full C1/C2 or macro acceptance from this result.

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
checker PASS only enforces selected logical ceilings; it does not establish physical completion.

Safe same-owner mechanical source/test splits can precede or run alongside the selected-hecs
conformance experiment. That gate remains mandatory before production storage migration, not
before organizing files. Keep all C0–C4 obligations and PR #579; no helper issues or routine stops.

### C1: precise lifetime and save requirements

- A detached Player is a valid owner, not a missing Player. Persistent-state queries remain
  available; commands requiring an active map return `NotActive`. Missing, stale-generation
  and inconsistent residence must be distinguishable internally; never fabricate defaults.
- Attach/detach, failed transfer, map destruction/unload and generation retirement use the
  same lifetime authority. Review or restrict mutable Map escapes that can bypass it.
  `ManagedMap::remove_all_players` currently clears a counter, and destroy/unload can remove
  storage without reconciling residence (`crates/wow-map/src/manager.rs`). This is an API
  inconsistency to resolve, **not a demonstrated live loss**; no production caller was found
  in this review. C++ `Map.cpp:1629-1643` requests evacuation and
  `MapManager.cpp:322-339` refuses destruction while players remain.
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
