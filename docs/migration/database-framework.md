# Migration: database-framework (MySQL connection pool, prepared statements, transactions, updater)

> **C++ canonical path:** `src/server/database/Database/` + `src/server/database/Updater/`
> **Rust target crate(s):** `crates/wow-database/`
> **Layer:** L1 infrastructure (under shared/datastores)
> **Status:** ⚠️ partial (4 typed pools + prepared-statement enums + transactions + DB updater work; missing: per-pool sync/async split, callback chaining type, QueryHolder, libmysql `Library_Init`)
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
| `src/server/database/Database/Transaction.h` | 119 | `TransactionBase`/`Transaction<T>` (collects queries), `TransactionTask` (5-attempt deadlock retry + `_deadlockLock` static mutex), `TransactionCallback` (future + `AfterComplete`) |
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
| `src/server/database/Updater/UpdateFetcher.h` / `.cpp` | 143 + 424 | Walks `sql/updates/<type>/`, hashes files (SHA1), compares against `updates` + `updates_include` tables, computes RELEASED/CUSTOM/ARCHIVED state, archives obsolete |

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
| `SQLQueryHolderBase` / `SQLQueryHolder<T>` | class / template | Vector of (stmt, future-result) pairs. Used by character-load: fire 30 queries in parallel, get one callback when all done |
| `SQLQueryHolderCallback` | class | The "all done" future for a holder |
| `TransactionBase` / `Transaction<T>` | class / template | Collects raw SQL strings + `unique_ptr<PreparedStatementBase>` ; `Append`/`PAppend` |
| `TransactionData` | struct | `std::variant<unique_ptr<PreparedStatementBase>, std::string>` (one row of the transaction) |
| `TransactionTask` | class | Executes a transaction; `TryExecute` retries 5× on deadlock; static `_deadlockLock` mutex |
| `TransactionCallback` | class | Future for an async transaction; `AfterComplete(std::function<void(bool)>)` |
| `DatabaseLoader` | class | Five work queues (`_open`, `_populate`, `_update`, `_prepare`) + `_close` rollback stack; `Load()` runs them; `DATABASE_LOGIN/CHARACTER/WORLD/HOTFIX` flags |
| `DBUpdater<T>` | template class | `Create`, `Populate`, `Update`, `Apply`, `ApplyFile`; per-T template specs map `GetConfigEntry()` / `GetTableName()` / `GetBaseFile()` |
| `DBUpdaterUtil` | class | Locates a `mysql` CLI binary on PATH for `ApplyFile` of large dump files |
| `BaseLocation` | enum | `LOCATION_REPOSITORY` (sql/ in repo) vs `LOCATION_DOWNLOAD` (TDB tarball) |
| `UpdateException` | class | Thrown on update failure |
| `UpdateFetcher` | class (in Updater/) | Walks `sql/updates/<type>/`, hashes, sorts, computes state |

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
| `DatabaseWorkerPool<T>::DelayQueryHolder(holder)` | Enqueue a multi-stmt holder, return `SQLQueryHolderCallback` | `SQLQueryHolderTask` |
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
| `TransactionTask::Execute(conn, trans)` | Wrap in mutex `_deadlockLock`, call `TryExecute` up to 5× | `MySQLConnection::ExecuteTransaction` |
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
| `UPDATE \`updates\` SET \`hash\` = ?, \`speed\` = ? WHERE \`name\` = ?` | Re-apply a CUSTOM update whose hash changed | any |
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
| hotfixes | 327 | 1 placeholder (`_PLACEHOLDER`) — Hotfix prepared statements not yet ported; data flows through DB2 readers + `HotfixBlobCache` instead |

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
| `crates/wow-database/src/statements/character.rs` | `file` | 1 | 284 | `exists_active` | file exists |
| `crates/wow-database/src/statements/world.rs` | `file` | 1 | 371 | `exists_active` | file exists |
| `crates/wow-database/src/statements/hotfix.rs` | `file` | 1 | 25 | `exists_active` | file exists |
| `crates/world-server/src/main.rs` | `file` | 1 | 818 | `exists_active` | file exists |
| `crates/wow-data/src/hotfix_blob_cache.rs` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |
| `crates/wow-database/src` | `module_dir` | 12 | 2262 | `exists_active` | directory exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-database/src/lib.rs` — 58 lines — public re-exports + four `LoginDatabase`/`WorldDatabase`/`CharacterDatabase`/`HotfixDatabase` type aliases
- `crates/wow-database/src/database.rs` — 178 lines — `Database<S: StatementDef>` wrapper around `sqlx::MySqlPool`
- `crates/wow-database/src/error.rs` — 21 lines — `DatabaseError` enum (`Connection`/`Query`/`UnregisteredStatement`)
- `crates/wow-database/src/params.rs` — 208 lines — `PreparedStatement` (SQL + `Vec<SqlParam>`) + `SqlParam` (15 variants matching TC's `PreparedStatementData`)
- `crates/wow-database/src/result.rs` — 198 lines — `SqlResult` (cursor over `Vec<MySqlRow>`) + `SqlFields` (single-row borrowed view)
- `crates/wow-database/src/transaction.rs` — 108 lines — `SqlTransaction` (collects + commits with 5-attempt deadlock retry) + private `bind_param` helper
- `crates/wow-database/src/updater.rs` — 391 lines — `DbUpdater::populate(base_sql)` + `update(source_dir)` + SHA1 hashing + statement splitter that handles line/block comments + `'` / `"` strings with escapes
- `crates/wow-database/src/statements/mod.rs` — 93 lines — `StatementDef` trait + 5 unit tests
- `crates/wow-database/src/statements/login.rs` — 327 lines — `LoginStatements` enum (137 variants) + `sql()` impl
- `crates/wow-database/src/statements/character.rs` — 284 lines — `CharStatements` enum (30 variants — partial)
- `crates/wow-database/src/statements/world.rs` — 371 lines — `WorldStatements` enum (86 variants)
- `crates/wow-database/src/statements/hotfix.rs` — 25 lines — `HotfixStatements::_PLACEHOLDER` only

