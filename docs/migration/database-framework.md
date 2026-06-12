# Migration: database-framework (MySQL connection pool, prepared statements, transactions, updater)

> **C++ canonical path:** `src/server/database/Database/` + `src/server/database/Updater/`
> **Rust target crate(s):** `crates/wow-database/`
> **Layer:** L1 infrastructure (under shared/datastores)
> **Status:** ⚠️ partial (4 typed pools + prepared-statement enums + transactions + QueryHolder + DB updater work; missing: per-pool sync/async split, callback chaining type, libmysql `Library_Init`)
> **Audited vs C++:** ✅ complete (2026-05-01)
> **Last updated:** 2026-05-01

---

## 1. Purpose

The TrinityCore database framework is an N-thread async MySQL connection pool with typed prepared statements, transactional batches, future-based query callbacks, and an auto-applier that runs pending `.sql` migration files at boot. Game code never touches `libmysqlclient` directly: it asks `LoginDatabase`/`CharacterDatabase`/`WorldDatabase`/`HotfixDatabase` for a typed `PreparedStatement<T>`, sets parameters by index, and either blocks on the result, gets a `QueryCallback` future, or appends to a `Transaction<T>`. The `DBUpdater<T>` template ships pending schema deltas from `sql/updates/<type>/` automatically so the binary can boot against a fresh or stale MariaDB instance.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `database/Database/AdhocStatement.cpp` | 38 | `prefix` |
| `database/Database/AdhocStatement.h` | 34 | `prefix` |
| `database/Database/DatabaseEnv.cpp` | 23 | `prefix` |
| `database/Database/DatabaseEnv.h` | 44 | `prefix` |
| `database/Database/DatabaseEnvFwd.h` | 98 | `prefix` |
| `database/Database/DatabaseLoader.cpp` | 189 | `prefix` |
| `database/Database/DatabaseLoader.h` | 78 | `prefix` |
| `database/Database/DatabaseWorkerPool.cpp` | 585 | `prefix` |
| `database/Database/DatabaseWorkerPool.h` | 243 | `prefix` |
| `database/Database/Field.cpp` | 169 | `prefix` |
| `database/Database/Field.h` | 142 | `prefix` |
| `database/Database/FieldValueConverter.cpp` | 48 | `prefix` |
| `database/Database/FieldValueConverter.h` | 50 | `prefix` |
| `database/Database/FieldValueConverters.h` | 112 | `prefix` |
| `database/Database/Implementation/CharacterDatabase.cpp` | 732 | `prefix` |
| `database/Database/Implementation/CharacterDatabase.h` | 611 | `prefix` |
| `database/Database/Implementation/HotfixDatabase.cpp` | 1916 | `prefix` |
| `database/Database/Implementation/HotfixDatabase.h` | 1122 | `prefix` |
| `database/Database/Implementation/LoginDatabase.cpp` | 204 | `prefix` |
| `database/Database/Implementation/LoginDatabase.h` | 195 | `prefix` |
| `database/Database/Implementation/WorldDatabase.cpp` | 90 | `prefix` |
| `database/Database/Implementation/WorldDatabase.h` | 104 | `prefix` |
| `database/Database/MySQLConnection.cpp` | 605 | `prefix` |
| `database/Database/MySQLConnection.h` | 116 | `prefix` |
| `database/Database/MySQLHacks.h` | 34 | `prefix` |
| `database/Database/MySQLPreparedStatement.cpp` | 201 | `prefix` |
| `database/Database/MySQLPreparedStatement.h` | 71 | `prefix` |
| `database/Database/MySQLThreading.cpp` | 34 | `prefix` |
| `database/Database/MySQLThreading.h` | 30 | `prefix` |
| `database/Database/MySQLWorkaround.h` | 26 | `prefix` |
| `database/Database/PreparedStatement.cpp` | 182 | `prefix` |
| `database/Database/PreparedStatement.h` | 125 | `prefix` |
| `database/Database/QueryCallback.cpp` | 221 | `prefix` |
| `database/Database/QueryCallback.h` | 67 | `prefix` |
| `database/Database/QueryHolder.cpp` | 94 | `prefix` |
| `database/Database/QueryHolder.h` | 81 | `prefix` |
| `database/Database/QueryResult.cpp` | 457 | `prefix` |
| `database/Database/QueryResult.h` | 85 | `prefix` |
| `database/Database/Transaction.cpp` | 107 | `prefix` |
| `database/Database/Transaction.h` | 119 | `prefix` |
| `database/Updater/DBUpdater.cpp` | 445 | `prefix` |
| `database/Updater/DBUpdater.h` | 96 | `prefix` |
| `database/Updater/UpdateFetcher.cpp` | 424 | `prefix` |
| `database/Updater/UpdateFetcher.h` | 143 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines (approx) | Purpose |
|---|---|---|
| `src/server/database/Database/MySQLConnection.h` | 116 | `MySQLConnection` + `MySQLConnectionInfo` + `ConnectionFlags` (ASYNC/SYNCH/BOTH); single libmysql handle wrapper |
| `src/server/database/Database/MySQLConnection.cpp` | 605 | `Open`, `Close`, `Execute`, `Query`, `_HandleMySQLErrno` (5-attempt reconnect), `Ping`, `ExecuteTransaction`, `StartWorkerThread` |
| `src/server/database/Database/DatabaseWorkerPool.h` | 243 | `DatabaseWorkerPool<T>` template — N async + M sync `T*` connections, `Execute`/`DirectExecute`/`AsyncQuery`/`Query`/`BeginTransaction`/`CommitTransaction`/`KeepAlive`/`WarnAboutSyncQueries`/`QueueSize` |
| `src/server/database/Database/DatabaseWorkerPool.cpp` | 585 | Pool implementation; round-robin worker dispatch via `Trinity::Asio::IoContext` per pool |
| `src/server/database/Database/PreparedStatement.h` | 125 | `PreparedStatementBase` (typed `setUInt8`…`setBinary`), `PreparedStatement<T>` (carries stmt-enum type), `PreparedStatementData` (`std::variant<…>`) |
| `src/server/database/Database/PreparedStatement.cpp` | 182 | Setter implementations; `PreparedStatementTask::Query`/`Execute` (run on a `MySQLConnection`) |
| `src/server/database/Database/Field.h` | 142 | `Field` (`GetUInt8`…`GetBinary`/`GetCString`/`GetString`), `DatabaseFieldTypes` enum, `QueryResultFieldMetadata` |
| `src/server/database/Database/Field.cpp` | 169 | Per-getter MySQL-type conversion + size-checked binary, `Converter` hook for value-converters |
| `src/server/database/Database/QueryCallback.h` | 67 | `QueryCallback` — holds a `QueryResultFuture` xor `PreparedQueryResultFuture`, supports `WithCallback`, `WithChainingCallback` (chain returns a new query), `InvokeIfReady` |
| `src/server/database/Database/QueryCallback.cpp` | 221 | Manual union management for the future-variant; queue of `QueryCallbackData` for chaining |
| `src/server/database/Database/QueryResult.h` | 85 | `ResultSet` (text protocol) + `PreparedResultSet` (binary protocol); both expose `Field*` rows + `NextRow` cursor |
| `src/server/database/Database/QueryResult.cpp` | 457 | libmysql `mysql_fetch_row` / `mysql_stmt_fetch` + per-column metadata copy |
| `src/server/database/Database/QueryHolder.h` | 81 | `SQLQueryHolder<T>` — fixed-size vector of `(stmt, result)` pairs; `SQLQueryHolderCallback` to fire one callback after **all** are ready (used for character-load fan-out) |
| `src/server/database/Database/QueryHolder.cpp` | 94 | Holder implementation |
| `src/server/database/Database/Transaction.h` | 119 | `TransactionBase`/`Transaction<T>` (collects queries), `TransactionTask` (deadlock retry loop capped by `DEADLOCK_MAX_RETRY_TIME_MS` + `_deadlockLock` static mutex), `TransactionCallback` (future + `AfterComplete`) |
| `src/server/database/Database/Transaction.cpp` | 107 | `Append`, `AppendPreparedStatement`, `Cleanup`, `TryExecute` retry loop |
| `src/server/database/Database/AdhocStatement.h` / `.cpp` | 34 + 38 | `BasicStatementTask` — runs a non-prepared SQL string on a worker |
| `src/server/database/Database/MySQLPreparedStatement.h` / `.cpp` | 71 + 201 | Per-connection libmysql `MYSQL_STMT*` wrapper; binds `MYSQL_BIND[]` from `PreparedStatementData` |
| `src/server/database/Database/MySQLThreading.h` / `.cpp` | 30 + 34 | `mysql_library_init` / `mysql_library_end` thread-safe wrappers |
| `src/server/database/Database/MySQLHacks.h` / `MySQLWorkaround.h` | 34 + 26 | Forward-decl shims for `MYSQL`, `MYSQL_RES`, `MYSQL_FIELD` to avoid leaking `<mysql.h>` everywhere |
| `src/server/database/Database/DatabaseEnv.h` / `.cpp` / `DatabaseEnvFwd.h` | 44 + 23 + 98 | Forward decls + typedefs (`QueryResult`, `PreparedQueryResult`, `SQLTransaction<T>`, futures) |
| `src/server/database/Database/DatabaseLoader.h` / `.cpp` | 78 + 189 | `DatabaseLoader::AddDatabase(pool, name)` registers per-DB Open/Populate/Update/Prepare/Close in 5 work queues; `Load()` runs them in order, rolling back via the close stack on failure |
| `src/server/database/Database/Implementation/LoginDatabase.h` / `.cpp` | 195 + 204 | `LoginDatabaseStatements` enum (137 values) + `LoginDatabaseConnection::DoPrepareStatements()` |
| `src/server/database/Database/Implementation/CharacterDatabase.h` / `.cpp` | 611 + 732 | `CharacterDatabaseStatements` enum (523 values) + prepare bodies |
| `src/server/database/Database/Implementation/WorldDatabase.h` / `.cpp` | 104 + 90 | `WorldDatabaseStatements` enum (56 values) + prepare bodies |
| `src/server/database/Database/Implementation/HotfixDatabase.h` / `.cpp` | 1122 + 1916 | `HotfixDatabaseStatements` enum (327 values) + prepare bodies |
| `src/server/database/Database/FieldValueConverter.{h,cpp}` / `FieldValueConverters.h` | 50 + 48 + 112 | Pluggable column converters (e.g. enum string → int) |
| `src/server/database/Updater/DBUpdater.h` / `.cpp` | 96 + 445 | `DBUpdater<T>::Create`/`Populate`/`Update` static methods; per-DB `GetConfigEntry`/`GetTableName`/`GetBaseFile` template specializations |
| `src/server/database/Updater/UpdateFetcher.h` / `.cpp` | 143 + 424 | Walks `sql/updates/<type>/`, hashes files (SHA1), compares against `updates` + `updates_include` tables, computes RELEASED/ARCHIVED state, archives obsolete |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `MySQLConnectionInfo` | struct | Parsed `host;port_or_socket;user;password;database;ssl` connection string |
| `ConnectionFlags` | enum bitmask | `CONNECTION_ASYNC=0x1`, `CONNECTION_SYNCH=0x2`, `CONNECTION_BOTH=0x3` — selects which sub-pool a prepared statement is built into |
| `MySQLConnection` | abstract class | One libmysql handle + worker thread + reconnect logic; `DoPrepareStatements()` pure virtual |
| `LoginDatabaseConnection` / `CharacterDatabaseConnection` / `WorldDatabaseConnection` / `HotfixDatabaseConnection` | concrete subclass | One per logical DB; supplies `DoPrepareStatements()` body |
| `DatabaseWorkerPool<T>` | template class | N-async + M-sync `T` connections; `T::Statements` typedef carries the stmt-enum |
| `LoginDatabaseStatements` / `CharacterDatabaseStatements` / `WorldDatabaseStatements` / `HotfixDatabaseStatements` | enum `: uint32` | One numeric ID per prepared statement; terminated by `MAX_*` sentinel for vector sizing |
| `PreparedStatementData` | struct (variant) | Single bind parameter; `std::variant<bool,uint8…int64,float,double,string,vector<uint8>,nullptr_t>` |
| `PreparedStatementBase` | class | `setUInt8`…`setBinary` API; holds `m_index` + `vector<PreparedStatementData> statement_data` |
| `PreparedStatement<T>` | template class | Type tag carrying which DB this stmt belongs to (compile-time check) |
| `PreparedStatementTask` | class | Lower-level: runs one stmt on a connection, fills `PreparedQueryResult` |
| `Field` | class | One column of one row; `GetUInt8…GetBinary`, `IsNull`, holds `_value`, `_length`, `_meta` |
| `DatabaseFieldTypes` | enum | `Null/UInt8/Int8/UInt16/Int16/UInt32/Int32/UInt64/Int64/Float/Double/Decimal/Date/Binary` |
| `QueryResultFieldMetadata` | struct | `TableName`/`Alias`/`Name`/`TypeName`/`Index`/`Type`/`Converter*` |
| `ResultSet` | class | Text-protocol result iterator — returned by `Query(char const* sql)` |
| `PreparedResultSet` | class | Binary-protocol result iterator — returned by `Query(PreparedStatement<T>*)` |
| `QueryCallback` | class | Future + chain queue. `WithCallback` (terminal), `WithChainingCallback` (next callback returns a `QueryCallback` and the chain continues). `InvokeIfReady()` polled from world tick. |
| `SQLQueryHolderBase` / `SQLQueryHolder<T>` | class / template | Fixed-size vector of (stmt, result) slots. Used by character-load: queue one async DB task, execute slots in order, get one callback when all done |
| `SQLQueryHolderCallback` | class | The "all done" future for a holder |
| `TransactionBase` / `Transaction<T>` | class / template | Collects raw SQL strings + `unique_ptr<PreparedStatementBase>` ; `Append`/`PAppend` |
| `TransactionData` | struct | `std::variant<unique_ptr<PreparedStatementBase>, std::string>` (one row of the transaction) |
| `TransactionTask` | class | Executes a transaction; if the first `TryExecute` deadlocks, retries under static `_deadlockLock` until the 60s retry window expires |
| `TransactionCallback` | class | Future for an async transaction; `AfterComplete(std::function<void(bool)>)` |
| `DatabaseLoader` | class | Five work queues (`_open`, `_populate`, `_update`, `_prepare`) + `_close` rollback stack; `Load()` runs them; `DATABASE_LOGIN/CHARACTER/WORLD/HOTFIX` flags |
| `DBUpdater<T>` | template class | `Create`, `Populate`, `Update`, `Apply`, `ApplyFile`; per-T template specs map `GetConfigEntry()` / `GetTableName()` / `GetBaseFile()` |
| `DBUpdaterUtil` | class | Locates a `mysql` CLI binary on PATH for `ApplyFile` of large dump files |
| `BaseLocation` | enum | `LOCATION_REPOSITORY` (sql/ in repo) vs `LOCATION_DOWNLOAD` (TDB tarball) |
| `UpdateException` | class | Thrown on update failure |
| `UpdateFetcher` | class (in Updater/) | Walks `sql/updates/<type>/`, hashes, sorts, computes RELEASED/ARCHIVED state |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `MySQLConnection::Open()` | `mysql_init` + `mysql_real_connect`; sets `MYSQL_OPT_RECONNECT=0` (we manage reconnect ourselves) | libmysql, `_HandleMySQLErrno` |
| `MySQLConnection::Execute(stmt)` | `mysql_stmt_bind_param` + `mysql_stmt_execute` | `MySQLPreparedStatement::BindParameters` |
| `MySQLConnection::Query(stmt)` | Execute + `mysql_stmt_store_result` → `PreparedResultSet` | as above |
| `MySQLConnection::ExecuteTransaction(trans)` | `START TRANSACTION` … per-stmt execute … `COMMIT` (or `ROLLBACK` on error) | `TransactionTask::TryExecute` |
| `MySQLConnection::Ping()` | `mysql_ping`; reconnects on failure | `_HandleMySQLErrno` |
| `MySQLConnection::PrepareStatement(idx, sql, flags)` | `mysql_stmt_prepare` if `flags & m_connectionFlags`; stores in `m_stmts[idx]` | libmysql |
| `DatabaseWorkerPool<T>::SetConnectionInfo(s, asyncN, syncN)` | Save info, sub-pool sizes | — |
| `DatabaseWorkerPool<T>::Open()` | Open `asyncN` + `syncN` `T` connections, prepare statements on each | `MySQLConnection::Open`, `PrepareStatements` |
| `DatabaseWorkerPool<T>::Execute(sql)` / `Execute(stmt)` | Enqueue one-way op on async sub-pool (fire-and-forget) | `BasicStatementTask` / `PreparedStatementTask` |
| `DatabaseWorkerPool<T>::DirectExecute(sql)` | Synchronous one-way op on a free sync connection | `GetFreeConnection`, `Unlock` |
| `DatabaseWorkerPool<T>::Query(sql, conn=null)` | Synchronous query → `QueryResult` | `GetFreeConnection` |
| `DatabaseWorkerPool<T>::Query(stmt)` | Synchronous prepared query → `PreparedQueryResult` | `GetFreeConnection` |
| `DatabaseWorkerPool<T>::AsyncQuery(stmt)` | Enqueue prepared query on async sub-pool, return `QueryCallback` | `PreparedStatementTask` |
| `DatabaseWorkerPool<T>::DelayQueryHolder(holder)` | Enqueue a multi-stmt holder task, return `SQLQueryHolderCallback` | `SQLQueryHolderTask` |
| `DatabaseWorkerPool<T>::BeginTransaction()` | `make_shared<Transaction<T>>()` | — |
| `DatabaseWorkerPool<T>::CommitTransaction(trans)` | Enqueue async commit | `TransactionTask` |
| `DatabaseWorkerPool<T>::AsyncCommitTransaction(trans)` | Enqueue async commit, return `TransactionCallback` | as above |
| `DatabaseWorkerPool<T>::DirectCommitTransaction(trans)` | Synchronous commit on a free sync connection | as above |
| `DatabaseWorkerPool<T>::ExecuteOrAppend(trans, stmt)` | If `trans` is non-null, `trans->Append(stmt)`; else `Execute(stmt)` | — |
| `DatabaseWorkerPool<T>::GetPreparedStatement(index)` | `new PreparedStatement<T>(index, _preparedStatementSize[index])` | — |
| `DatabaseWorkerPool<T>::EscapeString(str)` | `mysql_real_escape_string` via a sync connection | libmysql |
| `DatabaseWorkerPool<T>::KeepAlive()` | For each connection: `Ping` (called from `World::Update` every `MaxPingTime`) | `MySQLConnection::Ping` |
| `DatabaseWorkerPool<T>::WarnAboutSyncQueries(bool)` | Debug-only thread-local flag; warn when a sync query runs from a thread that should be async-only | — |
| `DatabaseWorkerPool<T>::QueueSize()` | Reports pending async ops (for metrics) | — |
| `PreparedStatementBase::setUInt8(idx, val)` (and 14 siblings) | Push `PreparedStatementData{ val }` at `statement_data[idx]` | — |
| `Field::GetUInt8()` (and 13 siblings) | `strtoul(_value, …)` with type-check vs `_meta->Type` | — |
| `Field::GetBinary()` | `vector<uint8>(_value, _value+_length)` | — |
| `QueryCallback::WithCallback(fn)` | Push terminal callback onto `_callbacks` queue | — |
| `QueryCallback::WithChainingCallback(fn)` | Push chaining callback; on invoke `fn(*this, result)` may `SetNextQuery(...)` | — |
| `QueryCallback::InvokeIfReady()` | If future ready: pop callback, run; if a chained `SetNextQuery` happened, swap future and recurse | — |
| `SQLQueryHolderCallback::AfterComplete(fn)` | Set the "all queries done" callback | — |
| `Transaction<T>::Append(sql)` / `Append(stmt)` | Push a `TransactionData` | — |
| `TransactionTask::Execute(conn, trans)` | On deadlock, wrap retries in mutex `_deadlockLock` and call `TryExecute` until `DEADLOCK_MAX_RETRY_TIME_MS` expires | `MySQLConnection::ExecuteTransaction` |
| `DatabaseLoader::AddDatabase(pool, name)` | Lazy-register: push closures into `_open`, `_populate`, `_update`, `_prepare` queues; push close-fn onto `_close` stack | — |
| `DatabaseLoader::Load()` | Run all four queues in order; on any failure: pop `_close` stack and return `false` | — |
| `DBUpdater<T>::Create(pool)` | If DB doesn't exist: `mysql -e "CREATE DATABASE …"` via CLI | `DBUpdaterUtil::GetCorrectedMySQLExecutable` |
| `DBUpdater<T>::Populate(pool)` | If DB has 0 tables: `ApplyFile(GetBaseFile())` via CLI | — |
| `DBUpdater<T>::Update(pool)` | `UpdateFetcher::Update` — hash all `.sql` under configured paths, apply pending, mark in `updates` table | `UpdateFetcher` |
| `UpdateFetcher::Update(...)` | Walk dirs in `updates_include`, sha1 each file, diff against `updates`, apply via `pool.DirectExecute` | `pool.DirectExecute` |

