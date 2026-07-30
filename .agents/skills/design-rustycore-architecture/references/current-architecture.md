# Current RustyCore architecture audit guide

Recalculate all metrics on the active HEAD. Use this file to find likely boundaries, not to claim
that old counts remain exact.

## Current structural reality

- `wow-world` is the central application/god crate and directly knows most internal crates.
- `WorldSession` contains session state, shared stores, represented player state, gameplay helpers,
  runtime bridges, persistence coordination, dispatch, and extensive inline tests.
- `wow-network::accept::SessionResources` currently carries application, database, catalog, loot,
  instance, runtime, and configuration resources across the network boundary.
- `wow-network::player_registry::SessionCommand` contains gameplay commands and packet payloads;
  network therefore depends upward on game data, database, instances, and loot.
- Legacy `wow_world::MapManager`, canonical `wow_map::MapManager`, and the global world loop coexist.
  The accepted runtime ADR requires one tick owner and method-by-method convergence.
- Player and creature state still have represented/session, legacy map, canonical entity/map, and
  registry mirrors in several paths.
- Several domain crates are nominal placeholders while their logic remains in `wow-world`,
  handlers, map, entities, or binaries.

Read `docs/migration/adr-runtime-tick-ownership.md` for the authoritative runtime sequence and
`docs/migration/STATE.md` for the current represented-versus-live boundary.

## Recalculate the workspace

Use commands like:

```bash
git status --short --branch
git log --oneline --decorate -8
find crates -path '*/src/*.rs' -type f -print0 | xargs -0 wc -l | sort -nr | head -40
for d in crates/*; do
  total=$(find "$d/src" -type f -name '*.rs' -print0 | xargs -0 cat | wc -l)
  printf '%8d %s\n' "$total" "$d"
done | sort -nr
cargo metadata --format-version 1 --no-deps
cargo tree --duplicates
cargo tree -e features
rg -n '^(pub )?mod |^pub use ' crates/*/src/lib.rs
rg -n 'impl WorldSession|impl Map|impl Player' crates
rg -n 'sqlx|PreparedStatement|Transaction|execute_or_warn' crates/wow-world/src/handlers
```

Adjust commands when generated files or test modules would distort the question.

## High-priority audit surfaces

Inspect these as project-wide symptoms:

- `crates/wow-network/src/accept.rs`: network/application resource boundary.
- `crates/wow-network/src/player_registry.rs` and `group_registry.rs`: application commands and
  social state inside network.
- `crates/wow-world/src/session.rs`: state ownership, dispatch, stores, runtime mirrors.
- `crates/wow-world/src/handlers/misc.rs`: unrelated feature families and opcode registrations.
- `crates/wow-world/src/handlers/character.rs`: character lifecycle mixed with inventory, bank,
  vendor, equipment, and persistence.
- `crates/wow-world/src/handlers/loot.rs`: packet handling, generation, authority, transactions,
  runtime application, and tests.
- `crates/world-server/src/main.rs`: composition root mixed with loaders, event logic, runtime
  routing, session factory, DB writers, and shutdown.
- `crates/world-server/src/spawn_store_loader.rs`: loading mixed with world-state and game-event
  ownership.
- `crates/wow-map/src/map.rs`: one legitimate aggregate with too many capabilities in one module.
- `crates/wow-entities/src/player.rs`: one legitimate aggregate whose substates and impls need
  private modularization.
- `crates/wow-data/src/spell.rs`: catalog model, derivation, corrections, and loaders mixed.
- `crates/wow-packet/src/packets/misc.rs`: unrelated protocol families.
- `tools/wow-test-bot/src/main.rs`: CLI, protocol, fixtures, scenarios, runtime preparation, and
  reporting mixed.

Large declarative packet/update-field files or statement tables may be acceptable. Prioritize
mixed state, I/O, rules, and ownership over raw line count.

## Known dependency debts to ratchet

Inspect rather than blindly preserve:

- `wow-network -> wow-data, wow-database, wow-instances, wow-loot`;
- `wow-data -> wow-database, wow-entities, wow-movement`;
- `wow-packet -> wow-loot, wow-movement`;
- broad `wow-world` and `world-server` dependency sets.

Do not add new upward edges. Remove them one seam at a time with compatibility tests.

## Recommended campaign order

1. Document owners, allowed edges, mirrors, and no-growth guardrails.
2. Decouple socket acceptance from application `SessionResources`.
3. Introduce typed catalogs, repositories, and runtime handles; replace setter groups
   incrementally.
4. Split obvious feature dumping grounds mechanically without changing behavior.
5. Extract one complete vertical use case at a time from handler to application/domain.
6. Modularize `Map`, `Player`, binaries, packet families, and QA scenarios while retaining one
   aggregate owner.
7. Retire legacy/canonical mirrors method by method according to the runtime ADR.
8. Promote stable private modules into domain crates only after dependency direction is clean.

Never begin with a wholesale split of `session.rs` or a worker pool over duplicated runtime state.
