# RustyCore Wiki Seed

This directory is a repository-backed seed for pages that can later be copied
into the GitHub Wiki. TrinityCore keeps its long-form operational docs in the
GitHub Wiki; RustyCore keeps the source-controlled version here first so changes
can be reviewed like normal code.

Suggested first wiki pages:

- **Home** — project scope, WotLK Classic target and
  [current-state evidence](../migration/STATE.md).
- **How to Build** — Rust toolchain, protoc, MariaDB, database setup.
- **How to Test with a Client** — auth/world ports, certificates, supported client build.
- **Contributing** — C++-first porting discipline and
  [local-first validation](../operations/local-first-development.md).
- **SQL Fixes** — database fix style, inspired by TrinityCore's SQL-fix guidelines.
- **Migration Roadmap** — link to the current [port plan](../migration/PORT_PLAN.md)
  and active issue/checkpoint; older roadmap tables are historical, not status evidence.
- **Modules** — distinguish the [implemented native login API](../architecture/modules.md)
  from the [approved ECS/native/Wasm delivery plan](../architecture/modularity-and-ecs-plan.md).

Keep the canonical project truth in the repository. The GitHub Wiki should be a
readable publication target, not the only place where migration instructions
exist. Do not copy mutable status tables or operating rules into a second source of truth;
link to the maintained documents. Editing this seed does not publish to the Wiki.
