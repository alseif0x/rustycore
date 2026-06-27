# Contraste C# vs C++ de referencias auditadas

Fecha: 2026-06-27

Scope: contraste inicial de los focos de mayor riesgo del inventario
`docs/audits/csharp-reference-audit.md` y registro incremental de fixes
cerrados contra C++.

Regla usada:

- `bug confirmado`: el Rust observado difiere del C++ canonico.
- `bug de paridad configurable`: el layout por defecto coincide, pero Rust ignora
  configuracion que C++ usa.
- `no bug de comportamiento`: el Rust coincide con C++ en el slice revisado; queda
  deuda de comentario/documentacion si aun cita C#.
- `pendiente`: no hay suficiente contraste byte-for-byte en esta pasada.

## Veredictos verificados

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| ByteBuffer bits | `crates/wow-packet/src/world_packet.rs:391,410,419` | `src/server/shared/Packets/ByteBuffer.h:155-217`; `ByteBuffer.cpp:78-91` | No bug de comportamiento | `ResetBitPos`, `ReadBit`, `ReadBits`, `WriteBit` y `WriteBits` tienen equivalente C++. El problema es solo que los comentarios dicen C#. |
| Packet compression threshold | `crates/wow-packet/src/compression.rs:20-30`; `crates/wow-network/src/world_socket.rs:661-664,1194-1198,1333-1336`; test `compression_threshold_uses_payload_len_like_cpp` | `src/server/game/Server/WorldSocket.cpp:541-550`; `WorldSocket.cpp:582-609` | Bug corregido `#CSharpAudit.COMPRESS.1` | C++ comprime si `packet.size() > 0x400`, donde `packet.size()` es payload sin opcode. Rust ya resta el opcode antes de decidir, usa `COMPRESSION_THRESHOLD = 0x400` y el test cubre payload `0x3FF`, `0x400` y `0x401`. |
| Packet compression format | `crates/wow-packet/src/compression.rs:53-176` | `WorldSocket.cpp:549-609`; `WorldSocket.cpp:42-53` | No bug de formato en el slice revisado | El wrapper `[UncompressedSize, UncompressedAdler, CompressedAdler]`, opcode+payload como input, `Z_SYNC_FLUSH`, adler seed `0x9827D8F1` y stream persistente coinciden. |
| World crypto AES-GCM | `crates/wow-crypto/src/world_crypt.rs:14,33,69,100-170`; `crates/wow-network/src/world_socket.rs:937,955` | `src/common/Cryptography/Authentication/WorldPacketCrypt.cpp:33-83`; `AES.h:30-38` | No bug de comportamiento | C++ usa tag de 12 bytes, IV `[u64 counter][u32 magic]`, magic `0x544E4C43` para client recv y `0x52565253` para server send, y counters incrementados tambien pre-init. Rust replica esto; los comentarios C# son misatribucion. |
| FeatureSystemStatus in-game layout | `crates/wow-packet/src/packets/misc.rs:2034-2174` | `Server/Packets/SystemPackets.cpp:61-186`; `Handlers/CharacterHandler.cpp:1457-1485` | No bug de layout en defaults | El orden de campos, flags, QuickJoin, Squelch y EuropaTicket coincide con C++ para los valores dummy/default. |
| FeatureSystemStatus config | `crates/wow-packet/src/packets/misc.rs:2040-2204`; `crates/wow-world/src/handlers/character.rs:13453`; `crates/wow-world/src/session.rs:21365-21382`; `crates/world-server/src/main.rs:4595-4639,11639-11658`; tests `feature_system_status_uses_cpp_config_flags` | `Handlers/CharacterHandler.cpp:1457-1485`; `World.cpp:584-588`; `World.cpp:1597-1599` | Bug corregido `#CSharpAudit.FEATURE.1` | C++ rellena support tickets/bugs/complaints/suggestions, `CharUndelete`, `BpayStore` e `IsMuted = !CanSpeak()` desde config/sesion. Rust ahora propaga esos configs a `WorldSession`, construye `FeatureSystemStatus::from_config_like_cpp` y el test verifica los bits C++ de BPay, undelete, muted y EuropaTicket. |
| FeatureSystemStatusGlueScreen layout | `crates/wow-packet/src/packets/misc.rs:2175-2266` | `Server/Packets/SystemPackets.cpp:188-263`; `Handlers/AuthHandler.cpp:104-122` | No bug de layout en defaults | El orden de 27 flags, `EuropaTicketSystemStatus`, campos numericos y arrays opcionales coincide con C++ para defaults. |
| FeatureSystemStatusGlueScreen config | `crates/wow-packet/src/packets/misc.rs:2211-2303`; `crates/wow-world/src/session.rs:21384-21393,30912`; `crates/world-server/src/main.rs:4595-4639,11639-11658`; tests `feature_system_status_glue_screen_uses_cpp_config_fields` | `Handlers/AuthHandler.cpp:104-122`; `World.cpp:584-588`; `World.cpp:951-955`; `World.cpp:1239`; `World.cpp:1597-1599` | Bug corregido `#CSharpAudit.FEATURE.1` | C++ usa `CharactersPerRealm`, `Expansion`, `CharUndelete`, `BpayStore` y support toggles. Rust ahora usa `FeatureSystemStatusGlueScreen::from_config_like_cpp`, con `CharactersPerRealm`, `CONFIG_EXPANSION` y los toggles propagados desde `SessionResources`; el test lee los flags y enteros del payload. |
| AccountDataTimes / account data packets | `crates/wow-packet/src/packets/misc.rs:1508-1680`; `crates/wow-world/src/session.rs:30958-30966` | `Server/Packets/ClientConfigPackets.cpp:20-69`; `ClientConfigPackets.h:29-87`; `WorldSession.cpp:986-996` | No bug de comportamiento | Rust ya usa `NUM_ACCOUNT_DATA_TYPES = 15`, escribe GUID + ServerTime + 15 timestamps y lee/escribe UpdateAccountData como C++. La referencia C# previa queda obsoleta. |
| EnumCharactersResult serializer | `crates/wow-packet/src/packets/character.rs:132-233` | `Server/Packets/CharacterPackets.cpp:189-340`; `CharacterPackets.h:117-225`; `ByteBuffer.cpp:92-113` | No bug de layout en el slice revisado | El orden de flags, counts, CharacterInfo, VisualItems y RaceUnlock coincide con C++. `ByteBuffer::append()` hace `FlushBits()`, igual que los writes primitivos Rust. Puede haber gaps funcionales de datos, pero no un bug C# de layout en este slice. |
| Character response codes | `crates/wow-packet/src/packets/character.rs:697-710` | `SharedDefines.h:6131-6229`; `CharacterPackets.cpp:365-370` | No bug en los codigos revisados | Los valores Rust visibles para create/delete/login coinciden con C++ `ResponseCodes`. El comentario C# debe cambiarse a `SharedDefines.h`. |
| Movement SetActiveMover | `crates/wow-packet/src/packets/movement.rs:1311-1324` | `MovementPackets.cpp:825-827`; `MovementPackets.h:440-447` | No bug de comportamiento | C++ lee `ObjectGuid ActiveMover`; Rust lee `packed_guid`. En este proyecto `ObjectGuid` via ByteBuffer usa el formato packed del stream. Comentario C# es deuda. |
| Movement MoveInitActiveMoverComplete | `crates/wow-packet/src/packets/movement.rs:1331-1362` | `MovementPackets.cpp:1094-1097`; `MovementPackets.h:703-708`; `MovementHandler.cpp:810-813` | No bug de parser | C++ lee `uint32 Ticks`; Rust lee `u32 ticks`. Comentario C# es deuda. |
| BNet REST form cookie | `crates/bnet-server/src/rest/handlers.rs:254-264`; test `login_form_headers_do_not_set_cookie_like_cpp` | `src/server/bnetserver/REST/LoginRESTService.cpp:181-188` | Bug corregido `#CSharpAudit.BNETREST.1` | C++ responde `GET /bnetserver/login/` seteando solo `Content-Type: application/json;charset=utf-8` y body JSON. Rust ya no genera `JSESSIONID` en el form inicial; el test comprueba que los headers no contienen `Set-Cookie`. |
| BNet REST SRP challenge cookie/session | `crates/bnet-server/src/rest/handlers.rs:17-25,312-319,518-531`; `crates/bnet-server/src/rest/mod.rs:138-171`; tests `headers_do_not_set_cookie_like_cpp` | `LoginRESTService.cpp:411-498` | Bug corregido `#CSharpAudit.BNETREST.2` | C++ guarda `Srp` en `session->GetSessionState()` del `LoginHttpSession` y no emite cookie en el challenge. Rust ya guarda `BnetSrp6` en `RestConnectionState`, que vive por conexion TLS, elimina `AppState.rest_sessions` y responde el challenge solo con `Content-Type`. |
| BNet wrong password body | `crates/bnet-server/src/rest/handlers.rs:882-891` | `LoginRESTService.cpp:279-289`; `LoginRESTService.cpp:446-452` | No bug en el slice revisado | C++ devuelve `authentication_state=DONE` sin error para cuenta/password invalidos. Rust hace lo mismo. Comentario C# debe reanclarse a C++. |
| ObjectGuid `has_entry()` | `crates/wow-core/src/guid.rs:328-331`; `guid.rs:546` | `Entities/Object/ObjectGuid.h:289-330` | Pendiente / no concluyente | C++ no tiene `HasEntry`; tiene `GetEntry()` y format traits por `HighGuid`. En Rust solo se usa para `Display`, no se encontro uso en layout/runtime. No marcar como bug runtime hasta que aparezca un uso funcional. |

## Pasada 2: subsistema BNet REST / SRP

