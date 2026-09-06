---
name: design-rustycore-architecture
description: Design, audit, or challenge RustyCore architecture and plans when ownership, runtime, dependency, or extension boundaries are in question. Compare alternatives against current evidence and the user's goals. Not for routine fixes or implementing an already approved behavior-preserving refactor.
---

# Design RustyCore Architecture

Evaluate architecture by the capabilities it enables and the risks it controls. This is a
decision method, not a fixed crate map, technology choice, issue sequence, or current-state audit.

## 1. Start with the requested outcome

Identify the capability, scope, and acceptance the user needs. Distinguish analysis from execution.
Do not measure success solely by fields moved, file sizes, crate counts, or closed issues.
Physical navigability is a separate acceptance criterion, not proof of semantic modularity.
Preserve useful completed work without treating its design as beyond challenge.

## 2. Establish relevant evidence

Read `AGENTS.md` completely and follow its kickoff. Consult relevant parts of `docs/migration/STATE.md`,
the active issue/checkpoint, `PORT_PLAN.md`, and affected ADRs; inspect the actual callers, owners,
manifests, and tests before relying on their claims. A bounded question does not require a fresh
whole-workspace inventory or a complete historical-log read.

For affected base-server behavior, locate exact C++ owners and call paths under
`/home/server/woltk-trinity-legacy`; use real captures when C++ is incomplete or ambiguous. Existing
Rust and old tests are not parity proof. Distinguish implemented, integrated, and parity-proven.
Keep architecture snapshots and campaign plans in project documents with a date, audited commit,
and coverage limits, not embedded in this skill.

## 3. Compare meaningful alternatives

For a consequential decision, compare retaining or improving the existing design with credible
alternatives. Explain capability benefits, complexity, migration cost, failure risks, and what
evidence would change the recommendation. Measure representative paths when performance matters.
Neither ECS nor a particular library, crate layout, or deployment topology is an end in itself.

Uncertain ownership is a subject for analysis, not a reason to forbid proposals. State assumptions;
resolve ownership and execution contracts before enabling new writers or concurrency. Existing
approved decisions govern execution until an authorized replacement; they are not correctness proof.

## 4. Preserve invariants, not physical layouts

Keep one canonical authority per mutable state and one execution owner per transition. Trace the
affected readers, writers, lifetime, persistence, and publication together; temporary mirrors need
an explicit sync direction and retirement condition. Semantic ownership does not require a fixed
class, struct, crate, or storage backend. Preserve C++ phase order and base behavior; handle proven
legacy defects through the project's fidelity policy, separately from behavior-preserving changes.

Keep domain rules separate from transport, SQL, and composition. Prefer private modules while
boundaries evolve; justify crates, traits, and tasks by a useful contract, not diagram compliance.
Do not expose internals just to relocate code or tests.

Never carry synchronous locks or map/entity state guards across `.await`, or perform I/O or packet
delivery under a map lock. An intentional asynchronous operation gate may span `.await` when its
owner, lock order, cancellation/recovery, and blocking scope are explicit; do not remove a persistence
fence merely to satisfy a blanket lock rule. Preserve coherent mutation/commit/publication and
backpressure semantics when changing an owner.

## 5. Demonstrate modularity when it is the goal

For module boundaries, source decomposition or refactor plans, read
`docs/architecture/module-design-guidelines.md` completely. Apply its independent semantic and
physical criteria to production, tests and fixtures; the project document owns the current
budgets and exception policy. Prefer a responsibility-oriented module tree inside justified crate
boundaries. Do not substitute a distributed God object for a giant file, or keep giant files merely
because the logical aggregate is valid. Treat example skeletons as guides, not mandatory layers.
Plan legacy file retirement inside the approved macro without adding routine approval gates.

Distinguish internal organization, external extension contracts, and private runtime storage.
For external extensibility, define acceptance through a useful independent module exercising the
public contract without patching core implementation or importing its storage, entity guards, SQL
connections, or packet writers. In design-only work, propose this acceptance; exercise it during
authorized implementation, not as a prerequisite to discussing alternatives.
Define the relevant before-decision hooks, confirmed-result notifications, or scoped behavior
capabilities, including composition order, conflicts, failures, and module-state lifetime/persistence.

Zero optional modules must preserve base behavior and required first-party scripts. Optional
customization needs an explicit behavior contract and cannot bypass core integrity invariants.
Address API compatibility and install/update/removal where relevant. A narrow API does not sandbox
trusted native code; do not promise isolation or hot
reload without an implementation and evidence. Do not freeze a universal plugin framework from a demo.

## 6. Deliver complete capabilities with proportional evidence

Honor the approved delivery size, including macrodeliverables, with coherent internal commits and
checkpoints rather than automatic micro-issues, micro-PRs, or repeated approvals. Select cuts by
complete operations and their dependency/bridge retirement, not a mandatory mechanical-move sequence.
Keep restructuring and intentional behavior changes distinguishable; do not silently expand or
reduce accepted scope.

Use focused positive/negative tests during iteration and affected integration/failure cases at
owner boundaries. Retain `AGENTS.md` and explicit issue acceptance gates for capture, live QA, and
final validation; do not repeat exhaustive audits after every helper or equate partial tests with
terminal acceptance. Report the verdict, supporting anchors, tradeoffs, remaining risks, and next
step concisely, with detail proportional to the decision.

## 7. Preserve authority and completion boundaries

Review-only requests remain read-only. Reuse approval within its stated scope; pause an unresolved
mutation while continuing safe investigation. Changes to plans, behavior, or external systems need
the applicable authority; this skill grants no new push, merge, deployment, or destructive permissions.
Once design and implementation are approved, use `$refactor-rustycore-safely` for behavior-preserving
restructuring without requesting the same approval again. Continue authorized work through acceptance,
not merely to the next internal checkpoint; distinguish local completion from publication.
