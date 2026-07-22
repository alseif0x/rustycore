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

For the void-storage atomic persistence round-trip, set
`WOW_BOT_VOID_STORAGE_SMOKE=1` for one disposable local bot. The harness
requires an empty pre-existing void store and an isolated item
entry. Across four fresh World authentications it unlocks storage, deposits the
item, verifies the generated void-item ID after relog, swaps slot `0` to `5`,
verifies the swap after another relog, withdraws the item, and finally verifies
an empty void store plus the bound inventory item after a fourth login. Every
phase checks both the exact response packets and committed CharacterDB state;
cleanup removes only the isolated item/void rows and restores money, player
flags, and position. Optional overrides are
`WOW_BOT_VOID_STORAGE_ITEM_ENTRY` (default `2589`) and
`WOW_BOT_VOID_STORAGE_TIMEOUT_SECS` (default `8`). It normally discovers the
vault keeper's map-owned ObjectGuid from the login update stream;
`WOW_BOT_VOID_STORAGE_RUNTIME_COUNTER` is an optional checked override.

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

For an atomic extended-cost vendor round-trip, set `WOW_BOT_VENDOR_SMOKE=1`
and select exactly one local bot account. The default fixture relocates that
offline character beside G'eras (entry `18525`, SQL spawn `96654`), seeds `30`
units of currency `42`, selects the unique item/cost row `30183`/`1642`, and
buys one item for `15` currency. The bot discovers the vendor's live
map-generated ObjectGuid from `SMSG_UPDATE_OBJECT` near the exact SQL position;
`WOW_BOT_VENDOR_RUNTIME_COUNTER` is only an optional checked override. It then
requires the exact C++ `VendorInventory`, instance-routed `SetCurrency`, and
realm-routed `BuySucceeded` plus `ItemPushResult` wire shapes, fences the
capture with a fixed `CMSG_PING`, verifies currency
`30→15` plus one persisted item after logout and a fresh authentication, and
finally removes the purchased item and restores the original currency row and
position. Optional fixture overrides are `WOW_BOT_VENDOR_ENTRY`,
`WOW_BOT_VENDOR_SPAWN_GUID`, `WOW_BOT_VENDOR_ITEM_ENTRY`,
`WOW_BOT_VENDOR_EXTENDED_COST`, `WOW_BOT_VENDOR_CURRENCY_ID`,
`WOW_BOT_VENDOR_CURRENCY_COST`, and `WOW_BOT_VENDOR_CURRENCY_QUANTITY`.

For the two-session equipment-set GUID and relog smoke, use two existing
disposable local identities:

```bash
WOW_BOT_DB_CONF=/absolute/path/to/worldserver.conf \
tools/wow-test-bot/target/debug/wow-test-bot \
  --config /absolute/path/to/ignored-config.json \
  --equipment-set-race-smoke \
  --equipment-set-account-a TESTBOT2@bot.local \
  --equipment-set-account-b TESTBOT3@bot.local \
  --equipment-set-timeout 15 \
  --report /tmp/rustycore-equipment-set-race.json
```

The preflight rejects non-`@BOT.LOCAL` accounts, online characters, duplicate
identities, and characters with pre-existing equipment/transmog sets. After
that verification it writes a fresh 64-byte World session key to each fixture
account. After both fresh World logins it barriers two real
`CMSG_SAVE_EQUIPMENT_SET` packets:
one ordinary equipment set and one transmog outfit. It requires distinct
nonzero GUIDs above the pre-run maximum, exact `SMSG_EQUIPMENT_SET_ID` fields,
durable rows in the two CharacterDB tables after logout, then two new
authentications whose `SMSG_LOAD_EQUIPMENT_SET` packets contain exactly the
saved sets. Cleanup waits for both characters to be offline, removes only rows
owned by the two verified GUIDs, and proves both tables returned to empty.

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

For the three-session group-capacity race, preload a normal four-member party,
restart `world-server` so its `GroupRegistry` loads that exact state, and keep
the configured leader plus both candidates offline and on the same faction.
The leader must already be a member; neither candidate may be. Provisioning is
forbidden in this mode, so use an ignored config containing the three existing
linked identities and either valid 64-byte `session_key_bnet` fixture keys or
credentials accepted by the live BNet fallback:

```bash
WOW_BOT_DB_CONF=/absolute/path/to/worldserver.conf \
tools/wow-test-bot/target/debug/wow-test-bot \
  --config /absolute/path/to/ignored-config.json \
  --group-capacity-race-smoke \
  --group-capacity-group-id <group-db-store-id> \
  --group-capacity-timeout 30 \
  --report /tmp/rustycore-group-capacity-race.json
```

