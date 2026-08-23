# Migration: Test Strategy (cross-cutting)

> **C++ canonical path:** N/A — this is a meta-doc consolidating test approaches across all per-module audits in `/home/server/rustycore/docs/migration/`.
> **Rust target crate(s):** workspace-wide (`crates/*` plus root `tests/` if introduced)
> **Layer:** L0–L8 (cross-cutting)
> **Status:** ⚠️ partial — `cargo test --workspace` passes 395 cases per AGENTS.md but ~zero are golden-vector cross-impl tests; coverage gap is the dominant residual risk per per-module §13 audits.
> **Audited vs C++:** N/A (meta-doc)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Consolidate the `[ ] Test:` items and audit-flagged "no golden vectors" findings scattered across every per-module migration doc into one strategy. Every audit so far (`crypto.md` §13.2 #3, `proto.md`, `handlers.md`, `combat.md`, `movement.md`, `entities.md`, `accounts.md`, etc.) flags the same root cause: tests verify Rust internal consistency but not byte-equivalence with the C++ canonical implementation. This doc names the required golden-vector inventory, the fixture organisation, the regression-suite layout, and the integration-harness shape that closes that gap.

---

## 2. Current state (consolidated from §11/§13 of every per-module doc)

- **Baseline.** `PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test --workspace` → **395 passed** (AGENTS.md, "Build / test"). Per-crate: `wow-crypto` 46, `wow-world` ~120 (incl. 12 `MapManager`), `wow-packet` ~50, `wow-database` ~30, rest distributed.
- **Test categories present.**
  - Round-trip serialization tests (most `wow-packet` modules).
  - Internal consistency tests (`wow-crypto`: encrypt-then-decrypt matches plaintext, but with no C++ reference output).
  - DB query-shape tests (`wow-database/tests/*.rs` against a live MariaDB if `RUSTYCORE_TEST_DB_URL` is set; otherwise skipped).
  - Algorithmic unit tests (math, GUID, BigUint).
- **Test categories absent / flagged repeatedly.**
  - **Golden vectors captured from a real C++ TC session** (the recurring "no cross-impl test vectors" flag in §13).
  - **PCAP replay tests** (decode a real client/server capture, assert opcode/parser path matches).
  - **Integration tests** spinning a `world-server` + MariaDB pair and exercising login → enter-world.
  - **Property/fuzz tests** against malformed inputs (handler dispatch, packet decoders, SRP6 invalid `A`).
  - **Regression suite** for the bugs already fixed once in this codebase (none catalogued yet).
- **Per-module flag inventory (sample).**
  - `crypto.md` §13.3 #2: capture HMAC chain (digest, session_key, encrypt_key) from a working C++ login as hardcoded vectors.
  - `crypto.md` §13.3 #2: SRP6 `(salt, verifier)` pair from `account` table → bake into a `#[test]`.
  - `proto.md`: BNet protobuf wire vectors per opcode.
  - `handlers.md`: per-opcode round-trip with bytes captured from official client.
  - `combat.md`: damage-formula golden tables (level × armor × resist matrix).
  - `movement.md`: spline path determinism (same input → same output bytes).
  - `entities.md`: UpdateObject diff payload byte-equality on known transitions.
  - `accounts.md`: SRP6 verifier generation matches C++ for known credentials.

---

## 3. Architecture (proposed test-pyramid layout)

```
┌─────────────────────────────────────────────────────────────┐
│ L4  END-TO-END   tests/e2e/      docker-compose: mariadb +  │
│                                  world-server + bnet-server │
│                                  scripted client (auth+world)│
├─────────────────────────────────────────────────────────────┤
│ L3  INTEGRATION  tests/it/       in-process: WorldSession    │
│                                  + MapManager + real packets │
│                                  + sqlite-tempdir or MariaDB │
├─────────────────────────────────────────────────────────────┤
│ L2  GOLDEN       crates/*/tests/golden/                      │
│                  hardcoded vectors captured from C++ TC      │
│                  (one binary per protocol family)            │
├─────────────────────────────────────────────────────────────┤
│ L1  PROPERTY     crates/*/tests/prop_*.rs (proptest/quickcheck)│
│                  + fuzz/ (cargo-fuzz on dispatch + decoders) │
├─────────────────────────────────────────────────────────────┤
│ L0  UNIT         crates/*/src/**/#[cfg(test)] mod tests      │
│                  current 395-test baseline lives here        │
└─────────────────────────────────────────────────────────────┘
```

Layer rules:

- **L0 unit** lives inside each crate's source as `#[cfg(test)] mod tests`. No external deps. Fast.
- **L1 property/fuzz** lives in `crates/<crate>/tests/prop_*.rs` for proptest, plus a top-level `fuzz/` cargo-fuzz workspace member (not yet present) for dispatch-table coverage.
- **L2 golden** lives in `crates/<crate>/tests/golden/` with binary fixtures under `crates/<crate>/tests/golden/data/` and a small loader. Vectors are committed bytes (hex or base64 in `.txt`, or raw `.bin`).
- **L3 integration** lives in `tests/it/` at workspace root. Spins up WorldSession against a sqlite-or-tempdir MariaDB schema and the real packet stack, but no docker.
- **L4 end-to-end** lives in `tests/e2e/` with `docker-compose.yml` provisioning MariaDB 10.6 + the two binaries; a smoke client logs in and walks one map.

### 3.1 Existing headless client harness

The available Rust bot harness lives outside this workspace at `/home/cdmonio/projects/wow-test-bot/rust-bot/`. It was built and validated against the C++ server in `/home/server/woltk-trinity-legacy/`; the C++-side reference docs are `LFG_TEST_HARNESS.md`, `BOT_PATCHES.md`, and `sql/custom/lfg_bot_test_migrations.md`.

Use it during the port, not after, but only as an E2E regression harness. It is not a correctness oracle. Every packet layout and server behavior it relies on must still be checked against C++ first.

Known harness dependencies to port or adapt before relying on it against Rust:
- BNet bot REST endpoints: `POST /login/srp/` and `POST /login/` with the bot-specific JSON shape and `session_key` response.
- Single-socket login path for bot accounts gated by the C++ `Bot.AccountPrefix` behavior, or an equivalent bot update that opens the Rust instance socket.
- LFG client opcodes and handlers used by the harness: `CMSG_DF_SET_ROLES`, `CMSG_DF_JOIN`, `CMSG_DF_PROPOSAL_RESPONSE`, `CMSG_DF_READY_CHECK_RESPONSE`.
- Test data hygiene for the bot accounts/characters and stale `groups` / `group_member` rows.

Minimum acceptance before using it as a migration gate: bot BNet auth -> world auth -> `CMSG_ENUM_CHARACTERS` -> `CMSG_PLAYER_LOGIN` passes against Rust, then LFG-specific gates can be enabled as that subsystem is ported.

---

## 4. Key practices

- **Golden vectors are bytes, not pretty-printed text.** Capture as `.bin` files; load with `include_bytes!`. A pretty-printed dump drifts; raw bytes don't.
- **Capture once, version forever.** Each vector has a sibling `.meta` (1-line markdown) recording: capture date, C++ commit (`5100ce3d8fc6` is the audit baseline per `crypto.md` §13), client build (`3.4.3.54261`), tooling. If a vector is re-captured, the old one is kept and dated.
- **Three tiers of "real input" for golden vectors**, ranked by cost:
  1. **Hand-crafted from C++ source.** Read the C++ function, run it on paper or via a one-shot TC unit-test executable, paste result into Rust test. Cheapest for algorithmic primitives (SRP6, HMAC, AES-GCM, SHA-interleave).
  2. **C++ TC `bot debug` log lines.** TC already prints HMAC chain at `WorldSocket.cpp:755`; many handlers have similar trace points. Free vectors — just enable the right logger and grep.
  3. **PCAP captures** of a real client talking to a working C++ server, parsed offline to extract per-opcode payloads. Highest fidelity; required for opcodes whose serialization is not algorithmic (UpdateObject, MovementInfo, gossip menu trees).
- **`cargo test --workspace` stays the green-bar gate.** No new strategy is allowed to silently skip a class of tests. Slow tests (golden + integration + e2e) gate behind `#[cfg_attr(not(feature = "slow-tests"), ignore)]` so the 395-baseline runs in <30 s while opt-in CI runs the full suite.
- **One regression test per bug fixed.** Currently zero exist by name. Backfill by sweeping git log for "fix:" commits and adding a named test per fix. Going forward, no bug-fix PR lands without one.
- **Fixture co-location.** Test data files live next to the test that loads them, not in a top-level `testdata/`. The `wow-data` DBC tests are the model.
- **No flaky time-based assertions.** Fixed seeds (`StdRng::seed_from_u64(0xC0DE_F00D)`), virtual clocks (`tokio::time::pause()`), no `sleep`.

---

## 5. Dependencies

**Tooling (some present, some required):**

- **Already in workspace:** `tokio` (with `rt-multi-thread,test-util`), `tracing-subscriber` (per `wow-logging`), `sqlx` (lets us swap to sqlite for L3).
- **Required additions:**
  - `proptest = "1"` — property tests (L1).
  - `cargo-fuzz` — fuzz harnesses for dispatch table + packet decoders.
  - `criterion = "0.5"` — bench harness (covered in `migration-perf-strategy.md`, listed here because the `tests/`/`benches/` split must agree).
  - `insta = "1"` — snapshot tests for printable output (chat formatting, log output, debug dumps). Lighter weight than maintaining hardcoded strings.
  - `testcontainers = "0.20"` (opt) — programmatic Docker MariaDB for L4 if `docker compose` is too heavy.
- **External required for full coverage:**
  - A reachable C++ TC build (`/home/server/woltk-trinity-legacy` already cloned at commit `5100ce3d8fc6` per `crypto.md` §13). One-shot binaries built from it become the golden-vector source.
  - The **3.4.3.54261 client** (already required for runtime; tests reuse the same binary). PCAPs captured against this client are the truth source for handler tests.
  - MariaDB 10.6 for L3/L4 (already required for runtime).
- **Dev-only dependencies stay in `[dev-dependencies]`** of each crate's `Cargo.toml`; never leak into release builds.

---

## 6. SQL / DB queries

N/A — this is a meta-doc, not a module that emits queries. Test-suite-specific schema fixtures (truncation scripts, seed data) belong under `tests/it/sql/` and load via the existing `wow-database` connection helper.

---

## 7. Wire-protocol packets

N/A — this doc does not own packets. It references the **catalogue of packets that need golden vectors**, which is the union of the §7 sections of every per-module doc. The proto-specific opcode list lives in `proto.md` and `handlers.md`; this doc's job is the testing methodology around them.

---

## 8. Current state in RustyCore

**Files / artifacts in `/home/server/rustycore`:**

- `Cargo.toml` (root) — workspace `[dev-dependencies]` block exists but has no `proptest`, no `criterion`, no `insta`, no `cargo-fuzz` member.
- Per-crate `tests/` folders: present in `wow-database/tests/` (DB-shape integration), `wow-data/tests/` (DBC parsing). Most other crates keep tests inside `src/**/#[cfg(test)] mod tests` only.
- Workspace-root `tests/` directory: **does not exist.** No L3/L4 home today.
- Workspace-root `benches/` directory: **does not exist.** Per `migration-perf-strategy.md`, criterion benches live per-crate when added.
- CI: **no `.github/workflows/`** in the public repo (per `AGENTS.md`, the `.claude/` and `.github/` flavours are gitignored locally). `cargo test --workspace` is the only gate.

**What's implemented:**

- 395 passing unit tests, organised inside each crate.
- DB-shape tests in `wow-database/tests/` against a live MariaDB connection (skipped when `DATABASE_URL` not set).
- Round-trip serialization tests in `wow-packet`.
- Algorithmic primitives tested round-trip in `wow-crypto`, `wow-math`, `wow-core`.

**What's missing vs the strategy in §3:**

- **L1 property/fuzz layer**: zero tests. No `proptest`. No fuzz targets.
- **L2 golden-vector layer**: zero hardcoded C++-derived vectors anywhere. Every audit's "no cross-impl test vectors" finding maps here.
- **L3 integration layer**: zero. No `tests/it/`. No in-process `WorldSession` smoke test.
- **L4 end-to-end layer**: zero. No `docker-compose.yml`, no scripted client.
- **Regression-bug catalogue**: zero named regression tests. Bugs that have been fixed are not re-tested.
- **Slow-test feature flag**: not present. All tests run on every `cargo test`.

**Suspicious / likely divergent (hypotheses pre-audit):**

- The 395-test baseline may shrink if `wow-database` integration tests start failing once schema drifts; a few are likely already "ignored" silently because they require a DB.
- Several crates' `mod tests` are testing _Rust serde derives or trait impls_, not protocol behaviour. Counting passes is misleading without a coverage breakdown.
- `wow-recastdetour` is FFI scaffolded (per AGENTS.md) — almost certainly has no real tests, only build-shape checks.

**Tests existing:**

- 395 across the workspace (AGENTS.md).
- Distribution by crate is not currently summarised anywhere. **#TEST.1** below is "produce that summary."

---

## 9. Migration sub-tasks

Numbered for cross-reference from `MIGRATION_ROADMAP.md` and from per-module audit "see migration-test-strategy" pointers.

Complexity: **L** (<1h), **M** (1-4h), **H** (4-12h), **XL** (>12h, split).

- [ ] **#TEST.1** Produce per-crate test-count and coverage summary (`cargo llvm-cov --workspace --html`); commit baseline as `docs/migration/_baseline-coverage.txt`. (M)
- [ ] **#TEST.2** Add workspace-level `slow-tests` feature flag; gate L2/L3/L4 tests behind it via `#[cfg_attr(not(feature = "slow-tests"), ignore)]`. (L)
- [ ] **#TEST.3** Create `tests/golden/` convention doc + first vector: SRP6 `(user, password) → (salt, verifier)` pair captured from C++ TC. Resolves `#CRYPTO.1` from `crypto.md`. (M)
- [ ] **#TEST.4** Capture HMAC chain (digest, session_key, encrypt_key) from a real C++ login (use the `WorldSocket.cpp:755` debug log) and bake into `wow-crypto/tests/golden_hmac_chain.rs`. Resolves `#CRYPTO.5` + part of `#CRYPTO.7`. (M)
- [ ] **#TEST.5** Capture an `SMSG_AUTH_RESPONSE` byte sequence from a working C++ server and assert Rust serializer produces identical bytes. (M)
- [ ] **#TEST.6** Capture an `SMSG_UPDATE_OBJECT` for a fresh creature spawn (CreateObject2) and a position-only update; assert Rust matches both. (H)
- [ ] **#TEST.7** Capture a `MovementInfo` (CMSG_MOVE_*) from PCAP and round-trip via `wow-packet::movement`. (M)
- [ ] **#TEST.8** Add `proptest` dev-dep to workspace; first prop test on `BigNumber` ↔ `num_bigint::BigUint` round-trip across endianness. (M)
- [ ] **#TEST.9** Add `cargo-fuzz` workspace member with target `fuzz_world_dispatch`: feed random bytes to the `wow-handler` dispatch table; assert no panic, no UB. (H)
- [ ] **#TEST.10** Add `tests/it/login_to_world.rs` — boots a `WorldSession` against an in-process MariaDB (or sqlite-as-mariadb-shim if feasible) and walks `CMSG_AUTH_SESSION → SMSG_AUTH_RESPONSE → CMSG_PLAYER_LOGIN → SMSG_LOGIN_VERIFY_WORLD`. (XL — split into sub-issues per opcode.)
- [ ] **#TEST.11** Add `docker-compose.yml` under `tests/e2e/`: MariaDB 10.6 + `bnet-server` + `world-server`, then wire the existing Rust bot harness at `/home/cdmonio/projects/wow-test-bot/rust-bot/` as the scripted client once its C++-validated auth/login prerequisites pass against Rust. CI step `docker compose up --wait && cargo test --features slow-tests -p e2e`. (H)
- [ ] **#TEST.12** Add an `insta`-based snapshot test for chat-message formatting and for log output formatting (covered in `logging.md` too). (M)
- [ ] **#TEST.13** Backfill regression tests for every commit matching `^fix:` in `git log`. Initial sweep produces a count; one named `regression_<short-sha>` test per. (H)
- [ ] **#TEST.14** Document a "vector capture playbook" — how to produce a new golden vector from C++: which logger to enable, which file to read, how to convert. Lives at `docs/migration/vector-capture-playbook.md`. (M)
- [ ] **#TEST.15** Add CI coverage for the exact `rust-toolchain.toml` / workspace `rust-version` contract running `cargo test --workspace` plus an opt-in `--features slow-tests` job. (M)
- [ ] **#TEST.16** Add a "test-debt watch" job: on each PR, fail if `cargo test --workspace --no-run` count drops below the committed baseline (catches accidentally-deleted tests). (L)

---

## 10. Regression tests to write

These are the **invariants** the test suite must protect; per-module audits list more, this section captures the **cross-cutting** ones that no single module owns.

- [ ] `cargo test --workspace` count never drops without an explicit baseline-update commit.
- [ ] Every `crates/*/tests/golden/` directory remains binary-stable: a `git diff` after running tests must be empty (no test re-writes its own input).
- [ ] Every `#[ignore]` test has a justifying comment pointing at the issue or doc that explains why.
- [ ] Property tests must seed deterministically — no `thread_rng()` in `proptest!`.
- [ ] Integration tests in `tests/it/` must clean up DB state between runs (transaction-rollback wrapper or explicit `TRUNCATE`).
- [ ] `slow-tests` feature compiles cleanly even when its dependencies (e.g. `testcontainers`) are pulled.
- [ ] No test depends on a network resource outside the workspace (no fetching from `github.com` in tests; all fixtures committed).
- [ ] Encryption / SRP6 / packet-byte tests **must** be golden against captured C++ output, not Rust-generated round-trip.
- [ ] If a regression test has a tracking issue ID, it appears in the test name or comment.

---

## 11. Notes / gotchas

1. **The 395-test number is not a quality metric.** It's mostly L0 unit. The audits' "no golden vectors" flag means the wire-correctness gate is currently visual: someone has to log in with a real client and confirm. That is not sustainable. `#TEST.3`–`#TEST.7` are the priority chain that fixes this.
2. **Capture once, treat as immutable input.** It is tempting, when a test fails after a refactor, to re-capture and re-bake the golden. **Don't.** A failing golden test is the signal that the refactor broke the wire format. If the format genuinely changed (rare), update with a `BREAKING:` commit and a new dated `.meta` file.
3. **MariaDB-only is a hard constraint.** Per AGENTS.md, `wow-database` targets MariaDB 10.6+. SQLite-based L3 tests are tempting but risk drift from real prepared-statement semantics. Prefer a tempdir MariaDB (or `testcontainers`) over SQLite. If SQLite is used as a fast smoke layer, keep one MariaDB-backed integration test as truth.
4. **The C++ legacy at `/home/server/woltk-trinity-legacy/` is the canonical reference per AGENTS.md.** Treat it as the test-oracle. Any time a test asks "what should this byte be?", the answer comes from running the corresponding C++ function, not from inspection of the Rust output.
5. **`_attic/` content is not a vector source.** Per AGENTS.md, `_attic/` is failed integration scaffolding. Do not bake `_attic/` outputs into golden tests; capture from C++ or the running official client only.
6. **Tests that allocate a `MapManager` need it `Arc<RwLock<…>>` shaped per AGENTS.md** "WorldSession and creature storage" section. The legacy `WorldSession.creatures` field is `#[deprecated]`. New tests should use `MapManager` directly to avoid being rewritten when the migration finishes.
7. **`PROTOC=/home/ubuntu/.local/protoc/bin/protoc` must be set even for `cargo test`** because `wow-proto` invokes `prost-build` in `build.rs`. CI configs that forget this fail at compile, not at test.
8. **Per-crate testing is fast** — use `cargo test -p <crate> --lib` for tight loops; `--workspace` is for green-bar verification only.

---

## 12. C++ → Rust mapping (high-level)

| C++ test machinery | Rust equivalent | Notes |
|---|---|---|
| `boost::test` (TC unit) | `#[test]` + `#[cfg(test)] mod tests` | Native; in-source. |
| TC `bot debug` log capture | `tracing` capture in tests via `tracing_subscriber::fmt::TestWriter` | See `logging.md` §8 for the test-friendly subscriber pattern. |
| TC stress test fixtures (rare) | `proptest` + `criterion` | Workspace deps `#TEST.8`, `migration-perf-strategy.md`. |
| C++ packet captures (manual hex) | `tests/golden/data/*.bin` + `include_bytes!` | Binary, not hex. |
| C++ `playerbots` simulator | `tests/e2e/` scripted client (TBD) | `#TEST.11`. May reuse existing playerbot infra against the Rust server. |
| `make check` | `cargo test --workspace` (fast) + `cargo test --workspace --features slow-tests` (full) | `#TEST.2`. |
| Coverage: `lcov` + `gcov` | `cargo llvm-cov --workspace` | `#TEST.1`. |
| TC `ASSERT(...)` | `assert!`, `debug_assert!`, `Result<>` errors | TC asserts in tests should be reproduced as `assert_eq!` against captured behaviour. |

---

## 13. Audit (2026-05-01)

> Audited 2026-05-01. Scope: this strategy doc itself, against the test claims in (a) `AGENTS.md`, (b) the per-module audits at `/home/server/rustycore/docs/migration/*.md` §13 sections, and (c) the actual repo at `/home/server/archived/rustycore_ARCHIVED_20260312/`. Not audited against C++ — this is a meta-doc.

### 13.1 Claim verification

| Claim in this doc | Source / verified against | Status |
|---|---|---|
| `cargo test --workspace` → 395 pass | `AGENTS.md` "Build / test" line `cargo test --workspace 395 passed` | ✅ asserted, not independently re-run as part of this audit |
| Per-module audits flag "no golden vectors" | `crypto.md` §13.2 #3, §13.3 #2 explicitly | ✅ confirmed for `crypto.md`; assumed for the other ~50 audits not re-read here |
| No `tests/` workspace root | `ls /home/server/archived/rustycore_ARCHIVED_20260312/` shows only crate-scoped tests | ✅ confirmed |
| No `benches/` workspace root | Same | ✅ confirmed |
| No `criterion`, `proptest`, `insta`, `cargo-fuzz` in deps | `grep criterion crates/*/Cargo.toml` returned 0 matches | ✅ confirmed |
| `wow-logging` is the only logging crate | `find crates -name "wow-logging" → 1 match` | ✅ confirmed |
| `_attic/` exists with renamed `.rs.txt` files | `AGENTS.md` "_attic" section + `find crates/wow-world/_attic` | ✅ confirmed by reference |
| MariaDB 10.6 is the runtime DB | `AGENTS.md` "Runtime" | ✅ confirmed |

### 13.2 Critical findings

1. **No L2/L3/L4 layer exists today.** The strategy in §3 is aspirational; everything below "L0 unit + a thin slice of L1 in `wow-database` integration" is greenfield. `#TEST.3`–`#TEST.11` together represent ~3-5 person-weeks. Sequencing matters: golden vectors (`#TEST.3`–`#TEST.7`) unblock confidence in primitives; integration (`#TEST.10`) requires those primitives stable; e2e (`#TEST.11`) is the last mile.
2. **Vector source is fragile.** The C++ commit pinned in `crypto.md` §13 is `5100ce3d8fc6`. If that tree changes, captured vectors drift. Recommendation: tag the commit (`git tag -a vectors-2026-05-01 5100ce3d8fc6`) and reference the tag in every `.meta` file.
3. **Coverage baseline is unmeasured.** §8 says 395 pass, but no breakdown by crate or by line-coverage exists. `#TEST.1` is the prerequisite for any "we are X% there" claim; without it, every other "test progress" number is anecdotal.
4. **Slow-test gating is unspecified.** Without `#TEST.2`, the moment integration tests land they will balloon `cargo test` from <30s to minutes, training developers to skip the gate. Add the feature flag **before** landing the first slow test.
5. **No CI today.** Per AGENTS.md, `.github/` is gitignored locally. Whatever this strategy proposes must live somewhere CI can find. `#TEST.15` is the missing piece; until it lands, `cargo test --workspace` is enforced only by developer discipline.

### 13.3 Recommended action — priority queue

1. **`#TEST.1` + `#TEST.2` (foundation)** — measure where we are; gate where we need to slow down. Both are ≤4 h. Land first.
2. **`#TEST.3` + `#TEST.4` (cryptography golden)** — directly resolves `crypto.md` §13.3 #2 ("vector tests, HIGH"), which is the single highest-impact test debt in the codebase. Every other module that uses crypto inherits the gain.
3. **`#TEST.6` + `#TEST.7` (wire-format golden for top-traffic packets)** — UpdateObject and MovementInfo are the two opcodes that, if subtly wrong, break gameplay silently. Cover them next.
4. **`#TEST.10` (in-process integration)** — once primitives are gold, prove the assembly. Pre-seeds confidence for the live-fire docker harness.
5. **`#TEST.11` + `#TEST.15` (CI loop closes)** — only valuable once 1-4 are present. Closes the long-term regression-prevention story.
6. **`#TEST.13` regression-debt sweep** — parallelisable with the above. One PR per fix.

### 13.4 Justifying the index status

Header status is `⚠️ partial` because the **structure** is documented but the **artifacts** (vectors, tests/, benches/, fuzz/, CI) do not exist. The 395 baseline is real but covers only L0. Status moves to `✅ done` when `#TEST.1`–`#TEST.5`, `#TEST.10`, `#TEST.11`, and `#TEST.15` are all checked. Status moves to `🔧 broken` if the 395 number ever drops without a baseline-update commit (i.e. silent test loss).

---

*Template version: 1.0 (2026-05-01).*
