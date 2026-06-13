# Migration: Common (foundation primitives)

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/common/` (excluding `Collision/`, covered separately)
> **Rust target crate(s):** `crates/wow-core/`, `crates/wow-config/`, `crates/wow-logging/`, `crates/wow-collections/`, scattered helpers in `wow-network/`, `wow-crypto/`, `wow-database/`
> **Layer:** L0 (foundation)
> **Status:** ⚠️ partial — most primitives present in idiomatic Rust form; several explicit C++ subsystems (Metric/InfluxDB, Errors-with-stack, full Logger framework, SmartEnum/EventMap/TaskScheduler) have no Rust equivalent; IPLocation store exists but is not wired through every C++ caller yet
> **Audited vs C++:** ⚠️ partial — SRP6 string normalisation now uses the C++ Basic-Latin-only helper; remaining gaps listed in §9/§13
> **Last updated:** 2026-05-01

---

## 1. Purpose

`src/common/` is TrinityCore's L0 foundation: every other layer (`shared/`, `server/game/`, `bnetserver/`, `worldserver/`) depends on it. It collects the cross-cutting primitives that have nothing to do with WoW gameplay: an Asio I/O reactor, a Boost-ptree-backed `.conf` parser, a hand-rolled logger with appender hierarchy, lock-free queues, the encoding/hashing/random utility belt, an ASCII-IPv4-range `IPLocation` lookup, and an InfluxDB metrics push pipeline. The migration here is the canonical example of "we don't port C++ to Rust — we replace each subsystem with the idiomatic Rust crate that already does the same thing": `tokio` for Asio, `tracing` for `Log`, `parking_lot` + `flume` + `dashmap` for `LockedQueue` / `MPSCQueue` / `ProducerConsumerQueue`, `chrono` / `std::time` for `Time/Timer.h`, `rand` for `Random`, `hex` / `base64` / `data-encoding` for `Encoding/`. The work is therefore mostly **mapping table + audit**, not reimplementation.

---

## 2. C++ canonical files

All paths relative to `/home/server/woltk-trinity-legacy/`.

| File | Lines | Purpose |
|---|---|---|
| `src/common/Asio/IoContext.h` | 75 | RAII wrapper over `boost::asio::io_context` with `Stop()` / `Run()` |
| `src/common/Asio/DeadlineTimer.h` | 35 | Typedef'd `boost::asio::basic_waitable_timer<steady_clock>` |
| `src/common/Asio/Strand.h` | 45 | `io_context::strand` typedef + helper |
| `src/common/Asio/IpAddress.h` | 35 | `make_address_v4` / `address_to_uint` helpers |
| `src/common/Asio/IpNetwork.h` + `.cpp` | 39+284 | `IsInLocalNetwork`, `IsInNetwork`, `SelectAddressForClient`, `ScanLocalNetworks` (RFC1918 + interface enumeration) |
| `src/common/Asio/Resolver.h` | 55 | DNS resolver stub on `tcp::resolver` |
| `src/common/Asio/AsioHacksFwd.h` | 68 | Boost forward-declarations to keep `Asio.hpp` out of headers |
| `src/common/Configuration/Config.h` + `.cpp` | 63 + 374 | `ConfigMgr` singleton; `boost::property_tree::ini_parser`; `OverrideWithEnvVariablesIfAny` (snake-case env mapping) |
| `src/common/Configuration/BuiltInConfig.h` + `.cpp` | 43+52 | Compile-time built-in defaults |
| `src/common/Containers/FlatSet.h` | 106 | `Trinity::Containers::FlatSet<K>` — sorted `std::vector` with binary-search lookup |
| `src/common/Containers/Utilities/` | (subdir) | `ArrayWrapper.h` etc. helpers |
| `src/common/Cryptography/CryptoConstants.h` | 37 | MD5/SHA1/SHA256/SHA512 digest sizes |
| `src/common/Cryptography/OpenSSLCrypto.h` + `.cpp` | 36 + ~50 | `threadsInit/Cleanup` (legacy OpenSSL <1.1 mutex callbacks) |
| `src/common/Cryptography/Argon2.h` + `.cpp` | 44 + ~120 | `Argon2id` PBKDF wrapper (BNet v2 2FA only) |
| `src/common/Debugging/Errors.h` + `.cpp` | 86+160 | `WPAssert`, `ABORT`, `ASSERT_NOTNULL`, `Trinity::Fatal` (logs+breakpoint+exit) |
| `src/common/Debugging/WheatyExceptionReport.{h,cpp}` | (Win-only) | MS minidump-style crash dump, never used on Linux |
| `src/common/Encoding/BaseEncoding.h` | 161 | Generic `GenericBaseEncoding<Encoding>` template (any bits-per-char) |
| `src/common/Encoding/Base32.{h,cpp}` | 39 + 55 | Base32 (TOTP secret encoding) |
| `src/common/Encoding/Base64.{h,cpp}` | 39 + 57 | Base64 (BNet REST tokens, web auth tickets) |
| `src/common/IPLocation/IPLocation.{h,cpp}` | 53+131 | Reads CSV "ip_from,ip_to,country_code,country_name" file into sorted `vector`, `upper_bound` lookup |
| `src/common/Logging/Log.{h,cpp}` | 171+408 | `Log` singleton: filter table → `Logger` → `Appender[]` (Console / File / DB), async via `IoContext`, %Y/%m/%d rotation |
| `src/common/Logging/Appender.{h,cpp}` | 61+92 | Abstract appender base |
| `src/common/Logging/AppenderConsole.{h,cpp}` | 62+211 | ANSI-color console output, per-level color |
| `src/common/Logging/AppenderFile.{h,cpp}` | 46+127 | Rotating file appender, %s timestamp substitution |
| `src/common/Logging/Logger.{h,cpp}` | 48+60 | Per-filter log level + appender list |
| `src/common/Logging/LogMessage.{h,cpp}` | 51+42 | Wire format for one log record (filter, level, time, text) |
| `src/common/Logging/LogOperation.{h,cpp}` | 41+34 | Async `boost::asio::post(strand, op)` wrapper |
| `src/common/Logging/LogCommon.h` | 59 | `LogLevel` / `AppenderType` / `AppenderFlags` enums |
| `src/common/Metric/Metric.{h,cpp}` | 247+330 | InfluxDB UDP/TCP push: `MPSCQueue<MetricData>`, batch timer, hostname tag, line-protocol formatter |
| `src/common/Platform/ServiceWin32.{h,cpp}` | 27+261 | Windows service (Linux build defines them out) |
| `src/common/Hacks/boost_program_options_with_filesystem_path.h` | 46 | Boost p.options ↔ `boost::filesystem::path` adapter (one-line workaround) |
| `src/common/Threading/LockedQueue.h` | 151 | `std::queue<T>` + `std::mutex` |
| `src/common/Threading/MPSCQueue.h` | 175 | Vyukov non-intrusive + intrusive lock-free MPSC |
| `src/common/Threading/ProducerConsumerQueue.h` | 121 | Bounded blocking SPMC/MPMC |
| `src/common/Threading/ThreadPool.h` | 48 | Boost-asio-backed pool |
| `src/common/Threading/ProcessPriority.{h,cpp}` | 29+98 | `setpriority`/`SetPriorityClass` per-OS shim |
| `src/common/Time/Timer.h` | 190 | `getMSTime()`, `IntervalTimer`, `TimeTrackerSmall`, `TimeTracker` |
| `src/common/Time/Timezone.{h,cpp}` | 38+183 | DST/TZ offset helpers (uses `std::chrono::tzdb` if available) |
| `src/common/Utilities/Util.{h,cpp}` | 535+915 | The kitchen sink: `Tokenize`, `strToUpper/Lower`, **`Utf8ToUpperOnlyLatin`** (load-bearing for SRP6 — see §13), `WStrToUtf8`, `Utf8toWStr`, hex-byte conv, `RemoveCRLF`, `StringEqualI`, `GetTypeName`, ANSI/OEM console hooks |
| `src/common/Utilities/Random.{h,cpp}` | 79+96 | `urand/irand/frand/rand_norm/rand_chance/urandweighted` over `RandomEngine` (singleton SFMT) |
| `src/common/Utilities/SFMTRand.{h,cpp}` | 44+117 | SFMT PRNG implementation |
| `src/common/Utilities/Hash.h` | 60 | `hash_combine` (Boost-style 0x9E3779B9) + `HashFnv1a` + `std::hash<std::pair>` specialisation |
| `src/common/Utilities/StringFormat.h` | 111 | `fmt::format`-thin-wrapper macro (`Trinity::StringFormat`) |
| `src/common/Utilities/StringConvert.h` | 277 | `StringTo<T>` typed parse with `Optional<T>` return |
| `src/common/Utilities/EnumFlag.h` | 131 | `DEFINE_ENUM_FLAG(E)` to opt an enum into bitwise ops |
| `src/common/Utilities/SmartEnum.h` | 129 | Reflection over enums (name ↔ value) |
| `src/common/Utilities/EventMap.{h,cpp}` | 300+219 | Event-id-keyed timer table used by creature AI |
| `src/common/Utilities/EventProcessor.{h,cpp}` | 120+133 | Generic timed-event scheduler |
| `src/common/Utilities/TaskScheduler.{h,cpp}` | 653+240 | Composable async-ish scheduler used by spell/AI scripts |
| `src/common/Utilities/MessageBuffer.h` | 139 | Resizable read/write byte buffer used by sockets |
| `src/common/Utilities/Optional.h` | 27 | `using Optional = std::optional` typedef |
| `src/common/Utilities/IteratorPair.h` | 69 | `boost::iterator_range`-style adapter |
| `src/common/Utilities/Tuples.h` | 65 | `std::apply` helpers |
| `src/common/Utilities/FlagsArray.h` | 146 | Fixed-size bit array |
| `src/common/Utilities/FuzzyFind.h` | 57 | substring-count based ranked lookup (`StringContainsStringI` per needle + optional bonus) |
| `src/common/Utilities/AsyncCallbackProcessor.h` | 62 | Promise-future drainer |
| `src/common/Utilities/Locales.{h,cpp}` | 31+51 | `LocaleConstant` enum + `LocaleNames[]` |
| `src/common/Utilities/Memory.h` | 49 | `std::default_delete` helpers |
| `src/common/Utilities/Concepts.h` | 33 | C++20 concepts |
| `src/common/Utilities/Containers.h` | 322 | Generic `RandomElement`, `EraseIf`, `MapEqualRange` etc. |
| `src/common/Utilities/Regex.h` | 35 | `std::regex` typedef alias |
| `src/common/Utilities/Duration.h` | 52 | `chrono` typedefs (`Milliseconds`, `Seconds`, `TimePoint`, `SystemTimePoint`) |
| `src/common/Utilities/StartProcess.{h,cpp}` | 70+293 | `boost::process::child` launcher |
| `src/common/Utilities/ByteConverter.h` | 67 | `EndianConvert` per-type |
| `src/common/Utilities/Types.h` | 74 | `int8`/`uint64`/etc. typedefs |
| `src/common/Banner.{h,cpp}` | small | Server-startup banner |
| `src/common/Common.{h,cpp}` | small | Misc |
| `src/common/CompilerDefs.h` | 70 | `TRINITY_COMPILER` / `TRINITY_PLATFORM` / `TRINITY_ENDIAN` macros |
| `src/common/GitRevision.{h,cpp}` | small | Embedded git-rev string |
| `src/common/Define.h` | ~150 | `TC_COMMON_API` / typedefs |
| **TOTAL (excl. Collision)** | **~11600 lines** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Trinity::Asio::IoContext` | class | RAII Boost.Asio executor |
| `Trinity::Asio::DeadlineTimer` | typedef | Steady-clock waitable timer |
| `Trinity::Asio::Resolver` | class | DNS resolver |
| `ConfigMgr` | singleton class | INI-format config |
| `Trinity::Containers::FlatSet<K>` | template class | Sorted-vector set |
| `Log` | singleton class | Top-level logger |
| `Logger` | class | Per-filter log channel |
| `Appender` (abstract), `AppenderConsole`, `AppenderFile`, `AppenderDB` | class | Output sinks |
| `LogMessage` | struct | One record |
| `LogLevel` | enum | `LOG_LEVEL_TRACE` … `LOG_LEVEL_FATAL` |
| `Metric` | singleton class | InfluxDB push |
| `MetricData` | struct | One metric record + intrusive queue link |
| `MetricDataType` | enum | `VALUE` / `EVENT` |
| `IpLocationStore` | singleton class | CSV-backed GeoIP |
| `IpLocationRecord` | struct | `{IpFrom, IpTo, CountryCode, CountryName}` |
| `Trinity::Impl::MPSCQueueNonIntrusive<T>` / `Intrusive<T,Link>` | template class | Vyukov queues |
| `LockedQueue<T>` | template class | `mutex+queue` blocking queue |
| `ProducerConsumerQueue<T>` | template class | Bounded blocking |
| `IntervalTimer` | struct | "fires every N ms" helper |
| `TimeTracker` / `TimeTrackerSmall` | struct | Countdown timer |
| `RandomEngine` | singleton class | SFMT PRNG wrapper |
| `LocaleConstant` | enum | `LOCALE_enUS` etc. |
| `EventMap` | class | Creature-AI scheduling |
| `EventProcessor` | class | Generic timed events |
| `TaskScheduler` | class | Composable scheduler |
| `MessageBuffer` | class | Byte buffer for sockets |
| `LogOperation` | struct | Async post wrapper |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `sConfigMgr->GetStringDefault(name, def)` | Fetch config value with fallback | Boost ptree |
| `sConfigMgr->LoadInitial(file, args, error)` | First config load | `LoadFile` |
| `sConfigMgr->OverrideWithEnvVariablesIfAny()` | Apply env-var overrides | `IniKeyToEnvVarKey` |
| `TC_LOG_INFO(filter, fmt, ...)` (macro → `sLog->OutMessage`) | Log entry | `Logger::write` → appenders |
| `Log::Initialize(IoContext*)` | Start async logger | `LoadFromConfig`, strand wire-up |
| `sIPLocation->GetLocationRecord(ip)` | Country lookup | `upper_bound` binary search |
| `sIPLocation->Load()` | Parse CSV at startup | file scan + sort |
| `sMetric->LogValue(category, value, tags)` | Push metric | MPSCQueue enqueue |
| `Trinity::Net::IsInLocalNetwork(addr)` | RFC1918 / loopback check | scanned interface table |
| `urand(min, max)` / `irand(min, max)` / `frand(min, max)` | Inclusive random | `RandomEngine::Instance()` |
| `rand_norm()` | `[0, 1)` random | `RandomEngine` |
| `getMSTime()` | App-uptime ms | `steady_clock::now() - start` |
| **`Utf8ToUpperOnlyLatin(string&)`** | **Uppercase ASCII Basic Latin only** for SRP6/account canonicalisation; non-ASCII Latin letters stay unchanged because `wcharToUpperOnlyLatin` is gated by `isBasicLatinCharacter` | `Utf8toWStr` → `wcharToUpperOnlyLatin` |
| `Trinity::Tokenize(str, sep, keepEmpty)` | Split on `char` | view-based |
| `WPAssert(cond)` / `ASSERT_NOTNULL(p)` | Abort with file:line on failure | `Trinity::Assert` |
| `Trinity::StringTo<T>(str)` | Typed parse → `Optional<T>` | `from_chars` / `strtoull` |
| `Trinity::StringFormat(fmt, args)` | `fmt::format` wrapper | libfmt |

