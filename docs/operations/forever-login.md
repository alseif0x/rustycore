# WoW Forever 1.60.1 login-only runbook

The target branch is **`forever`**, renamed from `1.60.1` on 2026-10-02.
This is an independent version line based on `3.4.3`, not a feature branch to
merge back wholesale. Historical acceptance names below refer to the old name.

This is a bounded, local smoke for the WoW Forever client build `1.60.1.70170`.
It exercises RustyCore's normal Battle.net REST SRPv2 flow and protobuf RPC realm
list flow against an isolated Auth database. The synthetic smoke alone does not
prove real-client login. The separate real-client evidence below now proves BNet
authentication, account/realm-ticket queries, the ruleset-selection UI and
acceptance of realm discovery in an isolated online-metadata probe. Modern
realm join remains unimplemented; this is not world authentication.
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

Follow-up read-only contrast: both the local 3.4.3 reference
`a22fd98b554f55156913158a6571bcf9fa0ceb48` (`Server/Session.cpp:620-646`) and
modern `6ebe044c` (`Shared::GameUtilities::GetLastCharPlayed`) return empty
success when there is no last character. The local realm reference derives
`2-1-0` from region 2/battlegroup 1; this is not proof of a Forever district
mapping. Client builder/parser areas `0x22adfe9`–`0x22ae431` reference
`Param_ContentSetIDFilter` and `Param_FilterToPreferredLocality`; these names
alone do not establish the added request's values. The response parser around
`0x22ae8e0`–`0x22aed19` reads `Param_RealmEntry`, `Param_LastPlayedTime` and
optional `Param_UtilityInfo`, calling `0x22b23e0` on a decoded entry. No
fabricated last-character row, guessed subregion rewrite or fallback realm
response has been installed to force progress. The next investigation needs
target-build request attributes and the empty-response continuation contract.

At committed candidate `580b26f1`, BNet passed 110/110 again, Python passed
23/23, and the tree/diff checks were clean. The same release binary (unchanged
build inputs) was restarted with `RUST_LOG=warn`; extended live V1/V2 smoke
passed again. The scoped campaign ended at `19:10:42Z`, **368 seconds** after
its start, including the UI diagnostic adjustments and candidate check.
This scoped duration does not make the blocked full final gate green or erase
the earlier 835-second campaign overrun. Subsequent closeout prose is a
documentation-only delta, not relabeled compiled evidence at a later SHA.

### Build-70170 ruleset-to-realm discovery contract

The safe diagnostic capture at `2026-10-02T19:28:47Z`–`19:29:07Z` established
that the real client requests subregion `70-1-70` both before and after ruleset
selection. After choosing district 1 (PvP), it sends signed
`Param_ContentSetIDFilter=136` and `Param_FilterToPreferredLocality=true`.
Before selection the locality flag is false; the first diagnostic deliberately
redacted negative integers, so that capture alone did not establish the initial
filter value. No opaque attributes, tickets or client secrets were recorded.

The extended hash-gated `forever_client_metadata.py` reports the approved PE's
`JamJSONRealmEntry` descriptor at RVA `0x49bdfd0` (14 fields) and
`JSONUtilityInfo` at `0x49bd6a0` (2 fields). Unknown converter types remain
unclassified, with their RVA, rather than inferred from member offsets.
The target response callback at `0x22ae8e0`–`0x22aed19` requires a blob
`Param_RealmEntry` and signed `Param_LastPlayedTime`; empty success reaches
`0x22aecf2` and invokes the completion with `0x80000135`. Coordinator
`0x2251d10` selects a nonzero realm and positive time; its immutable client
feature byte at `0x48b7308` is enabled, so the ordinary empty-last-character
fallback to `RealmListRequest` is not the active path. These are local
code-only diagnostic anchors, not redistributed client bytes.

