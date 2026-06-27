# Migration: Performance Strategy (cross-cutting)

> **C++ canonical path:** N/A — this is a meta-doc consolidating perf goals across all per-module audits in `/home/server/rustycore/docs/migration/`.
> **Rust target crate(s):** workspace-wide (`crates/*` plus per-crate `benches/` to be created)
> **Layer:** L0–L8 (cross-cutting)
> **Status:** ❌ not started — zero `criterion` benches present; targets defined per-module but never measured.
> **Audited vs C++:** N/A (meta-doc)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Consolidate every "Benchmark X vs C++" item that per-module audits have generated (e.g. `#PACKETS.6 Benchmark serialization Rust vs C++ on packet típico 512 bytes`, `#CRYPTO.12 Benchmark SRP6 verify vs C++`, `#MAPS.N Benchmark grid-update at 64×64`) into one performance plan: target tps, target latency, packet/sec, concurrent sessions, MapManager update frequency, criterion-bench layout, and the profiling-tool kit needed to validate them.

The Rust port's headline value-prop over the C# original (and over C++ TC) is throughput-per-core and tail-latency. Without measurement, that claim is theatre. This doc specifies the measurement.

---

## 2. Current state (consolidated from §11 of every per-module doc + AGENTS.md)

- **No criterion benches exist.** `grep criterion crates/*/Cargo.toml` returns zero hits. No `benches/` directory at workspace root or in any crate.
- **No live-fire load test.** No equivalent of `playerbots` driving the Rust server has been run.
- **Release profile is tuned for binary size + perf**: `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` (AGENTS.md "Build / test"). Release binaries ≈ 10 MB.
- **Per-module benchmark goals already named:**
  - `#PACKETS.6` — serialization Rust vs C++ on a typical 512-byte packet.
  - `#CRYPTO.12` — SRP6 verify vs C++.
  - `#CRYPTO.5` — AES round-trip throughput (implicit; the test exists, the bench does not).
  - `#MAPS.*` — grid update at 64×64; `MapManager` per-tick cost.
  - `#NETWORK.*` — concurrent connection handling; per-session memory.
  - `#DB.*` — prepared-statement latency; connection-pool saturation.
  - `#HANDLERS.*` — dispatch-table lookup cost vs C++ switch.
- **Profiling tooling: not installed in repo.** No `flamegraph` config, no `tokio-console` integration in `wow-logging`, no `perf` runbook.
- **Realistic load profile not defined.** Production target ("wowchad.work.gd", per AGENTS.md "Runtime") sets no published concurrency/tps targets.

---

## 3. Architecture (proposed bench + profile layout)

```
┌────────────────────────────────────────────────────────────────┐
│ MACRO-LOAD     tests/load/                                     │
│                k6 / locust-style scripted clients,             │
│                docker-compose mariadb + world-server,          │
│                report tps, p50/p95/p99 latency, RAM, CPU       │
├────────────────────────────────────────────────────────────────┤
│ MICRO-BENCH    crates/<crate>/benches/*.rs (criterion)         │
│                one bench file per hot path                     │
│                criterion HTML reports → target/criterion/       │
├────────────────────────────────────────────────────────────────┤
│ PROFILE        ad-hoc: cargo flamegraph, perf record,          │
│                tokio-console (live), heaptrack (alloc),        │
│                samply (cross-platform alt to perf)             │
├────────────────────────────────────────────────────────────────┤
│ RUNTIME OBS    tracing spans + metrics (histogram, counter)    │
│                exposed via /metrics on bnet-server :8081       │
│                prometheus scrape; grafana dashboard            │
└────────────────────────────────────────────────────────────────┘
```

The four layers serve different questions:

- **MACRO-LOAD** answers "does this build hold up to N players for M minutes?" — the only layer that produces marketing-grade numbers.
- **MICRO-BENCH** answers "did this PR regress the inner loop?" — runs in CI on every push; alerts on >5% regression.
- **PROFILE** answers "where is the time going?" — engineer-driven, ad-hoc, not in CI.
- **RUNTIME OBS** answers "is production healthy?" — live, exposed, scraped.

