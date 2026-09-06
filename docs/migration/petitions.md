# Migration: Petitions

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Petitions/` (`PetitionMgr`, `Petition`) + `src/server/game/Handlers/PetitionsHandler.cpp`
> **Rust target crate(s):** `crates/wow-world/src/petitions/` (the in-memory store + lifecycle), `crates/wow-database/` (CHAR_INS/UPD/DEL_PETITION* prepared statements + `petition`/`petition_sign` schema migrations), `crates/wow-packet/src/packets/petitions.rs` (currently absent — opcodes are in `wow-constants` but no struct/parsers), `crates/wow-world/src/handlers/petitions.rs` (also absent — no `WorldSession::Handle*Petition` equivalents).
> **Layer:** L6 (Game systems — depends on Items L4 (charter item 5863), Guild L6, Player L4, ObjectMgr L1 for name validation, World config for cost & min signatures; depended on by Guild creation flow)
> **Status:** ❌ not started — opcodes are listed in `crates/wow-constants/src/opcodes.rs` (CMSG_PETITION_BUY 0x34c8, CMSG_PETITION_SHOW_LIST 0x34c7, CMSG_PETITION_SHOW_SIGNATURES 0x34c9, CMSG_QUERY_PETITION 0x3277, CMSG_SIGN_PETITION 0x3533, CMSG_DECLINE_PETITION 0x3534, CMSG_TURN_IN_PETITION 0x3535, CMSG_PETITION_RENAME_GUILD 0x36d1, CMSG_OFFER_PETITION 0x32fd; SMSG_PETITION_SHOW_LIST 0x26bf, SMSG_PETITION_SHOW_SIGNATURES 0x26c0, SMSG_QUERY_PETITION_RESPONSE 0x291b, SMSG_PETITION_SIGN_RESULTS 0x274c, SMSG_PETITION_RENAME_GUILD_RESPONSE 0x29fa, SMSG_TURN_IN_PETITION_RESULT 0x274e, SMSG_PETITION_ALREADY_SIGNED 0x259f, SMSG_OFFER_PETITION_ERROR 0x26b6) but no handlers, no `Petition` struct, no in-memory store, no SQL loader, no charter-item integration.
> **Audited vs C++:** ❌ not audited
> **Audited vs Rust impl:** ✅ 2026-05-01 — see §13
> **Last updated:** 2026-05-01

---

## 1. Purpose

A petition (a.k.a. "guild charter") is the in-game item that a player must purchase from a Petitioner NPC and circulate among prospective members; once it carries the configured minimum number of signatures (`CONFIG_MIN_PETITION_SIGNS`, default 4 in 3.4.3), the owner can turn it in to create the actual `Guild`. The Petitions module owns the lifecycle of that intermediate object — its persistent rows in `petition` + `petition_sign`, the in-memory store keyed by the charter item's GUID, the cluster of ~9 wire opcodes that drive the buy/show/sign/decline/rename/offer/turn-in UI, and the handoff to `GuildMgr::AddGuild` once turn-in succeeds. Note: TrinityCore 3.4.3 keeps **only the guild charter** (item id `5863`); arena-team charters (items `23560` / `23561` / `23562`) were removed when arena teams disappeared in modern expansions, but the source still mentions arena-team petitions historically.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Petitions/PetitionMgr.cpp` | 228 | `prefix` |
| `game/Petitions/PetitionMgr.h` | 89 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Petitions/PetitionMgr.h` | 89 | `PetitionTurns` enum (6 result codes), `PetitionSigns` enum (8 result codes), `Signature` typedef (`pair<accountId, ObjectGuid>`), `Petition` struct (PetitionGuid, OwnerGuid, PetitionName, Signatures vector), `PetitionMgr` singleton |
| `src/server/game/Petitions/PetitionMgr.cpp` | 228 | Anonymous-namespace `_petitionStore: unordered_map<ObjectGuid, Petition>`; `LoadPetitions` + `LoadSignatures`; `AddPetition` / `RemovePetition` / `GetPetition` / `GetPetitionByOwner` / `RemovePetitionsByOwner` / `RemoveSignaturesBySigner`; `Petition::IsPetitionSignedByAccount` / `AddSignature` / `UpdateName` / `RemoveSignatureBySigner` |
| `src/server/game/Handlers/PetitionsHandler.cpp` | 464 | All `WorldSession::Handle*Petition*` opcode handlers + reply senders: `HandlePetitionBuy`, `HandlePetitionShowSignatures`, `SendPetitionSigns`, `HandleQueryPetition`, `SendPetitionQueryOpcode`, `HandlePetitionRenameGuild`, `HandleSignPetition`, `HandleDeclinePetition` (no-op stub — client doesn't show the result), `HandleOfferPetition`, `HandleTurnInPetition`, `HandlePetitionShowList`, `SendPetitionShowList` |

Out-of-tree touchpoints:
- `src/server/game/Entities/Item/Item.cpp` — `Item::SetPetitionId(uint64)`, `Item::GetPetitionId()`, `Item::SetPetitionNumSignatures(u32)`, charter item display id `16161`.
- `src/server/game/Guilds/Guild.cpp` — `Guild::Create(player, name)` + `Guild::AddMember(trans, guid)`, called from `HandleTurnInPetition`.
- `src/server/game/Guilds/GuildMgr.cpp` — `GuildMgr::GetGuildByName(name)`, `GuildMgr::AddGuild(guild)`.
- `src/server/game/Globals/ObjectMgr.cpp` — `ObjectMgr::IsValidCharterName(name)` (charter name validator), `ObjectMgr::IsReservedName(name)`.
- `src/server/game/Cache/CharacterCache.cpp` — `sCharacterCache->GetCharacterAccountIdByGuid(guid)`, `GetCharacterTeamByGuid(guid)`.
- `src/server/game/Server/WorldSession.h` — declares the `WorldPackets::Petition::*` packet structs.
- `src/server/game/World.cpp` — `CONFIG_MIN_PETITION_SIGNS`, `CONFIG_CHARTER_COST_GUILD`, `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GUILD`.

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Petition` | struct | The in-memory record: `PetitionGuid: ObjectGuid` (the charter item GUID), `OwnerGuid: ObjectGuid` (Player), `PetitionName: string`, `Signatures: vector<Signature>` |
| `Signature` | typedef | `pair<uint32 accountId, ObjectGuid playerGuid>` |
| `SignaturesVector` | typedef | `vector<Signature>` |
| `PetitionMgr` | singleton class | Owns the `_petitionStore` map, exposes load + lifecycle |
| `PetitionTurns` | enum (u8 wire code) | `OK=0`, `ALREADY_IN_GUILD=2`, `NEED_MORE_SIGNATURES=4`, `GUILD_PERMISSIONS=11`, `GUILD_NAME_INVALID=12`, `HAS_RESTRICTION=13` — payload of SMSG_TURN_IN_PETITION_RESULT |
| `PetitionSigns` | enum (u8 wire code) | `OK=0`, `ALREADY_SIGNED=1`, `ALREADY_IN_GUILD=2`, `CANT_SIGN_OWN=3`, `NOT_SERVER=5`, `FULL=8`, `ALREADY_SIGNED_OTHER=10`, `RESTRICTED_ACCOUNT_TRIAL=11`, `HAS_RESTRICTION=13` — payload of SMSG_PETITION_SIGN_RESULTS |
| Constants | — | `GUILD_CHARTER_ITEM_ID = 5863`, `CHARTER_DISPLAY_ID = 16161`, max signatures hard-coded to 10 (`signs > 10` check in `HandleSignPetition`), client-side display also caps at 9 visible signers + owner |

