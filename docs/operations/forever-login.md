# WoW Forever 1.60.1 login-only runbook

This is a bounded, local smoke for the WoW Forever client build `1.60.1.70170`.
It exercises RustyCore's normal Battle.net REST SRPv2 flow and protobuf RPC realm
list flow against an isolated Auth database. The synthetic smoke alone does not
prove real-client login. The separate real-client evidence below now proves BNet
authentication, account/realm-ticket queries and the ruleset-selection UI, but
not successful realm selection.
Neither procedure starts `world-server` or authenticates a World `AuthSession`.

The procedure is operator-only. It mutates the disposable Auth database and
issues normal login/ticket requests. Do not point it at a shared realm, reuse a
non-empty schema, or put a password, database URL, certificate, private key or
session ticket in a command line, report or log.

## Fixed fixture and evidence

- Client evidence: `.build.info` reports `1.60.1.70170`; the tested `WowB.exe`
  SHA-256 is
  `369ce842043f6177850947274287fc5a6cec3ee033a891faa0400a1d0a475d8e`.
- Runtime root: `target/forever-login` (gitignored and private). The fixture
  reads `bnetserver.conf` and `account-password` from this directory; neither
  file belongs in Git or command output.
- MariaDB: disposable `mariadb:11.4`, published only on `127.0.0.1:13316`,
  database `auth_forever_70170`.
- Rust listeners: REST `127.0.0.1:18081`, RPC TLS
  `127.0.0.1:1119`; the seeded Auth row names the realm
  `RustyCore Forever - Login Test`, build `70170`, offline, with world metadata
  `127.0.0.1:18085`.
- Fixture identity: Battle.net `FOREVER@LOCAL.TEST`, linked game account `1#1`.
  The fixture generates fresh SRP salt/verifier material and refuses to seed
  when `account`, `battlenet_accounts` or `realmlist` is non-empty.

The disposable schema provenance is TrinityCore's
`sql/base/auth_database.sql` at commit
`ac16f8ec88625d732a22a197839183aeec41e951`. The Rust fixture then applies the
repository Auth migration manifest and validates the runtime schema before its
single seed transaction. The C++ comparison anchors are
`src/server/game/Accounts/BattlenetAccountMgr.cpp:34-68` and
`src/server/shared/Realm/RealmList.cpp:64-94,98-114`.

## Prepare the isolated target

Use an operator-controlled private preparation helper to create the ignored
runtime directory and TLS files. The local campaign used:

```bash
python3 /tmp/prepare-forever-runtime.py
export RUNTIME="$PWD/target/forever-login"
```

The helper is not a repository deliverable; inspect its output only for the
non-secret paths it creates. Confirm that the disposable MariaDB 11.4 instance
is listening on `127.0.0.1:13316`. If it must be recreated, use a private Docker
env file and a fresh container; never inline its credentials:

```bash
docker run --name rustycore-forever-mariadb \
  --env-file /absolute/path/to/private-mariadb.env \
  --publish 127.0.0.1:13316:3306 \
  --detach mariadb:11.4
```

Initialize the fresh database from the pinned Auth baseline with an
operator-controlled MariaDB client and private client-credentials file. Do not
paste the password into shell history. Do not drop, truncate or clean an
existing database to make the fixture pass. The pinned dump contains the stock
`realmlist` row `(id=1, name='Trinity', address='127.0.0.1',
localAddress='127.0.0.1', localSubnetMask='255.255.255.0', port=8085,
icon=0, flag=0, timezone=1, allowedSecurityLevel=0, population=0,
gamebuild=56014, Region=1, Battlegroup=1)`. On a newly created disposable
database only, verify that exact row and that `account` and
`battlenet_accounts` are empty, then remove exactly that stock row before
running the fixture. A safe operator-controlled sequence is:

```sql
USE auth_forever_70170;

SELECT id, name, address, localAddress, localSubnetMask, port, icon, flag,
       timezone, allowedSecurityLevel, population, gamebuild, Region, Battlegroup
FROM realmlist
WHERE id = 1;

SELECT (SELECT COUNT(*) FROM account) AS account_rows,
       (SELECT COUNT(*) FROM battlenet_accounts) AS battlenet_rows,
       (SELECT COUNT(*) FROM realmlist) AS realm_rows;
-- Proceed only for account_rows=0, battlenet_rows=0, realm_rows=1.

DELETE FROM realmlist
WHERE id = 1
  AND name = 'Trinity'
  AND address = '127.0.0.1'
  AND localAddress = '127.0.0.1'
  AND localSubnetMask = '255.255.255.0'
  AND port = 8085 AND icon = 0 AND flag = 0 AND timezone = 1
  AND allowedSecurityLevel = 0 AND population = 0 AND gamebuild = 56014
  AND Region = 1 AND Battlegroup = 1;
```

Require one affected row and re-check `realmlist` is empty. If the row differs,
or the database is not freshly created and disposable, stop; the Rust fixture
never performs this deletion automatically. Keep the BNet process and every
other writer off this database during provisioning. The emptiness checks and
the seed are intentionally conservative; the single seed transaction aborts on
ID collisions and never overwrites an existing row.

## Provision the fixture

From the repository root, pass exactly the generated config path and private
password-file path to the example:

```bash
cargo run --locked --release -p bnet-server --example forever_login_fixture -- \
  "$RUNTIME/bnetserver.conf" "$RUNTIME/account-password"
```

Expected output is a single provisioning summary without account or ticket
secrets. A second invocation must fail with a non-empty-table refusal; that is a
guard against overwriting the fixture, not a repair instruction.

## Start BNet and run the smoke

Start the already-built BNet binary in one terminal. Keep the log level at
`warn`: informational login logs can contain ticket-bearing fields.

```bash
RUST_LOG=warn ./target/release/bnet-server \
  --config "$RUNTIME/bnetserver.conf"
```

In another terminal, run the normal REST/RPC smoke (not a bot-only bypass):

```bash
python3 tools/wow-test-bot/forever_bnet_smoke.py --runtime "$RUNTIME"
```

The smoke reads at most 1024 bytes from the private password file, accepts the
same 128-character password bound as the fixture, pins the runtime certificate
while using TLS hostname `localhost`, and reports only booleans plus build/version
and decoded realm metadata. It checks the Rust/C++ `DONE`/no-ticket shape for a
rejected wrong SRPv2 `M1`, a valid server `M2` and ticket,
`Connect`/`OnLogonComplete`, the realm-list ticket request, and the decompressed
realm-list record. The expected logical realm is `cfgRealmsId=1`,
`wowRealmAddress=0x02010001`, `deleting=false`, build `70170`, version `1.60.1`,
and offline. The realm-list blob has no network address or port; `18085` is the
database realm metadata only. This procedure stops before `RealmJoinRequest`.

The wire implementation follows the current Rust RPC/authentication/game
utilities modules and C++ fork `a22fd98` (`Server/Session.cpp` and
`REST/LoginRESTService.cpp`). It is a protocol smoke, not evidence that the
retail/Arctium-derived launcher handles the modern client certificate loader.

## Acceptance boundary

### Modern connection identity contract

On this branch, `rpc/identity.rs` owns one identity per `RpcSession`. Server PID
and startup epoch are process-wide; fallback client ID and creation epoch are
connection-scoped. `Connect` returns both process IDs, current server milliseconds,
an explicit bindless value (default true), and the uppercase 33-byte CIID. The
outgoing session writer includes that same CIID in every RPC header, including
status-only errors and server notifications. Wire field 13 is still named
`client_id` in the inherited protobuf; no schema rename or world protocol change
is required.