---

## 4. Key practices

- **Targets first, code second.** Each module that lands a bench commits to a number (e.g. "SRP6 verify ≤ 200 µs single-thread on Ryzen 7 5800X"). Numbers are written down, not negotiated post-hoc. Tracked in §10 below.
- **Headline server targets** (consolidated; revise once measured):
  - **1000 concurrent world sessions** per `world-server` instance, single host, 8 physical cores.
  - **Per-session steady-state CPU**: ≤ 0.5 % of one core (i.e. 1000 sessions ≤ 5 cores; 3 cores headroom).
  - **Per-session RAM**: ≤ 2 MB resident.
  - **World-tick latency**: 50 ms cadence (20 ticks/s) maintained at full load. p99 tick wall-time ≤ 25 ms (half the budget).
  - **Packet send throughput**: ≥ 50 k packets/s aggregate. Median `send_packet` cost ≤ 5 µs.
  - **Login throughput**: ≥ 50 logins/s sustained; p99 SRP6 verify ≤ 5 ms.
  - **MapManager update**: 64×64 grid for one map updated within ≤ 10 ms when half full (~2k entities).
- **Headline DB targets**:
  - Prepared-statement round-trip p99 ≤ 2 ms on a co-located MariaDB.
  - Connection-pool max wait p99 ≤ 1 ms (i.e. pool is never saturated under target load).
- **Criterion conventions:**
  - One `benches/<area>.rs` file per hot path (`benches/packet_serialize.rs`, `benches/srp6_verify.rs`, `benches/map_tick.rs`).
  - Bench groups named after the function under test, not the file.
  - Inputs come from the same `tests/golden/data/*.bin` fixtures the regression suite uses (see `migration-test-strategy.md` §3 L2). One source of truth.
  - Every bench writes its baseline to `target/criterion/<name>/base/`; CI compares against the committed baseline in `docs/migration/_perf-baseline/`.
- **No microbench is allowed to allocate inside the hot loop**. Use `criterion::black_box` and pre-allocated buffers; if your code path requires an allocation, that's the bench's first finding.
- **Rust vs C++ comparisons are run on the same machine, same kernel, same MariaDB.** Cross-machine numbers are noise. Document the bench host's CPU/RAM/kernel/MariaDB version in the bench's `.meta` file (mirror the test-strategy convention).
- **Tail latency over throughput.** Report p50, p95, p99, p99.9. Average tps without tails is a vanity metric.
- **Profile under load, not at idle.** Flamegraphs of an idle server are useless. Profile while a load test is running.

---

## 5. Dependencies

**Tooling required:**

- `criterion = "0.5"` workspace dev-dep — micro-benches.
- `cargo-flamegraph` — sampling profiler with kernel symbols on Linux. Requires `perf_event_paranoid <= 1`.
- `samply` — cross-platform alternative to flamegraph; produces Firefox Profiler JSON.
- `perf` (linux-tools) — counters, sched events, cache misses.
- `tokio-console` — async-task introspection, requires `console-subscriber` integrated in `wow-logging` behind a feature flag.
- `heaptrack` (Linux) — allocation tracking; useful one-shot diagnostic.
- `bytehound` (alt) — same.
- `prometheus = "0.13"` + `metrics = "0.23"` + `metrics-exporter-prometheus = "0.15"` — runtime metrics.
- `axum-prometheus` (opt) — auto-instruments the existing Axum REST surface (`bnet-server` :8081 per AGENTS.md).
- `criterion-cycles-per-byte` (opt) — RDTSC-based fine-grained measurement.

**Load-test tooling:**

