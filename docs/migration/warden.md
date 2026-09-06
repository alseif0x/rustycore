# Migration: Warden (anti-cheat)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/game/Warden/` — `Warden.{h,cpp}` (base class, 274+131 lines), `WardenWin.{h,cpp}` (Windows-client variant, 560+86 lines), `WardenMac.{h,cpp}` (Mac-client variant, 244+43 lines), `WardenCheckMgr.{h,cpp}` (check loader/registry, 191+140 lines), `enuminfo_WardenCheckMgr.cpp` (auto-generated SmartEnum metadata, 169 lines), `Modules/WardenModuleWin.h` (1239 lines — embedded Warden module binary), `Modules/WardenModuleMac.h` (613 lines — embedded module binary).
> **Rust target crate(s):** **No crate exists.** The opcodes `Warden3Data` (CMSG 0x35ed, SMSG 0x2577), `Warden3Disabled` (SMSG 0x2823), `Warden3Enabled` (SMSG 0x2822) are present in `crates/wow-constants/src/opcodes.rs` (lines 671, 1619-1621); nothing else exists. No handler arm in `crates/wow-world/src/session.rs`. No `Warden` struct, no `WardenCheckMgr`, no module binary, no DB schema, no SQL loader, no SmartEnum scaffolding, no per-OS variant. **0 lines of Warden code.**
> **Layer:** L7 (anti-cheat infrastructure — depends on Crypto L1 (RC4 + SHA1 + HMAC + MD5), WorldSession L4, World config L1, Auth Bn-Net session key L1; depended on by Account banning + Player kick + GM tooling for cheat report)
> **Status:** ❌ not started — only opcode constants exist (3 of them). All ~3690 lines of C++ Warden code have no Rust counterpart. The Trinity Warden module binaries (`WardenModuleWin.h` / `WardenModuleMac.h` — large embedded RC4-encrypted blobs) are not vendored in the Rust repo either.
> **Audited vs C++:** ✅ audited 2026-05-01 (status confirmed ❌ — exactly 3 opcode constants, no module code)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Warden is Blizzard's client-side anti-cheat. The server boots an opaque RC4-encrypted module to the client (the binary blob is in `Modules/WardenModule{Win,Mac}.h` — verbatim bytes captured from a real WoW client and re-uploaded), the client loads it, and the server then sends periodic encrypted "check" requests asking the module to scan client memory / scan loaded DLL list / check SHA1 of MPQ files / evaluate Lua snippets / verify timing. The module replies with check results; the server compares against expected values and applies a configured action (Log/Kick/Ban). The protocol is RC4-symmetric using a session key derived from the BNet auth `K`, and uses a small custom XOR cipher inside check requests as an additional obfuscation. Per-OS variants exist because the memory addresses checked, the function offsets in `WoW.exe` / `WoW.app/Contents/MacOS/World of Warcraft`, and the embedded module binaries are different.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `game/Warden/Modules/WardenModuleMac.h` | 613 | `prefix` |
| `game/Warden/Modules/WardenModuleWin.h` | 1239 | `prefix` |
| `game/Warden/Warden.cpp` | 274 | `prefix` |
| `game/Warden/Warden.h` | 131 | `prefix` |
| `game/Warden/WardenCheckMgr.cpp` | 191 | `prefix` |
| `game/Warden/WardenCheckMgr.h` | 140 | `prefix` |
| `game/Warden/WardenMac.cpp` | 244 | `prefix` |
| `game/Warden/WardenMac.h` | 43 | `prefix` |
| `game/Warden/WardenWin.cpp` | 560 | `prefix` |
| `game/Warden/WardenWin.h` | 86 | `prefix` |
| `game/Warden/enuminfo_WardenCheckMgr.cpp` | 169 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/game/Warden/Warden.h` | 131 | Base class + `enum WardenOpcodes` (3 CMSG + 6 SMSG opcodes inside the warden subprotocol — distinct from the world opcodes), wire structs `WardenModuleUse`, `WardenModuleTransfer`, `WardenHashRequest`, helper struct `ClientWardenModule { Id, Key, CompressedData, CompressedSize }` |
| `src/server/game/Warden/Warden.cpp` | 274 | Base class implementation: `MakeModuleForClient` (compute MD5 of module data → ID), `SendModuleToClient` (chunked 500-byte send), `RequestModule` (tell client to use module), `Update(diff)` (re-fire `RequestChecks` after `_checkTimer`), `DecryptData` / `EncryptData` (RC4 in/out), `IsValidCheckSum` / `BuildChecksum` (5×u32 XOR-fold of SHA1), `ApplyPenalty(check)` (Kick/Ban/Log dispatch), `HandleData(buff)` (decrypt then dispatch on inner opcode), `ProcessLuaCheckResponse` (the `_TW\t<id>` token from the in-game Lua sandbox — the `SendAddonMessage` mechanism). `WorldSession::HandleWardenData` glue. |
| `src/server/game/Warden/WardenCheckMgr.h` | 140 | `enum WardenActions : u8 { LOG=0, KICK=1, BAN=2 }`. `enum WardenCheckCategory : u8 { INJECT_CHECK_CATEGORY, LUA_CHECK_CATEGORY, MODDED_CHECK_CATEGORY, NUM_CHECK_CATEGORIES }`. `enum WardenCheckType : u8 { NONE_CHECK=0, TIMING_CHECK=87, DRIVER_CHECK=113, PROC_CHECK=126, LUA_EVAL_CHECK=139, MPQ_CHECK=152, PAGE_CHECK_A=178, PAGE_CHECK_B=191, MODULE_CHECK=217, MEM_CHECK=243 }`. `constexpr GetWardenCheckCategory(type)`, `GetWardenCategoryCountConfig(category)`, `IsWardenCategoryInWorldOnly(category)`. `struct WardenCheck { CheckId, Type, Data, Address, Length, Str, Comment, IdStr[4], Action }`. `WARDEN_MAX_LUA_CHECK_LENGTH = 170`. `WardenCheckResult = vector<u8>`. `class WardenCheckMgr` (singleton). |
| `src/server/game/Warden/WardenCheckMgr.cpp` | 191 | `LoadWardenChecks()` reads `WorldDatabase.warden_checks` (`SELECT id, type, data, result, address, length, str, comment FROM warden_checks ORDER BY id ASC`), validates type, populates `_checks` vector + `_checkResults` map (for MEM_CHECK/MPQ_CHECK expected bytes) + `_pools[category]` index. `LoadWardenOverrides()` reads `CharacterDatabase.warden_action` (`SELECT wardenId, action FROM warden_action`) and overrides per-check Action. |
| `src/server/game/Warden/enuminfo_WardenCheckMgr.cpp` | 169 | Auto-generated by Trinity's `enumutils` tool from the `// EnumUtils: DESCRIBE THIS` annotations on the enums above; provides `EnumUtils::Iterate<E>()`, `EnumUtils::ToConstant(E)`, `EnumUtils::ToTitle(E)`. |
| `src/server/game/Warden/WardenWin.h` | 86 | Windows variant. `struct WardenInitModuleRequest` — packed (size 56 bytes) — contains hardcoded `WoW.exe` function offsets to be hooked by the module: `SFileOpenFile=0x00024F80`, `SFileGetFileSize=0x000218C0`, `SFileReadFile=0x00022530`, `SFileCloseFile=0x00022910`, `FrameScript::Execute=0x00419210`, `PerformanceCounter=0x0046AE20` (all relative to `0x00400000` PE base). Class `WardenWin : public Warden`. |
| `src/server/game/Warden/WardenWin.cpp` | 560 | Windows logic. Lua eval format strings (`_luaEvalPrefix`/`_luaEvalMidfix`/`_luaEvalPostfix`) totaling 255 minus reserved Lua check len. `Init(session, K)` — derive 16-byte input/output keys via `SessionKeyGenerator<SHA1>` from BNet `K`, init RC4, build module for client, request module. `InitializeModuleForClient` populates `module.{CompressedData, CompressedSize, Key}` from the embedded `Module` record in `WardenModuleWin.h`. `InitializeModule` sends the 3-part `WardenInitModuleRequest` with the function offsets. `RequestHash` sends `WARDEN_SMSG_HASH_REQUEST` with `_seed`. `HandleHashResult` verifies SHA1 response equals `Module.ClientKeySeedHash`, then **rotates keys** to `Module.ClientKeySeed` / `Module.ServerKeySeed`. `RequestChecks` builds the check packet: random-shuffle pools, build TIMING_CHECK byte + per-check inner format (MEM/PAGE_A/B/DRIVER/MODULE/MPQ/LUA_EVAL each have specific layouts with `_inputKey[0]`-XOR obfuscation), trim to 450 bytes max, encrypt, send. `HandleCheckResult` parses response: length+checksum verify, TIMING_CHECK delta, then per-check result based on `WardenCheckType` (MEM_CHECK compares bytes, PAGE/DRIVER/MODULE expect `0xE9`, LUA_EVAL expects `result_byte` then optional string, MPQ_CHECK SHA1 compare). `DEBUG_ForceSpecificChecks` for testing. |
| `src/server/game/Warden/WardenMac.h` | 43 | Class `WardenMac : public Warden`. No `WardenInitModuleRequest` — Mac client doesn't need server-supplied function offsets in init. |
| `src/server/game/Warden/WardenMac.cpp` | 244 | Mac variant. Hardcoded `_seed = 4D 80 8D 2C 77 D9 05 C4 1A 63 80 EC 08 58 6A FE`. `Init` is similar to Win but skips the 3-part init module request. `HandleHashResult` does an obscure 4-`int` XOR/sub/mul transformation (`0xDEADBEEFu`, `0x35014542u`, `0x5313F22u`, `0x1337F00Du`) on the seed before hashing — this is the Mac module's specific key-rotation algorithm. `RequestChecks` is much simpler than Win — only timing + a few of the 9 check types are implemented. `InitializeModule` is a no-op (the Mac client auto-initializes). |
| `src/server/game/Warden/Modules/WardenModuleWin.h` | 1239 | The Windows Warden module binary, 18 KB compressed and RC4-encrypted with `Module.ModuleKey`, embedded as a C `std::array<uint8, N>`. Includes `Module.Seed` (16 bytes), `Module.ClientKeySeedHash` (20 bytes SHA1), `Module.ClientKeySeed` (16 bytes), `Module.ServerKeySeed` (16 bytes), `Module.ModuleKey` (16 bytes RC4 key), and `Module.Module` (the actual ~18 KB blob). |
| `src/server/game/Warden/Modules/WardenModuleMac.h` | 613 | The Mac Warden module — different blob (`Module_0DBBF209A27B1E279A9FEC5C168A15F7_Data`), smaller, uses MD5 `0DBBF209A27B1E279A9FEC5C168A15F7` as identifier. |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Warden` | abstract base class | Owns `_session`, `_inputKey/_outputKey/_seed` (16 bytes each), `_inputCrypto/_outputCrypto: ARC4`, `_checkTimer` (when to next ask), `_clientResponseTimer` (when client must respond by), `_dataSent`, `_module: Optional<ClientWardenModule>`, `_initialized`. Virtuals: `Init(session, K)`, `InitializeModule()`, `RequestHash()`, `HandleHashResult(buf)`, `HandleCheckResult(buf)`, `InitializeModuleForClient(module)`, `RequestChecks()`. |
| `WardenWin` | derived class | Windows variant. Adds `_serverTicks` (for TIMING_CHECK round-trip), `_checks: [(vector<u16>, iterator); 3]` (per-category check pool with rolling iterator), `_currentChecks: vector<u16>` (in-flight). |
| `WardenMac` | derived class | Mac variant. Stripped down — does not implement `RequestChecks` like Win does (only 1 of the 9 check types is meaningfully active). |
| `WardenCheckMgr` | singleton | Owns `_checks: vector<WardenCheck>` (indexed by CheckId), `_checkResults: unordered_map<u16, vector<u8>>` (expected MEM/MPQ result bytes), `_pools: array<vector<u16>, 3>` (per-category index of valid CheckIds for round-robin shuffling). |
| `WardenCheck` | struct | One check definition: `CheckId`, `Type` (WardenCheckType), `Data` (binary blob — for PAGE_CHECK and DRIVER_CHECK), `Address` (u32 — for MEM_CHECK / PAGE / PROC), `Length` (u8 — same), `Str` (string — for LUA / MPQ / DRIVER / MODULE: the human-readable name or Lua snippet), `Comment` (description), `IdStr[4]` (zero-padded 4-char ASCII id for LUA), `Action` (per-check action override). |
| `ClientWardenModule` | struct | Module identifier sent to client: `Id[16]` (MD5 of compressed data), `Key[16]` (RC4 key the client should use to decrypt), `CompressedData` ptr, `CompressedSize`. |
| `WardenModuleUse` | wire-packed struct (37 bytes) | `Command`, `ModuleId[16]`, `ModuleKey[16]`, `Size: u32`. Sent to client to tell it to use the module. |
| `WardenModuleTransfer` | wire-packed struct (1+2+500 = 503 bytes) | `Command`, `DataSize: u16`, `Data[500]`. Chunked module upload. |
| `WardenHashRequest` | wire-packed struct (17 bytes) | `Command`, `Seed[16]`. |
| `WardenInitModuleRequest` | wire-packed struct (~56 bytes, Win only) | 3 sequential mini-requests, each `(Command, Size, CheckSumm, Unk1, Unk2, Type/Lib_id, Function[1..4])`. The 6 hardcoded `WoW.exe` function offsets are embedded here. |
| `WardenOpcodes` | enum | Inner-protocol opcodes. CMSG: `MODULE_MISSING=0`, `MODULE_OK=1`, `CHEAT_CHECKS_RESULT=2`, `MEM_CHECKS_RESULT=3` (only sent if MEM_CHECK bytes don't match — separately from CHEAT_CHECKS_RESULT), `HASH_RESULT=4`, `MODULE_FAILED=5`. SMSG: `MODULE_USE=0`, `MODULE_CACHE=1`, `CHEAT_CHECKS_REQUEST=2`, `MODULE_INITIALIZE=3`, `MEM_CHECKS_REQUEST=4`, `HASH_REQUEST=5`. **All these are inner opcodes encrypted within `WARDEN3_DATA` (0x35ed/0x2577) world packets.** |
| `WardenActions` | enum (u8) | `LOG=0`, `KICK=1`, `BAN=2`. |
| `WardenCheckCategory` | enum (u8) | `INJECT_CHECK_CATEGORY=0` (driver/page/module — DLL injection / executable patching), `LUA_CHECK_CATEGORY=1` (in-game Lua sandbox tampering), `MODDED_CHECK_CATEGORY=2` (MPQ files / memory bytes — addon / data file modification). |
| `WardenCheckType` | enum (u8) | The 10 check types — see §2 row 3. **Values are not sequential:** 0, 87, 113, 126, 139, 152, 178, 191, 217, 243 — these are Warden's own type constants used as wire bytes (XOR'd with `_inputKey[0]`). |

Constants:

- `WARDEN_MAX_LUA_CHECK_LENGTH = 170` — enforced at load time to make sure prefix + check + suffix fits in 255 bytes.
- `Trinity::Crypto::HMAC_SHA1::DIGEST_LENGTH = 20` — used for MODULE_CHECK seed+digest.
- Module sizes: Win module ~18 KB, Mac module ~12 KB. Burst chunk size 500 bytes.
- `_checkTimer` initial value: `10 * IN_MILLISECONDS = 10000ms` (10 seconds).
- Hold-off after a check round: `CONFIG_WARDEN_CLIENT_CHECK_HOLDOFF` (default 30 seconds).
- Response timeout: `CONFIG_WARDEN_CLIENT_RESPONSE_DELAY` (default 600 seconds = 10 minutes).
- Per-category checks-per-round: `CONFIG_WARDEN_NUM_INJECT_CHECKS` (default 9), `CONFIG_WARDEN_NUM_LUA_CHECKS` (default 1), `CONFIG_WARDEN_NUM_CLIENT_MOD_CHECKS` (default 1).
- Ban duration: `CONFIG_WARDEN_CLIENT_BAN_DURATION` (default 86400 seconds = 24h).
- Master switch: `CONFIG_WARDEN_ENABLED` (default false in TC dist).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `Warden::Update(diff)` | Per-tick: if `_initialized`, either count down `_clientResponseTimer` (and kick on overflow) or count down `_checkTimer` and fire `RequestChecks`. | `RequestChecks`, `_session->KickPlayer` |
| `Warden::HandleData(buff)` | Decrypt incoming buffer with `_inputCrypto` (RC4), read inner opcode byte, dispatch on it: `MODULE_MISSING` → `SendModuleToClient`, `MODULE_OK` → `RequestHash`, `CHEAT_CHECKS_RESULT` → `HandleCheckResult`, `HASH_RESULT` → `HandleHashResult` then `InitializeModule`. | `SendModuleToClient`, `RequestHash`, `HandleCheckResult`, `HandleHashResult`, `InitializeModule` |
| `Warden::SendModuleToClient()` | Chunked send of module's compressed bytes — 500 bytes per chunk, opcode `WARDEN_SMSG_MODULE_CACHE`. Each chunk encrypted with `_outputCrypto`. | `EncryptData`, `_session->SendPacket(SMSG_WARDEN3_DATA)` |
| `Warden::RequestModule()` | Send `WARDEN_SMSG_MODULE_USE` with `WardenModuleUse{ ModuleId, ModuleKey, Size }`. | `EncryptData` |
| `Warden::MakeModuleForClient()` | Init `_module = ClientWardenModule{...}` from per-OS `InitializeModuleForClient` then compute `_module->Id = MD5(_module->CompressedData, _module->CompressedSize)`. | `Trinity::Crypto::MD5::GetDigestOf` |
| `Warden::DecryptData(buf, len)` / `EncryptData(buf, len)` | RC4 stream cipher inplace. | `_inputCrypto.UpdateData` / `_outputCrypto.UpdateData` |
| `Warden::IsValidCheckSum(checksum, data, length)` | Verify the 5×u32 XOR-fold of SHA1: compute SHA1 of `data[0..length]`, view as `u32[5]`, XOR all five, compare to `checksum`. | `Trinity::Crypto::SHA1::GetDigestOf` |
| `Warden::BuildChecksum(data, length)` | Compute 5×u32 XOR-fold of SHA1. | — |
| `Warden::ApplyPenalty(check)` | Resolve action: per-check `check->Action` if non-null, else default from `CONFIG_WARDEN_CLIENT_FAIL_ACTION`. Dispatch: KICK → `KickPlayer`; BAN → `BanAccount(BAN_ACCOUNT, accountName, duration, reason, "Server")`; LOG → no-op. Returns the action-name string for logging. | `_session->KickPlayer`, `sWorld->BanAccount`, `AccountMgr::GetName` |
| `Warden::ProcessLuaCheckResponse(msg)` | Called by `WorldSession::HandleAddonMessage` when receiving an addon-channel message starting with `_TW\t`: parse `<id>` after the tab, look up `WardenCheck` by id, if it's a `LUA_EVAL_CHECK` apply penalty (because the in-Warden Lua eval delivers its result via this back-channel — addon message instead of warden packet). Returns `true` if it was a Warden token (and should not be relayed to other addons). | `sWardenCheckMgr->GetCheckData`, `ApplyPenalty` |
| `WardenWin::Init(session, K)` | Derive `_inputKey` + `_outputKey` (16 bytes each) from `SessionKeyGenerator<SHA1>(K)`, set `_seed = Module.Seed`, init RC4, call `MakeModuleForClient` then `RequestModule`. | `SessionKeyGenerator`, `MakeModuleForClient`, `RequestModule` |
| `WardenWin::InitializeModule()` | Send `WardenInitModuleRequest` (3-part) with the 6 `WoW.exe` function offsets, encrypted. | `EncryptData` |
| `WardenWin::RequestHash()` | Send `WardenHashRequest{ Seed = _seed }`. | `EncryptData` |
| `WardenWin::HandleHashResult(buff)` | Read 20-byte SHA1 response, compare to `Module.ClientKeySeedHash`. On mismatch → penalty. On match: rotate `_inputKey ← Module.ClientKeySeed`, `_outputKey ← Module.ServerKeySeed`, re-init both RC4 streams, set `_initialized = true`. | `Trinity::Crypto::SHA1`, `_inputCrypto.Init` |
| `WardenWin::RequestChecks()` | The big one — see §2 row 7. Builds a randomized check batch trimmed to <450 bytes, packed with per-check XOR-obfuscated type bytes, sent encrypted. | `Trinity::Containers::RandomShuffle`, `Trinity::Crypto::HMAC_SHA1::GetDigestOf` (for MODULE_CHECK), `Trinity::Crypto::GetRandomBytes<4>` (MODULE_CHECK seed) |
| `WardenWin::HandleCheckResult(buff)` | Validate length and checksum, parse TIMING_CHECK timing, per-check parse expected response (MEM bytes, PAGE/DRIVER/MODULE expect `0xE9`, MPQ SHA1 compare, LUA result byte), record `checkFailed` if any failed → `ApplyPenalty(failed_check)`. Reset `_checkTimer = max(1, CONFIG_WARDEN_CLIENT_CHECK_HOLDOFF) * 1000`. | `IsValidCheckSum`, `ApplyPenalty` |
| `WardenMac::Init(session, K)` | Identical scaffold to WardenWin but seed is hardcoded constant. | — |
| `WardenMac::HandleHashResult(buff)` | The XOR/sub/mul transformation on the Mac seed, then SHA1 compare. | — |
| `WardenCheckMgr::LoadWardenChecks()` | Read `world.warden_checks` table (8 columns), validate per-row: skip if type unknown, skip LUA checks with id > 9999 or string len > 170, populate `_checks[id]` + `_checkResults[id]` (for MEM/MPQ) + `_pools[category]`. Init `wardenCheck.IdStr` from `format("{:04}", id)` for LUA. | `WorldDatabase.Query`, `sWorld->getIntConfig(CONFIG_WARDEN_CLIENT_FAIL_ACTION)` |
| `WardenCheckMgr::LoadWardenOverrides()` | Read `characters.warden_action`, validate (action must be 0-2, checkId must exist in `_checks`), override `_checks[id].Action`. | `CharacterDatabase.Query` |
| `WardenCheckMgr::GetCheckData(id)` / `GetCheckResult(id)` | Lookup by id; ASSERTs on out-of-range. | — |
| `WardenCheckMgr::GetAvailableChecks(category)` | Return `_pools[category]` for round-robin shuffling. | — |
| `WorldSession::HandleWardenData(packet)` | Forward `packet.Data` (encrypted bytes) to `_warden->HandleData`. | `Warden::HandleData` |

---

## 5. Module dependencies

**Depends on:**
- `Crypto` — `Trinity::Crypto::ARC4` (RC4 stream), `Trinity::Crypto::SHA1` (hash + key generation), `Trinity::Crypto::HMAC_SHA1` (MODULE_CHECK), `Trinity::Crypto::MD5` (module ID), `Trinity::Crypto::GetRandomBytes<4>` (MODULE_CHECK seed).
- `Auth/SessionKeyGenerator` — derives `_inputKey` and `_outputKey` from BNet auth `K`. The same `K` is used for world packet header crypto, so the Warden keys must be derived **after** `K` is known but **before** Warden init begins. This couples Warden to the auth flow in a hard ordering constraint.
- `WorldSession` — `KickPlayer`, `SendPacket(SMSG_WARDEN3_DATA)`, `GetAccountId`, `GetLatency`, `GetRemoteAddress`, `GetPlayer()` (for in-world LUA checks), `PlayerLoading()`.
- `World` config — 9 config keys (see "Constants" in §3).
- `WorldDatabase` — `warden_checks` table.
- `CharacterDatabase` — `warden_action` table.
- `LoginDatabase` — `BanAccount` writes (when penalty is BAN).
- `AccountMgr` — `GetName(accountId)` for ban reason text.
- `EnumUtils` (Trinity's smart-enum macro tooling) — for `ToConstant`/`ToTitle`/`Iterate` on the enums.

**Depended on by:**
- `WorldSession::HandleAddonMessage` — calls `_warden->ProcessLuaCheckResponse(msg)` for every addon-channel message starting with `_TW\t`, gating before normal addon-message dispatch.
- World tick loop — `_warden->Update(diff)` per session per frame.
- BNet/auth flow — `Warden` is constructed and `Init(session, K)` called right after the world handshake completes (only if config-enabled).
- Cheat reporting — `WardenCheckMgr` is the source of truth for what checks exist; GM tools read it for filing reports.

---

## 6. SQL / DB queries (if any)

Schema (3.4.3 world DB + character DB):

```sql
CREATE TABLE warden_checks (
  id      SMALLINT UNSIGNED PRIMARY KEY,
  type    TINYINT UNSIGNED NOT NULL,         -- WardenCheckType
  data    VARBINARY(24)    DEFAULT NULL,     -- PAGE_CHECK / DRIVER_CHECK
  result  VARBINARY(24)    DEFAULT NULL,     -- MEM_CHECK / MPQ_CHECK expected bytes (SHA1 for MPQ)
  address INT UNSIGNED     DEFAULT 0,        -- MEM / PAGE / PROC
  length  TINYINT UNSIGNED DEFAULT 0,        -- MEM / PAGE / PROC
  str     TEXT             DEFAULT NULL,     -- LUA / MPQ / DRIVER / MODULE: name or Lua snippet (max 170 chars for LUA)
  comment VARCHAR(255)     DEFAULT 'Undocumented Check'
);

