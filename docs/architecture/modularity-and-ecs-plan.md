# Native/Wasm modules, shared hooks and selective hecs — execution plan

**Decision date:** 2026-09-05. **Reviewed production code:** `93e4002a` on the
#578 branch; reviewed laboratory/planning HEAD: `ee9a0128`. This is a bounded
architecture review, not a new whole-port parity audit or an implemented ECS migration.

This plan supersedes the earlier “ECS review next”, unconditional backend-selection,
snapshot-only extension and automatic post-M6 Wasm directions. It preserves the full
[port plan](../migration/PORT_PLAN.md), useful completed ownership/module work, and
#578's complete C0–C4 acceptance. The user has approved the direction and plan update;
publication, deployment and destructive operations keep their separate approval rules.

**Latest decision — 2026-09-05:** select **private, selective `hecs`** for composable entity
state, retaining cohesive domain aggregates. Native Rust is the default execution path for
first-party and custom modules; **Wasm is a planned, operator-optional execution path of the
same extension contracts**, including a tested second source language. Hooks, state/lifecycle
and host integrity are shared, not two independently designed gameplay APIs.

This is an architectural selection now, not a claim that implementation acceptance has passed.
It supersedes the earlier preferred-candidate wording, the proposal to defer selection until
production integration, and the proposed three-backend preselection contest. A finite independent
module/conformance proof runs **before production storage migration** (§5); it can falsify the
selection through a named backend limitation, not keep it perpetually undecided. That proof has
not run. The completed [V1 laboratory](modularity-lab-results.md) contains 34 contract checks and
120 corrected-campaign samples on aarch64, not an arbitrary-module or multilang SDK proof.

**Explicit scope expansion:** #583 now includes the bounded Wasm execution/second-language
delivery, not just trusted Rust. Consequently #153/#133 closure requires that delivery. The
broader #99 language/ecosystem roadmap still has an M6 re-audit; this bounded delivery no longer
waits for M6. No new micro-issues, production dependency, deployment or code publication follows from
this plan update. C0–C4 and all existing durability/operator acceptance remain intact.

## 1. Outcome and current evidence

RustyCore should support useful gameplay extensions in independent repositories without
forking core implementation. The target is a **modular monolith with capability-specific
contracts**, not merely more files/crates, a universal plugin framework or microservices.
Base-server behavior remains anchored to TrinityCore-derived 3.4.3 C++; optional custom
behavior has an explicit contract and cannot bypass core integrity.

Throughout this plan, zero/disabled modules means zero **optional extensions**. Required
first-party base scripts remain enabled; neutrality must not remove behavior needed for C++ parity.

| Boundary | Implemented at the reviewed code | Acceptance still required |
| --- | --- | --- |
| Canonical Player/Map authority | Migrated Player families, generation-checked active/detached lifetime, retirement of whole-Player Session write-back and directory copies | Complete lifetime/save, operations, phase/publication and inherited boundaries in #578; see its [checkpoint](session-578-checkpoint.md) |
| External source/build modules | #228–#231: API, compositor/lock, CLI/skeleton and typed configuration; real `player.login → message` | Stateful behavior, composable policies, durable state/reward and operator lifecycle in #583 |
| Private entity storage | Production private `HashMap` entity records; `hecs` only in isolated experiments | Selected selective `hecs`: pre-migration conformance and real-owner integration under #578; external-consumer validation under #583 |
| Schema migration | `rustycore-db` owns immutable checksummed migrations and fail-closed startup compatibility | Module artifact/history retention, state upgrade and recovery workflow under #583 |
| Wasm execution | Core Wasm exercised only in the isolated lab; no production sandbox | Bounded Wasm adapter and second-language module in #583, optional to enable; hot reload remains excluded |

The external module product is tracked by #99; this milestone's concrete implementation
is #583. Closing #133 requires #578 **and #583**, followed by #153's independent audit.
It does not require closing the entire evolving #99 ecosystem epic.

## 2. Authorities and dependency direction

Internal organization follows [module design and source navigability](module-design-guidelines.md):
each completed family must have both a correct owner/narrow dependencies and manageable physical
production/test files. The guide defines project budgets, bounded exceptions, a Rust submodule
skeleton and incremental legacy retirement. It supplements this plan; ECS and extension hooks do
not discharge physical decomposition. #578 C2/C4 own the remaining core cuts and physical checker
extension; #583 applies the policy to its own SDK/modules, and #153 verifies both completed macros.

