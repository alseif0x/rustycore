# Claude Porting Instructions

This legacy prompt path is retained for existing references. Use
[AGENTS.md](../../AGENTS.md), which `CLAUDE.md` imports, as the shared operating
guide. There is no Claude-specific branch, approval, commit, or completion policy.

For a session, read [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md), and the
active issue/checkpoint. For technical contrast, use the
[C++ porting methodology](../CPP_TO_RUST_PORTING_METHODOLOGY.md).
Current architecture work also follows the
[modularity plan](../architecture/modularity-and-ecs-plan.md) and
[module design guidelines](../architecture/module-design-guidelines.md).

The former prompt embedded `develop`, base `1af9223`, the `96.97%` represented
headline, a required frozen-handoff read, and an obsolete runtime-clock model.
Those are historical references, not instructions or evidence about current HEAD.
See the [historical module index](_INDEX.md) and
[frozen handoff](current-session-handoff.md) only for a specific historical question.

Continue through authorized acceptance with coherent internal checkpoints.
A helper, testable slice, or represented gap does not require a new issue/PR,
fresh approval, or automatic `#NEXT` entry. Missing prerequisites must remain
visible: resolve them within approved scope, or report the affected blocker;
do not silently expand the task or claim partial runtime is complete.

Preserve all explicit approval requirements in AGENTS.md and the user's request.
Review-only work remains read-only. Publishing does not authorize merging, and
a test environment is not blanket authority for destructive or runtime actions.

*Workflow redirect refreshed 2026-09-05; historical results were not revalidated.*
