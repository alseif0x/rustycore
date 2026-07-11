# Local PR preflight and Codex review

The local preflight catches deterministic CI failures and review findings before a branch is
pushed. Its profiles mirror the required Rust CI jobs. GitHub keeps the enforcing Cargo commands
inline in the workflow so changing a pull request's local wrapper cannot silently weaken a
required check.

The entry point is:

```bash
./tools/pr-preflight.sh --help
```

It reads `rust-toolchain.toml` and runs the required jobs with Rust `1.88.0`. Protobuf-dependent
commands require the version pinned in `.protoc-version` (`28.3`); the script uses `PROTOC` when
set, then checks the project's usual local install and `PATH`. GitHub downloads the version from
the same file. Review commands require an authenticated `codex` CLI.
Printing `review` or `full` with `--dry-run` does not require Codex or its execution-time helpers.

## Profiles

| Command | Purpose | Starts services or mutates a database? |
|---|---|---:|
| `self-test` | Test harness parsing and pinned-version invariants | No |
| `diff [BASE]` | Whitespace-check committed, staged, and unstaged changes | No |
| `format` | Run harness self-tests and both formatting checks from CI | No |
| `check` | Run locked core checks, bot check, and server builds from CI | No |
| `test` | Run the four focused library suites from CI | No |
| `ci` | Run `format`, `check`, and `test` | No |
| `quick [BASE]` | Run `diff`, `format`, and `check` during iteration | No |
| `capture` | Test the committed C++↔Rust capture-diff fixtures without `protoc` | No |
| `review [BASE]` | Review a clean committed diff with Codex in read-only mode | No |
| `review-uncommitted` | Review staged, unstaged, and untracked work during iteration | No |
| `full [BASE]` | Run `diff`, `ci`, `capture`, and `review` | No |
| `stable` | Check/build server binaries with latest stable Rust | No |
| `qa-login` | Run the integrated live login bot | **Yes** |

`BASE` defaults to `origin/3.4.3`. Use `--dry-run` before a command to print its underlying
commands without executing them or provisioning optional review tools.

`qa-login` is intentionally outside `full`. It requires running services and may create/update
local QA accounts and session data, so it refuses to run without explicit acknowledgement:

```bash
./tools/pr-preflight.sh --allow-runtime-qa qa-login
```

Fresh C++ or Rust packet recording is also intentionally excluded: the capture scripts can
restart services and require an interactive client flow. The safe `capture` profile only tests
the fixtures already committed to the repository and does not invoke protobuf tooling.

## Recommended maintainer flow

During implementation, run focused tests and optionally review the uncommitted patch:

```bash
./tools/pr-preflight.sh quick
./tools/pr-preflight.sh review-uncommitted
```

Commit locally before the final pre-push pass. The clean tree matters because it makes the local
review target the same committed diff that GitHub will see:

```bash
git fetch origin
git commit
./tools/pr-preflight.sh full origin/3.4.3
```

If Codex reports findings, fix them, amend or add the appropriate behavior-complete commit, and
rerun `full`. Push only after it passes. Then open the PR into `3.4.3`; GitHub still runs the same
deterministic command lists and requires its independent Codex review on the exact remote HEAD.

Local review does **not** satisfy branch protection and can differ from the GitHub reviewer because
the model run and context are independent. Its purpose is to remove avoidable push/review/fix
cycles, not to create a maintainer bypass.

## Codex review behavior

The policy in `tools/codex-review-policy.md` supplements `AGENTS.md`, and
`tools/codex-review-schema.json` makes the result machine-readable. Codex runs ephemerally with
user configuration, hooks, plugins, apps, and approval prompts disabled, inside a read-only
sandbox. The harness interprets the structured review rather than assuming a successful process
exit means a clean patch. Review is limited to 1800 seconds by default; set
`CODEX_REVIEW_TIMEOUT_SECONDS` to another positive number when needed.

Exit status:

- `0`: selected deterministic profiles passed and the requested review was clean;
- `10`: Codex returned one or more findings, or judged the patch incorrect;
- `64`: local usage, dependency, dirty-tree, base-ref, toolchain, or protoc error;
- `65`: Codex did not return the expected structured result;
- any other nonzero Codex status is preserved as an operational failure.

Review artifacts are deleted after a clean result. They are retained and printed when review fails.
Set `CODEX_REVIEW_KEEP_ARTIFACTS=1` to retain them after a clean review as well.

## CI source of truth

`.github/workflows/rust-ci.yml` executes the required Cargo commands directly. These local
profiles mirror the workflow-owned command lists:

```text
Format                <-> ./tools/pr-preflight.sh format
Check core crates     <-> ./tools/pr-preflight.sh check
Focused library tests <-> ./tools/pr-preflight.sh test
Latest stable         <-> ./tools/pr-preflight.sh stable
```

When a required command changes, update the workflow and its matching local profile together. The
workflow is authoritative for branch protection; `Format` also runs the harness self-test, but no
required job trusts the pull request's wrapper as its sole enforcement path.