| Responsibility | Canonical authority | Extension access |
| --- | --- | --- |
| Identity, incarnation, residence and transfer | Named Player lifetime authority coordinated with Map admission | Scoped handles/queries; never storage identities or a second Player mirror |
| Inventory, money, combat, progression and their invariants | Cohesive domain owner for each operation | Validated decisions/actions; host enforces integrity and publication |
| Map-local simulation, creature/encounter lifecycle | Admitted Map runtime phase and entity/domain owners | Bounded synchronous behavior capabilities |
| Group, account and matchmaking/LFG state | Their own explicitly named lifetime/domain owners | Domain-specific commands/results; not everything belongs to Map or ECS |
| Custom rules and module state | Module defines its rules/schema; host controls scoped access and lifecycle | Namespaced state and declared capabilities |
| Durable I/O, transport, composition | Persistence adapters, connection owner and composition root | Owned inputs/results; no SQL connection, packet writer or resource bag |

One logical authority does not require one giant struct or one OS/Tokio task per map.
Private storage may change without changing public gameplay contracts. Keep cohesive
aggregates for invariants; do not split every scalar into a component or move all Session
responsibilities into a new Map god object. Typed internal content adapters remain possible;
official scripts need full C++ fidelity and are not automatically a stable third-party API.
Internal and external adapters must share documented semantic call points without accidentally
executing the same hook twice.

### One module product, two execution adapters

- **First-party/custom is provenance, not an execution mechanism.** Both can use the public
  module API. First-party base-server implementation may retain private internal APIs where full
  C++ fidelity needs them; that is not a hidden privilege promised to every external module.
- **Native Rust:** source-built independent crates, reproducible composition, explicit rebuild
  and restart. No stable dynamic Rust ABI, sandbox, panic containment or hot reload promise.
- **Wasm:** select Wasmtime as the initial host implementation (V1 tested 47.0.3), with a
  versioned Core Wasm ABI and generated/documented bindings. Start with Rust and a small C
  reference guest to prove a non-Rust producer. This is a support matrix, not a claim that any
  language/program compiling to Wasm already works. WIT/Component Model is not selected by the
  Core Wasm test; introducing it requires preserving the same synchronous callback contract.
- The operator chooses exactly one artifact/executor per module identity. Native and Wasm
  modules may coexist and compose; the same module must never execute twice because both
  artifacts are present. Startup rejects duplicate identity, incompatible API/state versions,
  unavailable required capabilities and conflicting exclusive policies before callbacks run.
- The shared contract defines typed inputs/results, error categories, lifetime, semantic hooks,
  state versions and capabilities. Native types and Wasm encodings need not have identical
  layouts. Neither public adapter exposes backend IDs, query guards or generic mutable entities.
- A module defines namespaced state/schema; the host controls admission and lifetime. Native
  state uses registered typed access, not a new central enum variant per module. Wasm uses
  bounded versioned records/opaque state access through the ABI, not Rust `TypeId`, pointers or
  guest structs inserted directly into ECS. Name the canonical physical owner for each scope.
  Schema version is not mutation revision: use short admitted access or incarnation/revision-checked
  writes, rejecting stale outer snapshots after nested callbacks. Representation optimizations may
  differ without creating a second authority for the same state.
- Switching native <-> Wasm with existing state requires an executor-independent durable format
  or an explicit validated conversion. An unsupported switch is rejected before callbacks, with
  data/history retained; no implicit reset, purge or migration merely because the executor changed.
- Wasm runs without ambient filesystem/network/DB access; host imports are explicit capabilities.
  Bound guest memory, execution fuel across the entire nested invocation, callback depth, host
  actions, payloads and host-side allocation/work. Fuel does not interrupt blocking host calls.
  Trap/error handling preserves already applied actions and records module/scope/call provenance;
  fail-open, reject or disable behavior is explicit per hook, never accidental executor behavior.
- Deliver a native-only build and a Wasm-enabled build, mixed-executor tests, pinned source/artifact
  provenance and the same upgrade/disable/data-retention rules. Loading a new Wasm artifact on
  restart does not require recompiling core, but does not imply hot reload of a running encounter.