---

## 5. Module dependencies

**Depends on:**
- `src/common/Logging` — `TC_LOG_INFO`, `TC_LOG_ERROR`, `TC_LOG_FATAL` (`sql.driver`, `sql.updates`)
- `src/common/Configuration/Config.h` — `sConfigMgr->GetStringDefault`/`GetIntDefault` for `<DB>DatabaseInfo` strings, `Updates.*` keys
- `src/common/Asio/IoContext.h` — each pool has its own `Trinity::Asio::IoContext` to drive worker threads
- `src/common/Threading/MPSCQueue.h` (used inside the io_context dispatch)
- `src/common/Utilities/StringFormat.h` — `Trinity::StringFormat` for `PExecute`/`PQuery`
- `src/common/Utilities/StartProcess.h` — for invoking the `mysql` CLI from `DBUpdater`
- `boost::filesystem` — file walking in `UpdateFetcher`
- `mysql` (libmysqlclient or MariaDB Connector/C) — the actual driver
- `OpenSSL` — only via `mysql_options(MYSQL_OPT_SSL_CA, …)`
- `zlib` (transitive via libmysql `CLIENT_COMPRESS`)

**Depended on by:**
- **All four typed pool consumers**: `worldserver/Main.cpp::StartDB`, `bnetserver/Main.cpp::StartDB`, `authserver/Main.cpp` (legacy)
- `src/server/game/**/*` — every gameplay module that touches persistence (chars, achievements, mails, guilds, BG state, …)
- `src/server/shared/Realm/RealmList.cpp` — login DB realm refresh
- `src/server/shared/Networking/AsyncCallbackProcessor.h` — `WorldSession::_queryProcessor.AddCallback(...)` etc.
- `src/server/scripts/**` — script DB queries
- `src/server/game/Cache/CharacterCache.cpp` — async character load via `SQLQueryHolder`

