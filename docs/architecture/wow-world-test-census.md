# wow-world test census (#1442 T0)

Owner: GitHub #1442, slice T0 of #1441 (move the `wow-world/unit_tests` tests that
have a domain owner to that crate). Measured at `origin/3.4.3` `412bfd06c`. This
is a planning inventory and a size ratchet. No test moved, and no behaviour changed.

- Tool: `tools/architecture/wow_world_test_census.py` (`census`, `check`, `self-test`;
  no Cargo), with its scanner `wow_world_test_scan.py`.
- Ledger: `tools/architecture/wow-world-test-census.json`, one row per test, with
  the columns listed in its `test_columns` key.
- Ceiling: `tools/architecture/wow-world-test-ceiling.json`.

## Result

The census covers all **3,731** tests: 3,709 in `unit_tests/` and 22 in `src/`.
That matches the attribute count, which the tool cross-checks. One test is
`#[ignore]`, and every `unit_tests/*.rs` file is mounted.

| Class | Meaning | Tests | Test-fn lines |
|---|---|---:|---:|
| **W** composition | Uses World logic, several domains, or only World fields. Stays in `wow-world`. | 3,265 (87.5 %) | 179,563 |
| **H** domain-movable | Builds a session, but every session call is a host-trait method of, or a one-statement forward into, one domain crate. Can move only with a test host for that crate. | 165 (4.4 %) | 4,353 |
| **P** session-free | Never builds or names a session. | 301 (8.1 %) | 9,965 |

Other totals:

- **Session use.** 3,430 tests build or name a `WorldSession`. 3,246 of them call a `cfg(test)` shim.
- **Lines.** The test functions add up to 193,881 lines. The other ≈39k lines of `unit_tests/` are fixtures, shims and mount files.

**Moving tests to domain crates is a small slice, not most of the suite.** It
confirms the plan's finding: World composition owns most of these tests.

- **T1 move-only:** an estimated **231 tests / ≈8.3k lines**. These are the P tests with a domain destination.
- **With a test host (T2 to T3):** an estimated **165 tests / ≈4.4k lines** more. This is the H class.
- **Together:** ≈6.5 % of the 193,881 test-function lines.

These figures are estimates from a name-based heuristic, not upper or lower
bounds. Some P and H rows will turn out not to move (see the limits below), and
a test can become movable after World code moves out of it.

The census counts each test once. The per-file heuristic in the plan counted 42
P tests. Counting per test finds more of them, because many mostly-composition
files also contain small session-free tests.

### Why W dominates (`reason` column)

| Reason | Tests | What it means |
|---|---:|---|
| `W-world-logic` | 3,047 | Calls `WorldSession` methods that hold World logic. Most are the fixture setup itself: `attach_player_controller_like_cpp`, `adopt_registered_canonical_player_fixture_like_cpp`, `set_canonical_map_manager`. Others are handlers that run World code before or after the domain call. |
| `W-core-only` | 87 | Every session call goes to `wow-world-core`, the hub that stays with composition. |
| `W-fields` | 71 | Only reads or writes `WorldSession` fields. |
| `W-multi-domain` | 36 | Session calls go to two or more domains. |
| `W-unreached` | 22 | The one routed domain does not reach every crate the test names. |
| `W-world-items` | 2 | Domain forwards, plus a `wow-world`-owned item. |

### By destination (tests: P / H)

| Destination | P | H | World host-trait methods (surface a test host must implement) |
|---|---:|---:|---:|
| `wow-world-lifecycle` | 30 | 36 | 17 |
| `wow-world-social` | 18 | 27 | 25 |
| `wow-world-instances` | 0 | 22 | 30 |
| `wow-world-application` | 27 | 14 | 83 |
| `wow-world-character` | 23 | 17 | 81 |
| `wow-world-inventory` | 22 | 14 | 76 |
| `wow-world-spell` | 41 | 5 | 92 |
| `wow-world-entities` | 25 | 4 | 113 |
| `wow-world-loot-ops` | 15 | 10 | 107 |
| `wow-world-quest` | 6 | 13 | 54 |
| `wow-world-core` | 18 | 0 | 4 |
| `wow-world-loot` | 5 | 0 | — |
| `wow-world-interaction` | 0 | 3 | 1 |
| `wow-world-player-conditions` | 1 | 0 | — |
| `wow-world` (stays) | 70 | 0 | — |