The harness waits for all three World logins, verifies both invite results,
barriers the two `CMSG_PARTY_INVITE_RESPONSE` sends, requires one `added` and
one exact `Invite/GROUP_FULL` result, then proves in CharacterDB that the four
original members plus exactly one candidate remain. Rust currently filters the
wire `PartyUpdate.PlayerList` through connected `PlayerRegistry` entries even
though C++ serializes every member slot; the harness pins that known boundary
to the connected leader/winner pair and uses the final DB assertion for the
complete five-member roster. A successful run intentionally leaves the winner
in the party. Remove only that verified row and restart `world-server` before
reusing the fixture.

For the two-session atomic loot-claim smoke, use the guarded local preflight
command with the feature-branch world executable pinned. The currently running
normal PM2 world may be a different build; its executable/profile is snapshotted
and restored after QA:

```bash
test -z "$(git status --porcelain=v1 --untracked-files=normal)"
TARGET_EXEC="$(realpath /absolute/path/to/issue-106/world-server)"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
RUST_CAPTURE_DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf \
RUST_CAPTURE_EFFECTIVE_CONFIG=/home/server/trinity-legacy-install/etc/worldserver.conf \
WOW_BOT_DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf \
WOW_BOT_WORLD_EXEC="$TARGET_EXEC" \
WOW_BOT_WORLD_EXEC_SHA256="$(sha256sum "$TARGET_EXEC" | awk '{print $1}')" \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$(sha256sum "$BOT_EXEC" | awk '{print $1}')" \
./tools/pr-preflight.sh --allow-runtime-qa \
  --ack-disposable-overworld-loot-race qa-loot-race
```

The DB fixture guard and the PM2 runtime config are separate pins on this host:
the former reads the full legacy `bin/worldserver.conf`, while the latter must
match the Rust PM2 profile's effective `etc/worldserver.conf` exactly.

This live mode is never part of normal CI. Both flags are mandatory because the
guard temporarily mutates a world GameObject fixture and two disposable
characters. The wrapper translates that explicit acknowledgement to the
binary's CLI-only, legacy-named
`--ack-disposable-overworld-loot-race` guard; direct binary runs must pass
`--loot-race-smoke --ack-disposable-overworld-loot-race` themselves. The
default disposable characters are `TESTBOT2@bot.local` and
`TESTBOT3@bot.local` (the versioned config maps them to character GUIDs `15`
and `16`). Setup rechecks each configured GUID's account ownership and requires
one character per account; every destructive cleanup query remains scoped to
those two verified GUIDs.

Both loot modes force `WOW_BOT_ENSURE_TEST_ACCOUNTS=0`: provision the two
disposable identities first. They also require an absolute
`WOW_BOT_FIXTURE_JOURNAL`. The bot creates that mode-0600 journal before its
first mutation, removes it after verified restoration, then atomically writes
the mode-0600 JSON marker `${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete`. The
marker records schema version `1`, the journal SHA-256, and the cleanup PID. A
matching journal+marker pair is recoverable/idempotent; a mismatched marker is
rejected. The guarded preflight supplies a private path automatically and will
not restart the normal PM2 world while the journal is pending or the marker is
missing, unsafe, or malformed. Loot workflows also have a hard end-to-end
deadline of 900 seconds; override it with
`WOW_BOT_LOOT_WORKFLOW_DEADLINE_SECS` or `--loot-workflow-deadline`.

The wrapper `exec`s loot-mode bots and enables Linux parent-death signalling so
TERM/INT reaches the handler registered before fixture mutation and a dead
wrapper cannot leave a detached mutator. If a run leaves a journal, keep the
normal world stopped and run explicit recovery with the same DB configuration:

```bash
WOW_BOT_DB_CONF=/absolute/path/to/worldserver.conf \
WOW_BOT_FIXTURE_JOURNAL=/absolute/path/to/fixture.journal \
tools/wow-test-bot/target/debug/wow-test-bot --recover-loot-fixture
```

Recovery performs no login or service action. Only after it verifies bounded
database restoration does it remove the journal and produce the cleanup marker;
the operator may then let the outer capture/preflight wrapper restore PM2.