Esta pasada trata `crates/wow-crypto/src/bnet_srp6.rs`,
`crates/bnet-server/src/rest/handlers.rs` y `rest/types.rs` como un
subsistema, no como comentarios sueltos. El contraste es estatico contra
TrinityCore C++; no se ha usado captura live.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| SRP constantes N/g | `crates/wow-crypto/src/bnet_srp6.rs:25-49` | `SRP6.cpp:30-34` | No bug | Los modulos v1/v2 y `g=2` coinciden byte-for-byte con C++. |
| SRP verifier almacenado | `bnet_srp6.rs:115-128` | `BigNumber.h:37,46`; `BigNumber.cpp:56`; `SRP6.cpp:36-38` | No bug | Rust lee `verifier` little-endian; C++ `BigNumber(std::vector<uint8>)` usa little-endian por defecto. La referencia a C# es misatribucion. |
| SRP `B = g^b + k*v` | `bnet_srp6.rs:130-132`; `bnet_srp6.rs:412-417` | `SRP6.cpp:67-70` | No bug | La formula y modulo `N` coinciden. |
| SRP `x` v1/v2 | `bnet_srp6.rs:364-400` | `SRP6.cpp:194-216` | No bug en algoritmo | v1 usa `SHA256(salt || SHA256(username:password))` interpretado little-endian por `BigNumber`. v2 usa PBKDF2-HMAC-SHA512 15000 y replica la correccion de signo antes de `% (N-1)`. Queda separado el bug de normalizacion de username. |
| SRP evidence `M1/M2` | `bnet_srp6.rs:199-206`; `bnet_srp6.rs:431-456` | `SRP6.cpp:157-186` | No bug en evidence padding | C++ `GetBrokenEvidenceVector` usa `(bits+8)>>3` y big-endian. Rust replica ese padding para `M1`/`M2`. |
| SRP v1 iterations | `crates/wow-crypto/src/bnet_srp6.rs:72,154-157`; test `challenge_v1_iterations_match_cpp` | `SRP6.h:165-166`; `LoginRESTService.cpp:468-470` | Bug corregido `#CSharpAudit.BNETSRP.1` | C++ `BnetSRP6v1Base::GetXIterations()` devuelve `1` y `LoginRESTService` serializa ese valor. Rust ahora devuelve `iterations=1` para v1 y conserva `15000` para v2. |
| SRP `u = H(A || B)` padding | `crates/wow-crypto/src/bnet_srp6.rs:189,419-445`; tests `compute_u_v1_pads_a_and_b_to_128_bytes_like_cpp`, `compute_u_v2_pads_a_and_b_to_256_bytes_like_cpp` | `SRP6.h:203-205`; `SRP6.h:226-228`; `BigNumber.h:127-133`; `BigNumber.cpp:189-193` | Bug corregido `#CSharpAudit.BNETSRP.2` | C++ hashea `A.ToByteArray<128>(false) || B.ToByteArray<128>(false)` en v1 y `A.ToByteArray<256>(false) || B.ToByteArray<256>(false)` en v2. Rust ahora paddea ambos operandos a ancho fijo antes del hash; los tests cubren ceros iniciales y prueban que el hash no usa bytes minimos. |
| SRP `k` v2+SHA512 | `crates/wow-crypto/src/bnet_srp6.rs:329-336,408-414`; test `compute_k_v2_sha512_pads_generator_to_256_bytes_like_cpp` | `SRP6.h:196`; `SRP6.h:218-219`; `BigNumber.h:127-133`; `BigNumber.cpp:189-193` | Bug corregido `#CSharpAudit.BNETSRP.3` | C++ calcula `k` con `N.ToByteArray<128/256>(false) || g.ToByteArray<128/256>(false)` para cualquier hash. Rust ahora usa el mismo ancho fijo tambien para v2+SHA512; el test demuestra que no usa `SHA512(N || g)` con bytes minimos. |
| SRP username normalizacion | `bnet_srp6.rs:270-273`; `handlers.rs:366-367`; `handlers.rs:476-477` | `LoginRESTService.cpp:273-274`; `LoginRESTService.cpp:292-302`; `LoginRESTService.cpp:429-456`; `Util.h:280-282`; `Util.cpp:795-803` | Bug confirmado de borde | C++ aplica `Utf8ToUpperOnlyLatin`; Rust usa `str::to_uppercase()` Unicode completo, y ademas lo aplica en handler y helper. ASCII coincide; no-Latin puede producir otro hash SRP/login DB key. |
| SRP challenge cuenta inexistente | `crates/bnet-server/src/rest/handlers.rs:482-535`; test `srp_challenge_missing_account_returns_done_like_cpp` | `LoginRESTService.cpp:443-449` | Bug corregido `#CSharpAudit.BNETREST.3` | C++ responde JSON `authentication_state=DONE` con `Content-Type: application/json;charset=utf-8`, sin revelar si la cuenta existe. Rust ya devuelve el mismo shape/status mediante `srp_challenge_missing_account_response_like_cpp`. |
| SRP challenge hex casing | `handlers.rs:525-533`; `handlers.rs:902-904` | `LoginRESTService.cpp:471-488`; `Util.cpp:849-866` | Bug de paridad textual | C++ usa `AsHexStr()`/`ByteArrayToHexStr()` uppercase. Rust serializa `modulus`, `generator`, `salt` y `public_b` con hex lowercase. El parse del cliente puede tolerarlo, pero no es 1:1. |
| Login ticket hex casing | `handlers.rs:765-769`; `handlers.rs:902-904` | `LoginRESTService.cpp:387-388`; `Util.cpp:849-866` | Bug de paridad textual | C++ genera `TC-` + 20 bytes uppercase. Rust genera `TC-` + 20 bytes lowercase. |
| Login directo error invalido | `handlers.rs:383`; `handlers.rs:440-449`; `handlers.rs:882-891` | `LoginRESTService.cpp:279-289`; `LoginRESTService.cpp:374-383` | No bug | Aunque el comentario cita C#, Rust termina usando `error_result()` con `DONE` y campos nulos, igual que C++. |

## Pasada 3: subsistema auth RSA / Ed25519

Contraste de `crates/wow-crypto/src/rsa_sign.rs`,
`crates/wow-crypto/src/ed25519ctx.rs`, `crates/wow-network/src/world_socket.rs`
y `crates/wow-packet/src/packets/auth.rs` contra C++.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| RSA ConnectTo key | `crates/wow-crypto/src/rsa_sign.rs:30-128` | `AuthenticationPackets.cpp:219-245` | No bug | Comparacion local con `openssl rsa -noout -modulus`: modulus C++ PEM y `RSA_MODULUS` Rust son iguales, 256 bytes, SHA256 `aff218d55ccc6b40f61bd34a4c46f42d2a4961c49aebd851c4f6eb7a34490397`. |
| RSA ConnectTo signBuffer | `rsa_sign.rs:223-239`; `wow-world/src/handlers/character.rs:3259-3265` | `AuthenticationPackets.cpp:279-306`; `RSA.cpp:464-493` | No bug para IPv4/IPv6 soportado | C++ firma `whereBuffer || uint32(type) || uint16(port)` con SHA256 PKCS#1 v1.5 y revierte la firma. Rust construye el mismo buffer y revierte la firma. C++ tambien enumera `NamedSocket`, pero comenta que no esta soportado por cliente Windows; Rust no implementa esa variante y no se usa en el flujo actual. |
| ConnectTo packet layout | `wow-packet/src/packets/auth.rs:415-443` | `AuthenticationPackets.cpp:308-313` | No bug | Orden `signature`, `whereBuffer`, `port`, `serial`, `con`, `key` coincide. |
| EnterEncryptedMode input HMAC | `wow-network/src/world_socket.rs:66-72`; `world_socket.rs:1296-1300` | `AuthenticationPackets.cpp:347-354` | No bug | Constantes `EnableEncryptionSeed` y `EnableEncryptionContext` coinciden; ambos firman `HMAC_SHA256(encrypt_key, [Enabled] || EnableEncryptionSeed)`. |
| EnterEncryptedMode Ed25519ctx | `wow-crypto/src/ed25519ctx.rs:22-98`; `world_socket.rs:1302-1307` | `Ed25519.cpp:135-148`; `curve25519.c:5434-5529`; `AuthenticationPackets.cpp:356-363` | No bug | C++ llama `ED25519_sign_ctx`; en el vendor local `Ed25519ctx = 0` y dom2 es `SigEd25519 no Ed25519 collisions || type || context_len || context`, seguido de `az[32..] || message` y luego `R || public_key || message`. Rust implementa el mismo dom2, clamp, nonce/challenge y salida `R || s`. |
| EnterEncryptedMode packet layout | `wow-packet/src/packets/auth.rs:325-337` | `AuthenticationPackets.cpp:361-363` | No bug | Ambos escriben 64 bytes de firma, bit `Enabled`, y flush de bits. |

## Pasada 4: `wow-packet` spell serializers

Contraste de las referencias C# de `crates/wow-packet/src/packets/spell.rs`
contra `SpellPackets.cpp/h`. Esta seccion cubre layout/parse de paquetes; el
runtime de `crates/wow-world/src/handlers/spell.rs` queda separado.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `SpellTargetData` read/write | `spell.rs:319-420` | `SpellPackets.cpp:160-188`; `SpellPackets.cpp:295-320` | No bug de layout | Mismo orden: reset bits, `Flags` 28 bits, bits de `Src/Dst/Orientation/MapID`, `Name` 7 bits, `Unit`, `Item`, opcionales y string. |
| `CastSpellRequest` parser | `spell.rs:444-535` | `SpellPackets.cpp:216-256` | No bug de parser | Mismo orden de `CastID`, `Misc[0..1]`, `SpellID`, `Visual`, trajectory, `CraftingNPC`, counts, currencies, bitfield `SendCastFlags/MoveUpdate/Weight/CraftingOrderID`, target, order, reagents, movement y weights. |
| `SpellCastData` minimo | `spell.rs:581-655`; `spell.rs:692-750` | `SpellPackets.cpp:374-431`; `SpellPackets.h:344-367` | No bug en defaults minimos | Para counts/opcionales cero, Rust escribe los campos en el mismo orden que C++: guids, spell/visual, flags, cast time, missile, dest index, immunities, prediction, counts/bits, target, hit targets, y log bit falso en `SpellGo`. No cubre ramas C++ con miss targets, remaining power, runes, target points o ammo. |
| `CastFailed` | `spell.rs:758-778` | `SpellPackets.h:449-461`; `SpellPackets.cpp:514-523` | Bug confirmado | C++ escribe `CastID`, `SpellID`, `SpellCastVisual`, `Reason`, `FailedArg1`, `FailedArg2`. Rust omite `SpellCastVisual`, por lo que `Reason` y argumentos quedan desplazados. |
| `CooldownEvent` | `spell.rs:786-798` | `SpellPackets.h:511-518`; `SpellPackets.cpp:576-582` | No bug | Ambos escriben `SpellID`, bit `IsPet`, flush. |
| `SpellCooldownPkt` | `spell.rs:803-838` | `SpellPackets.h:559-577`; `SpellPackets.cpp:618-634` | No bug | Ambos escriben `Caster`, `Flags`, count, y por entrada `SrecID`, `ForcedCooldown`, `ModRate`. |

## Pasada 5: `wow-packet` combat serializers

Contraste de las referencias C# de `crates/wow-packet/src/packets/combat.rs`.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `AttackStart` / `SAttackStop` | `combat.rs:63-101` | `CombatPackets.cpp:26-51`; `CombatPackets.h:61-80` | No bug | Mismo orden `Attacker`, `Victim`; `SAttackStop` anade bit `NowDead` y flush. |
| `AIReaction` / `CancelCombat` / `BreakTarget` | `combat.rs:107-147` | `CombatPackets.cpp:140-147`; `CombatPackets.h:130-141` | No bug | `CancelCombat` no tiene payload; `AIReaction` escribe guid y reaction; `BreakTarget` escribe guid. |
| `AttackerStateUpdate` sub-buffer base | `combat.rs:191-215` | `CombatLogPackets.cpp:346-405`; `CombatLogPackets.h:303-324` | No bug en campos internos del caso simple | Para `SubDmg=false` y sin flags condicionales de block/rage/unk, el contenido interno coincide: hit info, attacker/victim, damage/original/over, SubDmg flag, victim state, attacker state, melee spell id y `ContentTuning`. |
| `AttackerStateUpdate` log bit / outer layout | `combat.rs:217-224` | `CombatLogPackets.cpp:407-412`; `CombatLogPacketsCommon.h:128-137` | Bug confirmado | C++ escribe `WriteLogDataBit(false)` y `FlushBits()` en el paquete exterior antes de `uint32(attackRoundInfo.size())`. Rust escribe ese bit dentro del sub-buffer `attackRoundInfo` y luego manda exterior como `uint32(size) + bytes`, por lo que el layout no es C++ 1:1. |

## Pasada 6: chat serializers y handler text-emote

Contraste de las referencias C# de `crates/wow-packet/src/packets/chat.rs`
y de las referencias runtime de `crates/wow-world/src/handlers/chat.rs`.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `ChatPkt` layout | `chat.rs:617-702` | `ChatPackets.cpp:189-226`; `ChatPackets.h:170-205` | No bug de layout | Rust escribe el mismo orden de campos, longitudes de bits, flags, strings y optionals deshabilitados. Deuda: el comentario debe citar C++, no C#. |
| Emote packet layouts | `chat.rs:817-906` | `ChatPackets.h:210-244`; `ChatPackets.cpp:229-260` | No bug de layout | `EmoteClient` no lee payload; `CTextEmote`, `STextEmote` y `EmoteMessage` coinciden en orden de GUIDs, ids, count, sequence variation y visual-kit ids. |
| Chat listen ranges | `wow-world/src/handlers/chat.rs:12-44`; broadcast users in `chat.rs` | `World.cpp:1323-1325`; `worldserver.conf.dist:1906-1926`; `ChatHandler.cpp:719`; `Player.cpp:21073-21114` | Bug de paridad configurable | Rust hardcodea 25/300/25 desde C#. C++ lee `ListenRange.Say`, `ListenRange.TextEmote`, `ListenRange.Yell` de config; la config distribuida pone Say/TextEmote en 40 y Yell en 300. |
| `HandleEmote` runtime | `wow-world/src/handlers/chat.rs:821-827` | `ChatHandler.cpp:664-672` | Bug runtime / incompleto | C++ valida vivo/no `UNIT_STATE_DIED`, dispara script `OnPlayerClearEmote` y setea `EMOTE_ONESHOT_NONE`. Rust solo parsea/loguea. |
| `HandleTextEmote` runtime | `wow-world/src/handlers/chat.rs:829-884` | `ChatHandler.cpp:674-732` | Bug runtime / incompleto | C++ valida `EmotesTextStore`, traduce `EmoteID` a animacion, maneja estados dance/read/sit/etc, scripts, criteria, AI `ReceiveEmote`, rango configurado y aura interrupts. Rust envia `STextEmote` y `EmoteMessage` directo con raw `EmoteID`. |

