# PlayerRegistry consumer cutover map

Issue #150 replaces the public `DashMap` alias with an opaque directory and removes the two
production lifecycle storage operations. The subsequent cutovers below retired the assigned
production `get`, `get_mut`, and `iter` compatibility calls. The header-only
[`player-registry-consumer-map.tsv`](player-registry-consumer-map.tsv) now retains their schema,
not a list of unfinished consumers. The syntax-aware `session-ownership-policy.json` remains the
executable non-reintroduction inventory. The current directory lives in
`wow_world::session::directory`; gameplay state is not directory-owned.

The assignment is responsibility-based even when a function lives in the session monolith:

- #192 closed runtime selection, fanout, presence and generation-checked delivery resolution; its
  rows were removed from the remaining-direct-access table when the slice landed.
- #193 closed combat, loot, inventory-reward and durable-loot access through bounded projections,
  incarnation addresses and ordered directory commands; its rows were removed on landing.
- #194 closed quest, spell, movement, vehicle and movement-visibility access through bounded
  projections, spatial recipient selection and generation-checked commands; its rows were removed.
- #195 closed group, chat, social, ready-check and group-membership consumers through owned
  projections, incarnation-aware addresses and generation-checked delivery; its rows were removed.
- #196 closed the generic broad-mirror synchronization helper and every test/fixture compatibility
  operation. Backing entries are private; tests use an explicitly feature-gated fixture API or the
  real generation-aware lifecycle API. The remaining-direct-access TSV is therefore empty.

Lifecycle `register_or_replace`, generation-aware `lookup_current`/`unregister`, and owned control
address resolution are stable directory operations introduced by #150 and are not compatibility
storage access. Any new direct operation is rejected by the exact syntax baseline rather than
being assigned implicitly.

Issue #252 retired the temporary broad mirror and removed its exact-field baseline. Gameplay
consumers now resolve bounded values from canonical Player/Map owners; this map remains the
directory-operation inventory and must not be used to reintroduce a gameplay projection.
