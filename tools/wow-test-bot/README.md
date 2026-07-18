# WoW Test Bot

Headless Rust bot used to smoke-test RustyCore and the legacy TrinityCore test
server. This copy is integrated into the RustyCore repo as a QA tool.

For the current RustyCore login gate, see:

- `RUSTYCORE_SMOKE.md`
- `run_rustycore_login_smoke.sh`

The wrapper also supports `WOW_BOT_QUEST_SMOKE=1` for questgiver/gossip QA. It
logs in, targets a configured creature entry/spawn, verifies the server responds
with gossip/list/details packets, and writes the quest ids/titles to the JSON
report. For mutating accept-flow QA, combine `WOW_BOT_QUEST_RESET=1`,
`WOW_BOT_QUEST_RELOCATE=1`, `WOW_BOT_QUEST_SET_RACE=<id>`,
`WOW_BOT_QUEST_SET_CLASS=<id>`, `WOW_BOT_QUEST_SET_LEVEL=<1-80>`, and
`WOW_BOT_QUEST_ACCEPT=1`; the bot will prepare the selected test character,
accept the expected quest, and verify `character_queststatus`. The bot always
requires the live map-generated creature GUID counter separately from the
`world.creature.guid` spawn identity, matching C++ `Creature::LoadFromDB`.

For quest objective persistence QA, add `WOW_BOT_QUEST_OBJECTIVE_PERSIST=1`
and `WOW_BOT_QUEST_OBJECTIVES=<storage:data,...>`. The bot seeds the expected
quest as active in `character_queststatus`, seeds nonzero
`character_queststatus_objectives` rows, logs in, sends logout, and verifies the
objective rows survived the server's logout save.

For a full personal-bank persistence round-trip, set `WOW_BOT_BANK_SMOKE=1`
and `WOW_BOT_BANK_RUNTIME_COUNTER=<live-counter>`. The bot creates one isolated
test item for the local bot character, opens a neutral banker, deposits the
item, logs out and authenticates again, withdraws it, logs out again, verifies
both DB transitions, then deletes the item and restores the original position.

For an innkeeper bind round-trip, set `WOW_BOT_HOMEBIND_SMOKE=1`. The bot relocates one local
test character beside a faction-friendly continent innkeeper, sends
`CMSG_BINDER_ACTIVATE`, requires `SMSG_SPELL_GO`, `SMSG_BIND_POINT_UPDATE`,
`SMSG_PLAYER_BOUND`, and `SMSG_GOSSIP_COMPLETE`, verifies
`character_homebind`, authenticates again, re-verifies persistence, and restores
the original position and homebind row. It discovers the map-owned low GUID
counter from the login `SMSG_UPDATE_OBJECT` stream;
`WOW_BOT_HOMEBIND_RUNTIME_COUNTER` remains an optional capture override.

For a full occupied-slot inventory-swap round-trip, set
`WOW_BOT_INVENTORY_SWAP_SMOKE=1`. The bot creates two different isolated items
in two free backpack slots, sends the real `CMSG_SWAP_INV_ITEM`, logs out and
authenticates again, swaps them back, verifies both items after both commits,
and removes the fixture. Optional entry overrides are
`WOW_BOT_INVENTORY_SWAP_ITEM_ENTRY_A/B` (defaults `2589`/`2592`).

For a complete rested-XP calculation and consumption round-trip, set both
`WOW_BOT_RESTED_XP_SMOKE=1` and `WOW_BOT_ACK_DISPOSABLE_RESTED_XP=1` when using
the wrapper. The bot records and later restores only the selected character
fields needed by this smoke; it does not back up every character/account table.
The underlying binary therefore requires the CLI-only
`--ack-disposable-rested-xp` flag and rejects shared or non-clean fixtures. It
verifies wilderness/resting offline accrual, attacks a nearby creature, validates
the `SMSG_LOG_XP_GAIN` rested split and DB persistence, relogs, and waits for the
target's normal runtime respawn to remove its DB row. The default target is Mana
Wyrm entry `15274`; see `RUSTYCORE_SMOKE.md` for the full destructive-fixture
contract and overrides. Cleanup is deliberately bounded: it restores selected
character fields plus exact achievement snapshots and removes only the listed
baseline-zero login rows. Any other rows created by the server remain visible,
which is why the acknowledged disposable identity and clean preflight are
mandatory.

`config.example.json` is versioned with blank passwords. Use `WOW_BOT_PASSWORD`,
the per-account `WOW_BOT_PASSWORD_<ACCOUNT>` override, or an ignored local
`config.json` for credentials. Do not commit real local bot passwords.

The RustyCore wrapper defaults to `WOW_BOT_ENSURE_TEST_ACCOUNTS=1`: it
generates an ignored `.env.local` password when none exists, then upserts only
local `@bot.local` BNet/game account rows with matching SRP credentials before
running. Set `WOW_BOT_GENERATE_LOCAL_PASSWORD=0` or
`WOW_BOT_ENSURE_TEST_ACCOUNTS=0` to disable that local QA bootstrap.

Build and test this bot with Rust 1.88.0 (`cargo +1.88.0 ...`), matching the
RustyCore toolchain.

For QA on a host that must not compile locally, the wrapper accepts an exact
prebuilt executable only when both `WOW_BOT_EXEC` (an absolute, canonical,
non-symlink path) and its `WOW_BOT_EXEC_SHA256` are supplied. The optional
`qa-artifact` PR workflow builds both `world-server` and `wow-test-bot` twice on
separate GitHub runners, requires byte-identical replicas, and publishes their
verified hashes. Without `WOW_BOT_EXEC`, the wrapper keeps the normal local
Rust 1.88.0 build behavior.
