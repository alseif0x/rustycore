# Login/world-load C++ parity plan

Status: active plan.

Goal: make RustyCore enter the world with the same observable order and packet/state
semantics as the C++ reference. This is not a diagnostic track and not a patch track.
Every completed item must be backed by exact C++ references, Rust implementation,
tests where possible, and a real-client check when the slice affects login/world load.

Reference source of truth:

- C++ repo: `/home/server/woltk-trinity-legacy`
- Login handler: `src/server/game/Handlers/CharacterHandler.cpp`
- Player login packets: `src/server/game/Entities/Player/Player.cpp`
- Map add/init visibility: `src/server/game/Maps/Map.cpp`
- Object accessor: `src/server/game/Globals/ObjectAccessor.cpp`
- Rust repo: `/home/server/rustycore`

Non-negotiable rules:

- C++ is the authority. Rust comments, old summaries, C# references, and prior AI
  conclusions are not authority.
- If a Rust function says `C#`, `csharp`, or implies C# packet order/layout, treat it
  as suspicious until re-anchored to C++ or removed.
- No diagnostic flag may become part of the final behavior. The final path must run
  without `RUSTYCORE_LOGIN_UPDATEOBJECT_DIAGNOSTIC` or equivalent behavior gates.
- If a dependency is missing while porting this flow, continue by implementing that
  dependency inside the same phase until the chain is complete. Do not leave it as a
  TODO, do not mark it as a future gap, and do not jump to another area with partial
  behavior still present.
- Do not merge to `main` unless explicitly requested.
- Each implementation slice gets its own commit after `fmt`, `check`, targeted tests,
  server restart, and real-client result when relevant.

## Canonical C++ order

This order is the shape Rust must follow.

1. `WorldSession::HandlePlayerLogin`
   - `CharacterHandler.cpp:1184` calls `Player::SendInitialPacketsBeforeAddToMap`.
   - `CharacterHandler.cpp:1224` calls `pCurrChar->GetMap()->AddPlayerToMap(pCurrChar)`.
   - `CharacterHandler.cpp:1241` calls `ObjectAccessor::AddObject(pCurrChar)`.
   - `CharacterHandler.cpp:1262` calls `Player::SendInitialPacketsAfterAddToMap`.
   - `CharacterHandler.cpp:1274` sets in-game time.

2. `Player::SendInitialPacketsBeforeAddToMap`
   - Source: `Player.cpp:23479`.
   - Sends login/session setup packets before the player is added to the map.
   - Ends by setting the moved unit to the player (`SetMovedUnit(this)`).

3. `Map::AddPlayerToMap`
   - Source: `Map.cpp:428`.
   - Required order:
     - load/ensure active player grid;
     - add player to grid;
     - set player map;
     - add player to world;
     - `SendInitSelf(player)`;
     - `SendInitTransports(player)`;
     - clear `player->m_clientGUIDs`;
     - `player->UpdateObjectVisibility(false)`;
     - `PhasingHandler::SendToPlayer(player)`;
     - instance/corpse/script side effects.

4. `Map::SendInitSelf`
   - Source: `Map.cpp:1877`.
   - Builds one `UpdateData` containing item create blocks and the player self create.
   - Item blocks must precede the player block inside the same `UpdateObject` packet.

5. `Map::SendInitTransports`
   - Source: `Map.cpp:1927`.
   - Sends transport create blocks after self create and before clearing visible GUIDs.

6. `Player::UpdateObjectVisibility(false)`
   - Called from `Map::AddPlayerToMap`.
   - This is the first normal visible-object pass. It must happen after self init and
     transports, and before post-add world state packets.

7. `PhasingHandler::SendToPlayer`
   - Called inside `Map::AddPlayerToMap`, after initial visibility.
   - Rust must send the phase packet once in this location, not again later unless C++
     does so for a separate reason.

8. `ObjectAccessor::AddObject`
   - Source: `ObjectAccessor.cpp:298`.
   - Called by `HandlePlayerLogin` after `AddPlayerToMap` returns and before
     `SendInitialPacketsAfterAddToMap`.

9. `Player::SendInitialPacketsAfterAddToMap`
   - Source: `Player.cpp:23592`.
   - Includes player visibility/update-zone/world-state/CUF/aura/item-duration
     post-map-entry packets.

## Current Rust divergences to fix first

These are already visible in `crates/wow-world/src/handlers/character.rs`:

- `ensure_login_player_controller_like_cpp`, canonical player attach, player registry,
  and `sync_object_accessor_player` currently run after post-add packets. In C++, the
  player is in grid/world before `SendInitSelf`, and `ObjectAccessor::AddObject` runs
  before `SendInitialPacketsAfterAddToMap`.
- `PhaseShiftChange::default_for(guid)` is sent twice in the current login sequence.
  C++ sends the map-change phase update inside `Map::AddPlayerToMap` after visibility.
