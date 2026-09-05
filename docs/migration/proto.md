# Migration: proto (Battle.net RPC protobuf)

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `src/server/proto/`
> **Rust target crate(s):** `crates/wow-proto/`
> **Layer:** L1 (infrastructure — consumed by `bnet-server`, `wow-network`, `wow-handler`)
> **Status:** ⚠️ partial — 5 services implemented, 2 unported `.proto` sources (Login, RealmList) hand-rolled in handlers, club services + 8 declared services are 0% covered
> **Audited vs C++:** ⚠️ partial — wire compat verified for 5 ported services; error code coverage 7/601
> **Last updated:** 2026-05-01

---

## 1. Purpose

This module owns the **Battle.net RPC wire format** and its supporting infrastructure: protobuf message types, the abstract `ServiceBase` dispatch interface, and the catalogue of Battle.net error codes. Battle.net is the connection layer the WoW 3.4.3 client uses for authentication, game-account selection, realm listing, friends, presence, club (guild/community), reporting, voice, etc.; it is a length-prefixed binary RPC where each request carries a `Header` protobuf identifying the target service (by 32-bit hash) and method id, followed by a service-specific payload.

The C++ tree contains both the **canonical service definitions** (a couple of `.proto` files plus a large body of pre-generated `.pb.{h,cc}` artefacts whose original `.proto` sources live outside the public Trinity tree — they're Blizzard's `bgs.low` protocol surface) and the **base classes for service implementation** (`ServiceBase`, error code enum). RustyCore re-derives this surface using the `prost` crate and a small set of recovered `.proto` files, plus hand-written service hash + status code constants.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `proto/BattlenetRpcErrorCodes.h` | 671 | `prefix` |
| `proto/Client/account_service.pb.h` | 4780 | `prefix` |
| `proto/Client/account_types.pb.h` | 10692 | `prefix` |
| `proto/Client/api/client/v1/channel_id.pb.h` | 278 | `prefix` |
| `proto/Client/api/client/v2/attribute_types.pb.h` | 857 | `prefix` |
| `proto/Client/api/client/v2/report_service.pb.h` | 485 | `prefix` |
| `proto/Client/api/client/v2/report_types.pb.h` | 1065 | `prefix` |
| `proto/Client/attribute_types.pb.h` | 1069 | `prefix` |
| `proto/Client/authentication_service.pb.h` | 3109 | `prefix` |
| `proto/Client/challenge_service.pb.h` | 608 | `prefix` |
| `proto/Client/channel_types.pb.h` | 2362 | `prefix` |
| `proto/Client/club_ban.pb.h` | 738 | `prefix` |
| `proto/Client/club_core.pb.h` | 6073 | `prefix` |
| `proto/Client/club_enum.pb.h` | 290 | `prefix` |
| `proto/Client/club_invitation.pb.h` | 2267 | `prefix` |
| `proto/Client/club_listener.pb.h` | 107 | `prefix` |
| `proto/Client/club_member.pb.h` | 3438 | `prefix` |
| `proto/Client/club_member_id.pb.h` | 212 | `prefix` |
| `proto/Client/club_membership_listener.pb.h` | 1498 | `prefix` |
| `proto/Client/club_membership_service.pb.h` | 1559 | `prefix` |
| `proto/Client/club_membership_types.pb.h` | 1308 | `prefix` |
| `proto/Client/club_name_generator.pb.h` | 1015 | `prefix` |
| `proto/Client/club_notification.pb.h` | 4522 | `prefix` |
| `proto/Client/club_range_set.pb.h` | 1708 | `prefix` |
| `proto/Client/club_request.pb.h` | 14861 | `prefix` |
| `proto/Client/club_role.pb.h` | 2330 | `prefix` |
| `proto/Client/club_service.pb.h` | 199 | `prefix` |
| `proto/Client/club_stream.pb.h` | 4552 | `prefix` |
| `proto/Client/club_tag.pb.h` | 676 | `prefix` |
| `proto/Client/club_type.pb.h` | 247 | `prefix` |
| `proto/Client/club_types.pb.h` | 81 | `prefix` |
| `proto/Client/connection_service.pb.h` | 2701 | `prefix` |
| `proto/Client/content_handle_types.pb.h` | 516 | `prefix` |
| `proto/Client/embed_types.pb.h` | 1233 | `prefix` |
| `proto/Client/entity_types.pb.h` | 355 | `prefix` |
| `proto/Client/ets_types.pb.h` | 186 | `prefix` |
| `proto/Client/event_view_types.pb.h` | 409 | `prefix` |
| `proto/Client/friends_service.pb.h` | 3384 | `prefix` |
| `proto/Client/friends_types.pb.h` | 2690 | `prefix` |
| `proto/Client/game_utilities_service.pb.h` | 2085 | `prefix` |
| `proto/Client/game_utilities_types.pb.h` | 441 | `prefix` |
| `proto/Client/global_extensions/field_options.pb.h` | 2342 | `prefix` |
| `proto/Client/global_extensions/message_options.pb.h` | 192 | `prefix` |
| `proto/Client/global_extensions/method_options.pb.h` | 764 | `prefix` |
| `proto/Client/global_extensions/range.pb.h` | 444 | `prefix` |
| `proto/Client/global_extensions/register_method_types.pb.h` | 84 | `prefix` |
| `proto/Client/global_extensions/routing.pb.h` | 84 | `prefix` |
| `proto/Client/global_extensions/service_options.pb.h` | 754 | `prefix` |
| `proto/Client/invitation_types.pb.h` | 848 | `prefix` |
| `proto/Client/message_types.pb.h` | 211 | `prefix` |
| `proto/Client/notification_types.pb.h` | 1270 | `prefix` |
| `proto/Client/presence_listener.pb.h` | 472 | `prefix` |
| `proto/Client/presence_service.pb.h` | 1869 | `prefix` |
| `proto/Client/presence_types.pb.h` | 1141 | `prefix` |
| `proto/Client/profanity_filter_config.pb.h` | 408 | `prefix` |
| `proto/Client/report_service.pb.h` | 376 | `prefix` |
| `proto/Client/report_types.pb.h` | 2324 | `prefix` |
| `proto/Client/resource_service.pb.h` | 533 | `prefix` |
| `proto/Client/role_types.pb.h` | 971 | `prefix` |
| `proto/Client/rpc_config.pb.h` | 984 | `prefix` |
| `proto/Client/rpc_types.pb.h` | 2138 | `prefix` |
| `proto/Client/semantic_version.pb.h` | 311 | `prefix` |
| `proto/Client/user_manager_service.pb.h` | 1896 | `prefix` |
| `proto/Client/user_manager_types.pb.h` | 662 | `prefix` |
| `proto/Client/voice_types.pb.h` | 536 | `prefix` |
| `proto/Login/Login.pb.h` | 3351 | `prefix` |
| `proto/RealmList/RealmList.pb.h` | 2671 | `prefix` |
| `proto/ServiceBase.cpp` | 69 | `prefix` |
| `proto/ServiceBase.h` | 66 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/proto/CMakeLists.txt` | 50 | Builds `proto` target — collects all `.pb.{cc,h}` + `ServiceBase.cpp` |
| `src/server/proto/ServiceBase.h` | 66 | `class ServiceBase` — abstract dispatch superclass for every RPC service. `CallServerMethod`, `SendRequest`, `SendResponse`, logging helpers |
| `src/server/proto/ServiceBase.cpp` | 69 | Logging helpers + continuation factory |
| `src/server/proto/BattlenetRpcErrorCodes.h` | **671** | `enum BattlenetRpcErrorCode : uint32` — ~330 named error codes (`ERROR_OK=0`, `ERROR_DENIED=3`, `ERROR_GAME_ACCOUNT_BANNED`, `ERROR_WOW_SERVICES_*` family, etc.) |
| `src/server/proto/PrecompiledHeaders/protoPCH.h` | <100 | PCH stub |
| `src/server/proto/Login/Login.proto` | 84 | **Real `.proto` source.** JSON-over-HTTP login form: `LoginForm`, `SrpLoginChallenge`, `LoginResult`, `GameAccountList`, etc. |
| `src/server/proto/Login/Login.pb.{cc,h}` | (gen) | Generated artefacts |
| `src/server/proto/RealmList/RealmList.proto` | 83 | **Real `.proto` source.** Realm browser ticket and update messages: `RealmListTicketIdentity`, `ClientInformation`, `RealmEntry`, `RealmState`, `RealmListUpdates`, `RealmListServerIPAddresses`, `IPAddress` |
| `src/server/proto/RealmList/RealmList.pb.{cc,h}` | (gen) | Generated artefacts |
| `src/server/proto/Client/club_service.proto` | 293 | **Real `.proto` source.** `service ClubService` — the only RPC service whose `.proto` is checked in. Methods like `CreateClub`, `JoinClub`, `LeaveClub`, `GetClub`, `SubscribeClub`, etc. |
| `src/server/proto/Client/club_listener.proto` | 121 | **Real `.proto` source.** `service ClubListener` — server-pushed notifications mirror of ClubService |
| `src/server/proto/Client/*.pb.{cc,h}` | (gen) | **53 distinct services**, all generated from `.proto` files **not present** in this tree (Blizzard internal `bgs.low` definitions). Includes account, attribute, authentication, challenge, channel, club_*, connection, content_handle, embed, entity, ets, event_view, friends, game_utilities, invitation, message, notification, presence, profanity_filter_config, report, resource, role, rpc_config, rpc_types, semantic_version, user_manager, voice |
| `src/server/proto/Client/api/` | (gen) | `field_options.pb.{cc,h}`, `message_options.pb.{cc,h}`, `method_options.pb.{cc,h}`, `range.pb.{cc,h}`, `register_method_types.pb.{cc,h}`, `routing.pb.{cc,h}`, `service_options.pb.{cc,h}` — protobuf custom-options metadata used by the codegen |
| `src/server/proto/Client/global_extensions/` | (gen) | (additional codegen extensions) |

**The asymmetry to note:** only **4 `.proto` source files** are checked in (`Login`, `RealmList`, `club_service`, `club_listener`). The other ~50 services exist only as their pre-generated `.pb.cc` / `.pb.h` artefacts. To extend the Rust port beyond what's already there, one must either re-derive the `.proto` schema by reverse-engineering the `.pb.h` (manageable for small services, painful for the full set) or obtain Blizzard's authoritative `bgs.low` protos out-of-band.

---

## 3. Classes / Structs / Enums

### From `ServiceBase.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `ServiceBase` | abstract class | Common base for every generated `*Service` class. Holds `service_hash_`, declares `CallServerMethod` (dispatcher), `SendRequest`/`SendResponse` (transport), and logging helpers |

### From `BattlenetRpcErrorCodes.h`

| Symbol | Kind | Purpose |
|---|---|---|
| `BattlenetRpcErrorCode` | enum uint32 | ~330 named error codes covering: `ERROR_OK`, generic errors (`ERROR_INTERNAL/TIMED_OUT/DENIED/NOT_EXISTS/INVALID_ARGS/NOT_IMPLEMENTED/...`), authentication (`ERROR_AUTHENTICATION_*`), game account state (`ERROR_GAME_ACCOUNT_*`), parental controls, geographic restrictions, WoW-specific (`ERROR_WOW_SERVICES_*`: realm down, banned, account suspended, ...), club/community errors, friends errors, presence, reporting, voice, etc. |

### From `Login/Login.proto`

| Symbol | Kind | Purpose |
|---|---|---|
| `ErrorResponse` | message | Empty error placeholder |
| `FormType` | enum | `LOGIN_FORM = 1` |
| `FormInput` | message | `input_id`, `type`, `label`, `max_length` — one form field |
| `FormInputs` | message | List of form fields + SRP6 url/javascript |
| `FormInputValue` | message | Submitted value pair |
| `LoginForm` | message | Submitted form payload (`platform_id`, `program_id`, `version`, `inputs[]`) |
| `SrpLoginChallenge` | message | SRP6 challenge from server (`version`, `iterations`, `modulus`, `generator`, `hash_function`, `username`, `salt`, `public_B`) |
| `AuthenticationState` | enum | `LOGIN / LEGAL / AUTHENTICATOR / DONE` |
| `LoginResult` | message | Final auth response with state, session token, account info |
| `LoginRefreshResult` | message | Token refresh |
| `GameAccountInfo` | message | Per-game-account metadata |
| `GameAccountList` | message | List of game accounts on the BNet account |

### From `RealmList/RealmList.proto`

| Symbol | Kind | Purpose |
|---|---|---|
| `RealmListTicketIdentity` | message | `gameAccountID`, `gameAccountRegion` |
| `ClientVersion` | message | `versionMajor/Minor/Revision/Build` |
| `ClientInformation` | message | `platform`, `buildVariant`, `type`, `timeZone`, `currentTime`, `textLocale`, `audioLocale`, `versionDataBuild`, `version`, `secret`, `clientArch`, `systemVersion`, `platformType`, `systemArch` |
| `RealmListTicketClientInformation` | message | Wrapper around `ClientInformation` |
| `RealmCharacterCountEntry` | message | `wowRealmAddress` + `count` |
| `RealmCharacterCountList` | message | Repeated `RealmCharacterCountEntry` |
| `RealmEntry` | message | One realm row: address, timezone, population state, category, version, flags, name, configs |
| `RealmState` | message | Realm up/down state |
| `RealmListUpdates` | message | Repeated `RealmState` |
| `IPAddress` | message | IPv4/IPv6 address |
| `RealmIPAddressFamily` | message | One IP family |
| `RealmListServerIPAddresses` | message | Repeated `RealmIPAddressFamily` |

### From `Client/club_service.proto` and `club_listener.proto`

| Symbol | Kind | Purpose |
|---|---|---|
| `service ClubService` | service | All "client → server" club operations (CRUD on clubs, members, invitations, streams, messages, roles, bans, tags, ranges) |
| `service ClubListener` | service | Server-pushed notifications mirroring the same domain |

### Implicit (in `bgs.low` generated artefacts)

The generated `.pb.h` set defines hundreds of message types and a similar number of service methods. Spot-check from filenames:

- `account_service.pb.h` — `AccountService` (account info queries, link types, GameAccount info)
- `account_types.pb.h` — `AccountId`, `GameAccountHandle`, etc.
- `attribute_types.pb.h` — `Attribute`, `Variant` (key→value generic payload)
- `authentication_service.pb.h` / `authentication_listener` — login flow
- `challenge_service.pb.h` — captcha-like challenges (web-pin, sms, etc.)
- `channel_types.pb.h` — chat channel metadata
- `club_*` — community/club system (most widely-implemented BNet service after auth)
- `connection_service.pb.h` — `Connect`, `Bind`, `KeepAlive`, `RequestDisconnect`
- `friends_service.pb.h` / `friends_types.pb.h` — friend lists, invitations
- `game_utilities_service.pb.h` — generic `ClientRequest` / `ClientResponse` used by realm list flow + many other ad-hoc commands
- `invitation_types.pb.h` — generic invitation envelope (party, friend, club)
- `message_types.pb.h` — generic message envelope
- `notification_types.pb.h` — generic notification envelope
- `presence_service.pb.h` / `presence_listener.pb.h` — online/idle/AFK + rich presence
- `profanity_filter_config.pb.h` — chat profanity rules
- `report_service.pb.h` / `report_types.pb.h` — abuse reporting
- `resource_service.pb.h` — resource (file blob) handle lookup
- `rpc_types.pb.h` — `Header`, `ProcessId`, `NoData`, `EntityId`, `MethodOptions`
- `semantic_version.pb.h` — version triples
- `user_manager_service.pb.h` — block list, ignore list
- `voice_types.pb.h` — voice channel metadata

---

## 4. Critical public methods / functions

### `ServiceBase`

| Symbol | Purpose | Calls into |
|---|---|---|
| `virtual void CallServerMethod(uint32 token, uint32 methodId, MessageBuffer)` | Top-level dispatch. Generated subclasses override and switch on `methodId` | per-method `Handle*` virtual |
| `virtual void SendRequest(uint32 hash, uint32 method, Message*)` | Send fire-and-forget RPC | transport |
| `virtual void SendRequest(uint32 hash, uint32 method, Message*, std::function<void(MessageBuffer)>)` | Send + register continuation | transport + token registry |
| `virtual void SendResponse(uint32 hash, uint32 method, uint32 token, uint32 status)` | Send error response | transport |
| `virtual void SendResponse(uint32 hash, uint32 method, uint32 token, Message*)` | Send success response | transport |
| `CreateServerContinuation(token, methodId, methodName, outputDescriptor)` | Build callback closure that serialises the response into the right outbound envelope | logging + `SendResponse` |
| `LogCallServerMethod`, `LogCallClientMethod`, `LogUnimplementedServerMethod`, `LogInvalidMethod`, `LogFailedParsingRequest`, `LogDisallowedMethod` | Standardised logging | `TC_LOG_*` macros |
| `GetServiceHash() const -> uint32` | Identify which service this instance handles | — |

### Generated services (pattern)

Each generated `XxxService` exposes:

- `static char const* GetName()` — service name string for logging
- `static uint32 GetServiceHash()` — usually `HashFnv1a(name)`
- `void XxxMethod(MethodInputType const&, std::function<void(MethodOutputType const&)>)` — server-side handler stub
- `void XxxMethod(MethodInputType const&)` — fire-and-forget client-side caller

---

## 5. Module dependencies

**Depends on:**
- `protobuf` (Google library, `MessageLite`, `Message`)
- `MessageBuffer` (TC's network buffer wrapper, in `src/server/shared/Networking/`)
- `Define.h` (for `uint32` typedefs and `TC_PROTO_API` export macro)
- (transitively) `boost::asio` callbacks via `MessageBuffer`

**Depended on by:**
- `bnet-server` — the entire executable is a thin shell over a few `*Service` subclasses
- `worldserver` — uses `BattlenetRpcErrorCode` for some error reporting back to the BNet realm-list flow
- `WorldServerSession` (for the realm-list ticket) — uses `RealmListTicketIdentity`, `RealmListTicketClientInformation`, `ClientInformation`
- `RealmList` server (separate component in TC) — uses `RealmEntry`, `RealmState`, `RealmListServerIPAddresses`, `IPAddress`
- Login flow — uses `Login.pb.h` types, the JSON-over-HTTP form layer
- Account / Friends / Club / Presence services (all the BNet RPC handlers) — use the corresponding generated types

---

## 6. SQL / DB queries (if any)

The proto module itself issues **no** SQL. It defines the wire types used by upstream BNet RPC handlers, which then query `auth` / `bnet_account` tables; those queries live in the consumer code (Account/Friends/Club services), not here.

No DBC/DB2 stores are owned here either.

---

## 7. Wire-protocol packets (if any)

The entire module **is** the wire protocol — but for **Battle.net RPC**, not the WoW world-protocol opcodes. There are no `OPCODE_*` symbols here; instead, the unit of transmission is:

```
[ Header (varint-length-prefixed protobuf) ][ Payload (length-prefixed protobuf) ]
```

The `Header` is `bgs.protocol.Header` (in `rpc_types.pb.h`), with fields:
- `service_id` (uint8 — local) or `service_hash` (uint32 — global)
- `method_id` (uint32)
- `token` (uint32 — request/response correlation)
- `status` (uint32 — error code on responses; absent on requests)
- `size` (uint32 — payload size)

| "Opcode" | Direction | Sent/Received in |
|---|---|---|
| `service_hash = AUTHENTICATION_SERVICE` (0x0DECFC01), method 1 = `Logon` | client → server | `AuthenticationService::Logon` |
| `service_hash = CONNECTION_SERVICE` (0x65446991), method 1 = `Connect` | client → server | `ConnectionService::Connect` |
| `service_hash = CHALLENGE_SERVICE`, method 3 = `OnChallenge*` | server → client | `ChallengeService` |
| `service_hash = ACCOUNT_SERVICE` (0x62DA0891) | client → server | `AccountService` |
| `service_hash = GAME_UTILITIES_SERVICE` (0x3FC1274D), method 1 = `ProcessClientRequest` | client → server | Realm list / generic command channel |
| `service_hash = AUTHENTICATION_LISTENER` (0x71240E35) | server → client | Logon push notifications |
| `service_hash = ACCOUNT_LISTENER` (0x54DFDA17) | server → client | Account info changes |
| `service_hash = FRIENDS_SERVICE` (0xA3DDB1BD) / `FRIENDS_LISTENER` (0x6F259A13) | both | Friends |
| `service_hash = PRESENCE_SERVICE` (0xFA0796FF) / `PRESENCE_LISTENER` (0x890AB85F) | both | Presence |
| `service_hash = REPORT_SERVICE` (0x7CAF61C9) / `REPORT_SERVICE_V2` (0x3A4218FB) | client → server | Abuse reports |
| `service_hash = RESOURCES_SERVICE` (0xECBE75BA) | client → server | Blob fetch |
| `service_hash = USER_MANAGER_SERVICE` (0x3E19268A) / `USER_MANAGER_LISTENER` (0xBC872C22) | both | Block lists |
| `Login.proto` payloads | HTTP POST/JSON (separate transport) | The *Login form* leg of the auth flow — JSON over HTTPS, **not** binary RPC |
| `RealmList.proto` payloads | nested inside `GameUtilities.ClientRequest` `Attribute`s | Realm list ticket request/response |

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |
| `crates/wow-proto/Cargo.toml` | `file` | 1 | 14 | `exists_manifest` | manifest exists; not counted as active Rust source |
| `crates/wow-proto/build.rs` | `file` | 1 | 30 | `exists_active` | file exists |
| `crates/wow-proto/src/lib.rs` | `file` | 1 | 224 | `exists_active` | file exists |
| `crates/wow-proto/proto/bgs/low/pb/client` | `module_dir` | 0 | 0 | `exists_empty` | directory exists; no active Rust source lines |
| `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

The Rust port is in `crates/wow-proto`, using `prost` + `prost-build` to compile a curated subset of `.proto` files at build time.

**Files in `/home/server/rustycore`:**
- `crates/wow-proto/Cargo.toml` — `prost` dep + `prost-build` build-dep
- `crates/wow-proto/build.rs` — compiles **11 `.proto` files** under `proto/bgs/low/pb/client/`
- `crates/wow-proto/src/lib.rs` — `pub mod bgs::protocol::{authentication, connection, challenge, game_utilities, account}::v1`, hand-written `service_hash` constants, hand-written `status` constants, `RESPONSE_SERVICE_ID = 0xFE`, 6 round-trip tests
- `crates/wow-proto/proto/bgs/low/pb/client/`
  - `rpc_types.proto` — `Header`, `ProcessId`, `NoData`, etc.
  - `entity_types.proto` — `EntityId`
  - `attribute_types.proto` — `Attribute`, `Variant`
  - `content_handle_types.proto` — content handle types
  - `semantic_version.proto` — version triples
  - `authentication_service.proto` — Logon flow
  - `connection_service.proto` — Connect/Bind/Disconnect
  - `challenge_service.proto` — captcha challenges
  - `game_utilities_service.proto` — Realm list channel
  - `account_types.proto` + `account_service.proto` — account info

**What's implemented:**
- Core RPC envelope (`Header`, `ProcessId`, `NoData`, `EntityId`, `Attribute`, `Variant`)
- 5 services as Rust modules: `authentication.v1`, `connection.v1`, `challenge.v1`, `game_utilities.v1`, `account.v1`
- Service-hash constants for **15 services** (5 implemented + 10 declared but unimplemented: `FRIENDS_SERVICE/LISTENER`, `PRESENCE_SERVICE/LISTENER`, `REPORT_SERVICE/V2`, `RESOURCES_SERVICE`, `USER_MANAGER_SERVICE/LISTENER`, `AUTHENTICATION_LISTENER`, `CHALLENGE_LISTENER`, `ACCOUNT_LISTENER`)
- A minimal `status` mod with `OK`, `ERROR_INTERNAL`, `ERROR_DENIED`, `ERROR_NO_GAME_ACCOUNT`, `ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED`, `ERROR_GAME_ACCOUNT_BANNED`, `ERROR_GAME_ACCOUNT_SUSPENDED` — **only 7 of ~330** error codes from `BattlenetRpcErrorCodes.h`
- 6 unit tests covering Header round-trip, EntityId round-trip, LogonRequest encode, ConnectRequest encode, ClientRequest with Attributes, service hash constants

**What's missing vs C++:**
- **No `Login.proto` port.** The HTTP/JSON login form layer is entirely absent — `bnet-server`'s `LoginForm` / `SrpLoginChallenge` / `LoginResult` / `GameAccountList` / `AuthenticationState` types must be hand-written elsewhere or the form posts re-implemented from scratch
- **No `RealmList.proto` port.** Realm-list responses are likely hand-rolled in `bnet-server`'s realm-list handler. The 12 messages (`RealmEntry`, `RealmState`, `ClientInformation`, etc.) need to be ported to ensure binary parity with retail clients
- **No `club_service.proto` / `club_listener.proto` port.** The entire community/club system is unreachable; clients that try to query clubs will hit unimplemented method ids
- **No port of the unrecovered C++ `.pb.h` set:** account-listener, friends_service/types/listener, presence_service/types/listener, report_service/types(/V2), resource_service, user_manager_service/types/listener, channel_types, club_* (~25 files), embed_types, ets_types, event_view_types, invitation_types, message_types, notification_types, profanity_filter_config, role_types, rpc_config, voice_types. To port them you must first **recover the `.proto` schema** by reading the `.pb.h` files and reverse-engineering, or obtain Blizzard's official `bgs.low` schema
- **`BattlenetRpcErrorCodes.h` is barely ported** (~7 of ~330 codes). Many codes are needed for accurate error reporting (e.g. `ERROR_GAME_ACCOUNT_BANNED_PERMANENTLY`, `ERROR_AUTHENTICATION_*` family for login error parity)
- **No `ServiceBase` analogue.** The Rust side dispatches services ad-hoc per-service. The C++ pattern of a base class with `CallServerMethod` virtual + standard logging helpers is not replicated. This is fine if the dispatch table is small, but as more services are added, refactoring to a trait + `inventory` pattern (mirroring `wow-handler`) would help
- **No `MethodOptions` / `ServiceOptions` / `FieldOptions` / `MessageOptions` custom-options ports.** These are used by the C++ codegen to attach metadata (e.g. `(method_options).disallow_dynamic_method`) and only matter if you want full schema fidelity. Not blocking
- **`prost` does not generate service stubs** by default — it only generates messages. The Rust side has to wire dispatch by hand (currently done in `bnet-server`). C++ has the `*Service` generated classes for free
- **No re-export of Login/RealmList types from the `lib.rs`** even if hand-written, no exposed module exists yet

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The two-tier hash mapping (`service_id` byte for local registration, `service_hash` for global identification) is implemented in the Rust side — but the convention "use `service_id` for sender-known recipients, `service_hash` for the first message" needs verification against a wire capture
- The `RESPONSE_SERVICE_ID = 0xFE` magic is referenced but it's not clear whether the bnet-server uses it consistently when dispatching responses
- The Login HTTP/JSON portion is hand-written somewhere (likely in `bnet-server`'s REST / Axum routes); risk of deviation from `Login.proto` field semantics — e.g. `required` vs `optional` field handling, JSON encoding conventions (`proto2 → JSON` rules vs ad-hoc)
- The realm-list flow stuffs proto messages inside `GameUtilities.ClientRequest`'s `Attribute` list, with `Attribute.name` keys like `Command_RealmListRequest_v1`. The Rust side covers `Attribute` but the **command-name conventions** (`Command_*_v1`, `Param_*`, `Param_RealmListBase_v1`, etc.) are only soft-validated; client-side validation is strict, so a typo silently breaks realm browsing
- 7 of 330 status codes ⇒ many error paths are reported as `ERROR_INTERNAL` instead of the precise code — observable client-side as a generic error popup instead of the specific localised message

**Tests existing:**
- 6 unit tests in `wow-proto/src/lib.rs` (Header, EntityId, LogonRequest, ConnectRequest, ClientRequest+Attribute, service-hash constants)
- No integration tests against a real client capture (.pcap)
- No tests on `account_service.proto` types (despite being compiled)
- No tests for the (missing) Login form or RealmList types

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#PROTO.WBS.001** Partir y cerrar la migracion auditada de `proto/BattlenetRpcErrorCodes.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/BattlenetRpcErrorCodes.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 671 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.002** Partir y cerrar la migracion auditada de `proto/Client/account_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4780 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.003** Partir y cerrar la migracion auditada de `proto/Client/account_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 10692 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.004** Cerrar la migracion auditada de `proto/Client/api/client/v1/channel_id.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/api/client/v1/channel_id.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.005** Partir y cerrar la migracion auditada de `proto/Client/api/client/v2/attribute_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/api/client/v2/attribute_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 857 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.006** Cerrar la migracion auditada de `proto/Client/api/client/v2/report_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/api/client/v2/report_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.007** Partir y cerrar la migracion auditada de `proto/Client/api/client/v2/report_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/api/client/v2/report_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1065 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.008** Partir y cerrar la migracion auditada de `proto/Client/attribute_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/attribute_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1069 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.009** Partir y cerrar la migracion auditada de `proto/Client/authentication_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/authentication_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3109 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.010** Partir y cerrar la migracion auditada de `proto/Client/challenge_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/challenge_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 608 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.011** Partir y cerrar la migracion auditada de `proto/Client/channel_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/channel_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2362 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.012** Partir y cerrar la migracion auditada de `proto/Client/club_ban.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_ban.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 738 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.013** Partir y cerrar la migracion auditada de `proto/Client/club_core.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 6073 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.014** Cerrar la migracion auditada de `proto/Client/club_enum.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_enum.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.015** Partir y cerrar la migracion auditada de `proto/Client/club_invitation.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_invitation.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2267 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.016** Cerrar la migracion auditada de `proto/Client/club_listener.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_listener.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.017** Partir y cerrar la migracion auditada de `proto/Client/club_member.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_member.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3438 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.018** Cerrar la migracion auditada de `proto/Client/club_member_id.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_member_id.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.019** Partir y cerrar la migracion auditada de `proto/Client/club_membership_listener.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_membership_listener.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1498 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.020** Partir y cerrar la migracion auditada de `proto/Client/club_membership_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_membership_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1559 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.021** Partir y cerrar la migracion auditada de `proto/Client/club_membership_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_membership_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1308 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.022** Partir y cerrar la migracion auditada de `proto/Client/club_name_generator.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_name_generator.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1015 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.023** Partir y cerrar la migracion auditada de `proto/Client/club_notification.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_notification.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4522 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.024** Partir y cerrar la migracion auditada de `proto/Client/club_range_set.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_range_set.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1708 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.025** Partir y cerrar la migracion auditada de `proto/Client/club_request.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 14861 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.026** Partir y cerrar la migracion auditada de `proto/Client/club_role.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_role.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2330 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.027** Cerrar la migracion auditada de `proto/Client/club_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.028** Partir y cerrar la migracion auditada de `proto/Client/club_stream.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_stream.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 4552 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.029** Partir y cerrar la migracion auditada de `proto/Client/club_tag.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_tag.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 676 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.030** Cerrar la migracion auditada de `proto/Client/club_type.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_type.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.031** Cerrar la migracion auditada de `proto/Client/club_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.032** Partir y cerrar la migracion auditada de `proto/Client/connection_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/connection_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2701 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.033** Partir y cerrar la migracion auditada de `proto/Client/content_handle_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/content_handle_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 516 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.034** Partir y cerrar la migracion auditada de `proto/Client/embed_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/embed_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1233 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.035** Cerrar la migracion auditada de `proto/Client/entity_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/entity_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.036** Cerrar la migracion auditada de `proto/Client/ets_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/ets_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.037** Cerrar la migracion auditada de `proto/Client/event_view_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/event_view_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.038** Partir y cerrar la migracion auditada de `proto/Client/friends_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/friends_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3384 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.039** Partir y cerrar la migracion auditada de `proto/Client/friends_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/friends_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2690 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.040** Partir y cerrar la migracion auditada de `proto/Client/game_utilities_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/game_utilities_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2085 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.041** Cerrar la migracion auditada de `proto/Client/game_utilities_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/game_utilities_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.042** Partir y cerrar la migracion auditada de `proto/Client/global_extensions/field_options.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/field_options.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2342 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.043** Cerrar la migracion auditada de `proto/Client/global_extensions/message_options.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/message_options.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.044** Partir y cerrar la migracion auditada de `proto/Client/global_extensions/method_options.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/method_options.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 764 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.045** Cerrar la migracion auditada de `proto/Client/global_extensions/range.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/range.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.046** Cerrar la migracion auditada de `proto/Client/global_extensions/register_method_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/register_method_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.047** Cerrar la migracion auditada de `proto/Client/global_extensions/routing.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/routing.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.048** Partir y cerrar la migracion auditada de `proto/Client/global_extensions/service_options.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/global_extensions/service_options.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 754 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.049** Partir y cerrar la migracion auditada de `proto/Client/invitation_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/invitation_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 848 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.050** Cerrar la migracion auditada de `proto/Client/message_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/message_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.051** Partir y cerrar la migracion auditada de `proto/Client/notification_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/notification_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1270 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.052** Cerrar la migracion auditada de `proto/Client/presence_listener.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/presence_listener.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.053** Partir y cerrar la migracion auditada de `proto/Client/presence_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/presence_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1869 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.054** Partir y cerrar la migracion auditada de `proto/Client/presence_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/presence_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1141 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.055** Cerrar la migracion auditada de `proto/Client/profanity_filter_config.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/profanity_filter_config.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.056** Cerrar la migracion auditada de `proto/Client/report_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/report_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.057** Partir y cerrar la migracion auditada de `proto/Client/report_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/report_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2324 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.058** Partir y cerrar la migracion auditada de `proto/Client/resource_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/resource_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 533 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.059** Partir y cerrar la migracion auditada de `proto/Client/role_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/role_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 971 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.060** Partir y cerrar la migracion auditada de `proto/Client/rpc_config.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/rpc_config.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 984 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.061** Partir y cerrar la migracion auditada de `proto/Client/rpc_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/rpc_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2138 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.062** Cerrar la migracion auditada de `proto/Client/semantic_version.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/semantic_version.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.063** Partir y cerrar la migracion auditada de `proto/Client/user_manager_service.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/user_manager_service.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1896 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.064** Partir y cerrar la migracion auditada de `proto/Client/user_manager_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/user_manager_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 662 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.065** Partir y cerrar la migracion auditada de `proto/Client/voice_types.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/voice_types.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 536 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.066** Partir y cerrar la migracion auditada de `proto/Login/Login.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/Login/Login.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 3351 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.067** Partir y cerrar la migracion auditada de `proto/RealmList/RealmList.pb.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/RealmList/RealmList.pb.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 2671 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#PROTO.WBS.068** Cerrar la migracion auditada de `proto/ServiceBase.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/ServiceBase.cpp`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#PROTO.WBS.069** Cerrar la migracion auditada de `proto/ServiceBase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/proto/ServiceBase.h`
  Rust target: `crates/wow-proto`, `crates/wow-proto/proto/bgs/low/pb/client`, `crates/bnet-server`, `crates/wow-handler`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numera los items para poder referenciarlos desde `MIGRATION_ROADMAP.md` sección 5.

Complejidad: **L** (low, <1h), **M** (med, 1-4h), **H** (high, 4-12h), **XL** (>12h, splitear).

### Phase A — port the two recovered `.proto` sources (Login + RealmList)

- [ ] **#PROTO.1** Copy `Login.proto` into `crates/wow-proto/proto/json/` (or similar), add to `build.rs`, expose as `pub mod json::login`. (L)
- [ ] **#PROTO.2** Hand-port realm-list types from `RealmList.proto` into `crates/wow-proto/proto/bgs/low/pb/client/realmlist_types.proto` (or similar). Add to `build.rs`, expose as `pub mod realmlist::v1`. (M — schema is straight-forward, ~12 messages)
- [ ] **#PROTO.3** Add a round-trip test for `LoginForm` and `RealmEntry` against captured wire bytes from a real client. (M — needs capture)
- [ ] **#PROTO.4** Verify `bnet-server`'s realm-list handler uses the new types instead of hand-rolled byte builders. (M)

### Phase B — port BattlenetRpcErrorCode in full

- [ ] **#PROTO.5** Generate a Rust `enum BattlenetRpcErrorCode` from `BattlenetRpcErrorCodes.h` (script: parse the C header, emit `pub const ERROR_FOO: u32 = 0x...` set or a `#[repr(u32)] enum`). (M)
- [ ] **#PROTO.6** Replace all `0x...` literal status codes in `bnet-server` with the named constants. (L)

### Phase C — port club service (only `.proto` source not yet ported)

- [ ] **#PROTO.7** Copy `club_service.proto` and `club_listener.proto` into the build, expose under `pub mod club::v1`. (M — 414 lines combined; ~30 message types, ~25 RPC methods)
- [ ] **#PROTO.8** Decide whether to implement the club service handlers or stub them. (M-decision)

### Phase D — recover and port the remaining ~50 services from `.pb.h`

This is the long-tail effort. Each service can be tackled independently.

For each `xxx.pb.h` without a checked-in `.proto`:

- [ ] **#PROTO.9** Reverse-engineer the `.proto` schema from `xxx.pb.h` (pattern: each `class` in the header maps to one `message`, each `enum` to one `enum`, field types/numbers from `*_field_number` constants and tag-prefix bytes in the descriptor pool). (M-H per service)

Concrete sub-tasks (one per service group):

- [ ] **#PROTO.10** `friends_types.proto` + `friends_service.proto` (friends list, invitations). (H)
- [ ] **#PROTO.11** `presence_types.proto` + `presence_service.proto` + `presence_listener.proto`. (H)
- [ ] **#PROTO.12** `account_listener.proto` (server push for account updates). (M)
- [ ] **#PROTO.13** `report_types.proto` + `report_service.proto` (+v2). (M)
- [ ] **#PROTO.14** `resource_service.proto`. (L)
- [ ] **#PROTO.15** `user_manager_types.proto` + `user_manager_service.proto` + `user_manager_listener.proto`. (M)
- [ ] **#PROTO.16** `channel_types.proto`. (L)
- [ ] **#PROTO.17** `notification_types.proto` + `message_types.proto` + `invitation_types.proto`. (M)
- [ ] **#PROTO.18** `profanity_filter_config.proto`. (L)
- [ ] **#PROTO.19** `role_types.proto`. (L)
- [ ] **#PROTO.20** `voice_types.proto`. (L)
- [ ] **#PROTO.21** `embed_types.proto` + `ets_types.proto` + `event_view_types.proto` (rich text + embeds for chat). (M)
- [ ] **#PROTO.22** Club domain (~25 files): `club_core/_enum/_type/_member/_member_id/_membership_*/_invitation/_listener/_notification/_request/_role/_service/_stream/_tag/_types/_ban/_name_generator/_range_set`. (XL — split per concept)

### Phase E — `ServiceBase` analogue

- [ ] **#PROTO.23** Define a `ServiceTrait` (or use `inventory`) for service-hash-based dispatch in `wow-network` or `wow-handler` mirror. (M)
- [ ] **#PROTO.24** Add structured logging helpers (`log_call_server_method`, `log_unimplemented`, etc.) to standardise output across services. (L)
- [ ] **#PROTO.25** Add a `disallow_dynamic_method` enforcement pass (mirrors C++ `LogDisallowedMethod`). (L)

### Phase F — protobuf custom options (low priority)

- [ ] **#PROTO.26** If/when needed, port `field_options`, `message_options`, `method_options`, `service_options`, `range`, `register_method_types`, `routing` from `Client/api/`. (M — only useful if you need schema-level metadata; not required for wire compatibility)

### Phase G — testing

- [ ] **#PROTO.27** Capture a full BNet handshake `.pcap` against a real client and add round-trip tests for every message we port. (M — once)
- [ ] **#PROTO.28** Fuzz prost decoders with malformed BNet packets to ensure no panics. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#PROTO.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 69 files / 115428 lines; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h`. Rust target: `crates/bnet-server`, `crates/wow-handler`, `crates/wow-proto`. | `cargo test -p bnet-server && cargo test -p wow-handler && cargo test -p wow-proto` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#PROTO.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 69 files / 115428 lines; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h`. Rust target: `crates/bnet-server`, `crates/wow-handler`, `crates/wow-proto`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#PROTO.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 69 files / 115428 lines; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h`. Rust target: `crates/bnet-server`, `crates/wow-handler`, `crates/wow-proto`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#PROTO.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 69 files / 115428 lines; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h`. Rust target: `crates/bnet-server`, `crates/wow-handler`, `crates/wow-proto`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Header { service_id, method_id, token, service_hash, size }` round-trip equals capture from a real client `Logon` packet
- [ ] Test: `LogonRequest { program: "WoW", platform: "Wn64", locale: "enUS", ... }` encodes to bytes byte-identical to a TC `bnet-server` capture
- [ ] Test: `ConnectRequest` encodes to bytes that the WoW 3.4.3 client accepts (tested by booting it against the bnet-server)
- [ ] Test: `RealmEntry { wowRealmAddress, name, flags, ... }` survives round-trip and `ClientInformation` matches a 3.4.3.54261 capture
- [ ] Test: `Attribute { name: "Command_RealmListRequest_v1", value: Variant { string_value: ... } }` is the exact byte sequence the realm-list flow expects
- [ ] Test: every defined `service_hash::*` constant equals the FNV-1a hash of the canonical service name string
- [ ] Test: `BattlenetRpcErrorCode::ERROR_OK == 0`, `ERROR_DENIED == 3`, `ERROR_GAME_ACCOUNT_BANNED == 0x0002000B` — discriminant parity with `BattlenetRpcErrorCodes.h`
- [ ] Test: `LoginForm` JSON encoding matches the Battle.net JSON-over-HTTPS form layer (the JSON shape, not protobuf binary)
- [ ] Test: `SrpLoginChallenge` populated by the bnet-server with version=1, iterations=15000 (or the wowchad value) and a valid SRP6 modulus matches a TC reference

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 69 files / 115428 lines; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h` | `crates/wow-proto/` \| ⚠️ partial — 5 services implemented, 2 unported `.proto` sources (Login, RealmList) hand-rolled in handlers, club services + 8 declared services are 0% covered |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#PROTO.DIV.001` | `crates/wow-proto/proto/bgs/low/pb/client` (`exists_empty`, 0 Rust lines) | 69 C++ files / 115428 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_request.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/account_types.pb.h`, `/home/server/woltk-trinity-legacy/src/server/proto/Client/club_core.pb.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. directory exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