- `k6` (Go-based, JS-scripted) — for the MACRO-LOAD layer; or `goose` (Rust); or a hand-written async-Rust scripted client that reuses `wow-network` to talk the actual game protocol. The scripted-Rust option is the most accurate but the most expensive.
- `playerbots` (the C++ TC playerbot fork) can be pointed at the Rust server for human-realistic load — same wire protocol, free realism.

**External required:**

- A reference C++ TC build at `/home/server/woltk-trinity-legacy/` (already cloned per AGENTS.md) compiled in `Release` to produce comparison numbers.
- Same MariaDB instance for both Rust and C++ comparison runs.
- A reproducible bench host: pinned CPU governor (`cpupower frequency-set -g performance`), turbo-boost off for variance reduction, isolated cores (`isolcpus`) optional for reproducibility.

---

## 6. SQL / DB queries

N/A as a producer; `wow-database` perf concerns route here through `#DB.*` items (prepared-statement latency, connection-pool saturation, transaction throughput). Specific bench targets:

- **`SEL_CHARACTER_BY_ACCOUNT`** p50 ≤ 200 µs, p99 ≤ 1.5 ms.
- **`SEL_CREATURE_TEMPLATE_BY_ENTRY`** (cache-warm) p50 ≤ 50 µs (in-memory cache hit), p99 ≤ 500 µs.
- **`UPD_CHARACTER_POSITION`** (write-heavy in production) p50 ≤ 500 µs, p99 ≤ 3 ms.
- **`INS_LOG`** (AppenderDB, see `logging.md`) p99 ≤ 5 ms — async; should not stall caller.

---

## 7. Wire-protocol packets

N/A as producer. Per-opcode perf budgets:

- **`SMSG_UPDATE_OBJECT`** (the heaviest opcode by volume): serialize p50 ≤ 20 µs for one entity, ≤ 200 µs for a 50-entity batch.
- **`SMSG_TIME_SYNC_REQ`**: ≤ 1 µs (trivial 4-byte payload).
- **`CMSG_MOVE_*`** decode: ≤ 5 µs per packet.
- **`SMSG_LOGIN_VERIFY_WORLD`** end-to-end (handle to first byte on wire): ≤ 500 µs.
- **AES-GCM encrypt** of a 512-byte packet: ≤ 5 µs (target equals or beats C++ OpenSSL on same key schedule).

---

## 8. Current state in RustyCore

**Files / artifacts in `/home/server/rustycore`:**

- Workspace `Cargo.toml`: release profile already optimised (`lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`). No `[[bench]]` blocks anywhere.
- `crates/*/Cargo.toml`: zero bench targets, zero criterion deps.
- `crates/*/benches/`: directory does not exist anywhere.
- `crates/wow-logging/src/lib.rs`: `init_logging` does **not** install `console-subscriber`, so `tokio-console` cannot attach today.
- `bnet-server` exposes Axum on :8081 (REST + auth) per AGENTS.md but does **not** expose `/metrics`.
- `world-server` has no metrics endpoint.

**What's implemented:**

- Tuned release profile (the foundation).
- The hot data structures recommended by AGENTS.md (`parking_lot`, `dashmap` over std). These are perf prerequisites; the structures alone don't prove the perf number.
- `MapManager` is in-tree (12 tests, ~890 lines per AGENTS.md), `parking_lot::RwLock` shaped, but its grid-update cost is unmeasured.
- `tokio` runtime with `rt-multi-thread`. Single global runtime, no per-tenant runtime. Number of worker threads not tuned per workload.

**What's missing vs C++ / vs target:**

- All benches.
- All target numbers in §4 are aspirational — none has been measured.
- No load-test scaffolding.
- No metrics / no Prometheus / no Grafana dashboard.
- No flamegraph runbook.
- No `tokio-console` opt-in.
- No CPU pinning / NUMA awareness in the runtime config (probably premature, but worth noting).

**Suspicious / likely divergent (hypotheses pre-measurement):**

