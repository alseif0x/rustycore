# Validation V2 canonical runner

Validation V2 is the clean-room validation path shared by local development and GitHub Actions.
The old `pr-preflight.sh` and `local-harness.sh` wrappers were retired in #331 (PR #335). Do not use
their former `full` or `capture` subcommands; code review and live QA have separate tools in
[local-first-development.md](local-first-development.md).

Run it through its single entry point:

```bash
./tools/validation-v2                              # level 1 / none (default)
./tools/validation-v2 2 --base origin/3.4.3       # level 2 / quick
./tools/validation-v2 3 --base origin/3.4.3       # level 3 / final
./tools/validation-v2 audit --base origin/3.4.3
./tools/validation-v2 self-test
```

No profile defaults to canonical profile `none` (numeric alias `1`). The numeric
aliases `2` and `3` select the canonical `quick` and `final` profiles. `audit` and
`self-test` are expert explicit commands, not additional daily development levels.

Level 1 prints **NOT VALIDATED**, performs no Git, Rust, `protoc`, metadata (including
Cargo metadata), lock or check command, produces no acceptance manifest, and exits `0` only as an
acknowledgement. It is not validation evidence.

For a completed nonempty delivery, retain diagnostics in the same campaign:

```bash
./tools/validation-v2 final --base origin/3.4.3 --require-changes --timings --logs
```

`--require-changes` rejects an empty changed-path scope in level 2/3 (`quick`/`final`) rather
than presenting it as acceptance. Without that flag, an empty scope remains a
permitted no-op, explicitly reported as **no checks executed**. This is not proof
that a build or all issue-specific checks passed.

`self-test` executes the separate hermetic contract suite in `tools/test_validation_v2.py`; fixture
code is not embedded in the production runner. This command does not probe or
require the host's protoc; its tests supply fake compiler/build tools and never
compile the server.

Level 2 (`quick`) collects committed, staged, unstaged, and untracked paths relative to
the exact base commit and performs only Git diff/whitespace checks, changed
shell/JSON/Python syntax, optional `actionlint`, and `cargo fmt` for routed workspace
or standalone tools. Cargo fmt may inspect manifests. Level 2 never runs `cargo
check`, `cargo test`, `cargo build`, `cargo run`, locked dependency metadata, a
protobuf probe, or self-test/architecture/contract suites.

Level 3 (`final`) collects the same path scope and preserves the current final
acceptance: it compiles the workspace reverse-dependent closure and runs library
tests for directly changed library packages. A root Cargo, toolchain, protobuf, or
build-script change explicitly expands compilation to `--workspace --all-targets`;
final retains directly changed library suites, or tests every workspace library when
no library source was directly changed. Root `.cargo/config.toml` and legacy
`.cargo/config` changes (including deletions) always select every workspace library
in final, even alongside a narrower source diff: their flags/targets affect every
package. They also select the standalone checker and QA-bot routes, because Cargo
reads the root configuration for their `--manifest-path` calls from this checkout.
These global changes can exceed an ordinary narrow-change budget; record that cost
rather than silently omitting affected consumers.

Levels 2 and 3 are alternative budgets, not mandatory successive stages. Level 2 is
local hygiene and is not final acceptance. At completed-delivery acceptance, plan the
missing issue-specific evidence and the committed-candidate `final`/level 3 once.
Its downstream check replaces an equivalent manual preflight; its full library suites
also provide evidence for the focused cases they actually execute. Keep additional
integration, ownership, capture and live checks whose acceptance is not covered. A
changed candidate or a failed check needs renewed affected evidence; a new agent,
commit message, or handoff does not by itself require recompiling unchanged inputs.
Do not use repeated compiler runs to discover consumers or drive one-field-at-a-time
replacements.

Cargo test batches use `--no-fail-fast`: a failing test binary does not hide the
remaining selected suites. This neither adds packages/features nor suppresses a
failure. Preserve the batch's package/target/feature selection when investigating
or rerunning it: dropping a `-p` can alter unified dependency features and rebuild
already compiled libraries. The failed-command message and manifest retain the
original argv; the runner never automatically retries or reuses an older verdict.

