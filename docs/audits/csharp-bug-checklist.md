# Checklist de auditoria C# vs C++

Fecha: 2026-06-27

Fuente:

- Inventario de referencias: `docs/audits/csharp-reference-audit.md`
- Contraste C++ y veredictos: `docs/audits/csharp-reference-contrast.md`

Regla operativa:

- Ninguna referencia C# se acepta como autoridad.
- Antes de marcar un bug como corregido, contrastar contra C++ local en
  `/home/server/woltk-trinity-legacy`.
- Cada fix debe tener test o verificacion proporcional al riesgo.
- Mantener este checklist actualizado en el mismo commit del fix/auditoria.

Estados:

- `[ ]`: pendiente.
- `[x]`: cerrado con fix o decision auditada.
- `no bug/comment`: comportamiento contrastado, queda reanclar comentario/docs.
- `audit`: falta contraste completo.

## Resumen

- Bugs confirmados contra C++: 51.
- Bugs corregidos: 7 (`#CSharpAudit.COMPRESS.1`, `#CSharpAudit.BNETREST.1`, `#CSharpAudit.BNETREST.2`, `#CSharpAudit.BNETREST.3`, `#CSharpAudit.FEATURE.1`, `#CSharpAudit.BNETSRP.1`, `#CSharpAudit.MOVEMENT.1`).
- Commit historico registrado: `#CSharpAudit.MOVEMENT.1` -> `98ceec4d`.
- Bugs pendientes de fix: 44.
- Referencias C# productivas localizadas hoy: 52 archivos bajo `crates/**`.
- Referencias C# documentales no-audit localizadas hoy: 39 archivos bajo `docs/**`.

## Criterio De Cierre Por Item

Para cerrar cualquier item de esta lista:

- Contrastar primero el Rust actual contra C++ local en `/home/server/woltk-trinity-legacy`.
- Documentar el ancla C++ usada en `docs/audits/csharp-reference-contrast.md` o en el item si es un cierre menor.
- Si hay cambio de implementacion, hacerlo en un commit propio con el checklist actualizado en el mismo commit.
- Anadir o actualizar test/verificacion proporcional al riesgo: byte-for-byte para packets, unit/integration para runtime, build/check para cambios compartidos.
- Marcar `[x]` solo cuando el fix o la decision auditada ya este verificada.
- No cerrar un subsistema por inferencia: si una referencia C# abre runtime, serializers, datos y docs, cada slice queda cerrado por separado.

## Cobertura De Referencias Productivas C#

Este mapa cubre los 52 archivos productivos con referencias C# encontrados con
`rg -l "C#|CSharp|CypherCore" crates --glob '!target/**'`. No implica fix; solo
indica donde se controla cada referencia.

| Grupo | Archivos | Control actual |
| --- | --- | --- |
| BNet REST/SRP/RPC/realm | `crates/bnet-server/src/main.rs`, `realm/mod.rs`, `rest/{handlers.rs,mod.rs,types.rs}`, `rpc/{session.rs,services/game_utilities.rs}` | `BNETREST.1-3` corregidos; bugs `BNETREST.4`, `BNETSRP.*` y RPC/realm/TLS quedan pendientes. |
| World login/session | `crates/world-server/src/main.rs` | Parcial en auth/account expansion/locale; pendiente reauditar todo el flujo world-login contra `WorldSocket.cpp`/`WorldSession.cpp`. |
| Network/crypto/auth | `crates/wow-network/src/{accept.rs,world_socket.rs}`, `crates/wow-crypto/src/{bnet_srp6.rs,ed25519ctx.rs,rsa_sign.rs,world_crypt.rs}` | Bugs BNet SRP; no-bug/comment en RSA/Ed25519/world crypt slices; `accept.rs` HMAC/seeds queda pendiente. |
| Packet core/compression | `crates/wow-packet/src/{world_packet.rs,compression.rs}` | `COMPRESS.1` corregido; bit APIs son no-bug/comment. |
| Packet serializers | `crates/wow-packet/src/packets/{auth.rs,character.rs,chat.rs,combat.rs,item.rs,misc.rs,movement.rs,party.rs,quest.rs,spell.rs}` | `FEATURE.1` corregido; bugs `SPELL`, `COMBAT`, `CHAT`, `PARTY`, `ITEM`, `MISC`, `QUEST`, `MOVEMENT`; slices no-bug/comment listados abajo. |
| World handlers | `crates/wow-world/src/handlers/{battlenet.rs,character.rs,chat.rs,combat.rs,group.rs,loot.rs,misc.rs,movement.rs,quest.rs,spell.rs}` | Bugs confirmados por dominio; `battlenet.rs`, spell runtime y restos de handlers quedan en `Pendientes De Auditoria`. |
| Data/model helpers | `crates/wow-data/src/{item_stats.rs,player_stats.rs,quest.rs,quest_xp.rs,skill.rs}` | Bugs `DATASTATS`, `ITEMSTATS`, `QUESTXP`, `SKILL`; slices DB2/helpers no-bug/comment abajo. |
| Database API/statements | `crates/wow-database/src/{lib.rs,params.rs,result.rs,statements/character.rs,statements/login.rs}` | Pendiente: no usar C# para afirmar coverage/nombres de statements. |
| Core/constants/proto/logging/AI | `crates/wow-core/src/guid.rs`, `crates/wow-constants/src/lib.rs`, `crates/wow-proto/src/lib.rs`, `crates/wow-proto/proto/bgs/low/pb/client/rpc_types.proto`, `crates/wow-logging/src/lib.rs`, `crates/wow-ai/src/lib.rs` | Pendiente salvo slices de corpse/loot ya contrastados; no usar C# como fuente de opcodes, proto, GUID semantics o logging. |