WorldPackets (defined in `WorldPackets/Petition/PetitionPackets.h`, not under `game/Petitions/`):

| Packet | Direction | Fields |
|---|---|---|
| `PetitionBuy` | C→S | `Unit: ObjectGuid` (petitioner NPC), `Title: string` |
| `PetitionShowList` | C→S | `PetitionUnit: ObjectGuid` |
| `ServerPetitionShowList` | S→C | `Unit: ObjectGuid`, `Price: u32` |
| `QueryPetition` | C→S | `PetitionID: u32` (= ItemGUID low part), `ItemGUID: ObjectGuid` |
| `QueryPetitionResponse` | S→C | `PetitionID: u32`, `Allow: bool`, `Info: PetitionInfo { PetitionID, Petitioner: ObjectGuid, MinSignatures, MaxSignatures, Title, BodyText, Flags }` |
| `PetitionShowSignatures` | C→S | `Item: ObjectGuid` |
| `ServerPetitionShowSignatures` | S→C | `Item: ObjectGuid`, `Owner: ObjectGuid`, `OwnerAccountID: ObjectGuid`, `PetitionID: u32`, `Signatures: vector<{ Signer: ObjectGuid, Choice: u8 }>` |
| `SignPetition` | C→S | `PetitionGUID: ObjectGuid`, `Choice: u8` |
| `PetitionSignResults` | S→C | `Item: ObjectGuid`, `Player: ObjectGuid`, `Error: i32` (PetitionSigns code) |
| `DeclinePetition` | C→S | `PetitionGUID: ObjectGuid` |
| `OfferPetition` | C→S | `ItemGUID: ObjectGuid`, `TargetPlayer: ObjectGuid` |
| `TurnInPetition` | C→S | `Item: ObjectGuid` |
| `TurnInPetitionResult` | S→C | `Result: i32` (PetitionTurns code) |
| `PetitionRenameGuild` | C→S | `PetitionGuid: ObjectGuid`, `NewGuildName: string` |
| `PetitionRenameGuildResponse` | S→C | `PetitionGuid: ObjectGuid`, `NewGuildName: string` |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `PetitionMgr::LoadPetitions()` | Read all rows from `petition` table at startup, populate `_petitionStore` (with `isLoading=true` so no DB write-back) | `CharacterDatabase.Query`, `AddPetition` |
| `PetitionMgr::LoadSignatures()` | Read all rows from `petition_sign`, attach to existing `Petition` records | `CharacterDatabase.Query`, `Petition::AddSignature(isLoading=true)` |
| `PetitionMgr::AddPetition(petitionGuid, ownerGuid, name, isLoading)` | Insert into the in-memory map + (when not loading) `INSERT INTO petition` | prepared statement `CHAR_INS_PETITION` |
| `PetitionMgr::RemovePetition(petitionGuid)` | Erase from map + DELETE petition row + DELETE all its signatures, in a single transaction | `CHAR_DEL_PETITION_BY_GUID`, `CHAR_DEL_PETITION_SIGNATURE_BY_GUID` |
| `PetitionMgr::GetPetition(petitionGuid) -> *Petition` | Map lookup by charter-item GUID | — |
| `PetitionMgr::GetPetitionByOwner(ownerGuid) -> *Petition` | Linear scan — there is at most one petition per owner | — |
| `PetitionMgr::RemovePetitionsByOwner(ownerGuid)` | Used on character delete: erase petition + all its signatures | `CHAR_DEL_PETITION_BY_OWNER`, `CHAR_DEL_PETITION_SIGNATURE_BY_OWNER` |
| `PetitionMgr::RemoveSignaturesBySigner(signerGuid)` | Used on character delete: remove signer from every petition they signed | iterate `_petitionStore`, `Petition::RemoveSignatureBySigner`, `CHAR_DEL_ALL_PETITION_SIGNATURES` |
| `Petition::IsPetitionSignedByAccount(accountId)` | Linear scan over `Signatures` checking `signature.first == accountId` (one account = one signature, even across multiple alts) | — |
| `Petition::AddSignature(accountId, playerGuid, isLoading)` | Append to `Signatures` + (when not loading) `INSERT INTO petition_sign` | `CHAR_INS_PETITION_SIGNATURE` |
| `Petition::UpdateName(newName)` | Mutate name + `UPDATE petition SET name = ? WHERE petitionguid = ?` | `CHAR_UPD_PETITION_NAME` |
| `Petition::RemoveSignatureBySigner(playerGuid)` | Find & erase from `Signatures` (linear), notify online owner via `SendPetitionQueryOpcode` so the charter UI refreshes | `WorldSession::SendPetitionQueryOpcode` |
| `WorldSession::HandlePetitionBuy(packet)` | Validate NPC interactability, in-guild check, name uniqueness/reserved-name/validity, item-template existence, money check, inventory space; then `ModifyMoney(-cost)`, `StoreNewItem(charter)`, set `petitionId` on item, `AddPetition` to mgr, send the new item to player | `Player::GetNPCIfCanInteractWith`, `GuildMgr::GetGuildByName`, `ObjectMgr::IsReservedName`, `IsValidCharterName`, `Player::HasEnoughMoney`, `CanStoreNewItem`, `ModifyMoney`, `StoreNewItem`, `Item::SetPetitionId`, `PetitionMgr::AddPetition` |
| `WorldSession::HandlePetitionShowSignatures(packet)` | Validate charter exists, player not already in a guild → send `ServerPetitionShowSignatures` | `PetitionMgr::GetPetition`, `SendPetitionSigns` |
| `WorldSession::SendPetitionSigns(petition, sendTo)` | Build the SMSG: PetitionGuid, OwnerGuid, OwnerAccountID, PetitionID, list of `{ Signer, Choice=0 }` | `CharacterCache::GetCharacterAccountIdByGuid` |
| `WorldSession::HandleQueryPetition(packet)` / `SendPetitionQueryOpcode(petitionguid)` | Build SMSG_QUERY_PETITION_RESPONSE; `MinSignatures = MaxSignatures = CONFIG_MIN_PETITION_SIGNS` (both fields fed the same value in 3.4.3) | `World::getIntConfig(CONFIG_MIN_PETITION_SIGNS)` |
| `WorldSession::HandlePetitionRenameGuild(packet)` | Validate item ownership, petition exists, name uniqueness + validity → `Petition::UpdateName`, send `PetitionRenameGuildResponse` | `Player::GetItemByGuid`, `GuildMgr::GetGuildByName`, `ObjectMgr::IsReservedName/IsValidCharterName`, `Petition::UpdateName` |
| `WorldSession::HandleSignPetition(packet)` | Validate petition exists, signer ≠ owner, faction interaction allowed (config), signer not already in guild / invited, signature count < 10, account hasn't already signed; then `Petition::AddSignature`, update charter item's `numSignatures` field, broadcast `PetitionSignResults` to signer + online owner | `Petition::IsPetitionSignedByAccount`, `Petition::AddSignature`, `Item::SetPetitionNumSignatures`, `ObjectAccessor::FindConnectedPlayer` |
| `WorldSession::HandleDeclinePetition(packet)` | No-op stub (the client never displays the decline result, so server discards) | — |
| `WorldSession::HandleOfferPetition(packet)` | Show another player my petition: validate target online, faction policy, target not in/invited to a guild → `SendPetitionSigns(petition, target)` | `ObjectAccessor::FindConnectedPlayer` |
| `WorldSession::HandleTurnInPetition(packet)` | The big one: validate item ownership, petition existence, owner-only, not-already-in-guild, name uniqueness, signature count ≥ `CONFIG_MIN_PETITION_SIGNS`, then **destroy charter**, `new Guild`, `Guild::Create(player, name)`, `GuildMgr::AddGuild`, iterate signatures `Guild::AddMember(trans, guid)` in a transaction, `PetitionMgr::RemovePetition`, send `TurnInPetitionResult{OK}` | `Player::DestroyItem`, `Guild::Create/AddMember`, `GuildMgr::AddGuild`, `PetitionMgr::RemovePetition` |
| `WorldSession::HandlePetitionShowList(packet)` / `SendPetitionShowList(guid)` | Validate NPC interactability, send `ServerPetitionShowList` with price | `Player::GetNPCIfCanInteractWith`, `World::getIntConfig(CONFIG_CHARTER_COST_GUILD)` |

