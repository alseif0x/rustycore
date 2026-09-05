# Validation V2 canonical runner

Validation V2 is the clean-room validation path shared by local development and GitHub Actions.
The old `pr-preflight.sh` and `local-harness.sh` wrappers were retired in #331 (PR #335). Do not use
their former `full` or `capture` subcommands; code review and live QA have separate tools in
[local-first-development.md](local-first-development.md).

Run it through its single entry point:

```bash
./tools/validation-v2 self-test
./tools/validation-v2 quick --base origin/3.4.3
./tools/validation-v2 final --base origin/3.4.3
./tools/validation-v2 audit --base origin/3.4.3
```

`self-test` executes the separate hermetic contract suite in `tools/test_validation_v2.py`; fixture
code is not embedded in the production runner.

The `quick` and `final` profiles collect committed, staged, unstaged, and untracked paths relative to the exact base
commit. `quick` validates repository hygiene and small syntax surfaces, formats Rust once, and
compiles test targets for directly changed workspace packages. `final` instead compiles the
workspace reverse-dependent closure and runs library tests for the directly changed library
packages. A root Cargo, toolchain, protobuf, or build-script change explicitly expands compilation
to `--workspace --all-targets`; it does not implicitly run every library suite.

A `final` run whose diff touches workspace Rust also enforces the curated hotspot LOC ceilings
(`check_architecture.py hotspot-ratchet`; timing depends on cached scanner/build state).
Every nonempty `final` diff also runs the cheap `check_architecture.py physical-files` scan,
including tooling-only, non-Rust, deletion, policy or generator-input changes. It enforces
new-file budgets and reviewed per-file migration ceilings without invoking Cargo. The other
architecture checks and exhaustive
persistence inventory run in `audit`; `final` alone does not verify renamed or relocated
persistence accesses. Run affected ownership/contract checks explicitly during architecture
work and satisfy the active macro's terminal acceptance before claiming completion. Physical
migration PASS is not closeout: run `physical-files --terminal` to reject unfinished oversized
legacy entries, independently of the logical totals. Changes to the physical module/policy
run its unit suite in `quick`; shared checker/scanner changes run architecture self-tests. See
[module design guidelines](../architecture/module-design-guidelines.md).

Paths classified as `documentation` run no Cargo command. Classification is directory-first:
even a README under `crates/`, `tools/wow-test-bot/` or
`tools/architecture/handler-contract-check/` takes that directory's Cargo route. The standalone
checker and QA bot use their own manifests. A final architecture-checker run executes all its
library tests, including the now syntax-only `repository_surface_can_be_collected`; it does not
recompute the exhaustive persistence inventory. Committed capture contracts belong to `audit`,
and live database/runtime/capture operations to explicit QA procedures. Commands run sequentially and each exact command appears at
most once. Neither profile calls a legacy wrapper or uses the network; Cargo is forced offline.

`audit` is the explicit global, read-only budget. It does not use changed-path scope: it runs the
architecture policy checks, handler contract and exhaustive session/persistence ratchets, all
workspace test targets, standalone QA-bot tests, and explicit `verify-required` checks for
`loot-single-item-claim` and `creature-spell-casting`. Other action-specific capture requirements
remain the responsibility of the active issue; the profile does not discover every required flow.
The generated `world-modules` launcher declares `test = false`: Cargo's explicit `--all-targets`
override is therefore excluded for that package, and the real launcher is compiled separately
with `cargo check -p world-modules`. Every
step has an owner name in the manifest and stops the audit immediately on failure. It never starts
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
```

which exits non-zero for a missing, unreadable, schema-mismatched, signalled, failed, or truncated
manifest — including a `passed` manifest that executed fewer commands than its plan declared. Rust
CI runs this step after every profile, before the artifact upload.

## protoc

Cargo build scripts need `protoc`, and it is not always on `PATH`. Before planning, the runner
resolves the version pinned in `.protoc-version`: an explicit `PROTOC`, then `PATH`, then
`$HOME/.local/protoc/bin/protoc`. A binary that reports a different version is rejected by name
rather than used, and a plan that compiles Rust without a resolved protoc fails immediately with
that reason instead of surfacing later as an unreadable prost-build error inside a build log. A
documentation-only plan needs no protoc.

Every run acquires a non-blocking, worktree-specific lock and writes a JSON manifest under
`target/validation-v2/manifests/`. The manifest (schema 4) records repository and toolchain
provenance, dirty state, kernel, timings, command results, signals, failure kinds, OOM-kill
deltas, resource limits, and peak child RSS. It
also records the resolved base, complete changed-path set, path classes, direct workspace packages,
reverse-dependent closure, metadata outcome, optional-linter omissions, and exact command plan. It
does not record the environment or command output. Set `VALIDATION_V2_MANIFEST` to choose a result
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
independent hosts, each running one profile, verifying its manifest and uploading the normalised
form, followed by a job that fails with a diff unless all twenty hash identically. It is
`workflow_dispatch` only — evidence, not a gate.

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
./tools/validation-v2 quick --base HEAD~1
```

`--base HEAD~1` is deliberate: at `origin/3.4.3` a fresh clone has no changed paths, so the
profiles would plan nothing. This checks the latest commit's routed scope, not a clean full-server
build; a documentation-only last commit may still run no Cargo commands. Use an explicit build
or the separately budgeted `audit` when that broader evidence is required.

An `audit` also acquires `/tmp/rustycore-validation-v2-heavy.lock`. That lock is deliberately not
derived from the checkout path, so audits in independent clones and worktrees cannot overlap on
one host. Lock diagnostics identify the active run id, PID, repository, HEAD, profile and start
time. `quick` and `final` never acquire this heavyweight lock. For hermetic tests only, its path
can be overridden with `VALIDATION_V2_HEAVY_LOCK`.

The conservative defaults are two Cargo jobs and a 900-second per-command timeout, except that
`audit` defaults to 3600 seconds: its exhaustive persistence inventory alone runs 870-900 seconds
on a four-core host, so the ordinary budget would kill it - correctly reported as
`failure_kind: timeout`, but for no useful reason. Controlled
overrides are validated before execution:

```bash
VALIDATION_V2_CARGO_JOBS=4 VALIDATION_V2_TIMEOUT_SECONDS=1200 \
  ./tools/validation-v2 final
```

Cargo jobs must be between 1 and 8; timeout must be between 30 and 3600 seconds. A concurrent run
fails immediately and reports the lock path and active owner instead of waiting invisibly.

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
uploads the manifest even on failure; signals and timeouts therefore cannot become silent passes.
Repository-level Actions concurrency serializes audits, while superseded external-PR final runs
are cancelled.

Publication evidence belongs to the final clean committed HEAD. The runner permits dirty
iteration and records it; its successful exit does not by itself certify a clean publication SHA
or completion of an issue's capture/live acceptance.