- Some comments around spell modifier/login setup still reference C# order. Those
  references must be removed or replaced with exact C++ anchors.
- Initial Rust visibility is much smaller than C++ in observed logs. This means the
  visible-object pass is not yet equivalent even if the packet does not crash.

## Phase 0 - Baseline and guardrails

Deliverables:

- Confirm clean branch base before each slice: no stash dependence, no hidden branch.
- Keep current server logs:
  - Rust world: `/tmp/rustycore-develop-world-live.log`
  - C++ world: `/tmp/trinity-cpp-world-live.log`
- Record a C++ successful login trace and a Rust login trace for the same account,
  character, map, and client build.
- Ensure Rust runs without behavior diagnostic flags.

Acceptance:

- `git status --short --branch` is captured before implementation.
- C++ login order above is re-confirmed from source and, when useful, logs.
- Any old C# references in the touched code are removed or converted to C++ references.

## Phase 1 - Reorder Rust login state to match C++ AddPlayerToMap

Intent: make Rust's internal state order match C++ before changing packet payloads.

Tasks:

- In `send_login_sequence`, split the login flow into named sections matching C++:
  - `SendInitialPacketsBeforeAddToMap`;
  - `AddPlayerToMap`;
  - `ObjectAccessor::AddObject`;
  - `SendInitialPacketsAfterAddToMap`.
- Move player controller/canonical player attach to the Rust equivalent of
  `Map::AddPlayerToMap` before the self `UpdateObject`.
- Ensure the player is registered in map/world state before self init if any later
  packet generation depends on it.
- Move `sync_object_accessor_player` to the C++ equivalent location: after
  `AddPlayerToMap` and before post-add packets.
- Decide explicitly whether `SessionState::LoggedIn` is part of Rust's AddToWorld
  equivalent or must remain later to protect packet handling; document the reason in
  code with a C++ anchor.
- Remove the duplicate phase packet. Keep exactly the C++ map phase send location.

Acceptance:

- Login trace names show the C++ order, not the old Rust phase names.
- No self create, transport init, visibility, phase, object accessor, or post-add packet
  is emitted out of order relative to the C++ list above.
- Existing login unit tests are updated to assert the order where practical.
- Real client reaches the world at least as far as before this phase.

## Phase 2 - Port self create packet layout exactly

Intent: `Map::SendInitSelf` parity.

Tasks:

- Re-derive the Rust self `UpdateObject` from C++ `Map::SendInitSelf`,
  `Player::BuildCreateUpdateBlockForPlayer`, `Item::BuildCreateUpdateBlockForPlayer`,
  and related object value builders.
- Preserve the C++ block order: all visible inventory/item create blocks first, then
  the player create block, in one `UpdateObject` packet.
- Remove any field order described as C# order. Re-anchor every contested field group
  to the C++ writer that produces it.
- For every conditional movement/update field in player create, document the exact C++
  condition and implement the same condition.
- If a required field depends on a not-yet-ported manager (skills, auras, inventory,
  reputation, quest log, collections, movement state), port the required manager surface
  immediately in this phase for login correctness before doing anything else.

Acceptance:

- Rust debug summary for self create matches C++ by block count, block order, movement
  section presence, values size trend, and all known field groups.
- No behavior diagnostic flag is needed to avoid client crash.
- Real client enters world without Error #132 during self create.

## Phase 3 - Port transport init exactly

Intent: `Map::SendInitTransports` parity.

Tasks:

- Port C++ transport selection for the current map.
- Port C++ transport create packet construction, including stationary position, server
  time, rotation, movement flags, and any gameobject fields used by transports.
- Port exact transport path/time calculation from C++ transport templates instead of
  approximating with partial DB position data.
- Ensure transport packets are sent after self create and before clearing visible GUIDs.

Acceptance:

- C++ and Rust transport block counts match for the same map and server time window.
- Packet order is self create -> transports -> clear visible GUIDs.
- Real client still enters world without crash.

## Phase 4 - Port initial visibility exactly

Intent: `Player::UpdateObjectVisibility(false)` parity.

Tasks:

- Port the C++ visibility source path:
  - visibility notifier;
  - object range rules;
  - phase checks;
  - grid/cell enumeration;
  - `HaveAtClient` / client GUID cache semantics.
- Rust must not cap visible creatures/gameobjects with arbitrary limits. If C++ sends
  405 blocks for the same login context, Rust must explain and match that count or the
  exact C++ reason for any difference.
- Make `client_visible_guids_like_cpp` follow the same lifecycle as C++
  `m_clientGUIDs`: cleared after init self/transports, repopulated by visibility.
- Build initial creature/gameobject create blocks from C++ object builders, not from
  C# layouts or simplified structs.
