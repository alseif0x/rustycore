# Validation V2 shadow runner

Validation V2 is a clean-room, shadow-only validation path. It does not replace branch protection,
the current local harness, the exhaustive preflight, architecture audits, capture validation, or
runtime QA yet.

Run it through its single entry point:

```bash
./tools/validation-v2 self-test
./tools/validation-v2 quick
./tools/validation-v2 final
```

`quick` runs whitespace and Rust formatting checks. `final` runs the same commands and one bounded
`wow-core` library smoke test. Commands run sequentially and exactly once. Neither profile calls
the legacy harnesses or exhaustive architecture scanner, starts services, accesses databases, or
uses the network. Cargo is forced offline.

Every run acquires a non-blocking, worktree-specific lock and writes a JSON manifest under
`target/validation-v2/manifests/`. The manifest records repository and toolchain provenance,
dirty state, kernel, timings, command results, signals, resource limits, and peak child RSS. It
does not record the environment or command output. Set `VALIDATION_V2_MANIFEST` to choose a result
path. Timestamps, durations, peak RSS, PIDs, and explicitly selected result paths naturally vary;
the profile, provenance, resource policy, command declarations, statuses, and exit semantics are
stable for an unchanged checkout.

The conservative defaults are two Cargo jobs and a 900-second per-command timeout. Controlled
overrides are validated before execution:

```bash
VALIDATION_V2_CARGO_JOBS=4 VALIDATION_V2_TIMEOUT_SECONDS=1200 \
  ./tools/validation-v2 final
```

Cargo jobs must be between 1 and 8; timeout must be between 30 and 3600 seconds. A concurrent run
fails immediately and reports the lock path and active owner instead of waiting invisibly.

Promotion from shadow status requires the wider comparison and migration gates tracked by #302.
