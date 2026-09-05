# Independent-module conformance — private experiment

This is the finite pre-production gate owned by #578, specified in
[the approved plan §5](../../../docs/architecture/modularity-and-ecs-plan.md).
It is a sibling of `modularity-lab`, not a revision of V1's source or evidence campaign.
Production Cargo dependencies, runtime, database and public module SDK remain unchanged.

Build/tests during implementation do not constitute the frozen-host/third-module
experiment. Only a complete source-bound campaign can produce a conformance verdict.
The maintained status and retained evidence belong to the
[issue checkpoint](../../../docs/architecture/session-578-checkpoint.md), not this frozen
method document. V1's results do not satisfy this experiment's gate.

## Contract and authorities

- The experimental core owns identity/incarnation, active/detached residence and private
  hecs storage. Native modules provide their own state type and codec; generic registration
  creates typed components distinguished by module type, not a central module enum.
- Modules only depend on `contract`. They obtain owned bounded snapshots and perform
  revision-checked writes. Mutable state/guest globals, backend IDs, storage borrows, SQL,
  packet writers and ambient I/O are not extension capabilities.
- The Wasm adapter uses host-owned namespaced bytes, not guest-owned canonical state.
  A single Wasmtime Store contains the experimental core and all loaded guest instances.
  Native and Wasm calls share the same invocation stack and host budgets; nested callbacks
  never replenish fuel or retain a state/guest-memory borrow across reentry.
- Both adapters validate the module's codec before accepting initial, written or replayed
  state, requiring bounded canonical round-trip bytes (`encode(decode(bytes)) == bytes`).
  Wasm `validate_state` reads an owned temporary projection through `validation_read`;
  ordinary imports are forbidden during validation, even if a guest ignores their error.
  Codec reads have a separate 256-read bound and use the same cumulative guest fuel, not the
  semantic host-call counter. Failed admission must not consume a mutation revision.
- Native code is trusted source. The host caps its admitted actions, depth and state, but
  does not promise to interrupt arbitrary native loops or contain native panics.
- Each module has a versioned encoding. Mock snapshot/replay and executor interchange
  must validate versions before activation; they prove neither SQL durability nor recovery
  from a real lost COMMIT. Backend locators are never replay identities.
  A change between native and Wasm execution is currently rejected with state retained;
  successful conversion is not implemented by this lab.
- Removal clears only that module's reversible contributions. Reset, detach and attach
  callbacks may implement module-specific rules without new central lifecycle cases.
  Schema version, incarnation and mutation revision remain distinct; resetting revision
  would introduce an ABA bug and is not allowed.

## C++ anchors and custom behavior

Source root: `/home/server/woltk-trinity-legacy/src/server/`.

- `scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:81-181`: publish phase and shield before
  nullable summon; failure does not undo prior effects. The sample is not a complete boss port.
- `game/Entities/Creature/TemporarySummon.cpp:249-264`: summon lifecycle callbacks execute
  synchronously before return. Dispatching them next tick is not equivalent.
- `game/AI/CreatureAI.cpp:219-242`: synchronous evade/reset ordering.
- `game/Maps/MapManager.cpp:287-318`: all update work precedes delayed updates. The experiment
  may test its own driver barrier; only production-linked tests can establish real scheduling.
- `game/Entities/Player/Player.cpp:2189-2226`: policy hook before XP award. The optional
  arithmetic contribution in this experiment is explicitly custom, not full GiveXP parity.

The base record remains present with zero optional modules. It represents only this narrow
contract fixture, not proof that required production scripts are complete or enabled.

## Freeze and third-party challenge

First implement and test encounter and policy in separate crates through the same host.
Then hash the contract, host, adapters, original modules and C bindings. Only after the
freeze may an independent third crate introduce a new state shape and lifecycle rule.
That step may change dependency declarations, declarative registration/composition and its
own code/tests; it may not edit frozen host/storage/ABI code or add central module cases.

Record every frozen path/hash and check exact additions/removals as well as file contents.
A required frozen-core correction invalidates that challenge: record the reason, correct and
validate the host, then run a genuinely new independent extension challenge. Do not silently
rehash a post-hoc successful implementation or call editing the host zero integration cost.

The third module must have a genuinely different state shape and meaningful lifecycle rule,
not a renamed copy or a constant-returning registration. Its native crate depends only on
`conformance-contract`; the same module must run as Rust Wasm and independently written C
Wasm. An integration target `driver/tests/<module>.rs` must exercise its complete lifecycle
through the real composition in four tests:

```text
independent_module_native_lifecycle
independent_module_rust_wasm_lifecycle
independent_module_c_wasm_lifecycle
independent_module_mixed_lifecycle
```

The runner checks actual registered/executed identities as well as the exact test set and
source freeze. An independent semantic review still must examine state/rules, dependency
directions and every allowed declarative diff; hashes cannot establish independence by
themselves. The review record binds the freeze SHA-256, complete current source hash set,
new module name/ID and reviewed declarative paths. This is engineering review, not another
routine user-approval gate. See `run.py::semantic_review` for the machine-readable record.

## Source layout and reproduction

| Path | Responsibility |
| --- | --- |
| `contract/` | Module/host vocabulary, scoped operations, Rust Core Wasm bindings |
| `host/src/{registry,storage,dispatch,lifecycle,checkpoint}.rs` | Registration, private owner/storage and admitted transitions |
| `host/src/wasm/` | Wasmtime execution, imports, pure codec validation and resource controls |
| `modules/encounter/`, `modules/policy/` | Independent non-Clone state types and rules |
| `c-guests/` | Freestanding C producers for the same ABI and contract |
| `driver/src/harness.rs` | Frozen executor adapter; never new-module integration logic |
| `driver/src/composition.rs` | Reviewed declarative module registration |
| `driver/src/{checks,bench}/` | Detailed functional oracles and separate timing workloads |
| `protocol.json`, `build.py`, `run.py`, `report.py`, `freeze.py` | Predeclared experiment, provenance, orchestration, verdict and challenge freeze |

