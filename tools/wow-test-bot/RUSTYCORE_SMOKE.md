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

The bot still supports the previous LFG path. Do not use LFG as the RustyCore
migration gate until the server-side LFG port is explicitly ready.

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

For DB-spawned creatures the bot intentionally uses `world.creature.guid` as
the visible GUID counter, matching C++ `Creature::LoadFromDB` /
`CreateFromProto`. A questgiver that only responds to a different counter is a
server GUID bug, not a bot override case.

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
