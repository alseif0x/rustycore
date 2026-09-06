# Migration documentation: current contracts and historical evidence

For current work, use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md), the
active issue/checkpoint and [AGENTS.md](../../AGENTS.md). Runtime architecture
uses the current ADRs and [modularity plan](../architecture/modularity-and-ecs-plan.md);
module decomposition follows [module design guidelines](../architecture/module-design-guidelines.md).

Document roles:

| Role | Documents | Interpretation |
| --- | --- | --- |
| Current status/plan | STATE, PORT_PLAN, active issue/checkpoint | Start here; claims still need current implementation/evidence. |
| Active bounded compatibility contracts | [player lifecycle](player-lifecycle-persistence-contract.md), [represented/live bridge](represented-live-bridge.md) | Preserve their accepted invariants; dated examples are not a full current audit or universal extension API. |
| Current defect register | [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md) | Preserve verified findings and contested/candidate distinctions; do not close defects through doc cleanup. |
| Technical module references | [_INDEX.md](_INDEX.md), per-domain Markdown, [config reference](config-reference.md) | Locate C++ classes/keys and dated findings; old coverage, task order and "done" labels are snapshots. |
| Historical planning proposals | refinement, creature/login plans, test/performance/harness strategies | Useful context, not independent gates, authority or delivery size. |
| Historical inventories | [inventory/](inventory/README.md) | Retained rows and generated summaries, not the current issue DAG or server completion percentage. |
| Archived append-log | [current-session-handoff.md](current-session-handoff.md) | Recovery pointer into Git, not a kickoff read. |
| Optional templates | [_TEMPLATE.md](_TEMPLATE.md), [implementation template](../implementation-template.md) | Evidence-writing aids, not a document/ID/PR requirement per helper. |

The 2026-09-05 cleanup inventoried paths and screened headers/workflow/source
claims in this directory, the audit/implementation records and older root-level
guides. It did not re-audit every historical gameplay assertion, regenerate
TSV inventories, rerun captures or certify old completion counts. Retired
instruction text remains recoverable from Git; current documents do not create
new deployment, push, merge or destructive-action authority.
