# RustyCore smoke tests

This bot can be used as a headless client harness for RustyCore. Treat it as an
E2E regression tool only: C++ TrinityCore remains the protocol/source-of-truth.

## Current safe gate

The first RustyCore gate is login-only:

```text
BNet auth -> world auth -> CMSG_ENUM_CHARACTERS -> CMSG_PLAYER_LOGIN -> SMSG_LOGIN_VERIFY_WORLD
```

This intentionally does not run Dungeon Finder/LFG. LFG is a later gate, after
the corresponding Rust server port is ready.

## What was adapted

- `--login-only`: stops immediately after `SMSG_LOGIN_VERIFY_WORLD`.
- `--quest-smoke`: after login, resolves one creature questgiver, sends
  `CMSG_GOSSIP_HELLO`, falls back to `CMSG_QUEST_GIVER_HELLO`, optionally sends
  `CMSG_QUEST_GIVER_QUERY_QUEST`, and reports the quest ids/titles received.
- `--bank-smoke`: runs a real `CMSG_BANKER_ACTIVATE` → `CMSG_AUTOBANK_ITEM` →
  logout/relogin → `CMSG_AUTOSTORE_BANK_ITEM` → logout persistence round-trip
  using an isolated local fixture item.
- `--inventory-swap-smoke`: creates two distinct occupied backpack slots, sends
  `CMSG_SWAP_INV_ITEM`, logs out/re-authenticates, swaps them back, and verifies
  both atomic DB transitions before cleaning the isolated local fixture.
- `--rested-xp-smoke`: records a bounded set of fields from one disposable local
  bot character, verifies offline wilderness/resting accrual, attacks a real
  creature, checks `SMSG_LOG_XP_GAIN` and DB consumption, relogs, restores those
  selected fields, and verifies natural target respawn cleanup. It requires the
  CLI-only `--ack-disposable-rested-xp` acknowledgement.
- `WOW_BOT_LOGIN_ONLY=1`: env equivalent of `--login-only`.
- `WOW_BOT_CLIENT_BUILD` / `WOW_BOT_BUILD`: build value printed by the smoke,
  default `54261`.
- `WOW_BOT_PASSWORD`: shared local password for accounts in `config.example.json`.
- `WOW_BOT_PASSWORD_<ACCOUNT>`: per-account override, with non-alphanumeric
  account characters replaced by `_` and letters uppercased (for example
  `WOW_BOT_PASSWORD_TESTBOT1_BOT_LOCAL`).
- `WOW_BOT_AUTH_DB_URL`, `WOW_BOT_CHAR_DB_URL`, `WOW_BOT_WORLD_DB_URL`: optional
  DB URL overrides. If omitted, the bot reads `LoginDatabaseInfo`,
  `CharacterDatabaseInfo`, and `WorldDatabaseInfo` from `WOW_BOT_DB_CONF`
  (default `/home/server/trinity-legacy-install/etc/worldserver.conf`).
- JSON reports now include `login_only`, `world_auth`, `enum_characters`, and
  `player_login_verified`. Quest smoke reports additionally include
  `quest_smoke_passed`, target entry/spawn/map, `quest_ids_seen`,
  `quest_titles_seen`, and `quest_failure`.

The bot crate follows RustyCore's Rust 1.88.0 toolchain. Use `cargo +1.88.0` for
standalone builds/tests of `tools/wow-test-bot`.

The bot still supports the previous LFG path. Do not use LFG as the RustyCore
migration gate until the server-side LFG port is explicitly ready.

## Personal bank persistence smoke

Use the live low counter announced for the selected neutral banker; it is not
the persistent `world.creature.guid` spawn id:

```bash
WOW_BOT_BANK_SMOKE=1 \
WOW_BOT_BANK_RUNTIME_COUNTER=<live-banker-counter> \
./run_rustycore_login_smoke.sh
```