---

## 5. Module dependencies

**Depends on:**
- `Items` — the petition is a charter item (id 5863, displayId 16161); `Item::SetPetitionId(itemGuidLow)`, `Item::SetPetitionNumSignatures(count)` mutate fields stored on the item template's runtime record. Item destruction in `HandleTurnInPetition`.
- `Guilds` — `Guild::Create(player, name)`, `Guild::AddMember(trans, guid)`, `GuildMgr::GetGuildByName`, `GuildMgr::AddGuild`. The whole point of the module is to bootstrap a `Guild`.
- `Player` — `GetNPCIfCanInteractWith`, `HasEnoughMoney`, `ModifyMoney`, `CanStoreNewItem`, `StoreNewItem`, `GetItemByGuid`, `DestroyItem`, `GetGuildId`, `GetGuildIdInvited`, `GetTeam`.
- `ObjectMgr` — `GetItemTemplate(5863)`, `IsReservedName`, `IsValidCharterName`.
- `CharacterCache` — `GetCharacterAccountIdByGuid`, `GetCharacterTeamByGuid`.
- `World config` — `CONFIG_MIN_PETITION_SIGNS` (default 4), `CONFIG_CHARTER_COST_GUILD` (in copper), `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GUILD`.
- `CharacterDatabase` — five prepared statements (insert/delete/update petition rows, insert/delete signatures).

