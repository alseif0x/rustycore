# WoW Forever 1.60.1 login-only runbook

The target branch is **`forever`**, renamed from `1.60.1` on 2026-10-02.
This is an independent version line based on `3.4.3`, not a feature branch to
merge back wholesale. Historical acceptance names below refer to the old name.

This is a bounded, local smoke for the WoW Forever client build `1.60.1.70170`.
It exercises RustyCore's normal Battle.net REST SRPv2 flow and protobuf RPC realm
list flow against an isolated Auth database. The synthetic smoke alone does not
prove real-client login. The separate real-client evidence below now proves BNet
authentication, account/realm-ticket queries, the ruleset-selection UI and
acceptance of realm discovery and modern realm join in an isolated probe.
The native client now also passes strict World `AuthSession` digest verification
and the derived session key is persisted. The signed encryption offer is now
acknowledged and an encrypted client ping authenticates. Real `WorldSession`
admission remains pending; this is not character access or a complete world
login. These probes do not start the full `world-server`.

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
The committed documentation delta `96c23258` passed
`validation-v2 final --base 8691506f --timings` at `20:24:41Z`, manifest
`20261002T202440.855307Z-1626115-final.json` (whitespace and physical-files;
no Cargo rerun). Including this required closeout gives a 497-second campaign
envelope from the original start. This timing/evidence-only append is checked
with `git diff --check`; no code or test inputs changed after `8691506f`.

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

### Strict build-70170 world authentication contract

The `forever` implementation is independent of 3.4.3 compatibility. Reuse does
not make inherited 16-bit world opcodes, SHA-256 derivation or packet layouts
valid for this target. The pinned Forever reference remains
`advocaite/TrinityCore@02245dcd245e7433e524577656177723d3e4992e`:
`src/server/game/Server/WorldSocket.cpp` (V2 preamble, AuthSession verification,
continued-session persistence and encryption transition),
`src/server/game/Server/Packets/AuthenticationPackets.cpp` (packet writers),
`src/common/Cryptography/SessionKeyGenerator.h` (SHA-512 generator), and the
Login database `UPD_ACCOUNT_INFO_CONTINUED_SESSION` statement. Do not dump the
authentication source's private-key declaration into logs or documentation.

`crates/wow-crypto/src/forever.rs` owns target SHA-512/HMAC derivation,
Ed25519-context signing and AES-256-GCM with a 12-byte tag. The plaintext V2
header is size-u32 plus tag-12; the opcode is u32 inside the framed data. The
direction-specific IV is counter-u64 plus direction-u32. Plain challenge,
AuthSession, offer and ACK consume counters; first encrypted packets start at
counter two in each direction. Invalid tags do not advance the counter and
the socket becomes terminally failed.

`crates/wow-network/src/forever/` owns one socket's phase, pending request,
keys and counters. Account admission and SQL remain in composition. Proof
requires the full 64-byte join key and the correct private 16-byte build key;
it checks a constant-time 24-byte digest. The guarded existing
`account.session_key_bnet` transition to 40 bytes must affect exactly one row
before the signed 69-byte encryption offer. No legacy statement column order
was changed. Cancellation leaves an incomplete transition unusable. The
fixture's admit-before-persist ordering and strict exact ticket/padding/length
checks are intentional fail-closed restrictions, not unqualified C++ parity.
Compression and playable WorldSession composition are not implemented here.

The production-linked `bnet-server` example `forever_world_fixture` binds only
`127.0.0.1:18085`, accepts one connection and admits only the disposable fixture
account, build/variant and online PvP realm. It does not create a WorldSession.
If encryption succeeds it sends an encrypted **denial**, not a fabricated
successful login, and authenticates the encrypted client response. That last
step passed with the native client's encrypted ping, but native parsing of the
server denial is not yet proven. Default fixture state stays offline.

The integrated `client-build-key-probe` tool is limited to the isolated
SHA-256-pinned executable. Its build-key observation leaves the native digest
implementation unchanged and writes only a new ignored private file. Its
certificate observer reports bounded count/region/flag/key-match metadata, not
certificate contents, credentials or private keys. The local client resource
provider is still experimental; neither the original installation nor the
official account was used for this campaign.

Native certificate parsing is anchored at client RVAs `0x471FE60`,
`0x47204C0`, `0x4720560`, constructor `0x1F25730` and selector `0x1F2C61B`.
The resource is PEM/X.509 (TACT file ID 7725530), not JSON. The key must be
Ed25519/32 bytes. `serverAuth` and vendor `.303.1.1.2`/`.303.1.1.3` EKU values
provide admission/flag metadata; `.303.1.2.<decimal>` provides RegionGroup.
The cached certificate's `.303.1.2.0` means group **0**, not geographic Region 2.
The server fixture now follows the reference's independent
`Network.EnterEncryptedModeRegionGroup` configuration (default 0); it must not
derive this selector from the realm address. Changing it blindly to 2 would
not address the current rejection.

#### World-auth acceptance and unresolved encryption boundary

Campaign start: **2026-10-02 21:51:22 UTC**, Linux x86_64, one Cargo job, same
checkout `target`. The default debug-profile test commands rebuilt dependencies
not present in that profile's cache; this must not be presented as a ten-minute
warm acceptance pass. The campaign envelope already exceeds 600 seconds, so
the ordinary performance target is **not met**. Error repair and continuing
certificate/protocol investigation remain identified separately.

```bash
./tools/validation-v2 final --base origin/3.4.3 --timings
cargo test --locked -p wow-crypto -p wow-network --lib --timings
cargo test --locked -p wow-network --lib --timings
cargo test --locked -p bnet-server --bin bnet-server \
  --example forever_world_fixture --timings
cargo build --release --locked -p bnet-server \
  --example forever_world_fixture --timings
python3 -m unittest discover -s tools/wow-test-bot -p 'test_forever*.py'
# Isolated runtime/private paths only; never copy their contents into Git:
./target/release/examples/forever_world_fixture --ack-isolated-probe \
  "$RUNTIME/bnetserver.conf" "$RUNTIME/build-auth-key-70170"
```

- `64612059` full final failed inherited hotspot limits before Cargo; manifest
  `20261002T215122.706666Z-1637881-final.json`. Its crypto suite initially passed
  58/59: the synthetic expected session/encryption vectors were incorrect.
  Production derivation was checked against exact C++ and independent SHA-512
  calculations; only those two expected vectors changed in `e48246ec`.
- `e48246ec`: **59 crypto and 37 network tests passed**; timing artifact
  `20261002T215324949Z`. Its release fixture build passed (83 seconds), Python
  passed **40/40**. Crypto and Python inputs have not subsequently changed.
- At `21:55:54Z`, native 70170 authentication and JcJ selection reached the
  strict fixture: **24-byte AuthSession proof verified**, then the guarded DB
  write stored a 40-byte session key. Client UI returned `WOW51900319` before
  encryption ACK; no encrypted session was established.
- `c4e36296` full final again failed the same inherited hotspot ratchet;
  manifest `20261002T220414.094630Z-1650244-final.json`. Whitespace, Python,
  rustfmt and physical-files checks passed. No ceilings were relaxed; affected
  inherited gameplay/runtime/architecture paths match fork base `2df57d6f`.
- `aa5f42c5`: **37 network / 133 BNet / 3 fixture tests passed**. Cargo timings
  `20261002T220532006Z` and `20261002T220542386Z`; the BNet debug build took
  171 seconds plus 23.63 seconds of tests. Release fixture build passed in
  21.87 seconds (`20261002T220918686Z`). The Windows observer published with
  .NET 8; no fixture account secrets were printed or committed.
- At `22:16Z`, the installed `aa5f42c5` fixture again verified the native digest
  and persisted 40 bytes. Instead of ACK it received **opcode `0x450007`,
  payload length 4** (disconnect). The certificate observer failed its bounded
  vector check; this is a failed diagnostic, not proof that the keyring is empty.
  The cached certificate is Ed25519 and its public key matches the signer; its
  EKU includes serverAuth plus vendor region/flag identifiers. Native selection
  and verification remain unresolved. Guarded restore returned the sole realm
  to `flag=2, icon=0`; BNet restarted at warn level and port 18085 is closed.

