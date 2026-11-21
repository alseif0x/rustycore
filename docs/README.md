# Documentation map

Use this page to find the maintained source for a question. A file's existence, a green
checkbox or an old percentage is not proof that the current server implements that behavior.

For this branch start with the [Forever setup](../README.md#required-checkout-layout), the
[Forever runbook](operations/forever-login.md) and the most recent [checkpoint](migration/STATE.md).
The inherited WotLK plan and its issue states describe their original target. Confirm merges
and acceptance on `forever` before counting a shared issue as delivered here.

## Current state and execution

| Question | Maintained source |
| --- | --- |
| What is implemented, integrated or still unproven? | [STATE.md](migration/STATE.md), its dated evidence boundaries and the active issue/checkpoint |
| What do we execute next? | [PORT_PLAN.md](migration/PORT_PLAN.md) and [GitHub #49](https://github.com/alseif0x/rustycore/issues/49) |
| How are agent responsibilities assigned for this runtime? | [orchestrate-rustycore](../.agents/skills/orchestrate-rustycore/SKILL.md) |
| What remains in the current architecture delivery? | [Core and module obligations](architecture/refactor-completion-plan.md) |
| Where are the shared refactor contracts? | [Refactor completion plan](architecture/refactor-completion-plan.md) |
| What must a quest reward make durable, and when? | [Quest reward operation contract](architecture/quest-reward-operation-contract.md) |
| What architecture and extension direction is approved? | [Modularity/ECS plan](architecture/modularity-and-ecs-plan.md) |
| How should responsibilities and files be organized? | [Module design](architecture/module-design-guidelines.md) and [dependency/ownership boundaries](architecture/ownership-and-boundaries.md) |
| Which oversized files still owe a #584:C4 split? | [C4 physical split plan](architecture/c4-physical-split-plan.md) |
| Which `wow-world` unit tests could move to a domain crate? | [wow-world test census](architecture/wow-world-test-census.md) (#1442) |
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

## WotLK reference sources inherited by Forever

On 2026-09-08 the user requested a complementary 3.3.5 source for gameplay missing or
suspect in the Classic fork. AzerothCore was selected for its WotLK focus, shared
TrinityCore/SunwellCore lineage, and combined core/scripts/world SQL. This is a practical
source choice, not a measured claim that it is more complete than CMaNGOS in every system.

| Reference | Local checkout and observed SHA | Use and limits |
| --- | --- | --- |
| Existing Classic 3.4.3 fork | `/home/server/woltk-trinity-legacy`, `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` | Reference for inherited WotLK behavior. Validate against build 70170 before using a wire or gameplay contract on Forever. Existing captures retain their own exact source/binary identities; this SHA does not relabel older QA. |
| [AzerothCore WotLK](https://github.com/azerothcore/azerothcore-wotlk) | `/home/server/azerothcore-wotlk-reference`, `a5e0e6b8f2bf878cb45cb1dc2251eb1448b9bbc3` | Detached shallow 3.3.5a reference, with `src`, world SQL base/updates and project metadata checked out. No build, database import or runtime is implied. |

The AzerothCore checkout is outside RustyCore and is not a dependency or submodule.
Its source/license/author information remains intact; record provenance when adapting
logic or data. Sparse/shallow history is not an absence-of-implementation proof: inspect
the exact upstream path/history when the active operation needs more context. Protocol,
data and gameplay differences must be resolved against the Forever build before adapting shared behavior here.

## Development, operations and extensions

- [WotLK documentation site](wiki/README.md): the published site is maintained on `3.4.3`. For Forever setup use the branch README.
  Published guides link to the maintained sources here; they are not a second status ledger.
- [AGENTS.md](../AGENTS.md): shared scope, approval, fidelity, validation and Git rules.
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

The [subsystem index](migration/_INDEX.md) locates the original C++ file mappings and bounded reports at an immutable revision. [Migration inventories](migration/inventory/README.md) retain their recorded schemas and evidence. Consult these for specific historical questions. Current work uses the branch checkpoint and open owner issues.

## Keeping documentation useful

- Update the owning current document instead of adding another status log or operating guide.
- Name the audited commit, scope and evidence kind for state claims; don't silently refresh
  an old table's date or imply a new full-port audit from a documentation edit.
- Keep useful historical evidence clearly marked. Retire duplicate instructions; preserve a
  short redirect when existing links or tools still need the old path.
- Recheck inbound links before moving/removing a document. Generated inventories retain their
  schemas and provenance; never rewrite implementation evidence to simulate completion.
- The project skills route development, design, refactoring and orchestration to these rules. They describe how to reason/refactor,
  not a fixed technology map, stale issue sequence or new approval process.