## 3. Three extension contracts, not one generic event bus

| Contract | Purpose and timing | Required semantics |
| --- | --- | --- |
| Pre-decision policy | XP/progression or difficulty contribution at a named decision point | Typed validated contribution; deterministic ordering, conflicts and provenance; core retains final integrity checks |
| Scoped behavior | NPC/encounter decisions inside an admitted owner phase | Bounded queries, actions and immediate results where C++ requires them; explicit reentry/partial-effect rules |
| Confirmed notification | Observe a completed transition at its documented confirmation point | Cannot retroactively veto a committed operation; owned payload, delivery/retry semantics and idempotency where relevant |

Every supported hook records: owner, exact C++ anchor or custom contract, admission, before/after
phase, input freshness, action/result, state scope, composition/conflict rule, failure/reentry,
persistence/publication and observability. “Confirmed” means the named operation boundary;
do not imply that every in-memory combat tick is durably committed to SQL.

Use narrow scoped capabilities, not `&mut Player`, `Map`, `hecs::World`, raw runtime IDs,
entity guards, generic queries, SQLx pools or packet writers. An operation may need a query
after an action; a flat deferred effect batch is not sufficient for every script.

Composition is explicit per contract: an ordered validated transform, an associative reduction,
or a declared exclusive policy. Do not silently use last-writer-wins. Each contribution carries
module/rule identity and a reason so the operator can explain a result and remove the relevant
contribution without overwriting another module's work. Bounds/overflow and conflicting exclusive
policies must have tested rejection behavior. The existing compositor's `(order, id)` registration
order and login registry's `ModuleId` callback order are different current contracts; #583 must
define the intended policy semantics rather than assume they already coincide.

### Synchronous behavior, reentry and failure

The C++ Anomalus path changes phase/casts before attempting a summon; failure does not undo
the earlier effects. Summoning may synchronously call `JustSummoned`/`IsSummonedBy` before
returning. Another path performs an action then reads the boss aura to choose its next timer.
Evade can synchronously call Reset. These constrain the host, not just the storage library.

Before enabling behavior, choose and test either short host-managed state accesses released
before a reentrant action, or explicit continuations preserving the same semantic barriers.
Do not retain a mutable module-state/ECS borrow across callbacks, defer all nested callbacks to
the next tick, or promise rollback of a whole callback after earlier actions succeeded. Record
action failure, partial effects, recursive dispatch limits and the safe outcome of exceeding
them. Revalidate incarnation/residence at each admitted action; a saved runtime handle is not
authority forever. Reentry guards must not silently suppress C++-required behavior.

No synchronous map/entity guard crosses `.await`; no blocking I/O or packet delivery occurs
under a map lock. Deliberate async operation gates may span I/O only with explicit lock order,
blocking scope, cancellation and recovery contracts. Do not remove established money/persistence
fences merely to satisfy a blanket “no locks” slogan. Results requiring later I/O use owned
projections and generation/revision-safe completion under the correct owner.

## 4. State, identity and durability

Classify every state field as core authority, module-owned authority or reconstructible cache.
Declare its scope: incarnation, encounter, map instance, character or account. Character-scoped
state follows a valid transfer; map-scoped state has an explicit detach/unload policy. Reset,
despawn, failed attach, replacement, logout and shutdown must dispose, retain or transfer it
deliberately. Detached Player state remains valid; active-map operations may return `NotActive`.

Separate three identities: protocol GUID, private incarnation-checked runtime handle, and durable
module scope/key. Never persist an ECS entity ID or treat a reused GUID as the old incarnation.
Persist only declared durable fields under module ID, scope/key and schema version. Do not
automatically serialize every component, transient boss timer or creature reference; C++ itself
normalizes transient encounter states on load.

For a completion reward, the reward and its durable receipt must be coherent. Prefer a single
authoritative transaction when both belong to the same database. Across databases, specify an
idempotent operation token and recovery protocol; do not claim distributed ACID or derive an
exactly-once guarantee from in-memory flags. Unknown COMMIT, retry, concurrent/newer mutation,
cancellation and restart must neither duplicate rewards nor acknowledge lost progress. Keep
the same logical reward/operation identity across retries, configuration or schema upgrades,
and new runtime incarnations; a new schema version must not accidentally grant the reward again.

