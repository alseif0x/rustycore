# Mapa de referencias C++ para contraste — índice histórico

Referencias localizadas en el registro del 2026-06-27; reorganizadas el
2026-09-05 sin revalidar rutas, líneas ni comportamiento actual. Este índice
**no es una auditoría C++ completa ni demuestra paridad**. Sirve para encontrar
el código canónico y las [ramas históricamente contrastadas](cpp-parity-findings.md).

Usar [STATE.md](../migration/STATE.md), [PORT_PLAN.md](../migration/PORT_PLAN.md)
y la issue/checkpoint vigente para decidir trabajo. El
[checklist](cpp-parity-checklist.md) conserva cierres y pendientes históricos,
no impone nuevas issues, micro-PRs ni una congelación previa a programar.

Raíz canónica: `/home/server/woltk-trinity-legacy`. Los nombres abreviados
y rangos siguientes son localizadores de la pasada original. Verificar el
archivo y caller reales antes de usarlos; cuando el código es incompleto o
ambiguo, identificar una captura real y su build, sin inventar un equivalente.

| Dominio | C++ source-of-truth inicial |
| --- | --- |
| ByteBuffer bits/strings | `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h:166-217`; `ByteBuffer.cpp:78-91` |
| World packet compression | `src/server/game/Server/WorldSocket.cpp:38-53,178-188,549-609`; `WorldSocket.h:89,139,172`; `worldserver.conf.dist:277-283` |
| Auth/encrypted mode | `WorldSocket.cpp:242-244,406-412,469,904,972,1014`; `AuthenticationPackets.cpp:332-356`; `WorldSocket.h:155` |
| BNet REST/RPC | `src/server/bnetserver/Main.cpp:179-220`; `Server/SslContext.cpp`; `Server/Session.cpp:149-214,515-829`; `REST/LoginRESTService.cpp:181-707`; `REST/LoginHttpSession.cpp:74` |
| Realm list / realm join | `src/server/shared/Realm/RealmList.cpp:253-425`; `RealmList.h:71-73`; `WorldserverGameUtilitiesService.cpp:86-138` |
| UpdateData / OutOfRange | `src/server/game/Entities/Object/Updates/UpdateData.cpp:29-59`; `UpdateData.h:47-52` |
| Object update / visibility | `Object.h:80-103`; `Object.cpp:138,251-282,286`; `Player.cpp:23204,23288,23346` |
| Character create/list | `Server/Packets/CharacterPackets.h:252-278`; `CharacterPackets.cpp:343-365`; `Handlers/CharacterHandler.cpp:553,2825` |
| Account data / feature status | `WorldSession.cpp:986`; `ClientConfigPackets.cpp:20,49-69`; `ClientConfigPackets.h:29-32,71-87`; `SystemPackets.cpp:61`; `SystemPackets.h:54-109`; `AuthHandler.cpp:104-106` |
| Movement packets | `Server/Packets/MovementPackets.cpp:129-169,306-314,356-357`; `Handlers/MovementHandler.cpp` |
| Spell packets/handler | `Server/Packets/SpellPackets.cpp/h`; `Handlers/SpellHandler.cpp`; `Spells/Spell.cpp`; `SpellMgr.cpp` |
| Quest runtime | `Handlers/QuestHandler.cpp`; `Server/Packets/QuestPackets.cpp/h`; `Entities/Player/Player.cpp`; `ObjectMgr.cpp` |
| Vendor/item | `Handlers/ItemHandler.cpp:567-694`; `Server/Packets/NPCPackets.cpp:96-160`; `Server/Packets/ItemPackets.cpp/h`; `Handlers/NPCHandler.cpp:49-91,373-381` |
| Loot/corpse | `Handlers/LootHandler.cpp`; `Loot/LootMgr.cpp`; `Loot/Loot.h`; `Entities/Creature/Creature.cpp` |
| Chat/social | `Server/Packets/ChatPackets.cpp/h`; `Handlers/ChatHandler.cpp`; `Entities/Player/Player.cpp` |
| Combat | `Server/Packets/CombatPackets.cpp/h`; `Handlers/CombatHandler.cpp`; `Entities/Unit/Unit.cpp` |
| Data/DB2/stats | `DataStores/*`; `Entities/Player/Player.cpp`; `ObjectMgr.cpp`; DB2 record definitions and loader paths |
| Database statements/API | `src/server/database/*`; prepared statement enums in C++ source |