---

## 6. SQL / DB queries (if any)

The framework itself emits very few queries (only schema bookkeeping). The bulk of queries are in `Implementation/<Db>Database.cpp`, registered as prepared statements indexed by enum value.

### Updater bookkeeping (DBUpdater)

| Statement / Source | Purpose | DB |
|---|---|---|
| `CREATE DATABASE IF NOT EXISTS \`x\` DEFAULT CHARACTER SET utf8mb4 …` | `DBUpdater::Create` (via `mysql` CLI when DB missing) | any |
| `INSERT INTO \`updates\` (\`name\`, \`hash\`, \`state\`, \`speed\`) VALUES (?, ?, ?, ?)` | Record an applied update (TC variant uses `INSERT ... ON DUPLICATE KEY UPDATE`) | any |
| `UPDATE \`updates\` SET \`hash\` = ?, \`speed\` = ? WHERE \`name\` = ?` | Re-apply an update whose hash changed | any |
| `UPDATE \`updates\` SET \`name\` = ? WHERE \`name\` = ?` | Detect rename (same hash, different filename) | any |
| `DELETE FROM \`updates\` WHERE \`name\` = ?` | Mark obsolete update as unapplied | any |
| `SELECT \`name\`, \`hash\`, \`state\` FROM \`updates\` ORDER BY \`name\`` | List applied updates | any |
| `SELECT \`path\`, \`state\` FROM \`updates_include\` ORDER BY \`path\`` | List directories to scan for updates | any |
| `SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE()` | Detect "DB is empty → populate" | any (Rust port; TC uses `mysql_query("SHOW TABLES")` and counts rows) |

### Loader bookkeeping (DatabaseLoader)

| Statement / Source | Purpose | DB |
|---|---|---|
| `SELECT * FROM \`version\` LIMIT 1` (TC: `World::LoadDBVersion`) | Read DB schema version banner | world |

### Counts of registered prepared statements (TC `MAX_*` sentinels)