## Bugs Confirmados

- [x] `#CSharpAudit.COMPRESS.1` - Corregido; threshold de compresion usa payload sin opcode como C++ (`packet.size() > 0x400`). Test: `compression_threshold_uses_payload_len_like_cpp`.
- [x] `#CSharpAudit.BNETREST.1` - Corregido; `GET /bnetserver/login/` ya no emite `JSESSIONID`, igual que C++ `HandleGetForm`. Test: `login_form_headers_do_not_set_cookie_like_cpp`.
- [x] `#CSharpAudit.BNETREST.2` - Corregido; `POST /bnetserver/login/srp/` ya no emite ni depende de `JSESSIONID`; usa estado por conexion como C++ `LoginHttpSession`. Tests: `headers_do_not_set_cookie_like_cpp`.
- [x] `#CSharpAudit.FEATURE.1` - Corregido; `FeatureSystemStatus` y glue screen ya reflejan config C++ de support/BPay/undelete/max chars/expansion e `IsMuted = !CanSpeak()`. Tests: `feature_system_status_uses_cpp_config_flags`, `feature_system_status_glue_screen_uses_cpp_config_fields`.
- [x] `#CSharpAudit.BNETSRP.1` - Corregido; challenge v1 envia `iterations=1` como C++ `BnetSRP6v1Base::GetXIterations()`. Test: `challenge_v1_iterations_match_cpp`.
- [ ] `#CSharpAudit.BNETSRP.2` - Calculo SRP `u` omite padding fijo 128/256 de C++.
- [ ] `#CSharpAudit.BNETSRP.3` - `k` para v2+SHA512 omite padding fijo de C++.
- [ ] `#CSharpAudit.BNETSRP.4` - Normalizacion login/SRP usa uppercase Unicode; C++ solo upper Latin basico.
- [x] `#CSharpAudit.BNETREST.3` - Corregido; SRP challenge de cuenta inexistente devuelve JSON `authentication_state=DONE` como C++, sin HTTP 400. Test: `srp_challenge_missing_account_returns_done_like_cpp`.
- [ ] `#CSharpAudit.BNETREST.4` - Hex en challenge/login ticket es lowercase; C++ es uppercase.
- [ ] `#CSharpAudit.SPELL.1` - `SMSG_CAST_FAILED` omite `SpellCastVisual`.
- [ ] `#CSharpAudit.COMBAT.1` - `SMSG_ATTACKER_STATE_UPDATE` coloca bit combat-log dentro de `attackRoundInfo`; C++ lo escribe fuera antes del size.
- [ ] `#CSharpAudit.CHAT.1` - Rangos chat/emote/yell hardcodeados desde C#; C++ usa config `ListenRange.*`.
- [ ] `#CSharpAudit.CHAT.2` - `CMSG_EMOTE`/`CMSG_SEND_TEXT_EMOTE` runtime simplificado frente a C++.
- [ ] `#CSharpAudit.PARTY.1` - `SMSG_PARTY_INVITE` serializa campos vacios/default no C++.
- [ ] `#CSharpAudit.PARTY.2` - `HandlePartyInviteOpcode` omite validaciones C++.
- [ ] `#CSharpAudit.PARTY.3` - `PartyIndex` se lee pero se descarta.
- [ ] `#CSharpAudit.PARTY.4` - Invite response/leave group modelan lifecycle distinto al C++.
- [ ] `#CSharpAudit.ITEM.1` - `AutoStoreBagItem` lee `ContainerSlotA/B` invertidos.
- [ ] `#CSharpAudit.ITEM.2` - Inventory move/equip/store/destroy simplificados frente a C++.
- [ ] `#CSharpAudit.MISC.1` - `ShowTradeSkill` parsea payload y envia SMSG; C++ CMSG es `Null`.
- [ ] `#CSharpAudit.MISC.2` - `AuctionHelloResponse` serializa auction house id y delays no C++.
- [ ] `#CSharpAudit.CHARACTER.1` - Enum character flags/data mapping diverge.
- [ ] `#CSharpAudit.CHARACTER.2` - Logout request completa instantaneo siempre; C++ puede delayed/denegar/countdown.
- [ ] `#CSharpAudit.LOOT.1` - `LootResponse` success/error usa valores no C++.
- [ ] `#CSharpAudit.LOOT.2` - Apertura de loot no replica `Player::isAllowedToLoot`.
- [ ] `#CSharpAudit.LOOT.3` - `HandleLootMoneyOpcode` no aplica money aura ni criteria.
- [ ] `#CSharpAudit.LOOT.4` - `Player::StoreLootItem` no replica cascada C++ completa.
- [ ] `#CSharpAudit.PROFICIENCY.1` - `SetProficiency::default_*` son tablas C# no canonicas.
- [ ] `#CSharpAudit.MISCWORLD.1` - Far teleport/worldport ack no replica secuencia C++ completa.
- [ ] `#CSharpAudit.AREATRIGGER.1` - `CMSG_AREA_TRIGGER` parser/handler incompleto frente a C++.
- [ ] `#CSharpAudit.CEMETERY.1` - Cemetery list lee byte no C++ y responde lista vacia.
- [ ] `#CSharpAudit.TAXI.1` - Taxi node status solo usa NPC flag; C++ valida muchas gates.
- [ ] `#CSharpAudit.QUESTXP.1` - `QuestXP::calculate_xp` usa fila cercana; C++ devuelve `0` si falta nivel.
- [ ] `#CSharpAudit.QUESTXP.2` - XP minimo escalado ignora `CONFIG_MIN_QUEST_SCALED_XP_RATIO`.
- [ ] `#CSharpAudit.QUESTCMSG.1` - Query/accept quest leen bits como byte.
- [ ] `#CSharpAudit.QUESTPKT.1` - `SMSG_QUERY_QUEST_INFO_RESPONSE` omite `ReadyForTranslation`.
- [ ] `#CSharpAudit.QUESTPKT.2` - `QuestRewardsBlock` layout no C++.
- [ ] `#CSharpAudit.QUESTREWARD.1` - Reward dialogs alimentan rewards incompletos/no escalados.
- [ ] `#CSharpAudit.QUESTPKT.3` - `SMSG_QUEST_GIVER_QUEST_COMPLETE` omite `ItemReward`.
- [ ] `#CSharpAudit.QUEST.1` - `can_take_quest` omite timed/breadcrumb gates.
- [ ] `#CSharpAudit.QUEST.2` - Accept quest omite side-effects C++.
- [ ] `#CSharpAudit.QUEST.3` - Complete/request/choose reward runtime parcial.
- [ ] `#CSharpAudit.DATASTATS.1` - `player_stats.rs` y proyecciones usan tablas/formulas C# para stats finales.
- [ ] `#CSharpAudit.ITEMSTATS.1` - Aplicacion directa de stats de item es parcial frente a `_ApplyItemBonuses`.
- [ ] `#CSharpAudit.SKILL.1` - Starting skills no filtran `Availability == 1` ni `MinLevel`.
- [ ] `#CSharpAudit.SKILL.2` - Starting skill info no replica `LearnDefaultSkill`.
- [ ] `#CSharpAudit.SKILL.3` - Starting/racial spells legacy no replica `LearnSkillRewardedSpells`.
- [x] `#CSharpAudit.MOVEMENT.1` - Corregido en `98ceec4d`; `HandleMovementOpcode` usa mover actual (`GetUnitBeingMoved()` representado).
- [ ] `#CSharpAudit.MOVEMENT.2` - Runtime generico movement omite ramas C++ de teleport/spline/transport/vehicle/under-map y estado generico completo.
- [ ] `#CSharpAudit.MOVEMENT.3` - `MoveInitActiveMoverComplete` side effects no son 1:1.

