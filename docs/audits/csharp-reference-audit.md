# Auditoria de referencias C# vs C++

Fecha: 2026-06-27

Scope: auditoria estatica, sin cambios de runtime.

Busqueda base:

```bash
rg -n -i 'c#|csharp|c-sharp|c sharp' --glob '!target/**' --glob '!prompt.md' .
```

Regla de auditoria:

- C++ en `/home/server/woltk-trinity-legacy` es la fuente de verdad.
- Una referencia a C# en codigo Rust, docs de implementacion o planes antiguos es un marcador de bug hasta que se contraste contra C++.
- Si C++ no responde el caso, solo se puede mantener comportamiento C# con evidencia externa: captura real cliente/servidor, version de cliente y justificacion.
- Esta auditoria no corrige codigo. Solo localiza referencias y ancla el "vs C++" que debe usarse en la siguiente fase.

## Resumen

Hay referencias C# en codigo productivo, docs de implementacion y planes de migracion. Los focos con mas riesgo son:

- Packets y serializers: `crates/wow-packet/src/**`.
- Handlers/runtime: `crates/wow-world/src/handlers/**`, `crates/world-server/src/main.rs`.
- Networking/crypto/auth: `crates/wow-network/src/**`, `crates/wow-crypto/src/**`, `crates/bnet-server/src/**`.
- Datos/DB: `crates/wow-data/src/**`, `crates/wow-database/src/**`.
- Docs antiguas que declaran C# como referencia correcta: `docs/world-entry-implementation.md`, `docs/phase4-analysis-and-fixes.md`, `docs/implementations/*.md`, `docs/NPC_VENDOR_FLOW.md`.

Tambien hay documentos buenos que ya dicen que C# no es autoridad. Esos se conservan como politica, pero no validan el codigo automaticamente.

## Produccion: referencias C# y contraste C++ requerido