- **Two transports.** Battle.net runs over **two** transports in parallel: (a) JSON-over-HTTPS for the *Login form* leg (uses `Login.proto` types, `proto2` syntax with `optimize_for = CODE_SIZE`), and (b) length-prefixed binary RPC over TLS for everything else (uses the `bgs.low` services). Don't conflate them — `bnet-server` exposes both on different ports (1119 binary RPC, 8081 REST/JSON).
- **`service_hash` vs `service_id`.** The first frame on a binary connection registers a service via `service_hash` (32-bit FNV-1a of the service name); subsequent frames refer to it by a small `service_id` byte. Both fields are in `Header`. `RESPONSE_SERVICE_ID = 0xFE` is the magic id for response messages.
- **Token correlation.** Each request has a `token` (uint32) that the response copies back. `bnet-server` must keep a `HashMap<token, oneshot::Sender<...>>` to route replies. Easy to forget when bridging async ↔ sync.
- **`proto2` with `optimize_for = CODE_SIZE`.** The C++ `.proto` files use proto2 syntax (`required` / `optional` are explicit). prost uses proto2 fine, but `required` fields become `Option<T>` in Rust and you must check `.is_some()` at decode-time or your code panics on `.unwrap()`. Lint for this.
- **Attribute / Variant generic envelope.** `bgs.protocol.Attribute { name: string, value: Variant }` and `Variant { bool_value, int_value, float_value, string_value, blob_value, message_value, fourcc_value, uint_value }` — used as a key/value bag for `GameUtilities.ClientRequest`. Realm list, realm-list-character-counts, profanity, etc. are all *transported as attribute lists inside ClientRequest*. The `name` strings are the de-facto API surface (`Command_RealmListRequest_v1`, `Param_RealmListBase_v1`, etc.) — typos there silently break the client.
- **Hash collisions.** Service hashes are 32-bit FNV-1a; collisions are theoretically possible but the canonical service name set is small and curated. Don't invent new services without checking for collision.
- **`message_value` in `Variant` is `bytes`, not nested message.** Holds a serialised inner protobuf — decode separately. This is how realm-list responses carry `RealmListUpdates` etc.
- **`profanity_filter_config.pb.h`** is the chat-profanity rules table; it ships as part of the protocol but is rarely used in private servers. Low porting priority.
- **Recovering `.proto` from `.pb.h`** is doable: each generated header has a `descriptor_pool_` table at the bottom that contains the binary `FileDescriptorProto`. Run `protoc --decode_raw < descriptor_bytes` to recover schemas. Tedious but reliable.
- **Custom field options** like `(method_options).disallow_dynamic_method` and `(service_options).inbound` are used to gate which methods accept dynamic dispatch and which require strict registration. You can ignore these for the Rust port unless you need exact-equivalent dispatch semantics; just hard-code the rule.
- **Login flow secret.** `RealmListTicketClientInformation.info.secret` is a 16-byte client nonce that gets HMAC'd into the world-server handshake. Don't strip it during protobuf decoding — it's load-bearing for `wow-crypto` HMAC validation in the world-server handover.
- **Don't blindly copy `bgs.low/pb/client/`** into the Rust crate — it's already pruned to what's actually used. If you need to add services, add them deliberately with the rationale documented (which client opcode triggers them?).
- **Versioning.** Battle.net protocol versions iterate independently from WoW client versions. The 3.4.3 client uses `bgs.low` from a specific era; later expansions add new services & methods. If you copy a `.pb.h` from a newer Trinity branch, it may not match what the 3.4.3 client expects. Stick to this tree's `.pb.h` set.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class ServiceBase` | `trait Service { fn service_hash() -> u32; async fn call_server_method(token: u32, method_id: u32, buf: BytesMut) -> Result<Bytes>; }` | Or use `inventory` static registration, mirroring `wow-handler::PacketHandlerEntry` |
| `enum BattlenetRpcErrorCode` | `#[repr(u32)] pub enum RpcError` (or `pub mod status { pub const ERROR_OK: u32 = 0; ... }`) | Discriminants must match C++ exactly |
| `google::protobuf::Message` | `prost::Message` | — |
| `MessageBuffer` | `bytes::BytesMut` (input) / `Vec<u8>` (output) | — |
| `std::function<void(MessageBuffer)>` continuation | `oneshot::Sender<Bytes>` or `Box<dyn FnOnce(...)>` | async-friendly version |
| Generated `class XxxService` (per `.pb.h`) | Module `pub mod xxx::v1 { /* messages */ }` + a separate `XxxServiceHandler` trait/impl | prost gives messages, dispatch is hand-written |
| `service_hash_` field | `const SERVICE_HASH: u32` on the trait impl | — |
| `Header` proto | `bgs::protocol::Header` (already done) | — |
| `bgs::protocol::Attribute`, `Variant` | `bgs::protocol::{Attribute, Variant}` (already done) | The key-value envelope pattern is the same |
| `Login.proto` (proto2 + `optimize_for=CODE_SIZE`) | prost-compiled module `json::login` | proto2 → `Option<T>` fields |
| `RealmList.proto` | prost-compiled module `realmlist::v1` | — |
| `EnumUtils` reflection (over `Header.METHOD_ID` etc.) | Hand-written `enum Method { Logon, Verify, ... }` per service | — |
| `LogCallServerMethod`, `LogFailedParsingRequest`, etc. | `tracing::info!`, `tracing::error!` macros with structured fields | Use `wow-logging` |
| `MethodOptions { disallow_dynamic_method = true }` | Compile-time const + runtime check in dispatch | Soft-enforce |