## Deuda No Bug De Comportamiento

Estos slices se contrastaron como no bug en comportamiento revisado, pero aun deben reanclar comentarios/docs desde C# a C++.

- [ ] `world_packet.rs` bit APIs -> `ByteBuffer.h` / `ByteBuffer.cpp`.
- [ ] `world_crypt.rs` y `world_socket.rs` counters/tag/nonce -> `WorldPacketCrypt.cpp` / `AES.h`.
- [ ] `character.rs` response codes -> `SharedDefines.h`.
- [ ] `character.rs` enum/list layout -> `CharacterPackets.cpp/h`.
- [ ] `movement.rs` active mover packets -> `MovementPackets.cpp/h`.
- [ ] `misc.rs` AccountDataTimes -> `ClientConfigPackets.cpp/h`.
- [ ] `rsa_sign.rs` ConnectTo firma/layout -> `AuthenticationPackets.cpp` / `RSA.cpp`.
- [ ] `ed25519ctx.rs` y `world_socket.rs` EnterEncryptedMode -> `AuthenticationPackets.cpp` / `Ed25519.cpp`.
- [ ] `spell.rs` packet layouts sin bug en slice revisado -> `SpellPackets.cpp/h`.
- [ ] `combat.rs` packet layouts simples sin bug en slice revisado -> `CombatPackets.cpp/h`.
- [ ] `chat.rs` chat/emote packet layouts -> `ChatPackets.cpp/h`.
- [ ] `party.rs` CMSG/SMSG layouts revisados -> `PartyPackets.cpp/h`, `AuthenticationPackets.cpp`, `GroupHandler.cpp`.
- [ ] `item.rs` serializers/parsers revisados salvo `AutoStoreBagItem` -> `ItemPackets.cpp/h`.
- [ ] `misc.rs` serializers revisados de difficulty/hotfix/movement-transfer/played-time/taxi/cemetery/query-time/XP -> packet C++ correspondiente.
- [ ] `handlers/misc.rs` `QueryTime` runtime -> `QueryHandler.cpp`, `QueryPackets.cpp`.
- [ ] `character.rs` equipment cache parse -> `CharacterPackets.cpp`, `Player.cpp`.
- [ ] `loot.rs` packet layouts basicos y release/corpse decay comun -> `LootPackets.cpp/h`, `LootHandler.cpp`, `Player.cpp`, `Creature.cpp`.
- [ ] `quest.rs` constantes/layouts/helpers revisados -> `QuestDef`, DB2, `QuestPackets.cpp/h`, `Player.cpp`.
- [ ] `item_stats.rs` subset de `ItemModType` y `ItemSparse` -> `ItemTemplate.h`, DB2.
- [ ] `skill.rs` DB2 layouts/indices/helpers revisados -> DB2/ObjectMgr/Player C++.
- [ ] `movement.rs` `MovementInfo` base, `SetActiveMover`, `MoveInitActiveMoverComplete`, processing flags y `ValidateMovementInfo` representado -> Movement C++.