## Pasada 7: party/group serializers e invite lifecycle

Contraste de la referencia C# de `crates/wow-packet/src/packets/party.rs`
y del handler `crates/wow-world/src/handlers/group.rs:809` contra
`PartyPackets.cpp/h` y `GroupHandler.cpp`. Esta pasada cubre layouts de
paquetes de party implementados y el ciclo invite/accept/leave; otros handlers
de grupo quedan como pendientes aunque varios ya tienen helpers `like_cpp`.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `PartyInviteClient` parser | `wow-world/src/handlers/group.rs:819-860` | `PartyPackets.cpp:45-60` | No bug de parser | El handler lee bit `hasPartyIndex`, reset, `TargetNameLen`/`TargetRealmLen` de 9 bits, `ProposedRoles`, `TargetGUID`, strings y party index opcional en el mismo orden que C++. |
| CMSG party parsers | `party.rs:37-555` | `PartyPackets.cpp:101-121`; `250-310`; `355-378`; `418-426`; `532-554`; `786-790` | No bug de layout en los parsers revisados | `ConvertRaid`, `PartyUninvite`, `SetLeader`, `SetAssistant`, `SetEveryoneIsAssistant`, `SetAssignment`, `SetRole`, `InitiateRolePoll`, `UpdateRaidTarget`, `RequestPartyJoinUpdates`, `RequestPartyMemberStats`, ready check, swap, loot, opt-out, minimap y silence siguen el orden C++ de bits, guids, enteros y opcionales. |
| SMSG party serializers base | `party.rs:616-629`; `684-944`; `951-1189` | `PartyPackets.cpp:29-40`; `126-129`; `240-248`; `277-285`; `313-349`; `381-413`; `429-437`; `446-526`; `562-574` | No bug de layout en los serializers revisados | `PartyCommandResult`, `GroupDecline`, ready check, role inform, raid target updates, raid markers, `GroupNewLeader`, `PartyUpdate`, `PartyMemberFullState` y estructuras auxiliares mantienen el orden C++. Los writes primitivos Rust flushean bits igual que `ByteBuffer::append()`. |
| `PartyInviteServer` layout | `party.rs:642-673` | `PartyPackets.cpp:62-85`; `AuthenticationPackets.cpp:26-45` | No bug de layout, pero valores runtime incorrectos | El orden wire coincide para la variante implementada: flags, `VirtualRealmInfo`, inviter guids, `Unk1`, roles, LFG counts, name y slots. El bug esta en los valores que alimenta el handler. |
| `PartyInviteServer` valores | `group.rs:840`; `group.rs:943-950`; `party.rs:650`; `party.rs:669` | `PartyPackets.cpp:88-98`; `PartyPackets.h:71`; `GroupHandler.cpp:181` | Bug confirmado | C++ propaga `packet.ProposedRoles`, `InviterBNetAccountId = session account GUID`, y realm actual/normalizado; `AllowMultipleRoles` queda false por defecto. Rust ignora `_proposed_roles`, manda BNet GUID vacio, realm strings vacios, `AllowMultipleRoles=true` y `ProposedRoles=0`. |
| `HandlePartyInviteOpcode` validaciones | `group.rs:819-961` | `GroupHandler.cpp:58-184` | Bug runtime / incompleto | C++ valida GM, faccion, instancia/dificultad, ignore/social, level restriction, target ya en grupo/invite, permisos de lider/asistente y `group->IsFull()` antes de enviar invite. Rust solo valida existencia, self-invite, pending invite y un full-check parcial del grupo del invitador. |
| PartyIndex invite/accept/leave | `group.rs:856-858`; `group.rs:973-988`; `group.rs:1293-1298`; `group.rs:1318` | `GroupHandler.cpp:114-120`; `GroupHandler.cpp:189-195`; `GroupHandler.cpp:335-357` | Bug confirmado | Rust lee `PartyIndex` en invite/accept/leave pero lo descarta o resuelve con `None`. C++ usa `packet.PartyIndex` para `GetGroup(...)`, `GetGroupCategory()` y leave. En Rust un `PartyIndex` no-HOME puede operar sobre HOME o no rechazar la categoria incorrecta. |
| `HandlePartyInviteResponseOpcode` lifecycle | `group.rs:973-1118` | `GroupHandler.cpp:187-238` | Bug runtime / incompleto | C++ usa `GetGroupInvite()`, quita invite, rechaza self-accept, revisa `IsFull()`, crea grupo con lider online y llama `AddMember()`/`BroadcastGroupUpdate()`. Rust modela un `pending_invites` target->inviter, no revalida full/self/category al aceptar y crea/extiende grupo por busqueda del invitador. |
| `HandleLeaveGroupOpcode` lifecycle | `group.rs:1293-1430` | `GroupHandler.cpp:335-357` | Bug runtime / incompleto | C++ maneja grupo real o invite pendiente, respeta `PartyIndex`, rechaza battleground y envia `SendPartyResult(PARTY_OP_LEAVE, OK)` antes de remover/disband. Rust ignora el `PartyIndex` leido, no cubre cancelacion de pending invite y no envia `PartyCommandResult` OK de leave. |
| `SetLootMethod` handler | `group.rs:2550-2554` | `GroupHandler.cpp:364-394` | No bug de comportamiento | C++ tiene el cambio de loot comentado como "not allowed to change". Rust solo parsea y retorna, equivalente al comportamiento efectivo. |

## Pasada 8: item/inventory packet layouts y handlers

Contraste de `crates/wow-packet/src/packets/item.rs:354,364` y de las
referencias inventory en `crates/wow-world/src/handlers/character.rs` contra
`ItemPackets.cpp/h`, `ItemPacketsCommon.cpp/h`, `ItemHandler.cpp` y constantes
de `Player.h`.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `ItemInstance`, `ItemBonuses`, `ItemModList` | `item.rs:19-89` | `ItemPacketsCommon.cpp:91-181`; `ItemPacketsCommon.h:33-75` | No bug de layout | Orden `ItemID`, random seed/id, bit `ItemBonus`, `Modifications` con count de 6 bits y bonus opcional coincide. Los casts signed/unsigned reproducen el write C++. |
| Purchase/time/enchant serializers | `item.rs:181-342` | `ItemPackets.cpp:69-116`; `ItemPackets.cpp:137-143`; `ItemPackets.cpp:331-339` | No bug de layout | `GetItemPurchaseData`, `SetItemPurchaseData`, refund result, expire refund, item time update e enchant time update coinciden en orden de GUIDs, enteros, bit opcional y contents. |
| `InvUpdate` | `item.rs:354-376` | `ItemPacketsCommon.cpp:247-258`; `ItemPacketsCommon.h:106-117` | No bug de parser | C++ lee count de 2 bits, reset bit pos, y pares `ContainerSlot`, `Slot`. Rust hace lo mismo; la referencia C# es deuda de comentario. |
| `SwapInvItem`, `AutoEquipItem`, `AutoEquipItemSlot`, `SwapItem`, `DestroyItem`, `CancelTempEnchantment` parsers | `item.rs:380-455`; `item.rs:467-538`; `item.rs:541-559` | `ItemPackets.cpp:193-224`; `ItemPackets.cpp:276-279`; `ItemPackets.cpp:304-308` | No bug de parser para esos packets | El orden C++ de `Inv`, slots, GUIDs y `Count/ContainerId/SlotNum` coincide. La nota previa sobre `OpenItem` se revalido aparte: C++ `OpenItem::Read()` lee `Slot` y luego `PackSlot`, igual que Rust en el handler correspondiente. |
| `AutoStoreBagItem` parser | `item.rs:486-513` | `ItemPackets.cpp:212-218`; `ItemHandler.cpp:707-734` | Bug confirmado | C++ lee `Inv`, `ContainerSlotB`, `ContainerSlotA`, `SlotA`; despues usa `ContainerSlotA` como source y `ContainerSlotB` como destino. Rust lee primero `container_slot_a` y luego `container_slot_b`, invirtiendo source/destination para cualquier payload real. |
| `InventoryChangeFailure` | `item.rs:562-639` | `ItemPackets.cpp:153-178`; `Player.cpp` `SendEquipError` callers | No bug de serializer en ramas revisadas | Rust escribe `BagResult`, dos GUIDs, `ContainerBSlot`, y los payloads condicionales de level, bind confirm y limit category en el mismo orden C++. |
| Inventory slot constants | `character.rs:12235-12241`; `wow-entities/src/player.rs:760-765` | `Player.h:621`; `Player.h:671-684` | No bug de constantes base | `INVENTORY_SLOT_BAG_0=255`, bag slots `30..34` e item slots `35..59` coinciden con C++. El comentario C# debe reemplazarse por `Player.h`. |
| `HandleSwapInvItemOpcode` runtime | `character.rs:11604-11753` | `ItemHandler.cpp:69-112`; `Player.cpp:12295-12530` | Bug runtime / incompleto | C++ exige `Inv.Items.size()==2`, valida `IsValidPos`, bank access, y delega en `Player::SwapItem`. Rust ignora `InvUpdate`, solo valida rango numerico, no aplica checks de bank/posicion ni la logica completa de `SwapItem`. |
| `HandleAutoEquipItemOpcode` runtime | `character.rs:11755-11830` | `ItemHandler.cpp:175-291` | Bug runtime / incompleto | C++ exige `Inv.Items.size()==1`, usa `CanEquipItem`, `CanUnequipItem`, `CanStoreItem`/`CanBankItem`, mueve items con `RemoveItem`/`EquipItem`/`StoreItem`, auto-unequip offhand y auras dependientes. Rust calcula un slot por `InventoryType` y delega a un swap simplificado. |
| `HandleAutoEquipItemSlotOpcode` runtime | `character.rs:11832-11889` | `ItemHandler.cpp:114-127`; `Player.cpp:12295-12530` | Bug runtime / incompleto | C++ permite source packed pos real desde `InvUpdate` y llama `Player::SwapItem`. Rust limita a `INVENTORY_SLOT_BAG_0` y no cubre nested bag/container parity. |
| `HandleSwapItem` / `HandleAutoStoreBagItemOpcode` runtime | `character.rs:11891-11976` | `ItemHandler.cpp:130-173`; `ItemHandler.cpp:699-743` | Bug runtime / incompleto | C++ soporta posiciones container-aware, valida `Inv.Items` esperado, `IsValidPos`, bank access, `CanStoreItem` y `StoreItem`. Rust solo soporta `container=255`, envia `InternalBagError` para nested bags y busca un backpack slot libre por cuenta propia. |
| `HandleDestroyItemOpcode` runtime | `character.rs:11978-12090` | `ItemHandler.cpp:294-327`; `Player.cpp` `DestroyItemCount`/`DestroyItem` | Bug runtime / incompleto | C++ opera sobre cualquier `(ContainerId, SlotNum)`, valida `CanUnequipItem` para equipment/bag pos, item existence y `ITEM_FLAG_NO_USER_DESTROY`, y luego `DestroyItemCount` o `DestroyItem`. Rust rechaza cualquier container distinto de 255 y solo modela direct inventory, aunque si soporta partial/full stack dentro de ese limite. |
| `HandleCancelTempEnchantmentOpcode` runtime | `character.rs:12093-12128` | `ItemHandler.cpp:1100-1120` | No bug en slice revisado | Ambos validan equipment slot, item existente y enchant temporal antes de limpiar/remover. Rust usa helper de aplicacion de enchantment; no se ha revisado toda la cascada de stats/auras fuera de esta ruta. |

## Pasada 9: misc serializers concretos