---

*Template version: 1.0 (2026-05-01).* Cuando se rellene, actualizar header de status y `Last updated`.

---

## 13. Audit (2026-05-01)

### 13.1 Audit summary

The Rust `wow-proto` crate is **structurally faithful but heavily under-scoped**. Of the 53 `.pb.h` files in `src/server/proto/Client/` plus the 4 `.proto` source files (`Login.proto`, `RealmList.proto`, `club_service.proto`, `club_listener.proto`), only **11 `.proto` files** are reconstructed in `crates/wow-proto/proto/bgs/low/pb/client/` and **5 services** have RPC handlers in `bnet-server/src/rpc/services/` (Connection, Authentication, Account, GameUtilities — Challenge has messages but no handler module, it is consumed only as the outbound `ChallengeListener` listener type from the auth handler). Wire-format spot-checks against `*.pb.h` field-number constants confirm parity for `Header`, `LogonRequest`, `LogonResult`, `ProcessId`, and `Attribute`. `Login.proto` and `RealmList.proto` are **not** built from prost; instead `bnet-server` hand-rolls equivalent serde structs in `rest/types.rs` and `realm/mod.rs`. The 53 `.pb.h` services from the `bgs.low` proto set (friends, presence, report, club_*, voice, embed, ets, channel, message, notification, invitation, role, profanity_filter_config, rpc_config, resource, user_manager, account_listener, presence_listener, friends_listener, club_listener, club_membership_*, etc.) are **entirely absent** — clients exercising those code paths receive `Unknown service hash` from the dispatcher and a status-1 (`ERROR_INTERNAL`) response. `BattlenetRpcErrorCodes.h` declares **601** named error constants; `wow_proto::status` declares **6** (`OK`, `ERROR_INTERNAL`, `ERROR_DENIED`, `ERROR_NO_GAME_ACCOUNT`, `ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED`, `ERROR_GAME_ACCOUNT_BANNED`, `ERROR_GAME_ACCOUNT_SUSPENDED`) — and the implemented RPC handlers do **not** reference them; they pass bare integer literals (`3`, `12`, `1`) to `send_logon_error` / `send_response_status` instead.

