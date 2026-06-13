# Migration Index — TrinityCore wotlk_classic → RustyCore

> Índice maestro de los docs de migración por módulo. Cada doc sigue `_TEMPLATE.md`.
> Si un módulo no aparece aquí o no tiene su `.md`, **es bug del índice** — añadirlo.

**Cómo usar:**
- Para refinar el plan antes de implementar: seguir [refinement-plan.md](refinement-plan.md); la Fase R es precondicion para evitar gaps.
- Inventarios base contra C++: [inventory/cpp-server-tree.md](inventory/cpp-server-tree.md), [inventory/cpp-files-by-module.md](inventory/cpp-files-by-module.md), [inventory/cpp-handlers-opcodes.md](inventory/cpp-handlers-opcodes.md), [inventory/cpp-sql-prepared.md](inventory/cpp-sql-prepared.md), [inventory/cpp-dbc-db2-stores.md](inventory/cpp-dbc-db2-stores.md), [inventory/cpp-config-keys.md](inventory/cpp-config-keys.md), [inventory/cpp-entity-types.md](inventory/cpp-entity-types.md) y [inventory/cpp-scripts-tree.md](inventory/cpp-scripts-tree.md).
- Para entender cualquier sistema: abrir su doc → secciones 1-7 son C++; 8 es estado actual; 9-12 son plan de trabajo; 13 es la auditoría 2026-05-01.
- Para auditar un módulo: ya lo está auditando este doc. Update sección 8 + columna "Audited" cuando termines.
- Para empezar a implementar: leer el doc del módulo + dependencias listadas en sección 5 + sección 13 (audit).

**Reglas:**
- No modificar código del módulo X sin leer su `<modulo X>.md` § 13 primero (ADR-007 del ROADMAP).
- Si encuentras divergencia C++ vs Rust no documentada: añadirla al doc en sección 8 ("Suspicious") + abrir issue/TODO en sección 9.
- La auditoría 2026-05-01 fue un primer barrido hecho por agentes y sirve como triage, no como prueba final. Antes de implementar o cerrar un punto, contrastar de nuevo contra C++.

---

## Tabla maestra

Leyenda Status: ❌ not started / ⚠️ partial / ✅ done / 🔧 broken.
Leyenda Audit: ❌ no auditado / ⚠️ auditado, divergencias documentadas / ✅ auditado y sin gaps abiertos (incl. n/a confirmados).
Layer: L0–L8 según `MIGRATION_ROADMAP.md` § 2.

### L0–L1 — Foundation & Infrastructure

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L0 | Globals | `src/server/game/Globals/` | [globals.md](globals.md) | ⚠️ ~8% | ⚠️ |
| L0 | Time | `src/server/game/Time/` | [time.md](time.md) | 🔧 (`to_packed` broken) | ⚠️ |
| L0 | Texts | `src/server/game/Texts/` | [texts.md](texts.md) | ❌ | ✅ |
| L0 | Tools | `src/server/game/Tools/` | [tools.md](tools.md) | ❌ | ✅ |
| L0 | Miscellaneous | `src/server/game/Miscellaneous/` | [miscellaneous.md](miscellaneous.md) | ⚠️ ~50% | ⚠️ |
| L1 | shared/Networking | `src/server/shared/Networking/` | [shared-networking.md](shared-networking.md) | ⚠️ ~75% | ⚠️ |
| L1 | shared/Packets | `src/server/shared/Packets/` | [shared-packets.md](shared-packets.md) | ⚠️ | ⚠️ |
| L1 | Crypto (SRP6/AES-GCM/HMAC) | `src/server/shared/Cryptography/` + `src/common/Cryptography/` | [crypto.md](crypto.md) | ✅ ~95% | ⚠️ |
| L1 | shared/Realm | `src/server/shared/Realm/` | [shared-realm.md](shared-realm.md) | ⚠️ (~78%; BNet RealmHandle packing/storage/cfg swap, typed flags/types, hostname resolution, subregion writer, normalized names, build-version lookup, `JamJSONRealmEntry` and strong RealmHandle fixed; golden/e2e pending) | ⚠️ |
| L1 | shared/Secrets | `src/server/shared/Secrets/` | [shared-secrets.md](shared-secrets.md) | ❌ (0%) | ⚠️ |
| L1 | shared/DataStores | `src/server/shared/DataStores/` | [shared-datastores.md](shared-datastores.md) | ⚠️ ~1.5% | ⚠️ |
| L1 | shared/Dynamic | `src/server/shared/Dynamic/` | [shared-dynamic.md](shared-dynamic.md) | n/a | ✅ |
| L1 | shared/JSON | `src/server/shared/JSON/` | [shared-json.md](shared-json.md) | n/a | ✅ |
| L1 | proto/ | `src/server/proto/` | [proto.md](proto.md) | ⚠️ (1% error codes) | ⚠️ |
| L1 | game/DataStores | `src/server/game/DataStores/` | [datastores.md](datastores.md) | ⚠️ (7/325 game-side) | ⚠️ |
| L1 | game/Storages | `src/server/game/Storages/` | [storages.md](storages.md) | ❌ (CMSG_WHO unregistered) | ✅ |
| L1 | game/Cache | `src/server/game/Cache/` | [cache.md](cache.md) | ❌ | ✅ |
| L1 | game/Accounts | `src/server/game/Accounts/` | [accounts.md](accounts.md) | ❌ (no AccountMgr/RBAC) | ⚠️ |
| L1 | game/Services | `src/server/game/Services/` | [services.md](services.md) | ⚠️ (BNet RPC NotImplemented) | ⚠️ |
| L1 | game/Support | `src/server/game/Support/` | [support.md](support.md) | ❌ | ✅ |

