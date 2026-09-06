# AGENTS.md

Shared operating guide for agents working in RustyCore. CLAUDE.md imports this file.
Use current code to establish implementation state and the legacy C++/capture evidence
to establish required base-server behavior. Neither old documentation nor existing Rust
is correctness proof. This guide does not override explicit user scope or approval gates.

## Project and sources of truth

- Repository: /home/server/rustycore; remote: https://github.com/alseif0x/rustycore.git.
- Behavioral reference: /home/server/woltk-trinity-legacy.
- Target: full functional parity with the TrinityCore-derived WoW 3.4.3 server, not a
  smaller compatible subset. A bounded milestone never silently reduces the full port.
- Integration/default branch: 3.4.3. One implementation macro-issue, one feature branch,
  one PR into 3.4.3. Main is only an optional stable release pointer; releases are tags.
- Toolchain is declared in rust-toolchain.toml / Cargo.toml (currently Rust 1.98, edition 2024).
- Development host is aarch64; hosted runners are x86_64. Label machine-dependent evidence.
- Local protoc: /home/ubuntu/.local/protoc/bin/protoc. Set PROTOC for protobuf-dependent builds.

Start each session with:

~~~bash
cd /home/server/rustycore
git status --short --branch
git log --oneline --decorate -8
sed -n '1,80p' docs/migration/STATE.md
~~~

Read the active issue/checkpoint and relevant changes before relying on them. Review
code-bearing changes against exact C++ paths for affected behavior; documentation-only
changes do not require a new whole-port audit or create a new parity base.

Documentation entry point: [docs/README.md](docs/README.md). Current authorities:

- [STATE.md](docs/migration/STATE.md): dated state and evidence boundaries, not an undated guarantee.
- [PORT_PLAN.md](docs/migration/PORT_PLAN.md) and GitHub #49: execution order, including the
  architecture track; issue numbers themselves are not execution order.
- [EXISTING-CODE-DEFECTS.md](docs/migration/EXISTING-CODE-DEFECTS.md): reported defects,
  each requiring current reproduction/contrast before being treated as still open.
- [Session checkpoint](docs/architecture/session-578-checkpoint.md): #578 acceptance and remaining work.
- [Modularity/ECS plan](docs/architecture/modularity-and-ecs-plan.md): current design and proof gates.
- [Ownership boundaries](docs/architecture/ownership-and-boundaries.md) and
  [module design](docs/architecture/module-design-guidelines.md): semantic/physical policy.
- Runtime ADRs and [clock trace](docs/architecture/runtime-clock-phase-trace.md): relevant
  decisions and dated evidence; verify current spawn/call paths before changing runtime owners.

Historical migration tables, percentage headlines, old checklists and the frozen
current-session-handoff.md are not active instructions or current completion proof.
Read them for a specific historical question, not as a mandatory session preflight.
Update the owning current document instead of creating another competing plan or status log.

## Scope, autonomy and completion

- Review-only requests mean inspect/report without mutations. An explicit request to review
  and fix documentation authorizes those document changes, not gameplay implementation.
- Continue authorized work through its acceptance criteria; a helper, commit or passing test
  is not an automatic stopping point. Small validated slices belong inside the approved
  macrodeliverable, not automatic micro-issues, micro-PRs or requests to "continue".
- Resolve routine uncertainty through inspection and tests. Ask only for material missing
  information that evidence cannot settle, new authority, or a material scope/design choice.
- Reuse explicit approval for its task, targets and conditions. Do not request it again unless
  those materially change or approval is withdrawn. Preserve explicit new-design review gates.
- "Test environment" is not blanket permission for destructive actions, publication or deployment.
- A stop condition pauses the affected mutation, not safe investigation. Resume when evidence
  resolves it within scope; do not force changes through unresolved behavior or overlapping work.
- Distinguish local completion from publication. No push without explicit authorization;
  push/PR creation does not authorize merge, deployment, restart or destructive database work.
- Do not claim an issue complete from a partial slice, or manual-test-ready without actually
  installing/restarting the target build and exercising the required client/runtime scenario.
