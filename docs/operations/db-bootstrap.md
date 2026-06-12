# RustyCore DB Bootstrap Runbook

> Canonical source: `/home/server/woltk-trinity-legacy/sql/`.
> Scope: WotLK Classic 3.4.3 RustyCore using the same four TrinityCore databases as C++:
> `auth`, `characters`, `world`, and `hotfixes`.

This runbook describes the operator path for preparing a MariaDB/MySQL instance
that RustyCore can boot against. It is intentionally conservative: C++ SQL
layout is the source of truth, and RustyCore must point at the same schemas,
ports, and realm metadata that a TrinityCore worldserver would use.

## Preconditions

- MariaDB 10.6+ or MySQL 8.x.
- `mysql` CLI available in `PATH`; RustyCore's updater uses the CLI for
  multi-statement dumps, matching TrinityCore's operational model.
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
`utf8mb4 / utf8mb4_unicode_ci`:

```bash
mysql -uroot -p < /home/server/woltk-trinity-legacy/sql/create/create_mysql.sql
```

If the DBs already exist, do not blindly re-run destructive drop/import steps on
a live realm. Take a SQL backup first and decide whether this is a fresh
bootstrap or an in-place update.

## 3. Import Base Schemas

For a fresh install, import the base dumps into the matching databases:

```bash
mysql -utrinity -ptrinity auth < /home/server/woltk-trinity-legacy/sql/base/auth_database.sql
mysql -utrinity -ptrinity characters < /home/server/woltk-trinity-legacy/sql/base/characters_database.sql
mysql -utrinity -ptrinity world < /home/server/woltk-trinity-legacy/sql/base/dev/world_database.sql
mysql -utrinity -ptrinity hotfixes < /home/server/woltk-trinity-legacy/sql/base/dev/hotfixes_database.sql
```

Important: `world_database.sql` only creates the schema. Import the matching TDB
world content dump after this step. Without it, the server may connect to DBs,
but gameplay content is effectively missing.

## 4. Apply Updates

RustyCore can run the updater during startup when `Updates.EnableDatabases` is
non-zero. The bitmask matches TrinityCore:

| Bit | DB |
|---:|---|
| 1 | login/auth |
| 2 | characters |
| 4 | world |
| 8 | hotfixes |
| 15 | all |

For a controlled bootstrap, either:

- let RustyCore apply updates by setting `Updates.SourcePath` to
  `/home/server/woltk-trinity-legacy` and `Updates.EnableDatabases = 15`, or
- apply the canonical SQL updates manually using the same lexicographic order
  as TrinityCore's updater.

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
LoginDatabaseInfo     = "127.0.0.1;3306;trinity;trinity;auth"
WorldDatabaseInfo     = "127.0.0.1;3306;trinity;trinity;world"
CharacterDatabaseInfo = "127.0.0.1;3306;trinity;trinity;characters"
HotfixDatabaseInfo    = "127.0.0.1;3306;trinity;trinity;hotfixes"

RealmID = 1
WorldServerPort = 8085
InstanceServerPort = 8086

Updates.SourcePath = "/home/server/woltk-trinity-legacy"
Updates.EnableDatabases = 15
Updates.AutoSetup = 1
```

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

For the tested WotLK Classic client path, `gamebuild` is `51943`, and
`win64AuthSeed` must be present for that build. Do not dump or commit session
keys or live secrets.

## 7. Start RustyCore

Recommended order:

```bash
cargo build -p bnet-server -p world-server --release
./target/release/bnet-server
./target/release/world-server
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

Preferred smoke harness:

```bash
/home/cdmonio/projects/wow-test-bot/rust-bot/run_rustycore_login_smoke.sh
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

## Current Gaps

- The canonical base SQL and large TDB content dump are not vendored into this
  repository yet (`#DBS.2` remains open).
- CI does not yet run a full clean-install against the canonical SQL files and
  world content (`#DBS.8` remains open).
- This runbook documents the operator path; it does not replace per-feature
  DB/runtime tests.