---

## 5. Module dependencies

**Depends on:**
- Boost (asio, filesystem, property_tree, system, process, program_options)
- libfmt (StringFormat)
- OpenSSL (CryptoConstants only references digest sizes; real algo modules live in `Cryptography/`)
- Argon2 (`Argon2.cpp` only)
- C++20 stdlib

**Depended on by:**
- **Everyone.** Every other crate/module in TrinityCore includes at least `Define.h`, `Errors.h`, and `Log.h`. `IoContext`, `MPSCQueue`, `MessageBuffer` are required by `shared/Networking`. `ConfigMgr` is required by every executable. `Metric` is opt-in but called from `Map`, `WorldSocket`, `Player::Update`. `IPLocation` is queried from `WorldSession::HandleAuthSession`.

---

## 6. SQL / DB queries

`AppenderDB` (a logger appender) writes to `auth.logs`:

| Statement / Source | Purpose | DB |
|---|---|---|
| `INS_LOG` (in `LoginDatabase`) | Insert one row from `AppenderDB::_Write` | auth |
| `realmlist`/`account` reads | None from `common/`; `Config.cpp` does **not** touch DB | — |

DBC/DB2: none — `common/DataStores/` (the binary parser primitives) is covered by `shared-datastores.md`.

---

## 7. Wire-protocol packets