CREATE TABLE warden_action (
  wardenId SMALLINT UNSIGNED PRIMARY KEY,    -- references warden_checks.id
  action   TINYINT UNSIGNED NOT NULL         -- WardenActions (0=LOG, 1=KICK, 2=BAN)
);
```

Queries (raw, not prepared statements):

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT MAX(id) FROM warden_checks` | Sizing the `_checks` vector | world |
| `SELECT id, type, data, result, address, length, str, comment FROM warden_checks ORDER BY id ASC` | Bulk load all checks | world |
| `SELECT wardenId, action FROM warden_action` | Action overrides | character |

No DB2/DBC stores. Warden is purely server-config-driven.

---

## 7. Wire-protocol packets (if any)

Outer (world-protocol) opcodes:

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_WARDEN3_DATA` (0x35ed) | client → server | `WorldSession::HandleWardenData` → forwards bytes to `Warden::HandleData` |
| `SMSG_WARDEN3_DATA` (0x2577) | server → client | All `Warden::SendXxx` methods (`SendModuleToClient`, `RequestModule`, `RequestHash`, `RequestChecks`, `InitializeModule`) wrap the encrypted payload in an `SMSG_WARDEN3_DATA` |
| `SMSG_WARDEN3_DISABLED` (0x2823) | server → client | (sent if `CONFIG_WARDEN_ENABLED = false` so client doesn't sit waiting; verify the exact sender — possibly in `WorldSession::HandleAuthSession` after handshake) |
| `SMSG_WARDEN3_ENABLED` (0x2822) | server → client | (sent if Warden becomes available; companion to Disabled) |

Inner (within `WARDEN3_DATA` payload) opcodes — defined in `enum WardenOpcodes`:

| Opcode (inner) | Direction | Purpose |
|---|---|---|
| `WARDEN_CMSG_MODULE_MISSING = 0` | C→S inside | Client says "I don't have this module cached, send it" |
| `WARDEN_CMSG_MODULE_OK = 1` | C→S inside | "I have the module loaded successfully" |
| `WARDEN_CMSG_CHEAT_CHECKS_RESULT = 2` | C→S inside | Response to a CHEAT_CHECKS_REQUEST batch |
| `WARDEN_CMSG_MEM_CHECKS_RESULT = 3` | C→S inside | Only sent if MEM_CHECK bytes don't match (otherwise embedded in CHEAT_CHECKS_RESULT) — TC marks this NYI/never reached in practice |
| `WARDEN_CMSG_HASH_RESULT = 4` | C→S inside | SHA1 response to HASH_REQUEST |
| `WARDEN_CMSG_MODULE_FAILED = 5` | C→S inside | "I tried to load and it failed" — TC NYI / logs only |
| `WARDEN_SMSG_MODULE_USE = 0` | S→C inside | "Use this module" — `WardenModuleUse{Command, ModuleId[16], ModuleKey[16], Size: u32}` |
| `WARDEN_SMSG_MODULE_CACHE = 1` | S→C inside | "Here's a chunk of the module" — `WardenModuleTransfer{Command, DataSize: u16, Data[500]}` |
| `WARDEN_SMSG_CHEAT_CHECKS_REQUEST = 2` | S→C inside | "Run these checks" — variable-length, see `WardenWin::RequestChecks` for layout |
| `WARDEN_SMSG_MODULE_INITIALIZE = 3` | S→C inside | "Initialize the module with these function offsets" — `WardenInitModuleRequest` (Win-specific) |
| `WARDEN_SMSG_MEM_CHECKS_REQUEST = 4` | S→C inside | NYI / unused (3.4.3 sends mem checks inside CHEAT_CHECKS_REQUEST) |
| `WARDEN_SMSG_HASH_REQUEST = 5` | S→C inside | "Hash this seed and tell me what you get" — `WardenHashRequest{Command, Seed[16]}` |

Wire layout of `CHEAT_CHECKS_REQUEST` (Win, the most complex):

```
u8 opcode = WARDEN_SMSG_CHEAT_CHECKS_REQUEST                 // 0x02
[strings block]
  for each LUA_EVAL_CHECK / non-empty-str check:
    u8 length
    bytes content       // LUA: prefix + str + midfix + idstr + postfix; others: just str
