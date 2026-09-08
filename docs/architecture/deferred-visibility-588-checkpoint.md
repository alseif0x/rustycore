# Deferred player visibility publication — #588

Approved responsibility, 2026-09-08. Parent #584; integration baseline `8c47af95`.
#585 remains closed. #587 is a separate implemented acquisition boundary awaiting
this dependency and its remaining live acceptance. This checkpoint owns #588,
not another global architecture plan.

## Trigger and contract

The same normalized trainer fixture passed C++ purchase, repeat rejection,
save/relogin and six retained projections, but Rust stopped before purchase:
the stationary player acknowledged active-mover initialization without receiving
the trainer CREATE. Existing Rust explicitly suppressed the notify in the ACK,
and the map's player relocation plans never reached Session publication.

The ACK now updates readiness/transport time and marks the canonical in-world
Player. The existing map update, after its move-list drains, selects expired
active-grid work before resetting notifies. MapManager retains one opaque intent
per canonical owner, then exports only actually updated maps after updater wait
and before delayed removal. The world-server tick carries those intents even
when its independent respawn-condition timer has not expired, and delivers them
after releasing the manager guard.

PlayerHandle identifies an incarnation; a separate checked residence revision
advances only on successful attachment. Detach/retirement and an away-and-back
attachment invalidate old work. A viewpoint is validated as the currently bound,
in-world object. An A→B→A viewpoint change permits a current recomputation of A:
the intent contains no historical candidate set or packet bytes.

The directory validates canonical admission and retains the obligation in one
coalesced slot on the existing durable mailbox rail. The existing Session pump
drains it before visibility-gated packets. General-queue saturation cannot drop
it; disconnected entries reject it. Map delivery is one synchronous producer,
with no spawned retry tasks or await between selections. The Session rechecks
its own handle, residence, viewpoint and login/disconnect state before invoking
the existing canonical scanner. No map guard spans packet publication or await.
The canonical scanner path does not suspend or load from the DB; it diffs the
real shared client ledger and sends UPDATE_OBJECT before initial creature packets.

## Ownership and reference boundaries

- `wow-map/manager/player_owner/visibility`: map selection and canonical identity.
- `world-server/runtime/deferred_visibility` and `tick_summary`: owned delivery
  after the existing map phase, independent of respawn cadence.
- `wow-world/session/directory/deferred_visibility`, mailbox and Session adapter:
  retained addressing, admission and actual client publication.
- Existing generic entry/exit refreshes retain their separate callers. They are
  not used as an untyped fallback for this map-selected obligation.

C++ source under `/home/server/woltk-trinity-legacy`: `MovementHandler.cpp:808`,
`Player.cpp:23045,23322`, `Maps/Map.cpp:830`, and
`Grids/Notifiers/GridNotifiers.cpp:30,137,217`. C++ is a behavioral reference,
supplemented by the paired capture; neither C++ nor current Rust proves itself.
This is an intentional behavior completion, not a behavior-preserving refactor.

The live map plan's empty previous-client set is never treated as authoritative.
Nearby reciprocal recipients can receive their own current-ledger refresh; this
represented adaptation may include other currently pending changes in that view.
Historical out-of-range reciprocal recipients absent from the visited set,
creature/AI relocation actions, and full transport/player initial-packet parity
are not established by the trainer scenario. Their remaining full-port work
stays under #584 and existing gameplay owners, not hidden as #588 evidence.

## Acceptance state

Local implementation, focused acceptance and committed-candidate final validation
are complete. Paired runtime acceptance remains pending. Final at
`328b721f9b96c68a03c161e60118ebaca755c492` passed nine commands and 5,094 tests
(two ignored, zero failed) in 822 seconds; the worktree was clean and Cargo used
one job. `/tmp/rustycore-588-final-manifest.json` was independently verified green;
log `/tmp/rustycore-588-final.log`. The actual focused-test
working tree was baseline `8c47af95` plus 19 changed Rust inputs, SHA-256
`8afc15595300131ad7f48798b7652d4fa202b26dbc1438dfead9a2d1e8282ec9`
(sorted relative paths, NUL, file contents, NUL). Do not relabel that as a later
commit run. Host: aarch64; Cargo used one job and checks ran sequentially.

- `cargo test --locked --offline --lib -p wow-map -p wow-world -p world-server
  visibility`: **97 pass** (31 map, 64 Session/world, 2 server composition),
  including 15 new map cases, 10 new Session cases and the new production
  tick/directory case. Log `/tmp/rustycore-588-visibility-acceptance.log`.
  Earlier focused runs passed 14 map and 10 Session tests. The first Session
  compilation failed on an incorrect path mount and fixture map-ID width;
  both were corrected. An initial `--bin world-server` invocation ran zero
  tests and establishes compilation only; the real composition target is `--lib`.
- `session-ownership-check check --syntax-only`: **PASS**, 283 production and
  433 fixture Session fields unchanged, 39 command variants, 592 direct-registry
  rows. The reviewed syntax delta is one typed command/consumer, one test
  constructor, two registry delivery references and two existing runtime bridge
  fingerprints. Existing array order and unrelated baseline entries are retained.
  Log `/tmp/rustycore-588-ownership-2.log`.
- `check_architecture.py check` and `self-test`: **PASS**, including six existing
  clocks, dependency edges, reviewed logical ceilings and physical migration.
  Logs `/tmp/rustycore-588-architecture-5.log` and
  `/tmp/rustycore-588-architecture-self-test.log`.
- Rendering the persistence policy from the checked snapshot matches the checked
  policy exactly. Snapshot, workflows, semantic policy and opcode TSV are byte
  identical to `8c47af95`. No exhaustive source persistence inventory was
  recomputed; no DB boundary or opcode registration changed.
- `cargo fmt --all -- --check` and `git diff --check`: **PASS**.
- `physical-files --terminal`: **FAIL on 100 retained migrations** under #584,
  not a full-core closeout. No baseline was regenerated to conceal them.

| Shared physical file | Before | After | Retained responsibility |
|---|---:|---:|---|
| `wow-world/src/session/mod.rs` | 76,584 | 76,560 | #584 C2/C4 gameplay and root decomposition |
| `wow-world/src/session/directory.rs` | 2,363 | 2,335 | #584 C4 remaining directory families |
| `world-server/src/runtime/map.rs` | 2,109 | 2,063 | #584 C4 remaining map/respawn composition |
| `wow-map/src/manager.rs` | 4,585 | 4,594 | Nine inseparable update-phase wiring lines; remaining manager split at #584 C4 |

New private production files are 15–170 lines; the two main new test files are
509 and 566 lines, with the server composition fixture at 164. Logical Session
attribution grows by 128 production/568 test lines for this required adapter and
its complete contract tests; world-server changes by -20 production/+128 test.
These explained, reviewed ceilings retain the named #584 exits. Touching the
shared callers does not transfer their unrelated remaining families into #588.

Remaining acceptance: paired stationary
trainer purchase/rejection/save/relogin captures. Earlier #587 green checks apply
only to their unchanged inputs; they do not validate #588.

The original #587 checkout and its unrelated `lfg-343-audit.md` remain preserved.
MiMo is outside the active workflow. Map identity and Session test work use the
existing Astra/high collaborator after a Luna spawn was rejected by the thread
limit; parent owns integration and the single sequential acceptance campaign.
