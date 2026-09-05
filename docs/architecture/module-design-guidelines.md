# Module design and source navigability

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

- **#133:** closure requires semantic boundaries and physical source/test navigability.
- **#578 C2:** both criteria for every completed operation family, including its tests.
  Safe same-owner mechanical splits can precede or run alongside the hecs conformance
  experiment. That experiment gates production storage migration, not source organization.
- **#578 C4:** inventory and finish the remaining core/adapter/composition/tooling hotspots
  in #133 scope, with file-specific exits; implement the physical ratchet in the existing
  architecture checker and retire blanket legacy exceptions. Do not reopen completed
  historical issues or transfer known work to the terminal auditor.
- **#583:** its SDK, hosts, bindings, module examples and supporting tooling meet the same
  criteria in its own macro; it does not inherit unfinished Session decomposition.
- **#153:** independently verify both implementation macros, file exceptions and semantic
  boundaries. It is an audit, not the implementation owner of known cleanup.

At this policy's adoption, `check_architecture.py` enforces selected **logical** owner
ceilings. Its physical report covers Rust under `crates/*/src`, not the entire promised
source/test/tooling scope, and it does **not** enforce the new physical budgets. A PASS
today is not physical completion. #578 C4 must extend that existing mechanism with a
reviewable file inventory, generated-source provenance and independent physical ceilings;
test new-file growth, rename/move escape, oversized tests, stale/expired exceptions and
baseline reductions without losing the logical owner checks. Keep daily checks incremental
and terminal coverage complete; do not add another permanent parallel architecture checker.

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