[separator]
u8 0x00                                                      // before TIMING
u8 (TIMING_CHECK ^ xorByte)                                  // 87 ^ _inputKey[0]
[per-check entries]
u8 (check.Type ^ xorByte)
   MEM_CHECK:    u8 0x00, u32 Address, u8 Length
   PAGE_A/B:     bytes Data, u32 Address, u8 Length
   MPQ_CHECK:    u8 stringIndex (1..n)
   LUA_EVAL:     u8 stringIndex (1..n)
   DRIVER:       bytes Data, u8 stringIndex (1..n)
   MODULE_CHECK: 4 random bytes (seed) + 20 bytes HMAC_SHA1(seed, check.Str)
   PROC_CHECK:   (commented out — never implemented)
[trailer]
u8 xorByte                                                   // = _inputKey[0]
```

`HandleCheckResult` parses:

```
u16 length                                                   // == buf.size() - rpos
u32 checksum                                                 // 5×u32 XOR-fold of SHA1(remaining)
[TIMING_CHECK]
  u8 result (0 = fail, ! = ok)
  u32 newClientTicks (used to calc round-trip)
[per-check from same shuffled list as in request]
  MEM_CHECK:    u8 status (0 = ok), if ok then bytes (length = expected.size())
  PAGE_A/B/DRIVER/MODULE: u8 status (0xE9 = ok)
  LUA_EVAL:     u8 result; if result == 0, u8 string_len + bytes (discarded server-side)
  MPQ_CHECK:    u8 status; if ok then 20 bytes SHA1
