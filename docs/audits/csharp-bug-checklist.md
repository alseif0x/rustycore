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
- Bugs corregidos: 21 (`#CSharpAudit.COMPRESS.1`, `#CSharpAudit.BNETREST.1`, `#CSharpAudit.BNETREST.2`, `#CSharpAudit.BNETREST.3`, `#CSharpAudit.BNETREST.4`, `#CSharpAudit.FEATURE.1`, `#CSharpAudit.BNETSRP.1`, `#CSharpAudit.BNETSRP.2`, `#CSharpAudit.BNETSRP.3`, `#CSharpAudit.BNETSRP.4`, `#CSharpAudit.MOVEMENT.1`, `#CSharpAudit.SPELL.1`, `#CSharpAudit.COMBAT.1`, `#CSharpAudit.CHAT.1`, `#CSharpAudit.CHAT.2`, `#CSharpAudit.ITEM.1`, `#CSharpAudit.PARTY.3`, `#CSharpAudit.PARTY.1`, `#CSharpAudit.MISC.1`, `#CSharpAudit.MISC.2`, `#CSharpAudit.LOOT.1`).
- Commit historico registrado: `#CSharpAudit.MOVEMENT.1` -> `98ceec4d`.
- Bugs pendientes de fix: 30.
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
| BNet REST/SRP/RPC/realm | `crates/bnet-server/src/main.rs`, `realm/mod.rs`, `rest/{handlers.rs,mod.rs,types.rs}`, `rpc/{session.rs,services/game_utilities.rs}` | `BNETREST.1-4` y `BNETSRP.1-4` corregidos; RPC/realm/TLS quedan pendientes. |
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
- [x] `#CSharpAudit.BNETSRP.2` - Corregido; calculo SRP `u` paddea `A`/`B` a 128 bytes en v1 y 256 bytes en v2 como C++ `CalculateU`. Tests: `compute_u_v1_pads_a_and_b_to_128_bytes_like_cpp`, `compute_u_v2_pads_a_and_b_to_256_bytes_like_cpp`.
- [x] `#CSharpAudit.BNETSRP.3` - Corregido; `k` para v2+SHA512 paddea `g` a 256 bytes como C++ `BnetSRP6v2`. Test: `compute_k_v2_sha512_pads_generator_to_256_bytes_like_cpp`.
- [x] `#CSharpAudit.BNETSRP.4` - Corregido; login/SRP username y password v1 usan `Utf8ToUpperOnlyLatin` como C++, sin uppercase Unicode completo. Tests: `utf8_to_upper_only_latin_matches_cpp_basic_latin_only`, `srp_username_does_not_apply_unicode_uppercase_like_cpp`.
- [x] `#CSharpAudit.BNETREST.3` - Corregido; SRP challenge de cuenta inexistente devuelve JSON `authentication_state=DONE` como C++, sin HTTP 400. Test: `srp_challenge_missing_account_returns_done_like_cpp`.
- [x] `#CSharpAudit.BNETREST.4` - Corregido; challenge REST, `server_evidence_m2` y login ticket usan hex uppercase como C++ `AsHexStr()`/`ByteArrayToHexStr()`. Tests: `hex_encode_uses_cpp_uppercase`, `make_login_ticket_uses_cpp_uppercase_hex`.
- [x] `#CSharpAudit.SPELL.1` - Corregido; `SMSG_CAST_FAILED` escribe `SpellCastVisual` entre `SpellID` y `Reason`, y `SpellCastVisual` ocupa un solo `int32` como C++ (`ScriptVisualID` esta comentado). Tests: `spell_cast_visual_serializes_one_int32_like_cpp`, `cast_failed_writes_visual_between_spell_and_reason_like_cpp`.
- [x] `#CSharpAudit.COMBAT.1` - Corregido; `SMSG_ATTACKER_STATE_UPDATE` escribe `WriteLogDataBit(false)`/`FlushBits()` en el paquete exterior antes de `attackRoundInfo.size()`, no dentro del sub-buffer. Test: `attacker_state_update_writes_custom_hit_info_like_cpp`.
- [x] `#CSharpAudit.CHAT.1` - Corregido; rangos say/text-emote/yell salen de `ListenRange.*` (`CONFIG_LISTEN_RANGE_*`) como C++ `World.cpp`, con fallback de codigo 25/25/300 y propagacion a `WorldSession`. Tests: `chat_listen_ranges_use_cpp_world_config_keys`, `say_uses_cpp_configured_listen_range_like_cpp`, `chat_emote_uses_cpp_configured_text_emote_range_like_cpp`, `send_text_emote_uses_cpp_configured_text_emote_range_like_cpp`, `yell_uses_cpp_configured_listen_range_like_cpp`.
- [x] `#CSharpAudit.CHAT.2` - Corregido; `CMSG_EMOTE` respeta alive/fake-death y limpia `EmoteState=EMOTE_ONESHOT_NONE`; `CMSG_SEND_TEXT_EMOTE` usa `EmotesText.db2` para validar/traducir a `Emote`, aplica ramas C++ de sleep/sit/kneel/none, dance/read, fake-death, `Unit::HandleEmoteCommand`, filtro de `SpellVisualKitIDs` por `Emotes.db2` mount-special, orden animacion antes de `STextEmote`, `VISIBILITY_RADIUS` para `SMSG_EMOTE`, `ListenRange.TextEmote` para `SMSG_TEXT_EMOTE`, y `RemoveAurasWithInterruptFlags(Anim)`. Anclas: `ChatHandler.cpp::HandleEmoteOpcode`, `ChatHandler.cpp::HandleTextEmoteOpcode`, `Unit.cpp::HandleEmoteCommand`, `DB2Structure.h::EmotesTextEntry`. Tests: `send_text_emote_requires_cpp_emotes_text_entry_like_cpp`, `send_text_emote_uses_cpp_configured_text_emote_range_like_cpp`, `send_text_emote_translates_emotes_text_and_uses_cpp_order_and_ranges_like_cpp`, `send_text_emote_keeps_spell_visual_kits_only_for_cpp_mount_special_like_cpp`, `send_text_emote_fake_death_skips_animation_but_keeps_text_like_cpp`.
- [x] `#CSharpAudit.PARTY.1` - Corregido; `SMSG_PARTY_INVITE` usa `ProposedRoles`, account GUID, realm actual/normalizado y `AllowMultipleRoles=false` como C++ `PartyInvite::Initialize`. Test: `party_invite_server_uses_cpp_inviter_values_like_cpp`.
- [ ] `#CSharpAudit.PARTY.2` - Parcial: corregidos valores `PartyResult` contra `SharedDefines.h`, `Group::IsFull()` 5/40 y checks de invite para target ya agrupado, permisos leader/assistant, raid de 5 miembros, GM target default, faccion default, instancia y dungeon difficulty; pendientes social-ignore/friend y knobs configurables `GM.AllowInvite`, `AllowTwoSide.Interaction.Group`, `PartyLevelReq` para cierre completo.
- [x] `#CSharpAudit.PARTY.3` - Corregido; invite/accept/leave conservan `PartyIndex` y no resuelven/fallan sobre HOME cuando C++ `Player::GetGroup(packet.PartyIndex)` pide otra categoria. Tests: `party_invite_party_index_instance_does_not_use_full_home_group_like_cpp`, `party_invite_response_party_index_mismatch_keeps_invite_pending_like_cpp`, `leave_group_party_index_instance_does_not_leave_home_group_like_cpp`.
- [ ] `#CSharpAudit.PARTY.4` - Invite response/leave group modelan lifecycle distinto al C++.
- [x] `#CSharpAudit.ITEM.1` - Corregido; `AutoStoreBagItem` lee `Inv`, `ContainerSlotB`, `ContainerSlotA`, `SlotA` como C++ `ItemPackets.cpp`, preservando `ContainerSlotA` como origen y `ContainerSlotB` como destino. Test: `auto_store_bag_item_parses`.
- [ ] `#CSharpAudit.ITEM.2` - Inventory move/equip/store/destroy simplificados frente a C++.
- [x] `#CSharpAudit.MISC.1` - Corregido; `CMSG_SHOW_TRADE_SKILL` se lee como `WorldPackets::Null`/`rfinish()` y el handler solo loguea sin enviar `SMSG_SHOW_TRADE_SKILL_RESPONSE`, que C++ marca `STATUS_UNHANDLED`. Tests: `show_trade_skill_reads_null_like_cpp`, `show_trade_skill_is_noop_null_like_cpp`.
- [x] `#CSharpAudit.MISC.2` - Corregido; `AuctionHelloResponse` escribe `Guid`, delays `uint32` en cero y bit `OpenForBusiness`, sin `AuctionHouseID`, como C++ `AuctionHousePackets.cpp`. Test: `auction_hello_response_writes_cpp_layout_without_auction_house_id`.
- [ ] `#CSharpAudit.CHARACTER.1` - Enum character flags/data mapping diverge.
- [ ] `#CSharpAudit.CHARACTER.2` - Logout request completa instantaneo siempre; C++ puede delayed/denegar/countdown.
- [x] `#CSharpAudit.LOOT.1` - Corregido; `LootResponse` success conserva `FailureReason=17` y `Threshold=2` como defaults C++, y error conserva `Threshold=2` mientras setea el `FailureReason` especifico. Anclas C++: `LootPackets.h:67-72`, `LootPackets.cpp:38-45`, `Player.cpp:8758-8765`, `Player.cpp:8778-8783`, `Loot.h:136-151`. Tests: `loot_response_success_defaults_write_cpp_failure_reason_and_threshold`, `loot_response_success_keeps_cpp_failure_and_threshold_defaults`, `loot_error_response_keeps_cpp_threshold_default_like_cpp`.
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
