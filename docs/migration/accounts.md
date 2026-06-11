# Migration: Accounts (AccountMgr / RBAC / BattlenetAccountMgr)

> **C++ canonical path:** `src/server/game/Accounts/`
> **Rust target crate(s):** `crates/wow-database/`, `crates/wow-network/`, `crates/bnet-server/`, `crates/wow-world/` (consumer)
> **Layer:** L1 — Account management & authorization
> **Status:** ❌ not started — confirmed via audit 2026-05-01 (only DB statement strings; no AccountMgr / RBACData logic; no `Utf8ToUpperOnlyLatin`; ~440 RBAC perms unrepresented)
> **Audited vs C++:** ✅ audited 2026-05-01 — every flagged absence reconfirmed; `RBAC_PERM_*` enum is genuinely unrepresented; `Utf8ToUpperOnlyLatin` has zero callers
> **Last updated:** 2026-05-01

---

## 1. Purpose

Owns the lifecycle of game-side **accounts** (`account` table) and **Battle.net accounts** (`battlenet_accounts` table) plus the **Role-Based Access Control** (RBAC) layer that decides which permissions an account has on which realm. Used by GM commands (`.account create`, `.account password`, `.account delete`), the BNet REST login flow (creating bot/test accounts), and every gameplay check that has to ask "is this account allowed to do X?" (e.g. `RBAC_PERM_JOIN_ARENAS`, `RBAC_PERM_SKIP_CHECK_CHAT_SPAM`).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Accounts/AccountMgr.cpp` | 607 | `prefix` |
| `game/Accounts/AccountMgr.h` | 99 | `prefix` |
| `game/Accounts/BattlenetAccountMgr.cpp` | 215 | `prefix` |
| `game/Accounts/BattlenetAccountMgr.h` | 50 | `prefix` |
| `game/Accounts/RBAC.cpp` | 282 | `prefix` |
| `game/Accounts/RBAC.h` | 1018 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Accounts/AccountMgr.h` | 99 | `AccountMgr` singleton interface; `AccountOpResult` enum; size limits |
| `src/server/game/Accounts/AccountMgr.cpp` | 607 | CRUD + SRP6 password hashing + RBAC permission table loader |
| `src/server/game/Accounts/RBAC.h` | 1018 | `RBACPermissions` enum (~440 perms), `RBACPermission`, `RBACData`, command-result enum |
| `src/server/game/Accounts/RBAC.cpp` | 282 | `RBACData` grant/deny/revoke logic; expand linked permissions; persist to DB |
| `src/server/game/Accounts/BattlenetAccountMgr.h` | 50 | `Battlenet::AccountMgr` namespace API (BNet email-keyed accounts) |
| `src/server/game/Accounts/BattlenetAccountMgr.cpp` | 215 | BNet SRPv1/v2 registration, link/unlink to game accounts |

Related (used by, lives elsewhere): `src/common/Cryptography/SRP6.h` (`Trinity::Crypto::SRP::GruntSRP6`, `BnetSRP6v1`, `BnetSRP6v2`); `src/server/database/Database/Implementation/LoginDatabase.cpp` (prepared statement registry).

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `AccountMgr` | class (singleton, `sAccountMgr`) | Game-account CRUD + RBAC permission cache |
| `AccountOpResult` | enum class : uint8 | `AOR_OK`, `AOR_NAME_TOO_LONG`, `AOR_PASS_TOO_LONG`, `AOR_EMAIL_TOO_LONG`, `AOR_NAME_ALREADY_EXIST`, `AOR_NAME_NOT_EXIST`, `AOR_DB_INTERNAL_ERROR`, `AOR_ACCOUNT_BAD_LINK` |
| `PasswordChangeSecurity` | enum | `PW_NONE`, `PW_EMAIL`, `PW_RBAC` (mode for `.account password`) |
| `MAX_ACCOUNT_STR / MAX_PASS_STR / MAX_EMAIL_STR` | macro | `16 / 16 / 64` (game accounts) |
| `MAX_BNET_EMAIL_STR / MAX_BNET_PASS_STR` | macro | `320 / 128` (BNet accounts; v2 supports longer) |
| `Battlenet::AccountMgr` | namespace | Free-function API on BNet accounts (no class state) |
| `SrpVersion` | enum class : int8 | `v1 = 1` (SHA256, case-insensitive, ≤16 chars), `v2 = 2` (PBKDF2-SHA512, case-sensitive, ≤128) |
| `rbac::RBACPermissions` | enum (uint32) | ~440 entries: 1–199 core perms/roles, 200+ command perms |
| `rbac::RBACPermission` | class | A permission record: id + name + linked permission ids |
| `rbac::RBACData` | class | Per-account computed permission set (granted ∪ linked − denied) |
| `rbac::RBACCommandResult` | enum | `RBAC_OK`, `RBAC_CANT_ADD_ALREADY_ADDED`, `RBAC_CANT_REVOKE_NOT_IN_LIST`, `RBAC_IN_GRANTED_LIST`, `RBAC_IN_DENIED_LIST`, `RBAC_ID_DOES_NOT_EXISTS` |
| `RBACPermissionContainer` | typedef `std::set<uint32>` | Set of permission ids |
| `RBACPermissionsContainer` | typedef `std::map<uint32, RBACPermission*>` | Global perm catalogue |
| `RBACDefaultPermissionsContainer` | typedef `std::map<uint8, RBACPermissionContainer>` | secLevel → default perms |

