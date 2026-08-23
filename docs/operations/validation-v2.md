# Validation V2 shadow runner

Validation V2 is a clean-room, shadow-only validation path. It does not replace branch protection,
the current local harness, the exhaustive preflight, architecture audits, capture validation, or
runtime QA yet.

Run it through its single entry point:

```bash
./tools/validation-v2 self-test
./tools/validation-v2 quick --base origin/3.4.3
./tools/validation-v2 final --base origin/3.4.3
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

Every run acquires a non-blocking, worktree-specific lock and writes a JSON manifest under
`target/validation-v2/manifests/`. The manifest records repository and toolchain provenance,
dirty state, kernel, timings, command results, signals, resource limits, and peak child RSS. It
also records the resolved base, complete changed-path set, path classes, direct workspace packages,
reverse-dependent closure, metadata outcome, optional-linter omissions, and exact command plan. It
does not record the environment or command output. Set `VALIDATION_V2_MANIFEST` to choose a result
path. Timestamps, durations, peak RSS, PIDs, and explicitly selected result paths naturally vary;
the profile, provenance, resource policy, routing, command declarations, statuses, and exit
semantics are stable for an unchanged checkout.

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
manifest while Validation V2 remains shadow-only; promotion must provide that dependency or a
hermetic replacement.

Promotion from shadow status requires the wider comparison and migration gates tracked by #302.