None. Common is pre-protocol — it ships no opcodes. The closest it gets is `MessageBuffer` (used by `shared/Networking/Socket`) and `WorldPacketCrypt` (which lives in `Cryptography/Authentication/` and is covered by `crypto.md`).

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**

| C++ subsystem | Rust location | Status |
|---|---|---|
| `Asio/` (IoContext, DeadlineTimer, Strand, Resolver, IpAddress, IpNetwork) | **`tokio` crate** + scattered usages in `crates/wow-network/src/{accept,world_socket}.rs` | ✅ replaced — Tokio runtime (`tokio::runtime::Runtime`) replaces `IoContext`; `tokio::time::{interval, sleep}` replaces `DeadlineTimer`; `tokio::net::TcpListener/TcpStream` replaces Boost.Asio sockets; no `Strand` equivalent needed (single-threaded `LocalSet` or `tokio::task::spawn_local` if required) |
| `Configuration/Config.{h,cpp}` | `crates/wow-config/src/lib.rs` (397 lines) | ⚠️ partial — handcrafted INI parser (key=value with `#` comments + quoted strings + case-insensitive lowercased keys); **no env-variable override**, **no nested-section support**, **no reload watcher**, **no built-in defaults** |
| `Containers/FlatSet.h` | none — code uses `std::collections::HashSet` and `BTreeSet` directly | ✅ obviated — sorted-vector set is a C++ space optimisation; in Rust `BTreeSet` covers the lookup case |
| `Containers/Utilities/ArrayWrapper.h` | n/a | ✅ obviated — `[T; N]` and slices |
| `Cryptography/` (constants, OpenSSLCrypto, Argon2) | partly in `crates/wow-crypto/` (see `crypto.md`); **Argon2 NOT implemented** | ⚠️ — see cross-reference below |
| `Debugging/Errors.h` (`ASSERT`, `ABORT`, `Fatal`) | Rust's `assert!`, `debug_assert!`, `panic!`, `unreachable!` + `wow_logging::install_panic_hook_like_cpp()` | ⚠️ partial — Rust panics now emit a structured `tracing::error!(fatal=true, file, line, column, panic_message)` before the default panic hook in `world-server`/`bnet-server`; no central `Trinity::Assert`, no `GetDebugInfo()` stack-dump capture, and no forced null-deref crash shim |
| `Encoding/Base32` | none | ❌ missing — only used by `TOTP.cpp` (2FA), which is also missing |
| `Encoding/Base64` | none in own crate; used via `base64` crate (cargo dep) only inside `bnet-server/rest` | ⚠️ — partial use of external `base64` crate; no centralised wrapper |
| `Encoding/Hex` (in `Util.cpp`) | `hex` crate referenced from `wow-crypto/src/bnet_srp6.rs:258, 454`; `wow-database` for blob hex columns | ✅ replaced via `hex` crate |
| `IPLocation/` | `wow_core::IpLocationStore` plus BNet/world loader/use | ⚠️ partial — CSV parser and lookup are common; BNet and world auth country locks use it; GM command callers still need AccountMgr/RBAC wiring |
| `Logging/` (Log, Logger, Appenders, LogMessage, LogOperation) | `crates/wow-logging/src/lib.rs` (464 lines) — re-export of `tracing` + `LogFilter` enum + macros | ⚠️ partial — `tracing` covers level filtering and structured fields, but **no per-filter file appenders, no rotating file appender, no DB-backed `AppenderDB` (so no `auth.logs` writes), no ANSI-colour console formatter beyond what `tracing-subscriber::fmt` provides** |
| `Metric/` (InfluxDB) | **none** | ❌ missing entirely — no metrics push pipeline; instrumentation reduced to `tracing` events |
| `Platform/ServiceWin32` | n/a — Linux-only deployment | ✅ obviated |
| `Hacks/boost_program_options_with_filesystem_path.h` | n/a — no Boost program_options dep; CLI args parsed with `clap` or hand-rolled | ✅ obviated |
| `Threading/LockedQueue` | `parking_lot::Mutex<VecDeque<T>>` ad-hoc | ✅ replaced |
| `Threading/MPSCQueue` (Vyukov) | `flume::unbounded()` / `flume::bounded(N)` (used in `wow-network/src/world_socket.rs`, `wow-world/src/session.rs`) | ✅ replaced — flume is bounded-MPMC; for true MPSC `tokio::sync::mpsc` is also available |
| `Threading/ProducerConsumerQueue` | `flume::bounded(N)` | ✅ replaced |
| `Threading/ThreadPool` | `tokio::task::spawn` / `tokio::runtime::Runtime` | ✅ replaced |
| `Threading/ProcessPriority` | none — defaults to OS scheduler | ❌ missing (low priority) |
| `Time/Timer.h` (`getMSTime`, `IntervalTimer`, `TimeTracker`) | `crates/wow-core/src/time.rs` — `ServerTime` (Instant), `GameTime` (Unix), `Diff(u32)`, `IntervalTimer` | ⚠️ partial — covers `getMSTime`, `Diff`, and C++ `IntervalTimer`; `TimeTracker`/`PeriodicTimer` remain open |
| `Time/Timezone` | partial — `GameTime::to_packed()` uses libc `localtime_r`; no standalone timezone helper layer | ⚠️ partial — packed WoW time now uses real calendar math like `WowTime::GetPackedTime`; full `Timezone.{h,cpp}` helpers/DST offset API remain open |
| `Utilities/Util.cpp::Tokenize` | `str::split` + `collect::<Vec<_>>()` | ✅ idiomatic replacement |
| `Utilities/Util.cpp::strToUpper/Lower` | `str::to_ascii_uppercase()` | ✅ for ASCII; ⚠️ for non-ASCII |
| **`Utilities/Util.cpp::Utf8ToUpperOnlyLatin`** | `wow_core::{utf8_to_upper_only_latin_like_cpp, utf8_to_lower_only_latin_like_cpp}` | ✅ C++ Basic-Latin-only upper semantics; Rust also exposes the deliberate symmetric lower helper for future ports; invalid UTF-8 cannot enter the Rust `&str` API |
| `Utilities/Random.{h,cpp}` (`urand`, `frand`, etc.) | `wow_core::random` helpers plus legacy direct `rand` callers | ⚠️ partial — central wrappers exist for C++ helper semantics; not every call site has been migrated yet |
| `Utilities/SFMTRand` | `rand` crate's default PRNG (currently ChaCha12 in `thread_rng`) | ⚠️ — different algorithm; loot-tier reproducibility from a captured C++ seed will not match. Acceptable for non-determinism-sensitive paths. |
| `Utilities/Hash.h::hash_combine` / `HashFnv1a` | none | ❌ missing — Rust hashing covered by `std::hash::Hasher`; FNV1a referenced nowhere in Rust code yet |
| `Utilities/StringFormat.h` | `format!()` macro / `tracing` field formatting | ✅ replaced |
| `Utilities/StringConvert.h::StringTo<T>` | `str::parse::<T>()` returning `Result<T, _>` | ✅ replaced |
| `Utilities/EnumFlag.h::DEFINE_ENUM_FLAG` | `bitflags!` crate macro | ✅ replaced |
| `Utilities/SmartEnum.h` | none (no reflection); enums are pattern-matched | ⚠️ — Rust uses derive macros and pattern matching; no name↔value reflection helper |
| `Utilities/EventMap.{h,cpp}` | none (no port) — creature AI is ECS-driven via `wow-ai` | ⚠️ — feature parity missing for legacy CreatureAI scripts; not yet needed |
| `Utilities/EventProcessor.{h,cpp}` | none | ❌ missing |
| `Utilities/TaskScheduler.{h,cpp}` | none | ❌ missing — used heavily by C++ spell scripts; affects script porting (`wow-script`) |
| `Utilities/MessageBuffer.h` | `Vec<u8>` + `bytes::BytesMut` ad-hoc | ✅ obviated |
| `Utilities/Optional.h` | `Option<T>` builtin | ✅ obviated |
| `Utilities/IteratorPair.h` | iterator combinators (`Iterator` trait) | ✅ obviated |
| `Utilities/Tuples.h` | tuple destructuring builtin | ✅ obviated |
| `Utilities/FlagsArray.h` | none | ⚠️ — only used by `Map.cpp` GameObject phase masks; not yet a blocker |
| `Utilities/FuzzyFind.h` | `wow_core::{string_contains_string_i_like_cpp, fuzzy_find_in_like_cpp}` | ✅ common helper ported — used by future GM commands `.tele <name>` etc.; command dispatcher/handlers remain open |
| `Utilities/AsyncCallbackProcessor.h` | `tokio::task::JoinHandle` | ✅ obviated |
| `Utilities/Locales.{h,cpp}` | `wow-constants` crate has `LocaleConstant` enum | ✅ replaced |
| `Utilities/Memory.h` | `Box<T>` / `Drop` trait | ✅ obviated |
| `Utilities/Concepts.h` | trait bounds | ✅ obviated |
| `Utilities/Containers.h` | iterator combinators | ✅ obviated |
| `Utilities/Regex.h` | `regex` crate | ✅ replaced |
| `Utilities/Duration.h` | `std::time::Duration` builtin | ✅ obviated |
| `Utilities/StartProcess.{h,cpp}` | `std::process::Command` | ✅ replaced |
| `Utilities/ByteConverter.h` | `u32::to_le_bytes` / `from_be_bytes` builtins | ✅ obviated |
| `Utilities/Types.h` | `u32`/`i64` etc. builtin | ✅ obviated |
| `wow-collections` crate (Rust-only) | `MultiMap<K, V, N>` (SmallVec-backed), `FlagArray` | — not a port; net-new |

