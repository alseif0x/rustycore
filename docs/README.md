# Documentation map

Use this page to find the maintained source for a question. A file's existence, a green
checkbox or an old percentage is not proof that the current server implements that behavior.

## Current state and execution

| Question | Maintained source |
| --- | --- |
| What is implemented, integrated or still unproven? | [STATE.md](migration/STATE.md), its dated evidence boundaries and the active issue/checkpoint |
| What do we execute next? | [PORT_PLAN.md](migration/PORT_PLAN.md) and [GitHub #49](https://github.com/alseif0x/rustycore/issues/49) |
| What remains in the current architecture delivery? | [Architecture program state](architecture/modularity-and-ecs-plan.md#architecture-program-state--2026-09-11) |
| How should Claude continue the complete refactor? | [Refactor completion plan](architecture/refactor-completion-plan.md) |
| What must a quest reward make durable, and when? | [Quest reward operation contract](architecture/quest-reward-operation-contract.md) |
| What architecture and extension direction is approved? | [Modularity/ECS plan](architecture/modularity-and-ecs-plan.md) |
| How should responsibilities and files be organized? | [Module design](architecture/module-design-guidelines.md) and [dependency/ownership boundaries](architecture/ownership-and-boundaries.md) |
| What reported defects need current verification? | [EXISTING-CODE-DEFECTS.md](migration/EXISTING-CODE-DEFECTS.md) |

Base-server behavior is established by versioned C++ references and appropriate target-build
captures, not by a Rust comment or a planning document. AGENTS.md owns the comparison and
adaptation rules; neither upstream is assumed complete or correct. An architecture plan can be
approved while its
implementation and acceptance remain open. Keep those statuses separate.

`PORT_PLAN.md` and GitHub #49 provide the general direction and issue scope. The
modularity/ECS and refactor-completion documents provide the technical ownership,
consumer, dependency and acceptance detail for that direction; they are not a rival
execution plan. On the current architecture track, #133 was closed on 2026-09-09,
while the technical gate remains #584 core → #583 native/Wasm product → #153 audit.

## Versioned behavioral references

On 2026-09-08 the user requested a complementary 3.3.5 source for gameplay missing or
suspect in the Classic fork. AzerothCore was selected for its WotLK focus, shared
TrinityCore/SunwellCore lineage, and combined core/scripts/world SQL. This is a practical
source choice, not a measured claim that it is more complete than CMaNGOS in every system.

| Reference | Local checkout and observed SHA | Use and limits |
| --- | --- | --- |
| Existing Classic 3.4.3 fork | `/home/server/woltk-trinity-legacy`, `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` | Target-version code. Existing captures retain their own exact source/binary identities; this SHA does not relabel older QA. |
| [AzerothCore WotLK](https://github.com/azerothcore/azerothcore-wotlk) | `/home/server/azerothcore-wotlk-reference`, `a5e0e6b8f2bf878cb45cb1dc2251eb1448b9bbc3` | Detached shallow 3.3.5a reference, with `src`, world SQL base/updates and project metadata checked out. No build, database import or runtime is implied. |

The AzerothCore checkout is outside RustyCore and is not a dependency or submodule.
Its source/license/author information remains intact; record provenance when adapting
logic or data. Sparse/shallow history is not an absence-of-implementation proof: inspect
the exact upstream path/history when the active operation needs more context. Protocol,
data and gameplay differences must be resolved for 3.4.3 before production adaptation.

## Development, operations and extensions

- [Documentation site](wiki/README.md): VitePress authoring and local build instructions.
  Published guides link to the maintained sources here; they are not a second status ledger.
- [AGENTS.md](../AGENTS.md): shared scope, approval, fidelity, validation and Git rules.
  [CLAUDE.md](../CLAUDE.md) imports it; it is not a second operating guide.
- [Validation V2](operations/validation-v2.md) and
  [local-first development](operations/local-first-development.md): actual validation profiles.
- [DB bootstrap](operations/db-bootstrap.md), [live client debugging](operations/live-client-debug.md)
  and [QA bot](../tools/wow-test-bot/README.md): scoped operational instructions, not authority
  to restart a server, mutate a database or expose secrets.
- [Module implementation guide](architecture/modules.md): the delivered author/operator tooling.
  Its current API is not the complete planned native/Wasm product.
- [Runtime tick ADR](migration/adr-runtime-tick-ownership.md),
  [entity-world ADR](migration/adr-map-runtime-entity-world.md) and
  [clock/phase trace](architecture/runtime-clock-phase-trace.md): decisions and evidence with
  dates/coverage limits; inspect current callers before changing an owner.

## Historical references

The per-subsystem files under migration/, numbered inventory campaigns, older audits and
implementation reports preserve source mappings and past experiments. Their old next-step
lists, commands and completion tables do not override the current sources above. Read them
when the active operation needs that evidence; do not load the whole archive for each task.

Older architecture checkpoints may still contain the pre-closure #133/#578 ordering. They
are historical evidence and must not be used as current instructions; use STATE.md and the
current architecture plan for status and sequence.

[MIGRATION_ROADMAP.md](MIGRATION_ROADMAP.md), [migration/_INDEX.md](migration/_INDEX.md)
and the root [MIGRATION_STATUS.md](../MIGRATION_STATUS.md) are not competing current plans.
The frozen migration/current-session-handoff.md is historical evidence, not an append target
or required session preflight. Only verified C++/capture evidence can approve protocol/gameplay behavior.

## Keeping documentation useful

- Update the owning current document instead of adding another status log or operating guide.
- Name the audited commit, scope and evidence kind for state claims; don't silently refresh
  an old table's date or imply a new full-port audit from a documentation edit.
- Keep useful historical evidence clearly marked. Retire duplicate instructions; preserve a
  short redirect when existing links or tools still need the old path.
- Recheck inbound links before moving/removing a document. Generated inventories retain their
  schemas and provenance; never rewrite implementation evidence to simulate completion.
- The two project skills route to these maintained rules. They describe how to reason/refactor,
  not a fixed technology map, stale issue sequence or new approval process.

The September 2026 cleanup audits documentation coherence and instruction routing; it does
not re-prove every gameplay claim or every row of the historical migration inventory.
