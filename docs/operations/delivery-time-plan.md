# Reduce delivery time on 3.4.3

Maintainer plan, 2026-11-20, integration base `3ae34a264`. The [validation guide](validation-v2.md) describes the workflow in this checkout.

## Where the hour went

PR #1494 took **63 min 52 s** from the replayed contributor commit at 15:07:55 UTC to the last required check at 16:11:47. This window excludes investigation before the replay and time awaiting merge. About 34 minutes elapsed before local acceptance started. That included implementation, evidence, tests and policy maintenance. The artifacts do not split that interval further. Acceptance, repair and rerun took 19 min 16 s. Publication and hosted checks occupied the remaining 10 min 35 s. The reviewer and Rust CI ran concurrently. The exhaustive scan also overlapped local work. [PR](https://github.com/alseif0x/rustycore/pull/1494) · [Rust CI](https://github.com/alseif0x/rustycore/actions/runs/44769285374) · [Review](https://github.com/alseif0x/rustycore/actions/runs/44769285412)

| Evidence | Measurement | Decision it informs |
| --- | --- | --- |
| 22 retained local final runs, 19–20 November | 12 green runs with a median of 4 min 49 s. Ten failed or interrupted attempts consumed 56 min 42 s in total. | Measure failures and waiting as well as passing runs. This sample comes from one checkout. |
| #1494, aarch64, green head `d2b3501be` | 706.871 s total. Architecture and syntax took 126.906 s, downstream compile 53.760 s, library test command 274.617 s, SQL module integration 217.451 s. | The module check cost 31%. Removing only that step in this case would give about 8 min 09 s **if all other times stayed equal**. |
| Cargo timings for the same command | Six suites reported 6.23 s executing 5,933 tests. Constructing the test targets took 215 s for `world-server` and 118 s for `wow-world`, partly in parallel. | Optimise the build while retaining the fast regression cases. |
| First attempt at `52ecf58d9` | A new World test called a nonexistent method. The earlier checks compiled two other crates. The attempt that detected the error took 412.460 s. | Compile the changed test before broad acceptance. |
| Persistence inventory | An auction configuration option changed four fingerprints of roughly 21 KB each. The derived boundary policy was identical. The independent x86_64 scan took 754.957 s. | Distinguish actual database flow from unrelated fields in the same initializer. |
| #1489, a second delivery | Local final took 906.666 s, including 223.4 s in module integration. | Validate proposed changes against several responsibilities. |

The local manifests are `20261120T154155.959988Z-539846-final`, `20261120T154924.905404Z-541635-final` and `20261120T105600.034096Z-495384-final` under `target/validation-v2/manifests/` in the review checkout.

## Replacement pilot

Develop the replacement independently on `perf/validation-from-scratch`, based
on `316a7387c`. Design its checks from the failures they must detect. Use the
existing runner and recorded results as the control for comparison. Earlier
optimisations and their measurements follow below.

### What the new validation must prove

| Changed contract | Required evidence |
| --- | --- |
| Documentation | Diff hygiene and touched links. Treat documents consumed as fixtures, build inputs, or policy as part of the consuming contract. |
| Behavior inside one package | Run the owning tests. Verify its production build when test compilation uses different configuration. |
| Public API, wire format, registration or startup | Exercise the affected producer and consumer. Build affected production targets. Run the applicable integration or capture tests. |
| SQL, durability, lifetime or runtime | Check the touched boundary with current ownership policy, transactional tests and the required runtime or database exercise. |
| Cargo features, build inputs, toolchain, checker or unknown scope | Require broad validation. Test changed feature forwarding in isolation, where Cargo feature unification could otherwise hide a failure. |

Use existing tests and domain checkers as evidence. The new entry point should
stay small and take the exact Git base and candidate. Derive Rust ownership
and dependency edges from Cargo metadata. Use explicit inputs for generated
code, captures and integration contracts so ambiguous edits select the wider
route. A plan lists each command and the condition that selected it. Each run
records its head, base, selected targets, features, elapsed time and exit status.

### Prototype and compare

