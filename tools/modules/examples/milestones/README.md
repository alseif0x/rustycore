# Example Milestones

The #1444 example module: a RustyCore trusted linked module that uses the
`player.login`, `player.*`, `chat.*`, `world.*` and `creature.*` hook families
of source API 2 together.

- **Greeting.** `player.login` greets the player by name ("Welcome" on the
  character's first login, "Welcome back" otherwise).
- **Level milestones.** Every `level_step`-th level (default: every level) the
  player gets `level_message`, with `{level}` replaced by the new level. With
  `announce_levels = true` the milestone is also announced to every player in
  the world on the next world tick: a player hook can only message its own
  player, so the module queues the text and its `world.update` sends it.
- **Quests.** `player.quest_rewarded` counts the quests each player turned in
  this session and tells the player the running total.
- **Kill milestones.** Kills are counted per player (`player.creature_kill`);
  every `kill_milestone`-th kill (default 100) is congratulated, and the
  milestone is written to the server log on the next world tick.
- **Chat filter.** `chat.message` is a pre-decision: a message containing one
  of `blocked_words` is denied and nobody receives it, the sender included.
  Words are matched whole and case-insensitively, so blocking `ass` does not
  block `class`. With no blocked word the module does not register the
  `chat.*` family at all, so the server builds no snapshot per message.
- **Creature deaths.** Every creature death (`creature.died`) is counted and the
  total is logged at shutdown.
- **Per-hook interest.** `enabled_hooks` declares the four `player.*` hooks
  the module implements, so the server builds no snapshot for the others.

- **SQL.** `data/sql/manifest.toml` declares one `characters` migration that
  creates the module's own table `mod_example_milestones_counts`, and a
  reviewed purge script that drops it. It shows the module SQL lifecycle
  (`docs/architecture/modules.md`, "Module SQL"); nothing writes the table
  yet, because the module API gives modules no database access (durable
  module state is #583).

## Configuration

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | boolean | `true` | `false` registers nothing |
| `kill_milestone` | integer | `100` | Congratulate every Nth kill (1 to 2³²−1) |
| `blocked_words` | string | `""` | Comma-separated single words to deny in chat |
| `level_message` | string | `"Milestone reached: level {level}!"` | Level message, 1–200 bytes, no control characters |
| `level_step` | integer | `1` | Send `level_message` on every Nth level (1–255) |
| `announce_levels` | boolean | `false` | Also announce level milestones to the world |

An invalid value or an unknown key prevents activation.

## State

The counters live in memory and are lost on restart; durable module state is
#583 work. A player's counters are forgotten by `player.logout`, which the
server raises only after the logout save is confirmed. If that save fails, the
hook never fires and the player's entry stays until the next restart. That is
bounded: one small entry per character, reused if the character logs in again.

## Trying it

From the RustyCore repository root:

```bash
python3 tools/modules/rustycore-module install --path tools/modules/examples/milestones
python3 tools/modules/rustycore-module sync
```

`install --path` copies this directory to `modules/milestones`, where the
`wow-module-api` path in `Cargo.toml` resolves. To build or test it, add
`"modules/milestones"` to `members` in the root `Cargo.toml` (do not commit that
change), then:

```bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc python3 tools/modules/rustycore-module test
```

Its SQL is applied only by an operator, after `sync` recorded it:

```bash
rustycore-db module apply example.milestones          # dry run
rustycore-db module apply example.milestones --yes
```

Remove it again with `rustycore-db module remove example.milestones --yes`
(once it was applied or recorded: this keeps the table and its history and
writes a tombstone), then
`python3 tools/modules/rustycore-module remove example.milestones`, revert the
workspace member, and run `sync`. `rustycore-db module purge
example.milestones --yes` later drops the table.

## Trust

This is in-process code compiled into the server binary with full privileges.
It requires a rebuild and restart to change. There is no isolation boundary,
no stable ABI and no hot reload.