**What's implemented:**
- Async runtime, sockets, channels, mutexes (Tokio + flume + parking_lot + dashmap).
- Config file parsing (with the gaps noted above).
- Structured logging via `tracing` (single-binary stdout/stderr; no rotating files).
- Time primitives for the server tick loop (`ServerTime`, `Diff`).
- Random via `rand::thread_rng()`.
- Hex encoding via `hex` crate.

**What's missing vs C++:**
- **`IPLocation` GM command wiring** — common store exists and BNet/world auth use it, but account lock-country GM commands are not wired yet.
- **`Metric`** — no InfluxDB or any metrics push.
- **`AppenderFile` / log rotation** — `tracing-subscriber` writes to stdout only; production needs at minimum rotating-file appender (`tracing-appender::rolling`).
- **`AppenderDB`** — log records are not written to `auth.logs`; the C# legacy server did this for audit/GM-action retention. Direct compliance/audit gap.
- **`Timezone` helper layer** — `GameTime::to_packed()` now uses real calendar math, but the standalone C++ `Timezone.{h,cpp}` helper API is not fully ported.
- **`ConfigMgr::OverrideWithEnvVariablesIfAny`** — production deployments commonly do `WORLD_SERVER_PORT=8085`-style overrides; not supported.
- **`ConfigMgr::Reload`** — config is parsed once at startup; SIGHUP-style reload not wired.
- **`TaskScheduler` + `EventProcessor`** — required when porting C++ creature/spell scripts that call `Schedule(2s, ...)`.
- **GM command consumers of `FuzzyFind`** — common ranked lookup is ported, but `.tele <name>` cannot use it until the command dispatcher and `cs_tele` are ported.
- **`Hash::hash_combine` / `HashFnv1a`** — used by C++ `MapManager` to compose `(map, instance)` keys; Rust uses `(u32,u32)` tuples directly — not strictly missing, just different idiom.
- **`Argon2`** — covered in `crypto.md`; deferred (BNet v2 2FA only).
- **`SFMTRand` reproducibility** — Rust uses a different PRNG algorithm.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- `wow-config` lowercases keys; C++ ptree preserves case. Cross-reference any handler that does `sConfigMgr->GetStringDefault("Some.NestedKey.Foo", ...)` — Rust treats `some.nestedkey.foo` and `Some.NestedKey.Foo` identically, C++ treats them differently. Not yet a known bug.
- `tracing` filter syntax (`info,wow_world=debug`) does **not** match TrinityCore's `Logger.root.level=3` / `Appender.Console=1,Console`-style config. The `LogLevel` enum in `wow-logging` is decorative — it never gates anything — and the actual filtering happens via `RUST_LOG` / `EnvFilter`.

