# Controlled modularity laboratory

Issue #578, approved experiment 2026-09-05. This laboratory is deliberately outside the
production Cargo workspace. It neither selects a live ECS backend nor closes #578/#583.
The two standalone crates avoid conflating entity storage with extension execution:

- `storage`: one owner/operation driver over aggregate/sidecar storage or selective hecs.
- `execution`: identical shared Rust decision logic compiled natively and to Core Wasm,
  over the same aggregate host. Wasmtime was the pinned execution candidate for V1.

**Post-campaign direction:** the approved [modularity/ECS plan](../../../docs/architecture/modularity-and-ecs-plan.md)
now selects private selective hecs and native Rust plus operator-optional Wasmtime/Core Wasm.
That decision does not install them or extend what V1 proved. Finite third-module conformance
inside #578 precedes production storage migration; #583 subsequently delivers the external
stateful-module contract and Rust/C Wasm evidence. #578 does not depend on finishing #583.

## Pre-registered controls and decision rules

`protocol.json` was written before performance measurements. The report embeds its exact
SHA-256, parameters, source digest, binary/guest hashes, toolchain and host architecture.
Changes to criteria after seeing results require an explicitly new protocol, not silent tuning.

1. Correctness is mandatory. Every declared functional check must pass before measuring.
   Compare ordered actions, callbacks and resulting state, not only a final checksum.
2. Storage: 1,000 and 10,000 entities, sparse/dense optional families, 25 warmup and 200
   measured ticks, 10 independent paired runs per configuration, seeds 42–51. The same seed
   defines both workloads; alternate execution order and start a new process for each sample.
   Entity insertion order is shuffled; structural churn and transfer are measured separately.
3. Execution: 10,000 and 100,000 invocations, 10 independent paired runs per configuration.
   Native and Wasm run the same logic/inputs, with real host calls in the measured operation.
   Both receive 256 warmup calls; measured event counts and final host/module observables
   must agree, in addition to the checksum.
   Separate compilation/instantiation from warm invocation; do not compare two different SDKs.
4. Do not run builds or other lab measurements concurrently with a campaign. This is a shared
   development host, not an isolated performance runner: record load and report variation.
5. Storage lab budget: median of paired `hecs/aggregate` update-p99 ratios <= 1.25 for each
   configuration, and each candidate RSS <= `1.5 * paired baseline RSS + 16,384 KiB`.
   These are explicit provisional laboratory tolerances, not production memory/tick SLAs.
   Report sorting, churn, transfer, creation and peak memory too; passing an update-only gate
   never hides unacceptable costs in these other operations.
6. Wasm lab budget: median warm invocation p99 <= 10,000 ns and process RSS <= 262,144 KiB.
   The sizing hypothesis is 100 bounded invocations per hypothetical 10ms frame, with 1ms
   reserved for extensions. Actual call density/bursts and production headroom are unmeasured;
   passing this threshold establishes feasibility for this workload, not server capacity.
7. Report all samples plus min/median/max between runs. A p99 from 200 ticks covers roughly
   two observations: it is a noisy diagnostic, not a reliable rare-tail estimate. No pooling
   that obscures between-process variation. No fastest-run selection or architecture mixing.

The verdict has separate dimensions: functional contract, lab resource budget, composition/
integration complexity, and missing production evidence. If a candidate fails, identify the
specific failure. Repeat only after a named correction or under a separately recorded workload;
do not reframe a failure as a passing result or keep two permanent live authorities.

## Contract cases and oracle boundaries

- Move a non-Clone incarnation active/detached/active without copying its authority; failed
  admission retains it; replacement invalidates stale handles. Core and optional state must
  travel according to their declared scope.
- Compose two independently registered rule/state families, reject explicit conflicts and
  remove only the corresponding contribution. Admission must not partially apply rejected
  commands; execution failure preserves effects already applied under its contract.
- Model encounter phase/shield before a nullable summon, synchronous callback before return,
  then query state. Release all mutable state borrows before actions that can reenter.
- Exercise actual invocation of lab owner phases across two maps, including a delayed-update
  barrier, rather than returning a fixed expected trace string.
- Execute the same Rust module natively and in Wasm; test reentry, read-after-action, fuel,
  memory and host-call limits, forged handles/actions, trap after an applied effect, reset and
  receipt replay. Budgets must be cumulative through nested callbacks, not refilled on reentry.
  Growth rejection must be caused by the configured cap, not Wasm32's absolute address limit;
  a nested-fuel check must detect an actual refill, not only assert some fuel was consumed.
