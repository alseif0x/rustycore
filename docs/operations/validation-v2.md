# Validation V2 canonical runner

Validation V2 is the clean-room validation path shared by local development and GitHub Actions.
Legacy wrappers remain frozen only for the measured retirement gate tracked by #302; they are not
called by this runner or by Rust CI.

Run it through its single entry point:

```bash
./tools/validation-v2 self-test
./tools/validation-v2 quick --base origin/3.4.3
./tools/validation-v2 final --base origin/3.4.3
./tools/validation-v2 audit --base origin/3.4.3
```

`self-test` executes the separate hermetic contract suite in `tools/test_validation_v2.py`; fixture
code is not embedded in the production runner.

Both profiles collect committed, staged, unstaged, and untracked paths relative to the exact base
commit. `quick` validates repository hygiene and small syntax surfaces, formats Rust once, and
compiles test targets for directly changed workspace packages. `final` instead compiles the
workspace reverse-dependent closure and runs library tests for the directly changed library
packages. A root Cargo, toolchain, protobuf, or build-script change explicitly expands compilation
to `--workspace --all-targets`; it does not implicitly run every library suite.

Documentation-only changes run no Cargo command. The standalone architecture checker and QA bot
are routed to their own manifests. A final architecture-checker run skips its repository-surface
test: exhaustive architecture, persistence inventory, capture, databases, and runtime QA belong to
future explicit `audit` or QA profiles. Commands run sequentially and each exact command appears at
most once. Neither profile calls a legacy wrapper or uses the network; Cargo is forced offline.

`audit` is the explicit global, read-only budget. It does not use changed-path scope: it runs the
architecture policy checks, handler contract and exhaustive session/persistence ratchets, all
workspace test targets, standalone QA-bot tests, and the required committed capture contracts.
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

An `audit` also acquires `/tmp/rustycore-validation-v2-heavy.lock`. That lock is deliberately not
derived from the checkout path, so audits in independent clones and worktrees cannot overlap on
one host. Lock diagnostics identify the active run id, PID, repository, HEAD, profile and start
time. `quick` and `final` never acquire this heavyweight lock. For hermetic tests only, its path
can be overridden with `VALIDATION_V2_HEAVY_LOCK`.

The conservative defaults are two Cargo jobs and a 900-second per-command timeout. Controlled
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

Legacy retirement still requires the wider comparison gates tracked by #302.