Contraste parcial de referencias C# de `crates/wow-packet/src/packets/misc.rs`
y handlers asociados. Esta pasada no cubre todo `misc.rs`; solo los bloques de
dificultad, hotfix, movement-transfer, played-time/taxi/cemetery/auction,
query-time, XP/level y show-trade-skill.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| Dungeon/Raid difficulty packets | `misc.rs:4534-4567` | `MiscPackets.cpp:250-263`; `Player.cpp:20669-20683`; `CharacterHandler.cpp:1113` | No bug de layout | C++ escribe `int32 DifficultyID` y raid anade `uint8 Legacy`. Rust coincide. La fuente debe ser `Player::SendDungeonDifficulty/SendRaidDifficulty`, no C#. |
| Hotfix `DBReply` / `HotfixConnect` | `misc.rs:4586-4755` | `HotfixPackets.cpp:28-55`; `HotfixPackets.cpp:67-78`; `HotfixPackets.cpp:105-125`; `HotfixPackets.h:46-93` | No bug de layout | Orden `TableHash`, `RecordID`, `Timestamp`, status de 3 bits, size/data y hotfix records `PushID`, `UniqueID`, `TableHash`, `RecordID`, size/status coincide. |
| `SetSpellModifier` empty | `misc.rs:4767-4805` | `SpellPackets.cpp:555-564`; `SpellPackets.h:490-505` | No bug en caso empty | C++ escribe `uint32 Modifiers.size()` y entradas si existen. Rust solo implementa `flat_empty/pct_empty` con count cero; layout correcto para fresh characters sin modificadores, no cubre entradas no vacias. |
| `SetProficiency` wire layout | `misc.rs:4809-4826` | `ItemPackets.cpp:145-151`; `ItemPackets.h:197-206` | No bug de layout | C++ escribe `uint32 ProficiencyMask`, `uint8 ProficiencyClass`; Rust coincide. |
| `SetProficiency::default_*` masks | `misc.rs:4831-4883`; `session.rs:14084-14090` | `SpellEffects.cpp:1785-1805`; `Player.cpp:244-245`; `Player.cpp:11093-11143`; `Player.cpp:21780-21787` | Bug confirmado / fuente C# no canonica | El packet layout es correcto, y `EffectProficiency` Rust envia masks acumulados como C++. Pero C++ no tiene tabla fija de proficiencies por clase: inicia `m_WeaponProficiency/m_ArmorProficiency=0`, OR-ea masks por spells, y `CanUseItem` valida required skill/spell. Rust usa `default_weapons(class)` como tabla C# en un gate de item visual/collection. Ver pasada 12. |
| `SuspendToken` / `ResumeToken` / `NewWorld` / `TransferPending` | `misc.rs:4891-5137` | `MovementPackets.cpp:657-674`; `MovementPackets.cpp:696-703`; `MovementPackets.cpp:1005-1027`; `MovementPackets.h:236-283`; `MovementPackets.h:623-651` | No bug de layout | Orden de map id, posiciones, flags opcionales, ship/spell, sequence index y reason bits coincide. Rust flushea bits antes de opcionales; C++ hace flush implicito al primer append byte y explicito al final, resultando el mismo wire. |
| `ShowTradeSkill` request/response | `misc.rs:5242-5310`; `character.rs:13786-13823` | `MiscHandler.cpp:1416-1419`; `Opcodes.cpp:2095` | Bug confirmado | C++ registra `CMSG_SHOW_TRADE_SKILL` como `WorldPackets::Null` y solo loguea; `SMSG_SHOW_TRADE_SKILL_RESPONSE` figura `STATUS_UNHANDLED`. Rust parsea payload y envia un response inventado desde estructura C#, por tanto no es paridad C++. |
| `PlayedTime` | `misc.rs:5889-5910`; `character.rs:3290-3308` | `CharacterPackets.cpp:567-581`; `MiscHandler.cpp:802-808` | No bug de layout | C++ lee el bit `TriggerScriptEvent`, responde `TotalTime`, `LevelTime`, bit `TriggerEvent` y flush. Rust escribe el mismo payload. |
| `TaxiNodeStatusPkt` | `misc.rs:5913-5930`; `handlers/misc.rs:2395` | `TaxiPackets.cpp:20-33`; `TaxiHandler.cpp:45-67` | No bug de layout | C++ escribe `Unit` y status de 2 bits. Rust coincide. El runtime de elegibilidad taxi no queda certificado en esta fila. |
| `RequestCemeteryListResponse` | `misc.rs:5933-5962`; `handlers/misc.rs:2214` | `MiscPackets.cpp:295-305`; `MiscHandler.cpp:367-397` | No bug de layout | C++ escribe bit `IsGossipTriggered`, flush, count y cemetery ids. Rust coincide. |
| `AuctionHelloResponse` | `misc.rs:5965-6002`; `character.rs:8732-8765` | `AuctionHousePackets.cpp:622-631`; `AuctionHousePackets.h:474-485`; `AuctionHouseHandler.cpp:995-1010` | Bug confirmado | C++ escribe `Guid`, `PurchasedItemDeliveryDelay`, `CancelledItemDeliveryDelay`, bit `OpenForBusiness`; no serializa `AuctionHouseID`. `SendAuctionHello` no setea delays, por defecto quedan `0`. Rust inserta `auction_house_id` antes del bit y su helper `open()` usa delays `3_600_000`, desplazando el layout y valores. |
| `QueryTimeResponse` | `misc.rs:6290-6305`; `handlers/misc.rs:2959` | `QueryPackets.cpp:411-416`; `QueryHandler.cpp:63-68` | No bug de layout | C++ escribe `CurrentTime`; Rust escribe `i64 current_time`. |
| `LogXpGain`, `ExplorationExperience`, `LevelUpInfo` | `misc.rs:12964-13066` | `CharacterPackets.cpp:617-626`; `MiscPackets.cpp:390-411`; `MiscPackets.h:519-531`; `SharedDefines.h:311` | No bug de layout | `LogXPGain` y `ExplorationExperience` coinciden. `LevelUpInfo` escribe `Level`, `HealthDelta`, `MAX_POWERS_PER_CLASS=10` powers, `MAX_STATS=5` stats y `NumNewTalents`; Rust coincide con C++ aunque el constructor C++ reserve 60 bytes. |

## Pasada 10: character enum helpers y logout

Contraste de referencias C# explicitas en
`crates/wow-world/src/handlers/character.rs` que no quedaron cubiertas por
character serializer, played-time o inventory.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| Equipment cache parse | `character.rs:1348-1371` | `CharacterPackets.cpp:178-185`; `CharacterPackets.cpp:188-197`; `Player.cpp:19468`; `Player.cpp:19603` | No bug de layout/parser | C++ guarda 5 campos por slot (`InvType`, `DisplayID`, `DisplayEnchantID`, `Subclass`, `SecondaryItemModifiedAppearanceID`) y parsea hasta 34 slots. Rust replica ese parser y el serializer ya estaba validado contra `VisualItemInfo`. |
| Character flags from DB | `character.rs:2535-2564`; `character.rs:2585-2599` | `CharacterPackets.cpp:119-148`; `SharedDefines.h:1035-1036` | Bug confirmado | C++ limpia ghost si `AT_LOGIN_RESURRECT`, mapea ghost y rename, anade `LOCKED_BY_BILLING` si hay ban, `DECLINED` si config/declined name, y muestra pet family/display solo para clases permitidas y no ghost. Rust ademas mapea `PLAYER_FLAGS_RESTING` a `CHARACTER_FLAG_RESTING` segun C#, pero C++ no lo hace en esta ruta; tambien lee `_banned_guid` pero no setea locked billing, no setea declined flag y deja `pet_family=0`. |
| Logout request runtime | `character.rs:3644-3675` | `MiscHandler.cpp:244-291`; `CharacterPackets.cpp:535-544` | Bug runtime / incompleto | C++ libera loot, calcula `instantLogout` por taxi/resting/RBAC, rechaza combat/falling/duel/freeze con reason 1/3/2, envia `LogoutResponse`, y si no es instant arranca timer/root/stun. Rust siempre envia `LogoutResponse::instant_ok()`, guarda y completa logout inmediatamente. |

## Pasada 11: loot packets y handlers base

La referencia superior de `crates/wow-world/src/handlers/loot.rs` apunta a C#,
asi que esta pasada trata el handler como subsistema, no como solo el comentario
de `DoLootRelease`. Alcance cerrado aqui: packet layouts basicos, apertura de
loot de criatura, autostore item, loot money, loot release, corpse release y
set-loot-specialization. No queda cerrado aun todo loot generation/templates,
roll completo, master-loot remoto ni storage DB end-to-end.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| CMSG loot parsers basicos | `loot.rs:42-193` | `LootPackets.cpp:33-35`; `68-94`; `106-113`; `133-137`; `LootPackets.h:86-106` | No bug de orden en parsers revisados | `LootUnit`, `LootItem`, `LootRelease`, `LootMoney`, `LootRoll`, `MasterLootItem` y `SetLootSpecialization` leen los mismos campos y orden que C++. Deuda: C++ usa `Array<LootRequest, 1000>` para loot/master-loot; Rust usa `Vec` sin cap explicito, pendiente de validar si el `Array` C++ impone limite runtime. |
| SMSG loot serializers basicos | `loot.rs:210-385` | `LootPackets.cpp:20-30`; `38-65`; `97-150`; `116-130`; `LootPackets.h:60-78` | No bug de layout en orden de campos | `LootItemData`, `LootResponse`, `LootRemoved`, `LootList`, `SLootRelease`, `LootReleaseAll`, `LootMoneyNotify`, `CoinRemoved`, `AELootTargets` y `AELootTargetsAck` escriben en el mismo orden que C++. Los bugs de esta pasada son valores alimentados al serializer, no el orden del serializer. |
| `LootResponse` valores success/error | `handlers/loot.rs:369-381`; `499-511`; `1051-1063`; `3925-3937`; `6319-6332` | `LootPackets.h:69-72`; `Player.cpp:8758-8766`; `Player.cpp:8776-8784` | Bug confirmado | C++ success deja `FailureReason=17` y `Threshold=2` por default, luego setea `Acquired=true`; Rust success manda `failure_reason=0`. C++ error tambien conserva `Threshold=2`; Rust `send_loot_error_like_cpp` manda `threshold=0`. |
| `HandleLootOpcode` flujo base | `handlers/loot.rs:194-282` | `LootHandler.cpp:216-260`; `Player.cpp:8747-8773` | Parcial: base no bug, gating bug abajo | Alive, tipo creature/vehicle, distancia 30, interrupcion de casts, removal de auras de looting, AE loot count/acks y `SendLoot`/`OnLootOpened` estan representados. La decision de si el jugador puede abrir loot no coincide completa con `Player::isAllowedToLoot`, ver bug dedicado. |
| `Player::isAllowedToLoot` gating | `handlers/loot.rs:3920-3923`; `handlers/loot.rs:7555-7562` | `Player.cpp:17987-18025` | Bug confirmado | C++ rechaza `HasPendingBind`, exige allowed looter y aplica `switch` por loot method: round-robin solo ganador o item personal; master/group/NBG solo ganador, over-threshold o item personal. Rust solo comprueba `isLooted`, item/money visible y allowed-looters en una capa superior, por lo que puede dejar abrir under-threshold round-robin/group loot a quien C++ rechazaria. |
| `HandleAutostoreLootItemOpcode` checks previos | `handlers/loot.rs:1074-1175` | `LootHandler.cpp:77-125`; `Player.cpp:25667-25697` | No bug en validaciones iniciales revisadas | Active loot view, GO owned/fishing-hole distance exception, creature existence/distancia, loot gone, allowed looter, blocked item y roll winner siguen la misma idea C++ en este slice. |
| `Player::StoreLootItem` cascada | `handlers/loot.rs:1177-1258`; `handlers/loot.rs:6559-6859`; `handlers/loot.rs:6931-6974` | `Player.cpp:25698-25745`; `LootHandler.cpp:128-139` | Bug runtime / incompleto | C++ usa `CanStoreNewItem`/`StoreNewItem`, notifica removals, decrementa loot, emite `SendNewItem`, actualiza criteria `LootItem/GetLootByType/LootAnyItem`, aplica `ApplyItemLootedSpell`, publica guild news para epicos y dispara `PROC_FLAG_LOOTED`. Rust guarda en inventory directo/backpack, envia updates/push result, pero no replica criteria, `ApplyItemLootedSpell`, guild news ni proc de looted en este slice. |
| `HandleLootMoneyOpcode` | `handlers/loot.rs:1261-1390` | `LootHandler.cpp:142-214`; `LootPackets.cpp:116-123` | Bug runtime / incompleto | Rust reparte dinero y envia `LootMoneyNotify`, pero fija `money_mod=0` y no aplica `SPELL_AURA_MOD_MONEY_GAIN`; C++ calcula `MoneyMod` por aura para cada receptor. C++ tambien actualiza criteria `MoneyLootedFromCreatures`; Rust solo actualiza oro/quest objective representado. |
| `DoLootRelease` creature/corpse comun | `handlers/loot.rs:6381-6502`; `wow-ai/src/lib.rs:1088`; `handlers/loot.rs:7537-7553` | `LootHandler.cpp:262-393`; `Creature.cpp:1377-1396`; `Creature.cpp:2942-2979` | No bug en el slice comun de release/corpse decay | Rust usa el loot activo interno, elimina looter, envia `SLootRelease`, limpia round-robin y notifica `LootList`, quita dynamic flag si fully looted, y calcula `skin=0` o `corpse_delay * Rate.Corpse.Decay.Looted` con ignore ratio. Queda pendiente validar el respawn DB/map end-to-end y todos los tipos GO/item/prospecting/milling fuera de los casos cubiertos por tests existentes. |
| `SetLootSpecialization` | `handlers/loot.rs:3742-3767` | `LootHandler.cpp:498-508` | No bug de comportamiento | C++ setea spec si existe en `ChrSpecializationStore` y `ClassID` coincide; `SpecID=0` limpia. Rust hace la misma validacion contra store/clase y limpia con 0. |