At `22:27–22:28Z`, a fresh isolated process with the integrated certificate
resource provider accepted the signed encryption offer. The observer reported
**one certificate, RegionGroup 0, flag 1, matching key**, and restored the native
selection instructions. Its provider call counter was one. The fixture
authenticated an **encrypted client ping `0x450006`, eight payload bytes** at
counter two. Thus strict proof, signed ACK and inbound AES-256-GCM are native
evidence, not just synthetic tests. The UI remained in game-server admission
before the one-shot listener closed; server AuthResponse parsing, WorldSession
admission and characters are not inferred from the ping. The sole realm was
restored offline/normal and BNet restarted. The ignored structured result is
`target/forever-login/native-world-auth-20261002T2227Z.json`.

This live probe used uncommitted source based on `aa5f42c5`, subsequently
committed without executable changes as **`b919ba12`**. The affected 37 network
and 3 fixture tests passed (4.74s build / 0.30s tests; Cargo timing
`20261002T222539334Z`); release fixture build passed in 20.43s
(`20261002T222604158Z`). The .NET publish passed. Four native CLI negative checks
rejected an outside-isolation path, a synthetic PRIVATE KEY block, malformed
certificate PEM and a wrong executable hash before any client modification.
No original-client file was touched. The local resource provider is limited to
the isolated hash-pinned build and does not disable native signature validation.
Its installation suspends/resumes only its own increment, preserves pre-existing
launcher suspension, restores code protection, and rolls back a failed detour.
It retains separate read-only PEM, executable code and writable diagnostic pages
until process exit. The launcher still needs operator coordination; this is not
an unattended client installer.

Native challenge (`0x4D0000`), AuthSession (`0x450001`), encryption ACK
(`0x450005`) and ping (`0x450006`) are proven here;
the disconnect opcode is directly observed. Remaining world IDs and layouts
are source-backed hypotheses until their specific native actions pass. Successful
synthetic encryption/signature tests are not character enumeration, character
creation, durable relogin or initial world-load evidence.

#### Committed-candidate validation closeout

At `98b5c061`, full final found a formatting-only failure in the fixture's new
configuration read; manifest `20261002T223039.137750Z-1661237-final.json`.
`ea87f09a` fixes only that wrapping. Full final on that clean committed candidate
passed whitespace, Python syntax, rustfmt and physical-files (2283 files), then
failed the unchanged inherited hotspot ratchet before Cargo:
`20261002T223105.115464Z-1661371-final.json`. Exact affected inherited paths were
again byte-identical to `2df57d6f`; the existing experimental publication waiver
is retained, not represented as a green full-workspace gate.

At `ea87f09a`, the complete affected **37 network / 3 fixture tests passed**
again, build 1.17s and tests 0.30s; timing `20261002T223210048Z`.
Earlier 59 crypto and 133 BNet results retain their actual revisions above;
their unchanged inputs were not needlessly recompiled. The native installed
fixture was built from `b919ba12`'s source before its commit; the later delta is
only the documented formatting. This is not a claim that a prior binary build
or live probe ran at a later SHA.

The measured campaign checkpoint is `21:51:22Z`–`22:33:07Z` (**2505 seconds**),
before this final documentation/publication closeout. Required closing checks
extend that envelope; they do not create a second nominal ten-minute campaign.
The 600-second target is **not met**. Coding/error repair and diagnostics were
interleaved, and no independent coding stopwatch was retained; this is not a
clean warm-cache benchmark. Cargo timing reports above separate actual build
costs, including the unintended cold debug-profile compilation. The closing
runner manifests record their own exact start/end times. Publication does not
mean a playable realm or authorization to merge into `3.4.3`.

### Build-70170 character-data prerequisite

#### Target account/character initialization implementation (2026-10-03, in progress)

The new `forever-world-server` product uses a separate canonical
`wow_world::forever::Session`, with SQL-free persistence contracts and target
MariaDB adapters. It does not construct a legacy Player or send legacy u16
packets. `PacketHandlerEntryFor` is generic, preserving the existing
registrations through a concrete legacy alias; the same generic declaration owns
Forever's u32 metadata and call thunk.
The legacy alias retains closure type inference; a generic-default-only attempt
failed compilation and was corrected centrally, not in hundreds of handlers.

The disposable container now also has `characters_forever_70170` (134 tables),
`world_forever_70170_02245` (253) and `hotfixes_forever_70170` (483). Bootstrap
uses source `02245dcd245e7433e524577656177723d3e4992e`, release archive
`TDB_full_1210.26091_2026_09_09.7z` with SHA-256
`fc5513334d7534a19f533124a95910193c8150379e5ed8d0db3e547ac2c5a3f5`,
and target updates ordered by **filename**, as `UpdateFetcher::PathCompare`.
An independent read-only check found all selected source update hashes match:
4 Character, 76 World and 10 Hotfix, with zero pending target updates.
Four missing target session-only Auth tables were added from pinned CREATE
definitions; no Auth dump/account/password reset was performed.

Bootstrap initially rejected a qualified `world.conditions` patch after exposing
a directory-order error. That diagnostic `world_forever_70170` copy remains
untouched; corrected loading used the new `_02245` destination. An earlier empty
Character destination was resumed only after proving it had zero tables. The
tracked bootstrap refuses nonempty reuse, cross-schema directives and source
hash mismatch; it does not drop/reset databases or render secret-bearing SQL errors.

Account query holders must succeed before the ordered eight-message initialization
batch is returned. A genuinely empty character query produces enum, recent-ally
release and Classic collection response in source order. These 70009 character
layouts/opcodes were exercised by the fresh native scenario below; remaining
unimplemented operations are not covered by that observation.
Nonempty unported collections/characters fail explicitly, not as empty results.
Creation, nonempty enumeration, player admission and initial world loading remain
open. Successful initialization, empty selection and creation UI are observed,
but no character creation transaction or playable world is claimed.