`rustycore-db` remains the sole schema migration authority. A module never runs arbitrary SQL
from a callback or startup hook. Compatibility, target DB, namespacing, immutable checksums,
dry-run, approval and incomplete-migration recovery apply to module migrations too. Retain
required migration manifests/artifacts and applied history after removing a module checkout;
the current DB compatibility check rejects history absent from the manifest used by the binary. Archived
entries alone are not proof that missing SQL artifacts are acceptable.

Treat **disable execution**, **remove installed code while retaining data/history**, and
**purge durable data** as different operator actions. Disabling stops callbacks according to a
documented drain/restart contract and removes reversible contributions; it does not roll back
legitimately earned rewards. Re-enable and upgrade have tested state compatibility. Purge or
destructive downgrade requires explicit authority; no automatic destructive down migrations.

## 5. ECS decision now: selective private `hecs`, cohesive aggregates retained

**Selected:** `hecs` (initial pinned baseline 0.11.1) for map-local, independently composable
entity/behavior state behind the canonical owner. Keep cohesive Player/Unit/domain aggregates
where they enforce complete invariants. No global ECS, scheduler replacement, public component
queries, wholesale Player decomposition or obligatory intermediate dense-arena migration.
Catalogs, accounts, matchmaking/LFG and durable I/O retain their own owners outside this choice.

### Rationale and alternatives

| Option | Decision and evidence |
| --- | --- |
| Current/improved aggregate + state registry | Viable modular implementation and the fallback; not rejected as incapable or unsafe. It does not currently implement the open state registry either. Keep aggregates where composition does not justify a change. |
| Dense generational aggregate + registry | Credible layout alternative, **not benchmarked**. Do not claim hecs beat it. Adding it first would introduce another migration without evidence that it solves an unmet requirement better. |
| Selective hecs | Chosen for typed optional-state composition and storage/query machinery without imposing a system scheduler. V1 supports feasibility at tested costs; the expected reduction in application-specific composition plumbing remains an architectural judgment to verify. |
| Broader ECS framework | Not selected: no demonstrated need for another resources/events/scheduler framework. This is not a claim that Bevy requires a renderer or cannot preserve our driver. |

V1's median paired update-p99 ratios are 0.548–0.744 against its three-HashMap aggregate, but
the timed path includes materializing observable rows. Churn costs 1.49–1.69x and transfers
1.31–1.64x as much. The fixture knows two state types and enumerates their combinations. These
numbers do **not** establish a production speedup, arbitrary module composition or superiority
over a dense store. The selection combines demonstrated feasibility with the desired composition
capability; it is not a claim of experimentally proven global optimality.

### Finite conformance proof before production migration

This is the **next authorized implementation checkpoint**, within #578, not a new issue or
completion of #583's SDK. The earlier three-candidate experiment becomes validation of the
selected design, not a prerequisite to naming the choice. Preserve the useful falsification test:

1. Define one private experimental host contract, anchored to the represented C++ owner/callback
   paths and clearly named custom behavior. Implement two independent modules; then freeze the
   host, adapter and contract sources and record their hashes.
2. Add a third module in a separate crate with a new state type and lifecycle rule. Only dependency,
   declarative registration and composition may change. No new host enum/match arm naming that
   type, module-specific storage adapter, broad entity borrow or exposed hecs/SQL/packet API.
3. Exercise zero optional modules (required base scripts remain), mixed/composing modules,
   conflict rejection, state isolation and bounded
   action -> synchronous callback -> read, including nullable action failure and failure after
   prior effects. Cover reset/removal, active/detached transfer, failed attach, replacement and
   stale incarnation, plus versioned snapshot/replay. Include outer read -> nested state mutation
   -> stale outer write: reject the obsolete write and retain the nested result. Label replay as
   a mock, not DB durability. Reject unsupported executor switches without discarding saved state.
4. Run equivalent cases as native Rust, Rust -> Core Wasm and C -> Core Wasm using the same
   semantic contract; also compose native and Wasm modules in one host. Test duplicate executor
   rejection, incompatible versions/capabilities and the resource/failure limits above.
