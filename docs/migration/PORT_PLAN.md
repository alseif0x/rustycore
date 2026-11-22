# Port scope and delivery plan

Use [#49](https://github.com/alseif0x/rustycore/issues/49) for current milestone dependencies and [STATE.md](STATE.md) for what has landed on this branch. The [architecture plan](../architecture/refactor-completion-plan.md) owns the core, module product and terminal audit sequence. The [September reconciliation](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/PORT_PLAN.md) contains the original dispositions, including closed issues and measured inventories.

The [detailed lane prerequisites and remaining operation contracts](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/migration/PORT_PLAN.md#4-execution-lanes-and-dependencies) supply the technical detail for the milestones below. Check current issue status when selecting a dependency.

## M6.1 money-claim recovery (#46, #1339, #1341)

The integration decision on 2026-11-22 at `d17febe40` is to resolve a shared
money claim against a receipt written in the **same Characters transaction**
as all of its recipient credits. A later balance can equal either expected
amount after an unrelated operation. A recipient at the money cap can have
the same balance for both outcomes.

The database adapter locks the claim identity before crediting any player,
then locks the recipients in GUID order. A retry with the same identity can
observe one immutable outcome. A locking read on the writer after submission
must establish whether that transaction committed. An unreadable receipt
keeps the source closed. A proven rollback releases the reservation once.
A proven commit consumes only the original pool and never pays again.
Character login loads the committed balances. Recovery must never replay
old session notifications or hold a map guard during SQL or delivery.

For a pool that can survive a restart, the saved receipt must identify its
realm, map, instance, persistent spawn, loot lifetime and shared or personal
scope. A map-generated loot GUID and an in-memory generation can both repeat.
An actual respawn or restock can produce a new grant. Disconnect, map reload
and process restart alone cannot mint another grant from a surviving pool.
Stored Item money already has a durable source row keyed by container ID.
For a persistent chest the key is realm, map, instance, spawn, generation
and shared or personal scope. The Characters writer stores its actual
retirement deadline before the map clears the pool. When the ordinary
respawn row is required, that row and the deadline commit together. The
writer releases the waiting map only on confirmation. A fresh process can
resume an interrupted release of an already-paid empty chest without a
second payout. Only the database clock reaching the recorded deadline
permits a successor generation.
The C++ anchors at `a5f8da2eb` are `WorldSession::HandleLootMoneyOpcode`
in `Handlers/LootHandler.cpp:142–213` and `GameObject::ClearLoot` in
`Entities/GameObject/GameObject.cpp:3711–3721`. The receipt is a Rust
durability requirement for its detached persistence worker.

The #1341 contract was reviewed and signed on 2026-11-22 before implementing
reopening. The receipt reviewer required the complete persistent world-source
identity and a fresh-process spent-source and restock control for acceptance.
The existing terminal quarantine remains in effect until the recovery owner
can fence that exact lifetime and token, drain existing waiters once, and
prove committed, rolled-back, unreadable and later-generation cases. Release
acceptance also requires a fresh process and real database, including a
second claim on the spent source and a legitimate new source.

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
