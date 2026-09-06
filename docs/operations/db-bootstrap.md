# RustyCore DB Bootstrap Runbook

> Canonical source: `/home/server/woltk-trinity-legacy/sql/`.
> Scope: WotLK Classic 3.4.3 RustyCore using the same four TrinityCore databases as C++:
> `auth`, `characters`, `world`, and `hotfixes`.

This runbook describes the operator path for preparing a MariaDB/MySQL instance
that RustyCore can boot against. It is intentionally conservative: C++ SQL
layout is the source of truth, and RustyCore must point at the same schemas,
ports, and realm metadata that a TrinityCore worldserver would use.

This is an operator runbook, not an instruction to execute every step during a
code/documentation task. Service stops, SQL imports/migrations and live smoke require
authorization for the named environment and data. Reuse an existing scoped approval;
do not infer destructive authority merely from "test environment". Take a recoverable
backup before mutation, and keep credentials/configuration out of Git and reports.

## Preconditions

- MariaDB 10.6+ or MySQL 8.x.
- `mysql` or `mariadb` CLI available for the operator-controlled base imports
  below. Runtime servers never invoke it.
- Canonical SQL tree present:
  - `/home/server/woltk-trinity-legacy/sql/create/create_mysql.sql`
  - `/home/server/woltk-trinity-legacy/sql/base/auth_database.sql`
  - `/home/server/woltk-trinity-legacy/sql/base/characters_database.sql`
  - `/home/server/woltk-trinity-legacy/sql/base/dev/world_database.sql`
  - `/home/server/woltk-trinity-legacy/sql/base/dev/hotfixes_database.sql`
  - `/home/server/woltk-trinity-legacy/sql/updates/{auth,characters,world,hotfixes}/wotlk_classic/`
- The out-of-tree TDB world content dump for this branch. The checked-in
  `sql/base/dev/world_database.sql` is DDL-only; it does not contain creature,
  quest, loot, or spawn content rows.

## 1. Stop Competing Servers

Before pointing RustyCore at the production-like DBs, stop any C++ worldserver
or bnetserver using the same ports or mutating the same DB rows.

Typical ports:

| Service | Port |
|---|---:|
| BNet RPC TLS | 1119 |
| Login REST | 8081 |
| World socket | 8085 |
| Instance socket | 8086 |

Use the deployment's process manager (`systemd`, `pm2`, shell session, etc.) to
stop the C++ services, then verify listeners are gone before starting Rust.

## 2. Create User and Databases

The canonical C++ SQL creates user `trinity` and four databases using
`utf8mb4 / utf8mb4_unicode_ci`. Its stock password is also `trinity` and its
grants include schema administration; these are bootstrap examples, not safe
credentials or least-privilege runtime grants. Review a private copy for the
intended host and separate migration/runtime identities before using it:

```bash
mysql -uroot -p < /home/server/woltk-trinity-legacy/sql/create/create_mysql.sql
```

If the DBs already exist, do not blindly re-run destructive drop/import steps on
a live realm. Take a SQL backup first and decide whether this is a fresh
bootstrap or an in-place update.

## 3. Import Base Schemas

For a fresh install, import the base dumps into the matching databases:

```bash
mysql -utrinity -p auth < /home/server/woltk-trinity-legacy/sql/base/auth_database.sql
mysql -utrinity -p characters < /home/server/woltk-trinity-legacy/sql/base/characters_database.sql
mysql -utrinity -p world < /home/server/woltk-trinity-legacy/sql/base/dev/world_database.sql
mysql -utrinity -p hotfixes < /home/server/woltk-trinity-legacy/sql/base/dev/hotfixes_database.sql
```

Important: `world_database.sql` only creates the schema. Import the matching TDB
world content dump after this step. Without it, the server may connect to DBs,
but gameplay content is effectively missing.

## 4. Apply and Validate RustyCore Migrations

Normal server startup has no schema-write authority. Use the explicit
administrative tool from the repository root after taking a backup:

```bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc
cargo run -p rustycore-db -- status --config worldserver.conf
cargo run -p rustycore-db -- migrate --dry-run --config worldserver.conf
cargo run -p rustycore-db -- migrate --config worldserver.conf
cargo run -p rustycore-db -- validate --config worldserver.conf
```

Add `--json` for stable automation output. Exit codes are `0` for compatible
or successful, `2` for invalid arguments, `3` for an incompatible/pending
schema, and `4` for an operational, lock, or migration failure. The command
contacts only the four configured MariaDB databases. It reads the local
`database/migrations/manifest.toml`; it neither scans `sql/old` nor downloads
artifacts. `status`, `validate` and `migrate --dry-run` still connect to the
configured databases; only the explicit non-dry-run `migrate` applies migrations.

The first `migrate` on an installation previously maintained by TrinityCore's
`updates` table imports exact filename plus normalized SHA-1 matches into the
new immutable SHA-256 history. For RustyCore migrations that predate this
manifest, source-controlled read-only schema fingerprints can prove the same
final columns and key invariants and import them without reapplying DDL. An
empty/different legacy hash or a partial fingerprint is not evidence and never
gets rehashed. This is the supported in-place transition from a correctly
imported `TDB343.24081` installation.

Do not mark the DB ready until `world.version` reports the current content
version:

```sql
SELECT db_version, cache_id FROM world.version LIMIT 1;
```

Expected for the current canonical `wotlk_classic` branch:

```text
TDB 343.24081 | 24081
```

RustyCore aborts world startup if this sentinel does not match.

## 5. Configure RustyCore

RustyCore reads TrinityCore-style semicolon DB strings:

```ini
LoginDatabaseInfo     = "127.0.0.1;3306;runtime_user;REPLACE_WITH_LOCAL_SECRET;auth"
WorldDatabaseInfo     = "127.0.0.1;3306;runtime_user;REPLACE_WITH_LOCAL_SECRET;world"
CharacterDatabaseInfo = "127.0.0.1;3306;runtime_user;REPLACE_WITH_LOCAL_SECRET;characters"
HotfixDatabaseInfo    = "127.0.0.1;3306;runtime_user;REPLACE_WITH_LOCAL_SECRET;hotfixes"

RealmID = 1
WorldServerPort = 8085
InstanceServerPort = 8086

```

`Updates.*` keys are no longer consumed by RustyCore daemons. Missing databases
must be created/imported explicitly; `world-server` and `bnet-server` connect,
perform bounded read-only compatibility queries, and refuse to open listeners
when migration state is pending, changed, missing, or incomplete.

For `bnet-server`, also configure TLS material with the same keys/certs used by
the C++ deployment:

```ini
CertificatesFile = "/path/to/bnetserver.cert.pem"
PrivateKeyFile = "/path/to/bnetserver.key.pem"
```

If the C++ install already has a known-good config pair, prefer copying that
pair into a temporary Rust runtime directory and overriding only the keys needed
for the smoke test.

## 6. Verify Realm Metadata

The active realm row in `auth.realmlist` must match the Rust worldserver port
and client build:

```sql
SELECT id, name, address, localAddress, port, gamebuild, flag
FROM auth.realmlist
WHERE id = 1;

SELECT build, win64AuthSeed
FROM auth.build_info
WHERE build = 51943;
```

`51943` is the historical client fixture used in this example, not an instruction
to overwrite a realm's current build. Match the actual approved client and realm;
the maintained bot wrapper currently defaults to `54261` unless overridden.
`win64AuthSeed` must be present for the selected build. Do not dump or commit session
keys or live secrets.

## 7. Start RustyCore

Build from the repository root with the pinned compiler/protoc. Start BNet first
and world second in separate terminals, or use the deployment's existing process
manager with explicitly selected configs; the first foreground server does not
return while serving. These commands do not install or restart managed services:

```bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo build --locked -p bnet-server -p world-server --release
# Terminal A:
./target/release/bnet-server --config /absolute/path/to/bnetserver.conf
# Terminal B:
./target/release/world-server --config /absolute/path/to/worldserver.conf
```

Startup evidence to expect:

- sanitized DB target logs for `login`, `character`, `world`, and `hotfix`;
- `Using World DB` with `TDB 343.24081` / `cache_id=24081`;
- `World server listening` on `8085`;
- `Instance server listening` on `8086`;
- realm `1` marked online.

## 8. Smoke Test

Login/realm/initial enter-world has already been proven against Rust in this
project, but every fresh DB/bootstrap should re-run a smoke before claiming it
is ready for manual client testing.

Preferred smoke harness (with valid credentials and existing disposable identities;
these overrides disable automatic password/account bootstrap, not normal login writes):

```bash
WOW_BOT_GENERATE_LOCAL_PASSWORD=0 WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
  tools/wow-test-bot/run_rustycore_login_smoke.sh
```

Minimum expected gate:

- BNet auth succeeds;
- world auth succeeds;
- character enum succeeds;
- player login reaches `SMSG_LOGIN_VERIFY_WORLD`;
- the world log shows the login sequence complete.

Passing this smoke does not prove gameplay/runtime parity. It only proves the
server can authenticate, enumerate characters, and enter the world with the
current DB/config pair.

## 9. Failure Checks

| Symptom | Check |
|---|---|
| Rust aborts with world DB version mismatch | `SELECT db_version, cache_id FROM world.version LIMIT 1`; import/apply the correct TDB/update set. |
| Realm appears disconnected | `auth.realmlist.port`, `flag`, `gamebuild`, and the actual `world-server` listener on `8085`. |
| BNet login fails before realm join | `auth.battlenet_accounts`, `auth.account`, `auth.build_info`, TLS cert/key paths, and bnet logs. |
| Character enum is empty | `characters.characters` rows for the account's linked game account and `auth.realmcharacters`. |
| World has no NPCs/quests | The TDB world content dump was not imported; base `world_database.sql` is DDL-only. |
| Startup asks for `rustycore-db migrate` | Run `status`, inspect the exact pending/checksum/incomplete result, back up, then run the explicit migration command. |
| Migration reports a held lock | Another administrator is migrating that database. Do not bypass the advisory lock; wait for it to finish. |
| Migration is incomplete | Stop. MariaDB DDL may have committed implicitly. Inspect the recorded script and live schema; restore the backup or repair deliberately before changing the history row. Automatic rollback/retry is intentionally forbidden. |

## Baseline Releases and Squashes

A baseline release is a maintainer operation, not an in-place upgrade:

1. Start from the pinned published TDB/content baseline and apply every active
   core migration with `rustycore-db migrate`.
2. Produce deterministic fresh-install dumps, record their checksums and bump
   the explicit baseline metadata in `database/migrations/manifest.toml`.
3. Mark migrations represented by that new dump `archived` in the manifest;
   keep their files and checksums immutable so existing installations still
   validate their applied history.
4. Add a new active migration for every later change. Never edit an already
   published migration and never require an existing realm to reimport the
   squash.

Fresh baseline acquisition/caching is owned by #255. `rustycore-db migrate`
uses only artifacts already present in the selected checkout/release.

## Current Gaps

- The canonical base SQL and large TDB content dump are not vendored into this
  repository yet (`#DBS.2` remains open).
- CI does not yet run a full clean-install against the canonical SQL files and
  world content (`#DBS.8` remains open).
- This runbook documents the operator path; it does not replace per-feature
  DB/runtime tests.
