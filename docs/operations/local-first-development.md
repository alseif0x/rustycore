# Local-first development

On `forever`, run local publication acceptance on the committed candidate. The default
comparison for feature work is `origin/forever`. Cargo needs the [pinned sibling extractor](../../README.md#required-checkout-layout).

For feedback on uncommitted edits, run `./tools/validate-change --working --base origin/forever`. Use `--plan` to inspect the package and consumer selection. For a focused library test, supply `--package <crate> --focus <test-name>`. The editing command and the final publication check serve different stages.

## Three local levels

`tools/validation-v2` supplies the local acceptance manifest. The [runner contract](validation-v2.md) describes its levels.

```bash
./tools/validation-v2                              # level 1 / none: NOT VALIDATED
./tools/validation-v2 2 --base origin/forever       # level 2 / quick hygiene
./tools/validation-v2 3 --base origin/forever --require-changes --timings --logs
```

Level 1 is the default working mode: no profile selects canonical `none` (numeric alias `1`),
prints **NOT VALIDATED**, performs no Git, Rust, `protoc`, Cargo-metadata, lock or check command,
produces no acceptance manifest, and exits `0` only as an acknowledgement. It is not validation
evidence. Continue authorized implementation, including writing tests and coherent checkpoints,
without automatic validation at internal boundaries or at the end of local implementation.

Level 2 (`2`/`quick`) is path-scoped hygiene: Git diff/whitespace, changed shell/JSON/Python
syntax, optional `actionlint`, and `cargo fmt` for routed workspace or standalone tools. Cargo fmt
may inspect manifests. It does not run `cargo check`, `cargo test`, `cargo build`, `cargo run`,
locked dependency metadata, a protobuf probe, or self-test/architecture/contract suites.

Level 3 (`3`/`final`) preserves the current final acceptance. It additionally enforces the curated
hotspot LOC ceilings when workspace Rust changed, plus the cheap repository-wide physical
source/test/tooling ratchet for every nonempty diff. Its normal mode permits only reviewed legacy
non-growth; macro closeout additionally requires `check_architecture.py physical-files --terminal`.
Required architecture, production-integration, capture and live evidence remains scope-dependent.
Neither level 2 nor level 3 runs the exhaustive persistence inventory, capture QA, a live database,
or a review unless those are explicitly added by the issue's acceptance. `audit` and `self-test`
remain expert explicit commands, not extra daily levels. Directory-first routing can format or
compile documentation under crate/tool directories; see [the runner contract](validation-v2.md).

Before authorized publication or issue completion, run the full required acceptance and report
implementation/unvalidated work separately from accepted evidence. Use
`./tools/validation-v2 verify --manifest <path> --require-profile final` when a consumer must
reject level-2 evidence as final. Push, merge, runtime authority and hosted CI are unchanged.

AGENTS.md owns execution cadence and exclusive validation scheduling. Implement the
complete delivery before running its acceptance; delegation adds no per-worker CI
or independent review gate. Run focused tests for affected behavior only when the
explicit acceptance campaign begins:

```bash
CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=1 \
  PROTOC=/home/ubuntu/.local/protoc/bin/protoc \
  cargo test --locked -p wow-world exact_test_name --lib
```

Validation commands must expose their real exit status. Do not append `| head`, `| grep`,
`; echo EXIT=$?`, or another pipeline that can turn a failed checker into a reported success. If
output must be retained, use `--logs` (private command logs beside the manifest) or
redirect it to a log and check the validator's own exit code. Keep the original
package/feature selection when investigating a failure; see the canonical runner
guide for `--no-fail-fast` batches and explicit, still-red `--keep-going` diagnostics.

Choose this focused run when its evidence is missing or a failure needs investigation; do not
rerun it merely because the same cases ran in `final`'s complete library suite. The runner
contract owns cache selection, disk cleanup and background-task tracking. Keep direct Cargo
on that same per-worktree target and preserve the active cache between macros.

Use focused checks within the approved issue or macro; an internal cut does not require a new
PR, exhaustive run or repeated approval. Before publication, commit the completed local work,
validate the publication candidate and retain the issue-specific acceptance evidence. Follow
the publication-evidence rules in `validation-v2.md` for unrelated untracked documents and
reuse across a validated documentation-only delta; do not duplicate those rules here.
Push, merge, deployment and destructive fixture authority remain distinct; reuse each existing
approval within its stated scope, and do not infer it from a passing validator or a test host.

## Review and runtime QA are separate tools

Validation does not review code and does not start servers.

- `./tools/local-review.sh review origin/forever` runs the optional local Codex review of the clean
  committed diff (`review-uncommitted` for work in progress). It is advice for the author; the
  external pull requests trigger the `Codex Review Gate` workflow.
- `tools/wow-test-bot/run_rustycore_login_smoke.sh` is the live login smoke. It needs the local
  runtime and MariaDB, and is never part of CI.
- `tools/qa-runtime.sh` orchestrates QA that needs a *different* build than the one deployed. It
  snapshots the live build, installs a candidate through `systemctl`, runs the scenario, and
  attempts restoration on normal and trapped exit paths; `self-test` exercises that restore against fake
  services and `snapshot` prints the live identity without touching anything. Destructive
  scenarios stay behind two explicit flags.

These guards are not crash-proof rollback. `SIGKILL`, host failure or failed restoration requires
operator recovery from retained evidence. `loot-race` installs and removes the chest fixture through
the capture wrapper's shared guard, with the world stopped for both writes (#373). "Stopped" is
verified, not assumed: the unit must be inactive with no main PID, the PID it served from gone and
no listener on the world/instance ports. If that cannot be proven during the restore, the chest
cleanup and the build copy are skipped, the fixture stays armed and the run exits 70. Every
swapping command (`login`, `loot-race`, `loot-item`) writes its report from the restore trap, so
the outcome is `passed-restored`, `failed-restored` or `restore-failed`, never a pass that hides a
failed restore.
It does not inherit that capture wrapper's journal-gated restart guarantee. Do not use
the systemd smoke as proof of complete fixture recovery or as a fresh capture replacement.

For a guarded login/world-entry check of a clean committed candidate:

```bash
./tools/qa-runtime.sh --allow-runtime-qa \
  --world-exec /absolute/path/to/candidate/target/debug/world-server \
  --report /tmp/rustycore-login-runtime-report.json login
```

Live QA candidates are **debug** builds (`cargo build --locked -p world-server`), not release.
A release build of `world-server` costs 15–25 minutes per iteration on this host; a debug build is
enough to prove functional behaviour and keeps fix → rebuild → rerun loops short. Use a release
build only for the actual deployment (`redeploy-rustycore.sh`) or when a scenario measurably needs
release speed (for example the bot times out against a debug server); say so in the evidence.
Build the candidate from the clean committed head you are validating, with the shared-target
precautions below.

`login` uses the maintained login wrapper with a hash-pinned bot binary. It disables account
provisioning, password generation and inherited fixture modes, verifies the bot's authentication,
character enumeration and world-entry JSON, and reports `passed-restored` only after the original
build is restored and serving. The private temporary evidence directory is printed; it retains
the original executable and bot logs/report for diagnosis. This is not a movement, combat or
packet-capture acceptance test. It still requires explicit permission to interrupt the service
and valid local bot credentials; it never runs as part of ordinary validation.

For a change confined to `wow-math`, two successful runs of `cargo fmt --all --check` took 19.768 and 19.684 seconds on the aarch64 development host on 2026-11-21. The corresponding `cargo fmt -p wow-math --check` runs took 0.099 and 0.101 seconds. This measures formatting only. Workspace-level inputs still require workspace-wide formatting.

## What runs remotely

External PRs into `forever` trigger Rust CI final acceptance and Codex review. Pull requests authored by the exact login `alseif0x` use the local acceptance result. The default-branch weekly audit runs on `forever`. A manual dispatch can run an audit or validate a committed change against its explicit ancestor.
GitHub reported `forever` as unprotected on 2026-11-21. Workflow results and branch-protection requirements
are separate. The `3.4.3` protection settings apply only to that branch.

## Trust boundary

Do not broaden the trusted condition to `COLLABORATOR`, `MEMBER`, or an author-association class.
The exact login allowlist is deliberate. External code continues to run under the existing
read-only pull-request workflows. Switching from one local AI agent to another
does not change either side of this boundary.

If another first-party identity is added later, update every trusted-author condition and this
document in one reviewed change. The gate must remain the same command for humans and agents so
the fast path does not fork into undocumented personal procedures.