- Follow real dependencies. Safe inspection and explicitly allowed isolated experiments may
  proceed before a production prerequisite, but do not enable production paths ahead of their gate.

## Fidelity and implementation

For protocol, gameplay, database, lifetime, persistence and runtime behavior:

1. Identify the complete in-scope operation and current callers/owners.
2. Locate exact C++ classes/functions before changing or approving the behavior. Use a real
   client/server capture when C++ is incomplete or ambiguous; do not silently invent parity.
3. Freeze relevant packet metadata/bytes/connection/order, admission/phase, state authority,
   transactions, rollback/unknown-COMMIT, cancellation, recovery and publication semantics.
4. Implement the smallest coherent faithful change with positive/negative and relevant
   integration/failure tests. Separate structural movement from intentional behavior changes.
5. Update the owning checkpoint/acceptance with exact source anchors, code targets, command,
   SHA, result and remaining boundary. Existing inventory gaps may be closed only with real
   implementation evidence; no new #NEXT row or percentage calculation for every helper.
6. Validate proportionally and commit coherent validated changes on the issue branch.
   Continue remaining authorized work; publication retains its own gate.

The behavioral audit is against C++ exclusively, supplemented by appropriate real captures where
that source is incomplete. If an affected comment/test relies on an unsupported earlier analysis,
locate the C++ equivalent and correct the evidence before approving the behavior. Pause an
unresolved mutation while continuing safe inspection; resume from evidence within scope, or ask
for a material choice/new authority that evidence cannot settle. An intentional departure requires
an explicit contract. A legacy bug repair must not be hidden inside a behavior-preserving refactor.

Do not bulk-close inventory rows or report planning/test-debt work as gameplay progress.
Use implemented, production-integrated and parity-proven as distinct evidence levels.

## Architecture and skills

Use the existing architecture skill for boundary/design questions and the safe-refactor skill
for approved behavior-preserving restructuring. They apply the maintained project documents;
they are not separate frozen architecture snapshots.

- Require both correct semantic ownership and manageable production/test/fixture files.
  The module-design guide owns the numeric budgets and bounded exception policy.
- Prefer private modules/submodules before crates. No crate/trait per helper, universal
  context, second mutable mirror, extra lock or public field merely to relocate code.
- Keep one canonical authority and execution owner per transition; trace readers, writers,
  lifetime, persistence and publication together. A detached Player is not automatically missing.
- Preserve C++ phase order and established persistence fences. No synchronous map/entity guard
  across await, I/O or packet delivery under a map lock. Intentional async operation gates need
  explicit lock order, cancellation/recovery and blocking-scope contracts.
- Read the current runtime composition and relevant trace; do not infer clock count or
  scheduling ownership from a stale summary, registration enum or an ECS dependency.
- PacketHandlerEntry is the single opcode registration and call source. Inspect
  crates/wow-world/src/session/registry.rs and actual registrations for the current thunk
  signature; do not copy an outdated snippet or reintroduce a dispatcher opcode match.
  Keep exact-set metadata/registration tests for changes to that boundary.
- The user-approved delivery track is bounded #578/#579 closeout, then #584's crate-focused
  core macrodeliverables, #583 and #153 before #133 closes. #584 retains unfinished C0–C4;
  do not require the entire core refactor in #578 or treat its closure as completing it.
  No next crate is selected. Analyze each crate before defining its implementation macro,
  include cross-crate consumers and preserve scoped regression/live acceptance. These
  evidence reviews do not add routine approvals or authorize merge/runtime operations.
- After playable M6.2/#47, perform the fresh whole-port planning pass before decomposing
  Part 2/#48. Do not prematurely create its child issue tree.

## Validation

Use [validation-v2](docs/operations/validation-v2.md) and
[local-first development](docs/operations/local-first-development.md) for the actual profiles.

~~~bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo check -p world-server
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo test -p wow-world <focused-test> --lib
cargo fmt --all -- --check
git diff --check
./tools/validation-v2 quick --base origin/3.4.3
~~~