**Depended on by:**
- `Player::DeleteFromDB` — calls `PetitionMgr::RemovePetitionsByOwner` and `RemoveSignaturesBySigner` on character delete.
- `WorldSession` — owns the 9 opcode handlers.
- Nothing else: petitions are leaf functionality.

---

## 6. SQL / DB queries (if any)

Schema (3.4.3 character DB):

```sql
CREATE TABLE petition (
  ownerguid     BIGINT UNSIGNED NOT NULL,
  petitionguid  BIGINT UNSIGNED NOT NULL DEFAULT 0,
  name          VARCHAR(24) NOT NULL DEFAULT '',
  PRIMARY KEY (petitionguid),
  KEY index_ownerguid_petitionguid (ownerguid, petitionguid)
);

CREATE TABLE petition_sign (
  ownerguid       BIGINT UNSIGNED NOT NULL,
  petitionguid    BIGINT UNSIGNED NOT NULL DEFAULT 0,
  playerguid      BIGINT UNSIGNED NOT NULL DEFAULT 0,
  player_account  INT  UNSIGNED NOT NULL DEFAULT 0,
  PRIMARY KEY (petitionguid, playerguid),
  KEY ownerguid (ownerguid),
  KEY playerguid (playerguid)
);
```

Prepared statements (defined in `CharacterDatabase` PreparedStatementID enum):

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT petitionguid, ownerguid, name FROM petition` (raw) | Startup load by `PetitionMgr::LoadPetitions` | character |
| `SELECT petitionguid, player_account, playerguid FROM petition_sign` (raw) | Startup load by `PetitionMgr::LoadSignatures` | character |
| `CHAR_INS_PETITION` (`INSERT INTO petition (ownerguid, petitionguid, name) VALUES (?, ?, ?)`) | `AddPetition` (non-loading) | character |
| `CHAR_DEL_PETITION_BY_GUID` (`DELETE FROM petition WHERE petitionguid = ?`) | `RemovePetition` | character |
| `CHAR_DEL_PETITION_SIGNATURE_BY_GUID` (`DELETE FROM petition_sign WHERE petitionguid = ?`) | `RemovePetition` | character |
| `CHAR_DEL_PETITION_BY_OWNER` (`DELETE FROM petition WHERE ownerguid = ?`) | `RemovePetitionsByOwner` (character delete) | character |
| `CHAR_DEL_PETITION_SIGNATURE_BY_OWNER` (`DELETE FROM petition_sign WHERE ownerguid = ?`) | `RemovePetitionsByOwner` | character |
| `CHAR_DEL_ALL_PETITION_SIGNATURES` (`DELETE FROM petition_sign WHERE playerguid = ?`) | `RemoveSignaturesBySigner` | character |
| `CHAR_INS_PETITION_SIGNATURE` (`INSERT INTO petition_sign (ownerguid, petitionguid, playerguid, player_account) VALUES (?, ?, ?, ?)`) | `Petition::AddSignature` | character |
| `CHAR_UPD_PETITION_NAME` (`UPDATE petition SET name = ? WHERE petitionguid = ?`) | `Petition::UpdateName` | character |

No DB2/DBC stores are involved; the charter item template is in `item_template` (world DB) and the cost/min-signature thresholds are world-config values, not DB2.

---

## 7. Wire-protocol packets (if any)

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_PETITION_BUY` (0x34c8) | client → server | `WorldSession::HandlePetitionBuy` |
| `CMSG_PETITION_SHOW_LIST` (0x34c7) | client → server | `WorldSession::HandlePetitionShowList` |
| `SMSG_PETITION_SHOW_LIST` (0x26bf) | server → client | `WorldSession::SendPetitionShowList` |
| `CMSG_PETITION_SHOW_SIGNATURES` (0x34c9) | client → server | `WorldSession::HandlePetitionShowSignatures` |
| `SMSG_PETITION_SHOW_SIGNATURES` (0x26c0) | server → client | `WorldSession::SendPetitionSigns` |
| `CMSG_QUERY_PETITION` (0x3277) | client → server | `WorldSession::HandleQueryPetition` |
| `SMSG_QUERY_PETITION_RESPONSE` (0x291b) | server → client | `WorldSession::SendPetitionQueryOpcode` |
| `CMSG_SIGN_PETITION` (0x3533) | client → server | `WorldSession::HandleSignPetition` |
| `SMSG_PETITION_SIGN_RESULTS` (0x274c) | server → client | `WorldSession::HandleSignPetition` (broadcast to signer + owner) |
| `SMSG_PETITION_ALREADY_SIGNED` (0x259f) | server → client | (signer-side notice; bound to `PETITION_SIGN_ALREADY_SIGNED` result code) |
| `CMSG_DECLINE_PETITION` (0x3534) | client → server | `WorldSession::HandleDeclinePetition` (no-op stub) |
| `CMSG_OFFER_PETITION` (0x32fd) | client → server | `WorldSession::HandleOfferPetition` |
| `SMSG_OFFER_PETITION_ERROR` (0x26b6) | server → client | (sent on offer-side validation failure) |
| `CMSG_TURN_IN_PETITION` (0x3535) | client → server | `WorldSession::HandleTurnInPetition` |
| `SMSG_TURN_IN_PETITION_RESULT` (0x274e) | server → client | `WorldSession::HandleTurnInPetition` |
| `CMSG_PETITION_RENAME_GUILD` (0x36d1) | client → server | `WorldSession::HandlePetitionRenameGuild` |
| `SMSG_PETITION_RENAME_GUILD_RESPONSE` (0x29fa) | server → client | `WorldSession::HandlePetitionRenameGuild` |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-world/src/petitions` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-packet/src/packets/petitions.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/wow-world/src/handlers/petitions.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-world/src/handlers` | `module_dir` | 14 | 8843 | `exists_active` | directory exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs` — all 18 opcodes listed above are present, no logic.
- No `crates/wow-world/src/petitions/`, no `crates/wow-packet/src/packets/petitions.rs`, no handlers in `crates/wow-world/src/handlers/`, no schema migration in `crates/wow-database/`.

**What's implemented:**
- Just opcode constants.

**What's missing vs C++:**
- Everything: `Petition` struct, `PetitionMgr` store, schema for `petition` + `petition_sign`, all 10 prepared statements, all 9 packet structs (request + response), all 9 handlers, the SendPetitionSigns/SendPetitionQueryOpcode/SendPetitionShowList helpers, integration with `Item` (set petitionId / numSignatures), integration with `GuildMgr::Create/AddGuild` and `Guild::AddMember`, character-delete cleanup hooks (`RemovePetitionsByOwner`, `RemoveSignaturesBySigner`), config keys (`CONFIG_MIN_PETITION_SIGNS`, `CONFIG_CHARTER_COST_GUILD`, `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GUILD`).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- A client that opens the petition UI today receives no response — depending on how the client handles missing `SMSG_QUERY_PETITION_RESPONSE`, it may just hang the dialog forever.
- Implementing this requires the Guild module to already expose `Guild::Create(player, name)` and `Guild::AddMember(transaction, playerGuid)` — verify that the Guild migration doc reflects this dependency.

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#PETITIONS.WBS.001** Cerrar la migracion auditada de `game/Petitions/PetitionMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`
  Rust target: `crates/wow-database`, `crates/wow-constants`, `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PETITIONS.WBS.002** Cerrar la migracion auditada de `game/Petitions/PetitionMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h`
  Rust target: `crates/wow-database`, `crates/wow-constants`, `crates/wow-world/src/handlers`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

