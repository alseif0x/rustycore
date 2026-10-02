# WoW Forever 1.60.1 login-only runbook

This is a bounded, local smoke for the WoW Forever client build `1.60.1.70170`.
It exercises RustyCore's normal Battle.net REST SRPv2 flow and protobuf RPC realm
list flow against an isolated Auth database. It does not start `world-server`,
authenticate a World `AuthSession`, or prove real-client login.

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
