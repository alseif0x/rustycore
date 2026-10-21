# Example Milestones

The #1444 example module: a RustyCore trusted linked module that uses the
`player.*`, `world.*` and `creature.*` hook families of source API 2 together.

- **Level milestones.** Every `level_step`-th level (default 10) the player gets
  a system message. With `announce_levels = true` the milestone is also
  announced to every player in the world on the next world tick: a player hook
  can only message its own player, so the module queues the text and its
  `world.update` sends it.
- **Kill milestones.** Kills are counted per player (`player.creature_kill`);
  every `kill_step`-th kill (default 100) is congratulated, and the milestone is
  written to the server log on the next world tick. The counter is forgotten at
  logout.
- **Creature deaths.** Every creature death (`creature.died`) is counted and the
  total is logged at shutdown.
- **Per-hook interest.** `enabled_hooks` declares the three `player.*` hooks
  the module implements, so the server builds no snapshot for the others.

The counters live in memory and are lost on restart; durable module state is
#583 work.

## Trying it

From the RustyCore repository root:

```bash
python3 tools/modules/rustycore-module install --path tools/modules/examples/milestones
python3 tools/modules/rustycore-module sync
PROTOC=/home/ubuntu/.local/protoc/bin/protoc python3 tools/modules/rustycore-module test
```

`install --path` copies this directory to `modules/milestones`, where the
`wow-module-api` path in `Cargo.toml` resolves. Remove it again with
`python3 tools/modules/rustycore-module remove example.milestones` and run
`sync`.

## Trust

This is in-process code compiled into the server binary with full privileges.
It requires a rebuild and restart to change. There is no isolation boundary,
no stable ABI and no hot reload.
