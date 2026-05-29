# CLAUDE.md

This file is the operating guide for Claude Code in this repository. Keep it factual and current. If it conflicts with the current worktree or with C++ source, the current worktree plus C++ source wins.

## Project And Source Of Truth

RustyCore is a Rust port of a TrinityCore-derived World of Warcraft Wrath/Cata-classic private server. The port target is full functional parity with the legacy C++ server, not a smaller compatible subset.

- Rust repo: `/home/server/rustycore`
- Legacy C++ reference: `/home/server/woltk-trinity-legacy`
- Remote: `https://github.com/alseif0x/rustycore.git`
- Main work branch: `develop`
- `main` is kept fast-forward synced at stable checkpoints.
- Rust toolchain: Rust 1.85, edition 2024.
- `protoc`: `/home/cdmonio/.local/protoc/bin/protoc`

Do not trust existing Rust, old AI summaries, or migration docs as correctness proof. Always contrast behavior against the C++ source before implementing or approving a change.

## Current Checkpoint

As of the last audited port state before this documentation refresh:

- Last audited port base: `1af9223 Add honest progress audit (R8-entities)`
- At that base, `develop`, `origin/develop`, `main`, and `origin/main` all pointed at `1af9223`.
- Tree expected clean on `develop`.
- Latest documented coverage count: `736/759 = 96.97%`.
- Latest handoff item: `TEST-DEBT / #NEXT.R8.ENTITIES.765`.

Start every session with:

```bash
cd /home/server/rustycore
git status --short --branch
git log --oneline --decorate -8
head -n 20 docs/migration/current-session-handoff.md
```

If HEAD has moved beyond `1af9223`, audit the commits instead of trusting their messages. A documentation-only commit that updates this file is not a new port base; code-bearing commits must still be reviewed against C++:

```bash
git log --oneline 1af9223..HEAD
git diff --stat 1af9223..HEAD
git diff 1af9223..HEAD
```

Only promote a newer commit to the reliable base after reviewing it against C++ and validating tests/docs.

## Mandatory Porting Method

Every implementation slice must follow this sequence:

1. Inspect current repo state and latest handoff.
2. Pick a real documented gap from `docs/migration/current-session-handoff.md` or the inventory files.
3. Locate exact C++ source anchors in `/home/server/woltk-trinity-legacy`.
4. Compare existing Rust against C++ before editing.
5. Implement the smallest faithful Rust change that moves the full port forward.
6. Add focused tests, preferably positive and negative branches.
7. Update migration docs/checklists with the new `#NEXT.R8.ENTITIES.xxx` item when closing a represented implementation gap.
8. Recalculate progress honestly.
9. Run validation.
10. Commit on `develop`, push, fast-forward `main`, push, and return to `develop` only at stable closure points.

Do not do "bulk close" inventory edits. A closed `#NEXT` item must correspond to real code and tests, with exact C++ refs, Rust targets, checks run, and remaining boundaries stated. Discovering or documenting a gap is useful, but it is not an implementation closeout.

Do not mark anything `manual-test-ready` unless it has actually been installed/restarted and exercised manually against the client/runtime.

## Build And Test

Use `PROTOC` explicitly for any command that may compile protobuf-dependent crates:

```bash
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p world-server
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p wow-world --lib
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p wow-map --lib
```

Fast iteration commands:

```bash
cargo fmt --check
cargo fmt --all -- --check
cargo test -p wow-world some_test_name --lib
cargo test -p wow-map some_test_name --lib
cargo clippy -p wow-map -p wow-world --all-targets
git diff --check
```

TSV inventory files must keep 9 tab-separated columns:

```bash
awk -F '\t' 'NF != 9 { print FNR ":" NF ":" $0; bad=1 } END { if (bad) exit 1; print "TSV_OK" }' docs/migration/inventory/r8-entities-miniphase.tsv
```

Current useful baselines from recent handoff:

- `wow-world --lib`: clean in recent runs.
- `wow-map --lib`: cleaned to `614/0` in `#NEXT.R8.ENTITIES.765`.
- `world-server` check passes with existing warnings.

If a test fails, do not assume production is wrong or the test is wrong. Contrast with C++ and document which one it is.

## Architecture: Current Runtime Reality

The runtime currently has three coexisting world models. This is important; old notes that describe a single pending `MapManager` integration are stale.

1. Legacy `wow_world::MapManager`
   - Shared as `Arc<RwLock<...>>` from `crates/world-server/src/main.rs`.
   - Shared across sessions.
   - Runs represented creature AI/combat through session-driven ticks such as `tick_creatures_sync` and `tick_combat_sync`.
   - Has no independent global clock; it advances when logged-in sessions tick it.

2. Canonical `wow_map::MapManager`
   - Owns the global canonical map tick loop (`spawn_canonical_map_update_loop`, about 10ms).
   - Has a C++-like `Map::Update` structure and map/spawn/respawn infrastructure.
   - Creature runtime update currently uses default context and does not dispatch real AI/combat side effects such as `AiUpdateTick` or `MeleeAttackIfReady`.

3. Global world loop
   - Ticks the canonical `wow_map::MapManager`.
   - Does not tick the legacy `wow_world::MapManager`.

Regression anchors from `#NEXT.R8.ENTITIES.764`:

- `canonical_map_update_visits_creature_with_no_real_ai_combat_effect_like_cpp`
- `two_sessions_sharing_legacy_map_manager_see_same_creature_state`

The old statement that `WorldSession` owns a `creatures: HashMap<ObjectGuid, CreatureAI>` field is false. Do not build new work around that field.

Incremental live-runtime roadmap from handoff:

1. Characterize current split. Done in `#NEXT.R8.ENTITIES.764`.
2. Give the legacy map a sessionless clock.
3. Add creature movement fanout from global tick via per-map session registry.
4. Move combat resolution to global clock, resolving once rather than per session.
5. Unify respawn from per-session queue into canonical runtime.
6. Move to a single source of truth for creatures, method by method.
7. Add real `SendObjectUpdates`, scripts, weather, threat, and remaining fanout.

Steps 2+ are architectural-risk work. Avoid big-bang rewrites. Previous `_attic/` attempts failed with large compile-error blasts; use them only as historical context.

## Important Current Open Gaps

The exact list changes as the port advances. Always read the handoff first. Current repeatedly documented gaps include:

- Full `ConditionMgr` target/searcher/map/world-state/active-event coverage.
- `Player::SatisfyQuestBreadcrumbQuest` recursive `CanTakeQuest` gate.
- `SatisfyQuestTimed`, day, week, month gates at accept.
- GM override visibility and server-side visibility infrastructure.
- AI override dialog status.
- Battleground chest `CanActivateGO`.
- Live-runtime / map-manager tick integration.
- Runtime install/restart/manual client-test readiness for many represented slices.

Do not use this list as exhaustive; use the migration inventory as the source for current planning.

## Migration Documents

Primary current-state docs:

- `docs/migration/current-session-handoff.md`
- `docs/migration/inventory/r8-entities-miniphase.md`
- `docs/migration/inventory/r8-entities-miniphase.tsv`
- `docs/migration/honest-progress-audit.md` (honest progress audit; or similarly named audit docs if present).
- `docs/MIGRATION_ROADMAP.md` (phase-ordered execution plan) and `docs/migration/_INDEX.md` (per-module status/audit). Use them for plan/order; their status snapshots predate the R8-entities work and have drifted, so they are not proof of current state.

Older snapshots such as `MIGRATION_STATUS.md` may be stale. They can help find concepts but are not proof of current parity.

When updating docs:

- Put newest items at the top where the file already follows reverse chronological order.
- Include C++ refs, Rust targets, acceptance, checks, and boundaries.
- Keep `represented-partial` unless full runtime parity is actually proven.
- Do not inflate progress with planning-only or test-debt-only work.

## Packet Handler Dispatch

The world server uses static registration via the `inventory` crate. A handler runs only if it both:

- has a dispatcher match arm, and
- registers a `PacketHandlerEntry` via `inventory::submit!`.

Forgetting `submit!` can silently drop the opcode even if the match arm exists. Each `PacketHandlerEntry` declares opcode, `SessionStatus`, and `PacketProcessing` mode. See `crates/wow-handler/src/lib.rs`.

Handler modules live under `crates/wow-world/src/handlers/`.

## Coding Patterns

- Prefer existing local helpers and `*_like_cpp` functions over inventing new abstractions.
- Use C++ names/order when mirroring C++ behavior.
- Collect packets into `Vec<Vec<u8>>` before sending when it avoids borrow conflicts.
- In tick methods, use `send_tx.send(pkt.to_bytes())` rather than `send_packet` if `send_packet` would double-borrow.
- `Position` fields are `.x`, `.y`, `.z`, `.orientation`; not `.o`.
- Import `wow_packet::ClientPacket` explicitly in handler modules that decode packets.
- Use `rg` for searching.
- Use `apply_patch` for manual edits.
- Do not revert unrelated user/agent changes without explicit instruction.

## Runtime / Config

Two primary binaries:

- `bnet-server`: Battle.net auth, TCP+TLS on `1119`, REST on `8081`. Reads `BNetServer.conf` and PEM files.
- `world-server`: game server, TCP on `8085` / `8086`. Reads `WorldServer.conf`.

MariaDB databases: `auth`, `characters`, `world`, `hotfixes`.

Gitignored runtime files may contain credentials or keys:

- `*.pem`
- `BNetServer.conf`
- `WorldServer.conf`
- root `world-server` / `bnet-server` binaries

Never stage credentials, certs, local configs, or built binaries.

## Git Discipline

Stable closeout workflow:

```bash
git status --short --branch
cargo fmt --check
# focused tests
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p world-server
git diff --check
git add <changed files>
git commit -m "<short faithful summary>"
git push origin develop
git checkout main
git merge --ff-only develop
git push origin main
git checkout develop
git status --short --branch
```

Only do this after the slice is genuinely validated. If the tree contains changes from another agent, audit them before building on top of them.

## Local Context Files

The `.gitignore` excludes local agent/workflow files that may exist and contain useful context, such as `AGENTS.md`, `PLAN.md`, `MIGRATION_STATUS.md`, `INVENTORY.md`, `memory/`, `.claude/`, `.agents/`, `.openclaw/`, and similar directories. Read them if useful, but do not commit ignored local context unless the user explicitly asks for it.