### 13.2 .proto file inventory comparison

| Component | C++ canonical | Rust port | Notes |
|---|---|---|---|
| `Login.proto` | yes (84 lines) | **no** — hand-rolled in `bnet-server/src/rest/types.rs` (~80 lines, serde structs) | Lossy: hand-roll uses snake_case JSON conventions, original is camelCase via proto2 `optimize_for=CODE_SIZE` |
| `RealmList.proto` | yes (83 lines) | **no** — hand-rolled in `bnet-server/src/realm/mod.rs` lines 330–392 (serde camelCase structs) | Hand-roll covers `RealmListUpdates`, `RealmEntry`, `ClientVersion`, `RealmCharacterCountList`, `RealmListServerIPAddresses`; missing `RealmListTicketIdentity`, `RealmListTicketClientInformation`, `ClientInformation`, `IPAddress`, `RealmIPAddressFamily` as schema (parsed inline from JSON in `game_utilities.rs::get_realm_list_ticket`) |
| `Client/club_service.proto` | yes (293 lines) | **no** | Entire community/club system unreachable |
| `Client/club_listener.proto` | yes (121 lines) | **no** | Entire community/club system unreachable |
| `Client/rpc_types.pb.h` | gen-only | yes (`rpc_types.proto`, reconstructed) | Field numbers verified ✓ |
| `Client/entity_types.pb.h` | gen-only | yes (`entity_types.proto`) | ✓ |
| `Client/attribute_types.pb.h` | gen-only | yes (`attribute_types.proto`) | ✓ |
| `Client/content_handle_types.pb.h` | gen-only | yes (`content_handle_types.proto`) | ✓ |
| `Client/semantic_version.pb.h` | gen-only | yes (`semantic_version.proto`) | ✓ |
| `Client/authentication_service.pb.h` | gen-only | yes (`authentication_service.proto`) | Field numbers verified for `LogonRequest`/`LogonResult` ✓ |
| `Client/connection_service.pb.h` | gen-only | yes (`connection_service.proto`) | ✓ |
| `Client/challenge_service.pb.h` | gen-only | yes (`challenge_service.proto`) | ✓ |
| `Client/game_utilities_service.pb.h` | gen-only | yes (`game_utilities_service.proto`) | ✓ |
| `Client/account_service.pb.h` + `account_types.pb.h` | gen-only | yes (`account_service.proto` + `account_types.proto`) | ✓ |
| `Client/account_types.pb.h` | gen-only | yes (`account_types.proto`) | ✓ |
| `Client/friends_service.pb.h` + `friends_types.pb.h` | gen-only | **no** | |
| `Client/presence_{service,listener,types}.pb.h` | gen-only | **no** | |
| `Client/report_{service,types}.pb.h` | gen-only | **no** | |
| `Client/resource_service.pb.h` | gen-only | **no** | |
| `Client/user_manager_{service,types}.pb.h` | gen-only | **no** | |
| `Client/channel_types.pb.h` | gen-only | **no** | |
| `Client/notification_types.pb.h` | gen-only | **no** | |
| `Client/message_types.pb.h` | gen-only | **no** | |
| `Client/invitation_types.pb.h` | gen-only | **no** | |
| `Client/profanity_filter_config.pb.h` | gen-only | **no** | |
| `Client/embed_types.pb.h` | gen-only | **no** | |
| `Client/ets_types.pb.h` | gen-only | **no** | |
| `Client/event_view_types.pb.h` | gen-only | **no** | |
| `Client/voice_types.pb.h` | gen-only | **no** | |
| `Client/role_types.pb.h` | gen-only | **no** | |
| `Client/rpc_config.pb.h` | gen-only | **no** | |
| `Client/club_*.pb.h` (~22 files) | gen-only | **no** | |
| `Client/account_listener.pb.h` | gen-only (in `account_service.pb.h`) | listener service hashes declared, no messages | |
| `Client/presence_listener.pb.h` | gen-only | **no** | |
| `Client/friends_listener.pb.h` | gen-only (in `friends_service.pb.h`) | **no** | |
| **Totals** | **4 .proto + 53 .pb.h** | **11 .proto** | Coverage: 11/57 schema files = ~19% |