- If missing creature/gameobject fields cause client instability, port those fields in
  this phase.

Acceptance:

- Rust visibility block count and object categories match C++ for the same player/map
  within deterministic DB and spawn conditions.
- The client sees the same classes of objects C++ sends at login: creatures,
  gameobjects, transports, players where applicable.
- NPCs that should be visible are visible without manual flags.
- Real client enters and remains connected.

## Phase 5 - Port creature create/load/runtime required by visibility

Intent: no fake or partial creature data in visible create blocks.

Tasks:

- Use `docs/migration/creature-port-no-gaps-plan.md` as the broader creature track,
  but for login visibility this phase must close every required field now.
- Port C++ `Creature::Create`, DB load, template load, model/display selection, level
  selection, stats, health, faction, flags, movement type, spawn position, equipment,
  dynamic flags, bytes fields, npc flags, quest/trainer/vendor flags required by create.
- Remove random movement from creatures that C++ would initialize idle/stationary.
- Port movement generator initialization only to the extent needed for login-visible
  state. If a creature should move at login in C++, port the required movement generator.
- Ensure creature create blocks never use placeholder level/health when C++ would derive
  template/scaled stats.

Acceptance:

- Sample creatures in the player login area match C++ entry, guid class, display,
  level, health, faction, flags, npc flags, position, orientation, movement type, and
  create block shape.
- NPCs that should not move do not move.
- NPCs that should move continue moving beyond the first few seconds when C++ does.
- Real client can select/interact with visible NPCs at least to the opcode handler
  boundary.

## Phase 6 - Port post-add packets exactly

Intent: `Player::SendInitialPacketsAfterAddToMap` parity.

Tasks:

- Port the exact post-add order from `Player.cpp:23592`.
- Implement or verify:
  - self visibility update;
  - zone update;
  - init world states;
  - CUF profiles;
  - login aura/effect packets;
  - self auras;
  - item/enchant durations;
  - difficulty packets.
- Do not send phase here unless C++ sends an additional distinct phase packet in this
  function for this client/build.

Acceptance:

- Rust post-add packet order matches C++ logs/source.
- Client controls are active after login: movement, camera, selection, chat input,
  inventory open, NPC targeting.
- Any missing handler discovered here is ported or the phase remains incomplete.

## Phase 7 - Player control and movement after login

Intent: prove the client is not only loaded, but interactive.

Tasks:

- Verify moved unit / active mover state from C++:
  - `SetMovedUnit(this)` before add-to-map;
  - no duplicate or contradictory active mover packet.
- Verify movement opcodes are accepted only after the correct session/world state.
- Verify Rust does not freeze movement by leaving the session in the wrong state or by
  failing map/object accessor registration.
- Contrast C++ movement handler login/teleport order for any remaining divergence.

Acceptance:

- Real client can move, rotate camera, zoom, target NPCs, open inventory, type normally,
  and remain connected.
- Server logs show movement packets processed and player map position updated.

## Phase 8 - NPC interaction surfaces visible immediately after login

Intent: the world is usable, not only visible.

Tasks:

- Port/verify object selection and gossip flow for visible NPCs.
- Port/verify quest giver status and quest list packets.
- Port/verify vendor inventory flow.
- Port/verify trainer list flow.
- Port/verify inventory open/equipment state if still broken.
- Any missing DB/template dependency required by those handlers is ported in this phase.

Acceptance:

- Real client can select NPCs, open gossip, see quests where C++ shows them, open vendor
  inventory, open trainer UI, and open player inventory without client crash.

## Phase 9 - Final parity run

Intent: prove this flow is closed.

Tasks:

- Run C++ and Rust with the same DB, same client build, same account/character, same map.
- Capture login-to-world packet/order logs from both.
- Compare:
  - packet order;
  - major packet sizes;
  - UpdateObject block counts;
  - self create block order;
  - transport count;
  - visibility count and object categories;
  - first post-login movement/control opcodes.
- Remove temporary logs or keep only controlled trace logs behind neutral trace flags.
- Update the handoff/audit docs with exact commit and evidence.

Acceptance:

- No diagnostic behavior flags.
- `cargo fmt --all`.
- `PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p wow-map -p wow-world -p world-server`.
- Targeted tests for touched modules.
- Server restarted from the built binary.
- Real client enters world, can move/control/select/interact, and no Error #132 or
  WOW51900319 occurs during the login path.

## Immediate next slice

Start with Phase 1 only:

- Reorder Rust login state and object accessor sync to the C++ structural order.
- Remove duplicate phase send.
- Replace touched C# comments with C++ anchors.
- Add trace markers matching C++ names.
- Build, restart, real-client test.

Do not proceed to packet payload changes until Phase 1 is clean, because packet payload
debugging is unreliable while Rust emits correct-looking packets from the wrong world
state/order.