When a known ordinary gate failure would otherwise force manually reconstructing
the remaining acceptance, `final --keep-going` collects the planned checks in one
invocation. It records `failure_policy: collect-independent` and remains **failed**
if any check fails. Timeout, OOM, signals, interrupted execution and lost command
logging stop this mode immediately. Default fail-fast behavior and explicit
ownership continuations remain unchanged without this flag. This option is not a
waiver, a publication pass, or permission to refresh policy ceilings; do not repeat
an unchanged red campaign or continue resource failures merely to gather output.

When acceptance requires architecture policy checks, their fixtures and syntax ownership,
use one measured campaign:

```bash
./tools/validation-v2 final --base origin/3.4.3 --architecture --timings
```

`--architecture` is exclusive to `final`. It replaces that plan's separate physical,
hotspot and architecture self-test scans with `check_architecture.py check --self-test`,
which executes their complete union plus dependency checks in one process. The existing
in-process inventories and per-file physical counts are reused, with no persisted scan
cache. Freeze source inputs for the invocation; the next invocation reads them anew.
The runner also executes
`session-ownership-check check --syntax-only`; both commands and their durations appear in
the same final manifest, and the policy step declares the syntax-ownership step as its
continuation. A red policy step therefore still runs the ownership command and the manifest
records both failures with the policy step's exit code; without that continuation a policy
breach, such as the reviewed-but-open hotspot ratchet, hides the ownership verdict. Existing
workspace compilation, test targets and test execution
are unchanged. Do not run those architecture commands separately again for the same inputs.
The flag also honors an explicit request when there are no changed paths. It does not cover
exhaustive persistence inventory, terminal physical closeout, production integration or live
QA when the issue requires those; include their actual additional time in campaign evidence.

A `final` run whose diff touches workspace Rust also enforces the curated hotspot LOC ceilings
(`check_architecture.py hotspot-ratchet`; timing depends on cached scanner/build state).
Every nonempty `final` diff also runs the cheap `check_architecture.py physical-files` scan,
including tooling-only, non-Rust, deletion, policy or generator-input changes. It enforces
new-file budgets and reviewed per-file migration ceilings without invoking Cargo. The other
architecture checks and exhaustive
persistence inventory run in `audit`; plain `final` does not verify renamed or relocated
persistence accesses. Run affected ownership/contract checks explicitly during architecture
work and satisfy the active macro's terminal acceptance before claiming completion. Physical
migration PASS is not closeout: run `physical-files --terminal` to reject unfinished oversized
legacy entries, independently of the logical totals. Changes to the physical module/policy
require the relevant final acceptance; shared checker/scanner changes are automatically routed
through final's architecture/self-test coverage. See [module design guidelines](../architecture/module-design-guidelines.md).

Paths classified as `documentation` run no Cargo command. Classification is directory-first:
even a README under `crates/`, `tools/wow-test-bot/` or
`tools/architecture/handler-contract-check/` takes that directory's Cargo route. The standalone
checker and QA bot use their own manifests. A final architecture-checker run executes all its
library tests, including the now syntax-only `repository_surface_can_be_collected`; it does not
recompute the exhaustive persistence inventory. Committed capture contracts belong to `audit`,
and live database/runtime/capture operations to explicit QA procedures. Commands run sequentially and each exact command appears at
most once. Neither level-2 nor level-3 profile calls a legacy wrapper or uses the network; Cargo is forced offline.

