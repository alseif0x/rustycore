# Local-first development

RustyCore uses a trusted-author split so first-party development is not serialized by hosted CI.
The policy is intentionally narrow:

- the required validation and reviewer jobs for a pull request authored by exactly `alseif0x`
  skip before allocating a runner;
- every other author, including bots and collaborators, keeps the existing remote checks;
- pushes to `3.4.3`, schedules and manual dispatches retain their configured hosted validation;
- first-party validation is local and proportional to the changed paths.

The runner is agent-agnostic. Kimi, Codex, Grok, Claude, any other AI agent, and a human
maintainer all invoke the same non-interactive command and receive the same process exit status.
Trust is derived only from the GitHub PR author's exact login; it is never derived from the tool
that wrote the code.

GitHub still creates the required check names for a trusted pull request, but their jobs are
evaluated as `skipped` before a runner is assigned. This preserves the existing protection for
external contributions without spending hosted compute or waiting for a remote review on normal
maintainer work.

## One gate, three budgets

There is exactly one validation entry point. `docs/operations/validation-v2.md` is its contract;
this document is only the trust policy around it.

```bash
./tools/validation-v2 quick --base origin/3.4.3   # while iterating
./tools/validation-v2 final --base origin/3.4.3   # before publishing the final commit
./tools/validation-v2 audit --base origin/3.4.3   # explicit exhaustive budget
```

`quick` and `final` are path-scoped: they plan from the committed, staged, unstaged and untracked
diff and run only what it touches. `final` additionally enforces the curated hotspot LOC ceilings
when workspace Rust changed, plus the cheap repository-wide physical source/test/tooling
ratchet for every nonempty diff. Its normal mode permits only reviewed legacy non-growth;
macro closeout additionally requires `check_architecture.py physical-files --terminal`.
Neither profile runs the exhaustive persistence inventory, capture QA, a live
database, or a review. `audit` covers committed capture contracts; live databases, fresh captures,
runtime QA and code review remain separate procedures. Directory-first routing can compile
documentation under crate/tool directories; see [the runner contract](validation-v2.md).

Run focused tests explicitly when behavior changes, for example:

```bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test --locked -p wow-world exact_test_name --lib
```

Validation commands must expose their real exit status. Do not append `| head`, `| grep`,
`; echo EXIT=$?`, or another pipeline that can turn a failed checker into a reported success. If
output must be retained, redirect it to a log and check the validator's own exit code.

Use focused checks within the approved issue or macro; an internal cut does not require a new
PR, exhaustive run or repeated approval. Before publication, commit the completed local work,
validate the publication candidate and retain the issue-specific acceptance evidence. Follow
the publication-evidence rules in `validation-v2.md` for unrelated untracked documents and
reuse across a validated documentation-only delta; do not duplicate those rules here.
Push, merge, deployment and destructive fixture authority remain distinct; reuse each existing
approval within its stated scope, and do not infer it from a passing validator or a test host.

## Review and runtime QA are separate tools

Validation does not review code and does not start servers.

- `./tools/local-review.sh review origin/3.4.3` runs the optional local Codex review of the clean
  committed diff (`review-uncommitted` for work in progress). It is advice for the author; the
  required remote gate for external authors is the `Codex Review Gate` workflow.
- `tools/wow-test-bot/run_rustycore_login_smoke.sh` is the live login smoke. It needs the local
  runtime and MariaDB, and is never part of CI.
- `tools/qa-runtime.sh` orchestrates QA that needs a *different* build than the one deployed. It
  snapshots the live build, installs a candidate through `systemctl`, runs the scenario, and
  attempts restoration on normal and trapped exit paths; `self-test` exercises that restore against fake
  services and `snapshot` prints the live identity without touching anything. Destructive
  scenarios stay behind two explicit flags.

These guards are not crash-proof rollback. `SIGKILL`, host failure or failed restoration requires
operator recovery from retained evidence. `loot-race` currently requires the chest fixture to
exist already; the systemd wrapper does not install/remove the PM2 capture wrapper's chest.
It also does not inherit that capture wrapper's journal-gated restart guarantee. Do not use
the systemd smoke as proof of complete fixture recovery or as a fresh capture replacement.

For a guarded login/world-entry check of a clean committed candidate:

```bash
./tools/qa-runtime.sh --allow-runtime-qa \
  --world-exec /home/server/rustycore/target/release/world-server \
  --report /tmp/rustycore-login-runtime-report.json login
```

`login` uses the maintained login wrapper with a hash-pinned bot binary. It disables account
provisioning, password generation and inherited fixture modes, verifies the bot's authentication,
character enumeration and world-entry JSON, and reports `passed-restored` only after the original
build is restored and serving. The private temporary evidence directory is printed; it retains
the original executable and bot logs/report for diagnosis. This is not a movement, combat or
packet-capture acceptance test. It still requires explicit permission to interrupt the service
and valid local bot credentials; it never runs as part of ordinary validation.

## What runs remotely

- a pull request whose author is not `alseif0x`;
- **a push to `3.4.3`, which is every merge**;
- the scheduled audit;
- an explicit `workflow_dispatch` run.

The merge-cadence case exists because skipping trusted PR jobs without push validation left
first-party work with no routine hosted enforcement before the next scheduled audit.
A skipped required check satisfies branch protection, so a first-party
pull request merged with every check reporting *skipped* — which reads as green. Regressions
reached `3.4.3` that way: #275 left both ownership ratchets red on HEAD, #277 hid roughly 985
production persistence accesses from the inventory, and #329 recorded two more from audit-only
ratchets that the local gate cannot see.

This does not weaken the trust boundary. Review-time validation stays local and stays the author's
job; what runs at merge is the enforcement nobody was performing. A failure there names a commit
already on `3.4.3`, so it should produce a focused issue and a fix on top, not a retroactive review
cycle over intermediate commits — the same rule the scheduled audit follows.

## Branch protection

`3.4.3` requires strict status checks, linear history, conversation resolution, and two contexts:
`Validation V2` and `Codex reviewer verdict`. Both skip for the trusted author and run for
everyone else, and a skipped required check satisfies protection — which is exactly why the
merge-cadence audit above exists.

The validation job carries a static name on purpose: GitHub publishes a skipped job's check under
the raw, unexpanded name expression, so a templated name can never be satisfied as a required
context.

## Trust boundary

Do not broaden the trusted condition to `COLLABORATOR`, `MEMBER`, or an author-association class.
The exact login allowlist is deliberate. External code continues to run under the existing
read-only pull-request workflows and required checks. Switching from one local AI agent to another
does not change either side of this boundary.

If another first-party identity is added later, update every trusted-author condition and this
document in one reviewed change. The gate must remain the same command for humans and agents so
the fast path does not fork into undocumented personal procedures.
