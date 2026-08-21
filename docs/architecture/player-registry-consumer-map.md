# PlayerRegistry consumer cutover map

Issue #150 replaces the public `DashMap` alias with an opaque directory and removes the two
production lifecycle storage operations. The exact remaining production `get`, `get_mut`, and
`iter` calls are assigned once in
[`player-registry-consumer-map.tsv`](player-registry-consumer-map.tsv). The syntax-aware
`session-ownership-policy.json` remains the executable non-growth inventory; this map assigns its
remaining storage operations to the consumer slices that must remove them.

The assignment is responsibility-based even when a function lives in the session monolith:

- #192 owns runtime selection, fanout, presence and delivery resolution.
- #193 owns combat, loot, inventory-reward and durable-loot consumers.
- #194 owns quest, spell, movement, vehicle and movement-visibility consumers.
- #195 owns group, chat, social, ready-check and group-membership consumers.
- #196 owns the generic broad-mirror synchronization helper and every test/fixture compatibility
  operation. It must remove the temporary compatibility guards, iterator, `insert`, `remove`,
  metadata helpers and entry representation after #192-#195 close production access.

Lifecycle `register_or_replace`, generation-aware `lookup_current`/`unregister`, and owned control
address resolution are stable directory operations introduced by #150 and are not compatibility
storage access. Any new direct operation is rejected by the exact syntax baseline rather than
being assigned implicitly.