5. Before measurement, fix populations, workloads, repetitions, hash set and CPU/RSS/action/state
   budgets in a new versioned protocol. Measure update, churn, transfer, dispatch and cold costs
   separately; retain every failed sample. V1 provisional budgets are not a server SLA. Report
   central code touched and state/lifecycle plumbing as well as timing; no favourable-run selection.
6. Record pass/fail of the selected implementation and exact remaining production boundaries.
   Continue into affected #578 integration only after this proof passes; do not postpone it until
   after production migration or the entire #583 SDK. This planned proof has **no result yet**.

An implementation error means fix and rerun the affected case. A Wasm/ABI defect is not evidence
against ECS. Reopen the backend decision only for a demonstrated hecs-specific obstacle such as
unavoidable duplicate authority, inability to support the independent-state/lifetime contract,
or unacceptable measured structural cost after bounded correction. Then compare against the
aggregate + generic registry fallback (a dense library only if layout is the diagnosed issue).
No perpetual candidate carousel, no frozen second live authority, no waived correctness gate.

After that proof, #578 still must exercise real save/admission/phase/two-map/backpressure/shutdown
paths and retire the superseded writers for each migrated family. #583 delivers the production
external-module and durable operator lifecycle. The [entity-world ADR](../migration/adr-map-runtime-entity-world.md)
records the selection and integration gates. Production still has no hecs dependency today.

## 6. Complete execution sequence and ownership

| Macro / epic | Deliverable and completion gate | Dependencies |
| --- | --- | --- |
| #578 / PR #579, under #133 | Pre-migration conformance of selected hecs; all C0–C4: admitted execution, Player lifetime/save, complete operations, runtime/publication/bridge retirement and final boundaries | Existing completed #378/#574; isolated native/Wasm conformance does not depend on a production SDK or #583 |
| #583, under #133 and #99 | Real external stateful modules, shared hooks, native/Wasm execution including a C reference guest, durable reward/state and complete author/operator lifecycle | #231 and #578 merged; contract research/conformance runs now inside #578 |
| #153 | Independent terminal audit of the complete #133 contract and evidence from both macros; known work is fixed by its implementation owner | #184, #578 and #583; not the closure of epic #99 |
| Next Part-1 port macro | Re-audit its actual residual path against current Rust/C++; implement complete gameplay responsibility with existing hard dependencies | Ordered #49 index, retaining M0–M6 and relevant prerequisites |
| #48 / Part 2 | Full 1:1 ledgers, nothing dropped; fresh audit/planning after playable #47/M6.2 | Existing Part-2 transition gate; no speculative child tree now |
| #99 future extensions | Expand the public API through real consumers and reusable semantic seams | Evidence-led; no issue/PR per field or callback |
| Wasm bounded delivery / broader language ecosystem | #583 delivers an optional executor with Rust/C bindings and explicit limits; further languages, WIT or hot reload are not implied | No M6 gate for the approved bounded #583 delivery; broader #99 expansion retains fresh planning |

First run the finite pre-migration conformance checkpoint above; the architecture selection is
already made. This sequence gates production storage migration, not safe same-owner source/test
splits, which may precede or run alongside the experiment. Then complete #578 lifetime/save and
C0 phase proof alongside whole-operation C2 integration, followed by C3/C4. Preserve the committed
Player work. The production module product
is one subsequent #583 PR, not a series of greeting-size deliverables. Internal focused commits
and checks are not user approval gates; runtime/publication authority remains separate.

### Reanalysis checkpoints — evidence before replication

These reviews happen inside the approved macros; they do not create another issue, PR or
routine approval request. Continue authorized work after a passing checkpoint. A failure pauses
the affected migration while its cause is investigated and corrected within scope.

