# 🚀 Estado de Migración RustyCore — WoW WotLK server en Rust

## 📊 Resumen Ejecutivo

**Proyecto**: rustycore — Migración de RustyCore (WoW 3.4.3.54261) de C# a Rust
**Dominio**: your-domain.example.com
**Rust Version**: 1.85 (edition 2024)
**Ubicación**: `/path/to/rustycore/`
**Referencia C#**: `/path/to/reference/`
**Estado General**: ~35% — infraestructura lista, jugador en mundo, combat/movement/loot funcional

---

## ✅ Infraestructura

- ✅ Workspace Rust con 29 crates
- ✅ **Compila limpio** (`cargo build --workspace`) — solo warnings, 0 errores, 323 tests
- ✅ Certificados SSL: `bnet_cert.pem`, `bnet_key.pem`, `bnet_fullchain.pem`
- ✅ Configuración: `BNetServer.conf`, `WorldServer.conf`

---

## ✅ Crates Foundation
- ✅ `wow-core` — tipos primitivos, GUID, Position, Time
- ✅ `wow-constants` — constantes del juego (ClientOpcodes, ServerOpcodes)
- ✅ `wow-config` — configuración
- ✅ `wow-logging` — logging (tracing)
- ✅ `wow-crypto` — SRP6, cifrado de mundo
- ✅ `wow-math` — matemáticas 3D (glam)
- ✅ `wow-collections` — colecciones custom

---

## ✅ Crates Infrastructure
- ✅ `wow-database` — SQLx + MariaDB (4 DBs: auth, characters, world, hotfix)
- ✅ `wow-network` — networking Tokio
- ✅ `wow-proto` — protobuf Battle.net (prost)
- ✅ `wow-data` — lectura DB2/DBC (WDC4 reader + HotfixBlobCache)

---

## ✅ Servidores

### bnet-server (Autenticación Battle.net)
- ✅ BNet TCP + TLS en puerto 1119
- ✅ REST API (Axum) en puerto 8081
- ✅ SRP6 login flow completo
- ✅ Handshake BNet protobuf con cliente WoTLK Classic
- ✅ Redirección al WorldServer

### world-server (Servidor del Mundo)
- ✅ TCP en puertos 8085/8086
- ✅ Handshake criptográfico con cliente
- ✅ Dispatch de packets por opcode via `inventory::submit!`
- ✅ `SessionStatus` guard (Auth / CharSelect / LoggedIn)

---

## ✅ Sistemas Implementados

### 🔐 Autenticación & Login
- ✅ `CMSG_AUTH_SESSION` — autenticación de cuenta
- ✅ Login sequence completa: `AccountDataTimes`, `TutorialFlags`, `FeatureSystemStatus`, `ClientCacheVersion`, `AvailableHotfixes`, `ConnectionStatus`, `SetTimeZoneInformation`, `LoginSetTimeSpeed`, `SetupCurrency`, `UndeleteCooldownStatusResponse`, `ServerTimeOffset`, `InitWorldStates`

### 👤 Character Select
- ✅ `CMSG_CHAR_ENUM` — lista de personajes desde DB
- ✅ `CMSG_PLAYER_LOGIN` — entrada al mundo
- ✅ Secuencia de login: `UpdateObject` (create player), `PhaseShiftChange`, `SendKnownSpells`, `SetProficiency`, `UpdateTalentData`, `SetSpellModifier`, `SetActionButtons`, `BindPointUpdate`, `InitWorldStates`

### 🧑 Jugador en el Mundo
- ✅ `SMSG_UPDATE_OBJECT` — create player block completo (UnitData + ObjectData)
- ✅ Spawn de criaturas cercanas (`send_nearby_creatures`)
- ✅ `SMSG_MONSTER_MOVE` — movimiento de NPCs
- ✅ `CMSG_LOGOUT_REQUEST` / `CMSG_LOGOUT_CANCEL` — logout con delay
- ✅ `CMSG_QUERY_TIME` → `SMSG_QUERY_TIME_RESPONSE`

### 🏃 Movimiento
- ✅ `MovementInfo` — leer/escribir (posición + flags)
- ✅ `CMSG_MOVE_*` (todos los opcodes de movimiento del cliente)
- ✅ `handle_movement` — actualiza `player_position`, comprueba aggro de criaturas
- ✅ `SMSG_MOVE_UPDATE` — broadcast de movimiento
- ✅ `CMSG_SET_ACTIVE_MOVER` / `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE`

