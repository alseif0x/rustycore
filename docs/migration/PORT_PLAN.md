# RustyCore — Port Plan (two-part: Playable → Full 1:1)

**Plan established:** 2026-06-27 · **Execution review:** 2026-09-05 at local HEAD `93e4002a`.
Supersedes the ordering role of `MIGRATION_ROADMAP.md` and the
planning role of `current-session-handoff.md`. Current state: [STATE.md](STATE.md);
architecture: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md); bugs in
already-shipped code: [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md) (the **D-track** —
fixing what exists, distinct from Part 1/2 which build what's missing).

**How to use the checklists:** mark gameplay capability items `- [x]` only when their stated
scope meets the definition of done in STATE.md §5 (live + capture-clean + exercised + C++-cited).
A bounded completed slice is not the broader milestone exit. Historical diagnoses and unchecked
rows are not current code audits; reconcile the next selected macro with its live issues and HEAD
before implementation. Architecture/tooling work uses its explicit, proportional acceptance.

## Current execution agreement — #133 / #578

The current ownership delivery remains one **macro-issue #578 / draft PR #579**. The subsequent
functional modularity proof is **#583 under #99**, then #153 independently audits both before
#133 closes. Keep coherent internal commits and checkpoints; do not turn
field families into micro-issues/PRs or require permission to continue between routine steps.
The contract-led plan and exact remaining boundaries are maintained in
[`session-578-checkpoint.md`](../architecture/session-578-checkpoint.md), not inferred from the
number of closed historical children, fields moved, or passing tests.

Each internal block must name its complete operation, input/admission contract, canonical owner,
mutation/commit/publication order, narrow dependencies, retired access/bridge and acceptance
evidence. Move every related reader/writer before claiming that boundary complete. A shared
resource bag with fewer outer fields, or gameplay spread over more Session impls, does not meet
the terminal contract. Already-known cuts stay in #578; #153 verifies them rather than absorbing
their implementation. Preserve the full #133 outcome.

The [module design guidelines](../architecture/module-design-guidelines.md) add independent
physical source/test acceptance to each semantic family: manageable files, bounded file-specific
exceptions and legacy retirement inside #578 C2/C4. They include a Rust submodule skeleton and
cover SDK/modules in #583. Safe mechanical splits need not wait for the hecs conformance gate,
which remains mandatory before production storage migration. The existing checker now enforces
physical migration ceilings and a separate terminal mode. Remaining legacy-file retirement and
semantic acceptance belong to #578 C2/C4, not #153 or a new micro-issue; a migration PASS is not
terminal acceptance. The owning checkpoint records the exact remaining ceilings and evidence.

During development, run affected-crate checks, focused positive/negative tests, formatting and
the inexpensive ownership/architecture checks. At an affected owner boundary, exercise bounded
production-path integration and failure cases, including stale generation, detached transfer,
save/logout and publication/backpressure as applicable. The complete exhaustive/final stack
belongs at macro acceptance, not every internal commit. Required capture/runtime evidence remains
an explicit gate, and this cadence grants no new deployment, push or merge authority.

The [reanalysis checkpoints](../architecture/modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication)
make the order explicit: conformance before production storage migration; review the first real
C1/C2 vertical with C0 admission/phase evidence before replicating it; reconcile all C0–C4 at #578
closeout before #583 production integration; audit both merged macros in #153. Review the next
gameplay macro just in time, then the entire port at #47/M6.2 before Part 2 planning. These are
evidence reviews inside the approved macros, not new issues or routine confirmation gates.

After #133, re-audit the next port macro just in time against current Rust, C++ and existing
evidence. Preserve links and hard dependencies from the ordered index while regrouping internal
work around coherent responsibilities, not outdated table/field diagnoses. Report implemented,
integrated and parity-proven separately; retain the full Part-1/Part-2 goal. Do not pre-granulate
#48 or manufacture a new issue tree for every internal cut.

The latest approved [complete modularity/ECS plan](../architecture/modularity-and-ecs-plan.md)
selects private, selective `hecs`, retaining cohesive domain aggregates. Before production storage
migration, #578 runs the finite independent-module conformance proof: a frozen host accepts a
third module/new state type, with equivalent native Rust/Rust Core Wasm/C Core Wasm and mixed
execution cases. The gate can falsify the selection through a concrete backend limitation; it is
not another indefinite candidate comparison. As reconciled on 2026-09-06 at `36d0ccbf`,
the finite gate has passed: see the [V2 evidence](../architecture/modularity-conformance-results.md)
and [owning checkpoint](../architecture/session-578-checkpoint.md). The next work is real-owner
integration and all C0–C4 lifetime/save/phase/operation obligations. No production hecs dependency
has been installed, and no SDK-wide prerequisite or new issue is added.

The following #583 macro delivers shared semantic hooks for native first-party/custom modules
and a bounded Wasm executor with Rust/C bindings, alongside policies, scoped/reentrant encounter
behavior, independent state composition, durable progress/reward and install/update/disable/recovery.
**This explicitly expands #133's closure:** Wasm is optional for the operator to enable, not
optional for #583/#153 acceptance. The bounded delivery no longer waits for M6; broader language
ecosystem expansion retains the fresh #99 planning gate. #583 depends on #231/#578; #578 does not
depend on #583 or a production SDK/Wasm executor. #153 audits both completed macros, not the entire
#99 epic. Native-only and Wasm-enabled builds must preserve the same supported hook contracts;
the plan does not promise every language, a stable native ABI or hot reload.

The [V1 laboratory is complete](../architecture/modularity-lab-results.md) at `ee9a0128`: its
corrected campaign passes the recorded functional/resource gates, but does not prove the new
independent-module/multilanguage gate or production integration. Native remains the default;
the next checkpoint is the specified conformance proof, not repeated V1 timings. This is one
expanded complete capability, not a PR per hook. Part 1, Part 2 and the D-track retain their goals
and hard dependencies; the architecture update is not a fresh audit of every historical gameplay
issue and does not change their completion states or publication/deployment approvals.

## Why two parts

Goal = **full functional parity with C++**, nothing dropped. The old plan represented every
C++ row and hit 98% breadth on a server where bags don't open. So the plan has two parts with
**different metrics**, run in this priority order:

- **Part 1 — Playable end-to-end** (critical path, sets order). Metric: milestone burndown M0–M6.
- **Part 2 — Full 1:1 parity** (exhaustive backlog, nothing dropped). Metric: per-domain
  coverage ledgers. Tracked from day one; sequenced after/alongside the Part 1 spine.

Both required. Part 1 makes it a game; Part 2 makes it TrinityCore. Functional parity requires
the C++ comparison and live evidence defined in STATE.md §5. A retained capture proves only its
recorded action/build/window. Fresh runtime availability must be checked, not inferred from the
historical server-swap notes. Structural commits use the proportional gates above.

**Core conversion principle:** audit the actual path from request to canonical mutation and
publication. The historical `represented_*_like_cpp` label does not prove that a current function
only records intent, nor that a feature is live. Reuse completed behavior and implement the
remaining contract, rather than rebuilding from old diagnosis text.

---

## Part 0 — Governance contracts and retained implementation debt

- **P0.1 — Active policy:** [STATE.md](STATE.md) owns dated current-state evidence;
  historical migration tables are not current status authorities. Enter through [docs/README.md](../README.md).
- **P0.2 — Active policy:** retain the old handoff through its Git-history pointer;
  update the owning current checkpoint, STATE.md and PORT_PLAN.md instead of another append-log.
- **P0.3 — Active policy:** AGENTS.md points to those authorities and the current workflow;
  documentation changes do not create a new parity-audit base or revive old percentage headlines.
- [ ] **P0.4** Retire `represented-complete` as a closure state in the inventory TSVs.
- [x] **P0.5** (issue #66) Repeatable **capture-diff harness** stood up.
  - `crates/capture-diff/` — parses the C++ PKT 3.1 log + the Rust
    `RUSTYCORE_PACKET_DUMP_DIR` dump, aligns by opcode/direction, reports count/order/value
    divergences. One command: `cargo run -p capture-diff -- diff <flow> [--strict]`. Capture
    scripts in `scripts/`; `import` installs a captured pair as a golden.
  - **Login golden = real capture** (2026-06-28): C++ TrinityCore vs RustyCore, same character,
    trimmed to the login flow, s2c-only, wired as a gated test (`cargo test -p capture-diff`).
    The committed baseline is the *current* real login divergence set (live equivalent of
    `world-load-audit.md`) and shrinks as login parity improves — re-pin with `import`.
  - **Note:** "capture-clean" per STATE.md §5 means *zero* divergences for a flow; login is
    captured and gated but **not yet clean** (the baseline holds the open divergences).

---

## Part 1 — Playable end-to-end (critical path)

### M0 — Simulation foundations (silent subdependencies; unblock M2–M5)
> Discovered in deep audit: without these, "alive world" and "real combat" are built on sand.
- [ ] **M0.1** Load the missing stat/data DB2 stores: `ChrClasses`, `FactionTemplate`,
  `CharBaseInfo`, `SpellPower`, `SpellCastTimes`, `SpellCooldowns`, stat-formula GameTables
  (`GtOCTBaseHP`, `GtCombatRatings`, `GtChanceToMeleeCrit`, …); wire onto sessions/runtime.
- [ ] **M0.2** Implement terrain height query (`GetHeightZ`/ground snap) so spawns/movement sit
  on the ground.
- [ ] **M0.3** Implement VMap line-of-sight (replace the `return true` stub) — gates spells,
  aggro, and pathing.
- [x] **M0.4** Player persistence safety: periodic save timer + one transaction for the
  represented multi-statement save landed in issue #17 / PR #88, with runtime and manual-client
  logout/relog persistence QA. Full C++ save breadth remains Part-2 parity work.
- [ ] **M0.5** Persist respawns to a DB respawn table (survive restart).
- [ ] **M0.6** Establish the **represented→live bridge** convention (a single place where a
  recorded intent is applied to live state), so M2–M5 convert consistently.
- [x] **M0.7 (D-track CRIT: stop data loss/dupe NOW)** — closed by issue #20 and merged PRs
  #89, #103, #105, #107, #109, #111, #113 and #115, plus the already-merged #88 save slice:
  load item enchantments + random properties on relog (**D-C1/D-C2**), persist bank/
  equipment-sets/void-storage (**D-C3/M6/M7**), wrap inventory swap + player save in
  transactions (**D-C4/D-C7**), close loot item/money TOCTOU dupe (**D-C5/D-C6**), make vendor
  buy + group full-check atomic (**D-C8/D-C9**). Every child merged with its required CI and
  current-HEAD Codex verdict; installed action-specific QA/captures are recorded in
  EXISTING-CODE-DEFECTS.md. HIGH/MED mechanics and the separately documented post-COMMIT crash
  journal boundary remain outside this scoped CRIT closeout.

### M1 — Clean, crash-free world entry
- [x] **M1.1** Fix **#7** (CUF profiles → bags don't open): match C++'s post-add
  `InitWorldStates → LoadCufProfiles → AuraUpdate → PhaseShiftChange` order and pin a live,
  non-empty C++/Rust capture pair.
- [x] **M1.2** Restore real compression **#8**: the `0x400` C++ threshold is active,
  one deflate stream survives the direct-send → async-writer ownership split, and live login QA
  decoded four consecutive large packets through one persistent inflater.
- [ ] **M1.3** Close the 33 login-burst divergences (**#9–#12**, `world-load-audit.md`):
  proficiency set, AccountDataTimes/TutorialFlags resend, ordering, FeatureSystemStatus, MOTD, etc.
  Issues #9–#11 are merged. Issue #12 retains the later movement/order rows and the full
  capture/original-client exit.
- [x] **M1.4** Re-audit and fix the bounded CREATE-block UpdateField VALUE gaps (issue #10,
  `world-load-audit.md` cross-cutting). Five findings were already fixed, player power slots were
  correct, canonical ParentRotation retains a documented architecture follow-up, and merged
  PR #123 closes selected non-mana creature power (including
  hotfix overlays and legacy-to-canonical state) plus runtime GameObject ArtKit.
- [ ] **M1 exit:** fresh character logs in, bags open, correct UI, no Lua errors, login burst capture-clean.

### M2 — A world that feels alive
- [x] **M2.1** Broadcast creature movement (issue #21): the global legacy tick launches
  random/waypoint splines and sends `SMSG_ON_MONSTER_MOVE` to nearby visible clients. PR #77
  supplied installed/original-client validation; issue #21 pins a real C++ compressed-waypoint
  packet and reproduces all 117 body bytes exactly in Rust.
- [x] **M2.2** Wire `MotionMaster::update()` into the runtime tick: every legacy
  `WorldCreature` owns a persistent stack, the global frame advances spline then ticks that stack
  exactly once, and selected random/waypoint execution is interrupted by normal-priority chase.
  The global aggro phase installs chase and emits its C++-shape movement stop in the same tick,
  while a highest-priority point/charge generator remains selected above chase and its
  represented finite lifecycle releases the selector proxy on completion. M2.5 supplies target
  pathing; moving every owner-dependent generator body behind one generic Unit interface remains
  wider movement architecture work.
- [x] **M2.3** Connect creature movement generators (random/waypoint) + load waypoint paths:
  startup already loaded the exact C++ parent/node query shape and current data resolves 7,698
  paths, 142,185 nodes and 5,419 waypoint spawns. The global owner now supplies measured elapsed
  `diff` to spline and generator timers, so scheduler delay cannot make random or waypoint
  re-arming lag behind a finalized leg. Long-horizon random and multi-node waypoint regressions pass;
  an installed bot run received two movement packets while the server published 627 across 327
  visible-work ticks. This does not claim Detour, formation/transport transforms, SmartAI
  callbacks or chase/threat parity.
- [x] **M2.4** Query the Detour navmesh (`find_path`) instead of straight-line fallback: the
  "never invoked" diagnosis was stale — `wow-recastdetour` is a real vendored Detour build and both
  live generators already launched corridors. Four contrasted defects in *what the query returned
  and when it ran* are closed against `PathGenerator.cpp`: the Detour filter is derived per owner
  (`CreateFilter`/`UpdateFilter`) instead of a hardcoded ground-only mask; a missing navmesh/tile
  now yields the C++ `BuildShortcut()` + `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` that callers
  launch, instead of a failure that froze wander in a 100 ms retry loop; `BuildPointPath` runs
  exactly once so a discarded straight pass can no longer leak `PATHFIND_SHORTCUT`/`SHORT` into a
  usable smooth path; and both endpoint tiles are demand-loaded, so a destination one `.mmtile`
  over no longer reports "no navmesh". Shortcut/failure `PathType` values are bit-exact.
  A deterministic ring-around-a-hole navmesh fixture proves the live waypoint tick routes around
  the obstacle with real intermediate points and fails when pathfinding is disabled.
  Four further C++ branches were closed in the same slice: the `CanFly()`/falling mesh-hole and
  far-from-poly shortcuts (`:180-202`, `:221-240`), `UNIT_STATE_IGNORE_PATHFINDING` from
  `flags_extra` (`Creature.cpp:1154-1155`), corridor reuse plus `GetPathPolyByPosition`
  (`:94-123`, `:291-413`), and **live pathing for chase and home** — both were faithful ports with
  no caller, and home previously *teleported* the creature on evade. Both have around-obstacle
  tests that fail with pathfinding disabled. Proven legacy defects are repaired rather than
  copied: an empty suffix retains the complete valid multi-poly prefix (there is no overlap to
  subtract), clamps movement to its reachable boundary, and recalculates a singleton before C++'s
  zero-length tail underflow; a disconnected singleton partial path is likewise clamped instead
  of straight-lining across the gap; and chase stores the computed move-away direction that C++
  reads but never assigns. Chase commits that direction only after a successful spline launch
  and drops the prior corridor before a direction-flip query. The 3D squared `< 3.0f` corridor
  lookup remains C++-faithful. The fail-closed `detour-chase-around-obstacle` flow pins the
  connected MMap/action/provenance contract; its reviewed C++/Rust pair is strict-CLEAN across the
  exact heartbeat → compressed chase spline → ping window (3/3 packets, empty baseline).
  Not claimed: point/charge, fleeing and confused have no live trigger (no fear/confuse aura
  handlers, no live `MovePoint` caller), so their ported generators stay unreachable; also open are
  mutual chase, VMap LOS, the `CanSwim()` mesh-hole halves, raycast/straight-path modes,
  liquid-aware `NormalizePath`, transports/formation/off-mesh links, and per-instance pathfinder
  concurrency.
- [x] **M2.5** Real threat: generate threat from damage/heal/taunt; target switch; aggro range by level diff; leash/evade home; call-for-help.
- [x] **M2.6 bounded wire/lifecycle slice (#26)** — Creature template-spell scheduling and
  publication; not completion of general creature spell execution.
  The bounded CombatAI/TurretAI slice reads template spell slots, schedules supported instant
  casts with C++ cooldown/range/target/visual rules, and publishes an atomic START/GO pair before
  the same-frame melee phase. The final issue-#26 P1 hardening removes GO's unconditional-hit
  assumption: bounded resolution is publishable only for a physical `DmgClass=MELEE` Creature
  spell against a Player attacked from behind, with zero spell/effect mechanics and complete
  Creature/Player source authority proving every omitted source hit-inert. Canonical local aura
  application/modifier/visible containers still must be empty; persisted/login sources may be
  nonempty only when their exact effects are proven neutral to this hit result. Player authority
  fails closed across persistence, login/zone reconciliation, map/area, guild, skills, quests,
  glyphs, active traits, pets/battle-pet slots, FFA/PvP/war mode, SpellArea/outdoor sources, and
  script/legacy/all-rank/SpellLinked hooks. Valid linked hooks and trigger IDs from rejected
  SpellLinked rows both block the candidate.

  A Creature-owned `0..=9_999` roll yields base `MISS` below `500` (5%) and `HIT` otherwise.
  The local order is cast then schedule and hit roll before cooldown; `NO_ATTACK_MISS` consumes
  one hit roll before forcing `HIT`. An accepted HIT publishes its topology, then tombstones
  before scheduling because C++ next consumes unrepresented launch-crit/effect-value draws;
  MISS may retain authority and draw its repeat delay. Spell/melee/movement RNG share a
  fail-closed Creature tombstone. Reaching the unrepresented valid-melee damage/outcome/proc branch sets it and emits
  no fabricated damage or wire. Because C++ uses a process-global RNG and Rust a per-Creature
  RNG, the represented guarantee is distribution and local causal order, not exact global draw
  interleaving. Unaccredited states publish neither START nor GO; event-slot clearing and other
  already-performed deterministic reset work remain, while a tombstone blocks future
  random-dependent scheduling. Final live authority also loads effective specialization hotfixes,
  corrects the external-ID `AreaTable` offsets that resolve Shattrath to Terokkar, and admits exact
  OutdoorPvPTF spell `33377` only after its XP/outgoing-damage auras and runtime hooks prove
  hit-inert. The final C++/Rust Cabal Interrogator/Eviscerate generation was
  recaptured from clean harness HEAD
  `42977e9accb24fc3921af075f4122e1f0180f4a2`. Fixture guard v2 verifies the stock
  `SmartAI`/difficulty-0 flags `0`, CAS-switches only the capture window to
  `CombatAI`/`CREATURE_STATIC_FLAG_NO_MELEE`, and restores the exact `SmartAI`/`0` state. The
  selected pair is an observed **HIT**, strict-CLEAN at 2/2 packets with an empty baseline, and
  `verify-required creature-spell-casting` is CLEAN. It is not proof of deterministic hit. This
  closes only the M2.6 wire/lifecycle slice: spell effects, damage/health mutation, the full Spell
  pipeline, and the other AI families remain later work. The residual functional contract is
  carried by the existing spell pipeline work (#30–#35) and full AI/spell ledgers, not erased by
  closing #26 or by this two-packet proof. Revalidate Creature callers as those capabilities land.
- [ ] **M2.7** Creature reactions: on-aggro/death/evade `creature_text` emotes/yells/sounds.
- [ ] **M2.8 terminal runtime ownership** — #28 closed its bounded writer cut and #371 removed
  the independent creature-local clock. Those closures do not prove full Map/runtime convergence;
  the remaining #578 ownership/phase contract must be demonstrated before this broader item closes.
- [ ] **M2 exit:** creatures patrol, path around walls, fight back with abilities, speak, respawn; two clients see identical state.

### M3 — Combat & spells (leveling-grade)
> D-track: M3 is also where the **existing melee is fixed** — today it has no damage formula,
> no hit table, no armor mitigation (D-H1/D-H2/D-H3); "melee works" only means it deals
> *some* number and broadcasts.
- [ ] **M3.0 (D-track)** Real melee math: weapon-damage + AP scaling, `CalcArmorReducedDamage`, level reduction, hit table (miss/dodge/parry/block/glancing/crit), dual-wield penalty, haste cap (D-H1/D-H2/D-M3/D-M4).
- [ ] **M3.1 / #30** Complete spell cast prerequisites: power/mana cost deduction, cast time
  (DB2), GCD + category cooldowns, range/LOS/facing, reagents. Re-audit the residual: the original
  issue's "no power deduction" and wholly missing-store premises are stale. Current
  `handlers/spell.rs` already checks/deducts canonical Player power, with accepted-cast and
  rejected-cast tests; that does not establish the complete `Spell::CheckCast` contract.
- [ ] **M3.2** Damage/heal calc pipeline: coefficients (SP/AP), crit, school resist, miss/dodge/parry/block, absorb shields (D-H3).
- [ ] **M3.3** Aura **periodic tick** (DoT/HoT) + the gameplay-critical SPELL_AURA_* types (stat mods, %mods, immunities).
- [ ] **M3.4** Proc system (proc flags/chance/charges/ppm → trigger spell).
- [ ] **M3.5** Expand spell effects 42→leveling set: CC (stun/root/fear/silence), charge/leap/knockback, interrupt, dispel, summon/totem, threat mods.
- [ ] **M3.6** Channeled spells, travel-time/missiles, ground-targeted AOE + `DynamicObject` area auras.
- [ ] **M3 exit:** a class levels via its normal rotation; DoTs tick; CC lands; mana matters; capture-clean SPELL_GO/AURA_UPDATE + health/aura deltas.

### M4 — World interaction (convert represented→live)
- [ ] **M4.1** GameObject-use → spell/teleport: **portals (#13)**, chests/loot GOs, quest objects, doors/buttons/levers, summoning.
- [ ] **M4.2** Item-use → cast spell: load `ItemEffect.db2`; wire `CMSG_USE_ITEM` executor (potions, scrolls, trinkets, hearthstone).
- [ ] **M4.3** Mail: register + implement send/list/take-item/take-money/delete/return (+DB mail tables).
- [ ] **M4.4** Player trade: real item/gold transfer with inventory checks.
- [ ] **M4.5** Auction House: list/bid/buy/sell/cancel live (+DB auction tables).
- [ ] **M4.6** Taxi/flight: node discovery + `MoveTaxi` flight-path movement.
- [ ] **M4.7** Quest objective auto-credit (D-track D-H4/H5/H6): wire kill-credit, item-loot, and area-trigger (explore) objectives so quests are completable; apply title/skill/spell rewards live; send queued mail rewards. **Verify D-H4 (kill credit) on a live kill first — agents disagreed whether it already works.**
- [ ] **M4 exit:** portals teleport; potions/hearthstone work; mail, trade, AH, flights usable end-to-end; quests of all common objective types complete.

### M5 — Death, transitions & persistence edges
- [ ] **M5.1** Death→ghost: PLAYER_FLAGS_GHOST, durability loss on death, ghost movement/interaction restrictions.
- [ ] **M5.2** Resurrection/corpse/graveyard: corpse spawn, repop teleport to nearest graveyard, reclaim corpse, spirit healer + res sickness.
- [ ] **M5.3** Hearthstone bind / innkeeper set-home (persist to DB).
- [ ] **M5.4** Login-on-transport re-seat (1230); zone weather (1231); world-state header (1233).
- [ ] **M5.5** Instance enter/leave basics; reconnect/logout integrity.
- [ ] **M5 exit:** all common transitions seamless and persistent.

### M6 — Stability soak

Multi-hour soak is the terminal stability test, not the first time concurrency is exercised.
Ownership-changing macros must already carry bounded integration/failure coverage for the
transitions they change; M6 then tests sustained composition and load.

- [ ] **M6.1** Multi-client multi-hour soak; fix runtime races, channel backpressure, lock-held-sends (ADR risks).
- [ ] **M6.2** Warning cleanup; periodic-save under load verified.
- [ ] **M6 exit:** stable session declared → "playable end-to-end".

---

## Part 2 — Full 1:1 parity (exhaustive backlog, nothing dropped)

Per-domain coverage ledgers. Each: current → target, with "done = live + capture-clean".
Sequenced after/alongside the M0–M6 spine; listed now so the long tail can't fall through.

> 🚦 **Part 2 transition gate (do NOT break this down yet).** Part 2 lives as a single epic
> (GitHub #48) on purpose. **When Part 1 (M0–M6) is essentially done** — "playable end-to-end"
> declared at M6.2 — **run a fresh planning pass**: re-audit HEAD (the live state will have moved
> a lot), then break each L-ledger into PR-sized child issues *at that point* (not now — premature
> granularity would create hundreds of issues that rot before they're touched). Source-reference
> verification (L26) rides along: anchor affected comments to C++ as each domain is contrasted. Trigger owner: whoever
> closes #47 (M6.2) opens the Part-2 planning pass.

- [ ] **L1 CMSG handlers** — 385 / 631 implemented (61%) → 631/631. (`cpp-server-opcodes.tsv`)
- [ ] **L2 SMSG fidelity** — partial → all capture-clean. (`r3-opcodes-registry.tsv`)
- [ ] **L3 Spell effects** — ~42 / 150 → 150. (`spells-effects.md`)
- [ ] **L4 Aura types (incl. periodic/proc)** — ~5 / ~255 → all. (C++ `SpellAuraEffects`)
- [ ] **L5 DBC/DB2 stores** — ~110 / ~325 (34%) → all needed. (`cpp-db2-stores.tsv`)
- [ ] **L6 Creature AI families** — partial (bounded live AggressorAI/CombatAI/TurretAI combat
  slices) → full AggressorAI/CombatAI/Guard/Passive/Critter/Turret/Vehicle behavior.
- [ ] **L7 SmartAI (SMART_SCRIPT)** — recognized-not-interpreted → full event/action/target interpreter. (`ai-smartscripts.md`)
- [ ] **L8 Movement generators** — disconnected → all wired (idle/wander/waypoint/chase/follow/point/flee/charge/taxi/transport).
- [ ] **L9 Pathfinding/terrain/vmap** — stub → full Detour + height + LOS + collision.
- [ ] **L10 Conditions** — default-true → full ConditionMgr. (`conditions.md`)
- [ ] **L11 Phasing** — partial → full PhaseMgr + change refresh. (`phasing.md`)
- [ ] **L12 Items: bank contents DB / reagent bank / equipment-set DB / durability-on-death / buyback / gift** — close the partials.
- [ ] **L13 Quests: skill/expansion gates, talent rewards, POI player-conditions, daily-pool persistence.**
- [ ] **L14 Mail / Calendar / Petitions** — absent → full. (`mails.md`/`calendar.md`/`petitions.md`)
- [ ] **L15 Auction House (+bot)** — represented → full. (`auctionhouse.md`)
- [ ] **L16 Battlegrounds / Arena / Battlefield / OutdoorPvP** — absent → full WotLK set.
- [ ] **L17 Instances** — lock/save/difficulty toggle, raid resets. (`instances.md`)
- [ ] **L18 Achievements / Reputation / Skills / Titles** — partial/stub → full.
- [ ] **L19 Pets / Vehicles / Totems (AI + lifecycle)** — partial → full. (`pets.md`)
- [ ] **L20 Content scripts** — ~0 / 294k LOC C++ → full by family (Commands/Spells/World/continents/raids/events/PvP). (`scripts*.md`)
- [ ] **L21 Warden / anticheat** — constants + sanitization → full enforcement. (`warden.md`)
- [ ] **L22 UpdateField VALUE completeness** — many 0/default → all computed. (`world-load-audit.md`)
- [ ] **L23 Config keys** — partial → full. (`cpp-world-config-registry.tsv`)
- [ ] **L24 DB prepared statements / loaders** — partial → full. (`cpp-sql-prepared.tsv`)
- [ ] **L25 Runtime convergence** — legacy global loop → canonical `Map::Update` owns the tick (incl. `SendObjectUpdates`, per-map visibility range, grid-unload despawn); retire legacy. (ADR steps 5–8)
- [ ] **L26 C++ source-reference verification** — retain issues #50–#64 and index #65 for
  traceability, not as proof that their original analysis was correct. Reproduce each selected
  residual against current Rust and exact C++ call paths; discard unsupported diagnoses without
  reviving completed work. Relevant families include character lifecycle (M5.5), area triggers
  (M4.7), GUID entry semantics, quest packets/rewards, item stats and movement. Preserve the
  independently C++-anchored evidence for #52/#60 and the bounded #62 skill work
  (`Player::LearnDefaultSkills` / `LearnSkillRewardedSpells`); verify actual issue/code state
  before relying on a historical completion label. Those slices do not close L26 or the broader
  gameplay ledgers. The [C++ findings](../audits/cpp-parity-findings.md) retain evidence and limits;
  no inherited analysis or finding count is itself a correctness source.

**Process:** drive each ledger to 100% under STATE.md §5. The R8 inventory TSVs feed these
ledgers but are **re-anchored** — a row counts only when live + capture-clean.

---

## Progress metric (replaces the 98% headline)

Report three things, never one blended number:
1. **Part 1:** M-milestone burndown (e.g. "M0 3/6, M1 done").
2. **Part 2:** per-domain ledger %s (e.g. "CMSG 61%, spell effects 28%, aura types 2%, scripts 0%").
3. **Validation:** count of capture-diff-clean flows.
