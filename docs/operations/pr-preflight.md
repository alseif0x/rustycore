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
set (resolving bare command names through `PATH`), then checks the project's usual local install
and `PATH`. GitHub downloads the version from the same file. Review commands require an
authenticated `codex` CLI.
Printing `review` or `full` with `--dry-run` does not require Codex or its execution-time helpers.

## Profiles

| Command | Purpose | Starts services or mutates a database? |
|---|---|---:|
| `self-test` | Test harness parsing and pinned-version invariants | No |
| `architecture` | Enforce architecture ratchets and report source hotspots | No |
| `diff [BASE]` | Whitespace-check committed, staged, and unstaged changes | No |
| `format` | Run harness self-tests and all three formatting checks from CI | No |
| `check` | Run locked core checks, bot check, and server builds from CI | No |
| `test` | Run focused suites, loot-race tests, and the required capture gates from CI | No |
| `ci` | Run `architecture`, `format`, `check`, and `test` | No |
| `quick [BASE]` | Run `diff`, `architecture`, `format`, and `check` during iteration | No |
| `capture` | Test committed captures and enforce required capture contracts without `protoc` | No |
| `review [BASE]` | Review a clean committed diff with Codex in read-only mode | No |
| `review-uncommitted` | Review staged, unstaged, and untracked work during iteration | No |
| `full [BASE]` | Run `diff`, CI (including architecture/capture), and `review` | No |
| `stable` | Check/build server binaries with latest stable Rust | No |
| `qa-login` | Run the integrated live login bot | **Yes** |
| `qa-loot-race` | Run the destructive two-session atomic-loot bot | **Yes** |

`BASE` defaults to `origin/3.4.3`. Use `--dry-run` before a command to print its underlying
commands without executing them or provisioning optional review tools.

The `architecture` profile evaluates locked `cargo metadata` against
`tools/architecture/dependency-policy.json`. Known current violations are explicit, issue-linked
baseline exceptions: a new edge still fails, and an exception whose edge has disappeared also
fails until the obsolete allowance is removed. Direct third-party `normal`/`build` dependencies
of foundation, domain/runtime, and application packages are pinned per package, so direct SQL,
network, configuration, process, or async-runtime infrastructure cannot enter an inward layer
silently. The intentionally narrow `wow-network`, `wow-packet`, and `wow-data` adapters are pinned
the same way; other adapters remain free to integrate their concrete infrastructure. Legitimate
external libraries require an explicit reviewed allowlist entry. Protected entries are also tied
to the policy's canonical crates.io source: same-named path, Git, or alternate-registry
dependencies fail closed. Metadata identity is validated globally; duplicate JSON keys,
package/node/member IDs, and ambiguous multi-ID direct dependencies also fail closed. The
self-test pins the exact locked/all-features Cargo metadata command. The profile additionally
prints a split of the largest Rust source files into production and exact top-level
`#[cfg(test)]` item ranges. A test range starts at the exact `cfg(test)` attribute and ends at
the attached item's closing brace or semicolon; later items and trailing lines remain production.
File size is not a universal gate, but each hotspot explicitly curated in
`runtime-ownership-ledger.json` has independent production, test, and total non-growth ceilings;
reductions pass without a baseline edit, while a rename/removal requires explicitly retiring or
replacing the audited path. Run
`python3 tools/architecture/check_architecture.py self-test` to exercise the workspace and
third-party dependency fixtures. Ownership, mirror rules, handler snapshot updates, and the
deliberate baseline-change procedure are documented in
`docs/architecture/ownership-and-boundaries.md`.

The `architecture` profile also runs the locked tests and repository check for the standalone
`tools/architecture/handler-contract-check` crate. It derives every production workspace package
that can normally link `wow-handler` from full locked Cargo metadata, walks each `lib`/`bin` module
graph without evaluating `cfg`, and enforces logical `wow_world::handlers` ownership. It also
walks every workspace production module graph to reject declarative macro definitions,
invocations, aliases, exports, or includes capable of emitting, forwarding, or mounting hidden
registrations from outside that closure. The existing 23 `wow-logging` exports, six `wow-proto`
generated includes, and seven `wow-script` non-handler inventory calls are exact removal/addition
ratchets; no neighboring source-generation capability is implied. Explicit module paths are
restricted to unconditional, in-package, non-symlink `.rs` files declared from file modules.
The logical registration and dispatcher roots come from the strict
`tools/architecture/handler-module-policy.json`, whose tracking issues must remain present and open
in the architecture ledger. The same Cargo module graph locates the single concrete
`WorldSession::dispatch_packet` independently of filename, including private external/inline
children and supported `#[path]` mounts. Production-capable conditional ownership, overlapping or
duplicate logical mounts, missing/duplicate dispatchers, and a dispatcher or registration outside
its declared owner fail closed. Inventory `collect`/`submit` aliases, nested or conditional registration grammar, macro-path or
metavariable forwarders, and registrations outside the owner fail closed; the sole handler
collector is the exact unconditional `inventory::collect!(PacketHandlerEntry)` at the
Cargo-declared `wow-handler` production `lib` root. Arbitrary expansion inside third-party crates
or external procedural macros is not claimed and must be made inspectable before use. The tool
ratchets registration-to-dispatch-arm coverage against the checked-in snapshot.