### 13.3 Service handler implementation status

For each service whose **server-side handler** exists in `bnet-server/src/rpc/services/`:

| Service | Hash | Methods exposed | Implemented | Stubbed (logged, no body) | NotImplemented | Status |
|---|---|---|---|---|---|---|
| `ConnectionService` | `0x65446991` | `Connect(1)`, `KeepAlive(5)`, `RequestDisconnect(7)` | 3 | 0 | 0 | **complete** for the methods the 3.4.3 client invokes |
| `AuthenticationService` | `0x0DECFC01` | `Logon(1)`, `VerifyWebCredentials(7)`, `GenerateWebCredentials(8)` | 3 | 0 | 0 | **complete** for the production login flow |
| `AccountService` | `0x62DA0891` | `GetAccountState(30)`, `GetGameAccountState(31)` | 2 | 0 | (~25 other methods unmapped) | **partial** — only the two methods the WoW client requests at logon are present; `GetLicenses`, `Subscribe`, `Unsubscribe`, `GetAccountStateByEntityId`, `GetSelectedGameAccount`, etc. are unhandled |
| `GameUtilitiesService` | `0x3FC1274D` | `ProcessClientRequest(1)` (sub-dispatched on `Command_*`), `GetAllValuesForAttribute(10)` | 2 + 4 sub-commands (`Command_RealmListTicketRequest_v1`, `Command_LastCharPlayedRequest_v1`, `Command_RealmListRequest_v1`, `Command_RealmJoinRequest_v1`) | 0 | `PresenceChannelCreated(2)`, `ProcessServerRequest(3)`, `OnGameAccountFlagsUpdated(4)` plus all listener methods | **partial** — covers realm-list flow only |
| `ChallengeService` | `0xBBDA171F` (listener side) | — | 0 server-side handler (we **send** `OnExternalChallenge(3)` from the auth handler, never receive) | 0 | all client-bound methods | **outbound-only** — no handler module exists |