### ⚔️ Combate
- ✅ `CMSG_ATTACK_SWING` → `handle_attack_swing`
- ✅ `CMSG_ATTACK_STOP` → `handle_attack_stop`
- ✅ `CMSG_SET_SHEATHED` → `handle_set_sheathed`
- ✅ `SMSG_ATTACK_START`, `SMSG_ATTACK_STOP`
- ✅ `SMSG_ATTACKER_STATE_UPDATE` — daño servidor-autoritativo
- ✅ `tick_combat_sync` — tick cada ~100ms, HP update vía `SMSG_UPDATE_OBJECT` (VALUES block)
- ✅ Session fields: `combat_target`, `in_combat`

### 🧠 Creature AI
- ✅ `CreatureAI` con state machine: `Idle`, `WalkingRandom`, `InCombat`, `Returning`, `Dead`
- ✅ Aggro radius: facción aliada = 0.0, hostil = 15.0 yardas
- ✅ Creature tick cada ~200ms
- ✅ Session field: `creatures: HashMap<ObjectGuid, CreatureAI>`

### 💬 Chat
- ✅ `CMSG_CHAT_MESSAGE_*` — mensajes de chat (Say, Yell, Party, etc.)
- ✅ `CMSG_CHAT_MESSAGE_WHISPER` — susurros
- ✅ `CMSG_CHAT_MESSAGE_EMOTE`
- ✅ `SMSG_CHAT` — respuesta del servidor
- ✅ `CMSG_CHAT_JOIN_CHANNEL` — stub (responde sin error)

### 🎒 Loot
- ✅ `CMSG_LOOT_UNIT` → `handle_loot_unit`
- ✅ `CMSG_LOOT_ITEM` → `handle_loot_item`
- ✅ `CMSG_LOOT_RELEASE` → `handle_loot_release`
- ✅ `SMSG_LOOT_RESPONSE`, `SMSG_LOOT_REMOVED`, `SMSG_LOOT_RELEASE`
- ✅ `generate_creature_loot()` — monedas basadas en nivel de criatura
- ✅ Session field: `loot_table: HashMap<ObjectGuid, CreatureLoot>`

### 📖 Spells & Habilidades
- ✅ `SMSG_SEND_KNOWN_SPELLS` — envío de spells conocidos
- ✅ Auto-aprendizaje de spells desde DBC por clase/raza
- ✅ `SMSG_SET_PROFICIENCY` — armas y armaduras por clase
- ✅ `SMSG_UPDATE_TALENT_DATA`
- ✅ `SMSG_SET_SPELL_MODIFIER` (flat y pct vacíos)
- ✅ `SMSG_SHOW_TRADE_SKILL` / `SMSG_SHOW_TRADE_SKILL_RESPONSE`

### ⏱️ Tiempo de Juego (`/played`)
- ✅ `CMSG_REQUEST_PLAYED_TIME (0x327A)` → `handle_request_played_time`
- ✅ `SMSG_PLAYED_TIME (0x26D5)` — packet con `total_played_time` + `level_played_time` + `trigger_event`
- ✅ `login_time: Option<Instant>` en `WorldSession` — set en `send_login_sequence`
- ✅ Columnas `totaltime`/`leveltime` confirmadas en tabla `characters` DB
- 🔄 **Pendiente**: cargar valores de DB en login y persistir en logout