## Pendientes De Auditoria

- [ ] `crates/bnet-server/src/{main.rs,rest/**,rpc/**,realm/**}` fuera de bugs REST/SRP ya confirmados: TLS/ALPN, HTTP raw, RPC framing, realm list JSON/zlib y GameUtilities.
- [ ] `crates/world-server/src/main.rs` en login ticket, account expansion, account data y locale.
- [ ] `crates/wow-network/src/accept.rs` seeds/HMAC/debug contra `WorldSocket.cpp` y `AuthenticationPackets.cpp`.
- [ ] Resto de `crates/wow-packet/src/packets/misc.rs` no cubierto por pasadas actuales.
- [ ] `crates/wow-world/src/handlers/battlenet.rs` fallback BNet/GameUtilities contra service dispatch C++.
- [ ] `crates/wow-world/src/handlers/spell.rs` runtime spell flow.
- [ ] `crates/wow-world/src/handlers/quest.rs` POI, share, status multiple, gossip/menu, loaders, condiciones/localizacion y persistencia.
- [ ] `crates/wow-world/src/handlers/misc.rs` handlers fuera de worldport/area-trigger/cemetery/taxi/query-time.
- [ ] `crates/wow-world/src/handlers/loot.rs` loot generation/templates/conditions, roll completo, master-loot remoto, GO/item/prospecting/milling storage y persistencia/respawn DB.
- [ ] `crates/wow-world/src/handlers/group.rs` handlers fuera de invite/accept/leave y layouts ya documentados.
- [ ] `crates/wow-world/src/handlers/character.rs` slices C# fuera de inventory item move/equip/store/destroy/cancel.
- [ ] `crates/wow-packet/src/packets/movement.rs` / `wow-world/src/handlers/movement.rs` monster spline completo, vehicle handlers fuera de MovementHandler y visibilidad/transport map-owned end-to-end.
- [ ] `crates/wow-data/src/*.rs` modulos no cubiertos en pasadas 14/15 y usos fuera de rutas documentadas.
- [ ] `crates/wow-database/src/*.rs` statement coverage/nombres.
- [ ] `crates/wow-core/src/guid.rs:329` `HasEntry` semantics por tipo GUID.
- [ ] `crates/wow-constants/src/lib.rs:9` opcodes/flags contra C++ opcode enums y packet registries.
- [ ] `crates/wow-proto/src/lib.rs` y `proto/bgs/**` service hashes/proto contra C++/capturas.
- [ ] `crates/wow-logging/src/lib.rs` categorias/config logging contra C++ solo si afectan comportamiento o diagnostico operativo.
- [ ] `crates/wow-ai/src/lib.rs` corpse removal ya toca loot/corpse; mantener anclado a C++ al auditar AI/lifecycle.

