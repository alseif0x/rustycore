# Controlled modularity experiment — 2026-09-05

## Reviewed verdict

**The corrected laboratory passes its functional and pre-registered resource gates.** It is
complete as an experiment, not a production backend migration or an external-module delivery.

| Axis | Decision supported by this evidence | What it does not authorize/prove |
| --- | --- | --- |
| Entity storage | Advance selective private `hecs` to the affected-family integration cut inside #578; retain cohesive aggregates for their invariants | No global ECS rewrite or wholesale Player decomposition; composition/maintenance superiority still needs real consumers |
| Module execution | Retain trusted native Rust as the initial default; Core Wasm is technically feasible for a concrete isolation need on this contract | No reason to require Wasm for all scripts; no production sandbox, public ABI, hot reload or WIT guarantee |
| Modularity | Share semantic capabilities and lifecycle rules independently of storage/execution; use #583's two real external consumers as acceptance | Neither a faster container nor a sandbox replaces the module contract, durable receipt or operator workflow |

The immediate next implementation remains the complete #578 owner/lifetime/operation work.
Use one real stateful family and its second composition case to finish the integration gate;
replace the superseded representation in that cut, not maintain two authorities. Do not repeat
synthetic runs instead of proving production save/transfer/admission and module independence.
If the affected real family lacks the composition benefit or fails correctness/resource needs,
retain its aggregate storage with that reason recorded. No new experiment or micro-PR tree is needed.

## Corrected measurements

Accepted evidence: [complete second campaign](evidence/modularity-lab-v1-aarch64-20260905-r2.json),
2026-09-05 12:22:32–12:23:23 UTC. **34 functional cases (16 storage + 18 execution), 120 fresh-process
samples**, ten paired runs per configuration, seeds 42–51, alternating backend order. All paired
work counts/checksums and execution final observables agree; exact storage state/trace equality
is checked in the functional fixtures. Sources and binaries stayed unchanged throughout.

Host: aarch64, four logical CPUs, Linux 6.17.0-1019-oracle, Rust 1.98.0; one-minute load average
1.29 at start and 1.75 at finish. Storage uses 25 warmup + 200 measured ticks and 1% structural
churn per tick; execution uses 256 warmup calls. Shared source digest:
`b79133ce1ea00a2ff0643d9601643c4849eb2517b1e7816ff7f3838ac006f5f7`.
Pre-registered protocol SHA-256, unchanged from before the first measurement:
`1c2767b2ae3bdfffa0e2681e3e3949545707201cf86bd9e2e86674167a66b71c`.

| Entities / optional state | Aggregate update p99, ms | hecs update p99, ms | Median paired hecs/aggregate ratio | Paired ratio min–max |
| --- | ---: | ---: | ---: | ---: |
| 1,000 / 25% | 0.291 | 0.217 | 0.744 | 0.677–0.869 |
| 1,000 / 100% | 0.314 | 0.214 | 0.677 | 0.460–0.712 |
| 10,000 / 25% | 4.167 | 2.827 | 0.660 | 0.598–1.389 |
| 10,000 / 100% | 5.492 | 2.912 | 0.548 | 0.468–0.583 |

The p99 columns are medians of per-process p99s; the ratio is the median of paired ratios,
not their ratio of medians. All four median ratios meet the predeclared `<=1.25` gate and all
paired RSS observations meet `hecs <= 1.5 * aggregate + 16,384 KiB`. This is not a worst-case
deadline guarantee: one 10,000/sparse pair is 1.389, and that hecs sample has a 6.283ms p99.

There is a real tradeoff: hecs structural churn costs **1.49–1.69x**, and transfer **1.31–1.64x**,
using ratios of median total costs over the four configurations. Median hecs RSS spans
3,074–11,156 KiB versus 3,148–11,890 KiB for the aggregate; dense 1,000-entity hecs is slightly
larger. Sorting the full observable rows costs about 213–218ms total over 200 ticks at 10,000
entities for either backend. That oracle canonicalization, checksum, churn and transfer are
reported separately, not hidden in an alleged pure iteration win. The timed update also includes
materializing observable rows; it is not a measurement of production `Map::Update`.

| Measured calls | Native p99, µs | Wasm p99, µs | Median paired Wasm/native total cost | Wasm maximum live RSS |
| --- | ---: | ---: | ---: | ---: |
| 10,000 | 0.12 | 0.52 | 1.668x | 8,740 KiB |
| 100,000 | 0.12 | 0.52 | 1.708x | 9,520 KiB |

All Wasm runs report 0.52µs p99, below the provisional 10µs gate; maximum live process RSS
is about 9.30MiB, below 256MiB. Median cold engine/module compilation is about 7.91ms and
instantiation 0.124ms, outside warm invocation timings. Native cold fields are zero placeholders,
not comparative startup measurements. Timer resolution is visible in 40ns steps; total costs
include workload/timer/checksum bookkeeping. The small seeded mix includes XP decisions,
successful/reentrant and failed summons, reset and an idempotent mock reward. It does not
estimate the frequency or complexity of real server callbacks. Version 2 is a correctness/
migration case, not the benchmark workload.