Additional versioned source is
[advocaite/TrinityCore Forever at 02245dcd](https://github.com/advocaite/TrinityCore/tree/02245dcd245e7433e524577656177723d3e4992e):
`Services/GameUtilitiesService.cpp::Shared::GetLastCharPlayed` selects a realm
by content set for a new account and returns RealmEntry, current signed time,
and UtilityInfo (`realmPermissions=0x200`). `shared/Realm/RealmList.cpp`
(`GetRealmIdForContentSet`, `FillRealmEntry`, `GetRealmEntryJSON`) and
`Realm.h` establish the version/security/online gates and modern population
conversion. Its `Realm.cpp` maps content 136 to district 1, independently
consistent with our real-client selection. This fork is complementary
implementation evidence, not a blanket certification of 70170 gameplay.

The branch's V2/build-70170 adapter now implements that discovery operation.
`ForeverCatalog` remains the immutable configuration owner and `RealmManager`
remains the only current realm-state owner. The optional binding config is:

```ini
Forever.RealmBindings = []
# Isolated fixture mapping; use only with the matching Auth realm:
# Forever.RealmBindings = [{"realmAddress":33619969,"contentSetID":136,"superDistrictID":1}]
```

Bindings require exact fields, positive content IDs, nonzero packed region/site/
realm, unique realm addresses/content IDs and an already configured district.
They have the same 16 KiB / 64-entry bounds as the district catalog and do not
change the inherited Auth schema. Configuration never overrides online state,
build, account security or district hold/disallow flags. Full packed addresses
are rechecked after the inherited low-ID lookup. The builder uses exact target
JSON spelling (`cfgRealmsID`, not inherited `cfgRealmsId`), content/district
fields and modern population/flag conversion. No BLEEP endpoint is advertised.

Intentional bounded differences from the complementary fork: there is no
first-unrelated-realm fallback, no guessed default content/district mapping,
and no permissive bool/float-to-content-ID coercion. The observed signed filter
is accepted (`-1` means no filter); other negative values, wrong types and
overflow fail malformed. Existing character data comes only from the selected
game account's Auth snapshot and must match the selected realm. A new account
receives a routing timestamp, **not** a fabricated character name/GUID or a
persisted last-played row. V1 and other client builds keep their old handler.

`RUST_LOG=warn,bnet.discovery=debug` enables a bounded allowlist trace of only
discovery commands and numeric/bool routing fields. Strings/blobs are length
only, unknown attributes are count only, and at most 16 recognized attributes
are projected. The diagnostics are off at the normal `warn` setting.

The diagnostic-only build based on `c33987a2` passed 115 BNet tests and a
49.70-second release build before the request capture above (work started
`19:25:10Z`). That evidence covers the trace module's original projection,
not the later routing implementation. The later acceptance must exercise the
combined routing/QA candidate; the diagnostic run is not relabeled as final.

The default integrated smoke retains its offline assertions and also checks
LastChar pre-ticket rejection, unfiltered/unknown-content empty responses,
offline non-advertisement and malformed-filter errors. Its explicit
`--expect-discovery-realm` mode instead validates the new no-character response
for the fixed content-136 fixture. It never changes DB state or starts a world
server, and marks the old RealmList metadata assertion as skipped in that mode.
Use that option only during a separately authorized isolated online-metadata
probe; restore the fixture offline afterwards. A discovery success is not a
World AuthSession or character-selection acceptance.

#### Native-client acceptance and current boundary

The combined acceptance began `2026-10-02T19:44:10Z` on x86_64. Initial
candidate `d35f4f0f` passed 29/29 Python tests and the hash-gated metadata CLI,
but Rust compilation found a private-compressor call. The correction keeps
UtilityInfo compression inside the realm discovery owner rather than exposing
the generic compressor. Corrected, clean candidate **`d8e728af`** passed:

- `cargo test --locked --release -p bnet-server --bin bnet-server --timings`:
  **124/124**, 31.40-second compile and 1.47-second tests.
- `cargo build --locked --release -p bnet-server --bin bnet-server --timings`:
  **PASS**, 53.00 seconds. Reports are
  `cargo-timing-20261002T194523134Z-dca5975bb34c7c17.html` and
  `cargo-timing-20261002T194605486Z-dca5975bb34c7c17.html` under `target/cargo-timings`.
- Python inputs are byte-identical to tested `d35f4f0f`, verified with
  `git diff --exit-code d35f4f0f HEAD -- tools/wow-test-bot`; their **29/29**
  result and metadata CLI are reused, not claimed as rerun at the corrected SHA.
- Restarted isolated BNet, default V1/V2 smoke **PASS** with offline realm;
  `--expect-discovery-realm` **PASS** during the temporary online-metadata probe;
  default smoke **PASS again** after restoration and restart with `RUST_LOG=warn`.

Cargo used one job, this checkout's `target`, and protoc 28.3. `final --base
origin/3.4.3 --timings` was run at both the initial and corrected candidates.
The corrected candidate manifest
`target/validation-v2/manifests/20261002T194849.554172Z-1617047-final.json`
passed whitespace, Python compile, rustfmt and physical-file checks (2272
files), then failed only the previously recorded inherited hotspot ratchet,
before workspace builds/tests. The initial manifest is
`20261002T194410.868835Z-1613955-final.json`. The six affected source/policy
paths listed in the publication section remain unchanged against `2df57d6f`.
This is **not** a green full final gate; the operator's scoped publication
waiver still applies. No actual world or gameplay acceptance is inferred.

Fresh native trace, corrected release binary:

| UTC | Operation | Observed result |
| --- | --- | --- |
| 19:47:45 | Initial LastChar, `70-1-70` | Signed filter **-1**, locality false; empty success followed by ruleset UI |
| 19:48:20.918 | Selected PvP LastChar, 254 bytes | Filter **136**, locality true; **382-byte successful response** |
| 19:48:20.971 | **RealmJoinRequest**, 298 bytes | Client supplies full `Param_RealmAddress=33619969` (`0x02010001`) |
| 19:48:20.971 | Join response | Status **3015 / 0xBC7**, no payload; current V2 adapter has no join implementation |
| 19:48:21.066 | Client disconnect | UI **BLZ51903015**, not the former empty-discovery WOW51900309 |

The probe changed only fixture realm 1's `flag=2,icon=0` to `flag=0,icon=1`,
under predicates for its exact build/region/site/port, and restored both
original values (one affected row each time). `account_last_played_character`
contained zero rows before and after. RealmBindings stayed explicit and the
client executable was not changed. The final BNet is warning-only, with the
realm offline. This temporary advertisement proves response consumption and
the next RPC request, **not** a playable realm or a running world listener.

Next dependency: implement modern join and World AuthSession together with
target-backed ticket/build-variant/crypto admission. Complementary fork
`02245dcd` now supplies relevant `WorldSocket.cpp`, `ClassicOpcodes.cpp`,
`ClassicClientOpcodes.inc`, `CharacterPackets.cpp` and `CharacterHandler.cpp`
anchors for further contrast. Its declaration of build 70170 is not by itself
proof that inherited 70009 packet layouts, our 3.4.3 runtime or world data work.

Acceptance closeout: `2026-10-02T19:44:10Z`–`2026-10-02T19:52:22Z`,
**492 seconds wall time**, including the roughly 37-second compiler-error
repair interval and documentation closeout. Coding before the campaign and the
earlier request-diagnostic run are separate costs. This stays below 600 seconds
for the scoped campaign but does not turn the blocked full final gate green.
Documentation candidate `b7b07cad` passed `quick --base d8e728af`, manifest
`20261002T195124.215466Z-1618524-quick.json`; reviewed code/tool/build inputs
remain byte-identical to the tested candidate. This closing timing note receives
only the documentation-delta check, with no repeated builds or live mutations.

### Modern realm-join contract on `forever`

The BNet adapter at code candidate `8691506f` follows `advocaite/TrinityCore`
commit `02245dcd`:
`GameUtilitiesService.cpp::{GetRealmListTicket,JoinRealm}`,
`RealmList.cpp::JoinRealm`, `RealmList/RealmList.proto::RealmJoinTicket`, and
`common/Utilities/FourCC.h`. V2 build 70170 retains the admitted client-info
`platformType`, `clientArch` and `type` on its RPC session. These are numeric
FourCC values, distinct from the Logon platform string `Wn64`.

The operation validates the selected game account and full packed realm address,
configured binding/district, current online/build/security state, and exact
32-byte client secret before issuing a join response. RealmManager remains the
current realm-state authority, with no guard retained over the database await.
The new statement records the 64-byte client-secret/server-secret concatenation
and the build in the inherited schema's `account.client_build` column, alongside
IP/locale/OS/timezone. A successful response requires exactly one affected row.
Database errors (including an unknown commit outcome) publish no success; a
subsequent normal join generates a new secret and can retry. No durable-session
or crash-recovery guarantee is inferred from this bounded operation.

The ordered response is `Param_RealmJoinTicket` (unprefixed JSON, no NUL),
`Param_ServerAddresses` (existing length/zlib/NUL Jam envelope), and
`Param_JoinSecret` (32 bytes). The JSON ticket contains `gameAccount`, `platform`,
`type` and `clientArch`. V1 keeps its account-name ticket and original statement.
Strict u32 address admission, explicit configuration and failure-before-success
are intentional differences from permissive/truncating or unchecked inherited
paths; there is no fallback into another ruleset.

The integrated smoke has an explicit `--expect-realm-join` mode, requiring
`--expect-discovery-realm`. It writes the disposable game-account's join session
key and validates the JSON ticket, variant, secret length and loopback address.
Default offline checks reject join and malformed/aliased addresses. The separate
`forever_world_probe.py --ack-isolated-probe` binds only `127.0.0.1:18085`,
accepts one connection, exchanges the exact V2 preamble from reference
`WorldSocket.cpp:82–99`, and closes **before AuthChallenge**. It has no account,
character or authentication bypass. Preamble success is not World AuthSession,
encryption, character selection or world loading. Keep the realm offline outside
an explicitly scoped probe and restore its original flags/type afterwards.

There is a further target-evidence gap: the reference's
`sql/custom/auth/2026_10_02_00_auth_classic_beta_70170.sql` explicitly states that
the Win-x64-WoWB build-auth key is unknown and refers to its optional check bypass.
That bypass is **not** imported here. The inherited zero fixture seeds are not
valid 70170 authentication evidence; a successful World AuthSession needs its
own target-backed key/crypto/packet contract and real client acceptance.

#### Realm-join acceptance — 2026-10-02

Validation began at `20:16:24Z` on clean committed candidate `8691506f`, Linux
x86_64, one Cargo job and the checkout's existing `target`. Exact commands:

```bash
export PATH=/home/joe/.cargo/bin:/usr/local/bin:/usr/bin:/bin
export PROTOC=/tmp/rustycore-protoc-28.3/bin/protoc
export CARGO_TARGET_DIR=/home/joe/projects/dream/wow/rustycore/target
export CARGO_BUILD_JOBS=1 VALIDATION_V2_CARGO_JOBS=1
export RUSTYCORE_CPP_REFERENCE_ROOT=/tmp/rustycore-forever-cpp
./tools/validation-v2 final --base origin/3.4.3 --timings
cargo test --locked --release -p bnet-server --bin bnet-server --timings
cargo test --locked --release -p wow-database --lib --timings
python3 -m unittest discover -s tools/wow-test-bot -p 'test_forever_*.py'
cargo build --locked --release -p bnet-server --bin bnet-server --timings
python3 tools/wow-test-bot/forever_bnet_smoke.py --runtime "$RUNTIME"
# Only with the isolated realm temporarily online/PvP:
python3 tools/wow-test-bot/forever_bnet_smoke.py --runtime "$RUNTIME" \
  --expect-discovery-realm --expect-realm-join
python3 tools/wow-test-bot/forever_world_probe.py --ack-isolated-probe
# Restore offline/normal metadata, restart isolated BNet, repeat default smoke.
```

- Full final **FAIL**, manifest
  `target/validation-v2/manifests/20261002T201625.054191Z-1623405-final.json`:
  whitespace, Python compilation, rustfmt and 2277-file physical scan passed;
  the inherited hotspot ratchet failed before workspace builds/tests. The
  publication-section paths were again byte-identical to fork base `2df57d6f`.
- BNet **133 passed**, 50.78s compilation plus 1.39s tests; Python **36 passed**.
  Database **362 passed / 2 ignored**. Its first invocation omitted the C++
  reference environment variable and returned 357 passed / 5 missing-reference
  failures / 2 ignored; rerunning with the existing reference passed (0.13s
  warm build). An earlier command also returned 127 because Cargo was not on
  PATH; the corrected environment above was used. Neither failure is hidden.
- Release binary **PASS**, 50.34s. Only the isolated BNet was restarted.
  The default offline smoke and explicit online discovery/join smoke passed,
  including pre-ticket rejection, malformed client variant, offline realm,
  overflowing/aliased/unknown address rejection and exact positive ticket shape.
- At approximately `20:22Z`, native build 70170 logged in, selected JcJ and
  reached the one-shot listener. It returned the exact V2 client preamble;
  `world_connection_preamble=true`, `world_authentication_tested=false`,
  `character_selection_tested=false`. Deliberate probe closure produced UI
  `WOW51900319`. This is fresh action-specific native evidence, not an
  AuthSession/encryption/character test. The client Connection.log was buffered
  at an older event; no new log-frame timing is claimed for this probe.
- Direct DB read confirmed game-account 1 has `client_build=70170` and a
  64-byte `session_key_bnet`, without exposing it; last-played-character rows
  remain zero. Guarded updates restored the sole realm to `flag=2, icon=0`;
  BNet was restarted at warn level, and no listener remains on world port 18085.
- The first restored-offline smoke returned unclassified **ValueError**.
  A diagnostic rerun that prints only exception locations passed with unchanged
  inputs. Its transient cause is unresolved; this is not a claim of repeated
  end-to-end stability. No credential reset or unrelated runtime change occurred.

The existing scoped experimental publication waiver applies to the renamed
`forever` line; this evidence does not make the full final gate green.
The implementation acceptance campaign ran `20:16:24Z`–`20:24:18Z` (474s),
including the failed invocations, diagnostic rerun and documentation-only quick
check (`20261002T202417.968476Z-1625987-quick.json`). No production-code repair
occurred during acceptance. The private UI helper's coordinate adjustment and
rebuild are included in that interval. The timing closeout, documentation-only
committed final check and publication occur afterwards and are separately
identified; they must not be relabeled as a green full-workspace final.

#### Manual-login follow-up

After the user's initial attempts returned client code 317 at the REST form
response, the unchanged installed binary completed their manual authentication
at `2026-10-02T20:01:47Z` and again at `20:02:59Z`. The sanitized client log
reports form/credentials/logon result 0, a valid realm-list ticket and the
SuperDistrict response. Selecting the configured JcJ card then returned code
309 with the realm offline. No password reset or authentication patch was made;
the cause of the earlier rejected attempts was not established.

The four reference rulesets are district/content `1/136` JcJ, `2/137` Normal/JcE,
`3/138` Roleplay and `4/140` Hardcore (`Realm.cpp`). Only `1/136` is configured in
the current isolated realm. Listing a mode would not implement its gameplay or
provision a usable realm. This mapping is reference evidence; only the JcJ card
has been exercised with the native client in this fixture.

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
On 2026-10-02 the operator explicitly authorized publishing the experimental
`1.60.1` branch despite that inherited failure. This is a scoped publication
waiver, not a passing final result, a baseline adjustment, merge authority, or
approval to deploy a shared realm. Candidate `580b26f1`'s focused evidence
supported initial publication `c33987a2` (whose later closeout changes were
documentation-only). The newer ruleset-routing implementation has its own
corrected candidate and acceptance above; it does not reuse the old production
tests as proof of new code. The separate `3.4.3` branch is not modified.

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