| Rust ref | Referencia C# localizada | C++ esperado para contraste | Veredicto |
| --- | --- | --- | --- |
| `crates/wow-packet/src/world_packet.rs:391,410,419` | `ResetBitPos`, `ReadBits`, `HasBit` descritos como C# | `src/server/shared/Packets/ByteBuffer.h:166-217`; `ByteBuffer.cpp:78-91` | Misatribucion. Las APIs existen en C++; reescribir comentarios contra C++ al tocar. |
| `crates/wow-packet/src/compression.rs:12,21,22,53,65` | Deflate persistente, threshold y flush atribuidos a C# | `src/server/game/Server/WorldSocket.cpp:38-53,178-188,549-609`; `WorldSocket.h:89,139,172`; `worldserver.conf.dist:277-283` | Alto riesgo si el layout no replica C++. Auditar bytes: `CompressedWorldPacket`, `MinSizeForCompression=0x400`, adler y `Z_SYNC_FLUSH`. |
| `crates/wow-network/src/world_socket.rs:400,558,790,937,955,1292` | Auth/encryption/counters/Ed25519 citan C# | `src/server/game/Server/WorldSocket.cpp:242-244,406-412,469,904,972,1014`; `AuthenticationPackets.cpp:332-356`; `WorldSocket.h:155` | Bug-marker. El comportamiento debe quedar explicado por `WorldSocket`/`WorldPacketCrypt` C++, no por C#. |
| `crates/wow-network/src/accept.rs:536,631,633` | Seeds/HMAC y debug comparan contra C# | `src/server/game/Server/WorldSocket.cpp`; `AuthenticationPackets.cpp`; `WorldSession.cpp` | Alta prioridad auth. Contrastar handshake y secretos contra C++ antes de modificar. |
| `crates/wow-crypto/src/world_crypt.rs:14,33,69` | AES-GCM tag de 12 bytes y counter explicados con C# | `src/server/game/Server/WorldPacketCrypt.*`; `WorldSocket.cpp` auth/encrypted mode | Bug-marker. Puede ser correcto, pero el comentario debe reanclarse a C++ o captura. |
| `crates/wow-crypto/src/rsa_sign.rs:18,28,222,234` | RSA store/signature reversed desde C# | `src/server/game/Server/Packets/AuthenticationPackets.cpp:332-356`; BNet/server auth crypto en C++ | Alto riesgo. Verificar parametros DER/signature contra C++ o captura. |
| `crates/wow-crypto/src/ed25519ctx.rs:15,27` | `Ed25519Operations.crypto_sign` C# | `AuthenticationPackets.cpp`; C++ crypto helpers usados por `EnterEncryptedMode` | Bug-marker. Reanclar o justificar con evidencia de cliente. |
| `crates/wow-crypto/src/bnet_srp6.rs:116,126,272,298,371,385,448,460,462,476` | BigInteger, endian, hex, broken evidence vector desde C# | `src/server/bnetserver/REST/LoginRESTService.cpp:247,411,537`; TC SRP/OpenSSL helpers | Alto riesgo. C# BigInteger no es fuente; auditar SRP byte-for-byte contra C++ BNet REST. |
| `crates/bnet-server/src/main.rs:4,637,638` | Drop-in C# BNetServer; TLS 1.2/no ALPN por C# | `src/server/bnetserver/Main.cpp:179-220`; `Server/SslContext.cpp`; `REST/LoginRESTService.cpp`; `REST/LoginHttpSession.cpp` | C++ existe. Mantener no-ALPN solo si coincide con C++ o captura. |
| `crates/bnet-server/src/rest/mod.rs:3,6,127,175,209` | HTTP raw/TLS/keep-open/header format desde C# | `src/server/bnetserver/REST/LoginRESTService.cpp:181-498`; `LoginHttpSession.cpp:74`; HTTP service C++ | Alto riesgo. Header/cierre de conexion deben contrastarse contra TC REST, no C#. |
| `crates/bnet-server/src/rest/types.rs:29,59`; `rest/handlers.rs:254,882` | Null fields, cookie, wrong password body desde C# | `LoginRESTService.cpp:181-498`; `CreateSrpImplementation:537` | Bug-marker. Revisar respuestas JSON/HTTP exactas contra C++. |
| `crates/bnet-server/src/rpc/session.rs:203,218,231`; `rpc/services/game_utilities.rs:30` | RPC responses y suffix desde C# | `src/server/bnetserver/Server/Session.cpp:149-214,515-712`; `Services/GameUtilitiesService.*` | Alto riesgo. RPC framing debe validarse con C++ header/protobuf. |
| `crates/bnet-server/src/realm/mod.rs:245,721` | Realm list/JSON types desde C# | `src/server/shared/Realm/RealmList.cpp:253-425`; `RealmList.h:71-73`; `WorldserverGameUtilitiesService.cpp:86-138` | C++ ancla clara. Reauditar JSON prefixes, zlib y atributos. |
| `crates/world-server/src/main.rs:483,11108,11114,11125,11804` | Login ticket/account expansion/locale por C# | `WorldSocket.cpp`; `WorldSession.cpp`; `AccountMgr`/login DB C++; `ClientConfigPackets.*` | Bug-marker. Especialmente expansion/account data debe seguir C++. |
| `crates/wow-packet/src/packets/character.rs:132,159,225,697` | EnumCharacters, CharacterInfo, RaceUnlock, response codes desde C# | `src/server/game/Server/Packets/CharacterPackets.h:252-278`; `CharacterPackets.cpp:343-365`; `Handlers/CharacterHandler.cpp:553,2825` | Alta prioridad. Character list/create debe revalidarse contra packet classes C++. |
| `crates/wow-world/src/handlers/character.rs:1315,2502,2503,3259,3611,11843,12185` | Equip display, flags, played time, logout, inventory slots desde C# | `Entities/Player/Player.cpp/h`; `Handlers/CharacterHandler.cpp`; `Server/Packets/CharacterPackets.*`; `MiscHandler.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasadas 3/8/10. RSA ConnectTo, played-time, inventory slot constants y equipment cache no son bug en el slice revisado; bugs confirmados en flags de enum character, logout instantaneo e inventory runtime simplificado. |
| `crates/wow-packet/src/packets/misc.rs:2185-2261` | `FeatureSystemStatus` glue en orden exacto C# | `Server/Packets/SystemPackets.cpp:61`; `SystemPackets.h:54-109`; `Handlers/CharacterHandler.cpp:1457-1459`; `AuthHandler.cpp:104-106` | Alto riesgo. Ya se sabe que docs antiguas empujan C#; C++ debe decidir layout. |
| `crates/wow-packet/src/packets/misc.rs:4534,4767,4809,4831,4891,4912,4932,5107,5242,5889,5913,5933,5965,6290,12964,13029` | Dungeon difficulty, movement/new-world, skill/proficiency, played time, taxi, cemetery, auction, time, XP/level C# | `Server/Packets/*Packets.cpp/h`; `Handlers/MiscHandler.cpp`; `MovementPackets.cpp`; `TaxiHandler.cpp`; `QueryHandler.cpp`; `Spells/SpellEffects.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasadas 9/12. Layouts de difficulty/hotfix/movement-transfer/played-time/taxi/cemetery/query-time/XP/level no bug; bugs confirmados en ShowTradeSkill, AuctionHelloResponse y `SetProficiency::default_*` como tabla C# no C++. |
| `crates/wow-world/src/handlers/misc.rs:2091,2179,2214,2395,2959` | Worldport ack, area trigger, cemetery, taxi, query time desde C# | `Handlers/MiscHandler.cpp`; `MovementHandler.cpp`; `TaxiHandler.cpp`; `QueryHandler.cpp`; packet classes C++ | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 13. `QueryTime` no bug; bugs confirmados en secuencia worldport, parser/flow de AreaTrigger, CemeteryList vacio y TaxiStatus simplificado. |
| `crates/wow-packet/src/packets/movement.rs:26,1311,1331` | Movement parse, SetActiveMover, MoveInitActiveMoverComplete desde C# | `Server/Packets/MovementPackets.cpp:25-230,825-835,1094-1097`; `Server/Packets/MovementPackets.h`; `Handlers/MovementHandler.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 16. Layout `MovementInfo`, parsers `SetActiveMover`/`MoveInitActiveMoverComplete` y serializer basico no son bug en el slice revisado. `#CSharpAudit.MOVEMENT.1` corregido: el handler generico usa el mover actual. Siguen abiertos `#CSharpAudit.MOVEMENT.2` y `#CSharpAudit.MOVEMENT.3`. |
| `crates/wow-world/src/handlers/movement.rs:38` | CMSG_MOVE thread-safety desde C# | `Server/Protocol/Opcodes.cpp:612-698,880`; `Handlers/MovementHandler.cpp` | Parcialmente contrastado en pasada 16. Registro de CMSG_MOVE principal, ACKs, `SetActiveMover`, `MoveInitActiveMoverComplete` y `MoveTimeSkipped` coincide en status/processing en los opcodes revisados; la referencia C# es deuda de comentario, no autoridad. |
| `crates/wow-packet/src/packets/spell.rs:8,583,769,785,804,807,817` | Spell packets/cast failed/cooldown desde C# | `Server/Packets/SpellPackets.cpp/h`; `Handlers/SpellHandler.cpp`; `Spells/Spell.cpp` | Alto riesgo serializer. Auditar cada packet class contra C++. |
| `crates/wow-world/src/handlers/spell.rs:20` | Handler/spell reference C# | `Handlers/SpellHandler.cpp`; `Spells/Spell.cpp`; `SpellMgr.cpp` | Bug-marker. |
| `crates/wow-packet/src/packets/combat.rs:159,180,191,204`; `crates/wow-world/src/handlers/combat.rs:10` | Combat packet format desde C# | `Server/Packets/CombatPackets.*`; `Handlers/CombatHandler.cpp`; `Entities/Unit/Unit.cpp` | Bug-marker. Revalidar sub-buffer/HitInfo/ContentTuning contra C++. |
| `crates/wow-packet/src/packets/chat.rs:617,817,829,863,886`; `crates/wow-world/src/handlers/chat.rs:12,17,41,821,831` | Chat/emote/ranges desde C# | `Server/Packets/ChatPackets.cpp/h`; `Handlers/ChatHandler.cpp`; `Entities/Player/Player.cpp` | Bug-marker. Config/ranges y packet layout deben salir de C++. |
| `crates/wow-packet/src/packets/party.rs:2`; `crates/wow-world/src/handlers/group.rs:809` | Party/group packet layout desde C# | `Server/Packets/PartyPackets.*`; `Groups/Group.cpp/h`; `Handlers/GroupHandler.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 7. Layouts CMSG/SMSG revisados mayoritariamente no bug; bugs confirmados en valores de `SMSG_PARTY_INVITE`, validaciones invite, `PartyIndex` ignorado y lifecycle accept/leave. |
| `crates/wow-packet/src/packets/item.rs:354,364`; `crates/wow-world/src/handlers/character.rs:11843,12185` | Inventory update/read and slot constants from C# | `Server/Packets/ItemPackets.cpp/h`; `Handlers/ItemHandler.cpp`; `Entities/Item/*`; `Player.cpp` inventory methods | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 8. `InvUpdate` y varios serializers/parsers son no bug; bug confirmado en `AutoStoreBagItem` A/B invertido y bugs runtime en inventory move/equip/store/destroy simplificados frente a C++. |
| `crates/wow-world/src/handlers/loot.rs:8,1640,6383`; `crates/wow-ai/src/lib.rs:1088` | Loot handler/corpse removal desde C# | `Handlers/LootHandler.cpp`; `Loot/LootMgr.cpp`; `Loot/Loot.h`; `Entities/Creature/Creature.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 11. Layouts de packets basicos y release/corpse decay comun no bug; bugs confirmados en valores de `LootResponse`, gating `isAllowedToLoot`, money aura/criteria y cascada `StoreLootItem`. Quedan pendientes loot generation/templates, roll completo, master-loot remoto y storage DB end-to-end. |
| `crates/wow-world/src/handlers/quest.rs:4968,4994,5634,6294,6354`; `crates/wow-data/src/quest.rs:16,61,83,279`; `quest_xp.rs:8,18`; `packets/quest.rs:15` | Quest packet/data/status logic desde C# | `Handlers/QuestHandler.cpp`; `Server/Packets/QuestPackets.cpp/h`; `Entities/Player/Player.cpp`; `ObjectMgr.cpp` quest loaders | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 14. Constantes, layout DB2 `QuestXP`, `RoundXPValue`, `is_available_for`, parser de choose-reward y algunos serializers basicos no son bug en el slice revisado. Bugs confirmados en `QueryQuestInfoResponse`, `QuestRewardsBlock`, `QuestGiverQuestComplete`, parsers query/accept, XP fallback/config y gates/side-effects runtime de quest. Quedan pendientes POI, quest share completo, loaders/normalizacion completa, quest status multiple, gossip/menu y persistencia end-to-end. |
| `crates/wow-data/src/player_stats.rs:8,33,44,62`; `item_stats.rs:22,201`; `skill.rs:665-678,722,837,902` | Stats/item/skill formulas and DB2 field order desde C# | `Entities/Player/Player.cpp`; `Entities/Unit/StatSystem.cpp`; `DataStores/DB2Stores.*`; `DataStores/DB2Structure.h`; `DataStores/DB2LoadInfo.h`; `ObjectMgr.cpp`; `SpellMgr.cpp` | Parcialmente contrastado en `docs/audits/csharp-reference-contrast.md` pasada 15. No bug en el subset de enum/layout `ItemSparse`, DB2 `SkillLineAbility`/`SkillRaceClassInfo`, indice por `SkillupSkillLineID`, `GetSkillRaceClassInfo`, `GetSkillRangeType` y helper representado `skill_rewarded_spells_like_cpp`. Bugs confirmados en formulas de stats/proyeccion de `character.rs`, aplicacion parcial de stats de items y rutas legacy de starting skills/spells. Quedan pendientes otros usos de `wow-data` fuera de este slice. |
| `crates/wow-database/src/lib.rs:4`; `params.rs:8,95`; `result.rs:4,344`; `statements/login.rs:3`; `statements/character.rs:3` | DB API/statement enums imitan C# | `src/server/database/*`; prepared statement enums C++ | Menor si es API interna, pero no debe justificar cobertura ni nombres. |
| `crates/wow-core/src/guid.rs:329` | `HasEntry` always true por C# | `ObjectGuid`/`ObjectGuidFactory` C++ | Bug-marker core. Verificar semantics por tipo GUID. |
| `crates/wow-constants/src/lib.rs:9` | Opcodes/flags translated from C# RustyCore | C++ opcode enums and packet registries | Alto riesgo global. No confiar en constants sin audit contra C++. |
| `crates/wow-proto/src/lib.rs:62,240`; `proto/bgs/low/pb/client/rpc_types.proto:2` | Service hashes/proto desde C#/CypherCore | `src/server/proto/*`; generated BNet proto C++; packet captures si falta source | Bug-marker protocol. |
| `crates/wow-logging/src/lib.rs:39` | Logging categories mirror C# | C++ logging categories/config | Bajo riesgo runtime, pero no usar para protocolo. |