---

## 4. Critical public methods / functions

### `AccountMgr` (game-account CRUD)

| Symbol | Purpose | Calls into |
|---|---|---|
| `AccountMgr::CreateAccount(username, password, email, bnetAccountId, bnetIndex)` | Insert row in `account`. Uppercases name/pass via `Utf8ToUpperOnlyLatin`. Generates SRP6 salt+verifier with `GruntSRP6`. Initializes realm character counts. | `LoginDatabase`, `Trinity::Crypto::SRP6::MakeRegistrationData<GruntSRP6>` |
| `AccountMgr::DeleteAccount(accountId)` | Kicks online player, deletes characters via `Player::DeleteFromDB`, wipes tutorials/account_data/bans/access in a single login transaction | `Player`, `LoginDatabase`, `CharacterDatabase` |
| `AccountMgr::ChangeUsername(accountId, newU, newP)` | Updates `username` + regenerates salt/verifier (verifier depends on username) | SRP6 |
| `AccountMgr::ChangePassword(accountId, newP)` | Regenerates salt/verifier; calls `sScriptMgr->OnPasswordChange` / `OnFailedPasswordChange` | SRP6, ScriptMgr |
| `AccountMgr::ChangeEmail / ChangeRegEmail` | Update mutable email / immutable registration email | LoginDatabase |
| `AccountMgr::CheckPassword(username, password)` | Loads salt+verifier, calls `AccountSRP6(...).CheckCredentials` | SRP6 |
| `AccountMgr::GetId(username)` / `GetName(id, &name)` / `GetEmail` | Lookup helpers | LoginDatabase |
| `AccountMgr::GetSecurity(accountId, realmId)` | Returns gmlevel for a (account, realm) tuple, default `SEC_PLAYER` | LoginDatabase |
| `AccountMgr::GetSecurityAsync(...)` | Async variant; returns `QueryCallback` |  AsyncQuery |
| `AccountMgr::GetCharactersCount(accountId)` | Sum of characters across realms (`CHAR_SEL_SUM_CHARS`) | CharacterDatabase |
| `AccountMgr::IsBannedAccount(name)` | Boolean ban check by name | LoginDatabase |
| `AccountMgr::IsPlayerAccount/IsAdminAccount/IsConsoleAccount(gmlevel)` | gmlevel classifiers | (pure logic) |
| `AccountMgr::HasPermission(accountId, permissionId, realmId)` | Builds an `RBACData`, calls `LoadFromDB`, queries `HasPermission` | RBACData |
| `AccountMgr::UpdateAccountAccess(rbac, accountId, secLevel, realmId)` | Updates `account_access` table; revokes old level | LoginDatabase |
| `AccountMgr::LoadRBAC()` | Loads `rbac_permissions`, `rbac_linked_permissions`, `rbac_default_permissions` filtered by current realm into in-memory caches | LoginDatabase |

### `Battlenet::AccountMgr` (BNet account CRUD)

| Symbol | Purpose |
|---|---|
| `CreateBattlenetAccount(email, password, withGameAccount, &gameAccountName)` | Insert BNet account with v2 SRP; if `withGameAccount`, also creates a game account named `<bnetId>#1` |
| `ChangePassword(accountId, newP)` | Regenerate SRPv2 salt/verifier |
| `CheckPassword(accountId, password)` | Picks SRP v1 or v2 from DB column; v1 uppercases password |
| `LinkWithGameAccount(email, gameAccountName)` / `UnlinkGameAccount(name)` | Manage `account.battlenet_account` + `account.battlenet_index` |
| `GetId(email)` / `GetName(id)` / `GetIdByGameAccount(gameAccountId)` / `GetMaxIndex(bnetId)` | Lookups |
| `GetSrpUsername(name)` | Returns hex-encoded SHA256 of uppercased email (BNet SRP uses *that* as username, never the email itself) |

### `rbac::RBACData` (per-account permission state)

