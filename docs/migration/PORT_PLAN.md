# RustyCore — Master port and delivery plan

**Reconciled 2026-09-13 under #584 / #787 / #748 / [master index #49](https://github.com/alseif0x/rustycore/issues/49).**
Source baseline for this reconciliation: `3.4.3` at
`4e3ad8f010176a6d500cf45c84ea752f179a47cf` (PR #834; the earlier `1143ed41`, `a9623787`, `276e3981`, `d934451a`, `7bb9a911`, `16303cc7`, `62c1369f`, `db125076`, `a3e97063`, `a96ee548`, `76a05081`,
`886e13ad`,
`5d8c079a` and `ebc3b3eb` references remain historical evidence for the issue inventory).
Initial inventory: **46 open issues**, all given a disposition below; #748 is this
bounded planning delivery. Administrative consolidation does not count as implementation.

The target remains **full functional parity with the TrinityCore-derived WoW 3.4.3
server**, with the approved native/Wasm module product. A playable milestone is an
intermediate acceptance point, not a smaller replacement target.

## 1. Direction from here

**#743 (group state application/reconciliation), #735 (reputation encapsulation) and
#787 (World/Map session-phase coordination) are delivered and accepted within their
recorded scopes.** P3.8 and P3.9 are integrated bounded visibility corrections
after P3.1–P3.7. P3.7 closes the measured Creature relocation
fanout gap, while P3.8 marks recipients for object admission/removal through the same
deferred Player-session rail. P3.1 retired the discarded canonical Creature writer,
P3.2 delivered canonical `Map::SendObjectUpdates` publication, and P3.3 now moves
`ProcessRespawns`/`UpdateSpawnGroupConditions` before object visitation while retaining
the legacy Creature owner. P3.3 and P3.4 are integrated: the
production map tick consumes one nearby-cell/source plan for the represented
`ObjectUpdater` families instead of scanning whole typed stores. The legacy Creature
writer and its complete effect consumer remain outside that structural cut. P3.5 now
uses each source's C++ activation radius, including Creature/Pet `m_SightDistance`
for inactive sources; P3.6 now applies the Player cinematic instance-distance override
once its represented camera cursor is active. P3.7 is the bounded fanout delivery from
`CreatureRelocationVisibilityPlan.player_visibility_updates` into the existing deferred
Player-session rail; P3.8 extends that rail to `Map::AddToMap`/`RemoveFromMap` lifecycle
recipient marking without delivering under a map mutation; P3.9 publishes directed
ordinary Creature DESTROY after map removal through that rail with map-incarnation
and `HaveAtClient` fences. Neither slice migrates the legacy Creature owner. The next primary
implementation is selected after a fresh audit of the remaining measured P2/P3/P4
residuals; no historical queue is implied.
No old issue is reopened and no residual is promoted to implementation merely from a
textual inventory.

The P2 item-modifier writer residual is also integrated by PR #816: the generic
`&mut PlayerItemBonusStateLikeCpp` closure is retired behind a named
Player-owned resolved-effect operation. Catalog/effect consumers remain in
`wow-world`, and the remaining item work is gameplay or data acceptance rather than
another generic ownership cut. #584 remains open for its other measured C0-C4
residuals; this delivery does not activate #583 or close the core gate.

Continue the remaining core under #584 by complete operations, execution/lifetime
boundaries and physical organization. #582 is closed after its decoder-only delivery;
#486 has its implementation integrated by PR #807 and remains open only for the
action-specific capture/live gate and administration mutations not represented by the
current Rust surface. #524's relation-query order correction is integrated by PR #803;
PR #822/#824/#826/#828/#830/#832/#834 now makes the production WDC4/SQL sequence table-granular through
`SkillLineAbility`, `SkillRaceClassInfo` and `TraitTree`/`SkillLineXTraitTree`, and indexes generic
`TraitSystemID` plus combat class trees for the production session authority, and rejects
invalid persisted node-entry ranks at login. The bounded projection, official/custom overlays,
WDC4 table-hash ownership and final `RecordRemoved`
filtering are implemented through PR #826/#828; Generic and Combat config validation are
integrated by PR #830/#832. Physically reconciled by PR #811, the issue remains open for
complete `TraitMgr` authority acceptance, remaining cross-store coverage and startup/live
evidence. In parallel with safe independent work, prepare
a playable circuit: effective equipment/stats → combat and death/recovery →
quests/loot/interactions → complete class kit, travel and durable services → soak.

