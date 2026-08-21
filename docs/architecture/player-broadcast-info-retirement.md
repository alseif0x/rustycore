# PlayerBroadcastInfo retirement ledger

`PlayerBroadcastInfo` is a temporary compatibility mirror, not a canonical Player model. Issue
#196 freezes its 80 fields in
[`player-broadcast-info-retirement.tsv`](player-broadcast-info-retirement.tsv), assigns each field
to one canonical owner and names the issue that removes it from this broad mirror.

The executable architecture check compares the TSV field set with the syntax-aware
`PlayerBroadcastInfo` baseline and rejects missing, duplicate, stale or added fields. It also
requires every cutover issue to be open in the architecture issue ledger. This makes a baseline
regeneration insufficient to grow the mirror silently.

The cutovers are deliberately narrow:

- #140 owns ordinary and durable Session mailbox endpoints and removes their fields from the
  gameplay projection.
- #189 moves durable loot persistence coordination out of the directory.
- #252 retires every remaining gameplay/presentation copy through minimum immutable canonical
  Player/Unit/Map/Group queries. It removes `PlayerBroadcastInfo` when the final field is gone and
  cannot replace it with another mega-projection.

`register_or_replace`, generation-aware lookup/unregister and opaque owned addresses remain stable
directory lifecycle behavior; they do not make the directory a gameplay owner.