## Pasada 12: `SetProficiency::default_*` y proficiencies

Contraste del pendiente de la pasada 9. El objetivo era confirmar si las tablas
por clase de `SetProficiency::default_weapons/default_armor` venian de C++ o si
eran carry-over C#.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `SetProficiency` packet layout | `misc.rs:4814-4825` | `ItemPackets.cpp:145-151`; `ItemPackets.h:197-206` | No bug de layout | Ya cerrado en pasada 9: C++ escribe `uint32 ProficiencyMask`, `uint8 ProficiencyClass`; Rust coincide. |
| `Spell::EffectProficiency` runtime | `session.rs:51598-51630` | `SpellEffects.cpp:1785-1805`; `Player.h:1447-1450`; `Player.cpp:21780-21787` | No bug en el efecto spell revisado | C++ toma `EquippedItemSubClassMask`, OR-ea en `m_WeaponProficiency`/`m_ArmorProficiency` y envia `SetProficiency` con el mask acumulado. Rust `apply_proficiency_effect_like_cpp` replica ese OR y envia el mask acumulado para weapon/armor. |
| `default_weapons/default_armor` como fuente de verdad | `misc.rs:4828-4885`; `session.rs:14084-14090` | `Player.cpp:244-245`; `Player.cpp:11093-11143`; `SpellEffects.cpp:1785-1805` | Bug confirmado | C++ no define una tabla estatica de masks por clase. Inicializa proficiencies a cero, aprende por `SPELL_EFFECT_PROFICIENCY`, y `CanUseItem` valida `RequiredSkill`/`RequiredSpell`/allowable class/race, no `default_weapons(class)`. Rust usa `default_weapons(class)` como gate para item visual/collection; `default_armor` no tiene uso runtime localizado salvo tests, pero sigue siendo una tabla C# no canonica. |

## Pasada 13: `wow-world` misc handlers con referencias C#

Contraste de `crates/wow-world/src/handlers/misc.rs:2091,2179,2214,2395,2959`.
Los packet layouts de cemetery/taxi/query time ya estaban revisados en pasada 9;
esta pasada cubre handler flow contra C++.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `CMSG_WORLD_PORT_RESPONSE` / far teleport ack | `handlers/misc.rs:2090-2176`; `session.rs:29980-30012` | `MovementHandler.cpp:44-165`; `MovementHandler.cpp:239-260`; `Player.cpp:1433-1473` | Bug runtime / incompleto | C++ envia `TransferPending` y `SuspendToken` desde `Player::TeleportTo`, `NewWorld` al recibir `CMSG_SUSPEND_TOKEN_RESPONSE`, y en `WorldPortResponse` valida mapa/coords, crea mapa/instancia, transport, `ResumeToken` con `m_movementCounter`, `SendInitialPacketsBeforeAddToMap`, `AddPlayerToMap`, BG/flight handling y `SendInitialPacketsAfterAddToMap`. Rust no tiene handler localizado para `SuspendTokenResponse`, envia `NewWorld` dentro de `WorldPortResponse`, fija `sequence_index=1` y solo reemite un subconjunto de paquetes iniciales/visibilidad. |
| `CMSG_AREA_TRIGGER` parser | `handlers/misc.rs:2180-2182` | `AreaTriggerPackets.cpp:66-70` | Bug confirmado | C++ lee `AreaTriggerID` y despues bits `Entered` y `FromClient`. Rust solo lee `u32 trigger_id`, por lo que no puede distinguir enter/leave ni from-client. |
| `HandleAreaTriggerOpcode` runtime | `handlers/misc.rs:2188-2204` | `MiscHandler.cpp:484-688` | Bug runtime / incompleto | C++ rechaza en flight, valida DBC/radio/conditions/scripts, procesa quest objectives y completion, rest/tavern/FFA PvP, battleground/outdoor PvP, corpse/no-corpse, transfer aborts, LFG exit y entrance locations antes de `TeleportTo`. Rust solo busca trigger en store y teleporta si tiene destino. |
| `CMSG_REQUEST_CEMETERY_LIST` | `handlers/misc.rs:2213-2219` | `MiscPackets.h:391-397`; `MiscHandler.cpp:367-399` | Bug confirmado | C++ request no tiene payload, busca graveyards por `zoneId`, filtra conditions y envia hasta 16 IDs; si no hay IDs, no envia respuesta. Rust intenta leer un byte opcional para gossip y siempre envia respuesta vacia. |
| `CMSG_TAXI_NODE_STATUS_QUERY` | `handlers/misc.rs:2393-2427` | `TaxiHandler.cpp:40-69`; `TaxiPackets.cpp:20-33` | Bug runtime / incompleto | C++ resuelve creature, hostilidad, `UNIT_NPC_FLAG_FLIGHTMASTER`, nearest taxi node por posicion/map/team, reaccion y taximask conocida para devolver None/Learned/Unlearned/NotEligible. Rust solo mira `npc_flags & 0x2000` y devuelve Unlearned o None, sin nearest node, hostility, reaction ni known taxi mask. |
| `CMSG_QUERY_TIME` | `handlers/misc.rs:2958-2970` | `QueryHandler.cpp:58-68`; `QueryPackets.cpp:411-416` | No bug de comportamiento | C++ responde `CurrentTime = GameTime::GetSystemTime()`. Rust responde UNIX seconds via `SystemTime`, equivalente en este slice. |

## Pasada 14: subsistema Quest packets/data/handlers

Las referencias C# de Quest cubren `wow-packet`, `wow-data` y `wow-world`.
Por eso esta pasada no trata las lineas como comentarios aislados: contrasta los
parsers CMSG principales, los serializers de reward/details/query, `QuestXP`,
`QuestTemplate::is_available_for`, `can_take_quest` y los handlers
accept/request/complete/choose-reward contra C++. Alcance cerrado aqui:
estructuras y rutas indicadas abajo. Quedan abiertos POI, quest share completo,
quest status multiple, gossip/menu completo, loaders/normalizacion completa,
condicionales/localizacion y persistencia end-to-end.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| Constantes Quest counts | `packets/quest.rs:15-19`; `wow-data/src/quest.rs:16-21` | `QuestDef.h:45-53`; `QuestPackets.h:190-200`; `QuestPackets.h:272-285` | No bug de valores | Los counts Rust `items=4`, `choice=6`, `reputation=5`, `currency=4`, `display_spells=3`, `item_drop=4` coinciden con C++ `QUEST_*_COUNT`. El comentario C# es misatribucion. |
| `QuestXP.db2` row layout y rounding | `quest_xp.rs:8-26`; `quest_xp.rs:140-149` | `DB2Structure.h:3097-3101`; `QuestDef.cpp:714-724` | No bug en layout/rounding | C++ `QuestXPEntry` es `ID` + `std::array<uint16, 10> Difficulty`; Rust carga `level` + 10 `u16` convertidos a `u32`. `round_xp` replica los umbrales C++ `<=100`, `<=500`, `<=1000`, resto. |
| `QuestXP::calculate_xp` fila ausente | `quest_xp.rs:71-116`; `quest_xp.rs:132-136` | `QuestDef.cpp:387-398` | Bug confirmado | C++ hace `sQuestXPStore.LookupEntry(quest_level)` y devuelve `0` si no existe fila. Rust llama `nearest(ql)` y usa el nivel mas cercano, inventando XP para niveles que C++ no recompensaria. |
| `QuestXP::calculate_xp` min scaled XP | `quest_xp.rs:110-116` | `QuestDef.cpp:404-409`; world config `CONFIG_MIN_QUEST_SCALED_XP_RATIO` | Bug de paridad configurable | C++ aplica `max(minScaledXP, xp)` si la config `CONFIG_MIN_QUEST_SCALED_XP_RATIO` es distinta de cero. Rust fija `min_scaled_xp = 0`, correcto solo con esa config desactivada. |
| `QuestTemplate::is_available_for` | `wow-data/src/quest.rs:279-301` | `Player.cpp:15060-15089`; `Player.cpp:15240-15260`; `Player.cpp:15037-15059` | No bug en race/class/min/max | Rust comprueba mask de raza, mask de clase, min level y max level igual que las gates C++ `SatisfyQuestRace`, `SatisfyQuestClass` y `SatisfyQuestLevel`. Deuda: comentario cita C#. |
| `CMSG_QUEST_GIVER_QUERY_QUEST` / `ACCEPT_QUEST` bit flags | `handlers/quest.rs:1342-1366`; `handlers/quest.rs:1359-1379` | `QuestPackets.cpp:547-557`; `QuestHandler.cpp:228-253`; `QuestHandler.cpp:105-225` | Bug confirmado | C++ lee `RespondToGiver`/`StartCheat` con `ReadBit()` despues de GUID y QuestID. Rust lee esos flags con `read_uint8()`, consumiendo un byte donde C++ consume un bit. |
| `CMSG_QUEST_GIVER_COMPLETE_QUEST` parser | `handlers/quest.rs:5010-5026` | `QuestPackets.cpp:414-419` | No bug de parser | Ambos leen `QuestGiverGUID`, `QuestID` y `FromScript` como bit. Los bugs de esta ruta son de runtime/valores, no de parser. |
| `CMSG_QUEST_GIVER_CHOOSE_REWARD` parser | `handlers/quest.rs:5134-5170`; `handlers/quest.rs:5569-5585` | `QuestPackets.cpp:315-328`; `QuestPackets.cpp:390-395`; `ItemPacketsCommon.cpp:176-205` | No bug de parser | Rust lee `LootItemType` de 2 bits, `ItemInstance` (`ItemID`, random seed/id, bonus bit, modifications, bonus) y `Quantity` como C++. |
| `SMSG_QUERY_QUEST_INFO_RESPONSE` ready bit | `packets/quest.rs:575-701` | `QuestPackets.cpp:87-230` | Bug confirmado | C++ escribe, despues de las longitudes de strings, `WriteBit(Info.ReadyForTranslation)` y luego `FlushBits()`. Rust omite ese bit y hace flush, desplazando objetivos/strings. |
| `QuestRewardsBlock` choice item layout | `packets/quest.rs:352-395` | `QuestPackets.cpp:283-324`; `ItemPacketsCommon.cpp:176-190` | Bug confirmado | C++ escribe cada `QuestChoiceItem` como `LootItemType` de 2 bits, despues `ItemInstance`, despues `Quantity`. Rust no escribe `LootItemType`, escribe `ItemID`, `Quantity`, un `u64 mask`, count y campos extra en otro orden. |
| `QuestRewardsBlock` valores alimentados | `handlers/quest.rs:4977-5010`; `handlers/quest.rs:5083-5129`; `packets/quest.rs:326-395` | `QuestDef.cpp:446-488`; `Player.cpp:14560-14595` | Bug runtime / incompleto | C++ `BuildQuestRewards` rellena counts, dinero escalado por rate, XP con rate/auras, titulo, reputation mask/valores, currencies, skill, treasure picker y tipos de choice items. Rust rellena una parte fija desde template y deja reputacion/currency/skill/faction flags/treasure picker en cero; usa `reward_money_difficulty` crudo en dialogs. |
| `SMSG_QUEST_GIVER_QUEST_COMPLETE` layout | `packets/quest.rs:478-503` | `QuestPackets.cpp:397-411`; `QuestPackets.h:343-359` | Bug confirmado | C++ escribe `QuestID`, `XPReward`, `MoneyReward`, skill, 4 bits (`UseQuestReward`, `LaunchGossip`, `LaunchQuest`, `HideChatMessage`) y despues `ItemReward` (`ItemInstance`). Rust no serializa `ItemReward`; el paquete queda truncado respecto a C++. |
| `QuestGiverRequestItems` serializer orden base | `packets/quest.rs:786-840` | `QuestPackets.cpp:493-538`; `QuestPackets.h:444-486` | No bug de layout base | El orden de GUID, creature id, quest id, emotes, flags, counts, collect/currency rows, bit `AutoLaunched`, conditional count, lengths y strings coincide para los campos representados. Los bugs estan en que Rust alimenta listas/condicionales vacias en rutas runtime. |
| `can_take_quest` gates C++ omitidas | `handlers/quest.rs:6284-6512` | `Player.cpp:14111-14118`; `Player.cpp:15179-15202`; `Player.cpp:15333-15347` | Bug runtime / incompleto | Rust cubre muchas gates en orden C++ pero reconoce `SatisfyQuestTimed` como gap y no implementa `SatisfyQuestBreadcrumbQuest` para `breadcrumb_for_quest_id`. C++ bloquea aceptar una segunda timed quest y bloquea breadcrumbs cuyo target quest no es tomable. |
| `HandleQuestgiverAcceptQuestOpcode` side-effects | `handlers/quest.rs:1359-1465` | `QuestHandler.cpp:105-225`; `Player.cpp:14240-14530` | Bug runtime / incompleto | C++ resuelve UNIT/GAMEOBJECT/ITEM o jugador que comparte quest, cierra gossip/clear sharing en fallos, valida `CanAddQuest`, ejecuta `AddQuestAndCheckCompletion`, source item/spell, criteria, scripts/AI, quest tracker, push-to-party-on-accept y launch gossip. Rust representa fuente/CanTake/slot/DB y envia complete popup, pero no replica esas ramas completas. |
| Reward/complete runtime | `handlers/quest.rs:4891-5129`; `handlers/quest.rs:5569-5750` | `QuestHandler.cpp:269-440`; `QuestHandler.cpp:533-588`; `Player.cpp:14208-14385`; `Player.cpp:14649+` | Bug runtime / incompleto | C++ para complete/request/choose usa `CanCompleteQuest`, `CanRewardQuest`, request-items con objetivos reales, item/currency/money objective checks, daily/week/month/seasonal/skill/reputation/rewarded gates, Battleground hook y `RewardQuest` completo. Rust tiene validaciones parciales y en incomplete response manda collect/currency vacios; la ruta de reward aun no cubre todos los efectos C++ de recompensa. |