- **Per-session memory** is likely well above the 2 MB target today because `WorldSession` still carries the deprecated `creatures: HashMap<ObjectGuid, CreatureAI>` per AGENTS.md "Creature storage" section — every session holds a copy of every visible creature. The MapManager migration directly addresses this.
- **`send_packet`** (tick-time) currently double-borrows `&mut self` in some paths per AGENTS.md "Patterns to follow" — the workaround is `send_tx`. Suggests the hot path has avoidable lock churn that profiling will expose.
- **DB latency at the moment** is likely fine (sqlx + MariaDB on localhost is fast) but **prepared-statement compile time** at startup might dominate cold-start; measure.
- **AES-GCM throughput** with the `aes-gcm` crate (not OpenSSL) on a non-AES-NI host can be 5-10× slower than C++ OpenSSL. On AES-NI it's competitive. Measurement first.
- **Tracing overhead** in hot paths: the structured `log_filter` field per log call (per `wow-logging/src/lib.rs:194-213`) allocates if formatting kicks in. Profile.

**Tests existing:**

- 395 unit tests but 0 benchmarks (AGENTS.md "Build / test").

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md` and from per-module audits' `Benchmark` items.

Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h).

- [ ] **#PERF.1** Add `criterion = "0.5"` to workspace `[workspace.dependencies]` and `dev-dependencies`. Configure `harness = false` for bench targets. (L)
- [ ] **#PERF.2** Create `docs/migration/_perf-baseline/` directory and document the canonical bench host (CPU/RAM/kernel/MariaDB version). All committed baselines reference this host. (L)
- [ ] **#PERF.3** First bench: `crates/wow-crypto/benches/srp6.rs` — verify single login, batch 100 logins. Resolves `#CRYPTO.12`. (M)
- [ ] **#PERF.4** `crates/wow-crypto/benches/aes_gcm.rs` — encrypt 64B / 256B / 512B / 4KB / 16KB. Compare AES-NI vs software fallback. Resolves implicit `#CRYPTO.5` perf side. (M)
- [ ] **#PERF.5** `crates/wow-packet/benches/serialize.rs` — typical `SMSG_UPDATE_OBJECT` of 1 / 10 / 50 entities; 512-byte synthetic packet. Resolves `#PACKETS.6`. (M)
- [ ] **#PERF.6** `crates/wow-world/benches/map_tick.rs` — MapManager grid update at 0%/25%/50%/100% fill. Includes `get_visible_creatures` 3×3 window cost. Resolves `#MAPS.*` perf. (H)
- [ ] **#PERF.7** `crates/wow-handler/benches/dispatch.rs` — opcode-table lookup vs C++ switch (synthetic; the dispatch table is `inventory`-backed). (M)
- [ ] **#PERF.8** `crates/wow-database/benches/prepared_stmt.rs` — `SEL_CHARACTER_BY_ACCOUNT`, `UPD_CHARACTER_POSITION` round-trip vs co-located MariaDB. (M)
- [ ] **#PERF.9** Add `console-subscriber` integration to `wow-logging` behind a `tokio-console` feature flag; document attach instructions. (M)
- [ ] **#PERF.10** Add `metrics` + `metrics-exporter-prometheus` to `world-server`, expose `/metrics` on a new admin port (e.g. :8087); first metrics: tick latency histogram, per-session count gauge, packets-sent counter. (H)
- [ ] **#PERF.11** Add `axum-prometheus` to `bnet-server` :8081 to expose REST latency/error counters. (M)
- [ ] **#PERF.12** Write a flamegraph runbook (`docs/runbooks/flamegraph.md`): how to attach `cargo flamegraph` to a running `world-server`, including required `perf_event_paranoid` and `kptr_restrict` settings. (M)
- [ ] **#PERF.13** Build a scripted-Rust load client under `tests/load/` that reuses `wow-network` to drive N synthetic sessions through login → enter-world → tick for M minutes. Reports tps, p50/p95/p99 latency, RAM, CPU. (XL — split per phase: login load, enter-world load, full-tick load.)
- [ ] **#PERF.14** Measure baseline numbers on the bench host for §4 targets; commit results to `docs/migration/_perf-baseline/2026-05-baseline.md`. Compare against C++ TC `Release` build on same host. (H)
- [ ] **#PERF.15** Add CI perf-regression job: run a subset of micro-benches (`#PERF.3`–`#PERF.7`), fail if any group regresses >5% vs committed baseline. (M)
- [ ] **#PERF.16** Per-session memory profile via `heaptrack`: identify top 10 allocations in a 1000-session steady-state. Compare pre- and post-MapManager-migration. (H)
- [ ] **#PERF.17** Tokio runtime tuning: investigate split runtime (one for I/O, one for game tick) vs single shared runtime; benchmark both. (H)
- [ ] **#PERF.18** Hot-path tracing audit: `cargo flamegraph` of a busy `world-server`; identify `tracing::*!` calls in the hot loop and gate behind compile-time level filter. (M)

