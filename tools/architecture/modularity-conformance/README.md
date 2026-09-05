# Independent-module conformance — private experiment

This is the finite pre-production gate owned by #578, specified in
[the approved plan §5](../../../docs/architecture/modularity-and-ecs-plan.md).
It is a sibling of `modularity-lab`, not a revision of V1's source or evidence campaign.
Production Cargo dependencies, runtime, database and public module SDK remain unchanged.

**Under construction: no conformance verdict or measurement yet.** Build/tests during
implementation do not constitute the frozen-host/third-module experiment. The campaign
must fail closed until all required cells and the predeclared protocol are implemented.

## Contract and authorities

- The experimental core owns identity/incarnation, active/detached residence and private
  hecs storage. Native modules provide their own state type and codec; generic registration
  creates typed components distinguished by module type, not a central module enum.
- Modules only depend on `contract`. They obtain owned bounded snapshots and perform
  revision-checked writes. Mutable state/guest globals, backend IDs, storage borrows, SQL,
  packet writers and ambient I/O are not extension capabilities.
- The Wasm adapter uses host-owned namespaced bytes, not guest-owned canonical state.
  A single Wasmtime Store contains the experimental core and all loaded guest instances.
  Native and Wasm calls share the same invocation stack and host budgets; nested callbacks
  never replenish fuel or retain a state/guest-memory borrow across reentry.
- Native code is trusted source. The host caps its admitted actions, depth and state, but
  does not promise to interrupt arbitrary native loops or contain native panics.
- Each module has a versioned encoding. Mock snapshot/replay and executor interchange
  must validate versions before activation; they prove neither SQL durability nor recovery
  from a real lost COMMIT. Backend locators are never replay identities.
- Removal clears only that module's reversible contributions. Reset, detach and attach
  callbacks may implement module-specific rules without new central lifecycle cases.
  Schema version, incarnation and mutation revision remain distinct; resetting revision
  would introduce an ABA bug and is not allowed.

## C++ anchors and custom behavior

Source root: `/home/server/woltk-trinity-legacy/src/server/`.

- `scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:81-181`: publish phase and shield before
  nullable summon; failure does not undo prior effects. The sample is not a complete boss port.
- `game/Entities/Creature/TemporarySummon.cpp:249-264`: summon lifecycle callbacks execute
  synchronously before return. Dispatching them next tick is not equivalent.
- `game/AI/CreatureAI.cpp:219-242`: synchronous evade/reset ordering.
- `game/Maps/MapManager.cpp:287-318`: all update work precedes delayed updates. The experiment
  may test its own driver barrier; only production-linked tests can establish real scheduling.
- `game/Entities/Player/Player.cpp:2189-2226`: policy hook before XP award. The optional
  arithmetic contribution in this experiment is explicitly custom, not full GiveXP parity.

The base record remains present with zero optional modules. It represents only this narrow
contract fixture, not proof that required production scripts are complete or enabled.

## Freeze and third-party challenge

First implement and test encounter and policy in separate crates through the same host.
Then hash the contract, host, adapters, original modules and C bindings. Only after the
freeze may an independent third crate introduce a new state shape and lifecycle rule.
That step may change dependency declarations, declarative registration/composition and its
own code/tests; it may not edit frozen host/storage/ABI code or add central module cases.

Record every frozen path/hash and check exact additions/removals as well as file contents.
A required frozen-core correction invalidates that challenge: record the reason, correct and
validate the host, then run a genuinely new independent extension challenge. Do not silently
rehash a post-hoc successful implementation or call editing the host zero integration cost.

Protocol, commands, exact matrix and retained campaign evidence will be added before the
first measurement. Existing V1 provisional results are not this gate's acceptance.
