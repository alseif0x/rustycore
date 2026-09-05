# World entry — current references

The former implementation narrative at this path is retired. Its packet layouts,
completion headline and reference assumptions are not accepted implementation
guidance. The original is recoverable from
`7eaf8ddc:docs/world-entry-implementation.md` in Git.

Current state and acceptance: [STATE.md](migration/STATE.md),
[PORT_PLAN.md](migration/PORT_PLAN.md) and the active issue/checkpoint.
Existing bounded C++/capture findings are retained in the
[world-load audit](migration/world-load-audit.md) and
[parity findings](audits/cpp-parity-findings.md); their original dates and limits
still apply.

For new contrast, start at `/home/server/woltk-trinity-legacy`:

- `src/server/game/Handlers/CharacterHandler.cpp`: player login ordering.
- `src/server/game/Entities/Player/Player.cpp`: before/after-add packets.
- `src/server/game/Maps/Map.cpp`: add/remove, visibility and updates.
- `src/server/game/Server/WorldSocket.cpp`: framing, compression and connection.
- `src/server/game/Server/Packets/AuthenticationPackets.cpp`: auth/ConnectTo.
- `src/server/game/Entities/Object/Updates/UpdateData.cpp`: object-update framing.

This pointer is not a new wire audit, runtime installation or parity claim.
*Reference routing refreshed 2026-09-05.*