| Symbol | Purpose |
|---|---|
| `GrantPermission(permId, realmId)` / `DenyPermission(...)` / `RevokePermission(...)` | Mutate granted/denied sets; persist if `realmId != 0` |
| `LoadFromDB() / LoadFromDBAsync() / LoadFromDBCallback(result)` | Load `rbac_account_permissions` + add `rbac_default_permissions[secLevel]`, then `CalculateNewPermissions()` |
| `HasPermission(permId)` | Tests against the calculated `_globalPerms` set |
| `SetSecurityLevel(id)` | Updates `_secLevel` and reloads from DB (default permissions change) |
| `CalculateNewPermissions()` | `_globalPerms = expand(granted) − expand(denied)` |
| `ExpandPermissions(set&)` | BFS over `RBACPermission::GetLinkedPermissions()` |

---

## 5. Module dependencies

**Depends on:**
- `Cryptography/SRP6.h` — `Trinity::Crypto::SRP::GruntSRP6`, `BnetSRP6v1`, `BnetSRP6v2` for password verification
- `LoginDatabase` (and prepared statement enums in `LoginDatabase.cpp`)
- `CharacterDatabase` (delete characters / count characters)
- `ScriptMgr` — fires `OnPasswordChange`, `OnEmailChange`, `OnFailedPasswordChange`, `OnFailedEmailChange` hooks
- `World` — `realm.Id.Realm` to scope `LoadRBAC` to current realm
- `Player`, `ObjectAccessor`, `WorldSession` — for forced kick on delete
- `Util.h` — `Utf8ToUpperOnlyLatin`, `utf8length`

**Depended on by:**
- `WorldSession` — `RBACData* GetRBACData()` (every permission check uses this)
- GM command handlers in `cs_account*.cpp`, `cs_rbac.cpp`, `cs_ban.cpp` (most `.account *` commands)
- `Player::CanJoinArena / CanFilterWhispers / etc.` — gameplay perms
- `WorldSocket::HandleAuthSession` — security level loaded into session at login
- `LoginRESTService` (bnetserver) — registration flow calls `Battlenet::AccountMgr::CreateBattlenetAccount`

---

## 6. SQL / DB queries (if any)

Statements registered in `LoginDatabase.cpp` and `CharacterDatabase.cpp`. All against `auth` DB unless noted.

| Statement | Purpose | DB |
|---|---|---|
| `LOGIN_INS_ACCOUNT` | Insert game account row | auth |
| `LOGIN_INS_REALM_CHARACTERS_INIT` | Seed `realmcharacters` with 0 chars per realm | auth |
| `LOGIN_DEL_ACCOUNT` / `_DEL_ACCOUNT_ACCESS` / `_DEL_REALM_CHARACTERS` / `_DEL_ACCOUNT_BANNED` / `_DEL_ACCOUNT_MUTED` | Cascade delete on `DeleteAccount` | auth |
| `LOGIN_SEL_ACCOUNT_BY_ID` | Existence check | auth |
| `LOGIN_UPD_USERNAME` / `LOGIN_UPD_LOGON` (salt, verifier) | Rename / regenerate verifier | auth |
| `LOGIN_UPD_EMAIL` / `LOGIN_UPD_REG_EMAIL` | Change email | auth |
| `LOGIN_GET_ACCOUNT_ID_BY_USERNAME` / `LOGIN_GET_USERNAME_BY_ID` / `LOGIN_GET_EMAIL_BY_ID` | Name/email lookups | auth |
| `LOGIN_GET_GMLEVEL_BY_REALMID` | `account_access` join | auth |
| `LOGIN_SEL_CHECK_PASSWORD_BY_NAME` / `LOGIN_SEL_CHECK_PASSWORD` | Load salt+verifier for SRP6 verification | auth |
| `LOGIN_SEL_ACCOUNT_BANNED_BY_USERNAME` | Banned check | auth |
| `LOGIN_INS_ACCOUNT_ACCESS` / `LOGIN_DEL_ACCOUNT_ACCESS_BY_REALM` | Set/clear gmlevel per realm | auth |
| `LOGIN_INS_BNET_ACCOUNT` / `LOGIN_UPD_BNET_LOGON` / `LOGIN_SEL_BNET_CHECK_PASSWORD` | BNet account create / change password / verify | auth |
| `LOGIN_SEL_BNET_ACCOUNT_ID_BY_EMAIL` / `LOGIN_SEL_BNET_ACCOUNT_EMAIL_BY_ID` / `LOGIN_SEL_BNET_ACCOUNT_ID_BY_GAME_ACCOUNT` / `LOGIN_SEL_BNET_MAX_ACCOUNT_INDEX` | Lookups | auth |
| `LOGIN_UPD_BNET_GAME_ACCOUNT_LINK` | Link / unlink (set NULL) game account ↔ bnet | auth |
| `CHAR_SEL_CHARS_BY_ACCOUNT_ID` / `CHAR_SEL_SUM_CHARS` | Character enumeration / count | characters |
| `CHAR_DEL_TUTORIALS` / `CHAR_DEL_ACCOUNT_DATA` / `CHAR_DEL_CHARACTER_BAN` | Cleanup on `DeleteAccount` | characters |
| Plain `SELECT id, name FROM rbac_permissions` | Bootstrap perm catalogue | auth |
| Plain `SELECT id, linkedId FROM rbac_linked_permissions ORDER BY id ASC` | Linked perms | auth |
| Plain `SELECT secId, permissionId FROM rbac_default_permissions WHERE realmId = X OR realmId = -1` | Defaults per security level for current realm | auth |
| `LOGIN_INS_RBAC_ACCOUNT_PERMISSION` / `LOGIN_DEL_RBAC_ACCOUNT_PERMISSION` / `LOGIN_SEL_RBAC_ACCOUNT_PERMISSIONS` | Per-account grant/deny persistence | auth |

