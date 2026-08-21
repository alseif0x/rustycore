# Local-first development

RustyCore uses a trusted-author split so first-party development is not serialized by hosted CI.
The policy is intentionally narrow:

- a pull request authored by exactly `alseif0x` allocates no GitHub-hosted validation runner;
- every other author, including bots and collaborators, keeps the existing remote checks;
- scheduled and manually dispatched workflows remain available for broad audits;
- first-party validation is local and proportional to the changed paths.

The harness is agent-agnostic. Kimi, Codex, Grok, Claude, any other AI agent, and a human
maintainer all invoke the same non-interactive command and receive the same process exit status.
Trust is derived only from the GitHub PR author's exact login; it is never derived from the tool
that wrote the code.

GitHub still creates the required check names for a trusted pull request, but their jobs are
evaluated as `skipped` before a runner is assigned. This preserves the existing protection for
external contributions without spending hosted compute or waiting for a remote review on normal
maintainer work.

## Daily harness

Use the lightweight harness instead of `tools/pr-preflight.sh` during normal development:

```bash
./tools/local-harness.sh quick
```

`quick` collects committed, staged, unstaged, and untracked paths relative to `origin/3.4.3`, then
runs only applicable checks:

- whitespace checks for the current diff;
- `bash -n` for changed shell scripts;
- `jq empty` for changed JSON;
- Rust formatting only when Rust inputs changed;
- `cargo check --locked --tests` only for directly affected workspace packages, compiling both
  production code and test targets without LLVM linking;
- fast standalone checker tests only when that checker changed;
- the QA bot's own format/check only when the bot changed.

It never runs the workspace-wide persistence inventory, capture QA, a live database, the full
workspace suite, or Codex review.

Before publishing the final commit, run:

```bash
./tools/local-harness.sh final origin/3.4.3
```

`final` applies the same lightweight compile-only gate to the final tree. It does not execute a
monolithic library suite merely because a file moved or a branch was rebased. Run focused tests
explicitly when behavior changes, for example:

```bash
PROTOC=/home/cdmonio/.local/protoc/bin/protoc \
  cargo test --locked -p wow-world exact_test_name --lib
```

Broad test execution remains available through `tools/pr-preflight.sh` for explicit audits and
through scheduled CI. Local harness success is evidence for the maintainer, not a status published
back to GitHub.

### Architecture and module changes

An architecture PR may need focused ownership evidence in addition to the path-routed harness. The
ordinary Session ownership command is the syntax-only ratchet:

```bash
cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check --syntax-only
```

When architecture policies, ledgers, boundary documentation, or checker behavior change, also run:

```bash
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py self-test
```

If an exact syntax surface moved intentionally, first run `check --syntax-only` against the old
snapshot and inspect its reported additions/removals. Generate `print-baseline` into a temporary
file, compare the complete semantic delta, install the snapshot only after that review, and rerun
`check --syntax-only`. Baseline regeneration never authorizes a new owner, public API, mirror,
dependency, or direct storage operation by itself.

The command below is deliberately **not** part of normal local development:

```bash
cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check
```

Without `--syntax-only`, the checker also recomputes the exhaustive workspace persistence
inventory. That can take many minutes and is reserved for a requested persistence audit,
release/scheduled audit, or a change that actually owns that inventory. Do not run it merely
because files or modules moved.

Validation commands must expose their real exit status. Do not append `| head`, `| grep`,
`; echo EXIT=$?`, or another pipeline/trailing command that can turn a failed checker into a
reported success. If output must be retained, redirect it to a log and check the validator's own
exit code; use `set -o pipefail` for any unavoidable pipeline.

The harness supports a routing-only dry run:

```bash
LOCAL_HARNESS_DRY_RUN=1 ./tools/local-harness.sh quick origin/3.4.3
```

It does not require an agent SDK, a model-specific CLI, prompts, or interactive input. Agents can
inspect the stable command interface with `./tools/local-harness.sh --help` and must treat a
non-zero exit status as a failed local gate. The harness invokes plain `cargo`, so rustup selects
the exact compiler from `rust-toolchain.toml`; it imposes no compiler stack or incremental-cache
workaround.

## What remains exhaustive

`tools/pr-preflight.sh` remains available for an explicitly requested audit, release preparation,
capture QA, or investigation of an architecture boundary. It is not the daily pre-push gate and
its local Codex review is optional.

Broad remote validation runs only in these cases:

- a pull request whose author is not `alseif0x`;
- the scheduled Rust CI audit;
- an explicit `workflow_dispatch` run.

A failure in a scheduled audit should produce a focused issue. It does not retroactively turn
every intermediate first-party commit into a review cycle.

## Trust boundary

Do not broaden the trusted condition to `COLLABORATOR`, `MEMBER`, or an author-association class.
The exact login allowlist is deliberate. External code continues to run under the existing
read-only pull-request workflows and required checks. Switching from one local AI agent to another
does not change either side of this boundary.

If another first-party identity is added later, update every trusted-author condition and this
document in one reviewed change. The local harness must remain the same command for humans and
agents so the fast path does not fork into undocumented personal procedures.