`audit` is the explicit expert global, read-only budget. It does not use changed-path scope: it runs the
architecture policy checks, handler contract and exhaustive session/persistence ratchets, all
workspace test targets, standalone QA-bot tests, and explicit `verify-required` checks for
`loot-single-item-claim` and `creature-spell-casting`. Other action-specific capture requirements
remain the responsibility of the active issue; the profile does not discover every required flow.
The generated `world-modules` launcher declares `test = false`: Cargo's explicit `--all-targets`
override is therefore excluded for that package, and the real launcher is compiled separately
with `cargo check -p world-modules`. Every
step has an owner name in the manifest. Architecture policy and fixtures use one
`check_architecture.py check --self-test` invocation, preserving their union and
dependency checks without scanning the same policy twice. Audit stops at the first
failed step, including this combined policy step: it does not spend the exhaustive
persistence budget after an already-red policy check. A green audit still requires
all 13 declared steps. This is separate from final's explicitly paired, bounded
syntax-ownership continuation described above. It never starts
services, connects to a database, records a fresh capture, regenerates a baseline, invokes Codex,
or calls either legacy wrapper. Those mutating or live operations require their own explicit QA
procedure.

## Verdict rules

A run is green only when every executed command is green. The runner refuses the failure modes
that used to read as a pass:

- a step that times out is `failed` even when the child traps `SIGTERM` and exits `0`, and
  `run_steps` stops on the step's status rather than on its exit code;
- a failed step whose exit code is nevertheless `0` returns exit `70`, never `0`;
- a terminating signal to the runner itself (`SIGINT`, `SIGTERM`, `SIGHUP`) is raised as an
  exception, so the child's process group is stopped, the interrupted command is recorded with
  `failure_kind: "interrupted"`, and the manifest is written with exit `128 + signal`;
- before writing, the manifest is re-read against the same rules a consumer applies; an
  inconsistent green is downgraded to exit `70`.

Each command records why it is not green in `failure_kind`: `oom`, `timeout`, `signal`,
`child-signal`, `exit`, or `interrupted`. `oom_kills` is the kernel OOM-kill delta charged to the
runner's cgroup for that command (`null` where cgroup v2 is unobservable), which is what separates
an OOM kill from a plain `kill -9`. `child_signal_reports` preserves a signalled grandchild that
Cargo hides behind its own exit `101` — for example `(signal: 6, SIGABRT)` from an aborted test
binary. `resources.memory_limit_kib` records the cgroup or host memory ceiling next to the peak
child RSS.

A runner killed outright (`SIGKILL`, OOM killer, cancelled job) cannot write anything, so the
consumer rule is explicit: **a missing manifest is a failed run.** Verify one with

```bash
./tools/validation-v2 verify --manifest <path>
./tools/validation-v2 verify --manifest <path> --require-profile final
```

which exits non-zero for a missing, unreadable, schema-mismatched, signalled, failed, or truncated
manifest — including a `passed` manifest that executed fewer commands than its plan declared. Rust
CI runs this step after every manifest-producing profile, before the artifact upload. The optional
`--require-profile <canonical-profile>` rejects evidence from a weaker or different profile; for
example, `--require-profile final` rejects a level-2/`quick` manifest as final evidence.

## protoc

Cargo build scripts need `protoc`, and it is not always on `PATH`. Levels 1 and 2 do
not resolve or probe it. For level 3/final and `audit`, before planning Cargo work,
the runner resolves the version pinned in `.protoc-version`: an explicit `PROTOC`,
then `PATH`, then `$HOME/.local/protoc/bin/protoc`. A binary that reports a different
version is rejected by name rather than used, and a plan that compiles Rust without a
resolved protoc fails immediately with that reason instead of surfacing later as an
unreadable prost-build error inside a build log. Documentation-only and level-2 plans
need no protoc.

Every manifest-producing level-2/3, `audit` or `self-test` run acquires a non-blocking, worktree-specific lock and writes a JSON manifest under
`target/validation-v2/manifests/`. The manifest (schema 4) records repository and toolchain
provenance, dirty state, kernel, timings, command results, signals, failure kinds, OOM-kill
deltas, resource limits, and peak child RSS. It
also records the resolved base, complete changed-path set, path classes, direct workspace packages,
reverse-dependent closure, metadata outcome, optional-linter omissions, and exact command plan. It
does not record the environment or command output. Optional `--logs` stores command
output separately, as described below. Set `VALIDATION_V2_MANIFEST` to choose a result
path. Timestamps, durations, peak RSS, PIDs, and explicitly selected result paths naturally vary;
the profile, provenance, resource policy, routing, command declarations, statuses, and exit
semantics are stable for an unchanged checkout.