**What's implemented:**
- Four logical pools, **each a single `sqlx::Pool<MySql>` sized by TC's `<Db>Database.WorkerThreads + <Db>Database.SynchThreads` keys in `world-server`**. Reachable via `LoginDatabase = Database<LoginStatements>`, `WorldDatabase = Database<WorldStatements>`, `CharacterDatabase = Database<CharStatements>`, `HotfixDatabase = Database<HotfixStatements>` aliases.
- Compile-time statement-to-DB binding via the `StatementDef` trait + `PhantomData<S>` on `Database<S>` — using a `WorldStatements` variant on `Database<LoginStatements>` is rejected by `rustc`. Equivalent to TC's `typename T::Statements` typedef but enforced more strongly (TC only enforces it for `GetPreparedStatement`).
- Full set of typed setters: `set_bool/set_i8/set_u8/set_i16/set_u16/set_i32/set_u32/set_i64/set_u64/set_f32/set_f64/set_string/set_bytes/set_null` matching TC's `setUInt8`…`setBinary` 1:1.
- Async query API: `db.query(&stmt).await` → `SqlResult` (cursor with `next_row()`, `read::<T>(col)`, `try_read`, `is_null`, `read_string` fallback for `utf8mb4_bin` columns); `db.execute(&stmt).await` for non-result statements; `db.direct_execute(sql)` / `db.direct_query(sql)` for raw SQL.
- Transactions: `SqlTransaction { statements: Vec<PreparedStatement> }` + `commit(&pool)` opens a real `sqlx::Transaction`, executes each statement, commits or rolls back. On MySQL error 1213 (deadlock) retries up to 5 times — same retry budget as TC's `TransactionTask::TryExecute`.
- `execute_or_append(trans, stmt)` mirroring TC's `ExecuteOrAppend`.
- DB Updater (`DbUpdater`): `populate(base_sql)` invokes the `mysql` CLI to apply a base dump if `information_schema.tables` reports 0 tables for the current DB; `update(source_dir)` reads from `updates_include` table, walks `$source_dir/sql/updates/...`, sha1-hashes each `.sql`, applies via sqlx (statement-by-statement using a hand-rolled splitter that respects `--`/`#` line comments, `/* */` block comments, and `'`/`"` string literals with escapes). Tracks applied files in `updates` table with hash + applied-at + apply-time-ms. Detects renames (same hash, different filename) and replays CUSTOM updates whose hash changed.
- `Updates.AutoSetup` config gate (default `1`) — when off, no auto-update runs.
- All four DBs are wired in `crates/world-server/src/main.rs` lines 170-272: open pool, `DbUpdater::new(pool, …).populate(...).update(...)` for auth + characters; `update(...)` only for world + hotfix (because base SQL for those is the TDB tarball, not in repo).
- 13 unit tests in `params.rs` + `statements/mod.rs` covering setter behaviour, sparse indices, statement-table-name presence, `'?'` placeholder counts.