This deliberate modern-client adaptation follows TrinityCore master
`6ebe044cbb9895b458fcd3244639acadff287809`:
`src/server/bnetserver/Services/ConnectionService.cpp::HandleConnect` and
`src/server/bnetserver/Server/Session.cpp::{Session,SendResponse,SendRequest}`.
It is not attributed to the 3.4.3 fork. The Python smoke omits client ID/bindless
in Connect to exercise their defaults and verifies CIID on the Connect reply,
an unknown-service status error, subsequent replies, and `OnLogonComplete`.

Before this change the real build-70170 client completed TLS and sent
`ConnectionService.Connect` (token 0), then sent `RequestDisconnect` (token 1,
error code 0) without `AuthenticationService.Logon`. The old Connect response
omitted both the fallback client ID and CIID. This narrows the observed failure;
it does not by itself prove that CIID is the only remaining incompatibility.

The CIID campaign on 2026-10-02 ran from `17:17:24Z` through `17:19:45Z`
(141 seconds) on x86_64, based at `27c58fd1` with the identity/consumer changes
uncommitted. Commands: `cargo test --locked --release -p bnet-server --bin
bnet-server --timings` (92/92), `cargo build --locked --release -p bnet-server
--bin bnet-server --timings`, Python `unittest discover` for
`test_forever_bnet_smoke.py` (12/12), targeted rustfmt, `git diff --check`,
`check_architecture.py physical-files`, and the restarted isolated runtime's
`forever_bnet_smoke.py`. Cargo used one job, the worktree's `target`, and local
protoc 28.3. The live smoke passed SRP negatives/mutual proof and the added
CIID/status-only checks through offline realm list. Timing reports are
`cargo-timing-20261002T171724637Z-dca5975bb34c7c17.html` and
`cargo-timing-20261002T171809147Z-dca5975bb34c7c17.html`.

The same real client then **passed Connect**: the sanitized sequence at
`17:19:38Z` was client `0x65446991:1`, token 0, payload 62 → server success,
payload 66/header 42 → client `0xC02F8216:1`, token 1, payload 503/header 90.
The latter service was unsupported, returned status 1, and was followed by
`RequestDisconnect`, token 2/error 0. Thus the CIID change has fresh real-client
progress evidence, but full login still fails at the next service. No world
server was started. No final publication runner or push was performed.

### Benilla comparison