The focused test profile separately runs both the four-test `wow-handler` integration suite and
the production-linked `wow-world` snapshot. Integration targets can exercise their own registry
submissions without adding `cfg(test)` registrations to production library sources. The formatting
profile checks the standalone crate explicitly because its own `[workspace]` keeps it outside the
root workspace; its local `.gitignore` prevents that crate's `target/` from dirtying architecture
or full preflight.

`qa-login` is intentionally outside `full`. It requires running services, may create missing
local QA auth rows, and writes normal login/session data, so it refuses to run without explicit
acknowledgement. Existing BNet/game identities and character ownership are validation-only: a
credential, numeric-ID, ownership, ban, online-state, or realm-count mismatch fails closed rather
than being rewritten.

```bash
./tools/pr-preflight.sh --allow-runtime-qa qa-login
```

`qa-loot-race` additionally refuses to trust whichever process happens to be
listening or whichever bot an ignored `.env.local` happens to select. Pin both
the feature-branch `world-server` and `wow-test-bot` files plus their SHA-256
digests. The
gate snapshots and accredits the current normal PM2 executable, drives the
repository's guarded `capture-rust.sh loot-two-session-atomic-race --yes`
through a FIFO, then accredits the replacement capture PID/path/hash and both
distinct listeners against that same PID before running the bot. The capture
identity must remain unchanged;
after the bot, the gate signals completion and waits for fixture cleanup plus
exact restoration of the original PM2 executable/profile.
The bot writes its report to a fresh private path; exit status zero is accepted
only when that report proves the exact two accounts, two successful logins,
party/target/loot observations, exactly one item winner, both removal fanouts,
money notifications of `10` and `0`, one persisted item, an exact persisted
money delta of `10`, and relog verification. The preflight passes that same
fresh path plus the pinned bot executable/hash to the guarded capture wrapper.
After stopping the capture world and restoring the fixture and normal PM2
profile, the wrapper independently revalidates the complete report contract;
missing, symlinked, malformed, failed, wrong-identity, split-runtime-target, or
wrong-bot evidence prevents atomic dump/manifest publication even though
cleanup and normal-runtime restoration still complete. A successful race
artifact retains a mode-0600 copy as `rust/race.bot-report.json`; the manifest
records its SHA-256 and final absolute path together with non-null race fixture
and bot-evidence contracts.

The preflight creates a private `WOW_BOT_FIXTURE_JOURNAL` path and forces
`WOW_BOT_ENSURE_TEST_ACCOUNTS=0`; loot QA therefore requires pre-provisioned
disposable accounts and never runs the generic account/character bootstrap.
The bot must durably journal before its first mutation, remove the pending
journal only after bounded restoration, and atomically create the mode-0600
JSON marker `${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete`. The marker pins the
journal SHA-256 and cleanup PID. A pending journal, unsafe/malformed marker, or
digest mismatch prevents the normal PM2 world from restarting. Cleanup/restoration
failure takes precedence over the original bot/signal exit status, and the
recovery directory is retained for inspection. The preflight tracks the bot PID,
terminates and waits for it before capture cleanup on signals, and the wrapper
executes the bot with Linux parent-death protection so no detached child can
continue mutating the fixture.

The bot is forced to loopback ports `8085`/`8086` (override the accredited
ports with `RUST_WORLD_PORT`/`RUST_INSTANCE_PORT`) and rejects a different
instance port advertised by `SMSG_CONNECT_TO`. Run this only on an isolated
host/network namespace whose firewall restricts the BNet/world/instance ports
to loopback, because the server process itself may use wildcard listeners:

```bash
test -z "$(git status --porcelain=v1 --untracked-files=normal)"
TARGET_EXEC="$(realpath /absolute/path/to/feature-branch/world-server)"
TARGET_SHA="$(sha256sum "$TARGET_EXEC" | awk '{print $1}')"
BOT_EXEC="$(realpath tools/wow-test-bot/target/debug/wow-test-bot)"
BOT_SHA="$(sha256sum "$BOT_EXEC" | awk '{print $1}')"
RUST_CAPTURE_DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf \
RUST_CAPTURE_EFFECTIVE_CONFIG=/home/server/trinity-legacy-install/etc/worldserver.conf \
WOW_BOT_DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf \
WOW_BOT_WORLD_EXEC="$TARGET_EXEC" \
WOW_BOT_WORLD_EXEC_SHA256="$TARGET_SHA" \
WOW_BOT_EXEC="$BOT_EXEC" \
WOW_BOT_EXEC_SHA256="$BOT_SHA" \
./tools/pr-preflight.sh --allow-runtime-qa \
  --ack-disposable-overworld-loot-race qa-loot-race
```