---

## 10. Regression tests to write (perf invariants)

Tracked as "the number must not regress" guardrails, not pass/fail tests.

- [ ] SRP6 verify single login p99 ≤ 5 ms (host-pinned).
- [ ] AES-GCM encrypt 512 B p50 ≤ 5 µs.
- [ ] `SMSG_UPDATE_OBJECT` 50-entity batch serialize p50 ≤ 200 µs.
- [ ] MapManager `get_visible_creatures` p99 ≤ 50 µs (3×3 window, half-full grid).
- [ ] Prepared-statement `SEL_CHARACTER_BY_ACCOUNT` p99 ≤ 1.5 ms (warm pool).
- [ ] World tick wall-time p99 ≤ 25 ms at 1000-session steady state.
- [ ] Per-session RSS ≤ 2 MB at 1000-session steady state.
- [ ] CPU steady-state ≤ 0.5 % per session at 1000-session.
- [ ] No allocation in `tick_combat_sync` hot path (verify via `dhat` or instrumented allocator in a debug build).
- [ ] `wow-handler` dispatch lookup ≤ 100 ns p99.

Each regression "test" is enforced by `#PERF.15` (CI perf-regression job).

---

## 11. Notes / gotchas

1. **Numbers in §4 are unmeasured targets, not facts.** The first run of `#PERF.14` will produce real numbers, which will replace these. Until then, treat §4 as goals to design against.
2. **Variance kills micro-benches.** On a developer laptop with thermal throttling, criterion can swing ±20 % run-to-run. Use a dedicated bench host with pinned governor; otherwise, treat micro-bench results as relative (PR vs main), not absolute.
3. **The 1000-session target is per `world-server` instance.** Sharding across instances per realm is the production strategy; one process is not expected to hold a full retail-sized realm.
4. **`tokio-console` is not free.** It adds a per-task overhead and is gated behind a feature flag for that reason. Never ship a release build with it enabled.
5. **`tracing` overhead is non-zero even when filtered out.** A `log_network!(trace, ...)` macro call still constructs the format args. For sub-µs hot paths, gate trace-level calls with `if tracing::enabled!(Level::TRACE)`.
6. **Compare against C++ on the same host with the same libc, same kernel, same MariaDB build.** A C++ build with `-O3 -flto` against a Rust `Release` build with `lto = "thin"` is fair. A C++ build with `-O0` is not, and a Rust `dev` build is not.
7. **`MapManager` is not yet wired into the live tick path** (AGENTS.md "Creature storage" section). Bench numbers for it today reflect the standalone module, not the in-flight game state. Re-bench after the migration is complete.
8. **`PROTOC=/home/cdmonio/.local/protoc/bin/protoc`** is required to build the workspace, hence required to run benches. CI configs for perf must set it.
9. **Prefer `samply` over `perf` on hosts where `perf_event_paranoid > 1`** and you can't change it. Same flamegraph output, different permissions model.
10. **The C# legacy at `/home/server/woltk-server-core/Source/`** (the original C# port that this Rust project replaces) is **not** the comparison baseline. The C++ TC at `/home/server/woltk-trinity-legacy/` is. The C# numbers are uninteresting except as a "must-beat-this" floor.