### 🗺️ Misc Handlers (stubs funcionales)
- ✅ `CMSG_SET_SELECTION (0x3528)` — selección de unidad
- ✅ `CMSG_AREA_TRIGGER (0x31D6)` — area triggers (log + stub)
- ✅ `CMSG_REQUEST_CEMETERY_LIST (0x3179)` — responde lista vacía
- ✅ `CMSG_TAXI_NODE_STATUS_QUERY (0x34A8)` — responde NOT_KNOWN
- ✅ `CMSG_MOVE_TIME_SKIPPED (0x3A1B)` — ignorado limpiamente
- ✅ `CMSG_QUERY_NEXT_MAIL_TIME` — responde sin correo
- ✅ `CMSG_LOADING_SCREEN_NOTIFY`, `CMSG_VIOLENCE_LEVEL`, `CMSG_OVERRIDE_SCREEN_FLASH`, `CMSG_QUEUED_MESSAGES_END`
- ✅ `CMSG_CHAT_UNREGISTER_ALL_ADDON_PREFIXES`, `CMSG_SET_ACTION_BAR_TOGGLES`, `CMSG_SAVE_CUF_PROFILES`
- ✅ `CMSG_GUILD_SET_ACHIEVEMENT_TRACKING`, `CMSG_GET_ITEM_PURCHASE_DATA`, `CMSG_REQUEST_FORCED_REACTIONS`
- ✅ `CMSG_REQUEST_BATTLEFIELD_STATUS`, `CMSG_REQUEST_RATED_PVP_INFO`, `CMSG_REQUEST_PVP_REWARDS`
- ✅ `CMSG_DF_GET_SYSTEM_INFO`, `CMSG_DF_GET_JOIN_STATUS`, `CMSG_CALENDAR_GET_NUM_PENDING`
- ✅ `CMSG_GM_TICKET_GET_CASE_STATUS`, `CMSG_GUILD_BANK_REMAINING_WITHDRAW_MONEY_QUERY`
- ✅ `CMSG_BATTLE_PET_REQUEST_JOURNAL`, `CMSG_ARENA_TEAM_ROSTER`, `CMSG_REQUEST_RAID_INFO`
- ✅ `CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS`, `CMSG_REQUEST_LFG_LIST_BLACKLIST`, `CMSG_LFG_LIST_GET_STATUS`
- ✅ `CMSG_GET_ACCOUNT_CHARACTER_LIST`, `CMSG_REQUEST_COUNTDOWN_TIMER`, `CMSG_CALENDAR_GET`
- ✅ `CMSG_AUCTION_LIST_BIDDER_ITEMS`, `CMSG_AUCTION_LIST_OWNER_ITEMS`, `CMSG_AUCTION_LIST_PENDING_SALES`
- ✅ `CMSG_COMMERCE_TOKEN_GET_LOG`, `CMSG_GAME_OBJ_USE`, `CMSG_GAME_OBJ_REPORT_USE`, `CMSG_CLOSE_INTERACTION`

### 📢 Chat Broadcast ✅ COMPLETO (2026-02-27)
- ✅ `PlayerRegistry` (`DashMap<ObjectGuid, PlayerBroadcastInfo>`) en `wow-network/src/player_registry.rs`
- ✅ Registro en login (`register_in_player_registry`), baja en logout (`unregister_from_player_registry`)
- ✅ Posición actualizada en cada movimiento (`update_registry_position`)
- ✅ `/say` broadcast a 25y, `/yell` a 300y, `/emote` a 25y en el mismo mapa
- ✅ Whisper real: forwarding al target por nombre si está online; fallback a echo si offline
- ✅ `STextEmote` + `EmoteMessage` broadcast a jugadores cercanos
- ⛔ TODO: broadcast de movimiento y UpdateObject (CreatePlayer) a recién llegados

### 🔮 Spells Básicos ✅ FASE 1 (2026-02-27)
- ✅ `CMSG_CAST_SPELL` (0x329C) — parser completo (`CastSpellRequest`) en `wow-packet/src/packets/spell.rs`
- ✅ Validación: jugador conoce el spell → `SpellCastResult::NotKnown` si no
- ✅ `SMSG_SPELL_GO` (0x2C36) — respuesta inmediata (instant-cast model) con visual
- ✅ `CMSG_CANCEL_CAST` (0x329F), `CMSG_CANCEL_CHANNELLING` (0x326A) — stubs
- ✅ `CastFailed` (0x2C54) — enviado si spell no conocido
- ⛔ TODO Fase 2: efectos mecánicos (heal/damage) vía Spell.db2
- ⛔ TODO Fase 2: cast-time timers (SMSG_SPELL_START → wait → SMSG_SPELL_GO)
- ⛔ TODO Fase 2: cooldowns

### 🏪 Vendor System ✅ COMPLETO
- ✅ `VendorInventory` — lista items del vendor al jugador (filtra items no en Item.db2)
- ✅ `BuyItem` — compra real: deduce gold, crea item en DB, añade a inventario, envía `UpdateObject`
- ✅ `SellItem` — venta real: borra item de DB, suma gold, envía `UpdateObject`
- ✅ Gold (`player_gold`) — cargado de DB en login, actualizado en buy/sell, `Coinage` en VALUES update

