# RustyCore

<p align="center">
  <img src="assets/brand/rustycore-logo.svg" alt="RustyCore logo" width="760">
</p>

**WoW Wrath of the Lich King Classic (3.4.3.54261) server emulator written in Rust.**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.98%2B-orange.svg)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/WotLK%20Classic-3.4.3.54261-blueviolet.svg)](#)
[![Discord](https://img.shields.io/badge/Discord-Join%20the%20community-5865F2.svg)](https://discord.gg/mH6ACpGPb2)

RustyCore is currently in the middle of a full C++ -> Rust port of a TrinityCore-style WotLK Classic server.

The goal is behavioral parity with the legacy C++ implementation first, and Rust-native cleanup only when the original behavior is understood. This is not a greenfield emulator that guesses the rules from scratch: packet formats, database behavior, world state, gameplay gates, and runtime order must be checked against the C++ source before being treated as correct.

The project is usable for parts of the login/world-entry path and contains a large amount of represented server logic, but the full gameplay runtime is still under active migration.

## Target

RustyCore currently targets:

- **Client:** WotLK Classic `3.4.3.54261`
- **Historical test game build:** `51943`; each capture/runtime report must identify its actual build
- **World DB expectation:** `TDB 343.24081`, `cache_id = 24081`
- **Reference implementation:** TrinityCore/WotLK-style C++ source
- **Integration and default branch:** `3.4.3`
- **Development flow:** feature branches -> PRs into `3.4.3`; `main` is an optional stable pointer

The inherited server tree includes modern-era surfaces such as Battle Pets and Black Market.
Their presence in Rust does not prove client support or live parity. Execution priority follows
the current port plan; the full C++-parity goal is not silently narrowed by a README exclusion.

## Current Status

RustyCore has many C++-contrasted systems represented in Rust, plus prior smoke testing for login, realm selection, character enumeration, and initial world entry.

The important distinction is:

- **Represented logic** means Rust code models a C++ behavior with tests and explicit boundaries.
- **Live runtime parity** means that behavior is wired into the actual server loop, map ownership, packet fanout, database lifecycle, visibility, and client-visible runtime.

Represented logic is useful, but it is not the same thing as a complete running server. The remaining high-value work is mostly around live runtime convergence: maps, movement, combat, visibility/fanout, respawns, scripts, and the exact order of C++ world/map updates.

Start with the [documentation map](docs/README.md). Current state and execution are tracked in:

- [STATE.md](docs/migration/STATE.md): dated implementation and evidence boundaries.
- [PORT_PLAN.md](docs/migration/PORT_PLAN.md): the current execution plan and full-parity goal.
- [Session #578 checkpoint](docs/architecture/session-578-checkpoint.md): the active macro.
- [Modularity/ECS plan](docs/architecture/modularity-and-ecs-plan.md) and
  [module design guidelines](docs/architecture/module-design-guidelines.md): current direction
  and jointly required semantic/physical acceptance.

The frozen handoff, old roadmap and percentage audits remain historical references, not
competing sources of current state. No documentation update is a new whole-port parity audit.

Older summaries and checklist percentages can drift. For port work, the C++ source and current Rust code are the authority.

## Architecture Snapshot

Documentation checked at `7eaf8ddc` (2026-09-05); this is not a new runtime deployment or
whole-port parity audit. Current decisions and remaining acceptance live in the links above.

The runtime is intentionally in a transition state while the port moves from session-owned behavior toward map-owned behavior.

- `bnet-server` handles Battle.net auth, REST glue, TLS RPC, realm discovery, and login-related DB work.
- `world-server` owns world startup, DB/store loading, realm and instance listeners, session creation, and map-loop orchestration.
- `wow-world` contains `WorldSession`, packet handlers, represented player/gameplay logic, and the legacy shared map manager.
- `wow-map` contains the canonical map-runtime direction: map update loop, map-owned objects, grids, respawns, and long-term world ownership.
- `wow-packet` owns packet read/write types and wire-shape tests.
- `wow-entities` contains canonical domain state and invariants, including Player.
- `wow-session` contains the extracted transport/session kernel, not gameplay ownership.
- `wow-persistence` defines SQLx-free semantic operation contracts.
- `wow-database` owns concrete SQLx adapters and prepared statement definitions.
- `wow-data` owns DBC/DB2-style data loading and typed stores.

The current runtime split is:

1. The legacy `wow_world::MapManager` is shared between sessions and still owns several represented creature/session behaviors.
2. The canonical `wow_map::MapManager` has the global map tick direction and C++-like map structures.
3. Migration work is moving tick ownership, creature updates, respawns, movement, visibility, and fanout into the correct map/global runtime without double-updating state.

That split is a migration bridge, not the final design.

## Tech Stack

- **Rust 1.98+** — edition 2024
- **Tokio** — async runtime and networking
- **Axum** — Battle.net REST API
- **SQLx + MariaDB** — login/auth, characters, world, and hotfix databases
- **hecs** — selected private, selective storage direction; isolated experiments only today,
  with conformance and production integration still pending
- **Wasmtime/Core Wasm** — planned operator-optional executor of shared module contracts;
  not an installed production SDK or a promise of arbitrary-language support
- **prost** — protobuf support for Battle.net protocol messages
- **tracing** — structured logging
- **zlib/miniz_oxide** — packet and account-data compression helpers

## Workspace Layout

```text
crates/
  bnet-server/       Battle.net authentication server
  world-server/      World/game server entry point
  wow-core/          Core types, GUIDs, time, networking helpers
  wow-constants/     Opcodes and game constants
  wow-crypto/        SRP6, AES-GCM and auth crypto
  wow-network/       Tokio sockets, session manager, registries
  wow-packet/        Packet serialization/deserialization
  wow-handler/       Packet handler metadata and dispatch support
  wow-world/         WorldSession, handlers, represented game logic
  wow-session/       Extracted transport/session kernel
  wow-entities/      Canonical domain entities and invariants
  wow-map/           Canonical map and runtime structures
  wow-data/          DBC/DB2 data loading and stores
  wow-database/      SQLx database layer and prepared statements
  wow-persistence/   Semantic persistence contracts without SQLx
  wow-module-api/    Current bounded source-module API
  world-modules/     Generated module composition
  wow-ai/            Creature AI work
  wow-chat/          Chat validation and routing helpers
  wow-loot/          Loot logic
  wow-script/        Script integration foundation
  wow-scripts/       Script crate experiments
  wow-anticheat/     Anticheat support
  wow-logging/       Logging helpers
```

Important documentation:

```text
docs/README.md       Documentation map and authority routing
docs/migration/      Current state/plan plus marked historical records
docs/architecture/   Current contracts, design decisions and checkpoints
docs/operations/     Runtime and DB operation notes
AGENTS.md            Agent/developer operating guide for this repo
MIGRATION_STATUS.md  Redirect to the current state document
```

## Requirements

- Rust `1.98+`
- MariaDB `10.6+`
- `protoc` for protobuf-dependent crates; set `PROTOC` if it is not available on `PATH`
- A WotLK Classic `3.4.3.54261` client for manual testing
- Imported Trinity/TDB-style databases
- Extracted game data under the configured `DataDir` (`dbc`, `db2`, `maps`, `vmaps`, `mmaps` depending on the test)

## Build

```bash
cargo build --workspace
cargo test --workspace
```

For protobuf-dependent checks:

```bash
PROTOC=/path/to/protoc cargo check -p world-server
```

Release build:

```bash
cargo build -p bnet-server -p world-server --release
```

## Configuration

RustyCore reads Trinity-style config names where possible:

```text
worldserver.conf
worldserver.conf.d/
bnetserver.conf
bnetserver.conf.d/
```

Useful default ports:

| Service | Default |
|---|---:|
| Battle.net RPC TLS | `1119` |
| Battle.net REST | `8081` |
| World socket | `8085` |
| Instance socket | `8086` |

The active realm row in `auth.realmlist` must match the Rust worldserver listener and the actual client build. Build `51943` belongs to historical smoke evidence; the maintained bot wrapper defaults to `54261` unless overridden. Neither a default nor a historical smoke proves acceptance of the current server commit.

Operational DB/bootstrap details are in:

- [docs/operations/db-bootstrap.md](docs/operations/db-bootstrap.md)

## Running

Recommended order:

```bash
cargo build -p bnet-server -p world-server --release
./target/release/bnet-server
./target/release/world-server
```

Expected startup evidence includes:

- DB target logs for login, character, world, and hotfix databases
- world DB version check for `TDB 343.24081` / `cache_id=24081`
- world listener on `8085`
- instance listener on `8086`
- realm marked online

If another server occupies those ports, do not stop it implicitly. Coordinate an authorized
runtime switch or use separately configured test ports before starting RustyCore.

## Smoke Testing

Login/realm/initial enter-world smoke testing is the first runtime gate. It does not prove gameplay parity, but it catches broken auth, realm config, character enum, and initial world entry.

The integrated client-emulation smoke bot lives under [`tools/wow-test-bot`](tools/wow-test-bot/README.md).
It is an explicit live-QA profile because it can update local test-account and session data.

Minimum smoke-test path:

- Battle.net auth succeeds
- world auth succeeds
- character enum succeeds
- player login reaches `SMSG_LOGIN_VERIFY_WORLD`
- world log shows the login sequence completing

Manual client testing should only be claimed when it has actually been run for that slice.

## Testing During Development

Common focused commands:

```bash
cargo fmt --all -- --check
cargo test -p wow-packet account_data --lib
PROTOC=/path/to/protoc cargo test -p wow-world dispatch_metadata_matches_cpp_for_registered_active_opcodes --lib
PROTOC=/path/to/protoc cargo check -p world-server
git diff --check
```

First-party development uses the canonical runner. Use `quick` while iterating and `final`
once before publishing the completed commit:

```bash
./tools/validation-v2 quick --base origin/3.4.3
./tools/validation-v2 final --base origin/3.4.3
```

It is non-interactive and agent-agnostic: humans, Kimi, Codex, Grok, Claude, and other agents run
the same command and read the same exit status. The remote trust decision depends only on the PR
author's exact GitHub login.

Pull requests authored by `alseif0x` allocate no remote validation runners; external PRs retain
the full hosted checks. `./tools/validation-v2 audit` is the exhaustive budget, and every push to
`3.4.3` runs it remotely. See
[`docs/operations/validation-v2.md`](docs/operations/validation-v2.md) and
[`docs/operations/local-first-development.md`](docs/operations/local-first-development.md).

Inventory TSV files are part of the migration state. Keep their column counts valid:

```bash
awk -F '\t' 'NF != 9 { print FNR ":" NF ":" $0; bad=1 } END { exit bad }' docs/migration/inventory/r8-entities-miniphase.tsv
awk -F '\t' 'NF != 11 { print FNR ":" NF ":" $0; bad=1 } END { exit bad }' docs/migration/inventory/r3-opcodes-registry.tsv
awk -F '\t' 'NF != 16 { print FNR ":" NF ":" $0; bad=1 } END { exit bad }' docs/migration/inventory/cpp-client-handlers.tsv
```

## Porting Method

Every meaningful gameplay change should follow this shape:

1. Work a real gap within the authorized issue; reproduce it if the historical diagnosis is stale.
2. Find the exact C++ source references.
3. Compare current Rust behavior against C++ before editing.
4. Implement coherent faithful slices within the approved macrodeliverable, not a PR per helper.
5. Add focused positive and negative tests.
6. Update the owning checkpoint and any affected implementation inventory with evidence.
7. Run checks.
8. Commit the slice on its issue-linked feature branch.
9. Continue the remaining authorized work; run `./tools/validation-v2 final --base origin/3.4.3`
   from clean committed HEAD before publication.
10. When push is authorized, push and open/update the issue's PR into `3.4.3`; that does not
    authorize merging or deployment. First-party remote jobs are intentionally skipped; external
    contributions retain the configured hosted checks/review.

Do not bulk-close rows. Do not mark a runtime feature complete just because a packet parser exists. Do not trust existing Rust code just because it compiles.

If C++ appears to have a bug, document the finding. Sometimes Rust should preserve legacy behavior for compatibility; sometimes a bug can be fixed deliberately. Either way, the decision should be explicit.

## Support The Project

RustyCore takes time: protocol research, C++ archaeology, Rust porting, DB work, packet tests, client testing, and long debugging sessions.

Donations are welcome if you want to support the time needed to keep the project moving.

| Network | Wallet |
|---|---|
| BTC | `bc1qeggjcl5guwmqr0aa4emufyzyh7nu5rkfrytqy8` |
| ETH / BNB | `0xfec63e014e0bd36d77b094ff27f7e7f5d7ab67aa` |
| Solana | `9ktt1zinmwwsZXGx9x1BM995FwbAfdNWe65v1mdPgDhn` |
| XRP | `rBVvKPrQAmd5uDZ89nDgz5HbSWVD6sTbg2` |

## License

RustyCore is licensed under GPL v3. See [LICENSE](LICENSE).

WoW protocol research and server behavior are based on the public work of the TrinityCore and MaNGOS communities.

World of Warcraft is owned by Blizzard Entertainment. This project is not affiliated with, endorsed by, or sponsored by Blizzard Entertainment.