| DB | TC stmts | Rust enum variants |
|---|---|---|
| login (auth) | 137 | 137 (`crates/wow-database/src/statements/login.rs`) |
| character | 523 | 30 (10% of TC's set; covers char enum/create/login/save/delete/inventory subset) |
| world | 56 | 86 (Rust adds some lookups not in TC; e.g. quest loaders) |
| hotfixes | 325 base + 325 max-id + 95 locale generated families; plus 3 direct `DB2Manager::LoadHotfix*` control queries | 15 named statements (3 control tables + 12 selected DB2 overlays) plus generated base/max-id/locale helpers preserving exact C++ SQL for future store ports |

---

## 7. Wire-protocol packets (if any)

Not applicable — the database framework does not handle WoW client packets. (Indirectly, every gameplay packet that mutates persisted state will go through this layer, but no opcode is owned by it.)

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-database` | `crate_dir` | 12 | 2262 | `exists_active` | crate exists |
| `crates/wow-database/src/lib.rs` | `file` | 1 | 58 | `exists_active` | file exists |
| `crates/wow-database/src/database.rs` | `file` | 1 | 178 | `exists_active` | file exists |
| `crates/wow-database/src/error.rs` | `file` | 1 | 21 | `exists_active` | file exists |
| `crates/wow-database/src/params.rs` | `file` | 1 | 208 | `exists_active` | file exists |
| `crates/wow-database/src/result.rs` | `file` | 1 | 198 | `exists_active` | file exists |
| `crates/wow-database/src/transaction.rs` | `file` | 1 | 108 | `exists_active` | file exists |
| `crates/wow-database/src/updater.rs` | `file` | 1 | 391 | `exists_active` | file exists |
| `crates/wow-database/src/statements/mod.rs` | `file` | 1 | 93 | `exists_active` | file exists |
| `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `crates/wow-database/src/statements/character.rs` | `file` | 1 | 1826 | `exists_active` | file exists |
| `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | current | `exists_active` | named control/overlay statements plus generated C++ hotfix helpers |
| `crates/world-server/src/main.rs` | `file` | 1 | current | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_cache.rs` | `file` | 1 | current | `exists_active` | `HotfixBlobCache` + DB control-table loaders |
| `crates/wow-database/src` | `module_dir` | 12 | 2262 | `exists_active` | directory exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-database/src/lib.rs` — 58 lines — public re-exports + four `LoginDatabase`/`WorldDatabase`/`CharacterDatabase`/`HotfixDatabase` type aliases
- `crates/wow-database/src/database.rs` — 178 lines — `Database<S: StatementDef>` wrapper around `sqlx::MySqlPool`
- `crates/wow-database/src/error.rs` — `DatabaseError` enum (`Connection`/`Query`/`TableMissing`/`UnregisteredStatement`) with MySQL `ER_NO_SUCH_TABLE` classification.
- `crates/wow-database/src/params.rs` — 208 lines — `PreparedStatement` (SQL + `Vec<SqlParam>`) + `SqlParam` (15 variants matching TC's `PreparedStatementData`)
- `crates/wow-database/src/result.rs` — 198 lines — `SqlResult` (cursor over `Vec<MySqlRow>`) + `SqlFields` (single-row borrowed view)
- `crates/wow-database/src/transaction.rs` — `SqlTransaction` (collects + commits with TC-style serialized deadlock retry) + private `bind_param` helper
- `crates/wow-database/src/updater.rs` — 391 lines — `DbUpdater::populate(base_sql)` + `update(source_dir)` + SHA1 hashing + statement splitter that handles line/block comments + `'` / `"` strings with escapes
- `crates/wow-database/src/statements/mod.rs` — `StatementDef` trait + unit tests
- `crates/wow-database/src/statements/login.rs` — 327 lines — `LoginStatements` enum (137 variants) + `sql()` impl
- `crates/wow-database/src/statements/character.rs` — 6207 lines — `CharStatements` enum (560 named variants, including compatibility aliases + generated C++ SQL parity harness — active WotLK registry closed by C++ name except future-scope BlackMarket)
- `crates/wow-database/src/statements/world.rs` — 371 lines — `WorldStatements` enum (86 variants)
- `crates/wow-database/src/statements/hotfix.rs` — `HotfixStatements` strategy marker, 3 control-table statements, selected DB2 overlay statements, and generated base/max-id/locale helpers validated against C++ `HotfixDatabase.cpp`
- `crates/wow-data/src/hotfix_cache.rs` — `HotfixBlobCache`, `hotfix_data` / `hotfix_blob` / `hotfix_optional_data` loading, locale masks and DBReply blob lookup

**What's implemented:**
- Four logical pools, **each a single `sqlx::Pool<MySql>` sized by TC's `<Db>Database.WorkerThreads + <Db>Database.SynchThreads` keys in `world-server`**. Reachable via `LoginDatabase = Database<LoginStatements>`, `WorldDatabase = Database<WorldStatements>`, `CharacterDatabase = Database<CharStatements>`, `HotfixDatabase = Database<HotfixStatements>` aliases.
- Compile-time statement-to-DB binding via the `StatementDef` trait + `PhantomData<S>` on `Database<S>` — using a `WorldStatements` variant on `Database<LoginStatements>` is rejected by `rustc`. Equivalent to TC's `typename T::Statements` typedef but enforced more strongly (TC only enforces it for `GetPreparedStatement`).
- Full set of typed setters: `set_bool/set_i8/set_u8/set_i16/set_u16/set_i32/set_u32/set_i64/set_u64/set_f32/set_f64/set_string/set_bytes/set_null` matching TC's `setUInt8`…`setBinary` 1:1.
- Async query API: `db.query(&stmt).await` → `SqlResult` (cursor with `next_row()`, `read::<T>(col)`, `try_read`, `read_typed::<T>(col)` / `try_read_typed`, `is_null`, `read_string` fallback for `utf8mb4_bin` columns); `db.execute(&stmt).await` for non-result statements; `db.direct_execute(sql)` / `db.direct_query(sql)` for raw SQL.
- Transactions: `SqlTransaction { statements: Vec<PreparedStatement> }` + `commit(&pool)` opens a real `sqlx::Transaction`, executes each statement, commits or rolls back. On MySQL error 1213 (deadlock), retries are serialized under a process-wide mutex for up to 60 seconds — same guardrail as TC's `TransactionTask::_deadlockLock` + `DEADLOCK_MAX_RETRY_TIME_MS`.
- `execute_or_append(trans, stmt)` mirroring TC's `ExecuteOrAppend`.
- DB Updater (`DbUpdater`): `populate(base_sql)` invokes the `mysql` CLI to apply a base dump if `information_schema.tables` reports 0 tables for the current DB; the CLI command mirrors TC's transactional `BEGIN; SOURCE file; COMMIT;` wrapper and preserves the CLI stderr in the returned error. `update(source_dir)` reads from `updates_include` table, walks `$source_dir/sql/updates/...`, sha1-hashes each `.sql`, applies via sqlx (statement-by-statement using a hand-rolled splitter that respects `--`/`#` line comments, `/* */` block comments, and `'` / `"` strings with escapes). Tracks applied files in `updates` table with hash + applied-at + apply-time-ms. Detects renames (same hash, different filename) and reapplies updates whose hash changed. State handling now mirrors this WotLK C++ branch's two-state `StateConvert`: exact `RELEASED` stays released; any other value is treated as `ARCHIVED`. If `updates_include` is empty for a known TC database family, RustyCore bootstraps the same path set used by the WotLK Classic base dumps (`custom`, `old/10.x`, `old/3.4.x`, `old/6.x`, `old/7`, `old/8.x`, `old/9.x`, `updates`).
- `Updates.AutoSetup` config gate (default `1`) — controls only TC-style missing-database auto-create; `Populate` / `Update` are governed by `Updates.EnableDatabases`.
- All four DBs are wired in `crates/world-server/src/main.rs` lines 170-272: open pool, `DbUpdater::new(pool, …).populate(...).update(...)` for auth + characters; `update(...)` only for world + hotfix (because base SQL for those is the TDB tarball, not in repo).
- Unit tests in `params.rs`, `statements/mod.rs`, and `updater.rs` cover setter behaviour, sparse indices, statement-table-name presence, `'?'` placeholder counts, updater redundancy gates, orphan cleanup threshold, and `updates_include` default path selection.

**What's missing vs C++:**
- **`ConnectionFlags::ASYNC` vs `SYNCH` separation**: TC opens **two sub-pools per logical DB** (configured via `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads`, e.g. `LoginDatabase.WorkerThreads=1`, `LoginDatabase.SynchThreads=1`). RustyCore's single `sqlx::Pool` blends sync and async traffic, but `world-server` now preserves TC's configured total connection budget by opening the pool with `WorkerThreads + SynchThreads`. (Cross-ref: `worldserver.md` §13.5 sub-task #WS.21 / #DB.1.)
- **`KeepAlive()` ping internals**: TC's `World::Update` calls `CharacterDatabase.KeepAlive()`, `LoginDatabase.KeepAlive()`, and `WorldDatabase.KeepAlive()` every `MaxPingTime` minutes (default 30). Rust now spawns the same world-server timer and runs `SELECT 1` against those three logical pools. This is operational parity for idle keep-alive, but not an internal 1:1 clone of TC's per-connection ping loop inside `DatabaseWorkerPool<T>`.
- **`QueryCallback` chaining**: TC's `WithChainingCallback(fn)` lets a callback fire a follow-up query and stay in the same chain (used heavily in account/char load multi-step flows). RustyCore replaces this with `async/await` — the chain becomes a sequence of `await` points in one async fn. **Acceptable divergence** for new code, but porting C++ that uses chaining-callbacks 1:1 needs to flatten into a single `async fn`.
- **`SQLQueryHolder` / `SQLQueryHolderCallback`**: TC's "fixed slots, execute as one async DB task, callback when all done" type. RustyCore now has `SqlQueryHolder` / `SqlQueryHolderResult` plus `Database::delay_query_holder_like_cpp`; callers still need to migrate character/pet loaders onto it.
- **`Field` DBMS-typed getters**: RustyCore now has `SqlResult` / `SqlFields` `read_typed::<T>(col)` and `try_read_typed::<T>(col)`, backed by `DatabaseFieldTypeLikeCpp` / `database_field_type_like_cpp(type_name)`. This mirrors TC's `Field::GetUInt8()` metadata guard (`DatabaseFieldTypes`) before sqlx decoding; `read::<T>` remains available for call sites that intentionally rely on sqlx's native conversion.
- **`FieldValueConverter` plug-in**: TC's pluggable column converters (e.g. `enum_string` → int, `time_t` → unix-ts) are absent. Rust port relies on direct sqlx decoding; consumers do their own conversion in handler code.
- **`no QueryProcessor` callback drain**: TC's `WorldSession::Update` polls `_queryProcessor.ProcessReadyCallbacks()` every tick. RustyCore's `WorldSession::process_pending().await` is invoked inside the per-session task loop, but **there is no global tick that drains DB callbacks across all sessions** — each session blocks on its own `.await`. (Cross-ref: `worldserver.md` §13.4 — the missing global tick. Specifically for DB: there is no central place where pending DB futures are reaped, but because each query is awaited directly there's no callback queue to drain in the first place.)
- **`PreparedStatement<T>` per-statement compile-time type** (the type tag T): RustyCore's `PreparedStatement` is **not parameterized by `S`** — it's a plain struct with a `&'static str sql`. The DB-binding constraint comes from `Database<S>::prepare(stmt: S)` ingesting an `S: StatementDef` and emitting a non-typed `PreparedStatement`. So once the `PreparedStatement` exists, you could in principle pass it to a different `Database<…>` (no compile-time prevention there) — TC catches this at the `pool.GetPreparedStatement(idx)` site. In practice this isn't exploitable because the `&'static str` is bound to one DB by construction.
- **`MySQLConnection::PrepareStatements()` actually prepares MYSQL_STMT objects on the server**: TC issues `mysql_stmt_prepare` for every `(connection × statement)` pair at pool `Open`; subsequent executes reuse the prepared handle (fast). sqlx caches prepared statements per-connection internally on first use, so the warm-up effect is amortized rather than upfront — measurable difference on the very first call to each statement, none afterward.
- **Hotfix prepared statements**: deliberate hybrid strategy. RustyCore ports C++ `DB2Manager::LoadHotfixData/Blob/OptionalData` as three named control-table statements and ports DB2 mirror-table statements only when a typed Rust store consumes them (`AreaTable`, `Mount*`, `CreatureDisplayInfo`, `CreatureModelData`, `Vehicle*`, `Phase*`, `UiMapXMapArt`). `HotfixStatements::base/max_id/locale` preserve exact generated C++ SQL for the 325 base, 325 max-id and 95 locale families so future store ports can stay mechanically faithful without pretending every overlay is live.
- **Character prepared statements**: 560 named Rust variants (including compatibility aliases) for TC's 523 `CHAR_*` statements, plus a generated parity harness that extracts all 523 C++ `PrepareStatement(CHAR_...)` SQL strings for coverage tests. The active WotLK named registry is closed by canonical C++ name; the only absent exact C++ names are the four BlackMarket statements intentionally deferred outside the active WotLK scope. Recent work added the initial pool-quest / ban / mail-list cluster, corrected `SEL_CHECK_NAME` + `SEL_SUM_CHARS` to C++ exact SQL, switched `SEL_ENUM` to the full C++ column shape with matching `handle_enum_characters` indexes, added the enum/undelete/customization/free-name/position/random-BG cluster, added the direct character-load auxiliary cluster, switched the active `SEL_CHARACTER` load query to the full C++ column shape with matching login-column indexes, added the direct auction/mail lifecycle cluster, added the direct item persistence / BOP trade / gems / transmog / account-transfer cluster, added the direct guild lifecycle / rank / bank-right / event-log / withdraw cluster, added the direct guild achievement / criteria / news cluster, added the direct channel / equipment-set / transmog-outfit / aura-save / currency-delete / account-data / tutorial cluster, added the direct petition / arena-team cluster, added the direct battleground-data / homebind / corpse cluster, added the direct GM bug / GM complaint / GM suggestion / LFG-data insert cluster, switched character creation to the full C++ `CHAR_INS_CHARACTER` row with matching save auxiliaries, added the direct group-difficulty / delete-info / invalid-cleanup / social / position / world-variable maintenance cluster, added the direct character-admin lookup / PINFO / item-count / item-by-entry search cluster, added the direct character-achievement / petition / declined-name / race-change / language / taxi / quest-cleanup cluster, added the direct faction-change / spell-cooldown / character-delete / action-bar cleanup cluster, added the direct quest-save / skill-save / spell-save / stats / trait / fishingsteps cluster, added the direct void-storage / CUF / calendar / pet / PvPStats / quest-tracker / aura-stored-location / WarMode cluster, and added the `SelectItemInstanceContent` aliases (`SEL_CHARACTER_INVENTORY`, `SEL_MAILITEMS`, `SEL_AUCTION_ITEMS`, `SEL_GUILD_BANK_ITEMS`) with SQL generated from the C++ macro. The macro is preserved byte-for-byte, including the suspicious C++ tail `iit.secondaryItemModifiedAppearanceSpec4, iit.itemModifiedAppearanceSpec5`; analyze that as a possible legacy bug before changing behavior.
- **`DBUpdater::Create`**: world-server now mirrors TC's missing-database fallback: if the first pool open returns MySQL `ER_BAD_DB_ERROR` (`1049`) and `Updates.AutoSetup` is enabled, RustyCore connects without a default schema, creates `CREATE DATABASE \`db\` DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`, and retries the pool open. Unlike TC it does not prompt on stdin or shell out to the `mysql` CLI for this step; the service-style non-interactive behavior is deliberate.
- **`Updates.CleanDeadRefMaxCount`** config key: RustyCore now mirrors TC's orphan cleanup gate: after matching all available update files, remaining `updates` rows are considered dirty/missing-source entries and are deleted only when the count is `<= Updates.CleanDeadRefMaxCount` or the value is negative. `Updates.Redundancy`, `Updates.AllowRehash`, and `Updates.ArchivedRedundancy` are also honored in the update decision path.
- **`updates_include` defaults**: RustyCore now bootstraps default `updates_include` rows for known TC database families when the table is empty, using the WotLK Classic base dump path layout. This also protects existing DBs whose table exists but has no rows before `update(source_dir)` runs.
- **`UpdateFetcher` "missing files" detection**: RustyCore tracks which applied `updates` rows were matched by source files and deletes orphaned rows using TC's `Updates.CleanDeadRefMaxCount` budget.
- **`DBUpdater::Populate` / `Update` failures are fatal** in TC (`return false` from `StartDB` aborts boot). RustyCore now propagates populate/update errors from world-server startup with DB-specific context, so a broken or empty DB fails fast instead of booting into later missing-table errors. (Cross-ref: `worldserver.md` §13.6.)
- **`MySQLConnection::EscapeString` / `mysql_real_escape_string`**: RustyCore exposes `wow_database::escape_string_like_cpp(value)` plus `Database::escape_string_like_cpp(&self, value)`. Prepared statements remain preferred, but legacy raw-SQL fragment ports now have the same MySQL special-byte escaping surface as TC.
- **`DatabaseWorkerPool::WarnAboutSyncQueries`**: RustyCore now has a task-local diagnostic scope (`warn_about_sync_queries_scope_like_cpp`) used around the current world-server session tick and selected canonical-map DB writes. DB calls made inside that scope emit `sql.performances` warnings. This is an operational diagnostic equivalent, not a sub-pool split: sqlx queries remain async.
- **`DatabaseWorkerPool::QueueSize`**: no equivalent — no queue exists; sqlx executes immediately on a free pool conn or `await`s for one. The metrics surface that consumed `QueueSize` (`db_queue_*` in TC's metrics) is therefore also gone.
- **`mysql_library_init` / `mysql_library_end`**: irrelevant — sqlx links against rust-mysql-async/`mysql_async` or against libmysql via FFI, init is automatic.
- **OpenSSL / TLS to MariaDB**: RustyCore now propagates TC's sixth `*DatabaseInfo` field: no `;ssl` writes `ssl-mode=DISABLED`, while `;ssl` writes `ssl-mode=REQUIRED` for sqlx pools, and `DbUpdater` passes TC's `--ssl` flag to the `mysql` CLI when applying base SQL files. This mirrors TC's `MYSQL_OPT_SSL_MODE` / `MYSQL_OPT_SSL_ENFORCE` boolean behavior plus `DBUpdater::ApplyFile` CLI arguments. CA / identity verification remains outside the TC `DatabaseInfo` schema for this branch.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The single 10-connection pool is shared across all 4 logical DBs in *some* views of the code? **(Audit: false — each DB has its own pool, see §13.2.)**
- Transaction deadlock retries now mirror TC's single static `_deadlockLock` retry gate. This avoids concurrent retry storms against the same locked rows.
- `read_string` fallback to `Vec<u8>` on `VARBINARY` columns is a specific quirk of `utf8mb4_bin` collation — character names in `characters.characters` use this collation. Any code using `read::<String>(col)` instead of `read_string(col)` for a name column will panic at runtime.
- `idle_timeout=1800s` (30 min) coincides with MariaDB's default `wait_timeout=28800s` (8 hours), so connection drops shouldn't be common — but `MaxPingTime` (TC default 30 min) was specifically tuned for shorter timeouts on hosted MariaDB.

**Tests existing:**
- 13 in `crates/wow-database/src/`:
  - `params.rs::tests`: 5 (set/sparse/overwrite/clear/all-types)
  - `statements/mod.rs::tests`: 8 (login/world SQL non-empty, table-name contains, unregistered empty, placeholder count)
- 0 integration tests (no live MariaDB in CI).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#DATABASE_FRAMEWORK.WBS.001** Cerrar la migracion auditada de `database/Database/AdhocStatement.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/AdhocStatement.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.002** Cerrar la migracion auditada de `database/Database/AdhocStatement.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/AdhocStatement.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.003** Cerrar la migracion auditada de `database/Database/DatabaseEnv.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseEnv.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.004** Cerrar la migracion auditada de `database/Database/DatabaseEnv.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseEnv.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.005** Cerrar la migracion auditada de `database/Database/DatabaseEnvFwd.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseEnvFwd.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [x] **#DATABASE_FRAMEWORK.WBS.006** Cerrar la migracion auditada de `database/Database/DatabaseLoader.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseLoader.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: Closed by `DatabaseLoaderLikeCpp`: open/populate/update/prepare queues run in C++ order and failure drains the close stack LIFO. Startup wiring into `world-server` remains an explicit later integration slice, not part of this file-level contract.
- [x] **#DATABASE_FRAMEWORK.WBS.007** Cerrar la migracion auditada de `database/Database/DatabaseLoader.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseLoader.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: Closed by `DatabaseLoaderLikeCpp` public API and TC database mask constants (`DATABASE_LOGIN`, `DATABASE_CHARACTER`, `DATABASE_WORLD`, `DATABASE_HOTFIX`, `DATABASE_MASK_ALL`). Startup wiring into `world-server` remains an explicit later integration slice.
- [ ] **#DATABASE_FRAMEWORK.WBS.008** Partir y cerrar la migracion auditada de `database/Database/DatabaseWorkerPool.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseWorkerPool.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 585 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.009** Cerrar la migracion auditada de `database/Database/DatabaseWorkerPool.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseWorkerPool.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.010** Cerrar la migracion auditada de `database/Database/Field.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Field.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.011** Cerrar la migracion auditada de `database/Database/Field.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Field.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.012** Cerrar la migracion auditada de `database/Database/FieldValueConverter.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/FieldValueConverter.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.013** Cerrar la migracion auditada de `database/Database/FieldValueConverter.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/FieldValueConverter.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.014** Cerrar la migracion auditada de `database/Database/FieldValueConverters.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/FieldValueConverters.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.015** Partir y cerrar la migracion auditada de `database/Database/Implementation/CharacterDatabase.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 732 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.016** Partir y cerrar la migracion auditada de `database/Database/Implementation/CharacterDatabase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 611 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.017** Partir y cerrar la migracion auditada de `database/Database/Implementation/HotfixDatabase.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1916 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.018** Partir y cerrar la migracion auditada de `database/Database/Implementation/HotfixDatabase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 1122 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.019** Cerrar la migracion auditada de `database/Database/Implementation/LoginDatabase.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/LoginDatabase.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.020** Cerrar la migracion auditada de `database/Database/Implementation/LoginDatabase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/LoginDatabase.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.021** Cerrar la migracion auditada de `database/Database/Implementation/WorldDatabase.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/WorldDatabase.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.022** Cerrar la migracion auditada de `database/Database/Implementation/WorldDatabase.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/WorldDatabase.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.023** Partir y cerrar la migracion auditada de `database/Database/MySQLConnection.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLConnection.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 605 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.024** Cerrar la migracion auditada de `database/Database/MySQLConnection.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLConnection.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.025** Cerrar la migracion auditada de `database/Database/MySQLHacks.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLHacks.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.026** Cerrar la migracion auditada de `database/Database/MySQLPreparedStatement.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLPreparedStatement.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.027** Cerrar la migracion auditada de `database/Database/MySQLPreparedStatement.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLPreparedStatement.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.028** Cerrar la migracion auditada de `database/Database/MySQLThreading.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLThreading.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.029** Cerrar la migracion auditada de `database/Database/MySQLThreading.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLThreading.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.030** Cerrar la migracion auditada de `database/Database/MySQLWorkaround.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLWorkaround.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.031** Cerrar la migracion auditada de `database/Database/PreparedStatement.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/PreparedStatement.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.032** Cerrar la migracion auditada de `database/Database/PreparedStatement.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/PreparedStatement.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.033** Cerrar la migracion auditada de `database/Database/QueryCallback.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryCallback.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.034** Cerrar la migracion auditada de `database/Database/QueryCallback.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryCallback.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [x] **#DATABASE_FRAMEWORK.WBS.035** Cerrar la migracion auditada de `database/Database/QueryHolder.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryHolder.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `crates/wow-database/src/query_holder.rs` + `Database::delay_query_holder_like_cpp`; slot execution order and empty-result nulling mirror C++.
- [x] **#DATABASE_FRAMEWORK.WBS.036** Cerrar la migracion auditada de `database/Database/QueryHolder.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryHolder.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `SqlQueryHolder` / `SqlQueryHolderResult`; fixed-size slots, out-of-range `false`, and indexed result access mirror C++.
- [x] **#DATABASE_FRAMEWORK.WBS.037** Cerrar la migracion auditada de `database/Database/QueryResult.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryResult.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `SqlResult` / `SqlFields` cursor APIs plus C++ `FieldTypeToString` metadata classification coverage (`TINY`, `SHORT`, `LONG`, `LONGLONG`, `YEAR`, `BIT`, `BLOB`, `STRING`, `VAR_STRING`). Live MySQL row-buffer ownership is n/a under sqlx.
- [x] **#DATABASE_FRAMEWORK.WBS.038** Cerrar la migracion auditada de `database/Database/QueryResult.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryResult.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by TC-compatible row/field count aliases (`row_count_like_cpp`, `get_row_count_like_cpp`, `get_field_count_like_cpp`) and `fetch_like_cpp`, while retaining existing Rust `read` / `try_read` accessors.
- [x] **#DATABASE_FRAMEWORK.WBS.039** Cerrar la migracion auditada de `database/Database/Transaction.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Transaction.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `SqlTransaction` commit/rollback execution, serialized 60s deadlock retry, raw SQL append, and idempotent `cleanup_like_cpp`; live deadlock DB integration remains optional because it requires a real two-connection lock scenario.
- [x] **#DATABASE_FRAMEWORK.WBS.040** Cerrar la migracion auditada de `database/Database/Transaction.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Transaction.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `SqlTransaction::{append, append_raw_sql_like_cpp, len, cleanup_like_cpp}` and `Database::execute_or_append` / `commit_transaction`. TC `TransactionCallback` is represented by Rust `async/await` rather than a separate callback class.
- [x] **#DATABASE_FRAMEWORK.WBS.041** Cerrar la migracion auditada de `database/Updater/DBUpdater.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/DBUpdater.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `DbUpdater::{populate, update}` plus world-server create-database fallback. `populate` now also mirrors TC's empty-base-file success path. File discovery/diff internals remain tracked under `UpdateFetcher` WBS.
- [x] **#DATABASE_FRAMEWORK.WBS.042** Cerrar la migracion auditada de `database/Updater/DBUpdater.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/DBUpdater.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `DbUpdater`, `UpdateDatabaseKindLikeCpp`, update-mask constants in `DatabaseLoaderLikeCpp`, and TC-style populate/update config gates. External `mysql` executable auto-detection remains simplified to PATH lookup by `Command::new("mysql")`.
- [x] **#DATABASE_FRAMEWORK.WBS.043** Cerrar la migracion auditada de `database/Updater/UpdateFetcher.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/UpdateFetcher.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by `DbUpdater::update` scan/hash/apply logic: include dirs, filename-order sorting, duplicate filename rejection, renamed-file detection, rehash/state update gates, and orphan cleanup threshold all mirror C++ behavior at unit level. Live MariaDB update application remains under #DB.16 integration coverage.
- [x] **#DATABASE_FRAMEWORK.WBS.044** Cerrar la migracion auditada de `database/Updater/UpdateFetcher.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/UpdateFetcher.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: closed by Rust equivalents for `UpdateConfigLikeCpp`, `AppliedUpdateFileLikeCpp`, `UpdateDatabaseKindLikeCpp`, `PopulateBaseActionLikeCpp`, and filename-ordered update file storage. C++ callback object shape is represented by `DbUpdater` methods and async sqlx calls.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [x] **#DB.1** Use TC database thread-count config for world-server pools: read `<Db>Database.WorkerThreads` + `<Db>Database.SynchThreads` for Login/Character/World/Hotfix and pass their sum to `Database::open_with_pool_size`. Defaults follow TC's loader default of `1 + 1`; invalid values fall back to `1` per side with a warning. (L) — **same as `worldserver.md` #WS.21**
- [x] **#DB.2** Implement DB keep-alive: spawn one shared world-server task using `MaxPingTime` (default 30) that runs `SELECT 1` against Character/Login/World, matching TC's `World::Update` keep-alive pool set. Hotfix is intentionally excluded because TC does not call `HotfixDatabase.KeepAlive()` here. (L) — **same as #WS.6**
- [x] **#DB.3** Make enabled `DbUpdater::populate` / `update` failures fatal: return error from `world-server/main.rs` instead of `tracing::warn!`. Boot now aborts if base SQL is missing, the DB is empty + populate fails, or an enabled update step fails. (L)
- [x] **#DB.4** Bootstrap `updates_include` rows on first `populate` / first empty-table `update`: insert the WotLK Classic default path set for auth, characters, world, and hotfixes so a fresh install actually applies updates without operator intervention. (M)
- [x] **#DB.5** Implement `DBUpdater::Create` equivalent for world-server startup: detect MySQL `Unknown database` (`1049`) on initial pool open, connect without a DB name, run TC's `CREATE DATABASE ... DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`, and retry. (M)
- [x] **#DB.6** Implement `Updates.Redundancy` (default `true`) and `Updates.AllowRehash` (default `true`): when redundancy is disabled, skip already-applied files; when rehash is enabled and the stored hash is empty, update the hash without reapplying; when hash matches but state changed, update state only. (L)
- [x] **#DB.7** Implement `Updates.ArchivedRedundancy` (default `false`): when false, skip redundancy checks for files that are archived both in the DB and the available update path; when true, changed archived updates can be reapplied. (L)
- [x] **#DB.8** Implement `Updates.CleanDeadRefMaxCount`: delete `updates` rows whose file no longer exists, up to N per run. Rust now mirrors TC's `cleanDeadReferencesMaxCount < 0 || orphan_count <= cleanDeadReferencesMaxCount` gate and leaves excessive dirty rows in place with an error log. (M)
- [x] **#DB.9** Add `WarnAboutSyncQueries`-style guard: track a Tokio task-local "in tick" flag and warn when DB calls run inside it. The guard is wired around the current world-server session tick and canonical event DB writes. Same as `worldserver.md` #WS.25. (M)
- [x] **#DB.10** Add a `QueryHolder`-style helper: `SqlQueryHolder` stores fixed prepared-statement slots, `Database::delay_query_holder_like_cpp` executes set slots in order and returns `SqlQueryHolderResult` indexed by slot. Empty/unset slots return `None`, matching C++ null `PreparedQueryResult`. Caller migration remains under player/pet load tasks. (M)
- [x] **#DB.11** Port the remaining active-WotLK character prepared statements as named Rust variants (cross-ref `INVENTORY.md`, `characters.md`). Current state: 560 named variants including compatibility aliases; active WotLK `CharacterDatabase` named coverage is closed by exact C++ enum name. The only absent exact C++ names are 4 BlackMarket future-scope statements, intentionally deferred unless the target version/scope changes. Generated tests extract all 523 exact SQL strings and verify the `SelectItemInstanceContent` aliases against the C++ macro expansion. (XL — split per-domain: inventory, achievements, mails, guilds, BG/arena state, etc.)
- [x] **#DB.12** Decide hotfix prepared-statement strategy: formalized as `HotfixStatementStrategyLikeCpp::ControlTablesAndSelectedDb2Overlays`. Rust keeps the three `DB2Manager::LoadHotfix*` control-table statements live, ports selected mirror-table overlays as each typed DB2 store consumes them, and keeps generated base/max-id/locale helpers tested against C++ for future overlay expansion. (L)
- [x] **#DB.13** Implement TLS connection options: the parsed sixth `*DatabaseInfo` field (`;ssl`) is passed through to sqlx as `ssl-mode=REQUIRED`; absence or any other value keeps TC-equivalent `ssl-mode=DISABLED`. `DbUpdater` also passes TC's `--ssl` flag to the `mysql` CLI for base-file imports. (M)
- [x] **#DB.14** Implement `EscapeString` for raw-SQL-fragment use cases: `escape_string_like_cpp` mirrors `mysql_real_escape_string` special-byte escaping (`NUL`, newline, carriage-return, backslash, quotes, Ctrl-Z) for UTF-8 strings, with a `Database` method for pool-style call sites. (L)
- [x] **#DB.15** Add populate/update error context: Rust reports `Could not populate/update the <DB> database`, and CLI base-file failures include both the SQL file path and `mysql` stderr. `ApplyFile` now also uses TC's `BEGIN; SOURCE file; COMMIT;` wrapper. (L)
- [ ] **#DB.16** Add integration test harness: spin up an embedded MariaDB (or Docker mariadb:10.6 in CI) and run `populate` + `update` against it; verify updates table population. (H)
  - [x] **#DB.16a** Add a gated live MariaDB harness (`updater::tests::live_mariadb_populate_and_update_records_updates_like_cpp`, ignored by default) that creates a disposable `rustycore_it_*` schema, applies a base SQL through the same `mysql -e "BEGIN; SOURCE ...; COMMIT;"` path used by TC/Rust `populate`, runs `update` twice, and verifies `base_marker`, ordered dependent updates, `populate` no-op on a non-empty DB, `updates` tracking rows without duplicate re-application, and `SqlResult`/`SqlFields` `VARBINARY` string fallback against real `MySqlRow` data.
    Run manually with a MariaDB server and `mysql` CLI available:
    `RUSTYCORE_DB_IT_USER=trinity RUSTYCORE_DB_IT_PASS=trinity RUSTYCORE_DB_IT_HOST=127.0.0.1 RUSTYCORE_DB_IT_PORT=3306 cargo test -p wow-database live_mariadb_populate_and_update_records_updates_like_cpp -- --ignored --nocapture`.
  - [x] **#DB.16b** Wire the live harness into CI with an ephemeral MariaDB service/container. `.github/workflows/wow-database-live.yml` starts `mariadb:10.6`, installs a MySQL-compatible client so Rust's TC-style `mysql` CLI path is exercised, waits for readiness, and runs the ignored live harness with `RUSTYCORE_DB_IT_*` credentials. Local validation covers workflow structure and normal Rust tests; the remote GitHub Actions run is the authoritative CI execution evidence.
- [x] **#DB.17** Audit deadlock-retry concurrency: Rust now replicates TC's `_deadlockLock` static mutex and 60-second retry window for transaction deadlocks. (L)
- [x] **#DB.18** Add a `DatabaseError::TableMissing` variant so callers can distinguish "DB not populated / structure out of date" from generic query errors. Rust maps MySQL `ER_NO_SUCH_TABLE` (`1146`, SQLSTATE `42S02`) into this variant, matching C++'s special handling of `ER_NO_SUCH_TABLE` in `_HandleMySQLErrno`. (L)
- [x] **#DB.19** Add a `Field`-style typed accessor with metadata: `SqlResult::read_typed::<u8>(col)` / `try_read_typed::<u8>(col)` check `column_type_name` through `DatabaseFieldTypeLikeCpp` before decoding, matching TC's `Field`/`DatabaseFieldTypes` guard. (M)
- [x] **#DB.20** Implement async `KeepAlive` from a free-standing `tokio::spawn` while the global world tick is still pending. Covered by #DB.2 / #WS.6; if a future global tick centralizes this, keep the same Character/Login/World-only scope.
- [x] **#DB.21** Add a `DatabaseLoader`-style sequencer: `DatabaseLoaderLikeCpp` queues async open/populate/update/prepare steps, keeps a LIFO close stack, exposes TC's DB update-mask constants, and rolls back registered closers on any phase failure. `world-server/main.rs` startup remains inline and can be migrated onto the sequencer in a later wiring slice. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#DATABASE_FRAMEWORK.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp`. Rust target: `crates/wow-database`. | `cargo test -p wow-database` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#DATABASE_FRAMEWORK.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp`. Rust target: `crates/wow-database`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#DATABASE_FRAMEWORK.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp`. Rust target: `crates/wow-database`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#DATABASE_FRAMEWORK.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp`. Rust target: `crates/wow-database`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Test: `Database::<LoginStatements>::open_with_pool_size(uri, 1)` succeeds; spawn 100 concurrent `query` calls against `SEL_REALMLIST` and verify all complete (queue serialization).
- [x] Test: `SqlTransaction` exposes unit coverage for the process-wide deadlock retry lock and the 60-second C++ retry window. A live MySQL deadlock integration test remains optional because it needs two connections and table locks.
- [x] Test: `DbUpdater::populate` is a no-op on a non-empty DB (returns `Ok(false)`). Covered by the ignored live MariaDB harness under `#DB.16a`; normal unit runs keep it skipped.
- [x] Test: `DbUpdater::populate` invokes `mysql` CLI and the database has the expected tables afterward. Covered by the ignored live MariaDB harness under `#DB.16a`; normal unit runs keep it skipped.
- [x] Test: `DbUpdater::update` applies new files in lexicographic order, records SHA1, and skips already-applied entries. Covered by the ignored live MariaDB harness under `#DB.16a`: the second update depends on the first update-created table, and a second `update()` run preserves one tracking row per file.
- [x] Test: update decision honors `Updates.Redundancy=false`, `Updates.AllowRehash`, and changed hashes.
- [x] Test: update decision does **not** re-apply an ARCHIVED-state file when `Updates.ArchivedRedundancy=false`, and does when enabled.
- [ ] Test: `DbUpdater::update` detects a renamed file (same hash, different filename) and updates the `name` column instead of re-applying. Pure C++ decision coverage is present, including the "old file still exists => treat as copy/new file" guard; live DB verification remains under #DB.16.
- [x] Test: `split_sql` correctly handles `--` line comments, `#` comments, `/* */` blocks, `'\''` escapes, and `\"` escapes (existing behaviour; pin in tests).
- [x] Test: `SqlResult::read_string` on a `VARBINARY` column returns the UTF-8 string (not panic). Pure fallback conversion is covered and shared by `SqlResult`/`SqlFields`; the ignored live MariaDB harness under `#DB.16a` now covers the real `MySqlRow` path.
- [x] Test: `PreparedStatementTask::Query` empty-result nulling is represented in `SqlQueryHolder`: empty `SqlResult` is normalized to `None`, matching C++ deleting a zero-row `PreparedResultSet` and returning null.
- [x] Test: `PreparedStatement` indexed setters with sparse indices fill intermediates with TC's default `bool false` value; explicit `set_null()` still binds `SqlParam::Null`. This mirrors C++ `PreparedStatementBase(capacity)` default-constructing `PreparedStatementData::data` to the first `std::variant` alternative (`bool false`).
- [x] Test: `PreparedStatementBase` fixed bind capacity is represented: Rust `PreparedStatement::new(sql)` preallocates the counted placeholder capacity with TC's default `bool false`, and setters assert when the caller binds past capacity, matching C++ `ASSERT(index < statement_data.size())`.
- [x] Test: `PreparedStatementData::ToString` / `MySQLPreparedStatement::getQueryString` debug expansion is represented by `PreparedStatement::expanded_sql_like_cpp()`, including bool/u8/i8 numeric widening, quoted strings, `BINARY`, `NULL`, and sparse default `0`.
- [x] Test: keep-alive config/scope/SQL mirrors C++ (`MaxPingTime`, Character/Login/World only, `SELECT 1`). Runtime timer firing remains a service/integration concern rather than a paused Tokio unit fake.
- [x] Test: `Updates.EnableDatabases` mask mirrors C++ `DatabaseLoader` / `DBUpdater<T>::IsEnabled`: default `15` enables Login/Character/World/Hotfix, partial masks gate per-DB auto-create and updater work, and mask `0` disables all updater work.
- [x] Test: `Updates.AutoSetup=0` mirrors C++ source semantics: it disables missing-database auto-create, but does **not** disable `Populate`/`Update`; `Updates.EnableDatabases=0` is the C++ switch that disables updater work.
- [x] Test: startup propagates DB updater failures with DB-specific context when `Updates.AutoSetup=1` (#DB.3).
- [x] Test: `LoginStatements::SEL_REALMLIST` and `WorldStatements::DEL_LINKED_RESPAWN` produce different `&'static str` SQL, and using `LoginStatements::SEL_REALMLIST` on a `Database<WorldStatements>` is covered by a rustdoc `compile_fail` test on `Database<S>::prepare`.
- [x] Test: hotfix strategy is explicit (`ControlTablesAndSelectedDb2Overlays`), control-table statements are distinguishable from selected DB2 overlays, and generated base/max-id/locale helpers stay validated against C++ `HotfixDatabase.cpp`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp` | `crates/wow-database/` \| ⚠️ partial (4 typed pools + prepared-statement enums + transactions + QueryHolder + DB updater work; missing: per-pool sync/async split, callback chaining type, libmysql `Library_Init`) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#DATABASE_FRAMEWORK.DIV.001` | `crates/wow-data/src/hotfix_cache.rs` exists and `crates/wow-database/src/statements/hotfix.rs` has named control/overlay statements plus generated helpers | refs: `/home/server/woltk-trinity-legacy/src/server/game/DataStores/DB2Stores.cpp:1539-1726`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h` | `resolved-as-strategy` | Former path drift corrected. Remaining work is per-store overlay consumption, not a missing cache module. |

<!-- REFINE.023:END known-divergences -->

- **TC ships two pools per logical DB**; merging them into one `sqlx::Pool` is a deliberate simplification, but at high login churn you can saturate the single pool while a transaction is mid-retry. Consider tuning `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads` upward; Rust will use their sum as the single pool size.
- **`PreparedStatement<T>` in C++ is type-tagged**; in Rust the tagging is at `Database<S>::prepare(stmt)` ingestion time only. Don't try to add `PhantomData<S>` to `PreparedStatement` itself — the existing test design (compile-fail) covers the misuse case at the right place.
- **sqlx prepared-statement caching**: sqlx caches prepared `MYSQL_STMT` handles per-connection automatically. First call to a statement on a fresh connection prepares; subsequent calls reuse. This is why we don't need TC's `MySQLConnection::PrepareStatements()` warm-up — but it does mean cold pool acquisitions pay one round-trip extra. Don't `pool.close()` and re-`open()` to "reload" prepared statements; just restart the binary.
- **`DBUpdater::Update` SQL splitter cannot handle DELIMITER**: TrinityCore's TDB world dump uses `DELIMITER //` for trigger definitions. RustyCore's `populate` shells out to `mysql` CLI specifically for this reason; `update` (which never sees triggers) is statement-by-statement. If a future update adds a trigger or stored procedure, **the splitter will fail** — extend it or fall back to the CLI for that file.
- **`Updates.EnableDatabases` default**: enabled database updater phases can try to reach a `mysql` CLI binary on `PATH` to run `populate`. If `mysql` is not installed (e.g. a dev VM with only `mariadb-client` symlinked differently), `populate` fails with an opaque `command not found`. Either install `mysql` or pre-populate manually.
- **`ConnectionFlags::SYNCH` queries in TC are guaranteed to run on a sync sub-pool** so that nothing outside the world thread blocks them. RustyCore loses this guarantee — **a sync query issued from inside an async tick can still starve when the merged pool is saturated by async work**. Watch out under heavy load; tune up `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads` before reaching for a redesign.
- **`updates` table schema**: TC's columns are `(name, hash, state, timestamp, speed)` with `state` as an enum. The Rust port creates the same schema with `IF NOT EXISTS` so it's interoperable — you can run TC's `worldserver` against the same DB and Rust will respect its `updates` rows. **Compatibility tested 2026-04: ✅.**
- **Deadlock retry is single-mutex in TC** (`TransactionTask::_deadlockLock`); RustyCore now mirrors that with a process-wide Tokio mutex and 60-second retry window. **#DB.17 closed.**
- **Boot order matters**: world-server can now create missing schemas when `Updates.AutoSetup=1`, but `populate` still needs the base SQL files and the `mysql` CLI on `PATH`. If `Updates.AutoSetup=0`, operators must pre-create and import the databases manually.
- **`max_allowed_packet`**: `populate` passes `--max-allowed-packet=1GB` to the CLI like TC. The sqlx pool inherits the server-side default (often 16 MiB). Large blob updates via `update()` may fail with `Packet too large`. Either bump server-side `max_allowed_packet` or chunk the migration.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `MySQLConnectionInfo` (parsed `host;port;user;pass;db;ssl`) | `wow_database::database::build_connection_string_with_ssl_like_cpp(host, port, user, pass, db, ssl) -> String` | Returns a `mysql://…` URL with explicit `ssl-mode=DISABLED/REQUIRED`; sqlx parses |
| `enum ConnectionFlags { ASYNC, SYNCH, BOTH }` | (none) | Single sqlx pool, no sub-pool flag |
| `class MySQLConnection` | `sqlx::pool::PoolConnection<MySql>` (acquired ad-hoc) | No standalone "connection" struct |
| `class LoginDatabaseConnection` (and 3 siblings) | (subsumed) | Replaced by `Database<LoginStatements>` (and 3 siblings) — the `<S>` type tag plays the same role as inheritance |
| `class DatabaseWorkerPool<T>` | `wow_database::Database<S: StatementDef>` (= `sqlx::Pool<MySql>` + `PhantomData<S>`) | Single pool, no sync/async split |
| `template <> class PreparedStatement<T>` | `wow_database::PreparedStatement` (no type tag) | Tagging happens at `Database<S>::prepare(stmt: S)` |
| `class PreparedStatementBase` | (merged into `PreparedStatement`) | No virtual base needed; setters live directly on the only concrete type |
| `struct PreparedStatementData (std::variant<…>)` | `enum SqlParam { Null, Bool, I8…U64, F32, F64, String, Bytes }` | 1:1 type list |
| `pStmt->setUInt8(idx, val)` | `stmt.set_u8(idx, val)` | And siblings (`set_i8` / `set_string` / `set_bytes` / `set_null`) |
| `class Field` | (merged into `SqlResult`/`SqlFields`) | Per-column getter is `result.read_typed::<T>(col)` when TC-style metadata validation matters; `read::<T>` remains the raw sqlx path |
| `Field::GetUInt8()` | `result.read_typed::<u8>(col)` | Checks MySQL column type metadata before sqlx decoding |
| `Field::GetCString()` | (use `read::<String>` then `.as_str()`) | No raw `*const c_char` lifetime to manage |
| `Field::IsNull()` | `result.is_null(col)` | — |
| `enum DatabaseFieldTypes` | `DatabaseFieldTypeLikeCpp` + `database_field_type_like_cpp(type_name)` | Rust classifies sqlx `column.type_info().name()` into TC-style categories |
| `class ResultSet` (text protocol) | `SqlResult` returned by `direct_query(sql)` | Text vs binary protocol indistinguishable in sqlx |
| `class PreparedResultSet` (binary protocol) | `SqlResult` returned by `query(&stmt)` | Same type as text; sqlx selects protocol |
| `class QueryCallback` | `async fn` returning `Result<SqlResult, DatabaseError>` | Chaining = sequential `await` |
| `qc.WithChainingCallback(fn)` | `let r1 = db.query(s1).await?; let r2 = db.query(s2_built_from(r1)).await?;` | Flatten chains into linear async code |
| `class SQLQueryHolder<T>` | `wow_database::SqlQueryHolder` | fixed-size prepared-query slots |
| `SQLQueryHolderCallback::AfterComplete(fn)` | `.await` the holder future, then call the Rust continuation directly | Callback object eliminated by async/await |
| `class TransactionBase` / `class Transaction<T>` | `wow_database::SqlTransaction` (untyped) | No `<T>` tag — txns are not type-bound to a DB; runtime check via the pool they're committed against |
| `trans->Append(sql)` / `trans->Append(stmt)` | `tx.append(stmt)` (no raw-SQL `Append` — wrap raw SQL as a `PreparedStatement::new`) | — |
| `pool.CommitTransaction(trans)` | `db.commit_transaction(tx).await?` | Async, with serialized deadlock retry for up to 60s |
| `class TransactionCallback` (`AfterComplete`) | `db.commit_transaction(tx).await.map(|_| post_action())` | — |
| `pool.Execute(sql)` (async fire-and-forget) | `tokio::spawn(async move { db.direct_execute(sql).await })` | Awaiting blocks the caller; spawn for fire-and-forget |
| `pool.DirectExecute(sql)` (sync) | `db.direct_execute(sql).await` | All Rust DB ops are async; "direct" = "no transaction" |
| `pool.PExecute("UPDATE foo SET x = {}", val)` | `db.direct_execute(&format!("UPDATE foo SET x = {val}")).await` | No native string-format wrapper |
| `pool.AsyncQuery(stmt)` | `db.query(&stmt).await` | sqlx is always-async |
| `pool.DelayQueryHolder(holder)` | `db.delay_query_holder_like_cpp(&holder).await` | Async/await replaces callback object; execution order is slot order like C++ `SQLQueryHolderTask::Execute` |
| `pool.GetPreparedStatement(idx)` | `db.prepare(LoginStatements::FOO)` | — |
| `pool.EscapeString(str)` | `db.escape_string_like_cpp(str)` / `wow_database::escape_string_like_cpp(str)` | Prefer bound parameters; this is for legacy raw-SQL fragments |
| `pool.KeepAlive()` | `Database::keep_alive_like_cpp()` + world-server keep-alive task | `SELECT 1` every `MaxPingTime` for Character/Login/World |
| `pool.WarnAboutSyncQueries(true)` | `warn_about_sync_queries_scope_like_cpp(async { ... })` | task-local warning scope, wired into current world-server ticks |
| `class DatabaseLoader` (5-queue sequencer + close stack) | `wow_database::DatabaseLoaderLikeCpp` | Core sequencer + rollback semantics ported; world-server wiring still inline |
| `class DBUpdater<T>` | `wow_database::updater::DbUpdater` (single non-generic struct) | The `<T>` tag is replaced by the per-DB connection params passed to `DbUpdater::new` |
| `DBUpdater::Create(pool)` | `open_with_pool_size_and_auto_create_like_cpp(...)` during world-server pool open | non-interactive; creates schema then retries |
| `DBUpdater::Populate(pool)` | `updater.populate(base_sql).await` | Shells to `mysql` CLI |
| `DBUpdater::Update(pool)` | `updater.update(source_dir).await` | Walks `sql/updates/`, hashes, applies via sqlx |
| `class UpdateFetcher` | (merged into `DbUpdater::update`) | — |
| `class UpdateException` | `anyhow::Error` (via `Result<()>`) | — |
| `class MySQLPreparedStatement` | (none — sqlx caches `MYSQL_STMT` per connection internally) | — |
| `mysql_library_init` / `mysql_library_end` | (none) | sqlx handles |
| `MYSQL_OPT_SSL_MODE` / `MYSQL_OPT_SSL_ENFORCE` + `DBUpdater::ApplyFile --ssl` | sqlx connection-string `?ssl-mode=DISABLED/REQUIRED` + updater CLI `--ssl` | Mirrors TC's `;ssl` boolean switch; CA verification is not part of this branch's `DatabaseInfo` schema |
| `mysql_real_escape_string` | `escape_string_like_cpp(value)` | Mirrors MySQL special-byte escaping for UTF-8 strings |
| `Trinity::Asio::IoContext` (per-pool) | implicit Tokio runtime | — |

---

## 13. Audit (2026-05-01)

**Audited:**
- C++: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLConnection.h` (116 lines), `DatabaseWorkerPool.h` (243 lines), `PreparedStatement.h` (125 lines), `Field.h` (142 lines), `QueryCallback.h` (67 lines) + `.cpp` (221 lines), `Transaction.h` (119 lines), `QueryResult.h` (85 lines), `QueryHolder.h` (81 lines), `DatabaseLoader.h` (78 lines), `Implementation/LoginDatabase.h` (195 lines) + `.cpp` (lines 1-80), `Implementation/CharacterDatabase.h`/`HotfixDatabase.h`/`WorldDatabase.h` (counted statements). `Updater/DBUpdater.h` (96 lines) + `.cpp` (1-120). Total ~2200 lines reviewed in detail; ~10 600 lines surveyed.
- Rust: `/home/server/rustycore/crates/wow-database/src/lib.rs`, `database.rs`, `params.rs`, `result.rs`, `transaction.rs`, `updater.rs`, `loader.rs`, `statements/mod.rs`, `statements/login.rs`, `statements/character.rs`, `statements/world.rs`, `statements/hotfix.rs`. Hotfix runtime cache lives in `crates/wow-data/src/hotfix_cache.rs`; world-server startup wiring lives in `crates/world-server/src/main.rs`.

### 13.1 Audit summary

The Rust port covers the **shape** of TC's database framework correctly: four typed pools, prepared statements indexed by DB-specific enums, transactions with deadlock retry, and an auto-applier for `sql/updates/`. The compile-time DB-to-statement binding is actually *stricter* in Rust than in TC because of how `Database<S>::prepare(s: S)` is gated.

The gaps fall in three buckets:

1. **Pool topology** (Medium): Rust still merges TC's `Sync`/`Async` sub-pools into one `sqlx` pool, but world-server now sizes that pool from TC's `WorkerThreads + SynchThreads` config and runs the `MaxPingTime` keep-alive task. Cross-references the worldserver audit (`worldserver.md` §13.5 / §13.6).
2. **Statement coverage** (High): login is 137/137 (✅), world is 86/56 (✅, Rust adds extras), characters is 30/523 (❌, ~6% ported), hotfixes use the formal hybrid strategy (3 control tables + selected overlays + generated C++ helpers; remaining overlays land per typed DB2 consumer).
3. **Updater operator UX** (Medium): `Create`, `updates_include`, and `Updates.CleanDeadRefMaxCount` are now covered for world-server startup.

The `QueryCallback`-chain pattern is **acceptably absent** because Rust's `async/await` covers the same need more directly. `SQLQueryHolder` now exists as an infrastructure helper (#DB.10); the remaining work is migrating character/pet load callers onto it.

### 13.2 Pool architecture parity

| TC | Rust | Parity |
|---|---|---|
| `DatabaseWorkerPool<T>` template, one instance per logical DB | `Database<S: StatementDef>`, one instance per logical DB | ✅ |
| Two sub-pools per DB: `_connections[IDX_ASYNC]` (N conns) + `_connections[IDX_SYNCH]` (M conns) | One `sqlx::Pool<MySql>` per DB | ⚠️ merged |
| `<Db>Database.WorkerThreads` / `.SynchThreads` config | read in `world-server` and summed into `open_with_pool_size` | ✅ #DB.1 |
| Per-pool `Trinity::Asio::IoContext` | implicit Tokio runtime | ✅ acceptable divergence |
| `MaxPingTime` keep-alive every N minutes | world-server task calls `keep_alive_like_cpp` for Character/Login/World | ✅ #DB.2 |
| `mysql_options(MYSQL_OPT_RECONNECT, 0)` + manual `_HandleMySQLErrno` retry (5 attempts) | sqlx auto-reconnect via pool acquisition | ⚠️ different mechanism, similar effect |
| `idle_timeout` ~ default | `idle_timeout=1800s` (`database.rs:47`) | ✅ matches TC operational expectation |
| `WarnAboutSyncQueries` debug guard | (none) | ❌ #DB.9 |
| `KeepAlive()` ping per pool conn | `SELECT 1` against each logical `sqlx` pool, not forced per internal connection | ⚠️ operational parity |
| Four logical pools (login/character/world/hotfix) | Four type aliases (`LoginDatabase` / `CharacterDatabase` / `WorldDatabase` / `HotfixDatabase`) | ✅ |
| Pool open in `worldserver/Main.cpp::StartDB` | Pool open in `world-server/src/main.rs` lines 177-228 | ✅ wired |
| `<Db>DatabaseInfo.{Host,Port,Username,Password,Database}` config keys | Same keys read via `wow_config::get_string_default` | ✅ |
| Pool size hardcoded? | `Database::open` still defaults to 10, but `world-server` uses `open_with_pool_size` from TC thread-count config | ✅ #DB.1 |

### 13.3 Prepared statement registry

| DB | TC enum count | Rust enum count | Coverage |
|---|---|---|---|
| login (`LoginStatements`) | 137 | 137 | ✅ 100% |
| character (`CharStatements`) | 523 | 30 | ❌ ~6% |
| world (`WorldStatements`) | 56 | 86 | ✅ super-set (Rust adds quest/spell loaders not in TC's 56) |
| hotfix (`HotfixStatements`) | 325 base + 325 max-id + 95 locale generated families; 3 direct control queries | 15 named live statements + generated helpers | ⚠️ hybrid strategy — control path live; overlays ported per DB2 consumer |

**Registry pattern**: TC binds `enum Statements : uint32` → SQL string in `<Db>DatabaseConnection::DoPrepareStatements()` via `PrepareStatement(idx, sql, flags)` calls (`LoginDatabase.cpp:26` onward). Rust binds via the `StatementDef` trait `fn sql(self) -> &'static str` per enum, with the SQL inline in a `match` arm (`statements/login.rs::sql_for`). Both are static, both are compile-time. **Functional parity for the patterns that are ported.** The mechanism is different but the constraint (one enum value → one SQL string per DB, type-checked) holds.

CLAUDE.md mentions a "prepared statement registry" as completed. Audit confirms: the **infrastructure** is complete; the **statement set** is complete for login + world, partial for character, and intentionally hybrid for hotfixes. Do not count every C++ hotfix overlay as WotLK runtime-complete until a typed Rust DB2 store consumes it.

### 13.4 Transactions

| TC | Rust | Parity |
|---|---|---|
| `TransactionBase::Append(sql)` / `Append(PreparedStatementBase*)` | `SqlTransaction::append(stmt)` (no raw-SQL append; wrap as `PreparedStatement::new`) | ⚠️ partial |
| `pool.BeginTransaction()` returns `SQLTransaction<T>` (typed) | `SqlTransaction::new()` (untyped) — runtime-bound to whichever pool you commit against | ⚠️ runtime-typed |
| `pool.CommitTransaction(trans)` (async) | `db.commit_transaction(tx).await` | ✅ |
| `pool.AsyncCommitTransaction(trans)` returns `TransactionCallback` | `commit_transaction` is already async, no separate callback type | ✅ acceptable divergence |
| `pool.DirectCommitTransaction(trans)` (sync) | (none) — same as commit | ✅ irrelevant in async land |
| `TransactionTask::Execute` retries deadlocks until `DEADLOCK_MAX_RETRY_TIME_MS` | `SqlTransaction::commit` retries MySQL error 1213 for up to 60s | ✅ |
| `TransactionTask::_deadlockLock` static mutex serializes retries | process-wide Tokio mutex serializes transaction deadlock retries | ✅ #DB.17 |
| `pool.ExecuteOrAppend(trans, stmt)` | `db.execute_or_append(trans, stmt).await` | ✅ |

### 13.5 QueryCallback / async chain

TC's `QueryCallback` exists because the world thread is **single-threaded** and synchronous: a query returns a future, the world thread polls `_queryProcessor.ProcessReadyCallbacks()` every tick, and the callback runs **on the world thread** when the future is ready. `WithChainingCallback` lets a callback return a new query and stay in the chain.

Rust `async/await` collapses this entirely: `let r = db.query(&stmt).await?` does the same thing in linear code. There is no "callback queue to drain" because the await point already serializes at the per-session task. **No port needed for the chain pattern itself.**

However, **the worldserver audit (`worldserver.md` §13.4) noted there's no global tick to drain callbacks**. For DB this manifests as: per-session `await`s mean per-session DB queries never block siblings, but also there is no central place to enforce per-session backpressure or to surface "DB queue depth" as a metric. (TC's `pool.QueueSize()` feeds `db_queue_*` Prometheus counters; absent in Rust — see #WS.13.)

| TC | Rust | Parity |
|---|---|---|
| `QueryCallback::WithCallback(fn)` (terminal) | `let r = db.query(&stmt).await?; fn(r);` | ✅ syntactically simpler |
| `QueryCallback::WithChainingCallback(fn)` | `let r1 = db.query(&s1).await?; let r2 = db.query(&s2(r1)).await?;` | ✅ |
| `QueryCallback::InvokeIfReady` polled from `World::Update` | (none) — `await` inline | ✅ acceptable divergence |
| `WorldSession::_queryProcessor.AddCallback(cb)` | `WorldSession::process_pending().await` (per-session) | ⚠️ no global drain — see `worldserver.md` |
| `pool.QueueSize()` for metrics | (none) | ❌ #WS.13 |
| `class SQLQueryHolder<T>` (fixed slots, all-done callback) | `SqlQueryHolder` + `SqlQueryHolderResult`; callers await `delay_query_holder_like_cpp` | ✅ #DB.10 infra |

### 13.6 DB Updater

| TC `DBUpdater<T>` step | Rust `DbUpdater` | Parity |
|---|---|---|
| `Create(pool)` — `mysql -e "CREATE DATABASE …"` if DB missing | world-server detects MySQL 1049, creates the schema with TC charset/collation, then retries opening the pool | ✅ #DB.5 |
| `Populate(pool)` — if 0 tables, apply `GetBaseFile()` via `mysql` CLI | `populate(base_sql)` — if `information_schema.tables` count = 0, shell `mysql … -e "BEGIN; SOURCE …; COMMIT;"` | ✅ |
| Failure to populate/update → `false` from `StartDB` → boot abort | startup propagates `populate` / `update` errors with DB-specific context | ✅ #DB.3 |
| `Update(pool)` — walk `updates_include`, sha1 each `.sql`, apply pending | `update(source_dir)` — walks paths from DB-resident `updates_include` table, sha1, applies via sqlx statement-by-statement | ✅ |
| Bootstrap `updates_include` rows on first run | inserts TC WotLK Classic default include rows when the table is empty | ✅ #DB.4 |
| `Updates.Redundancy=true` / `Updates.AllowRehash=true` gates redundancy checks and empty-hash rehash | Config-backed update decision | ✅ #DB.6 |
| `Updates.ArchivedRedundancy=false` skips archived redundancy | Config-backed update decision | ✅ #DB.7 |
| `Updates.CleanDeadRefMaxCount=3` deletes orphaned `updates` rows | tracks unmatched applied rows and deletes them when TC's cleanup budget allows | ✅ #DB.8 |
| Detects renamed files (same hash, different name) | ✅ (`updater.rs:139-149`) | ✅ |
| Re-applies changed update file on hash mismatch | ✅ (`updater.rs:174-194`) | ✅ |
| Tracks apply time in `updates.speed` column | ✅ (`updater.rs:166`) | ✅ |
| `mysql_real_escape_string` | `escape_string_like_cpp(value)` | ✅ #DB.14 |
| `LOAD DATA INFILE` support | (none) | ⚠️ not currently used by the Rust updater path |
| Logger `sql.updates` separate channel | tracing default channel | ⚠️ |
| SQL splitter handles `DELIMITER //` / triggers | ❌ — splitter only handles comments + strings; triggers blow up | ⚠️ documented in §11; `populate` shells out to CLI specifically to avoid this |

**The Rust port is interoperable with TC's `updates` table schema**: a server can boot under either binary and the other will respect already-applied updates.

### 13.7 Field accessors

| TC `Field::GetX()` | Rust `SqlResult::read::<X>` | Parity |
|---|---|---|
| `GetUInt8` / `GetInt8` / `GetUInt16` / `GetInt16` / `GetUInt32` / `GetInt32` / `GetUInt64` / `GetInt64` | `read::<u8>` / `read::<i8>` / `read::<u16>` / `read::<i16>` / `read::<u32>` / `read::<i32>` / `read::<u64>` / `read::<i64>` | ✅ |
| `GetFloat` / `GetDouble` | `read::<f32>` / `read::<f64>` | ✅ |
| `GetString` / `GetCString` / `GetStringView` | `read::<String>` (+ `read_string` fallback for `VARBINARY`) | ✅ |
| `GetBinary` (bounded) | `read::<Vec<u8>>` | ✅ |
| `GetBool` (returns `GetUInt8() == 1`) | `read::<bool>` (sqlx) | ✅ |
| `IsNull` | `is_null(col)` | ✅ |
| Per-column `_meta->Type` mismatch warning (debug) | ❌ — sqlx panics on decode failure | ⚠️ #DB.19 |
| Pluggable `BaseDatabaseResultValueConverter` | ❌ — handler does conversion | ⚠️ N/A — no consumers in port yet |

### 13.8 Test coverage

13 unit tests covering parameter binding behaviour and statement table presence. **Zero integration tests** exercising a live MariaDB. The CI pipeline does not start a DB; `cargo test --workspace` passes purely with unit tests. **Recommendation**: add #DB.16 to bring up `mariadb:10.6` in a sidecar and run `populate` + `update` + a transaction round-trip.

### 13.9 Cross-references

- **`worldserver.md` §13.5**: world-server now sizes each single `sqlx` pool from TC's `<Db>Database.WorkerThreads + <Db>Database.SynchThreads` keys — same closure as #WS.21 / #DB.1.
- **`worldserver.md` §13.6**: DB Updater populate/update failures now propagate and abort world-server boot — same closure as #DB.3.
- **`worldserver.md` #WS.6**: DB keep-alive ping implemented for Character/Login/World — same closure as #DB.2/#DB.20.
- **`worldserver.md` #WS.25**: `WarnAboutSyncQueries` debug guard is represented by `wow_database::warn_about_sync_queries_scope_like_cpp` and wired into the current world-server tick paths — #DB.9.
- **`worldserver.md` §13.4**: no global tick → no central DB callback drain. Acceptable for now because all DB ops are awaited per-session.
- **`bnetserver.md`**: also notes missing DB keep-alive (same root cause).

### 13.10 Verdict

**⚠️ partial.** The framework is structurally correct and operates daily. Active-WotLK `CharacterDatabase` statement naming is closed; the remaining DB risk is integration/runtime behaviour, especially #DB.16 live MariaDB populate/update coverage and the known sync/async sub-pool divergence. The framework is suitable for the dev-server workload and small-to-medium private-server deployment. It may still behave differently from TC under multi-thousand-concurrent-login conditions because Rust merges TC's sync/async sub-pools into one `sqlx` pool.

---

*Template version: 1.0 (2026-05-01).*
