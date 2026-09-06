# Live client debug notes

These notes are for validating the current Rust runtime against the local WotLK
Classic client and Trinity-compatible databases. They are operational notes, not
port completion claims.

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