Compare both entry points on the same change and host. Record cold setup,
warm feedback, and time to a complete verdict separately. Challenge the
routes with the failures below before considering a switch. A required gate
needs equivalent coverage of feature, production and state boundaries and
repeatable savings. Move the global audit off the PR path only when the
candidate already checks the relevant inputs.

Pilot targets are 30 seconds to useful feedback on a warm local leaf change
and two minutes for ordinary hosted validation with a warm cache. Use five
minutes as a separate target for cross-crate contract changes such as #1494.
An ordinary change that routinely needs the full server build misses the
design target. Move its regression into its owning package where the same
behavior can be exercised there. Run production-linked tests when changes
can affect production composition. Track reviewer and queue time in the
total delivery measurement.

### First workload to replace

The hosted final for #1494 took 425.543 s with a restored Cargo cache.
The downstream test-target check took 152.764 s, the library test build and
execution took 163.276 s, and the global hotspot scan took 90.770 s. The two
Cargo commands spent their longest compilation on `world-server`.
Measure which production and test targets the two commands both compile.
Evaluate the hotspot rule against affected owners first and retain the full
scan for changes to its policy or uncertain mount inputs. The two steps under
review cost 243.534 s in that run. This is measured work to examine. Savings
depend on the cost and coverage of their replacements.
[Hosted timing artifacts](https://github.com/alseif0x/rustycore/actions/runs/44829841624/artifacts/14675574137)

An exploratory combined Cargo command on `ee15604ee`, with the same tree as
`316a7387c`, ran the six affected library suites and
`production_handler_registry_contract` together. It passed 5,941 tests with
two ignored in 221.900 s on aarch64 with three jobs, existing dependency
artifacts and a fresh production test build. Cargo compiled `world-server`.
The original 239.985 s local final used a different target set and cache state.
The 221.900 s run proves the combined test route for this candidate. A paired
comparison must still account for the omitted consumer and architecture checks.
With the same command and the production test already built, removing a row
from its expected registry fixture failed the production comparison in
28.778 s. Restoring the fixture passed in 28.190 s. The fixture edit exercises
the assertion. The production registration replay is recorded below.

### Run the independent pilot

From the checkout:

```bash
# During editing, run the named test for the changed package.
./tools/validate-change --base HEAD --working --focus distance_2d_ignores_y
# For a multi-package diff, name the affected package containing the test.
./tools/validate-change --base HEAD --working --package wow-world --focus auction_hello_request_uses_auctioneer_faction
# See the selected commands for a committed candidate, then execute them.
./tools/validate-change --base origin/3.4.3 --plan
./tools/validate-change --base origin/3.4.3
```

The planner reads workspace manifests without fetching Cargo dependencies.
Cargo fetches missing dependencies for the selected targets during execution.
The declared path graph on `316a7387c` matched all 395 workspace edges in
Cargo's all-feature resolution. Set `CARGO_NET_OFFLINE=true` for an offline
build with populated Cargo caches. Set `PROTOC` for
protobuf-dependent builds and use the checkout's own target directory. The
pilot defaults to one Cargo job and honours `CARGO_BUILD_JOBS`. It runs the
changed packages' test targets, checks their production builds and affected
consumers, checks downstream test targets and checks member formatting. Focused
mode runs matching library tests and rejects a filter that executes zero tests.
Documentation-only edits under `docs/operations` and `docs/audits` check
whitespace and relative inline file links without starting Cargo.

Exit 0 records the checks shown for the selected mode. Exit 2 means the diff
contains a path needing additional acceptance, including untracked inputs and
changed member manifests. Other nonzero exits identify a failed check or an
invalid invocation. A successful pilot is editing or comparison evidence.
Required publication checks continue through the existing workflow. Dependency
edges describe build relationships. They cannot prove client packet captures,
durability, database migrations, runtime behaviour or the #1241 net-move
contract. A change involving those contracts still needs their specific evidence.

### Local failure replays

These timings include starting the pilot through its exit on aarch64, with
three Cargo jobs and an existing target directory. The checkout was
`ee15604ee`, which has the same Git tree as integration revision `316a7387c`.
Each defect was applied to the working tree, tested, repaired and tested
again. The second invocation can reuse compilation from the first.

| Change exercised | Failure detected | Corrected result |
| --- | ---: | ---: |
| An extra coordinate changed the 2D distance | 5.606 s | 0.833 s, one focused test |
| #1496 added four bytes to `InspectResult` | 4.566 s, 594 bytes instead of 590 | 3.388 s, one focused test |
| #1494 reintroduced the test call to the missing auction handler from `52ecf58d9` | 67.359 s, compiler rejected the method call | 106.563 s, one focused test |
| Removed a bank registrar from the real server composition | 285.959 s, production registry disagreed with the snapshot | 327.866 s for the full route, including 12.714 s for the production-linked test command |
| Removed the required state writer in the runtime ownership ledger | 3.186 s, checker identified `test_only_fixtures` and its missing writer | 59.853 s, full architecture policy check |

The packet replay inserted `w.write_uint32(0)` immediately before
`self.talents.write(w)`. The auction replay used the original
`crates/wow-world/unit_tests/handlers/economy/tests/auction.rs` from the
failing commit. The registration replay removed the call to
`wow_world_inventory::register_bank_handlers_like_cpp` in
`crates/wow-world/src/handler_composition.rs` without changing the expected
snapshot. The production-linked test reported the missing registrations.
The largest cost on the corrected registration was checking downstream
production and test targets (252.410 s). Rustc compiled both the ordinary
and test configurations of the `world-server` library. This first full run
exceeded the shared-change budget by 27.866 s. With those configurations
warm, a fresh comment edit in the same composition file took **66.208 s**
for the full pilot. The downstream check then took 7.863 s. Both costs
matter on a developer machine: the faster run assumes that the required
Cargo configurations already exist in that worktree's cache.

The CLI regression suite uses a small Git/Cargo workspace to exercise a broken
documentation link, mixed documentation and unknown input, a broken downstream
test, a production build that only compiles with a test feature, and broken
Cargo feature forwarding. Run `python3 tools/ci/test_validate_change.py` to
reproduce those cases without compiling the server.

### Paired elapsed times

The following edits were held constant while the current `validation-v2 final`
and the independent pilot ran on the same checkout for each pair.

| Edit on aarch64 | Existing final | Independent pilot |
| --- | ---: | ---: |
| One local link in this document | 1.965 s | 0.180 s |
| Comment in `wow-math`, warm cache | 40.135 s | 0.758 s |
| Same math change again, with the pilot running before the control | 40.747 s | 0.589 s |

The math pilot ran all 29 tests, checked the production library and checked
formatting. The existing runner spent 37.246 s of its 40.135 s on the global
hotspot ratchet. `wow-math` had no reverse package dependencies and the edit
touched none of the eight audited owners. These results establish the
measured saving for this leaf. They do not establish the same saving for
arbitrary Rust edits or equivalence of global architecture coverage.

On hosted Ubuntu x86_64 (`2bb13186c`), Actions took **42 s** from run
creation to completion, including 3 s before the job, checkout, Rust setup
and both measurements. The first pass checked documentation and the leaf
crate in **5.370 s** from an empty Cargo target. The second ran the warm
math gate in **0.294 s**. Both ran all 29 tests. The workflow used one job
and restored no Rust build cache. It ran on an isolated branch, so these
42 s exclude contributor review and merge time. [Hosted run](https://github.com/alseif0x/rustycore/actions/runs/44837541667)

Planning the complete #1494 diff (`31841d891..316a7387c`) selected six
changed packages and twenty additional consumer packages. It returned exit 2
for the persistence snapshot, session ownership policy, test ceiling and test
census. The timings for the individual auction regression do not establish
the five-minute target for accepting that whole PR. A cutover for shared
changes needs that comparison including all four inputs.

### Independent full validation

Run `./tools/validate-full --base origin/3.4.3` on a committed candidate.
Use `--report /tmp/full-result.json` to retain the commit pair, commands,
exit codes and elapsed time. `--plan` prints the proposed work. An uncovered
input exits with code 2. A build-only or policy-only report supplies partial
evidence. The default command runs both in sequence on the development host.

`./tools/validate-full --workspace --base HEAD` requests an independent
audit of the entire workspace, including the full persistence inventory,
native checker fixtures, both committed capture contracts, the SQL launcher
and the standalone bot. Its production build covers every workspace member,
including independent binaries such as `bnet-server`. It also works on a
commit with no diff. The reusable
Actions workflow runs test targets, the ordinary production configuration,
the generated SQL launcher and policy on independent runners. With
`whole_workspace: true` it also starts a
dedicated inventory runner. One verdict requires every selected part to
pass. Local execution uses one checkout and runs the checks in sequence.

The build uses Cargo's resolved dependency graph to run the changed packages
and their consumers together, including integration targets. A second check
uses the production feature configuration. Changes to the launcher or its
composition inputs also exercise the generated SQL launcher. A change to
the module list alone fetches the locked dependencies before the offline
compositor starts on its own runner. The policy
checks exercise the architecture fixtures, session ownership and reviewed
persistence data. Rust source and module-graph changes also run the native
handler contract. It examines registrations in the source, including
configuration-dependent registrations that the debug tests can miss.
On the auction checkout, gating the battle-pet journal opcode with
`#[cfg(debug_assertions)]` made the source guard fail in 50.179 s.
Supported local-binding fingerprint changes are checked
against both Git revisions. Changes to the persistence scanner, semantic
policy or an unsupported snapshot shape require the native full inventory.
The independent global audit and any required live evidence have their own
scope.

On aarch64 the auction replay used tree `dec6207b8` from #1494, base
`31841d891`, three Cargo jobs and the checkout's existing debug target.
It passed 6,387 Rust tests across affected targets, with two ignored, plus
the SQL compositor and policy checks. The first complete run following two
deliberately broken candidates took **235.662 s**. Repeating the same tree
with the built artifacts in place took **152.625 s**. The earlier local
final took **239.985 s** under a different cache history. The full build
detected the missing auction handler method in **13.550 s** and the
four extra bytes in the inspect packet in **66.606 s**.

On the same local checkout and commit `c5a74048e`, the existing final spent
**735.734 s** checking the new standalone validator and its benchmark
workflow. It routed the new standalone Rust parser to a workspace build.
The independent validator ran its own regression suite and workflow checks
on that candidate in **7.654 s**. The existing final exceeded the ten-minute
warm publication budget on this change.

The initial hosted replay checked out `316a7387c` on fresh Ubuntu x86_64
runners. All arms missed the dependency cache. The previous complete
change validation took **11 min 07 s** from run creation through its job.
The independent validation took **9 min 04 s** through the slower of its
two jobs. The build spent 258 s on test targets, 119 s on production and
87 s on SQL composition. Both implementations passed. Setting Cargo's job
count after cache restoration allows the integration cache key to match.
[Cold comparison](https://github.com/alseif0x/rustycore/actions/runs/44844996285)

Subsequent runs checked the same auction tree with a hit on the same
integration dependency cache. Times include the queue and setup. The
reference time ends when its job completed. For the last two runs the
independent time includes its aggregate verdict.

| Hosted comparison on #1494 | Elapsed |
| --- | ---: |
| Existing full change validation | [9 min 08 s](https://github.com/alseif0x/rustycore/actions/runs/44845909587) |
| Independent build and policy on two runners | [11 min 18 s](https://github.com/alseif0x/rustycore/actions/runs/44845909587) |
| Tests parallel to production with SQL and policy | [5 min 44 s](https://github.com/alseif0x/rustycore/actions/runs/44846574051) |
| Independent tests, production, SQL and policy | [4 min 14 s](https://github.com/alseif0x/rustycore/actions/runs/44847824091) |

The four-runner result used 13 min 45 s of aggregate runner time including
the verdict, compared with 9 min 04 s of active time for the reference job.
Two preparatory runs failed on an unused tooling prerequisite and on missing
repository history for policy provenance. The passing replay includes both
fixes. These are individual measurements on hosted x86_64 machines.
The four-minute result covers change acceptance. The repository-wide audit
additionally rebuilds the entire persistence inventory.

On the aarch64 development host, the first independent serial workspace audit of
the same tree passed in **34 min 40 s**. It used the existing target
directory and three Cargo jobs. The broader workspace test configuration
took 9 min 48 s, and the fresh persistence census alone took 17 min 40 s.
This global measurement includes the native checker suite, bot, both pinned
capture flows and the SQL launcher. Run it separately from editing feedback.

On hosted x86_64 the previous integration audit of `316a7387c` took
**22 min 13 s** from run creation to job completion. The independent global
audit of the same tree passed in **11 min 58 s** through its aggregate
verdict. This first hosted measurement checked the production launcher
and its dependencies. Review subsequently extended production coverage to
every workspace member. Its five workers and verdict used 29 min 17 s of cumulative runner
time. The exhaustive inventory finished last. An initial run exposed a
missing offline SQL dependency on an isolated runner. Fetching the locked
dependencies before SQL composition fixes fresh runners.
[Previous integration audit](https://github.com/alseif0x/rustycore/actions/runs/44831037358)
· [Independent global audit](https://github.com/alseif0x/rustycore/actions/runs/44848959133)

With every independent production binary included, the global audit passed
in **12 min 26 s** and used 30 min 38 s of aggregate runner time. All five
reports contain the same auction commit. The production report records
`cargo check --locked --workspace`, and the inventory report records the
fresh native check. [Complete production coverage](https://github.com/alseif0x/rustycore/actions/runs/44851618587)

An isolated aarch64 experiment cached parsed syntax trees inside one
invocation of the native inventory checker. It passed the same inventory
with 44,916 cache hits out of 52,947 parses and took 1,066.399 s. The
baseline took 1,060.396 s. The published validator uses the normal parser.

## First delivery and measured changes

The first delivery changes the implementer's feedback loop, the order of compiler and architecture checks, and SQL composition routing. On aarch64, the focused test reproduced the original #1494 compiler error at `52ecf58d9` in 192.506 s (Cargo exit 101). The earlier full attempt ended after 412.460 s. These were different commands with different cache histories. In a separate negative check, removing the public `run_with_composition` export made Cargo reject the real generated SQL launcher. The checkout was restored after the experiment.

Replaying the clean game commit `d2b3501be` against `3ae34a264` took **218.227 s** with the candidate planner, the same aarch64 worktree, three jobs, `final --architecture --timings --logs` and a warm cache. The final implementation produced the identical ordered command plan. Both architecture checks and all 5,933 library tests passed, including the auction regression. The original run took 706.871 s. The removed SQL command accounted for 217.451 s in that run. Cache state also differed, particularly for the library tests (274.617 s originally, 38.077 s in the replay). This measures local acceptance; measuring review through hosted verdicts needs a published delivery.

As a control, the omitted SQL command was run directly after the replay on the same checkout and cache. It passed in **6.801 s** and restored the checkout. This is the measured marginal cost with both build configurations already warm. The historical 217.451 s cost applies to the earlier cache state. Reusing a clean review worktree between candidates is therefore part of the operating change. Measure the next real reviews before prioritising steps 3–6.

1. **Catch errors in the owning target.** The implementer now exercises the target that compiles the changed regression, including consumers in a different crate. The integrator can take a bounded fix through its tests. A larger assignment includes its callers and tests together. In a final Rust run, the downstream compile precedes the hotspot or combined architecture scans. A #1241 distribution PR retains its programme-specific checklist. [Working rules](../../AGENTS.md) · [Local workflow](local-first-development.md)

2. **Route the SQL module build by its inputs.** The planner recognises literal boolean configuration fields in the session runtime initializer. The rest of the source must match the comparison base. Changes to composition, its public surface or coupled build inputs retain the full example build, as do ambiguous startup edits. Comparing the plans against the recorded #1494 and #1489 manifests removed only the SQL command from #1494 and preserved every command for #1489. [Runner](../../tools/validation-v2) · [Planner tests](../../tools/test_validation_v2.py)

3. **Reduce manual policy maintenance.** The architecture checker maintainer starts with the four `session_resources` aliases. For a resolvable aggregate, fingerprint each database-bearing field, its source, destination and operation. Preserve contexts and multiplicity. Unknown expressions keep the full fingerprint. Compare both inventories on one baseline and review changed semantic groups before migrating the snapshot. Tests must show that changing just the auction flag yields zero database changes, while swapping handles, introducing an escape or changing SQL or transaction behaviour is still detected. Generate mechanical census and line totals together using the existing tools. Pilot reporting harmless line drift for ordinary work, with lost tests, new dependencies, widened APIs and material hotspot growth still visible for review. Exercise both a harmless test edit and a real boundary breach before changing enforcement. [Alias collector](../../tools/architecture/handler-contract-check/src/persistence_access/body_visit.rs) · [Test census](../architecture/wow-world-test-census.md)

4. **Assign each acceptance check an environment.** The CI maintainer compares the actual local and hosted commands. For external PRs, propose focused local evidence and checks missing from CI, then one hosted final with a verdict on the current head. Local `--architecture` adds checks beyond the default hosted final. Route or retain those explicitly before removing overlap. Keep first-party local final, independent integration audits and the external-author boundary. Start the external review once per ready head alongside CI using the configured integration, and surface a missing trigger promptly. Prove handling of fork PRs, stale reviews, maintainer PRs and integration pushes. Keep the required check names stable. Switch publication rules only after comparing coverage on both routes. [Rust CI](../../.github/workflows/rust-ci.yml) · [Reviewer gate](../../.github/workflows/codex-review-gate.yml)

5. **Bound the cost of repairs.** The runner maintainer credits tests already covered by final and applies the existing documentation-only reuse rule. On failure, show the exact target, command and features for reproduction. Measure remaining repeats before building cross-commit reuse. If warranted, begin with deterministic source-only checks. Match the base, input contents, selected command, checker version and relevant environment, and cite the original green run. Changed features, generated inputs, source mounts or uncertain dependencies trigger execution. Leave binary rebuild decisions to Cargo. Verify that a changed checker, renamed file and stale result invalidate the receipt. Any disagreement with full execution restores the normal route.

6. **Shorten the builds still on the critical path.** The build maintainer compares a warm leaf edit and a shared protocol/config edit on aarch64, then checks the result on hosted x86_64. Use Cargo timings with the same toolchain, jobs and comparable cache state. Measure the current linker against supported aarch64 LLD and test whether a commit-only change forces an expensive server rebuild. Change one variable at a time. If the large test targets still dominate, move domain-local tests to existing owner crates using the distribution work already underway. Keep production wiring tests in the server and compare test identities, features and ignored cases across owners. Adopt an optimisation only when representative runs improve and debug usability, version reporting and coverage pass. [Build configuration](../../.cargo/config.toml) · [Build-input probe](../../tools/measure_build_inputs.py)

## Day-to-day development measurements

The main development checkout retained 154 successful `final` runs with changed paths
and recorded commands from 23 August to 1 November 2026. Their median was about 11 minutes,
and 82 exceeded the 600-second warm-campaign target. This is a historical sample of final
checks across different scopes and cache states. It cannot measure time spent editing or
waiting for people.

The module-mount scan used by the hotspot check now avoids parsing a file a second time
to find explicit module paths when its source lacks either required token. On the same
aarch64 checkout, running the already-built release checkers in sequence took 36.000 s
before and 27.113 s after. Both emitted byte-identical JSON for all 979 mounts, including
349 test-only mounts. These times exclude compilation. The contributor and refactor
guides describe how to exercise the owning test during development. The QA wrapper uses
the existing debug build for ordinary Rust scenarios.

On integration revision `b17af773c`, putting the four extra bytes from #1496 back into
`InspectResult` made the packet test fail in 6.840 s (594 bytes, expected 590).
Restoring the encoder made the same test pass in 3.999 s. Both runs used a warm
aarch64 debug cache, one Cargo job and this command:

```bash
cargo test --locked -p wow-packet --lib inspect_result_default_talents_have_exact_54261_size
```

Both runs executed one test. With a deliberate formatting error in the same source,
the workspace and member rustfmt checks both identified the file and exited 1,
in 12.718 s and 0.493 s respectively. These measurements cover a packet edit.
Shared server rebuilds and whole-delivery acceptance have separate elapsed times.

### Native aarch64 test linking

On the #1494 revision `d2b3501be`, the native GCC driver linked each debug test
binary twice using the same Rust objects, libraries and arguments. The second
linker was the LLD 22.1.8 supplied with Rust 1.98. The two targets used opposite
linker orders. Each duration covers just one link.

| Test target | GNU ld | Rust's LLD | Suites with each linker |
| --- | ---: | ---: | --- |
| `world-server` | 8.596 s | 1.485 s | 643 passed, 1 ignored |
| `wow-world` | 11.657 s | 1.622 s | 4,175 passed, 1 ignored |

Both versions listed the same test identities. For each target, `addr2line`
resolved a sampled test symbol to the same Rust source line. In a small Cargo
fixture, enabling the native GCC linker lookup kept the existing target fresh.
After a source edit, the new ELF identified LLD. Changing Cargo's target linker
setting instead rebuilt the unchanged fixture. The
[local development guide](local-first-development.md) shows the native setup.

For a complete warm edit, changing the auction-hello regression to expect house
6 instead of 7 made the same filtered test fail in 42.232 s with GNU ld and
30.640 s with LLD. Restoring the expectation passed in 31.773 s with LLD.
Both runs used the same six-package `--lib` feature selection as the #1494
replay, one Cargo job and the same checkout. These times include compilation,
linking and test execution. The LLD failure was measured first.

## How to choose evidence

| Change | First relevant result | Acceptance to retain |
| --- | --- | --- |
| Documentation or instructions | Inspect the diff and links | Existing documentation-delta rule when inputs and base qualify |
| Domain behaviour or fixtures | Owning target and regression case | Affected consumers and domain suite |
| Shared packets, API or configuration | Bytes or policy precedence, real producer and consumer | Dependent builds, integration and action-specific captures where required |
| SQL, lifetime, ownership, runtime or modules | Relevant transaction, lifecycle or composition case | Applicable exhaustive inventory, live durability/relogin, runtime or composed build |
| Checker, toolchain, global inputs or uncertain effects | Planner contracts and known failure fixtures | Conservative wider acceptance including affected standalone tools |

Use the union for changes spanning several rows. Larger features get early evidence on each working operation and one terminal acceptance for the complete behaviour and required live scenarios.

## Pilot and exit criteria

Use existing manifests, Cargo timings and GitHub timestamps. Record review start, first relevant result after a coherent edit, candidate ready for acceptance, required verdicts complete and actual merge. Include retry counts and causes. On larger developments also measure implementation time and waiting per coherent operation. Count overlapping jobs once in elapsed time. Label cold builds, warm builds, external queues and live QA. Keep these few numbers in the existing PR evidence.

For the next ten applicable deliveries, report the median, the slowest case and each budget miss. Include a shared-contract change and a larger delivery. If one is absent, report the gap and measure the next such delivery. Keep the existing **10-minute warm local acceptance budget**. Pilot **30 seconds to the first relevant result for a warm single-package edit with a focused test**, **5 minutes for a shared-contract edit**, and **20 minutes median from a ready ordinary external candidate to both required verdicts**. For larger deliveries, target **half the validation and coordination waiting per coherent operation** against a comparable pre-change delivery. Track total review-start-to-ready time as well so that shifting work earlier cannot hide it. These are proposed performance targets.

Aim for a **30-minute median from active review start to required green checks** on comparable external corrections such as #1494. Record time from contribution to first maintainer response separately. For recoverable failures, use the next coherent edit as the feedback start while keeping the original start for total elapsed time. When comparable historical data is missing, record a baseline on the first similar operation before making a savings claim.

For each change, retain a known regression that its route must catch. If a required failure escapes, restore the broader route and repair the selection rule. Stop adding process once the targets hold with equivalent coverage. If the pilot misses, choose the next intervention by the largest measured delay.