The pass requires all of these results: banker interaction opened, the fixture
item persisted from an empty backpack slot to an empty bank slot, a new full
authentication/login observed it there, withdrawal persisted back to the
backpack, and the second logout completed. Setup is restricted to `@bot.local`
accounts; cleanup removes only the generated item and restores the original
character position. `WOW_BOT_BANK_ITEM_ENTRY` (default `2589`) and
`WOW_BOT_BANK_TIMEOUT_SECS` are optional overrides.

## Innkeeper homebind persistence smoke

```bash
WOW_BOT_HOMEBIND_SMOKE=1 \
./run_rustycore_login_smoke.sh
```

The pass requires the triggered bind spell plus all three bind/gossip response
packets, a matching `character_homebind` row, and a second complete
authentication/login that observes the same persisted row. Setup is restricted
to `@bot.local` accounts and cleanup restores the original character position
and homebind exactly.
`WOW_BOT_HOMEBIND_RUNTIME_COUNTER` is an optional override; the current
default discovers the map-owned low counter from the login update stream.

## Direct inventory swap persistence smoke

```bash
WOW_BOT_INVENTORY_SWAP_SMOKE=1 \
./run_rustycore_login_smoke.sh
```

The pass requires two isolated items to exchange occupied backpack slots, a
full relog to observe the persisted forward state, and the inverse exchange to
persist after the second logout. Setup and cleanup are restricted to
`@bot.local` accounts. `WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_A/B` (defaults
`2589`/`2592`) and `WOW_BOT_INVENTORY_SWAP_TIMEOUT_SECS` are optional.

## Rested XP accrual and consumption smoke

Run the complete offline-accrual, kill-consumption, DB-persistence, and relog
round-trip with:

```bash
WOW_BOT_RESTED_XP_SMOKE=1 \
WOW_BOT_ACK_DISPOSABLE_RESTED_XP=1 \
./run_rustycore_login_smoke.sh
```

The wrapper converts `WOW_BOT_ACK_DISPOSABLE_RESTED_XP=1` into the mandatory
`--ack-disposable-rested-xp` CLI flag. The bot binary deliberately has no
environment-variable bypass for that acknowledgement, and the flag is rejected
unless rested-XP smoke is enabled.

The default fixture uses Mana Wyrm entry `15274`, simulates `86400` seconds
offline, and allows `120` seconds for the live protocol phase. The conservative
bound accommodates the current unarmed canonical-player damage boundary while
still failing closed on an invalid target or a stalled combat stream. It validates both
offline rates (wilderness and a resting location), then gives the character a
known rest pool and attacks a real nearby creature. A pass requires the
corresponding `SMSG_LOG_XP_GAIN`, its base/rested split, matching XP/rest values
in the character DB, and a fresh authentication/login that observes the same
persisted values and `restState`. The live bot does not yet decode the nested
ActivePlayer XP/RestInfo fields inside `SMSG_UPDATE_OBJECT`; their atomic mask
and values remain covered by focused packet/unit tests, not claimed as a live
wire assertion here.

Each rested-XP phase requests a normal logout, handles the stock C++ wilderness
countdown (including time-sync traffic), then closes both realm and instance
sockets and waits for a stable offline character row before reading
persistence. The bounded DB wait also covers C++'s 60-second raw socket-loss
session expiry if a runtime closes before `SMSG_LOGOUT_COMPLETE`. The workflow
does not assume that `Player::GiveXP` writes the database before character
save.

Useful overrides:

- `WOW_BOT_RESTED_XP_CREATURE_ENTRY` / `--rested-xp-creature-entry`: target
  creature template entry; default `15274`.
- `WOW_BOT_RESTED_XP_CREATURE_GUID` / `--rested-xp-creature-guid`: optional
  exact persistent `world.creature.guid` spawn identity.
- `WOW_BOT_RESTED_XP_RUNTIME_COUNTER` / `--rested-xp-runtime-counter`:
  optional live map-generated `ObjectGuid` low counter. It is fail-closed: the
  same counter must be discovered near the selected SQL spawn in that login's
  `SMSG_UPDATE_OBJECT` stream; it cannot bypass spawn discovery.