| Point | Question the evidence must answer |
| --- | --- |
| Before production storage migration — #578 conformance (§5) | Can the frozen host accept independent state and native/Rust-Wasm/C-Wasm composition without module-specific core edits, duplicate authority, stale reentrant writes or violated limits? This finite gate can falsify a concrete backend premise; V1 did not pass it. |
| First production C1/C2 vertical, with its C0 contract — before replicating the pattern | Does one complete operation work through real admission, canonical lifetime/save and ordered publication, including controlled I/O, late acknowledgements, replacement/detach and affected phase/backpressure failures? Check semantic ownership and physical source/test boundaries together. A fixture-only success or renamed phase cannot justify scaling the pattern. |
| C4 — complete #578 balance, before #583 starts production integration | Do all C0–C4 exits hold at the validated SHA, including every remaining owner/bridge, inherited decision, persistence classification and physical-file exception? Reconcile the whole macro, not only its last successful vertical; #583 must not inherit unfinished Session work. |
| #583 first real external-module integration, before extending its API | Can independent authors exercise the shared hooks and state contract without a core patch? Validate the supported native/Wasm behavior and real durable/operator lifecycle as they become available; do not extrapolate from a greeting or mock replay. |
| #153 after both macros merge | Independently audit the complete #133 acceptance and evidence at integration HEAD. Known implementation work stays with #578/#583, not the auditor. |
| After architecture, then #47/M6.2 | Re-audit each selected gameplay macro just in time. At the playable exit, perform the fresh whole-port state/plan review before decomposing Part 2/#48; architecture closure is not full-port parity. |

The first production vertical is a focused design stress test, not a second global architecture
audit. Repeat a broader review earlier only when evidence invalidates a shared contract or the
approved scope materially changes; do not wait for #153 to discover a pattern-wide defect.

### #583 reference capability and acceptance

Use one externally maintained encounter/progression module and a second separately configured
module contributing state/policy in the same scope. The reference covers an XP/progression
decision, transient encounter phases/timers and failing/reentrant actions, and a durable
completion/reward. Select concrete supported actions against merged #578 and record exact
anchors before enabling them. This is not an implicit commitment to port all Nexus content,
all spell effects or LFG inside the architecture issue; missing required paths cannot be replaced
by stubs and called acceptance. Necessary bounded host seams belong in #583, unrelated gameplay
gaps stay with their port owners.

| Proof | Required evidence |
| --- | --- |
| Useful independence | Separate module repositories/commits, public API only, no module-specific core branches; zero-module neutrality and explicit custom behavior |
| Execution portability | Native-only and Wasm-enabled builds; equivalent Rust native/Rust Wasm/C Wasm cases; mixed executors, no duplicate execution, ABI/capability/version rejection and bounded failure behavior |
| Stateful behavior | Real owner paths; action success/failure and read-after-action; synchronous reentry; timers/reset/despawn; second module composition/conflict |
| Lifetime | Two sessions/maps, active/detached transfer, stale generation/replacement, failed attach/unload and shutdown |
| Durable progress/reward | Restart/relogin, duplicate operation, rollback, unknown COMMIT, cancellation and newer mutation; coherent receipt and reward |
| Author/operator lifecycle | Real Git v1 install → v2 update → build/restart → disable/re-enable/remove; lock reproduction, compatibility rejection and retained migration history |
| Diagnostics | Explain contributions/order/conflicts; structured failure results and bounded state/callback diagnostics without secrets |

The CLI currently rewrites Git manifest provenance during install; the subsequent dirty-check
path is an **inspection-based upgrade risk, not a reproduced failure**. #583 must test the real
Git workflow, fixing it if necessary, rather than relying only on path-source fixtures.

## 7. Validation, stop conditions and future decisions

Terminal acceptance includes independent physical-file and logical-owner reports under the
module design guide, including tests/fixtures and file-specific exceptions. The existing checker
currently enforces selected logical ceilings, not the new physical policy; #578 C4 implements
that extension and retires the remaining core monoliths before #153's audit. No permanent Session
exception, known future split plan or moved-file count substitutes for completion.

Use focused positive/negative tests and inexpensive architecture/syntax checks during a cut.
At affected boundaries, use production-linked dev/release integration tests, controlled I/O
interleavings and explicit ticks. Tests must use the production composition/dispatch path;
`cfg(test)` fixture worlds and constant phase strings do not prove it. Before macro publication,
run clean-HEAD `validation-v2 final` plus the issue's complete acceptance and inventories.
Do not repeat exhaustive persistence scans for every internal helper or metadata change.

Capture-diff is required for changed bytes, metadata, connection selection or observable order;
fresh action-specific captures are distinct from existing regression goldens. Live lifecycle
changes need authorized runtime QA. Real MariaDB crash/restart/relogin evidence is required for
durable recovery claims; controlled mock futures cannot prove storage durability. Missing live
authority pauses that mutation/acceptance step, not safe inspection or remaining local work.

