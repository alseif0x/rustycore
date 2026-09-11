# Module design and source navigability

**Allocation/status revision, 2026-09-11:** #133 was closed on 2026-09-09. The
remaining core semantic/physical work previously assigned below to #578 C2/C4 is
owned by coordination epic #584 and its analyzed crate-focused implementation
children. #578/#585/#587/#588/#589/#716/#718/#722/#737 are integrated and closed in
their bounded scopes. The technical gate remains #584 core → #583 native/Wasm
product → #153 independent audit; #583 does not block unrelated gameplay work, but
its production module product waits for required #584 work. Keep the Rust/Wasm/C
mixed product mandatory even though operator activation is optional. This changes
delivery allocation, not budgets, invariants or global terminal acceptance. No next
crate is selected. Each macro includes its consumers and both criteria below; #153
must not inherit implementation work. References to #578 below describe the former
allocation or historical evidence unless explicitly updated.

**Approved project policy: 2026-09-05.** Applies to RustyCore's internal refactors,
new code, integrated tooling and module SDK/examples. This is a maintained design
contract, not a claim that the current tree already complies. The initial review
used production code `93e4002a`, unchanged at planning HEAD `816d5c84`.

## 1. Two independent acceptance criteria

A completed responsibility must have **both**:

1. **Semantic modularity:** a named canonical owner, narrow inputs/results, explicit
   dependencies, preserved invariants and complete reader/writer migration.
2. **Physical navigability:** cohesive, manageable source and test files, with a
   discoverable module tree. A reader should find the rule, use case, adapter and
   tests without loading an entire subsystem's implementation.

Splitting a giant `impl WorldSession` into fifty files is useful mechanical progress,
not proof that Session no longer owns gameplay. Conversely, moving authority to Player
does not justify a 70,000-line Player file. A legitimate aggregate can span many private
modules while retaining one identity, one state and invariant-preserving operations.

## 2. Structure by responsibility, using Rust's boundaries

Use a **modular monolith, domain-oriented within the existing dependency layers**:

- Domain modules express rules, invariants and state transitions. They do not retain
  Session, SQL connections, packet writers or composition/configuration bags.
- Application use cases coordinate complete operations through narrow domain and
  persistence capabilities. They preserve admission, commit classification, canonical
  mutation and publication order; they are not a new all-purpose `GameService`.
- Adapters translate protocol/persistence representations and effects. Handlers decode,
  admit and invoke; repositories follow transaction boundaries, not one trait per table.
- Composition constructs concrete dependencies and supervises lifecycle. It does not
  become a second owner of gameplay.

Prefer named features such as `quests`, `inventory` and `combat` over growing global
`services`, `managers`, `helpers` or `utils` buckets. A small cohesive area can remain
one module; do not create four empty layers for every operation. DDD informs language,
invariants, aggregate boundaries and explicit relationships; it does not prescribe a
PHP-style directory tree, one crate per aggregate, or microservices. A feature folder
is not automatically a bounded context, especially when it shares Player invariants.

Use private modules/submodules first. `mod` declares a module; `use` only imports a
path into scope. Both `quests.rs` + `quests/` and `quests/mod.rs` + child files are
valid; follow the local convention and do not rename every `mod.rs` for style alone.
Keep root files as a small facade, declarations and essential wiring. Expose only the
required operations through `pub use` or deliberately scoped visibility; do not make
state public to move tests. Add a crate only for a useful independently checked API,
dependency/build boundary or real external consumer. No organizational marker traits,
new locks, cloned mirrors or untyped service locators merely to split source files.

Source layout does not select a storage engine. Private selective hecs, public module
hooks and native/Wasm execution retain the decisions and gates in the
[modularity/ECS plan](modularity-and-ecs-plan.md). None substitutes for the boundaries here.

## 3. Physical budgets and bounded exceptions

These are **project navigation budgets**, not universal Rust requirements:

| Physical handwritten file | Required treatment |
| --- | --- |
| Usually 200–800 lines | A useful target for cohesive files, **not a minimum** and not permission to pad or fragment code. Small facades and simple modules can be much shorter. |
| Above 1,000 lines | Review cohesion and the next natural split during the ordinary task review. The agent can resolve this with evidence; it is not a new user-approval gate. |
| Above 2,000 lines at a responsibility/macro closeout | Split by responsibility, or record a concrete file-specific exception with the evidence below. A generic legacy or aggregate exemption is insufficient. |