Aggregate (services with at least a handler module): **complete: 2** (Connection, Authentication), **partial: 2** (Account, GameUtilities), **outbound-only: 1** (Challenge), **NotImplemented services that have proto messages but no handler: 0** within the implemented set.

Aggregate (services declared in `service_hash` but with no proto and no handler): **10** — `FRIENDS_SERVICE`, `FRIENDS_LISTENER`, `PRESENCE_SERVICE`, `PRESENCE_LISTENER`, `REPORT_SERVICE`, `REPORT_SERVICE_V2`, `RESOURCES_SERVICE`, `USER_MANAGER_SERVICE`, `USER_MANAGER_LISTENER`, `ACCOUNT_LISTENER`, `AUTHENTICATION_LISTENER`, `CHALLENGE_LISTENER` (the listeners are used outbound from the existing handlers, but no schema is declared for the messages they push beyond the few types in the implemented services). **Effectively NotImplemented at the service-method dispatch level: 12 declared + ~40 services not even declared (club_*, voice, channel, embed, ets, event_view, invitation, message, notification, profanity_filter_config, role, rpc_config).**

### 13.4 Error code coverage

- `BattlenetRpcErrorCodes.h`: **601** named constants in `enum BattlenetRpcErrorCode : uint32` (verified by `grep -cE '^\s+ERROR_'` over the enum body).
- `wow_proto::status`: **6** named constants (`OK`, `ERROR_INTERNAL`, `ERROR_DENIED`, `ERROR_NO_GAME_ACCOUNT`, `ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED`, `ERROR_GAME_ACCOUNT_BANNED`, `ERROR_GAME_ACCOUNT_SUSPENDED`). One of them (`ERROR_WOW_SERVICES_GAME_ACCOUNT_LOCKED = 0x0002_0014`) does not appear in `BattlenetRpcErrorCodes.h` under that exact name — it looks like a hand-curated value, worth reconciling.
- Coverage: **6 / 601 ≈ 1.0%**, considerably worse than the 7/330 figure in the pre-audit doc (the original count of 330 undercounted; the actual enum has 601 entries, and Rust declares 6 not 7).
- Worse: even the 6 declared constants are **never used** in `bnet-server` source. Every error path in `services/authentication.rs`, `services/game_utilities.rs`, and `session.rs::dispatch_request` passes a bare numeric literal (`3`, `12`, `1`). This means a refactor to "add proper named codes" needs both the constants table **and** a sweep of bare literals.