## Pasada 15: `wow-data` stats, item stats y skills

Las referencias C# de `wow-data` no se cierran por comentario aislado. Esta pasada
contrasta el subsistema que alimenta login/stat updates: `player_stats.rs`,
`item_stats.rs`, `skill.rs` y los usos directos localizados en
`handlers/character.rs`. Alcance cerrado aqui: formulas de stats usadas en login
y update, subset de `ItemSparse` usado por stats/templates, layout DB2 de
`SkillLineAbility`/`SkillRaceClassInfo`, indice por skill, range type y rutas
legacy de starting skills/spells. Quedan fuera otros modulos `wow-data` no
citados aqui y rutas de stats que usen exclusivamente helpers representados
`*_like_cpp` fuera de `character.rs`.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| Bonus de stamina/intellect como fragmento matematico | `player_stats.rs:31-50` | `StatSystem.cpp:279-295` | No bug del fragmento | C++ `GetHealthBonusFromStamina` y `GetManaBonusFromIntellect` usan `min(20)` + resto por `10`/`15`, igual que el fragmento Rust. Esto no valida la ruta completa de max health/mana. |
| Fuente y proyeccion de stats base | `player_stats.rs:8-10`; `player_stats.rs:205-226`; `character.rs:5480-5553`; `character.rs:12314-12568` | `ObjectMgr.cpp:4206-4247`; `ObjectMgr.cpp:4411-4427`; `Player.cpp:2365-2420`; `StatSystem.cpp:298-330`; `StatSystem.cpp:333-425`; `StatSystem.cpp:502-731` | Bug confirmado | Rust carga `race,class,level,str,agi,sta,inte,spi,basehp,basemana` desde `player_levelstats` y calcula health/mana/AP/crit/dodge con formulas directas. C++ carga stats primarios desde `player_classlevelstats` + modificadores raciales, toma base mana de `GtBaseMP`, hace `SetCreateHealth(0)`, y despues calcula max health/power/AP/crit/dodge/parry/spell crit mediante `StatSystem` con unit mods, class coefficients, level bonus, ratings, auras y diminishing returns. |
| `ItemModType` subset | `item_stats.rs:22-52` | `ItemTemplate.h:28-100` | No bug de valores para el subset listado | Los valores Rust visibles para mana, health, primarios, ratings unificados, AP/RAP, expertise, armor penetration y spell power coinciden con el enum C++ en ese subset. Deuda: el comentario no debe decir que viene de C#. |
| `ItemSparse` field order usado por stats/templates | `item_stats.rs:253-355`; `item_stats.rs:450-735` | `DB2Structure.h:2297-2373`; `DB2LoadInfo.h:3028-3164` | No bug en los campos representados | Rust trata el `ID` como id del record y despues alinea los campos logicos con `ItemSparseEntry`: `AllowableRace`, strings, `DmgVariance`, `BagFamily`, `StartQuestID`, flags, damage, resistances, stat amounts, stat types, inventory type y quality. Esta validacion solo cubre los campos leidos en esta ruta. |
| Aplicacion de stats de item en `character.rs` | `item_stats.rs:144-245`; `character.rs:5450-5475`; `character.rs:12268-12309` | `Player.cpp:7712-7995`; `ItemTemplate.h:28-100` | Bug runtime / incompleto | C++ `_ApplyItemBonuses` aplica stats via unit mods y ratings, scaling stat distribution/value, AP a melee y ranged, ratings separados (`HIT_RANGED`, `HIT_SPELL`, haste melee/ranged/spell), mana/health regen, spell penetration, extra armor, resistencias por escuela y stats combinados 71-74. Rust suma helpers parciales; por ejemplo `ITEM_MOD_ATTACK_POWER` solo entra en `gear_ap` y no en `gear_rap`, aunque C++ lo aplica a ambos. |
| `SkillLineAbility` DB2 layout usado | `skill.rs:663-695`; `wdc4.rs:478-485` | `DB2Structure.h:3324-3341`; `DB2LoadInfo.h:4546-4569` | No bug de layout usado | El field 1 que Rust llama "extra" es el `ID` C++ dentro de `SkillLineAbilityLoadInfo`; `iter_records()` devuelve el id tambien como record id. Los campos usados (`RaceMask`, `SkillLine`, `Spell`, ranks, masks, flags, `SkillupSkillLineID`) quedan alineados. |
| `SkillRaceClassInfo` DB2 layout usado | `skill.rs:721-739` | `DB2Structure.h:3354-3363`; `DB2LoadInfo.h:4585-4599` | No bug de layout usado | Rust lee `RaceMask`, `SkillID`, `ClassMask`, `Flags`, `Availability`, `MinLevel`, `SkillTierID` en el mismo orden logico que C++ cuando el id va separado. |
| Indice por skill para `SkillLineAbility` | `skill.rs:584-597`; `skill.rs:697-708`; `skill.rs:1088-1094` | `DB2Stores.cpp:1328-1329`; `DB2Stores.cpp:2578-2580` | No bug de indice | C++ indexa por `SkillupSkillLineID` si existe, si no por `SkillLine`; Rust hace la misma normalizacion en fixtures y loader. |
| `GetSkillRaceClassInfo` y `GetSkillRangeType` | `skill.rs:789-833`; `skill.rs:1181-1189` | `DB2Stores.cpp:2583-2596`; `ObjectMgr.cpp:9006-9027`; `ObjectMgr.h:957-976` | No bug en esos helpers | Rust replica match por race/class mask y range type: tier => rank, runeforging => mono, armor => mono, languages => language, resto => level. |
| `starting_skills` como player-create skills | `skill.rs:753-771`; `skill.rs:839-876`; `character.rs:5187-5201` | `ObjectMgr.cpp:3982-3996`; `Player.cpp:23822-23838` | Bug confirmado | C++ solo anade a `PlayerInfo.skills` los `SkillRaceClassInfo` con `Availability == 1` y `LearnDefaultSkills` salta `MinLevel > GetLevel()`. Rust expande todos los records por raza/clase sin filtrar `Availability` y `starting_skill_info` no filtra `min_level`, por lo que puede crear skill slots que C++ no daria. |
| `starting_skill_info` valores de skill | `skill.rs:835-876`; `character.rs:5187-5201` | `Player.cpp:23840-23879`; `ObjectMgr.h:957-974`; `DBCEnums.h:1869-1876` | Bug confirmado | C++ usa `LearnDefaultSkill`: language `300/300`, level `1/max` o `max/max` por `SKILL_FLAG_ALWAYS_MAX_VALUE`, DK `(level-1)*5`, mono `1/1`, rank con tier y step `1`. Rust fija `rank=max_rank`, `starting_rank=1`, `step=0`, `max_rank=level*5` para todo y ademas exige que el skill tenga abilities. |
| `starting_spells` / `racial_spells` legacy path | `skill.rs:879-1040`; `character.rs:5115-5178` | `Player.cpp:23954-24006`; `DB2Stores.cpp:2578-2580`; `DBCEnums.h:1862-1865` | Bug runtime / incompleto | La ruta legacy usa `level*5`, heuristica de class-skill/known-skill y filtros parciales. C++ exige `SpellInfo`, filtra acquire method incluyendo quest fallback con flag/condition, caso especial Riding con `NumSkillUps == 1`, race/class masks, required level `max(SpellLevel, BaseLevel)` y compara contra el skill value real. El helper Rust `skill_rewarded_spells_like_cpp` representa esa logica mejor, pero no es la ruta usada cuando `db_count == 0`. |
| `skill_rewarded_spells_like_cpp` helper representado | `skill.rs:1096-1168`; `character.rs:5133-5168` | `Player.cpp:23954-24006` | No bug en el helper revisado | El helper usa el indice C++ por skill, requiere callback de niveles como `SpellInfo`, soporta acquire 1/2 y quest fallback, caso Riding, race/class masks, required level y `MinSkillLineRank`. La fidelidad depende de que el caller pase callbacks reales; esta fila no valida `starting_spells`. |

## Pasada 16: subsistema Movement packets/handlers

Las referencias C# de Movement cubren parser binario, opcodes y handler runtime.
Esta pasada contrasta `MovementInfo`, `SetActiveMover`,
`MoveInitActiveMoverComplete`, registro de los CMSG_MOVE revisados,
`ValidateMovementInfo` y el handler generico `HandleMovementOpcode`.
Alcance cerrado aqui: layout basico y las rutas indicadas. Quedan fuera
monster spline completo, vehicle handlers fuera de MovementHandler y el modelo
map-owned de visibilidad/transport end-to-end.