Use the repository's pinned Rust toolchain, the `wasm32-unknown-unknown` target, and Clang/
wasm-ld 18 for the C producer. The C build uses neither libc nor WASI. `build.py` accepts
`--clang`, `--wasm-ld` and `--library-path`; its defaults locate the locally extracted
aarch64 toolchain under the ignored repository `target/modularity-conformance/c-toolchain`.
These defaults do not install system packages. Other machines must supply their own paths.
The build record captures Rust/C compiler versions, Clang/lld binary hashes, all source hashes and the actual driver/
guest artifact paths and hashes. Rust dependencies are locked; offline builds require the
dependencies to be available locally.

Run from the repository root. Choose a new evidence directory per attempt; record outputs
outside this source tree so a campaign does not mutate its own freeze.

```bash
python3 -m unittest discover -s tools/architecture/modularity-conformance -p 'test_*.py'
cargo fmt --manifest-path tools/architecture/modularity-conformance/Cargo.toml --all -- --check
mkdir -p target/modularity-conformance/campaign-01
python3 tools/architecture/modularity-conformance/build.py \
  --output target/modularity-conformance/campaign-01/two-module-build.json
python3 tools/architecture/modularity-conformance/run.py \
  --build-record target/modularity-conformance/campaign-01/two-module-build.json \
  --output target/modularity-conformance/campaign-01/prefreeze.json.gz
python3 tools/architecture/modularity-conformance/freeze.py create \
  --validation-report target/modularity-conformance/campaign-01/prefreeze.json.gz \
  --output target/modularity-conformance/campaign-01/freeze.json
```

Only after that successful freeze may the independent author create the third module. Once
its permitted changes and semantic review are complete, rebuild to a **new** build record.
The placeholder below denotes the actual new module name, not an existing producer:

```bash
python3 tools/architecture/modularity-conformance/freeze.py check \
  --freeze target/modularity-conformance/campaign-01/freeze.json --module MODULE_NAME
python3 tools/architecture/modularity-conformance/build.py \
  --output target/modularity-conformance/campaign-01/three-module-build.json
python3 tools/architecture/modularity-conformance/run.py \
  --build-record target/modularity-conformance/campaign-01/three-module-build.json \
  --freeze target/modularity-conformance/campaign-01/freeze.json --module MODULE_NAME \
  --review-record target/modularity-conformance/campaign-01/semantic-review.json \
  --measure --output target/modularity-conformance/campaign-01/campaign.json.gz
```

Omit both `--measure` and `--review-record` for a correctness-only challenge run before
writing the actual semantic review; it is never decision-eligible. The
runner retains command exit status, stdout/stderr and errors, checks functional equivalence
before measuring, and rejects changed source/artifacts. Do not benchmark while other builds
or laboratory measurements are running. Preserve failed attempts rather than replacing them
with a later successful sample. A broken host freeze requires a new challenge, not merely a
new JSON hash of the old third module.

Native-only compilation remains supported without enabling `wasm`; it does not run the full
cross-executor campaign. The real guest resource/import tests require the built Rust and C
artifacts above. A single `conformance-driver checks MODE` output is not a campaign verdict.

## Predeclared measurement boundary

`protocol.json` is the numeric authority. Ten independent repetitions (seeds 42–51), four
executor modes, two populations/densities and two dispatch counts/workloads produce 320
fresh-process samples. Rotation by seed reduces fixed mode-order bias; it does not turn one
aarch64 host into x86_64 or production evidence.

- Storage visits every base entity, including those with zero optional state. Sparse means
  one quarter have optional state, with independently shuffled insertion and traversal.
  Each measured tick separately times update, 1% optional-state removal/reinstallation and
  1% active–detached–active transfer to the opposite map. Warmup precedes measurement.
- Dispatch measures a whole root call, including its module fanout, synchronous callbacks,
  scoped host calls and budget reset. The policy case checks returned values; reentry expects
  rejection of an obsolete outer state write after a successful nested mutation. This is not
  an empty function-call microbenchmark or a pure hecs query benchmark.
- Keep chronological raw durations, per-run distributions, work/result counters, complete
  final-state digests, cold compile/instantiate phase costs and live process RSS. Functional
  acceptance uses full ordered oracles, including callback returns, not just digests.
- Report all repetitions and min/median/max. A p99 from 200 ticks is a noisy batch statistic.
  Dividing a batch percentile by entity count gives an amortized batch cost, **not** a tail
  latency for one entity. The 10,000-entity bound allows a 500ms batch in this experiment;
  it does not demonstrate that production fits a 10ms map tick.
- Limits are provisional rejection gates, not a server SLA or proof that hecs outperforms
  an unmeasured alternative. Native source is trusted and not CPU/panic sandboxed. Wasm
  memory/fuel controls do not bound arbitrary blocking host work. Failed guest instantiations
  consume bounded Store capacity until that Store is dropped; live replacement is not proved.

Passing this finite checkpoint permits the approved next production integration step, not
closure of #578. Real lifetime/save, admission/phase, publication/backpressure/shutdown,
durability, deployment and complete C++ parity retain their own evidence and authority gates.