The race fixture is the existing Tattered Chest template (entry `2846`, loot
template `2278`) plus wrapper-owned spawn `9106001`. The guard temporarily
installs that exact spawn and changes only the chest addon's original `0/0`
money bounds to deterministic `10/10`; the loot template must contain exactly
one unconditional item `38`. It fails closed on template/data drift, a
pre-existing spawn or GameObject respawn, loot conditions, or pool/event/linked,
spawn-group, addon, or override ownership. Its full template-addon fields are
pinned as well. The group must report C++ `PERSONAL_LOOT` (`5`). The default runtime
counter is `0` (auto): each client discovers and preserves the complete live
GameObject ObjectGuid from `SMSG_UPDATE_OBJECT`. The SQL spawn id remains
`9106001`, but, as in C++, the live low counter is generated independently by
the map and is not assumed to equal that spawn id. A nonzero
`WOW_BOT_LOOT_RACE_RUNTIME_COUNTER` is only a strict override; it never replaces
live discovery.

The smoke snapshots and relocates both characters, forms the real two-player
party, positions both clients at the chest, and synchronizes two
`CMSG_GAME_OBJ_USE` requests for the same packed GUID. A separate barrier proves
that both `SMSG_LOOT_RESPONSE` packets arrived before the competing item and
money claims begin. It verifies one global item award, exact database
persistence after logout/relogin, and fail-closed loser evidence.

Chest money intentionally does not use the corpse group split. In C++
`HandleLootMoneyOpcode`, `LOOT_CHEST` keeps `shareMoney=false`: the first
serialized requester receives all `10` copper with `SoleLooter=true`, the
second receives `0` with `SoleLooter=true`, and both active viewers observe two
matching `CoinRemoved` notifications because both requests notify viewers. The
database total must therefore increase by exactly `10`, and the persisted
winner must match the positive wire notification.

After both clients log out, the bot restores its bounded character, item,
group, and respawn snapshots. The outer fixture guard stops the QA world before
removing unchanged spawn `9106001` and restoring the addon's `0/0` money bounds;
external drift is reported instead of overwritten. Cleanup validates the whole
spawn, including persisted `state`, before any delete; the bot must not reset
that field to conceal runtime drift.

For the item proof, every observed `ItemPushResult` is retained before checking
its entry. The harness requires one winner payload, at most one identical copy
per socket, one exact `LootGone` on the loser, and one exact `LootRemoved` per
viewer; it repeats that proof after the later money settle window. Rust's
complete `StoreLootItem` side-effect cascade is deliberately tracked by #55,
so this #106 atomicity smoke accepts either winner-only delivery or one
identical C++ group broadcast per socket, but never a divergent or duplicate
logical grant. The winner's complete wire Item GUID,
player owner, entry, count, and top-level backpack slot are then keyed back to
one `item_instance` plus `character_inventory` row and checked again after
relogin. The database stores only the Item GUID counter (not its wire high/realm
half) and does not persist packet-only fields such as `QuestLogItemID`, display,
or delivery flags; those fields are validated from the wire where the fixed
fixture makes them deterministic, not claimed as database-verifiable evidence.

Overrides are available through `WOW_BOT_LOOT_RACE_ACCOUNT_A/B`,
`WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY`,
`WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID`,
`WOW_BOT_LOOT_RACE_RUNTIME_COUNTER`, `WOW_BOT_LOOT_RACE_ITEM_ENTRY`, and
`WOW_BOT_LOOT_RACE_TIMEOUT_SECS`. The older
`WOW_BOT_LOOT_RACE_CREATURE_ENTRY` and
`WOW_BOT_LOOT_RACE_CREATURE_SPAWN_GUID` names remain fallback aliases so
existing callers do not break; they describe the GameObject only in race mode.

The preflight first snapshots and accredits the exact normal PM2 executable,
then drives `capture-rust.sh loot-two-session-atomic-race --yes` through a FIFO.
After its ready marker, it accredits the new capture PID against the pinned
target path/hash and world/instance listeners. That identity must not restart or
change during the bot run. The preflight then signals completion, waits for the
capture wrapper to stop the QA world and restore the fixture plus original PM2
profile, and accredits the restored original executable again. The wrapper gets
the same fresh `WOW_BOT_REPORT` path and pinned bot path/hash and independently
requires the exact successful two-session report before it can publish a
completed capture. A failed report therefore still drives fixture/runtime
cleanup but leaves no completed race artifact. Bot failure, capture drift, or
wrapper cleanup failure all remain failures.

