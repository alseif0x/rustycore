# GroupRegistry and PendingInvites cutover map

Issue #151 replaces the two public `DashMap` aliases with opaque owner types. Immutable access now
returns owned `GroupInfo` / `PendingInviteLikeCpp` snapshots, and whole-registry searches consume
owned snapshot collections. The backing maps are private to `wow_network::group_registry`.

The remaining mutable compatibility surface is assigned exactly once in
[`group-registry-consumer-map.tsv`](group-registry-consumer-map.tsv):

- #197 closed pending-invite create/replace/cancel/expire/decline/accept, first-group creation and
  capacity-safe joins behind atomic typed operations. No production handler mutates
  `PendingInvites` or inserts a newly created group directly.
- #198 owns membership/disband, leadership, subgroup/role, difficulty, marker, loot-state and
  ready-check mutations.
- #199 owns database loading, persistence/publication separation, fixture builders and final
  removal of `insert`, `remove`, `get_mut` and their exposed guards.

C++ anchors are `Groups/GroupMgr.h:28-55` and `Groups/GroupMgr.cpp:78-110` for private store plus
identity lookup/update ownership, `Entities/Player/Player.h:2558-2562` for the player-local pending
invite pointer, and `Handlers/GroupHandler.cpp:105-210` for the invite/category/capacity sequence.
The transition preserves the represented Rust packet and persistence order while serializing the
C++ invite/create/join sequence inside the owner.

The syntax-aware `session-ownership-policy.json` is the executable non-growth inventory. #151
dropped it from 621 to 613 exact direct-registry rows; #197 lowers it again to 607. Bounded owned `get`, `contains_key`,
`snapshots` and `matching_guids` queries are stable facade operations, not
permission to expose a map iterator or backing entry.