| Area | Rust ref | C++ ref | Veredicto | Evidencia |
| --- | --- | --- | --- | --- |
| `MovementInfo` read layout | `movement.rs:24-266` | `MovementPackets.cpp:104-230` | No bug de layout base | Rust lee GUID, flags1/2/3, time, XYZO, pitch, step-up, remove-force count/index, force GUIDs, bits de standing/transport/fall/spline/height/remote/inertia/adv-flying y optionals en el mismo orden C++. |
| `MovementInfo` write layout | `movement.rs:268-354` | `MovementPackets.cpp:25-102`; `MovementPackets.cpp:200-224` | No bug de layout base con invariant normal | El orden de campos, bits, flush y bloques opcionales coincide. Diferencia de invariant: C++ decide `hasTransportData` por `!transport.guid.IsEmpty()`, Rust por `Option<TransportInfo>`; no es bug si `Some` nunca contiene GUID vacio. |
| `SetActiveMover` parser y handler basico | `movement.rs:1306-1324`; `handlers/movement.rs:366-388` | `MovementPackets.cpp:825-835`; `MovementHandler.cpp:543-548` | No bug en parser/efecto basico | Ambos leen solo GUID packed. C++ solo loguea si el active mover no coincide; Rust tambien solo loguea/advierte y no modifica estado. |
| `MoveInitActiveMoverComplete` parser | `movement.rs:1326-1345` | `MovementPackets.cpp:1094-1097`; `MovementHandler.cpp:810-815` | No bug de parser | Ambos leen un `uint32 Ticks`. Los bugs estan en side effects Rust, no en el parser. |
| Registro processing de movement revisado | `handlers/movement.rs:37-80`; `handlers/movement.rs:2115-2251` | `Opcodes.cpp:612-698`; `Opcodes.cpp:880` | No bug en status/processing revisado | C++ marca los CMSG_MOVE principales y ACKs revisados como `STATUS_LOGGEDIN`/`PROCESS_THREADSAFE`, `CMSG_SET_ACTIVE_MOVER` como thread-unsafe y `CMSG_MOVE_TIME_SKIPPED` como inplace. Rust coincide para esas entradas. |
| `ValidateMovementInfo` representado | `wow-anticheat/src/lib.rs:111-258`; `session.rs:42480-42506` | `Player.cpp:28453-28539` | No bug en reglas representadas | Rust cubre root/fixed vehicle, root+moving, hover, ascend/descend, left/right, strafe, pitch, forward/backward, water-walk/ghost, feather-fall, fly/security/aura, gravity/can-fly+falling y spline elevation add/remove en el orden C++. |
| Handler generico usa `GetUnitBeingMoved()` | `handlers/movement.rs:105-364`; test `handle_movement_uses_current_mover_guid_like_cpp` | `MovementHandler.cpp:312-430` | Bug corregido `#CSharpAudit.MOVEMENT.1` | C++ toma `Unit* mover = _player->GetUnitBeingMoved()`, valida `movementInfo.guid` contra ese mover, fuerza `movementInfo.guid = mover->GetGUID()`, guarda tiempo/flags/posicion en el mover y emite `MoveUpdate` desde ese GUID. Rust ya usa `player_moved_unit_guid_like_cpp()` como mover esperado, no pisa posicion/flags del player cuando el mover es otro unit, reubica la criatura representada si existe en el map manager y manda `SendIfVisibleLikeCpp` con `source_guid=mover_guid`. |
| Handler generico omite ramas C++ de teleport/spline/transport/vehicle | `handlers/movement.rs:105-364` | `MovementHandler.cpp:324-430`; `MovementHandler.cpp:432-455` | Bug runtime / incompleto | C++ ignora movimiento mientras el player esta siendo teletransportado, requiere `mover->movespline->Finalized()`, anade/cambia/remueve transport passengers, resetea transport si no hay transport/vehicle, aplica vehicle seat turning con retorno temprano, y hace under-map/battleground/death flow. Rust cubre solo parte: sanitize, GUID/coords, offset transport basico, fall/auras/sit/jump, posicion, visibilidad y broadcast. |
| `MoveInitActiveMoverComplete` side effects | `handlers/movement.rs:390-406`; `session.rs:37416-37432`; `session.rs:42052-42070` | `MovementHandler.cpp:810-815`; `Player.h:487,2775,2787` | Bug/representacion parcial | C++ setea local flag `OVERRIDE_TRANSPORT_SERVER_TIME`, transport server time y llama `UpdateObjectVisibility(false)`. Rust setea campos representados, no ejecuta el notify de visibilidad C++ y envia inmediatamente un `UpdateObject` parcial de ActivePlayerData. Eso puede ser una adaptacion temporal, pero no es 1:1 C++. |

## Bugs Confirmados Y Estado

1. `#CSharpAudit.COMPRESS.1`: corregido. El threshold de compresion usa payload sin opcode como C++ (`packet.size() > 0x400`); test: `compression_threshold_uses_payload_len_like_cpp`.
2. `#CSharpAudit.BNETREST.1`: corregido. `GET /bnetserver/login/` ya no emite `JSESSIONID`; C++ solo setea `Content-Type` y body JSON. Test: `login_form_headers_do_not_set_cookie_like_cpp`.
3. `#CSharpAudit.BNETREST.2`: corregido. `POST /bnetserver/login/srp/` ya no emite ni depende de `JSESSIONID`; el SRP vive en `RestConnectionState` por conexion HTTP como `LoginHttpSession::GetSessionState()` en C++.
4. `#CSharpAudit.FEATURE.1`: corregido. `FeatureSystemStatus` y `FeatureSystemStatusGlueScreen` ya reflejan los configs C++ de support/BPay/undelete/max chars/expansion y `IsMuted = !CanSpeak()`. Tests: `feature_system_status_uses_cpp_config_flags`, `feature_system_status_glue_screen_uses_cpp_config_fields`.
5. `#CSharpAudit.BNETSRP.1`: corregido. Challenge v1 envia `iterations=1` como C++ `BnetSRP6v1Base::GetXIterations()`. Test: `challenge_v1_iterations_match_cpp`.
6. `#CSharpAudit.BNETSRP.2`: corregido. Calculo SRP `u` paddea `A`/`B` a 128 bytes en v1 y 256 bytes en v2 como C++ `CalculateU`. Tests: `compute_u_v1_pads_a_and_b_to_128_bytes_like_cpp`, `compute_u_v2_pads_a_and_b_to_256_bytes_like_cpp`.
7. `#CSharpAudit.BNETSRP.3`: corregido. `k` para v2+SHA512 paddea `g` a 256 bytes como C++ `BnetSRP6v2`. Test: `compute_k_v2_sha512_pads_generator_to_256_bytes_like_cpp`.
8. `#CSharpAudit.BNETSRP.4`: normalizacion de login/SRP username usa uppercase Unicode en Rust; C++ solo upper Latin basico.
9. `#CSharpAudit.BNETREST.3`: corregido. SRP challenge de cuenta inexistente devuelve JSON `authentication_state=DONE` como C++, sin HTTP 400 ni mensaje `Account not found`. Test: `srp_challenge_missing_account_returns_done_like_cpp`.
10. `#CSharpAudit.BNETREST.4`: hex en challenge/login ticket es lowercase en Rust; C++ es uppercase.
11. `#CSharpAudit.SPELL.1`: `SMSG_CAST_FAILED` omite `SpellCastVisual` en Rust; C++ lo serializa entre `SpellID` y `Reason`.
12. `#CSharpAudit.COMBAT.1`: `SMSG_ATTACKER_STATE_UPDATE` coloca el bit de combat-log dentro de `attackRoundInfo`; C++ lo escribe en el paquete exterior antes del size.
13. `#CSharpAudit.CHAT.1`: rangos chat/emote/yell hardcodeados desde C#; C++ usa config `ListenRange.*`.
14. `#CSharpAudit.CHAT.2`: handlers `CMSG_EMOTE`/`CMSG_SEND_TEXT_EMOTE` son runtime simplificado y no reproducen validacion/scripts/criteria/DB2/rango C++.
15. `#CSharpAudit.PARTY.1`: `SMSG_PARTY_INVITE` se serializa con BNet account GUID vacio, realm vacio, roles propuestos ignorados y `AllowMultipleRoles=true`; C++ alimenta esos campos desde sesion/realm/request y deja `AllowMultipleRoles=false`.
16. `#CSharpAudit.PARTY.2`: `HandlePartyInviteOpcode` omite validaciones C++ de GM/faccion/instancia/social/level/target group/invite/permisos/full-check completo.
17. `#CSharpAudit.PARTY.3`: `PartyIndex` se lee pero se descarta en invite/accept/leave; C++ lo usa para resolver/rechazar la categoria de grupo.
18. `#CSharpAudit.PARTY.4`: `HandlePartyInviteResponseOpcode` y `HandleLeaveGroupOpcode` modelan un lifecycle distinto al C++ (`GetGroupInvite`, self/full/category checks, cancelacion de invite y `PartyResult` OK de leave).
19. `#CSharpAudit.ITEM.1`: `AutoStoreBagItem` lee `ContainerSlotA/B` invertidos frente a C++.
20. `#CSharpAudit.ITEM.2`: handlers inventory move/equip/store/destroy son direct-inventory-only o simplificados y omiten validaciones/logica C++ de `InvUpdate`, `IsValidPos`, bank access, `CanEquipItem`, `CanStoreItem`, `CanUnequipItem` y `Player::SwapItem`.
21. `#CSharpAudit.MISC.1`: `ShowTradeSkill` Rust parsea payload y envia `SMSG_SHOW_TRADE_SKILL_RESPONSE`; C++ trata `CMSG_SHOW_TRADE_SKILL` como `Null`, solo loguea, y marca el SMSG como unhandled.
22. `#CSharpAudit.MISC.2`: `AuctionHelloResponse` Rust serializa `auction_house_id` y delays de 1h; C++ no serializa auction house id y deja delays default 0 en `SendAuctionHello`.
23. `#CSharpAudit.CHARACTER.1`: enum character flags/data mapping diverge: Rust mapea resting desde C#, omite locked billing/declined y no rellena pet family como C++.
24. `#CSharpAudit.CHARACTER.2`: logout request completa logout instantaneo siempre; C++ puede denegar, responder delayed y arrancar countdown/root/stun.
25. `#CSharpAudit.LOOT.1`: `LootResponse` success/error usa valores no C++ (`failure_reason=0` en success y `threshold=0` en error); C++ conserva defaults `FailureReason=17`, `Threshold=2`.
26. `#CSharpAudit.LOOT.2`: apertura de loot no replica `Player::isAllowedToLoot`: falta `HasPendingBind` y gating por `LootMethod`/round-robin/over-threshold.
27. `#CSharpAudit.LOOT.3`: `HandleLootMoneyOpcode` no aplica `SPELL_AURA_MOD_MONEY_GAIN` ni criteria `MoneyLootedFromCreatures`; `MoneyMod` sale siempre `0`.
28. `#CSharpAudit.LOOT.4`: `Player::StoreLootItem` Rust no replica toda la cascada C++ de storage/criteria/`ApplyItemLootedSpell`/guild news/`PROC_FLAG_LOOTED`.
29. `#CSharpAudit.PROFICIENCY.1`: `SetProficiency::default_weapons/default_armor` son tablas C# no canonicas; C++ aprende proficiencies por `SPELL_EFFECT_PROFICIENCY` y `CanUseItem` no usa una tabla estatica por clase.
30. `#CSharpAudit.MISCWORLD.1`: far teleport/worldport ack no replica la secuencia C++ `SuspendTokenResponse -> NewWorld -> WorldPortResponse -> ResumeToken/AddToMap/initial packets`.
31. `#CSharpAudit.AREATRIGGER.1`: `CMSG_AREA_TRIGGER` Rust omite bits `Entered`/`FromClient` y el handler salta conditions/scripts/quests/tavern/BG/PvP/corpse/transfer-abort C++.
32. `#CSharpAudit.CEMETERY.1`: `CMSG_REQUEST_CEMETERY_LIST` Rust lee un byte no C++ y siempre responde lista vacia; C++ busca graveyards por zona/conditions y no responde si no hay.
33. `#CSharpAudit.TAXI.1`: taxi node status Rust solo usa NPC flag; C++ valida creature/hostilidad/nearest node/reaccion/taximask para None/Learned/Unlearned/NotEligible.
34. `#CSharpAudit.QUESTXP.1`: `QuestXP::calculate_xp` usa fila de nivel mas cercano cuando falta `QuestXP` para el nivel; C++ devuelve `0`.
35. `#CSharpAudit.QUESTXP.2`: XP minimo escalado ignora `CONFIG_MIN_QUEST_SCALED_XP_RATIO`; C++ puede elevar el XP minimo por config.
36. `#CSharpAudit.QUESTCMSG.1`: `CMSG_QUEST_GIVER_QUERY_QUEST` y `CMSG_QUEST_GIVER_ACCEPT_QUEST` leen flags bit (`RespondToGiver`/`StartCheat`) como byte en Rust; C++ usa `ReadBit()`.
37. `#CSharpAudit.QUESTPKT.1`: `SMSG_QUERY_QUEST_INFO_RESPONSE` Rust omite el bit `ReadyForTranslation` antes del flush.
38. `#CSharpAudit.QUESTPKT.2`: `QuestRewardsBlock` serializa `QuestChoiceItem` con orden/layout no C++; falta `LootItemType` de 2 bits y `ItemInstance`/`Quantity` estan en orden incorrecto.
39. `#CSharpAudit.QUESTREWARD.1`: reward dialogs alimentan `QuestRewardsBlock` con datos incompletos/no escalados: money crudo, XP/reputacion/currency/skill/faction flags/treasure picker ausentes o cero.
40. `#CSharpAudit.QUESTPKT.3`: `SMSG_QUEST_GIVER_QUEST_COMPLETE` Rust omite `ItemReward` (`ItemInstance`) que C++ serializa al final.
41. `#CSharpAudit.QUEST.1`: `can_take_quest` no implementa `SatisfyQuestTimed` ni `SatisfyQuestBreadcrumbQuest`.
42. `#CSharpAudit.QUEST.2`: `HandleQuestgiverAcceptQuestOpcode` Rust no replica side-effects C++ de item/player sharing, close gossip, `CanAddQuest`, source item/spell, scripts, quest tracker, push-to-party y launch gossip.
43. `#CSharpAudit.QUEST.3`: complete/request/choose reward runtime es parcial: request-items vacio, `CanRewardQuest` incompleto y `RewardQuest` no cubre todas las gates/side-effects C++.
44. `#CSharpAudit.DATASTATS.1`: `player_stats.rs` y las proyecciones de `character.rs` usan tablas/formulas C# para stats finales; C++ carga stats desde `player_classlevelstats` + race modifiers, base mana desde `GtBaseMP`, `CreateHealth=0` y recalcula health/power/AP/crit/dodge/parry/spell crit via `StatSystem`.
45. `#CSharpAudit.ITEMSTATS.1`: la aplicacion directa de stats de item en `character.rs` es parcial frente a C++ `_ApplyItemBonuses`: falta AP a ranged desde `ITEM_MOD_ATTACK_POWER`, scaling stat values, ratings separados, regen, spell penetration, extra armor, resistencias por escuela y stats combinados.
46. `#CSharpAudit.SKILL.1`: `starting_skills`/`starting_skill_info` usan todos los `SkillRaceClassInfo` por mascara y no filtran `Availability == 1` ni `MinLevel`; C++ `PlayerInfo.skills` y `LearnDefaultSkills` si lo hacen.
47. `#CSharpAudit.SKILL.2`: `starting_skill_info` no replica `LearnDefaultSkill`: no distingue language/level/mono/rank, flags `SKILL_FLAG_ALWAYS_MAX_VALUE`, DK special case, tier max/step ni skills sin abilities.
48. `#CSharpAudit.SKILL.3`: `starting_spells`/`racial_spells` legacy no replica `LearnSkillRewardedSpells`: faltan `SpellInfo`/required-level real, quest fallback con flag/condition, Riding `NumSkillUps`, skill value real y parte de la politica C++ de adquisicion.
49. `#CSharpAudit.MOVEMENT.1`: corregido. `HandleMovementOpcode` usa el mover actual (`GetUnitBeingMoved()` representado) para validar, actualizar lo representable y emitir `MoveUpdate`; test: `handle_movement_uses_current_mover_guid_like_cpp`.
50. `#CSharpAudit.MOVEMENT.2`: runtime generico de movement omite ramas C++ de teleport-in-progress, `movespline->Finalized()`, alta/baja/reset de transport passenger, vehicle turning con retorno temprano, estado generico completo `Unit::m_movementInfo` para movers no-criatura/no representados y parte del under-map/BG/death flow.
51. `#CSharpAudit.MOVEMENT.3`: `MoveInitActiveMoverComplete` no es 1:1: Rust no ejecuta `UpdateObjectVisibility(false)` como C++ y envia un update parcial inmediato de ActivePlayerData.