Record evidence kind (old-Rust equivalence, C++ contract, production integration, live/capture),
SHA, command, result, host architecture and unproven boundaries. Planning, successful builds,
test counts and moved fields are not gameplay completion percentages. #153 audits a completed
delivery; it must not receive a bucket of unresolved SDK, persistence or owner implementation.

Native modules are trusted source Rust: rebuild and restart, no stable native ABI, hot reload,
sandbox, guaranteed panic isolation or preemptive CPU budget. Capabilities constrain the API,
not native code's filesystem/network access. The selected Wasmtime/Core Wasm adapter must earn
the ABI, resource, host-capability, failure and lifecycle acceptance above before being enabled.
Its optional installation does not make its #583 acceptance optional. Blocking analytics/webhooks belong outside the
synchronous simulation path and need no in-process ECS access.

## 8. Evidence and source anchors

Recheck locations against the implementation HEAD; these support the decision, not full parity.

- Rust: `crates/wow-map/src/map/entity_world.rs`, `crates/wow-map/src/map/runtime.rs`,
  `crates/wow-map/src/manager/player_owner.rs`;
  `crates/wow-module-api/src/{hook,effect,registry}.rs`; `tools/modules/{compose.py,rustycore-module}`;
  `crates/wow-database/src/migration.rs`; `tools/architecture/map-runtime-spike`.
- C++ under `/home/server/woltk-trinity-legacy/src/server/`: `game/Entities/Player/Player.cpp:2189-2226`
  (XP hook order); `scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:81-170,232-241`
  (state, partial failure and read-after-action); `game/Entities/Object/Object.cpp:1956-1972`
  and `game/Entities/Creature/TemporarySummon.cpp:249-264` (synchronous summon callbacks);
  `game/AI/CreatureAI.cpp:219-242` (evade/reset); `game/Instances/InstanceScript.cpp:374-473`
  and `game/Instances/InstanceScriptData.cpp:137-142` (durable transitions and transient reset);
  `game/Maps/Map.cpp:666-813`, `game/Maps/MapManager.cpp:287-318` and
  `game/Server/WorldSession.cpp:64-108` (execution/admission/barriers).