**Tests existing:**
- `wow-config`: 19 tests (parse, comments, case-insensitive lookup, error paths). No env-var override tests (because feature is missing).
- `wow-core`: 5 tests in `time.rs` (round-trip elapsed, packed time, has_passed, diff).
- `wow-collections`: tests for `MultiMap` and `FlagArray`.
- `wow-logging`: minimal — re-exports `tracing` macros without unit tests.
- No tests for any C++-equivalent behaviour (this is by design — they are different crates, not ports).

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md`. Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [x] **#COMMON.1** Implement `utf8_to_upper_only_latin_like_cpp(s: &str) -> String` in `wow-core` and call it from Grunt SRP6 `compute_x` / `compute_client_evidence`. C++ contrast corrected the stale assumption: `Utf8ToUpperOnlyLatin` only uppercases ASCII Basic Latin because `wcharToUpperOnlyLatin` is gated by `isBasicLatinCharacter`. (M)
- [x] **#COMMON.2** Vector tests: `Utf8ToUpperOnlyLatin("caféÀßÿ") == "CAFéÀßÿ"`, `Utf8ToUpperOnlyLatin("straße") == "STRAßE"`, and Greek text unchanged; these match C++ Basic-Latin-only output and avoid Rust Unicode case expansion. (M)
- [ ] **#COMMON.3** Add `tracing-appender::rolling::daily()` file appender to `wow-logging::init_logging()` so production has rotating logs. (L)
- [ ] **#COMMON.4** Add `AppenderDB`-equivalent: a tracing layer that flushes WARN+ events into `auth.logs` via `wow-database`. (M)
- [ ] **#COMMON.5** Implement `wow-config::reload()` with SIGHUP wire-up in both binaries. (M)
- [ ] **#COMMON.6** Implement env-var override (`OverrideWithEnvVariablesIfAny`) for `wow-config`. Map `World.Server.Port` → `WORLD_SERVER_PORT` per the C++ snake-case rule. (M)
- [x] **#COMMON.7** Replace `wow-core::time::GameTime::to_packed()`'s approximate date math with libc `localtime_r` fields, matching C++ `WowTime::GetPackedTime` field layout. Full `Timezone` helper API remains a separate future gap. (L)
- [x] **#COMMON.8** Add `IntervalTimer` struct to `wow-core::time` (Update/Passed/Reset, mirrors C++ `IntervalTimer` for signed diffs, pass check, negative clamp, and modulo overshoot reset). (L)
- [x] **#COMMON.9a** Port C++ `IPLocation` CSV range store and `GetLocationRecord` lookup into `wow-core` (`IpLocationStore`) and reuse it from BNet country-lock auth. (M)
- [x] **#COMMON.9b** Wire shared `IpLocationStore` into world auth (`WorldSocket::HandleAuthSession`): worldserver loads `IPLocationFile`, passes the store to each `WorldSocket`, and rejects locked accounts on IP/country mismatch following C++ fail-open semantics for empty/missing country data. (M)
- [ ] **#COMMON.9c** Wire shared `IpLocationStore` into future AccountMgr/RBAC GM command callers that use `sIPLocation` in C++ account lock-country commands. (M)
- [ ] **#COMMON.10** `Metric` port: choose between (a) `metrics` + `metrics-exporter-prometheus` for pull-style or (b) `influxdb-rs` for push. (H)
- [ ] **#COMMON.11** Port `TaskScheduler` (composable async scheduler) — needed before any C++ spell-script port can compile. (XL — split into Schedule/RepeatedSchedule/Async/Group).
- [ ] **#COMMON.12** Port `EventMap` for legacy CreatureAI shim. (M)
- [x] **#COMMON.13** Centralised `wow-core::random` module wrapping `rand::thread_rng()` with `urand`/`irand`/`frand`/`rand_norm`/`rand_chance`/`roll_chance_*`/`urandweighted` mirror functions plus `*_with_rng_like_cpp` variants for deterministic tests. Existing call sites can migrate incrementally. (L)
- [ ] **#COMMON.14** Document choice of PRNG (currently ChaCha12 via `thread_rng`); decide whether SFMT reproducibility matters — if yes, depend on `sfmt` crate. (L)
- [x] **#COMMON.15** Implement `wow-core::utf8_to_lower_only_latin_like_cpp`, the Basic-Latin-only symmetric counterpart to `Utf8ToUpperOnlyLatin`. C++ has broader `wcharToLower`; this Rust helper is intentionally narrower to prevent accidental Unicode lowercase expansion in future ports. (L)
- [x] **#COMMON.16** Wire the useful logging side of `Errors::Fatal`: `wow_logging::install_panic_hook_like_cpp()` logs `fatal=true`, source location and panic message through `tracing::error!` before delegating to Rust's default panic hook. `world-server` and `bnet-server` install it after tracing setup. (L)
- [x] **#COMMON.17** Port `FuzzyFindIn` semantics into `wow-core`: byte-wise ASCII `StringContainsStringI`, ranked matches by number of matching needles, and optional bonus. C++ does **not** use Levenshtein/`strsim`; old docs were corrected. (L)
- [x] **#COMMON.18a** Port the Unix IPv4/IPv6 subset of `Trinity::Net::ScanLocalNetworks` / `IsInLocalNetwork` plus `SelectAddressForClient` priority selection into `wow-core::net`; IPv4 bnet/world callers use scanned interface networks with a `/24` fallback if scanning returns none. (M)
- [ ] **#COMMON.18b** Add Windows `GetAdaptersAddresses` parity for `ScanLocalNetworks` and wire IPv6 address selection through callers once realm/LoginREST address resolution stores IPv6 candidates. (M)

---

## 10. Regression tests to write

- [x] `Utf8ToUpperOnlyLatin("caféÀßÿ") == "CAFéÀßÿ"` matches C++ Basic-Latin-only output.
- [x] `Utf8ToUpperOnlyLatin("ß")` does **not** become `"SS"` or `"ẞ"`; non-ASCII stays unchanged under this specific C++ helper.
- [x] `Utf8ToUpperOnlyLatin("CAFÉ123")` is idempotent.
- [ ] `wow-config` env-var override: `WORLD_SERVER_PORT=9999 ./world-server` overrides `WorldServerPort = 8085`.
- [ ] `wow-config` reload preserves `keepOnReload=true` keys.
- [x] `IntervalTimer::Passed()` / `Reset()` correctness for C++ threshold and overshoot semantics.
- [ ] `getMSTime()` Rust equivalent never wraps within a 49-day session window (`u32` ms ⇒ fine; but check the cast in `time.rs:18`).
- [x] `IPLocation::GetLocationRecord` half-open range lookup, quote stripping, lowercase country code, and non-IPv4 rejection are covered by `wow-core::ip_location` tests.
- [ ] `Metric` smoke test: 1000 enqueues drain in <100ms without dropping.
- [ ] Logger filter: `wow_world=debug` admits a `debug!` from `wow-world` but suppresses one from `wow-network`.
- [x] Time-packed field layout uses real calendar dates and matches C++ `WowTime::GetPackedTime` for representative UTC dates, including year-boundary coverage.

---

## 11. Notes / gotchas

1. **`Utf8ToUpperOnlyLatin` is the only common-layer function on the SRP6 hot path.** Every other Util.cpp helper is non-load-bearing for protocol correctness. See §13.
2. **No Strand equivalent.** Boost.Asio's `strand` serialises completion handlers across threads. Tokio's analogue is `LocalSet` + `spawn_local`, but the Rust port does not currently need it: the world-server tick loop is already single-threaded per session via `flume` channels.
3. **`tracing` log filter ≠ TrinityCore log filter.** TC's filter strings are subsystem names like `server.network`, `entities.player`. `tracing` uses module paths like `wow_world::handlers::character`. There is no automatic mapping; the `LogFilter` enum in `wow-logging::lib.rs` is a manual override that sets a structured field but does not affect filtering.
4. **`ConfigMgr` lowercases keys.** A handler reading `"World.Realm.Id"` and `"world.realm.id"` will get the same value, unlike C++. Currently safe because no config file uses two such keys; document if you add `wow-config` features.
5. **`Timezone` helper API is still partial.** `GameTime::to_packed` no longer uses the old 365.25-day approximation; it uses libc `localtime_r` and the C++ `WowTime::GetPackedTime` bit layout. The remaining gap is the broader `Timezone.{h,cpp}` offset/DST helper surface.
6. **No `Trinity::Fatal` equivalent.** Rust panics already get a backtrace under `RUST_BACKTRACE=1`, but they don't go through `tracing` first, so production logs will not have the assertion message in the structured-log stream — only in stderr. Sub-task #COMMON.16.
7. **`flume::bounded(256)` capacity** matches the C++ `ProducerConsumerQueue` default. Don't change without revisiting `wow-network::session_mgr`.
8. **`MessageBuffer.h` was a hand-rolled vector with read/write cursors.** Rust uses `bytes::BytesMut` or `Vec<u8>` directly; check that `wow-packet` `WorldPacket::from_bytes` doesn't accidentally rebuild this primitive.
9. **`SFMTRand` vs `ChaCha12`.** If a regression test ever depends on "given seed X, urand returns Y", it must be rewritten — Rust's PRNG is different. No current test depends on this.
10. **`Hacks/boost_program_options_with_filesystem_path.h`** is genuinely a Boost-version-specific workaround; safe to skip forever.
11. **`Platform/ServiceWin32.cpp`** is a Windows-only daemon adapter. Rust target is Linux; ignore unless we ever ship Windows binaries.

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `Trinity::Asio::IoContext` | `tokio::runtime::Runtime` | Multi-thread by default; switch to `current_thread` if needed |
| `Trinity::Asio::DeadlineTimer` | `tokio::time::Sleep` / `tokio::time::Interval` | `interval.tick().await` ≈ `timer.expires_after(d); timer.async_wait(cb)` |
| `Trinity::Asio::Strand` | `tokio::task::LocalSet` + `spawn_local` | Currently unused |
| `tcp::resolver` | `tokio::net::lookup_host` | Async DNS |
| `boost::asio::ip::address` | `std::net::IpAddr` | — |
| `Trinity::Net::IsInLocalNetwork(addr)` / `SelectAddressForClient` | `wow_core::net::{scan_local_ip_networks_like_cpp,select_ip_address_for_client_like_cpp}` | Unix IPv4/IPv6 done; Windows/caller IPv6 storage open under #COMMON.18b |
| `ConfigMgr` (singleton) | `wow_config::load_config(path)` + `get_value::<T>(key)` | Lowercased keys |
| `Trinity::Containers::FlatSet<K>` | `BTreeSet<K>` | Drop the C++-only space optimisation |
| `Log` (singleton) + `Logger` + `Appender*` | `tracing::subscriber` + `tracing-subscriber::fmt::Layer` | `init_logging("info")` in `wow-logging` |
| `LogMessage` | `tracing::Event` | — |
| `LogLevel::LOG_LEVEL_INFO` | `tracing::Level::INFO` | — |
| `IpLocationStore` | `wow_core::IpLocationStore` | CSV parser and half-open `upper_bound` lookup done; not yet wired to every C++ caller |
| `Metric` | NOT YET PORTED | sub-task #COMMON.10 |
| `LockedQueue<T>` | `parking_lot::Mutex<VecDeque<T>>` | — |
| `MPSCQueue<T, &T::Link>` | `flume::unbounded::<T>()` | Lock-free MPMC; intrusive variant unused in Rust |
| `ProducerConsumerQueue<T>` | `flume::bounded::<T>(N)` | — |
| `ThreadPool` | `tokio::runtime::Runtime` workers | Implicit |
| `SetProcessPriority(MEDIUM_PRIORITY_CLASS)` | NOT YET PORTED | nice-level shim missing |
| `getMSTime()` | `wow_core::time::ServerTime::elapsed_ms()` | `Instant::now() - start` |
| `IntervalTimer` | `wow_core::IntervalTimer` | sub-task #COMMON.8 |
| `TimeTracker` | NOT YET PORTED | — |
| `Trinity::TimeBreakdown(t)` | `chrono::DateTime::from_timestamp(t, 0)` | external crate |
| `urand(min, max)` | `rand::thread_rng().gen_range(min..=max)` | inclusive range |
| `frand(0.0, 1.0)` | `rand::thread_rng().gen::<f32>()` | `[0,1)` |
| `RandomEngine::Instance()` | `rand::thread_rng()` | thread-local |
| `Trinity::Tokenize(s, ',', false)` | `s.split(',').filter(\|x\| !x.is_empty()).collect()` | iterator-style |
| `strToUpper(s)` | `s.to_ascii_uppercase()` | ASCII only |
| `Utf8ToUpperOnlyLatin(s)` | `wow_core::utf8_to_upper_only_latin_like_cpp` | C++ Basic-Latin-only semantics; used by Grunt SRP6 |
| `WStrToUtf8(w, &out)` | `OsString` ↔ `String` round-trip | rarely needed (no UTF-16 wire data) |
| `WPAssert(cond)` | `assert!(cond)` / `debug_assert!(cond)` | panic on fail |
| `ABORT_MSG(fmt, ...)` | `panic!(fmt, ...)` | — |
| `ASSERT_NOTNULL(p)` | `Option::expect(p, "...")` / pattern-match `Some` | type-system enforced |
| `Trinity::Impl::ByteArrayToHexStr(buf, len)` | `hex::encode(buf)` | external crate |
| `Trinity::Impl::HexStrToByteArray(str, out, n)` | `hex::decode_to_slice(str, out)` | — |
| `Trinity::StringFormat("{} {}", a, b)` | `format!("{a} {b}")` | builtin |
| `Trinity::StringTo<u32>(s)` | `s.parse::<u32>().ok()` | — |
| `DEFINE_ENUM_FLAG(Foo)` | `bitflags! { struct Foo: u32 { ... } }` | external crate |
| `EnumUtils::ToString(Foo::A)` | `format!("{:?}", Foo::A)` for Debug; for prod use a manual `as_str()` | no derive in stdlib |
| `EventMap::ScheduleEvent(id, ms)` | NOT YET PORTED | sub-task #COMMON.12 |
| `TaskScheduler::Schedule(2s, [](){...})` | NOT YET PORTED | sub-task #COMMON.11 |
| `MessageBuffer` | `bytes::BytesMut` / `Vec<u8>` + manual cursor | inline in `wow-packet` |
| `Trinity::hash_combine(seed, x)` | derive `Hash` or compose with `(K1,K2)` tuple | unused so far |
| `Trinity::HashFnv1a(s)` | `fnv::FnvHasher::default()` if needed | not yet a dep |
| `Optional<T>` | `Option<T>` | builtin |
| `LocaleConstant` | `wow_constants::Locale` enum | already ported |
| `EndianConvert(x)` | `u32::to_le_bytes(x)` / `from_be(x)` | builtins |
| `int8`/`uint64`/`uint32` | `i8`/`u64`/`u32` | builtins |

---

## 13. Audit (2026-05-01)

> Audited 2026-05-01 by sub-agent against C++ at `/home/server/woltk-trinity-legacy` commit `5100ce3d8fc6`. Scope: every subdirectory under `src/common/` except `Collision/` (covered separately) and `DataStores/` (covered by `shared-datastores.md`). The `Cryptography/` subdirectory cross-references `crypto.md`; this audit only re-checks the items NOT covered there (`CryptoConstants.h`, `OpenSSLCrypto.{h,cpp}`, `Argon2.{h,cpp}`).

### 13.1 Subsystem-by-subsystem

| Subsystem | C++ ref | Rust ref | Status | Divergence |
|---|---|---|---|---|
| Asio runtime | `Asio/IoContext.h:30` `boost::asio::io_context` | `tokio::runtime::Runtime` | ✅ | Idiomatic replacement; Tokio's executor is feature-equivalent for the server's needs (TCP accept + timer + DNS). |
| DeadlineTimer | `Asio/DeadlineTimer.h:24` | `tokio::time::interval` (`bnet-server/main.rs:228`, `bnet-server/realm/mod.rs:206`) | ✅ | One-shot deadline = `tokio::time::sleep(d).await`; recurring = `interval.tick().await`. |
| Resolver | `Asio/Resolver.h:30` | `tokio::net::lookup_host` | ✅ | — |
| `IsInLocalNetwork(addr)` / `SelectAddressForClient` | `Asio/IpNetwork.cpp:32-134`, Unix `ScanLocalNetworks` at `:221-268` | `wow-core::net::{scan_local_ip_networks_like_cpp, IpNetworkLikeCpp, select_ip_address_for_client_like_cpp}` | ⚠️ | Unix IPv4/IPv6 scan and address-priority semantics are shared and tested; Windows `GetAdaptersAddresses` and caller-level IPv6 address storage remain open. |
| `IpAddress::from_string` | `Asio/IpAddress.h:24` | `IpAddr::from_str` (std) | ✅ | — |
| `ConfigMgr` INI parser | `Configuration/Config.cpp:39-60` (`bpt::ini_parser::read_ini`) | `wow-config/src/lib.rs:52-87` (hand-rolled key=value) | ⚠️ | C++ supports nested sections (`[Net] Port=8085`); Rust does not. Fine because `worldserver.conf` is flat key=value. |
| Env-var override | `Configuration/Config.cpp` `OverrideWithEnvVariablesIfAny` | NONE | ❌ | Sub-task #COMMON.6. |
| Reload | `ConfigMgr::Reload` | NONE | ❌ | Sub-task #COMMON.5. |
| Built-in defaults | `Configuration/BuiltInConfig.cpp:30-45` | NONE | ❌ | Rust defers to `get_value_default::<T>(key, default)` at every callsite. Equivalent in practice. |
| `FlatSet<K>` | `Containers/FlatSet.h:26-100` | NONE (use `BTreeSet`) | ✅ | Consciously dropped; `BTreeSet` covers the use case. |
| `CryptoConstants::Constants::SHA256_DIGEST_LENGTH_BYTES = 32` | `Cryptography/CryptoConstants.h:31` | `sha2::Sha256::output_size() == 32` (compile-time via `digest::OutputSizeUser`) | ✅ | Constant facts; not a divergence. |
| `OpenSSLCrypto::threadsSetup` (legacy <1.1) | `Cryptography/OpenSSLCrypto.cpp` | n/a | ✅ | Not needed: Rust crypto uses `sha2`, `aes-gcm`, `hmac`, `ed25519-dalek` — none require OpenSSL global init. |
| `Argon2::Hash` | `Cryptography/Argon2.cpp` | NONE | ❌ | Cross-reference `crypto.md` §13. Acceptable gap (BNet v2 2FA only). |
| `WPAssert(cond)` | `Debugging/Errors.h:56` | `assert!(cond)` | ✅ | Both abort+log on fail. Rust panic carries backtrace via `RUST_BACKTRACE=1`. |
| `Trinity::Fatal(...)` (logs+breakpoint+exit) | `Debugging/Errors.cpp` | `panic!(...)` + `wow_logging::install_panic_hook_like_cpp()` | ⚠️ | Rust panics now pre-log into `tracing` with `fatal=true` and source location before the standard panic output. Remaining divergence: no 10s sleep, no null-deref crash shim, no `GetDebugInfo()` stack dump. |
| `GetDebugInfo()` (stack capture for ASSERT) | `Debugging/Errors.cpp:50-110` (libunwind) | NONE | ⚠️ | Rust panic backtrace covers this; less rich (no per-thread state dump) but adequate. |
| `Encoding::Base32` | `Encoding/Base32.cpp` | NONE | ❌ | Only used by TOTP; deferred. |
| `Encoding::Base64` | `Encoding/Base64.cpp` | `base64` cargo crate | ✅ | Used in `bnet-server/rest/`. |
| `Encoding::Hex` (`Util.cpp::ByteArrayToHexStr`) | `Utilities/Util.cpp:849-869` | `hex` cargo crate | ✅ | `hex::encode` / `hex::decode`. |
| `IpLocationStore::Load` (CSV parser) | `IPLocation/IPLocation.cpp:36-107` | `wow_core::IpLocationStore::from_csv_like_cpp`; BNet and worldserver load `IPLocationFile` | ✅ | Shared store is used by auth paths; GM command wiring remains #COMMON.9c. |
| `IpLocationStore::GetLocationRecord(ip)` (binary search) | `IPLocation/IPLocation.cpp:109-125` | `wow_core::IpLocationStore::country_for_ip_like_cpp` | ✅ | Uses C++ half-open upper-bound semantics and rejects non-IPv4. |
| `Log` singleton + `Logger` per-filter | `Logging/Log.h:51-100` | `tracing::subscriber::set_global_default(...)` | ✅ | Different abstraction (event/span vs filter/appender); functionally equivalent for stdout. |
| `AppenderConsole` (ANSI colours) | `Logging/AppenderConsole.cpp:60-180` | `tracing-subscriber::fmt::Layer.with_ansi(true)` | ✅ | — |
| `AppenderFile` (rotating, %s strftime substitution) | `Logging/AppenderFile.cpp:30-127` | NONE | ❌ | Sub-task #COMMON.3. Production logs to stdout only. |
| `AppenderDB` (insert into auth.logs) | `Logging/AppenderDB.cpp` (in `shared/Logging/` actually, but wired here) | NONE | ❌ | Sub-task #COMMON.4. |
| `Metric::ScheduleSend` (InfluxDB push) | `Metric/Metric.cpp` | NONE | ❌ | Sub-task #COMMON.10. No metrics at all. |
| `MetricData` MPSCQueue | `Metric/Metric.h:65` (intrusive) | n/a | ❌ | Same. |
| `Platform/ServiceWin32.cpp` | (Win-only) | n/a | ✅ | Linux deployment; not needed. |
| `LockedQueue<T>` | `Threading/LockedQueue.h:30-100` | `parking_lot::Mutex<VecDeque<T>>` ad-hoc | ✅ | — |
| `MPSCQueueIntrusive` (Vyukov) | `Threading/MPSCQueue.h:93-175` | `flume::unbounded()` (`wow-network/src/world_socket.rs:613-616`) | ✅ | Lock-free, MPMC. C++ intrusive variant is a pointer-saving optimisation; Rust drops it for clarity. |
| `ProducerConsumerQueue<T>` (bounded) | `Threading/ProducerConsumerQueue.h` | `flume::bounded::<T>(256)` | ✅ | Same capacity. |
| `ThreadPool::Run(N)` | `Threading/ThreadPool.h:30-48` | `tokio::runtime::Builder::new_multi_thread().worker_threads(N)` | ✅ | — |
| `SetProcessPriority` | `Threading/ProcessPriority.cpp:30-90` | NONE | ❌ | Low priority. |
| `getMSTime()` | `Time/Timer.h:33-38` | `wow_core::time::ServerTime::elapsed_ms()` (`time.rs:17-19`) | ✅ | Both monotonic from app start. **Caveat**: Rust returns `u64`, C++ returns `uint32` — the `as u64` cast in `time.rs:18` is correct, but any port that reads a C++ `getMSTime()` value off the wire/disk and expects 32-bit wrapping behaviour will need `as u32`. |
| `IntervalTimer::Update / Passed / Reset` | `Time/Timer.h:62-100` | `wow_core::IntervalTimer::{update,passed,reset}` | ✅ | Signed diff update, negative clamp, pass threshold, current/interval accessors and overshoot-preserving reset are covered by focused tests. |
| `Timezone` (DST/offset) | `Time/Timezone.cpp:30-180` | partial — `GameTime::to_packed` uses libc `localtime_r`; no public offset helper API | ⚠️ | Packed time no longer has 365.25-day drift; broader timezone helper API remains open. |
| `Tokenize(str, sep, keepEmpty)` | `Util.cpp:56-72` | `str.split(sep).filter(...).collect()` | ✅ | Idiomatic. |
| `strToUpper(s)` | `Util.cpp:481` (`std::transform` with `charToUpper`) | `s.to_ascii_uppercase()` | ✅ | ASCII-only. Identical for ASCII input. |
| **`Utf8ToUpperOnlyLatin(string&)` / Basic-Latin lower helper** | **`Util.cpp:795-804`, `Util.h:124-130`, `Util.h:280-285`** | **`wow_core::utf8_to_upper_only_latin_like_cpp`; `wow_core::utf8_to_lower_only_latin_like_cpp`; `wow-crypto/src/srp6.rs` uses upper for `compute_x` and `compute_client_evidence`** | **✅** | **C++ contrast corrected stale docs: only ASCII Basic Latin is uppercased by `Utf8ToUpperOnlyLatin`; Rust lower helper is a deliberate symmetric boundary, not C++ `wcharToLower`'s broader Latin-1 behavior.** |
| `WStrToUtf8` / `Utf8toWStr` | `Util.cpp:401-477` (uses `utf8cpp`) | NONE — Rust `str` is UTF-8 natively | ✅ | Conceptually obviated by Rust's encoding model. |
| `wstrCaseAccentInsensitiveParse` (per-locale) | `Util.cpp:484-758` | NONE | ❌ | Used by chat search/filter for fr/de/es/it. Not yet a blocker — chat filtering is not yet implemented. |
| `RemoveCRLF(s)` | `Util.cpp:839-847` | `s.trim_end_matches(['\r', '\n'])` | ✅ | — |
| `StringEqualI(a,b)` | `Util.cpp:891-894` | `a.eq_ignore_ascii_case(b)` | ✅ | ASCII-only on both sides. |
| `urand(a,b)` / `irand` / `frand` / `rand_norm` / `rand_chance` | `Random.cpp:30-90` | `wow_core::random::*_like_cpp` | ✅ | Helper semantics centralized; PRNG algorithm still differs (ChaCha12/thread_rng vs SFMT). Sub-task #COMMON.14. |
| `urandweighted` | `Random.cpp:80-95` | `wow_core::urandweighted_like_cpp` | ✅ | Uses `rand::distributions::WeightedIndex`, same weighted-index contract; PRNG algorithm differs. |
| `RandomEngine::Instance()` | `Random.h:67-77` | `rand::thread_rng()` | ✅ | Thread-local. |
| `SFMTRand` | `Utilities/SFMTRand.cpp` | NONE | ⚠️ | Different PRNG. |
| `Hash::hash_combine` | `Utilities/Hash.h:28-31` | NONE | ⚠️ | Use derive `Hash` + `(A,B)` tuples. |
| `HashFnv1a(s)` | `Utilities/Hash.h:33-42` | NONE | ❌ | Not used in Rust yet. |
| `StringFormat(fmt, args)` | `Utilities/StringFormat.h:30-90` | `format!(fmt, args)` | ✅ | — |
| `StringTo<T>(s)` | `Utilities/StringConvert.h:60-200` | `str::parse::<T>()` | ✅ | — |
| `DEFINE_ENUM_FLAG(E)` | `Utilities/EnumFlag.h:26` | `bitflags!` macro | ✅ | — |
| `SmartEnum::ToString(E::A)` (reflection) | `Utilities/SmartEnum.h:30-129` | NONE | ⚠️ | Use `Debug` derive or hand-rolled `as_str()`; no auto-generated table. |
| `EventMap::ScheduleEvent(id, ms)` | `Utilities/EventMap.cpp:30-219` | NONE | ❌ | Sub-task #COMMON.12. |
| `EventProcessor::AddEvent` | `Utilities/EventProcessor.cpp:30-133` | NONE | ❌ | — |
| `TaskScheduler::Schedule(2s, fn)` | `Utilities/TaskScheduler.cpp:30-240` | NONE | ❌ | Sub-task #COMMON.11; **affects spell-script port**. |
| `MessageBuffer` (read/write cursor) | `Utilities/MessageBuffer.h:30-139` | `Vec<u8>` + manual offset, or `bytes::BytesMut` | ✅ | Inline in `wow-packet`. |
| `FuzzyFindIn(container, needles, contains, bonus)` | `Utilities/FuzzyFind.h:30-57`, `Util.cpp:896-900` | `wow_core::fuzzy_find_in_like_cpp` / `fuzzy_find_in_with_bonus_like_cpp` | ✅ | Ported as substring-count ranking, not Levenshtein. Command consumers still pending under `commands.md`. |
| `Locales::LocaleNames[]` | `Utilities/Locales.cpp:30-50` | `wow-constants::Locale` enum | ✅ | — |
| `EndianConvert<T>(&x)` | `Utilities/ByteConverter.h:30-67` | `u32::to_le_bytes` etc. builtins | ✅ | — |
| `Optional<T>` | `Utilities/Optional.h:24` | `Option<T>` | ✅ | builtin |
| `EnumUtils::Iterate<E>()` | `Utilities/SmartEnum.h:60-100` | `strum::IntoEnumIterator` (external crate) | ⚠️ | Not currently a dep. |
| Tests covering common-layer behaviour | None in C++ for `Util.cpp::Utf8ToUpperOnlyLatin`; SRP6 tests indirectly cover it | focused `wow-core::string` vectors plus SRP6 self-tests | ✅ for this helper | Tests assert Basic-Latin-only behaviour and no Unicode expansion. |

### 13.2 Critical findings

1. **✅ `Utf8ToUpperOnlyLatin` is now represented, and the previous Latin-1 finding was wrong.**
   - **C++ refs:** `src/common/Utilities/Util.cpp:795-804`, `Util.h:124-130`, `Util.h:280-283`.
   - **Correct semantics:** C++ converts UTF-8 to wide chars, then calls `wcharToUpperOnlyLatin`. Despite the name, that helper first checks `isBasicLatinCharacter`, which only returns true for `A-Z/a-z`. Therefore `caféÀßÿ` becomes `CAFéÀßÿ`, not `CAFÉÀẞŸ`.
   - **Rust refs:** `wow_core::utf8_to_upper_only_latin_like_cpp`; `wow-crypto/src/srp6.rs` uses it for Grunt SRP6 `compute_x` and `compute_client_evidence`.
   - **Why this matters:** `str::to_uppercase()` would be wrong because it can expand or alter Unicode. `to_ascii_uppercase()` happened to match valid UTF-8 content, but a named helper prevents future drift and documents the C++ behaviour.

2. **⚠️ `IPLocation` common store exists and auth paths use it; GM commands remain open.**
   - **Done:** `wow_core::IpLocationStore` ports the C++ numeric IPv4 CSV parser and `upper_bound`-style lookup. BNet auth and world auth both load/use `IPLocationFile` for `lock_country` parity.
   - **Still open:** GM account lock-country commands are future AccountMgr/RBAC work. Sub-task #COMMON.9c.

3. **❌ `Metric` (InfluxDB push) is missing entirely.**
   - C++ pushes `players_online`, `tick_diff_ms`, `map_load_ms`, `db_query_ms` etc. to InfluxDB every N seconds.
   - Rust has no equivalent. Operations cannot dashboard server health beyond `tracing` log lines.
   - Sub-task #COMMON.10.

4. **❌ `AppenderFile` and `AppenderDB` are missing.**
   - Production logs go to stdout only (whatever `init_logging()` writes).
   - No rotating file appender → systemd journal must handle rotation (acceptable with `journald` but not portable).
   - No DB-backed audit log → GM-action retention requirement (TC writes high-severity events to `auth.logs`) is not met.
   - Sub-tasks #COMMON.3, #COMMON.4.

5. **⚠️ Full `Timezone` helper API remains partial.**
   - `wow-core::GameTime::to_packed()` now uses libc `localtime_r` and real calendar fields. Remaining work is the C++ `Timezone` offset/DST helper surface, not packed-time date math.

6. **⚠️ `Trinity::Fatal` is only partially mirrored.**
   - Rust panics now pre-log through `tracing` via `wow_logging::install_panic_hook_like_cpp()` in both server binaries. Remaining divergence is intentional: Rust keeps its configured unwind/abort behavior instead of TrinityCore's sleep + forced crash path, and there is no `GetDebugInfo()` stack-dump equivalent beyond Rust backtraces.

7. **⚠️ `wow-config` lowercases keys.**
   - Currently safe — no config file relies on case sensitivity — but future divergence risk if someone adds two keys differing only in case.

8. **⚠️ `ScanLocalNetworks` partial.**
   - `bnet-server` and `world-server` now share C++-like address selection plus Unix IPv4/IPv6 interface scanning. The active BNet/world callers still operate on IPv4 realm addresses, Windows parity remains open, and if IPv4 scanning returns no usable networks Rust falls back to the old `/24` approximation. Sub-task #COMMON.18b.

9. **⚠️ `urandweighted` is open-coded at every loot site.**
   - Centralise into `wow-core::random::weighted_choice` to avoid drift. Sub-task related to #COMMON.13.

10. **⚠️ `SFMTRand` reproducibility.**
    - Rust uses ChaCha12 (`thread_rng()`), C++ uses SFMT. Loot rolls cannot be replayed across implementations. Acceptable as long as no test depends on it. Sub-task #COMMON.14.

### 13.3 Recommended action — priority queue

1. **`#COMMON.3` + `#COMMON.4` — file & DB appenders (HIGH for production, MED for dev).** Plug `tracing-appender::rolling::daily` for files; add a thin `tracing-subscriber` layer that writes WARN+ to `auth.logs` via `wow-database`.
2. **`#COMMON.10` — Metric push (HIGH for production).** Either Prometheus pull (`metrics-exporter-prometheus`) or InfluxDB push. Decide before launch.
3. **`#COMMON.9c` — IPLocation GM command wiring (MED).** Reuse `wow_core::IpLocationStore` from future AccountMgr/GM commands.
4. **`#COMMON.5` + `#COMMON.6` — config reload + env-var override (MED).** Standard production deployments expect both.
5. **`#COMMON.11` — `TaskScheduler` port (XL, but unblocks spell-script porting).**
6. Remaining items are quality-of-life: migrate random call sites to the central helpers where useful, `TimeTracker`/`PeriodicTimer`, full `Timezone` helper API.