```

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-constants/src/opcodes.rs` | `file` | 1 | 1642 | `exists_active` | file exists |
| `crates/wow-world/src/session.rs` | `file` | 1 | 3138 | `exists_active` | file exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-constants/src/opcodes.rs:671` — `Warden3Data = 0x35ed` (CMSG)
- `crates/wow-constants/src/opcodes.rs:1619-1621` — `Warden3Data = 0x2577` (SMSG), `Warden3Disabled = 0x2823`, `Warden3Enabled = 0x2822`
- **No other Warden code anywhere.** No `Warden` struct, no `WardenWin/WardenMac` variants, no `WardenCheckMgr`, no module binary, no SQL schema, no SmartEnum-equivalent scaffolding, no per-OS distinction in client detection, no inner-opcode enum, no inner packet structs.
- The world-server dispatcher (`crates/wow-world/src/session.rs`) has **no match arm** for `ClientOpcodes::Warden3Data`. Incoming Warden packets currently land in the unknown-opcode default branch.

**What's implemented:** Three opcode integers.

**What's missing vs C++ (everything):**
- Base `Warden` struct + RC4 input/output streams + module state machine.
- `WardenWin` variant + the 6 hardcoded `WoW.exe` function offsets + the `WardenInitModuleRequest` 3-part init packet + Lua eval prefix/midfix/postfix strings.
- `WardenMac` variant + the hardcoded Mac seed + the 4-int XOR/sub/mul key-derivation transformation.
- `WardenCheckMgr` singleton + the per-category check pool with rolling iterator + round-robin shuffle.
- `WardenCheck` struct + `WardenCheckResult` (expected MEM/MPQ bytes).
- The 10 `WardenCheckType` constants (87, 113, 126, 139, 152, 178, 191, 217, 243).
- The 3 `WardenCheckCategory` enum + `GetWardenCheckCategory(type)` const-fn.
- The 3 `WardenActions` (LOG/KICK/BAN) + `ApplyPenalty(check)` dispatcher.
- `WorldSession::HandleWardenData` glue → forward to `Warden::HandleData`.
- `Warden::Update(diff)` per-tick — needs to be wired into the session-update loop.
- `Warden::HandleData(buf)` — RC4 decrypt + inner opcode dispatch.
- `MakeModuleForClient` (MD5 over compressed data), `SendModuleToClient` (chunked 500-byte send), `RequestModule`, `RequestHash`.
- `IsValidCheckSum` / `BuildChecksum` (5×u32 XOR-fold of SHA1).
- `RequestChecks` — the most complex method (see §7 wire layout) with per-check-type packing + XOR-byte obfuscation + 450-byte size cap.
- `HandleCheckResult` — the matching parser.
- `HandleHashResult` — verify + key rotation to `Module.ClientKeySeed` / `Module.ServerKeySeed`.
- `ProcessLuaCheckResponse` — `_TW\t<id>` token handling from the addon channel.
- `WardenCheckMgr::LoadWardenChecks` (raw `SELECT id, type, data, result, address, length, str, comment FROM warden_checks ORDER BY id ASC` against world DB) + `LoadWardenOverrides` (against character DB).
- The 9 config keys (`CONFIG_WARDEN_ENABLED`, `..._CLIENT_RESPONSE_DELAY`, `..._CLIENT_CHECK_HOLDOFF`, `..._CLIENT_FAIL_ACTION`, `..._CLIENT_BAN_DURATION`, `..._NUM_INJECT_CHECKS`, `..._NUM_LUA_CHECKS`, `..._NUM_CLIENT_MOD_CHECKS`).
- The two SQL tables (`world.warden_checks`, `characters.warden_action`).
- **The two embedded Warden module binaries** (`WardenModuleWin.h` 1239 lines, `WardenModuleMac.h` 613 lines) — these are 18+12 KB of RC4-encrypted bytes. Vendoring them into Rust is a license-grey-area act (Blizzard never authorized redistribution, but TrinityCore has done so since 2009 without challenge).
- Per-OS client detection — currently the server learns the client OS from `CMSG_AUTH_SESSION` build/locale fields; the Rust port may not parse that correctly.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- A 3.4.3 client expects either `SMSG_WARDEN3_ENABLED` or `SMSG_WARDEN3_DISABLED` after auth. Sending neither may hang the client at the loading screen. **Verify what RustyCore currently sends after `CMSG_AUTH_SESSION` — if neither, add an explicit `Warden3Disabled` send.**
- The opcode constant `Warden3Data` for SMSG (0x2577) is not currently referenced anywhere in the codebase outside the opcode table — easy to typo and not notice.
- Anyone enabling Warden mid-session would also need to reset session ARC4 state — verify against the C++ flow if this is even reachable.

**Tests existing:** 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#WARDEN.WBS.001** Partir y cerrar la migracion auditada de `game/Warden/Modules/WardenModuleMac.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 613 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.002** Partir y cerrar la migracion auditada de `game/Warden/Modules/WardenModuleWin.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1239 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.003** Cerrar la migracion auditada de `game/Warden/Warden.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Warden.cpp`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.004** Cerrar la migracion auditada de `game/Warden/Warden.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Warden.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.005** Cerrar la migracion auditada de `game/Warden/WardenCheckMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenCheckMgr.cpp`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.006** Cerrar la migracion auditada de `game/Warden/WardenCheckMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenCheckMgr.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.007** Cerrar la migracion auditada de `game/Warden/WardenMac.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenMac.cpp`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.008** Cerrar la migracion auditada de `game/Warden/WardenMac.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenMac.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.009** Partir y cerrar la migracion auditada de `game/Warden/WardenWin.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 560 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.010** Cerrar la migracion auditada de `game/Warden/WardenWin.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.h`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#WARDEN.WBS.011** Cerrar la migracion auditada de `game/Warden/enuminfo_WardenCheckMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/enuminfo_WardenCheckMgr.cpp`
  Rust target: `crates/wow-constants/src/opcodes.rs`, `crates/wow-world/src/session.rs`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

**Strong recommendation: skip Warden until everything else is working.** Warden is "bonus" anti-cheat; a 3.4.3 server without it functions identically to a server with it from the client's perspective (provided we send `SMSG_WARDEN3_DISABLED` so the client knows to skip the check). Warden adds value only against players running modified clients, which on a private server is a low priority compared to the gameplay backlog.

- [ ] **#WARDEN.1** **(MINIMAL — DO THIS FIRST)** Send `SMSG_WARDEN3_DISABLED` (opcode 0x2823, empty body) immediately after `SMSG_AUTH_RESPONSE` so the client knows Warden is off and doesn't wait. Add a unit test that the byte sequence matches a known-good capture. (L)
- [ ] **#WARDEN.2** Define enums in `crates/wow-constants/src/warden.rs`: `WardenActions` (u8), `WardenCheckCategory` (u8), `WardenCheckType` (u8 with the 10 non-sequential values), `WardenOpcodes` (the 6 CMSG + 6 SMSG inner opcodes). Plus constants `WARDEN_MAX_LUA_CHECK_LENGTH = 170`, `MODULE_CHUNK_SIZE = 500`, `CHECK_PACKET_MAX = 450`. (M)
- [ ] **#WARDEN.3** Add a feature gate `wow-world/Cargo.toml: warden = []` so the entire Warden codepath is `#[cfg(feature = "warden")]`. Default = off. (L)
- [ ] **#WARDEN.4** Define `WardenCheck` struct + `WardenCheckResult = Vec<u8>` in `crates/wow-world/src/warden/check.rs`. (L)
- [ ] **#WARDEN.5** SQL migrations for `warden_checks` (world DB) and `warden_action` (character DB). (L)
- [ ] **#WARDEN.6** Implement `WardenCheckMgr` singleton in `crates/wow-world/src/warden/check_mgr.rs` — `load_warden_checks` (raw SELECT against world DB) + `load_warden_overrides` (raw SELECT against character DB) + `get_check_data(id)` / `get_check_result(id)` / `get_available_checks(category)`. Validate: skip unknown types; skip LUA checks with id > 9999 or string len > 170. Init `id_str` from `format!("{:04}", id)` for LUA checks. (H)
- [ ] **#WARDEN.7** Define base `Warden` trait (or struct with virtuals via dispatch enum) in `crates/wow-world/src/warden/warden.rs` with `_session_id`, `_input_key/_output_key/_seed: [u8; 16]`, `_input_crypto/_output_crypto: SArc4` (already in `crates/wow-crypto/src/sarc4.rs`), `_check_timer/_client_response_timer: u32`, `_data_sent/_initialized: bool`, `_module: Option<ClientWardenModule>`. (M)
- [ ] **#WARDEN.8** Implement `Warden::update(diff)`, `decrypt_data` / `encrypt_data` (just RC4 inplace), `is_valid_check_sum` / `build_checksum` (5×u32 XOR-fold of SHA1 — careful: read the `keyData` union pattern in C++ as `[u8; 20] -> [u32; 5]` LE). (M)
- [ ] **#WARDEN.9** Implement wire-packed structs `WardenModuleUse` (37 bytes), `WardenModuleTransfer` (503 bytes), `WardenHashRequest` (17 bytes), `WardenInitModuleRequest` (~56 bytes Win-only). Use `#[repr(C, packed)]` and unit-test sizes match `static_assert`s. (M)
- [ ] **#WARDEN.10** Implement `Warden::make_module_for_client` (MD5 over compressed bytes via `crates/wow-crypto`), `send_module_to_client` (chunked 500-byte loop), `request_module` (37-byte `WardenModuleUse`). All encrypt with `_output_crypto` then wrap in `SMSG_WARDEN3_DATA`. (M)
- [ ] **#WARDEN.11** Implement `Warden::handle_data(buf)` — RC4 decrypt + inner opcode dispatch on `WardenOpcodes` (MODULE_MISSING/OK/CHEAT_CHECKS_RESULT/MEM_CHECKS_RESULT/HASH_RESULT/MODULE_FAILED). (M)
- [ ] **#WARDEN.12** Implement `Warden::apply_penalty(check) -> &'static str` — Kick / Ban / Log dispatch. Wire to existing `Session::kick` and add a new `World::ban_account(account_name, duration_secs, reason, gm)` if not present. (M)
- [ ] **#WARDEN.13** Implement `Warden::process_lua_check_response(msg)` — parse `_TW\t<id>` token, look up check by id, if `LUA_EVAL_CHECK` apply penalty. Wire from `crates/wow-world/src/handlers/chat.rs` addon-message handler. (M)
- [ ] **#WARDEN.14** Implement `WardenWin` in `crates/wow-world/src/warden/win.rs` — `init(session, k)` (SessionKeyGenerator-style derivation from BNet K), `initialize_module_for_client`, `initialize_module` (the 3-part init request with the 6 hardcoded `WoW.exe` function offsets), `request_hash`, `handle_hash_result` (verify + rotate keys), `request_checks` (the big one — XL), `handle_check_result` (the matching parser — XL). (XL — split as: scaffold; init/hash; request_checks per-type packing; handle_check_result per-type parsing)
- [ ] **#WARDEN.15** Implement `WardenMac` in `crates/wow-world/src/warden/mac.rs` — `init` with hardcoded seed, `handle_hash_result` with the 4-int XOR/sub/mul transformation (`0xDEADBEEF`, `-0x35014542`, `+0x5313F22`, `*0x1337F00D`). (H)
- [ ] **#WARDEN.16** Vendor `WardenModuleWin.h` and `WardenModuleMac.h` content into `crates/wow-world/src/warden/modules/{win,mac}.rs` as `static MODULE_DATA: &[u8] = ...`, `static MODULE_KEY: [u8; 16] = ...`, `static MODULE_SEED: [u8; 16] = ...`, `static CLIENT_KEY_SEED: [u8; 16] = ...`, `static SERVER_KEY_SEED: [u8; 16] = ...`, `static CLIENT_KEY_SEED_HASH: [u8; 20] = ...`. **License-grey-area decision needed** (see §11). (M, mostly clerical once a precedent is set) (M)
- [ ] **#WARDEN.17** Add the 9 config keys to `crates/wow-config/`: `WardenEnabled` (default false), `WardenClientResponseDelay` (default 600), `WardenClientCheckHoldoff` (default 30), `WardenClientFailAction` (default 0=LOG), `WardenClientBanDuration` (default 86400), `WardenNumInjectChecks` (default 9), `WardenNumLuaChecks` (default 1), `WardenNumClientModChecks` (default 1). (L)
- [ ] **#WARDEN.18** Wire `WorldSession::handle_warden_data(packet)` into the dispatcher in `crates/wow-world/src/session.rs` for `ClientOpcodes::Warden3Data`. (L)
- [ ] **#WARDEN.19** Wire `Warden::update(diff)` into the session tick loop. (L)
- [ ] **#WARDEN.20** Detect client OS from `CMSG_AUTH_SESSION` (the `Build` field encodes platform + locale via the 4-char tags `Wn64` / `Mc64` / `Wn32`); construct `WardenWin` or `WardenMac` accordingly. (M)
- [ ] **#WARDEN.21** Construct `Warden` after auth completes in `crates/wow-world/src/handlers/auth.rs` (or wherever auth-session result is finalized). Pass the BNet `K` (session key). (M)
- [ ] **#WARDEN.22** Documentation cross-link: `warden.md` ↔ `crypto.md` (RC4/SHA1/HMAC dependency) ↔ `handlers.md` (chat addon message intercept) ↔ `auth.md` (session key K availability ordering) (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#WARDEN.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 11 files / 3690 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp`. Rust target: `workspace / target pending`. | `cargo test --workspace` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#WARDEN.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 11 files / 3690 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp`. Rust target: `workspace / target pending`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#WARDEN.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 11 files / 3690 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp`. Rust target: `workspace / target pending`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#WARDEN.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 11 files / 3690 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp`. Rust target: `workspace / target pending`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Warden::build_checksum(known_input)` matches the C++ output for a fixture input (5×u32 XOR-fold of SHA1).
- [ ] Test: `WardenInitModuleRequest` packed size is exactly 56 bytes and field offsets match the `static_assert` in `WardenWin.h`.
- [ ] Test: `WardenModuleUse` serialized size is exactly 37 bytes (1+16+16+4).
- [ ] Test: `WardenModuleTransfer` serialized size is exactly 503 bytes (1+2+500).
- [ ] Test: `WardenHashRequest` serialized size is exactly 17 bytes.
- [ ] Test: `MAKE_UNIT_ACTION_BUTTON`-style RC4 encrypt/decrypt round-trip for a known key/seed/payload triplet.
- [ ] Test: `WardenWin::request_checks` produces a byte-identical packet to a fixture for a deterministic check pool of 3 known checks.
- [ ] Test: `WardenWin::handle_check_result` correctly identifies a failed MEM_CHECK (response bytes != expected) and triggers `apply_penalty`.
- [ ] Test: `WardenMac::handle_hash_result` 4-int transformation produces the expected SHA1 input bytes for the constant `mod_seed`.
- [ ] Test: `WardenCheckMgr::load_warden_checks` skips a row with unknown type (e.g. 200) and a LUA row with `len(str) > 170`.
- [ ] Test: `WardenCheckMgr::load_warden_overrides` rejects action > 2.
- [ ] Test: `process_lua_check_response("_TW\t1234")` looks up check 1234, applies penalty if it's a LUA_EVAL_CHECK; rejects "_TW\tnotanumber" with bogus-response penalty.
- [ ] Test: `MIN_WARDEN.1` regression — when Warden is disabled by config, sending `SMSG_WARDEN3_DISABLED` after auth produces the expected 4-byte (header) packet on the wire.
- [ ] Test: 5×u32 XOR-fold checksum bit-byte-order verification (treat SHA1 output as LE u32 array — verify against a known TC capture).

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 11 files / 3690 lines; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp` | **No crate exists.** The opcodes `Warden3Data` (CMSG 0x35ed, SMSG 0x2577), `Warden3Disabled` (SMSG 0x2823), `Warden3Enabled` (SMSG 0x2822) are present in `crates/wow-constants/src/opcodes.rs` (lines 671, 1619-1621); nothing else exists. No handler arm in `crates/wow-world/src/session.rs`. No `Warden` struct, no `WardenCheckMgr`, no module binary, no DB schema, no SQL loader, no SmartEnum scaffolding, no per-OS variant. **0 lines of Warden code.** \| ❌ not started — only opcode constants exist (3 of them). All ~3690 lines of C++ Warden code have no Rust counterpart. The Trinity Warden module binaries (`WardenModuleWin.h` / `WardenModuleMac.h` — large embedded RC4-encrypted blobs) are not vendored in the Rust repo either. |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#WARDEN.DIV.001` | _none generated_ | 11 C++ files / 3690 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleWin.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/Modules/WardenModuleMac.h`, `/home/server/woltk-trinity-legacy/src/server/game/Warden/WardenWin.cpp` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

- **`WARDEN_CMSG_MEM_CHECKS_RESULT = 3` is essentially never received** — TC marks it `NYI` with a debug log. The mem checks are returned inside `CHEAT_CHECKS_RESULT` (opcode 2). Don't waste time implementing the dedicated mem-checks path.
- **The 4-int Mac transformation `keyIn[0] ^= 0xDEADBEEFu; keyIn[1] -= 0x35014542u; keyIn[2] += 0x5313F22u; keyIn[3] *= 0x1337F00Du;` operates on `int` (signed 32-bit)** — wrapping arithmetic is required. In Rust use `u32::wrapping_sub`, `wrapping_add`, `wrapping_mul`, or `i32` with the casts. The `keyOut[i]` calc uses the **original** `keyIn[i]` values (saved before mutation), not the post-mutation ones — re-read the C++ carefully.
- **Per-OS wire-format differences are subtle.** `WardenInitModuleRequest` (Win, 56 bytes) is **not sent** by `WardenMac` — its `InitializeModule` is empty because the Mac client self-initializes. If you copy the Windows codepath wholesale into the Mac variant you'll send a packet the Mac client doesn't expect and crash it.
- **The Windows function offsets are hardcoded for a specific `WoW.exe` build.** `0x00024F80 + 0x00400000 = SFileOpenFile` is correct for the original WoLK client (3.3.5a build 12340). The 3.4.3.54261 client may have different offsets; if so, the constants need updating. **This is the #1 thing that breaks Warden when porting between client versions.** Capture a real packet from the running 3.4.3.54261 client and confirm.
- **Five-u32 XOR-fold of SHA1 is byte-order-sensitive.** TC reads SHA1 as `union { u8 bytes[20]; u32 ints[5]; }` — on little-endian x86 this works fine; on big-endian (which we don't care about for x86 servers) it would be wrong. In Rust, do `u32::from_le_bytes` on each 4-byte chunk.
- **`Module.Seed` is the seed sent to the client in `WARDEN_SMSG_HASH_REQUEST`. It's NOT a random seed — it's a fixed 16-byte sequence baked into the module binary.** The client computes SHA1 of (seed XOR'd through some module-internal state) and returns the digest; the server compares to `Module.ClientKeySeedHash` (also baked into the module). On match, both server and client rotate to `Module.ClientKeySeed` / `Module.ServerKeySeed` (also baked) for the rest of the session. **The "session key derivation" you might assume is happening is actually a key swap to pre-baked constants.** This is why the same WoW.exe + same Module → same keys every session — Warden does NOT depend on the per-session BNet K for its packet crypto, only the BNet K for the **initial** RC4 keys before module-init is complete.
- **The `_inputKey[0]` byte is used as a per-check XOR mask in `RequestChecks`.** The `xorByte = _inputKey[0]` shows up at the start AND end of the cheat-checks-request packet (start is the TIMING_CHECK obfuscation, end is the trailer). **Read the bytes correctly:** `buff << uint8(TIMING_CHECK ^ xorByte)` then later `buff << uint8(check.Type ^ xorByte)`, and finally `buff << uint8(xorByte)` (the bare key byte) at the end as a trailer/terminator. Many TC ports get this off-by-one because the trailer `xorByte` looks vestigial.
- **Lua eval check delivery is split across two packet protocols.** The check itself goes out via `SMSG_WARDEN3_DATA` → `WARDEN_SMSG_CHEAT_CHECKS_REQUEST`; but the **response** comes back via `SMSG_MESSAGECHAT` (addon channel) prefixed with `_TW\t`. Don't try to parse Lua results in `HandleCheckResult` — TC's `HandleCheckResult` for `LUA_EVAL_CHECK` just reads a dummy result byte to advance the offset, and the actual check happens in `ProcessLuaCheckResponse`. The hex `\t` (0x09) is a literal tab character in the prefix.
- **`HandleCheckResult` decoding order matches the order the checks were sent in `RequestChecks`** — the checks are randomly shuffled, but both functions iterate the SAME shuffled `_currentChecks` vector. Don't shuffle a copy in one and the original in the other.
- **TC's `Trinity::Containers::EraseIf` is used to trim `_currentChecks` to <450 bytes** based on per-check serialized size; mirror that exact size logic (`GetCheckPacketBaseSize` switch on type) or you'll send packets the client truncates.
- **`Warden::Update` implements a watchdog**: if `_dataSent` and the client doesn't respond within `CONFIG_WARDEN_CLIENT_RESPONSE_DELAY` (default 600s), kick the player. Be careful not to trigger this for laggy-but-honest players on 3G; a longer timeout is fine.
- **Vendoring the embedded module binaries (`WardenModuleWin.h` / `WardenModuleMac.h`) is a license question.** TrinityCore has shipped them publicly since 2009 with no public Blizzard pushback, but Blizzard never authorized redistribution. RustyCore is a private 3.4.3 project; the safe play is (a) skip Warden entirely, or (b) require operators to fetch the modules from a captured client and place them in a build-time path the Rust crate `include_bytes!`-imports (similar to how some TrinityCore forks now do `tools/extract_warden`).
- **For testing without real client:** TC has a `DEBUG_ForceSpecificChecks` debug command that lets you queue specific check IDs for the next request batch. Useful for validating wire format against a fixture without waiting for the random shuffle to land on the check you want.
- **The `enuminfo_WardenCheckMgr.cpp` (169 lines) is auto-generated by Trinity's enumutils tool from `// EnumUtils: DESCRIBE THIS` annotations.** Don't try to port it — Rust has `strum::EnumIter` / `strum::Display` which provide equivalent functionality directly via derive macros.
- **The `_TW` Lua eval prefix structure is precisely engineered to fit in 255 bytes**: `static_assert((sizeof(_luaEvalPrefix)-1 + sizeof(_luaEvalMidfix)-1 + sizeof(_luaEvalPostfix)-1 + WARDEN_MAX_LUA_CHECK_LENGTH) == 255)`. The 255 is the max length of an addon-channel message; if WARDEN_MAX_LUA_CHECK_LENGTH (170) is changed, the static assert catches it. Mirror the assert in Rust.
- **Most operators run with `CONFIG_WARDEN_ENABLED = false`** — even on retail-style servers Warden adds operational complexity (false positives, banned legitimate players using AHK for accessibility, etc.). For a 3.4.3 dev/private server it's fine to defer this indefinitely.
- **Order of operations matters in `Init`:** `_seed` must be set before `MakeModuleForClient`, and `_inputCrypto/_outputCrypto` must be initialized with `_inputKey/_outputKey` before `RequestModule` (since `RequestModule` encrypts). The C++ Init does these in the right order; copy that order exactly.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Warden` (abstract) | `pub trait Warden { fn init(&mut self, session: &mut WorldSession, k: &SessionKey); fn initialize_module(&mut self); fn request_hash(&mut self); fn handle_hash_result(&mut self, buf: &mut Bytes); fn handle_check_result(&mut self, buf: &mut Bytes); fn initialize_module_for_client(&mut self, module: &mut ClientWardenModule); fn request_checks(&mut self); }` | Either a trait + dispatch enum, or just `enum WardenImpl { Win(WardenWin), Mac(WardenMac) }` with explicit dispatch |
| `class WardenWin` | `pub struct WardenWin { base: WardenBase, server_ticks: u32, checks: [(Vec<u16>, usize); 3], current_checks: Vec<u16> }` | `usize` instead of iterator — Rust safe-iterator semantics |
| `class WardenMac` | `pub struct WardenMac { base: WardenBase }` | — |
| `class WardenCheckMgr` (singleton) | `pub struct WardenCheckMgr { checks: Vec<WardenCheck>, check_results: HashMap<u16, Vec<u8>>, pools: [Vec<u16>; 3] }` + `static WARDEN_CHECK_MGR: OnceCell<RwLock<WardenCheckMgr>>` | Or `LazyLock` in Rust 1.80+ |
| `struct WardenCheck` | `pub struct WardenCheck { pub check_id: u16, pub type_: WardenCheckType, pub data: Vec<u8>, pub address: u32, pub length: u8, pub str_: String, pub comment: String, pub id_str: [u8; 4], pub action: WardenActions }` | Rename `type` and `str` to avoid keyword collisions |
| `Trinity::Crypto::ARC4` | `wow_crypto::SArc4` | Already exists in `crates/wow-crypto/src/sarc4.rs` |
| `Trinity::Crypto::SHA1::GetDigestOf(data, len)` | `Sha1::digest(data)` from `sha-1` crate (or `wow-crypto`'s wrapper) | — |
| `Trinity::Crypto::HMAC_SHA1::GetDigestOf(seed, str)` | `hmac::Hmac::<Sha1>::new_from_slice(seed)?.chain_update(str.as_bytes()).finalize()` | — |
| `Trinity::Crypto::MD5::GetDigestOf` | `md-5` crate `Md5::digest` | — |
| `enum WardenActions : u8` | `#[repr(u8)] pub enum WardenActions { Log = 0, Kick = 1, Ban = 2 }` | — |
| `enum WardenCheckCategory : u8` | `#[repr(u8)] pub enum WardenCheckCategory { Inject = 0, Lua = 1, Modded = 2, NumCategories = 3 }` | — |
| `enum WardenCheckType : u8` | `#[repr(u8)] pub enum WardenCheckType { None = 0, Timing = 87, Driver = 113, Proc = 126, LuaEval = 139, Mpq = 152, PageA = 178, PageB = 191, Module = 217, Mem = 243 }` | Non-sequential discriminants — Rust supports them on `repr(u8)` enums |
| `enum WardenOpcodes` (inner) | `#[repr(u8)] pub enum WardenInnerOpcodeC2S { ModuleMissing=0, ModuleOk=1, CheatChecksResult=2, MemChecksResult=3, HashResult=4, ModuleFailed=5 }` plus `WardenInnerOpcodeS2C { ModuleUse=0, ModuleCache=1, CheatChecksRequest=2, ModuleInitialize=3, MemChecksRequest=4, HashRequest=5 }` | Two enums to avoid ambiguity (C++ has them in the same enum but `MODULE_USE` (S2C, 0) and `MODULE_MISSING` (C2S, 0) collide) |
| `struct WardenModuleUse` (packed) | `#[repr(C, packed)] pub struct WardenModuleUse { command: u8, module_id: [u8; 16], module_key: [u8; 16], size: u32 }` | Add `static_assert!(size_of::<WardenModuleUse>() == 37)` via `const _: () = assert!(...)` |
| `struct WardenModuleTransfer` | `#[repr(C, packed)] pub struct WardenModuleTransfer { command: u8, data_size: u16, data: [u8; 500] }` | — |
| `struct WardenHashRequest` | `#[repr(C, packed)] pub struct WardenHashRequest { command: u8, seed: [u8; 16] }` | — |
| `struct WardenInitModuleRequest` | `#[repr(C, packed)] pub struct WardenInitModuleRequest { ... }` | 56 bytes — verify via `assert_eq!(size_of::<...>(), 56)` |
| `struct ClientWardenModule { Id, Key, CompressedData, CompressedSize }` | `pub struct ClientWardenModule { pub id: [u8; 16], pub key: [u8; 16], pub compressed_data: &'static [u8] }` | `&'static [u8]` ties it to the embedded module bytes |
| `IsValidCheckSum(checksum, data, length)` | `pub fn is_valid_check_sum(checksum: u32, data: &[u8]) -> bool { build_checksum(data) == checksum }` | — |
| `BuildChecksum(data, length)` | `pub fn build_checksum(data: &[u8]) -> u32 { let h = Sha1::digest(data); let mut acc = 0u32; for chunk in h.chunks(4) { acc ^= u32::from_le_bytes(chunk.try_into().unwrap()); } acc }` | — |
| `SessionKeyGenerator<SHA1>(K).Generate(out, 16)` | Custom `SessionKeyGenerator` in `crates/wow-crypto` (the `K`-derived rolling SHA1 from the BNet auth handshake) | This generator is used elsewhere too (world-packet header crypto); reuse if already present |
| `WorldDatabase.Query("SELECT id, type, data, result, address, length, str, comment FROM warden_checks ORDER BY id ASC")` | `sqlx::query!("...")` against the world DB pool | sqlx — keep as raw query (not prepared) since it runs once at startup |
| `sWorld->BanAccount(BAN_ACCOUNT, accountName, duration, reason, "Server")` | `world.ban_account(BanType::Account, account_name, duration_secs, reason, "Server").await?` | — |
| `Trinity::Containers::RandomShuffle(vec)` | `vec.shuffle(&mut rand::thread_rng())` | `rand::SliceRandom::shuffle` |
| `Trinity::Containers::EraseIf(vec, pred)` | `vec.retain(|x| !pred(x))` | Note inverted condition |
| `EnumUtils::Iterate<E>()` | `<E as strum::IntoEnumIterator>::iter()` (with `#[derive(EnumIter)]`) | — |
| `EnumUtils::ToConstant(E)` / `ToTitle(E)` | `strum::EnumString` + `strum::Display` (with `#[strum(serialize="LOG")]` annotations) | — |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