- AzerothCore is a product/capability reference, not the 3.4.3 behavioral oracle:
  [module structure](https://www.azerothcore.org/wiki/the-modular-structure),
  [hooks](https://www.azerothcore.org/wiki/hooks-script),
  [AutoBalance](https://github.com/azerothcore/mod-autobalance).
- Storage alternatives: [hecs](https://github.com/Ralith/hecs),
  [HashMap disjoint mutation](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get_disjoint_mut),
  [slotmap](https://docs.rs/slotmap/latest/slotmap/),
  [Bevy ECS](https://docs.rs/bevy_ecs/latest/bevy_ecs/).
- Wasm: [language support varies](https://component-model.bytecodealliance.org/language-support.html),
  [Wasmtime resource configuration](https://docs.rs/wasmtime/47.0.3/wasmtime/struct.Config.html).
  Component-language tooling is contextual evidence, not proof our Core Wasm ABI supports it.

## 9. Earlier plan synchronization evidence — 2026-09-05 (before the latest decision)

On the aarch64 development host, with production HEAD still `93e4002a` and planning changes
uncommitted above `32d9a683`:

- Created #583 and updated/read back the exact bodies of #133, #578, #153, #99 and #49.
  All remain open; completed predecessor states are preserved. #99's title now makes Wasm optional.
- The architecture ledger records #583 with parents `[133,99]`, prerequisites `[231,578]`,
  and #153 prerequisites `[184,578,583]`; its documented topological order agrees.
- `check_architecture.py check` and `self-test`: PASS; syntax-only Session ownership: PASS.
- Preserved persistence issue-reference/classification check and
  `checked_persistence_policy_matches_the_checked_snapshot` (release): PASS. The 60 historical
  nonstable groups still attributed to #153 are preserved; #578 C4 retains their semantic
  reconciliation obligation. No exhaustive persistence inventory was regenerated.
- Live `check_architecture.py refresh-issue-state --check`: PASS; read-only remote validation
  required network permission. The check did not rewrite other policies or ledgers.
- `git diff --check`: PASS. Independent adversarial review found no dependency cycle and led
  to explicit shared C0 workload contracts and stable reward identity across upgrades/retries.

These are planning/metadata checks, not clean-HEAD macro-final validation, a live module demo,
gameplay parity or a new benchmark. No production code, runtime, schema, migration inventory,
PR contents or dependency allowlist changed. No commit or push was made. Earlier skill edits
and the unrelated local LFG audit remain separate from this plan update.

## 10. Latest decision synchronization and validation — 2026-09-05

At laboratory HEAD `ee9a0128`, before the documentation commit:

- Updated and read back exact title/body/state for GitHub #133, #578, #583, #153, #99 and #49.
  All remain open. #99/#583 titles now identify native/Wasm; #583 explicitly expands #133's
  closure. Removed the contradictory post-M6 prerequisite and Wasm exclusion for this delivery.
- The existing approved DAG is retained: #578 depends on #378/#574; #583 on #231/#578;
  #153 on #184/#578/#583. No new issue or inverse SDK dependency. Ledger/title/doc synchronization
  and live `refresh-issue-state --check`: PASS.
- On aarch64, `check_architecture.py check` / `self-test`: PASS; 38 packages, 101 workspace
  edges. `session-ownership-check check --syntax-only`: PASS, 282 production + 432 fixture fields.
- Preserved persistence policy/workflow reference/classification check: PASS, 120 references.
  `checked_persistence_policy_matches_the_checked_snapshot` in release: 1 passed, 0 failed.
  No exhaustive persistence scan or baseline regeneration; the historical #153 reconciliation
  obligation stays in #578 C4.
- `git diff --check`: PASS. Independent design reviews support the scoped hecs choice and led
  to explicit stale-outer-write and native/Wasm state-switch rejection acceptance.

These checks validate planning/metadata consistency, not the unimplemented conformance gate,
new benchmark results, production ECS/Wasm integration, DB durability, live/capture behavior or
clean-HEAD macro-final acceptance. No runtime/code, dependency allowlist, DB, PR or issue state
was changed. This documentation commit preserves the earlier approved plan changes while leaving
skill edits and the unrelated LFG audit outside it; no push or merge is included.

## 11. Module-design policy adoption — 2026-09-05

Reviewed above planning HEAD `816d5c84`, with production code still `93e4002a`:

- Updated/read back exact GitHub bodies #133, #578, #583, #153, #99 and #49. Titles, states
  and the existing dependency DAG are unchanged; all six remain open. No new issue or PR.
- Added [module design guidelines](module-design-guidelines.md): independent physical and
  semantic acceptance, project size budgets, bounded exceptions, test/fixture decomposition
  and an illustrative Rust submodule skeleton. #578 C2/C4 own remaining core decomposition
  and the physical checker extension; #583 applies the policy to its own module product.
- Updated the existing architecture and safe-refactor skills, preserving the earlier approved
  simplification. The canonical policy stays in project docs, not duplicated skill snapshots.
  A four-scenario independent instruction review led to conditional opcode checks, a thin
  `main`/composition distinction and an explicit non-universal persistence example. No new
  routine approval or micro-PR requirement was added.
- Only three narrative hotspot-ledger fields changed; numeric ceilings, members, paths,
  issue references and the historical persistence snapshot remain identical to HEAD.
- On aarch64, architecture check/self-test, syntax-only Session ownership and live
  `refresh-issue-state --check`: PASS. Both skill validators and `git diff --check`: PASS.
  `checked_persistence_policy_matches_the_checked_snapshot` (release): 1 passed, 0 failed.
- Bounded validation against the persistence policy rules: PASS, 1,041 classified groups,
  1,038 workflows and 120 preserved issue references. Policy/workflows/snapshot are identical
  to HEAD; historical #153 attribution still requires C4's semantic reconciliation. This is
  reference/classification consistency, not a fresh exhaustive source inventory or DB evidence.

This is policy/planning and metadata validation, not implementation of physical enforcement,
source decomposition, production ECS/Wasm migration, gameplay parity, live/capture acceptance
or clean-HEAD macro-final validation. No production source, dependency, schema, runtime or PR
was changed. The unrelated local LFG audit stays outside this change; no push or merge.