The core/module acceptance chain remains **required #584 core → #583 → #153**.
The former umbrella #133 is already closed. Its closure is neither evidence that this
chain passed nor an extra future task. The module product need not block every
independent gameplay delivery; its production activation still requires its actual
core prerequisites. Complete the approved architecture/module acceptance before
declaring the whole Part-1 program accepted at #47.

The M0–M6 headings remain milestone identifiers; obsolete `[NN]` priority prefixes
are retired. Neither is an unconditional execution order. Issue numbers, crate names and file counts
do not define dependencies. The tables below distinguish preferred order from
capabilities that actually block a consumer.

## 2. Authority, evidence and limits

- This file and #49 own overall direction and issue allocation.
  [STATE.md](STATE.md) owns dated implementation/evidence status.
- [The refactor completion plan](../architecture/refactor-completion-plan.md)
  owns the detailed P0–P6 continuation; [modularity/ECS design](../architecture/modularity-and-ecs-plan.md)
  and [module design](../architecture/module-design-guidelines.md) own contracts and budgets.
  Issue bodies define bounded delivery and acceptance, not competing master plans.
- Required base behavior comes from target C++ at
  `/home/server/woltk-trinity-legacy`, SHA
  `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, and appropriate target-build captures.
  The pinned complementary AzerothCore reference and its limits are in
  [docs/README.md](../README.md); 3.3.5 wire/SQL are not substitutes for 3.4.3.
- This review read every open issue and traced representative current production
  paths, relevant C++ and existing evidence. It is **not** a full opcode/effect/DB2
  census, a fresh live scenario, a durability proof or a new whole-port parity base.
  Partial source findings and hypotheses are distinguished in the updated issues.
- No historical percentage, count of implemented enums, passing mock or closed
  umbrella proves present capability. Preserve the exact source/runtime identity
  of previous tests and captures; do not relabel them as testing this plan.

## 3. Integrated foundations to reuse

These predecessor issues are closed. Reuse their integrated responsibilities and
inspect their recorded limits when changing a consumer; do not repeat them from an
old unchecked checklist.

| Foundation | Closed issue scope and retained boundary |
| --- | --- |
| Data, terrain, LOS, respawn, persistence and integrity | #14–#20, #52, #60, #62, #64; their bounded delivery does not prove every store/stat/save path. |
| World entry, packet handling and movement/AI | #7–#11, #21–#26, #50, #53, #57, #66. #26 proves its bounded creature cast wire/lifecycle, not full effects or combat AI. |
| Runtime clock cuts and homebind | #28/#371 and #44. They do not complete runtime convergence, all transfers or item-use. |
| Persistence/migration and module foundation | #169/#574/#256 and #228–#231. Reuse SQLx-free contracts, the migration authority and the narrow external login API. |
| Canonical Player, finalization, acquisition, visibility and cast | #578/#585/#587/#588/#589. Preserve residence/incarnation, save fences and metadata; broader gameplay remains open. |
| Recent architecture repair | #716 analyzer; #718 represented quest-reward transaction; #722 named Player operations; #737 item-runtime ownership. Real reward recovery evidence and generic mutation residuals are not discharged by these closures. |

The old high/medium findings in [EXISTING-CODE-DEFECTS.md](EXISTING-CODE-DEFECTS.md)
are leads with dated evidence. Contrast them before making them current blockers;
retain their owning capability even if a diagnosis proves obsolete.

## 4. Execution lanes and dependencies

A lane describes a result and its acceptance. It does not create one PR per row,
force unrelated work to wait, or permit a partially completed macro to be closed.

| Lane | Preferred work | Entry / exit contract |
| --- | --- | --- |
| **A — Core architecture** | Measured P2/P3/P4 residuals under #584; #743, #735 and #787 delivered | One canonical authority and execution owner, complete consumers, explicit lifetime/persistence/publication and terminal physical/dependency dispositions. See §6. |
| **F1 — Character foundations** | #61, #63; relevant #12, #486 and #524 corrections | Equipment reaches effective stats; movement and entry consume the integrated owners. Weather or all vehicles do not block unrelated combat. |
| **F2 — Combat and recovery** | #29, residual #30, #31; consolidated #43 and #54 | A real fight can reach death, release/recovery and safe logout. Accept numeric combat only with effective stats and the necessary aura/absorb participants. |
| **F3 — Progression and interaction** | consolidated #41; #55, #56, #13, #36 and #51 | Accept → progress → complete/reward, real loot and GO/item interactions. Shared consumers must compose; queues or recorded requests alone do not pass. |
| **F4 — Complete class/simulation kit** | #32, #33, #34, #35, #27 | Executable effect/aura/proc/AI chains with lifecycle and observable outcomes. A participant needed by F2/F3 is delivered there, not delayed merely because its family is listed here. |
| **F5 — Travel and durable services** | #40/#45, remaining #12; #37, #38, #39 | Flight/instance/reconnect coherence; mail, trade and AH complete their transactional lifecycles. These may advance when their real prerequisites are ready. |
| **X — Stateful extension product** | #583 under #99, then #153 audit | Required core accepted first; real native/Rust-Wasm/C-Wasm/mixed behavior, durable module state/reward and author/operator lifecycle. Wider #99 expansion is separate. |
| **F6 — Part-1 terminal acceptance** | #46, then #47 | Integrated functional scenarios, multi-client soak and real save/restart/relogin under load; no known integrity failures deferred to soak. |
| **O — Environment and QA support** | #255, #351; #279/#352 when applicable | Reproducible setup and usable guarded QA. A specific broken fixture/DNS target blocks its live scenario, not every local implementation. |
| **B — Bounded side delivery** | #582 | Reconcile and integrate the existing decoder branch; it does not activate or complete matchmaking. |
| **D — Optional developer experience** | #260 | Typed configuration foundation while preserving current .conf behavior; not a prerequisite for #231, #583 or gameplay. |
| **L — Full-parity continuation** | #48 and source-evidence index #65 | Preserve all remaining target behavior. Refresh the whole-port plan at #47 before creating a detailed Part-2 child tree. |

### Hard capability relationships

- Equipment → effective Unit/Player stats (#61) precedes accepting damage numbers
  in #29/#31. Analysis and preservation of existing combat paths may proceed earlier.
- #13 and #36 use the shared cast contract delivered by #589 and extended by #30;
  each owns its GO/item admission and side effects. They do not clone cast ownership.
- #31 supplies damage/heal application to periodic/proc consumers; required aura,
  absorb or trigger participants are integrated with the affected operation.
  These are shared capability contracts, not a circular demand that all of #31
  and all of #32 must each close before the other can start.
- #43 supplies the complete death/recovery cycle used by combat, instance and
  resurrection scenarios. #54 extends logout admission and reuses #585 finalization.
- #41/#55/#56 share objective-credit integration, with one owner for each
  transition. #743 supplies group-state consistency; #51 retains protocol/lifecycle
  acceptance. Neither relation justifies merging unrelated responsibilities.
- The reward-by-mail criterion of #41 and auction delivery in #39 require the
  relevant durable mail capability of #37. Quests without mail may be developed
  and tested earlier; **#41 must not close while its required mail criterion is
  missing**. Schedule that producer before final quest closeout. Trade #38 does
  not require AH or mail.
- #583 waits for required #584 core; #153 waits for accepted #584 and #583.
  #99 is a product umbrella, not an implementation prerequisite that must close.
- #46 starts after the selected functional flows work; #47 requires their complete
  Part-1 acceptance and #46. Failures found while building an operation are repaired
  there rather than being stored until F6.

## 5. Complete disposition of the initial open issues

“Revalidate” means a scoped current-source/behavior check at the start of delivery,
not another project-wide audit. A consolidation is closed as superseded, with all
acceptance retained by the recipient; it does not mark functionality complete.

| Issue | Primary owner / lane | Disposition and concrete next scope |
| --- | --- | --- |
| [#12](https://github.com/alseif0x/rustycore/issues/12) | F1/F5, entry/transitions | Revalidate transport attachment/re-seat and weather/world-state presentation; transport columns are already loaded. Track remaining entry/cinematic/hotfix presentation findings here when reproduced. |
| [#13](https://github.com/alseif0x/rustycore/issues/13) | F3, GO-use | Integrate the real spell/portal operation from existing GO dispatch and effective templates/scripts. |
| [#27](https://github.com/alseif0x/rustycore/issues/27) | F4, creature reactions | Trace Talk/text loading and AI/script consumers; deliver event, locale/range and multi-recipient behavior. |
| [#29](https://github.com/alseif0x/rustycore/issues/29) | F2, melee | Complete calculation/outcomes with equipment stats and both Player/Creature consumers; reuse current timers/owners. |
| [#30](https://github.com/alseif0x/rustycore/issues/30) | F2, cast prerequisites | Complete residual CheckCast, resources/reagents and history/cooldowns around integrated #589. |
| [#31](https://github.com/alseif0x/rustycore/issues/31) | F2, damage/heal | Calculation → canonical application → threat/death/publication, with exact modifier and failure behavior. |
| [#32](https://github.com/alseif0x/rustycore/issues/32) | F4, aura lifecycle | Apply/update/periodic/remove, real modifiers and required persistence; no historical aura-count target. |
| [#33](https://github.com/alseif0x/rustycore/issues/33) | F4, procs | Trace real events/trigger consumers, filters, RNG/chance/PPM/charges and recursion/failure. |
| [#34](https://github.com/alseif0x/rustycore/issues/34) | F4, spell effects | Complete selected class chains with their targets, auras, movement and summons; preserve full-port coverage separately. |
| [#35](https://github.com/alseif0x/rustycore/issues/35) | F4, channels/missiles/areas | Runtime lifecycle and effects, beyond the existing packet shapes; reuse the execution clock. |
| [#36](https://github.com/alseif0x/rustycore/issues/36) | F3, item-use | ItemEffect is loaded; deliver registered item admission, cast, charges/consumption and real effects. |
| [#37](https://github.com/alseif0x/rustycore/issues/37) | F5, mail | Complete online/offline mail, attachments/money/COD and durable recovery; supply reward/AH consumers. |
| [#38](https://github.com/alseif0x/rustycore/issues/38) | F5, trade | Complete two-Player item/gold exchange beyond the existing accepted-state protocol. |
| [#39](https://github.com/alseif0x/rustycore/issues/39) | F5, AH | Complete target-version auction lifecycle and mail delivery, not a response invented for a legacy no-op opcode. |
| [#40](https://github.com/alseif0x/rustycore/issues/40) | F5, taxi | Discovery/route/payment → flight/map transitions → landing/recovery; existing taxi state is not flight execution. |
| [#41](https://github.com/alseif0x/rustycore/issues/41) | F3, quest lifecycle | Receives #58/#59: admission, acceptance/sharing/source items, objectives, completion and full in-scope reward; reuse #718. |
| [#42](https://github.com/alseif0x/rustycore/issues/42) | Superseded by #43 | Transfer death, durability and ghost restrictions into the complete recovery macro; close administratively, not as implemented. |
| [#43](https://github.com/alseif0x/rustycore/issues/43) | F2, death/recovery | Receives #42: death/CORPSE → release/ghost → corpse/graveyard → reclaim/healer/resurrection and relog. |
| [#45](https://github.com/alseif0x/rustycore/issues/45) | F5, instances | Revalidate bind/save/difficulty/reset and actual admission; finish enter/leave/reconnect using existing transfer/finalization. |
| [#46](https://github.com/alseif0x/rustycore/issues/46) | F6, soak | Measured multi-client stability on an identified installed build, after incremental operation acceptance. |
| [#47](https://github.com/alseif0x/rustycore/issues/47) | F6, playable exit | Real save/recovery under load and all required functional flows; triggers the next full-port planning pass. |
| [#48](https://github.com/alseif0x/rustycore/issues/48) | L, full-parity umbrella | Retain complete coverage, remove stale percentages; add explicit social/LFG and authentication/network coverage. |
| [#49](https://github.com/alseif0x/rustycore/issues/49) | Master index | Maintain this direction and complete allocation; not an implementation PR or a future prerequisite to its children. |
| [#51](https://github.com/alseif0x/rustycore/issues/51) | F3, group lifecycle | Revalidate current invite/accept/decline/cancel/leave/disband/category behavior; separate from #743 delivery guarantees. |
| [#54](https://github.com/alseif0x/rustycore/issues/54) | F2, logout admission | Deny/delay/instant/cancel/countdown using the existing durable finalizer. |
| [#55](https://github.com/alseif0x/rustycore/issues/55) | F3, loot | Existing gates are not globally missing; finish actual modifiers/grants/credit and multi-client consistency. |
| [#56](https://github.com/alseif0x/rustycore/issues/56) | F3, area-trigger | Bits, conditions, scripts and tavern paths exist; trace residual explore/BG/corpse/transfer semantics. |
| [#58](https://github.com/alseif0x/rustycore/issues/58) | Superseded by #41 | Preserve timed-active exclusivity and recursive breadcrumb admission as explicit quest criteria. |
| [#59](https://github.com/alseif0x/rustycore/issues/59) | Superseded by #41 | Preserve acceptance/completion/reward participants and evidence, without redoing #718. |
| [#61](https://github.com/alseif0x/rustycore/issues/61) | F1, equipment/stats | Trace existing modifier planning into effective stats and reversibility before accepting combat numbers. |
| [#63](https://github.com/alseif0x/rustycore/issues/63) | F1, movement | Complete residual mover/transport/vehicle/teleport branches; preserve #588 deferred visibility. |
| [#65](https://github.com/alseif0x/rustycore/issues/65) | L, source-evidence index | Correct finding/issue status and retain exact C++ provenance; not a separate implementation queue or fresh count. |
| [#99](https://github.com/alseif0x/rustycore/issues/99) | X, module ecosystem | #583 is the selected stateful product; wider language/WIT/hot-reload proposals remain later capability-led planning. |
| [#153](https://github.com/alseif0x/rustycore/issues/153) | X/A, terminal audit | Audit accepted #584/#583 and their evidence; do not absorb known implementation work or await #133 reopening. |
| [#255](https://github.com/alseif0x/rustycore/issues/255) | O, reproducible bootstrap | Reuse integrated migration/status authority; deliver pinned artifact, cache/offline import and setup diagnostics. |
| [#260](https://github.com/alseif0x/rustycore/issues/260) | D, typed JSON config | Optional bounded foundation on the current toolchain; keep .conf/overlays/environment semantics and offline validation. |
| [#279](https://github.com/alseif0x/rustycore/issues/279) | O, QA credential rotation | Explicit disposable-fixture recovery with DB/file failure handling; create-only provisioning remains the default. |
| [#351](https://github.com/alseif0x/rustycore/issues/351) | O, guarded loot QA | Reconcile current runtime/capture orchestration and chest fixture ownership; preserve restore guarantees and prove the actual smoke. |
| [#352](https://github.com/alseif0x/rustycore/issues/352) | O, realm address operations | Revalidate configured DNS/IP and restart diagnostics. No silent fallback or code change inferred from the historical incident. |
| [#486](https://github.com/alseif0x/rustycore/issues/486) | F1, target identity query | **Implementation integrated by PR #807 (`86a0eb97`).** The canonical cache/connected-target path returns target game/BNet identities; keep open for action-specific packet/live evidence and unrepresented undelete/barber mutation coverage. |
| [#524](https://github.com/alseif0x/rustycore/issues/524) | F1, skill startup order | PR #803 fixed the relation-query order; PR #822/#824 (`7bb9a911`) separates the WDC4/SQL stages, PR #826 (`d934451a`) applies official/custom `SkillLineXTraitTree` overlays, PR #828 (`276e3981`) retains the WDC4 table hash and applies final table-scoped removals, PR #830 (`a9623787`) indexes `TraitSystemID` trees and validates Generic configs, PR #832 (`1143ed41`) indexes `ChrSpecialization` class masks and validates Combat configs, and PR #834 (`4e3ad8f0`) rejects missing or over-ranked persisted node entries, all in production order `SkillLine` → `SkillLineAbility` → `SkillRaceClassInfo` → `TraitTree`/`SkillLineXTraitTree`, preserving fail-before-publication boundaries. The bounded projections remain installed in production sessions; profession, Generic and Combat hydration fail closed without a linked tree or valid node-entry rank. Keep the issue open for complete node topology, cost/condition/loadout consumers, final cross-store orchestration and startup/live evidence. |
| [#582](https://github.com/alseif0x/rustycore/issues/582) | B, existing LFG decoders | **Closed/integrated as `21686375` (PR #797).** Six C++-faithful client decoders; no handler, queue or matchmaking claim. |
| [#583](https://github.com/alseif0x/rustycore/issues/583) | X, stateful native/Wasm | Deliver the preserved M0–M4 product after required core; the external login API and laboratory are insufficient. |
| [#584](https://github.com/alseif0x/rustycore/issues/584) | A, core coordinator | Own remaining P2/P3/P4 and C0–C4 dispositions. PR #816 integrates the P2 item-bonus writer retirement: resolved state application is a named Player-owned operation and the generic `&mut PlayerItemBonusStateLikeCpp` bridge is gone. P3.4–P3.9 are integrated in the current delivery, including directed ordinary Creature DESTROY after map removal with map-incarnation/`HaveAtClient` fences (PR #820, `62c1369f`). The legacy Creature writer, AI/combat, scripts, FlyByCamera, CREATE/Pet/corpse/transport parity and exact capture/live gates remain explicit later boundaries. |
| [#735](https://github.com/alseif0x/rustycore/issues/735) | A, reputation boundary | **Closed/delivered.** Player owns reputation state and named transitions; catalogs, packets and persistence consumers remain outside the domain boundary. |
| [#743](https://github.com/alseif0x/rustycore/issues/743) | A, group consistency | **Closed/delivered.** GroupRegistry remains authoritative and dropped state-bearing commands converge through the session boundary. |
| [#787](https://github.com/alseif0x/rustycore/issues/787) | A, session-phase coordination | **Integrated as `d14a9a67` (PR #792; accepted at `76369bda`).** World runs before Map, phase permits remain live through finalization/retirement, and shutdown/replacement barriers are covered by production-linked tests and guarded login/save/relogin QA. |

**Planning delivery:** [#748](https://github.com/alseif0x/rustycore/issues/748) owns
this documentation/issue reconciliation and its validation. Closing it does not close #49
or any gameplay acceptance. #42/#58/#59 retain their history and redirects to recipients.

## 6. Finish architecture without another endless rewrite

### A1 — Group consistency and reputation (delivered)

#743 and #735 are closed in their bounded scopes. Their retained contracts are the
single GroupRegistry authority with convergence for dropped state-bearing commands,
and Player-owned reputation state with catalog/packet resolution outside the domain.
Do not reopen either as a generic mailbox rewrite or a whole-manager move.

### A2 — Remaining application and persistence boundaries

Use the actual generic Player/item mutation callers in the ownership ledger.
For each complete operation, migrate its rules, readers, writers, persistence and
publication; retire broad access or justify a bounded stable seam under the existing
policy. Replacing a closure with a differently named generic closure is not retirement.
A short canonical access adapter is not automatically an independent gameplay owner.

Preserve #718's represented quest transaction and #585's finalization. Complete the
remaining account/character, save/acknowledgement, unknown-COMMIT, cancellation and
recovery obligations where their promised contract is not discharged. Real DB/restart/
relogin evidence remains necessary; source guards and controlled futures alone are
not a durability claim. Unrelated missing gameplay is allocated to its functional macro.

### A3 — Production execution, lifetime and private storage

Trace startup and the current Session/map/legacy calls, using the dated
[clock/phase trace](../architecture/runtime-clock-phase-trace.md) as a starting point.
The current source still selects GlobalLegacy for its creature path and starts the
canonical map loop; that is not by itself a demonstrated double tick.
The #787 coordination contract is accepted. The P2 item-modifier writer is also
closed as an ownership residual: `PlayerItemModifierRuntimeStateLikeCpp` applies
resolved state through a named operation and Session no longer lends its bonus
record to a generic closure. Reconcile admission, phases/barriers,
one resolution, backpressure and transfer/detach/unload/shutdown before removing
another bridge. P3.3 integrated the finite phase correction: after the admitted map
session pass, run `ProcessRespawns` and `UpdateSpawnGroupConditions` before object
visitors, using map incarnations from the tick plan so a replacement map cannot
inherit work. P3.4 now carries canonical player/viewpoint, represented
far-combat/aura/summon and active-non-player sources into one deduplicated nearby
selection, invokes existing per-object consumers only for selected in-world objects,
preserves the all-transport loop and C++ ordering, and keeps delivery outside map
guards. P3.5 resolves `WorldObject::GetGridActivationRange` from the canonical source:
Players and active objects use the map visibility range, inactive Creatures/Pets use
their `m_SightDistance`, and unsupported or missing records fail closed. P3.6 adds
the Player cinematic override as `max(DEFAULT_VISIBILITY_INSTANCE,
Map::GetVisibilityRange())` once the represented camera cursor is active. The Rust
selector still does not own FlyByCamera lookup or cinematic movement. P3.7 reuses the
same source list and per-source activation radius for relocation marking, then turns
affected Players from `CreatureRelocationVisibilityPlan.player_visibility_updates`
into one coalesced deferred visibility intent per Player. The legacy Creature writer
and its complete effect consumer remain a separate migration boundary. P3.8 now marks
nearby in-world Players from `Map::AddToMap` and `Map::RemoveFromMap` before/while the
source is attached, reusing the same deferred intent rail; this supplies recipient
selection without packet delivery under a map mutation. P3.9 now captures ordinary
Creature DESTROY recipients at removal and publishes one typed command per current
Session after the map guard, fenced by map incarnation and `HaveAtClient`; CREATE,
Pet/corpse/transport, transport fanout and live capture remain separate acceptance
gates.

Keep the selected private hecs direction and finite V2 conformance evidence.
Integrate it only with real owners/consumers and the lifetime/reentry contract;
no global ECS, public raw storage API, extra runtime clock or dependency-only “migration.”
P3.1 and P3.2 are integrated bounded contracts: the Creature owner is explicit and
the discarded canonical Creature plan is skipped under the legacy/session owner;
canonical `SendObjectUpdates` snapshots are published after guards are released.
P3.3 preserves those owners while correcting respawn/condition phase order and the
admitted-incarnation boundary. P3.4 is a delivered selection/phase correction, not
Creature AI migration: it retires the map-wide typed-store scans in the production
path, covers nearby inclusion and out-of-cell exclusion, and leaves a measurable
retirement path for the remaining unrepresented C++ sources. P3.5 closes the
source-specific Creature/Pet activation-radius mismatch without moving the AI owner.
P3.6 is a selection correction only; P3.7 is a visibility-intent fanout correction
only. Neither promotes cinematic movement, script dispatch, directed CREATE/DESTROY
publication, or Creature AI into the current macro. Define later migration contracts
from traced consumers under #584, never as a speculative issue per bridge or crate.

### A4 — Physical and dependency closeout

Apply the same semantic/physical policy to production, tests, fixtures, adapters,
composition and tools. Replace arbitrary numerical file partitions with cohesive
responsibilities when closing their family; do not conduct a blind global rename.
Private modules precede earned crates. Preserve visibility, exact registration sets
and persistence inventories while retiring the real legacy accesses.

Migration non-growth is not terminal acceptance. Measure remaining oversized files,
logical owners, permitted dependency exceptions and production bridges at closeout;
every residual needs the policy's specific accepted disposition. #153 audits that
result; it is not the implementation owner of known splits.

## 7. Part 1 acceptance — a complete playable circuit

The M0–M6 names remain milestone identifiers. Their broad exits cannot be inferred
from a child issue's closure or from a single working spell/class.

| Milestone | Required integrated outcome |
| --- | --- |
| M0 foundations | Effective data/stats, terrain/LOS/pathing, respawn and persistence/integrity prerequisites used by the selected scenarios. Preserve accepted predecessor coverage and identify residuals. |
| M1 entry | Create/login, bags/UI and correct identity/initial publication, with no client/Lua failures and scoped capture-clean entry flows; #12/#486 own relevant remaining paths. |
| M2 world | Movement, visibility, patrol/path/aggro/evade, abilities, reactions and respawn; multiple clients agree. Bounded #26/#28 acceptance is not this full exit. |
| M3 combat/classes | Normal class rotations, melee/spell outcomes, resources/cooldowns, periodic effects/CC/procs and required channels/areas, with effective stats and correct death/recovery consequences. |
| M4 interaction | GO/item-use, common quest objective and reward types, groups/loot, mail/trade/AH and flights work end-to-end. Earlier quest development does not remove commercial services from this exit. |
| M5 lifecycle | Death/release/corpse/resurrection, homebind, transport/weather/world state, instances and deny/delay/cancel logout; reconnect/relogin preserve the correct outcome. |
| M6 stability | Multi-session/map soak and actual periodic-save/recovery under load; no lost acknowledged state, duplicate grant, known unresolved integrity defect or unbounded queue failure. |

At #47, record a concrete scenario matrix with target client/build/data, representative
class/race/level/map coverage and every required milestone exit. One successful class
or login is evidence for that case, not all Part 1. Preserve wider target coverage for
Part 2; any scope adjustment needs an explicit retained owner, not a smaller headline.

The environment for required QA must be reproducible and its addresses/fixtures usable.
That requirement is not a claim that all optional authoring tooling (#260), the full
#99 ecosystem or a production LFG queue must be finished to call Part 1 playable.

## 8. Part 2 — full target parity, with an explicit coverage map

#48 retains the complete target beyond the playable cases. The old numeric
baselines (385/631 handlers, 42/150 effects, 5/255 auras, 110/325 stores and script LOC)
are historical and are retired as current status or fixed acceptance denominators.
Measure the target-version inventory and real consumers when the relevant domain is
audited. A registered opcode, represented request or loaded table alone is not coverage.

| Ledger | Required target coverage |
| --- | --- |
| L1 | Client packet handlers, metadata/admission, operation and errors. |
| L2 | Server packet layouts, values, recipients, connection and observable order. |
| L3 | Spell effects and their complete application chains. |
| L4 | Aura types, periodicity, stacking/removal, modifiers and proc interactions. |
| L5 | Required DB2/DBC/GameTable catalogs, effective overlays/removals and consumers. |
| L6 | Creature AI families and lifecycle, beyond bounded combat slices. |
| L7 | SmartAI event/action/target execution and script integration. |
| L8 | Movement generators and transport/taxi/vehicle integration. |
| L9 | Terrain, pathfinding, LOS, collision and required extraction/data support. |
| L10 | Conditions and their actual operation consumers. |
| L11 | Phasing and visibility refresh across state/lifecycle changes. |
| L12 | Complete item/inventory/bank/equipment/durability/buyback/gift behavior. |
| L13 | Complete quest rules, objective types, rewards and persistent state. |
| L14 | Mail, calendar, petitions and their delivery/lifecycle. |
| L15 | Target-version auction house and associated behaviors. |
| L16 | Battlegrounds, arenas, battlefields and outdoor PvP. |
| L17 | Instances, save/bind/difficulty/lockouts/resets and raids. |
| L18 | Achievements, reputation, skills, talents/glyphs, titles and progression. |
| L19 | Pets, vehicles, totems and related AI/state/lifetime. |
| L20 | Required first-party content scripts by family, independently of optional modules. |
| L21 | Warden/anticheat and target-supported enforcement. |
| L22 | UpdateField values and derived state, not only layouts. |
| L23 | Supported server configuration and effective runtime consumers. |
| L24 | Database statements/loaders, schemas, transactions and recovery. |
| L25 | Remaining production runtime, grids, visibility/object updates and legacy retirement. |
| L26 | Source-reference verification, including the historical #65 findings. |
| **L27** | **Explicit social coverage:** groups/raids, guilds, friends/ignores/channels and automatic LFG, beyond #51/#582. |
| **L28** | **Explicit authentication/network coverage:** bnet/world account and realm flows, session/connection lifecycle and target-required security/protocol behavior. |

L27/L28 make previously implicit coverage visible; they do not create child issue
trees or promise unrelated platform features. The map is a planning coverage aid,
not proof that every target operation has already been inventoried.

### Part 2 transition gate

This review establishes the full direction **now**. After #47/M6.2, refresh the
whole-port inventory against that later integration and versioned C++/data/captures,
then decompose the remaining L-ledgers into complete implementation macros.
Do not create hundreds of speculative child issues today. Existing justified
functional work may proceed earlier through its actual dependencies.

The owner closing #47 records the new review base and hands the remaining scope
to #48/#49. Reconcile delivered behavior and evidence before calculating any coverage
denominator or proposing a first Part-2 implementation.

## 9. Delivery, validation and maintenance

1. Select a ready complete responsibility from this plan. Reuse its issue/branch;
   declare exact operation, current consumers, source/data contract, dependencies,
   retirement list and acceptance. Distinguish unknowns from established gaps.
2. Finish the implementation, consumers and tests with coherent internal commits.
   Keep behavioral repair separate from structural movement. No issue/PR per helper,
   routine approval round or partially implemented macro passed off as complete.
3. At delivery acceptance, run affected unit/production-linked/failure tests,
   architecture/metadata checks and the applicable final profile. The parent or one
   assigned executor schedules heavyweight validation sequentially on the shared host.
4. Changed bytes/metadata/connection/order need scoped packet/capture evidence;
   new action-specific live scenarios differ from regression goldens. Randomized
   combat needs controlled inputs/RNG or justified distribution/causal checks.
   Real durability claims need real DB/restart/relogin evidence.
5. Update the owning issue/checkpoint and current status with exact tested SHA,
   command, host, result and limits. Remove superseded accesses/baselines only from
   reviewed semantic evidence. Preserve prior evidence identities and user work.
6. Close an implementation issue only after integrated scoped acceptance. Close
   superseded tracking issues administratively with full scope transferred and
   backlinks. Do not close #49 merely because a planning PR lands.
7. Reconcile this index after a macro lands, a dependency changes or a concrete new
   defect alters priority. Do not append another contradictory “current plan.”
   Urgent integrity faults move ahead of preferences; an unverified historical
   diagnosis does not silently become a new global blocker.

Use [AGENTS.md](../../AGENTS.md) and [validation-v2](../operations/validation-v2.md)
for the actual commands, authority and evidence reuse. Planning changes create no
new runtime result. Report **implemented**, **integrated** and **parity-proven**
separately; do not replace acceptance with percentages of files, fields or closed issues.
