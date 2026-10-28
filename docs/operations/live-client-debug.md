# Live client debug notes

These notes are for validating the current Rust runtime against the local WotLK
Classic client and Trinity-compatible databases. They are operational notes, not
port completion claims.

## Native server packet capture collection

`tools/qa-runtime.sh capture` collects packets emitted by the selected native
server around one explicit executable scenario runner. It is a collection
record, not a gameplay or lifecycle acceptance result. A zero runner exit is
recorded separately and never means that death, corpse, save, or relog behavior
passed. The captured bytes are native Rust `.bin`/`.meta` files or a native C++
`.pkt`; a bot trace is not a substitute.

The command requires `--allow-runtime-qa`, `--backend rust|cpp`, a fresh
absolute `--output-dir`, a pinned `--candidate-sha256`, and an absolute
executable `--scenario-runner`. The script creates the output directory with
mode `0700`; run it as the same account as the systemd service and use a path
without whitespace. The service must initially have no Rust packet dump
environment configured. The existing exclusive runtime lock, PID/listener
guards, executable snapshot, verified stop and restore trap cover both
backends. Existing `login`, `loot-race`, `loot-item`, and `trade` commands keep
their packet-dump refusal.

Example RustyCore collection (substitute the actual candidate SHA and the
already implemented scenario runner):

```bash
./tools/qa-runtime.sh --allow-runtime-qa \
  --backend rust \
  --world-exec "$PWD/target/debug/world-server" \
  --candidate-sha256 <candidate-sha256> \
  --output-dir /tmp/rustycore-native-death-rust-<run-id> \
  --scenario-runner /absolute/path/to/death-scenario-runner \
  capture
```

Rust adds only
`/run/systemd/system/<unit>.service.d/43-qa-capture.conf` with
`RUSTYCORE_PACKET_DUMP_DIR`, reloads systemd, and verifies the effective unit
and candidate process environment. It removes that exact run-owned file and
reloads systemd before starting the original executable.

For C++, pass `--world-exec` explicitly; the capture command does not default
to the Rust debug binary. The host reference candidate is
`/home/server/woltk-trinity-legacy/build-r6/src/server/worldserver/worldserver`
with SHA-256
`80f143841b5a7b443022480500760ce336dc83052f05ddd7e3b7be6a6605b02a`. Its
source revision is `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`; record the
derivation as `derivedR6; not clean upstream`, not as a clean upstream build.
No reference build or server restart is part of preparing this command.

```bash
./tools/qa-runtime.sh --allow-runtime-qa \
  --backend cpp \
  --world-exec /home/server/woltk-trinity-legacy/build-r6/src/server/worldserver/worldserver \
  --candidate-sha256 80f143841b5a7b443022480500760ce336dc83052f05ddd7e3b7be6a6605b02a \
  --source-path /home/server/woltk-trinity-legacy \
  --source-revision a5f8da2ebf5424bf0450ca4e08843ecbf72577bd \
  --derivation 'derivedR6; not clean upstream' \
  --output-dir /tmp/rustycore-native-death-cpp-<run-id> \
  --scenario-runner /absolute/path/to/death-scenario-runner \
  capture
```

Both backends digest the same configured worldserver file. Its host default is
`/home/server/trinity-legacy-install/etc/worldserver.conf`, selected by
`QA_CAPTURE_CPP_CONFIG`; override it with `QA_CAPTURE_WORLD_CONFIG` when the
active service uses another path. The digest
selects `DataDir`, locale, realm and listener ports, `PlayerSaveInterval`, and
the death/corpse/fall/durability settings read by the current Rust config and
TrinityCore `World.cpp`, preserving unset keys without storing config text.
For C++, `PacketLogFile` must be initially empty. The helper journals the
active config hash and owner/mode, fsyncs a private backup before editing the
existing `LogsDir` and `PacketLogFile` keys (exactly one assignment for each is
required), and restores only if the file still matches the run's installed
hash. These settings feed
`PacketLog::Initialize` and `PacketLog::LogPacket` in
`src/server/game/Server/Protocol/PacketLog.cpp`.

