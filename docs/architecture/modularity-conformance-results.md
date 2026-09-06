# Independent modularity conformance V2 — results

**2026-09-05 · aarch64 · source `c67acbfd` · issue #578 / PR #579**

## Verdict and decision

**PASS** for this finite pre-migration proof: two modules were validated and the host frozen;
a third independent module then passed native Rust, Rust Core Wasm, C Core Wasm and mixed
composition without module-specific core changes. All 320 preregistered samples passed
the functional comparison and provisional cost/resource gates. The retained supervisor
reports `passed: true`, `decision_eligible: true`, and no errors.

Continue with the selected private selective `hecs` design and native-first, operator-optional
Wasm direction. No hecs-specific obstacle was found by this test; no new backend contest
is justified by these results. This is **not** a production speedup, public SDK acceptance,
proof of superiority over an unmeasured dense store/registry, or completion of #578/#583.

## Evidence and reproducibility

- [Raw campaign](evidence/modularity-conformance-v2-campaign-20260905.json.gz), 18,400,678 bytes;
  SHA-256 `5a956add1328bb67aeb54585ceb8638b81b27dccc376d7dd376d42b4963e0d98`.
  Includes every command, build/artifact/source identity, full functional oracles, 320 samples
  and chronological observations, not only selected quantiles.
- [Readable machine summary](evidence/modularity-conformance-v2-campaign-20260905.summary.json)
  retains all 32 mode/configuration summaries (10 runs each), min/median/max, resource
  maxima and the source hash set. It is a derived index; the raw report remains authoritative.
- [Original two-module freeze](evidence/modularity-conformance-v2-prefreeze-20260905.freeze.json)
  and [passing pre-freeze evidence](evidence/modularity-conformance-v2-prefreeze-20260905.json.gz).
  Freeze SHA-256 `5aeadb7a4a889bdfc879f9c69898c85325ea555479204ec303b9a8880fbc9424`.
- [Third-module correctness](evidence/modularity-conformance-v2-expedition-20260905.json.gz)
  and [actual semantic review](evidence/modularity-conformance-v2-expedition-20260905.review.json).
  The original 57-source freeze has 53 byte-identical files, four reviewed declarative
  dependency/registration deltas and seven allowed extension files; 64 sources are bound
  to the final campaign. Host, ABI, original module rules, driver oracles, benchmark and
  supervisor implementations were not changed for the third module.
- Campaign: 19:27:24–19:36:01 UTC, Linux aarch64, Rust 1.98, hecs 0.11.1,
  Wasmtime 47.0.3; freestanding C guests built with Clang/LLD 18.1.3.
  No concurrent laboratory build or benchmark. Other host services were not stopped;
  before/after load is retained. No x86_64 sample is pooled into these results.
- Local closeout: `git diff --check`, the unchanged-source freeze check and the VitePress
  build in `docs/wiki` pass. `validation-v2 quick --base 325200df` also passes (root formatting,
  JSON and workspace/all-targets compilation with existing warnings), manifest
  `target/validation-v2/manifests/20260905T194113.454935Z-1127654-quick.json`.
  That generic route does not run this standalone lab's tests; the explicit campaign above does.
  This is not the clean-HEAD final publication gate for the entire PR.

Run from the repository root after restoring the recorded toolchain:

```bash
# Use fresh output paths outside the laboratory source tree; do not recreate the freeze.
python3 tools/architecture/modularity-conformance/build.py --output /tmp/v2-new-build.json
python3 tools/architecture/modularity-conformance/run.py \
  --build-record /tmp/v2-new-build.json --output /tmp/v2-new-campaign.json.gz \
  --freeze docs/architecture/evidence/modularity-conformance-v2-prefreeze-20260905.freeze.json \
  --module expedition \
  --review-record docs/architecture/evidence/modularity-conformance-v2-expedition-20260905.review.json \
  --measure
```

The build record must match actual loaded artifacts and current source hashes. A source change
invalidates the retained semantic review; reproducing measurements is not a new independent
extension challenge. A necessary frozen-core correction requires retaining the failed challenge
and testing a genuinely new extension after correction, not rehashing Expedition as new evidence.

## What the extension established

Expedition (ID 73) owns a non-Clone, variable-length stampbook through a contract-only crate.
It rejects noncanonical state, duplicate checkpoints do not advance revisions, reset preserves
lifetime history, detach/attach suspends/restores its derived contribution, and retirement,
replacement and removal retain identity and isolation rules. C independently implements its
15–23-byte codec and lifecycle through the same frozen ABI.