**What's missing vs C++:**
- **`ConnectionFlags::ASYNC` vs `SYNCH` separation**: TC opens **two sub-pools per logical DB** (configured via `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads`, e.g. `LoginDatabase.WorkerThreads=1`, `LoginDatabase.SynchThreads=1`). RustyCore's single `sqlx::Pool` blends sync and async traffic, but `world-server` now preserves TC's configured total connection budget by opening the pool with `WorkerThreads + SynchThreads`. (Cross-ref: `worldserver.md` §13.5 sub-task #WS.21 / #DB.1.)
- **`KeepAlive()` ping internals**: TC's `World::Update` calls `CharacterDatabase.KeepAlive()`, `LoginDatabase.KeepAlive()`, and `WorldDatabase.KeepAlive()` every `MaxPingTime` minutes (default 30). Rust now spawns the same world-server timer and runs `SELECT 1` against those three logical pools. This is operational parity for idle keep-alive, but not an internal 1:1 clone of TC's per-connection ping loop inside `DatabaseWorkerPool<T>`.
- **`QueryCallback` chaining**: TC's `WithChainingCallback(fn)` lets a callback fire a follow-up query and stay in the same chain (used heavily in account/char load multi-step flows). RustyCore replaces this with `async/await` — the chain becomes a sequence of `await` points in one async fn. **Acceptable divergence** for new code, but porting C++ that uses chaining-callbacks 1:1 needs to flatten into a single `async fn`.
- **`SQLQueryHolder` / `SQLQueryHolderCallback`**: TC's "fire N queries in parallel, callback when all done" type. Used by `CharacterCache::_LoadCharacterFromDB`, login fan-out, achievement load. RustyCore has no equivalent type — current code awaits queries serially or uses `tokio::try_join!` ad-hoc. No fan-out helper or rollback semantics on partial failure.
- **`Field` DBMS-typed getters**: TC's `Field::GetUInt8()` checks `_meta->Type == DatabaseFieldTypes::UInt8` and warns if mismatched (debug build). RustyCore's `SqlResult::read::<T>(col)` uses sqlx's `Decode` impls and panics on type mismatch, with no per-column metadata typed-name available beyond `column_type_name(idx)`.
- **`FieldValueConverter` plug-in**: TC's pluggable column converters (e.g. `enum_string` → int, `time_t` → unix-ts) are absent. Rust port relies on direct sqlx decoding; consumers do their own conversion in handler code.
- **`no QueryProcessor` callback drain**: TC's `WorldSession::Update` polls `_queryProcessor.ProcessReadyCallbacks()` every tick. RustyCore's `WorldSession::process_pending().await` is invoked inside the per-session task loop, but **there is no global tick that drains DB callbacks across all sessions** — each session blocks on its own `.await`. (Cross-ref: `worldserver.md` §13.4 — the missing global tick. Specifically for DB: there is no central place where pending DB futures are reaped, but because each query is awaited directly there's no callback queue to drain in the first place.)
- **`PreparedStatement<T>` per-statement compile-time type** (the type tag T): RustyCore's `PreparedStatement` is **not parameterized by `S`** — it's a plain struct with a `&'static str sql`. The DB-binding constraint comes from `Database<S>::prepare(stmt: S)` ingesting an `S: StatementDef` and emitting a non-typed `PreparedStatement`. So once the `PreparedStatement` exists, you could in principle pass it to a different `Database<…>` (no compile-time prevention there) — TC catches this at the `pool.GetPreparedStatement(idx)` site. In practice this isn't exploitable because the `&'static str` is bound to one DB by construction.
- **`MySQLConnection::PrepareStatements()` actually prepares MYSQL_STMT objects on the server**: TC issues `mysql_stmt_prepare` for every `(connection × statement)` pair at pool `Open`; subsequent executes reuse the prepared handle (fast). sqlx caches prepared statements per-connection internally on first use, so the warm-up effect is amortized rather than upfront — measurable difference on the very first call to each statement, none afterward.
- **Hotfix prepared statements**: 1 placeholder vs TC's 327. Hotfix data flow in RustyCore goes through DB2 readers + `HotfixBlobCache` (`crates/wow-data/src/hotfix_blob_cache.rs`), not through prepared statements against `hotfixes.*` tables. This is a deliberate divergence — the C++ hotfix system uses MySQL-resident DB2 deltas that the world server merges with disk DB2 at load; RustyCore so far precomputes the merged blob.
- **Character prepared statements**: 30 vs TC's 523. Most character persistence flows are still missing port (cf. `INVENTORY.md` lookup). The 30 ported cover: char enum, create, delete, login, save (basic fields), max-guid lookup, online flag, name lookup, position update.
- **`DBUpdater::Create`**: not implemented. `populate` requires the database to already exist (the connection string would fail otherwise, before sqlx ever connects). TC's `DBUpdater::Create` runs `mysql -e "CREATE DATABASE …"` when the connection itself fails because the DB doesn't exist. Operator must pre-create databases.
- **`Updates.CleanDeadRefMaxCount`** config key: read but not acted on yet. RustyCore still never deletes orphaned `updates` rows, so a removed file leaves a stale row. `Updates.Redundancy`, `Updates.AllowRehash`, and `Updates.ArchivedRedundancy` are now honored in the update decision path.
- **`updates_include` defaults**: TC bootstraps `updates_include` rows on first `Populate` (e.g. `('$/sql/updates/auth', 'RELEASED')`). RustyCore creates the table empty and warns "no updates_include entries" if operator hasn't populated it. **First-boot UX gap**: a fresh install will skip applying any updates until the operator manually inserts rows.
- **`UpdateFetcher` "missing files" detection**: TC removes `updates` rows whose `.sql` file is no longer present (with `Updates.CleanDeadRefMaxCount` budget). RustyCore never deletes from `updates`, so a removed file leaves a stale row.
- **`DBUpdater::Populate` / `Update` failures are fatal** in TC (`return false` from `StartDB` aborts boot). RustyCore now propagates populate/update errors from world-server startup with DB-specific context, so a broken or empty DB fails fast instead of booting into later missing-table errors. (Cross-ref: `worldserver.md` §13.6.)
- **`MySQLConnection::EscapeString` / `mysql_real_escape_string`**: not exposed. sqlx's parameter binding handles escaping, but the "raw escape this user-supplied string for use in a non-prepared SQL fragment" path (used by some chat commands and mass updates in TC) is unavailable.
- **`DatabaseWorkerPool::WarnAboutSyncQueries`**: no equivalent. Detecting a synchronous query issued from inside a tick (vs an async one) requires some `tokio::task::block_in_place` audit hook that doesn't exist. (Cross-ref: `worldserver.md` sub-task #WS.25.)
- **`DatabaseWorkerPool::QueueSize`**: no equivalent — no queue exists; sqlx executes immediately on a free pool conn or `await`s for one. The metrics surface that consumed `QueueSize` (`db_queue_*` in TC's metrics) is therefore also gone.
- **`mysql_library_init` / `mysql_library_end`**: irrelevant — sqlx links against rust-mysql-async/`mysql_async` or against libmysql via FFI, init is automatic.
- **OpenSSL / TLS to MariaDB**: TC's `MYSQL_OPT_SSL_CA` is not configured in RustyCore; sqlx defaults to `?ssl-mode=PREFERRED` meaning encryption is only negotiated if the server supports it without cert validation.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- The single 10-connection pool is shared across all 4 logical DBs in *some* views of the code? **(Audit: false — each DB has its own pool, see §13.2.)**
- Transaction deadlock retries deplete the pool's connections quickly under contention. With 5 retries × N concurrent transactions, all 10 conns can be tied up retrying. TC's `_deadlockLock` is a single static mutex, serializing retries; RustyCore retries concurrently.
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
- [ ] **#DATABASE_FRAMEWORK.WBS.006** Cerrar la migracion auditada de `database/Database/DatabaseLoader.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseLoader.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.007** Cerrar la migracion auditada de `database/Database/DatabaseLoader.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/DatabaseLoader.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
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
- [ ] **#DATABASE_FRAMEWORK.WBS.035** Cerrar la migracion auditada de `database/Database/QueryHolder.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryHolder.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.036** Cerrar la migracion auditada de `database/Database/QueryHolder.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryHolder.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.037** Cerrar la migracion auditada de `database/Database/QueryResult.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryResult.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.038** Cerrar la migracion auditada de `database/Database/QueryResult.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/QueryResult.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.039** Cerrar la migracion auditada de `database/Database/Transaction.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Transaction.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.040** Cerrar la migracion auditada de `database/Database/Transaction.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Transaction.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.041** Cerrar la migracion auditada de `database/Updater/DBUpdater.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/DBUpdater.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.042** Cerrar la migracion auditada de `database/Updater/DBUpdater.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/DBUpdater.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.043** Cerrar la migracion auditada de `database/Updater/UpdateFetcher.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/UpdateFetcher.cpp`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#DATABASE_FRAMEWORK.WBS.044** Cerrar la migracion auditada de `database/Updater/UpdateFetcher.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/database/Updater/UpdateFetcher.h`
  Rust target: `crates/wow-database`, `crates/wow-database/src`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** <1h, **M** 1-4h, **H** 4-12h, **XL** >12h.

- [x] **#DB.1** Use TC database thread-count config for world-server pools: read `<Db>Database.WorkerThreads` + `<Db>Database.SynchThreads` for Login/Character/World/Hotfix and pass their sum to `Database::open_with_pool_size`. Defaults follow TC's loader default of `1 + 1`; invalid values fall back to `1` per side with a warning. (L) — **same as `worldserver.md` #WS.21**
- [x] **#DB.2** Implement DB keep-alive: spawn one shared world-server task using `MaxPingTime` (default 30) that runs `SELECT 1` against Character/Login/World, matching TC's `World::Update` keep-alive pool set. Hotfix is intentionally excluded because TC does not call `HotfixDatabase.KeepAlive()` here. (L) — **same as #WS.6**
- [x] **#DB.3** Make `DbUpdater::populate` / `update` failure fatal when `Updates.AutoSetup=1`: return error from `world-server/main.rs` instead of `tracing::warn!`. Boot now aborts if base SQL is missing, the DB is empty + populate fails, or an enabled update step fails. (L)
- [ ] **#DB.4** Bootstrap `updates_include` rows on first `populate`: insert `('$/sql/updates/auth', 'RELEASED')` etc. so a fresh install actually applies updates without operator intervention. (M)
- [ ] **#DB.5** Implement `DBUpdater::Create` equivalent: detect "Unknown database" sqlx error on initial connect, run `CREATE DATABASE IF NOT EXISTS` via a connection string with no DB name, retry. (M)
- [x] **#DB.6** Implement `Updates.Redundancy` (default `true`) and `Updates.AllowRehash` (default `true`): when redundancy is disabled, skip already-applied files; when rehash is enabled and the stored hash is empty, update the hash without reapplying; when hash matches but state changed, update state only. (L)
- [x] **#DB.7** Implement `Updates.ArchivedRedundancy` (default `false`): when false, skip redundancy checks for files that are archived both in the DB and the available update path; when true, changed archived updates can be reapplied. (L)
- [ ] **#DB.8** Implement `Updates.CleanDeadRefMaxCount`: delete `updates` rows whose file no longer exists, up to N per run. (M)
- [ ] **#DB.9** Add `WarnAboutSyncQueries`-style guard: track a `tokio::task::TaskLocal<bool>` "in tick" flag; warn (or panic in debug) when a sync query runs inside it. Same as `worldserver.md` #WS.25. (M)
- [ ] **#DB.10** Add a `QueryHolder`-style helper: a struct that batches N `PreparedStatement`s, executes them concurrently with `tokio::try_join_all`, returns a `Vec<SqlResult>` indexed by slot. Used by character load. (M)
- [ ] **#DB.11** Port the remaining ~493 character prepared statements (cross-ref `INVENTORY.md`, `characters.md`). (XL — split per-domain: inventory, achievements, mails, guilds, BG/arena state, etc.)
- [ ] **#DB.12** Decide hotfix prepared-statement strategy: either port all 327 from `HotfixDatabase.cpp` for runtime hotfix mutations, OR formalize the "merged blob at boot" approach and document that hotfix DB is read-once-at-startup. (H if port; L if formalize)
- [ ] **#DB.13** Implement TLS connection options: `<Db>DatabaseInfo.SSLMode` config key passed through to sqlx connection options (`ssl-mode=REQUIRED` / `VERIFY_CA` etc.). (M)
- [ ] **#DB.14** Implement `EscapeString` for raw-SQL-fragment use cases: expose `MySqlPool::escape` equivalent or document that `direct_execute` consumers must use bound parameters. (L)
- [ ] **#DB.15** Add per-connection-attempt logging on populate / update error so operator sees which file failed. Currently the error message includes the path but not stderr from the `mysql` CLI in `populate` failure case. (L)
- [ ] **#DB.16** Add integration test harness: spin up an embedded MariaDB (or Docker mariadb:10.6 in CI) and run `populate` + `update` against it; verify updates table population. (H)
- [ ] **#DB.17** Audit deadlock-retry concurrency: replicate TC's `_deadlockLock` static mutex (or document why concurrent retries are fine). (L)
- [ ] **#DB.18** Add a `DatabaseError::TableMissing` variant so callers can distinguish "DB not populated" from "query syntax error". (L)
- [ ] **#DB.19** Add a `Field`-style typed accessor with metadata: `SqlResult::read_typed::<u8>(col)` that checks `column_type_name == "TINYINT UNSIGNED"` before decoding. Optional, debug-only. (M)
- [x] **#DB.20** Implement async `KeepAlive` from a free-standing `tokio::spawn` while the global world tick is still pending. Covered by #DB.2 / #WS.6; if a future global tick centralizes this, keep the same Character/Login/World-only scope.
- [ ] **#DB.21** Add a `DatabaseLoader`-style sequencer: a struct that registers (open / populate / update / close) closures per DB and rolls back on failure. Currently `world-server/main.rs` does this inline 4× with no rollback. (M)

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
- [ ] Test: `SqlTransaction::commit` retries on MySQL error 1213 deadlock up to 5 times then surfaces the error (use a mock `MySqlPool` or a deliberate deadlock with `LOCK TABLES`).
- [ ] Test: `DbUpdater::populate` is a no-op on a non-empty DB (returns `Ok(false)`).
- [ ] Test: `DbUpdater::populate` invokes `mysql` CLI exactly once and the database has the expected tables afterward.
- [ ] Test: `DbUpdater::update` applies new files in lexicographic order, records SHA1, and skips already-applied entries.
- [x] Test: update decision honors `Updates.Redundancy=false`, `Updates.AllowRehash`, and changed hashes.
- [x] Test: update decision does **not** re-apply an ARCHIVED-state file when `Updates.ArchivedRedundancy=false`, and does when enabled.
- [ ] Test: `DbUpdater::update` detects a renamed file (same hash, different filename) and updates the `name` column instead of re-applying.
- [ ] Test: `split_sql` correctly handles `--` line comments, `#` comments, `/* */` blocks, `'\''` escapes, and `\"` escapes (existing behaviour; pin in tests).
- [ ] Test: `SqlResult::read_string` on a `VARBINARY` column returns the UTF-8 string (not panic).
- [ ] Test: `PreparedStatement` indexed setters with sparse indices fill intermediates with `SqlParam::Null` (already covered).
- [x] Test: keep-alive config/scope/SQL mirrors C++ (`MaxPingTime`, Character/Login/World only, `SELECT 1`). Runtime timer firing remains a service/integration concern rather than a paused Tokio unit fake.
- [ ] Test: `Updates.AutoSetup=0` skips both `populate` and `update`; the `updates` table is not created.
- [x] Test: startup propagates DB updater failures with DB-specific context when `Updates.AutoSetup=1` (#DB.3).
- [ ] Test: `Database<LoginStatements>::prepare(LoginStatements::SEL_REALMLIST)` and `Database<WorldStatements>::prepare(WorldStatements::DEL_LINKED_RESPAWN)` produce different `&'static str` SQL — and using `LoginStatements::SEL_REALMLIST` on a `Database<WorldStatements>` is a compile error (compile-fail test via `trybuild`).
- [ ] Test: `_PLACEHOLDER` HotfixStatement returns empty string; `Database::<HotfixStatements>::query` on it returns `DatabaseError::UnregisteredStatement`.

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 44 files / 10590 lines; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp` | `crates/wow-database/` \| ⚠️ partial (4 typed pools + prepared-statement enums + transactions + DB updater work; missing: per-pool sync/async split, callback chaining type, QueryHolder, libmysql `Library_Init`) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#DATABASE_FRAMEWORK.DIV.001` | `crates/wow-data/src/hotfix_blob_cache.rs` (`missing_declared_path`, 0 Rust lines) | 44 C++ files / 10590 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.h`, `/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/CharacterDatabase.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **TC ships two pools per logical DB**; merging them into one `sqlx::Pool` is a deliberate simplification, but at high login churn you can saturate the single pool while a transaction is mid-retry. Consider tuning `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads` upward; Rust will use their sum as the single pool size.
- **`PreparedStatement<T>` in C++ is type-tagged**; in Rust the tagging is at `Database<S>::prepare(stmt)` ingestion time only. Don't try to add `PhantomData<S>` to `PreparedStatement` itself — the existing test design (compile-fail) covers the misuse case at the right place.
- **sqlx prepared-statement caching**: sqlx caches prepared `MYSQL_STMT` handles per-connection automatically. First call to a statement on a fresh connection prepares; subsequent calls reuse. This is why we don't need TC's `MySQLConnection::PrepareStatements()` warm-up — but it does mean cold pool acquisitions pay one round-trip extra. Don't `pool.close()` and re-`open()` to "reload" prepared statements; just restart the binary.
- **`DBUpdater::Update` SQL splitter cannot handle DELIMITER**: TrinityCore's TDB world dump uses `DELIMITER //` for trigger definitions. RustyCore's `populate` shells out to `mysql` CLI specifically for this reason; `update` (which never sees triggers) is statement-by-statement. If a future update adds a trigger or stored procedure, **the splitter will fail** — extend it or fall back to the CLI for that file.
- **`Updates.AutoSetup=1` default**: a fresh checkout will try to reach a `mysql` CLI binary on `PATH` to run `populate`. If `mysql` is not installed (e.g. a dev VM with only `mariadb-client` symlinked differently), `populate` fails with an opaque `command not found`. Either install `mysql` or pre-populate manually.
- **`ConnectionFlags::SYNCH` queries in TC are guaranteed to run on a sync sub-pool** so that nothing outside the world thread blocks them. RustyCore loses this guarantee — **a sync query issued from inside an async tick can still starve when the merged pool is saturated by async work**. Watch out under heavy load; tune up `<Db>Database.WorkerThreads` / `<Db>Database.SynchThreads` before reaching for a redesign.
- **`updates` table schema**: TC's columns are `(name, hash, state, timestamp, speed)` with `state` as an enum. The Rust port creates the same schema with `IF NOT EXISTS` so it's interoperable — you can run TC's `worldserver` against the same DB and Rust will respect its `updates` rows. **Compatibility tested 2026-04: ✅.**
- **Deadlock retry is single-mutex in TC** (`TransactionTask::_deadlockLock`); RustyCore retries concurrently. In adversarial workloads (e.g. many parallel character-save txns hitting the same row) this can amplify deadlocks rather than suppress them. **#DB.17.**
- **Boot order matters**: `Database::open` requires the database to exist (sqlx fails on `Unknown database`). TC's `DBUpdater::Create` uses the `mysql` CLI to `CREATE DATABASE` against the server (no DB context) — RustyCore doesn't, so an operator must pre-create the four databases. Document this in `README.md` setup section.
- **`max_allowed_packet`**: `populate` passes `--max-allowed-packet=1073741824` (1 GiB) to the CLI. The sqlx pool inherits the server-side default (often 16 MiB). Large blob updates via `update()` may fail with `Packet too large`. Either bump server-side `max_allowed_packet` or chunk the migration.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `MySQLConnectionInfo` (parsed `host;port;user;pass;db;ssl`) | `wow_database::database::build_connection_string(host, port, user, pass, db) -> String` | Returns a `mysql://…` URL; sqlx parses |
| `enum ConnectionFlags { ASYNC, SYNCH, BOTH }` | (none) | Single sqlx pool, no sub-pool flag |
| `class MySQLConnection` | `sqlx::pool::PoolConnection<MySql>` (acquired ad-hoc) | No standalone "connection" struct |
| `class LoginDatabaseConnection` (and 3 siblings) | (subsumed) | Replaced by `Database<LoginStatements>` (and 3 siblings) — the `<S>` type tag plays the same role as inheritance |
| `class DatabaseWorkerPool<T>` | `wow_database::Database<S: StatementDef>` (= `sqlx::Pool<MySql>` + `PhantomData<S>`) | Single pool, no sync/async split |
| `template <> class PreparedStatement<T>` | `wow_database::PreparedStatement` (no type tag) | Tagging happens at `Database<S>::prepare(stmt: S)` |
| `class PreparedStatementBase` | (merged into `PreparedStatement`) | No virtual base needed; setters live directly on the only concrete type |
| `struct PreparedStatementData (std::variant<…>)` | `enum SqlParam { Null, Bool, I8…U64, F32, F64, String, Bytes }` | 1:1 type list |
| `pStmt->setUInt8(idx, val)` | `stmt.set_u8(idx, val)` | And siblings (`set_i8` / `set_string` / `set_bytes` / `set_null`) |
| `class Field` | (merged into `SqlResult`/`SqlFields`) | Per-column getter is `result.read::<T>(col)` |
| `Field::GetUInt8()` | `result.read::<u8>(col)` | sqlx handles MySQL→Rust decoding |
| `Field::GetCString()` | (use `read::<String>` then `.as_str()`) | No raw `*const c_char` lifetime to manage |
| `Field::IsNull()` | `result.is_null(col)` | — |
| `enum DatabaseFieldTypes` | (none) | sqlx exposes `column.type_info().name()` (string) instead |
| `class ResultSet` (text protocol) | `SqlResult` returned by `direct_query(sql)` | Text vs binary protocol indistinguishable in sqlx |
| `class PreparedResultSet` (binary protocol) | `SqlResult` returned by `query(&stmt)` | Same type as text; sqlx selects protocol |
| `class QueryCallback` | `async fn` returning `Result<SqlResult, DatabaseError>` | Chaining = sequential `await` |
| `qc.WithChainingCallback(fn)` | `let r1 = db.query(s1).await?; let r2 = db.query(s2_built_from(r1)).await?;` | Flatten chains into linear async code |
| `class SQLQueryHolder<T>` | (none) — use `tokio::try_join!(q1, q2, q3)` ad-hoc | TODO: build a typed helper (#DB.10) |
| `SQLQueryHolderCallback::AfterComplete(fn)` | `let (r1, r2, r3) = try_join!(...)?; fn(r1, r2, r3)` | — |
| `class TransactionBase` / `class Transaction<T>` | `wow_database::SqlTransaction` (untyped) | No `<T>` tag — txns are not type-bound to a DB; runtime check via the pool they're committed against |
| `trans->Append(sql)` / `trans->Append(stmt)` | `tx.append(stmt)` (no raw-SQL `Append` — wrap raw SQL as a `PreparedStatement::new`) | — |
| `pool.CommitTransaction(trans)` | `db.commit_transaction(tx).await?` | Async, with 5-attempt deadlock retry |
| `class TransactionCallback` (`AfterComplete`) | `db.commit_transaction(tx).await.map(|_| post_action())` | — |
| `pool.Execute(sql)` (async fire-and-forget) | `tokio::spawn(async move { db.direct_execute(sql).await })` | Awaiting blocks the caller; spawn for fire-and-forget |
| `pool.DirectExecute(sql)` (sync) | `db.direct_execute(sql).await` | All Rust DB ops are async; "direct" = "no transaction" |
| `pool.PExecute("UPDATE foo SET x = {}", val)` | `db.direct_execute(&format!("UPDATE foo SET x = {val}")).await` | No native string-format wrapper |
| `pool.AsyncQuery(stmt)` | `db.query(&stmt).await` | sqlx is always-async |
| `pool.DelayQueryHolder(holder)` | (TODO #DB.10) | — |
| `pool.GetPreparedStatement(idx)` | `db.prepare(LoginStatements::FOO)` | — |
| `pool.EscapeString(str)` | (none) — use bound parameters | TODO #DB.14 |
| `pool.KeepAlive()` | `Database::keep_alive_like_cpp()` + world-server keep-alive task | `SELECT 1` every `MaxPingTime` for Character/Login/World |
| `pool.WarnAboutSyncQueries(true)` | (TODO #DB.9 / #WS.25) | — |
| `class DatabaseLoader` (5-queue sequencer + close stack) | (inline in `world-server/src/main.rs`) | TODO #DB.21 — formalize into a sequencer struct |
| `class DBUpdater<T>` | `wow_database::updater::DbUpdater` (single non-generic struct) | The `<T>` tag is replaced by the per-DB connection params passed to `DbUpdater::new` |
| `DBUpdater::Create(pool)` | (TODO #DB.5) | — |
| `DBUpdater::Populate(pool)` | `updater.populate(base_sql).await` | Shells to `mysql` CLI |
| `DBUpdater::Update(pool)` | `updater.update(source_dir).await` | Walks `sql/updates/`, hashes, applies via sqlx |
| `class UpdateFetcher` | (merged into `DbUpdater::update`) | — |
| `class UpdateException` | `anyhow::Error` (via `Result<()>`) | — |
| `class MySQLPreparedStatement` | (none — sqlx caches `MYSQL_STMT` per connection internally) | — |
| `mysql_library_init` / `mysql_library_end` | (none) | sqlx handles |
| `MYSQL_OPT_SSL_CA` etc. | sqlx connection-string `?ssl-mode=REQUIRED&ssl-ca=…` | TODO #DB.13 |
| `mysql_real_escape_string` | (none) — use bound params | TODO #DB.14 |
| `Trinity::Asio::IoContext` (per-pool) | implicit Tokio runtime | — |

---

## 13. Audit (2026-05-01)

**Audited:**
- C++: `/home/server/woltk-trinity-legacy/src/server/database/Database/MySQLConnection.h` (116 lines), `DatabaseWorkerPool.h` (243 lines), `PreparedStatement.h` (125 lines), `Field.h` (142 lines), `QueryCallback.h` (67 lines) + `.cpp` (221 lines), `Transaction.h` (119 lines), `QueryResult.h` (85 lines), `QueryHolder.h` (81 lines), `DatabaseLoader.h` (78 lines), `Implementation/LoginDatabase.h` (195 lines) + `.cpp` (lines 1-80), `Implementation/CharacterDatabase.h`/`HotfixDatabase.h`/`WorldDatabase.h` (counted statements). `Updater/DBUpdater.h` (96 lines) + `.cpp` (1-120). Total ~2200 lines reviewed in detail; ~10 600 lines surveyed.
- Rust: `/home/server/rustycore/crates/wow-database/src/lib.rs` (58 lines), `database.rs` (178 lines), `params.rs` (208 lines), `result.rs` (198 lines), `transaction.rs` (108 lines), `updater.rs` (391 lines), `statements/mod.rs` (93 lines), `statements/login.rs` (327 lines), `statements/hotfix.rs` (25 lines). Wiring point: `crates/world-server/src/main.rs` lines 170-272.

### 13.1 Audit summary

The Rust port covers the **shape** of TC's database framework correctly: four typed pools, prepared statements indexed by DB-specific enums, transactions with deadlock retry, and an auto-applier for `sql/updates/`. The compile-time DB-to-statement binding is actually *stricter* in Rust than in TC because of how `Database<S>::prepare(s: S)` is gated.

The gaps fall in three buckets:

1. **Pool topology** (Medium): Rust still merges TC's `Sync`/`Async` sub-pools into one `sqlx` pool, but world-server now sizes that pool from TC's `WorkerThreads + SynchThreads` config and runs the `MaxPingTime` keep-alive task. Cross-references the worldserver audit (`worldserver.md` §13.5 / §13.6).
2. **Statement coverage** (High): login is 137/137 (✅), world is 86/56 (✅, Rust adds extras), characters is 30/523 (❌, ~6% ported), hotfixes is 0/327 (❌, **deliberate divergence** — replaced by DB2-blob cache).
3. **Updater operator UX** (Medium): no `Create` step; `updates_include` not bootstrapped on first run; missing `Updates.CleanDeadRefMaxCount` cleanup.

The `QueryCallback`-chain pattern is **acceptably absent** because Rust's `async/await` covers the same need more directly. The `SQLQueryHolder` fan-out, however, has no Rust equivalent and is needed for character load (#DB.10).

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
| hotfix (`HotfixStatements`) | 327 | 1 (`_PLACEHOLDER`) | ❌ 0% — deliberate divergence (cf. §8) |

**Registry pattern**: TC binds `enum Statements : uint32` → SQL string in `<Db>DatabaseConnection::DoPrepareStatements()` via `PrepareStatement(idx, sql, flags)` calls (`LoginDatabase.cpp:26` onward). Rust binds via the `StatementDef` trait `fn sql(self) -> &'static str` per enum, with the SQL inline in a `match` arm (`statements/login.rs::sql_for`). Both are static, both are compile-time. **Functional parity for the patterns that are ported.** The mechanism is different but the constraint (one enum value → one SQL string per DB, type-checked) holds.

CLAUDE.md mentions a "prepared statement registry" as completed. Audit confirms: the **infrastructure** is complete; the **statement set** is complete only for login + world. Character and hotfix coverage is partial-or-stub. **Update CLAUDE.md or treat as known partial.**

### 13.4 Transactions

| TC | Rust | Parity |
|---|---|---|
| `TransactionBase::Append(sql)` / `Append(PreparedStatementBase*)` | `SqlTransaction::append(stmt)` (no raw-SQL append; wrap as `PreparedStatement::new`) | ⚠️ partial |
| `pool.BeginTransaction()` returns `SQLTransaction<T>` (typed) | `SqlTransaction::new()` (untyped) — runtime-bound to whichever pool you commit against | ⚠️ runtime-typed |
| `pool.CommitTransaction(trans)` (async) | `db.commit_transaction(tx).await` | ✅ |
| `pool.AsyncCommitTransaction(trans)` returns `TransactionCallback` | `commit_transaction` is already async, no separate callback type | ✅ acceptable divergence |
| `pool.DirectCommitTransaction(trans)` (sync) | (none) — same as commit | ✅ irrelevant in async land |
| `TransactionTask::TryExecute` retries 5× on deadlock | `SqlTransaction::commit` retries 5× on MySQL error 1213 (`transaction.rs:48-65`) | ✅ |
| `TransactionTask::_deadlockLock` static mutex serializes retries | (none) — concurrent retries | ⚠️ #DB.17 |
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
| `class SQLQueryHolder<T>` (fan-out, all-done callback) | (none) — ad-hoc `tokio::try_join!` | ❌ #DB.10 |

### 13.6 DB Updater

| TC `DBUpdater<T>` step | Rust `DbUpdater` | Parity |
|---|---|---|
| `Create(pool)` — `mysql -e "CREATE DATABASE …"` if DB missing | (none) | ❌ #DB.5 |
| `Populate(pool)` — if 0 tables, apply `GetBaseFile()` via `mysql` CLI | `populate(base_sql)` — if `information_schema.tables` count = 0, shell `mysql … -e "SOURCE …;"` | ✅ |
| Failure to populate/update → `false` from `StartDB` → boot abort | startup propagates `populate` / `update` errors with DB-specific context | ✅ #DB.3 |
| `Update(pool)` — walk `updates_include`, sha1 each `.sql`, apply pending | `update(source_dir)` — walks paths from DB-resident `updates_include` table, sha1, applies via sqlx statement-by-statement | ✅ |
| Bootstrap `updates_include` rows on first run | (none) — table created empty; no defaults | ❌ #DB.4 |
| `Updates.Redundancy=true` / `Updates.AllowRehash=true` gates redundancy checks and empty-hash rehash | Config-backed update decision | ✅ #DB.6 |
| `Updates.ArchivedRedundancy=false` skips archived redundancy | Config-backed update decision | ✅ #DB.7 |
| `Updates.CleanDeadRefMaxCount=3` deletes orphaned `updates` rows | (none) | ❌ #DB.8 |
| Detects renamed files (same hash, different name) | ✅ (`updater.rs:139-149`) | ✅ |
| Re-applies CUSTOM-state file on hash change | ✅ (`updater.rs:174-194`) | ✅ |
| Tracks apply time in `updates.speed` column | ✅ (`updater.rs:166`) | ✅ |
| `mysql_real_escape_string` / `LOAD DATA INFILE` support | (none) | ❌ #DB.14 |
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
- **`worldserver.md` #WS.25**: `WarnAboutSyncQueries` debug guard missing — #DB.9.
- **`worldserver.md` §13.4**: no global tick → no central DB callback drain. Acceptable for now because all DB ops are awaited per-session.
- **`bnetserver.md`**: also notes missing DB keep-alive (same root cause).

### 13.10 Verdict

**⚠️ partial.** The framework is structurally correct and operates daily. The deferred items (#DB.10 query holder, #DB.11 char statements, and remaining updater/load-tolerance details) are quality-of-life and load-tolerance improvements rather than missing capability. The framework is suitable for the dev-server workload and small-to-medium private-server deployment. It may still behave differently from TC under multi-thousand-concurrent-login conditions because Rust merges TC's sync/async sub-pools into one `sqlx` pool, and the `_attic/` character migration cannot finish until #DB.11 unblocks the prepared-statement set.

---

*Template version: 1.0 (2026-05-01).*