- Distinguish new instance/configuration reload from a genuine binary/schema upgrade. Any
  receipt/recovery model is an in-memory contract test, not MariaDB crash durability.

C++ anchors under `/home/server/woltk-trinity-legacy/src/server/`:

- `scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:81-181,232-244`: initialization, timers,
  partial effects and action-then-query ordering. The lab does **not** port every boss spell,
  AI selection rule or Nexus encounter behavior.
- `game/Entities/Creature/TemporarySummon.cpp:249-264`: callbacks before summon return.
- `game/AI/CreatureAI.cpp:219-242`: synchronous evade/reset.
- `game/Maps/MapManager.cpp:287-318`: all map updates precede delayed updates.
- `game/Instances/InstanceScriptData.cpp:137-142`: transient encounter states normalize on load.
- `game/Entities/Player/Player.cpp:2189-2226`: XP hook ordering; optional lab policy arithmetic
  is explicitly custom, not a claim to port the full GiveXP implementation.

## Reproduction

Prerequisite: Rust toolchain and `wasm32-unknown-unknown` target. No DB/server credentials,
runtime restarts or downloads of executable third-party modules are required. Cargo fetches
the pinned build dependencies; the guest is built from the checked-in Rust source.

```bash
rustup target add wasm32-unknown-unknown
CARGO_BUILD_JOBS=2 cargo build --release --locked --manifest-path tools/architecture/modularity-lab/storage/Cargo.toml
CARGO_BUILD_JOBS=2 cargo build --release --locked --manifest-path tools/architecture/modularity-lab/execution/Cargo.toml
cargo build --release --locked --target wasm32-unknown-unknown --manifest-path tools/architecture/modularity-lab/execution/guest/Cargo.toml
CARGO_TARGET_DIR=tools/architecture/modularity-lab/execution/guest/target-v2 cargo build --release --locked --target wasm32-unknown-unknown --features v2 --manifest-path tools/architecture/modularity-lab/execution/guest/Cargo.toml
python3 tools/architecture/modularity-lab/run.py --output /tmp/rustycore-modularity-results.json
```

The runner performs functional checks first, preserves raw samples and evaluates budgets.
It fails closed on a failed/missing check, malformed result, semantic disagreement, timeout,
missing memory observation or an exceeded declared budget. Budget failure still writes a full
report; it is not evidence that a library is universally unsuitable. `--smoke` is for runner
diagnostics only and is explicitly ineligible for a decision. `--suite` can isolate one axis.

Build artifacts and ordinary results are ignored. Preserve the bounded evidence report under
`docs/architecture/evidence/` with its source/binary hashes when documenting the verdict.

## Limits that must remain visible

This is an executable **laboratory model**, not the real Map/Session owner, full SDK, fresh
client capture, end-to-end encounter, distributed race test or real durable storage test.
After the independent-state conformance gate, affected-family production ownership/integration
still needs #578 acceptance. External consumers, durable operator lifecycle and the bounded
native/Wasm SDK separately need #583 acceptance; a V1 pass closes neither macro.
Existing `production_login_player_owner` tests cover early hydration only, not that full proof.
The storage fixture knows two optional types in `Bundle`/`Row`/`Store` and enumerates their
four combinations on ECS extraction. It proves their coexistence and lifetime, not addition of
arbitrary third-party state types without core changes or a winner in API maintenance.

The Wasm experiment uses a minimal Core Wasm ABI. It does not establish that a Component Model/
WIT interface preserves synchronous reentry or provides compatible state upgrades. Fuel limits
guest computation, not a blocking/expensive host function; guest memory limits do not cover
all host allocation. Host capabilities, bounded actions/results and recovery remain our code's
responsibility. Wasm traps do not undo already-applied host effects. No hot reload, whole-process
sandbox guarantee, stable public ABI or production decision is created by this experiment.

Campaigns and the reviewed verdict are retained in
[the result report](../../../docs/architecture/modularity-lab-results.md). A raw automatic
success flag may be superseded by the accompanying adversarial review; retain both records.
This README has post-campaign clarifications. `run.py` includes Markdown in its source digest,
so reproducing the historical digest requires the recorded campaign tree, not this later
documentation revision. Existing evidence and recorded source/binary hashes are not rewritten.

Primary references: [hecs](https://github.com/Ralith/hecs),
[Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html),
[host-call limits](https://docs.rs/wasmtime/47.0.3/wasmtime/struct.Config.html#interaction-with-blocking-host-calls),
[Component Model invariants](https://github.com/WebAssembly/component-model/blob/main/design/mvp/Explainer.md).