## Determinism

Two runs of the same profile on the same commit must produce the same manifest once the fields
that cannot repeat are removed. The runner owns that comparison form:

```bash
./tools/validation-v2 normalize --manifest <path>
```

The contract is an **allowlist**, not a denylist: every field a manifest may carry is named in the
runner, so a field added later fails the comparison instead of slipping through it. Dropped:
`run_id`, `started_at`, `ended_at`, `duration_seconds`, `peak_child_rss_kib`, and the three
timestamps inside each command. Replaced with a placeholder because they describe the host, not
the run: `provenance.repository_root`, `provenance.kernel`, `resources.memory_limit_kib`,
`locks.repository`, `locks.heavy`. Everything else — profile, status, exit code, runner error and
signal, HEAD, dirty state, both Rust versions, the entire plan, and every command's argv, section,
status, failure kind, OOM delta and signal reports — is compared exactly.

For an explicitly requested determinism campaign, twenty local runs, keeping every manifest
and stopping on the first failure (not an ordinary iteration requirement):

```bash
set -euo pipefail
validation_evidence_dir=$(mktemp -d)
for run in $(seq 1 20); do
  VALIDATION_V2_MANIFEST="$validation_evidence_dir/$run.json" ./tools/validation-v2 self-test > /dev/null
  ./tools/validation-v2 normalize --manifest "$validation_evidence_dir/$run.json" > "$validation_evidence_dir/$run.norm"
done
test "$(sha256sum "$validation_evidence_dir"/*.norm | awk '{print $1}' | sort -u | wc -l)" -eq 1
```

Twenty isolated GitHub runs are the `Validation determinism` workflow: a 20-job matrix on
independent hosts, each running the hermetic `self-test` profile, verifying its manifest and uploading the normalised
form, followed by a job that fails with a diff unless all twenty hash identically. It is
`workflow_dispatch` only — evidence, not a gate. It does not offer `quick`/`final`,
which require project-specific offline Cargo and pinned protoc preparation and
are not hermetic twenty-run orchestration fixtures.

## Fresh clone

The checkout must not rely on an older target directory or generated artifact. First install
the toolchain from `rust-toolchain.toml`, the protoc version from `.protoc-version`, and `ripgrep`.
CI also installs pinned actionlint and checksum-verified C++ statement references; see
`.github/workflows/rust-ci.yml`. Cargo is forced offline during validation, so prepare dependencies
with the same explicit fetches as CI:

```bash
git clone https://github.com/alseif0x/rustycore.git fresh && cd fresh
cargo fetch --locked
cargo fetch --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml
cargo fetch --locked --manifest-path tools/wow-test-bot/Cargo.toml
./tools/validation-v2 self-test
./tools/validation-v2 2 --base HEAD~1
```

`--base HEAD~1` is deliberate: at `origin/3.4.3` a fresh clone has no changed paths, so the
level-2/3 profiles would plan nothing. This checks the latest commit's routed scope, not a clean
full-server build; a documentation-only last commit may still run no Cargo commands. Use level 3,
an explicit build, or the separately budgeted `audit` when that broader evidence is required.

An `audit` also acquires `/tmp/rustycore-validation-v2-heavy.lock`. That lock is deliberately not
derived from the checkout path, so audits in independent clones and worktrees cannot overlap on
one host. Lock diagnostics identify the active run id, PID, repository, HEAD, profile and start
time. `quick` and `final` never acquire this heavyweight lock. For hermetic tests only, its path
can be overridden with `VALIDATION_V2_HEAVY_LOCK`.