## Scope and acceptance

The user approved an isolated, finite comparison now, including the Wasm feasibility probe
before M6. This is not approval to replace the production entity store, install a Wasm runtime
in the server, deploy/restart, mutate databases or close #578/#583. The production code under
review remains `93e4002a`; the experiment started above documentation HEAD `32d9a683` on #578.

The [reproducible laboratory](../../tools/architecture/modularity-lab/README.md) separates:

- Aggregate/sidecar storage versus `hecs =0.11.1`, with one owner/operation driver.
- Identical Rust logic compiled natively and to actual Core Wasm, hosted by `wasmtime =47.0.3`,
  with the aggregate host held fixed. The second guest is a genuinely different binary/schema.

The architecture skill required comparing capabilities and failure semantics, not treating
an ECS iteration win as modularity. Accordingly the lab tests move-only lifetime, reciprocal
combat admission, optional state, synchronous callbacks, partial effects, resource limits and
version migration. C++ call-point anchors and the intentionally custom workload are documented
in the laboratory README; this is not a complete Anomalus/Nexus or XP implementation.

## Evidence history and adversarial review

The [first raw campaign](evidence/modularity-lab-v1-aarch64-20260905.json) is retained but
**superseded for decisions** by its [review record](evidence/modularity-lab-v1-aarch64-20260905.review.json).
Its automatic success flags preceded discovery of three defects:

1. Requesting 65,536 extra memory pages failed at Wasm32's absolute address limit, even without
   the intended 3 MiB host cap. The check needed a valid-under-Wasm32 request above our cap and
   a positive request below it.
2. Checking that some fuel was consumed did not detect a nested callback refilling its budget.
   The check needed exhaustion inside reentry, before the independent depth limit.
3. The native adapter recorded host errors and continued executing, whereas the Wasm import
   trapped immediately. A callback could therefore mutate module state only in the native run.
   Fallible host calls need immediate propagation, distinct from an ordinary nullable summon
   result; neither implementation may roll back effects already applied.

The repeat is justified by those named corrections, not by selecting faster samples. Population,
event mix, seeds, repetitions and the pre-registered resource thresholds are unchanged. The raw
reports retain each source/binary digest; only the corrected campaign may support the verdict.

## Production boundaries that remain open

- The storage fixture knows two optional state types and explicitly handles four combinations.
  It does not prove arbitrary externally defined state, maintenance superiority or a public SDK.
- Its map phases are actually invoked but remain single-threaded lab owners, not the production
  Map/Session scheduler, saturated delivery, shutdown or parallel worker barrier.
- The generation-checked lab handles do not prove that all production GUID-based command paths
  reject stale incarnations. No real Player save/teleport/login flow is replaced by the fixture.
- Reward/receipt recovery uses an in-memory host mock. Migration/replay checks do not prove
  MariaDB durability, unknown COMMIT, process crash, operator hot reload or live Git installation.
- Core Wasm reentry does not establish Component Model/WIT compatibility. Fuel cannot stop
  an expensive/blocking host call, and guest memory caps do not bound every host allocation.
  A narrow native API is not a sandbox. Both execution adapters still require host integrity.
- Measurements are aarch64-only on a shared host, with no CPU pinning or frequency control.
  A p99 from 200 ticks is about two observations; preserve between-run variation and do not
  extrapolate these tiny decision functions into a server capacity or x86_64 SLA.

No fresh capture, live runtime exercise, DB migration, production dependency or macro-final
validation is claimed by this experiment. #578 retains C0–C4; #583 retains independent external
module, durable lifecycle and real operator acceptance; #153 audits their completed result.

## Focused validation

On the aarch64 development host, with `PROTOC` explicitly set for protobuf-dependent crates:

| Check | Result and boundary |
| --- | --- |
| Storage lab, `cargo test --locked --offline` and `--release` | 4/0 in each profile; 16 contract checks also execute in the real release CLI |
| Execution lab, `cargo test --release --locked --offline` | 3/0, including the actual guest contract suite and tests that reject removal of the memory cap and fuel refills; no full debug Wasmtime rebuild |
| Standalone Rust formatting / strict clippy | PASS for storage, execution host and both Wasm guest variants |
| Runner Python unit tests | 7/0; missing required checks, invalid numbers, population/work-count disagreement and missing observables reject |
| `wow-world --test production_login_player_owner` | 3/0 dev and 3/0 release; early hydration and drop only, not full login/save/teleport |
| `wow-map --lib manager::player_owner::tests` | 8/0; existing production-owner unit tests, not new end-to-end coverage |
| `wow-map --lib map::runtime::tests` | 5/0; existing focused runtime tests |
| `check_architecture.py check` / `self-test` | PASS; 38 workspace packages and 101 workspace edges remain unchanged |
| `session-ownership-check -- check --syntax-only` | PASS; 282 production + 432 fixture Session fields; no exhaustive persistence regeneration |

Builds/checks were stopped before the timing campaign. Existing production tests are a regression
baseline, not proof that the lab adapters have been integrated. No `validation-v2 final` or
clean-HEAD macro acceptance is claimed while the larger #578 work and earlier plan edits remain open.