The campaign repeats 51 host tests, 89 functional case executions and four full third-module
lifecycle tests. Their comparisons include complete canonical state, revisions, contributions,
root results and ordered traces with nested callback returns. Sparse/zero optional composition,
stale outer writes, nullable action failure, limits and rejected executor switches are covered.
The extension also passed two codec unit tests and strict native/Wasm module Clippy separately.

A calls=4 failure deliberately occurs after publishing a stamp but before its contribution.
The write remains; duplicate retry does not count history twice; detach/attach reconciles the
derived effect. This is an explicit partial-effect contract, **not rollback or durable recovery**.
The author did not implement the core/initial modules but previously worked on the supervisor;
this is not a blind third-party SDK study.

## Costs

Seeds 42–51, four execution modes with rotated order, eight configurations, fresh process per
sample. Storage uses 1k/10k entities, 25%/100% optional membership, 25 warmup plus 200 measured
ticks, separately timed 1% churn and 1% transfer. Dispatch uses 10k/100k policy or reentry roots
after 256 warmup roots. No threshold was changed after measurement began.

Representative dense/10k storage and 100k reentry configurations:

| Mode | 10k dense update batch p99 | Churn per operation | Transfer per operation | 100k reentry root p99 | Maximum RSS high-water |
| --- | ---: | ---: | ---: | ---: | ---: |
| native | 25.44 ms | 5.60 µs | 5.87 µs | 2.24 µs | 13.92 MiB |
| rust-wasm | 34.77 ms | 5.86 µs | 7.58 µs | 4.60 µs | 24.36 MiB |
| c-wasm | 32.74 ms | 5.49 µs | 7.24 µs | 3.74 µs | 23.65 MiB |
| mixed | 29.97 ms | 5.82 µs | 7.21 µs | 3.08 µs | 23.93 MiB |

Time columns are medians across all ten runs: per-run batch p99, mean churn/transfer cost per
operation, and root p99 respectively. RSS is the maximum high-water over every configuration
for that mode, not the representative-row median. Full spread and sparse/policy results are
retained in the machine summary; no slow run was removed.

The provisional update gate allows 50 µs per total entity after amortizing the batch p99
(**500 ms for 10k entities**); churn/transfer allow 100 µs per operation, root dispatch 50 µs,
and both live/high-water RSS 256 MiB. All groups pass. Across all Wasm artifacts/samples, maximum
cold compilation was 70.10 ms and maximum instantiation 3.82 ms, under 5 s and 100 ms respectively.
Native cold-Wasm cost is not applicable; it does not mean native Rust builds are free.

**The observed 25–35 ms dense update batches do not meet a hypothetical 10 ms whole-map frame.**
That was not this experiment's acceptance budget, and these isolated dispatch-heavy batches
are not a production frame measurement. Measure actual population/activity, frequency, batching,
phase budget and total frame headroom in the first real integration before scaling this pattern.

## Boundaries and next checkpoint

- This is a per-handle dispatch/lookup/codec workload, not a pure hecs query benchmark. Warmup
  leaves most Encounter UPDATE calls in phase 1; churn reintroduces phase 0. No dedicated
  Expedition STAMP performance claim follows from the preregistered workloads.
- Functional fixtures compare full traces; timed campaigns compare retained result/state
  digests and operation counts. Those digests do not establish every intermediate callback
  value in millions of timed roots.
- Native modules are trusted, not fuel-limited/preemptible/sandboxed. Wasm limits are bounded
  host admission tests, not a general security audit of Wasmtime.
- Same-incarnation in-memory snapshot/replay is not SQL durability, crash recovery, receipts,
  state migration or hot reload. All guest/core limits and unsupported-switch behavior remain.
- Strict driver Clippy still reports two existing frozen-code lints: `drop_non_drop` in
  `driver/src/bench/dispatch.rs:96` and `large_enum_variant` in `driver/src/harness.rs:61`.
  They were not silently edited or reclassified as passing. The finite campaign passes its
  declared gates, not a blanket warning-free claim.
- Next: the first **production C1/C2 vertical with C0 admission/phase evidence**, reviewed
  before replication: exact incarnation/residence, coherent save and revision-safe acknowledgement,
  failure/unload, two-map ordering, delivery outside owner locks and retirement of superseded
  writers. Retain every C0–C4 requirement, #583 external/durable/operator delivery and #153 audit.
  No production ECS/Wasm dependency, restart, database mutation, push or macro closure occurs here.