### L2 — Packets & Dispatch

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L2 | game/Handlers | `src/server/game/Handlers/` | [handlers.md](handlers.md) | ⚠️ ~22% (2 bugs/5 spot-checks = 40%) | ⚠️ |
| L2 | game/Server | `src/server/game/Server/` | [server.md](server.md) | ⚠️ ~23% | ⚠️ |

### L3 — World & Maps

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L3 | Maps | `src/server/game/Maps/` | [maps.md](maps.md) | 🔧 (no Map::Update) | ⚠️ |
| L3 | Grids | `src/server/game/Grids/` | [grids.md](grids.md) | 🔧 (coords/cell base ported; NGrid/GridState missing) | ⚠️ |
| L3 | World | `src/server/game/World/` | [world.md](world.md) | ❌ (no `struct World`) | ⚠️ |
| L3 | Phasing | `src/server/game/Phasing/` | [phasing.md](phasing.md) | ❌ (always Unphased=0x08) | ⚠️ |

### L4 — Entities

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L4 | Entities | `src/server/game/Entities/` | [entities.md](entities.md) | 🔧 ~5% (no UpdateMask) | ⚠️ |
| L4 | Entities/Pet | `src/server/game/Entities/Pet/` | [pets.md](pets.md) | ❌ (16 opcodes + 1 stub) | ✅ |
| L4 | Entities/Item (Inventory) | `src/server/game/Entities/Item/` + Player inventory | [inventory.md](inventory.md) | ⚠️ ~26% (base Item+Bag + Player storage/ObjectAccessor lookup + visible item/modifier state; InventoryType bridge fixed; no ownership/runtime/DB) | ⚠️ |

> Nota: Entities es un mega-módulo con ~16 sub-tipos (Object, WorldObject, Unit, Player, Creature, GameObject, Pet, DynamicObject, AreaTrigger, Conversation, Corpse, Vehicle, Transport, SceneObject, Totem, Item). Si su doc supera ~500 líneas, splitear en `entities-<subtipo>.md`.

### L5 — Engines

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L5 | Movement | `src/server/game/Movement/` | [movement.md](movement.md) | ⚠️ (parser-only, 0 generators) | ⚠️ |
| L5 | Combat | `src/server/game/Combat/` | [combat.md](combat.md) | ❌ (`wow-combat` 0 lines) | ✅ |
| L5 | Spells | `src/server/game/Spells/` | [spells.md](spells.md) | ❌ (`wow-spell` 0 lines) | ✅ |
| L5 | AI | `src/server/game/AI/` | [ai.md](ai.md) | ❌ (no SmartAI; 1 struct) | ✅ |