- [ ] **#PET.1** Define `PetitionTurns` (u8) and `PetitionSigns` (u8) enums in `crates/wow-constants/src/petitions.rs` with the exact wire codes (L)
- [ ] **#PET.2** Define `Petition` struct in `crates/wow-world/src/petitions/petition.rs` with `petition_guid`, `owner_guid`, `name`, `signatures: Vec<(u32 accountId, ObjectGuid playerGuid)>` (L)
- [ ] **#PET.3** Add SQL migrations for `petition` + `petition_sign` tables in `crates/wow-database/migrations/character/` (L)
- [ ] **#PET.4** Add prepared-statement enum entries `CHAR_INS_PETITION`, `CHAR_DEL_PETITION_BY_GUID`, `CHAR_DEL_PETITION_SIGNATURE_BY_GUID`, `CHAR_DEL_PETITION_BY_OWNER`, `CHAR_DEL_PETITION_SIGNATURE_BY_OWNER`, `CHAR_DEL_ALL_PETITION_SIGNATURES`, `CHAR_INS_PETITION_SIGNATURE`, `CHAR_UPD_PETITION_NAME` in `crates/wow-database/src/statements.rs` (M)
- [ ] **#PET.5** Implement `PetitionMgr::load_petitions` (raw `SELECT petitionguid, ownerguid, name FROM petition`) populating `_petitionStore: HashMap<ObjectGuid, Petition>` (M)
- [ ] **#PET.6** Implement `PetitionMgr::load_signatures` (raw `SELECT petitionguid, player_account, playerguid FROM petition_sign`) attaching to existing entries (M)
- [ ] **#PET.7** Implement `PetitionMgr::add_petition`, `remove_petition`, `get_petition`, `get_petition_by_owner`, `remove_petitions_by_owner`, `remove_signatures_by_signer` with their SQL side-effects (H)
- [ ] **#PET.8** Implement `Petition::is_petition_signed_by_account`, `add_signature` (with optional DB write), `update_name`, `remove_signature_by_signer` (M)
- [ ] **#PET.9** Define wire packet structs in `crates/wow-packet/src/packets/petitions.rs`: `PetitionBuy`, `PetitionShowList`, `ServerPetitionShowList`, `QueryPetition`, `QueryPetitionResponse` (with `PetitionInfo` sub-struct), `PetitionShowSignatures`, `ServerPetitionShowSignatures`, `SignPetition`, `PetitionSignResults`, `DeclinePetition`, `OfferPetition`, `TurnInPetition`, `TurnInPetitionResult`, `PetitionRenameGuild`, `PetitionRenameGuildResponse` (XL — split as: client packets, server packets)
- [ ] **#PET.10** Implement `WorldSession::handle_petition_buy` — NPC interactability (`UNIT_NPC_FLAG_PETITIONER`), feign-death removal, in-guild check, name uniqueness via `GuildMgr::get_guild_by_name`, reserved-name + valid-charter-name checks, item template lookup (5863), money check, inventory-space check, charge money, store new item, `SetPetitionId(item.guid.low())`, `PetitionMgr::add_petition` (H)
- [ ] **#PET.11** Implement `handle_petition_show_list` + `send_petition_show_list` (M)
- [ ] **#PET.12** Implement `handle_petition_show_signatures` + `send_petition_signs` (M)
- [ ] **#PET.13** Implement `handle_query_petition` + `send_petition_query_opcode` — `MinSignatures = MaxSignatures = CONFIG_MIN_PETITION_SIGNS` (M)
- [ ] **#PET.14** Implement `handle_petition_rename_guild` + `PetitionRenameGuildResponse` (M)
- [ ] **#PET.15** Implement `handle_sign_petition` — all the validation cascade (signer ≠ owner, faction policy, not in/invited to guild, max 10 signatures, account-uniqueness check), then write the signature + update item's `numSignatures` field + broadcast `PetitionSignResults` to signer and (if online) owner (H)
- [ ] **#PET.16** Implement `handle_decline_petition` as a no-op stub matching C++ (with the same comment explaining why) (L)
- [ ] **#PET.17** Implement `handle_offer_petition` — target online, faction policy, target not in/invited to guild, then `send_petition_signs(petition, target)` (M)
- [ ] **#PET.18** Implement `handle_turn_in_petition` — owner-only, signature count check, name uniqueness re-check (concurrent guild-create race), destroy charter item, `Guild::create(player, name)`, `GuildMgr::add_guild`, transactional `Guild::add_member(trans, signature.player_guid)` per signature, `PetitionMgr::remove_petition`, send `TurnInPetitionResult{OK}` (XL — depends on Guild module exposing these methods)
- [ ] **#PET.19** Wire `PetitionMgr::remove_petitions_by_owner` + `remove_signatures_by_signer` into `Player::delete_from_db` flow (M)
- [ ] **#PET.20** Add config keys `CONFIG_MIN_PETITION_SIGNS` (default 4), `CONFIG_CHARTER_COST_GUILD` (default 1000 copper, i.e. 10 silver), `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GUILD` (default false in 3.4.3) — verify defaults against `worldserver.conf.dist` (L)
- [ ] **#PET.21** Add NPC flag `UNIT_NPC_FLAG_PETITIONER` to `crates/wow-constants` if not already present (L)
- [ ] **#PET.22** Add `Item::set_petition_id` / `get_petition_id` / `set_petition_num_signatures` accessors on the Item entity (M, depends on Item entity having a place for those fields — usually stored as `ItemFields::ITEM_FIELD_ENCHANTMENT[ENCHANTMENT_SLOT_PETITION_ID]` or a dedicated property in 3.4.3) (M)
- [ ] **#PET.23** Documentation cross-link: `petitions.md` ↔ `guilds.md` (turn-in dependency) ↔ `inventory.md` (charter item destruction) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#PETITIONS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 317 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-database`. | `cargo test -p wow-constants && cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#PETITIONS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 317 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#PETITIONS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 317 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#PETITIONS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 317 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h`. Rust target: `crates/wow-constants`, `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `PetitionMgr::add_petition` with `is_loading=false` writes a row to `petition`; with `is_loading=true` does not.
- [ ] Test: `PetitionMgr::remove_petition` deletes both the `petition` row and all matching `petition_sign` rows in a single transaction.
- [ ] Test: `Petition::add_signature` enforces account-uniqueness via `is_petition_signed_by_account` (alts on the same account cannot sign twice).
- [ ] Test: signature count cap of 10 — adding an 11th returns without writing.
- [ ] Test: `HandlePetitionBuy` rejects when player already has a guild → no item created, no money deducted.
- [ ] Test: `HandlePetitionBuy` rejects on duplicate guild name.
- [ ] Test: `HandlePetitionBuy` deducts `CONFIG_CHARTER_COST_GUILD` and stores a charter item with the right item id (5863) and a non-zero petitionId.
- [ ] Test: `HandleSignPetition` rejects when signer == owner.
- [ ] Test: `HandleSignPetition` rejects cross-faction sign when `CONFIG_ALLOW_TWO_SIDE_INTERACTION_GUILD = false`.
- [ ] Test: `HandleSignPetition` happy path → signature appended, charter item's `numSignatures` updated, `SMSG_PETITION_SIGN_RESULTS{OK}` sent to signer and online owner.
- [ ] Test: `HandleTurnInPetition` rejects with `NEED_MORE_SIGNATURES` when count < `CONFIG_MIN_PETITION_SIGNS`.
- [ ] Test: `HandleTurnInPetition` rejects with `ALREADY_IN_GUILD` if owner is already in a guild between buy and turn-in.
- [ ] Test: `HandleTurnInPetition` happy path → guild created, all signers added as members, charter destroyed, petition row + signatures deleted.
- [ ] Test: `RemovePetitionsByOwner` on character delete cleans up the petition row and all signatures.
- [ ] Test: `RemoveSignaturesBySigner` on character delete cleans up that player's signatures everywhere AND notifies online owners (via `SendPetitionQueryOpcode`).
- [ ] Test: SMSG_PETITION_SHOW_SIGNATURES wire format matches a known-good byte sequence given a fixture (owner, item, 3 signers).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 317 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h` | `crates/wow-world/src/petitions/` (the in-memory store + lifecycle), `crates/wow-database/` (CHAR_INS/UPD/DEL_PETITION* prepared statements + `petition`/`petition_sign` schema migrations), `crates/wow-packet/src/packets/petitions.rs` (currently absent — opcodes are in `wow-constants` but no struct/parsers), `crates/wow-world/src/handlers/petitions.rs` (also absent — no `WorldSession::Handle*Petition` equivalents). \| ❌ not started — opcodes are listed in `crates/wow-constants/src/opcodes.rs` (CMSG_PETITION_BUY 0x34c8, CMSG_PETITION_SHOW_LIST 0x34c7, CMSG_PETITION_SHOW_SIGNATURES 0x34c9, CMSG_QUERY_PETITION 0x3277, CMSG_SIGN_PETITION 0x3533, CMSG_DECLINE_PETITION 0x3534, CMSG_TURN_IN_PETITION 0x3535, CMSG_PETITION_RENAME_GUILD 0x36d1, CMSG_OFFER_PETITION 0x32fd; SMSG_PETITION_SHOW_LIST 0x26bf, SMSG_PETITION_SHOW_SIGNATURES 0x26c0, SMSG_QUERY_PETITION_RESPONSE 0x291b, SMSG_PETITION_SIGN_RESULTS 0x274c, SMSG_PETITION_RENAME_GUILD_RESPONSE 0x29fa, SMSG_TURN_IN_PETITION_RESULT 0x274e, SMSG_PETITION_ALREADY_SIGNED 0x259f, SMSG_OFFER_PETITION_ERROR 0x26b6) but no handlers, no `Petition` struct, no in-memory store, no SQL loader, no charter-item integration. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#PETITIONS.DIV.001` | `crates/wow-world/src/petitions` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 317 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#PETITIONS.DIV.002` | `crates/wow-packet/src/packets/petitions.rs` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 317 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |
| `#PETITIONS.DIV.003` | `crates/wow-world/src/handlers/petitions.rs` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 317 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Petitions/PetitionMgr.h` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- The "PetitionID" in `SMSG_QUERY_PETITION_RESPONSE` is the **low part of the petition GUID** (`uint32(petitionguid.GetCounter())`) — the client treats it as a `u32` even though server-side the canonical key is a 64-bit `ObjectGuid`. Don't be tempted to use the high part.
- The `PetitionGuid` is the **charter item's** GUID. There is no separate "petition GUID" — when the player loses or destroys the charter item, the petition is orphaned (and `RemovePetition` is the only path to clean it up).
- C++ uses `ObjectGuid::Create<HighGuid::Item>(uint64)` to deserialize from DB — in Rust, mirror this so `petition.petitionguid` produces an Item-typed ObjectGuid, not Player.
- `RemovePetitionsByOwner` only deletes one petition (the inner `for` loop has a `break`) because the C++ assumes at most one petition per owner. Preserve that invariant — `HandlePetitionBuy` enforces it by calling `GetPetitionByOwner` and removing any pre-existing one before the new charter is stored.
- `HandleDeclinePetition` is intentionally empty: the client sends the opcode but doesn't act on a server reply, so TC discards. The commented-out alternate implementation (sending `PetitionDeclined` to owner) is dead code. Mirror the empty handler exactly.
- The `Choice` field in `ServerPetitionShowSignatures::PetitionSignature` is always `0` server-side. It exists in the wire format but TC never populates it.
- Charter item display id is `16161` (not the icon — the actual UI texture). Item id `5863`. These constants live in `PetitionsHandler.cpp` (`CHARTER_DISPLAY_ID`, `GUILD_CHARTER_ITEM_ID`) — port them as named constants, do not hard-code.
- 3.4.3 had arena-team petitions (items 23560/23561/23562 for 2v2/3v3/5v5). The legacy code still has vestiges (e.g. comments) but the `HandlePetitionBuy` has *only* the guild path. If the project ever decides to bring back arena teams, the petition module will need to handle a `chartertype` enum and dispatch.
- Both `MinSignatures` and `MaxSignatures` in the query response are filled with the same `CONFIG_MIN_PETITION_SIGNS` value (default 4). Modern expansions removed signatures entirely — for 3.4.3 we keep the requirement.
- The signature persistence schema does **not** store `Choice` — it's a wire-only field.
- `Petition::RemoveSignatureBySigner` notifies the owner via `SendPetitionQueryOpcode` — this is "side-effecty" for what looks like a data structure method. The Rust port can choose to keep that side effect inside the manager method OR factor it out into the caller; either is fine, but be consistent with the C++ to avoid missing notifications on character delete.
- `IsValidCharterName` is a separate validator from `IsValidGuildName` — charter names are stricter (no embedded apostrophes, no leading whitespace). Replicate the C++ char-by-char rules exactly.
- The `CHAR_INS_PETITION_SIGNATURE` statement parameter order is `(ownerguid, petitionguid, playerguid, account)`. Off-by-one here corrupts the table.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `struct Petition` | `pub struct Petition { pub petition_guid: ObjectGuid, pub owner_guid: ObjectGuid, pub name: String, pub signatures: Vec<Signature> }` | Plain owned struct |
| `typedef pair<u32, ObjectGuid> Signature` | `pub struct Signature { pub account_id: u32, pub player_guid: ObjectGuid }` | Named fields beat tuples here |
| `class PetitionMgr` (singleton) | `pub struct PetitionMgr { store: HashMap<ObjectGuid, Petition> }` + `static PETITION_MGR: OnceCell<RwLock<PetitionMgr>>` | `RwLock` because handlers run on per-session tasks; mostly read |
| `unordered_map<ObjectGuid, Petition>` | `HashMap<ObjectGuid, Petition>` | Same complexity profile |
| `Petition* PetitionMgr::GetPetition(...)` | `fn get_petition(&self, guid: ObjectGuid) -> Option<&Petition>` | Borrow |
| `Petition* PetitionMgr::GetPetitionByOwner(...)` | `fn get_petition_by_owner(&self, owner: ObjectGuid) -> Option<&Petition>` | Linear; that's fine — at most one per owner |
| `void PetitionMgr::AddPetition(..., bool isLoading)` | Two methods: `fn load_petition(...)` (no SQL) and `fn create_petition(...)` (with SQL). The `is_loading` bool flag is a smell — split it. | — |
| `enum PetitionTurns / PetitionSigns` | `#[repr(u8)] pub enum PetitionTurns / PetitionSigns` | Wire-stable codes |
| `WorldPackets::Petition::*` | `crates/wow-packet/src/packets/petitions.rs` structs implementing `ServerPacket` / parser fns for client packets | Match the existing pattern in other `packets/*.rs` files |
| `WorldSession::Handle*Petition*` | `pub async fn handle_*(session: &mut WorldSession, packet: ...) -> Result<()>` in `crates/wow-world/src/handlers/petitions.rs` | Same pattern as other handlers |
| `CharacterDatabasePreparedStatement* CHAR_INS_PETITION` | `crate::statements::CharStatement::InsPetition` + `bind` | sqlx prepared statement |
| `CharacterDatabaseTransaction trans` | `let mut tx = self.char_db.begin().await?` … `tx.commit().await?` | Match existing transactional pattern |
| `CHARTER_DISPLAY_ID = 16161`, `GUILD_CHARTER_ITEM_ID = 5863` | `pub const CHARTER_DISPLAY_ID: u32 = 16161;`, `pub const GUILD_CHARTER_ITEM_ID: u32 = 5863;` | In `crates/wow-constants/src/petitions.rs` |
| `Item::SetPetitionId(uint64)` / `SetPetitionNumSignatures(u32)` | `Item::set_petition_id` / `set_petition_num_signatures` | These mutate item fields stored in the EnchantmentSlot at `ENCHANTMENT_SLOT_PETITION_ID` in 3.4.3 — verify against the inventory crate |