The scenario runner receives `RUSTYCORE_CAPTURE_DIR`,
`RUSTYCORE_CAPTURE_BACKEND`, `RUSTYCORE_CAPTURE_CANDIDATE_SHA256`, and the
stable `RUSTYCORE_CAPTURE_REPORT` path (`scenario-report.json`). The harness
stores its own `capture-report.json` in the same private directory with source
HEAD/dirty state and host architecture, original and candidate executable
paths/hashes, runner and bot paths/hashes, before/after listener PID starttime
and systemd restart count, route ports, selected-config digests, the runner
exit, and each native artifact's name, size, and SHA-256. It does not print
packet bytes, process environments, or configuration values.

`captured-restored` means collection completed and the original runtime/config
was verified after restoration. `failed-restored` records a failed runner or
missing native artifact after successful restoration. `restore-failed` holds
the normal restart when stop verification, config compare-and-swap, or restore
verification fails; use the private recovery paths in the report and do not
start over an unresolved capture config. Keep the raw output private and use
the existing capture-diff raw-pair workflow for later comparison.

## Historical local fixture observations

The following undated account/character observations are retained as debugging
context, not verified current database state. Re-read only the selected authorized
fixture before using it; do not choose or modify a character merely because it is
named here. The current implementation/acceptance checkpoint is
[STATE.md](../migration/STATE.md). No live DB inspection accompanies this documentation update.

- The real runtime config points RustyCore at the same MariaDB schemas used by
  the Trinity install: `auth`, `characters`, `world`, and `hotfixes`.
- `characters.characters.instance_id` exists in the live schema, so the Rust
  `UPD_CHARACTER_POSITION` statement can persist map instance together with
  position.
- The active local mount collection data currently exists only for
  `auth.battlenet_account_mounts.battlenetAccountId = 1`.
- Account `1#1` is linked to Battle.net account `1`.
- Character `Luqe` has Riding skill `10`; many account mounts should fail
  the C++ mount capability check for that character.
- Character `Luqedos` has Riding skill `300` and is the better local manual
  mount test candidate for account `1#1`.
- Test/bot accounts without `account.battlenet_account` links or without rows
  in `battlenet_account_mounts` should have no account mount collection by
  C++ semantics.

## What to check when position does not persist

1. Confirm the running `world-server` binary was rebuilt after the latest save
   commits. A stale release binary can make source-level fixes invisible.
2. Watch the world log for:
   - `Saving player on disconnect`
   - `Player::SaveToDB represented save committed in one CharacterDatabase transaction`
   - `Finished disconnect save`
3. A `Finished disconnect save` message marks completion of the disconnect
   routine, not successful commit. Check for `Failed to commit Player::SaveToDB
   represented transaction` or `Player::SaveToDB represented transaction COMMIT
   outcome is unknown`, keyed by character GUID. Preserve the unknown-outcome
   fence; do not retry SQL or reset dirty state blindly.
4. If `Skipping Player::SaveToDB represented save because no coherent player
   snapshot is available` appears, inspect the movement/login path before
   adding duplicate save code.

These log anchors are in `crates/wow-world/src/session/lifecycle/{logout,persistence}.rs`.
Record the installed binary identity and action-specific evidence; source inspection
and logs alone do not prove position persisted through a fresh login.

## What to check when mounts cannot be used

1. Confirm the game account is linked to a Battle.net account. Rust now logs a
   warning when account mount loading is skipped because that link is missing.
2. Confirm `battlenet_account_mounts` has rows for that Battle.net account.
   Rust logs a zero-count load when the collection table has no rows.
3. Confirm `Mount.db2` contains the source spell. Rust filters invalid account
   mount rows the same way C++ `CollectionMgr::LoadAccountMounts` does through
   DB2 mount lookup.
4. Confirm the character has enough Riding skill and is in an area where the
   chosen mount type has a valid `MountCapability` row.
5. Watch the world log for `Rejecting represented mount cast`; that line
   includes the spell id, mount type, Riding skill, map/area, water state, and
   reject reason.

## C++ references

- `CollectionMgr::LoadAccountMounts` loads mounts by Battle.net account id and
  filters them through `sDB2Manager.GetMount`.
- `CollectionMgr::AddMount` learns the mount source spell before evaluating
  player conditions.
- `Player::SaveToDB` writes the full character save, including position
  (`Entities/Player/Player.cpp:19312`); `Server/WorldSession.cpp:633,670` saves
  before removing the Player from Map. `SavePositionInDB` is a separate helper,
  not the full logout-save call chain.