### L6 — Game systems

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L6 | Quests | `src/server/game/Quests/` | [quests.md](quests.md) | ⚠️ ~5% (daily flag bug) | ⚠️ |
| L6 | Loot | `src/server/game/Loot/` | [loot.md](loot.md) | ❌ (silent loot loss bug) | ⚠️ |
| L6 | Chat | `src/server/game/Chat/` | [chat.md](chat.md) | ⚠️ ~25% (proximity routing bug) | ⚠️ |
| L6 | Social (friends/ignore) | `src/server/game/Social/` | [social.md](social.md) | ⚠️ (`/ignore` no whisper filter) | ⚠️ |
| L6 | Groups | `src/server/game/Groups/` | [groups.md](groups.md) | ⚠️ ~8% | ⚠️ |
| L6 | Guilds | `src/server/game/Guilds/` | [guilds.md](guilds.md) | ❌ (89 opcodes, 2 stubs) | ✅ |
| L6 | Mails | `src/server/game/Mails/` | [mails.md](mails.md) | ❌ <1% | ✅ |
| L6 | AuctionHouse | `src/server/game/AuctionHouse/` | [auctionhouse.md](auctionhouse.md) | ⚠️ stub-only | ⚠️ |
| L6 | AuctionHouseBot | `src/server/game/AuctionHouseBot/` | [auctionhousebot.md](auctionhousebot.md) | ❌ | ✅ |
| L6 | Calendar | `src/server/game/Calendar/` | [calendar.md](calendar.md) | ⚠️ stub-only | ⚠️ |
| L6 | Achievements | `src/server/game/Achievements/` | [achievements.md](achievements.md) | ❌ (`wow-achievement` 0 bytes) | ✅ |
| L6 | Reputation | `src/server/game/Reputation/` | [reputation.md](reputation.md) | ⚠️ ~3% (1000 zero-pairs) | ⚠️ |
| L6 | Skills | `src/server/game/Skills/` | [skills.md](skills.md) | ⚠️ ~10-15% | ⚠️ |
| L6 | BlackMarket | `src/server/game/BlackMarket/` | [blackmarket.md](blackmarket.md) | n/a for WotLK; frozen/future-version only | ✅ |
| L6 | BattlePets | `src/server/game/BattlePets/` | [battlepets.md](battlepets.md) | n/a for WotLK; frozen/future-version only | ✅ |
| L6 | Petitions | `src/server/game/Petitions/` | [petitions.md](petitions.md) | ❌ (charter UI hang) | ⚠️ |
| L6 | Pools | `src/server/game/Pools/` | [pools.md](pools.md) | ❌ | ✅ |
| L6 | Conditions | `src/server/game/Conditions/` | [conditions.md](conditions.md) | ❌ (default-true keystone) | ⚠️ |

### L7 — Instances, BG, Arenas, PvP

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L7 | Instances | `src/server/game/Instances/` | [instances.md](instances.md) | ❌ (raid panel hangs) | ⚠️ |
| L7 | Battlegrounds | `src/server/game/Battlegrounds/` | [battlegrounds.md](battlegrounds.md) | ❌ (1/11 opcodes) | ⚠️ |
| L7 | Battlefield | `src/server/game/Battlefield/` | [battlefield.md](battlefield.md) | ❌ (zone unreachable) | ✅ |
| L7 | OutdoorPvP | `src/server/game/OutdoorPvP/` | [outdoorpvp.md](outdoorpvp.md) | ❌ (5 zones) | ✅ |
| L7 | DungeonFinding | `src/server/game/DungeonFinding/` | [dungeonfinding.md](dungeonfinding.md) | 🔴❌ (CRIT: CMSG_DF_JOIN unregistered → infinite spinner) | ⚠️ |
| L7 | Scenarios | `src/server/game/Scenarios/` | [scenarios.md](scenarios.md) | n/a (post-WoLK) | ✅ |

### L8 — Content & service

| Layer | Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|---|
| L8 | Scripting | `src/server/game/Scripting/` | [scripting.md](scripting.md) | ❌ (`wow-script` 0 bytes) | ✅ |
| L8 | scripts/ (content) | `src/server/scripts/` | [scripts.md](scripts.md) | ❌ (`wow-scripts` 0 bytes; 294k LOC C++) | ✅ |
| L8 | Warden | `src/server/game/Warden/` | [warden.md](warden.md) | ❌ (3 opcode constants) | ✅ |
| L8 | Events | `src/server/game/Events/` | [events.md](events.md) | ❌ | ✅ |
| L8 | Weather | `src/server/game/Weather/` | [weather.md](weather.md) | ❌ | ✅ |
| L8 | Achievements scripts | (parte de Achievements) | (en achievements.md) | — | — |

### Binarios

| Module | C++ path | Doc | Status | Audit |
|---|---|---|---|---|
| bnetserver | `src/server/bnetserver/` | [bnetserver.md](bnetserver.md) | ⚠️ (base64 ticket bug) | ⚠️ |
| worldserver | `src/server/worldserver/` | [worldserver.md](worldserver.md) | 🔧 (canonical map tick + gated legacy creature runtime bridge; full `World::Update` owner still open) | ⚠️ |

