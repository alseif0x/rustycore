# Trusted linked modules

Issue #228 earned the source API; issue #229 makes independent module repositories
compilable into the server through an explicit, reproducible step.

**Bounded source review:** local feature HEAD `7eaf8ddc`, 2026-09-05 (production code
`93e4002a`; no fresh live acceptance in this document review). #228–#231 provide
the source/build tooling, typed configuration and the bounded
`player.login -> SendSystemMessageSelf` contract. They do not yet provide general gameplay
policies, behavior scripting or host-managed persistent module state. The
[approved modularity and ECS plan](modularity-and-ecs-plan.md) defines the next #99
acceptance, implemented by the single #583 macro after #231/#578 and audited by #153
before #133 closes. #578 does not wait for the whole SDK. The approved direction below
is not an inventory of implemented APIs.

## Trust model, stated plainly

These are **in-process trusted modules, not sandboxed plugins**. A module is compiled
into the server binary, runs with its full privileges, and requires a rebuild and
restart to change. There is no isolation boundary, no stable native ABI, and no hot
reload. Install only code you would merge.

What the API does prevent is *accidental* reach: `wow-module-api` is classified
`foundation` with an empty external allowlist, so a module cannot obtain a
`WorldSession`, `Player`, `Map`, database pool or packet writer through a type.

## Layout

```
modules/<checkout>/module.toml     operator-managed, untracked
modules.lock.toml                  generated record of what was composed
crates/world-modules/              generated compositor crate
tools/modules/compose.py           sync | check
tools/modules/fixtures/            copyable example module
tools/modules/examples/milestones/  example.milestones: the source API 2 example (#1444)
```

## `module.toml`

```toml
[module]
id = "example.greeter"          # lowercase [a-z0-9_.], starts with a letter
version = "1.0.0"
display_name = "Example Greeter"
order = 0                       # optional; operator composition order

[build]
package = "example-greeter"     # the Cargo package this checkout provides
crate_path = "."                # relative to this manifest, must stay inside it
registrar = "example_greeter::register"

[compatibility]
source_api = "1"
```

## Composing

```bash
python3 tools/modules/compose.py sync     # regenerate lock + compositor
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo build -p world-modules  # composed server
python3 tools/modules/compose.py check    # CI: fail if the tree drifted
```

`sync` is an **explicit operator step**. The build never runs it or the module manager's
Git installation/update workflow; no compositor `build.rs` discovers or rewrites the
source tree implicitly. Ordinary dependency downloads still follow Cargo's configuration.