`RUST_CAPTURE_DB_CONF` is the credential source used by the bounded fixture
guard; `RUST_CAPTURE_EFFECTIVE_CONFIG` is independently pinned to the exact
config selected by the PM2 cwd/argv profile. They are intentionally different
on the current host, and the capture wrapper fails closed if the latter does
not match PM2.

`RUST_WORLD_PORT` and `RUST_INSTANCE_PORT` must be different. The fixture guard
also rejects orphan `gameobject_addon`, `gameobject_overrides`, `spawn_group`,
pool, event, or linked-respawn rows before claiming spawn `9106001`. Cleanup
checks the complete pinned spawn (including persisted `state`) and addon row
before its first write; it never resets state merely to make deletion pass.

For a no-build QA run, also provide the independently pinned
`WOW_BOT_EXEC`/`WOW_BOT_EXEC_SHA256`. Caller-supplied runtime and bot variables
take precedence over ignored `.env.local` defaults.

Fresh C++ or Rust packet recording is also intentionally excluded: the capture scripts can
restart services and perform guarded fixture mutations around an interactive client flow. The
`capture` profile does not invoke protobuf tooling. It tests committed fixtures and runs
`verify-required` for both ready milestone contracts: `loot-single-item-claim` and
`creature-spell-casting`. The local `capture`, `test`, `ci`, and `full` profiles enforce both
contracts, as does the GitHub focused-test job through direct commands. Each contract is backed by
a matched C++/Rust capture pair, completed schema-v3 manifests and lineage, and an empty committed
divergence baseline. Artifact, lineage, effective-config, executable, fixture-cleanup, or semantic
drift fails closed. The separate two-client loot race remains runtime evidence rather than a golden,
because global packet logs merge concurrent sessions.
RAW capture-manifest schema v3 distinguishes PM2's configured entry
PID/start-time/path/hash/profile from the unique PID/start-time owning both
listeners: they may be identical for a direct Rust binary, while the legacy
non-`exec` shell wrapper requires a verified descendant listener. It also
attests that the capture runtime/process tree is absent, fixture cleanup was
verified, and the normal runtime is healthy before no-overwrite publication.
For `loot-single-item-claim`, guard, canonical TESTBOT2/TESTBOT3 fixture
identity, pinned bot executable, and a semantically validated successful bot
report are mandatory on both sides. C++ derives its effective config from the
PM2 argv/wrapper and requires an exact canonical `CPP_CONF` match. Required
recording also needs a clean committed RustyCore harness/source. The known-dirty
legacy C++ checkout is identified by HEAD plus an explicit dirty flag and
deterministic worktree-state digest; its pinned live listener-binary SHA-256
remains primary. The shared private mode-0700 orchestration lock, output paths,
and raw packet inventory reject symlinks; DB password loading and both mysql
calls suppress `bash -x` so credentials and `MYSQL_PWD` cannot enter logs.

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
sandbox. Read-only shell inspections remain available, while writes and approval escalation are
blocked. The harness also checks the JSON event log for the profile's required successful
inspections: the merge-base diff for committed review, or unstaged, staged, and untracked-file
discovery for uncommitted review. Each must be its own exact successful command-execution event,
so a later command cannot mask an inspection failure. The harness then interprets the structured
result rather than assuming a successful process exit means a clean patch. Review is limited to
1800 seconds by default; set
`CODEX_REVIEW_TIMEOUT_SECONDS` to another positive number when needed.

Exit status:

- `0`: selected deterministic profiles passed and the requested review was clean;
- `10`: Codex returned one or more findings, or judged the patch incorrect;
- `64`: local usage, dependency, dirty-tree, base-ref, toolchain, or protoc error;
- `65`: Codex did not return the expected structured result or inspection event log;
- any other nonzero Codex status is preserved as an operational failure.

Review artifacts are deleted after a clean result. They are retained and printed when review fails.
Set `CODEX_REVIEW_KEEP_ARTIFACTS=1` to retain them after a clean review as well.

## CI source of truth

`.github/workflows/rust-ci.yml` executes the required Cargo commands directly. These local
profiles mirror the workflow-owned command lists:

```text
Architecture boundaries <-> ./tools/pr-preflight.sh architecture
Format                <-> ./tools/pr-preflight.sh format
Check core crates     <-> ./tools/pr-preflight.sh check
Focused library tests <-> ./tools/pr-preflight.sh test
Latest stable         <-> ./tools/pr-preflight.sh stable
```

When a required command changes, update the workflow and its matching local profile together. The
workflow is authoritative for branch protection; `Check core crates` executes the architecture
checker directly and `Format` also runs the harness self-test, but no required job trusts the pull
request's wrapper as its sole enforcement path. The `quick`, `ci`, and `full` aggregate profiles
include `architecture`.