---

## Conteo

- **Total módulos enumerados:** 64.
- **Docs creados:** 64/64.
- **Docs auditados vs C++:** 64/64 — Fase A primer barrido completo (2026-05-01).
- **Status real (post-audit):**
  - 🔧 broken: 5 (worldserver, maps, grids, entities, time)
  - ❌ not started: 28
  - ⚠️ partial: 27
  - ✅ done: 0 (todos los antiguos ✅ downgraded)
  - n/a: 4 (shared-dynamic, shared-json, battlepets, scenarios)

> **Verdict global Fase A:** la primera ronda de auditoría revela que el server está al ~5-15% funcional vs C++ TrinityCore. Los 9 módulos que estaban marcados ✅ antes de auditar todos requerían downgrade. Los crates `wow-combat`, `wow-spell`, `wow-script`, `wow-scripts`, `wow-loot` están a 0 líneas. La capa de wire/handler es lo único vivo; debajo, la simulación es prácticamente no-op. Tres bugs reales de wire confirmados: `read_quest_choose_reward` corrompe selección, `quest.is_repeatable` flag wrong, BG/loot/social handlers silenciosos.

---

## Bugs reales detectados durante auditoría (priorizados)

| Pri | Módulo | Bug | Impacto | Ref |
|---|---|---|---|---|
| 🔴 P0 | dungeonfinding | `CMSG_DF_JOIN` no registrado → spinner infinito en "Find Group" | UX bloqueante | `dungeonfinding.md` §13 |
| 🔴 P0 | worldserver | No existe `MapManager::update()` global — mundo congelado con 0 sesiones | Simulación rota | `worldserver.md` §13 |
| 🔴 P0 | grids | `GRID_SIZE=64` en Rust vs `533.33` en C++ — creature.position hashea a grid wrong | Spawns mal ubicados | `grids.md` §13 |
| 🔴 P0 | entities | Sin `UpdateMask` — cada broadcast es full re-create | Banda + CPU | `entities.md` §13 |
| 🟠 P1 | handlers | `handle_quest_giver_choose_reward` decodifica wrong (Choice = u32,u32 vs LootItemType+ItemInstance+Quantity) | Selección de reward corrompida | `handlers.md` §13 |
| 🟠 P1 | conditions | `evaluate_conditions` ausente; vendor/loot/gossip/trainer default-true | Items condicionales se muestran a todos | `conditions.md` §13 |
| 🟠 P1 | quests | `is_repeatable` chequea `0x4000 (DEPRECATED)` en vez de `0x1000 (DAILY)` | Daily quests no resetean | `quests.md` §13 |
| 🟠 P1 | loot | `CMSG_LOOT_ITEM` marca slot taken pero NO añade item al inventario | Pérdida silenciosa de loot | `loot.md` §13 |
| 🟠 P1 | chat | Party/raid/guild routing por proximidad en vez de membership | Privacy leak + dropped msgs | `chat.md` §13 |
| 🟠 P1 | social | `/ignore` no filtra whispers; `INSERT IGNORE` en vez de `REPLACE` | Acoso no bloqueable | `social.md` §13 |
| 🟠 P1 | reputation | Login envía 1000 pares (flags=0, standing=0) hardcoded | Rep pane all-neutral | `reputation.md` §13 |
| 🟠 P1 | phasing | Stub envía siempre `PhaseShiftFlags::Unphased=0x08` | Wire mismatch latente | `phasing.md` §13 |
| 🟡 P2 | crypto | Ed25519 priv key hardcoded vs C++ que carga PEM por build_info | Login-blocker si pubkey mismatch | `crypto.md` §13 |
| 🟡 P2 | crypto | `compute_x` uppercases internally; C++ `Utf8ToUpperOnlyLatin` en caller | Latin-1 supplement break | `crypto.md` §13 |
| 🟢 fixed | shared-realm | `wow_realm_address = r.id` (raw) vs C++ packed `(Region<<24)\|(Site<<16)\|Realm`, y `cfg_timezones_id`/`cfg_categories_id` invertidos | Corregido 2026-06-13 con tests de realm-list JSON, subregion, packed address y JoinRealm lookup | `shared-realm.md` §13 |
| 🟡 P2 | bnetserver | `extract_auth_ticket` usa base64 verbatim (TC base64-decoda + truncate `:`) | Falla con tokens spec-compliant | `bnetserver.md` §13 |
| 🟡 P2 | shared-packets | `read_float` acepta NaN/Inf (C++ throws) | Hostile client poison | `shared-packets.md` §13 |
| 🟢 fixed | inventory | Item.db2 `inventory_type i8→u8` cast colisionaba con `INVENTORY_SLOT_BAG_0=255`; `INVTYPE_BAG=18` también iba a slot 3 | Cerrado en `#NEXT.R8.ENTITIES.046` | `inventory.md` §13 |
| 🟡 P2 | time | `to_packed` usa 30-day months + wrong year anchor | `SMSG_LOGIN_SET_TIME_SPEED` wrong | `time.md` §13 |
| 🟡 P2 | groups | race-5 Undead = Alliance; HP/power=1000/500 placeholder | Grupo party display roto | `groups.md` §13 |
| 🟡 P2 | server | 11 silent log-only no-ops (`Emote`, `SendTextEmote`, `WorldPortResponse`, `TrainerBuySpell`, 7 quest opcodes) | Cliente no obtiene response | `server.md` §13 |
| 🟡 P2 | server | 6 SessionStatus mismatches (CMSG_QUERY_* en Authed cuando C++ exige LoggedIn) | Pre-login DB lookups | `server.md` §13 |
| 🟡 P2 | server | 54 PacketProcessing mismatches (DB-touching en Inplace bloquea I/O) | Latencia bajo carga | `server.md` §13 |

