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
The native client also passes strict World `AuthSession` digest verification,
signed encryption, canonical Forever account initialization and database-backed
empty enumeration. The creation UI and a human warrior preview have been
observed. Character creation/persistence, nonempty enumeration and world entry
remain pending. This uses `forever-world-server`, not the legacy full
`world-server`; historical login-only probes below retain their dated boundaries.

The procedure is operator-only. It mutates the disposable Auth database and
issues normal login/ticket requests. Do not point it at a shared realm, reuse a
non-empty schema, or put a password, database URL, certificate, private key or
session ticket in a command line, report or log.

## Working spell-container build prerequisite

The unpublished creation candidate requires `forever-name-regex`,
`forever-spell-traversal` and `forever-spell-random` for `forever-world-server`.
The traversal adapter replays
only startup keys using the target source's std/Boost container shapes; domain
corrections and raw records remain in Rust. This is an explicit **fresh-startup
reference-server contract**: Linux x86_64 GNU, GCC 15.2.0, libstdc++ release 15
headers dated `20260321`, Boost 1.83.0 and non-debug containers. It is not a
client requirement or an assertion that all C++ toolchains have identical
traversal. Reload with retained source bucket capacity is not implemented.
The legacy binary has no dependency on this feature. Builds refuse unsupported
compiler/host/header inputs and do not download, generate or repair vendor code.

