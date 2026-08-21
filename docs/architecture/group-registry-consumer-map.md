# GroupRegistry and PendingInvites cutover map

Issue #151 replaces the two public `DashMap` aliases with opaque owner types. Immutable access now
returns owned `GroupInfo` / `PendingInviteLikeCpp` snapshots, and whole-registry searches consume
owned snapshot collections. The backing maps are private to `wow_network::group_registry`.

The remaining mutable compatibility surface is intentionally unchanged in behavior and assigned
exactly once in
[`group-registry-consumer-map.tsv`](group-registry-consumer-map.tsv):

- #197 owns pending-invite create/replace/cancel/accept, group creation and capacity-safe joins.
- #198 owns membership/disband, leadership, subgroup/role, difficulty, marker, loot-state and
  ready-check mutations.
- #199 owns database loading, persistence/publication separation, fixture builders and final
  removal of `insert`, `remove`, `get_mut` and their exposed guards.

C++ anchors are `Groups/GroupMgr.h:28-55` and `Groups/GroupMgr.cpp:78-110` for private store plus
identity lookup/update ownership, `Entities/Player/Player.h:2558-2562` for the player-local pending
invite pointer, and `Handlers/GroupHandler.cpp:105-210` for the invite/category/capacity sequence.
This boundary slice deliberately preserves the represented Rust behavior; #197 performs the
atomic invite/create/join convergence.

The syntax-aware `session-ownership-policy.json` is the executable non-growth inventory. At this
slice it drops from 621 to 613 exact direct-registry rows. Bounded owned `get`, `contains_key`,
`snapshots` and `matching_guids` queries are stable facade operations, not
permission to expose a map iterator or backing entry.