---

## 12. C++ → Rust mapping (high-level)

| C++ TC perf machinery | Rust equivalent | Notes |
|---|---|---|
| TC `WorldSession::Update` cost | `WorldSession::tick_*` (in `wow-world`) | Compare via `#PERF.6` + integrated tick bench. |
| TC `MapManager::Update` (per map per tick) | `MapManager::tick` (in-tree, per AGENTS.md) | `#PERF.6`. |
| TC OpenSSL EVP_Cipher AES-GCM | Rust `aes-gcm` crate (RustCrypto) | AES-NI parity expected; software-fallback gap is real. `#PERF.4`. |
| TC `BigNumber` (BN_mod_exp) | Rust `num_bigint::BigUint` | `num_bigint` is generally 1.5-3× slower than OpenSSL BN. SRP6 verify is one-time-per-login so unlikely to dominate; bench to confirm. `#PERF.3`. |
| TC SQL prepared statements (mysql C client) | Rust `sqlx::Pool<MySql>` | Compare round-trip latency. `#PERF.8`. |
| TC packet serialization (manual `ByteBuffer`) | Rust `wow-packet::WorldPacket` (per AGENTS.md) | `#PERF.5`. |
| TC `Trinity::Asio` strands | Rust `tokio` tasks + per-session `mpsc` | Different scheduler model; tail latency comparison matters more than throughput. |
| TC `playerbots` load harness | Rust scripted client at `tests/load/` (`#PERF.13`) | Or point existing `playerbots` at Rust server (same wire protocol). |
| TC `gprof` / `valgrind --callgrind` | `cargo flamegraph`, `samply`, `perf record` | Native equivalents. `#PERF.12`. |
| TC `valgrind --tool=massif` | `heaptrack` / `bytehound` | `#PERF.16`. |
| TC homemade metrics (counters in code) | `metrics` crate + Prometheus exporter | `#PERF.10`, `#PERF.11`. |

---

## 13. Audit (2026-05-01)

> Audited 2026-05-01. Scope: this strategy doc itself, against the perf claims in (a) `AGENTS.md`, (b) per-module §11/§13 sections, and (c) the actual repo at `/home/server/archived/rustycore_ARCHIVED_20260312/`. Not audited against C++ — measurement is the explicit gap this doc creates work to close.

### 13.1 Claim verification

| Claim in this doc | Source / verified against | Status |
|---|---|---|
| Zero criterion benches in repo | `grep criterion crates/*/Cargo.toml` returned 0 hits | ✅ confirmed |
| Release profile uses `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` | `AGENTS.md` "Build / test" | ✅ confirmed |
| Release binaries ~10 MB | `AGENTS.md` "~10 MB binaries" | ✅ confirmed (claim, not re-built here) |
| `bnet-server` exposes Axum REST on :8081 | `AGENTS.md` "Runtime" | ✅ confirmed |
| `world-server` has no admin/metrics port today | Inferred from `AGENTS.md` (only :8085 / :8086 listed) | ✅ confirmed by absence |
| `wow-logging` does not install `console-subscriber` | `crates/wow-logging/src/lib.rs` `init_logging` body | ✅ confirmed |
| `MapManager` not wired into live tick path | `AGENTS.md` "Creature storage" — explicit "not yet wired" | ✅ confirmed |
| `parking_lot` / `dashmap` are workspace deps | `AGENTS.md` "Patterns to follow" | ✅ confirmed (claim) |

### 13.2 Critical findings