Prepare the official source archive separately. Its SHA256 is
`6478edfe2f3305127cffe8caf73ea0176c53769f4bf1585be237eb30798c3b8e`, from
the [official Boost release manifest](https://archives.boost.io/release/1.83.0/source/boost_1_83_0.tar.bz2.json).
If either target already exists, inspect it instead of overwriting it:

```bash
mkdir -p target/forever-login
test ! -e target/forever-login/boost_1_83_0.tar.bz2 && \
  curl --fail --location --output target/forever-login/boost_1_83_0.tar.bz2 \
  https://archives.boost.io/release/1.83.0/source/boost_1_83_0.tar.bz2
# Extract only after a successful check into an absent directory.
printf '%s  %s\n' \
  '6478edfe2f3305127cffe8caf73ea0176c53769f4bf1585be237eb30798c3b8e' \
  'target/forever-login/boost_1_83_0.tar.bz2' | sha256sum --check --strict && \
test ! -e target/forever-login/boost_1_83_0 && \
  tar --extract --bzip2 --file target/forever-login/boost_1_83_0.tar.bz2 \
  --directory target/forever-login boost_1_83_0/boost boost_1_83_0/LICENSE_1_0.txt
```

`FOREVER_BOOST_HEADERS_ROOT` can select another extracted directory with **only**
`boost/` and `LICENSE_1_0.txt`. Build admission hashes every file and relative
path in ASCII order, rejects symlinks and extra root entries, and requires the
SHA256 of the GNU checksum-line manifest:
`6442dc47d86b5c718d698d65a33ab098c13074a56ff2d7ca695689ffac41b711`.
That fingerprint was obtained from the locally downloaded, archive-hash-checked
official extraction; it is not a compiled traversal result. No vendor source
or archive is staged. The separate Regex checkout below is still required.

The random capability compiles pristine SFMT-19937/SSE2 and SFMTRand, plus
verbatim GetRng/frand/rand32 source fragments from immutable Git objects at
`02245dcd245e7433e524577656177723d3e4992e`. Source defaults to
`target/forever-cpp-reference`; `FOREVER_CPP_SOURCE_ROOT` may select another
Git object store containing that exact commit. Mutable physical checkout files
are not inputs. Dependency copies are generated only under Cargo `OUT_DIR`,
with their notices retained. No download or runtime reseed is performed.
The same pinned GNU/x64 contract applies; legacy builds do not require it.
SFMT's notice in [docs/licenses/SFMT.txt](../licenses/SFMT.txt) must accompany
distributed binaries. This working capability is **uncompiled**, and full
custom/positivity phase integration remains pending; its existence does not
admit character creation or constitute numeric parity evidence.

At completed-delivery acceptance, the parent must run both the native source
comparison and target-binary tests in the same exclusive campaign, alongside
the affected library/actual-file/final/live evidence:

```bash
python3 tools/wow-test-bot/test_forever_spell_traversal_oracle.py
python3 tools/wow-test-bot/test_forever_spell_random_oracle.py
cargo test --locked -p world-server --bin forever-world-server \
  --features forever-name-regex,forever-spell-traversal,forever-spell-random --timings -- --quiet
```

These commands are **not executed for the working candidate**. The native
driver obtains exact Hash.h and Difficulty declarations from pinned Git
objects at `02245dcd245e7433e524577656177723d3e4992e`, not sparse/modified
physical reference headers. It builds in a disposable temporary directory;
there are no DB, client, asset, network or live-mode operations. Its independent
source-shaped containers must match both complete output orders and each ID's
`equal_range` relative order. This is source-container evidence, not playable
character/world or a replacement for real captures. Earlier commands/results
below retain their dated published-code feature sets.

## Native Forever name engine

The name adapter uses `--features forever-name-regex`; the current working
Forever binary additionally requires the two spell adapters documented above.
Its composition adapter uses Boost.Regex 1.83, pinned to official source commit
`4cbcd3078e6ae10d05124379623a1bf03fcb9350` (tag `boost-1.83.0`). Builds never
download headers and reject a different commit or modified/untracked include
files. Prepare that source separately; do not commit it or client data:

```bash
git clone --branch boost-1.83.0 --depth 1 https://github.com/boostorg/regex.git \
  target/forever-login/boost-regex-1.83
cargo build --locked -p world-server --bin forever-world-server \
  --features forever-name-regex,forever-spell-traversal,forever-spell-random --timings
```

If the checkout already exists, inspect its identity instead of overwriting it.
`FOREVER_BOOST_REGEX_ROOT` may point to another clean checkout of the same pin.
The present ABI is Linux `wchar_t32`, with each UTF-16 unit widened separately;
Windows and other host behavior are not claimed. Standalone Boost.Config is
absent, so both bridge and oracle explicitly enable `BOOST_HAS_THREADS` and its
vendor cache synchronization. Parallel Rust tests and an eight-thread compilation/
matching regression exercise that contract; test serialization is not the fix.
The adapter explicitly imbues
the source's composed environment locale without changing process-global locale.
Its standalone oracle compares that to source-style default `boost::wregex`
construction under the same global locale. A missing locale or failed pattern
compilation refuses startup; matching/query failures never become “available.”

The canonical authenticated registry owns `440071`; ordered realm delivery uses
`46001B` and the exact sequence/result pair. Source anchors at target
`02245dcd245e7433e524577656177723d3e4992e` are
`CharacterHandler.cpp:1687-1719`, `ObjectMgr.cpp:8682-8805`,
`DB2Stores.cpp:1416-1458,2852-2864`, `Regex.h:21-32`,
`Locales.cpp:28-41`, `CharacterDatabase.cpp:53`, and
`Opcodes.cpp:345,1431`. Profanity/locale-reserved precedes global-reserved,
then SQL reserved, then the normalized-name collision read. Success is raw `0`,
occupied is `27`; this is not a name reservation or Create success.

This isolated composition loads ordinary-security default RBAC and follows its
valid linked permission graph for permission `17`, which bypasses **only** SQL
reserved names. Explicit account grants/denials or nonzero account security
remain rejected, rather than silently ignored. SQL reserved names come from
CharacterDB; collision uses `SELECT 1 FROM characters WHERE name = ?`, without
account or soft-delete filters. Account locale is read and mapped through the
currently loaded esES-only DBC locale/default; charset comes from the real realm
timezone category. MinPlayerName defaults to two and clamps to 1–12; strict mask
defaults to zero. There is no new legacy Player/session mirror.

The existing private name-capture opt-in now also records a non-overwriting
`<request-file>.result` sidecar **after** transport send. Its `FNS1` header is
build/opcode/payload-length LE32 followed by the eight-byte result payload.
Both files remain private; the integrated read-only probe can compare their
sequence and layout without printing names or sequence values. This server-side
observation does not establish native client acceptance, creation or persistence.
Current local acceptance, on `60384cb47c97de030f896aef4ab78c2e95ae2c06`
with the implementation delta:

```bash
g++ -std=c++17 -O2 -pthread -DBOOST_REGEX_STANDALONE \
  -Itarget/forever-login/boost-regex-1.83/include \
  crates/world-server/src/forever/name_regex/bridge.cpp \
  crates/world-server/src/forever/name_regex/oracle.cpp \
  -o target/forever-login/name-regex-oracle
./target/forever-login/name-regex-oracle
cargo test --locked -p wow-data -p wow-database -p wow-persistence \
  -p wow-world --lib --timings -- --quiet
cargo test --locked -p world-server --bin forever-world-server \
  --features forever-name-regex --timings -- --quiet
# From tools/wow-test-bot:
python3 -m unittest -v test_forever_name_availability_probe test_forever_character_create_probe
```

Parent-exclusive, one Cargo job, same absolute checkout target and private
protoc path. Oracle **PASS 11**, with one expected invalid-expression error per
engine and zero locale-restore errors. Libraries **PASS 795/366/35/4071**
(database two ignored, world one ignored); compilation 2m01s, timing
`20261003T030619005Z`. Target **PASS 13**, compilation 6.92s, timing
`20261003T031405477Z`; Python **PASS 9**. Earlier target attempts failed native
archive linkage (`20261003T030828324Z`) then SIGABRT in Boost's unguarded cache
(`20261003T031139698Z`). Explicit consumer linkage and `BOOST_HAS_THREADS`
repair those findings; passing evidence retains default parallel tests plus
the eight-thread/256-compilation regression. No test serialization or vendor
header modification is used. Library code is unchanged by those native fixes.

### Name availability native acceptance — 2026-10-03

Committed candidate `4ff81a1c399e8f76f296bef25f3bda2087dda103`, clean tree:
normal build with the feature passes in **2m58s**, timing
`20261003T031517367Z`. At **03:18 UTC**, all actual tables, overlays/removals,
SQL reserved/default RBAC and all locale/global expressions load and compile.
Startup retains esES 1057/2 and global 2559, appearance 116 indexes,
availability 33 races, achievements 434, metadata 3052 and TactKey 522.

Fresh native build-70170 sessions on the isolated ordinary account:

| Action | Fresh server-side request/result | Observed native UI |
| --- | --- | --- |
| Normal nine-letter synthetic name, default minimum two | `440071`, 22 bytes (9/7 name/surname, unknown bits zero); `46001B`, 8 bytes, raw result **0** | Green name check; Finish enabled |
| Triple-letter synthetic name | **No name request sent** | Local red rejection; not evidence of server result 107 |
| Same valid-length client input, temporarily configured server minimum ten | `440071`, 22 bytes (9/7, unknown bits zero); `46001B`, 8 bytes, raw result **99** | Red name check; Finish disabled |

The private capture opt-in records only one request/result pair per process;
the normal binary is restarted sequentially for distinct observations. Both
paired captures pass the independent integrated probe's exact header/build/
opcode/length/sequence checks, without rendering names or sequence values.
The probe deliberately keeps its own client-acceptance/creation flags false:
native UI observation is separate evidence, not inferred from server send.
No Finish/Create is sent in these checks. Live occupied-name, DB2/SQL-reserved
rejection and real query-failure injection are **not** established by these
native cases; those branches have scoped operation tests and source contracts.

Private request/result SHA-256 evidence (files are not distributed):

- Default request `b5c7ca774150e7c81bb13cfd1fc59b7f1082ef7f049f856defe1852aa9da0d3e`;
  result `91cf19fee479ebda9d54b8c54cc73dfb94c657a3a4957fb9d999bb07b9fe1bf2`.
- Minimum-ten request `71082b11d02962df903946e48e2e5943fedc91fdffa865611db75b7d9f957075`;
  result `35ec855ad1687319ac29909b963049cfc984620155ae0de39e32d69120a5bf3c`.

At **03:24 UTC**, SIGINT exits zero, the temporary private MinPlayerName line is
removed and config remains mode 600. Guarded realm restoration affects exactly
one fixture row (offline/normal 2/0); account online=0, persisted world key length
40, Character count=0 and port18085 closed. BNet was not restarted or modified;
restored positive/negative REST and V1/V2/offline-join smoke passes. Its RPC
logon record reports a 64-byte auth key; that is protocol metadata, not a
claim that the stored world key changed. The final database read still shows
online=0/key length40, Character count=0 and realm 2/0. The original official
client/account are untouched.

Committed publication command:

```bash
./tools/validation-v2 final --base origin/forever --architecture --timings
```

**FAIL 1**, manifest
`target/validation-v2/manifests/20261003T032416.991151Z-1817919-final.json`,
head `4ff81a1c`, dirty=false; **03:24:16.990–03:25:07.000 UTC**, **50.009s**.
Physical ratchet **PASS 2344**. Policy 21.947s and syntax ownership 27.705s fail
the same inherited hotspot totals, taunt/insertion signatures and two legacy
bridges. Implicated legacy paths and architecture tools remain byte-identical
to `60384cb4`; no policy limits/baselines change. Cargo final stages are not
reached; credit the explicitly executed scoped suites above, not a green final.
The existing experimental publication exception is only for this unchanged
inherited debt. New failures/growth are not waived.

Creation, nonempty enumeration and initial world loading remain open. The
complete campaign still starts at `2026-10-02 23:54:58 UTC`; at this final's end
it totals **12609 seconds**, exceeding the 600-second target. Coding/error-repair
time is not separately measured reliably; these runs are not a new ten-minute
campaign. Restored BNet smoke and documentation-delta checks occur after that
final and remain additional campaign costs.

Documentation-only `9cd5a501fc83d9ae5e7464fbe4b7845bca15739d` is checked by
`quick --base 4ff81a1c399e8f76f296bef25f3bda2087dda103`: **PASS**, dirty=false,
manifest `target/validation-v2/manifests/20261003T032634.368688Z-1818137-quick.json`,
03:26:34.368–03:26:34.433 UTC, 0.065s. Only README, STATE and this runbook differ;
the actual built/live-tested code SHA remains `4ff81a1c`, not relabeled as the
documentation SHA. Campaign elapsed at that check's end is **12696.433s**.
The subsequent metadata clarification here does not change executable inputs.

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

##### Independent creation sources and local-list codecs — 2026-10-03

The active full goal remains durable creation, populated selection and world
entry. `forever` has no 3.4.3 compatibility obligation. The next foundation uses
the pinned Forever C++ `02245dcd245e7433e524577656177723d3e4992e`, not legacy
Rust as correctness proof. No creation handler, GUID allocation, save or success
response is enabled by this source/codec delivery.

`wow-persistence::forever::creation` defines one SQL-free startup batch;
`wow-database::forever::ForeverCreationWorldRepository` reads all eight World
SQL families in the order of `ObjectMgr.cpp:3855-4458::LoadPlayerInfo`. The base
definition keeps all sixteen columns, independent nullable NPE coordinates/
transport and intro movie/scene fields. Optional item/custom-spell/cast-spell/
action/XP tables may be genuinely empty; query/decode failures are fatal, and
empty definitions/race stats/class stats fail closed. No old prepared-statement
offset or 3.4.3 transport-template join is used. This is a complete query batch,
not a cross-query SQL snapshot or a validated PlayerInfo.

`wow-world::forever::creation::WorldSources` consumes that batch as the single
immutable source owner composed in the target binary. It filters race/class
identities against final effective target records, preserves supplementary rows, and exposes
unvalidated definitions, primary-stat derivation, source-contrasted normal-location
admission, effective XP lookup/base-mana conversion and metadata counts.
The opt-in native Create observer can report source presence alongside appearance
validation; neither observation admits or saves a Player.

Two exact target differences make the legacy stats projection unsuitable:
`ObjectMgr.h:628-631::PlayerLevelInfo` stores signed **int32** stats, whereas
legacy Rust uses `u16`; and `ObjectMgr.cpp:4303-4395` allocates/composes stats
only for races present in `player_racestats`. Missing racial rows must not turn
into zero modifiers. Gap fallback is decided on strength **after** adding the
racial modifier, not on raw class strength. The new target derivation preserves
negative/wide values, requires nonzero combined level-one strength, fills later
gaps from the preceding combined row, and rejects integer overflow rather than
manufacturing usable stats. Configured level cap and validated map/model pair
admission remain separate requirements. No legacy gameplay path is modified.

Local initialization acquisition (2026-10-03, working candidate over `ccb99f8c`)
now uses a separate guarded `--ack-character-initialization-tables` mode in the
integrated client-data probe. Source contracts are `DB2Metadata.h::{Map,
PowerType,ChrSpecialization,ChrClassesXPowerTypes,Movie}Meta`, `DB2LoadInfo.h`,
`map_extractor/System.cpp::ExtractGameTables` and `GameTables.h` at `02245dcd`.
Offline reads obtain complete PowerType (six records, hash `8D899A57`, layout
`14BBEEA1`), ChrSpecialization (ten records, `A00F8E60` / `DAB4CA4B`) and original
`gt/BaseMp.txt` / `gt/HpPerSta.txt` / `gt/xp.txt` (8129/890/3347 bytes; 15/1/5
float columns). No account, installation or DB is changed.

The next successful all-catalog acquisition uses private directory ending
`20261003T0414Z`, including customization/name/TACT/available-Achievement and all
five initialization files. It additionally obtains complete
ChrClassesXPowerTypes (15 rows, 481 bytes, hash `C0315ACF`, layout `70DA1F8C`,
one physical byte field plus class parent lookup) and Movie (three rows, 470
bytes, hash `032DFA13`, layout `F53888FA`, six fields). Contracts are
`DB2Metadata.h:3516-3538,15329-15351` and `DB2LoadInfo.h:1034-1046,4165-4180`.
No field/hash is inferred from 3.4.3. Acquired client files remain ignored/private.

Strict Map acquisition fails on missing section keys even after the existing
private public-provenance key import. Separate explicit
`--ack-available-initial-map` preserves an unchanged 9574-byte `Map.available.db2`
prefix, hash `BD84CD62` / layout `D43AFAC3`: 71 plaintext records and three
unknown sections with 2/5/1 records. This follows
`DB2FileLoader.cpp:1915-1933::DB2EncryptedSectionHandling::Skip`. No Map.db2 is
manufactured, excluded rows are not zero-filled, and unknown map admission is
not permitted. Private acquisition directory ends `20261003T0401Z`; assets
remain ignored and must not be published. Both prior failed-acquisition
directories are retained as evidence, not reused or removed.

The completed acquisition tool builds locally with one job; its final synthetic
header/prefix self-test and ten pre-storage CLI guard tests pass (exit 0).
Two ordinary acquisitions correctly fail; the explicit available-map acquisition
then succeeds (exit 0). This isolated data-bootstrap diagnostic is separate from
ordinary Rust acceptance and does not reset the already exceeded 600-second
campaign. No new Cargo/library/native Player test is claimed.

New `wow-data::forever_initialization::InitializationRecords` reads the five
numeric baselines through the checked target reader, with explicit complete vs
available-prefix Map selection. The transient batch is consumed by
`InitializationCatalog` after official/custom overlays and final removals; it is
composed into target startup and the opt-in observer, **not** Player creation.
`wow-persistence::forever::initialization` owns SQL-free DTOs;
`wow-database::forever_hotfix::initialization` reads both five-table batches;
the target binary consumes them without another mutable authority. Query/decode/
duplicate failure publishes no catalog, and independent SQL reads do not claim
a snapshot. Signed width anchors are `HotfixDatabase.cpp:378-380,452-457,
1198-1204,1271-1274,1389-1391` and the corresponding base SQL tables.
The specialization
reader distinguishes DB2Meta parent field 4 from its observed physical
ParentLookupCount=0, as permitted by `DB2FileLoader.cpp:1773-1792`; existing
tables retain their own physical-header gates. Written synthetic regressions
cover prefix/ordinary-reader isolation, unavailable-ID preservation, drift,
extent and in-record-parent handling, but have **not** executed yet.

The effective indexes contrast `src/server/shared/DataStores/DB2Store.cpp:
127-134::DB2StorageBase::LoadFromDB`, `DB2Stores.cpp:1165-1178,3489-3494` (class
powers ordered by class/type, duplicate pairs keep first ascending ID),
`1267-1282,2189-2200` (specialization index-four preference/fallback, pet override
storage distinct from Player classes), and `1491-1502` (first power ID wins;
unknown/negative enums remain unindexed). Corrupt negative specialization/power
indexes and more than ten unique class powers fail closed instead of following
unsafe source array access. Movie is a numeric/presence projection, not a full
locale/string/wire serializer. All five tables register as known typed hotfix
stores; the existing Valid-status/full-wire guard remains, not false Unknown.

`WorldSources::normal_start` contrasts `ObjectMgr.cpp:3881-3928`,
`MapManager.h:93-95`, `MapManager.cpp:377-379`, `GridDefines.h:199-216` and
`Position.cpp:207-214`: effective map presence, all XYZ finite/bounded, finite
orientation, types 1..5 instance rejection and **both** gender models. Only
source-contrasted pure coordinate/orientation math is reused. Negative exact
TAU remains TAU; negative zero retains its bits. Missing/removed/unknown maps
have no fallback. This normal-location view does not admit NPE transport,
intro movies/scenes or the complete PlayerInfo. Four meaningful normal-start
regressions are written but unexecuted.

The separate target GameTable loader intentionally admits only a conservative
finite decimal asset subset pending full actual-file source-oracle comparison.
Linux source `StringConvert.h` uses `std::stold` then float conversion, whereas
the Windows branch uses `from_chars`; malformed values can become source zero
and some extreme/special values remain source nonfinite. Any unsupported asset
number must fail closed in the target loader, not silently substitute a new value.
This is a stricter documented asset-admission boundary, not full StringTo parity
or a legacy gameplay repair. No Player health/mana/XP path may be enabled from
these raw files before effective data, source-oracle and level-coverage evidence.

The target-only `wow-data::forever_game_tables` module and synthetic tests
are now written and reviewed. It owns all three immutable vectors, keeps the
unused zero row, uses physical order and fifteen-column class mapping, and
publishes no partial three-file result. Root integrates its export plus the
read-only `forever_initialization_tables` example, which requires private-root
and explicit available-map acknowledgement and prints counts/fingerprints only.
Header CR retention and first-CR data truncation now match source
`Util.cpp:57-74,797-804`, rather than Rust `str::lines`/trailing-only CR handling.
Neither the new Rust suites nor that actual-file example have executed. Earlier
`cargo fmt --all` and `git diff --check` exit 0 apply only to their earlier dirty
inputs; these hygiene checks are not acceptance. No new
commit/push, Create handler, Player save or world runtime is introduced here.

Integrated read-only QA now owns `GameTableOracle.h`, `gt_oracle.cpp` and
`test_gt_oracle.py`. The Linux source-style oracle uses full-consumption
`std::stold` then f32 and source `.value_or(0)` behavior; it is source-contrasted,
not a linked complete C++ server. Its canonical LE-f32 FNV-1a fingerprint covers
every numeric column/physical row, including unused zero, without emitting cells
or claiming cryptographic integrity. Commands on the working candidate over
`ccb99f8c`, 2026-10-03 04:20 UTC:
`cmake --build target/forever-login/client-data-probe-build -j1` succeeds;
`ctest --test-dir ... --output-on-failure` passes 1/1 synthetic test;
`FOREVER_INITIAL_GT_ORACLE_BIN=... python3 .../test_gt_oracle.py` passes five,
with the actual Rust diff explicitly skipped (one). The read-only
`forever-initial-gt-oracle --ack-private-initial-gt-oracle <private-0414Z-dir>`
succeeds: each table has **124 rows**, fingerprints respectively
`12129432390339173315`, `3040788807092638946`, `14413540324253136651`.
Actual-file Rust-vs-C++ differential QA remains unexecuted. These separate
bootstrap diagnostics neither reset nor satisfy the exceeded 600-second campaign.

`creation::progression` now consumes raw XP overrides into one immutable vector.
Source anchors `ObjectMgr.cpp:4409-4458,4461-4476,7990-7995`: physical GT row
order, valid unsigned-float truncation, SQL Level=0 permitted, overrides at/above
configured cap ignored, then below-cap zero gaps take prior XP+12000 with defined
uint32 wrapping. Out-of-range lookup returns source zero; missing vector coverage
and undefined/nonfinite/negative float casts fail startup, not fabricated XP.
Base mana uses all fifteen target class columns and clamps row lookup to configured
cap; missing rows are errors. Five positive/negative regressions remain unexecuted.
The target bootstrap retains `World.cpp:752` / `SharedDefines.h:107-137` reference
MaxPlayerLevel default **90**, clamp **1..123**, only as source configuration.
That retail-shaped constant is **not** native Forever playable-level proof; actual
target starting/max-level policy and complete Player admission remain required
before Create. No health, initial skills/items/spells, GUID or save is inferred.
Fresh read-only isolated SQL still reports Character count **0**; World is stopped.

After combining the five-store/normal-start/progression/oracle working delta,
`cargo fmt --all` and `git diff --check` both exit 0. Largest newly owned files
remain below 1000 physical lines (probe main 946, local-list codec 965; other
new catalog/application modules 107-385). No acceptance/baseline ceiling was
changed. These are hygiene/source-navigation observations, not Cargo compilation,
native creation, durability or publication evidence. The goal remains active.

The subsequent seven-store working delta adds numeric **ChrClasses and ChrRaces**
to that same effective immutable owner; no second presence authority remains in
the target startup path. Source anchors are `DB2Metadata.h::{ChrClasses,
ChrRaces}Meta`, `DB2LoadInfo.h:982-1032,1203-1272`,
`DB2Structure.h:698-743,865-921` and `HotfixDatabase.cpp:364-375,435-449`
at `02245dcd`. Class hash/layout are `F5889D8C` / `AFC9B0C2`, 43 physical
fields with inline **byte** ID 29; Race remains the existing checked 51-field
target layout with external IDs. Numeric projections retain flags, faction,
cinematics, starting-level metadata, display power, stat/AP factors and signed
race/class enum fields. DB2 `StartingLevel` is retained as data, **not** silently
chosen as the actual Player start-level policy.

`wow-persistence::forever::initialization` and the SQL adapter now consume both
seven-table overlay batches. The composition owns all seven stores only after
every baseline, overlay and removal succeeds. `CharacterCatalog::load` and
`WorldSources::load` now consult final effective Class/Race presence, so removed
identities cannot survive via the old baseline-only ID sets. The reader checks
inline-ID width from the table schema rather than a hardcoded 32-bit read and
rejects copy IDs that exceed a byte Class ID. Synthetic byte-ID/copy, identity
overlay/removal/duplicate and DTO signed-width regressions are written, not run.
This does not add full typed hotfix wire serializers or bypass their Valid-status
guard. Earlier five-store acquisition/test results above remain their actual
tested scope; they are not relabeled as seven-store Rust acceptance.

`creation::vitals::{PreEquipmentVitals,InitialPower}` is a private-field transient
numeric projection, not a Player or persistence DTO. Exact target source anchors:
`Player.cpp:2394-2580,504-507,27354-27413`, `StatSystem.cpp:90-99,285-332`
and `Unit.cpp:10081-10185`. It requires effective Class/Race/Power metadata and
the source class-power index, computes agility armor times two, zero CreateHealth
plus stamina times GT health ratio (source fallback **10** for a missing row),
uint32 truncation and zero-max-health clamp to one. Mana uses GT BaseMP, not
PowerType MaxBasePower. Focus/energy/mana start full; initial-login flag `0x2000`
fills other powers. `DefaultPower`/`0x20` do not select Create's current values.
DK displayed-rune pre-InitStats behavior is distinct from displayed runic power.
Source zero-next-level-XP wraps initial XP to UINT32_MAX; undefined float casts,
armor/stat overflow and nonrepresentable signed mana fail closed rather than
inventing saturation. Seven positive/negative vitals tests are written, unrun.

`WorldSources::primary_stats` now also ignores SQL class-level rows above the
configured cap (`ObjectMgr.cpp:4303-4351`) and extrapolates above-cap requested
levels using `ObjectMgr.cpp:4494-4562`. Its cap-minus-one loop, nine source class
branches, unchanged Spirit and absent newer-class cases are retained; no old
3.4.3/DK growth formula is substituted. Three progression regressions and one
source-row/cap regression are written, unrun. The opt-in observer checks numerical
readiness only at level one, not the account's actual starting level. Source
`Player.cpp:24649-24716` / `World.cpp:752-757,777,1063-1093` still own the
pending configurable normal/allied/hero/GM/template-level and money policy.
Full stats/update-fields, taxi/talents/skills/default spells, instantiated items,
GUID allocation, durability and fresh world packets remain required. No runtime
start, DB write, Character save, new Rust/native acceptance or push occurred in
this working phase.

At **04:59 UTC**, after the seven-store/vitals/cap changes and their consumers,
`env PATH=/home/joe/.cargo/bin:/usr/local/bin:/usr/bin:/bin cargo fmt --all`
and `git diff --check` both exit 0. The first formatting invocation without the
task PATH failed with exit 127 (Cargo not found); it did not format or compile
anything. Newly owned identity/effective/SQL/vitals/composition files remain
85-434 physical lines. These are hygiene and file-size observations only;
all newly written Rust regressions and actual-file differential QA remain
unexecuted. The complete goal and the existing exceeded campaign stay open.

At **05:15 UTC**, source starting-level/money/template integration extends this
same uncommitted candidate over `ccb99f8c`. `creation::StartingPolicy` consumes
one numeric configuration input plus an Arc to the sole `CharacterTemplates`
record owner. `Player.cpp:24649-24716`, `World.cpp:752-757,777,899-909,
1024-1033,1070-1093`, `Player.h:1043`, `RaceMask.h:54-68` and `RBAC.h:63,94`
are exact `02245dcd` anchors. Normal/allied/hero levels clamp 1..MaxPlayerLevel;
GM clamps 1..123 then rises to normal start, **not** down to MaxPlayerLevel.
Allied flags/Dracthyr select allied level; only allied flags override all class
money. DK Pandaren Alliance/Horde uses allied level/money, not neutral Pandaren;
DH and Evoker-money keep their separate branches. Permission 10 controls template
level, 41 controls GM level, and both select maxima rather than lower a start.
DB2 StartingLevel values are still metadata, not actual start policy.

The private target binary `start_config` preserves the source's **signed**
GetIntDefault/GetInt64Default reads before conversion into uint32/uint64 config
arrays, then source range clamps. Negative signed values and out-of-signed-range
unsigned text therefore must not be parsed directly as unsigned. Three written
configuration cases cover defaults, signed conversion/fallback and cap bounds.
This is not a new whole-config/Boost/environment-override equivalence claim;
runtime source-style cap default 90 remains **reference-only**, not native
Forever playable-cap proof.

`CreationWorldRepository::load_character_templates` owns a separate complete
two-query read: `character_template_class` (TemplateId/FactionGroup/Class), then
`character_template` (Id/Name/Description/Level). Zero rows may be legitimate;
query/width failures are fatal, never fabricated absent templates. DTOs live in
`wow-persistence::forever::creation::templates`, SQL/decode in the database
adapter's same responsibility, rules in `creation::templates`. Exact source
`CharacterTemplateDataStore.cpp:36-107` and `DBCEnums.h:1009-1015`: player+Alliance
or player+Horde bit combinations accept extra bits/both teams; unknown or removed
effective Class records are skipped. A template without valid classes is skipped;
labels/descriptions and byte Level are retained. Duplicate SQL primary IDs fail
the whole batch as an integrity guard. Class vectors keep source row order and
duplicates; the record store remains unordered like C++. Cross-language map
iteration equivalence and native multi-template wire/UI are **not** claimed.

`StartingPolicy::select` resolves the signed request template ID through that
server catalog (uint32 conversion preserved), never treats the ID as a level,
and distinguishes a genuinely absent row from startup failure. AuthResponse
uses an Arc to the **same immutable template records**, with permission-10
publication per `AuthHandler.cpp:53-55` and `AuthenticationPackets.cpp:149-169`.
Only its wire projection clones strings; no second mutable catalog or Player
mirror is added. Source template Level is not serialized in AuthResponse.
The opt-in Create observer now evaluates stats/mana/vitals at the selected
reference level rather than hardcoded level one; observation still is not
admission, a save or a success response.

The accompanying permission boundary extraction moves the old default graph
DTO/query/expansion from names into `forever::permissions`. Query text/order,
security-zero/realm roots, empty-known/empty-link behavior, self-link filtering,
cycles and unknown-ID handling are intentionally retained; permission 17 remains
SQL-reserved-name bypass only. The same immutable expanded set is now shared
through Session identity for 10/17/41. No account grant/deny manager is claimed:
the runtime still rejects customized RBAC/nonzero security before encrypted
admission. Old name-only DTO/function paths and the cached bypass bool are
retired; this structural change is separate from the new level/template behavior.
The architecture/safe-refactor guides informed that single-owner boundary;
no dependency, registry metadata, mutable lock or architecture ceiling changed.

Written coverage is **18 new tests** (six start policy, four template catalog,
two Session-linked publication/failure, one permission extension, two SQL shapes,
three binary config), plus the preserved relocated graph regression. All remain
**unexecuted**; earlier passing tests are not relabeled as evidence for this SHA.
One optional Luna spawn failed with the thread limit; root owns integration and
exclusive final acceptance. `cargo fmt --all` exits 0 with the task PATH; no
Cargo build/test, new live query, DB write, runtime installation/start, Character
save or push occurred. Source cap, private GT differential, populated native enum,
full initial skills/spells/items, GUID/durability/relogin and initial world-load
acceptance remain required. The already exceeded acceptance campaign is not reset.

At **05:31 UTC**, guarded birth data and SQL tier preparation continue on
`ccb99f8c` plus the uncommitted macro delta. Exact `02245dcd` evidence:
`DB2Metadata.h:3206-3247,19625-19690,19715-19738`,
`DB2Structure.h:658-675,3688-3752`,
`ObjectMgr.cpp:3968-4125,8016-8031,9005-9031,9074-9104`,
`Player.cpp:5768-6038,25544-25735` and
`DB2Stores.cpp:1539-1548,3028-3048`. Profession child lines copy the parent
rank and must not raise it. Skill rewards use effective SpellInfo, conditions,
race/class masks and AddSpell cascades; a set of spell IDs is not full birth
initialization. This continuation does not implement those Player transitions.

The optional probe `--ack-character-birth-tables` acquires complete SkillLine
(154 rows/hash `B53DC9D6`), SkillRaceClassInfo (186/`06ADE420`),
CharacterLoadout (114/`E00A47FB`) and CharacterLoadoutItem (846/`65C39BA7`).
Fresh strict acquisition exits **1** on SkillLineAbility (`FF4446F6`), after
reporting 7833 plaintext and five unknown one-row sections. Its already saved
private diagnostic files are preserved; no full/zero-filled ability file is
written. A separate `--ack-available-birth-abilities` option, requiring the birth
acknowledgement, follows `DB2FileLoader.cpp:1915-1926::Skip` and saves only the
unchanged `SkillLineAbility.available.db2` prefix through byte 346918.
`BirthAbilityPrefix.h` checks exact flags/index, parent lookup count one,
six section offsets/counts/relationship sizes and plaintext boundary; all five
excluded keys must remain unavailable. No generic partial-table relaxation,
automatic fallback, unknown-record promotion or source-client modification occurs.

The second acknowledged acquisition exits **0**, saving those five files
with mode 0600 under the private 0700 directory
`target/forever-login/client-data-birth-70170-20261003T0525Z`.
Metadata-only sizes are 13724/4428/346918/1182/13850 bytes respectively.
No assets or key material are staged. `AcquisitionSchemas.h` mechanically
relocates existing metadata definitions unchanged and adds the independent birth
family; `main.cpp` remains the sole CASC/private-output owner (987 lines).
The safe-refactor skill informed that narrow split, not a gameplay owner change.

Asset diagnostic commands/results, during the **05:22:18–05:28:07 UTC** window:

```text
cmake -S tools/wow-test-bot/client-data-probe -B target/forever-login/client-data-probe-build -DRUSTYCORE_FOREVER_CPP_REF=<pinned-reference> -> 0
cmake --build target/forever-login/client-data-probe-build --target forever-client-data-probe -j1 -> 0
ctest --test-dir target/forever-login/client-data-probe-build --output-on-failure -> 0, 1/1
FOREVER_CLIENT_DATA_PROBE_BIN=<built-probe> python3 tools/wow-test-bot/client-data-probe/test_cli.py -> 0, 10/10
probe --ack-local-client-data <local-storage> <new-private-directory> esES --ack-character-birth-tables --ack-available-achievements --ack-public-tact-keys <private-file> -> 1, required encrypted ability sections unavailable
probe <same-acknowledgements/new-output> --ack-available-birth-abilities -> 0
```

The probe build/self-test/CLI run was repeated only after implementing the new
explicit prefix gate; no unchanged failing acquisition/campaign was repeated.
These are isolated asset-bootstrap diagnostics, **not** Rust/full-delivery QA.
The pre-existing over-600-second ordinary campaign remains exceeded.

`wow-data::forever_birth` now defines all five raw numeric projections with
source widths/signs, two-word masks, real uint16 parent/skill IDs and an explicit
unknown-ability count. It has no SQL, Player mutation or effective catalog claim.
`CreationDb2` and the new private `wdc4::available::birth_abilities` retain
ordinary encrypted-file rejection and opt into only the exact ability prefix.
The read-only `forever_birth_tables` example emits counts only. Two new numeric
and two prefix regressions are written, plus ten private-copy QA cases in
`test_rust_birth.py`; none are executed or presented as actual Rust decode
evidence. These tests cover counts/negative admission, **not** complete numeric
differential, recursive spell learning or inventory parity.

The existing startup repository expands from eight PlayerInfo reads to nine
with all sixteen uint32 `skill_tiers` values, after PlayerInfo, per
`World.cpp:1702,1776`. No per-table repository trait or mutable mirror is added.
`creation::skill_tiers` consumes rows into one immutable owner; lookups preserve
index clamp-to-15 then zero backtracking. Zero is legitimate; missing tier is
None; duplicate SQL primary identities fail as a composition integrity guard.
Values are not prematurely narrowed to uint16. Five additional Rust cases
(SQL shape, three tier cases, one WorldSources composition) remain unexecuted.
Read-only local information_schema/count inspection confirms ID + sixteen
unsigned INT columns and 59 rows; that does not prove Rust adapter decoding.
Formatting/diff hygiene pass. No Cargo build/test, DB write, runtime start,
GUID/save/success, commit or push occurred. Effective birth hotfix composition,
complete SpellInfo/ItemTemplate and native create/world acceptance remain open.

At **05:43 UTC**, the working candidate composes the five effective birth stores
and their SQL/composition consumers. `forever_birth::BirthCatalog` owns final
numeric rows; private indexes contain only IDs. Source
`DB2Store.cpp:127-133`, `DB2DatabaseLoader.cpp:27-174`,
`DB2Stores.cpp:1539-1548,3028-3048` and `ObjectMgr.cpp:3968-3989` at `02245dcd`
freeze official-before-custom, final removal and derived-index behavior. Baseline
duplicate IDs still indicate malformed acquisition. **Overlay duplicates are
not rejected**: these target SQL tables have a primary key `(ID,VerifiedBuild)`;
the loader overwrites existing records in query-row order and installs new
indexes after reading all rows. No numeric build ordering or deterministic
cross-database SELECT ordering is invented. This is a new-family behavior
contract, not a silent repair of another feature's older duplicate guard.

Parent-line and ability vectors follow ascending effective DB2 IDs. Ability
grouping uses nonzero signed-int16 SkillupSkillLineID, else unsigned SkillLine;
the C++ conditional promotes to int then converts the uint32 map key, including
negative bit patterns. The raw effective loadout-item index does **not** prove
ItemTemplate presence, counts, equipped positions or item instantiation; those
remain part of the pending birth operation. Race/class rows expose DB2 storage
order only, not `unordered_multimap` first-match equivalence. The five-row
unavailable baseline diagnostic survives SQL composition; overlays/removals
cannot certify complete unknown-ID coverage from that count.

`wow-persistence::forever::birth` owns transient SQL-free rows with signed mask
words, `wow-database::forever_hotfix::birth` the ten official/custom projections,
and the private target-binary `birth` module consumes them into numeric records.
SQL masks preserve both int32 bit patterns as uint64; full labels/locales/wire
serializers are not claimed. All reads/decode must finish before publication;
real empty tables differ from query/decode errors, and no DB snapshot is claimed.
The existing repository is extended, without a trait/crate per table, lock,
second Player authority or legacy-layout dependency. The architecture guide
informed that one immutable record owner and separation from learning/inventory.

Read-only local information_schema inspection confirms the source's numeric
types and added `Field_1_60_1_69876_003` / two
`Field_5_5_4_67090_014` columns, which are absent from the older base SQL but
present after target migrations. Counts are zero SkillLine/SkillRaceClassInfo/
CharacterLoadout/CharacterLoadoutItem overlays and **seven SkillLineAbility
rows**; no `hotfix_data` rows advertise these five hashes. This evidence does
not execute the new Rust SQL adapter or certify those seven effective values.

The isolated binary loads/publishes one Arc to BirthCatalog before listening.
CLI order is now `<existing four args> [--ack-available-initial-map]
[--ack-available-birth-abilities] [--ack-private-*-capture <new-file>]`; omission
retains strict complete-ability loading, never an automatic prefix fallback.
All five hashes are registered as known typed stores, so existing unsupported
serializer errors/Valid-status guards remain explicit rather than missing-record
replies. The startup count log does not mean Player skills/spells/items or Create
are admitted. The catalog is runtime-composed but not Player-integrated/native
accepted. Unordered matching, full templates and recursive birth remain open.

Eight new catalog regressions cover all-store precedence/removals, legal
overlay duplicate overwrites, signed skillup grouping/storage order, final
reparenting/retargeting and duplicate item relations, removal-ID bits/later Valid,
race/class order boundaries and retained baseline uncertainty. One SQL-shape
and one all-width/mask binary composition case are also written. All ten are
**unexecuted**. Production/test files remain separate (effective 201 lines,
catalog tests 263, SQL adapter 149, binary composition 234; startup 418).
No fresh Cargo/native result or publication is claimed.

To make later startup use one complete private directory, only the 27 explicitly
whitelisted DB2/GT assets were copied into
`target/forever-login/client-data-runtime-70170-20261003T0542Z`: nineteen existing
DB2 + five birth DB2 + three GT text files. Each `cmp` exits 0, as do comparisons
of the two separately acquired ChrClasses/ChrRaces baselines. Copy commands
exit 0 and preserve 0600 files/0700 root and gt directories. Coreutils warns
that `--no-clobber` portability may change; use `--update=none` for future copies,
not a reason to replay this completed copy. Git exclusion is verified. Neither
source acquisition directories nor the installation were changed, and no config,
private build key/list, log, executable or account data was copied. These byte
comparisons are asset preparation, not Rust decoding or gameplay evidence.
Formatting/diff hygiene pass; no build/test, DB write, restart, save, commit or
push occurred. The full goal and exceeded ordinary campaign remain open.

Remaining complete Player initialization dependencies are explicit:

The **05:54 UTC** working delta implements quantity/list rules only, against
`02245dcd245e7433e524577656177723d3e4992e`:
`ObjectMgr.cpp:3319-3426,3823-3848,3968-4096`,
`ItemTemplate.h:855,941-944`, `DB2Structure.h::{Item,ItemSparse,ItemEffect,
ItemXItemEffect}Entry`, `SharedDefines.h:153,396-397`, and
`RaceMask.h::GetRaceBit/HasRace`. Ownership is
`wow-data::forever_birth::item_quantities::ItemQuantitySources` for four
final numeric projections and ID-only effect indexes, and
`wow-world::forever::creation::WorldSources::initial_items` for the transient
ordered source list. The architecture skill informed this narrow source owner,
not another Player mirror, SQL pool, lock, trait or full template copy.

The quantity constructor explicitly requires **already effective** records;
it does not load raw DB2, apply overlays, or certify complete ItemTemplate.
The real target producer/startup consumer are still pending. Its join requires
both Item and ItemSparse, skips dangling effects, preserves signed foreign-key
bits and lower_bound insertion before equal legacy slots in ascending relation
ID order. Vendor count is at least one; food category 11 gives four (DK ten),
drink category 59 gives two. Only consumable/food-drink amounts are clamped;
nonpositive/INT_MAX Stackable becomes `0x7FFFFFFE` per source. No inventory
placement, durability, item stats or use checks are implied.

List preparation filters Purpose 9, source uint8 class and target race mask,
retains duplicate items, and sets context only for a nonempty successful join.
Loadout IDs above uint16 cannot truncate into another relation key. SQL race/
class zero remain wildcards; absent identities/items and amount zero are skipped.
Positive int8 amounts append without merging/clamping; every negative amount,
including less than -1, removes all equal entries, following the source's
log-and-remove behavior. Overrides do not change item context. RaceMask maps
Dracthyr/Earthen/Haranir and Skyborne 95/96 to the source bits; unknown races
never match even an all-bits mask. No race-ID-minus-one fallback is added.

Six quantity, two mask and eight source-list regressions are written, including
duplicate-effect ties, missing joins, signed keys, all-store duplicate guards,
wide loadout IDs, Skyborne second-word masks, wildcard/signed overrides and
final relation removal. They are **not executed**; commands for completed-delivery
acceptance remain the affected `wow-data`/`wow-world` complete suites plus final
and the required native/durable campaign. No build/test, new client acquisition,
DB write, installation/restart, character/item GUID, save, commit or push was
performed by this delta. It cannot justify enabling Create or reporting the
goal achieved. Full templates/skills/spells/inventory/save/world remain required.

At **05:57–05:59 UTC**, integrated local acquisition diagnostics on the same
uncommitted candidate add `CharacterItemSchemas` from target `DB2Metadata.h`
and two explicit CLI modes. The safe-refactor skill informed the cohesive
argument-parser extraction into `ProbeOptions.h`; acquisition/key validation/
private output remain in main, without a second runtime/SQL owner. Existing
options, duplicates, dependency guards, messages and validation order are
retained. New item opt-ins increase the maximum argc only for their flags.
The complete item mode remains strict; there is no available-item mode yet.

Two complete acquisitions, without and with the existing private public-key
list, exit **1** on Item's unavailable sections; only classes/races are saved
in their new ignored private directories. Initial metadata inspection exits
**1** at ItemSparse's complete-file size bound. The refined metadata-only path
allocates only header/section buffers, leaves complete-file limits unchanged,
checks source header layout/field metadata and reports unsupported normal
readers rather than pretending to parse sparse data. Fresh metadata inspection
exits **0**, reporting all four tables below; **no item file is saved**.

| store | hash / layout | bytes | declared records | known / unknown section records | source shape |
| --- | --- | ---: | ---: | --- | --- |
| Item | `50238EC2` / `9A2A4834` | 302290 | 9092 | 9033 / 59 | 16 normal fields, eight sections |
| ItemSparse | `919BE54E` / `6FCC3191` | 6999130 | 19236 | 19167 / 69 | 68 **sparse** fields, flags 5, eight sections |
| ItemEffect | `4002A5B1` / `4CA77678` | 125562 | 7620 | 7580 / 40 | nine normal fields, seven sections |
| ItemXItemEffect | `00CB674F` / `96F083AD` | 190146 | 12628 | 12588 / 40 | one in-record field + parent field, six sections |

Declared/section counts exclude separately reported copies and do not establish
effective identity coverage. All unknown-section keys remain unavailable after
the private import; no key identifiers/material or rows are printed. The metadata
artifact label is `client-data-item-metadata-70170-20261003T0602Z` (an artifact
name, not an assertion of the observed acquisition time). It contains only the
unchanged base outputs and acknowledged Achievement prefix, not item records.
0600 files/0700 root and Git exclusion are verified. Failed partial diagnostic
directories are preserved; no original artifact/install/config is overwritten.

Final isolated diagnostic commands are CMake build `-j1`, CTest's synthetic
header contract and the integrated `test_cli.py` pre-storage guards. They pass;
these are not the ordinary Cargo campaign, sparse/item decoding, a native Create
capture or save/restart/relogin proof. No Rust tests/build, runtime start, DB
write, commit/push occurred. Source-backed sparse records and explicit encrypted
Skip now have concrete target evidence and are the next item-producer work;
this is not a blocker on all remaining safe implementation.

At **06:04–06:05 UTC**, target-only `--ack-available-item-tables` acquires the
four exact original plaintext prefixes, with `--ack-character-item-tables` and
the previously scoped private key list. It is mutually exclusive with metadata
inspection; default complete extraction stays strict. `ItemPrefixes.h` freezes
each actual hash/layout/header/flags/ID/parent/section/copy/key-marker/extent,
not a general Skip/sparse waiver. All excluded keys must remain unavailable.
Only the exact sparse prefix is allowed above the normal complete-file limit.
No encrypted bytes or excluded rows are zero-filled/manufactured.

| artifact | bytes | known direct records | known copies | unknown direct records |
| --- | ---: | ---: | ---: | ---: |
| Item.available | 301411 | 9033 | 22788 | 59 |
| ItemSparse.available | 6971318 | 19167 | 57 | 69 |
| ItemEffect.available | 125074 | 7580 | 5015 | 40 |
| ItemXItemEffect.available | 189486 | 12588 | 0 | 40 |

The first acknowledged acquisition exits **1** at the sparse extent after saving
Item's original prefix. Exact `DB2FileLoader.cpp:1019-1050,1938-1968` source
contrast shows two ID lists: catalog IDs before copies/six-byte catalog entries,
then the separate `IdTableSize` read by Load. The corrected extent includes
both (`14 * CatalogDataCount + 8 * CopyTableCount` for this sparse/no-parent
contract), rather than weakening the assertion. Final acknowledged acquisition
exits **0**; artifact root is
`target/forever-login/client-data-item-prefixes-70170-20261003T060357Z`
(artifact labels do not assert the measured wall-clock acquisition time).
All four outputs are 0600; root is 0700 and Git exclusion is checked. The failed
diagnostic directory remains private/preserved; source installation and prior
artifacts/config are untouched. Known/unknown counts are not effective identity
coverage; copies in unavailable sections are distinct and not installed.

CMake build `-j1`, CTest's synthetic header/prefix self-test and eleven integrated
CLI guards pass on the corrected source. These isolated diagnostics are not a
Cargo/ordinary campaign, a decoded sparse ItemTemplate, equip/use/save acceptance
or native creation capture. Field-width metadata inspection finds 68 sparse
fields: 0 unused bits for the first 35, 16 for the next 17, 24 for the last 16;
the source schema has **five** flags and variable inline strings. Old sparse
offsets/four-flag assumptions must not be reused. Full goal remains open; the
actual sparse record/copy reader and effective item producer are next. No DB
write, runtime install/start, character save, commit or publication occurred.

At **06:13 UTC**, `wow-data::forever_birth::item_sparse` implements a new
target-only raw baseline operation. Source SHA remains
`02245dcd245e7433e524577656177723d3e4992e`, with exact owners
`DB2Metadata/DB2LoadInfo/DB2Structure::ItemSparse`,
`DB2FileLoader.cpp:1019-1050,1291-1317,1455-1584,1938-1968`. The maintained
architecture boundaries informed one transient numeric batch, separate private
decode/prefix/test responsibilities, and no legacy reader/Player mirror.

`SparseItemRecords::load_available` explicitly reads only the named acknowledged
prefix; a bounded read retains at most prefix-size+one byte before the exact
extent gate. Schema/section/key markers/primitive widths/first catalog and all
omitted section extents remain checked. Physical field offsets and normal
compression columns are not sparse offsets: the source walks five NUL-terminated
inline byte strings and typed arrays sequentially. They are not retained as
names/locales or treated as required UTF-8. All 98 numeric cells retain their
signed/unsigned/float bits, with five flags, ten-element stat arrays, both
race-mask words, two zones and three socket types. Numeric domain/equip
admission and full string/wire serialization remain separate, unfinished work.

The sparse catalog's IDs are authoritative; the extra ID table contributes to
extent but is unused by the source sparse implementation. Each catalog entry's
uint16-sized record must remain inside the known plaintext body; malformed
duplicate baseline IDs and unsafe ranges fail closed. Copy processing follows
source file order, supports prior copies and target overwrite, and skips zero,
missing or out-of-bound **source** IDs. A zero target is not confused with a
zero source. Result records are in ascending storage-ID order. Only direct
unknown count 69 is retained diagnostically; this is not effective ID coverage.
`quantity_projection` is still baseline data until official/custom/removal
composition, never full ItemTemplate/CanEquip proof.

Eight synthetic regressions cover variable strings/padding, all target arrays/
mask widths/five flags, signed and raw float bits, truncated/cross-record/size
guards, complete copy skip/order/overwrite cases, duplicate/range guards,
schema/visibility/width/second-ID-table drift and non-publication of missing
records. They are **unexecuted**. Files remain cohesive (facade 114, decode 128,
prefix 185, tests 244, example 34 lines). The production-linked counts-only
example has an explicit private-root acknowledgement; its build and actual-file
run are also **pending**:

```bash
cargo build --locked -j1 -p wow-data --example forever_sparse_items
target/debug/examples/forever_sparse_items --ack-private-sparse-item-prefix \
  "$PWD/target/forever-login/client-data-item-prefixes-70170-20261003T060357Z"
```

Do not treat those unexecuted commands as passing, or later counts as full
numeric differential/native acceptance. Formatting/diff hygiene pass. No Cargo
campaign, runtime start, DB write, GUID/item instantiation, save, commit or push
occurred. The three remaining raw stores, effective item overlays/removals,
full templates, skills/spells/inventory and Create/save/world remain required.

The **06:24 UTC** working candidate now completes the four-store **numeric**
producer (not full ItemTemplate initialization). Source remains
`02245dcd245e7433e524577656177723d3e4992e`:
`DB2Metadata/DB2LoadInfo::{Item,ItemEffect,ItemXItemEffect}`,
`DB2FileLoader.cpp:613-632,635-696,955-977`,
`HotfixDatabase.cpp:870-872,980-981,1067-1081,1104`,
`DB2Store.cpp:127-133`, `DB2DatabaseLoader.cpp:27-174` and the final-removal
pass in `DB2Stores.cpp:1539-1548`.

Code targets are `wdc4/available/items`, `CreationDb2::open_item_prefix`,
`forever_birth::item_records::{ItemRecords,ItemCatalog}`,
`wow-persistence::forever::items`,
`ForeverHotfixRepository::load_item_overlays`, binary `forever/items.rs` and
`bootstrap::load`. The three regular prefixes freeze their actual hashes,
layouts, all known/unknown sections, copy/parent extents and exact bounded
read sizes. Normal open still rejects encrypted/full/sparse unsupported files;
none of the non-item copy contracts are broadened. Target item copies process
file order, skip zero/unresolved sources, overwrite targets, retain prior-copy
sources and allow zero targets, with header maximum enforced.

All four checked baseline reads complete before returning the transient batch.
The SQL batch has 17/99/10/3 numeric columns per table, keeping five flags,
three ten-element stat arrays, both signed race-mask words, zones/sockets and
the target signed/narrow field widths. Locale strings are deliberately absent.
All eight official/custom reads complete before publication; errors are fatal,
observed row order/repeated overlay IDs are retained, and no SQL snapshot is
claimed. DTOs are consumed into final baseline -> official -> custom -> removal
maps. Startup owns one immutable catalog; the quantity projection is a derived
transient view, not another mutable authority or equip/instance proof.

The working runtime now **requires** `--ack-available-item-tables` after the
optional Map/ability acknowledgements and before any capture pair. This strict
candidate only supports the acquired prefix set; there is no implicit full-load
fallback. Existing combined runtime artifacts have not been updated/installed.
The four hashes are marked known, while unported full wire serialization keeps
the existing Valid-status rejection fence. No new packets/success are sent.

Fresh read-only metadata/count inspection finds 18/105/11/4 physical SQL columns
(including omitted strings/VerifiedBuild) and 5018/5019/13/7 stored rows. Sparse
numeric widths match the target DTO including five flags and both signed mask
words. These are SQL metadata/raw-row counts, not effective unique counts or
successful Rust decoding. Three prefix/copy/parent, three effective composition,
two SQL-shape/failure and one consuming-width regressions are written,
**unexecuted**; previous eight sparse regressions remain unexecuted too.

The additional all-four-table read-only consumer is implemented with a
private-root acknowledgement and counts-only output. These commands remain
**pending**, to be included in complete-delivery acceptance:

```bash
cargo build --locked -j1 -p wow-data --example forever_item_tables
target/debug/examples/forever_item_tables --ack-private-item-prefixes \
  "$PWD/target/forever-login/client-data-item-prefixes-70170-20261003T060357Z"
```

Formatting/diff hygiene pass; no Cargo campaign, actual-file Rust execution,
DB write, runtime install/start, save, commit or push occurred. Full templates
(durability/spec/addons/bonuses), learned skills/spells, instantiated equipment,
durable creation and native world entry still require implementation/acceptance.

The **06:37 UTC** delta adds target numeric template initialization. New
acquisition opt-in `--ack-item-template-tables` obtains complete ItemSpec,
ItemSpecOverride and GemProperties using `DB2Metadata.h` at pinned `02245dcd`.
`--ack-item-table-metadata-only` can inspect this group independently too;
ordinary/default item acquisition remains unchanged and has no empty fallback.

| Table | FileDataId | Actual hash / layout | Complete bytes / records |
| --- | ---: | --- | --- |
| ItemSpec | 1135120 | `08DA6E2A` / `83F3D113` | 228 / **0** |
| ItemSpecOverride | 1134576 | `149AAE79` / `B292998C` | 412 / **9** |
| GemProperties | 1343604 | `9C00EA6D` / `86487AD2` | 212 / **0** |

The first acquisition at `06:27:24 UTC` rejects a zero-byte section-header read
(CASC error 22), leaving its new private diagnostic directory untouched.
`DB2FileLoader.cpp:1788-1802,1860-1915` explicitly reads section headers only
when SectionCount is nonzero, always reads primitive field metadata and checks
the full file extent. The corrected extractor preserves genuine empty files;
record counts/strings/column/common/palette contradictions or missing metadata
reject instead of manufacturing absence. The successful new output is
`target/forever-login/client-data-item-templates-70170-20261003T062838Z`.
All three files are 0600, root 0700 and ignored; original installation and prior
artifacts are unchanged. CMake `-j1`, synthetic header self-test and twelve CLI
guards pass in both isolated diagnostics; final acquisition exits zero. These
are asset diagnostics, not the ordinary campaign, Rust decoding or creation.

Rust targets: `forever_birth::item_specs` raw/effective records,
`CreationDb2`'s **target-only** complete-empty-file gate,
`InitializationCatalog::specialization_by_id`, SQL-free
`wow-persistence::forever::item_specs`,
`ForeverHotfixRepository::load_item_spec_overlays`,
`CreationWorldRepository::load_item_addons`, consuming binary `item_specs.rs`,
and `wow-world::forever::creation::NumericItemTemplates`. Startup loads all
three baselines, six ordered official/custom queries and the addon batch,
applies final removals, and consumes raw sources into one nested immutable
owner with derived metadata/effect IDs. The temporary spec/relic catalog is
dropped after derivation. No new crate/dependency, mutable mirror, writer,
legacy gameplay/architecture ceiling or baseline is introduced.

Exact behavioral sources are `ObjectMgr.cpp:3020-3442`,
`DBCEnums.h:1551-1595`, `ItemTemplate.h` ItemModType/SocketColor/ItemSubclass,
`ItemTemplate.cpp:296-299`, `SharedDefines.h:174,380-391,1063`,
`DB2Stores.cpp:1380-1381`, and `HotfixDatabase.cpp:792,1087-1092` at `02245dcd`.
Durability preserves f32 operations/rounding and the <=28 level penalty.
MAX_ITEM_QUALITY is **nine**, so quality eight uses the source's implicit zero
initializer. Armor >robe/non-weapon/armor short-circuits happen before array
indexing; invalid source array indexes fail closed. ItemSpecStats preserves
weapon/armor/cloak/relic mapping, eleven gem-relic tests truncated by source's
ten-stat capacity, dedup and the exact recognized mod switch (not generalized
old hit/haste variants). Spec masks preserve five slots/class and 80-bit
all-spec fallback independently for each of three level ranges. Source ignores
MinLevel here, uses MaxLevel >40 / >=110, and override presence suppresses
matching even when its references are unresolved; override matching does not
filter AllowableClass. Invalid class-zero shifts/spec bits fail closed.
Effects retain duplicate relations and lower_bound-before-equal order. Addons
skip non-templates, start with source zeros, and swap inverted money bounds.
Full strings/wire, random bonus selection, script names/use/equip and item
instances remain explicitly outside this **numeric** template result.

Thirteen new Rust regressions are written, **unexecuted**: four raw/effective
spec cases, six template/durability/spec-stat/override/bounds/composition cases,
two SQL-shape cases and one consuming-width case. Formatting/diff hygiene
passes after formatting the final test delta. No Cargo compile/test or
actual-file Rust numeric result is claimed. Probe main is 1013 lines: cohesion
review identifies synthetic self-test versus CASC acquisition as its next natural
split, but this file remains one acquisition/output owner, below the terminal
2000-line budget; no policy exception/ceiling change is made.

At 06:37 UTC, fresh read-only SQL inspection confirms numeric widths, 4/68/0 raw spec/
override/gem rows and 625 addons. It also finds Valid hotfix metadata counts:
Item **4457**, ItemSparse **4457**, ItemEffect **13**, ItemXItemEffect **7**,
ItemSpec **4**, ItemSpecOverride **68** (plus target removals).
By **code/data inspection**, not an attempted live start, the working startup's
retained known-store Valid-status fence would reject these unported serializers.
Full seven-store record delivery is therefore a required next operation, not
an optional polish step. `src/server/shared/DataStores/DB2Store.cpp:41-85`
writes metadata field/array order, excludes external IDs from record bytes,
and selects localized strings. Its `DB2DatabaseLoader.cpp:27-174` main table
updates **enUS**, retaining existing locale slots; locale overlays are separate.
Numeric row replacement must not erase baseline esES strings, nor copy main
SQL text blindly into esES. No placeholder/empty success, status rewrite or
guard removal is authorized/introduced.

Fresh diagnostics confirm zero Characters and no listener at 18085. No DB
write, install/start, native action, character save, commit or push occurred.
Combined runtime assets are not updated/installed. Full Create, initial Player
skills/spells/equipment/persistence, native populated selection and world entry
remain unproven; the goal stays active.

### Seven item hotfix serializers and localized text — 2026-10-03 06:50 UTC

Implemented and connected in the **unvalidated working candidate**, based on
`02245dcd245e7433e524577656177723d3e4992e`:

- `src/server/shared/DataStores/DB2Store.cpp:41-85`: external-ID exclusion,
  metadata field/array order and literal locale selection; Item's payload is
  43 bytes, ItemEffect 24, relation 8, ItemSpec 7, override/gem 6 each.
  ItemSparse is five terminated strings plus 302 numeric bytes, not its
  physical 356-byte header record size or compressed/sparse DB2 record bytes.
- `DB2DatabaseLoader.cpp:27-174,175-310`: main SQL writes enUS, existing locale
  slots survive numeric replacement, empty AddString does not clear a slot.
  New IDs are indexed at batch end, so repeated new-ID rows do not accumulate
  prior row strings; existing IDs do. Locale SQL only updates existing rows.
- `DB2Stores.cpp:618-644`: main official/custom batches precede locale batches.
  This isolated server has only the acquired esES baseline; it adds no unseen
  locale asset or enUS-to-esES fallback. Missing locale slots remain Source's
  actual empty slots. A read-only four-byte header check confirms locale mask
  **64** at offset 168; the sparse reader now requires that esES header, matching
  `DB2FileLoaderSparseImpl::AutoProduceStrings:1183-1193`, before retaining text.
  `Common.h:105` / `ByteBuffer.h:349-367` select one locale
  and serialize a C-string prefix plus NUL, including SQL embedded-NUL cases.
- `HotfixDatabase.cpp:1067-1084`: full 104-column sparse projection including
  Description/Display3/Display2/Display1/Display, and two six-column esES
  official/custom locale reads. NULL text uses Source's empty string; binary
  text is retained without lossy conversion. Reads are not a snapshot claim.
- `Handlers/HotfixHandler.cpp:25-58,79-139`: DBQueryBulk replies in request
  order, HotfixConnect uses typed records before blob fallback and downgrades
  actual missing known records; unavailable serializers remain explicit errors.
  `DB2Stores.cpp:1866` permits no optional data on these seven stores/Tact.

Code owners: `wow-data::forever_birth::item_sparse::strings` and sparse
composition retain text; `wow-database::forever_hotfix::items::{sparse,locales}`
read full DTOs; `world-server::forever::{items,bootstrap}` consume/compose them.
`wow-data::forever_hotfix::items` implements the seven immutable serializers;
`wow-world::forever::handlers` invokes them through the existing registry.
Template rules and delivery share one canonical `Arc<ItemCatalog>`, while the
effective three-store spec catalog remains alive for delivery. No cloned raw
catalog, cached serialized record mirror, mutable lock, new crate/dependency,
legacy wire reuse or baseline/ceiling change is introduced. The retained
Valid-status guard exempts **only** the implemented seven stores plus Tact;
other known stores stay guarded. No hotfix status/database row is rewritten.

Eight additional regressions are written, **unexecuted**: four complete numeric/
string payload/shared-owner tests, one SQL locale-shape test, one effective
baseline/enUS/esES/new-ID/removal test and two registered Session delivery tests.
Existing sparse decoder/SQL shape and missing-row checks are updated. Final
formatting/diff hygiene is the only code check; no Rust compile/test, actual-file
numeric/text comparison or action-specific native capture is claimed. Full
delivery acceptance retains all of those checks, plus the inherited publication
gate and its unchanged legacy architecture finding.

Fresh read-only diagnostics at 06:50 UTC: 5019 sparse SQL rows, no NULL in the
queried Description/Display fields, 561 official esES locale rows, zero
Characters and no World listener on 18085. These counts are not effective-row
or decoder acceptance. No DB write, runtime asset update/install/start, native
action, character save, commit or push occurred. The goal stays active;
full Player initialization/skills/spells/bonuses/equipment, durable Create and
populated selection/world entry remain required and are not manual-test-ready.

### Independent target spell acquisition — 2026-10-03 07:08–07:13 UTC

The branch remains an independent modern Classic target. Only target-compatible
code/data is reused; 3.4.3 SpellInfo layouts or fallback spellbooks are not
production inputs for this operation. No legacy source or architecture ceiling
is changed by this working acquisition delta.

Source anchors at `02245dcd245e7433e524577656177723d3e4992e`:

- `src/server/game/Spells/SpellMgr.cpp:2496-2735::LoadSpellInfoStore` and
  `SpellMgr.h:619-642::SpellInfoLoadHelper` join spell names, effects and the
  per-difficulty/reagent/power/visual stores; difficulty fallback and source
  ordering remain future assembly requirements, not an invented empty SpellInfo.
- `SpellInfo.cpp:1325-1526` consumes the resulting stores and dependent
  cast-time/duration/range/proc data. SpellMisc has **17 attribute words**;
  the old 16-word assumption is not reused. Radius, category, learn/form,
  summon and battle-pet dependencies are explicit acquisition inputs too.
- `src/server/game/DataStores/DB2Metadata.h` defines the 36 independent
  FileDataId/layout/field/index/parent contracts in `SpellInfoSchemas.h`.
  Table hashes come from actual client headers, not guessed source values.
- `src/common/DataStores/DB2FileLoader.cpp:1858-1968` separates ordinary
  reads from explicit encrypted-section Skip, preserving normal record,
  string, external-ID, copy and relationship extents.

`ProbeOptions.h` requires `--ack-spell-info-tables`; default/existing acquisition
groups are unchanged. `--ack-spell-table-metadata-only` independently requests
bounded header/section observation, never saving spell record bodies. The
normal classes/races and selected Achievement operation still save their own
private assets. All 36 real 70170/esES headers match the source schema contracts;
20 tables report unknown encrypted sections despite the existing private public
TACT list. Strict acquisition correctly exits one at SpellName before saving it.

The additional `--ack-available-spell-info-tables` requires the spell group,
rejects metadata-only combination and is limited to the observed **esES storage**.
`SpellPrefixes.h` owns the twenty captured table/hash/layout/flags/record/section/
full-file/known-prefix gates. All excluded section keys must still be unavailable.
The remaining sixteen tables use ordinary complete reads. Original header and
section bytes are preserved; no encrypted body is accessed, synthesized, promoted
to known absence or silently replaced by legacy data. Numeric headers declare
locale mask `0xFFFFFFFF`; SpellName/BattlePetSpecies declare esES mask `64`.
That actual distinction is checked per contract, not inferred from storage locale.
No complete-file 4 MiB bound or ordinary reader is relaxed.

Successful private output:
`target/forever-login/client-data-spell-prefixes-70170-20261003T071225Z`.
It contains **20 readable spell prefixes + 16 complete spell tables**, plus
the normal classes/races/Achievement-prefix outputs (39 files total).
Examples of available/unknown direct rows are SpellName **17565/569**,
SpellEffect **42409/1261**, SpellMisc **31720/906**, and SpellPower **3429/37**;
SpellName additionally has 14173 readable copies. Unknown direct/copy counts
are distinct, and do not establish an effective unknown-ID coverage claim.
Complete tables include 3468 Shapeshift, 52 LearnSpell, 17 Difficulty direct rows
and six real zero-section tables with primitive field metadata intact.
Files are mode 0600 in a mode-0700 ignored directory; installation, original
artifacts and current runtime assets are unchanged. Failed/metadata diagnostic
directories remain private and preserved rather than overwritten.

Parent-exclusive isolated diagnostic commands:

```bash
cmake --build target/forever-login/client-data-probe-build \
  --target forever-client-data-probe --parallel 1
ctest --test-dir target/forever-login/client-data-probe-build --output-on-failure
FOREVER_CLIENT_DATA_PROBE_BIN="$PWD/target/forever-login/client-data-probe-build/forever-client-data-probe" \
  python3 tools/wow-test-bot/client-data-probe/test_cli.py
# Local acquisition: normal guards, a new private output, no network/account/SQL.
target/forever-login/client-data-probe-build/forever-client-data-probe \
  --ack-local-client-data "$PWD/target/forever-login/client" \
  "$PWD/target/forever-login/<new-private-spell-output>" esES \
  --ack-spell-info-tables --ack-available-spell-info-tables \
  --ack-public-tact-keys "$PWD/target/forever-login/<private-key-list>" \
  --ack-available-achievements
```

Initial synthetic acceptance caught the TargetRestrictions/Totems schema indices;
actual-prefix admission then rejected the previously unobserved numeric locale
mask. Both are corrected using source/header evidence. Final CMake build,
**one synthetic schema/prefix CTest and 14 CLI tests pass**, and acknowledged
actual acquisition exits zero. CLI tests cover explicit/duplicate/unknown flags,
group/order/locale/private-file/no-overwrite guards; synthetic prefix cases cover
metadata drift, unknown-key promotion, section totals, extent and truncation.
The full selection/Create/world campaign and its previously exceeded ordinary
600-second budget are not reset or passed by these isolated asset diagnostics.

There is still **no Rust spell-reader/effective-SQL/SpellInfo acceptance**,
learned skill/spellbook, inventory, Player success, durable Create or world entry.
No Cargo campaign, runtime start/install, native action, DB write, commit or push
occurs here. Full source construction, server-side spell data, learning effects,
save/restart/relogin and native world acceptance remain mandatory.

#### Independent raw Rust spell inputs

The following working delta is **not compiled or executed**. Public
`wow-data::forever_spells::SpellRecords::load` now decodes all 36 source record
families, partitioned into private core/cost/dependency modules. These are raw
startup inputs, not an effective hotfix catalog, assembled SpellInfo or a
learned/saved Player. The composition publishes a batch only after every table
and every typed row finishes successfully; a late error returns no partial batch.

The existing `wdc4::creation::CreationDb2` remains the one checked numeric
adapter. Closed `creation/spells/schema.rs` contracts combine exact
`02245dcd` DB2Metadata/DB2Structure/DB2LoadInfo field types and the observed
70170 hashes/flags/locales/parents. There is no generic caller-selected schema,
new crate/dependency, permissive legacy getter, Player mirror or architecture
baseline change. Only compatibility-proven reader mechanics are reused;
legacy spell records and 16-word attributes are not inputs to this branch.

`SpellBaseline::Complete` reads only `.db2`, bounds each file at 4 MiB and
rejects unavailable sections. `AvailablePrefixes` is an explicit choice of the
twenty captured `.available.db2` contracts in `available/spells.rs`; it still
requires the other sixteen complete files. Missing, malformed or encrypted
input never silently switches modes or becomes a complete empty table. The
six genuine empty tables require their intact primitive field metadata.
Unknown counts describe direct baseline rows, not complete effective ID/copy
coverage or proof that a requested spell does not exist.

Additional loader source anchors:

- `DB2FileLoader.cpp:354-376::LoadTableData` reserves the complete logical
  record extent before the complete string extent, including skipped sections.
  `RecordGetString:798-803` uses a relative uint32 displacement from the
  physical source field, not from a compacted prefix or copied target ID.
  `creation/spells/text.rs` checks that extent and admits only readable,
  terminated raw bytes. It does not invent unknown zero-filled string pools.
- `AutoProduceRecordCopies:613-632` consumes copies in file order, skips
  zero/unresolved sources and overwrites targets; inline IDs become the target
  ID. `FillParentLookup:635-696` overrides in-record parents and leaves absent
  extra parents zero-initialized. These target policies extend only the closed
  spell family; unrelated legacy table admission is unchanged.
- `LoadHeaders:1740-1808` requires primitive field metadata even when a real
  empty table has zero column-metadata bytes.

`SpellText` retains raw per-locale bytes without lossy UTF-8 conversion or
implicit locale fallback. The captured baseline inserts only esES (locale 6);
SQL enUS/localized overlays and their source update semantics remain future
effective-composition work. Numeric records retain signed integer widths,
f32 bits, four-word masks, two-target/eight-reagent arrays and all **17** Misc
attribute words.

Written, **unexecuted** evidence:

- Ten Rust tests across closed-prefix admission, checked records and locale
  storage, including a synthetic complete-file batch through the public loader
  and every one of the 36 typed consumers. Source-width sentinels cover signed
  values, high unsigned bits, NaN bits and array tails; a missing final dependency
  returns an error rather than the preceding 35 tables.
- `examples/forever_spell_tables.rs` is a production-linked read-only consumer
  emitting only schema names and known/unknown counts, with
  `player_admitted:false` and `spell_info_assembled:false`.
- `client-data-probe/test_rust_spells.py` contains ten acknowledged private-copy
  QA tests. Count bounds distinguish readable direct rows from materialized
  copies, whose header count is not an exact new-ID count. Negative cases cover
  all 36 hashes/layouts/locales, missing/truncated files, strict mode including
  a prefix renamed as complete, unknown-section promotion, genuine empty
  metadata, private-root guards and the 4 MiB full-read bound.

At completed-delivery acceptance, build the example under the same exclusive
Cargo campaign/target as the affected library, then invoke actual-file QA:

```bash
FOREVER_ACK_PRIVATE_DATA_TESTS=1 \
FOREVER_CLIENT_DATA_DIRECTORY="$PWD/target/forever-login/client-data-spell-prefixes-70170-20261003T071225Z" \
FOREVER_SPELL_TABLES_BIN="$PWD/target/debug/examples/forever_spell_tables" \
  python3 tools/wow-test-bot/client-data-probe/test_rust_spells.py
```

Only disposable copies are modified by those tests. Original assets, game,
accounts, keys, SQL and runtime are not accessed by this consumer/QA path.
No Cargo campaign, runtime install/start, native action, DB write, commit or
push was performed for this delta. Effective official/custom/locale/removal
composition, source SpellInfo/difficulty assembly, server spell data, recursive
learning, item/equipment integration and the full durability/world operation
remain mandatory before Create is enabled.

#### Effective raw spell stores and typed hotfix delivery — 2026-10-03 07:36–07:48 UTC

This subsequent working delta implements the complete **36-store raw
composition/serialization prerequisite**, not the remaining SpellInfo/gameplay
operation. Everything described as Rust implementation below is **uncompiled
and unexecuted**; source inspection, SQL metadata/count diagnostics and
formatting/diff hygiene are separate evidence levels.

Source at `02245dcd245e7433e524577656177723d3e4992e`:

- `HotfixDatabase.cpp:263-269,594-600,1579-1820` contains the 36 exact main
  projections (351 columns) and six locale projections. No `SELECT *`, extra
  ID/build sorting, old spell table or 3.4.3 SQL layout is substituted.
- `DB2Store.cpp:127-145::LoadFromDB/LoadStringsFromDB` applies official then
  custom for each store. `DB2DatabaseLoader.cpp:28-185::Load` updates existing
  numeric rows in query order, uses main-table **enUS** strings, and publishes
  newly allocated indices only after the batch. Repeated new IDs therefore
  start fresh; they must not accumulate earlier new-ID strings.
- `DB2DatabaseLoader.cpp:187-287::LoadStrings/AddString` skips absent numeric
  IDs, updates nonempty selected-locale text and does not erase on empty/NULL.
  `DB2Stores.cpp:1741-1809::LoadHotfixData` performs final removals. A later nonremoved status
  revokes an earlier removal; signed removal IDs preserve uint32 ID bits.
- `DB2Store.cpp:41-85::WriteRecord` excludes the external ID, includes inline
  IDs and every metadata field/array/parent, and writes the selected locale.
  `Common.h:105-114::LocalizedString::operator[]` returns exactly that slot,
  without fallback. Embedded NUL terminates the wire C-string; raw DTO/storage
  bytes are retained before serialization.

Ownership and production wiring:

- `wow-persistence::forever::spells` provides dependency-free typed main and
  locale batches; `wow-database::forever_hotfix::spells` owns 72 main/12 esES
  prepared reads and strict full-column decoders. Any failed read/decode stops
  publication; no SQL transaction/snapshot, empty-result fallback, or writer
  is introduced. Legal repeated IDs remain in their observed query order.
- `world-server/forever/spells` consumes DTOs once into target raw records.
  Main strings are explicitly locale zero; locale records remain raw esES
  contributions. Numeric Source widths/f32 bits are unchanged except for the
  necessary SQL signed-word -> raw unsigned flag128 conversion.
- `SpellRecords::finish` consumes baseline/main/locale inputs, then publishes
  the 36 private effective maps in `SpellCatalog` only after all compositions
  and final removals finish. Unknown direct baseline counts are retained;
  SQL batches cannot assert that excluded baseline coverage is known empty.
  Inputs are not retained as mutable mirrors. No learned state/Player is made.
- Target bootstrap owns one immutable `Arc<SpellCatalog>` and passes the same
  allocation to `ForeverHotfixCatalog::with_spell_stores`; no cached byte or
  record clone is retained. `forever_hotfix/spells` contains all 36 serializers,
  grouped into core/cost/dependency private modules. Known unattached/unported
  stores remain explicit errors; registering a hash alone is not a serializer.
- Bootstrap now requires all spell files, defaulting to strict complete mode.
  The optional `--ack-available-spell-info-tables` follows mandatory
  `--ack-available-item-tables` and precedes any capture arguments. Only its
  twenty frozen contracts use prefixes; the remaining sixteen stay complete.
  No installed runtime directory was populated, build installed or process
  started with this new prerequisite. Valid typed metadata permits these 36
  serializers, not arbitrary stores. Spell optional-data rows are excluded
  under the retained source allowlist policy, not treated as arbitrary bytes.

Fresh read-only Docker MariaDB metadata/count diagnostics at 07:46–07:48 UTC:
all **351** selected columns exist. All eight nullable columns are text and
use source empty-string semantics. Eight `flag128` words in SpellEffect and
SpellClassOptions are signed SQL `int`, despite their unsigned raw words;
the SQL DTOs now retain `i32` and composition uses `as u32` to preserve all
bits. Three actual SpellEffect rows contain a negative mask word, so this
is a real consumer requirement, not merely synthetic sentinel coverage.
There are 677 official rows, zero custom rows and 19 nonempty tables among
the 36. Characters remain zero; port 18085 has no listener. The first metadata
invocation failed from shell quoting; the corrected invocation exits zero.
No schema/data/account/game write occurred, and no row values or secrets were
printed. These are not Rust-adapter, save or native wire acceptance results.

Written **83 additional, unexecuted** Rust tests:

- Two exact main/locale projection tests; 36 complete numeric/text DTO
  conversions plus one all-six locale conversion test. Sentinel cases retain
  signed/unsigned extrema, array tails, flag128 high bits, NaN payloads and
  invalid UTF-8/embedded NUL bytes without loss or fallback.
- Six effective composition cases cover official/custom duplicate order,
  all six localized families, empty updates, repeated new IDs, missing locale
  IDs, table-specific/signed-bit/final/revoked removals, invalid locales,
  duplicate baselines and false SQL unknown-coverage assertions.
- Thirty-six full metadata-order/width serializer goldens plus two text and
  production-catalog delivery cases. Delivery requires all registered hashes,
  verifies the shared `Arc`, rejects invalid locales and preserves unrelated
  unported-store errors. The preceding synthetic public reader test now also
  exercises all 36 effective maps and all-table final removals.

No Cargo build/test/acceptance campaign, native action/capture, runtime install/
start, DB write, commit or publication is claimed. The ordinary campaign's
previously exceeded 600-second budget remains failed, not reset by these
diagnostics. New serializers still need action-specific target wire/capture
acceptance in the complete delivery. Effective raw rows are **not** full source
SpellInfo/difficulty fallback, server-side spell corrections/data, rewarded/
recursive learning, inventory/Player initialization or durable creation.
Selection/Create/world, restart/relogin and native initial loading remain the
full goal; its status stays active.

#### Target spell joins and constructor precursor — 2026-10-03 07:55–08:08 UTC

Working code based on HEAD `ccb99f8c`, **not compiled or executed**. This
implements another prerequisite inside the same selection/Create/world
delivery, not a completed spellbook, Player initialization or goal.

Ownership decision: retain the effective raw `Arc<SpellCatalog>` as canonical
DB2 input authority and add private `wow-world::forever::spells` children for
the semantic join and constructor. Reusing the inherited SpellInfo/attribute
layout would import unsupported version behavior; cloning the 36 raw stores
into a second gameplay catalog would add redundant payload ownership and
future synchronization risks. `SpellLoadPlan` instead owns immutable IDs and
indices; `SpellInputs` borrows the selected typed rows. Temporary original
helpers are discarded after acyclic resolution. Constructor seeds own only
derived scalars/sets/vectors and borrow raw dependencies. No new crate, trait,
lock, mutable Player mirror or Session-held spell state is introduced. The
largest new file is the 530-line constructor test; all new production files
are below 400 lines. No architecture baseline/ceiling is changed.

Exact source contracts at reference SHA
`02245dcd245e7433e524577656177723d3e4992e`:

- `SpellMgr.cpp:2496-2735` and `SpellMgr.h:619-642`: all 15 scalar families,
  32 effect slots, five cost slots and four vectors. Ascending effective DB2
  storage IDs select the last scalar/slot; signed SpellID and Difficulty
  promotion is retained. PowerDifficulty is looked up by Power.ID and overrides
  both difficulty and order index. Empower stages sort ascending Stage,
  inserting before equal stages; visuals sort descending signed Priority and
  **unsigned** CasterPlayerConditionID, before equal pairs. Labels/currency
  rows append in storage order. Names alone do not instantiate helpers;
  unnamed helpers do not become constructor inputs.
- `SpellMgr.cpp:2644-2728`: each named key fills only missing scalar/array
  slots; each vector uses the first nonempty fallback, not concatenation.
  Traversal stops when the next Difficulty record is absent. Resolving against
  original helpers along the entire acyclic chain preserves the first-available
  result despite the C++ unordered in-place iteration. Fallback never creates
  a new spell/difficulty key. Cycles fail startup explicitly rather than loop.
- `SharedDefines.h:1344-1707,2964-3113`, `SpellAuraDefines.h:85-753`,
  `DBCEnums.h:2445,2478` and `SpellDefines.h:197`: enum ceilings are 361
  effects, 665 auras and 153 targets; the Classic guard skips unsupported
  effects and reports counts. Four spell-modifier aura kinds report MiscValue
  at or above 41 but retain the row. These ceilings do **not** prove executable
  Rust support. The port rejects negative/out-of-range effect/cost array
  indices and negative implicit targets; the C++ assertions/unchecked indexing
  do not define safe behavior there. These deliberate fail-closed startup
  admissions are not described as parity for undefined behavior.
- `LanguageMgr.cpp:40-47`: every accepted language effect registers its
  unsigned language ID and spell, with skill unresolved/zero, even if another
  row later overwrites that effect slot or the helper lacks a name. Duplicate
  registrations are retained; deterministic storage order here is not a claim
  about unordered_multimap iteration or full `LoadLanguages`/translation.
- `SpellMgr.cpp:2504-2540`, `BattlePetMgr.cpp:183-186`: nonzero signed creature
  IDs select the last species storage ID; summon property lookup preserves
  signed-to-unsigned bits, requires minipet slot five and the 64-bit journal
  flag `0x00200000`, then replaces the spell association. Missing properties,
  species, flags or wrong slots do not manufacture associations. This is an
  immutable source association, not a character's pet journal.
- `SpellInfo.cpp:403-449,1325-1526`, `SpellInfo.h:~207-265,324-437`:
  constructor projection copies all 65 scalar/array groups plus base PPM,
  17 attribute words and selected dependency pointers. Effect storage ends at
  the highest occupied slot plus one, with source-default blank effects in
  gaps, not 32 manufactured active effects. Effect values preserve 23 mapped
  fields plus index/radii; PvP/group-size coefficients remain raw inputs
  because this source constructor does not copy them. Labels deduplicate at
  the constructor stage, not during raw join; empower milliseconds stay signed.
- `DB2Stores.cpp:1562-1563,3082-3088` preserves PPM modifier storage order and
  only assigns modifiers when the PPM record exists. One ID-only index avoids
  a full modifier scan per spell. `ObjectDefines.h:87-90::MAKE_PAIR64` and
  `FlagsArray.h:113-126` establish stance packing and zero flag128 defaults.

Startup composition creates `Runtime.spell_load_plan` after effective SQL/
locale/removal composition, shares its catalog allocation with the hotfix
serializer, and projects seeds to count effect slots. Diagnostics contain
counts only. No data installation or runtime invocation of this new path
has occurred. Sixteen written, **unexecuted** regressions cover all scalar
joins, signed keys, independent effect/cost fallback, source vector ties,
unknown enum guards/modifier warnings, invalid bounds/cycles, side indices,
all constructor fields/defaults/dependency identities, sparse blank effects,
NaN bits and raw effective hotfix/removal/unknown-coverage propagation.

Targeted `rustfmt --edition 2024` and `git diff --check` pass. No Cargo/type/test
campaign, DB write, native action/capture, runtime start, commit or publication
is claimed. This does not reset the previously exceeded ordinary 600-second
acceptance campaign. The next required work remains server-side spell data,
corrections/custom attributes/implicit target/immunity semantics and ordered
recursive skill/spell application, followed by complete inventory/Player/save
and real native initial loading/restart/relogin. Create remains disabled;
the full goal stays active and 3.4.3 compatibility is not a Forever requirement.

Read-only follow-up, **08:09–08:10 UTC**, prepares the next required source
boundary without pretending it is implemented. The isolated World schema
contains **4,400** server spell rows, **3,200** server effect rows, **141**
custom attribute rows and **five** SQL learned-spell rows. Information-schema
inspection covers all **122** columns of the four source tables; both
DifficultyID columns are signed int32 and the server SpellName is nullable.
Only metadata/counts are exposed, not spell rows or account data. A first
combined count assumed a `spell_ranks` table and failed with missing-table
1146; the corrected four-table count succeeds. No schema change or invented
empty fallback followed that failed diagnostic.

`World.cpp:1383-1417,1470-1488` freezes the required order: client SpellInfo
inputs -> server spells -> corrections -> SkillLineAbility map -> custom
attributes -> diminishing/immunities/target caps; full languages load follows
the ability map. Ranks precede learned skills, spell-specific/aura state and
learned spells. `SpellMgr.cpp:823-~905::LoadSpellRanks` derives chains from
effective SkillLineAbility.SupercedesSpell, not a SQL ranks table.

`SpellMgr.cpp:2749-2993::LoadSpellInfoServerside` reads 34 effect columns,
then 83 spell columns. These are a separate internal spell namespace, not
hotfix replacements of SpellName/SpellEffect. Server effects cannot attach
to any constructed regular spell; server spell rows cannot override even a
name-only DB2 SpellName identity. Difficulty/bounds/enum/radius checks and
observed SQL row order require their own semantic admission before publication.
Missing radius logs do not actually rewrite the raw ID in this source; the
constructor lookup produces a null dependency. Source effect records zero
the remaining DB2-only fields. Duplicate-emplace/name lifetime behavior still
needs its exact container review before implementing that mutation; it is not
declared last-row-wins by analogy with hotfix storage.

`SpellMgr.cpp:2995` custom attributes and `:1001-1162::LoadSpellLearnSpells`
remain later semantic consumers, not blindly copied flags/learn requests.
The latter has an early return on empty SQL results before DB2-derived
learning, so changing that source behavior cannot be hidden in this port.
Fresh `ss -ltnp 'sport = :18085'` still finds no World listener. None of these
server/learning sources is yet consumed by Rust, and no DB/runtime mutation,
new acceptance result, commit or publication is claimed.

#### Server-spell acquisition and owned constructor definitions — 2026-10-03 08:11–08:27 UTC

Working implementation against published HEAD `ccb99f8c`, **not compiled or
executed**. The preceding read-only counts/metadata are not acceptance of this
new adapter, registry or production startup path. Create remains disabled.

Ownership decision: consume the earlier `SpellLoadPlan` into a single
`SpellDefinitionSeeds` owner for the constructor-derived client/server
definitions. Retaining the plan plus a separately mutable spell catalog would
create redundant derived authorities; inserting server names into raw DB2
stores would violate target namespace and hotfix behavior. The selected
boundary instead moves derived fields once, retains only dependency IDs into
the canonical immutable `Arc<SpellCatalog>`, transfers client side indices,
and retires helper/PPM-load indices. Borrowed constructor views remain useful
transient inputs, not another persistent state owner. Server strings have
owned lifetime per unique definition; no self-referential pointers, raw record
clones, new trait/crate/lock or Session-owned gameplay state are introduced.
All newly added files are below 500 lines (largest: 434-line client regression
suite). Existing physical/ownership ceilings and legacy implementations are
unchanged, not rebaselined.

Adapters and source contract at
`02245dcd245e7433e524577656177723d3e4992e`:

- `wow-persistence::forever::spells::server` retains all **117** source SQL
  columns in typed, SQL-free DTOs, including 17 attribute words, two 64-bit
  stance words, two proc flag words, two aura/channel interrupt words and four
  family/class mask words. SQL DifficultyID stays int32 until the semantic
  Difficulty:int16 boundary; effect mask words stay signed SQL int32 and cast
  bit-preservingly to uint32. Nullable main names become empty raw bytes, not
  lossy text or another locale's name. Rows are deliberately not Debug.
- `wow-database::forever::ForeverSpellWorldRepository` reads the exact
  `SpellMgr.cpp:2756-2766,2873-2891` **34-column effect** and **83-column main**
  queries in that order. Both complete before returning the batch; query,
  column-count and typed-field failures remain errors. Empty successful tables
  are real empty input, not a substitute for failure. No SQL ORDER BY,
  cross-query snapshot or transaction is claimed. The exact projection tests
  are written, not executed against either synthetic rows or the live schema.
- `SpellMgr.cpp:2805-2869`: server effects cannot attach to any constructed
  regular difficulty. The check is not merely raw name presence or difficulty
  zero. Nonzero effect difficulty must exist after source enum narrowing.
  Source upper index/effect/aura/target failures skip the row; only otherwise
  admitted negative index/aura/target cases fail startup, explicitly bounding
  the source's unsafe indexing rather than fabricating support. Missing
  nonzero radii increment warnings but do not rewrite to zero; actual
  constructor lookup, including an existing ID zero, determines the dependency.
- `SpellMgr.cpp:2899-2993`: even a name-only DB2 SpellName identity prohibits
  a main server override. Main server rows do not have the effect row's
  Difficulty-presence check; the port does not invent one. Main scalar/link
  assignments retain all **55** source groups, signed aura type bits, full
  uint32 dependency IDs and untouched source-default fields. PvP/group-size
  effect coefficients remain in the acquisition DTO because the constructor
  does not assign them; server ScalingClass is zero because the source zeros
  its temporary DB2 effect and never loads that field.
- `SpellMgr.cpp:40-99` establishes unique `(SpellID,Difficulty)` identity and
  owned server names. On repeated main keys the first object/name/effects
  remain, while the source reapplies all selected scalar/link assignments.
  [Boost's hashed-index emplace contract](https://www.boost.org/doc/libs/1_89_0/libs/multi_index/doc/reference/hash_indices.html#emplace)
  confirms that failed unique insertion returns the existing blocker. The
  local Boost header was absent; no library installation or version claim is
  made. Name pointers alias the same C-string in all twelve source locale
  slots, not a client-locale fallback; the definition view preserves that and
  stops at embedded NUL. Client missing locales stay missing.
- `SpellInfo.cpp:1527-1547`: server effects apply in observed SQL row order,
  last row per effect index, with blank gaps and corrected indices up to the
  highest present slot. Orphan effect groups never create spell definitions;
  server-only language effects do not perform the separate client-load language
  registration. Client constructor fields/effects move intact and every
  dependency still resolves to the canonical raw catalog record.
- `SpellMgr.cpp:692-710`: lookup returns the exact key first, then the first
  existing fallback definition, with no guessed DIFFICULTY_NONE default or
  synthetic key. A missing difficulty ends lookup. Cycles error only if no
  definition has already been found; that safe undefined-loop admission is
  explicit. The sorted Rust iteration order is not a claim about C++'s hashed
  SpellInfo-map traversal and is not yet used for order-sensitive gameplay.

`world-server::forever::bootstrap` now acquires these two World inputs and
composes the constructor registry after final client hotfix/removal composition.
`Runtime.spell_definitions` replaces the retained load-plan field; startup
counts include client joins, source admission and derived effect slots only.
Raw hotfix serialization still reads the original shared catalog; server names
and corrections are not silently sent as replacement client DB2 records.
No World process or installed assets have been changed to invoke this path.

Written **17 additional, unexecuted** regressions: two exact SQL projections;
three complete client materialization/canonical identity/side-index cases;
four all-field server projection, sparse effects, duplicate-name/last-field
order and orphan cases; eight collision, upper/lower admission, signed enum,
exact/fallback/cycle and empty-batch cases. Numeric tests preserve NaN payloads,
signed flags and wide dependency IDs. Failures do not return a published
registry; no learned spellbook/Player state is supplied by these APIs.
Targeted rustfmt and `git diff --check` pass as hygiene, not type/test/live
acceptance. No Cargo campaign, native action, DB write, runtime start, commit
or push is claimed; the failed ordinary 600-second budget remains recorded.

The full required remainder stays in the same active goal: source corrections,
custom attributes, implicit-target/immunity and other derived spell semantics;
ordered recursive skill/spell application; complete inventory/Player/save;
and native selection/Create/initial-world/restart/relogin acceptance. These
constructor definitions alone cannot justify enabling Create or reporting the
client ready to enter the world.

#### ID-specific spell corrections — 2026-10-03 08:29–08:37 UTC

Working code, **uncompiled/unexecuted**, now adds the complete ID-specific
portion of target `SpellMgr::LoadSpellInfoCorrections` at reference
`02245dcd245e7433e524577656177723d3e4992e`. This reference contains modern
and Classic behavior; it is not a claim that inherited 3.4.3 corrections or
every referenced spell ID are native-70170 proven. Missing target definitions
stay missing. No legacy enums, wire layouts or runtime fallback are imported.

Exact contracts reviewed:

- `SpellMgr.cpp:3378-3392::ApplySpellFix`: each requested ID applies to every
  existing difficulty returned by `_GetSpellInfo`; a missing spell is logged
  and skipped. `:3394-3403::ApplySpellEffectFix` checks vector length only,
  including existing blank gaps, and skips an absent effect without appending.
- `:3405-5257` contains **191 ordered groups, 374 spell-ID requests (373
  distinct IDs) and 87 effect-fix blocks**. All are implemented, including
  attribute OR/clear, low interrupt words, scalar/effect assignments, class
  masks, dependency lookups, additive base points, custom flags and negative
  effect bits. The duplicate request for 51597 retains both source groups.
  Radius/range/duration lookup failure assigns null, not the prior link.
- Constants use this reference's `SharedDefines.h`, `SpellMgr.h:372-431`
  radii and `SpellInfo.h:143-169` custom attributes; low interrupt masks use
  `SpellDefines.h::SpellAuraInterruptFlags`. `SpellInfo.h:350-351` owns the
  initially zero custom flags and empty 32-bit negative-effects set.

`spells/definitions/corrections.rs` owns the consuming startup operation and
metadata counters. Private `general`, `encounters` and `campaigns` modules
retain source order while operating on the same private `Definition` records;
no mutable copy, Session, SQL handle or new lock is introduced. Exact ID
ranges in the canonical map select all signed difficulties. A second call
returns `IdCorrectionsAlreadyApplied`, preventing double-additive corrections.
The source executes the loader once; this guard is not a new gameplay rule.
`Runtime.spell_definitions` composition invokes this phase before Arc
publication; count-only diagnostics still state the unfinished global phase.

Nine written, **unexecuted** tests cover the exact empty request inventory;
all signed difficulties; blank versus missing slots; repeated-ID source order;
checked dependency replacement; all 17 attribute/interrupt-word preservation;
server-only flags; additive one-shot admission; and production join/raw Arc
identity. Targeted rustfmt and `git diff --check` are hygiene only. No Cargo
campaign, live action, DB mutation, runtime start, commit or publication is
claimed. The complete ordinary budget's earlier failure remains recorded.

This is **not complete LoadSpellInfoCorrections or executable SpellInfo**.
The `:5259-5321` trajectory-range/global pass depends on implicit-target
metadata and the source hashed container's observable traversal; the current
BTree iteration is not silently substituted for that pass. `:5323-5336`
also changes existing SummonProperties 121/647 Title and 628 Control.
`DB2HotfixGenerator.h::ApplyHotfix` mutates the canonical raw store, skips
missing rows, and defaults `notifyClient=false`; these calls do not insert
new hotfix metadata. That remaining raw-store transition must be integrated
without creating a second mutable raw-record authority or claiming client
notifications. The immutable raw catalog remains untouched by **this**
ID-specific definition phase. Custom attributes/derived target semantics,
recursive skill/spell learning, full inventory/Player/save and native
selection/Create/initial-world/restart/relogin remain in the same active goal.

#### Implicit-target metadata and effect queries — 2026-10-03 08:38–08:45 UTC

Working code, **uncompiled/unexecuted**, adds all 153 rows of the pinned
`02245dcd` implicit-target table. `wow-world::forever::spells::targets`
owns checked identity and pure metadata queries; its private table retains
object, reference, selection category, check and direction for every row,
including named/unlabelled NYI entries. No shared 3.4.3 enum or catch-all
unit/area fallback is reused. The readonly production `SpellEffectView`
resolves both owned target IDs through this same table, with its existing
constructor/correction admission invariant; target zero remains exact NONE/NYI.
No raw catalog copy, SQL dependency, mutable Player or new RNG is introduced.

Exact reviewed contracts:

- `SpellInfo.h:41-105` fixes enum ordinals; `SpellInfo.cpp:246-401` fixes all
  **153 × 5** entries. `:78-106` exposes exact getters and treats only AREA
  and CONE as area selection, not LINE, TRAJ or NYI. NYI rows can retain a
  non-NONE object/reference, for example target 11; they are not zero-filled.
- `:44-70::GetTargetFlagMask` includes all eleven object kinds, including
  effect-only ITEM/CORPSE_ENEMY/CORPSE_ALLY. `SpellDefines.h:311-332` supplies
  the target flag bits, not a legacy wire layout substitution.
- `:140-221::GetExplicitTargetMask` consumes the old src/dst flags, handles
  target 89's trajectory special case, then records any supplied location.
  Target A before B is observable. UNIT_AND_DEST provides destination while
  a UNIT referencing DEST does not; RAID_CLASS uses the source default UNIT
  requirement, and a corpse implicit target is not silently assigned the
  free object-type mask.
- `:108-132::CalcDirectionAngle` performs double constants/calculations before
  float conversion; RANDOM alone draws once, then multiplies two floats.
  `src/common/Utilities/Random.h::rand_norm` declares a normalized float.
  The caller supplies that capability; metadata does not create an RNG engine.
- `:455-497` fixes effect/aura queries: eight area-aura effect kinds, plus
  APPLY_AURA and APPLY_AURA_ON_PET for unit-owned auras, PERSISTENT_AREA_AURA
  for aura recognition, and nonzero ApplyAuraName. Nonzero aura data on a
  blank/DUMMY/CHARGE effect does not make that effect an aura. Area targeting
  itself has no IsEffect gate, as in the source query.

Thirteen written, **unexecuted** regressions: nine metadata/mask/direction/
identity cases and four production-view effect cases. The C++-source-derived
numeric goldens retain FNV64 `2c656d80c2bb9917` for all 765 metadata bytes
and `3fc399c1f87473a5` for all **612** explicit-mask initial states (mask
little-endian plus resulting src/dst flags). These are reviewed synthetic
source goldens, not real-client captures or an executed C++/Rust oracle.
Cases cover fixed angle bits, seven RANDOM rows, draw counts, blank slots,
both target positions and all 361 admitted effect kinds' aura classification.
Targeted rustfmt and `git diff --check` are hygiene only; no Cargo campaign,
native action, DB write, runtime start, commit or publication is claimed.

The trajectory/global correction phase is still not invoked. Read-only
inspection of `dep/boost/CMakeLists.txt` shows minimum Boost 1.74 on non-Windows
and 1.78 on Windows, not a fixed implementation or target container traversal.
`SpellMgr.cpp:40-64` uses a hashed unique first index, not ordered keys.
Current BTree iteration is therefore not claimed to reproduce the `:5259`
range-propagation pass; no fixed-point repair or guessed source order was
enabled. Remaining global corrections, three canonical SummonProperties
changes, effect-target/immunity/other derived semantics and recursive learning
must be integrated before ready SpellInfo/Player publication. Selection,
creation, complete save and native initial-world/restart/relogin remain the
unchanged active objective; the failed ordinary acceptance budget remains
recorded, and no inherited architecture baseline is reset.

#### General corrections and source traversal inputs — 2026-10-03 08:48–09:06 UTC

Working code, **uncompiled/unexecuted**, now implements all general rules in
`02245dcd245e7433e524577656177723d3e4992e::SpellMgr.cpp:5259-5336`.
`wow-world::forever::spells::SpellDefinitionSeeds::with_global_corrections`
consumes the existing definition authority; no second mutable SpellInfo,
Session lock, SQL handle or raw-record clone is introduced. It requires ID
corrections first, rejects reapplication, validates both supplied key sets
and requires exclusive `Arc::get_mut` access before changing any field.
Strong aliases and Weak observers are rejected without copy-on-write.

Exact reviewed contracts and retained order:

- `:5259-5280`: primary-index traversal, ascending effect slots, then each
  triggered spell's existing difficulty in second-index lookup order. The
  parent range is read inside each child iteration. `SpellInfo.cpp:3953-3962`
  uses negative `RangeMax[0]` by default and zero for null range. Only strict
  child-less-than-parent comparison replaces the link; equal/NaN values do
  not. This is one prescribed pass, not transitive fixed-point completion.
- `:5282-5306`: five movement effects get `SPEED_CHARGE=42.f` only for zero
  speed, zero family and absent attribute-9 delay bit. Cone repair precedes
  area-aura redirection, including cone metadata on blank effects. G3D
  `g3dmath.h:133,839-856` computes the zero tolerance in float precision.
  The canonical effect predicates in `definitions.rs` serve both this
  correction owner and `SpellEffectView`, retaining AREA/CONE versus LINE/
  TRAJ and the eight source area-aura effect kinds.
- `:5309-5321`: actual magnet aura clears both proc words; vehicle aura sets
  attribute-5 facing, icon 135754 sets passive, and single-target attribute
  with zero MaxAffectedTargets sets one. Non-aura effect data is not an aura.
- `:5323-5336`, `DB2HotfixGenerator.h::ApplyHotfix` and
  `SharedDefines.h:6664-6685`: existing SummonProperties 121/647 get Totem
  **Title=4**, and 628 gets PET **Control=2**, not slot MINIPET=5.
  `wow-data::forever_spells::apply_summon_properties_patches` mutates only
  present canonical records in place. Missing/removed rows stay missing;
  unchanged fields and availability remain intact. These source calls use
  default `notifyClient=false`, so no new hotfix metadata is inserted.

**The operation remains unconnected to production bootstrap.** The source's
Boost CMake file specifies minimum versions, not a pinned traversal. Review
also establishes that `SpellMgr.cpp:2496` first builds a **std unordered**
helper map, not a sorted map. `src/common/Utilities/Hash.h::hash_combine` and
its `std::hash<pair<K,V>>` determine that map's pair hash;
`DBCEnums.h:934` fixes Difficulty's signed int16 underlying type.
`SpellMgr.cpp:40-64,2708-2735` then inserts named clients in helper traversal
order into a Boost hashed-unique composite ID/difficulty index plus a
hashed-non-unique ID index. Server main rows subsequently call emplace in
their SQL query order, including duplicate requests.

The new private `join::HelperInputs` retains the **first accepted helper
insertion** across all 21 source joins alongside the canonical lookup map.
Unnamed helpers remain in that ID-only history because they affect hash
layout even though they never manufacture a definition. Unknown effects
skip before helper insertion; later valid joins can first insert that key.
Server admission records each request after the raw SpellName collision
guard, including duplicates. `SpellTraversalInputs` exposes only borrowed
IDs and named client membership for a future producer. It is not a copied
raw payload or a second mutable value authority. On global success these
startup histories and supplied traversal vectors retire.

`SpellTraversal::new` accepts explicitly **unverified producer output**.
Exact-set shape admission checks missing/duplicate/foreign keys in both
orders; it does not prove native hash/toolchain provenance. No BTree order,
fixed-point propagation, new native bridge, downloaded dependency or guessed
hash implementation is enabled. The source-order producer and its evidence
remain the integration prerequisite. Bootstrap moves the sole raw Arc into
the plan/definition owner and obtains the shared raw reader afterwards, so
the future global consuming call can be inserted before publication without
copy-on-write. Current bootstrap still invokes **only ID corrections**.

Nineteen new written, **unexecuted** regressions cover twelve global cases,
three raw-patch cases, all 21 helper join families/first insertion/unknown
admission, and server-request duplicate/collision history. Prescribed
forward/reverse chain cases intentionally produce different terminal ranges;
these synthetic tests are not a source-container oracle or client capture.
Alias/Weak rejection, signed difficulties, inactive/equal/NaN ranges,
movement guards, cone-before-area, fuzzy zero, flags/word preservation,
raw pointer identity/removal/coverage and history retirement are explicit.
Targeted rustfmt and `git diff --check` pass as hygiene only. No Cargo/build/
test campaign, live action, DB write, process start, commit or publication is
claimed. All working Rust since published `ccb99f8c` remains uncompiled and
unexecuted. Full custom attributes/derived spell semantics, recursive
learning, inventory/Player/save and native selection/Create/world/restart/
relogin remain in the unchanged goal; ordinary-budget failure and inherited
architecture/publication evidence remain recorded.

#### Source-container producer integrated in working startup — 2026-10-03 09:05–09:23 UTC

The global operation above now has a producer in working composition; **neither
the producer nor the working Rust is compiled/executed/installed**. Source
review of `02245dcd::SpellMgr.cpp:2580-2646` found the candidate's join order
was not exact. Empower stages must precede equipped items, labels precede
levels, powers precede reagents, and reagent currencies precede scaling.
`wow-world::forever::spells::join` and its all-21-family trace fixture now
reflect that source order. The earlier trace test was unexecuted and therefore
provided no passing evidence. Final acceptance must cover the corrected
trace together with native source traversal, not rely on the previous prose.

Architecture decision for this responsibility: retain the Rust canonical
record/rule owner and use a narrow **IDs-only native replay** in composition.
A sorted Rust map or repeated fixed point changes source-observable range
results. Reimplementing GNU/Boost's private bucket/group algorithms in Rust
would add a second version-sensitive container implementation with more
proof burden. Offline captured orders would require data/query-history
fingerprints and a regenerated artifact on every admitted input change.
The selected adapter instead executes source-shaped containers with pinned
headers/toolchain at startup. It does not execute C++ spell/gameplay code,
own raw records, introduce a plugin/framework, retain a mutable mirror,
change SQL/query order or broaden runtime authority. A reviewed equivalent
Rust producer with the same source/oracle evidence could replace this
private composition adapter without changing the domain correction owner.

Concrete ownership and failure contract:

- `world-server/src/forever/spell_traversal.rs` receives borrowed helper,
  named-client and server-request IDs from the existing seed owner. Temporary
  ABI arrays are initialized, length-checked, disjoint and caller-owned.
- Private `abi.hpp` fixes `uint32 + int16`, size 8/alignment 4; padding is
  never serialized or compared. No C++ object, name, raw record or exception
  crosses the ABI. `bridge.cpp` reconstructs a fresh std helper map using
  exact 64-bit Trinity pair-hash arithmetic. Its hash intentionally is **not
  noexcept**, retaining GNU's source hash-code caching trait. No reserve/
  rehash hint or address-based hash is added. Dummy mapped values do not
  participate in hash/equality; source Hash.h is independently used by QA.
- Named keys are only a membership filter on helper traversal. Both Boost
  index declarations retain source const uint32/signed-enum key extraction.
  Each admitted SQL emplace attempt is replayed, even a duplicate. Impossible
  helper duplicates, unknown/duplicate client membership, client-ID/server
  collisions or an incorrect final capacity fail before output publication.
- Native return catches all exceptions and sets written=0 on failure; staged
  containers retire before return. Rust validates result/count, constructs
  the two explicit domain traversals and invokes the existing consuming
  global correction. Domain exact-set/phase/raw-exclusivity checks still own
  admission. Native/helper histories retire on success; validated key order
  becomes the canonical readonly primary/per-spell collection indexes for
  subsequent source-ordered passes, not a second mutable value authority.
  `records()` uses source order after global admission; `corrected_difficulties`
  returns None before admission and Some(empty) for an absent spell afterwards.
  Arc readers are shared only after both correction phases finish. There is
  no copy-on-write or retained native container.

The explicit initial reference-server contract is [documented above](#working-spell-container-build-prerequisite):
fresh startup, Linux x86_64 GNU, GCC 15.2.0, libstdc++ 15 header date 20260321,
Boost 1.83.0, non-debug containers. Source CMake requires **C++20** at
`cmake/macros/ConfigureBaseTargets.cmake:16`; the adapter uses that standard.
This choice does not claim the client requires those libraries/compiler or
that a different C++ server host yields the same hash traversal. Source reload
after clear can retain Boost bucket capacity; reload is not implemented by
this one-shot startup operation. Cross-host/reload support needs its own
matching source evidence, not a relaxed pin.

On this host, the official archive download's SHA256 matches the release
manifest. Only `boost/` and `LICENSE_1_0.txt` were extracted into a previously
absent ignored target directory. The checksum-line tree fingerprint is
`6442dc47d86b5c718d698d65a33ab098c13074a56ff2d7ca695689ffac41b711`.
These are dependency preparation/read-only identity facts, not C++ build/test
acceptance. `build/forever_spell_traversal.rs` rejects altered/missing/extra
files, symlinks and extra root entries before compile; builds perform no
network or vendor repair. Cargo adds a feature and required-feature gate,
but no new package/lockfile dependency; cc was already optional. The legacy
binary never selects this feature.

Eight new written, **unexecuted** binary regressions cover ABI layout, empty/
capacity, signed difficulties/unnamed/duplicate history, malformed admission,
concurrent stateless replay, the production join-to-global-to-raw-publication
sequence, premature raw alias and replay-after-retirement rejection.
One additional domain regression preserves the admitted primary and per-ID
relative order for later readers without retaining replay histories. The
binary integration case also checks both canonical reader orders against its
actual producer output. Nine new Rust tests are written in this slice; none
has been run.
The independent `oracle.cpp` plus integrated
`tools/wow-test-bot/test_forever_spell_traversal_oracle.py` are also written,
**not run**. The driver obtains exact Hash.h and Difficulty declarations
from immutable pinned Git objects, not sparse/modified physical headers,
then compiles in an automatically retired temporary directory. Forty-two
positive and eight negative synthetic cases are authored, including std/
Boost bucket thresholds, 201 difficulties of one ID, forward/reverse helper
insertions, unnamed helpers, nonconsecutive SQL duplicates, uint32 extrema,
each equal_range's relative order and invalid-output nonpublication. No
production PairHash or replay implementation is imported into the reference
construction. This is not a full SpellMgr/client capture or native world test.

Targeted rustfmt and `git diff --check` pass as hygiene, not type/test/native
acceptance. No C++ build, Cargo campaign, live action, DB write, process start,
commit or push occurred. Fresh port-18085 inspection remains empty. Full
custom/derived metadata, ordered recursive learning, inventory/Player/save
and real populated selection/Create/world/restart/relogin remain in the
unchanged goal. Previous failed ordinary budget, inherited architecture
findings and publication gates stay recorded. Do not mark the complete
creation or ready SpellInfo operation finished from this startup integration.

#### Skill-line spell relations and effect-target derivation — 2026-10-03 09:23–09:33 UTC

Working code, **uncompiled/unexecuted**, now integrates target
`02245dcd245e7433e524577656177723d3e4992e::SpellMgr::LoadSkillLineAbilityMap`
and adds complete pure effect-target/fresh explicit-mask calculations.
These are required spell initialization inputs, not a learned Player,
full custom-attribute phase, executable spells or Create/save/world success.

Exact reviewed source contracts:

- `SpellMgr.cpp:1952-1963` inserts every final known SkillLineAbility relation
  into `std::multimap<uint32,entry const*>` in DB2 storage order. There is no
  SpellInfo-existence, nonzero, race/class/rank/availability filter. Signed
  Spell converts to the uint32 key. Equal keys retain insertion order.
  `:119-127::IsPartOfSkillLine` compares **SkillLine**, not SkillupSkillLineID.
  `World.cpp:1390-1396` loads this map after corrections and before custom
  attributes; later language/rank/learning responsibilities remain separate.
- `SpellInfo.h:107-112` fixes NONE/EXPLICIT/CASTER ordinals;
  `SpellInfo.cpp:959-1323` fixes **361 × 2** effect-target entries. An implicit
  NONE effect can still have a used UNIT object, and these properties must
  not be collapsed. Unknown effect IDs are rejected, not mapped to NONE.
  `:830-866` provides/misses targets using exact object masks and flag groups;
  `SpellDefines.h:339-342` supplies UNIT/CORPSE group masks. A provided corpse
  covers unit requirements, but a provided unit does not cover corpse.
- `SpellInfo.cpp:4576-4613` processes active effects only, A before B, sharing
  src/dst state across the full loop. It adds missing object flags only for
  EXPLICIT effects. Both positive/negative max ranges must equal zero before
  suppressing missing UNIT/GAMEOBJECT/CORPSE/DEST; ITEM/GAMEOBJECT_ITEM/SOURCE
  are not stripped by that branch. Null range is zero; NaN is not equal to zero.
  `DBCEnums.h:2438` effect bit `0x00100000` suppresses only that effect's
  **required** flags, not available flags or location transitions.
  `SharedDefines.h:932` attribute-13 bit `0x8000` suppresses only required
  raw Spell.Targets supplement, not required effect masks. Raw target bits
  are preserved rather than normalized through legacy enums.

Canonical ownership and composition:

`BirthCatalog::skill_ability_records/skill_ability` exposes shared final
relations without record cloning. Private `definitions::abilities` owns one
IDs-only per-spell index and a shared Arc of that same BirthCatalog allocation.
`with_skill_line_abilities` requires global/source-order admission, rejects
replacement and distinguishes unadmitted None from admitted Some(empty).
Missing definitions never manufacture spells; official/custom/final removals
precede index construction. The baseline unknown count remains an acquisition
fact, not certified final ID completeness. Experimental bootstrap invokes
this phase before publishing readers and logs metadata counts only.

Private `effect_targets` owns the complete checked 361-row metadata table;
production `SpellEffectValues/SpellEffectView` uses it for implicit kind,
provided and missing target masks. Private `definitions::target_masks`
derives a transient `ExplicitTargetMasks` result from current readonly owned
fields and raw range links. No cached initialized mask, mutable copy, new
lock, SQL handle, RNG or Player state is introduced. The explicit method name
`derive_explicit_target_masks` does **not** stand in for an admitted complete
custom-attribute phase. That phase must invoke/adopt its result in source order
when its other dependencies and rules are implemented; it is not enabled early.

Twenty new written, **unexecuted** Rust tests: six skill-map, five complete
effect metadata/missing-mask, seven target-mask and two production-effect-view
cases. They cover phase/reapplication, all relation key bits/order, matching
field, raw pointer identity/lifetime, overlays/removals/unknown counts,
every metadata cell, 49,096 effect/flag/location combinations, exact group
coverage, A/B and inter-effect location order, blank slots, zero/null/NaN
range distinctions, optional-effect/raw-target flags and pure view behavior.
Source-derived FNV64 goldens are `bbf9d1685b3a6000` (722 bytes) and
`ef3e73b4237a23e5` (361 effects × 34 flag masks × four location states,
LE32 outputs). They are reviewed synthetic goldens, not an executed native
oracle, fresh client capture or any passing Rust test result.

Source `SpellMgr.cpp:2995-3375` still requires complete custom attributes,
including SQL spell_custom_attr, Talent, SpellItemEnchantment, SpellVisual/
SpellVisualMissile/SpellVisualEffectName and LiquidType dependencies, recursive
positivity and its subsequent passes. No empty fabricated catalog or partial
classification is used to waive those requirements. Diminishing/immunity,
later source-specific/rank/learned skill/spell behavior, inventory/Player/save
and native populated selection/Create/world/restart/relogin remain required.
Targeted rustfmt and `git diff --check` pass only as hygiene; no Cargo/build/
native campaign, DB mutation, process/client action, commit or push occurred.
Fresh World-18085 inspection is empty; all working Rust after `ccb99f8c`
remains uncompiled/unexecuted. The same goal, failed ordinary budget and
architecture/publication gates remain intact.

#### Custom-attribute SQL prefix — 2026-10-03 09:34–09:43 UTC

Working code, **uncompiled/unexecuted**, connects only the initial SQL portion
of target `02245dcd245e7433e524577656177723d3e4992e`'s custom-attribute loader.
This is not the complete `LoadSpellInfoCustomAttributes` operation and does
not enable Create, learn spells, persist a Player or enter the world.

Exact source contracts:

- `SpellMgr.cpp:2995-3039` queries exactly `SELECT entry, attributes FROM
  spell_custom_attr`, preserving the observed query order and repeated rows.
  Empty is valid. Missing spell identities skip without construction; existing
  identities iterate all source second-index equal_range difficulties.
- `SpellInfo.h:143-169` fixes SHARE_DAMAGE at `0x00000008`; `0x00000002`
  is CONE_BACK and must not trigger its guard. `SharedDefines.h:1348` fixes
  SCHOOL_DAMAGE effect kind at 2. `SpellInfo.cpp:460-463,1557-1563` implements
  `HasEffect` as kind equality anywhere in the full effect vector, not an
  aura, positive-magnitude, first-effect or default-difficulty test.
- For a SHARE_DAMAGE row without that effect on one difficulty, the entire
  attribute word is skipped **for that difficulty only**. Otherwise custom
  flags are bitwise-OR assigned, retaining prior ID-correction flags and all
  unknown uint32 bits. Zero words still reach an assignment. Source `count`
  increments once for each existing-ID row even if all difficulties reject;
  it is not a count of changed spells or successful difficulty assignments.
- `World.cpp:1390-1396` orders both corrections, skill-line ability map, then
  custom attributes. The new SQL prefix is admitted after the skill map and
  source collection indexes, with repeat/replacement rejected. Completing
  this prefix must not stand in for the later derived passes.

Ownership and production composition:

`wow-persistence::forever::spells::SpellCustomAttributeRow` is the SQL-free
two-word input. The existing concrete `ForeverSpellWorldRepository` now has
a separate `load_spell_custom_attributes` operation, keeping its original
two-query server-spell batch intact. Private database `spells::custom`
requires exactly two columns and source uint32 metadata; NULL, signed/text,
wrong-width and decode failures are errors, with checked narrowing and no
flag masking or row-value logging. The entire read returns before mutation.
Empty acquisition produces an empty batch, not an invalid empty decoded row.
No new per-table trait, pool or query capability is introduced.

Private world `definitions::custom_sql` owns the consuming
`with_sql_custom_attributes` transition. It borrows the canonical keys and
mutates only existing `Definition.custom_attributes`; input rows retire and
raw catalog records are untouched. `Definition::has_effect` is the canonical
query shared by the guard and readonly `SpellDefinitionView::has_effect`.
`SqlCustomAttributeCounts` explicitly exposes **only partial SQL** evidence;
None before admission differs from Some(zero) after a valid empty batch.
The experimental bootstrap acquires rows through the existing World repository
and invokes the phase before sharing definition readers. It logs counts only,
with a continued full-custom-attributes/SpellInfo/learning/Player warning.

Twelve new written, **unexecuted** Rust tests: eight domain and four database
cases. They cover premature/repeated admission, empty batches, every signed
difficulty without manufacturing default, per-difficulty whole-word rejection,
existing-row/all-rejected source counts, absent IDs, duplicate/zero/bit-2 rows,
uint32 extrema, full-vector/effect-vs-aura queries, prior real ID-correction
flags, seventeen attribute words, negative-slot state and raw pointer identity.
Database fixtures check query/shape/primitive narrowing and the same metadata
classifier used by production; they are not actual SQLx-row integration tests.

Read-only isolated MariaDB schema/count inspection confirms `entry` and
`attributes` are nonnullable `int(10) unsigned`, with **141** rows. No rows,
credentials or private values were printed, and no DB write occurred.
Targeted rustfmt and `git diff --check` pass as hygiene, not type/build/test
acceptance. HEAD remains `ccb99f8caedec328f049b1a93a8d68142f9a4e57`; no commit
or push occurred. World 18085 remains stopped in fresh listener inspection.

Remaining derived custom-attribute phases still require Talent,
SpellItemEnchantment, SpellVisual/SpellVisualMissile/SpellVisualEffectName and
LiquidType acquisition and effective composition, source CalcValueAsInt and
recursive positivity semantics, first-pass/cross-spell and subsequent rules
in exact collection order. Existing pure target-mask derivation must be
adopted there, not relabeled as initialized early. Diminishing/immunity,
later specific/rank/learning, full inventory/Player/save and native populated
selection/Create/world/restart/relogin remain mandatory. No dependency is
waived with a fabricated empty store or partial classification. The same
active goal and previous failed ordinary-budget/architecture/publication
boundaries remain intact.

#### Custom-attribute dependency stores — 2026-10-03 09:44–10:03 UTC

The current working candidate expands the canonical spell pipeline from 36 to
**42 stores**, adding Talent, SpellItemEnchantment, SpellVisual,
SpellVisualMissile, SpellVisualEffectName and LiquidType. This implements
acquisition, raw/effective composition and full hotfix delivery inputs, **not**
the derived custom-attribute loader, SpellInfo readiness or learned Player state.
All working Rust since published `ccb99f8c` remains uncompiled/unexecuted.

Reference is still `02245dcd245e7433e524577656177723d3e4992e`:

- `SpellMgr.cpp:2995-3375::LoadSpellInfoCustomAttributes` names these dependencies.
- `DB2Structure.h` entries start at Talent:4357, SpellItemEnchantment:3987,
  SpellVisual:4236, SpellVisualMissile:4293, SpellVisualEffectName:4258 and
  LiquidType:2855. Corresponding `DB2LoadInfo.h` starts are 6092, 5483,
  5915, 5992, 5946 and 3778; their complete fields/arrays are retained.
- `HotfixDatabase.cpp:1173-1179,1660-1667,1782-1807,1826-1831` supplies
  six main projections (204 additional columns) and two additional locale
  projections. The effective batch now uses 555 main columns, 84 main reads
  and 16 locale reads; observed query/duplicate order is not sorted.
- `src/server/shared/DataStores/DB2DatabaseLoader.cpp:96-156` retains
  same-width numeric bits and distinguishes localized enUS main strings from
  plain strings. `Database/FieldValueConverters.h:64-74` round-trips casts:
  twelve unsigned SQL enchantment columns become signed metadata cells without
  losing their high bit. SQL DTOs retain unsigned widths; composition casts
  into raw signed records. This is not checked narrowing or masking.
- `src/common/DataStores/DB2FileLoader.cpp:798-803` adds four bytes per
  string-array component to the displacement address. LiquidType retains all
  six plain texture strings, 38 float cells and four coefficients. It does
  not acquire locale overlays or an enUS fallback.
- `DB2Stores.cpp:1565-1566` constructs missile-set membership in ascending
  final storage-ID order. The private secondary index stores only IDs and
  returns borrowed canonical records after all overlays/removals.

Real client metadata observation at **09:46–09:47 UTC** passes all six source
layout/ID/parent contracts. Fresh explicit acquisition at **09:58–09:59 UTC**
succeeds with 24 exact readable prefixes and 18 complete files, preserving
the previous extraction. New dependency evidence:

| Store | Hash / layout | Known / unknown direct records | Readable copies | Complete / admitted bytes |
| --- | --- | ---: | ---: | ---: |
| Talent | `F9A4265F / 147B0045` | 432 / 0 | 0 | 23954 / 23954 |
| SpellItemEnchantment | `E05AC589 / 952B72B2` | 2199 / 1 | 17 | 330036 / 329881 |
| SpellVisual | `F72496D9 / 4B85C90F` | 2273 / 19 | 3786 | 125674 / 124866 |
| SpellVisualMissile | `51A28350 / EC765EB2` | 722 / 1 | 0 | 35442 / 35390 |
| SpellVisualEffectName | `02E18F32 / 2245CEE6` | 2683 / 6 | 240 | 49315 / 49213 |
| LiquidType | `6613BED3 / D1ECEEC9` | 53 / 0 | 0 | 14222 / 14222 |

The 42-table assets are in ignored private output
`target/forever-login/client-data-spell-prefixes42-70170-20261003T095905Z`;
the suffix is a directory identifier, not the diagnostic start time.
Directory mode is 0700 and every file mode is 0600. No key, body or text value
is printed/staged. Unknown sections/copies remain unknown; no zero-filled
recovery or fabricated empty catalog. Copy counts are not materialized-row
counts. The complete reader remains strict and available mode is explicit.

Isolated acquisition diagnostics (Linux x86_64, outside ordinary Rust acceptance):

- `cmake --build target/forever-login/client-data-probe-build --target
  forever-client-data-probe --parallel 1`: exit 0, 0.805s for the final 42/24 tool.
- `ctest --test-dir target/forever-login/client-data-probe-build
  --output-on-failure`: header/prefix self-test passes, 0.03s.
- `FOREVER_CLIENT_DATA_PROBE_BIN=<private-built-tool> python3
  tools/wow-test-bot/client-data-probe/test_cli.py`: 14 pass, 0.048s.
- Fresh acknowledged local spell-group/available acquisition exits 0.
  These results prove acquisition guards/known bytes only, not Rust
  decoding, SQLx row loading, effective wire parity or character readiness.

Read-only MariaDB information-schema inspection at **09:59 UTC** confirms
all 204 new selected columns exist. Ten nullable columns are text; numeric
columns are nonnullable. Twelve enchantment columns are unsigned SQL despite
signed DB2 metadata: Charges, Effect1–3, ConditionID, RequiredSkillID/Rank,
MinLevel, MaxLevel, IconFileDataID, TransmogUseConditionID and TransmogCost.
No row values, credentials or DB mutations are involved.

Twenty new authored, **unexecuted** Rust cases cover six full serializers,
six full SQL-to-record conversions, four effective-composition/index cases,
three string-array/copy/parent cases and one strict empty-decoder/query-count
case. Existing complete-batch, metadata/prefix, query and locale tests now
include all 42 stores/eight text families. Synthetic goldens preserve invalid
UTF-8, NUL prefixes, signed/unsigned extrema, NaN payloads and every array cell;
they are not source-executed/native captures. The actual-file QA remains ten
unrun cases, now requiring the new 42-table fixture.

Targeted rustfmt and `git diff --check` are hygiene only. No Rust/Cargo/native
campaign, new World build/start, DB write, client action, commit or push occurs.
Fresh World-18085 inspection has no listener; protected legacy/runtime/
architecture policy and root Cargo manifests remain unchanged. Architecture
guidance keeps one shared canonical catalog, IDs-only indices and private
responsibility modules, not a second mutable store or new per-table crate.

The next required semantic work remains exact `CalcValueAsInt`, recursive
positivity and the ordered first/cross-spell/subsequent custom-attribute passes,
then derived diminishing/immunity and source rank/learning consumers. Full
inventory/Player/GUID/save and native selection/Create/world/restart/relogin
remain required. Create stays disabled; goal and prior failed ordinary
600-second/architecture/publication boundaries are unchanged.

#### Spell-value GameTable inputs — 2026-10-03 10:03–10:16 UTC

The working target now acquires and owns three complete spell-value text tables.
This is a mandatory input batch for later CalcValue/custom attributes, not the
calculation itself, executable SpellInfo or Player readiness. The 42 DB2 stores
and 555 hotfix SQL-column totals are unchanged. All working Rust over published
`ccb99f8c` remains uncompiled/unexecuted; Create/save/world remains disabled.

Pinned reference `02245dcd245e7433e524577656177723d3e4992e`:

- `SpellInfo.cpp:516-754::CalcValue/CalcBaseValue` needs SpellScaling and the
  item-level multipliers. `SpellInfo.h:218-250` stores BasePoints as float;
  `SpellDefines.h:490` defines the resulting SpellEffectValue as double.
  Future calculation must preserve that promotion, rounding and clamp contract.
- `GameTables.h:137-166,286-348` supplies all 24 scaling columns and the
  four-column multiplier structures, including fifteen class selectors and
  ten special negative selectors. `GameTables.cpp:157-188` /
  `ItemTemplate.h:397-434` classify inventory types; ammo/thrown/relic/default
  take Armor, not Weapon. Unknown scaling selectors have genuine source zero
  only with an existing row; missing rows remain unknown.
- `System.cpp::ExtractGameTables` pins IDs 1391660/1391670/1980632.
  Existing `GameTables.cpp:44-109` parser/source-style Linux oracle is reused.
  Rust conservatively admits finite decimal values via f64-to-f32; actual-file
  Rust differential remains required, not assumed from matching native hashes.

Production ownership: private `wow-data::forever_game_tables::spell_values`
uses the existing numeric parser; `SpellDefinitionSeeds` admits a single
immutable Arc through `with_value_game_tables`, rejecting replacement. Startup
loads all three before publishing definitions. No second mutable store, runtime
owner, crate, dependency, legacy fallback or new readiness flag is introduced.
An explicitly acknowledged read-only Rust example emits counts/fingerprints
only; it has not been built or run.

Guarded fresh acquisition **10:11:20-10:11:36 UTC**, Linux x86_64:

| File | FileDataId | Complete bytes | Numeric columns | Physical rows including zero | Full LE-f32 FNV-1a |
| --- | ---: | ---: | ---: | ---: | ---: |
| SpellScaling.txt | 1391660 | 14732 | 24 | 124 | 17553420352509375934 |
| CombatRatingsMultByILvl.txt | 1391670 | 17178 | 4 | 1301 | 14096777290496385253 |
| StaminaMultByILvl.txt | 1980632 | 41518 | 4 | 1301 | 14300170924856646253 |

Private ignored output:
`target/forever-login/client-data-spell-value-gt-70170-20261003T101118Z`.
The suffix is an artifact identifier, not start time. Root/gt modes are 0700,
all six files 0600 (the default two DB2 files and explicit Achievement prefix
are also retained). Existing assets are untouched; no keys, cells or bodies
are printed, staged or committed. Stored-file native oracle at **10:11:59 UTC**
returns exactly the same three counts/fingerprints. This is not a level-cap
or Player-formula proof.

Isolated native diagnostics **10:10:48-10:11:59 UTC**, outside ordinary acceptance:

- CMake builds `forever-client-data-probe` and `forever-initial-gt-oracle`
  with `--parallel 1`: exit 0; the initial yielded 1.001s is not full build time.
- CTest header/prefix/numeric-header self-test: one pass, 0.03s.
- `test_cli.py`: 14 pass, 0.052s.
- `test_gt_oracle.py`: ten synthetic/guard cases pass, two acknowledged
  actual Rust-consumer cases skipped, 0.018s.
- Explicit local acquisition with `--ack-spell-value-game-tables`,
  `--ack-available-achievements` and acknowledged existing public-key file:
  exit 0. Read-only `--ack-private-spell-value-gt-oracle`: exit 0.
  Commands/actual differential opt-ins are in the probe README.

Nine new Rust cases are authored, unexecuted: seven cover every class/special
selector, all 256 inventory values, row ordering/missing rows, signed zero,
complete fingerprints, invalid text and mandatory full-file admission; two
cover canonical ownership, raw catalog preservation and repeated admission.
Targeted rustfmt/diff checks are hygiene only. No Cargo campaign, World build/
start, DB write, client action, commit or push was performed.

Remaining CalcValue dependencies include ExpectedStat, ExpectedStatMod,
ContentTuning, ContentTuningXExpected and RandPropPoints, plus the exact ordered
expected-modifier reduction and item-stat selection. Startup uses no caster/
target but still requires scaling, expected stats and random variance; simple
BasePoints or zero-filled inputs are not a substitute. Then recursive positivity,
ordered custom passes, diminishing/immunity and source rank/learning remain,
followed by full Player/GUID/inventory/save and native creation/world/relogin.
The active goal and prior failed 600-second/architecture/publication evidence
boundaries remain unchanged.

#### Expected-stat and item-value dependencies — 2026-10-03 10:16–10:27 UTC

The working pipeline expands 42 to **48 canonical DB2 stores / 642 SQL columns**.
Added full fields: ExpectedStat, ExpectedStatMod, ContentTuning,
ContentTuningXExpected, RandPropPoints and MythicPlusSeason. The latter is a
real transitive dependency of expected-modifier reduction, including null-caster
startup; it cannot be assumed absent from content tuning zero. Existing indices
0-41 and the 24 available-prefix contracts are unchanged.

Pinned `02245dcd245e7433e524577656177723d3e4992e` anchors:

- `DB2Structure.h`: ExpectedStat:1646, ExpectedStatMod:1663,
  ContentTuning:987, ContentTuningXExpected:1024, RandPropPoints:3577,
  MythicPlusSeason:3152. Matching `DB2LoadInfo.h`:1904/1926/1363/1391/4881/4181;
  matching `DB2Metadata.h`:7541/7571/5064/5121/18572/15466.
- `HotfixDatabase.cpp:480-488,636-644,1277-1278,1475-1478` fixes all
  87 added projection columns and official/custom predicates. Read-only
  MariaDB information-schema inspection confirms every selected column,
  nonnullable numeric types and matching SQL DTO widths; no row values/writes.
  Queries are 96 main plus 16 locale, not a consistent SQL snapshot. The later
  storage-bound implementation below adds zero to two conditional aggregate
  reads; it does not add a DB2 store or main projection column.
- `DB2Stores.cpp:1296-1298,1327-1328,2502-2639` supplies final ascending-ID
  indexing and complete expected-stat reduction. Level/expansion last-ID wins,
  no ContentSet filter; expansion falls back to -2, genuinely missing baseline
  returns 1.0 before stat selection, None returns zero with an existing baseline.
  Modifier membership excludes missing final modifier rows. Min season is
  inclusive, max exclusive; missing season rows do not suppress the modifier.
  Nine stat fields and Warrior/Paladin/Rogue/Mage class modifiers are retained.
- `DBCEnums.h:988-1000` supplies the target ExpectedStatType values.
  `ItemEnchantmentMgr.cpp:107-176`, `ItemTemplate.h:397-434,536` and
  `SharedDefines.h:384-389` supply random-property inventory/subclass/quality
  selection, including ranged-right wand and Rare/Heirloom SuperiorF.
- `DB2FileLoader.cpp:96-107,1977-1991` supplies the two extra-parent fixture
  formats. Parent uint32 values preserve the high bit; they are not in-record
  columns or invented level/ContentTuning defaults.

Production private `value_inputs` modules implement full raw decoding, SQL DTOs/
strict decoders, consuming conversion, canonical baseline/official/custom/final
removal composition and six full hotfix serializers. No locale overlays are
added for these numeric stores. All RandPropPoints float and integer arrays
are retained, not only the startup SuperiorF[0] projection.

Private `effective::expected_values` implements EvaluateExpectedStat and
GetRandomPropertyPoints on borrowed canonical final rows. Secondary indexes
contain IDs only and are built after removals. Expected modifier f32 product
finishes before baseline multiplication, then class modifier; it is not
reassociated or promoted to double. Unknown baseline coverage is an explicit
error, not a missing-row numeric fallback. These operations are implemented
but not executed or yet consumed by complete CalcValue/custom/Player logic.

Fresh 70170/esES metadata and complete acquisition **10:18–10:22 UTC**:

| Store | FileDataId | Hash / layout | Direct rows / copies | Complete bytes | Fields / file fields |
| --- | ---: | --- | ---: | ---: | ---: |
| ExpectedStat | 1937326 | 27A7E77F / 0FD90F9C | 133 / 0 | 7087 | 12 / 11 |
| ExpectedStatMod | 1969773 | 280851FB / 8C41CCCE | 4 / 1 | 666 | 9 / 9 |
| ContentTuning | 1962930 | BD3635FF / A3E13004 | 100 / 0 | 1422 | 19 / 19 |
| ContentTuningXExpected | 2976765 | E2526D09 / 897A4313 | 29 / 0 | 723 | 4 / 3 |
| RandPropPoints | 1310245 | 5D8CEFA6 / 4FD22743 | 290 / 10 | 36254 | 10 / 10 |
| MythicPlusSeason | 2400282 | 2AD89A7A / DC94262F | 1 / 0 | 387 | 5 / 5 |

All six are complete; no new encrypted-prefix allowance is invented. Acquisition
uses the same explicit spell-info / available-spell-info group, acknowledged
public-key file and Achievement-prefix option. Private output
`target/forever-login/client-data-spell-prefixes48-70170-20261003T102145Z`
has 51 files: 48 spell/dependency files plus default classes/races/Achievement.
Twenty-four spell files are exact known prefixes, 24 complete. Root 0700,
all files 0600; previous 36/42-table assets preserved. Copy counts are not
asserted to be newly materialized-row counts. No keys/body/cells are emitted.

Isolated C++ acquisition diagnostics, not Rust/native-character acceptance:
CMake probe build at **10:17:52 UTC**, exit 0, 0.850s;
CTest header/prefix/schema self-test passes (0.03s);
14 CLI guards pass (0.049s); metadata and full acquisition exit 0.

Twenty-three new authored **unexecuted** Rust cases: six full wire goldens,
six full SQL DTO conversions, three composition/removal/duplicate cases,
one strict SQL empty-row case and seven expected-stat/item-value cases.
Existing complete-batch decoding, all schema guards and exact query tests
now cover 48 stores/642 columns; synthetic parent/IEEE/array extrema are
preserved. Ten actual-file Rust QA cases now require the new 48-table assets
and remain unrun. All working Rust since `ccb99f8c` remains uncompiled.

Targeted rustfmt/diff checks are hygiene only. No new World build/start,
DB write, character creation, client action, commit or push. Architecture
guidance preserves one canonical immutable catalog and IDs-only private
indices, with no new service/lock/crate or legacy fallback. Next semantic
work is complete null-caster CalcValue/CalcValueAsInt, variance/rounding and
recursive positivity in the exact ordered custom-attribute passes. Full
derived spell/rank/learning, Player/inventory/GUID/save and native
creation/world/restart/relogin remain mandatory. No readiness or prior
failed acceptance/performance boundary is waived.

#### Null-caster values, source index bounds and random capability — 2026-10-03 10:39–11:00 UTC

Working changes based on `ccb99f8caedec328f049b1a93a8d68142f9a4e57`,
**uncompiled/uninstalled/unexecuted**, now implement the null-caster/null-target
CalcBaseValue, CalcValue and guarded CalcValueAsInt operation. This is not the
live Player/traits/combo/mastery/modifier operation, full custom attributes,
recursive positivity, learned spells, Create or world entry.

Exact `02245dcd245e7433e524577656177723d3e4992e` source anchors:

- `SpellInfo.cpp:516-752,869-963`, `SpellInfo.h:266-267`,
  `SpellDefines.h:490`: f32 base calculations promote into f64 SpellEffectValue;
  level selection is spell/base/min/max, not MaxLevel. Source class-zero early
  return precedes coefficient arithmetic. Item-level defaults to one; -8/-9
  lookup then assert the last allocated slot, other selectors use Rare Chest.
  Optional rating/stamina multipliers query ItemSparse ID zero as written.
  Expected-stat calculation uses level one, content-tuning expansion but
  tuning/class/season arguments zero. Base and final round phases differ;
  nonrandom party aura 271 is expected-stat eligible but not final-round eligible.
  Variance scales the calculated base even with a bp override; the caller must
  supply its actual draw, never an invented mean. Final clamp is f64 ±2e9;
  undefined NaN/int32 conversions fail explicitly rather than Rust zero filling.
- `GameTables.h::GetSpellScalingColumnForClass`: named selectors dereference
  the physical row; unknown selectors return source zero without dereferencing
  it. Required absent inputs and genuine optional missing rows remain distinct.
- `DB2FileLoader.cpp:955-978`, `DB2Store.h:43-53,80-84`,
  `DB2DatabaseLoader.cpp:48-63,79-85,159-186`,
  `HotfixDatabase.cpp:1478`: regular-file allocation includes unresolved copy
  destinations, not header MaxID or the last surviving record. SQL performs a
  separate unfiltered MAX(ID)+1 after each nonempty result; UInt64 narrows to
  uint32. An overwrite-only batch can grow the physical pointer array without
  publishing GetNumRows. Final EraseRecord does not shrink that bound.
  Concurrent SQL deletion can leave a physically written row outside the public
  counter; it is not exposed as a valid final record. Defined source writes and
  out-of-bounds/uint32-overflow boundaries are kept distinct.
- `Random.cpp:28-35,60-65,78-81`, `Random.h:89-98`,
  `SFMTRand.cpp:29-48`, `dep/SFMT/CMakeLists.txt`: source TLS SFMTRand,
  entropy array of SFMT_N32 words or time fallback, uint32 RandomEngine and
  std::uniform_real_distribution<float>, SFMT_MEXP=19937/SSE2. The build
  imports original headers/seeding/library and extracts those function bodies
  verbatim into OUT_DIR; no handwritten SFMT port or legacy Rust RNG fallback.
  Only the frand max>=min ASSERT is replaced by a boundary precheck/assert.
  Pristine source noexcept construction/allocation retains its fatal failure
  boundary; the FFI catch does not turn source termination into a recoverable
  error or invent fallback entropy.

Ownership: private `wow-world::forever::spells::definitions::values` borrows
the existing definition, raw catalog, GameTables and ItemCatalog. Its result
is transient; no numeric mirror, extra Item Arc, lock, service, crate or ready
marker is introduced. Private `world-server::forever::spell_random` supplies
the exception-contained native numerical capability, not gameplay rules. It
is not called as speculative startup warmup: draws are source-order observable.
Production invocation awaits the complete ordered custom/positivity phase.

Read-only MariaDB diagnostic at **10:52 UTC**, using
`SELECT MAX(ID)+1 FROM hotfixes_forever_70170.rand_prop_points WHERE FALSE`,
confirms **LONGLONG UNSIGNED**, nullable aggregate; no IDs, values, credentials
or row changes. The DTO/converter retains the separate size observation rather
than inferring a snapshot from row IDs. Query count is 112 fixed spell/locale
reads plus zero to two aggregate reads for nonempty RandPropPoints batches.
Main projection remains **48 stores / 642 columns**. SFMT's tracked license
matches the pinned dependency SHA256
`9d6db9b6702bf80c926b36d61f6282bf48d0a9a8044db2bd31e524aa08ccac0f`.

Thirty-four **authored, unexecuted** Rust cases cover the value mapping/rounding
matrix, f32/f64 and level order, optional inputs, variance/error/cast behavior,
public definition lookup, source file/copy/SQL/removal bounds, DTO observation
conversion, native ABI and a real catalog-constructor → producer → value path.
The native source-random oracle is authored but **not compiled or run**. Its
planned 40,960 seeded float-bit comparisons use separately extracted source
RandomEngine/frand, including zero-width draws and SFMT state refills. QA
seeded state is stack-local and never reseeds production TLS. Entropy/TLS
production sequences are not falsely claimed identical to a fixed seed.

Targeted rustfmt and `git diff --check` pass as syntax/whitespace hygiene only.
No Cargo/CI/native acceptance, World restart, DB write, character save, client
action, new commit or push. Full custom/positivity/derived spell/rank/learning,
Player/inventory/GUID/save and native creation/world/restart/relogin acceptance
remain mandatory; prior failed validation/performance boundaries remain open.

#### Live ordered positivity rules — 2026-10-03 11:02–11:08 UTC

Private `definitions::positivity` now implements the complete source
`_isPositiveEffectImpl` / `_InitializeSpellPositivity` rule operation from
`02245dcd SpellInfo.cpp:4614-5107`. It is **uncompiled/unexecuted and not yet
invoked by the production full custom phase**. No public ready marker or
independent whole-catalog positivity pass is introduced.

Exact supporting anchors: `SpellInfo.cpp:465-497,1758-1761,1884-1892`,
`SharedDefines.h` target passive/debuff/buff/unique attributes, families,
effects and mechanics, `SpellAuraDefines.h` all affected aura IDs,
`SpellDefines.h:152-195` uint8 SpellModOp, `DBCEnums.h:2430` IsHarmful.
Trigger lookup retains `SpellMgr.cpp:692-710` exact/difficulty fallback,
including the actual returned definition identity in visited membership.

The existing Definition owns all 32 negative bits. Rules borrow the live
canonical collection and current value producer. A local visited set contains
keys/slot IDs only and is used solely for membership, not iteration; no row
mirror, sorted graph reduction, eager value cache or second mutable authority.
Root effects mutate their negative bit immediately after each check. Recursive
checks do **not** publish the child's result. Visited membership is shared over
all root effects, inserted only after early inactive/negative/passive/debuff/
buff/harmful returns, and is not cleared on return. CalcValue/draw happens
before family/ID/mechanic and whole-spell early returns. This preserves source
results that a simultaneous snapshot/fixpoint pass would change.

Whole-spell heal/instakill/aura checks preserve per-effect interleaving rather
than collecting flags first. Periodic-with-value/client triggers filter child
positive targets; non-aura triggers do not. Modifier operations cast misc to
uint8 and read all live negative bits, including preexisting bits outside the
current active effect vector. Additional dummy/stun/fear/taunt/transform/speed
checks inspect only **later** negative slots with identical targets, without
requiring those later slots to be active. Earlier negatives and different
targets are not generalized into another rule.

Eleven additional **authored, unexecuted** Rust cases cover early precedence,
source draw order, whole-spell interleaving, every effect kind, sign/target/
per-level aura matrices, all 256 modifier operation bit patterns, shared visits,
child nonpublication, trigger target filters, later-only matching, actual
difficulty fallback identity and invalid cycle guards. Combined with the prior
value/bounds/RNG slice, **45 cases are authored, none executed**. Targeted
rustfmt and diff whitespace checks pass as hygiene only; no Cargo/native test
campaign, build, commit, push, DB write, runtime start or client action.

The required next integration is the **complete** `SpellMgr.cpp:2995-3375`
custom-attribute phase: source primary order, live enchant-proc mutations,
binary/CalcValue draws before each definition's positivity, school changes,
talent/cone/family/target-mask/ammo/LeaveWorld work, then the source second pass
and liquid-aura flags. The source second binary pass's unusual condition is
retained, not silently repaired. Positivity must not be enabled ahead of this
ordering contract. Full derived spell/rank/learning, Player/inventory/GUID/save
and fresh native creation/world/restart/relogin evidence remain mandatory.

#### Ordered custom-attribute startup — 2026-10-03 11:09–11:22 UTC

Working `definitions::custom` now contains the complete derived portion of
`02245dcd245e7433e524577656177723d3e4992e SpellMgr.cpp:3039-3375`, after the
existing `:2995-3039` SQL prefix. It is connected in `forever::bootstrap::load`
after native global correction, the skill map, SQL custom attributes and value
GameTables, **before** sharing definitions/raw inputs with Runtime. Effective
sparse items finish first; numeric item templates and spell values share the
same item allocation. No second raw catalog/value/negative-bit mirror exists.

One source-primary traversal performs, in order, every effect slot's bleed,
aura/save/crit/direct/threat/movement/pickpocket flags and live enchantment proc
writes; binary calculation; normal-plus-magic school removal; live positivity;
talent/cone/family attributes; retained explicit masks; ammo and LeaveWorld.
Foreign enchant writes use the canonical source equal-range order and all signed
difficulties, skipping only definitions with an active proc-trigger aura.
The parent mechanic is shifted even for blank slots, while the effect mechanic
is shifted only when active; source-undefined shifts reject the consumed owner.

Binary value calculation/cast/draw precedes zero-control qualification and the
exception IDs/families. AlwaysHit skips that pass without clearing an existing
SQL bit. Family-specific CC flags occur **after** binary and positivity. Mixed
school removal retains high bits. Width uses the source double-only fuzzyNe
epsilon, not global cone-angle fuzzyEq's float epsilon. Explicit masks are
retained on the existing Definition once at the source position; the preexisting
fresh derivation API remains distinct from that retained state.

Ammo follows final canonical visual/missile/effect-name links and speed > 0;
only types 6/7 qualify. LeaveWorld reads interrupt word zero. A second primary
pass preserves the source's unusual **not-binary** condition, performs trigger
lookup using DIFFICULTY_NONE rather than the parent's difficulty, and clears
CantCrit definitions. Final liquid-store rows mark every existing signed
difficulty without manufacturing a default. Supporting anchors:
`SpellInfo.cpp:2658-2667,4576-4613,4614-5107`, `SpellInfo.h:143-176`,
`g3dmath.h:133,825-864`, `DBCEnums.h` enchantment/visual enums and
`SharedDefines.h` effect/attribute/mechanic/family/school values. Reference
files were read from pinned Git objects; stale physical reference placeholders
were neither trusted nor repaired.

The consuming API rejects missing SQL/order/skill admission, missing complete
GameTables, unknown rows in the six required dependency stores and repeat
application. Failures return an error and drop the consumed owner, not a partial
ready Arc. Counts describe this phase only, not fully executable SpellInfo.
The native TLS SFMT draw capability now has a production caller; no warmup draw,
await, packet delivery, map/entity lock or client callback is added here.

Seventeen new **authored, unexecuted** Rust cases cover admission, full primary
ordering, live recursive negative bits, random failures, finite effect/aura flag
sets, undefined shifts, enchant proc writes, binary exclusions/zero-control/
exceptions/first-match draws, cone epsilon, ammo links, LeaveWorld, crit/liquids,
talents and second-pass condition/difficulty/cycle guards. The existing target
binary native RNG case now also exercises the real client-row constructor,
native container replay, corrections, skill/SQL/GT admission and full custom
phase with shared raw identity. No synthetic traversal fixture is claimed to
prove native hash order. Targeted rustfmt and `git diff --check` pass as hygiene
only; no Cargo/build/native campaign has run and all working Rust remains
uncompiled/uninstalled. No DB/runtime/client/publication action in this slice.

Current source startup order (`World.cpp:1383-1405,1470-1486`) requires
diminishing, immunity and target caps after this phase, then ranks/required/
learned skills, specific/aura state and learned spells at their proper dependency
positions. Those operations, actual Player/skill/spell/inventory transitions,
GUID allocation and durable save, native populated selection/create/world,
unknown-COMMIT/cancellation and restart/relogin acceptance remain unfinished.
Create is still disabled; this checkpoint does not claim manual-test readiness
or replace the prior failed acceptance/performance evidence.

#### Paused integration and source checkpoint publication — 2026-10-03

The operator explicitly paused the full selection/creation/world-entry objective
to await the ongoing `3.4.3` architecture/API refactor, then authorized publishing
all current non-private work on `forever`. The checkpoint preserves authored
source, readers, diagnostic tooling and tests; it is not a release or completed
macro acceptance. It does not import/merge `3.4.3`, deploy/restart a server or
resume the paused integration. Independent Rust extraction work was subsequently
authorized in the separate `rustycore-extractors` repository.

Private client rows/assets, local configs, keys/certificates, credentials, logs,
binaries and build outputs are excluded. Source file/mime and secret-pattern
inspection precedes staging; the domain skill-directory allowlist remains narrow.
The current new Rust code remains uncompiled/uninstalled and Create disabled.
The committed-candidate final result and publication SHA are recorded at closeout;
the earlier inherited-debt waiver cannot relabel new failures or authored tests
as green. The original objective remains paused, not complete.

The first committed-candidate final at `f05c9e3d484fb7cb2a6a92dc823f39a3d80eb816`
failed in **14.57s** (14:01:07–14:01:22 UTC), before Cargo, on a NEW cross-package
`#[path]` fixture mount. Manifest `forever-snapshot-f05c9e3d-final.json` records
the failure; it is not covered by the inherited-debt waiver. The safe-refactor
skill guided a narrow test-only boundary repair: `wow-data` keeps the canonical
synthetic constructors and exposes six through the opt-in `test-fixtures` facade;
`wow-world` enables it only as a dev-dependency instead of mounting dependency
source. No constructor values, test registrations, gameplay, runtime owner or
client assets change. `unit_condition` remains crate-private. This fixture API
exists for the real cross-crate test consumer, not a temporary production bridge.
Final evidence for the repaired committed candidate follows at closeout.

Repaired code candidate `5d2c5b4ec0630bb10db0b3dbfa24843a5ae95754` ran
`validation-v2 final --base ccb99f8caedec328f049b1a93a8d68142f9a4e57 --timings`
with one Cargo job, pinned protoc and this checkout's absolute target, clean
provenance, **14:04:54–14:05:27 UTC / 32.96s**. Manifest
`forever-snapshot-5d2c5b4e-final.json` records passing hygiene, physical-file
policy (2696 files), Python compilation and rustfmt; the cross-package parser
failure is gone. Final still exits **1** on the unchanged inherited Session/
Map/character/quest/composition/Player hotspot ceilings, before production Cargo
checks or library tests. `git diff --quiet` against published `ccb99f8c` confirms
the affected Session, Map, character/quest and Player trees, composition root and
architecture checker/policy are byte-identical. No baseline was adjusted.
Publication therefore uses only the existing inherited-debt experimental waiver:
this is preserved WIP source, NOT passing build/test, fixture-runtime acceptance,
manual-test readiness or completed character/world integration. The subsequent
closeout changes only this document; its own validation retains the actual SHA.

#### Source cast-definition resolution — 2026-10-03

**13:40–13:53 UTC**, working candidate over published `ccb99f8caedec328f049b1a93a8d68142f9a4e57`,
on `forever`; immutable C++ pin remains `02245dcd245e7433e524577656177723d3e4992e`.
The previous goal delivery made implementation progress (RemoveSpell and shared
range); this continuation addresses the outstanding override-set cast selection,
not a newly narrowed selection/create/world goal. Full playable acceptance is
still unproven. No blocker, complete/paused status or new publication is inferred.

Source contracts inspected from immutable Git objects:

- `Player.h:3272`: canonical override groups are a default
  `std::unordered_map<uint32,std::unordered_set<uint32>>`. `Player.cpp:31011-31024`
  inserts into the inner set and removes by key, deleting the outer group when
  empty. No-op insert/remove calls do not create history events; group deletion
  retires its bucket/link history. Physical spellbook state is not an eligibility
  filter for override selection.
- `Player.cpp:30999-31008::GetCastSpellInfo`: visit each replacement in actual
  inner-set order, call Context.AddSpell BEFORE lookup, then map-difficulty
  GetSpellInfo. The first present candidate immediately recurses; failed lookups
  continue siblings, but a returned child is not backtracked. Unit fallback is
  reached only after this group is exhausted.
- `Unit.h:1490-1505` / `Unit.cpp:14584-14643`: one five-word zero-initialized
  visited array and zero trigger flags per wrapper. Equality precedes the empty
  slot test, so ID zero is rejected; the initial spell is NOT added up front.
  Missing definitions consume a slot. Source cycles are bounded by this exact
  context and ordinary fallback, not a new graph-cycle failure or depth guard.
  Regular aura type 332 precedes triggered type 333; both reenter the virtual
  Player phase with the SAME context/flags. The live source aura list is a
  `std::forward_list<AuraEffect*>` (`Unit.h:664`), not DB2/storage/sorted order.
- `AuraEffect.cpp:1286-1297` (`Spells/Auras/SpellAuraEffects.cpp`),
  `SpellInfo.cpp:1975-1986`: nonzero signed MiscValue promotes to uint32 and
  matches the current spell ID directly. Zero MiscValue uses family/mask: family
  zero bypasses both filters; matching nonzero family and empty effect mask
  affect all that family; otherwise any of four intersecting mask words suffice.
- `SpellAuraEffects.h:59`: GetAmountAsInt truncates the LIVE float amount to
  int32, then lookup/context promote it to uint32. Invalid nonfinite/out-of-range
  float narrowing is an explicit controlled error, not Rust saturation or a
  fabricated target. Unmatched auras do not read/narrow the amount.
- `SharedDefines.h:750,852`, `SpellDefines.h:276,280,284`: successful aura
  replacement sets OR CLEARS ignore-power (0x4) and ignore-shapeshift (0x400)
  from the source aura definition's attribute words 8/11 (0x40000/0x200).
  Type 332 clears ignore-cast-time (0x40); 333 sets it. Missing target info
  changes none of these flags. Player-only replacements retain flags.
- `SpellMgr.cpp:692-710`: existing exact/fallback lookup is reused, including
  no invented regular-difficulty fallback. Corrupt fallback cycles retain its
  controlled error rather than source nontermination or silently missing data.

Implementation: `wow-world::forever::player::spell_book::cast` owns the complete
read-only selection coordinator; `overrides` remains sole group membership/
history authority. The architecture skill guided retention of this canonical
owner and private modules rather than a second Unit/Player mirror or locked
container. `SpellDefinitionView::belongs_to` verifies the exact derived definition
and raw-catalog allocation; equal IDs/shared DB2 records are insufficient.
`CastOverrideAura` borrows a semantic definition, physical slot and live amount;
both source-ordered applied-aura lists are required and have no default producer.
Unavailable Unit data must not become empty lists or base-point amounts. There
is NO actual applied-Unit producer yet. Returned info is still not a triggered
cast, aura effect execution, persistence result or world admission.

`PlayerSpellBook::source_override_spells` admits provider length and the exact
canonical key set, rejects unknown/duplicate IDs and excludes mutable borrows.
Absent groups do not call the provider. `world-server::forever::spell_book_order::override_order`
uses the new distinct `spell_traversal/override_order.cpp` translation unit:
default GNU unordered_set insert/erase(key), numeric successful history only,
no reserve/rehash hint, payload mirror or persistent native container. The
existing Linux x64 GNU 15.2.0/libstdc++15 pinned capability and Boost-header gate
remain unchanged; unordered_map order is not offered as unordered_set proof.
All temporary allocations/validation finish before native output writes;
exceptions do not cross the ABI and failure leaves written=0/output untouched.
Lifetime history replay cost/memory remains unbounded and must be reviewed at
runtime integration; this continuation does not assert a production budget.

**Authored, all unexecuted:** 19 domain cases cover exact context, self/pair
cycles, shared missing-ID capacity, source order/zero/known-state independence,
Player/Unit phase precedence and reentry, regular/triggered flags, live amounts,
negative promotion, four-word masks, actual exact/fallback difficulties,
invalid-key provider output/failure, foreign definitions/bad slots and group
retirement versus partial history. Four additional production-linked ABI cases
cover canonical membership, full retirement, bad histories/capacity and null/
empty output contracts. Previous book/learning/removal tests remain registered.

Integrated parent-exclusive QA extends
`tools/wow-test-bot/test_forever_spell_traversal_oracle.py`: 420 independent
set histories use the EXACT `m_overrideSpells` declaration extracted from pinned
Player.h, including duplicate/no-op calls, erase/reinsert, complete outer-group
retirement and 1024 transient nodes. `cast_oracle.cpp` prepares 22 independent
cases using the actual extracted `Player::GetCastSpellInfo`, Unit resolver/context,
AuraEffect::IsAffectingSpell and SpellInfo::IsAffected bodies plus exact context/
trigger enums. Synthetic minimal SpellMgr/map/applied-aura collaborators prove
only this resolver's source contract when executed, NOT real Unit execution,
native packets, durability or effective scripts. Its numeric map fixture has
exact lookup only; real fallback is separately covered by the domain cases.
No copyrighted client assets, rows, passwords or private logs are added.

During implementation only targeted rustfmt formatting/check and
`git diff --check` passed (hygiene, NOT compile/test acceptance). The first bare
rustfmt invocation was unavailable on PATH (exit 127); rerunning the existing
binary via the documented `/home/joe/.cargo/bin` PATH completed and its check
passed. Code stays uncompiled/uninstalled. No Cargo/build/test campaign, QA
oracle execution, DB mutation, runtime/client action, commit or push. Files are
private/scoped and below 1000 physical lines: cast coordinator 240, domain tests
641, native adapter 91, new ABI tests 163, replay 48, set oracle 100, extracted-body
fixture 214 and QA driver 213; no file exception needed.

Still required: actual same-Player skills/book backedges and Unit/conditions/
criteria/collections/pet/inventory effects; effective publication hook/reentry
admission; complete Player/GUID allocation/durable save, unknown-COMMIT and
cancellation handling; compiled installed target plus populated selection,
creation/world and DB save/restart/relogin native evidence. Create remains off.
Earlier failed ordinary/publication acceptance and experimental waiver remain
unchanged; authored cases/oracles are not green evidence. Goal remains active.

#### Source RemoveSpell and skill-range sharing — 2026-10-03

At **13:24–13:38 UTC**, working candidate based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57` adds source RemoveSpell to the
same canonical book and shared immutable learning sources. This is **authored,
uncompiled/unexecuted coordination with mandatory effects**, not installed Unit,
pet/skill/inventory/mail state or client-ready creation. No production effect
executor exists; Create stays disabled.

Pinned `02245dcd245e7433e524577656177723d3e4992e` inspected source anchors:
`Player.cpp:3227-3444::RemoveSpell`, `:22473-22478::RemovePetAura`,
`:13401-13424::SetCanTitanGrip`, `:26392-26443::AutoUnequipOffhandIfNeed`,
`SpellMgr.cpp:615-622::GetPetAura`, `Unit.h:716::SetCanDualWield`,
`SharedDefines.h:1386,1501` effect IDs 40/155 and `World.h:715` uint32 config.
Prior anchors for AddSpell, rank identities, source tier clamp/backtrack,
skill ranges and publication hooks remain applicable. No inherited 3.4.3 layout
or undocumented alternate source is substituted.

The operation guards absent/Removed/Temporary/already-disabled entries first.
It asserts the next-rank definition **before** HasSpell, skips talent next ranks,
removes ordinary higher ranks with learn_low_rank=false and then required spells
in source order, using recursive default suppression flags. After those calls it
re-searches the original node, returning only if absent: the source does **not**
repeat its state/disabled guards after child effects. Captured active/dependent/
trait values survive subsequent effects. New entries erase; saved entries become
Removed; disabling preserves New or dirties another state without node erasure.
Membership witnesses change only on physical erasure, not payload mutation.

Owned-aura removal follows that mutation. Every physical effect slot, including
blank gaps, receives one required GetPetAura/RemovePetAura command. The producer
must use source `(spell<<8)+slot` uint32 lookup, real canonical pet-aura membership
and the current pet's entry-selected aura. A missing catalog is an error, not
an invented all-empty catalog. Primary profession refunds use source uint32
wrapping addition and config ceiling, not saturation or unrestricted increment.

Dependent-skill removal either zeros the source skill or restores the previous
node. Source's gap-search quirk queries **first** of a newly visited previous rank,
including first_spell_in_chain(0) when exhausting the walk; it is not normalized
to the nearest learned node. Pure rank/cap queries retain source order. Fixed
maximum nodes first clamp to previous value; zero maximum nodes use source range
and flags. Final rank/cap clamp down to the restored maximum, with no invented
minimum one on removal.

The narrow private `SpellLearningSources::learned_skill_range` now supplies the
same source tier/range/AlwaysMax calculation to AddSpell and RemoveSpell. The
safe-refactor skill guided this **same-owner private rule extraction**: original
public paths, source phase order, signed `(step-1)` promotion to uint32, uint16
narrowing, lazy level getter and prefix/failure semantics remain unchanged.
AddSpell still raises rank/cap with Classic minimum one before range/flags;
RemoveSpell separately applies its lower clamps. No SQL, packet, registry,
durability, runtime owner or mutable-state mirror changes belong to the move.
All 31 earlier learning cases remain registered, unexecuted; this is not new
passing regression evidence.

Dependent child removal consults the same reverse canonical nodes: another
different **active relationship** whose source HasSpell is true preserves the
child, even when that teacher's book active flag is false. Disabled/Removed
teachers do not preserve it. The parent node itself is not filtered by Active
or AutoLearned. If preserved, its override cleanup is skipped; otherwise the
child removal completes and nonzero override membership is removed even when
the child was absent. Source lower-rank reactivation manually syncs dependency
both directions, calls actual AddSpell with learning=false/default favorite/trait
and existing disabled flag, and publishes Superseded **without an IsInWorld gate**
when AddSpell returns true. That suppresses final Unlearned; disabled lower ranks
remain disabled. Invalid lower load's all-character cleanup remains fenced by
UnauthorizedGlobalCleanup after the already completed removal prefix.

Captured trait cleanup and complete original-ID override-group retirement follow
the lower-rank phase. Passive TitanGrip removal first removes the penalty aura,
then its required setter resets capability, both subclass masks and penalty ID;
passive DualWield removal uses its actual Unit setter. Non-passive effects do not
disable either capability. Configured offhand checking executes independently
before final Unlearned, with the caller's suppression flag only on that final
message. The complete required offhand capability is **not a field toggle**:
source can move an item to storage or, if full, remove it from inventory, save
and mail it through a transaction. Concrete inventory/mail/durability ownership
and COMMIT classification are still missing; source queued COMMIT is not an ACK
and no mocked command establishes real persistence. No such DB/mail operation
was executed in this checkpoint.

Required-spell metadata can contain a pre-mutation recursive cycle. An eligible
node repeating on the same internal next/required path is an explicit
RecursiveRemovalCycle error with earlier completed removals retained, not a
pretended source success or a rewritten relationship catalog. Admission guards
run before this path check. External aura/skill reentry starts its own path, so
legitimate same-ID reentry is not rejected by a blanket global ID guard. Source
assert/null-dereference boundaries yield MissingAssertedDefinition errors at the
corresponding phase, not invented spell metadata. Source-undefined paths are not
claimed as successful parity or automatically retried/saved.

29 additional cases are authored: 16 removal/order/admission/reentry/override
scenarios and 13 skill/pet/profession/weapon/failure cases. They cover state and
disabled guards, New erase versus saved retention, recursive flags/order, talent
exclusion, active other-teacher protection, inactive/disabled teachers, ungated
downgrade publication, manual dependency demotion, disabled lower ranks, source
required-cycle prefix errors, legitimate callback reentry and post-child missing
versus already-Removed re-search, override cleanup independent of parent active/
auto flags, source assertion timing and indirect global-cleanup fencing. Skill
cases cover tier/value/cap clamp, first-rank/zero-ID gap-search quirks, fixed
maximum previous nodes, zero-step wide-tier narrowing/AlwaysMax, uint32 profession
wrap/ceiling, physical pet slots, data unavailability, passive/non-passive weapon
flags, trait-prefix failure, offhand failure and publication order.

Recording effects are **not** real Unit/pet/SetSkill/inventory/mail/client proof.
The tiny numeric learn-node flag injector is cfg(test) inside the private
learn-spell owner, retaining existing identity/reverse indices; production fields
were not widened for test access. This fixture injection is not actual DB2
admission proof. All 29 new cases and all earlier cases remain unexecuted.
Targeted rustfmt and `git diff --check` exit 0 as hygiene only. Production removal
files are 189/63 lines, capability facade 232, shared source rule 114 and add-skill
consumer reduced from 107 to 78. Tests/fixtures are 590/409/516, below 1000; no
new physical exception. No Cargo/build/test campaign, DB write, runtime/client
action, commit or push. Working Rust remains uncompiled/uninstalled.

Actual Player and SkillSet/book effect composition, conditions, Unit/aura/pet/
inventory/mail/collection/packet executors, effective hook/invalidation admission,
unordered-set cast order, GUID/durable save, visible-pet selection and populated
native selection/create/world with cancellation/unknown-COMMIT/restart/relogin
remain required. Prior failed acceptance/performance evidence and experimental
publication waiver keep their recorded limits; this checkpoint is not green.

#### Source AddSpell and LearnSpell coordinators — 2026-10-03

At **13:10–13:23 UTC**, working candidate based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57` adds source AddSpell/LearnSpell
coordinators to the one canonical book. These are **authored, uncompiled and
unexecuted operations with required effects**, not an installed Player spell
engine, real starting learned spells, saved character or world admission.
Create remains disabled; no production SpellLearningEffects executor exists.

Pinned `02245dcd245e7433e524577656177723d3e4992e` inspected source anchors:
`Player.cpp:2761-3118::AddSpell`, `:3161-3186::HandlePassiveSpellLearn`,
`:3188-3225::LearnSpell`, `:31011-31029::Add/RemoveOverrideSpell`,
`:25544-25599::LearnDefaultSkill`; `Player.h:204-232,352` states/traits/map;
`SpellInfo.cpp:1652-1685` profession queries, `:4423-4448,4502-4524` ranks;
`SpellMgr.cpp:107-111,692-710` primary skill/definition queries;
`ObjectMgr.cpp:8022-8031,9076-9100` tier clamp/backtrack and range;
`Unit.h:944` uint8 level*5; `SharedDefines.h:504,529` cast-when-learned and
allow-outside-form bits. Packet-hook/invalidation anchors and limits remain in
the preceding dated book-order checkpoint; the hook is not silently dropped.

Private learning modules separate existing-entry transitions, add, learn,
passives, rank replacement, skill commands and scoped source admission. Shared
sources require admitted rank/required/learn-skill/learn-spell phases and the
same BirthCatalog by pointer identity as WorldSources; NumericItemTemplates is
borrowed from composition for actual IsSpellValid checks. Missing or invalid
normal learning returns false without inserting. Invalid non-world load would
invoke source **all-character** DeleteSpellFromAllPlayers; the new operation
instead returns UnauthorizedGlobalCleanup and does not execute that unapproved
global destruction or report ordinary completion.

Existing entries preserve source's early flag-match return **before** trait and
favorite updates, nondependent-to-dependent promotion without inverse demotion,
load dirty-state rules, old-trait override removal (including zero), active
transition before disable transition, disabled enabling without reinsert,
Removed erase/reinsert as Changed and Temporary conflict erasure. Active-change
returns and passive effects use captured source flags, not a later snapshot.
Previous rank learning completes before try-emplace. If reentry already inserted
the original spell, the outer insert writes Changed and its requested payload;
there is no blanket spell-ID recursion guard that rejects this legitimate path.

Rank replacement requests admitted native order and validates the exact book
key set before mutation. It skips Removed/missing definitions, but intentionally
tests old.active **without** excluding disabled entries, compares first-rank
identity and source uint8 rank, publishes before changing the affected active
flag and preserves New versus Changed. AddSpell's final bool uses captured
active/disabled/superseded_old, even where final entry.active differs. This is
not durability or guaranteed client-visible success.

Publication's mandatory capability accepts the book read-only. Its concrete
adapter **must first admit the effective OnPacketSend hooks as non-book-mutating**;
arbitrary hook-driven node insertion/erase/reentry is not implemented or enabled
by this contract. The dated source hook audit found no pinned built-in override,
not an installed-runtime guarantee. No producer is installed and this gate is
still open. Do not infer full live iterator fidelity from the read-only provider
or synthetic rank tests.

Cast precedence retains non-loading talent+LearnSpell, then passive handling,
then SkillStep, then cast-when-learned. SkillStep returns false only after an
actual required cast command. Passive handling computes stance permission;
equipped-item aura effects first test existing aura and actual item fit, call
required AddAura and return false regardless of stance permission. Otherwise
caster-aura-state checking is short-circuited by stance. Source-undefined shifts
beyond 64 are errors retaining the admitted prefix. Actual triggered Spell/Unit
execution remains required and callbacks receive the same canonical book.

Trait and dependent override membership is now canonical book state. Its
unordered-set history is retained for future cast selection; the unordered-map
order ABI does **not** prove unordered-set GetCastSpellInfo traversal. Empty
groups retire as source requires. Actual TraitDefinition/Mount lookup is a
mandatory capability: None/false may represent genuine absence only after the
real catalog is available; unavailable data is an error, never invented absence.
Primary profession points use physical Skill effects, root profession category
and source rank-one semantics, not first learn-skill-node guesses.

Learn-skill commands query live pure value/cap in source order, retain Classic
minimum one, source range precedence, uint16 narrowing and AlwaysMax. A zero
step's `(step-1)` promotes to signed int then uint32 before tier clamp/backtrack,
not uint16 wrapping or rejecting it. Existing rank/cap is not reduced. fromSkill
suppresses that dependent command. The no-node path preserves storage-ordered
ability relations, live HasSkill and Runeforging's special case, and shares the
already source-contrasted default calculation. Its required SetSkill executor
must call actual PlayerSkills::set_skill with the same full effect boundary;
the recording fixture only observes commands, not those real effects.

Dependent relationships recurse into the same book with source active/world
choices; auto-learned entries are skipped as learning calls but active overrides
are still processed. Criteria retain duplicate skill lines and source pair/order
unless PlayerLoading; mount lookup/add follows all learned state and receives
loading=!IsInWorld. LearnSpell snapshots disabled/active/favorite, calls AddSpell,
publishes captured favorite/trait/suppression only on true/in-world, then learns
disabled next/required spells or updates the quest objective on the ordinary
non-disabled path (including duplicate/failed ordinary adds).

Architecture skill influenced canonical ownership, scoped immutable sources and
one complete-use-case required capability instead of Session gameplay, mirrors,
new locks/RNG or optional no-op effects. Entry borrows end before mutable cast/
aura/skill/criteria/mount reentry. Operations are synchronous; failure retains
the completed prefix and is **not** rollback, save success or safe automatic
retry. Recursion follows source; native-stack/resource limits and concrete
reentrant Unit effects still need acceptance. No arbitrary depth cutoff is
presented as source parity.

31 cases are authored across admission (5), existing entries (5), passive/cast
(5), learning/ranks/reentry/order (11) and skill/override commands (5). They cover
invalid learning/global-cleanup fencing, canonical source mismatch, effect/data
unavailability, every important existing-entry early return, real book rank/
dependent recursion, source pre-mutation publication, the captured-active quirk,
disabled next/required recursion, duplicate quest updates, trait/favorite flags,
criteria duplicates/mount ordering, malformed order, cast/mount reentry,
previous-rank skill reentry inserting the original before outer try-emplace,
and a SQL learning cycle terminating via actual existing flags. Synthetic phase
fixtures execute actual derivation on shared numeric catalogs; their key order
and recording effects are **not native startup, real Unit/SetSkill, database,
packet-hook, durability or client evidence**. None of these cases is executed.

Targeted rustfmt and `git diff --check` exit 0 as hygiene only. Largest new
production file is the 179-line capability facade; operation modules are 36–159,
overrides 34 and the cfg(test) definition fixture 35. Largest test fixtures/
scenario files are 429/388, below the ordinary review threshold; no exception.
No Cargo/build/test campaign, DB write, runtime/client action, commit or push.
All working Rust remains uncompiled/uninstalled. RemoveSpell, actual Player
field/effect composition, conditions, Unit casts/auras/pet/profession/inventory,
override cast order, GUID/durable save, visible-pet selection and populated
native selection/create/world with cancellation/unknown-COMMIT/restart/relogin
remain unfinished. Prior failed acceptance/performance and the experimental
publication waiver keep their recorded limits; this checkpoint is not green.

#### Canonical Player spellbook and native order — 2026-10-03

At **13:00–13:09 UTC**, working candidate based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57` adds canonical spellbook state and
the complete source temporary/favorite/query operations. This is **authored,
uncompiled/unexecuted domain work**, not full AddSpell/RemoveSpell/LearnSpell,
learned starting spells, saved character or world admission. Create stays off.

Pinned `02245dcd245e7433e524577656177723d3e4992e` source anchors inspected:
`Player.h:204-232,352` exact states, signed trait bitfields and **unordered_map**;
`Player.cpp:2761-2991` existing-entry/add/rank replacement branches,
`:3130-3160` temporary operations, `:3448-3458` favorite mutation,
`:3736-3757` known/active queries, `:30929-30936` superseded publication and
`:6371-6374` SendDirectMessage; `WorldSession.cpp:223-307::SendPacket` and
`ScriptMgr.cpp:1503-1512,2433-2435::OnPacketSend` plus `ScriptMgr.h:241`.
`SpellInfo.cpp:4423-4435,4502-4524` distinguishes
rank-chain membership, first-rank identity and uint8 rank comparison. AddSpell's
remaining cast/skill/dependent/criteria/mount phases were also inspected; they
remain unimplemented as Player operations. Public GetSpellMap consumers were
located with pinned Git grep; no new mutable map escape is exposed in Rust.

`player::spell_book` owns one private map of state/active/dependent/disabled/
favorite/optional trait. States retain 0–4 values. Signed trait DefinitionId and
Rank explicitly narrow to the source 24/8-bit GNU representation. HasSpell
ignores active but excludes Removed/disabled; HasActiveSpell additionally needs
active. Neither invents a zero-ID rejection. AddTemporarySpell does no definition
lookup and never overwrites any existing node, even Removed/disabled. New entries
are value-initialized before Temporary/active/dependent/disabled assignments.
RemoveTemporarySpell erases only Temporary entries. SetSpellFavorite dirties only
Unchanged, even when assigning the existing value; other states remain intact.

All node writers use private try-emplace/erase operations. Their successful
membership events retain the bucket/link history, while payload-only changes do
not add events. This is a keys-only index witness owned by the same book, **not**
SQL, a durability journal, a second mutable Player or another RNG/lock. Replay
cannot be replaced with reconstruction from final membership: erased transient
nodes can have caused rehash, and erase does not shrink the source container.
The history currently grows with successful membership changes for this book's
lifetime; replay cost is linear in that history. This unexecuted implementation
does not establish a production performance/memory bound. Any future compaction
must prove the preserved native bucket/link state, not silently discard history.

Existing pinned Linux x64 GNU/Boost container capability now includes transient
`std::unordered_map<uint32,uint8>` membership replay, without reserve/rehash
hints. It accepts numeric Insert/Erase events only, validates duplicate inserts,
missing erasures, action/pointer/count contracts and completes fallible work
before writing. Exceptions do not cross the ABI; failure writes count zero and
leaves output unchanged. No native container survives the call. The Rust
composition adapter supplies exact disjoint arrays. Domain source_entries checks
the exact canonical key set with unique IDs, then returns borrowed entries in
provider order; set admission alone is **not order-fidelity proof**. No provider
or cached order is stored on Player.

This API is read-only traversal, not full rank replacement. Source
SendSupercededSpell constructs its packet and delegates SendDirectMessage to
Session::SendPacket; the inspected rank loop itself changes only payload flags,
not membership. **SendPacket invokes ScriptMgr::OnPacketSend with the Session**,
so const packet publication does not by itself prove no Player mutation/reentry.
Pinned Git grep found the dispatch/base hook declarations and no target
`src/server/scripts` override; that is not installed-runtime hook acceptance.
The future full coordinator must audit effective hooks and preserve or explicitly
admit the source-defined no-invalidation behavior before applying traversal
keys. Do not hold borrowed entries across arbitrary reentrant casts/learning,
insert/erase, or treat a precomputed key list as equivalent to an invalidated
native iterator. Full recursive learning and
its separate required effect/publication boundaries remain unfinished. Invalid
load's source DeleteSpellFromAllPlayers is a global destructive cleanup and is
not authorized or enabled by this component.

Architecture skill influenced private canonical ownership and a transient
IDs-only container dependency instead of native Player state or a sorted fake.
No crate, trait-per-helper, Session gameplay owner, SQL/config/packet/registry
change or new mutable mirror/lock is introduced by this book component.

Eight domain cases cover all state/active/disabled query combinations (including
zero), value initialization, every existing-node temporary guard, removal and
reinsert history, favorite same-value dirtying, signed trait narrowing, canonical
borrow/provider history and invalid-provider rejection without mutation. Four
production-linked native cases cover ABI layout, actual book/provider composition
across rehash boundaries, negative action/history/count guards with untouched
output, and null/empty/fully erased contracts. **All cases are authored only.**
The existing parent-exclusive source-container QA now extracts actual
PlayerSpellState/PlayerSpellTrait/PlayerSpell and PlayerSpellMap from immutable
Player.h objects, not manually copied reference types or production internals.
Its 420 independent cases cover 21 sizes, four ID patterns and five membership
histories, including erased transient rehash pressure, complete erase/reinsert
and actual nontrivial payloads; signed trait-field checks are also authored.
Only counts are reported. This is not yet executed order/bitfield evidence.

Targeted rustfmt and `git diff --check` exit 0 as hygiene only. Production book
and traversal files are 153/43 lines, Rust adapter 52, C++ replay 54 and oracle
138; tests are 212/155, shared oracle driver 145. No new physical exception.
No Cargo/build/test campaign, runtime/client action, DB write, commit or push.
All working Rust remains uncompiled/uninstalled. Full Player/conditions/Unit
effects, inventory/equip, GUID/durable save, visible-pet integration, populated
native selection/create/world and cancellation/unknown-COMMIT/restart/relogin
acceptance remain required. The prior failed ordinary acceptance budget and
experimental publication waiver retain their recorded boundaries.

#### Source default and reward learning — 2026-10-03

At **12:45–12:55 UTC**, working candidate based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57` now integrates the complete
default/reward admission and dispatch operations into canonical PlayerSkills.
This is **authored, uncompiled/unexecuted coordination**, not real ConditionMgr,
AddSpell/LearnSpell effects or a created character. No production executor is
installed and Create remains disabled.

Pinned `02245dcd245e7433e524577656177723d3e4992e` inspected this turn:
`Player.cpp:25544-25599::LearnDefaultSkills/LearnDefaultSkill` and
`:25678-25725::LearnSkillRewardedSpells`, `ObjectMgr.cpp:9076-9100` range rules,
`DBCEnums.h:2375-2382` acquire methods, `ConditionMgr.h:192,338-339,356-357`,
`ConditionMgr.cpp:1179-1198,2903-2915` and `Unit.h:769,944` level access.
AutomaticSkillRank is **1**, AutomaticCharLevel **2**, NeverLearned **3**,
LearnedOrAutomaticCharLevel **4**; no recalled cross-version enum is used.

Private `set::rewards` replaces the old generic `learn_rewards` callback in both
stored/absent SetSkill branches. It queries regular/fallback definitions first,
then acquire method. Method 4 evaluates nonzero ShowFutureSpellPlayerConditionID
before source-type-35 not-grouped ability conditions, **before** masks/level.
Methods 1/2 do not receive those condition checks. Missing definitions and other
acquire methods skip; race masks use the target mapping and class masks preserve
signed bits/wildcards. Required level is max(SpellLevel,BaseLevel), compared
against the source uint8 GetLevel view, not its wider stored field. Only method
1 below MinSkillLineRank removes the **exact** ability Spell ID, not first rank;
uint32 skill value narrows to int32 for the signed comparison. Other cases choose
AddSpell or LearnSpell from the live world-state query for each row. The explicit
AddSpell bool is ignored as in source; it is not durable success. SkillupSkillLine
selects the relation group, while the command's fromSkill is original SkillLine.
Every duplicate/source storage order and signed spell ID is retained.

Condition/spellbook commands remain mandatory methods on the same complete
SetSkill effect boundary, not a new trait per helper. Concrete ConditionMgr must
own its grouped-list/negative/inversion rules. Source missing conditions can
legitimately yield true **only after that actual admitted catalog/evaluator is
available**; unavailable evaluation is an error, never a fabricated true/false.
Read-only level/world queries and synchronous callbacks operate against the same
Player incarnation. No new SQL, native holder, mutable mirror, RNG or lock exists.
The architecture skill guided replacing the opaque callback with domain-owned
admission while keeping real external effects explicit and still required.

Private `set::defaults` consumes borrowed final RC records from WorldSources'
existing storage-ordered default index, not the unordered first-match index.
It checks HasSkill immediately before each individual call, then signed MinLevel
against live GetLevel, then source range/rank calculation and real SetSkill.
Prior reward/child mutations can satisfy later or duplicate skills; prior level
changes can admit later records that were absent from the original snapshot
request list. Ordinary source capacity/missing/inventory returns do not abort
the loop; required-effect errors abort the unadmitted construction with its
completed prefix, no rollback or success publication.

The safe-refactor skill guided the narrow shared-rule extraction in
`creation::skills`: same WorldSources owner, original public input-plan paths,
same source order/types/range precedence and unchanged existing tests. The new
borrowed-record/shared calculation APIs are crate-private; no packet, SQL,
registration, durability fence or runtime owner changes. Initial startup still
rejects level zero. The newly callable live rule preserves C++ uint8-to-int
promotion for a zero-level DK calculation: (-1)*5 narrows to uint16 before
max/min, rather than Rust unsigned subtraction panic. This does not relax
startup admission or alter any formerly valid nonzero startup result.

Nine new reward cases cover method/storage/missing-definition gates, exact
condition and short-circuit order, exact-vs-first-rank removal, signed value/rank,
level precedence, AddSpell=false continuation and live world switching, skillup/
fromSkill/signed-ID handling, target race/class masks and unavailable-condition
prefix failure. Six new default cases cover actual preallocation-to-SetSkill
composition, all range kinds, duplicate/reward reentry, live level changes,
signed/default/orphan filters, effect failure and ordinary capacity continuation.
One shared-rule case covers defined zero-level promotion while retaining the
initial guard. All 18 earlier SetSkill cases now use real synthetic reward
relations and the explicit command boundary where learning is expected; none
rely on an unconditional opaque reward callback. **All are unexecuted**.
The cfg(test) definition fixture sets/removes only synthetic level/condition
fields; recording commands are not a real spellbook, native-container order,
condition catalog or Player/live durability proof.

Targeted rustfmt and `git diff --check` exit 0 as hygiene only. New production
files are 48/78 lines and the effect boundary 246; the shared rule file 265.
New/default/reward/fixture test files remain below 400 lines. No Cargo/build/test
campaign, runtime/client action, DB write, commit or push occurred. All working
Rust is uncompiled/uninstalled. Concrete conditions, full spellbook/Unit casts
and aura/criteria/enchantment/mount/inventory effects, full Player/equip, GUID/
durable save and populated native selection/create/world plus cancellation,
unknown-COMMIT/restart/relogin acceptance remain unfinished. Prior failed
performance/publication evidence is unchanged; the full goal remains active.

#### Source SetSkill coordinator — 2026-10-03

Continuation **12:33–12:43 UTC**, working candidate based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57`. The complete source field/state
coordinator is authored, **not compiled, tested or production-integrated**.
Its mandatory effect boundary has no production executor and does not enable
Create. Recording callbacks are not real Spell/Unit/inventory acceptance.

Pinned `02245dcd245e7433e524577656177723d3e4992e` source inspected in full:
`Player.cpp:5768-6011::SetSkill`, `:6013-6039` Classic child detection/final
synchronization, `:25729-25759` profession slots, `Player.h:754-769` three-item
profession slot layout, `UpdateFields.h:1488` two int32 profession fields and
`SpellAuraDefines.h` aura kinds 30/400/98. Skill removal uses the existing
canonical `first_spell_in_chain`, preserving every storage-ordered skillup
relation (including duplicates and signed ID bits).

`player::skills::set` owns one synchronous coordinator and source final-sync
exit path; private `apply` owns stored/absent transitions and `professions`
owns the two linked canonical fields/queries. The scoped immutable source view
borrows BirthCatalog from WorldSources, checks the admitted RC lookup and ranks,
and requires the spell relation owner to share **that same catalog object**.
It is not a general Player/Session context. No mutable record copy, native Player,
lock, task, RNG or SQL/packet writer is added. Eight mandatory effect methods
cover enchantments, reward learning, mount updates, criteria, each aura-kind
refresh, profession item moves, inventory-full display and spell removal. They
have no defaults; there is no production no-op executor or success queue.
The architecture skill guided this complete-use-case boundary instead of
putting gameplay in Session or treating a request list as applied skills.
The future concrete Player executor remains part of this same full delivery,
not an optional reduced milestone.

Source order is preserved: parent admission, downward enchantment effects before
field writes, reward learning, upward enchantments/mount, criteria, source state
transition, profession assignment and aura refresh. Unchanged versus Deleted
relearning and New versus saved deletion remain distinct. Absent skills apply
zero-rank child requests/profession assignment, write fields, run enchantments,
then insert status before bonus refresh/rewards/criteria. Deactivation moves
profession slots one-by-one before clearing the line; inventory-full retains
earlier moves and the primary fields but **still runs final child sync**.
Final sync captures parent pure values once and queries each child live.
The stored-child parent-step gate differs intentionally from the absent-child
branch, which has no current-step guard, even for a zero-rank request.

No hidden legacy repair: the source rejects first-free slot zero; it captures
an absent parent's free slot **before** inserting children, allowing the source
slot aliasing/overwrite case. Profession lookup includes zero/signed identity
bits and assignment finds the first zero without duplicate-ID filtering,
despite its contradictory source comment. These quirks are recorded by tests,
not silently replaced with reservation/rebasing/deduplication. Full live and
persistence acceptance must retain this explicit evidence; any intentional
repair requires a separate behavior contract.

Required-effect failures and source-undefined recursive dependency cycles are
explicit errors, retaining the completed prefix without final-success publication
or a fabricated rollback. The unadmitted construction must be discarded rather
than saved/retried/published. Source ordinary missing-line return precedes the
final-sync guard; ordinary capacity/inventory returns preserve that guard.
Reentrant reward effects temporarily borrow the same PlayerSkills and source
captures/read-later status behavior is preserved, not mirrored or precomputed.
No durability claim comes from these in-memory operations.

**Eighteen authored, unexecuted recording-effect cases** cover preallocation
composition plus real bonus mutation, down/up/equal/riding order, Deleted/New/
Changed states, Classic parent copies, first RC/tier lookup and branch-specific
parent gates, final child comparisons, same-skill reward reentry, profession
projection, complete deactivation fields and duplicate first-rank spell removal,
partial bag moves/full error/final sync, successful second-slot unlearning,
zero-rank no-op, missing/capacity/slot-zero returns, source captured-slot aliasing,
high-ID narrowing, authority/phase/class guards, required-effect prefix failure
and active parent cycles. The narrow cfg(test) spell fixture derives actual ranks
on the same BirthCatalog but synthesizes definition traversal; it is **not native
container order, complete SpellInfo or real client proof**.

Targeted rustfmt and `git diff --check` exit 0 as hygiene only. Production files
are 28/218/245 lines, with test files below 400 and the rank fixture 41; no
physical exception is needed. At **12:44 UTC**, `git check-ignore -v` identified
the existing private-agent `skills/` rule as hiding all **15** files beneath
`crates/wow-world/src/forever/{creation,player}/skills`. The `.gitignore` change
adds only those two exact game-domain directory allowlists. `git status
--untracked-files=all` and ordinary `rg` now discover them; private local agent
skill/config paths remain excluded. No force-add, broad agent-context exception
or staging was performed. This packaging correction is not build/test evidence.
No Cargo/build/test campaign, DB write, runtime/
client action, commit or push. All working Rust remains uncompiled/uninstalled.
Actual reward/Unit aura/criteria/enchantment/mount/spell effects, inventory/equip,
full Player construction, GUID allocation/save, populated native selection/create/
world and cancellation/unknown-COMMIT/restart/relogin acceptance remain unfinished.
This does not replace failed performance/publication evidence or finish the goal.

#### Spell validity and Player skill fields — 2026-10-03

Working continuation at **12:26–12:33 UTC**, based on published
`ccb99f8caedec328f049b1a93a8d68142f9a4e57`, adds domain operations needed by
the complete Player construction. They are **authored, uncompiled/unexecuted**,
not a successful Create or native selection/world milestone.

Pinned `02245dcd245e7433e524577656177723d3e4992e` evidence:

- `SpellMgr.cpp:143-232::IsSpellValid` and
  `SpellInfo.cpp:1647-1650::IsLootCrafting`: physical CreateItem/CreateLoot
  effects check real template existence, zero items require the source loot
  kinds; LearnSpell recurses using regular difficulty. Only crafting checks
  positive reagent IDs, after all effects, ignoring counts. The domain uses
  `NumericItemTemplates::template`, not a raw Item.db2 presence guess.
- `Player.cpp:5750-5764::InitializeSkillFields`, `:6077-6189` skill getters,
  `Player.h:687-704,2409-2422` and `UpdateFields.h:679-693`: one canonical
  300-field array and an ID/slot/status index; only line and StartingRank=1
  are initially written. Pure ranks remain zero. Permanent field values are
  unsigned in arithmetic but the named bonus getter narrows them to int16;
  signed sums clamp only below zero, then narrow to uint16, not saturate.
- `Player.cpp:5780-5793,6015-6045`: Classic child requests with nonzero rank
  copy their live parent step/pure rank/cap for parent categories 9/11;
  missing/deleted/zero-rank parents normalize them to zero. This is admission
  input only, not executed SetSkill or its final child synchronization.
- `Player.cpp:5699-5714::ModifySkillBonus`, `Player.h:2420-2422` and
  `DB2Stores.cpp:1539-1541,3028-3031`: actual temporary/permanent field writes
  precede depth-first child traversal in the final storage-index order.
  Missing/deleted/zero-rank nodes prune all their descendants. No category,
  SkillLine-existence or skill-ID-zero gate is added. Narrowing preserves bits;
  bonus changes do not dirty save states or alter pure ranks/steps/caps.

`forever::player::skills` owns the mutable fields; its private `bonus` module
executes the mutation with borrowed iterators from the existing immutable
BirthCatalog child index. No second index, raw-record clone, lock, RNG, SQL
connection, Session state or native Player is introduced. The architecture
skill guided this canonical ownership. Status lookup is currently unordered;
no status-iteration or source persistence-order API is claimed. Full save must
establish its separate ordered transaction/durability contract.

Validity uses an explicit frame stack and exact definition-key active path,
not cached results, ID-only cycle checks or a finite depth cap. Bonus propagation
also uses an explicit active path. Source-undefined recursive cycles and signed
addition overflow produce explicit errors. A bonus error retains only completed
prefix writes, not a rollback or success; an unadmitted construction must be
discarded instead of learning/saving/publishing it. No admitted runtime currently
calls this operation. Validity is read-only and retains raw catalog identity.

Seven validity cases cover missing/noncrafting, loot exceptions, physical then
reagent query order, repeated depth-first children, early-invalid versus cycles,
fallback identity and a 4096-definition finite chain. Nine skill-field cases
cover initialization/capacity/narrowing, every inactive/live state, pure/bonus
arithmetic and Classic child normalization. Six bonus cases cover both kinds,
unchanged save states/pure values, subtree pruning, zero/missing identities,
signed overflow with exact completed prefix, active versus pruned cycles and
all 300 descendant fields. Existing source-preallocation composition additionally
checks that a bonus cannot learn a preallocated zero-rank skill. The existing
native spell-constructor composition now invokes validity against actual synthetic
numeric-template construction. These are **authored tests, not passing evidence**
or independent client/durability acceptance.

Targeted rustfmt and `git diff --check` exit 0 as hygiene only. No Cargo/build/test
campaign, DB write, runtime/client action, commit or push occurred. All new files
remain below 1,000 physical lines. Create remains disabled. Actual full SetSkill
must still execute parent admission, reward learning, criteria, enchantments,
auras, mounts, profession-item movement and RAII final child synchronization on
early returns. AddSpell/Unit effects, conditions, collections, equipment,
GUID allocation/save, populated native selection/create/world and real
cancellation/unknown-COMMIT/restart/relogin acceptance remain required. This
checkpoint does not replace failed performance/publication evidence or complete
the full goal.

#### Source first-match skill lookup — 2026-10-03

At **12:13 UTC**, working creation startup adds the exact-set race/class
lookup dependency needed by Player skill/spell operations. Pinned
`02245dcd DB2Stores.cpp:445` declares `std::unordered_multimap<uint32,
SkillRaceClassInfoEntry const*>`; :1546-1548 inserts final RC records in
ascending storage ID **only if the SkillLine exists**. :3038-3059 returns the
first equal-range candidate matching race/class. Availability, MinLevel,
tier/rank and specificity are not selection preferences. A sorted-storage
first match is not source-faithful when multiple rows match.

The existing Linux x64 GNU15.2.0/libstdc++15/20260321 container capability
now also builds `spell_traversal/birth_lookup.cpp`. It replays the default
uint32-key unordered_multimap, original insert order and equal_range without
reserve/rehash hints; only identifiers cross the ABI. Source pointer payloads
do not participate in hash/equality; the independent oracle uses the exact
extracted target typedef with stable pointer values to check this assumption.
Ascending output **group keys** are ABI organization, not a claim about global
unordered traversal; within each group native equal-range order is retained.
Input storage IDs must be strictly ascending; pointer/capacity/history errors
leave output untouched and written zero. Exceptions never cross the FFI.
All temporary native containers retire before return. No new crate, feature,
RNG, lock, raw-record clone or native Player owner is added.

`world-server::forever::birth_skill_lookup` consumes final BirthCatalog rows,
filters only absent SkillLines and passes IDs to the existing native capability.
`creation::skills::lookup` validates the exact eligible set, uniqueness and
contiguous ascending key groups, then retains only record-ID vectors. The
same WorldSources/BirthCatalog owns these lookups; selected records are borrowed
directly. Exact-set validation is not independent proof of provider ordering.
The public source query enforces valid class shift inputs, then applies only
source race/class masks. Before the lookup phase it returns an explicit missing
prerequisite, not an invented storage-order answer. Bootstrap admits this phase
after birth-skill sources and logs counts only, without enabling Player creation.
Storage-ordered default requests and this first-match lookup remain distinct.

Four authored domain cases cover first-match versus specificity/default filters,
same raw-record identity, mask mismatch, zero skill/RC IDs, orphan/empty handling,
exact-set/group order rejection and phase/repeat/invalid-class guards. Four
native cases cover ABI layout, empty input, untouched failure outputs, strict
history, deterministic/concurrent exact sets and actual final-storage producer
admission. `test_forever_spell_traversal_oracle.py` now obtains the exact typedef
from immutable Git objects and includes `birth_lookup_oracle.cpp`: **84**
pointer-payload differential cases across rehash boundaries, repeated/colliding
groups, zero and high record IDs. **All are unexecuted**, including C++ compilation;
determinism/set assertions alone are not reference-order proof.

Targeted rustfmt and diff whitespace checks pass as hygiene; no Cargo/CI/test
campaign, runtime/client action, DB write, commit or push. All new files remain
below 1,000 lines. The architecture skill guided the canonical keys-only index
and separation of container replay from matching/gameplay rules. All working
Rust remains uncompiled/uninstalled and Create stays disabled.

Next operation evidence was inspected from pinned `Player.cpp:391-565,
2761-3118,5750-6055,25525-25599,25678-25725`. Real `SetSkill` includes parent
admission, live spell/criteria/enchantment/aura effects, profession items and
RAII final child synchronization even on early returns. Classic 1.60 profession
children copy the parent's step/value/max and **must not set the parent's rank**;
`IsClassicProfessionChildSkill` tests parent categories 9/11, and
`SyncClassicProfessionChildSkills` visits all children. It is not correct to
save the current DefaultSkillRequest list as learned state or borrow a legacy
SetSkill that lacks those target-specific rules. These actual Player operations,
full inventory/equip, GUID/save and populated native selection/create/world plus
unknown-COMMIT/cancellation/restart/relogin acceptance remain required. Neither
this prerequisite nor the unexecuted oracle completes the goal.

#### Rank and learning startup — 2026-10-03

Continuation recorded at **12:07 UTC**, working candidate based on
`ccb99f8caedec328f049b1a93a8d68142f9a4e57`. Five more startup operations
are authored and wired after target caps, before immutable publication. This
is **uncompiled/unexecuted spell metadata**, not an instantiated Player,
successful Create, executable full SpellInfo or native world acceptance.

Exact pinned `02245dcd245e7433e524577656177723d3e4992e` anchors:

- `SpellMgr.cpp:823-895`: final SkillLineAbility storage builds a temporary
  ascending predecessor map and hasPrev set, then ascending roots. Later
  records overwrite links but do not remove the old destination from hasPrev.
  Converging roots overwrite shared canonical nodes; last propagation walks
  their actual mutable predecessors. All difficulties reference the same node,
  whose endpoints retain exact keys resolved by regular-difficulty lookup.
  uint8 rank increments wrap. Rootless cycles are source no-ops; reachable
  infinite walks and undefined out-of-range rank queries explicitly reject
  instead of hanging/dereferencing null. `:233-295` owns the SpellMgr rank-query
  defaults, distinct from SpellInfo's unranked GetRank default of one.
- `SpellMgr.cpp:897-956`, `SpellInfo.cpp:4435-4439,4504-4506`: requirements
  check source and required regular lookup, first-rank **definition identity**
  (including unranked self), then duplicate reverse pair. Observed SQL order
  is retained within equal keys. Source does not DAG-normalize requirements.
- `SpellMgr.cpp:958-999`: primary definition order, regular difficulty only,
  first physical Skill(118)/DualWield(40) effect. Skill uses the full existing
  null-caster CalcValueAsInt and existing native SFMT draw capability, followed
  by source uint16 narrowing; DualWield uses constant skill 118/step/value/max
  one without a draw. There are no speculative effect/difficulty draws.
- `SpellMgr.cpp:5340-5352`, `SpellInfo.cpp:2645-2655,2718-2800,2807-2989`:
  each primary definition gets Specific then AuraState. Family/ID/flag and
  real-IsAura rules preserve precedence, physical effect-zero polymorph checks,
  standing-versus-scroll branching and tracking-30645 early return. AuraState
  retains category/family/dispel before full active-effect mechanic mask,
  frost then IDs then banish. Undefined mechanic shifts/missing polymorph slots
  consume the failed phase, not partially publish its owner.
- `SpellMgr.cpp:1001-1169`: empty SQL returns **before** effect and DB2 passes.
  Nonempty-but-rejected SQL still permits those passes. Accepted SQL duplicates
  and Active values remain; talent rejection tests custom bit `0x00800000` on
  the **teaching** source. An IDs-only accepted-SQL snapshot provides membership
  without copying nodes. Regular source effects retain duplicate teaching
  pairs unless present in that SQL snapshot; TargetA pet/passive/talent/SkillStep
  determines AutoLearned. DB2 follows storage-ID order, suppresses SQL pairs
  and already-present automatic pairs, and preserves signed learned/override
  ID bits. Reverse iteration sorts teacher ID then equal-key insertion order,
  referencing the same node allocation rather than cloned relationships.
- `World.cpp:1470-1486`: rank/required precede learned skills and specific/aura,
  then learned spells. The interleaved SpellGroups responsibility and later
  proc/target-position/gameplay operations are **not** implemented by these
  five phases and are not claimed complete.

The two exact World queries are separate from the 49 DB2 stores / 668 hotfix
SQL columns. A fresh read-only information_schema observation confirms
`spell_required.spell_id/req_spell` are nonnullable signed INT; the source
`FieldValueConverters.h:58-70` round-trip conversion preserves their bit pattern
when GetUInt32 is requested. The strict Rust adapter therefore reads i32 then
casts to u32, rather than requiring unsigned SQL metadata. Learning entry/SpellID
are unsigned INT and Active unsigned TINYINT (nonzero means true). Query/decode
failure remains an error, never an empty successful batch. No rows, secrets,
credentials or raw client content were emitted by this diagnostic.

Owners/consumers: private `definitions::{ranks,required,learn_skills,specific,
learn_spells}` retain derived canonical state; the existing raw SpellCatalog
and BirthCatalog remain shared, not cloned. Specific/aura live directly on
Definition and are read through its view; rank/learning maps retain keys and
indices only. `wow-persistence::forever::spells` owns the two SQL DTOs,
`wow-database::forever::spells::learning` their strict adapter, and
`world-server::forever::bootstrap` phase sequencing, native draw and publication.
Startup logs emit counts only. No extra RNG, lock, Session gameplay owner,
raw-record mirror or retained temporary rank/SQL-node replay is introduced.
The architecture skill guided these same-owner private boundaries.

Twenty-five synthetic domain cases are authored: five ranks, three requirements,
three skills, eight specific/aura and six learned-spell cases. They include
overwrites/converging/rootless/reachable cycles, uint8 wrap/signed IDs,
admission and repeat guards, equal-key order, RNG failure/no-draw/truncation,
every classification flag/ID, precedence/undefined inputs, SQL-empty versus
rejected-nonempty behavior, exact talent/target flags, duplicate priority,
fallback lookup and shared reverse-node identity. One strict SQL query/empty-row
case and the existing real constructor/native replay/RNG composition are also
extended through all five phases. **None has run.** The native composition uses
explicitly synthetic empty ability/UnitCondition stores and one synthetic SQL
teaching pair; it is not actual-file, Player or durability evidence.

Targeted rustfmt and `git diff --check` pass as hygiene only; no Cargo/CI/test
campaign, runtime/client action, DB write, commit or push. All new production
and test files are below 1,000 lines, retaining the private domain-oriented tree.
The full authorized creation/world delivery still needs real Player skill/spell/
aura effects, inventory/templates/equip, GUID allocation and complete durable
save; populated native selection/create/world plus unknown-COMMIT/cancellation,
restart and relogin acceptance remain mandatory. Create stays disabled. This
checkpoint does not reset prior failed acceptance or the 600-second overrun.

#### Target-cap startup — 2026-10-03

At **11:46–11:49 UTC**, working `definitions::target_caps` implements all
25 groups / 35 IDs from `02245dcd SpellMgr.cpp:5422-5576`, using
ApplySpellFix's existing-difficulty contract at :3378-3393 and
SpellInfo.cpp:3655-3703 `_LoadSqrtTargetLimit`. Constructor caps default to
zero; source patches assign max 5/8 and non-diminished zero to every existing
difficulty, in admitted equal_range order. No missing spell/effect is synthesized.
The source calls never specify a non-diminished value-holder diagnostic.

Assignment precedes diagnostics: a missing holder, absent physical slot or
different CalcBaseValue increments a source-scoped warning count without
changing/discarding the hardcoded limit. Same-spell holders query the current
definition; foreign holders query the same difficulty through the canonical
exact/fallback lookup. Blank effect slots are eligible. The existing null-caster
CalcBaseValue supplies its result, then direct int32 truncation is checked;
there is no CalcValue, variance, speculative RNG draw or cached value. A lookup
cycle, failed numeric dependency or undefined NaN/overwide cast consumes/drops
the operation rather than publishing partial state.

Limits live in the canonical Definition and are borrowed/read through its
existing view, not a second spell/value owner. Bootstrap connects this phase
after immunities and before immutable publication. Five new synthetic domain
cases cover the full ordered inventory/defaults, every holder contract,
all-difficulty application/fallback versus exact priority, missing/mismatched
diagnostics, invalid casts/cycles and admission/repeat/empty guards. The native
constructor/replay/custom/diminishing/immunity composition test also includes
caps. These tests are authored **unexecuted**; all working Rust remains
uncompiled/uninstalled. Targeted rustfmt and diff whitespace checks pass only
as hygiene, not acceptance. No Cargo/CI/test campaign, runtime start, client
action, DB write, commit or push. Ranks/required/learned skills, specific/aura
states and learned spells, actual Player/inventory/GUID/save and native
selection/create/world/durability/restart/relogin remain required.

#### UnitCondition and immunity startup — 2026-10-03

Implementation continuation at **11:32–11:45 UTC**, worktree based at
`ccb99f8caedec328f049b1a93a8d68142f9a4e57`; all working Rust is still
**uncompiled/uninstalled/unexecuted**. The previous checkpoint below describes
its earlier disabled state, not the current composition.

The guarded native acquisition adds UnitCondition to `SpellInfoSchemas.h`.
The existing CMake target was rebuilt using `--parallel 1` (exit zero).
The initial acquisition was rejected before creating its output directory
because `--ack-spell-info-tables` was missing; the corrected invocation adds
that acknowledgement alongside local-data/available-spell/Achievement/public-key
acknowledgements. No key material or private path is retained here. The existing
process handle completed exit zero, without a duplicate launch after observation.
Only local build-70170/esES bytes were acquired, without client login or DB writes.
UnitCondition reports file ID 1120959, WDC5, 346 rows, four fields/total fields,
record size three, table hash `0x0E540EFD`, layout `0x215FAF83`, flags four,
locale mask `0xFFFFFFFF`, external IDs, no parent and seven copies.
The complete acquisition has 52 files: 24 spell prefixes, 25 complete spell
files, classes/races/Achievement. Root/UnitCondition modes are 0700/0600.
No asset bytes or rows are staged, printed or promoted to public fixtures.

Exact pinned `02245dcd` inputs remain DB2Metadata.h:25793-25813,
DB2Structure.h:4941-4949, DB2LoadInfo.h:6906-6939 and
HotfixDatabase.cpp:2102-2106. `wow-data::forever_spells` now owns all flags,
eight uint8 variables/operators and eight int32 values, with strict checked
reading, baseline duplicate rejection, observed official/custom overwrite order
and signed-ID final removals. Its existing typed delivery emits all 52 bytes,
excluding the external ID. SQL remains a read-only overlay, not a DB2 import.
There are now 49 spell stores / 668 main SQL columns and 114 fixed overlay/locale
queries (plus the existing conditional RandPropPoints allocation observations).
Synthetic full-field fixture/conversion/overlay/removal/byte tests are authored;
actual-file QA counts are extended but unrun. This is acquisition evidence, not
Rust consumer or real-client delivery acceptance.

Diminishing now looks up the canonical effective UnitCondition directly,
including zero, and rejects incomplete condition coverage before mutation.
Its arbitrary presence callback is retired; only the borrowed existing source
numerical selector remains. Bootstrap connects it after the complete custom
phase and before immutable publication. Native composition tests use an actual
empty synthetic catalog, not a fake production condition provider.

The subsequent immunity phase follows SpellInfo.cpp:3482-3653,
SpellMgr.cpp:5364-5420 and World.cpp:1401-1405. A read-only information_schema
observation confirms the eight projected World fields: signed int/tinyint/
smallint/bigint masks, two mediumtexts and two signed tinyint flags. No rows
were inspected or changed. `ForeverSpellWorldRepository` strictly loads the
eight-column result; the SQL-free DTO retains signed widths and raw text bytes.
Domain parsing follows SharedDefines school/dispel/mechanic widths (7/12/37),
source truncation warnings, Util.cpp:57-72 nonempty comma tokens and
StringConvert.h integral whole-token decimal parsing. Invalid tokens are skipped,
not normalized; repeated source IDs overwrite masks, append lists and OR flags.

Architecture review used design-rustycore-architecture and the module-design
guide. Retaining immutable input DTOs avoids widening them with mutable runtime
data; a second full effect store would duplicate authority, while changing every
constructor/correction DTO into a new wrapper would mix structural churn with
this behavior addition. Chosen derived state is private under the existing
Definition: sparse physical-slot immunity metadata plus the allowed-mechanic
projection, without copied raw effects/definitions. SpellEffectView borrows those
same entries; the canonical creature catalog stays in SpellDefinitionSeeds.
One synchronous consumed startup writer performs the phase; no lock, await,
Session, Player, SQL connection or callback is retained. New private files are
cohesive and below the physical budget; whole-macro policy acceptance remains due.

All eight immunity aura cases visit every existing slot, even inactive/gap
slots. Exact source ID exceptions, max-duration-100 handling, independent
attribute mechanic additions and optional-info allocation are retained.
The movement/loss-control expression evaluates to **0x49967eae**, not its stale
source comment's 0x49967ca6; disarm/silence bits are preserved. Source-undefined
mechanic/dispel shifts and INT_MIN duration abs return explicit consumed-phase
errors. Bootstrap connects immunity after diminishing, before publishing readers.
Seven new domain cases and one SQL shape/projection case are authored; the
production-linked native composition case also runs through immunity in its
unexecuted test body. These do not prove live Unit immunity application/removal.

Targeted rustfmt and `git diff --check` are hygiene only; no Cargo/CI/test campaign,
runtime start, client action, DB write, commit or push occurs here. Target caps,
ranks/required/learned skills, specific/aura states and learned spells, actual
Player/inventory/GUID/save, native selection/create/world and durability/restart/
relogin acceptance remain unfinished. Creation and world entry stay disabled.

#### Diminishing rules and source visual selection — 2026-10-03 11:23–11:30 UTC

Authored, **uncompiled/unexecuted**, `definitions::diminishing` implements
`02245dcd SpellInfo.cpp:2991-3457` classification/type/maximum/duration and
`:4526-4573` null-caster visual selection. `SpellMgr.cpp:5354-5362` traverses
the same primary source order after custom attributes (`World.cpp:1395-1399`).
The existing Definition retains constructor-default diminish info and the
once-computed result. The consumed operation owns its scoped phase counts;
errors drop it instead of returning partial state. No Player, Session, SQL,
second definition mirror or fully-ready SpellInfo claim is introduced.

Classification reads **all 32** live negative bits before active taunt, global
IDs and the exact ordered family rules. It does not infer groups from mechanics.
Type and maximum use source enums (`SharedDefines.h:6641-6661,7426-7433`).
Duration is computed independently, even after positivity/taunt/global-ID early
returns, with the source 3/4/6/8-second exceptions. Priest visual checks are
separate short-circuit calls: no first-visual substitution or cached/fixpoint
selection. Family IDs/masks remain target values, not inherited legacy rules.

Null-caster selection rejects caster-player conditions before unit-condition
lookup, ignores source-ignored viewer conditions and looks up **zero** unit
condition IDs too. An absent nonzero UnitCondition remains eligible; a present
condition rejects with null caster. It keeps the first eligible priority and
candidate order. A singleton ignores Probability without drawing; multiple
candidates supply exact f32-to-f64 weights. A no-match result still performs
the source SpellXSpellVisual ID-zero lookup. Raw visual records stay shared.

Architecture review retained one domain owner and the existing numerical
capability instead of caching visuals or creating another RNG/native spell
owner. Native selection now imports the actual pinned `Random.cpp` urand and
urandweighted bodies into the same GetRng/RandomEngine TLS used by frand.
`Containers.h:143-159` chooses discrete weights only when the ordered double
sum is positive; otherwise selection is uniform, including zero/negative/NaN
totals. The FFI rejects source-undefined positive-total discrete inputs rather
than silently repairing weights. Source headers/SFMT/toolchain/license pin
remain unchanged. Seeded QA uses temporary stack state and does not reseed
production. No eager draw is added to bootstrap.

Twelve new domain cases cover admission/default/repeat guards, the complete
source-ordered phase, identity and repeated draws, independent duration,
every single family flag bit, global/family ID precedence, active-taunt and
positive priority, monk exclusions, null-condition short circuits, absent
nonzero/zero rows, ignored viewer conditions, priorities, singleton/raw weights,
zero fallback, invalid selection and propagated failure. One new native ABI
case covers weighted/uniform inputs and pre-write guards. The real constructor
and native-replay composition case now also consumes native selection through
the diminishing operation with **explicit synthetic** condition presence, not
a live UnitCondition claim. The native oracle adds 61,440 exact choices to its
40,960 float draws; none of these new tests/oracle comparisons has run.
Targeted rustfmt and `git diff --check` pass only as hygiene, not acceptance.

**Production diminishing integration is intentionally pending complete
UnitCondition inputs.** Source `DB2Structure.h:4941-4949`,
`DB2Metadata.h:25793-25813`, `DB2LoadInfo.h:6906-6939` define complete flags,
eight uint8 variables/operators and eight int32 values (26 SQL columns),
file ID 1120959/layout 0x215FAF83. `DB2Stores.cpp:401,1044` loads the store;
`HotfixDatabase.cpp:2102-2106` defines the official/custom queries. A read-only
information_schema query on isolated hotfixes_forever_70170 verified those 26
columns/types plus VerifiedBuild, without reading cells or changing the DB.
Next implementation must retain effective baseline/overlays/removals and
typed delivery on the existing catalog; a nonzero-ID heuristic, copied ID
mirror or fake empty production store is not the contract. No new DB2 store
or SQL column is counted as integrated yet.

No build, native campaign, runtime start, client action, DB write, commit or
push in this slice. Immunities/target caps, ranks/required/learned skills,
specific/aura state and learned spells, actual Player/inventory/GUID/save,
native selection/create/world and durability/restart/relogin acceptance remain
unfinished. The full goal stays active and prior failed acceptance evidence
remains in force.

### Initial skill-field sources and ordered default inputs — 2026-10-03 06:59 UTC

Working code, **not compiled or executed**, now composes the effective Birth
catalog into `WorldSources` through `with_birth_skills`. `Runtime.birth` and
the source index share the same immutable `Arc<BirthCatalog>` allocation;
pair indices contain IDs only, not cloned raw records or learned-rank mirrors.
`initial_skill_fields` returns a transient field/input result, not a Player,
serialized update fields, learned skills, or persistence success.

Exact `02245dcd245e7433e524577656177723d3e4992e` contracts:

- `Player.cpp:5750-5765::InitializeSkillFields`: iterate SkillLine storage IDs,
  include any race/class-matching RC record without Availability/MinLevel filters,
  preallocate SkillLineID and StartingRank=1; learned Rank/Step/bonuses still zero.
  `Player.h:154` / `Updates/UpdateFields.h:681` declare **300** slots. This is
  source-bound initialization, not native 70170 update-field layout proof.
- `DB2Stores.cpp:3038-3050`: race masks use target RaceMask mapping and zero
  wildcards; signed ClassMask zero/-1 are preserved. Only **existence** is used
  for preallocation, so no arbitrary sorted winner replaces its unordered index.
- `ObjectMgr.cpp:4103-4120`: availability-one default RC inputs retain ascending
  DB2 storage ID, including duplicate SkillIDs, across valid World definitions.
  `Player.cpp:25544-25602`: signed int8 MinLevel filters at the selected level;
  HasSkill must be checked immediately before each actual call, after preceding
  spell/skill effects. The request result does not preempt that mutable decision.
- `ObjectMgr.cpp:9076-9100::GetSkillRangeType`: a present tier takes precedence
  over runeforging (960), armor category eight and language category ten;
  missing SkillLine is RANGE_NONE. A missing tier means range fallback, while
  a known zero-valued tier stays a rank range. Signed tier IDs promote to uint32,
  not uint16 aliases. Source tier values narrow uint32 -> uint16 at SetSkill input.
- `Player.cpp:25562-25602` / `Unit.h:944`: languages are 300/300, armor/runeforging
  one/one, ordinary caps are level*5, AlwaysMax is flag 0x10. DK starts at
  max(1,(level-1)*5), capped by the real level/tier maximum. Display-only mono flag
  0x400 is not a server range override. Zero player level/invalid class shifts fail
  before publication; an omitted skill-source composition is an explicit error.

Implementation owners: `wow-world::forever::creation::skills` owns immutable
pair indices/transient requests; `wow-data::forever_birth` remains the raw
effective record authority; `WorldSources::skill_tier_value` retains canonical
World tier values. `world-server::forever::bootstrap` attaches the same Birth
owner and reports counts only. Six written, unexecuted tests cover preallocation/
capacity, masks/storage order/duplicates, range/tier/narrowing/zero cases, DK
values, effective removals/shared authority, and missing composition/identity.
No new crate, SQL query, trait, mutable lock, legacy Player/Session rule or
architecture baseline/ceiling is introduced.

The rest of `SetSkill` is still mandatory: `Player.cpp:5769-6050` refreshes
auras, grants/removes rewarded spells, updates criteria and enchantments,
resolves parent/profession-child state and synchronizes children. None of those
effects is simulated as successful by this source result. `SpellMgr.cpp:2496+`
must construct genuine target SpellInfo, followed by its `LoadSpellLearnSkills`
and `LoadSpellLearnSpells`; copying the older SpellInfo wire/data model or using
an empty spellbook would not complete the creation operation.

Fresh read-only SQL metadata: 59 skill tiers, 292 raw playercreateinfo rows,
zero RC SQL rows with Availability=1, zero Characters. These counts do not
prove effective baseline/default skill coverage or decoded results. Formatting/
diff hygiene only; no Cargo check/test, actual-file Rust result, runtime start/
asset update, native action, learned/saved skill, DB write, commit or push.
Full Player/Create/persistence and populated selection/world entry stay open;
the full goal remains active, not achieved, blocked or manual-test-ready.

- `ObjectMgr.cpp:3849-3975`: Map coordinate/non-instance admission, both effective
  race/gender models, NPE transport spawn and optional Movie/Scene validation.
- `ObjectMgr.cpp:3319-3420,3974-4100`: loadouts and their items use effective
  **Item + ItemSparse DB2**, item effects/metadata and SQL addon/override data;
  there is no required `item_template` SQL table in this target schema.
- `ObjectMgr.cpp:4103-4120` / `Player.cpp:25544+`: SkillRaceClassInfo,
  SkillTiers and spell-derived skill effects; knowing a skill ID is insufficient.
- `Player.cpp:391-561,2360-2392,25086+` / `DB2Stores.cpp:2189-2200,2866+`:
  starting level/money, XP, health/power, taxi masks, talents/spec, professions,
  valid actions, transmog and fully instantiated inventory in source order.
  **BaseMP and Xp are `gt/*.txt` GameTables**, not DB2 files. No new-character
  InitGlyphs call is invented; later load/initial packets are separate.
- `Player.cpp:20665-21078::SaveToDB`: core row plus every nonempty creation-linked
  customization/inventory/spell/action/skill/reputation/stat/etc. save family,
  and the independent Login last-player/count transaction.

GUID authority and durability are not bypassed. Source
`ObjectMgr.cpp:7375-7383::SetHighestGuids` seeds Player/Item sequences from actual
MAX values; `ObjectGuidSequenceGenerator.cpp:39-54` checks the 40-bit counter
overflow boundary. Source `CharacterHandler.cpp:1000-1047` waits for Character
commit but merely **queues** Login work before cache/script/Create success;
destroying Session can discard its callback while Character work commits.
The implementation still needs one canonical execution/GUID owner, an explicit
unknown-COMMIT/cancellation/reconciliation/publication contract and a real
save/restart/relogin test. This batch does not decide or conceal a repair of
those source semantics, create a second Player mirror, or imply distributed ACID.

The separate `wow-packet::forever::{character_result,character_list}` codecs
follow `CharacterPackets.cpp:220-461,518-524`: uint32 Create code + packed GUID;
local character body with nineteen visual items, customizations, timestamps,
tabard, **SuperDistrict** and **surname**, followed by restrictions/mail.
`PacketOperators.h::SizedCString` (mail) is distinct from Name/Surname's
SizedString. They do not prepend the old u16 opcode or register handlers.
Source Classic translation of core `4501AC` suggests **4601AB**, but no native
70170 Create response or populated enum has established that transport hypothesis.
Local enum exposes zero region-wide/warband sections as an explicit bounded DTO,
not as proof those account responsibilities are empty or permanently out of scope.
Existing native-validated empty enum remains unchanged. Synthetic fixtures must
not be presented as native acceptance or private client rows.

The working candidate over `ccb99f8c` now integrates the **local normal-list
operation** (2026-10-03 04:41 UTC); it is not installed or accepted yet. Exact
sources at `02245dcd` are `CharacterDatabase.cpp:35-47,68-87` (28 base columns,
19 × eight equipment columns, optional declined-name column and ordered choices),
`CharacterPackets.cpp:88-204,220-289,410-412` (body/flags/default restrictions),
`CharacterHandler.cpp:412-475,559-665` (appearance/legitimate characters),
`ObjectGuid.cpp:887-912,966-970`, `Services/ClubUtils.cpp:21-24`,
`CharacterCache.cpp:74-90`, `DB2Stores.cpp::GetChrSpecializationByIndex`,
`SharedDefines.h::CharacterFlags{,2,3}` and `Player.h::{PlayerFlags,AtLoginFlags}`.

Ownership is `wow-persistence::forever::selection` for private SQL-free rows,
`wow-database::forever::selection` for the complete holder and checked decoding,
`wow-world::forever::selection` for pure target projection, and the existing
canonical `Session`/registered enum thunk for admission/publication. No opcode
match or legacy Player/CharacterCache mirror is introduced. Both queries must
succeed before rows are published; this is not repeatable-read/snapshot evidence.
LEFT JOIN NULL values become source zero without hiding decode/width errors.
Optional genitive remains at index 180; surname is appended independently.
SQL is static after its two fixed shapes are cached, not an owned-statement API
expansion. Schema metadata reads confirm 153 equipment columns and the existing
surname field; no schema/account/character data was mutated.

Projection preserves query order and truncates at the source's 200 local rows;
it groups choices by guid, uses the saved talent-group index (not creation's
default specialization), source uint16 spec conversion, service-flag priority,
resurrection-before-ghost/pet visibility, all nineteen signed/unsigned equipment
fields, default restriction/mail data and maximum level starting at one.
Unrepresentable/duplicate Player counters fail closed as an explicit integrity
guard, not a claim that C++ rejects its corrupt-data cases. The same field-based
appearance validator serves Create observation and enum; its adjacent-duplicate
test now matches C++ `==`, with sorted choices supplied by the holder. Arbitrary
250/200 defensive caps were removed from enum's uint32-count vectors; Create's
fixed 250-choice request bound is unchanged.

The Session revokes its previous legitimate set when refresh starts, enters a
Selecting phase, and installs the new set only after full encoding and required
writes. Billing/transfer-locked rows are not authorized. A cancelled pending
operation cannot resume ordinary authenticated handlers; transport close/drop
ends the incarnation. Error/close clears the set. Invalid appearance clears
choices and requests `at_login |= 8` only when no existing service flag applies;
the source's Flags2 assignment (not OR) is retained. **Intentional execution
contract:** source queues Execute, whereas this isolated adapter awaits each
idempotent OR before publication and fences it by account/deleted-state ownership.
A zero-row acknowledgement is confirmed against the intended flag/owner. A lost
ACK, failed confirmation or cancellation can leave an applied OR, but produces no
list/authority; a fresh session retries the idempotent operation. Multiple writes
are not a transaction and no all-or-nothing durability claim is made.

Surname is read from its canonical persisted row per holder, without duplicating
the source's global mutable cache or claiming parity with a stale cache after
external edits. Collections remain explicitly empty-only at account admission,
so an empty item-appearance ownership set is supported by that admission, not
guessed. Eligible non-ghost pets with a nonzero entry remain **UnsupportedState**
until effective CreatureTemplate/family integration is ported. Ghost pets are
hidden as source requires. This boundary does not retire full selection/create/
world requirements or declare raw SQL pet absence.

Read-only Auth metadata finds no `contentSetId` column: the already implemented
BNet adapter intentionally uses `Forever.RealmBindings` instead. World now reads
the same exact-field/bounded configuration for isolated realm `0x02010001`, checks
source content mapping (136→PvP/1,137→Normal/2,138→RP/3,140→Hardcore/4), and uses
that content/district in GlueScreen and the local character body. Missing or
conflicting bindings fail startup; no fake missing-column SQL fallback, migration,
or BNet realm-state override is applied. Sources are existing
`bnet-server/realm/forever/bindings.rs`, target `Realm.cpp:50-74`,
`RealmList.cpp:330-341`, `AuthHandler.cpp:118` and
`CharacterPackets.cpp:260`. Correcting default Glue content 137 to bound content
136 is an observable initial-byte change requiring a fresh native capture.

Sixteen new regressions cover SQL column order/optional projection, populated
body/guids/equipment/saved spec/flags, billing/resurrection, invalid appearance,
unsupported pet boundaries, local truncation, guid-grouped choices, configured
ruleset mapping, normal-list authorization/publication, write/codec failure,
cancelled refresh, byte-identical empty enum and enum vectors above 250 choices.
Two isolated-binding tests cover source mapping and rejection. All are written
but **unexecuted**. Hygiene only: `cargo fmt --all` exit 0 and `git diff --check`
exit 0 at 04:40–04:41 UTC. No Cargo build/test, SQL-row integration run, new native
capture, World restart, commit or push occurred. The source/packet/data changes
remain within the full pending macro; real Create/save/restart/relogin and initial
world loading are still required before playable/manual-test-ready claims.

Acceptance and publication evidence for this candidate is recorded below once
executed; implementation by itself is not passing evidence.

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