- `WOW_BOT_RESTED_XP_OFFLINE_SECS` / `--rested-xp-offline-secs`: simulated
  offline interval; default `86400`. It must fit `uint32` and be smaller than
  the current Unix timestamp.
- `WOW_BOT_RESTED_XP_TIMEOUT_SECS` / `--rested-xp-timeout`: live protocol
  timeout; default `120`. Natural-respawn cleanup uses the larger of this value
  and the persisted runtime `respawnTime` plus a 15-second tick grace. A pass
  requires observing the DB transition from a present timer to a stable absent
  row; absence alone is not treated as proof of respawn. The precheck accepts
  only SQL respawns from 30 through 600 seconds, and the observed wait has a
  fail-closed 900-second safety bound.

The GUID overrides are not normally required: the bot selects the requested
spawn near the fixture position and discovers its live counter in the login
`SMSG_UPDATE_OBJECT` stream. The wire GUID does not contain the persistent SQL
spawn id, so the harness links both identities fail-closed through entry, map,
and proximity to the selected spawn's SQL home position. Once it discovers the
live position, it first acknowledges active-mover initialization with
`CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` (required by C++ visibility), then sends
a complete `CMSG_MOVE_HEARTBEAT` to stand one yard away
and face the target before `CMSG_ATTACK_SWING`. During the bounded combat wait,
it answers every periodic `SMSG_TIME_SYNC_REQUEST` with the C++-layout
`CMSG_TIME_SYNC_RESPONSE`. This mirrors a real client and keeps the session
active while a full-health target takes longer than the idle-session interval,
instead of silently ceasing to advance player auto-attacks.

Without an explicit trusted runtime counter, preflight also rejects another
same-entry/map SQL spawn whose movement radius overlaps the selected target's
matching sphere. This prevents a nearby wanderer from satisfying discovery
while cleanup watches the wrong persistent spawn.

This is a destructive **disposable-fixture-only** smoke, not a rollback-safe test
for a normal character. Its cleanup restores the explicitly recorded
`characters` fields used by the workflow (level/XP/rest flags and bonus,
logout marker, location, health/powers, kill counters, and played time). Because
preflight proves they start empty, cleanup also removes only this fixture's
deterministic login/save rows from `character_glyphs`, `character_reputation`,
`character_skills`, and `battlenet_account_transmog_illusions`. These rows are
the bounded defaults materialized by a stock C++ login/save on the disposable
fixture; an unexpected protected-table mutation is still left visible and
fails the next preflight. It also snapshots and exactly
restores `character_achievement` and `character_achievement_progress`, which a
real C++ kill can mutate, plus `character_trait_config` and
`character_trait_entry`, where C++ login/save can materialize missing
specialization defaults. It likewise preserves optional homebind, fishing, and
battleground rows, all game-account last-played rows, and Battle.net pet slots;
stock C++ can replace or materialize those during login/save. The smoke fails
closed unless `PlayerSave.Stats.MinLevel=0`, because enabling that diagnostic
table makes C++ rewrite `character_stats` on logout, and unless
`PlayerStart.AllSpells=0`, which prevents configuration-driven spell
materialization. Other
character/account/Battle.net tables remain outside that bounded restore.

The precheck therefore requires an `@bot.local` identity whose configured email
matches the Battle.net owner of the selected game account, with exactly one
character on its game account and exactly one game account on its Battle.net
identity, `characters.at_login = 0`, no Recruit-A-Friend or group membership,
and no active quest/objective/criteria state. It also rejects non-empty
high-risk state in `character_inventory`, pets, auras, spell cooldowns/charges,
skills, glyphs, talents, spells/favorites, action bars, reputation,
equipment/transmog sets, CUF profiles, corpses, tutorials, account instance
locks, guild membership, void storage, Battle.net pets, and Battle.net
collection tables. The target must have no on-kill
reputation and no pre-existing respawn row. These checks reduce collateral
mutation; they do not turn the bounded field restore into a complete database
backup. If the server creates rows in any other protected table during the
login/logout cycle, cleanup intentionally leaves them visible so the next
preflight fails instead of hiding a new side effect.