No DBC/DB2 stores.

---

## 7. Wire-protocol packets (if any)

None directly. `AccountMgr` is invoked indirectly through:
- `CMSG_AUTH_SESSION` / BNet `LogonRequest` (REST) — pre-flight CheckPassword
- GM-command CMSG (e.g. `.account *`) handled in `WorldSession::HandleChatMessageOpcode`

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-network` | `crate_dir` | 6 | 1716 | `exists_active` | crate exists |
| `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `crates/wow-world` | `crate_dir` | 17 | 12778 | `exists_active` | crate exists |
| `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `crates/wow-network/src/world_socket.rs` | `file` | 1 | 1023 | `exists_active` | file exists |
| `crates/bnet-server/src/rest/handlers.rs` | `file` | 1 | 573 | `exists_active` | file exists |
| `crates/wow-constants` | `crate_dir` | 10 | 5477 | `exists_active` | crate exists |
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-database/src/statements/login.rs` — declares the SQL constants (`SEL_RBAC_ACCOUNT_PERMISSIONS`, `INS_RBAC_ACCOUNT_PERMISSION`, `DEL_RBAC_ACCOUNT_PERMISSION`, `INS_ACCOUNT`, `UPD_LOGON`, `UPD_USERNAME`, `UPD_EMAIL`, `UPD_REG_EMAIL`, `SEL_CHECK_PASSWORD*`, `GET_GMLEVEL_BY_REALMID`, `INS/DEL_ACCOUNT_ACCESS*`, `INS_BNET_ACCOUNT`, `SEL_BNET_*`) — strings only, no callers.
- `crates/wow-database/src/statements/character.rs` — `SEL_SUM_CHARS`, `DEL_TUTORIALS`, `DEL_ACCOUNT_DATA`, `DEL_CHARACTER_BAN`, `SEL_CHARS_BY_ACCOUNT_ID` — same: declared, not used.
- `crates/wow-network/src/world_socket.rs` — uses `SEL_ACCOUNT_INFO_BY_NAME` to resolve a session-key/security/expansion bundle from the realm-join ticket. This is a **WoW client login fast-path** that bypasses `AccountMgr::HasPermission`; everything else (RBAC, Banned check, password change) is missing.
- `crates/bnet-server/src/rest/handlers.rs` (573 lines) — implements the BNet REST login flow (`POST /bnetserver/login/srp/`, `POST /bnetserver/login/`) inline; reads `battlenet_accounts` directly via SQL strings, does **not** route through a `BattlenetAccountMgr` API layer.

**What's implemented:**
- BNet login via SRPv2 in `bnet-server/src/rest/handlers.rs` (read-only path)
- Game-side session key lookup (`world-server/src/main.rs::DbAccountLookup`)
- All raw SQL statement strings registered in `wow-database`
- Ban-expiry housekeeping timer in `bnet-server/src/main.rs`