Count physical lines, including comments and blanks. Handwritten tests, fixtures,
integration tests and integrated tool sources count too; moving production code into
tests or another language/directory is not retirement. Keep production/test attribution
as useful additional data, not a way to bypass a large physical file. Source generated
from a reproducible generator is reported separately with its generator/input provenance;
do not relabel handwritten tables or fixtures as generated without that evidence.

Each exceptional file records its exact path, responsibility, observed count and reviewed
ceiling, implementation owner/issue, reason a coherent split is currently unsafe or less
clear, and a **bounded exit condition or named review checkpoint**. A temporary exception
expires at that checkpoint unless its evidence and ceiling are explicitly reviewed again;
being attached to an open issue does not renew it. A justified cohesive exception may remain
at closeout only with that explicit record. Session/Map/Player identity alone, an unfinished
ownership migration or the historical 4,000-line signal is never a standing exception.

Do not enforce a new global limit by blocking all work on pre-existing files. During
migration, inventory the legacy files, assign their responsibility splits to the existing
macro and use per-file non-growth ceilings. Tighten those ceilings after each coherent
validated reduction. Necessary focused tests or an inseparable in-scope transition may
increase a ceiling only through an explained, reviewed delta and a retained split exit;
never automatically refresh baselines to make checks green. Once a responsibility is
declared complete, its files must meet the terminal budget or the specific exception rule.

Retain the **logical-owner** inventory as a separate metric: it catches gameplay still
coupled to Session across many files. Logical totals are not subject to a per-file cap,
and physical splitting alone must not remove logical ownership debt.

### What a physical division cannot reach

**Review correction, 2026-09-10, #716:** the earlier examples describe limits of
item relocation, not proof that useful private decomposition is impossible. Preserve
their measurements without treating a parser item or a LOC ceiling as the design owner.

**Keep one implementation while delegating cohesive work.** Rust's single applicable
trait impl can contain short methods that delegate to private modules without changing
the trait, type or state owner. `player/lifecycle_adapter.rs` already has `economy`,
`login_reads` and `save_plan` children; its bodies can use those responsibilities.
An exhaustive `match` can retain its dispatch and delegate branch bodies. A large
`CharStatements` enum may instead justify a cohesive file-specific exception. Evaluate
these cases separately; a single item does not establish a blanket exemption.

**Review wiring growth without erasing semantic debt.** The hotspot ratchet counts
the whole owner, so private module declarations/imports and formatting can raise its
physical line total even when operations are unchanged. #713 measured +63/+43 test
lines for loot/quest fixture splits; #697 recorded +6 Session lines from path wrapping.
Those are costs to review, not evidence that the new module boundary is wrong. An
explained delta may update only the affected ceiling while preserving the logical owner,
its complete descendants and retirement obligation, as the existing policy permits.
Keep the before/after evidence; never reset all baselines or count relocation as retired
gameplay ownership. Adding a helper does not require a new issue or approval round.

**Keep one operation coordinator and its ordering contract.** Long operations such as
`handle_void_storage_transfer_with_generators_like_cpp` and the spell-effects executor
can extract cohesive private phases while retaining one coordinator, authority and
transaction/publication sequence. Review captured values, borrows, cancellation, early
returns and failure paths before extraction. If a faithful split is unclear, retain a
specific bounded exit; do not infer that the operation must remain one physical function.

**Names and imports are part of navigability.** New private files name a rule, operation,
adapter or scenario. `state_1`, `ops_2` or `scenarios_14` is a temporary relocation label
unless its number has domain meaning. Reunite definitions and operations by responsibility
when completing the family. Prefer explicit production imports and deliberate facade
exports; `use super::*` in a small test or a transitional `#[path]` mount can remain when
its real dependencies, visibility and analyzer coverage are preserved. Do not widen state
visibility merely to make a split compile or replace these criteria with a zero-glob quota.

## 4. Tests and a complete operation

Keep small private unit tests beside the rule, and split larger suites by responsibility
and scenario. Share narrow fixture builders only where genuinely reusable; do not replace
`session_tests.rs` with a giant `test_support.rs`. Preserve every test registration, feature
gate and production/private behavior under test. Renaming or relocating tests must not
silently reduce the set executed. Production-linked integration tests remain distinct
from `cfg(test)` fixture-only paths.