---

## Histórico

| Fecha | Cambio |
|---|---|
| 2026-05-01 | Índice creado, 45 módulos enumerados, 0 docs todavía |
| 2026-05-01 | Primera ronda completa: 64/64 docs con plantilla de 12 secciones, plan de migración por módulo, gotchas y mapping C++→Rust. Audit aún 0/64. |
| 2026-05-01 | Fase A (audit) — primer batch: crypto / shared-packets / shared-networking auditados vs C++ wotlk_classic. Status downgrade: shared-networking ✅→⚠️ (~75%), shared-packets ✅→⚠️, crypto ✅→⚠️ pero validado byte-exact en SRP6/AES-GCM/nonce. Audit: 3/64. |
| 2026-05-01 | Fase A — segundo batch: server / proto / bnetserver / worldserver auditados. Hallazgos: server con 23% opcode coverage (145/621), worldserver con divergencia arquitectónica BREAKING (no existe MapManager::update; ticks viven en sesiones individuales — 0 sesiones = mundo congelado), bnetserver con bug de base64 ticket extraction, proto con 6/601 error codes mapeados. Audit: 7/64. |
| 2026-05-01 | Fase A — tercer batch: shared-secrets / shared-datastores auditados. shared-secrets ✅→❌ (SecretMgr ausente; doc anterior decía AES-CBC+HMAC pero TC usa AES-GCM 12-byte tag). shared-datastores ✅→⚠️ (1.5% tablas DB2 parseadas; HotfixBlobCache misnamed). Audit: 9/64. |
| 2026-05-01 | Fase A Wave A — game core: maps/grids/world/entities/pets/inventory/handlers + L5 motors. Hallazgos: GRID_SIZE 8.33× equivocado, sin UpdateMask, wow-combat/spell/movement crates 0 líneas, 2/5 spot-checks de handlers son bugs reales (40% defect). Audit: 19/64. |
| 2026-05-01 | Fase A Wave B — L6 game systems (16 módulos): quests/loot/chat/social/groups/guilds/skills/reputation/mails/auctions/calendar/achievements/petitions/pools/conditions/phasing + scripting/scripts/warden/events/weather. Bugs P0/P1 confirmados: silent loot loss, /ignore-no-filter, daily-flag wrong, condition default-true, phasing always-Unphased. Audit: 35/64. |
| 2026-05-01 | Fase A Wave C — L0/L1 foundation + L7 PvP + n/a markers (21 módulos). 🔴 CRIT: CMSG_DF_JOIN unregistered → "Find Group" infinite spinner. Bugs: time::to_packed broken, RealmHandle packing missing, services BNet RPC NotImplemented para todo. n/a confirmados: shared-dynamic/json/battlepets/scenarios. Audit: 64/64 — PRIMER BARRIDO COMPLETO. |

---

*Mantener actualizada esta tabla cada vez que se cree o modifique un doc de migración.*