1. **The targets in §4 are unmeasured.** Every number ("1000 sessions", "p99 ≤ 25 ms tick", "≥ 50 k packets/s") is the author's reasonable guess derived from C++ TC heuristics and standard async-Rust capabilities. **None has been verified on this codebase.** Until `#PERF.14` runs, treat them as design constraints, not promises.
2. **No comparison plumbing today.** The doc proposes "Rust vs C++ on the same host" but neither side has bench infrastructure pointed at the other. C++ TC has unit benches but does not run them as a load harness. Rust has neither. `#PERF.13` + `#PERF.14` together close this; expect ~1-2 weeks of work to get both running on a pinned host.
3. **Flag: `tracing` overhead is unaudited.** The `wow-logging` macros (`log_network!` etc.) build a structured field on every call. In a 50 k packets/s hot loop that is non-trivial. `#PERF.18` is the audit; the fix may be a compile-time `release-max-level-info` filter on `tracing` (well-supported via `tracing/release_max_level_*` features).
4. **Flag: per-session memory is likely above target.** §8 hypothesises this based on the deprecated `WorldSession.creatures` field. Until MapManager migration completes (AGENTS.md "Creature storage"), the 2 MB/session target is unreachable. Order of operations: complete migration → measure → tune.
5. **Flag: `aes-gcm` (RustCrypto) vs OpenSSL EVP performance.** On AES-NI hosts, RustCrypto's `aes-gcm` is competitive; off AES-NI it's measurably slower. Production hosts should be assumed AES-NI-capable (any x86-64 from the last decade), but the bench (`#PERF.4`) needs to confirm.
6. **`#PERF.15` (CI perf-regression job) requires a stable bench host.** Cloud CI runners (GitHub Actions hosted runners) have ±30 % variance. Either provision a self-hosted runner pinned to a known machine, or relax the regression threshold to 20 %+ (which makes the gate near-useless). Self-hosted runner is the recommended path; `#PERF.2` documents the host.

### 13.3 Recommended action — priority queue

1. **`#PERF.1` + `#PERF.2` (foundation, ≤2 h)** — wire criterion into the workspace; document the bench host. Without these, no later number is reproducible.
2. **`#PERF.3` + `#PERF.4` + `#PERF.5` (cryptographic + packet primitives)** — these are the most-cited per-module bench items and the easiest to land. Three small bench files; ~1 day's work.
3. **`#PERF.10` + `#PERF.11` (runtime metrics)** — exposes a `/metrics` endpoint so any production deployment is observable. Independently valuable from microbenches; lands earlier in priority.
4. **`#PERF.13` + `#PERF.14` (load harness + first measurement)** — produces the first row of real numbers in §4. Replaces the targets with measurements.
5. **`#PERF.6` (MapManager bench)** — gated by the MapManager migration completing per AGENTS.md "Creature storage". Defer until that migration is wired in; otherwise the bench measures a dead path.
6. **`#PERF.15` (CI gate)** — requires `#PERF.1`–`#PERF.5` and the self-hosted runner. Land last.
7. **`#PERF.9` (`tokio-console`) + `#PERF.12` (flamegraph runbook)** — engineer-tooling, anytime.
8. **`#PERF.17` (runtime split)** — only worth investigating if `#PERF.14` shows tail-latency issues attributable to scheduler contention. Otherwise YAGNI.

### 13.4 Justifying the index status

Header status is `❌ not started` because zero of the 18 sub-tasks have been done; no bench, no metrics, no measurement. Status moves to `⚠️ partial` once `#PERF.1`–`#PERF.5` and `#PERF.10` are in place. Status moves to `✅ done` once `#PERF.14` produces measurements that confirm or refine §4 targets, `#PERF.15` is gating CI, and `#PERF.13` load harness has been run at least once at the 1000-session target. Status becomes `🔧 broken` if any committed perf baseline silently regresses without follow-up.

---

*Template version: 1.0 (2026-05-01).*