**Verdict: ❌ confirmed — exactly 3 opcode constants, zero implementation.**

```
$ grep -rn -i "warden" crates/ --include='*.rs'
crates/wow-constants/src/opcodes.rs:671:    Warden3Data = 0x35ed,
crates/wow-constants/src/opcodes.rs:1619:    Warden3Data = 0x2577,
crates/wow-constants/src/opcodes.rs:1620:    Warden3Disabled = 0x2823,
crates/wow-constants/src/opcodes.rs:1621:    Warden3Enabled = 0x2822,
```

Four hits in total, but `Warden3Data` appears once on each side of the protocol (CMSG 0x35ed at line 671, SMSG 0x2577 at line 1619), so this is the §0 doc claim "3 opcode constants" verified — one CMSG + two SMSG distinct opcodes. No struct, no handler arm in `wow-world/src/session.rs`, no DB schema, no SQL loader, no `WardenCheck` / `WardenCheckMgr` / `WardenWin` / `WardenMac`, no embedded module binaries (the ~18 KB `WardenModuleWin.h` and ~12 KB `WardenModuleMac.h` blobs are not vendored).

`grep -rn -i "warden"` finds only the four opcode definitions — nothing else.

**No silent-default bug.** The Warden opcodes that arrive on CMSG `0x35ed` (`Warden3Data`) currently land on the dispatcher's "unknown opcode" fallback because no `inventory::submit!(PacketHandlerEntry { opcode: Warden3Data, ... })` registration exists. The client receives no `Warden3Enabled` (0x2822) at session start, which is exactly what an "anti-cheat off" deployment looks like — clients won't run Warden checks but won't disconnect over it either. Functionally equivalent to `CONFIG_WARDEN_ENABLED=false` in TC dist (also the default upstream).

**Recommendation:** Warden is a low-priority anti-cheat layer that the project can defer. For a private server's threat model the cost/benefit is poor: porting requires (a) RC4 + custom XOR ciphers, (b) embedding two large opaque encrypted blobs of unknown provenance, (c) per-OS variants tied to `WoW.exe` PE offsets that may drift across 3.4.3 sub-revisions, (d) a `warden_checks` SQL schema + content. Consider documenting Warden as explicit non-goal in scope unless cheating becomes an operational issue.
