# AGENTS.md

This file is the shared operating guide for AI agents working in this repository (Claude Code, Codex, and others). `CLAUDE.md` imports this file via `@AGENTS.md`, so Claude Code reads it too. Keep it factual and current. If it conflicts with the current worktree or with C++ source, the current worktree plus C++ source wins.

## Project And Source Of Truth

RustyCore is a Rust port of a TrinityCore-derived World of Warcraft Wrath/Cata-classic private server. The port target is full functional parity with the legacy C++ server, not a smaller compatible subset.

- Rust repo: `/home/server/rustycore`
- Legacy C++ reference: `/home/server/woltk-trinity-legacy`
- Remote: `https://github.com/alseif0x/rustycore.git`
- Branch model: **version branches, TrinityCore-style** (#4). The integration **and default**
  branch is **`3.4.3`** (the WoW 3.4.3 target; cf. TrinityCore's `3.3.5`/`cata_classic`). All work
  is feature branches → PR into `3.4.3`; releases are tags on `3.4.3`.
- `main` is retained as an optional stable pointer (ff-advanced at release checkpoints); it is no
  longer the default and not used for day-to-day work. A pre-rebrand backup is `backup/pre-3.4.3-rebrand`.
- Rust toolchain: Rust 1.98.0, edition 2024.
- `protoc`: `/home/ubuntu/.local/protoc/bin/protoc` (it is on `PATH`; the harnesses discover it
  rather than hard-coding a home, so a different machine needs no edit here)
- Development host: **aarch64**. GitHub runners are x86_64, so any measurement that depends on the
  machine — stack budgets, timings, perf numbers — must say which of the two produced it.

Do not trust existing Rust, old AI summaries, or migration docs as correctness proof. Always contrast behavior against the C++ source before implementing or approving a change.

### Reference Priority

The legacy C# server and older C#-based notes are secondary historical references only. They are useful for finding intent, old diagnostics, or previous packet experiments, but they are not an authority for this port.

For protocol, gameplay, database, map/runtime, and persistence behavior, the final implementation must be anchored to the C++ source under `/home/server/woltk-trinity-legacy` or to a real client/server packet capture when C++ is incomplete or ambiguous. Do not approve a layout, field order, bit count, opcode response, or runtime rule merely because a Rust comment says "C# format", "C# ref", or "matches C#".

When touching code that still cites C#:

1. Treat the C# citation as suspect until checked.
2. Locate the equivalent C++ packet/class/function.
3. Update the comment to cite C++ once verified.
4. If C++ and C# disagree, stop and document the discrepancy before changing Rust.
5. If keeping C# behavior intentionally, explain why C++ does not answer the case and add the packet capture/client-build evidence.

## Current Checkpoint

**Status source of truth (2026-06-27 refresh):** the per-module status snapshots below and in
`_INDEX.md` / `MIGRATION_ROADMAP.md` §3 are **stale and not authoritative**. Use these instead:

- `docs/migration/STATE.md` — honest current state (audited HEAD, subsystem-level).
- `docs/migration/PORT_PLAN.md` — the two-part plan (Part 1 playable M0–M6, Part 2 full-1:1 ledgers) + D-track.
- `docs/migration/EXISTING-CODE-DEFECTS.md` — bugs found in already-shipped code.
- `docs/migration/adr-runtime-tick-ownership.md` — runtime architecture decision.

**Plan execution:** work the GitHub issues in the order of the pinned index issue (port-plan
tracking). Part 1 = playable end-to-end (milestones M0–M6, issues #7–#47 + the C#-audit issues
#50–#66); validate every PR with the capture-diff harness (#66). **When Part 1 is done
("playable end-to-end" at M6.2 / #47), run a fresh planning pass before Part 2**: re-audit HEAD,
then break the Part-2 epic (#48, ledgers L1–L25) into PR-sized child issues at that point — see
the "Part 2 transition gate" in `PORT_PLAN.md`. Do not pre-granulate Part 2 now.

The old `1af9223` "last audited base" is **~1400 commits stale** — do not treat it as the
reliable base, and do not cite the `96.97%` / `98.15%` headline (it measured "represented",
not a working server; see STATE.md §0/§1). The 1.8 MB `current-session-handoff.md` is **frozen**
(append-log; read STATE.md instead).

Start every session with:

```bash
cd /home/server/rustycore
git status --short --branch
git log --oneline --decorate -8
sed -n '1,80p' docs/migration/STATE.md
```

If HEAD has moved, audit the commits against C++ instead of trusting their messages. A
documentation-only commit is not a new port base; code-bearing commits must still be reviewed
against C++ before being relied upon.

## Working An Issue (session kickoff)

The port plan is tracked as GitHub issues (`alseif0x/rustycore`), ordered by a `[NN]` prefix
in the title (the pinned `[INDEX]` issue lists them top-to-bottom). **GitHub's #numbers are
creation order — ignore them; follow `[NN]`.** One issue = one session = one branch = one PR.

To start a session on an issue, the kickoff is:

```
Work issue #<N> (alseif0x/rustycore).
1. Read: AGENTS.md, docs/migration/STATE.md, and the issue (gh issue view <N>).
2. C++ is the source of truth (/home/server/woltk-trinity-legacy): contrast BEFORE editing.
3. Smallest faithful change + focused tests (positive/negative); validate with PROTOC=... cargo check/test.
4. Git: create the branch LINKED to the issue with `gh issue develop <N> --base 3.4.3`
   (not a bare `git checkout -b`), 1 issue = 1 PR into `3.4.3` (put `Closes #<N>` in the PR body), commit per gap, NO push unless asked.
   Once push is approved, open the PR immediately; creating the PR is not the same as closing/merging it.
5. For first-party PRs authored by exactly `alseif0x`, use `./tools/validation-v2 final` plus
   focused evidence. External PRs retain remote CI/review. Require capture-diff only when bytes,
   metadata, connection choice or observable ordering changed, and runtime QA only for a live
   lifecycle/runtime change.
```

**Linking the branch/PR to the issue:** the repo's **default branch is `3.4.3`** (the version/
integration branch), so a PR into `3.4.3` with `Closes #<N>` in its body **links the issue and
auto-closes it on merge** — always put it in the PR body (GitHub honors closing keywords only
for PRs targeting the default branch). To also get a linked *branch* in the issue's *Development*
panel, start the branch with `gh issue develop <N> --base 3.4.3` (not a bare `git checkout -b`);
this must happen **at branch creation** — an existing branch **cannot be linked retroactively**
(the `createLinkedBranch` API only creates new branches from the issue; to re-attach a closing
keyword on an existing PR you must toggle its body via REST `gh api -X PATCH repos/.../pulls/N`).

Caveats: respect dependency order — don't start an issue whose deps are open (e.g. M3 spell
prerequisites need M0.1 stores; many "done" criteria need the harness `[01]/#66`). Work issues
in `[NN]` order from the top of the `[INDEX]`. The issues are self-contained (C++ refs + Rust
target + done criteria inline). Part 2 (epic #48) stays a single epic until Part 1 lands.

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
10. Commit on the issue's feature branch, push, and open a PR into `3.4.3` (`Closes #<N>`).
    For an `alseif0x` PR, the local final harness and focused evidence are the required gate;
    hosted checks intentionally skip. For any other author, require the configured remote checks
    and reviewer verdict. In both cases, fix or explicitly defer actionable review comments and
    resolve conversations before merge. Tag releases on `3.4.3`.

Do not do "bulk close" inventory edits. A closed `#NEXT` item must correspond to real code and tests, with exact C++ refs, Rust targets, checks run, and remaining boundaries stated. Discovering or documenting a gap is useful, but it is not an implementation closeout.

Do not mark anything `manual-test-ready` unless it has actually been installed/restarted and exercised manually against the client/runtime.

## Build And Test

Use `PROTOC` explicitly for any command that may compile protobuf-dependent crates:

```bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo check -p world-server
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p wow-world --lib
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p wow-map --lib
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

Local-first commands for ordinary development:

```bash
# During iteration (path-routed lightweight checks):
./tools/validation-v2 quick --base origin/3.4.3

# Before push, after committing to a clean HEAD:
./tools/validation-v2 final --base origin/3.4.3
```

Run focused tests explicitly when behavior changes. `./tools/validation-v2 audit` is the
exhaustive budget; it is not the daily pre-push gate and every push to `3.4.3` runs it remotely.
See `docs/operations/validation-v2.md` and `docs/operations/local-first-development.md`.

For ordinary architecture/module work, use the syntax-only Session ownership ratchet:

```bash
cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check --syntax-only
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py self-test
```

`session-ownership-check -- check` **without** `--syntax-only` also recomputes the exhaustive
workspace persistence inventory and can run for many minutes. It is reserved for an explicitly
requested persistence audit, release/scheduled audit, or investigation that actually changes that
inventory; it is not an ordinary iteration or pre-push command. Do not pipe validation through
`head`, `grep`, or a trailing command that can hide its exit status. When an exact syntax baseline
must move, generate `print-baseline` into a temporary file, review the semantic delta, install it
only after that review, and rerun `check --syntax-only`.

TSV inventory files must keep 9 tab-separated columns:

```bash
awk -F '\t' 'NF != 9 { print FNR ":" NF ":" $0; bad=1 } END { if (bad) exit 1; print "TSV_OK" }' docs/migration/inventory/r8-entities-miniphase.tsv
```

Current useful baselines from recent handoff:

- `wow-world --lib`: clean in recent runs.
- `wow-map --lib`: cleaned to `614/0` in `#NEXT.R8.ENTITIES.765`.
- `world-server` check passes with existing warnings.

If a test fails, do not assume production is wrong or the test is wrong. Contrast with C++ and document which one it is.

## QA Bot / Client Automation

The live client QA bot is an integrated project QA tool, not throwaway scratch code or a separate
side project.

- Integrated bot path: `tools/wow-test-bot`
- It is intentionally excluded from the root Cargo workspace so its live-QA dependencies and local
  runtime assumptions do not affect normal `cargo check/test` or CI.
- Temporary experiments may use a `/tmp/...` copy when sandbox permissions require it, but useful
  bot improvements must be ported back to `tools/wow-test-bot` and committed with the RustyCore PR
  before the work is considered preserved.
- Keep server fixes and bot-tooling changes logically separated in the commit message/body when
  practical. Mention the bot scenario and report path in issue/PR QA notes when a PR depends on a
  new bot capability.

Baseline login smoke:

```bash
cd /home/server/rustycore/tools/wow-test-bot
WOW_BOT_PASSWORD='local-password' ./run_rustycore_login_smoke.sh
```

Useful overrides:

```bash
WOW_BOT_PASSWORD='local-password' WOW_BOT_ACCOUNT=TESTBOT5@bot.local ./run_rustycore_login_smoke.sh
WOW_BOT_PASSWORD='local-password' WOW_BOT_REPORT=/tmp/rustycore-bot-report.json WOW_BOT_LOG=/tmp/rustycore-bot.log ./run_rustycore_login_smoke.sh
WOW_BOT_PASSWORD='local-password' BNET_HOST=127.0.0.1 BNET_PORT=8081 WORLD_HOST=127.0.0.1 WORLD_PORT=8085 REALM_ID=1 ./run_rustycore_login_smoke.sh
```

Quest/gossip smoke for QA of one questgiver:

```bash
WOW_BOT_QUEST_SMOKE=1 \
WOW_BOT_QUEST_CREATURE_ENTRY=<creature-template-entry> \
WOW_BOT_QUEST_EXPECT_ID=<quest-id> \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

Optional quest overrides include `WOW_BOT_QUEST_CREATURE_GUID` for an exact
`world.creature.guid`, `WOW_BOT_QUEST_MAP_ID`, `WOW_BOT_QUEST_FORBID_ID`,
`WOW_BOT_QUEST_FORBID_TITLE_CONTAINS`, `WOW_BOT_QUEST_QUERY_DETAILS=0`,
`WOW_BOT_QUEST_RESET=1`, `WOW_BOT_QUEST_RELOCATE=1`,
`WOW_BOT_QUEST_RUNTIME_COUNTER=<visible-counter>`, `WOW_BOT_QUEST_SET_RACE=<id>`,
`WOW_BOT_QUEST_SET_CLASS=<id>`, `WOW_BOT_QUEST_SET_LEVEL=<1-80>`, and
`WOW_BOT_QUEST_ACCEPT=1`.

For deterministic quest accept QA, prefer a fully specified flow that prepares the selected test
character before login, for example:

```bash
WOW_BOT_QUEST_SMOKE=1 \
WOW_BOT_QUEST_CREATURE_ENTRY=15278 \
WOW_BOT_QUEST_RUNTIME_COUNTER=90 \
WOW_BOT_QUEST_MAP_ID=530 \
WOW_BOT_QUEST_EXPECT_ID=9393 \
WOW_BOT_QUEST_RESET=1 \
WOW_BOT_QUEST_RELOCATE=1 \
WOW_BOT_QUEST_SET_RACE=10 \
WOW_BOT_QUEST_SET_CLASS=3 \
WOW_BOT_QUEST_SET_LEVEL=3 \
WOW_BOT_QUEST_ACCEPT=1 \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

The bot authenticates against the live `bnet-server`, writes/uses auth DB session data, connects to
`world-server`, enumerates characters, and enters world. It requires the local runtime and MariaDB
databases (`auth`, `characters`, `world`, `hotfixes`) to be available. Do not print, stage, or
commit bot credentials, local configs, DB URLs with secrets, generated logs containing secrets, or
runtime certificates. `tools/wow-test-bot/config.example.json` is versioned with blank passwords;
use `WOW_BOT_PASSWORD`, `WOW_BOT_PASSWORD_<ACCOUNT>`, or an ignored local `config.json`.
`tools/wow-test-bot/.env.local` is also ignored and may be used for local-only passwords or DB URL
overrides. If DB URL env vars are omitted, the bot reads database connection info from
`WOW_BOT_DB_CONF` (default `/home/server/trinity-legacy-install/etc/worldserver.conf`).

When extending the bot for an issue:

1. Anchor packet layouts/opcodes to C++ source or to a real capture before sending new packets.
2. Add a focused CLI mode or wrapper script for the QA scenario, not a one-off manual edit.
3. Report pass/fail as structured JSON fields so PR QA can cite the exact checks.
4. Keep the first version narrow: login, one interaction, one expected server response, then expand.
5. Commit useful bot work under `tools/wow-test-bot`; do not leave the only copy in `/tmp`.

Runtime bot QA complements, but does not replace, `cargo` validation and CI. A PR is not
`manual-test-ready` merely because bot code exists; install/restart the target server build, run the
bot scenario, and record the result in the PR/issue. Until CI has a full live server + DB fixture,
bot runs are local QA evidence rather than required GitHub status checks.

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

Version-branch model (#4): `3.4.3` is the default/integration branch; one feature branch per
issue → PR into `3.4.3`. No `develop`→`main` ff dance.

Per-issue closeout workflow:

```bash
# at kickoff: gh issue develop <N> --base 3.4.3 --checkout   (creates the linked branch)
git status --short --branch
# focused tests
git add <changed files>
git commit -m "<short faithful summary>"
./tools/validation-v2 final --base origin/3.4.3
git push origin <feature-branch>        # NO push unless asked
# after push, open the PR into 3.4.3 with `Closes #<N>` in the body.
# alseif0x PRs use the local evidence above and allocate no hosted validation runner.
# External PRs must satisfy the configured remote checks and reviewer verdict.
# Resolve or explicitly defer every actionable review comment before merge.
```

Only do this after the slice is genuinely validated. If the tree contains changes from another agent, audit them before building on top of them.

Branch protection on `3.4.3` keeps linear history and conversation resolution. Remote validation
jobs are author-gated: they skip for the exact trusted login `alseif0x` and remain required for
external authors. Never broaden trust to an author-association role.

## Local Context Files

The `.gitignore` excludes local agent/workflow files that may exist and contain useful context, such as `AGENTS.md`, `PLAN.md`, `MIGRATION_STATUS.md`, `INVENTORY.md`, `memory/`, `.claude/`, `.agents/`, `.openclaw/`, and similar directories. Read them if useful, but do not commit ignored local context unless the user explicitly asks for it.
