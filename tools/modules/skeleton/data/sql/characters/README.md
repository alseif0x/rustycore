# characters SQL

Put this module's characters migrations here and list each one in
`data/sql/manifest.toml` (see `docs/architecture/modules.md`, "Module SQL").
A module creates and changes only its own tables, named with its prefix
(`mod_<id with . as _>_`). Nothing here is applied until an operator runs
`rustycore-db module apply <id>` (#1444 H6b); a recorded file never changes, so a fix
is a new migration.