### 13.4 Justifying the status badge

`⚠️ partial` is correct. The Asio↔Tokio replacement is genuinely complete and idiomatic; same for queues, mutexes, central random helper semantics, encoding, errors, `IntervalTimer`, packed-time calendar math, Grunt SRP6 Basic-Latin string normalisation, IPLocation store/lookup for BNet/world auth, and Unix IP network scanning. Where Rust simply uses the corresponding crate (`tokio`, `flume`, `parking_lot`, `dashmap`, `rand`, `hex`, `regex`, `bitflags`) the migration is real and ✅. The ⚠️ comes from three genuine gaps: (a) remaining `IPLocation` GM command wiring (forensics-affecting but blocked on AccountMgr/RBAC), (b) `Metric` (ops-affecting), (c) the logger framework gaps (file appender, DB appender). Plus the smaller deferred items (`TaskScheduler`, `EventMap`, `TimeTracker`/`PeriodicTimer`, full `Timezone` helper API).

Recommendation: keep ⚠️ partial until either the production gaps (#COMMON.3/#COMMON.4 logging, #COMMON.9c IPLocation GM command wiring, #COMMON.10 metrics) are ported or explicitly carved out as `wow-ops` future work with owner/sign-off. Do not promote Common to ✅ solely because the SRP6 helper and common IPLocation auth paths are closed.

---

*Template version: 1.0 (2026-05-01).*