### 13.5 Wire compatibility

Field-number parity was confirmed by cross-referencing `static const int kXxxFieldNumber` constants in `*.pb.h` against the field tags in the reconstructed `.proto` files for the messages exercised on the wire:

| Message | C++ field numbers (sampled) | Rust field numbers | Match |
|---|---|---|---|
| `bgs.protocol.Header` | 1=service_id, 2=method_id, 3=token, 4=object_id, 5=size, 6=status, 11=service_hash, 13=client_id | 1, 2, 3, 4, 5, 6, 11, 13 | ✓ |
| `bgs.protocol.authentication.v1.LogonRequest` | 1=program, 2=platform, 3=locale, 4=email, 6=application_version, 7=public_computer, 10=allow_logon_queue_notifications, 12=cached_web_credentials, 14=user_agent, 15=device_id, 16=phone_number | identical | ✓ |
| `bgs.protocol.authentication.v1.LogonResult` | 1=error_code, 2=account_id, 3=game_account_id, 8=geoip_country, 9=session_key | identical | ✓ |
| `bgs.protocol.ProcessId` | 1=label, 2=epoch | identical | ✓ |
| `bgs.protocol.Attribute` / `Variant` | 1=name, 2=value (Attribute); standard 1..N for typed Variant fields | identical | ✓ |
| `JSON.RealmList.RealmEntry` | proto2 fields 1..10 (`wowRealmAddress`, `cfgTimezonesID`, …) | hand-rolled via serde with `rename_all = "camelCase"` (no protobuf field numbers since not built from .proto) | **N/A on the wire** — RealmList is JSON-over-HTTP, not protobuf-over-RPC, so field numbers don't apply. JSON key names match (`wowRealmAddress` ↔ `wow_realm_address` + camelCase rename). |