For example, accepting or rewarding a quest spans protocol admission, domain eligibility,
an application operation, the appropriate persistence contract and ordered publication.
The exact sequence follows the C++ operation and existing durability guarantees, not a
universal template that adds a database transaction to every action. Domain rule tests,
application failure/interleaving tests and packet/capture tests belong with those boundaries.

Record before/after files and sizes, the final owner/dependencies, retired access/bridges,
and focused evidence for each completed family. Keep useful small implementation commits
inside the approved macro. Neither a file split nor a green size check is the whole task.
Do not create a PR, issue or user confirmation for every helper.

## 5. Example skeleton — target shape, not an implemented directory migration

The following selectively expands current crates. Names are illustrative; preserve actual
public paths and registrations during migration. Each directory has a small module root
(`mod.rs`, omitted below where uninteresting), not an implicit auto-loaded folder.

```text
crates/
├── wow-world/src/
│   ├── session/
│   │   ├── mod.rs                 # Session facade; no gameplay rule dump
│   │   ├── dispatch.rs            # sole registered handler call path
│   │   ├── admission.rs           # connection/status admission
│   │   └── lifecycle/             # session-facing lifecycle adapters
│   ├── handlers/quest/
│   │   ├── accept.rs              # decode/admit/invoke
│   │   └── reward.rs
│   ├── application/quests/
│   │   ├── accept.rs              # complete operation, narrow capabilities
│   │   ├── reward.rs
│   │   └── tests/
│   │       ├── accept.rs
│   │       └── reward_failures.rs
│   └── presentation/quests/
│       ├── dialog.rs              # result → packets/recipient intent
│       └── rewards.rs
├── wow-entities/src/player/
│   ├── mod.rs                     # one Player identity and private state
│   ├── quests/
│   │   ├── mod.rs                 # narrow domain API
│   │   ├── eligibility.rs         # CanTakeQuest rules
│   │   ├── objectives.rs
│   │   └── tests/
│   │       ├── eligibility.rs
│   │       └── objectives.rs
│   ├── inventory/                 # invariants, not another Player copy
│   ├── progression/
│   └── combat/
├── wow-map/src/
│   ├── map/
│   │   ├── mod.rs                 # Map facade and explicit phase order
│   │   ├── runtime/               # admitted simulation operations
│   │   ├── visibility/
│   │   ├── respawn/
│   │   └── entity_world/          # private storage implementation
│   └── manager/
│       ├── player_owner.rs        # incarnation/residence authority
│       └── tests/                 # lifetime/transfer/failure cases
├── wow-persistence/src/
│   ├── lib.rs                     # semantic contracts, small facade
│   ├── player_save/               # operation DTOs and classified outcomes
│   └── quest_reward/              # only if a real operation needs this port
├── wow-database/src/
│   └── player_lifecycle/          # SQL adapters, transaction plans, tests
└── world-server/src/
    ├── main.rs                    # entrypoint (already small)
    ├── app.rs                     # process construction/supervision facade
    ├── bootstrap/                 # catalogs/config/repositories/session wiring
    └── runtime/                   # task supervision and delivery wiring
```

The former `session_tests.rs`, `map_tests.rs` and `main_tests.rs` distribute to the
responsibilities they exercise, including transport/session tests left near Session,
not only the quest examples drawn here. Preserve integration scenarios in crate-level
`tests/` targets when they need the real public/production composition path.
The SDK, native modules, Wasm host/bindings and integrated QA tools use the same rules;
this tree does not create a new SDK layout or claim #583 has already delivered it.

