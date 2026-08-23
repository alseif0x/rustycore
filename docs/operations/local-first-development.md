# Local-first development

RustyCore uses a trusted-author split so first-party development is not serialized by hosted CI.
The policy is intentionally narrow:

- a pull request authored by exactly `alseif0x` allocates no GitHub-hosted validation runner;
- every other author, including bots and collaborators, keeps the existing remote checks;
- scheduled and manually dispatched workflows remain available for broad audits;
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
when workspace Rust changed. Neither runs the exhaustive persistence inventory, capture QA, a live
database, or a review; those belong to `audit` and to the explicit procedures below.

Run focused tests explicitly when behavior changes, for example:

```bash
cargo test --locked -p wow-world exact_test_name --lib
```

Validation commands must expose their real exit status. Do not append `| head`, `| grep`,
`; echo EXIT=$?`, or another pipeline that can turn a failed checker into a reported success. If
output must be retained, redirect it to a log and check the validator's own exit code.

## Review and runtime QA are separate tools

Validation does not review code and does not start servers.

- `./tools/local-review.sh review origin/3.4.3` runs the optional local Codex review of the clean
  committed diff (`review-uncommitted` for work in progress). It is advice for the author; the
  required remote gate for external authors is the `Codex Review Gate` workflow.
- `tools/wow-test-bot/run_rustycore_login_smoke.sh` is the live login smoke. It needs the local
  runtime and MariaDB, and is never part of CI.

## What runs remotely

- a pull request whose author is not `alseif0x`;
- **a push to `3.4.3`, which is every merge**;
- the scheduled audit;
- an explicit `workflow_dispatch` run.

The merge-cadence case exists because the first three bullets alone left first-party work with no
remote enforcement at all. A skipped required check satisfies branch protection, so a first-party
pull request merged with every check reporting *skipped* — which reads as green. Regressions
reached `3.4.3` that way: #275 left both ownership ratchets red on HEAD, #277 hid roughly 985
production persistence accesses from the inventory, and #329 recorded two more from audit-only
ratchets that the local gate cannot see.

This does not weaken the trust boundary. Review-time validation stays local and stays the author's
job; what runs at merge is the enforcement nobody was performing. A failure there names a commit
already on `3.4.3`, so it should produce a focused issue and a fix on top, not a retroactive review
cycle over intermediate commits — the same rule the scheduled audit follows.

## Trust boundary

Do not broaden the trusted condition to `COLLABORATOR`, `MEMBER`, or an author-association class.
The exact login allowlist is deliberate. External code continues to run under the existing
read-only pull-request workflows and required checks. Switching from one local AI agent to another
does not change either side of this boundary.

If another first-party identity is added later, update every trusted-author condition and this
document in one reviewed change. The gate must remain the same command for humans and agents so
the fast path does not fork into undocumented personal procedures.
