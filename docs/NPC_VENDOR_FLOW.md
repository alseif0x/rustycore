# NPC vendor — C++ reference map

This replaces an unsupported historical flow description; it is a navigation
aid, not a newly validated vendor implementation. Original history:
`7eaf8ddc:docs/NPC_VENDOR_FLOW.md`.

Canonical root: `/home/server/woltk-trinity-legacy`.

- `src/server/game/Handlers/NPCHandler.cpp`: gossip admission and selection.
- `src/server/game/Handlers/ItemHandler.cpp`: inventory listing and vendor buy/sell.
- `src/server/game/Server/Packets/NPCPackets.cpp`: gossip/vendor serialization.
- `src/server/game/Server/Packets/ItemPackets.cpp`: item transaction packets.
- `src/server/game/Entities/Player/Player.cpp`: interaction, storage and payment rules.

Trace validation, inventory/money mutation, persistence and packet publication
as one operation; packet presence alone does not prove an implemented vendor.
Use [STATE.md](migration/STATE.md), [PORT_PLAN.md](migration/PORT_PLAN.md) and the
active issue for current scope. Existing dated findings are in
[cpp-parity-findings.md](audits/cpp-parity-findings.md).

*Reference routing refreshed 2026-09-05; no new runtime/capture proof.*