The user-supplied [Benilla](https://github.com/samwhosung/benilla) was inspected
at `99b5600e250d2da2e26461dd33361b8d2d84458b`. Its
`crates/benilla-protocol/src/auth.rs`, `lib.rs::logon`, and
`world/session.rs::WorldSession::connect_queued` implement Vanilla
`1.12.1.5875`: realmd TCP/SRP6, legacy realm list and world auth/header crypto.
It is a useful reference for phase-specific failures and parser tests, **not**
an implementation of BNet TLS/protobuf, CIID, modern auth tokens or build 70170.
No Benilla wire constants or crypto were imported into this branch.

### Modern authentication and HTTP-session contract

The `1.60.1` adapter implements Authentication V2, original service hash
`0xC02F8216`, methods Logon (1), VerifyAuthToken (2), GenerateAuthToken (3).
It uses V2 protobuf layouts, not V1 messages under another hash. Its listener
`0x9DA8116B` sends ExternalChallenge (4) and LogonComplete (1). Admission shares
V1's existing DB ticket/expiry/IP/country/ban checks; the extracted loader does
not publish account state. V1 retains its prior notification/reply order. V2
sends the successful reply, then LogonComplete, then publishes authentication,
following `Shared::Authentication::HandleVerifyAuthToken`. Transport failure
does not produce a second reply or publish authentication. V2 accepts the six
modern platform names, validates WoW title/locale, and decodes device `UTCO` as
a timezone-name hash (not raw minutes). The protos are the consumed/emitted
projection, not a complete implementation of every modern BNet service.

Exact source: TrinityCore `6ebe044cbb9895b458fcd3244639acadff287809`,
`src/server/bnetserver/Services/AuthenticationService.cpp` (Shared and V2),
`ClientBuildInfo.cpp`, `src/common/Time/Timezone.cpp`, and generated V2
`authentication_{types,service,listener}.pb.{h,cc}` under
`src/server/proto/Client/api/client/v2/`. Proto field numbers and method IDs
come from those generated sources. The real client requested this service
after Connect; no post-auth Account V2/GameUtilities V2 support is implied.

At `17:38:29Z` the real client accepted V2 Logon and received its HTTPS challenge,
but rejected an IP-literal URL (`BLZ51914003`). Changing only the isolated
`LoginREST.ExternalAddress` and `LoginREST.LocalAddress` to `localhost` reached
GET `/bnetserver/login/` and POST `/bnetserver/login/srp/` at `17:40:03Z`–
`17:40:04Z`. Both returned 200 on distinct TCP connections; no proof POST
followed, and the UI showed `WOW51900317`. Therefore use the certificate's
`localhost` hostname for this fixture. TLS remains enabled and verified.

The subsequent REST repair deliberately changes the inherited behavior:

- JSON `public_B` follows `src/server/proto/Login/Login.proto`, not the Rust
  member spelling `public_b`.
- `rest::RestSessions`, owned by the listener, is the sole registry for normal
  login SRP state. A TCP connection acquires one random `JSESSIONID` lease,
  requires the same remote IP on reconnect, and publishes the C++ Secure,
  HttpOnly, SameSite=None cookie. Unknown IDs never become server IDs.
- The last connection drop starts five-minute inactivity retention. Admission
  lazily removes expired entries; unlike C++'s minute sweep, unused entries may
  occupy memory until the next admission, but cannot be reacquired after expiry.
  Active leases pin the canonical state. No new cleanup task is introduced.
- The registry's synchronous lock never spans await. A per-session async gate
  serializes SRP replacement/proof and ticket DB work, blocks only that login
  session, and is released before output. Cancellation releases both guard and
  lease; an already-created challenge remains recoverable after failed output.
  This does not add a DB durability/unknown-COMMIT guarantee.
- `BaseHttpSocket::HandleMessage` is the lifetime reference; `LoginHttpSession`
  obtains the cookie state; `HttpService` owns IP matching/inactivity and mirrors
  request keep-alive. Responses now honor close/keep-alive instead of advertising
  close while retaining the socket. Bot-only endpoints remain connection-local.
- `HandlePostLogin` retains SRP after a rejected proof. Tests no longer mistake
  cookie-free handler headers for absence of a transport-layer cookie. Raw HTTP
  headers/bodies and login inputs are not logged by the changed transport.

Keeping only connection-local SRP (even with corrected keep-alive) was rejected:
the real client already opens different connections. No cookie-to-IP fallback,
password bypass or synthetic authentication success was added. Cookie lifecycle,
wrong-IP/unknown-cookie isolation, per-session serialization, keep-alive policy,
JSON casing and cross-connection wrong/valid SRP proofs are acceptance cases.

The initial V2 campaign ran `17:35:18Z`–`17:39:28Z` (250 seconds), based at
`4c446a13` with uncommitted changes: wow-proto 11/11, BNet 96/96, Python 13/13,
release build, rustfmt/diff/physical checks and live V1/V2 smoke. It verified
malformed/invalid V2 admission, reply/notification order, generated tokens and
fresh cached-token login. The later hostname and REST diagnostics are separate;
that campaign alone did not prove real-client login.

The REST-cookie acceptance slice (`17:49:46Z`–`17:52:36Z`, 170 seconds) passed
BNet 102/102, Python 15/15, release build, rustfmt/diff/physical-file checks and
the restarted live V1/V2 smoke using separate TLS connections for SRP challenge,
wrong proof and valid proof of the same challenge. At `17:52:26Z`, `WowB.exe`
first submitted the proof POST, but received the no-ticket rejection. That first
rejection's precise cause was not captured and must not be attributed to a
specific arithmetic bug. A subsequent metadata-only diagnostic build did not
change SRP arithmetic. At `17:55:32Z` the same real client sent A/M1 with
512/64 hex characters, received a ticket/M2 response, sent V2 VerifyAuthToken
(method 2, 45-byte payload), received success followed by V2 LogonComplete
(83-byte payload), and requested Account V2 methods 101/104/201/203. Those
unsupported requests returned status 1 and the client disconnected. Three
further real attempts at `18:03:02Z`, `18:03:09Z` and `18:03:16Z` also completed
SRP and sent VerifyAuthToken without changing crypto. This is real BNet-auth
success, not yet a successful realm-selector UI or a world login.

Code inspection separately found that REST's hex byte-pair decoder discarded
the final nibble of odd-length integers. `HandlePostLogin` constructs C++
`BigNumber` from hex integers, so valid odd lengths must preserve their value.
The repair accepts those integer forms and rejects malformed strings instead
of silently skipping characters. An oversized-A guard intentionally fails closed
before Rust's fixed-width hashing assertion; this is input hardening, not a
claim that the C++ assertion failure is preserved. These findings do not prove
the cause of the first uncaptured rejection.

The complete modern-login investigation includes repeated acceptance after live
findings and diagnostic builds; its total wall span exceeds 600 seconds. The
ordinary ten-minute end-to-end performance target is **not met**. The bounded
slice timings above identify actual executions, not separate claims that the
whole delivery met that budget; investigation/editing time was not timed apart
precisely enough to subtract it from the full span.

### Account V2 and the real-client post-auth boundary

The next implementation is still login-only. At TrinityCore
`6ebe044cbb9895b458fcd3244639acadff287809`, the exact anchors are
`Services/AccountService.cpp` (V2 handlers 101/104/201/203),
`Services/GameUtilitiesService.cpp` (`Shared::{FindParamValue,GetRealmListTicket}`
and V2 adapters), and generated
`src/server/proto/Client/api/{client,common}/v2/*.pb.{h,cc}`.

- `rpc/services/account_v2.rs` projects the existing authenticated account
  snapshot; it creates no second account authority. It provides account ID and
  privacy flag 7, game-account display name, and ordered ban/suspension metadata.
  `SEL_BNET_ACCOUNT_INFO` appends `ab.bandate` at column 13, preserving columns
  0–12. Restriction creation/expiry use the stored timestamps in milliseconds,
  not the current clock. An explicit fail-closed authentication gate is retained;
  the referenced C++ V2 handlers assume a populated account.
- `rpc/services/game_utilities/v2.rs` supports realm-list ticket, empty-fixture
  last-character query, subregions, and the existing realm-list reader. V2 uses
  its actual Variant tags (string 4/blob 5/uint 6), first-match parameter order,
  and the NUL-terminated `AuthRealmListTicket` marker. V1 wire behavior is not
  replaced. The service hash `0x5DBB51C2` is the generated service's
  `OriginalHash`, not the FNV hash of its present fully qualified name.
- World join, BLEEP proxies and Forever's new SuperDistrict list are not
  implemented. Nonempty last-character/modern utility-info and full modern
  realm schema parity are also not proven by the empty offline fixture. Do not
  advertise this bounded adapter as a complete modern GameUtilities service.

After the new release build was installed in the isolated BNet process, the
V1/V2 live smoke passed at `18:20 UTC`, including
all four Account V2 queries, unauthenticated/malformed/unknown-method rejection,
subregions, the exact V2 realm-ticket marker and decoded offline realm metadata.
No account secrets or session bytes are retained in the report.

The real build-70170 client then produced this action-specific trace on
`2026-10-02T18:20:49Z`–`18:20:50Z`. Sizes are protobuf payload bytes; these
sanitized frame observations are not a full decrypted payload capture:

| Client operation | Request bytes | Observed server result |
| --- | ---: | --- |
| Connect | 62 | Success, 66 bytes |
| Auth V2 Logon | 503 | Challenge notification, then empty success |
| Normal HTTPS SRP proof | A/M1: 512/64 hex chars | Ticket/M2 response, 230-byte JSON body |
| Auth V2 VerifyAuthToken | 45 | Empty success, then 83-byte OnLogonComplete |
| Account V2 101 / 104 | 0 / 0 | Success, 6 / 0 bytes |
| Account V2 201 / 203 | 11 / 11 | Success, 10 / 0 bytes |
| RealmListTicket | 727 | Success, 49 bytes |
| FetchBleepProxies | 50 | Not implemented, status 3015; client continues |
| GetAllValuesForAttribute | 38 | Success, 9 bytes |
| LastCharPlayed | 185 | Empty success |
| SuperDistrictList | 99 | Not implemented, status 3015; client disconnects |

The isolated window displayed **“Actualmente no hay reinos disponibles.
(WOW51900309)”**. This is genuine successful authentication followed by a
realm-discovery failure, not a completed login UI. `SuperDistrictList` is absent
from the pinned modern C++ command dispatcher; the public master file inspected
on this date also lacks it. The failed request alone does not establish the
correct response schema, so no guessed success or 3.4.3 packet substitution was
added. A target-build response capture or independently established client
decoder contract is the next evidence prerequisite for that operation.

The client was run in its own Wine prefix/client copy using the diagnostic
launcher setup. Original client files were preserved. The launcher still needed
diagnostic intervention; unattended startup and the experimental in-memory
certificate provider are not accepted deliverables. This does not affect the
observed server-side SRP proof, but limits reproducibility of the UI procedure.

Post-auth acceptance started at `2026-10-02T18:11:01Z`, in the dirty worktree
based at `051559a6`. Commands used one Cargo job, the checkout's `target`, and
`PROTOC=/tmp/rustycore-protoc-28.3/bin/protoc` on this x86_64 host:

```bash
cargo test --locked --release -p wow-proto -p wow-crypto -p wow-database --lib --timings
cargo test --locked --release -p bnet-server --bin bnet-server --timings
RUSTYCORE_CPP_REFERENCE_ROOT=/tmp/rustycore-forever-cpp \
  cargo test --locked --release -p wow-proto -p wow-database --lib --timings
cargo test --locked --release -p wow-proto --lib --timings
cargo build --locked --release -p bnet-server --bin bnet-server --timings
python3 -m unittest discover -s tools/wow-test-bot -p test_forever_bnet_smoke.py
cargo fmt --all -- --check
git diff --check
python3 tools/architecture/check_architecture.py physical-files
python3 tools/wow-test-bot/forever_bnet_smoke.py --runtime "$RUNTIME"
```

Results: crypto 53/53; BNet 108/108; database 362 passed/2 ignored with the
explicit C++ reference; protobuf 11/11; Python 16/16; format, diff and physical
checks passed. The initial DB run failed five reference-location checks because
`/home/server/...` is absent here. The rerun used checkout `a22fd98b554f55156913158a6571bcf9fa0ceb48`,
not the documentation's unavailable default reference pin. One newly added
protobuf test incorrectly derived GameUtilities OriginalHash from the present
name; the test was corrected against the generated C++ constant and passed.
The production hash was already correct. The release build took 48.42 seconds;
Cargo reports are under `target/cargo-timings`. The only production change
after the initial 108-test run was command-name-only debug tracing, exercised by
the release build/live trace and the committed-candidate rerun below.

At committed candidate `ee710e4eb15947b750d279d04324ae45c7ff88eb`, the BNet
binary suite passed again (108/108), Python passed (16/16), and physical-files,
rustfmt and diff checks passed. The installed binary's V1/V2 live smoke passed
again after restarting only the isolated BNet process at `RUST_LOG=warn`.
The candidate tree was clean during these checks; earlier crypto/database/
protobuf results remain evidence from the explicitly identified dirty tree,
not retrospectively relabeled runs at this SHA. Production/test inputs were
unchanged except the described tracing line and corrected protobuf test.

The post-auth campaign ended at `2026-10-02T18:24:56Z`: **835 seconds** from
`18:11:01Z`, including investigation/corrections and the final candidate rerun.
The 600-second target was not met. This closeout documentation delta is outside
that measured interval and does not change compiled inputs. No final runner,
push, PR or world-server test was performed. The disposable MariaDB and BNet
listeners remain local; diagnostic logging has been disabled.

### Build-70170 SuperDistrict discovery contract

The next adapter is based on the approved original client hash above, not the
3.4.3 realm-list layout. `tools/wow-test-bot/forever_client_metadata.py --client
/path/to/original/WowB.exe` verifies SHA-256 **before** interpreting PE data and
prints only schema names, types, offsets and RVAs. No executable or extracted
client bytes are distributed. The reproducible metadata anchors (image base
`0x140000000`) are:

- `JSONSuperDistrictList` descriptor RVA `0x49bd060`, field `superDistricts`;
  the type-chain entry pointer stored at `0x49bcf28` points to `0x49bcf50`.
- `JamJSONSuperDistrictEntry` descriptor RVA `0x49bcf50`: `superDistrictID`
  is int32 at member offset 0, `holdDownUntilTime` is uint32 at offset 4,
  `disallowLogin` is bool at offset 8. The offset array uses **u16**, not u32.
  Type RVAs are respectively `0x48e3420`, `0x48e3530`, `0x48e2ed0`.
- The locally decoded client callback checks `Param_SuperDistrictList` at
  RVA `0x22af2cc`, blob Variant case 5 at `0x22af319`, then invokes the Jam
  decoder at `0x22af377` with descriptor `0x49bd060` and size cap `0x40000`.
  String getter `0x931120` returns `JSONSuperDistrictList`.
- Consumer `0x24c6630` walks 12-byte entries and resolves the ID through
  `0x630cd0` against the client's `AvailableSuperDistrict` data. Unknown IDs
  are skipped; a valid JSON response does **not** prove a visible selector.
  It reads bool at `0x24c68a6` and uint32 hold time at `0x24c68b3`.

`realm/forever.rs` owns an immutable, explicitly configured catalog, loaded
once by `main.rs` into `AppState`. Under `[bnetserver]`, optional configuration is:

```ini
# Empty by default; no assumption that a 3.4.3 realm ID is a SuperDistrict ID.
Forever.SuperDistricts = []
```

Entries require exactly `superDistrictID` (positive int32), `disallowLogin`
(bool), and `holdDownUntilTime` (uint32). Invalid/duplicate entries fail startup;
operator input is capped at 16 KiB / 64 entries. The isolated diagnostic fixture
uses `[{"superDistrictID":1,"disallowLogin":false,"holdDownUntilTime":0}]`
as an explicit diagnostic choice. The 19:07 UTC client test below establishes
that this ID is recognized as PvP; no other district mapping is claimed.
V2 requires authenticated build 70170 and a selected game account from the
realm ticket. It returns `Param_SuperDistrictList` with little-endian u32
uncompressed length, zlib, and NUL-terminated
`JSONSuperDistrictList:{"superDistricts":[...]}`. No mutable realm cache,
database district rows or world listener are introduced.

`Command_FetchBleepProxiesRequest_v1` separately returns an empty
`Param_BleepProxyList` / `JSONBleepProxyList:{"proxies":[]}` envelope. Its
reference is TrinityCore `6ebe044cbb9895b458fcd3244639acadff287809`,
`Services/GameUtilitiesService.cpp::Shared::GetBleepProxies` and
`RealmList.proto::BleepProxyList`. This advertises no proxy service.

The extended smoke checks pre-ticket rejection, the exact bounded district
envelope/schema, and the empty proxy response.

Scoped acceptance started `2026-10-02T19:04:34Z` in the dirty worktree based at
`a53a88d0`, with the same one-job/target/protoc environment as above. BNet passed
110/110 (26.98-second compile, 1.37-second tests), Python
`unittest discover -s tools/wow-test-bot -p 'test_forever_*.py'` passed 23/23,
the metadata CLI verified the original client, and rustfmt/diff/physical-file
checks passed (2266 files). Release build passed in 47.93 seconds. After
restarting only the isolated BNet, the extended V1/V2 live smoke passed with
district 1, both pre-ticket rejection and post-ticket acceptance, and empty
BLEEP proxies. No world server or character database was used.

Fresh native-client trace at `19:07:08Z`–`19:08:13Z`:

| Operation | Request bytes | Response / visible outcome |
| --- | ---: | --- |
| Normal REST SRP and Authentication V2 | Same flow as above | Successful authentication |
| Realm-list ticket | 733 | Success, 49 bytes |
| FetchBleepProxies | 50 | Success, 77 bytes; client polls again every ~5 seconds |
| Initial LastCharPlayed | 185 | Empty success |
| SuperDistrictList | 99 | Success, 131 bytes; **ruleset-selection UI displays PvP** |
| LastCharPlayed after choosing PvP | 254 | Empty success, then client requests disconnect |

The user-visible state advances from the former immediate no-realms failure to
`Elige tu conjunto de reglas` with a selectable `JcJ` card. Choosing it still
ends in `WOW51900309`; no `RealmListRequest`, `RealmJoinRequest`, world socket,
character enumeration/creation or world load was observed. Empty BLEEP is an
implemented response, not a proven usable proxy service. The next unresolved
contract is the transition after the selected ruleset's last-character query.
Client-input helper adjustments were private diagnostic work, not a shipped
launcher. This does not establish unattended startup.

### Publication validation boundary

At README candidate `a53a88d0`, `validation-v2 final --base origin/3.4.3
--timings` did **not** pass. Two bootstrap attempts required fetching missing
locked dependencies. The subsequent runner manifest
`target/validation-v2/manifests/20261002T185029.358620Z-1594545-final.json`
passed whitespace, Python compilation, rustfmt and physical-file checks, then
failed the inherited hotspot ratchet before workspace build/tests. The failing
areas are Session, Map, character/quest handlers, world-server composition and
Player. Their source and architecture-policy paths are byte-identical to fork
base `2df57d6f`, verified with `git diff --exit-code` over `crates/wow-world`,
`crates/wow-map`, `crates/world-server`, `crates/wow-entities`,
`tools/architecture`, and `docs/architecture`. No limits were relaxed.
The routine final gate remains blocked; scoped login tests do not replace it.

### Previous fixture campaign

The first live run of the original helper on 2026-10-02 at 16:17 UTC passed the
negative SRP proof, valid `M2`/ticket, RPC logon completion and decoded offline
realm build. The final local campaign ran from `2026-10-02T16:52:11Z` through
`16:53:32Z` (81 seconds) in the worktree based at `2df57d6f`, with uncommitted
worktree changes. It passed the fixture example tests (3/3), the `bnet-server`
binary suite (87/87), the Python smoke tests (11/11), the live REST/RPC smoke,
rustfmt, `git diff --check`, and the physical-files architecture check. The
live smoke passed wrong-M1 rejection, valid M2/ticket, RPC logon completion,
realm ticket, and decoded offline realm build/version `1.60.1.70170`. A prior
cold release build took 7m04 and is separate from this 81-second campaign.
No final runner, push, or publication was performed.

No world server login or World `AuthSession` was exercised. A real-client UI
attempt was exercised in isolated Wine with DXVK after removing the rejected
`-gxapi` argument; several TLS connections were accepted by the server, but the
client terminated with `WOW51900340` before completing login. The
Arctium-derived launcher revision `f2eb6c9` is not sufficient for the Client
Modern path, and an experimental in-memory certificate provider is not a
deliverable. Do not promote this runbook's accepted Auth/realm-list smoke to
full client or world-login compatibility; the real-client attempt failed at
the client UI boundary.