**What's missing vs C++:**
- `AccountMgr::CreateAccount / DeleteAccount / ChangeUsername / ChangePassword / ChangeEmail / ChangeRegEmail / CheckPassword(by name|by id) / GetId / GetName / GetEmail / GetSecurity(Async) / GetCharactersCount / IsBannedAccount / IsPlayerAccount / IsAdminAccount / IsConsoleAccount / HasPermission / UpdateAccountAccess / LoadRBAC` — none implemented
- `Battlenet::AccountMgr::CreateBattlenetAccount / ChangePassword / CheckPassword / LinkWithGameAccount / UnlinkGameAccount / GetId / GetName / GetIdByGameAccount(Async) / GetMaxIndex / GetSrpUsername` — only `GetId`-equivalent inline in REST handler; account creation API absent
- `rbac::RBACData` struct + `GrantPermission/DenyPermission/RevokePermission/LoadFromDB/HasPermission/CalculateNewPermissions/ExpandPermissions` — absent. There is no permission cache, no per-session `rbac` field, no enum for the ~440 `RBAC_PERM_*` ids
- `rbac::RBACPermission` (perm + linked permissions list) — absent
- `LoadRBAC` global cache (`rbac_permissions`, `rbac_linked_permissions`, `rbac_default_permissions`) — never loaded
- `RBAC_PERM_*` enum values — `wow-constants` has no equivalent
- GM-command surface (`.account *`, `.rbac *`) — none of these commands route to anything
- `ScriptMgr` hooks (`OnPasswordChange`, etc.) — module exists but these specific hooks are not wired

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Without RBAC, every gameplay check that should call `HasPermission` is either always-true or hard-coded by gmlevel — needs an audit of `wow-world` for hidden `gmlevel` constants.
- BNet REST registration may be silently writing rows that don't satisfy the same invariants as `Battlenet::AccountMgr::CreateBattlenetAccount` (specifically the `GetSrpUsername` = SHA256(email) hashing).
- The Rust `world-server` reads `account.session_key_bnet` as a 64-byte varbinary. #BNET.11 closed the old C#/hex ambiguity against Trinity C++: TC writes 64 raw bytes with `setBinary`, and Rust writes the same `client_secret[32] || server_secret[32]` raw bytes.
- `Utf8ToUpperOnlyLatin` semantics (TC's "upper-case Latin only, leave Cyrillic etc. alone") — Rust `.to_uppercase()` is **not** equivalent. Anything that compares username/password/email against a stored value must match this exact normalization or accounts created on the C++ stack will fail to log in via Rust.

**Tests existing:**
- 0 tests against AccountMgr/RBAC. The wow-database statement-string tests cover SQL syntax only.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#ACCOUNTS.WBS.001** Partir y cerrar la migracion auditada de `game/Accounts/AccountMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 607 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#ACCOUNTS.WBS.002** Cerrar la migracion auditada de `game/Accounts/AccountMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.h`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ACCOUNTS.WBS.003** Cerrar la migracion auditada de `game/Accounts/BattlenetAccountMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/BattlenetAccountMgr.cpp`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ACCOUNTS.WBS.004** Cerrar la migracion auditada de `game/Accounts/BattlenetAccountMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/BattlenetAccountMgr.h`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ACCOUNTS.WBS.005** Cerrar la migracion auditada de `game/Accounts/RBAC.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#ACCOUNTS.WBS.006** Partir y cerrar la migracion auditada de `game/Accounts/RBAC.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`
  Rust target: `crates/wow-database`, `crates/wow-network`, `crates/bnet-server`, `crates/wow-world`, `crates/wow-constants`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1018 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

- [ ] **#ACC.1** Crear `crates/wow-account/` (nuevo crate, layer L1) con módulos `account_mgr`, `bnet_account_mgr`, `rbac`. (M)
- [ ] **#ACC.2** Implementar `Utf8ToUpperOnlyLatin` byte-for-byte equivalente al de TC (sólo letras Latin1 a-z → A-Z, resto intocado). Tests con strings con cirílico, chino, acentos. (L)
- [ ] **#ACC.3** Definir `enum AccountOpResult` y constantes de longitud (`MAX_ACCOUNT_STR=16`, `MAX_PASS_STR=16`, `MAX_EMAIL_STR=64`, `MAX_BNET_EMAIL_STR=320`, `MAX_BNET_PASS_STR=128`). (L)
- [ ] **#ACC.4** Implementar `AccountMgr::create_account / delete_account / change_username / change_password / change_email / change_reg_email` — incluida la transacción de borrado de personajes en `delete_account`. (H)
- [ ] **#ACC.5** Implementar `AccountMgr::check_password(username|id, password)` reutilizando `wow_crypto::srp6::GruntSRP6` (verificar que existe; si no, añadirlo). (M)
- [ ] **#ACC.6** Lookups y getters: `get_id, get_name, get_email, get_security, get_security_async, get_characters_count`. (L)
- [ ] **#ACC.7** Helpers de gmlevel: `is_player_account / is_admin_account / is_console_account / is_banned_account`. (L)
- [ ] **#ACC.8** Generar el enum `RbacPermissions` con los ~440 valores de `RBAC.h` (script de extracción + tabla en `wow-constants`). (M)
- [ ] **#ACC.9** Implementar `struct RbacPermission { id, name, linked_permissions: HashSet<u32> }` + `RbacPermissionsContainer`. (L)
- [ ] **#ACC.10** Implementar `RbacData { id, name, realm_id, sec_level, granted, denied, global_perms }` con `grant / deny / revoke / has_permission / load_from_db / calculate_new_permissions / expand_permissions`. (H)
- [ ] **#ACC.11** Implementar `AccountMgr::load_rbac` cargando `rbac_permissions`, `rbac_linked_permissions`, `rbac_default_permissions` filtrado por realm actual. Singleton arrancado al boot del world-server. (M)
- [ ] **#ACC.12** Implementar `AccountMgr::has_permission(account_id, perm, realm_id)` y `update_account_access(rbac, account_id, sec_level, realm_id)`. (M)
- [ ] **#ACC.13** Cablear `WorldSession` para llevar un `Arc<RwLock<RbacData>>` cargado tras `AuthSession`. Reemplazar comprobaciones por `gmlevel` por `session.has_permission(RBAC_PERM_*)`. (H)
- [ ] **#ACC.14** Implementar `Battlenet::AccountMgr::create_battlenet_account(email, password, with_game_account)` — incluye la creación opcional de game account `<id>#1`. (M)
- [ ] **#ACC.15** Implementar `Battlenet::AccountMgr::change_password / check_password / link / unlink / get_id / get_name / get_id_by_game_account / get_max_index / get_srp_username` (último = hex(SHA256(uppercase(email)))). (M)
- [ ] **#ACC.16** Refactorizar `bnet-server/src/rest/handlers.rs` para invocar la API de `Battlenet::AccountMgr` en lugar de SQL inline. (M)
- [ ] **#ACC.17** Añadir hooks de scripting: `on_password_change`, `on_failed_password_change`, `on_email_change`, `on_failed_email_change`. (L)
- [ ] **#ACC.18** Comandos GM `.account create / delete / set password / set addon / set sec / lock / list` y `.rbac *`. (XL — en otro batch, depende de un dispatcher de comandos)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#ACCOUNTS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 2271 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-constants`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | `cargo test -p bnet-server && cargo test -p world-server && cargo test -p wow-constants` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#ACCOUNTS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 2271 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-constants`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#ACCOUNTS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 2271 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-constants`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#ACCOUNTS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 2271 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-constants`, `crates/wow-database`, `crates/wow-network`, `crates/wow-world`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Utf8ToUpperOnlyLatin("Élise — Привет — naïve")` produces the **exact** same byte sequence as TC's `Utf8ToUpperOnlyLatin` (snapshot from C++ run).
- [ ] Test: `create_account("Foo", "bar", "x@y")` — username/password/email get uppercased before INSERT; second call returns `AOR_NAME_ALREADY_EXIST`; resulting salt+verifier reproduce `GruntSRP6` registration data byte-for-byte.
- [ ] Test: `change_password` regenerates the verifier such that `check_password(username, new_pass) == true` and `check_password(username, old_pass) == false`.
- [ ] Test: `delete_account` is transactional — kills `account`, `account_access`, `realmcharacters`, `account_banned`, `account_muted`, plus `characters.tutorials / account_data / character_banned` for that account; an interruption in the middle does not leave orphans.
- [ ] Test: `Battlenet::create_battlenet_account(email, password, true)` produces a game account named `<bnetId>#1` whose password is the first 16 chars of the BNet password (uppercased), and the `account.battlenet_account = bnetId, battlenet_index = 1` link is set.
- [ ] Test: `RbacData::has_permission` after `load_from_db` matches expanded set: defaults for sec_level + granted − denied with linked permission expansion working transitively.
- [ ] Test: `RBAC_PERM_SKIP_CHECK_CHAT_SPAM` granted ⇒ `WorldSession::can_skip_chat_spam_check()` returns `true`. (Integration test once handlers exist.)
- [ ] Test: `get_security(account_id, realm_id=-1)` (the global access row) returns the right level when both `(account, -1)` and `(account, realm)` rows exist.
- [ ] Test: SRPv2 BNet check — sample (email, password) from the C++ test fixtures verifies; SRPv1 path also verifies a v1-stored account.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 6 files / 2271 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp` | `crates/wow-database/`, `crates/wow-network/`, `crates/bnet-server/`, `crates/wow-world/` (consumer) \| ❌ not started — confirmed via audit 2026-05-01 (only DB statement strings; no AccountMgr / RBACData logic; no `Utf8ToUpperOnlyLatin`; ~440 RBAC perms unrepresented) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#ACCOUNTS.DIV.001` | _none generated_ | 6 C++ files / 2271 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.h`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/AccountMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/game/Accounts/RBAC.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **Username/password/email are always stored uppercased** (Latin-only). This is *load-bearing* for SRP6 — the verifier is computed against the uppercased form. Any normalization mismatch breaks login silently.
- **BNet SRP username is NOT the email**. It is `bytes_to_hex(SHA256(uppercase(email)))`. This is what's fed into `BnetSRP6v2`. Confusing this with the email is a recurring source of "password rejected" bugs.
- **SRPv1 vs SRPv2**: `battlenet_accounts.srp_version` column. v1 = SHA256, ≤16 chars, case-insensitive (uppercase). v2 = PBKDF2-SHA512, ≤128 chars, case-sensitive. New accounts always v2; v1 path exists only for reading legacy records.
- **`LoadRBAC` filters by current realm**: `WHERE realmId = {} OR realmId = -1`. A change to the realm id in `worldserver.conf` requires a full RBAC reload.
- **Permissions are inherited transitively** via `rbac_linked_permissions` (a permission can link to another, which can link to another). `ExpandPermissions` does BFS until stable. There's an explicit guard against self-link but no cycle detection beyond that — TC relies on the DB being well-formed.
- **`HasPermission(0, ...)` always returns false** with an error log; `accountId=0` is the sentinel for "no session" / "unauthenticated".
- **Race on delete**: `DeleteAccount` directly calls `WorldSession::KickPlayer + LogoutPlayer(false)`. Replicating this from a non-game thread requires bouncing through the session's command queue — don't call `KickPlayer` from a sqlx callback.
- **`rbac_default_permissions`** is keyed by `secId` (security level uint8). `secLevel = 255` means "uninitialized"; `RBACData` never matches default perms in that state.
- WotLK 3.4.3 *Classic* keeps the BNet schema from retail Cataclysm+, even though the gameplay is WotLK. The `battlenet_accounts` / `battlenet_account_bans` tables are **not** the legacy 2.x auth tables.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class AccountMgr` (singleton `sAccountMgr`) | `struct AccountMgr` + `OnceLock<AccountMgr>` (or simple module-level functions) in `crates/wow-account/src/account_mgr.rs` | Stateless except for the RBAC cache (`HashMap<u32, RBACPermission>` + `HashMap<u8, HashSet<u32>>`). Use `parking_lot::RwLock` for the cache. |
| `enum class AccountOpResult : uint8` | `enum AccountOpResult` (no repr needed; never crosses FFI) | — |
| `Utf8ToUpperOnlyLatin(std::string&)` | `wow_account::util::utf8_to_upper_only_latin(&mut String)` | Must match TC byte-for-byte. Don't use `str::to_uppercase`. |
| `Trinity::Crypto::SRP::GruntSRP6` / `BnetSRP6v1` / `BnetSRP6v2` | `wow_crypto::srp6::{GruntSrp6, BnetSrp6V1, BnetSrp6V2}` | Already partial in `wow-crypto`; verify v1 + v2 BNet exist. |
| `Trinity::Crypto::SRP6::MakeRegistrationData<Algo>(name, pass)` | `wow_crypto::srp6::make_registration_data::<Algo>(&name, &pass) -> (Salt, Verifier)` | Returns 32-byte salt + variable-length verifier. |
| `LoginDatabasePreparedStatement* stmt = LoginDatabase.GetPreparedStatement(LOGIN_INS_ACCOUNT)` | `let mut stmt = login_db.prepare(LoginStatements::INS_ACCOUNT)` | Already exists in `wow-database`. |
| `LoginDatabaseTransaction trans = LoginDatabase.BeginTransaction(); trans->Append(stmt); LoginDatabase.CommitTransaction(trans);` | `let mut tx = login_db.begin_transaction().await?; tx.execute(&stmt).await?; tx.commit().await?;` | Use `wow_database::transaction::Transaction`. |
| `class rbac::RBACData` | `struct wow_account::rbac::RbacData { id: u32, name: String, realm_id: i32, sec_level: u8, granted: HashSet<u32>, denied: HashSet<u32>, global: HashSet<u32> }` | Methods take `&mut self`. |
| `enum rbac::RBACPermissions` (442 values) | `pub enum RbacPermissions { ... }` in `crates/wow-constants/src/rbac.rs` (or a `pub mod rbac { pub const PERM_X: u32 = ...; }`) | Probably easier as `const`s than enum, given the spread of values 1..1000+. |
| `class rbac::RBACPermission` | `struct RbacPermission { id: u32, name: String, linked: HashSet<u32> }` | Cheap to clone if needed. |
| `RBACPermissionsContainer` (`std::map<uint32, RBACPermission*>`) | `HashMap<u32, RbacPermission>` (no need for `Arc`; AccountMgr owns) | — |
| `namespace Battlenet::AccountMgr { ... }` | `pub mod bnet_account_mgr` (free functions) | — |
| `QueryCallback GetSecurityAsync(...)` | `async fn get_security_async(...) -> u32` | Naturally async in Rust; no callback indirection needed. |
| `sAccountMgr->GetRBACPermission(id)` | `account_mgr().get_rbac_permission(id) -> Option<&RbacPermission>` | — |
| `sScriptMgr->OnPasswordChange(accountId)` | `wow_scripts::hooks::on_password_change(account_id)` | Wire only after the script hook system is present. |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Method:** `grep -rE "(RBAC|rbac|HasPermission|has_permission|AccountMgr|account_mgr|Utf8ToUpperOnlyLatin|utf8_to_upper_only_latin)" crates/`. Inspected `crates/wow-database/src/statements/login.rs`, `crates/wow-network/src/world_socket.rs`, `crates/bnet-server/src/rest/handlers.rs`, and verified `crates/wow-account/` does not exist.

**Verdicts on flagged absences:**

1. **`AccountMgr` / `Battlenet::AccountMgr` — CONFIRMED ABSENT.** No `account_mgr` module, no `wow-account` crate, no `CreateAccount` / `DeleteAccount` / `ChangePassword` / `CheckPassword(by_name|by_id)` / `HasPermission` / `LoadRBAC` symbols anywhere in the workspace. The only matches are SQL-statement-string declarations in `wow-database/src/statements/login.rs` (`SEL_RBAC_ACCOUNT_PERMISSIONS`, `INS_RBAC_ACCOUNT_PERMISSION`, `DEL_RBAC_ACCOUNT_PERMISSION`) — declared, never called.
2. **`RBAC_PERM_*` enum (~440 values) — CONFIRMED ABSENT.** No `rbac` module in `wow-constants`. No `RBACPermissions` enum, no `RBACData` struct, no `RBACPermission` catalogue, no `expand_permissions` / `calculate_new_permissions` logic. Every gameplay check that should call `HasPermission(RBAC_PERM_*)` either short-circuits to `true` or hard-codes a `gmlevel` numeric — needs an audit pass during #ACC.13.
3. **`Utf8ToUpperOnlyLatin` — CONFIRMED ABSENT and load-bearing.** Zero hits for `utf8_to_upper_only_latin` / `Utf8ToUpperOnlyLatin` / `upper_only_latin` across `crates/`. **This is critical for SRP6 verifier compatibility:** TC's `MakeRegistrationData<GruntSRP6>(name, pass)` computes `H(salt | H(uppercased_name : uppercased_pass))` — any Rust-side normalization that uses `str::to_uppercase()` instead of TC's Latin-only upcase will produce a different verifier and silently lock out every account that was created on the C++ stack (and vice-versa). Recommend implementing #ACC.2 *before* #ACC.4-5, with a snapshot test against a known C++ output for at least one Cyrillic + one accented-Latin string.

**Other findings during the audit:**

- **BNet REST login (read-only path) works** — `crates/bnet-server/src/rest/handlers.rs` (~573 LOC) implements SRPv2 verification inline against `battlenet_accounts`, but does **not** route through any `BattlenetAccountMgr` API layer. Account *creation* via REST is therefore not wired.
- **World-side session-key fetch works** — `crates/wow-network/src/world_socket.rs` uses `SEL_ACCOUNT_INFO_BY_NAME` to bundle session_key + security + expansion from the realm-join ticket. This is a *fast-path* that bypasses RBAC entirely.
- **No `OnPasswordChange` / `OnEmailChange` script hooks wired** despite `wow-script(s)` crates existing.
- **`session_key_bnet` storage format** — #BNET.11 closed the 64-byte varbinary vs 64-hex-char ambiguity against Trinity C++: WotLK BNet realm join stores 64 raw bytes, not ASCII hex. Future regressions should verify the 32-byte client secret and 32-byte server secret lengths instead.
- **GM-command surface (`.account *`, `.rbac *`)** entirely absent — depends on a command dispatcher that doesn't yet exist, so #ACC.18 is correctly deferred to a later wave.

**Status verdict:** ❌ not started (no change). The flagged risks (#ACC.2 SRP normalization, #ACC.8-12 RBAC) are real and high-impact. Recommend the migration order: #ACC.2 → #ACC.5 (verify SRP byte-equivalence on a fixture) → #ACC.4 → #ACC.8 → #ACC.10–12 → #ACC.13. Don't try to land RBAC and AccountMgr in one PR — they're separable.
