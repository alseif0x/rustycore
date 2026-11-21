# RustyCore structure and conventions

**Scope:** the whole workspace, including existing crates, refactors, and new development. This
is the structural standard. If work conflicts with it, either change the work or document a
reasoned change to this standard.

**Authority and separation of concerns:**

- For Forever behavior, trace the exact 70170 client build, captures, data and production path.
  Use the WotLK C++ checkout and AzerothCore to investigate inherited gameplay. Record which
  target-build evidence supports packet bytes, phase order, admission, persistence and lifecycle.
  See the [Forever runbook](../operations/forever-login.md).
- Rust structure—crates, layers, modules, visibility, and tests—is designed for this repository.
  C++ directory names may inspire domain boundaries, but do not dictate Rust crates.
- Never justify a structural choice only with “C++ does it this way.” Use this standard and
  repository evidence.

## 1. Principles

1. **Acyclic layers, dependencies pointing down.** The compiler should prevent a crate from
   depending on a layer that depends on it.
2. **Keep the application thin.** Rules live below; the application orchestrates them.
3. **One owner for each state and transition.** Do not add mirrors, universal contexts, or locks
   just to move code.
4. **Keep boundaries narrow.** A domain accepts data and returns an intent for the application to
   apply. A session-type `&mut` never crosses into a domain crate.
5. **Prefer a module before a crate.** A crate needs its own contract, compiler-enforced direction,
   and at least two consumers or a useful build-isolation benefit.
6. **Keep units manageable.** Size ceilings are work constraints: loading 100,000 lines to change
   one rule blocks effective review.
7. **Separate movement from behavior.** A gameplay fix has its own change, source anchor, and test;
   never hide it in a code move.
8. **Report evidence in levels:** compilation, crate suite, composition, then one final validation.

## 2. Layers and dependency direction

| layer | crates | may depend on |
|---|---|---|
| 0 foundation | `wow-core`, `wow-constants`, `wow-config`, `wow-crypto`, `wow-logging`, `wow-math`, `wow-proto`, `wow-collections` (utility) | 0 |
| 1 data and durability | `wow-data` (catalogs only), `wow-persistence`, `wow-database` | 0 |
| 2 model and state | `wow-entities`, `wow-map`, `wow-movement`, `wow-packet`, `wow-network` | 0–1 |
| 3 domain rules | `wow-<domain>`: `combat`, `conditions`, `spell`, `spell-acquisition`, `loot`, `quest`, `items`, `pets`, `social`, `economy`, `instances`, `battlegrounds`, `dungeon-finding`, `progression`, `ai`, `account-collections`, `anticheat` | 0–2, never layer 4 |
| 4 application | `wow-world` | 0–3 |
| 5 composition | `world-server`, `bnet-server`, `world-modules` | 0–4 |

Upward dependencies and cycles are prohibited. Domain crates must not know `WorldSession`, the
transport, or the database. Shared types belong to their semantic owner; only genuinely universal
vocabulary belongs in `wow-core`.

## 3. Names

- Crates use `wow-<concept>` for foundation/model and `wow-<domain>` for rules. Binaries are
  `world-server` and `bnet-server`; test support is `wow-test-support`; E2E tests are
  `wow-e2e-tests`.
- A module name states its responsibility. Do not use `misc`, `utils`, `common`, `helpers`,
  or `shared` as catch-all modules. A module without a semantic owner should not exist.
- Domain names may follow AzerothCore where they fit (for example `combat`, `spells`, `loot`,
  `quests`, `groups`, `guilds`, `chat`, `auction`, `trade`, `mail`,
  `battlegrounds`, `dungeon-finding`, `instances`, `movement`, `conditions`,
  `reputation`, `profession`, `trainer`, `pets`). This is guidance, not a requirement.

### 3.1 Inside a domain crate

```
wow-<domain>/src/
├─ model.rs     domain data; no Session dependency
├─ rules/       pure functions: input → decision or intent
├─ state.rs     only when the domain owns mutable state
├─ adapter.rs   temporary consumer-import bridge, removed during migration
└─ tests/       focused scenarios beside the rules they exercise
```

### 3.2 Inside the application (`wow-world`)

```
wow-world/src/
├─ session/
│  ├─ state/         WorldSession and owning substates (transport, Player access,
│  │                 loot, spells, persistence fence, catalogs)
│  ├─ construction.rs · admission · lifecycle/ · driver/ · persistence/ · publication/
│  ├─ directory.rs · registry.rs   (single opcode registry)
│  ├─ handlers/<domain>/          (decode → domain → apply → publish adapters)
│  └─ tests/                       (integration scenarios and fixtures)
└─ runtime/                        (tick orchestration; map state belongs in wow-map)
```

### 3.3 Parity naming (user decision, 2026-09-24)

Suffixes `_like_cpp` and type names such as `...LikeCpp` are legacy provenance markers. Do not
use them in new code or refactors; new identifiers name the domain.