---

## 13. Audit (2026-05-01)

| Claim | Verified | Evidence |
|---|---|---|
| 0 lines petition Rust impl | ✅ | `grep -rn "Petition" crates/wow-world crates/wow-handler → 0` (NO `Petition` struct, NO `PetitionMgr`, NO `petitions/` module, NO handler file) |
| Doc claim "all 18 opcodes registered" | ❌ off-by-one | Actual count is **17**: 9 client (`PetitionBuy`, `PetitionShowList`, `PetitionShowSignatures`, `QueryPetition`, `SignPetition`, `DeclinePetition`, `TurnInPetition`, `PetitionRenameGuild`, `OfferPetition`) + 8 server (`PetitionShowList`, `PetitionShowSignatures`, `QueryPetitionResponse`, `PetitionSignResults`, `PetitionRenameGuildResponse`, `TurnInPetitionResult`, `PetitionAlreadySigned`, `OfferPetitionError`) in `crates/wow-constants/src/opcodes.rs`. Doc lists 18 in YAML but enumerates the same 9+8=17. Off-by-one wording. |
| 0 dispatch arms in `session.rs` | ✅ | `grep -E "Petition" crates/wow-world/src/session.rs → 0` |
| 0 `inventory::submit!` registrations | ✅ | `grep -rn "PetitionBuy\|SignPetition\|TurnInPetition\|QueryPetition\|OfferPetition\|DeclinePetition\|PetitionShowList\|PetitionShowSignatures\|PetitionRenameGuild" crates/wow-world/src → 0` |
| `UNIT_NPC_FLAG_PETITIONER` defined | ✅ | `crates/wow-constants/src/unit.rs:333` `const PETITIONER = 0x40000` |
| Item flag `PETITION = 0x2000` defined | ✅ | `crates/wow-constants/src/item.rs:898` |
| No `petition`/`petition_sign` schema | ✅ | grep in `crates/wow-database` → 0 |

**Silent-hang risk:** REAL but different from doc's framing. Because no opcodes are even dispatched (no match arm), the petition opcodes hit the unhandled-opcode default path. Client buys a charter (no `CHARTER_DISPLAY_ID` reservation, no money charge, no `Petition` row), then opens the petition UI → no `SMSG_QUERY_PETITION_RESPONSE` → dialog hangs. Doc's "18 opcodes registered but no handlers" was wrong: it's "17 opcodes constants only, ZERO dispatched, ZERO handlers" — even worse than register-without-handler.

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.
