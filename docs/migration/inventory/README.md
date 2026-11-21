# Migration inventories

The tab-separated inventories in this directory preserve the evidence and machine-readable inputs from each campaign. Runtime consumers include the world configuration registry and packet-handler inventory. Keep their schemas, IDs and recorded values intact.

| Campaign | Recorded scope |
| --- | --- |
| C++ | Source files, configuration keys, DB2 stores, entity types, SQL, scripts and opcodes |
| R2–R6 | Original scope and divergences, cross-system registries, dependency ordering, smoke gates and gap audits |
| R7–R8 | Dated batches, test and implementation evidence with per-row boundaries |

Historical report names in the data resolve at the commit below. For work selection use the [current checkpoint](../STATE.md), [delivery plan](../PORT_PLAN.md) and [subsystem index](../_INDEX.md).

## Source tables and archived reports

| Report at its recorded revision | Local tables |
| --- | --- |
| [cpp-config-keys](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-config-keys.md) | [cpp-config-dynamic.tsv](cpp-config-dynamic.tsv), [cpp-config-keys.tsv](cpp-config-keys.tsv), [cpp-world-config-registry.tsv](cpp-world-config-registry.tsv) |
| [cpp-dbc-db2-stores](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-dbc-db2-stores.md) | [cpp-db2-load-pipeline.tsv](cpp-db2-load-pipeline.tsv), [cpp-db2-stores.tsv](cpp-db2-stores.tsv), [cpp-sql-prepared.tsv](cpp-sql-prepared.tsv) |
| [cpp-entity-types](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-entity-types.md) | [cpp-entity-files.tsv](cpp-entity-files.tsv), [cpp-entity-types.tsv](cpp-entity-types.tsv) |
| [cpp-files-by-module](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-files-by-module.md) | Source and original method in the archived report |
| [cpp-handlers-opcodes](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-handlers-opcodes.md) | [cpp-client-handlers.tsv](cpp-client-handlers.tsv), [cpp-server-opcodes.tsv](cpp-server-opcodes.tsv) |
| [cpp-scripts-tree](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-scripts-tree.md) | [cpp-scripts-files.tsv](cpp-scripts-files.tsv), [cpp-scripts-registrations.tsv](cpp-scripts-registrations.tsv), [cpp-scripts-types.tsv](cpp-scripts-types.tsv) |
| [cpp-server-tree](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-server-tree.md) | Source and original method in the archived report |
| [cpp-sql-prepared](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/cpp-sql-prepared.md) | [cpp-sql-inline.tsv](cpp-sql-inline.tsv), [cpp-sql-prepared.tsv](cpp-sql-prepared.tsv) |
| [r2-known-divergences](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r2-known-divergences.md) | [r2-known-divergences.tsv](r2-known-divergences.tsv) |
| [r2-product-scope](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r2-product-scope.md) | [r2-product-scope.tsv](r2-product-scope.tsv) |
| [r2-rust-targets](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r2-rust-targets.md) | [r2-rust-targets.tsv](r2-rust-targets.tsv) |
| [r2-task-wbs](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r2-task-wbs.md) | [r2-task-wbs.tsv](r2-task-wbs.tsv) |
| [r2-tests-required](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r2-tests-required.md) | [r2-tests-required.tsv](r2-tests-required.tsv) |
| [r3-cross-registry-summary](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-cross-registry-summary.md) | Source and original method in the archived report |
| [r3-database-registry](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-database-registry.md) | [r3-database-registry.tsv](r3-database-registry.tsv) |
| [r3-e2e-harness](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-e2e-harness.md) | [r3-e2e-harness.tsv](r3-e2e-harness.tsv) |
| [r3-opcodes-registry](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-opcodes-registry.md) | [r3-opcodes-registry.tsv](r3-opcodes-registry.tsv) |
| [r3-runtime-managers](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-runtime-managers.md) | [r3-runtime-managers.tsv](r3-runtime-managers.tsv) |
| [r3-scripts-registry](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-scripts-registry.md) | [r3-scripts-registry.tsv](r3-scripts-registry.tsv) |
| [r3-update-fields-registry](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r3-update-fields-registry.md) | [r3-update-fields-registry.tsv](r3-update-fields-registry.tsv) |
| [r4-dependency-dag](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-dependency-dag.md) | [r4-dependency-dag.tsv](r4-dependency-dag.tsv) |
| [r4-dependency-gate-summary](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-dependency-gate-summary.md) | Source and original method in the archived report |
| [r4-implementation-gates](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-implementation-gates.md) | [cpp-config-keys.tsv](cpp-config-keys.tsv), [r4-implementation-gates.tsv](r4-implementation-gates.tsv) |
| [r4-roadmap-order](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-roadmap-order.md) | [r4-roadmap-order.tsv](r4-roadmap-order.tsv) |
| [r4-smoke-gates](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-smoke-gates.md) | [r4-smoke-gates.tsv](r4-smoke-gates.tsv) |
| [r4-temporary-stubs](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r4-temporary-stubs.md) | [r4-temporary-stubs.tsv](r4-temporary-stubs.tsv) |
| [r5-gap-audit](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r5-gap-audit.md) | [r5-gap-audit.tsv](r5-gap-audit.tsv) |
| [r6-next-miniphase](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r6-next-miniphase.md) | [cpp-world-config-registry.tsv](cpp-world-config-registry.tsv), [r6-next-miniphase.tsv](r6-next-miniphase.tsv) |
| [r7-l1-character-prep-batches](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l1-character-prep-batches.md) | [r7-l1-character-prep-batches.tsv](r7-l1-character-prep-batches.tsv) |
| [r7-l1-db2-store-batches](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l1-db2-store-batches.md) | [r7-l1-db2-store-batches.tsv](r7-l1-db2-store-batches.tsv) |
| [r7-l1-hotfix-prep-batches](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l1-hotfix-prep-batches.md) | [r7-l1-hotfix-prep-batches.tsv](r7-l1-hotfix-prep-batches.tsv) |
| [r7-l1-infra-miniphase](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l1-infra-miniphase.md) | [r7-l1-infra-miniphase.tsv](r7-l1-infra-miniphase.tsv) |
| [r7-l2-packets-miniphase](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l2-packets-miniphase.md) | [cpp-client-handlers.tsv](cpp-client-handlers.tsv), [r7-l2-packets-miniphase.tsv](r7-l2-packets-miniphase.tsv) |
| [r7-l3-maps-miniphase](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r7-l3-maps-miniphase.md) | [r7-l3-maps-miniphase.tsv](r7-l3-maps-miniphase.tsv) |
| [r8-entities-miniphase](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/inventory/r8-entities-miniphase.md) | [r8-entities-miniphase.tsv](r8-entities-miniphase.tsv) |

Offline access: `git show d287f333d69d643ccfd7f84426e51a925836bded:docs/migration/inventory/<report>.md`. Annexes in the archived Markdown remain available alongside the raw data.