### 😊 Emotes ✅
- ✅ `CMSG_EMOTE` — limpia estado emote del cliente
- ✅ `CMSG_SEND_TEXT_EMOTE` — envía `STextEmote` (texto en chat) + `EmoteMessage` (animación)
- ⛔ Animación correcta — BLOQUEADO: EmotesText.db2 cifrado, hotfixes vacío. Usa EmoteID directo como fallback.

### 💀 Corpse Despawn & Respawn ✅
- ✅ Criatura muerta sin lootear: permanece 30s, luego despawna automáticamente
- ✅ Criatura looteada (fully): despawna inmediatamente tras cerrar ventana de loot
- ✅ Respawn queue: criatura reaparece en su spawn point tras `respawn_time_secs`
- ✅ Envía `CREATE` block al cliente cuando respawnea

### 🛬 Taxi Status ✅
- ✅ `TaxiNodeStatusQuery` — devuelve `Unlearned` (2) para flight masters, `None` (0) para el resto
- ✅ Detecta flight master por `npc_flags & 0x2000`

### 📦 DB2 / Hotfix Data
- ✅ `Wdc4Reader` — lector de archivos DB2 formato WDC4
- ✅ `HotfixBlobCache` — cache en memoria de blobs por `(table_hash, record_id)`
- ✅ Sirve `SMSG_DB_REPLY` para `CMSG_DB_QUERY_BULK`
- ✅ Integrado en `world-server/src/main.rs` (carga en startup)

---

## 🔄 Crates en Esqueleto (próximas iteraciones)
- 🔄 `wow-map` — mapas e instancias
- 🔄 `wow-spell` — sistema de spells (lógica completa)
- 🔄 `wow-pvp` — PvP/battlegrounds
- 🔄 `wow-achievement` — logros
- 🔄 `wow-script` — scripting hooks
- 🔄 `wow-scripts` — scripts de contenido
- 🔄 `wow-recastdetour` — pathfinding (FFI)
- 🔄 `wow-social` — amigos/ignorados
- 🔄 `wow-chat` — canales de chat (solo Say/Yell/Whisper funcional)

---

## 🎯 Próximos Pasos

### Sesión siguiente — Media/Difícil
1. **Area Triggers reales** — `areatrigger_teleport` en world DB → teleport a destino
2. **CreatePlayer broadcast** — enviar bloque CREATE a sesiones que entran al mapa (multi-player visible)
3. **Spells Fase 2** — efectos mecánicos (heal/damage) cargando Spell.db2 + cast-time timers + cooldowns
4. **Auras** — `SMSG_AURA_UPDATE` — aplicar buff/debuff visible al cliente

### Sesión posterior — Difícil
5. **Pathfinding** — FFI Recast/Detour, mobs se mueven por navmesh
6. **Instancias/Dungeons**

---

## 📝 Patrones Clave (guía para futuras sesiones)

- **Two-step dispatch**: `inventory::submit!` OBLIGATORIO — sin él el opcode no se despacha
- **Collect-then-send**: `Vec<Vec<u8>>` antes de enviar para evitar borrow conflicts
- **`send_packet` vs `send_tx`**: usar `send_tx.send(pkt.to_bytes())` en tick methods (no `send_packet`) para evitar double-borrow
- **`Position` fields**: `.x`, `.y`, `.z`, `.orientation` (NO `.o`)
- **`ClientPacket` trait**: `use wow_packet::ClientPacket;` explícito en cada handler

### Archivos clave
| Archivo | Contenido |
|---------|-----------|
| `crates/wow-world/src/handlers/character.rs` | ~3300 líneas — login, char select, played time |
| `crates/wow-world/src/session.rs` | ~1220 líneas — WorldSession, tick loop |
| `crates/wow-world/src/handlers/misc.rs` | ~560 líneas — 40+ handlers misc/stub |
| `crates/wow-packet/src/packets/misc.rs` | ~1930 líneas — todos los packets misc |
| `crates/wow-packet/src/world_packet.rs` | bit-packing: `read_bit`, `write_bit`, `flush_bits`, packed GUID |
| `crates/wow-constants/src/opcodes.rs` | todos los opcodes cliente/servidor |

---

**Última actualización**: 2026-02-27
**Build**: `cargo build --workspace` → ✅ 0 errores, 138 handlers
