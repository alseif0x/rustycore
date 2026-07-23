# RustyCore — Port Plan (two-part: Playable → Full 1:1)

**Date:** 2026-06-27 · Supersedes the ordering role of `MIGRATION_ROADMAP.md` and the
planning role of `current-session-handoff.md`. Current state: [STATE.md](STATE.md);
architecture: [adr-runtime-tick-ownership.md](adr-runtime-tick-ownership.md); bugs in
already-shipped code: [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md) (the **D-track** —
fixing what exists, distinct from Part 1/2 which build what's missing).

**How to use the checklists:** every actionable line is a `- [ ]` checkbox. Mark `- [x]`
only when the item meets the §"Definition of done" in STATE.md §5 (live + capture-clean +
exercised + C++-cited). Each milestone/ledger also has a header checkbox = "all its items done".

## Why two parts

Goal = **full functional parity with C++**, nothing dropped. The old plan represented every
C++ row and hit 98% breadth on a server where bags don't open. So the plan has two parts with
**different metrics**, run in this priority order:

- **Part 1 — Playable end-to-end** (critical path, sets order). Metric: milestone burndown M0–M6.
- **Part 2 — Full 1:1 parity** (exhaustive backlog, nothing dropped). Metric: per-domain
  coverage ledgers. Tracked from day one; sequenced after/alongside the Part 1 spine.

Both required. Part 1 makes it a game; Part 2 makes it TrinityCore. **Validation gate for
both = C++ capture diff** (a C++ TrinityCore 3.4.3 server runs on the same DBs — see
`[[cpp-legacy-server-swap]]`).

**Core conversion principle:** most handlers are `represented_*_like_cpp` (validate + record
intent, no mutation — see STATE.md §0). Almost every item below is "convert represented→live
+ capture-validate", not "write from scratch".

---

## Part 0 — Governance fixes (do first, cheap)

- [ ] **P0.1** Adopt [STATE.md](STATE.md) as the single status source; mark `_INDEX.md` /
  `MIGRATION_ROADMAP.md` §3 status columns historical.
- [ ] **P0.2** Freeze `current-session-handoff.md` (1.8 MB append-log); replace with a short
  rolling note pointing at STATE.md + PORT_PLAN.md.
- [ ] **P0.3** Fix the stale `AGENTS.md` "Current Checkpoint" (cites `1af9223`, 1402 commits
  behind); re-anchor to audited HEAD; stop citing 96.97% / 98.15%.
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
- [ ] **M1.1** Fix **#7** (CUF profiles → bags don't open).
- [ ] **M1.2** Restore real compression **#8** (one persistent deflate stream per socket; re-enable threshold).
- [ ] **M1.3** Close the 33 login-burst divergences (**#9–#12**, `world-load-audit.md`):
  proficiency set, AccountDataTimes/TutorialFlags resend, ordering, FeatureSystemStatus, MOTD, etc.
- [ ] **M1.4** Fix the CREATE-block UpdateField VALUE gaps (AuraState, DK DisplayPower,
  BoundingRadius, non-mana power, GO ParentRotation/ArtKit — `world-load-audit.md` cross-cutting).
- [ ] **M1 exit:** fresh character logs in, bags open, correct UI, no Lua errors, login burst capture-clean.

### M2 — A world that feels alive
- [ ] **M2.1** Broadcast creature movement: serialize + send `SMSG_MONSTER_MOVE` from computed splines (currently never sent).
- [ ] **M2.2** Wire `MotionMaster::update()` into the runtime tick; route generators through the priority stack (not inline).
- [ ] **M2.3** Connect creature movement generators (random/waypoint) + load waypoint paths.
- [ ] **M2.4** Query the Detour navmesh (`find_path`) instead of straight-line fallback.
- [ ] **M2.5** Real threat: generate threat from damage/heal/taunt; target switch; aggro range by level diff; leash/evade home; call-for-help.
- [ ] **M2.6** Creature spell casting in combat (from `creature_template` spell list; cooldowns).
- [ ] **M2.7** Creature reactions: on-aggro/death/evade `creature_text` emotes/yells/sounds.
- [ ] **M2.8** Formalize runtime owner per ADR (single-owner, no double resolution; respect `Map::Update` phase order).
- [ ] **M2 exit:** creatures patrol, path around walls, fight back with abilities, speak, respawn; two clients see identical state.

### M3 — Combat & spells (leveling-grade)
> D-track: M3 is also where the **existing melee is fixed** — today it has no damage formula,
> no hit table, no armor mitigation (D-H1/D-H2/D-H3); "melee works" only means it deals
> *some* number and broadcasts.
- [ ] **M3.0 (D-track)** Real melee math: weapon-damage + AP scaling, `CalcArmorReducedDamage`, level reduction, hit table (miss/dodge/parry/block/glancing/crit), dual-wield penalty, haste cap (D-H1/D-H2/D-M3/D-M4).
- [ ] **M3.1** Spell cast prerequisites: power/mana cost deduction, cast time (DB2), GCD + category cooldowns, range/LOS/facing, reagents.
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
> granularity would create hundreds of issues that rot before they're touched). The C#-purge (L26)
> rides along: re-anchor remaining C# comments as each domain is contrasted. Trigger owner: whoever
> closes #47 (M6.2) opens the Part-2 planning pass.

- [ ] **L1 CMSG handlers** — 385 / 631 implemented (61%) → 631/631. (`cpp-server-opcodes.tsv`)
- [ ] **L2 SMSG fidelity** — partial → all capture-clean. (`r3-opcodes-registry.tsv`)
- [ ] **L3 Spell effects** — ~42 / 150 → 150. (`spells-effects.md`)
- [ ] **L4 Aura types (incl. periodic/proc)** — ~5 / ~255 → all. (C++ `SpellAuraEffects`)
- [ ] **L5 DBC/DB2 stores** — ~110 / ~325 (34%) → all needed. (`cpp-db2-stores.tsv`)
- [ ] **L6 Creature AI families** — selection-only → AggressorAI/CombatAI/Guard/Passive/Critter/Turret/Vehicle real behavior.
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
- [ ] **L26 C#-reference purge** — re-anchor code + docs to C++. Source: `docs/audits/csharp-reference-audit.md` + `csharp-reference-contrast.md` (51 `#CSharpAudit.*`; tracked as issues #50–#64, index #65). Code still cites C# as authority (wow-packet/wow-network/wow-crypto/bnet-server/wow-data); contaminated docs include world-entry/phase4/vendor/template material. Per AGENTS.md "Reference Priority": contrast vs C++, re-anchor comment or file a behavior bug. **Remaining behavioral findings include:** CHARACTER.1, CHARACTER.2 (→ M5.5), AREATRIGGER.1 (→ M4.7), `guid.rs` HasEntry, QUESTPKT.1/2/3 + QUESTREWARD.1, and the wider ITEMSTATS/MOVEMENT/QUEST findings. ITEM.2/#52 and DATASTATS.1/#60 are merged. SKILL.1/2/3/#62 is implemented on its local branch with C++ `LearnDefaultSkills`/`LearnSkillRewardedSpells`, real WDC4 IDs, live login QA and exact 43-spell semantic capture parity. These bounded slices do not close L26 or their wider gameplay ledgers.

**Process:** drive each ledger to 100% under STATE.md §5. The R8 inventory TSVs feed these
ledgers but are **re-anchored** — a row counts only when live + capture-clean.

---

## Progress metric (replaces the 98% headline)

Report three things, never one blended number:
1. **Part 1:** M-milestone burndown (e.g. "M0 3/6, M1 done").
2. **Part 2:** per-domain ledger %s (e.g. "CMSG 61%, spell effects 28%, aura types 2%, scripts 0%").
3. **Validation:** count of capture-diff-clean flows.
