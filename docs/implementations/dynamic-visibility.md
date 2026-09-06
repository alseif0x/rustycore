# Dynamic visibility — reference routing

The former implementation note is retired; its completion label is not current
parity evidence. Original history:
`7eaf8ddc:docs/implementations/dynamic-visibility.md`.

Use [STATE.md](../migration/STATE.md), [PORT_PLAN.md](../migration/PORT_PLAN.md)
and the active issue/checkpoint. The dated [movement audit](../audits/movement.md)
and [bounded parity findings](../audits/cpp-parity-findings.md) retain existing
C++ anchors, findings and coverage limits; this redirect performs no new audit.

Canonical root: `/home/server/woltk-trinity-legacy`. Start with
`src/server/game/Handlers/MovementHandler.cpp`,
`src/server/game/Server/Packets/MovementPackets.cpp` and, for visibility,
`src/server/game/Maps/Map.cpp`,
`src/server/game/Entities/Object/Object.cpp` and
`src/server/game/Entities/Object/Updates/UpdateData.cpp`.
Read the affected caller and runtime owner before accepting behavior.

*Reference routing refreshed 2026-09-05; no implementation gap closed.*