- Keep C++ provenance where it is useful: in the rule comment and `File.cpp:line` anchor, commit
  message, and ADR/checkpoint. **Parity is in the logic, not the name.**
- Do not mass-rename existing identifiers in this programme. Thousands of names, baselines,
  registered signatures, and callers would change; any mass rename needs its own campaign and
  acceptance.
- Opportunistic renaming is appropriate when a domain is extracted or a file is genuinely
  rewritten. Keep a temporary `adapter.rs` or `pub use` alias if compatibility requires it, and
  mark it for removal.
- Apply the same rule to tests: do not add `_like_cpp` to new tests or rename old tests for style.

## 4. Choosing a cut

1. Classify the leaf: a pure rule with no session type or I/O belongs in a domain crate; state
   stays behind its owner's narrow contract; lifecycle, driver, persistence, and publication
   orchestration stay in the application.
2. Move one complete operation at a time and cite its C++ anchor; cite functions, not line ranges.
3. When a domain needs to write, return an intent enum for the application to apply.
4. Do not add helper traits, public fields, mirrored state, or locks merely to move code.

## 5. Visibility

From narrowest to broadest: `private` → `pub(super)` → `pub(in crate::<owner>)` →
`pub(crate)` → `pub`.

- For moved code, default to `pub(in crate::<owner>)`, preserving its previous effective scope.
- Use `pub(crate)` only when a sibling module genuinely needs the item.
- Use `pub` only for the crate's real API, with an external-layer consumer or integration test.
- Widening visibility just to make a move compile signals a poor cut. Reconsider the boundary.

## 6. Tests and fixtures

| what is tested | location |
|---|---|
| internal rule requiring private access | `#[cfg(test)]` inside the domain crate |
| public domain contract | `<crate>/tests/` |
| composition: packet bytes, order, phase admission, COMMIT/persistence, login/logout | `wow-world/tests/` or `wow-e2e-tests` |
| fixtures/builders shared by multiple crates | `wow-test-support` as a dev-dependency |

For every test move, count before and after: `#[test]`/`#[tokio::test]`, `inventory::submit!`
(opcode registrations), and the scenario list. Compilation alone does not prove a moved test still
runs.

## 7. Size budgets

| unit | ceiling | target |
|---|---:|---:|
| physical file | 1,000 lines | 400–600 |
| logical owner (hotspot ratchet) | 20,000 | — |
| domain crate | about 60,000 | — |
| application layer (`wow-world`) | 200,000 | — |
| ownerless catch-all module (`misc`/`utils`/`common`) | 0 | 0 |

Change ceilings only with a reviewed delta after validated retirement. Never regenerate them
blindly or raise them merely to pass a phase.

## 8. Decision tree for new code

1. Deterministic rule with no session or I/O? Put it in a domain crate's `rules/`.
2. Canonical mutable state? Keep it with its existing owner and a narrow API.
3. Network bytes? Use `wow-packet`/`wow-network`. Durability? Use
   `wow-persistence`/`wow-database`.
4. Session orchestration? Keep it in the application.
5. Bootstrap and wiring? Keep them in composition.
6. No contract and one consumer? Use a private module, not a crate.

## 9. Change checklist

- [ ] The cut is one complete operation with a cited C++ anchor or approved reference.
- [ ] Dependencies point down; no cycle or new back-edge is introduced.
- [ ] Each state and transition has one owner and writer; no mirrors or extra locks.
- [ ] No session-type `&mut` crosses into a domain; inputs are DTOs and outputs are intents.
- [ ] Visibility is as narrow as practical; nothing is widened only to compile.
- [ ] Tests follow §6; scenarios, assertions, and counts are preserved.
- [ ] File, owner, and crate budgets in §7 are respected.
- [ ] Movement and behavior changes are separated.
- [ ] Physical ledger/policy changes have a reviewed delta, never a blind regeneration.
- [ ] Evidence is labeled accurately: compilation ≠ suite ≠ composition ≠ final validation.

## 10. Mechanical enforcement

- `[workspace.lints]` plus crate-level policy enforce rustc/Clippy, `unsafe`, and public API
  documentation rules.
- `cargo xtask check-layers` checks allowed layer edges; `cargo xtask structure-audit` checks
  sizes, largest files, stubs, and crates without consumers.
- `cargo-machete` and `cargo-deny` monitor unused dependencies, licenses, advisories, and
  duplicate versions.
- Decide optional subsystem features once and document them here. The base build must not require
  optional features.
- Mark vendored code (for example, `wow-recastdetour`) as exempt from size and naming budgets;
  retain its dependency classification in `dependency-policy.json`.

Anything outside these checks is convention, not an enforced standard.

## 11. Changing this standard

Explain the reason and impact in a documentation change. Update
[wow-world-distribution-plan.md](wow-world-distribution-plan.md) when phases or budgets change and
[workspace-structure-programme.md](workspace-structure-programme.md) when phase order or status
changes. Do not create competing documents: this is the structural standard, the distribution
plan governs `wow-world`, and the workspace programme orders the full-workspace work.