After the workflow, the harness waits for the server's normal runtime respawn
to remove the target's `respawn` row and verifies stable absence. It never
deletes that row manually. If the bounded wait expires, the smoke fails and
reports the remaining spawn/map/`respawnTime`; wait for the runtime respawn
before retrying. Do not interrupt cleanup, never point this mode at a player's
normal character, and inspect `rested_xp_failure` after any failure.

Default artifacts:

```text
/tmp/rustycore-bot-rested-xp-smoke.log
/tmp/rustycore-bot-rested-xp-smoke-report.json
```

The report summary exposes `rested_xp_smoke_passed`, both offline bonuses,
target entry/spawn/runtime ids, XP packet `amount`/`original`, DB XP/rest values
before and after consumption, `rested_xp_relog_verified`, and
`rested_xp_failure`.

## Default RustyCore smoke command

Use the wrapper:

```bash
./run_rustycore_login_smoke.sh
```

By default it runs one account:

```text
TESTBOT1@bot.local
```

It writes:

```text
/tmp/rustycore-bot-login-only.log
/tmp/rustycore-bot-login-only-report.json
```

The expected report shape for a pass is:

```json
{
  "login_only": true,
  "results": [
    {
      "account": "TESTBOT1@bot.local",
      "world_auth": true,
      "enum_characters": true,
      "player_login_verified": true,
      "join_result": null
    }
  ]
}
```

If no bot password is configured, the wrapper generates one in ignored
`tools/wow-test-bot/.env.local` and exports it for the run. By default it also
passes `--ensure-test-accounts`, which upserts only local `@bot.local` BNet/game
account rows with SRP credentials matching that password, clears local test
account lock/ban state, verifies the configured character GUID exists, and
syncs `realmcharacters`.

Disable these local QA helpers with:

```bash
WOW_BOT_GENERATE_LOCAL_PASSWORD=0 \
WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
./run_rustycore_login_smoke.sh
```

## Useful overrides

```bash
WOW_BOT_PASSWORD='local-password' WOW_BOT_ACCOUNT=TESTBOT2@bot.local ./run_rustycore_login_smoke.sh

BNET_HOST=127.0.0.1 BNET_PORT=8081 \
WORLD_HOST=127.0.0.1 WORLD_PORT=8085 \
INSTANCE_HOST=127.0.0.1 REALM_ID=1 \
WOW_BOT_BUILD=54261 \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

If the DB names or credentials differ from the runtime config, either point at
that config:

```bash
WOW_BOT_DB_CONF=/path/to/worldserver.conf \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

or set explicit DB URLs through an ignored `tools/wow-test-bot/.env.local`.

## Quest / gossip smoke

Use this when QA needs to prove that a visible questgiver actually responds and
that the offered quest set matches class/race/level expectations:

```bash
WOW_BOT_QUEST_SMOKE=1 \
WOW_BOT_QUEST_CREATURE_ENTRY=15513 \
WOW_BOT_QUEST_EXPECT_ID=<hunter-training-quest-id> \
WOW_BOT_QUEST_FORBID_TITLE_CONTAINS='Mage' \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

For deterministic accept-flow QA, let the bot prepare a test character before
login:

```bash
WOW_BOT_QUEST_SMOKE=1 \
WOW_BOT_QUEST_CREATURE_ENTRY=15278 \
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

For objective load/save QA, seed an active quest with objective counters, then
force a real logout and compare `character_queststatus_objectives` after the
server save:

```bash
WOW_BOT_QUEST_SMOKE=1 \
WOW_BOT_QUEST_CREATURE_ENTRY=15278 \
WOW_BOT_QUEST_MAP_ID=530 \
WOW_BOT_QUEST_EXPECT_ID=9393 \
WOW_BOT_QUEST_OBJECTIVE_PERSIST=1 \
WOW_BOT_QUEST_OBJECTIVES=0:1 \
WOW_BOT_PASSWORD='local-password' \
./run_rustycore_login_smoke.sh
```

Useful quest overrides:

- `WOW_BOT_QUEST_CREATURE_ENTRY`: required creature template entry.
- `WOW_BOT_QUEST_CREATURE_GUID`: optional exact `world.creature.guid` spawn.
- `WOW_BOT_QUEST_GUID_COUNTER`: optional live `ObjectGuid` low counter. C++ `Creature::LoadFromDB` uses a map-generated lowguid for the live creature and keeps the DB spawn guid separately.
- `WOW_BOT_QUEST_MAP_ID`: optional map override used for GUID construction.
- `WOW_BOT_QUEST_EXPECT_ID`: require this quest id in list/details.
- `WOW_BOT_QUEST_FORBID_ID`: fail if this quest id is offered.
- `WOW_BOT_QUEST_FORBID_TITLE_CONTAINS`: fail if any offered title contains this
  text, case-insensitive.
- `WOW_BOT_QUEST_QUERY_DETAILS=0`: skip the non-mutating
  `CMSG_QUEST_GIVER_QUERY_QUEST` details probe.
- `WOW_BOT_QUEST_RESET=1`: remove the expected quest from the selected bot
  character's active/rewarded quest tables before login.
- `WOW_BOT_QUEST_RELOCATE=1`: move the selected bot character near the resolved
  creature spawn before login.
- `WOW_BOT_QUEST_SET_LEVEL=<1-80>`: set the selected bot character level before
  login so class/race/level filters are tested deterministically.
- `WOW_BOT_QUEST_SET_RACE=<id>`: set the selected bot character race before
  login for race-gated quest QA.
- `WOW_BOT_QUEST_SET_CLASS=<id>`: set the selected bot character class before
  login for class-gated quest QA.
- `WOW_BOT_QUEST_ACCEPT=1`: send `CMSG_QUEST_GIVER_ACCEPT_QUEST` after details
  and verify the quest persisted in `character_queststatus`.
- `WOW_BOT_QUEST_OBJECTIVE_PERSIST=1`: seed the expected quest and objective
  rows, logout, and verify `character_queststatus_objectives` survived the
  server save.
- `WOW_BOT_QUEST_OBJECTIVES=<storage:data,...>`: objective rows to seed for
  persistence QA, using C++ `QuestObjective::StorageIndex` values.
- `WOW_BOT_QUEST_OBJECTIVE_STATUS=<n>`: optional quest status for the seeded
  `character_queststatus` row; defaults to `3` (incomplete).
- `WOW_BOT_ENSURE_TEST_ACCOUNTS=0`: skip the default local `@bot.local`
  account/password bootstrap.
- `WOW_BOT_ALLOW_NONLOCAL_ACCOUNT_BOOTSTRAP=1`: allow
  `--ensure-test-accounts` to touch non-`@bot.local` accounts. Keep this off for
  normal QA.

When no exact spawn guid is provided, the bot uses the selected character's
saved map/position and picks the nearest `world.creature` row for the requested
entry. The character must still be close enough in-game for RustyCore's
interaction-distance checks.

For DB-spawned creatures, `world.creature.guid` is the persistent spawn
identity while the live `ObjectGuid` low counter is map-generated. Quest and
bank mode therefore accepts an explicit runtime counter instead of pretending
the two identities are interchangeable.

## Known notes

- The bot updates `account.session_key_bnet` for the selected test account.
- `config.example.json` intentionally keeps passwords blank. Use env overrides
  or an ignored local `config.json`; never commit real bot passwords.
- `tools/wow-test-bot/.env.local` is ignored and is the right place for local
  DB URL overrides or bot passwords when needed.
- The current RustyCore BNet server does not expose the old C++ bot-only
  `/login/srp/` route. The bot falls back to `/bnetserver/login/`, then writes
  the generated world key into the auth DB for the world handshake.
- `SMSG_CONNECT_TO` / instance socket is handled by the current bot code; older
  C++ notes saying realm-socket-only are stale for this tree.
- Keep raw tickets/session keys out of chat and commit messages. Use the report
  booleans and sanitized log grep lines for status.