The conservative defaults are one Cargo job and a 900-second per-command timeout, except that
`audit` defaults to 3600 seconds: its exhaustive persistence inventory alone runs 870-900 seconds
on a four-core host, so the ordinary budget would kill it - correctly reported as
`failure_kind: timeout`, but for no useful reason. Controlled
overrides are validated before execution:

```bash
VALIDATION_V2_CARGO_JOBS=1 VALIDATION_V2_TIMEOUT_SECONDS=1200 \
  ./tools/validation-v2 final
```

Cargo jobs must be between 1 and 8; timeout must be between 30 and 3600 seconds. A concurrent run
fails immediately and reports the lock path and active owner instead of waiting invisibly.

### Cargo artifacts and disk space

The ordinary local acceptance performance budget is **600 seconds for the complete
campaign**, including necessary additional checks, with the active cache warm. Measure
implementation/error-repair time separately. The target does not waive any required
coverage or declare cold bootstrap, exhaustive audits or live QA complete in ten minutes.
Record those distinct costs explicitly. A run longer than 600 seconds has not met the
performance target even if every correctness check passes; a timeout is not a speedup.

Collect Cargo's stable timing reports in the campaign that is already required:

```bash
./tools/validation-v2 final --base origin/3.4.3 --timings
```

`--timings` is useful for `final` and `audit`; level 2 accepts it for compatibility but
has no check/test/build/run command to instrument. It adds Cargo's reporting flag to
planned check/test/build/run commands, before any program-argument separator, without
changing their package/target selection or running another build. The manifest records
the instrumented commands; timestamped reports remain in `target/cargo-timings` under the
selected Cargo target. See [Cargo timing reports](https://doc.rust-lang.org/cargo/reference/timings.html).
Keep the runner's total time and the duration of required extra checks; individual crate
reports do not measure the complete acceptance campaign. Benchmark changes in job count,
profiles or linking on representative unchanged inputs before adopting them, with exclusive
validation ownership and measured memory headroom. Avoid an extra warmup merely for timing.

For the narrower question of Git-revision invalidation and dev-profile precedence,
there is an opt-in isolated diagnostic:

```bash
python3 tools/measure_build_inputs.py --timeout 30 \
  --output target/validation-v2/build-inputs.json
```

Use a new report path for each run. It copies the current `world-server/build.rs`
and dev-profile tables into tiny local fixtures, pins the repository toolchain,
and uses private temporary Cargo homes/targets, offline and one job. Its Git commits
affect only the fixtures. Incremental codegen is disabled only in these controlled
fixtures; the report records that difference from the normal dev environment.
It records cold, unchanged warm and documentation-only
commit artifacts/revisions, plus an external dependency/proc-macro comparison with
the wildcard override removed. It does not alter the checkout's profiles, build
script, active cache, runtime or database. Fixture unit tests are routed by final;
the real Cargo diagnostic remains opt-in and sequential with other validation.

This proves input invalidation and effective options, not a representative server
speedup. Its isolated cold costs and wall times are diagnostic context, not the
ordinary warm-workspace benchmark. Keep the actual report, source SHA/hashes and
limitations; an incomplete observation is not a green result. Do not replace the
embedded server revision with a stale value to make a build appear reusable.

The runner uses `<checkout>/target` by default, matching ordinary Cargo commands in that
workspace. An explicit nonempty `CARGO_TARGET_DIR` is respected; relative values are resolved
against the checkout. Keep one target per active worktree rather than sharing a mutable cache
between independent worktrees. Do not change debug, incremental or codegen flags midway through
acceptance to save space: those changes invalidate reusable compilation and need their own
measured tradeoff. Both `check` and `test` remain required where routed; they produce different
artifacts and are not interchangeable evidence.

For direct acceptance involving the standalone checker or bot, run from the checkout root:

```bash
export CARGO_TARGET_DIR="$PWD/target"
export CARGO_BUILD_JOBS=1
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc
```

The former `target/validation-v2/cargo/<worktree-hash>` is a separate legacy cache. After
switching a checkout to the new runner, it can be removed only when no process uses it;
retain `target/validation-v2/manifests` and any required evidence. No automatic cache deletion
occurs in validation. The first run may need artifacts absent from the selected target.

On the shared development host with a 193 GiB filesystem, aim to retain at least 30 GiB free
before starting a large Rust acceptance. Use `df -h` and a targeted `du` to measure pressure.
If space is tight, inspect inactive worktrees' generated targets first. Check active agents,
Cargo processes and open files before deleting any selected directory; exclude tracked or
user-authored files, running/deployed executables, rollback builds, databases, captures and
manifests. Do not delete an entire worktree just to reclaim its generated artifacts. Recheck
free space after cleanup. Deleting the active incremental cache at each macro is not a
retention policy and needlessly repeats compilation; if safe cleanup cannot restore headroom,
pause new builds and report the concrete storage need while continuing safe inspection.

### Visible output and background tasks

Invoke the runner directly so its progress and real exit status remain visible. Never pipe
validation into `tail`, `head`, or `grep` and treat the consumer's status as success. To retain
output, prefer `--logs` or redirect to a log, preserve the runner's exit code, then
inspect that log separately.

`--logs` creates `<manifest-stem>-logs/` beside the manifest, with mode 0700, and
one mode-0600 file per executed command. Each starts with the exact argv and start
time followed by combined stdout/stderr; numbered filenames identify the planned
section. Output still streams normally. No environment is dumped. Existing log
directories/files, including symlinks, are rejected rather than overwritten.
Use a fresh `VALIDATION_V2_MANIFEST` path for each logged invocation. A log I/O
failure cannot yield a green run (`failure_kind: output-log`). Logs survive test
failure and interruption; absence/truncation of a manifest remains a failed run.

Logs are opt-in because command output can contain sensitive data: keep private
logs local, inspect before sharing, and never stage them. Hosted Rust CI enables
logs/timings for its non-live checks and uploads only the explicit manifest/log
directory and Cargo timing HTMLs, never the whole target or local configuration.
Do not run a suite again solely because the interactive output was truncated.

The agent tool's foreground wait is independent of the runner's per-command timeout. When
Claude or another harness returns a background task ID after a wait expires, keep tracking
that exact task and its output until it exits. Do not start a replacement or launch a manual
Cargo warmup while it may still be running. A wait expiry is not a failed validation. `SIGTERM`
or exit 143 is a real interrupted run: check the manifest, process state, and harness task
result before retrying once the cause is understood. Do not assume disk pressure or OOM from
the signal alone, disable permission controls, or detach a job to evade a harness rejection.
Do not stop unrelated processes or reuse a green manifest from a different candidate.

The base must already exist locally. Validation never fetches it:

```bash
git fetch origin 3.4.3
./tools/validation-v2 final --base origin/3.4.3
```

Workflow YAML uses `actionlint` when installed. Its absence is an explicit optional skip in the
manifest locally. GitHub Actions installs the pinned, checksum-verified actionlint release before
running Validation V2, so changed workflow syntax is always checked remotely.

Rust CI checks out the exact event SHA with full history, prepares the pinned Rust/protoc/actionlint
and locked Cargo inputs, then invokes this same executable once. External pull requests run the
bounded `final` profile against the exact pull-request base SHA. First-party pull requests are
skipped and do not wait for hosted validation. Pushes to `3.4.3`, the weekly schedule, and explicit
`audit` dispatches run the exhaustive profile on an independent GitHub host. Every hosted run
uploads the manifest and diagnostic artifacts even on failure; signals and timeouts
therefore cannot become silent passes. Manually dispatched `final` requires an explicit
base resolving to a strict ancestor of the checked-out SHA, checked before expensive
toolchain/dependency setup. It also uses `--require-changes`; a same-tree or empty
diff cannot masquerade as completed acceptance. PRs retain their exact event base.
Repository-level Actions concurrency serializes audits, while superseded external-PR final runs
are cancelled.

## Publication evidence without redundant builds

Validate the committed publication candidate, not the cosmetic cleanliness of the directory.
Normally run `final --base origin/3.4.3` with no tracked modifications. Unrelated untracked
documents may remain when inspection establishes that no build script, test, generator or
configuration consumes them. Record their paths and hashes, verify tracked files still match
HEAD after the run, and retain the manifest's truthful `dirty: true` value. Do not delete,
stage, hide or relocate another task's files merely to obtain `dirty: false`. An uncertain
input or a source/configuration change needs isolation or appropriate revalidation.

A successful final run may also be reused when the only subsequent committed changes are
reviewed documentation or operating-instruction edits with no executable, schema, policy-data,
dependency, build, fixture or test-input changes. Run `quick --base <validated-sha>` for that
delta, check the exact diff and unchanged integration base, and record both SHAs and manifests.
The evidence is a tested code revision plus a validated documentation delta, not a claim that
the older run executed at the new SHA. Changes outside this exception require the normal final
profile. Do not restart an already-running equivalent validation merely for an evidence note.

Publication validation and merge acceptance are distinct: neither this exception nor a green
profile waives explicit review, capture/live acceptance, or push/merge/runtime permissions.

## #1232 workflow acceptance — 2026-09-22

Implementation candidate `e1190ff5c4c8b4b848daad95995224599885caba`, base
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, clean tracked/untracked tree, aarch64
Neoverse-N1 host, Rust 1.98.0. Command:

```bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc VALIDATION_V2_CARGO_JOBS=1 \
  ./tools/validation-v2 final --base origin/3.4.3 --require-changes --timings --logs
```

The pinned actionlint 1.7.12 ARM64 binary was available on PATH for this run; its
official archive SHA-256 was checked before the campaign:
`325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6`.
Downloading/preparing that tool was setup, not a compilation-speed measurement.

Code-acceptance campaign: **11:18:21–11:18:51 UTC (30 seconds)** including manifest
verification and provenance readback. The runner itself took **10.321 seconds**,
11:18:21.942–11:18:32.264, with **7/7 checks passed**, no optional skips, no failed
attempts or repairs. Manifest:
`target/validation-v2/manifests/20260922T111821.942954Z-2-final.json`.

Covered: physical source policy (2,244 files), hygiene/Python syntax, runner
contracts (including logging permissions/no overwrite, interruption, empty scope,
continued ordinary failures/fatal stops and unchanged batch selection), actionlint
for both workflows, and hermetic workflow/base ancestry/injection/empty-tree tests.
The independent collaborator wrote the CI changes without running a separate
campaign; the parent validated the integrated candidate once.

No production Rust changed or compiled. This evidence does **not** claim a faster
full Rust build, a hosted workflow run, exhaustive audit/live QA, or resolution of
the pre-existing global hotspot debt. No architecture ceiling, trust boundary,
compiler profile, gameplay or runtime service changed. The documentation-only
evidence delta after this candidate uses the reuse rule above; it must not be
reported as a rerun of this manifest at a later SHA.

### Follow-up: fail-fast, Cargo configuration and measured inputs

Code candidate `5dd4a0ffbe5656027646adcd3221b65929258023`, same integration base
`9daa13f6`, clean tree, aarch64 host, Rust 1.98.0. Final used the same command
above, with the checkout's absolute `CARGO_TARGET_DIR` and actionlint on PATH.
It passed **8/8 steps in 9.748 seconds**, including eight diagnostic unit tests.
Manifest: `target/validation-v2/manifests/20260922T120941.503363Z-1295226-final.json`.
The additional command was:

```bash
python3 tools/measure_build_inputs.py --timeout 30 \
  --output target/validation-v2/build-inputs-5dd4a0ff.json
```

The diagnostic passed in **2.132 seconds**. All four Cargo builds used private
temporary targets, offline, one job; no production crate or active cache was built
or cleared. The build-script source hash was
`7fdfdfcedf30ad300bc365dd0951db58cc19ef51127ae72f0173d00e0db4dd29` and the profile
manifest hash was `a1ebfa5a00c9d4086f2aa8e360736da141469ac4c81e971fb8a5566e73cb1a45`.

- Audit regression fixtures now prove that a red initial policy stops after two
  steps, before Cargo/persistence scanning, while a green plan executes all 13.
  Final's bounded syntax-ownership pairing and explicit keep-going remain intact.
- Both Cargo configuration spellings select workspace compilation and all library
  suites, plus the standalone tool routes; mixed changes/deletions and duplicate
  routing are covered. This closes a false-green gap, not a claim that global
  configuration acceptance is cheap.
- The unchanged tiny library was fresh (0.025 s build); a README-only commit made
  it non-fresh (0.070 s), with the new commit actually found in the artifact.
  This demonstrates revision invalidation, not its cost in `world-server` with
  normal incremental codegen.
- The external library/proc macro both used opt-level **2** with the current
  wildcard; without that wildcard they used **0/1**, respectively. Thus the
  build-override comment in root Cargo.toml is not a guarantee of opt-level 1
  for external proc macros or a net build-time saving. This matches Cargo's
  [override precedence](https://doc.rust-lang.org/cargo/reference/profiles.html#overrides).
  No profile or revision-provenance behavior was changed; representative timing
  is still required before adopting such an optimization.

Code acceptance window: **12:05:37–12:10:16 UTC (279 seconds)**, including the
first final run, one inconclusive experiment, diagnosis/repair, corrected final,
corrected experiment and manifest/provenance readback. The first final at
`94353726` passed; its diagnostic returned **2/inconclusive** in 2.752 seconds,
retained as `target/validation-v2/build-inputs-94353726.json`. A metadata-only
probe showed the initial fixture dependencies had become workspace members despite
their exclusion entries. Repair/diagnosis occupied approximately 12:06:22–12:09:41
(199 seconds), separately from command execution. The corrected fixture puts them
outside the app workspace and checks actual metadata membership before compiling;
the unchanged inconclusive experiment was not repeated.

Implementation/review before that window was not timed. The documentation-only
evidence commit and its quick validation occur after this code window and reuse
the green code evidence; they do not relabel either manifest. The complete
closeout timing, including that additional check, is reported in the handoff.
No full-server speedup, exhaustive audit, live acceptance, push or merge is claimed.

### Follow-up: explicit local development levels

Code candidate `46751735e3da1063fa3caba6c160f6f2129386d1`, integration base
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, clean tree, aarch64 host, Rust
1.98.0. The final command was:

```bash
PATH=/tmp/rustycore-1232-actionlint.Ow7AT6:$PATH \
PROTOC=/home/ubuntu/.local/protoc/bin/protoc \
VALIDATION_V2_CARGO_JOBS=1 CARGO_TARGET_DIR=/home/server/rustycore/target \
  ./tools/validation-v2 final --base origin/3.4.3 --require-changes --timings --logs
./tools/validation-v2 verify \
  --manifest target/validation-v2/manifests/20260922T211123.183692Z-1797163-final.json \
  --require-profile final
```

The retained `actionlint` 1.7.12 ARM64 archive matched SHA-256
`325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6` before
the campaign. The runner took **10.051 seconds** and passed **8/8 checks**, with
no optional skips; the complete campaign including manifest verification and
provenance readback took **45 seconds (21:11:23–21:12:08 UTC)**. The manifest
above was verified with `--require-profile final`.

The checks covered the physical policy (2,246 files), changed-file hygiene and
Python syntax, the runner's contract suite, both workflows with `actionlint`,
workflow contract tests and all eight build-input diagnostic tests. The runner
contract fixtures deliberately exercise command failures, signals and timeouts;
one recorded child `SIGABRT` belongs to that passing fixture suite and did not
fail the campaign. The plan had no workspace or Cargo metadata selection and
ran no Cargo commands; no production Rust was compiled. This accepts the new
default/quick/final contracts and verification-level guard, not a full Rust build
speedup, hosted workflow, exhaustive audit or live acceptance.
