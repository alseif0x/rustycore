# RustyCore Wiki Seed

This directory is a repository-backed seed for pages that can later be copied
into the GitHub Wiki. TrinityCore keeps its long-form operational docs in the
GitHub Wiki; RustyCore keeps the source-controlled version here first so changes
can be reviewed like normal code.

Suggested first wiki pages:

- **Home** — project scope, WotLK Classic target, current status.
- **How to Build** — Rust toolchain, protoc, MariaDB, database setup.
- **How to Test with a Client** — auth/world ports, certificates, supported client build.
- **Contributing** — C++-first porting discipline and required checks.
- **SQL Fixes** — database fix style, inspired by TrinityCore's SQL-fix guidelines.
- **Migration Roadmap** — link back to `docs/MIGRATION_ROADMAP.md` and current audit docs.

Keep the canonical project truth in the repository. The GitHub Wiki should be a
readable publication target, not the only place where migration instructions
exist.
