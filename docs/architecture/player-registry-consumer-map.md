# PlayerRegistry consumer cutover map

Issue #150 replaces the public `DashMap` alias with an opaque directory and removes the two
production lifecycle storage operations. The exact remaining production `get`, `get_mut`, and
`iter` calls are assigned once in
[`player-registry-consumer-map.tsv`](player-registry-consumer-map.tsv). The syntax-aware
`session-ownership-policy.json` remains the executable non-growth inventory; this map assigns its
remaining storage operations to the consumer slices that must remove them.

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

The temporary broad mirror is separately tracked field by field in
[`player-broadcast-info-retirement.tsv`](player-broadcast-info-retirement.tsv). Its exact
membership is cross-checked against the syntax baseline, so a field cannot be added by merely
regenerating that baseline.