## No bugs de comportamiento, pero si deuda de comentario

Estos deben corregirse solo despues de cada slice, reemplazando C# por C++ anchors:

- `world_packet.rs` bit APIs -> `ByteBuffer.h` / `ByteBuffer.cpp`.
- `world_crypt.rs` y `world_socket.rs` counters/tag/nonce -> `WorldPacketCrypt.cpp` / `AES.h`.
- `character.rs` response codes -> `SharedDefines.h`.
- `character.rs` enum/list layout -> `CharacterPackets.cpp/h`.
- `movement.rs` active mover packets -> `MovementPackets.cpp/h`.
- `misc.rs` AccountDataTimes -> `ClientConfigPackets.cpp/h`.
- `rsa_sign.rs` ConnectTo firma/layout -> `AuthenticationPackets.cpp` / `RSA.cpp`.
- `ed25519ctx.rs` y `world_socket.rs` EnterEncryptedMode -> `AuthenticationPackets.cpp` / `Ed25519.cpp` / `dep/openssl_ed25519/curve25519.c`.
- `spell.rs` `SpellTargetData`, `CastSpellRequest`, `SpellCastData` minimo, `CooldownEvent`, `SpellCooldownPkt` -> `SpellPackets.cpp/h`.
- `combat.rs` `AttackStart`, `SAttackStop`, `AIReaction`, `CancelCombat`, `BreakTarget`, y campos internos simples de `AttackerStateUpdate` -> `CombatPackets.cpp/h` y `CombatLogPackets.cpp/h`.
- `chat.rs` `ChatPkt`, `EmoteClient`, `CTextEmote`, `STextEmote`, `EmoteMessage` -> `ChatPackets.cpp/h`.
- `party.rs` CMSG/SMSG layouts revisados, salvo bugs runtime de invite/lifecycle -> `PartyPackets.cpp/h`, `AuthenticationPackets.cpp`, `GroupHandler.cpp`.
- `item.rs` `ItemInstance`, purchase/time/enchant serializers, `InvUpdate`, parsers de swap/equip/destroy/cancel, e `InventoryChangeFailure`, salvo `AutoStoreBagItem` -> `ItemPackets.cpp/h`, `ItemPacketsCommon.cpp/h`, `Player.h`.
- `misc.rs` dificultad, hotfix, movement-transfer, played-time, taxi-node-status, cemetery-list, query-time, XP/exploration/level-up layouts -> `MiscPackets.cpp/h`, `MovementPackets.cpp/h`, `HotfixPackets.cpp/h`, `CharacterPackets.cpp/h`, `TaxiPackets.cpp/h`, `QueryPackets.cpp/h`.
- `misc.rs` `SetProficiency` packet layout y spell effect runtime -> `ItemPackets.cpp/h`, `SpellEffects.cpp`, `Player.cpp/h`.
- `handlers/misc.rs` `QueryTime` runtime -> `QueryHandler.cpp`, `QueryPackets.cpp`.
- `character.rs` equipment cache parse -> `CharacterPackets.cpp`, `Player.cpp`.
- `loot.rs` packet layouts basicos, `SLootRelease` layout, corpse release decay comun y `SetLootSpecialization` -> `LootPackets.cpp/h`, `LootHandler.cpp`, `Player.cpp`, `Creature.cpp`.
- `quest.rs` constantes count, `QuestXPEntry` layout, `RoundXPValue`, `QuestTemplate::is_available_for`, parser `CMSG_QUEST_GIVER_COMPLETE_QUEST`, parser `CMSG_QUEST_GIVER_CHOOSE_REWARD`, y layout base `QuestGiverRequestItems` -> `QuestDef.h/cpp`, `DB2Structure.h`, `QuestPackets.cpp/h`, `Player.cpp`.
- `item_stats.rs` valores del subset `ItemModType` y campos `ItemSparse` representados -> `ItemTemplate.h`, `DB2Structure.h`, `DB2LoadInfo.h`.
- `skill.rs` layout DB2 de `SkillLineAbility`/`SkillRaceClassInfo`, indice `SkillupSkillLineID`, `GetSkillRaceClassInfo`, `GetSkillRangeType` y helper `skill_rewarded_spells_like_cpp` -> `DB2Structure.h`, `DB2LoadInfo.h`, `DB2Stores.cpp`, `ObjectMgr.cpp`, `Player.cpp`.
- `movement.rs` `MovementInfo` read/write layout base, `SetActiveMover` parser, `MoveInitActiveMoverComplete` parser, movement processing flags revisados y `ValidateMovementInfo` representado -> `MovementPackets.cpp/h`, `Opcodes.cpp`, `MovementHandler.cpp`, `Player.cpp`.

## Pendientes de contraste byte-for-byte

No se han cerrado en esta pasada:

- Resto de `crates/wow-packet/src/packets/misc.rs` no cubierto por la pasada 9: serializers por dominio.
- `crates/wow-world/src/handlers/spell.rs`: runtime spell flow; el layout de `wow-packet/src/packets/spell.rs` ya tiene pasada propia arriba, pero el handler sigue pendiente.
- `crates/wow-world/src/handlers/quest.rs`: quedan pendientes POI, quest share completo, quest status multiple, gossip/menu completo, loaders/normalizacion completa, condicionales/localizacion y persistencia end-to-end. La pasada 14 solo cierra/abre los slices Quest indicados.
- `crates/wow-world/src/handlers/misc.rs`: handlers restantes fuera de worldport/area-trigger/cemetery/taxi/query-time.
- `crates/wow-world/src/handlers/loot.rs`: queda abierto en generacion de loot/templates/conditions, roll completo, master-loot remoto, GO/item/prospecting/milling storage y persistencia/respawn DB end-to-end. La pasada 11 solo cierra packets basicos y rutas core indicadas.
- `crates/wow-world/src/handlers/group.rs`: quedan pendientes handlers no cubiertos a fondo fuera de invite/accept/leave y los layouts de party documentados arriba.
- `crates/wow-world/src/handlers/character.rs`: quedan pendientes otros slices C# fuera de inventory item move/equip/store/destroy/cancel.
- `crates/wow-packet/src/packets/movement.rs` / `wow-world/src/handlers/movement.rs`: quedan pendientes monster spline completo, vehicle handlers fuera de MovementHandler y visibilidad/transport map-owned end-to-end. La pasada 16 solo cierra/abre los slices indicados.
- `crates/wow-data/src/*.rs`: quedan pendientes modulos no cubiertos en pasadas 14/15 y usos de stats/items/skills fuera de las rutas documentadas aqui.
- `crates/wow-database/src/*.rs`: statement coverage/nombres. Puede ser API interna, pero no debe usarse para afirmar paridad C++.

## Criterio para la siguiente pasada

Auditar por dominio, no por aparicion textual de C#:

1. Escoger un serializer/handler concreto.
2. Abrir Rust y C++ packet class/handler.
3. Comparar orden de bits, flush, enteros, strings, optionals y config.
4. Clasificar con uno de los cuatro estados de este documento.
5. Solo despues de clasificar, abrir bug o cambiar comentario en una fase separada.