The compositor invokes registrars in the operator's `order`, then module id
(`tools/modules/compose.py::ordered` / `render_main`). Hook callbacks of every family instead
run in `ModuleId` order, because `ModuleRegistry` stores modules in a `BTreeMap`. These are two
distinct deterministic orders, neither inferred from linker inventory: `modules.lock.toml`'s
`enabled_order` records that registration order, and module SQL (#1444 H6) is applied in it
(then by version); it never sets callback order. Operator composition order does not set
callback order; a pre-decision family states its own composition rule (for `chat.message`, the
first `Deny` wins and `ReplaceText` chains in `ModuleId` order).

## What the compositor refuses before compiling

- an id, version, package, crate path or registrar that fails its format rule;
- a `crate_path` that escapes the module checkout;
- a duplicate module id or duplicate Cargo package across checkouts;
- a `source_api` this server does not provide;
- a module crate with a normal dependency (`[dependencies]` or `[target.*.dependencies]`) on
  any server workspace crate other than `wow-module-api`, whether by name, by a `path` into
  the server tree or by `workspace = true`, including through a helper crate inside the module
  checkout (#1445, ADR-002). It does not follow a path dependency that leaves both the checkout
  and the server tree: native modules are trusted code, and this rule guards against accidental
  coupling to server internals, not against a deliberate workaround. `wow-module-api` re-exports `ObjectGuid`, so neither a module nor
  its tests need `wow-core`; development dependencies are not checked.
  `tools/modules/fixtures/forbidden_dependency/` is the refusal fixture.

## `modules.lock.toml`

Records identity, version, package, source path, requested ref, resolved commit,
registrar, source API, enabled order and a content digest for every composed module,
plus its SQL record (below). It holds no credentials and no URLs with secrets.

## Module SQL (#1444 H6)

A module that needs tables ships `data/sql/manifest.toml`:

```toml
format = 1
min_core_schema = "2026.08.02.00"   # a core migration version this module builds on

[[migrations]]
database = "characters"              # characters or world only
version = "2026.10.21.00"            # dot-separated numbers, unique per database
description = "per-character milestone counts"
file = "characters/2026_10_21_00_counts.sql"   # relative to data/sql

[[purge]]                            # reviewed DROP scripts, run only by an explicit purge
database = "characters"
file = "purge/characters.sql"
```

Owner decisions (2026-10-21, recorded on #1444):

- **Scope.** A module changes only `characters` and `world`, and there only its own tables,
  named `mod_<id with . as _>_…` (`example.milestones` → `mod_example_milestones_`). No
  ALTER or DML on core tables; `auth` and `hotfixes` are closed to modules for now. Two
  modules may not share a prefix.
- **Record.** `compose.py sync` validates the manifest (files exist inside `data/sql`,
  `min_core_schema` is a version this core's migration manifest reaches) and records it in the
  lock: `[module.sql]` with `min_core_schema` and `source`, one `[[module.sql.migration]]` per
  file with its SHA-256 (CRLF normalized, as `rustycore-db` hashes), and `[[module.sql.purge]]`.
- **Immutable.** A migration recorded in the lock keeps its checksum and may not be dropped
  from the manifest: `sync`/`check` refuse a change, and the startup check refuses applied
  history whose checksum differs. A fix is a new migration file.
- **Namespace.** Module history lives in the same `rustycore_schema_history` table, with
  `component = "module:<id>"`.
- **Embedded.** When any module has SQL or a tombstone exists, the generated compositor calls
  `world_server::run_with_composition(args, modules, include_str!("../../../modules.lock.toml"))`.
  The startup check (`ModuleSchema::from_lock` + `MigrationManifest::with_module_schema`) then
  treats a composed module's migrations as `active` — the server refuses to start while one is
  pending — and a tombstoned module's as `archived`: its history is accepted with its
  checksums and nothing is required. The plain `world-server` binary and a zero-module
  composition accept no module history; their compositor is unchanged.
- **Removal keeps data.** A removed module leaves a tombstone in the lock: a block after the line
  `# --- tombstones: written by rustycore-db module remove; …` holding `[[tombstone]]` entries
  shaped like `[[module]]` ones. `compose.py` keeps that block byte for byte and refuses to
  drop a module whose SQL is in the lock unless it is tombstoned. A re-installed module must
  continue its tombstoned history. Removal never deletes data; a separate explicit purge runs
  the module's reviewed DROP scripts.
- **Static guard.** `compose.py` (`tools/modules/module_sql.py`) at sync and `rustycore-db`
  (`wow_database::migration::module_sql`) before applying or purging check every statement, with
  comments removed and strings emptied (`--` is a comment only before whitespace, as in MariaDB;
  backslash escapes are assumed, which holds because `SET` is refused):
  - migrations may `CREATE TABLE` the module's tables with columns first and then only the
    options `ENGINE` (InnoDB, Aria, MyISAM, MEMORY), `[DEFAULT] CHARSET`/`CHARACTER SET`,
    `COLLATE`, `COMMENT`, `AUTO_INCREMENT`, `ROW_FORMAT`; `ALTER TABLE` them with only
    `ADD`/`DROP`/`MODIFY`/`CHANGE`/`ALTER` clauses and the same options; `CREATE`/`DROP INDEX` on
    them; and `INSERT`/`REPLACE`/`UPDATE`/`DELETE` their rows without naming another table;
  - purge scripts may only `DROP TABLE IF EXISTS` them, so a retried purge is safe;
  - refused everywhere: qualified names, `SELECT`, `JOIN`, `RENAME`, `UNION`, partitions
    (`PARTITION`, `EXCHANGE`, `CONVERT TABLE`), `TABLESPACE`, `DATA`/`INDEX DIRECTORY`,
    `CONNECTION`, other engines, file I/O, sequences, side-effecting functions (`LOAD_FILE`,
    `GET_LOCK`, `RELEASE_LOCK`, `SLEEP`, `BENCHMARK`, …), executable `/*! */` comments, foreign
    keys to other tables and every other statement. `ALGORITHM=` and `LOCK=` clauses are refused
    too, so online-DDL hints are not available to module tables;
  - two modules may not have one table prefix inside the other (`mod_demo_` vs `mod_demo_x_`).
  Both guards are tested against the shared `tools/modules/sql_guard_vectors.json`.
- **Commands** (dry run unless `--yes`, which also requires `--config`), `rustycore-db module <apply|remove|purge> <id>`:
  - `apply` runs the composed module's pending migrations per database in version order. It
    tolerates only pending module migrations: it refuses while the schema has any other problem
    (checksum drift, incomplete rows, unknown history), while an earlier module in `enabled_order` has a
    pending migration, when `min_core_schema` is newer than the core manifest, when a file no
    longer matches its recorded checksum, or when the guard refuses it. Like `purge`, it checks
    every recorded file of both databases before it connects, and with `--yes` it dry-runs both
    databases before changing either, so a `world` refusal never follows a changed `characters`
    (#1468). It holds the same
    advisory lock as `migrate`, and a failed statement leaves the row incomplete for explicit
    recovery, like a core migration.
  - `remove` keeps every table and history row. It copies the recorded files to
    `modules/.retired/<id>/data/sql` and writes the module's tombstone into the lock. Then
    `rustycore-module remove <id>` (which refuses before that step), `sync` and a rebuild.
  - `purge` (only for a tombstoned, no longer composed module) runs the retained, re-verified and
    guarded DROP scripts, deletes the module's history rows once none of its tables remain (so an
    interrupted purge can be rerun), then its tombstone and the retained copy. Run `sync`
    afterwards.
- **Tests** use a throwaway database: the ignored `live_mariadb_module_apply_remove_and_purge`
  (`RUSTYCORE_DB_IT_USER`, `_HOST`, `_PORT`, `_PASS`, as the other `live_mariadb*` tests), run in
  CI by `.github/workflows/wow-database-live.yml`. Never point them at a server database.
- **Core migrations stay separate.** `rustycore-db status|validate|migrate` read the lock
  (`--lock`, default `modules.lock.toml`) so module history is validated, but `migrate`
  applies core migrations only; module migrations are applied in `enabled_order`, then
  version, by the operator's explicit module command.

## The example module (#1444)

`tools/modules/examples/milestones/` (`example.milestones`, source API 2) uses the `player.login`,
`player.*`, `chat.*`, `world.*` and `creature.*` families together: a login greeting, a
configurable `level_message` on level milestones, a per-session count of rewarded quests, a self
message every `kill_milestone`-th kill (also logged on the next `world.update`), a `chat.message`
pre-decision that denies messages containing one of `blocked_words`, an optional world
announcement for level milestones queued from the player hook and sent by `world.update`, and a
creature death counter logged at shutdown. `enabled_hooks` declares its four `player.*` hooks, and
the `chat.*` family is registered only when a word is blocked. Its counters are in memory only;
its README lists the typed configuration and the state boundary. It also ships module SQL
(#1444 H6c): one `characters` migration creating `mod_example_milestones_counts` and its purge
script, which show the lifecycle below; nothing writes that table until #583 gives modules
durable state. Install it with
`rustycore-module install --path tools/modules/examples/milestones`, `sync`, and add
`modules/milestones` to the workspace members to build or test it; `test_rustycore_module.py`
checks that it installs and composes, and `tools/modules/check_sql_compositor.py` does exactly
that install/sync/member step, runs its tests and `cargo check -p world-modules` on the SQL-mode
compositor, and restores the checkout (`final` runs it for module-surface changes, #1468).

## The zero-module build is unchanged

With nothing under `modules/`, the compositor is a no-op that calls
`world_server::run_with_modules` with an empty registry, and `world-server`'s own
binary still calls `world_server::run`. That entry creates an empty registry; each World
adapter entry checks `registry.handles(<family>)` and returns before constructing a snapshot
or invoking callbacks, and the domain hook ports default to doing nothing.
This is the current zero-module source path, not fresh capture proof for every server action.

## The module manager

`tools/modules/rustycore-module` is the author/operator workflow. Every command is
non-interactive and never prompts, so a shell, a script or an agent drives it the same way.

| Command | Does |
|---|---|
| `new <id>` | scaffold from the official skeleton |
| `install --path P` / `--git URL [--ref R] [--commit C]` | register a checkout |
| `update <checkout> [--ref R] [--commit C]` | fetch a Git checkout; select a revision when requested |
| `remove <id>` | delete exactly one validated checkout |
| `list` | report installed modules |
| `sync` / `check` | regenerate, or verify without writing |
| `build` / `test` | cargo build/test the composed server |
| `doctor` | diagnose the installation |

Add `--json` to any command for machine-readable manager reports; manager errors go to stderr
as JSON carrying the same exit code. `build` and `test` also inherit Cargo's output, so their
entire stdout is not guaranteed to be a single JSON document.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | usage or validation error |
| 2 | requested module not found |
| 3 | source or network error |
| 4 | refused: dirty or conflicting checkout |

### Safety

- Only `install` and `update` perform manager-controlled Git network operations. Cargo
  build/test may download dependencies and execute build scripts from the trusted source.
- Neither ever executes a script from the fetched repository: the manager clones or copies,
  reads `module.toml`, and stops.
- A rejected install leaves nothing behind — the partial checkout is removed.
- `update` refuses a checkout with local modifications rather than discarding them.
- `remove` resolves the id to exactly one validated checkout inside `modules/` and refuses any
  path that escapes it.
- The manager never runs SQL. Module SQL is recorded by `sync` and applied only through
  `rustycore-db` (see "Module SQL").

The Git v1→v2 workflow still needs product-level acceptance. Inspection found that
`install` calls `_pin`, which writes revision metadata into the checkout's `module.toml`,
while `update` rejects any dirty checkout before fetching. This can make the manager's own
metadata block an update; the risk has **not been reproduced** in this review. Existing
local-path/composition tests do not establish a successful Git upgrade and restart cycle.

### The skeleton

`rustycore-module new` produces a module that compiles and tests as-is: manifest, `src/lib.rs`
with a working `register`, a focused hook test asserting the module greets only on first login,
an example configuration, `data/sql/{characters,world}/` and a README stating the
trust model. Optional directories are documented rather than generated empty.

## Typed configuration

Options are namespaced by validated `ModuleId`, so two modules cannot collide on a key.

```toml
# module.toml — immutable package defaults, shipped with the module
[config]
enabled = true
welcome_text = "Example Greeter is installed."
```

```toml
# conf/modules/example.greeter.toml — operator overrides, outside the module repository
welcome_text = "Welcome to our realm!"
```

The override path is outside the checkout so module source updates do not replace that file.
Keep credentials out of module repositories and embedded options: composition writes typed
configuration into generated source/binaries, which are not secret storage. A separate path
does not itself guarantee that configuration values contain no secrets.

`sync` merges defaults with overrides, validates them, and **embeds the typed result** in the
generated compositor. A module therefore reads its configuration exactly once, at registration:
no callback touches a file, and there is no live reload to race against.

In the current skeleton, `enabled` is interpreted by the module itself: it suppresses the
greeting but does not prevent registration. It is not a host-managed disable/unload contract.
Changing embedded configuration requires sync, rebuild and restart.

### Validation happens before activation

A module takes the options it knows about and calls `finish()`. Anything left over is an
operator typo and fails registration, so a misspelled key is never silently ignored. Wrong types
and values the type system cannot express — a blank string, a negative count — are refused the
same way. A module that refuses its configuration is **not registered**, so an invalid value is
caught at startup rather than at a player's login.

### Digest

Every module's exact configuration has a deterministic digest, recorded in `modules.lock.toml`
and reported by `list` and `doctor`. It is computed identically on both sides — the Rust
`ModuleConfig::digest` and `compose.py` pin the same literals in their tests — so the lock always
describes what the module will actually see. Insertion order does not affect it, and the digest
is fixed at construction so it still describes what the module was *given* after it has read
its options.

## Source API compatibility

`module.toml` declares the `source_api` it was written against. Composition refuses anything this
server does not provide, with an actionable message, before a single line is compiled:

```
modules/incompatible_api/module.toml: source_api '3' is not supported;
this server provides ['1', '2']. Update the module or pin an older server.
```

`wow_module_api::SOURCE_API` and `COMPATIBLE_SOURCE_APIS` are the Rust side of the same
numbers; `test_rustycore_module.py` asserts the composer agrees with them.

## Source API 2: hook families, ports and the World adapter (#1445)

API 2 is **additive**. An API 1 module (the skeleton, `register_player_login_module`,
`PlayerLoginSnapshot` built with a struct literal, an exhaustive match on `PlayerLoginEffect`)
compiles and runs unchanged; the server records `source_api = "2"` and each module keeps its own.

**Registry by capability.** A module registers one `ModuleHooks` set with `register_module`,
offering one `Arc` per hook family it implements, so one value can serve several families and
keep one in-memory state. `register_player_login_module` is that call with only `player.login`.
A set with no family is refused. Families follow AzerothCore's script classes
(`src/server/game/Scripting/ScriptDefines/*.h`); `HookFamily` names them and
`registry.handles(family)` lets an owner skip building a snapshot nobody receives.

**Contract shape.** `wow-module-api` re-exports `ObjectGuid`, so a module names every type it
needs through that one crate. Snapshots are owned copies; API 2 snapshots are `#[non_exhaustive]` with a
constructor, so fields can be added without breaking modules. Effects are per family and
validated as one batch, all or nothing (`EffectBatch`). Confirmed-result hooks return
`NotificationEffect::SendSystemMessageSelf`; the pre-decision type `ChatDecision`
(`Allow`/`Deny`/`ReplaceText`) is declared for `chat.message`.

**Failure isolation.** Each callback runs under `catch_unwind` with `AssertUnwindSafe`: what a
panicking callback queued is discarded, and the module is disabled for every family for the rest
of the process (`HookOutcome::disabled`, `registry.is_disabled`). The other modules' batch still
applies. Native modules are trusted code; this is isolation from bugs, not a sandbox. It needs
unwinding panics, so no workspace profile may set `panic = "abort"` (a `wow-module-api` test
checks it). `world-server` registers `wow_module_api::module_callback_in_progress` with the
`wow-logging` panic hook, which then logs such a panic as contained instead of
`Trinity::Fatal`-like.

**Ports, not dependencies.** Domain crates never depend on `wow-module-api`. A hook point is a
provided method on the domain's existing host trait with an empty default body, taking a plain
domain event (for login, `ContinueLoginHostLikeCpp::player_login_hook_like_cpp` with
`PlayerLoginHookEventLikeCpp`). Where building the event costs anything (canonical Player reads
take the map lock), the port has a provided interest query too
(`player_login_hook_wanted_like_cpp`, default `false`, answered by the World from
`registry.handles`), so a zero-module server builds nothing. `WorldSession` overrides the port and calls the World adapter,
`crates/wow-world/src/session/module_hooks.rs`, the one place that converts the event to the
public snapshot, dispatches and applies the validated effects through the canonical owner. Test
hosts implement nothing. No `wow-world*` dependency edge is added, so `wow_world_closure.py` and
`dependency-policy.json` are unchanged; `wow-world-application`, `wow-world-lifecycle` and
`wow-world-core` no longer depend on the module API at all.

**Registry ownership.** The compositor's `Arc<ModuleRegistry>` is process-owned and immutable.
The session factory installs the same `Arc` the dispatch catalogs carry into each session once,
through `WorldSession::install_module_hooks_like_cpp` (a second install is refused and asserted
in debug builds); it replaces the former test-only
`SessionCore::module_registry_like_cpp`. Nothing else is installed into the session this way.

**Ordering and locks.** A port is called where C++ calls `sScriptMgr`, after the transition and
its persistence boundary (#169), and never with a split borrow, map guard, money guard or
respawn-order guard held, nor across an `.await` with a guard. World and map hooks dispatch from
`world-server` after the map lock is released. Modules never mutate the player spell map (#754):
no effect can.

`tools/modules/fixtures/incompatible_api/` is that fixture, and `doctor --json` reports
`supported_source_apis` alongside each module's own.

## Hook catalogue (#1444 first wave)

Every hook below is a confirmed-result notification unless marked otherwise. Each one is raised
after its transition and that transition's persistence boundary, exactly once, never under a
map, money or respawn-order guard, and only when the port's interest query says a module wants
it: `registry.handles_player_hook(<hook>)` for a `player.*` hook (AzerothCore
`CALL_ENABLED_HOOKS`; a module declares its hooks with `PlayerModule::enabled_hooks`, every hook
by default), `registry.handles(<family>)` for the others. `player.creature_kill` and
`player.spell_cast` are **non-persistent observations**: they report an in-memory transition
(the death state, a cast sent to the client) that has no persistence boundary of its own, not a
durable or committed result.

| Hook | Family / method | AzerothCore | RustyCore call site | Effects |
|---|---|---|---|---|
| `player.login` | `PlayerLoginModule::on_player_login` | `PlayerScript.h:332`, `CharacterHandler.cpp:1136` | `wow-world-application` `continue_login/login.rs`, last step, after `mark_player_online` | self message |
| `player.logout` | `PlayerModule::on_logout` | `PlayerScript.h:338`, `WorldSession.cpp:828` | `wow-world-application` `session_lifecycle/finalization.rs`, `Retirement` step: after `CharacterSave` and the offline marks, after the registry cleanup, before the map removal | none (session closing) |
| `player.level_changed` | `PlayerModule::on_level_changed` | `PlayerScript.h:263`, `Player.cpp:2585` | `wow-world-quest` `quest/reward/xp_grants.rs` `give_xp_like_cpp`, once per level gained (TrinityCore `Player.cpp:2231-2236`), only after the XP save applied | self message |
| `player.money_changed` | `PlayerModule::on_money_changed` | `PlayerScript.h:282`, `Player.cpp:11688` | `wow-world-quest` `quest/objectives.rs`, the post-commit `MoneyChanged` drain every money path feeds (outside the exclusive money fence); a quest reward raises its own, see below | self message; the amount cannot be changed (#583) |
| `player.quest_accepted` | `PlayerModule::on_quest_accepted` | `PlayerScript.h:750`, `PlayerQuest.cpp:426` | `wow-world-quest` `quest_session/accept_handlers.rs`, quest giver and shared-quest confirmation (after its source-item step, TrinityCore `Player.cpp:14436`), only once the status save applied or a completion reward committed it | self message |
| `player.quest_rewarded` | `PlayerModule::on_quest_rewarded` | `PlayerScript.h:245`, `PlayerQuest.cpp:903` | `wow-world-quest` `quest/reward/coordinator.rs`, last step of the reward, after its single commit, the reward XP's level hooks and the reward's own money hook (AzerothCore order: `GiveXP`, `ModifyMoney`, `OnPlayerCompleteQuest`); carries the committed balances and the offered XP | self message |
| `player.creature_kill` | `PlayerModule::on_creature_kill` | `PlayerScript.h:254`, `Unit.cpp:14298`; TrinityCore `Unit.cpp:10755` (`OnCreatureKill`) | `wow-world-entities` `creature_kill_session.rs`, the kill completion every player kill ends in (spell damage at once, map-tick melee through the session's pending-kill queue): once per `setDeathState(JUST_DIED)`, after the map lock of that transition is released, only for the session's own player | self message; non-persistent observation |
| `player.spell_cast` | `PlayerModule::on_spell_cast` | `PlayerScript.h:326`, `Spell.cpp:3825`; TrinityCore `Spell.cpp:3671` | `wow-world-spell` `execution_session/execution.rs`, the only player `SMSG_SPELL_GO` producer: once per cast with the player as caster, after the SPELL_GO and before the effects; `triggered` is C++ `TRIGGERED_IGNORE_GCD`, the one trigger flag represented (every `CastSpell(..., true)` carries it) | self message; nothing can change spells or auras (#754); non-persistent observation |
| `item.equipped` | `PlayerModule::on_equip` | `PlayerScript.h:416`, `PlayerStorage.cpp:2940` | `wow-world-inventory` `item_session`: equip into an empty slot (`moves.rs`), a real swap (`real_swap.rs`, once per item that lands in an equipment position) and the child equip, after the inventory transaction committed and the position changed | self message |
| `player.loot_item` | `PlayerModule::on_loot_item` | `PlayerScript.h:431`, `Player.cpp:13918` | `wow-world-loot-ops` `loot_handlers/item.rs`, a directly looted item once durably stored, raised after the request's loot-removed notifications; roll winners and master-loot recipients are deliberately not reported, for AzerothCore parity: it raises `OnPlayerLootItem` only at the end of `StoreLootItem` (`Player.cpp:13918`) and other hooks for rolls and master loot (`Group.cpp:1652/1736`) | self message |
| `player.loot_money` | `PlayerModule::on_loot_money` | `LootScript.h:44`, `LootHandler.cpp:273` | `wow-world-loot-ops` `session_loot/handlers/money.rs`, once for the looter after the group fanout committed (`money` is the whole `loot->gold`, `share` what the looter received), or after an item's money was durably applied; on the creature/GameObject path a repeated request on an emptied loot reports 0, as AzerothCore does; the item-held loot path raises nothing on a repeat | self message |
| `chat.message` (**pre-decision**) | `ChatModule::on_chat_message` | `PlayerScript.h:657-709` (`OnPlayerCanUseChat`); TrinityCore `PlayerScript::OnChat` | `wow-world-social` `chat_handlers`: say, yell, free-text emote, party, raid, raid warning, instance chat and whisper, after every C++ check (language, mute, flood, length, `ValidateMessage`, links, GM silence, alive, level; group resolved; whisper receiver found), right before the send | `ChatDecision`: `Allow`, `Deny`, `ReplaceText` |
| `world.config_loaded` | `WorldModule::on_config_loaded` | `WorldScript.h:53`, `World.cpp:300` (`OnAfterConfigLoad(false)`) | `world-server` `app.rs`, `WorldModuleHooksLikeCpp::after_config_load_like_cpp`, after the databases are started, the online accounts cleared and the realm marked offline, before the realm list is read (C++ reaches `LoadConfigSettings` only after `StartDB`); `reload` is always `false` (no reload exists) | log, world announcement |
| `world.startup` | `WorldModule::on_startup` | `WorldScript.h:74`, `Main.cpp:395` (TrinityCore `Main.cpp:413`) | `world-server` `app.rs`, after both listeners are bound and the realm is marked online, beside the internal `on_startup_like_cpp`; the already spawned map loop dispatches no `world.update` before it | log, world announcement |
| `world.update` | `WorldModule::on_update` | `WorldScript.h:71`, `World.cpp:1342` (TrinityCore `World.cpp:2862`) | `world-server` `runtime/map/update_loop.rs`, once per completed canonical tick, at its end, after the map manager, spawn metadata and respawn-order guards were released and after game events; only after `world.startup`; `diff_ms` is the time since the previous dispatch | log, world announcement; 5 ms budget per module, throttled warning and per-module metric |
| `world.shutdown` | `WorldModule::on_shutdown` | `WorldScript.h:77`, `Main.cpp:419` (TrinityCore `Main.cpp:440`) | `world-server` `app.rs`, after the session flush and the respawn-writer drain, beside the internal `on_shutdown_like_cpp`, before the realm is marked offline. C++ runs it before `KickAll`/`UpdateSessions(1)`/`StopNetwork`; RustyCore's later point is an intentional deviation, so no module runs while sessions are still being finalized | log, world announcement |
| `creature.added_to_world` | `CreatureModule::on_added_to_world` | `AllCreatureScript.h:43` (`OnCreatureAddWorld`, `Creature::AddToWorld`) | the creature is marked where it enters its map: a fresh canonical incarnation (`wow-world-entities` `creature_registry.rs`, the admission grid loads and respawns use) or a creature record added through `wow-map` `add_map_object_record_to_map_like_cpp`; the canonical map tick's lifecycle phase (`wow-world-creature` `creature_lifecycle_tick.rs`) reports and clears the mark on the first update that visits it, and `world-server` dispatches after the tick's guards were released (`runtime/map/update_loop.rs`, after the creature phases' delivery); non-persistent observation | log, world announcement |
| `creature.died` | `CreatureModule::on_died` | `UnitScript.h:111` (`OnUnitDeath`, raised from `Unit::Kill`, `Unit.cpp:14307`) | `wow-entities` `Creature::set_kill_death_state_runtime_like_cpp`, the kill's transition from alive into `JUST_DIED` (player melee and spells, creature melee and damage share); a forced despawn, `DisappearAndDie` or forced respawn raises nothing, as in AzerothCore; pets are not creatures here and are not reported; reported and dispatched like `added_to_world`, exactly once per transition; the killer is not reported (see `player.creature_kill`); non-persistent observation | log, world announcement |

An operation that runs on the session host raises its hook through a provided host-trait method;
one that runs inside a borrowed context (`QuestRewardCx` and the objective drain on it) raises it
through the `QuestModuleHookPortLikeCpp` the World hands the context. Both reach the same World
adapter. A quest that completes and auto-rewards on accept reports `quest_rewarded` before
`quest_accepted`, because the accept only becomes durable through that reward's commit. A hook
whose persistence failed or is unknown is not raised: modules only observe durable results.
The reward's hooks run at the end of the reward future: if that future is cancelled after the
commit, its `money_changed` and `quest_rewarded` are lost together (the reward itself stays
committed); modules must not treat a missing notification as a missing reward.

**World family (#1444 H2).** `world.*` hooks have no session. `world-server`
`module_hooks.rs` (`WorldModuleHooksLikeCpp`) dispatches them on the composition root's own
tasks, asks `registry.handles(World)` first, and applies `WorldEffect`s: `Log` writes one line
attributed to the module; `AnnounceWorld` is the port of C++ `World::SendWorldText`
(`World.cpp:2982`): each non-empty line (TrinityCore `LineFromMessage` skips empty ones) as a
`CHAT_MSG_SYSTEM` to every registered session whose player is in the world
(`PlayerRegistry::in_world_registrations_like_cpp`), addressed with the realm's virtual address
(`virtual_realm_address_like_cpp`, the formula the sessions use). Texts follow the system-message
rules, and one dispatch sends at most 8 lines across its announcements; the rest is cut and logged,
so a module cannot fill the players' socket queues. `world.update` runs on the map loop: each
module's callback is timed apart from effect application against a 5 ms budget, the first
overrun of each module is logged at once and later ones are summarized every 10 s, and calls,
total, maximum and overruns are kept per module. A tick the loop skips (map timer not passed,
barrier held, abandoned tick, shutdown, or before `world.startup`) dispatches nothing, and the
next dispatch's `diff_ms` covers it.

**Creature family (#1444 H3b).** `creature.*` hooks have no session; their effects are the
`world.*` ones. The creature carries two runtime marks, set where C++ calls the hook
(`Creature::AddToWorld`, a kill's transition into `JUST_DIED`) and consumed by the canonical tick's
lifecycle phase, which visits every creature the map updates (the cells around players and
active objects). So an event is reported by the tick after its transition, from that tick's
outcome, once every guard of the tick was released; the map loop asks `handles(Creature)` before
the guards, and without a creature module the phase reads nothing. **Coverage limit:** only a
creature in the tick's admitted selection (the map's NearbyCells update plan) is visited, so an
event for a creature outside it may never be reported, not just reported late: a creature added
and removed between two ticks, a game-event or spawn-group creature in a cell no update visits,
or one that leaves the selection before it is visited. Collecting the events from the map's
`AddToMapOutcome` instead (plan row 16) is #1462. Effect warnings of the per-tick hooks (`world.update`, `creature.*`), such as a rejected
batch or an announcement cut at the line cap, go through the same throttle as the budget
warning: the first is logged, later ones are summarized every 10 s with the suppressed count.

**Combat hooks (#1444 H3).** Both are non-persistent observations (see above).
`player.spell_cast` is raised once the cast is visible (`SMSG_SPELL_GO` sent) and before its
effects apply; a notification observes a cast that already happened, so the module's message
follows the SPELL_GO. A cast that fails before SPELL_GO (cast checks, targets, power) raises
nothing. This differs from AzerothCore, which calls `OnPlayerSpellCast` at the start of
`Spell::_cast` (`Spell.cpp:3825`), before its `CheckCast` (`Spell.cpp:3851`), so a C++ module
also sees casts that then fail. Creature and GameObject casts on the map tick raise nothing. A group kill
reports only the killing player (the group reward fanout is not ported).

**Chat pre-decision (#1444 H4).** `chat.message` is the first hook that can change what happens.
`HookFamily::Chat` is a family of its own, so a server whose modules ignore chat builds nothing
per message. The chat handlers run in a borrowed context, so the World lends it a
`ChatModuleHookPortLikeCpp`. Decisions compose in `ModuleId` order: each module sees the text the
previous ones left, the first `Deny` drops the message, and a panicking module is disabled with
its decision discarded. A replaced text goes through the client-text rules again (511 bytes,
`ValidateMessage`, emptiness, hyperlink shape); one that fails them drops the message rather than
fall back to the text a module wanted replaced, and a module-inserted link never kicks the
sender. Guild and officer chat, channels, AFK/DND replies and addon messages are not offered:
guild chat and channel broadcast are not ported, and in C++ a veto after the AFK/DND toggle has
no effect. A whisper is reported with `LANG_UNIVERSAL`, as AzerothCore `Player::Whisper` passes it
(`Player.cpp:9667-9671`).

**Security consequence: chat filtering fails open.** A chat module that panics is disabled for the
rest of the process, like any module, and its decision is discarded. A module used as a
profanity or spam filter therefore stops filtering after its first panic, and messages are sent
unfiltered from then on (the disable is logged). A filter that must fail closed is out of scope
until #583.

**Equip hook coverage (#1444 H4).** `item.equipped` is raised only by the equip handlers (empty
slot, real swap, child equip). Unlike AzerothCore, it is not raised for the equipment loaded at
login (`QuickEquipItem` in `_LoadInventory`, `PlayerStorage.cpp:6008`), for a stack merged into an
equipped stack (`PlayerStorage.cpp:2932`), or for a new character's starting outfit
(`Player.cpp:724`).

## Approved next direction — #583 under #99, not implemented yet

The [modularity and ECS plan](modularity-and-ecs-plan.md) keeps external extension contracts
independent of private runtime storage. #99 owns the module product; #578 retains its full
canonical ownership/lifetime/phase acceptance and supplies the boundaries the modules use.
#583 is the single next functional module macro; #153 audits it and #578 before #133 closes.
Use coherent macrodeliverables with internal checkpoints, not a hook-by-hook micro-PR tree.
Neither a pilot module nor a backend choice closes the full #133/#578 contract.

### Useful extension contracts

Earn three kinds of capability through real modules, without prebuilding a universal framework:

- **Pre-decision policies:** typed allow/deny/modify results applied at a named point before
  the canonical transition, with explicit ordering, conflicts and failure behavior.
- **Behavior capabilities:** scoped actions and typed replies, including interactions that
  trigger nested/reentrant callbacks. Define immediate versus deferred execution, lifetime,
  cancellation and ordering per interaction; scripts are not all passive observers, and
  deferring every action can change C++ behavior.
- **Confirmed-result notifications:** immutable observations after the relevant transition
  and persistence boundary, not a veto after the change is committed.

The host validates and applies permitted actions through canonical owners. Modules receive no
Session/Map internals, entity guards, SQL connections, ECS handles or packet writers. Preserve
C++ hook placement and base-server behavior with zero optional modules (required base scripts
remain enabled); optional custom rules have their
own explicit contract and cannot bypass core identity, economy or transaction invariants.

### Selected execution product — native Rust and operator-optional Wasm

This is **planned #583 functionality**, not a new interpretation of the currently implemented
source manager commands above. First-party and custom modules use the same extension contract;
provenance does not select an executor or make native code isolated.

- Native Rust remains the default: independent crates, explicit composition/build/restart.
- Wasmtime/Core Wasm is the selected optional adapter. Deliver versioned ABI/bindings and a
  real Rust and C guest before claiming multilang support; broader language support or WIT is
  not implied. V1 tested Rust -> Core Wasm only, not the second language or production loader.
- Share the semantic hooks, scoped actions/results, error categories and state/lifecycle rules.
  Native representations and Wasm encodings may differ. Native registered types must not require
  host cases per module; Wasm state is bounded/versioned data, not Rust pointers or ECS components
  exported to the guest. Prevent stale outer state writes after synchronous nested callbacks.
- Native and Wasm modules can compose in one host. Select exactly one executor/artifact per ID;
  reject duplicates, incompatible versions/capabilities and exclusive conflicts before dispatch.
- Schema version is distinct from mutation revision: reject stale outer state writes after
  nested callbacks. An executor switch with existing data needs a compatible durable format or
  explicit validated conversion; unsupported switches reject before callbacks with data retained.
- Wasm imports have explicit capabilities, no ambient filesystem/network/DB access, and bounded
  guest memory, nested-invocation fuel, depth, actions, payloads and host work. Fuel does not bound
  blocking host I/O. Test trap/error partial effects and the hook's reject/disable/fallback policy.
- Ship native-only and Wasm-enabled configurations. Artifact replacement is an explicit validated
  lifecycle operation with restart/drain semantics, not automatic hot reload. A new compatible
  Wasm guest can be loaded on restart without rebuilding core.

Optional **to enable** is not optional **to deliver**: this expands #583 and therefore #133/#153
acceptance. The bounded executor/Rust+C proof does not wait for M6; broader ecosystem expansion
still receives a fresh review. Existing native trust warnings remain true.

`hecs` is selected privately for composable entity state; cohesive domain aggregates remain.
Neither execution adapter depends publicly on hecs. The finite third-module conformance proof in
the [plan §5](modularity-and-ecs-plan.md#5-ecs-decision-now-selective-private-hecs-cohesive-aggregates-retained)
runs before production storage migration, without waiting for all of #583. It has not run yet.

### State, persistence and removal

Module-owned state needs a host-managed namespace, declared scope (for example character,
map instance or realm), version and lifetime. Define transfer/detach, unload, restart and
retirement behavior for each scope; persist only declared durable state. This is extension
state, not a second Player, money balance or inventory. Concrete storage APIs/formats must be
earned by the use case, not frozen here as a generic all-database repository.

A reward and the module's durable receipt/progress must have a coherent, retry-safe
commit/recovery contract. Independently saving each side is not sufficient. Confirmed-result
publication must not duplicate a reward after a lost response or restart. Preserve the same
logical reward/receipt identity across retries, new runtime incarnations and schema/config
upgrades; a data version must not accidentally become a new reward entitlement.

Module migrations extend the existing `rustycore-db` authority (#1444 H6, "Module SQL" above):
immutable checksums, `module:<id>` component history, and an embedded lock that makes the
startup check require composed modules' migrations and accept tombstoned history as `archived`.
Durable module state built on it (receipts, retry-safe rewards) remains the work below.
**Disable is not purge:** deactivate callbacks while retaining data/history; deleting module
code must not implicitly delete data. Purging requires a separate explicit operation.
MariaDB DDL may already be committed after failure: do not promise automatic rollback or a
safe binary downgrade without a proven compatible schema or explicit recovery procedure.

### Product acceptance

An independent repository must exercise useful customization through the public contract,
with no core patch or forbidden internal dependency. For a stateful module, acceptance includes
Git v1 installation, configuration, real runtime use, restart/state recovery, an approved v1→v2
migration, disable/restart with retained state, and reactivation or code removal with an honest
data-retention contract. Exercise failed upgrades, incompatible versions, interrupted operations,
duplicate requests and reward/receipt recovery where applicable. Preserve the last usable
installation until the replacement's required checks pass; distinguish source composition,
build, schema compatibility and runtime activation instead of treating `install` as deployment.

Also exercise the same supported behavior as native Rust, Rust Wasm and C Wasm, plus mixed
executor composition, zero optional modules, incompatible ABI/capabilities, duplicate execution
rejection, resource exhaustion and preserved partial effects. The real install/update/disable
workflow must cover the Wasm artifact as well as the native source module. Do not present lab
snapshot/replay as MariaDB durability or an imported guest as completed encounter integration.

These are implementation acceptance requirements, not results of this documentation update.
Use focused tests during internal cuts and the relevant production/live evidence at acceptance;
existing migration, deployment, push and merge approvals remain unchanged.

Follow the [shared reanalysis cadence](modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication):
#578 first proves conformance and a real C1/C2 vertical before replication, then completes its
C4 balance. #583 tests its first independent production module before extending the API and
finishes the durable/operator contract before #153 audits both macros. These are internal
evidence checkpoints, not another issue or routine approval for each hook.

## Remaining boundaries

Remote checkout management and typed configuration are already implemented by #230/#231.
The broader policy/behavior/state contracts and integrated migration/disable lifecycle above
remain #583 work under #99, now including the operator-optional Wasm adapter and Rust/C proof.
No production sandbox, native ABI stability or hot reload exists. The selected executor/backend
direction is not a claim of installation or acceptance; current source manager commands and the
bounded login hook remain the implemented product at the reviewed code above.
