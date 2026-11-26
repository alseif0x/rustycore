# Creature relocation on 3.4.3 (#1354)

## Stationary graphical observer in Northshire

The retained capture from the [queued-respawn client run](1342-real-client-2026-11-24.md)
also contains an independent movement encounter. On map 0, the prepared graphical
client 3.4.3.54261 was standing at `(-8948, -80)`. On that player's instance
connection, the server created a Diseased Young Wolf (entry 299, runtime counter 209),
removed it from visibility and created the same full GUID again.

| Frame | Packet | Observation |
| --- | --- | --- |
| 381 | UpdateObject | Initial Unit CREATE, 99.10 yards from the observer in the horizontal plane. |
| 392 | OnMonsterMove | The wolf's 5,331 ms spline started 96.43 yards away and ended 106.02 yards away. The destination comes from the nested spline path. |
| 407 | UpdateObject | The wolf was in the ordinary out-of-range portion of the removal list. The Unit created in this same packet was a different wolf. |
| 449 | UpdateObject | The original wolf was created again, 99.18 yards from the observer. |

Visibility was 100 yards. Eleven server visibility snapshots in the window
reported the same player position. The only client packet on that connection
between frames 381 and 449 was a time-sync response (406). This is wire evidence
of what the graphical client received. The capture was made with server
`70d950d3c`. Only integration tests and documentation changed between that
revision and the merged `3.4.3` revision `3961b4d26`.

The reference at `a5f8da2eb` routes movement through `Unit::UpdatePosition`
and `Map::CreatureRelocation`. Same-cell moves mark the creature for
visibility immediately. Deferred moves mark it after the move-list drain.
`Map::ProcessRelocationNotifies` visits the affected cells on the map update.

## Stationary protocol observers in Durotar

An isolated aarch64 debug world at `3961b4d26` ran with binary SHA-256
`b010b6de7d7966bd6cf9099fcca51beb90cb8ea5651425de0f9f771fe88797fe`.
Two authenticated, stationary protocol clients observed the same Den Grunt
(entry 5952, counter 36, persistent spawn 308514) on map 1. It walked the
36-node database path 24681120. The original waypoint, spawn and terrain data
were used. Observer 15 received the initial CREATE, normal removal and new
CREATE at frames 285, 1207 and 1397. The isolated world and Battle.net
processes were stopped and the four cloned schemas and their role removed.

Both captures can be checked offline with their retained native packet
directories. Player 16 is the graphical observer in Northshire. Player 15 is
the protocol observer in the Durotar sequence above.

```sh
python3 tools/qa/verify_creature_relocation.py "$NORTHSHIRE_PACKETS" --observer 16 --entry 299 --counter 209
python3 tools/qa/verify_creature_relocation.py "$DUROTAR_PACKETS" --observer 15 --entry 5952 --counter 36
```

The graphical wolf's normal removal (407) has full-frame SHA-256
`6b4663316f97f2d5f496fe6d32eb562fecf802874746a5d90900be78456f2192`.
The Durotar guard's removal (1207) has SHA-256
`f7f4adefcf22c8c1d46a34edf4297159ea397ab3f1de6f9b8b4d4e80836ab699`.

## Patrol restart at an unloaded grid

At Trinity-derived reference `a5f8da2eb`, `Map::MoveAllCreaturesInMoveList`
calls `Map::CreatureRespawnRelocation` when a creature cannot enter its target
grid. If its spawn grid is loaded, the server clears combat and motion, relocates
the creature and reselects its default generator. If that grid is also
unavailable, the map queues removal. See `src/server/game/Maps/Map.cpp`
(1239–1290, 1504–1535). For a patrol, `MotionMaster::DirectInitialize` selects
a new default and `MotionMaster::Update` initializes the selected generator
before updating it. `WaypointMovementGenerator<Creature>::DoInitialize` looks
up the creature's current PathId and sets the initial delay to 1,000 ms.

The regression drove a patrol to its second node before sending it towards an
unloaded grid. With the movement code from `3961b4d26`, the patrol returned to
its spawn and never launched the first leg again. On `e927ee777`, the map-owned
movement tick resolves the current default route from the same immutable
catalog used at startup. Three `blocked_patrol` cases in `wow-world-creature`
check restart at node one, a changed PathId, and a missing route. They assert
the initial delay, movement packet and destination where a path exists and
exactly one lookup in every case.

The final code at `6cc5c230a` also preserves the selected slot's initialized
state on its first tick. All three regressions passed at this revision.
An isolated aarch64 debug world exercised the blocked-grid path on this code
with executable SHA-256
`02677f4c89ebfbf4819193a45806e6e64fab1fffb81c2400f4fbaa628de91091`.
In its disposable world schema, only node 1 of path 24681120 was redirected
to `(-599.75, -4250, 37.7236)`. Pathfinding was disabled for this run to make
the edge crossing deterministic. The observer stood at `(-599.75, -4372.67)`;
the guard's spawn was within sight and the target grid beyond it. The
server sent four splines for the same guard, all from the
saved spawn to that first node. Their IDs were 2 through 5 at frames 85,
147, 229 and 305. Frames 143, 219 and 223 independently show the
create, out-of-range removal and recreation. The protocol client stayed
connected and stationary for the 55-second observation. The four cloned
schemas and disposable database role were removed after the run.

Two earlier fixes on the branch cover the other reported defects. A failed
map rebind restores the original binding and discards the mutation result.
An inaccessible chase target retains its sampled position, as in
`ChaseMovementGenerator::DoUpdate`. The corresponding regression names are
`a_map_rebind_discards_the_result_and_restores_the_original_binding_like_cpp`
and `chase_update_handles_blocked_lost_target_arrival_and_inaccessible_like_cpp`.
