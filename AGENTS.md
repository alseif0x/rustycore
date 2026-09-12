# AGENTS.md

Shared operating guide for agents working in RustyCore. CLAUDE.md imports this file.
Use current code to establish implementation state and versioned C++/capture evidence
to establish required base-server behavior. Neither old documentation nor existing Rust
is correctness proof. This guide does not override explicit user scope or approval gates.

## Project and sources of truth

- Repository: /home/server/rustycore; remote: https://github.com/alseif0x/rustycore.git.
- Target-version reference: /home/server/woltk-trinity-legacy (3.4.3).
- Complementary gameplay reference: /home/server/azerothcore-wotlk-reference (3.3.5a).
  The pinned source checkout and its coverage are recorded in docs/README.md.
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

Audit behavior against the relevant versioned C++ paths and appropriate real captures.
The user approved AzerothCore as a complementary source for missing or suspect gameplay in
the Classic fork; neither core is complete or infallible, and shared ancestry is not independent
proof. Compare the complete operation, scripts and effective data before adapting its logic.
Keep 3.4.3 packet layouts, IDs/data schemas, admission and lifecycle contracts explicit: 3.3.5
wire formats or SQL are not drop-in replacements. Resolve version differences using target-build
evidence and an explicit behavior contract; record source SHA/functions and retained uncertainty.
Use secondary references selectively for the active responsibility, not as a new whole-port audit.
If an affected comment/test relies on an unsupported earlier analysis,
locate the C++ equivalent and correct the evidence before approving the behavior. Pause an
unresolved mutation while continuing safe inspection; resume from evidence within scope, or ask
for a material choice/new authority that evidence cannot settle. An intentional departure requires
an explicit contract. A legacy bug repair must not be hidden inside a behavior-preserving refactor.

Do not bulk-close inventory rows or report planning/test-debt work as gameplay progress.
Use implemented, production-integrated and parity-proven as distinct evidence levels.

## Architecture and skills

Use [orchestrate-rustycore](.agents/skills/orchestrate-rustycore/SKILL.md) for development
coordination when useful bounded independent work can be delegated. This explicitly
requests selective subagent work, with Luna as the usual bounded implementation
collaborator, not delegation for every task. Project Codex defaults
live in `.codex/config.toml` and `.codex/agents/`; they do not override runtime permissions.

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
- #133 was closed on 2026-09-09. #578/#585/#587/#588/#589/#716/#718/#722/#737 are
  integrated and closed in their bounded scopes. The current architecture plan and
  STATE.md own the selected delivery and its evidence; do not infer current status
  from older dated checkpoints or wait for #133 to close again.
  The technical gate remains required core #584 → native/Wasm product #583 → independent
  audit #153. #584 retains unfinished C0–C4 core work; #583 owns the preserved M0–M4
  native/Wasm product and does not block an unrelated gameplay macro, while its
  production integration waits for the required core work. The Rust/Wasm/C mixed
  product remains mandatory even though its operator activation is optional.
  Select the next prepared responsibility from the current STATE.md and refactor
  completion plan; this operating guide does not keep a second dated next-issue list.
  Analyze each responsibility before defining its macro,
  include cross-crate consumers and preserve scoped regression/live acceptance. These
  evidence reviews do not add routine approvals or authorize merge/runtime operations.
- After playable M6.2/#47, perform the fresh whole-port planning pass before decomposing
  Part 2/#48. Do not prematurely create its child issue tree.

## Validation

Implement the complete authorized delivery first, including its tests and consumers;
then run affected acceptance tests, applicable QA and publication validation. During
implementation use inspection, not CI, builds or test runs per internal change or
worker handoff. At final acceptance, fix findings and rerun affected evidence as needed.
An explicit user request for an earlier diagnostic run remains authoritative.
Do not claim unexecuted evidence as passing.

The user's ordinary local acceptance budget is ten minutes for the complete campaign,
including the required additional checks, on this host with the active warm cache.
Measure coding/error-repair time separately. Use `final --timings` to locate compilation
cost within that same run; record the full campaign start/end and any checks outside it.
Exceeding 600 seconds means the performance target is not met. Do not hide the overrun,
split it into nominally separate ten-minute checks, repeat a failed campaign unchanged,
or remove acceptance to claim success. Cold bootstrap and exhaustive/live acceptance
remain explicit separate costs and must not be passed off as the ordinary warm run.

Plan acceptance once for the completed delivery. The commands below are scope-dependent
examples, not a checklist to run before `quick` and again before `final`. `final` already
checks affected downstream test targets and runs the changed libraries' complete suites;
credit the focused cases actually executed by those suites. Add the required integration,
ownership, capture or live evidence they do not cover. Do not run a separate workspace
check or identical library suite just to warm up `final`. After a failure, fix the related
findings together and rerun the affected acceptance; new code still needs final evidence
at its committed candidate. Do not use repeated Cargo calls as a search/automatic-edit
loop for consumers that can be found by inspection. A zero-test filter proves nothing.

The parent owns validation scheduling, or assigns one exclusive validation executor.
Run heavyweight builds, tests, exhaustive scans and live QA sequentially, including
across worktrees; no worker starts its own parallel campaign. Check for active work
and available RAM/disk before launching. On this shared host start Cargo with one job
(`VALIDATION_V2_CARGO_JOBS=1` for the runner); increase only with demonstrated headroom.
Do not kill unrelated processes to obtain resources. Agent-count limits are not resource locks.

Keep direct Cargo and runner commands on the same per-worktree target directory; the
runner defaults to `<checkout>/target` and respects explicit `CARGO_TARGET_DIR`. For
standalone manifests set it to the checkout's absolute `target` too. Do not share one
target across active worktrees. Follow the disk and long-running-command procedures in
`docs/operations/validation-v2.md`: preserve the active incremental cache, clean inspected
inactive build artifacts first, retain real exit codes, and track an existing background
task instead of launching a duplicate after its foreground wait expires.

Use [validation-v2](docs/operations/validation-v2.md) and
[local-first development](docs/operations/local-first-development.md) for the actual profiles.

~~~bash
export PROTOC=/home/ubuntu/.local/protoc/bin/protoc
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$PWD/target"
cargo check -p world-server
cargo test -p wow-world <focused-test> --lib
cargo fmt --all -- --check
git diff --check
./tools/validation-v2 quick --base origin/3.4.3
~~~

Choose the real library/binary/integration target; do not assume every crate has a library.
Run affected production-linked integration targets explicitly when required; library tests
alone do not establish production composition. Record evidence at the actual tested SHA.

Ownership/module acceptance commands (select by affected scope, not per helper):

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