Choose the real library/binary/integration target; do not assume every crate has a library.
Run affected production-linked integration targets explicitly when required; library tests
alone do not establish production composition. Record evidence at the actual tested SHA.

Ordinary ownership/module iteration:

~~~bash
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check --syntax-only
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py self-test
~~~

The Session check without --syntax-only recomputes the exhaustive persistence inventory.
Use it for an explicitly requested relevant audit, actual inventory changes or required
macro/terminal acceptance, not routine helpers. Metadata-only plan changes still require
the applicable preserved persistence-reference and snapshot-policy consistency checks.
Never blindly regenerate a baseline to hide drift; review the semantic delta and tighten
ceilings after validated retirement. Keep changed TSV schemas valid; the R8 ledger has nine
tab-separated columns. Do not pipe checks through a command that masks their exit status.

Before an authorized push, validate the committed publication candidate:

~~~bash
./tools/validation-v2 final --base origin/3.4.3
~~~

Use the publication-evidence rules in `docs/operations/validation-v2.md`: unrelated
untracked documents do not require a new worktree or recompilation when their lack
of build/test influence is verified. Preserve them and record the manifest's actual
dirty status. Reuse green evidence for unchanged code with a validated documentation-only
delta; never relabel an earlier run as having tested a later SHA.

For exactly alseif0x-authored PRs, local final plus focused evidence is the required gate;
configured hosted validation/reviewer jobs intentionally skip. External authors retain
configured remote checks/review. Never broaden trust to an author-association role.
The audit profile is exhaustive, not the routine pre-push profile; integration-branch
pushes run it remotely. Preserve explicit issue acceptance even when a profile is green.

Capture-diff applies to changed bytes, metadata, connection or observable order. Fresh
action-specific captures are distinct from regression goldens. Live lifecycle/runtime changes
need authorized runtime QA; real durability claims need real DB/restart/relogin evidence,
not only mocked futures. Missing runtime authority does not block remaining safe local work.

## Runtime and sensitive data

The integrated QA bot is tools/wow-test-bot, outside the root workspace. Read its README and
RUSTYCORE_SMOKE.md plus the relevant operation guide before use. Live modes can write auth,
session or character data; scope the target/accounts and obtain any required runtime authority.
Preserve useful QA improvements in the integrated tool, not only in /tmp; record structured
scenario results. Bot code existing does not establish live acceptance.

Use the current configuration loader and examples: worldserver.conf / bnetserver.conf are
preferred, with legacy mixed-case compatibility filenames. The processes are world-server
and bnet-server; the MariaDB schemas are auth, characters, world and hotfixes. Read the
operation guide/current config safely before acting; documentation is not runtime authorization.

Never print, stage or commit credentials, local configs, certificates/keys, secret-bearing logs,
DB URLs with secrets, built binaries or private QA configuration. Use environment/local ignored
configuration for secrets. A file being ignored does not make it safe to print.

## Git and local work

Reuse the active issue branch. At first implementation branch creation use:

~~~bash
gh issue develop <N> --repo alseif0x/rustycore --base 3.4.3 --checkout
~~~

PRs target 3.4.3 with Closes #<N> in the body. After an authorized push, create the PR if it
does not already exist; update the existing PR rather than duplicating it. Resolve or explicitly
defer actionable review findings and resolve conversations before an authorized merge.
Do not infer merge authority from a commit, push or PR request.

Use rg for search and apply_patch for manual edits. Preserve unrelated dirty work and other
agents' changes; inspect overlaps before relying on them. Stage exact validated paths only.
Do not revert, delete or overwrite unrelated work to obtain a clean tree.

AGENTS.md, CLAUDE.md and the selected .agents/skills resources are tracked shared instructions.
Other local agent/workflow paths may be ignored; consult git ls-files and .gitignore rather than
assuming. Do not force-add private local context unless explicitly requested. Keep one source
for each rule; retire or redirect superseded guidance without discarding useful evidence.