**Verdict:** binary RPC wire format for the 5 ported services is compatible with the C++ TrinityCore client. **No field-number divergences detected** in the spot-checks. The risk surface is the **un-ported** services: any future client packet that reaches a service hash absent from the dispatch table will silently fail.

### 13.6 JSON encoding (proto3 zero-value vs RapidJSON)

The old analysis tried to dismiss the JSON default/null hypothesis using the
observations below. Those observations do not establish canonical JSON parity;
compare the actual C++ serializer or an identified capture before accepting it.

1. The only JSON-encoded payloads are `Login.proto` and `RealmList.proto` — both `proto2` with `option optimize_for = CODE_SIZE`. Proto2 has **no** "zero-value omission" rule; presence is tracked explicitly via `required` / `optional`.
2. The recorded Rust path uses hand-rolled `serde::Serialize` structs (`AuthResult`, `RealmEntry`, `ClientVersion`, etc.) with `Option<T>`. Whether absent fields emit null or are omitted depends on the concrete serializer attributes; this is not proof of the client's expected representation.
3. Required fields (e.g. `RealmEntry.wow_realm_address` as `i32`) are unconditionally serialized as their concrete value, including `0` if applicable — same as `RapidJSON` would do for a `proto2` `required uint32` field.

A sample Rust message was recorded as `{"wowRealmAddress":0,...}`. The former
byte-equivalence claim did not demonstrate a canonical C++ comparison and is
withdrawn; no replacement parity verdict is asserted here.

The one residual JSON risk is that the hand-rolled struct field set must stay in sync with the `Login.proto` / `RealmList.proto` schemas as Trinity evolves them — there is no compile-time check linking the two. If Trinity adds a field to `RealmEntry`, the Rust hand-roll silently misses it.

### 13.7 Recommended sub-tasks

(Prefix `#PROTO.A.*` to distinguish from the pre-audit `#PROTO.*` items in §9, which remain valid.)

- **#PROTO.A1** Replace the 6 hand-curated `wow_proto::status` constants with a generated full-coverage table. Write a one-shot Python/awk script that parses `BattlenetRpcErrorCodes.h` and emits a Rust file with all 601 `pub const ERROR_*: u32 = 0x…;` entries plus a `name(code: u32) -> &'static str` lookup. (M, ~2h)
- **#PROTO.A2** Sweep `bnet-server/src/rpc/services/` and `session.rs` for bare numeric literals passed to `send_logon_error`, `send_response`, `send_response_status`, replace with named constants. Add a clippy lint or a wrapper enum to prevent regression. (L, ~1h)
- **#PROTO.A3** Port `Login.proto` to `crates/wow-proto/proto/json/login.proto` (proto2, package `Battlenet.JSON.Login`) and replace `bnet-server/src/rest/types.rs` hand-rolls with prost-generated types + `serde_json` round-trip via `prost-types::Value` or a manual `impl Serialize`. Field-number wire compat is irrelevant (JSON transport) but schema fidelity is critical for forward maintenance. (M, ~3h)
- **#PROTO.A4** Same for `RealmList.proto` → `realm/mod.rs` hand-roll. Note the `proto2 → JSON` field naming is camelCase verbatim (`wowRealmAddress`, not `wow_realm_address`); current hand-roll uses serde `rename_all = "camelCase"` which is correct, but the proto-derived solution needs an explicit per-field `#[serde(rename = "wowRealmAddress")]` or a custom serialize impl. (M, ~3h)
- **#PROTO.A5** Add a regression test that captures a full BNet handshake against the live `bnet-server` (TLS handshake → `Connect` → `Logon` → `VerifyWebCredentials` → `RealmListTicketRequest` → `RealmListRequest` → `RealmJoinRequest`) and asserts byte-identical responses to a recorded golden. Place under `crates/bnet-server/tests/`. (H, needs test infrastructure ~6h)
- **#PROTO.A6** For the 12 service hashes declared in `wow_proto::service_hash` without a handler, decide per-service: (a) implement a real handler (port the proto from `.pb.h`), (b) implement an explicit `NotImplemented` stub that returns `ERROR_NOT_IMPLEMENTED = 0x0D` rather than the current `ERROR_INTERNAL = 0x01`, or (c) remove the constant if the 3.4.3 client never sends it. Audit one wire capture to disambiguate. (M-decision, ~2h + per-service implementation)
- **#PROTO.A7** Reconstruct `friends_service.proto` and `friends_types.proto` from `friends_service.pb.h` / `friends_types.pb.h` using `protoc --decode_raw` on the embedded `descriptor_pool_` blob (see §11 note). Same for `presence_*`, `report_*`, `resource_service`, `user_manager_*`. Each tackled as a separate PR. (XL — split per service)
- **#PROTO.A8** The `CHALLENGE_LISTENER` hash in Rust (`0xBBDA171F`) does not match the C++ `bgs.protocol.challenge.v1.ChallengeListener` FNV-1a; verify with `printf 'bnet.protocol.challenge.ChallengeListener' | python3 -c 'import sys; …'` and reconcile. Mismatched listener hashes silently break server→client push notifications. (L, ~30min)
- **#PROTO.A9** Move the realm-list-ticket inline JSON parsing in `services/game_utilities.rs::get_realm_list_ticket` (lines 86–129) onto the new `RealmListTicketIdentity` / `RealmListTicketClientInformation` proto types from #PROTO.A4. The current code does ad-hoc `serde_json::Value` traversal, missing fields like `gameAccountRegion`, `clientArch`, `systemArch`. (M, ~2h, blocked by #PROTO.A4)
- **#PROTO.A10** Add a `#[deny(missing_docs)]` pass over `wow-proto::service_hash` and `wow-proto::status` so each constant carries a docstring explaining when the client/server emits it. Doubles as cross-reference documentation. (L, ~1h)

### 13.8 Header status update

Old: `❌` / `⚠️ partial`
New (this audit): **`⚠️ partial`** (kept). The crate is wire-compatible for the 5 services it implements and binary-faithful at the field-number level, but coverage is too low to upgrade to ✅ — the un-ported `bgs.low` services + the missing `Login.proto` / `RealmList.proto` schema imports + the 1% error-code coverage prevent that. Promotion to ✅ requires #PROTO.A1, #PROTO.A3, #PROTO.A4, and at least one of #PROTO.A7 (e.g. friends or presence) to land.