The bot endpoints are forced to `WORLD_HOST=127.0.0.1`, the accredited world
port, `INSTANCE_HOST=127.0.0.1`, and an expected `INSTANCE_PORT`; the bot rejects
a different port in `SMSG_CONNECT_TO`. Because the server may still bind a
wildcard listener, run this destructive QA only on an isolated host or network
namespace whose firewall restricts the BNet/world/instance ports to loopback
traffic. This prevents another client from entering the temporary fixture world
or a green result coming from a service other than the accredited process.

For a capture-diff golden, use the separate single-session item-only mode:

```bash
FIXTURE_RECOVERY_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-loot-item.XXXXXX")"
FIXTURE_JOURNAL="$FIXTURE_RECOVERY_DIR/fixture.journal"
if WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
    WOW_BOT_FIXTURE_JOURNAL="$FIXTURE_JOURNAL" \
    WOW_BOT_LOOT_ITEM_CAPTURE=1 \
    WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
    ./run_rustycore_login_smoke.sh \
    && test ! -e "$FIXTURE_JOURNAL" \
    && test -f "${FIXTURE_JOURNAL}.cleanup-complete"; then
  rm -f -- "${FIXTURE_JOURNAL}.cleanup-complete"
  rmdir -- "$FIXTURE_RECOVERY_DIR"
else
  echo "fixture recovery evidence retained at $FIXTURE_RECOVERY_DIR" >&2
  false
fi
```

It uses the separate deterministic Doctor Maleficus fixture (entry `21779`,
unique world spawn `1117`, guaranteed item `30712`, and no coin pool)
with the same fail-closed character, inventory, respawn, and cleanup checks.
The item is The Doctor's Key: account A's exact top-level keyring slot
`bag=0, slot=106` must start empty, the wire push and persisted inventory row
must both use that slot, and cleanup must restore it to empty. A matching key
placed in an ordinary backpack slot does not pass.
Its full runtime ObjectGuid is auto-discovered by default as above; the SQL
spawn id is not assumed to be the live ObjectGuid counter.
Only account A connects; account B stays offline and is used solely by the
existing bounded fixture snapshot. The bot kills and opens the creature, sends
one `CMSG_LOOT_ITEM`, and requires exactly one matching
`SMSG_LOOT_REMOVED` plus one owner-correct `SMSG_ITEM_PUSH_RESULT`, refuses any
money or inventory-failure packets, then emits a fixed-serial zero-latency
`CMSG_PING` fence immediately after those two responses and verifies its Pong
before logout. It does not retain the two-client smoke's two-second evidence
settle window. It also verifies the one item row persisted
and that neither fixture character's money changed before cleanup. This mode
must not be confused with the two-session race; it exists so the global C++ and
Rust packet logs contain one attributable session for
`loot-single-item-claim`.

`config.example.json` is versioned with blank passwords. Use `WOW_BOT_PASSWORD`,
the per-account `WOW_BOT_PASSWORD_<ACCOUNT>` override, or an ignored local
`config.json` for credentials. Do not commit real local bot passwords.

Outside loot modes, the RustyCore wrapper defaults to
`WOW_BOT_ENSURE_TEST_ACCOUNTS=1`: it
generates an ignored `.env.local` password when none exists, then either creates
both missing local `@bot.local` BNet/game auth rows or validates an already
complete identity exactly before running. It never updates credentials,
reassigns a character, repairs a partial BNet/game identity, clears a ban, or
changes an online/realm-count mismatch. Set `WOW_BOT_GENERATE_LOCAL_PASSWORD=0` or
`WOW_BOT_ENSURE_TEST_ACCOUNTS=0` to disable that local QA bootstrap.
Loot modes override this variable to `0` even when the caller or `.env.local`
sets it to `1`.
Explicit caller environment values always win over `.env.local`, including
mode/acknowledgement, endpoint, executable/hash, and credential variables; the
wrapper suppresses xtrace while it loads and restores those values.

Build and test this bot with Rust 1.88.0 (`cargo +1.88.0 ...`), matching the
RustyCore toolchain.

For QA on a host that must not compile locally, the wrapper accepts an exact
prebuilt executable only when both `WOW_BOT_EXEC` (an absolute, canonical,
non-symlink path) and its `WOW_BOT_EXEC_SHA256` are supplied. The optional
`qa-artifact` PR workflow builds both `world-server` and `wow-test-bot` twice on
separate GitHub runners, requires byte-identical replicas, and publishes their
verified hashes. Without `WOW_BOT_EXEC`, the wrapper keeps the normal local
Rust 1.88.0 build behavior.