## Docs contaminadas o historicas

| Doc ref | Problema | C++ vs esperado | Veredicto |
| --- | --- | --- | --- |
| `docs/world-entry-implementation.md:8,192,482,579,654,657,748,770,786,842,865-898,919-998,1049,1062` | Declara C# como implementacion de referencia y compara Rust contra C# | `WorldSocket.cpp`; `WorldSession.cpp`; `AuthenticationPackets.cpp`; `UpdateData.cpp`; `Object.cpp`; `MovementPackets.cpp`; `CharacterHandler.cpp` | Documento contaminado. Usar solo como historial, no como autoridad. |
| `docs/phase4-analysis-and-fixes.md:15,63,91,146,166,197,218,226,246,257,301` | Afirma que "lo que hace C#" es correcto y pide reescrituras siguiendo C# | `CharacterPackets.*`; `CharacterHandler.cpp`; `ClientConfigPackets.*`; `SystemPackets.*`; `UpdateData.cpp`; `Object.cpp` | Documento contaminado. Hay que reauditar cada fix contra C++. |
| `docs/implementation-template.md:15,22,23,30,91` | Template fomenta "C# Reference" | Debe ser "C++ Reference" y fallback captura | Bug de proceso. Cambiar en una fase de docs, no en esta auditoria. |
| `docs/implementations/dynamic-visibility.md:13,17,24,86,87,121` | Visibilidad y OutOfRange descritos desde C# | `UpdateData.cpp:29-59`; `Object.cpp:251-282`; `Player.cpp:23346`; map/grid C++ | Contaminado. |
| `docs/implementations/move-init-active-mover-complete.md:11,15,21` | Active mover descrito desde C# | `MovementPackets.cpp`; `MovementHandler.cpp`; `Player.cpp` login/movement | Contaminado. |
| `docs/implementations/set-active-mover.md:15,21` | Set active mover descrito desde C# | `MovementPackets.cpp`; `MovementHandler.cpp` | Contaminado. |
| `docs/NPC_VENDOR_FLOW.md:1,3,5,36,40,49,68,69,85` | Vendor flow usa C# como diagnostico y solucion | `Handlers/ItemHandler.cpp:567-694`; `Server/Packets/NPCPackets.cpp:96-160`; `ItemPackets.cpp`; `NPCHandler.cpp:49-91,373-381` | Contaminado. Reauditar referencias negativas de vendor contra C++. |
| `docs/migration/movement.md:342` | Parser cita `PacketHandlerExtensions.Read` de TC C# port | `MovementPackets.cpp:129-169` y `ByteBuffer.h` | Bug-marker tecnico. |
| `docs/migration/bnetserver.md:227,254,485,494,576` | Mezcla C#/TC para TLS, RPC y REST; ya detecta un carry-over C# | `bnetserver/Main.cpp`; `SslContext.cpp`; `Server/Session.cpp`; `REST/LoginRESTService.cpp`; `RealmList.cpp` | Reauditar BNet desde C++. |
| `docs/migration/shared-networking.md:168,332,374,447,449,509-514,578` | Ya detecta misatribucion C# de counters | `WorldSocket.cpp`; `WorldPacketCrypt` | Util como aviso, pero las referencias de codigo siguen contaminadas. |
| `docs/migration/db-schemas.md:111,171,259,287`; `shared-datastores.md:134`; `globals.md:537,580` | DB/hotfix/loaders se comparan con C# | `database/*`; `DataStores/*`; `ObjectMgr.cpp`; prepared statement enums C++ | C# no debe marcar cobertura. |
| `docs/migration/current-session-handoff.md` e inventarios `docs/migration/inventory/*` | Historial contiene menciones C# y algunas prohibiciones correctas | C++ anchors del inventario por dominio | No usar como autoridad directa; sirve solo como log historico. |