A tiny Rust example of one type implemented through a private submodule in the same crate
([Rust Reference: multiple inherent implementations](https://doc.rust-lang.org/reference/items/implementations.html#inherent-implementations)):

```rust
// player/mod.rs
mod progression;

pub struct Player {
    level: u8,
}

// player/progression.rs
use super::Player;

impl Player {
    pub fn level(&self) -> u8 {
        self.level
    }
}
```

This defines one `Player`, not two objects or an additional crate. Child modules can
access the parent's private items. Where a child owns a private substate, use its narrow
operations rather than widening all its fields for sibling access. Actual gameplay
methods must still enforce the relevant invariants; the getter only illustrates layout.

## 6. Implementation ownership and honest enforcement

- **#133:** closed on 2026-09-09 in the tracker. Its technical acceptance is carried
  forward by the required #584 core work, #583 product and #153 audit; do not reopen
  the umbrella or wait for another #133 transition.
- **#584 C0–C4:** owns the remaining core/adapter/composition/tooling hotspots and
  every completed operation family's semantic and physical criteria, including its
  tests and consumers. Safe same-owner mechanical splits can precede or accompany
  the finite hecs conformance evidence; conformance gates production storage
  integration, not source organization.
- **#583:** its SDK, hosts, bindings, module examples and supporting tooling meet the same
  criteria in its own macro, including the required Rust/Wasm/C mixed product. It does
  not inherit unfinished core decomposition, and its implementation does not block
  unrelated gameplay macros.
- **#153:** independently verify both implementation macros, file exceptions and semantic
  boundaries. It is an audit, not the implementation owner of known cleanup.

The #578 C4 implementation above `8f5caedc` adds the physical branch to the **same**
`check_architecture.py` entrypoint; it is not another standalone checker:

```bash
python3 tools/architecture/check_architecture.py physical-files
python3 tools/architecture/check_architecture.py physical-files --json
python3 tools/architecture/check_architecture.py physical-files --terminal
```

`physical-file-policy.json` records 103 oversized legacy paths at introduction, their
observed/per-file ceilings, concrete split targets and the `578:C4` review checkpoint.
These are **migration debt, not terminal exceptions**. Growth or a missing/renamed legacy
path fails until its exact policy row is reviewed. Reductions pass and the ceiling should
be tightened after the validated split. New handwritten files default to the 2,000-line
ceiling; files above 1,000 are reported for ordinary cohesion review. No policy-generation
command exists. `--terminal` rejects every still-oversized migration entry: a normal
migration PASS is not proof the physical deliverable is complete.

Inventory covers tracked and untracked nonignored repository Rust, Python, shell,
C/C++, JS/TS and protobuf sources, including integration tests, tooling, vendored
code and extensionless shebang scripts. It is not restricted to `crates/*/src`.
Tracked source cannot disappear behind a new ignore rule. Ignored build outputs/private
files, documents, SQL/data inventories and arbitrary external compiler inputs are not
this repository-source inventory; existing logical/cfg/module-mount checks remain independent.
Source symlinks fail closed; non-source documentation/assets may remain symlinks.

Generated attribution requires pinned output/generator/input hashes, a reproduction
command and a hash-pinned JSON reproduction record matching that exact provenance and
reproduced output hash. The read-only checker verifies the recorded chain; it does **not**
execute generators or independently prove an unevidenced reproduction claim. Review and
actually reproduce a generator before adding its entry. Initially there are zero generated
waivers: `misc_generated.rs` is counted as handwritten despite its name. Terminal exceptions
also start empty; each needs path, responsibility, measured count/ceiling, issue, rationale,
review checkpoint and bounded review/expiry dates. Completed checkpoints and expired dates
invalidate the exception; merely leaving its issue open does not renew it.

`check` and `self-test` enforce physical and logical guards. `validation-v2 final` includes
the cheap physical scan for every nonempty diff, including tooling-only or generation-input
changes; workspace Rust additionally retains its independent logical ratchet. Changes to
the physical module/policy run its adversarial unit suite during `quick`; changes to the
shared checker/scanner run the existing architecture self-test. Macro closeout must also
run `physical-files --terminal`; it is deliberately not the daily migration gate. All
remaining core physical splits belong to #584; #583 applies the policy to its own
product and #153 verifies both without inheriting implementation.

## 7. Evidence and design references

Initial physical examples (lines including blanks/comments, at `816d5c84`): Session root
76,793; Session tests 96,845; Map tests 18,288; `world-server/app.rs` 5,652;
`wow-persistence/lib.rs` 4,513. These are navigability observations, not production-only
LOC or parity percentages. Recompute at the implementation checkpoint.

Relevant C++ anchors under `/home/server/woltk-trinity-legacy/src/server/game/`:
`Entities/Player/Player.cpp:14087` (`CanTakeQuest`) and `:15675` (quest dialog),
`Maps/Map.cpp:666` (update phases), `Server/WorldSession.cpp:64` (packet processing).
They anchor responsibilities/behavior, not a requirement to reproduce C++ file sizes.

The Rust Book explains [module privacy and submodules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html),
[separating modules into files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)
and [Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).
DDD's [bounded contexts](https://martinfowler.com/bliki/BoundedContext.html) concern
model boundaries and relationships, not a mandatory Rust skeleton. The hybrid layout
and numeric budgets above are this project's design choice, not claims made by those sources.
