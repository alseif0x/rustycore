# Documentation map

Use this page to find the maintained source for a question. A file's existence, a green
checkbox or an old percentage is not proof that the current server implements that behavior.

## Current state and execution

| Question | Maintained source |
| --- | --- |
| What is implemented, integrated or still unproven? | [STATE.md](migration/STATE.md), its dated evidence boundaries and the active issue/checkpoint |
| What do we execute next? | [PORT_PLAN.md](migration/PORT_PLAN.md) and [GitHub #49](https://github.com/alseif0x/rustycore/issues/49) |
| What remains in the current architecture macro? | [#578 checkpoint](architecture/session-578-checkpoint.md) |
| What architecture and extension direction is approved? | [Modularity/ECS plan](architecture/modularity-and-ecs-plan.md) |
| How should responsibilities and files be organized? | [Module design](architecture/module-design-guidelines.md) and [dependency/ownership boundaries](architecture/ownership-and-boundaries.md) |
| What reported defects need current verification? | [EXISTING-CODE-DEFECTS.md](migration/EXISTING-CODE-DEFECTS.md) |

Base-server behavior is established by the legacy C++ source or appropriate real captures,
not by a Rust comment or a planning document. An architecture plan can be approved while its
implementation and acceptance remain open. Keep those statuses separate.

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