Achievement's actual 70170/esES header has table hash `D2EE2CA7`, layout
`6FC5281B`, 19 fields and inline ID 3. Its two sections contain 434 readable and
9 unknown-key records. Normal optional TACT import follows the reference extractor
and the public [TACTKeys source](https://github.com/wowdev/TACTKeys), pinned at
`71b75360752840dd62a412013a38b5a972524a50`; private key-file SHA-256 is
`6d83241ed776f9e74e27ed65d1ec52f1a1464a4e9b16bed497258fdd61020b52`.
It still does not decrypt the second section. Explicit available-prefix acquisition
and reader support follow `DB2CascFileSource::HandleEncryptedSection` /
`DB2FileLoader::LoadTableData`'s **Skip** path. The nine inaccessible IDs must
remain absent, without zero-fill or an assertion that the whole table was read.
Ordinary WDC5 loading remains strict; no keys/assets are tracked.

At **2026-10-03 00:19 UTC**, the normal debug `forever-world-server` loaded
33 availability races, 434 readable achievements and 2940 effective hotfix records.
The isolated native client passed strict proof and signed encryption, accepted
the real account initialization batch, sent `0x440014` with an empty body and
displayed the empty character list. After the first-run Classic/Improved experience
choice, «Crear personaje» opened a human warrior 3D preview. The client also sent
a hotfix request (`0x440011`, 2344 bytes) and encrypted pings. Unregistered requests
were ignored with metadata-only diagnostics; this is not whole-session protocol
completeness. Neither the original installation nor official account was used or
modified. Screenshots remain private in the ignored QA context.

Real DB observation: account 1 online, 40-byte continued key, zero characters.
SIGINT cancellation exited successfully and released online to zero. The exact
realm predicate restored `flag=2, icon=0`, affecting one row; port 18085 closed.
Restarted isolated BNet's V1/V2 offline smoke passes.
Subsequent source review found two defaults to correct: BNet notification
suppression is true (`BattlenetPackets.h:63-72`) and empty enum's maximum character
level starts at 1 (`CharacterPackets.h:291`). Focused assertions now cover both.
The two other availability observations are explicit isolated policy: active
expansion 0, achievement-requirement bypass disabled. Arbitrary expansion/config
support is not claimed. The repeated enum diagnostic was also corrected to describe
only an actual outgoing enum, not every later ping.

The **00:24 UTC repeat** with those defaults accepted initialization/empty enum,
but stayed on a black scene/loading bar with creation disabled and subsequently
showed WOW51900319; cleanup released the online flag. Creation UI is therefore
an earlier observation, **not a stable current readiness gate**. Requests
`0x440010` (DBQueryBulk, 294/230 bytes) were unimplemented. No missing target
store is silently labeled an absent record. Separately, the socket still imposed
the diagnostic 30-second entire-frame wait on authenticated idle connections.
The in-progress repair separates the unchanged bounded frame-completion read
from the character-phase remaining deadline: target World.cpp:726,1061 defaults
`SocketTimeOutTime` to 900000ms/900s. The canonical Session owns monotonic activity;
initialization and registered ordinary packets reset it, ping/unregistered packets
do not (`WorldSession::ResetTimeOutTime`, WorldSocket.cpp:438/HandlePing).
Partial-frame reads cannot extend the remaining deadline. Player-active/queue
semantics are not claimed.

The completed idle delta passes **42 network / 4048 world tests (1 ignored)**,
report `20261003T003042839Z` (1m00s compilation), plus the target-binary test
`20261003T003149252Z` (35.43s). Normal target build without fixture features passes,
`20261003T003235740Z` (32.90s). The five new transport scenarios cover idle timeout,
wrong phase, cancellation, partial-frame deadline and independent client-direction
AES-256-GCM nonce/counter-2 acceptance. Session assertions cover non-refreshing
ping/unregistered requests, deadline expiry and registered enum refresh.

Fresh native repetition at **00:33–00:37 UTC** accepted initialization and empty
enum, remained connected for more than three minutes and sent successive encrypted
pings. It still showed the loading/disabled-creation UI. Metadata-only observations
identify `DBQueryBulk` hash **DF2F53CF**, count 44/84 and payload 182/342 bytes:
exactly `4 + 2 + 4*N`, consistent with the source's 13-bit count. No requested
record bytes or keys were logged. Pinned source identifies **TactKey.db2**:
`DB2Stores.cpp:350`, `DB2LoadInfo.h:6066–6089`, `DB2Metadata.h:22769–22785`
(FDID 1302850), and the custom Classic update's table-hash metadata at lines
113–115. This is the next typed-store/DBReply responsibility; missing/unported
key records must not be fabricated as successful or silently absent. Source
Classic mapping yields DBReply wire `4A0000`; target payload acceptance remains
to be exercised after implementation. The earlier enabled creation preview is
not relabeled as this later binary's result.

SIGINT again released account online to zero, guarded realm restoration affected
one row and port 18085 closed. BNet-only restoration smoke is recorded separately.

Scoped acceptance on dirty `58b4fbde` plus this implementation: 764 data,
362 database (2 ignored), 767 packet, 35 persistence and **4048 world (1 ignored)**
tests pass. One target-binary strict-ticket test passes; the legacy binary compiled
with zero tests (not behavioral proof). Python guards: 48 pass; acquisition CLI:
9 pass; CASC header CTest: 1 pass. Normal target build passes without fixture features.
The first packet test expected success-presence bit 1 instead of native/source
MSB `0x80`; production encoding was already correct, only the expectation changed.
Two intermittent legacy world fixtures used victim-only
`MOD_ATTACKER_MELEE_HIT_CHANCE` on the attacker, leaving a 5% miss band.
`02245dcd` Unit.cpp::MeleeSpellMissChance (12538–12543) confirms attacker
`MOD_HIT_CHANCE`. Correcting only these fixtures yielded the green full suite,
without changing combat production code. Initial binary compilation also exposed
a missing final `Ok(())`, corrected before its passing test/build.

Cargo timing reports include `20261002T235744273Z` (3m30s),
`20261003T000137401Z` (2m50s), `20261003T001040988Z` (3.23s),
`20261003T001127386Z` (3m04s normal build) and `20261003T002044553Z` (2.97s).
The campaign began at **2026-10-02 23:54:58 UTC** and includes failures, repairs,
checks and live/restoration closeout; it exceeded 1500 seconds before publication.
The ordinary 600-second target is **not met**. Earlier schema import/CASC acquisition
were separate prerequisite costs; coding/repair was interleaved, not independently
timed. Committed-candidate final evidence and the full campaign end follow at publication.

Committed candidate **06ab5ef9c99d0d8759477dd2e01e2f9853c81107** was validated with
`final --base origin/forever --architecture --timings` (Forever is the independent
publication line, not a PR into 3.4.3). Clean manifest
`20261003T003834.512629Z-1735193-final.json` records **failed**, 49.970s:
architecture policy 21.547s and syntax ownership continuation 27.706s. Physical
files pass (2304 files), but inherited hotspot ceilings remain red; the generic
declaration in legacy registry additionally adds 22 lines to that owner. Its shared
metadata responsibility is being relocated to a private root module, preserving
the concrete alias and all registrations rather than raising a ceiling.

Syntax ownership exposes pre-existing taunt/creature-insert signature and two
bridge fingerprint mismatches. The exact three affected source files
(`spell_effects/effect_combat.rs`, `world_entities/creature_registry.rs`,
`legacy_runtime/creature_lifecycle_tick.rs`) are byte-identical to fork base
`2df57d6f` and the preceding Forever publication. No ownership baseline is regenerated.
This is newly executed evidence of inherited debt, not a green full final or an
assertion that the scanner audits Forever's separate Session. The new Session's
four-entry exact-set/thunk/admission tests remain its scoped registry evidence.
Publication closeout and the relocation's renewed affected checks follow separately;
full character creation/world goal remains active.

The follow-up is a mechanical relocation, not gameplay ownership retirement:
`PacketHandlerFn` and `PacketHandlerEntryFor` now have one declaration in private
`wow-world/src/packet_registry.rs` (47 lines). The legacy registry retains its
concrete alias, inventory and lookup table and publicly re-exports the same types;
Forever's existing import path, four registrations, thunk signatures, fields and
admission semantics are unchanged. Legacy `session/registry.rs` is 62 lines,
11 fewer than the preceding published `58b4fbde`, with no ceiling adjustment.
The completed relocation's `cargo test --locked -p wow-world --lib --features
test-fixtures --timings -- --quiet` passes **4048**, with one ignored, on the
scoped dirty worktree based at `06ab5ef9`; timing `20261003T004259279Z`
records an 88.4-second build and 1.29-second test execution. This renews the
actual legacy and Forever exact-set/dispatch tests, not just compilation.
The normal production consumer `cargo build --locked -p world-server --bin
forever-world-server --timings` also passes (2m26s,
`20261003T004605955Z`), without `test-fixtures`. No packet/state behavior changed
in this move, so the earlier native scenarios retain their actual binary/input
revision rather than being relabeled as live tests of this relocation.

At clean committed relocation candidate **`1e631b4d1dd1134985f7c3faaa17894ac7531f97`**,
`final --base origin/forever --architecture --timings` records **failed** in
manifest `20261003T004856.104854Z-1742493-final.json`, 49.948s (policy 21.596s,
syntax ownership 27.856s). Physical files pass (2305); inherited hotspot and the
same taunt/creature ownership mismatches remain. Session production LOC is
95306, 33 fewer than `06ab5ef9`; no new finding or baseline/ceiling refresh is
introduced. The runner stops before Cargo/hygiene: the affected World suite,
normal production build, rustfmt and whitespace evidence above are separate
scoped checks, not a green final. The operator's experimental-publication waiver
is retained; this is neither playable acceptance nor merge/deployment authority.

The complete account-phase campaign checkpoint **2026-10-02 23:54:58Z–2026-10-03
00:49:46Z is 3288 seconds**, including failures/repairs, required checks and three
native scenarios/restoration. Closing documentation validation/publication extends
that same envelope; the ordinary **600-second target is not met**. It is not reset
by the mechanical move. Character persistence and world admission remain open.

Publication reached `origin/forever` **`d0bb654cf3e16fbecb72b42b8e0eae3fed60cba6`**
at 00:50:22Z (same campaign envelope: **3324 seconds**). Documentation-only quick
manifest `20261003T005012.241643Z-1742719-quick.json` passes against tested code
candidate `1e631b4d`; its executable inputs are unchanged. Final remains red,
not reused as green. Remote `3.4.3` remained
`24a513855e1d4c55f208cc3e53a5f23973b5f675`; no cross-version PR/merge was made.

##### Typed TactKey delivery — scoped acceptance and native observation

The native requests identify **TactKey.db2**, hash `DF2F53CF`, with 44/84 IDs.
Target source `02245dcd` anchors: `DB2Metadata.h:22769-22785` (FDID 1302850,
layout `CBA490FC`, external metadata ID, one unsigned 16-byte array),
`DB2LoadInfo.h:6066-6089`, `HotfixDatabase.cpp:1820-1823`, shared
`DataStores/DB2Store.cpp::LoadFromDB` (official then custom) and `WriteRecord`
(skip the external ID, serialize only the 16 field bytes),
`DB2Stores.cpp::LoadHotfixData:1741-1806` (last status per ID; only RecordRemoved
erases), and `HotfixHandler.cpp:25-57,78-130` / `HotfixPackets.cpp:58-76`.
Classic opcode mapping names native request `440010` and response `4A0000`;
fresh native batch delivery is recorded below.

The operator-only probe adds parameterless `--ack-tact-key-table`. Normal complete
CASC extraction, with the existing private source-provided key list, succeeds:
**69 records / 1654 bytes**, WDC5/v5, one field/section, 16-byte records,
flags 4, header ID index **0**, external IDs 276 bytes, parent count 0. Column
metadata is uncompressed, offset 0 / width 128 bits. Private file SHA-256 is
`bdf268fa9eefda4ae23773599a69b542666fd9b7e1172fca135c183ab1b7b9e5`;
artifact directory `target/forever-login/client-data-tact-70170-20261003T0105Z`,
mode 0600, not a repository asset or fixture. No keys/records are printed.
An initial attempt correctly refused an existing output directory; two subsequent
attempts exposed an overly strict header-index guard. The source metadata's
IndexField=-1 is **not** the native header's IndexField: `DB2FileLoader::LoadHeaders`
does not equate them. Actual ChrRaces likewise uses flags 4 / header index 0.
The corrected guard preserves external-ID list sizes, hashes, array/schema,
non-sparse and normal-read requirements. CMake, one expanded synthetic CTest
and **10 CLI guards pass**; this is acquisition evidence, not native DBReply QA.

The new immutable `wow-data::forever_hotfix` catalog owns effective key fields:
mandatory actual baseline → complete official/custom SQL batches → metadata's
final removal decisions → immutable session resource. Read-only SQL metadata
finds **498 official / 108 custom** rows, 517 unique IDs (89 cross-batch overlaps),
no duplicate IDs within a batch and no negative VerifiedBuild. Both queries must
succeed before publication; duplicate unordered batch IDs explicitly reject as
unsupported ambiguity. The cache registers table presence without copying raw
key records. Metadata-only errors and no Debug/Clone keep values out of logs.
TactKey itself allows no optional data in this source: `LoadHotfixOptionalData`
permits a TactKey optional key only **for BroadcastText**. TactKey optional rows
are skipped, not appended blindly; the actual fixture has zero such rows.

The registered authenticated/Inplace DBQueryBulk operation (`Opcodes.cpp:423`)
preserves requested ID order
and duplicates, 13-bit bounds and source DBReply timestamp/status/size/field bytes.
Missing records in the complete effective TactKey store receive Invalid with no
data; other table hashes remain an explicit unsupported boundary, not fabricated
absence in an incomplete target catalog. HotfixConnect uses the same canonical
typed records, then source SQL-blob fallback, then actual RecordRemoved/Invalid
status. No SQL runs per client packet, no auth/encryption verification is bypassed,
and no plaintext record content is captured. Character saving/world remain open.

Affected Rust acceptance began **2026-10-03 01:02:36Z** on dirty `d0bb654c`:
`cargo test --locked -p wow-data -p wow-database -p wow-packet -p wow-persistence
-p wow-world --lib --features wow-world/test-fixtures --timings -- --quiet`.
Compilation passed in 2m59.5s (`20261003T010236485Z`), and **774 data tests
passed**. The command then failed five inherited database source-contract tests
because `RUSTYCORE_CPP_REFERENCE_ROOT` had been omitted; all failures were missing
reference files, not TactKey assertions. With that environment set to the pinned
reference path, the unchanged database/packet/persistence/world suites passed
(`20261003T010610648Z`, no compilation), including **4052 world tests / one
ignored**. The failed first run is not relabelled as green.

Target-binary test and normal production build passed sequentially. The normal
build took 2m51s (`20261003T010954493Z`). A final diagnostic-only correction
removed the obsolete “Unported DBQueryBulk” label; its renewed bin test passes
**1/1**, 1.27s (`20261003T011254284Z`), followed by the normal build, 1.10s
(`20261003T011255605Z`). Library executable inputs are unchanged by that label.

The installed normal binary loaded **522 effective TactKey records**, 33
availability races, 434 readable Achievement rows and 3052 hotfix records.
At **01:13–01:15 UTC**, fresh strict native login/ACK/init and empty enumeration
were followed by target queries for **16 and 68 TactKey IDs**. Both reply batches
contained zero Valid records: these requested IDs are absent from this complete
effective catalog, so the source-defined Invalid/empty response is used. No
request IDs or record/key values were logged. The client proceeds to creation,
renders race/class models (including Cielonato and human), and opens **human
warrior personalization** with appearance controls. This observation does not
prove the earlier loading cause or general repeatability. No Finish/name/save
operation was submitted; Character row count remains **zero**.

At **01:15:58Z**, isolated World had exited cleanly, account online was zero,
the exact fixture realm was restored offline/normal, BNet restarted in warn mode,
and restored V1/V2 positive/negative/offline smoke passed. This stage's recorded
acceptance/restoration interval is **802 seconds**; the 600-second performance
target is **not met**. It extends the already over-budget full account/character
campaign rather than resetting its envelope. Final publication validation for
this delivery remains separate; prior inherited-red evidence is not a new pass.

##### Native Create request and customization prerequisites — 2026-10-03

The isolated product adds a separate optional diagnostic flag:
`--ack-private-character-create-capture <new-private-file>`, after its existing
four arguments. Only an already authenticated **`440070` CreateCharacter** is
recorded; all other opcodes, including auth, ping, TACT queries and hotfixes,
are excluded. The output must be under the ignored isolated runtime, is created
with mode 0600 / create-new, and has a 64-KiB payload bound. The recorder does
not register/admit creation, alter Session ownership or send success. Existing
files/symlinks and out-of-root destinations reject; two new synthetic tests
exercise scope, permissions, opcode exclusion, framing, size and no-overwrite.
The target-binary suite passes **3/3** (`20261003T011847751Z`, 1.53s), followed
by the normal build (`20261003T011849326Z`, 1.49s). Unchanged library inputs
retain the separately dated TactKey acceptance above.

The integrated Windows probe adds `--character-name-input <isolated WowB.exe>
<fixture first name> <fixture surname>`. It validates the existing target SHA,
exact process path, bounded ASCII fixture names and the observed Win32 client
rectangle **1432x1018** (X11 window 1440x1052), and only queues name-field UI
input, never Submit. Two initial diagnostic attempts rejected an incorrect
1440x1024 client-area assumption; no field input occurred. Corrected .NET
publish and real name input pass, as do **five native negative CLI tests**
(invalid names/arity and non-fixture paths). The original installation and
official account were not used.

At **01:19–01:22 UTC**, a second strict native session/empty enum reaches
race/class models and human warrior personalization; TactKey query batches
**32/52 IDs** receive actual Invalid replies. The visible Finish action submits
**opcode `440070`, 106 payload bytes**. Exactly that action is recorded in
private `character-create-70170-20261003T0119Z.bin` (122 total bytes, mode 0600,
SHA-256 `4472b82df9bced5198974c58fdeaea8c26b5d675fb7cf10a4ea3c7f2f6e607f5`).
The new read-only `forever_character_create_probe.py` validates framing/build/
opcode and the source layout, returning metadata only, never either name or
choice values. Its **three synthetic tests pass**. Actual metadata: race 1,
class 1, sex 0, name/surname lengths 9/7, nine customization pairs, no template
or flags, unknown int32=-1, season=0, no duplicate options. Wire options are
not sorted; `CharacterPackets.cpp:487-516` sorts them after reading. The
70009-annotated source layout matches this fresh **70170** action; this is not
server admission/save proof. Name-availability request `440071` (22 bytes) is
also observed but remains unported and was not captured.

World exited cleanly, account online returned to zero, Character row count
remained zero, the exact realm was restored offline/normal, and restored BNet
V1/V2 smoke passed at **01:22:05Z**. No create success was sent. Private captures,
client assets and binaries are not staged or distributed.

The expanded operator data probe adds independent, parameterless
`--ack-character-customization-tables`. CMake/CTest (**1/1 expanded schema
self-test**) and **10 CLI guard tests** pass. Complete normal local CASC
acquisition into private `client-data-customization-70170-20261003T0122Z`
succeeds for all six files, with every section readable and every file mode 0600:

| Table | FDID | Layout | Physical rows | Bytes |
| --- | ---: | --- | ---: | ---: |
| ChrModel | 3384313 | 03FAB755 | 127 | 8342 |
| ChrCustomizationOption | 3384247 | DCC2A86E | 1173 | 31868 |
| ChrCustomizationChoice | 3450554 | 9559C358 | 10447 | 286600 |
| ChrCustomizationReq | 3450453 | CA154412 | 48 (+444 copies) | 4986 |
| ChrRaceXChrModel | 3490304 | A203BC29 | 116 | 2122 |
| ChrCustomizationReqChoice | 3580359 | F925BC6F | 446 | 6976 |

Source `02245dcd` anchors: the six `DB2Metadata.h` structs, corresponding
`DB2LoadInfo.h:1046-1200`, `DB2FileLoader.cpp::LoadHeaders` metadata/section-ID
rules, and `DB2Stores.cpp:1181-1249,2147-2165` model/race/gender/option/choice/
dependent-requirement indexes. ReqChoice uses one file field plus a parent
relationship, not a TactKey-style array. Presence and schema acquisition do not
establish decoded effective fields, official/custom SQL overlays, appearance
validation, Player initialization or durable creation. Those remain the next
implementation boundary; no client-data rows become public test fixtures.

The complete source creation review also identifies a durability boundary:
`CharacterHandler.cpp:1000-1047` generates a GUID, constructs independent
Character/Login transactions, waits for the Character async commit, and only
then submits Login work. `DatabaseWorkerPool.cpp:331-355::CommitTransaction`
merely queues that Login transaction; it does **not** acknowledge durability
before cache insertion/script callback/Create success. A destroyed Session can
drop the callback while Character work still commits (`AsyncCallbackProcessor.h`).
Do not claim that the reference waits for both transactions, or silently hide a
repair of that source behavior inside a compatibility refactor. The Forever
creation implementation still needs an explicit commit/publication/recovery
contract, including cancellation and unknown commit outcomes.

##### Native name-availability wire — 2026-10-03

The new `wow-packet::forever::name_availability` payload codecs follow pinned
`02245dcd245e7433e524577656177723d3e4992e`:
`CharacterPackets.cpp:464-484`, `CharacterPackets.h:302-325`. The request has
LE32 sequence, MSB-first 6-bit name length, three unvalidated bits, 6-bit surname
length, alignment, then two UTF-8 strings. Truncation/trailing bytes reject and
the strings have no Debug implementation. The response encodes exactly two LE32
values (sequence and raw result); candidate server opcode `46001B` is not a fresh
native-response observation. No handler is registered, no available-name reply
or reservation is fabricated, and Create still repeats its own admission checks.

`--ack-private-name-availability-capture <new-private-file>` is a **separate**
opt-in, mutually exclusive with the existing Create capture. It admits only
authenticated opcode `440071`, excludes Create/auth/TACT and uses a 132-byte
bound (six framing bytes plus two strings of at most 63 bytes). The private
create-new/no-overwrite/root/0600 guards remain; its header is `FNR1`, build,
opcode and length. A repeated matching capture rejects rather than overwriting.
The integrated read-only `forever_name_availability_probe.py` independently
checks that framing/layout/UTF-8 and prints only lengths/flags/hash, never names
or sequence. The production observer calls the Rust decoder only for the
explicitly selected diagnostic opcode. Normal non-opt-in Session is unchanged.

Completed-slice acceptance, parent-exclusive one Cargo job on Linux x86_64:

- `cargo test --locked -p wow-packet --lib --timings -- --quiet`: **783 pass**,
  8.01s compilation, timing `20261003T021327419Z`; includes an independent source
  `10 04` / synthetic gear+fd literal, truncation, UTF-8, six-bit bounds and raw
  response results 0/27/98/104.
- `cargo test --locked -p world-server --bin forever-world-server --timings
  -- --quiet`: **4 pass**, 2m41s, timing `20261003T021345008Z`; includes separate
  name/Create allowlists, 132/133-byte boundary, framing, privacy and no overwrite.
- `python3 -m unittest discover -s tools/wow-test-bot -p 'test_forever*probe.py'`:
  **12 pass** (including the existing probe suites); no client/server data writes.
- Normal `cargo build --locked -p world-server --bin forever-world-server
  --timings`: **pass**, 2m39s, timing `20261003T021630095Z`.

At **02:19–02:21 UTC**, the normal installed artifact starts from the seven real
appearance baselines/SQL and admits a fresh native login/digest/ACK/init/empty
enum. Ruleset PvP and human male warrior personalization render. TactKey batches
64/20 receive actual Invalid replies. Synthetic names are entered with the
hash-pinned scoped UI helper. Finish emits **Create `440070`/106 bytes followed
by name check `440071`/22 bytes** in this observation; only the latter is captured.
Rust decoding and the independent Python inspector agree: 9/7 byte lengths,
unknown bits zero. Private artifact `name-availability-70170-20261003T0219Z.bin`
is 38 bytes/mode0600, SHA-256
`475125b51a1515e2986a1ae2467e955a9c5e11ef87f9a2bce25b680a7f67913c`.
No names/raw bytes/client assets are staged. Neither request gets a fabricated
success; the observed ordering is not an admission/persistence contract.

SIGINT exits 0, online=0/key length40, Character count=0. Guarded realm restoration
affects exactly one row; only isolated BNet restarts in warn mode. Restored V1/V2
positive/negative/offline smoke passes and port18085 is closed. The original
installation and official account are untouched.

Next complete operation remains name policy + effective rule data + collision
lookup + ordered result. Exact source: `ObjectMgr.cpp:157-174,8732-8805` uses
utf8cpp UTF-16 units and finite custom case mappings, **not** Rust expanding
Unicode casing; minimum configured length, max12, realm Cfg_Categories creation
charset and triple-repeat checks precede `DB2Stores.cpp:1416-1457,2852-2864`
Perl/icase regexes. NamesProfanity and NamesReservedLocale yield PROFANE;
global NamesReserved yields RESERVED. `ObjectMgr.cpp:8682-8730` loads SQL
`reserved_name` from **CharacterDatabase**, not World; current fixture has zero
rows. `CharacterHandler.cpp:1687-1719` then queries `CHAR_SEL_CHECK_NAME` and
returns 27 if present / 0 if absent. Surname is ignored by that source handler.
The three hotfix SQL tables are empty in this fixture; that is not a substitute
for the required DB2 baselines. Target string offsets and regex/case behavior
must be verified instead of invoking the permissive inherited getters.
Source inconsistency remains explicit: `NamesReservedLocaleLoadInfo` binds the
global reserved SQL statement despite a separate locale statement existing;
do not silently label a repair as source parity.

Starting Player work also uses target World tables `player_racestats` plus
`player_classlevelstats` (`ObjectMgr.cpp:4303-4402`), not an assumed old
`player_levelstats`. Read-only fixture counts are 33 race rows/1170 class-level
rows/292 creation rows; human warrior has one matching start/race/level1 row.
These counts do not prove initialized stats, equipment, save or world entry.
The active macro goal remains selection, durable creation and world entry.

The original complete campaign began **2026-10-02 23:54:58 UTC**; this phase
began 02:13:27 and does not reset that envelope. Coding/error-repair time is not
reliably separable. The **600-second target remains exceeded**. Committed final
publication acceptance for this slice is recorded below, separately from these
scoped/native passes.

Publication final on clean committed **`46eb14157d5de2a73edd19fb73ffb47f09321e9d`**:
`./tools/validation-v2 final --base origin/forever --architecture --timings`
returns **1**, not green. Manifest
`20261003T022215.237024Z-1793592-final.json` records
02:22:15.236–02:23:04.477 UTC / **49.24s**, dirty=false. Physical files pass
(2328 files). Policy (21.646s) and syntax ownership (27.304s) fail the same
inherited hotspot totals and obsolete taunt/creature-insertion/bridge baselines
as the prior appearance final. All three implicated legacy source files are
byte-identical to published `11da286c`; this slice adds no growth to those owners
and changes no ceilings/baselines. Cargo stages are not reached, so the separate
scoped/native acceptance above is not labelled a green whole-final result.
Publication uses the user's existing **unchanged inherited-debt experimental
exception**, not an exemption for new failures. The complete campaign at this
checkpoint is **8886.477 seconds** from 23:54:58; its 600-second target fails.
The following closeout changes only README/runbook/state prose; reused code
evidence retains the exact code-candidate identity above.

##### Checked target name data — 2026-10-03 (local prerequisites validated)

The independent Forever target uses the four complete 70170 WDC5 baselines,
not empty SQL tables as a substitute and not inherited permissive string
getters. The integrated acquisition tool adds the explicit parameterless
`--ack-name-validation-tables`; default acquisition does not read these files.
Each new output is mode0600 under the ignored isolated root, never overwritten.
The normal local CascLib path does not download, invent keys or zero-fill
unavailable sections. Original client files and the official account are untouched.

Fresh private acquisition at 02:27 UTC succeeds with these header contracts:

| Table | Runtime hash / layout | Records / copies | String bytes | File bytes |
| --- | --- | ---: | ---: | ---: |
| NamesProfanity | DA82D96C / F227E638 | 6595 / 0 | 28647 | 108131 |
| NamesReserved | 25C1CB13 / 2B2D5D97 | 2559 / 0 | 30959 | 51703 |
| NamesReservedLocale | 3ACAE305 / 7B9823D4 | 2 / 0 | 14 | 338 |
| Cfg_Categories | C7ED797D / 8710BE94 | 91 / 9 | 690 | 2278 |

Exact target source remains `02245dcd245e7433e524577656177723d3e4992e`:
`src/common/DataStores/DB2FileLoader.cpp:354-376,798-803` materializes concatenated
physical records followed by concatenated pools. The checked target reader
resolves a uint32 displacement relative to its physical source field; copies
retain that source. It checks complete pools/records, address bounds, NUL and
UTF-8 without lossy fallbacks. Zero displacement follows source EmptyDb2String.
Synthetic tests cover cross-section pool strings, copies, invalid addresses,
unterminated/invalid UTF-8 and incomplete pools. Legacy getters are unchanged.

`wow-data::forever_names` owns the immutable effective catalogs: baseline,
official SQL, custom SQL, then final RecordRemoved. Duplicate IDs within an
unordered batch fail; effective Language is signed BYTE and validated only after
overlay/removal. Source anchors `DB2Stores.cpp:1416-1457` and `Common.h:50-65`
establish twelve locales, global profanity excluding none(9), eight locale-mask
bits and Cfg_Categories' English fallback. No pattern values enter diagnostics.
`wow-persistence::forever::names` supplies SQL-free transient DTOs; database
adapters perform four explicit projections per official/custom batch. Startup
requires both batches and all files before publishing prerequisite success.
Known-store registration still rejects Valid hotfixes whose target serializer
is not implemented; DBQuery cannot silently report those stores absent.

Intentional **Forever** contract, not a source-parity claim: locale-reserved
overlays read `names_reserved_locale(ID,Name,LocaleMask)`. The inconsistent
`NamesReservedLocaleLoadInfo` binding to the global reserved statement is not
copied. `HotfixDatabase.cpp:314-317,1281-1295` declares the correct separate
columns/tables; acquisition itself performs no SQL-binding repair.

Pure `wow-world::forever::name_rules` implements normalization, UTF-16 length,
finite source casing, configured character families and triple-repeat rejection
from `ObjectMgr.cpp:157-174,8732-8805` / `Util.h:121-337` / `Util.cpp:370-423`.
It is **not** a full name policy or availability result. Source wide regexes use
Boost Perl/icase/optimize and the UTF-8 process locale; std::wregex is explicitly
unsuitable. Rust regex/Unicode case is not silently substituted. Regex integration,
SQL reserved-name lookup, collision query, registry admission and response delivery
remain open. Startup catalog loading is not a registered handler or a saved Player.

Parent-only prerequisite diagnostic acceptance at 02:27:29 UTC: CMake build -j1
passes (~1s), CTest passes 1/1 (0.01s), CLI tests pass 10 (0.024s). The fresh
actual-file acquisition above passes. Combined library acceptance
`cargo test --locked -p wow-data -p wow-database -p wow-persistence -p wow-world --lib --timings -- --quiet`
passes 795/365/35/4062 tests (database two ignored, world one ignored), compile
159.3s; timing report `20261003T023936200Z`. An earlier compilation failed on
a shadowed new locale array; the related new-code binding/import were corrected
before this passing rerun. Target binary test
`cargo test --locked -p world-server --bin forever-world-server --timings -- --quiet`
passes five tests, compile 3m00s, timing `20261003T024233698Z`. Both commands use
one job and this checkout's absolute target; libraries requiring the inherited
C++ fixture use `/tmp/rustycore-forever-cpp`, not as 70170 behavior authority.
These runs are at HEAD `edce7c7d` with the implementation dirty; a later commit
must not relabel their tested SHA. Code candidate
`03e1ef425eb0df56f46356b25ebc94eb81c9cd5f` contains that unchanged source snapshot.
Normal `cargo build --locked -p world-server --bin forever-world-server --timings`
at this clean commit passes (3m01s, `20261003T024553081Z`). At 02:49 UTC its
normal artifact starts with private `client-data-name-rules-70170-20261003T0227Z`
and actual disposable SQL. All eight name projection queries succeed (the four
SQL tables are actually empty); startup loads 1057 profanity patterns and two
locale-reserved patterns for esES, plus 2559 global-reserved patterns. Existing
appearance/race/achievement/hotfix prerequisites still load. No native session,
name result or save is attempted; realm stays offline/normal. SIGINT exits 0;
read-only restoration checks show online=0/key length40/Character count=0 and
port18085 closed. BNet was not modified or restarted by this startup check.

Publication final on that clean committed candidate:
`./tools/validation-v2 final --base origin/forever --architecture --timings`
returns **1**, not green. Manifest
`20261003T024921.399740Z-1805296-final.json` records
02:49:21.399–02:50:10.924 UTC, **49.524s**, dirty=false. Physical limits pass
(2337 files); policy (21.797s) and syntax ownership (27.405s) fail the same
inherited Session/Map/Character/WorldServer/Quest/Player totals, obsolete taunt/
creature-insertion signatures and two legacy bridges as published `edce7c7d`.
The three implicated legacy files are byte-identical to that base. No related
owner grows and no ceilings/baselines/dependency policy change. Cargo stages
are not reached; the separate library/binary/startup acceptance above is not
labelled a whole-final pass. Publication retains the explicitly authorized
unchanged inherited-debt experimental exception, not an exemption for new errors.
The full campaign at this final end is **10512.924 seconds**; subsequent
documentation-only closeout/hygiene checks are additional time, not a new budget.
This closeout changes only README/state/runbook prose; code evidence retains
its actual precommit or committed identity, rather than being relabelled later.
The campaign still starts 2026-10-02 23:54:58 UTC; its 600-second target remains
exceeded, and coding/error-repair time is not reliably separable.

##### Checked target appearance implementation — 2026-10-03 (local/native validated)

The next delivery uses a private `wdc4::creation` numeric view, leaving legacy
getters unchanged. Source anchors at `02245dcd`: **common/DataStores/**
`DB2FileLoader.cpp:635-696,807-922`, `DB2Metadata.h` and `DB2LoadInfo.h` for
the seven tables. Packed offsets use `PackedDataOffset * 8 + CompressionData`
offset/width, not the first column-offset pair. Typed array strides, palette
cardinality, full palette/common blobs, relationship overrides, extra-parent
zero initialization, ordered materialized copies and `GetMaxId:955-977`'s
declared maximum (including copy IDs) are checked. Unsupported
widths/arrays and missing data fail rather than substituting numeric zero.
ChrRaceXChrModel's race/sex are **byte** fields, not int32; Req race masks are
two int32 cells, not one uncompressed int64. No strings/private names are
included in this projection or its diagnostics.

`wow-data::forever_appearance` consumes seven mandatory real DB2 baselines,
official and custom SQL projections in that order, and final RecordRemoved
statuses from the complete ordered metadata batch. The immutable catalog
preserves `DB2Stores.cpp:1181-1249`: last model wins, options append, one last
derived race per UnalteredVisualRaceID, choices grouped by option and required
choices grouped by requirement/option. This is **not** a full typed DBQuery
serializer for those tables. Startup fails on ambiguous duplicate batch IDs.

The pure `wow-world::forever::appearance` creation validator follows
`CharacterHandler.cpp:559-665`, `DBCEnums.h::ChrCustomizationReqFlag` and
`RaceMask.h::GetRaceBit`, including Forever races 95/96 in the second mask
word. It checks option/choice membership, uniqueness, class/race masks,
source-unconditional achievement rejection, real item appearance ownership,
creation's absent Player/quest state and all dependency groups (any selected
choice per group). Missing requirement records do not invent restrictions.
Invalid classes and unsorted/duplicate/zero options are rejected at its public
input boundary; the target codec sorts the native pairs beforehand.

The modern Create decoder retains name/surname privately, the target optional
template/flags/unknown fields, source int32 season and sorted appearance pairs.
It checks the complete remaining pair region before allocation and enforces
`CharacterPackets.h:57-65`'s **250**-entry source capacity. No opcode handler,
Create success or DB writes are enabled by this codec.

The existing opt-in private native observer is production-linked to the
startup effective catalog and validator. Empty item appearance ownership is
only valid because the admitted-account loader currently rejects nonempty
collections; this must be replaced by the real collection owner when that
boundary is expanded. The observer reports only valid/count metadata and
still does not admit or persist creation. Tests, actual-file/startup and fresh
action acceptance below pass; none of these changes establish playable world
entry or resolve the separate creation transaction/recovery contract.

Initial scoped acceptance (`20261003T014802260Z`, 2m45s compilation) passed
785 data, 364 database (two ignored), 777 packet, 35 persistence and 4058 world
tests (one ignored). Target binary tests passed 3/3 (`20261003T015114132Z`,
3m15s compilation), then normal build passed (`20261003T015459712Z`, 2m57s).
Final source review identified the additional declared-maximum assertion above;
its test and source-correct fixture maximum are added. Affected acceptance and
normal installation are being repeated; the earlier run is not relabeled as
testing that repair. Unchanged database/packet/persistence source retains its
separate green evidence. Timing remains part of the campaign begun at
2026-10-02 23:54:58Z, not a fresh ten-minute campaign. The 600-second target was
already exceeded; no cache/coverage/architecture ceiling was removed.

The repair's affected suites pass **786 data / 4058 world** (one ignored),
`20261003T015836289Z`, 2m34s compilation. Unchanged packet/database/persistence
and private-recorder test inputs retain their separately dated evidence above;
no prior binary-test run is labeled as testing the new reader. The normal
candidate is rebuilt (`20261003T020146289Z`, **35.64s**) and exercised:

- Seven actual private DB2 files and all 14 official/custom SQL projection
  queries load successfully; **116** effective race/gender option indexes.
- Fresh strict native session accepts encrypted initialization and empty enum;
  TactKey batches **48/36** IDs receive real Invalid replies (zero valid).
- Human male warrior personalization is rendered. Fixture UI input and Finish
  submit **440070 / 106 bytes / nine choices**. The normal product's opt-in
  observer decodes/sorts the request and reports **valid=true** against the
  effective catalog. No Create handler/success or Character writes occur.
- Private mode-0600 capture `character-create-70170-20261003T0202Z.bin` is
  122 bytes; SHA-256
  `1be73d5769fd09512411fce05eff18a85d08984bd0b4ba1b6daa160bb59fe6f7`.
  Read-only inspection confirms name/surname lengths 9/7, race/class 1/1,
  sex 0, season 0, no template/duplicate options and unsorted wire pairs.
  Names/choices/assets are not distributed.
- SIGINT exits 0, Character count remains **0**, online=0/key length40,
  exact realm restoration affects one row and World port closes. An initial
  cleanup diagnostic used the obsolete `sessionkey` column and failed before
  its UPDATE; corrected `session_key_bnet` diagnostic/restoration succeeds.
  Restarted warn-level BNet's restored V1/V2 offline smoke passes, restoring
  its 64-byte authentication key. Formatting and diff hygiene pass.

At **02:05:02Z**, the full campaign from 23:54:58Z is **7804 seconds**; the
ordinary 600-second performance target is not met. This additional phase starts
at 01:48:02Z but does not reset that campaign. Repair/coding interleaves and has
no separately reliable wall-clock total.

Appearance publication candidate **`eee5fc3958aee9bca22361506b64049a2609cf75`**
contains the locally/native-validated source unchanged. Clean-candidate
`validation-v2 final --base origin/forever --architecture --timings` fails
exit1, **49.984s**, manifest
`20261003T020650.044248Z-1786793-final.json` (02:06:50.044–02:07:40.027Z).
Physical ratchet passes **2325 files**. Hotspot counts are unchanged from the
preceding publication (21.696s), and syntax ownership reports the same obsolete
taunt/creature-insertion signatures and two inherited bridges (27.756s).
The three implicated legacy files remain byte-identical to `5abe8c04`.
No policy ceiling/baseline is relaxed and no new failure is hidden. The runner
stops before Cargo; the separately executed scoped suites/normal build/native
evidence above are not described as a green full-final run. The existing
experimental exception applies only to this unchanged inherited debt.

The final campaign checkpoint at 02:07:40.027Z is **7962 seconds** from the
original start, not a compliant 600-second campaign. This documentation-only
closeout retains those exact candidate/evidence boundaries. Only `forever` is
authorized for publication; remote `3.4.3` is still
`24a513855e1d4c55f208cc3e53a5f23973b5f675`. No PR/merge into that branch,
runtime deployment or playable-world completion is inferred.

`Player.cpp:391-561::Create` and `20665-21078::SaveToDB` require more than a
main Character insert: effective starting position/stats/models/skills/spells/
items and all creation-linked saves. Source homebind INSERT is deferred to
first login; a SQL-only row or artificial successful GUID is not acceptance.

Publication validation on clean code candidate
**`3c56508ebe3b1a3031112b744a98ae2f5666f49b`** ran
`validation-v2 final --base origin/forever --architecture --timings`:
manifest **`20261003T012648.324135Z-1762789-final.json`**, **51.206s**, exit 1.
Physical source ratchet passes **2313 files**; no newly introduced file/size
failure. The same inherited Session/Map/Character/WorldServer/Quest/Player
hotspot limits remain red (22.547s), followed by the same obsolete taunt and
creature-insertion signatures and two legacy bridge baselines (27.705s).
All three affected legacy source files are byte-identical to `d0bb654c`;
the Session LOC values remain 95306 production / 128635 test / 223941 total,
unchanged from that preceding publication. No ceiling/baseline was relaxed.
The runner stops before downstream Cargo/hygiene; the separately executed
scoped suites, installed normal binary/native actions, fmt and diff checks
above are not a green final. The existing operator waiver is for experimental
Forever publication of inherited debt only, not parity or merge/deployment.
The full account/character campaign already reaches **5561 seconds** at this
final's 01:27:39Z checkpoint; closeout/publication extends it. The 600-second
performance target remains unmet. Subsequent documentation-only closeout has
unchanged executable inputs and is validated separately, not relabelled as
having been tested by this earlier final.

At **2026-10-02 22:51:27 UTC**, the operator-only
[`client-data-probe`](../../tools/wow-test-bot/client-data-probe/README.md)
read two actual files from the installed `wow_classic_beta` build **70170**, using
the hash-pinned reference's CascLib. The installation was opened read-only;
no login, account operation, download, imported missing key or zero-filled
fallback was used. Private outputs are mode 0600, in the ignored fixture only.
They are not redistributable repository fixtures.

- `ChrClasses`: FDID **1361031**, **6200 bytes**, WDC5/version 5; 9 records,
  43 fields, table hash `F5889D8C`, layout `AFC9B0C2`; inline ID column 29.
- `ChrRaces`: FDID **1305311**, **22095 bytes**, WDC5/version 5; 58 records,
  51 fields, table hash `53F1783C`, layout `4F44C796`; external 232-byte ID list.

Both files have one unencrypted DB2 section and no pallet/common data or parent
lookup. The target schemas match `02245dcd`'s
`src/server/game/DataStores/DB2Metadata.h::{ChrClassesMeta,ChrRacesMeta}`.
`src/common/DataStores/DB2FileLoader.h::DB2Header` has **204 bytes**, including
version and a 128-byte schema field; `DB2FileLoader::LoadHeaders` checks WDC5,
version 5 and metadata layout/field compatibility. WDC4's 72-byte header is not
interchangeable. Existing `character_progression` field offsets remain inherited
and are **not** used as the target character-availability contract.

The first acquisition selected `enUS` and failed with **CASC error 1007
(`ERROR_FILE_OFFLINE`)**. The installed client selects `esES`; opening that
storage locale succeeded. In this pinned `CascOpenFile.cpp`, the file-open locale
argument is ignored: selection belongs to storage/root opening. This is evidence
for these files, not a general claim that every missing file is a locale mismatch.

Acquired rows/IDs alone do not define playable combinations. The target world
SQL race/class expansion and race-unlock requirements, actual Character/Auth
query-holder results and the ordered initialization packets are still required.
There is no admitted WorldSession, character creation or initial world-load
acceptance at this boundary. A presence catalog must not advertise every DB2
race as playable, or silently fill an absent DB result with invented values.

The native `AuthResponse` decoder was independently traced through dispatcher
RVA `8E11F0` → `7EEFC0` → callback/wrapper `232DCD0`: a u32 result and flags
(`bit7` success, `bit6` wait). The error body `03 00 00 00 00` is complete;
the earlier waiting UI does not prove a missing field. Native Pong descriptor
`A1AF10` gives **`0x4D0009`**, and dispatcher `A1AF50` reads a serial-only u32
before callback `1F2A970`. These are format/ID findings, **not native acceptance
of a newly sent Pong or successful AuthResponse**.

The implemented data boundary reuses the binary reader, not the inherited
typed field offsets. `wow-data::forever_character_ids::ForeverCharacterIds`
requires WDC5 and exact table/layout/field/ID-source metadata, rejects zero,
duplicate and unresolved-copy IDs, and owns only immutable presence sets.
`forever_character_tables` is a read-only production-linked example, not a
WorldSession integration. Sparse/encrypted WDC5 is rejected. Only the target
regular header/ID path is claimed; remaining inherited compression, strings and
relationship accessors have no new whole-WDC5 parity claim.

The remaining complete admission operation is source-backed by `02245dcd`:

- `WorldSocket.cpp::HandleAuthSession` / encrypted ACK adds the admitted session
  only after credential, permission and encryption transitions.
- `WorldSession.cpp:1273–1388`, both account query holders, requires real
  CharacterDB `account_data`, `account_tutorial`, `account_instance_times` and
  Auth collection, pet/slot, realm-count, appearance/transmog/warband/player-data
  queries. Zero rows are legitimate for a fresh account; absent tables/failed
  queries are not equivalent to zero rows.
- `WorldSession.cpp:1392–1480` joins both holders, loads state, then publishes
  AuthResponse, timezone, glue status, cache version, available hotfixes, global
  account-data times, tutorials and BNet connection status in source order.
  Character counts and battle pets are loaded into their owning session state.
- `AuthHandler.cpp::SendAuthResponse` and
  `ObjectMgr.cpp::LoadRaceAndClassExpansionRequirements` / race unlock loading
  require actual `class_expansion_requirement` and `race_unlock_requirement`.
  The custom `sql/custom/world/2026_09_27_00_world_forever_baseline_01.sql` and
  `_02.sql` add availability/unlock rows for 95/96; they are not a complete
  world database.
- `CharacterHandler.cpp::EnumCharactersQueryHolder` / enum callback and
  `CharacterDatabase.cpp:68–87` require expired-ban cleanup, enum/customization
  queries and availability/unlock state. A truly empty account may return a
  valid empty enum; that does not authorize inventing creation options.

The fork's extra recent-ally response is commented for **70009**, not exact
70170 native evidence. It remains a hypothesis until action-specific capture.
Only the isolated Auth fixture currently exists; the required Character/world/
hotfix data and real session owner must be integrated before successful admission.

#### Character-data candidate acceptance

At committed **`ba74a2ceef57e819625a603057d628aa8a45fc52`** on Linux x86_64:

- Full final manifest `20261002T230637.362191Z-1667649-final.json` passes
  whitespace, Python syntax, rustfmt and physical files (2289), then fails the
  unchanged inherited hotspot ratchet in 27.799s. Protected gameplay/runtime/
  architecture paths remain byte-identical to fork `2df57d6f`; no limits changed.
- `cargo test --locked -p wow-data -p wow-network --lib --timings`: **760/37
  passed**, build 37.80s, tests 0.02/0.30s; timing `20261002T230712006Z`.
  Existing asset-conditional tests do not prove installed-data acceptance.
- Strict fixture **4 passed**, build 3.17s; timing `20261002T230757003Z`.
  BNet binary **133 passed**, build 2.22s, tests 24.30s;
  timing `20261002T231052799Z`. Release fixture build **passed, 24.01s**;
  timing `20261002T231139702Z`.
- Actual-file `forever_character_tables` consumer **passed**, build 31.26s;
  timing `20261002T230835420Z`. It reads 9/58 IDs and confirms presence of
  95/96. A fresh acquisition by the committed C++ tool also succeeds and is
  decoded by that binary; client assets remain private and uncommitted.
- Pinned CMake configure/build, **one CTest header contract**, **six CLI guard
  tests** and **nine production-linked private-copy negatives passed**. The
  negatives cover table/layout, ID source, truncation, old format, sparse,
  encrypted sections, short ID lists and missing copy sources. Only temporary
  private copies are changed; no installation/account/database mutation.

At **23:13–23:14Z**, the installed `ba74a2ce` release fixture repeats strict
native proof, 40-byte persistence, signed ACK and encrypted ping. It sends
serial-only Pong followed by encrypted ERROR_DENIED=3; the fresh UI shows
**BLZ51900003**, consistent with that deliberate denial. There is no successful
AuthResponse or WorldSession, and Pong's native latency callback was not observed.
No further client frame arrives within five seconds. The guarded realm restore
returns flag/icon to 2/0, port 18085 closes, BNet restarts at warn level and its
restored-offline smoke passes. Structured metadata is ignored at
`target/forever-login/data-world-probe-20261002T2314Z.json`.

Acceptance for this completed data slice began **23:06:32Z**; publication
closeout records the measured bounds. The preceding acquisition/bootstrap/coding
phase is separate, not a warm acceptance run. The earlier world-crypto campaign
extended through its 22:35:08Z smoke (**2626 seconds** from 21:51:22Z), so its
600-second target remains **not met**, not reset by this slice.

At documentation-only candidate **`61ab7b9a`**, committed publication final
`20261002T231549.927099Z-1672760-final.json` again passes hygiene/physical checks
then fails the same inherited hotspot limits before Cargo. The code/test inputs
are unchanged from `ba74a2ce`, so its green scoped and native evidence is reused
with its actual tested SHA, not relabeled as a new build.

The measured complete-campaign checkpoint **23:06:32Z–23:16:46Z is 614 seconds**,
including the required data, negative, transport/runtime and closing checks.
The **600-second target is not met**, even before the remaining documentation
validation/publication closeout. No command was removed or reclassified to claim
a warm pass. Exact final endpoint and publication candidate are retained in the
ignored structured scenario metadata named above. The initial data-example
subset needed additional compilation (31.26s); this is not a clean warm-cache
performance benchmark. Closing checks extend this envelope, not a new campaign.

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