The "primary owner" (`owner` column) is the domain crate that most of a test's
symbols resolve to, whatever its class. For W tests it says where the
composition is centred:

| Owner | W tests |
|---|---:|
| spell | 784 |
| inventory | 409 |
| entities | 359 |
| character | 328 |
| quest | 321 |
| loot-ops | 186 |
| social | 185 |
| core | 182 |
| lifecycle | 147 |
| visibility | 113 |
| instances | 109 |
| application | 62 |
| loot | 29 |
| creature | 24 |
| player-conditions | 14 |
| interaction | 5 |
| `wow-world` | 8 |

### T1 list (P with a domain destination)

These are the ledger rows with `class == "P"` and `destination != "wow-world"`: 231 tests.

- **Reasons.** 206 of them are `P-domain`. The other 25 are `P-lowest-covering`: the primary owner does not reach every referenced crate, so the destination is the lowest crate that does.
- **Dev-dependencies.** Three of these rows list `dev_deps`, the dev-dependencies the destination still needs. Five H rows list them too.
- **Largest sources:**
  - `handlers/character_tests/login.rs`: 17 (source-text contract tests of application and character)
  - `battle_pet_purchase/tests/scenarios_1.rs`: 12
  - `handlers/character_tests/{persistence,item_1,spell}.rs`: 11, 10 and 10
  - `spell_acquisition/application/tests/validation.rs`: 9

The 70 P tests that stay are of two kinds:

- **`P-world-items` (37):** handler registration, dispatch tables, and `wow-world` planners.
- **`P-library` (33):** they name no `wow-world*` symbol.

Print the list with:

    python3 tools/architecture/wow_world_test_census.py census --tests | grep '^P wow-world-'

### H candidates

The 165 H rows are T2/T3 candidates. The largest groups are:

| Source | H tests |
|---|---:|
| `account_data/tests/client_state.rs` | 21 |
| `battlegrounds/tests/pvp.rs` | 14 |
| `calendar/tests/calendar.rs` | 14 |
| `scenarios_persistence_1.rs` | 9 |
| `economy/tests/trade/session.rs` | 8 |

Social has the second-largest H group (27 tests) and a host-trait surface of 25
methods. Together these make it a sound T2 pilot. Lifecycle (36 tests, 17 methods)
is the cheapest follow-up. The candidates are not yet proven movable. Each one
still needs the following:

- a test host for the destination crate's host traits, which T2 measures against the ≤10 % host-lines budget;
- a reproduction of the World-built context (`*_test_cx_like_cpp`, `split_*_mut`) that the forward goes through;
- the `world_test_consumer` contract below, for the domains that use it.

## `world_test_consumer` branch inventory

These are production functions that behave differently when a World test fixture
is present. Outside comments, they match one of these:

- `world_test_consumer_like_cpp` (its definition or a call);
- `test_fixtures_ref_like_cpp().is_some()` or `.is_none()` (`fixture-check`);
- `let Some(..) = ..test_fixtures_ref_like_cpp()`, which includes `if let` and let-chains (`fixture-binding`).

 The ledger lists them
per function (`world_test_branches`) and records whether each site exists only
under `cfg(any(test, feature = "test-fixtures"))`.

There are 109 sites in 100 functions. 75 of the sites are test-fixtures-gated.

| Crate | Functions | Sites (definition / consumer / fixture-check / fixture-binding) | Gated sites |
|---|---:|---|---:|
| inventory | 22 | 1 / 11 / 5 / 6 | 15 |
| instances | 17 | 1 / 1 / 16 / 0 | 16 |
| application | 15 | 1 / 7 / 8 / 0 | 9 |
| spell | 15 | 1 / 3 / 11 / 1 | 11 |
| entities | 13 | 0 / 0 / 14 / 0 | 14 |
| quest | 12 | 1 / 11 / 1 / 0 | 4 |
| character | 5 | 1 / 2 / 5 / 0 | 5 |
| social | 1 | 0 / 0 / 1 / 0 | 1 |

**Proposal.** A domain host for T2/T3 must not inherit these branches silently.
Before a domain's tests move, take one of two actions for each of that domain's
sites:

- **Retire it.** The test seeds the canonical Player or map state that production reads.
- **Pin a contract.** The host exposes `test_fixtures_ref_like_cpp()` with the same meaning, and a test proves that both branches are taken where intended.

