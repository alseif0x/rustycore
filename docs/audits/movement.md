# Movement parsing audit (#A06)

The May 11, 2026 audit compared selected movement operations with the 3.4.3 C++ reference. The [original audit at the recorded repository revision](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/audits/movement.md) preserves the wire tables, follow-up identifiers and later pathfinding notes. Check the [branch state](../migration/STATE.md) and [delivery plan](../migration/PORT_PLAN.md) before using a finding for new work.

| Reference under `src/server/game` | What the audit compared |
| --- | --- |
| `Entities/Object/MovementInfo.h` | Movement flags, position, GUID, time, transport and optional fall and inertia fields. |
| `Server/Packets/MovementPackets.cpp` | MovementInfo write order (25–99), read order (104–170), optional transport fields (181–198) and movement ACK layouts (800–989). |
| `Handlers/MovementHandler.cpp` | Near-teleport acknowledgement (263–306), movement validation and broadcasts (312–430), forced-speed acknowledgement (470–539), knockback and other acknowledgements (552–666). |

The audit recorded the fall-data write rule (#A06.1), preservation of the standing-object GUID and inertia (#A06.2), partial input and transport validation (#A06.3–4), and client time adjustment (#A06.5). Later entries distinguish packet parsing and represented state from production effects in Unit, Aura, Map, Vehicle and MotionMaster. The pathfinding notes identify Detour chase and home coverage, outstanding terrain and line-of-sight behavior, and a connected scenario awaiting a real capture. Use the original rows for exact evidence and reproduce a remaining gap on the intended branch before assigning it.