## Docs que ya establecen la regla correcta

Estos ficheros contienen referencias a C#, pero son utiles porque declaran que no es autoridad:

- `CLAUDE.md:21-31`: C# es secundario; layouts/opcodes/reglas deben anclarse a C++ o captura.
- `docs/MIGRATION_ROADMAP.md:8,16`: C# es secundario y sospechoso por defecto.
- `docs/migration/claude-porting-instructions.md:82`: no mantener C# como autoridad.
- `docs/migration/login-world-load-cpp-parity-plan.md:21-23,104,125,168,220,359`: C++ authority.
- `docs/migration/creature-port-no-gaps-plan.md:17,73,105,112-133,319,451,479`: C# en Creature es bug-marker.
- `docs/migration/entities-creature.md:469`: reauditar C# contra C++.
- `docs/migration/pets.md:407,576`: C# incompleto; usar Trinity C++.
- `docs/migration/logging.md:314`: ignorar C# para protocolo.

## Indice C++ por dominio

Usar estas anclas antes de aceptar cualquier cambio derivado de una referencia C#.

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

## Siguiente fase recomendada

No desarrollar todavia. Convertir esta auditoria en issues por dominio:

1. Packet serializers: `wow-packet`.
2. Auth/network/crypto: `wow-network`, `wow-crypto`, `bnet-server`.
3. Login/world entry: `world-server`, `wow-world` character/misc.
4. Gameplay handlers: quest, loot, chat, combat, group, movement.
5. Data/DB: `wow-data`, `wow-database`, constants/proto.
6. Docs cleanup: reemplazar templates/docs contaminadas por C++ anchors despues de cada contraste.

Definition of done para cada fila:

- Se compara el Rust actual con el C++ indicado.
- Se documenta si coincide, difiere o necesita captura.
- Si coincide: se reemplaza la referencia C# por file:line C++.
- Si difiere: se abre bug de comportamiento con C++ anchor.
- No queda ninguna referencia C# como autoridad de orden, layout, opcode, constante o regla runtime.