Social has a single gated site, `set_represented_guild_id_like_cpp`, which keeps
it the safest pilot. The instances and entities sites are almost all gated
`fixture-check` sites:

- **instances:** rest flags, exploration and zone/area updates;
- **entities:** visibility, resurrection and creature-kill paths.

Retire or pin those sites before T3a.

## Method and its limits

The census is name-based Rust scanning, like `session_method_index.py`. It does
not expand macros or resolve types. The rules are in the tool's docstring.

1. Each test gets the closure of its test-side helpers and test-side types. A
   helper is resolved first in the same file, then in the ancestor modules, then
   in the nearest directory, then everywhere.
2. Symbols vote for the `wow-world*` crates that define them. Explicit
   `wow_world_x::`, `use` and `include_str!` paths decide. The session methods a
   test calls add their bodies' votes, one level deep.
3. A session method routes to one domain in two cases: it is a host-trait impl,
   or it is a one-statement forward whose body resolves to that domain.

It errs toward W and toward "stays":

- An ambiguous helper adds every candidate.
- A name defined in several crates splits its vote.
- Any World route makes the test W.

Two rules can err the other way, so treat the H and W-fields rows as candidates:

- **H ignores direct field access.** H looks only at the session *methods* a test
  calls. It does not look at reads and writes of session fields such as
  `session.fixtures` or `session.core`. Those fields are World state that a test
  host would have to reproduce. 11 of the 165 H tests access them in the test
  body, and the review counted about 30 once helpers are included.
- **W-fields hides domain calls.** A W-fields test calls no session method, but it
  may pass the session to a domain function, as in `wow_world_x::f(&mut session, ..)`.
  That shape is the one T2 rewrites onto a host, so some W-fields tests are really
  H-like. 22 of the 71 do this in the test body, and the review counted 32 once
  helpers are included.

Known scope gaps:

- **Integration tests are out of scope.** `crates/wow-world/tests/` (integration
  tests, a separate crate per file) is outside the census and the ratchet.
- **Macro-generated tests are not seen.** A test that a macro generates is invisible,
  and so is one whose attribute a macro emits. The raw-attribute count still catches
  a hand-written `#[test]` the census missed. The test attribute is any path that
  ends in `test`: `#[test]`, `#[tokio::test]`, `#[::tokio::test]` and
  `#[core::prelude::v1::test]`.

Library crates (`wow-data`, `wow-persistence` and others) are not in the symbol
table, so a P destination is a candidate, not a proof. Every T1 move still
compiles the test in its destination crate and keeps the test-identity multiset
(issue acceptance). If that compile rejects a candidate, the census caught a
false P, and the test stays.

## Ratchet

`check` runs in `validation-v2 final` when any of these change:

- `crates/wow-world*`;
- `Cargo.lock`;
- the ledger or the ceiling;
- the tool or its scanner;
- `session_method_index.py` or `hotspot_metrics.py`.

The tool's unit suite also runs when the tool or those helpers change. `check` fails if:

- the ledger differs from the tree;
- a `unit_tests` source is unmounted;
- a test-visible source is mounted from outside `src/` and `unit_tests/`;
- the test attributes in every `crates/wow-world/**/*.rs` (outside `tests/`) differ from the mounted test functions;
- the live suite exceeds the newest reviewed ceiling entry;
- the newest entry is a `raise` and the live suite falls below it.

The T0 baseline entry is:

| Metric | Value |
|---|---:|
| `unit_tests_lines` | 232,745 |
| `unit_tests_tests` | 3,709 |
| `src_tests` | 22 |

How the ceiling changes:

- **Moving or deleting tests passes.** The check then suggests a `tighten` entry.
  The exception is when the newest entry is a `raise`: a raise records the
  reviewed suite exactly, so you must also record the reduction:
  `census --write --tighten --issue N --reason "..."`.
- **Adding a test takes one command:**
  `python3 tools/architecture/wow_world_test_census.py census --write --raise --issue N --reason "..."`.
  It regenerates the ledger and appends a `raise` entry at exactly the live
  metrics. Review the ledger's class and destination delta and the new entry in
  the PR. `effective_ceiling` rejects a growth not marked `raise` and a raise that
  raises nothing.