## Docs Contaminadas

Estas referencias no son fuente de verdad. Se limpian solo cuando el dominio
correspondiente ya tenga ancla C++ o captura.

- [ ] `docs/MIGRATION_ROADMAP.md`
- [ ] `docs/world-entry-implementation.md`
- [ ] `docs/phase4-analysis-and-fixes.md`
- [ ] `docs/implementation-template.md`
- [ ] `docs/implementations/dynamic-visibility.md`
- [ ] `docs/implementations/move-init-active-mover-complete.md`
- [ ] `docs/implementations/set-active-mover.md`
- [ ] `docs/NPC_VENDOR_FLOW.md`
- [ ] `docs/migration/accounts.md`
- [ ] `docs/migration/movement.md`
- [ ] `docs/migration/bnetserver.md`
- [ ] `docs/migration/claude-porting-instructions.md`
- [ ] `docs/migration/client-tools.md`
- [ ] `docs/migration/commands.md`
- [ ] `docs/migration/common.md`
- [ ] `docs/migration/creature-port-no-gaps-plan.md`
- [ ] `docs/migration/shared-networking.md`
- [ ] `docs/migration/entities-conversation.md`
- [ ] `docs/migration/entities-corpse.md`
- [ ] `docs/migration/entities-creature.md`
- [ ] `docs/migration/entities-gameobject.md`
- [ ] `docs/migration/entities-player.md`
- [ ] `docs/migration/entities-sceneobject.md`
- [ ] `docs/migration/entities-transport.md`
- [ ] `docs/migration/db-schemas.md`
- [ ] `docs/migration/shared-datastores.md`
- [ ] `docs/migration/globals.md`
- [ ] `docs/migration/logging.md`
- [ ] `docs/migration/login-world-load-cpp-parity-plan.md`
- [ ] `docs/migration/migration-perf-strategy.md`
- [ ] `docs/migration/packets-by-domain.md`
- [ ] `docs/migration/pets.md`
- [ ] `docs/migration/proto.md`
- [ ] `docs/migration/current-session-handoff.md`
- [ ] `docs/migration/inventory/creature-port-matrix.tsv`
- [ ] `docs/migration/inventory/r2-product-scope.md`
- [ ] `docs/migration/inventory/r2-product-scope.tsv`
- [ ] `docs/migration/inventory/r8-entities-miniphase.md`
- [ ] `docs/migration/inventory/r8-entities-miniphase.tsv`
