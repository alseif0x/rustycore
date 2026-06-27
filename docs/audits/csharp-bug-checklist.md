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

## Bugs Confirmados

- [ ] `#CSharpAudit.COMPRESS.1` - Threshold de compresion usa longitud con opcode en Rust; C++ usa payload sin opcode.
- [ ] `#CSharpAudit.BNETREST.1` - `GET /bnetserver/login/` emite `JSESSIONID` en Rust; C++ no.
- [ ] `#CSharpAudit.BNETREST.2` - `POST /bnetserver/login/srp/` emite y depende de `JSESSIONID`; C++ usa session state HTTP.
- [ ] `#CSharpAudit.FEATURE.1` - `FeatureSystemStatus` y glue screen estan hardcoded a defaults y no reflejan config C++.
- [ ] `#CSharpAudit.BNETSRP.1` - Challenge v1 envia `iterations=0`; C++ envia `1`.
- [ ] `#CSharpAudit.BNETSRP.2` - Calculo SRP `u` omite padding fijo 128/256 de C++.
- [ ] `#CSharpAudit.BNETSRP.3` - `k` para v2+SHA512 omite padding fijo de C++.
- [ ] `#CSharpAudit.BNETSRP.4` - Normalizacion login/SRP usa uppercase Unicode; C++ solo upper Latin basico.
- [ ] `#CSharpAudit.BNETREST.3` - SRP challenge de cuenta inexistente revela HTTP 400; C++ devuelve `DONE`.
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

- [ ] Resto de `crates/wow-packet/src/packets/misc.rs` no cubierto por pasadas actuales.
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

## Docs Contaminadas

- [ ] `docs/world-entry-implementation.md`
- [ ] `docs/phase4-analysis-and-fixes.md`
- [ ] `docs/implementation-template.md`
- [ ] `docs/implementations/dynamic-visibility.md`
- [ ] `docs/implementations/move-init-active-mover-complete.md`
- [ ] `docs/implementations/set-active-mover.md`
- [ ] `docs/NPC_VENDOR_FLOW.md`
- [ ] `docs/migration/movement.md`
- [ ] `docs/migration/bnetserver.md`
- [ ] `docs/migration/shared-networking.md`
- [ ] `docs/migration/db-schemas.md`
- [ ] `docs/migration/shared-datastores.md`
- [ ] `docs/migration/globals.md`
- [ ] `docs/migration/current-session-handoff.md`
- [ ] `docs/migration/inventory/*`
