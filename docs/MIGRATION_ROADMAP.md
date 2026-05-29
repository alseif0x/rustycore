# Migration Roadmap — TrinityCore (wotlk_classic) → RustyCore (Rust)

> Plan operativo para migrar **todo** TrinityCore C++ a Rust. Este documento es la fuente de verdad para prioridad, orden y TODO list. El inventario de estado por módulo vive en `docs/migration/_INDEX.md`. Se actualiza al cierre de cada fase.

**Repos de referencia:**
- C++ origen: `/home/server/woltk-trinity-legacy` (TrinityCore branch `wotlk_classic`)
- Rust destino: `/home/server/rustycore` (este repo, GitHub `alseif0x/rustycore`)
- C# legacy: `/home/server/woltk-server-core/Source/` (referencia secundaria, mismo modelo)

**Reglas inviolables:**

1. **Antes de implementar** cualquier sistema, leer su contraparte C++ en TrinityCore. Nunca improvisar a oído. Lecciones del bridge MapManager fallido (`_attic/`) costaron 176 errores de compilación.
2. **Antes de extender** cualquier sistema ya migrado, **auditarlo contra C++**. Lo que está marcado ✅/⚠️ en este documento puede tener bugs, divergencias o piezas que faltan respecto al C++. **Nada se da por bueno hasta auditoría**. Un sistema "implementado" sin auditar es un riesgo, no una ventaja.
3. Los docs creados por agentes anteriores son útiles como índice, pero no son prueba de corrección. Cada task se valida contra C++ en el momento de ejecutarla.

### Revisión del plan 2026-05-07

Contraste realizado contra el árbol C++ real en `/home/server/woltk-trinity-legacy/src/server/`:

- Inventario top-level correcto: C++ contiene `bnetserver`, `worldserver`, `database`, `proto`, `shared`, `game` y `scripts`.
- `game/` contiene 49 subdirectorios funcionales si se ignora `PrecompiledHeaders`; el plan cubre todos por módulo o como parte de `Entities`/`Scripts`.
- `shared/` contiene 7 módulos reales si se ignora `PrecompiledHeaders`: `DataStores`, `Dynamic`, `JSON`, `Networking`, `Packets`, `Realm`, `Secrets`.
- `scripts/` no es un bloque genérico solamente: tiene `Commands`, `Spells`, `Battlefield`, `Events`, `OutdoorPvP`, `World` y scripts por continente/expansión. La fase de contenido debe mantener esa subdivisión cuando llegue.
- La matriz histórica de este roadmap estaba más optimista que `_INDEX.md`. Desde esta revisión, `_INDEX.md` manda para status/audit; este roadmap manda para orden de ejecución.
- La Fase 0 necesitaba afinarse: en C++ `ObjectGridLoader` no consulta directamente cada tabla por celda. Carga GUIDs preclasificados por `ObjectMgr`/`AreaTriggerDataStore` (`GetCellObjectGuids`, `GetAreaTriggersForMapAndCell`) a partir de `SpawnData`, difficulty, personal phases y respawn state. La cola inmediata se ajusta para no implementar un loader Rust incorrecto.
- R6 corrige el siguiente paso operativo: antes de continuar la Fase 0/L3 Maps, se ejecuta `#NEXT.L0.CONFIG.001` (L0 config parity / startup config schema). Maps no se descarta; queda bloqueado por los gates L0/L1/L2 definidos en R4.

---

## 1. Visión general

### 1.1 Topología C++ que hay que migrar

TrinityCore expone dos binarios ejecutables principales, una librería de scripts linkada al worldserver y 64 módulos documentados en `docs/migration/_INDEX.md`:

**Binarios / librerías:**
- `bnetserver` — autenticación Battle.net (BNet protobuf, SRP6, REST)
- `worldserver` — servidor de juego (sockets WoW, dispatch, todos los sistemas)
- (`scripts` se compila como librería linkada al worldserver)

**Capas (`src/server/`):**
```
shared/      Networking, Packets, Realm, Secrets, DataStores, Dynamic, JSON
game/        49 subdirectorios funcionales (ignorando PrecompiledHeaders)
proto/       definiciones protobuf BNet
database/    capa SQL común a todos los servidores
scripts/     contenido scripteado (commands, spells, continentes, world, events, PvP)
```

**game/** se subdivide en (alfabético, sin agrupar):
```
Accounts          Conditions       Loot          Pools         Spells
Achievements      DataStores       Mails         Quests        Storages
AI                DungeonFinding   Maps          Reputation    Support
AuctionHouse      Entities         Miscellaneous Scenarios     Texts
AuctionHouseBot   Events           Movement      Scripting     Time
Battlefield       Globals          OutdoorPvP    Server        Tools
Battlegrounds     Grids            Petitions     Services      Warden
BattlePets        Groups           Phasing       Skills        Weather
BlackMarket       Guilds           ─             ─             World
Cache             Handlers         ─             ─
Calendar          Instances        ─             ─
Chat              ─                ─             ─
Combat            ─                ─             ─
```

### 1.2 Topología Rust actual (29 crates)

```
crates/
  bnet-server       wow-database      wow-pvp
  world-server      wow-ecs           wow-recastdetour
  wow-achievement   wow-handler       wow-script
  wow-ai            wow-logging       wow-scripts
  wow-chat          wow-loot          wow-social
  wow-collections   wow-map           wow-spell
  wow-combat        wow-math          wow-world
  wow-config        wow-network
  wow-constants     wow-packet
  wow-core          wow-proto
  wow-crypto        wow-data
```

### 1.3 Métrica de avance

La métrica de estado por módulo se mantiene en `docs/migration/_INDEX.md`. No duplicar porcentajes antiguos aquí: ya demostraron quedarse obsoletos y optimistas.

Estado operativo tras el primer barrido de auditoría:

- Total módulos enumerados en `_INDEX.md`: 64.
- Docs por módulo: 64/64.
- Ningún módulo se considera `done` de forma plena contra C++; las marcas `✅` en docs antiguos deben tratarse como sospechosas si no tienen contraste de líneas C++ y tests.
- Estimación global útil para planificación: servidor funcional de forma muy parcial; no usar porcentajes altos heredados como criterio de prioridad.

---

## 2. Capas y dependencias

Grafo de dependencias (← lee como "X depende de Y"):

```
                 ┌─────────────────┐
                 │  L0 Foundation  │  core, constants, config, logging, math, collections
                 └────────┬────────┘
                          │
                 ┌────────┴────────┐
                 │  L1 Infra       │  crypto, database, network, proto, data (DB2/DBC)
                 └────────┬────────┘
                          │
                 ┌────────┴────────┐
                 │  L2 Packets     │  packet, handler (dispatch table)
                 └────────┬────────┘
                          │
                 ┌────────┴────────┐
                 │  L3 World/Maps  │  Map, MapManager, Grid, Cell, ObjectGridLoader  ◄── 🔧 rehacer
                 └────────┬────────┘
                          │
                 ┌────────┴────────────────┐
                 │  L4 Entities            │  Object/WorldObject/Unit/Player/Creature/GameObject
                 └────────┬────────────────┘
                          │
       ┌──────────────────┼──────────────────┬──────────────────┐
       │                  │                  │                  │
   ┌───┴─────┐      ┌────┴─────┐       ┌────┴─────┐      ┌────┴─────┐
   │ L5      │      │ L5       │       │ L5       │      │ L5       │
   │ Movement│      │ Combat   │       │ Spells   │      │ AI       │
   │ Path    │      │ Damage   │       │ Auras    │      │ Smart    │
   └───┬─────┘      └────┬─────┘       └────┬─────┘      └────┬─────┘
       └──────────────────┴──────────────────┴──────────────────┘
                                  │
                          ┌───────┴────────┐
                          │  L6 Game Systems│  Quests, Loot, Inventory, Social,
                          │                 │  Group, Chat, Vendor, Trainer, Mail,
                          │                 │  Auction, Calendar, Achievements,
                          │                 │  Reputation, Skills, Talents
                          └───────┬─────────┘
                                  │
                          ┌───────┴─────────┐
                          │  L7 Battlegrounds│  BG, Arena, OutdoorPvP, Battlefield,
                          │  Instances       │  Instance lock, Difficulty,
                          │  Phasing         │  PhaseMgr, Conditions
                          └───────┬─────────┘
                                  │
                          ┌───────┴─────────┐
                          │  L8 Content     │  Scripts (bosses, NPCs, instances)
                          │                 │  GM commands, Warden, LFG
                          └─────────────────┘
```

**Regla de oro**: una capa solo se considera "trabajable" cuando la inferior está al menos en estado **estable** (compila + tests). No se puede tocar L7 si L4 (entidades) está incompleto.

---

## 3. Estado por módulo (snapshot heredado)

> **No usar esta tabla para decidir si algo está correcto o terminado.** Se conserva como mapa visual de módulos, pero el estado operativo/auditado vive en `docs/migration/_INDEX.md`. Las marcas `✅` de esta sección son heredadas y no equivalen a "port completo contra C++".

Leyenda:
- ✅ done — implementado y tests verdes, cubre el 90%+ de la superficie C++
- ⚠️ partial — implementado parcialmente, falta funcionalidad significativa
- 🔧 broken — implementado pero diseño incorrecto, hay que rehacer
- ❌ missing — no empezado

### L0 Foundation

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Globals` | wow-core | ✅ | — |
| `Time` | wow-core | ✅ | — |
| `Miscellaneous` | wow-core / wow-collections | ✅ | — |
| `Texts` (string formatting) | wow-core | ⚠️ | i18n, broadcast text |

### L1 Infrastructure

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `shared/Networking` | wow-network | ✅ | — |
| `shared/Secrets` | wow-crypto | ✅ | — |
| `shared/DataStores` (DBC) | wow-data | ✅ | — |
| `game/DataStores` (cliente DB2) | wow-data | ⚠️ | varios stores: WMOAreaTable, AreaTable, MapDifficulty |
| `game/Storages` (server-side stores) | wow-data | ⚠️ | varios pendientes |
| `database/` | wow-database | ⚠️ | falta updater de schema, muchos prepared statements |
| `proto/` (BNet protobuf) | wow-proto | ✅ | — |
| `Cache` | wow-data | ⚠️ | hotfix cache OK, falta player cache |

### L2 Packets & Dispatch

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `shared/Packets` (header, encryption) | wow-packet | ✅ | — |
| `Handlers/` (138+ handlers) | wow-handler + wow-world/handlers | ⚠️ | ~75% packets cubiertos; faltan muchos opcodes |

### L3 World/Maps — 🔧 NÚCLEO DE REWRITE

> **DRIFT (2026-05-29):** este snapshot está desactualizado. Existe ya un `wow_map::MapManager`
> canónico con tick global (`spawn_canonical_map_update_loop`) y estructura tipo `Map::Update`,
> y loaders loaded-grid (`creature/gameobject_loaded_grid`). La realidad actual del runtime y la
> decisión de convergencia (tick ownership, sin doble tick; legacy = motor transitorio, canónico =
> destino estructural) están en **`docs/migration/adr-runtime-tick-ownership.md`**. Progreso honesto:
> `docs/migration/honest-progress-audit.md`. El trabajo de runtime vivo va por estas fases L3/L4,
> NO por el namespace `#NEXT.R8.ENTITIES.*`.

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Maps/Map` | wow-world/map_manager.rs | 🔧 | sin Cell anidado, sin máquina de estados, sin lifecycle |
| `Maps/MapManager` | wow-world/map_manager.rs | 🔧 | singleton OK pero sin update loop, sin DoForAllMaps con lock |
| `Grids/Grid` + `NGrid` | wow-map (parcial) | ❌ | falta NGrid/GridInfo/GridState; `Cell` base ya existe |
| `Grids/Cell` (8×8 dentro de NGrid) | wow-map::cell | ⚠️ | Cell base y GUID containers; falta visitante/entidades reales |
| `Grids/GridStates` (Active/Idle/Removal) | (no existe) | ❌ | sin máquina de estados, grids no se descargan |
| `Grids/ObjectGridLoader` | (no existe) | ❌ | sin lazy load DB → grid |
| `Maps/MapUpdater` (thread pool por map) | (no existe) | ❌ | actualmente todo serializa por RwLock global |
| `Maps/TerrainMgr` + `GridMap` | wow-map (coords/cell parcial) | ❌ | no hay carga de mapas .map/vmap/mmaps de cliente |
| `Maps/MapReference` / `MapRefManager` | (no existe) | ❌ | iteración de jugadores en map |
| `Phasing/PhaseMgr` | (no existe) | ❌ | personal/group phases |
| `Maps/SpawnData` | (no existe) | ❌ | unified spawn descriptors |

### L4 Entities

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Entities/Object/Object` (base) | wow-core (parcial) | ⚠️ | GUID OK, falta jerarquía polimórfica |
| `Entities/Object/WorldObject` | (mezclado en session) | ⚠️ | posición, mapa, fases, eventos |
| `Entities/Unit/Unit` | (no existe formal) | ❌ | health/power/stats/auras/threat — disperso en session |
| `Entities/Player/Player` | wow-entities + wow-world/session.rs | ⚠️ | base entidad iniciado; login/inventario/quests siguen mezclados en sesión |
| `Entities/Creature/Creature` | wow-entities + wow-ai/map_manager legacy | ⚠️ | base entidad iniciado; AI/spawn/loot siguen duplicados fuera |
| `Entities/GameObject/GameObject` | wow-entities + wow-world parcial | ⚠️ | base entidad iniciado; lifecycle/scripts siguen fuera |
| `Entities/Pet/Pet` | wow-entities | ⚠️ | base entidad iniciado; Create/Load/DB/AI pendiente |
| `Entities/DynamicObject` | wow-entities | ⚠️ | base entidad iniciado; Aura/Spell/Map runtime pendiente |
| `Entities/AreaTrigger/AreaTrigger` | wow-entities + wow-data/area_trigger | ⚠️ | base entidad iniciado; templates/runtime/actions pendientes |
| `Entities/Conversation` | wow-entities | ⚠️ | base entidad iniciado; data store/start/runtime pendiente |
| `Entities/Corpse` | wow-entities | ⚠️ | base entidad iniciado; create/load/persistence pendiente |
| `Entities/Vehicle` | wow-entities | ⚠️ | base kit/seats iniciado; auras/events/accessories pendiente |
| `Entities/Transport` (MO) | wow-entities | ⚠️ | base transport iniciado; TransportMgr/path/runtime pendiente |
| `Entities/SceneObject` | wow-entities | ⚠️ | base entidad iniciado; create/map/aura removal pendiente |
| `Entities/Totem` | wow-entities | ⚠️ | base entidad iniciado; TempSummon/Minion runtime pendiente |
| `Entities/Item` | wow-entities | ⚠️ | base Item+Bag+Player storage/ObjectAccessor lookup/visible item state iniciado; InventoryType y visible modifier helpers corregidos; ownership/DB/runtime pendiente |

### L5 Engines: Movement, Combat, Spells, AI

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Movement/MovementInfo` | wow-packet | ⚠️ | lectura base cercana a C++; writer fall-data y campos opcionales pendientes (#A06) |
| `Movement/MoveSpline` | (no existe) | ❌ | spline real con control points |
| `Movement/MovementGenerator` | (no existe) | ❌ | random/waypoint/follow/escort |
| `Movement/PathGenerator` (Detour) | wow-recastdetour | ❌ | crate scaffold, FFI no conectado |
| `Movement/spline/MoveSplineInit` | (no existe) | ❌ | constructor de splines server-side |
| `Combat/CombatManager` | wow-combat | ⚠️ | auto-attack OK, falta swap target / threat |
| `Combat/ThreatManager` | (no existe) | ❌ | sistema de aggro real |
| `Combat/Damage` (school, resistance, mitigation) | wow-combat | ⚠️ | physical OK, falta schools mágicas |
| `Spells/Spell` (engine de cast) | wow-spell | ⚠️ | cast OK, falta projectile, channel real |
| `Spells/SpellMgr` | wow-spell | ⚠️ | parcial |
| `Spells/SpellEffects` (151 efectos) | wow-spell | ⚠️ | DAMAGE/HEAL/AURA básicos, faltan ~140 |
| `Spells/Auras/AuraEffect` | wow-spell | ⚠️ | aura básico, falta periodic real |
| `Spells/SpellHistory` (cooldowns) | wow-world | ⚠️ | cooldowns visibles, falta GCD per-school |
| `AI/CreatureAI` (interfaz base) | wow-ai | ⚠️ | sí pero monolítica |
| `AI/SmartAI` (data-driven) | (no existe) | ❌ | smart_scripts table |
| `AI/ScriptedAI` (boss scripting) | wow-script | ❌ | crate vacío |
| `AI/PetAI` | (no existe) | ❌ | hunter/warlock pets |
| `AI/CombatAI` | (no existe) | ❌ | helper genérico para mobs |

### L6 Game Systems

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Quests/QuestDef` + `QuestMgr` | wow-data + handlers/quest | ⚠️ | accept/complete OK; falta quest pool, daily/weekly, escort, repeatable |
| `Loot/LootMgr` | wow-loot | ⚠️ | drops básicos; falta group rules, conditions, master loot |
| `Loot/LootPackets` | wow-packet | ✅ | — |
| `Skills` | wow-data | ⚠️ | tabla SkillLineAbility OK; falta skill gain, profession recipes |
| `Reputation/ReputationMgr` | (no existe) | ❌ | factions, paragon, exalted bonuses |
| `Chat/Chat` (channels) | wow-chat | ⚠️ | say/yell/whisper OK; falta global channels (Trade/General/LFG) |
| `Mails/MailMgr` | (no existe) | ❌ | sistema de correo COD/items |
| `AuctionHouse/AuctionMgr` | (no existe) | ❌ | listing, bidding, expiración |
| `AuctionHouseBot/` | (no existe) | ❌ | bot que compra/vende |
| `BlackMarket/` | (no existe) | ❌ | subastas especiales |
| `Calendar/CalendarMgr` | (no existe) | ❌ | eventos del calendario |
| `Achievements/AchievementMgr` | wow-achievement (vacío) | ❌ | criterios + progreso |
| `Groups/Group` | wow-social | ⚠️ | invite/accept/leave; falta loot rules, ready check, role check |
| `Guilds/Guild` | (no existe) | ❌ | guild bank, MOTD, ranks, achievements |
| `Petitions/Petition` | (no existe) | ❌ | charter para guilds/arenas |
| `Pools/PoolMgr` | (no existe) | ❌ | spawn pools (rotación de NPCs raros) |
| `Conditions/ConditionMgr` | (no existe) | ❌ | condiciones para drops, gossip, spells |
| `BattlePets/BattlePetMgr` | (no existe) | ❌ | sistema de mascotas combatientes (fuera de WoLK) |
| `OutdoorPvP/OutdoorPvP` (WG, EP, etc.) | wow-pvp (vacío) | ❌ | zonas PvP de mundo abierto |
| `Battlefield/Battlefield` (Wintergrasp) | wow-pvp | ❌ | WG es batalla de zona programada |

### L7 Instances, BG, Arenas, Phasing

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Instances/InstanceLockMgr` | (no existe) | ❌ | bloqueo de jugador a instancia |
| `Instances/InstanceScript` | (no existe) | ❌ | API de scripting de instancia |
| `Instances/InstanceSaveMgr` | (no existe) | ❌ | persistencia entre sesiones |
| `Battlegrounds/Battleground*` (WSG, AB, EotS, etc.) | wow-pvp | ❌ | colas, mapas, capturas |
| `Battlegrounds/ArenaTeamMgr` | wow-pvp | ❌ | rated arena |
| `Phasing/PhaseMgr` | (no existe) | ❌ | personal/group/spell phases |
| `Scenarios/Scenario*` | (no existe) | ❌ | escenarios de 3 jugadores (post-Cata) |
| `DungeonFinding/LFGMgr` | (no existe) | ❌ | cola dungeon |

### L8 Content + Service

| Módulo C++ | Crate Rust | Estado | Pendiente |
|---|---|---|---|
| `Scripting/ScriptMgr` | wow-script (vacío) | ❌ | API para scripts (boss, instance, gossip) |
| `scripts/` (~3000 scripts C++) | wow-scripts (vacío) | ❌ | content scripts — el grueso del trabajo total |
| `Chat/ChatCommands` (.tele, .gm, etc.) | (no existe) | ❌ | comandos GM |
| `Warden/Warden*` | (no existe) | ❌ | anticheat client-side |
| `Server/WorldSocket` | wow-network | ✅ | — |
| `Server/World` (loop principal) | world-server/main.rs | ⚠️ | tick loop OK, falta orquestación de Maps |
| `Support/Ticket` (GM tickets) | (no existe) | ❌ | sistema de tickets |
| `Accounts/AccountMgr` | wow-database | ⚠️ | login OK, falta gestión de cuenta GM |
| `Services/AccountService`, `BattlepayService` | wow-network | ⚠️ | BNet endpoints |
| `Weather/WeatherMgr` | (no existe) | ❌ | clima dinámico por zona |
| `Events/GameEventMgr` | (no existe) | ❌ | eventos temporales (Hallow's End, etc.) |
| `Cache` (player cache para queries) | (no existe) | ❌ | nombre→guid cache |

---

## 4. Fases de migración (orden ejecutable)

Cada fase es un commit (o pequeño grupo de commits) mergeable a `main` con `cargo check` + `cargo test` verdes. **No se salta a la siguiente sin la anterior cerrada.**

### Fase R — Refinamiento WBS completo (precondición)

> Antes de seguir implementando, convertir el plan en una estructura de tareas/subtareas verificable contra C++. El procedimiento completo vive en `docs/migration/refinement-plan.md`.

- **R.1** Inventariar el árbol C++ completo: archivos, handlers/opcodes, SQL, DB2/DBC, config, entidades y scripts.
- **R.2** Actualizar cada módulo de `docs/migration/*.md` con WBS granular: task IDs, C++ refs, dependencias, aceptación y tests.
- **R.3** Crear registros transversales para opcodes, SQL, update fields, runtime managers y scripts.
- **R.4** Revisar dependencias y gates de implementación; reordenar roadmap si C++ demuestra que el orden actual es incorrecto.
- **R.5** Hacer gap audit: ningún archivo/opcode/script relevante de C++ queda sin dueño o exclusión explícita.
- **R.6** Elegir la siguiente mini-fase solo cuando tenga C++ refs y tests definidos.

### Fase A — Auditoría obligatoria de lo existente (precondición)

> Esta fase **se ejecuta en paralelo** con Fase 0 y siguientes; cada módulo se audita **antes** de extenderlo. No bloquea Fase 0 (Maps ya está auditado y la conclusión es "rehacer"). Bloquea Fase 1+ porque Entities depende de saber qué hay realmente en `wow-core`/`wow-world`.

**Objetivo:** para cada módulo marcado ✅/⚠️ en sección 3, producir un mini-informe `docs/audits/<modulo>.md` con:

- Archivos C++ canónicos del módulo (cite líneas).
- Archivos Rust correspondientes y resumen de qué hacen.
- **Tabla de divergencias**: feature C++ → estado Rust → ¿bug? ¿missing? ¿extra? ¿correcto pero distinto y aceptable?
- TODO list de fixes específicos del módulo (pueden añadirse a la sección 5).
- Cambia la columna "Auditado vs C++" del módulo de ❌ → ⚠️ (si parcial) o ✅ (completo).

**Orden recomendado de auditoría** (por dependencias y por probabilidad de bugs):

- **A.1** Maps & Grids (✅ ya hecho, conclusión: rehacer en Fase 0)
- **A.2** Packets & Dispatch (alta superficie, alta probabilidad de divergencias en wire format)
- **A.3** Network (BNet handshake, WorldSocket encryption)
- **A.4** Crypto (SRP6, AES-GCM, HMAC) — cifrado roto = todo se cae
- **A.5** Database (statements, transacciones, prepared)
- **A.6** Foundation (GUID, Position, Time)
- **A.7** Movement (parsing y validación)
- **A.8** Combat (damage calc, miss table)
- **A.9** Spells & Auras
- **A.10** Quests
- **A.11** Inventory
- **A.12** Loot
- **A.13** Chat
- **A.14** Social, Group/Raid
- **A.15** Trainer/Vendor/Gossip
- **A.16** Resto

**Cómo auditar (proceso por módulo):**

1. Localizar archivos C++ de referencia (`/home/server/woltk-trinity-legacy/src/server/`).
2. Leer directamente los `.h/.cpp` relevantes y citar símbolos/archivos concretos. Si se usa cualquier agente auxiliar, su salida solo sirve como pista y se verifica manualmente contra C++.
3. Leer el código Rust correspondiente.
4. Producir tabla de divergencias.
5. Para cada divergencia: clasificar como **bug** (Rust diverge mal) / **missing** (Rust no implementa) / **extra** (Rust hace de más) / **OK** (divergencia aceptable, ej. idiom Rust).
6. Para bugs y missing críticos: añadir TODO en sección 5.
7. Commit `docs(audit): audit <modulo>` con el mini-informe.

### Fase 0 — Fundación de Maps (rehacer L3) — *🔧 siguiente tras gates L0-L2*

> El bloqueante de TODO lo demás. Sin Map/Grid/Cell correctos, ni entidades, ni AI, ni multi-player escalable.
> Nota R6: esta fase no se reanuda hasta cerrar `#NEXT.L0.CONFIG.001`.

- **0.1** `wow-map`: constantes (`SIZE_OF_GRIDS=533.3333`, `MAX_NUMBER_OF_GRIDS=64`, `MAX_NUMBER_OF_CELLS=8`), tipos `GridCoord`, `CellCoord`, `MapKey`, conversión `compute_grid_coord(x,y)` / `compute_cell_coord(x,y)`. Tests unitarios contra `GridDefines.h`. **Cerrado #001-#003.**
- **0.2** `wow-map`: `GridInfo`, `GridStateKind` y `NGrid` 8×8 `Cell`, siguiendo `NGrid.h` y `GridStates.cpp`.
- **0.3** `wow-map`: `Map` skeleton con `EnsureGridCreated`, `EnsureGridLoaded`, `LoadGridObjects`, `ResetGridExpiry`, `CanUnload`, `Update`, sin handlers todavía.
- **0.4** Spawn stores previos al loader: `SpawnData`, creature/gameobject/areatrigger spawn stores por `(map, difficulty, cell_id)` como C++ `ObjectMgr::AddSpawnDataToGrid` y `AreaTriggerDataStore`.
- **0.5** `ObjectGridLoader`: carga lazy desde esos stores, no desde queries ad hoc por celda; incluir corpses, respawn state y personal phase hooks como pendientes explícitos.
- **0.6** `MapManager` + `MapUpdater`: `i_maps`, `create_map`, `find_map`, `update`, `destroy_map`, delayed update y pool opcional según `MapManager.cpp`.
- **0.7** Integración worldserver: arrancar update loop global, reemplazar `crates/wow-world/src/map_manager.rs` legacy y migrar handlers que tocan `self.creatures`.
- **0.8** Quitar campos legacy `creatures`/`visible_creatures` de `WorldSession`.

### Fase 1 — Entidades canónicas (L4)

- **1.1** `wow-entities`: base `Object` / `WorldObject` con map/cell/phase/current cell y GUID typing contrastado contra `Entities/Object/`.
- **1.2** `ObjectAccessor`/map stored object registry equivalente a `Globals/ObjectAccessor.*` y `MapStoredObjectTypesContainer`; sin esto los handlers acabarán reintroduciendo lookups por sesión.
- **1.3** Update fields temprano (`Entities/Object/Updates/` + `UpdateFields.h`): masks/deltas por tipo antes de expandir Player/Unit, para no seguir generando full re-create.
- **1.4** `Unit`, `Player`, `Creature`, `GameObject`, `Corpse`, `DynamicObject`, `AreaTrigger`, `Pet`, `Transport`, `Vehicle`, `SceneObject`, `Conversation`, `Totem`; `Taxi` se trata como soporte de Player/Transport, no como sistema suelto.
- **1.5** Mover IA pura de `wow-ai` a `Creature`/AI refs sin mezclar comportamiento con sesión.
- **1.6** Refactor `WorldSession` para que `player` sea una referencia/controlador de entidad, no la entidad completa.

### Fase 2 — Movement & Pathfinding (L5)

- **2.1** `wow-movement` (renombrar/extender `wow-recastdetour`): MoveSpline real con control points.
- **2.2** Pathfinding: bindings FFI a Detour reales (no stubs), cargar navmesh `.mmtile` del cliente.
- **2.3** `MovementGenerator`: Idle / Random / Waypoint / Follow / Confused / Fleeing.
- **2.4** Server-side movement validation (anticheat básico): velocidad, jump, teleport range.

### Fase 3 — Combat & Threat (L5)

- **3.1** `wow-combat`: school resistances, miss/dodge/parry/block tablas reales por nivel.
- **3.2** `ThreatManager` per-Unit: tabla de threat, switch target, taunt.
- **3.3** Damage events: `SMSG_ATTACKER_STATE_UPDATE` con todos los campos (school mask, hit info, blocked, absorbed, resisted).
- **3.4** XP/Honor del kill (interactúa con L6 Quests para kill credit).

### Fase 4 — Spells & Auras (L5)

- **4.1** `wow-spell`: SpellEffect handlers para los 151 effects (al menos 30 más comunes en WoLK: damage, heal, aura, summon, teleport, charge...).
- **4.2** Aura periódica real (DoT/HoT con tick interval).
- **4.3** Channeled spells (mind flay, drain life).
- **4.4** Projectile spells (arrow, fireball con velocity).
- **4.5** GCD per-school + spell history persistente entre sesiones.

### Fase 5 — AI escalable (L5)

- **5.1** `wow-ai`: trait `CreatureAI` con métodos `update_ai`, `enter_combat`, `kill_unit`, `damage_taken`, `move_in_line_of_sight`.
- **5.2** `SmartAI` data-driven (lee `smart_scripts` de world DB).
- **5.3** `ScriptedAI` interfaz para scripts C++/Rust (boss en instancia).
- **5.4** `PetAI`, `CombatAI` genéricos.

### Fase 6 — Game systems pendientes (L6)

> Cada uno es un sub-proyecto. Orden por dependencias y prioridad de jugabilidad.

- **6.1** Inventory completo: bags, bank, durability, transmog, soulbound rules.
- **6.2** Chat channels globales (Trade, General, LookingForGroup) — depende de PhaseMgr para área.
- **6.3** Reputation: factions, paragon, repmod buffs.
- **6.4** Mail: items, COD, expiración, attachment limits.
- **6.5** Quest features avanzadas: pool, daily/weekly, escort, repeatable, area quests.
- **6.6** Achievements + criterios + persistencia.
- **6.7** Group: loot rules (FFA/group/master), ready check, role check, raid markers.
- **6.8** Guilds completas: bank, MOTD, ranks, perks, achievements.
- **6.9** Auction House + AHBot.
- **6.10** Calendar + events.
- **6.11** Black Market.

### Fase 7 — Instances, BG, Arenas, Phasing (L7)

- **7.1** Instance lock + difficulty + map switch flow (ConnectTo en realm separado).
- **7.2** InstanceScript trait + persistencia de estado.
- **7.3** Phasing: PhaseMgr por player y por área.
- **7.4** Conditions engine.
- **7.5** Battlegrounds (4-5 BGs WoLK: WSG, AB, EotS, AV, SotA, IoC) — colas, mapa, captura.
- **7.6** Arenas: rated, skirmish, conquista.
- **7.7** OutdoorPvP zones (WG, EP, HP, TF).
- **7.8** Battlefield (Wintergrasp como caso especial).
- **7.9** LFGMgr: cola dungeon finder.

### Fase 8 — Content & Service (L8)

- **8.1** ScriptMgr API: registro de scripts, hooks (boss, gossip, instance, npc, item, spell, area).
- **8.2** Migrar `scripts/Commands` primero para GM tooling mínimo (`.tele`, `.gm`, `.level`, `.item`, `.additem`, `.lookup...`), porque acelera la validación runtime de todo lo anterior.
- **8.3** Migrar `scripts/Spells`, `scripts/World`, `scripts/Events`, `scripts/Battlefield`, `scripts/OutdoorPvP` y scripts por zona/continente en bloques separados. No tratar `scripts/` como una masa única.
- **8.4** Warden (opcional, anticheat client-side).
- **8.5** Weather, GameEvents, Tickets, AccountMgr GM.

---

## 5. TODO list operativo (próximas 40+ acciones, ordenadas)

> Esta es la cola accionable. Cada ítem tiene un commit/PR esperado. Marcar `[x]` al cerrar.

### Auditorías iniciales (Fase A) — paralelas a Fase 0

> Cada auditoría produce `docs/audits/<modulo>.md` con tabla de divergencias y TODOs específicos.

- [ ] **#A01** Auditar **Packets & Dispatch** (`wow-packet`, `wow-handler`, `wow-world/handlers/`) vs `src/server/shared/Packets/` + `Handlers/`. ¿Wire format correcto? ¿Bit-packing fiel? ¿Opcodes en sync con cliente 3.4.3.54261?
- [ ] **#A02** Auditar **Network/WorldSocket** (`wow-network`) vs `src/server/Server/WorldSocket.cpp` + `WorldSocketMgr`. Encryption flow, header bytes, dispatch.
- [ ] **#A03** Auditar **Crypto** (`wow-crypto`) vs `src/server/shared/Cryptography/`. SRP6 idéntico al usado por cliente, AES-GCM nonce construction, HMAC-SHA256 keys.
- [ ] **#A04** Auditar **Database** (`wow-database`) vs `src/server/database/`. Statements registrados, prepared, transacciones, escapeo.
- [ ] **#A05** Auditar **Foundation** (`wow-core`) vs `src/server/game/Globals/` + `src/server/shared/`. GUID encoding, Position math, Time.
- [x] **#A06** Auditar **Movement parsing** (`wow-packet/movement.rs`, handlers/movement.rs) vs `src/server/game/Server/Packets/MovementPackets.*`, `Entities/Object/MovementInfo.h` y `Handlers/MovementHandler.cpp`. Resultado: `docs/audits/movement.md`; `MovementInfo::read` está cerca del wire C++, pero el writer y handler quedan en ⚠️ con subtareas:
  - [x] **#A06.1** Corregir `MovementInfo::write` para que `hasFallData = falling flags || fallTime != 0` y `hasFallDirection = falling flags`, como `MovementPackets.cpp`.
  - [x] **#A06.2** Representar `standingOnGameObjectGUID` e `inertia` en `MovementInfo`; no descartarlos al leer ni forzarlos a `false` al escribir.
  - [x] **#A06.3** Endurecer validación mínima de `handle_movement`: GUID cargado y exactamente el jugador actual, más `Position::is_valid_map_coord_like_cpp()` para X/Y/Z/orientación; los guards de teleport/movespline quedan ligados al runtime Movement/Map cuando existan equivalentes.
  - [x] **#A06.4** Validar transport sin integración `TransportBase` completa: offset ±75, distancia > grid tras teleport y world coord con transport. El reset/add-remove real de passenger queda para Transport/Map runtime.
  - [x] **#A06.5** Portar `AdjustClientMovementTime`: cola circular de 6 muestras, pendientes por sequence, delta filtrado por latencia y ajuste de `MovementInfo.time` antes de rebroadcast.
  - [ ] **#A06.6** Separar side effects C++ de movement: fall damage, aura interrupts, pet unsummon, sit-to-stand, under-map damage y jump procs.
    - [x] **#A06.6.1** Portar side effects representables hoy: remover auras `LandingOrFlight`/`Jump`, sentar-a-stand en movimiento y registrar hooks de unsummon temporal de pet / jump proc.
    - [x] **#A06.6.2** Portar base de `Player::HandleFall` / `UpdateFallInformationIfNeed`: `m_lastFallTime/Z`, umbral 14.57, fórmula C++ de daño, clamp a max health y aplicación al estado representado de salud/vida.
    - [x] **#A06.6.3** Portar under-map damage: `MapManager::min_height_like_cpp` expone el fallback C++ `TerrainInfo::GetMinHeight == -500.0` cuando no hay grid, `handle_movement` marca `PLAYER_FLAGS_IS_OUT_OF_BOUNDS` representado, aplica `DAMAGE_FALL_TO_VOID` por vida máxima y mata si el daño ambiental no lo hizo. La altura real de terrain/grid y el hook Battleground siguen en las tareas Map/Terrain/Battleground, no se finge aquí.
    - [x] **#A06.6.4a** Portar los modificadores/guards de aura usados por `Player::HandleFall`: `HOVER`, `FEATHER_FALL`, `FLY`, inmunidad normal, `SAFE_FALL`, `MODIFY_FALL_DAMAGE_PCT`, `CHEAT_GOD`, inmunidad ambiental y aura 43621 (Gust of Wind) sobre el modelo de aura representado actual.
    - [ ] **#A06.6.4** Conectar hooks registrados a Pet runtime y Proc/Aura runtime completos: `UnsummonPetTemporaryIfAny`, `Unit::ProcSkillsAndAuras(PROC_FLAG_JUMP)` y aura runtime real en vez de `visible_auras` representado.
  - [x] **#A06.7** Portar efectos de `MoveInitActiveMoverComplete`: lee `Ticks`, setea `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME`, calcula `TransportServerTime = GameTimeMS - Ticks`, emite VALUES update mínimo de `ActivePlayerData::{LocalFlags,TransportServerTime}` y refresca visibility como C++.
  - [ ] **#A06.8** Inventariar y portar ACK movement opcodes (`KnockBack`, speed, force, collision height, spline done, time skipped).
    - [x] **#A06.8a** Portar parsers/handlers base contrastados con C++ para ACKs genéricos (`HandleMovementAckMessage`), speed ACKs, `MoveKnockBackAck`, `MoveSetCollisionHeightAck`, `MoveTimeSkipped`, `MoveSplineDone` y `MoveTeleportAck`: Rust valida GUID/coord, ajusta time en knockback, suma `TimeSkipped` al tiempo de movimiento representado y registra eventos para los runtime gaps.
    - [x] **#A06.8b** Portar counters y anticheat representados de `HandleForceSpeedChangeAck` / `HandleMoveSetModMovementForceMagnitudeAck`: Rust mapea opcode a `UnitMoveType`, mantiene `m_forced_speed_changes` / `m_movementForceModMagnitudeChanges` equivalentes, compara contra `playerBaseMoveSpeed * rate` con tolerancia `0.01`, respeta transport como C++, registra corrección si el cliente va lento y kick si declara más velocidad/magnitud. El `SetSpeedRate` productivo, el paquete de corrección al cliente y `MovementForces::GetModMagnitude()` real quedan bajo `#A06.8g`/runtime Unit-MovementForce porque hoy no existe ese runtime completo.
    - [x] **#A06.8c** Emitir broadcasts C++ pendientes para ACKs portados: `SMSG_MOVE_UPDATE_KNOCK_BACK`, `SMSG_MOVE_SKIP_TIME`, `SMSG_MOVE_UPDATE_APPLY_MOVEMENT_FORCE`, `SMSG_MOVE_UPDATE_REMOVE_MOVEMENT_FORCE`, `SMSG_MOVE_UPDATE_MOD_MOVEMENT_FORCE_MAGNITUDE`, excluyendo al jugador local como C++.
    - [x] **#A06.8d** Portar `MovementForce` wire (`MoveApplyMovementForceAck`/`MoveRemoveMovementForceAck`): GUID, origin/direction XYZ, transport id, magnitude, `Unused910` y type de 2 bits; handlers validan GUID/coord, ajustan time y registran force id/type.
    - [x] **#A06.8e** Integrar side effects representables de `MoveSplineDone` con Taxi/MotionMaster: Rust distingue vuelo taxi en progreso, registra branch sin `FLIGHT_MOTION_TYPE`, representa teleport multi-map/teleport-node actualizando mapa/posición, limpia destino final como `CleanupAfterTaxiFlight`, remueve flags taxi/control, desmonta estado representado, resetea fall info y registra cast de Honorless Target 2479 si `pvpInfo.IsHostile`. El runtime real `PlayerTaxi`, `FlightPathMovementGenerator`, `SMSG_ON_MONSTER_MOVE` completo, costes/intermediate nodes y early landing quedan para la fase Taxi/MotionMaster porque hoy no existe esa base completa.
    - [x] **#A06.8f** Integrar side effects representables de `MoveTeleportAck` con near-teleport: Rust mantiene semáforo near y destino representado, ignora ACKs si no hay teleport o si el GUID no coincide, aplica mapa/posición destino, resetea fall info, actualiza zone/area representados, ejecuta ramas PvP/Honorless Target 2479 representadas, registra resummon temporal de pet y `ProcessDelayedOperations`. El runtime real `Player::TeleportTo`, `SMSG_MOVE_TELEPORT`, `SMSG_MOVE_UPDATE_TELEPORT`, access checks, vehicle/transport/duel/BG/arena cleanup y Map/Terrain zone resolver quedan para la fase Teleport/Map/Movement completa.
    - [ ] **#A06.8g** Reemplazar la validación conservadora de ACKs por `Player::ValidateMovementInfo` completo sobre flags cuando Unit/Aura/Vehicle runtime esté disponible.
      - [x] **#A06.8g.1** Portar la sanitización representable de `Player::ValidateMovementInfo`: Rust muta `MovementInfo.flags` en los ACK paths portados como C++ para `ROOT` sin vehículo fijo, pares incompatibles (`ASCENDING/DESCENDING`, `LEFT/RIGHT`, `STRAFE_LEFT/STRAFE_RIGHT`, `PITCH_UP/PITCH_DOWN`, `FORWARD/BACKWARD`), `HOVER`, `WATER_WALK`, `FALLING_SLOW`, `FLYING/CAN_FLY`, `FALLING` con `DISABLE_GRAVITY/CAN_FLY` y `SPLINE_ELEVATION` según `stepUpStartElevation`.
      - [ ] **#A06.8g.2** Conectar `ValidateMovementInfo` completo e integrado en todos los ACK/broadcasts cuando existan `Unit::HasAuraType`, `VehicleInfo::VEHICLE_FLAG_FIXED_POSITION`, mover controlado, `SPELL_AURA_FLY` vs `SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED` real y runtime Aura/Vehicle productivo. No dar esta pieza por cerrada con el modelo representado.
    - [ ] **#A06.8h** Portar `SMSG_ON_MONSTER_MOVE` / `MovementMonsterSpline` contra `MovementPackets.cpp` y `MoveSpline`.
      - [x] **#A06.8h.1** Corregir el wire puro de `MonsterMove`/`MonsterMoveStop`: Rust escribe `MovementMonsterSpline` + `MovementSpline` en el orden C++ (`ID`, `Destination`, `CrzTeleport`, `StopDistanceTolerance`, flags/elapsed/moveTime/fade/mode/transport/seat, counts bit-packed, face payload, puntos y packed deltas) y corrige `Done = 0x20`/stop sin flags de move.
      - [x] **#A06.8h.2** Reactivar el envío lineal representado desde `tick_creatures_sync` usando el builder C++-like ya testeado: cuando una criatura inicia wander representado, Rust emite `SMSG_ON_MONSTER_MOVE` con `MovementMonsterSpline` lineal, spline id incrementado, duración C++-like mínima 500ms y punto destino único.
      - [ ] **#A06.8h.3** Implementar `MoveSpline`/`MotionMaster` real: CatmullRom, cyclic, parabolic/fall, spline filters, spell visual extra data, transport motion, pathgen y lifecycle completo.
        - [x] **#A06.8h.3a** Crear `crates/wow-movement` y portar el núcleo contrastado de `MoveSplineFlag`, `MonsterMoveType`, `FacingInfo`, `SpellEffectExtraData`, `MoveSplineInitArgs::Validate`, `computeFallTime`, `computeFallElevation`, almacenamiento CatmullRom-compatible, duración por segmentos, `MoveSpline::Initialize`, `ComputePosition`, parabolic/fall elevation, `updateState`, cyclic wrap y `Finalize` contra `MoveSplineFlag.h`, `MovementTypedefs.h`, `MovementUtil.cpp`, `MoveSplineInitArgs.h`, `Spline.cpp` y `MoveSpline.cpp`.
        - [ ] **#A06.8h.3b** Completar el núcleo `MoveSpline` pendiente: rewrite de `Enter_Cycle` preservando duración C++, `AnimTierTransition`, curvas DB2 de `SpellEffectExtraData`, Bezier si resulta necesario por callers, y fixtures adicionales contra C++.
          - [x] **#A06.8h.3b.1** Portar `Enter_Cycle` de `MoveSpline::_updateState`: al completar el primer ciclo, Rust elimina el primer vértice de la ruta y reconstruye el spline preservando la duración anterior como C++.
          - [x] **#A06.8h.3b.2** Portar `AnimTierTransition` en `MoveSplineInitArgs`/`MoveSpline` y conservar `effect_start_time` para el futuro mapper de `MovementSpline::AnimTierTransition`.
          - [x] **#A06.8h.3b.3** Portar el helper de `SplineImpl.h::computeIndex/evaluate_percent` como `compute_position_percent`, con reglas C++ para `t=1.0`.
          - [ ] **#A06.8h.3b.4** Conectar curvas DB2 de `SpellEffectExtraData::{ProgressCurveId,ParabolicCurveId}` y ampliar fixtures contra C++ cuando exista store/mapper real.
        - [ ] **#A06.8h.3c** Conectar el `MoveSpline` real a `Unit`/`MoveSplineState` y mapper `MoveSpline -> MovementMonsterSpline`, incluyendo extras packet (`SplineFilter`, `SpellEffectExtraData`, `JumpExtraData`, `AnimTierTransition`) y `SMSG_FLIGHT_SPLINE_SYNC` cíclico.
          - [x] **#A06.8h.3c.1** Portar el mapper packet-side contrastado con `MovementPackets.cpp::MonsterMove::InitializeSplineData`: Rust expone path data C++-like desde `MoveSpline`, serializa opcionales `SplineFilter`/`SpellEffectExtraData`/`JumpExtraData`/`AnimTierTransition` en el orden C++ y construye `MovementMonsterSpline` con flags, face, duración, fade, puntos y packed deltas.
          - [x] **#A06.8h.3c.2a** Conectar el wander representado de criaturas a `wow_movement::MoveSpline`: `WorldCreature` conserva el spline activo, avanza posición con `MoveSpline::update_state/compute_position`, sincroniza `MoveSplineState` representado y emite `SMSG_ON_MONSTER_MOVE` mediante el mapper real.
          - [ ] **#A06.8h.3c.2** Sustituir el `MoveSplineState` representado de `Unit`/criaturas por `wow_movement::MoveSpline` real y usar el mapper anterior en broadcasts.
          - [ ] **#A06.8h.3c.3** Portar `SMSG_FLIGHT_SPLINE_SYNC` y el sync cíclico asociado.
        - [ ] **#A06.8h.3d** Implementar `MoveSplineInit::Launch/Stop`: current-position chaining, sustitución de `path[0]`, selección/clamp de velocidad, flags de movimiento, transport-local transform, `MonsterMove` broadcast y stop con tolerancia 2.
          - [x] **#A06.8h.3d.1** Portar el núcleo reutilizable de `MoveSplineInit::Launch/Stop` en `wow-movement`: builder inicial, `MoveTo/MovebyPath`, corrección de `path[0]` con posición real o spline activo, `initialOrientation`, `enter_cycle`, flags forward/backward/root, selección/clamp de velocidad, `MoveSpline::Initialize`, resultado de duración y stop `Done` con tolerancia 2. El wander representado ya usa este builder.
          - [x] **#A06.8h.3d.2** Conectar `MoveSplineInit::Stop` al runtime representado de criaturas: una criatura que muere/interrumpe movimiento calcula la posición actual del spline, reinicializa `MoveSpline` con `Done`, limpia target/progreso y emite `SMSG_ON_MONSTER_MOVE` stop con `StopDistanceTolerance=2`.
          - [x] **#A06.8h.3d.3** Completar los setters portables de `MoveSplineInit` contrastados contra `MoveSplineInit.h/.cpp`: `SetFirstPointId`, `SetTransportEnter/Exit`, `SetParabolic`, `SetParabolicVerticalAcceleration`, `SetAnimation`, `SetFacing` spot/angle/target-con-angulo, `SetSpellEffectExtraData` y `DisableTransportPathTransformations`. Quedan explícitos para el runtime `Unit` real el constructor con contexto de transporte, el cálculo de `SetFacing(Unit const*)` desde posición de Unit/target y la transformación global/local de transporte.
        - [ ] **#A06.8h.3e** Implementar `MotionMaster` real y primeros generators (`Generic`, `Point`, `MoveJump`, `MoveCharge`, `MoveFall`, `MoveCirclePath`) antes de pathgen completo.
          - [x] **#A06.8h.3e.1** Alinear el primer puente representado de `PointMovementGenerator`: `MotionSubsystem::move_point` registra `POINT_MOTION_TYPE` con `UNIT_STATE_ROAMING` como C++, y el lanzamiento/finalizacion/interrupcion del spline de criatura marca/limpia `UNIT_STATE_ROAMING_MOVE` durante el movimiento activo. Quedan pendientes `Initialize/Update/Finalize` completos, pathgen, `MovementInform` y generadores runtime reales.
          - [x] **#A06.8h.3e.2** Portar la matematica portable de `MotionMaster::MoveJump`/`CalculateJumpSpeeds`: helpers `compute_jump_max_height_like_cpp` y `calculate_jump_speeds_like_cpp` contrastados con `MotionMaster.cpp` y `MovementUtil.cpp`, sin acoplar todavia a `UnitMoveType`/velocidades reales del Unit.
          - [x] **#A06.8h.3e.3** Portar el estado/lifecycle representado de `GenericMovementGenerator`: constructor estructural con `MOTION_MODE_DEFAULT`, `MOTION_PRIORITY_NORMAL`, `MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING`, `UNIT_STATE_ROAMING`, duracion, arrival spell opcional, reglas de `Initialize` para reactivacion no soportada, `Update` por duracion/finalized/cyclic, `Deactivate` y `Finalize` con inform. Queda pendiente ejecutar la lambda real contra `MoveSplineInit(Unit*)`, `CastSpell` y `CreatureAI::MovementInform`.
          - [x] **#A06.8h.3e.4** Portar los wrappers estructurales de `MotionMaster::LaunchMoveSpline`, `MoveJump` y `MoveJumpWithGravity`: validacion `IsInvalidMovementGeneratorType`, `GenericMovementGenerator` de tipo `EFFECT_MOTION_TYPE`, prioridad `MOTION_PRIORITY_HIGHEST` para saltos, `UNIT_STATE_JUMPING`, arrival spell metadata y `MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH` solo en `MoveJumpWithGravity`. Quedan pendientes el `MoveSplineInit` ejecutable, destino real, velocidades de `UnitMoveType`, spell effect extra data, `MoveJumpTo`, charge/knockback/fall y broadcast desde `Unit`.
          - [x] **#A06.8h.3e.5** Portar los wrappers representados de `MotionMaster::MoveKnockbackFrom` y `MoveFall`: guard de player/speed para knockback, `EFFECT_MOTION_TYPE`, prioridad `MOTION_PRIORITY_HIGHEST`, `MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH` en knockback, guards de altura/root-stun/player para fall y spline genérico solo para no-player. Quedan pendientes raycast/path real, `SetParabolic`, `SetOrientationFixed`, `SetFall`, `SetFallInformation`, hover offset y `Unit::SetFall`.
          - [x] **#A06.8h.3e.6** Portar el lifecycle representado de `PointMovementGenerator`: constructor `INITIALIZATION_PENDING`, `Initialize` con `EVENT_CHARGE_PREPATH`, guard `UNIT_STATE_NOT_MOVE`/casting representado, `Update` por spline finalized/interrupted/speed-update, `Deactivate`, `Finalize` con limpieza de `UNIT_STATE_ROAMING_MOVE` e inform `EVENT_CHARGE_PREPATH -> EVENT_CHARGE`. Quedan pendientes destino/path real, `MoveSplineInit`, `SignalFormationMovement`, `CreatureAI::MovementInform` real y `AssistanceMovementGenerator`.
          - [x] **#A06.8h.3e.7** Conectar el caso directo sin pathgen de `PointMovementGenerator` al runtime representado de `WorldCreature`: `begin_point_movement_like_cpp` crea el generator, ejecuta `Initialize`, marca `UNIT_STATE_ROAMING_MOVE` y lanza el `MoveSplineInit` real existente para destino directo; `EVENT_CHARGE_PREPATH` solo marca roaming move como C++ y no lanza spline desde el generator; finalize limpia `ROAMING_MOVE` y registra `MovementInform(POINT,id)` en estado AI canonico. Quedan pendientes pathgen, close-enough, final orientation/facing, spell effect extra data, formation signal, dispatch real a SmartAI/scripts y MotionMaster update generico.
          - [x] **#A06.8h.3e.8** Portar la forma representada de `MotionMaster::MoveSeekAssistance`, `AssistanceMovementGenerator::Finalize` y `AssistanceDistractMovementGenerator`: `EVENT_ASSIST_MOVE=1009`, delay C++ `CreatureFamilyAssistanceDelay=1500`, prioridad/base-state/flags de los generators y plan de efectos `AttackStop`, `CastStop`, `DoNotReacquireSpellFocusTarget`, `REACT_PASSIVE`, `SetNoCallAssistance(false)`, `CallAssistance()`, distract y `REACT_AGGRESSIVE`. Quedan pendientes `CallAssistance` real, radio/config completos, AI/map visit real y ejecucion de `AssistanceDistract` sobre un `Unit`.
          - [x] **#A06.8h.3e.9** Portar la forma representada de `IdleMovementGenerator`, `RotateMovementGenerator` y `DistractMovementGenerator`: default idle con prioridad normal e initialized, `StopMoving` en init/reset idle/rotate, rotate con `UNIT_STATE_ROTATING`, direccion left/right, clamp de orientacion y regla de inform sin flag en finalize, distract con `UNIT_STATE_DISTRACTED`, prioridad highest, stand-up, timer `diff > timer` y retorno a orientacion de home. Quedan pendientes ejecutar `MoveSplineInit` real de rotate/distract, `SetFacingTo(home)` real y `CreatureAI::MovementInform` real.
          - [x] **#A06.8h.3e.10** Conectar el puente runtime representado de `DistractMovementGenerator`/`RotateMovementGenerator` para criaturas: `WorldCreature` crea el generator, ejecuta init/update representado y lanza un `MoveSplineInit` real de facing-only (`MoveTo(pos actual) + SetFacing(angle)`) como C++; `Distract::Initialize` muta `UnitData::StandState` real a stand cuando corresponde, `Distract::Finalize` aplica la orientacion home y `Rotate::Finalize` registra el payload `MovementInform(ROTATE,id)` en el estado AI canonico de la criatura. Quedan pendientes dispatch real a SmartAI/scripts y MotionMaster update generico.
          - [x] **#A06.8h.3e.11** Portar la superficie representada de `MotionMasterFlags`, `MotionMasterDelayedActionType` y `DelayedAction`: flags exactos `NONE/UPDATE/STATIC_INITIALIZATION_PENDING/INITIALIZATION_PENDING/INITIALIZING`, `MOTIONMASTER_FLAG_DELAYED = UPDATE | INITIALIZATION_PENDING`, enum `CLEAR..INITIALIZE` con IDs C++ 0..7, payloads representados `Add/Remove/Clear/Initialize`, cola FIFO y validator. Quedan pendientes el `MotionMaster::Update` generico y callbacks reales de finalize para todos los generator types.
          - [x] **#A06.8h.3e.12** Despachar el `GenericMovementInform` ya representado hacia `CreatureAI::MovementInform` canonico para criaturas, contrastado con `GenericMovementGenerator.cpp::Finalize`; no aplica a `Idle`, `Distract` ni `AssistanceDistract`. Queda pendiente el `CastSpell` real de arrival spell y el runtime generico de `MotionMaster`.
          - [x] **#A06.8h.3e.13** Portar el driver representado de `MotionMaster::Update`: respeta stall por `INITIALIZATION_PENDING|INITIALIZING`, set/clear de `UPDATE`, inicializacion/reset del top, update de `Idle/Generic/Point/Rotate/Distract`, pop natural si termina y drenado final de delayed actions. Queda pendiente mover esto a un `MotionMaster` runtime real con owner `Unit` y callbacks completos de finalize.
          - [x] **#A06.8h.3e.14** Portar `MovementDefines::{ChaseRange, ChaseAngle}` a `wow-movement`: constructores con `CONTACT_DISTANCE=0.5`, tolerancia `pi/4`, normalizacion C++ de orientacion y `IsAngleOkay` con wrap en `2*pi`. Prepara `FollowMovementGenerator`/`ChaseMovementGenerator`.
          - [x] **#A06.8h.3e.15** Portar `MovementDefines::{JumpArrivalCastArgs, JumpChargeParams}` a `wow-movement`: arrival spell/target, union `Speed/MoveTimeInSec` como enum tipado, `TreatSpeedAsMoveTimeSeconds`, `JumpGravity` y opcionales `SpellVisualId/ProgressCurveId/ParabolicCurveId`.
          - [x] **#A06.8h.3e.16** Crear el skeleton runtime de `wow-movement` para `MovementGenerator`/`MotionMaster`: enum `MovementGeneratorType`, slot/mode/priority/flags C++ exactos, trait `MovementGenerator`, `MotionMasterFlags`, tipos de delayed action y cola FIFO con closures/validator. Quedan pendientes concrete generators y owner `Unit`.
          - [x] **#A06.8h.3e.17** Portar `IdleMovementGenerator` runtime a `wow-movement`: constructor default/normal/initialized, init/reset como `StopMoving` representado, update siempre true, deactivate noop y finalize marca `FINALIZED`.
          - [x] **#A06.8h.3e.18** Portar `RotateMovementGenerator` runtime a `wow-movement`: `RotateDirection` con IDs C++ exactos, constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_ROTATING`, init/reset/deactivate/finalize equivalentes, cálculo de facing por `diff * 2π / maxDuration` con clamp C++, duración e `INFORM_ENABLED`, y salida representada para spline facing/transporte hasta conectar owner `Unit` real.
          - [x] **#A06.8h.3e.19** Portar `DistractMovementGenerator` y `AssistanceDistractMovementGenerator` runtime a `wow-movement`: prioridad highest/normal, `UNIT_STATE_DISTRACTED`, init/reset con stand-up y facing spline, regla C++ estricta `diff > timer`, finalize de distract hacia orientación home y finalize de assistance hacia `REACT_AGGRESSIVE` representados hasta conectar owner `Unit` real.
          - [x] **#A06.8h.3e.20** Añadir `MotionMaster` runtime inicial en `wow-movement`: default + activos ordenados por prioridad C++, deferral bajo `UPDATE/INITIALIZATION_PENDING`, `add/remove/clear` por slot/mode/priority/type, update del top con init/reset/pop natural y refs representadas de `BaseUnitState`. Sigue pendiente owner `Unit`, factory default por criatura/player, callbacks reales y conexión al tick de mapa.
          - [x] **#A06.8h.3e.21** Portar `GenericMovementGenerator` runtime a `wow-movement`: inicializador `FnOnce(&mut MoveSplineInit)`, constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_ROAMING`, init con launch real sobre `MoveSpline`, no-resume tras deactivation, update por duración salvo cyclic/finalized, finalize con arrival spell e inform representados. `CastSpell` y `CreatureAI::MovementInform` reales siguen bajo Spell/AI/Unit runtime.
          - [x] **#A06.8h.3e.22** Portar la forma runtime directa de `PointMovementGenerator` y `AssistanceMovementGenerator` a `wow-movement`: constructor/flags/base-state, `EVENT_CHARGE_PREPATH`, bloqueo por no-move/casting, `UNIT_STATE_ROAMING_MOVE`, direct `MoveSplineInit` con speed/facing/final orient/spell extras/close-enough, speed-update/interrupted relaunch action, finalize `POINT` inform y plan de asistencia. PathGenerator real, owner Unit y dispatch AI real siguen abiertos.
          - [x] **#A06.8h.3e.23** Portar `AbstractFollower` y la forma runtime representada de `FollowMovementGenerator` a `wow-movement`: target add/remove events, constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_FOLLOW`, init/reset con `StopMoving`/pet-speed/inform, duration/check timer, `PositionOkay`, selección de ángulo, bloqueo por no-move/casting, `UNIT_STATE_FOLLOW_MOVE`, inform por target counter, deactivate/finalize y speed-change. PathGenerator/GetNearPoint/owner Unit reales siguen abiertos.
          - [x] **#A06.8h.3e.24** Portar la forma runtime representada de `ChaseMovementGenerator` a `wow-movement`: constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_CHASE`, range-check timer, `PositionOkay` con min/max/angle/LOS, mutual chase, lost target/no-move/casting, `UNIT_STATE_CHASE_MOVE`, cannot-reach plan, walk-mode, inform por target counter, deactivate/finalize y speed-change. PathGenerator/GetNearPoint/LOS real/owner Unit siguen abiertos.
          - [x] **#A06.8h.3e.25** Portar la forma runtime representada de `FleeingMovementGenerator` y `TimedFleeingMovementGenerator` a `wow-movement`: constructor default/highest/`INITIALIZATION_PENDING`/`UNIT_STATE_FLEEING`, flag `UNIT_FLAG_FLEEING`, quiet distances 28..43, retries LOS/path, path length limit 30, `UNIT_STATE_FLEEING_MOVE`, random delay 800..1500, speed-update relaunch, finalize especializado Player/Creature y timed flee con `MovementInform(TIMED_FLEEING,0)`. PathGenerator/ObjectAccessor/MovePositionToFirstCollision/owner Unit reales siguen abiertos.
          - [x] **#A06.8h.3e.26** Portar la forma runtime representada de `ConfusedMovementGenerator` a `wow-movement`: constructor default/highest/`INITIALIZATION_PENDING`/`UNIT_STATE_CONFUSED`, flag `UNIT_FLAG_CONFUSED`, referencia inicial, `StopMoving` en initialize/reset, hop aleatorio `4*frand-2` con angulo `2*pi`, retries LOS/path, path length limit 30, `SetWalk(true)`, delay 800..1500, `UNIT_STATE_CONFUSED_MOVE`, speed-update relaunch y finalize especializado Player/Creature. PathGenerator/MovePositionToFirstCollision/owner Unit reales siguen abiertos.
          - [x] **#A06.8h.3e.27** Portar la forma runtime representada de `HomeMovementGenerator<Creature>` a `wow-movement`: constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_ROAMING`, `SetNoSearchAssistance(false)`, interrupcion por root/stunned/distracted, limpieza `UNIT_STATE_ALL_ERASABLE & ~UNIT_STATE_EVADE`, launch home con facing home y run, update por spline finalized/interrupted, deactivate/finalize limpiando roaming/evade e inform `JustReachedHome` representado. Owner Unit, `UpdateAllowedPositionZ`, VehicleKit y AI reales siguen abiertos.
          - [x] **#A06.8h.3e.28** Portar la forma runtime representada de `RandomMovementGenerator<Creature>` a `wow-movement`: constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_ROAMING`, pausa/resume, duracion opcional, referencia inicial, wander distance desde owner si es 0, pasos 2..10, destino `frand(0,wanderDistance)` + angulo `2*pi`, retries LOS/path sin bloquear `FARFROMPOLY`, `SetWalk` segun `CreatureRandomMovementType`, pausas 4..10s, `SignalFormationMovement`, finalize con `MovementInform(RANDOM,0)`. PathGenerator/MovePositionToFirstCollision/owner Unit reales siguen abiertos.
          - [x] **#A06.8h.3e.29** Portar la forma runtime representada de `PathMovementBase` y `WaypointMovementGenerator<Creature>` a `wow-movement`: path/nodo actual, constructores DB/path, pausa/resume, reset position, init con delay 1000 sin marcar `INITIALIZED`, timer de espera, `OnArrived`, `MovementInform(WAYPOINT,id)`, `WaypointReached`, `WaypointStarted`, final de ruta, modo repetir/ida-vuelta, wait/random al final de path, speed/move-type/facing/generatePath/transport transform y finalize. WaypointManager SQL, PathGenerator, owner Unit, MotionMaster real y AI dispatch real siguen abiertos.
          - [x] **#A06.8h.3e.30** Portar la forma runtime representada de `FormationMovementGenerator` a `wow-movement`: constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_FOLLOW_FORMATION`, `AbstractFollower`, intervalo 1200ms, prediccion 1.65s del spline del lider, flip de angulo por waypoints configurados, parada cuando el lider termina spline predicho, `UNIT_STATE_FOLLOW_FORMATION_MOVE`, facing/inform al llegar y finalize/deactivate. `CreatureGroup`/lider real, owner `Unit`, `MoveSplineInit` real y loader `creature_formations` siguen abiertos.
          - [x] **#A06.8h.3e.31** Portar la forma runtime representada de `FlightPathMovementGenerator` a `wow-movement`: constructor default/highest/`INITIALIZATION_PENDING`/`UNIT_STATE_IN_FLIGHT`, reset con `CombatStopWithPets`, flags taxi/control, launch fly/smooth/uncompressed/walk a 32.0, `GetPathAtMapEnd`, path shortening por distancia 40y/map/teleport/stop-delay, costes con discount, cambio de taxi destination, eventos departure/arrival, preload end grid, teleport-node resume/skip y finalize con clear taxi/dismount/flags/benchmark/teleport final. `PlayerTaxi`, DB2 `TaxiPath*`, owner `Player`, `MoveSplineInit` real, `SMSG_FLIGHT_SPLINE_SYNC` y handlers taxi reales siguen abiertos.
          - [x] **#A06.8h.3e.32** Portar la forma runtime representada de `SplineChainMovementGenerator` a `wow-movement`: constructor default/normal/`INITIALIZATION_PENDING`/`UNIT_STATE_ROAMING`, `SplineChainLink`, `SplineChainResumeInfo`, resume parcial desde `(SplineIndex, PointIndex, TimeToNext)`, clamp de punto invalido, `MovebyPath` vs `MoveTo`, velocidad opcional, walk mode, ajuste de `_msToNext` si `Launch()` devuelve duracion distinta, update por temporizador/final spline, `GetResumeInfo`, deactivate/finalize limpiando roaming move e inform `MovementInform(SPLINE_CHAIN,id)`. Loader SQL `script_spline_chain_*`, owner `Unit`, `MoveSplineInit` real, MotionMaster real y scripts reales siguen abiertos.
          - [x] **#A06.8h.3e.33** Portar los hooks runtime de `MotionMaster::PropagateSpeedChange` y `StopOnDeath` a `wow-movement`: `PropagateSpeedChange` notifica solo al current generator como C++, `StopOnDeath` respeta `MOVEMENTGENERATOR_FLAG_PERSIST_ON_DEATH`, limpia y empuja idle solo si el owner esta en mundo, y siempre solicita `StopMoving` salvo persist. Owner `Unit`, factory idle real y llamada desde muerte/velocidad reales siguen abiertos.
        - [ ] **#A06.8h.3f** Implementar pathgen/Detour o política de fallback explícita contrastada con `PathGenerator.cpp`; no cerrar `#A06.8h.3` mientras `wow-recastdetour` siga vacío.
          - [x] **#A06.8h.3f.1** Portar la primera slice portable de `PathGenerator`: constantes/flags C++ exactos, estado base, setters `SetUseStraightPath`/`SetPathLengthLimit`/`SetUseRaycast`, fallback sin navmesh `BuildShortcut` + `PATHFIND_NORMAL|PATHFIND_NOT_USING_PATH`, helpers `Dist3DSqr`/`InRange`/`InRangeYZX`, `AddFarFromPolyFlags` y forma de `ShortenPathUntilDist` con LOS representado por callback. Detour FFI, MMapManager, filtros, `BuildPolyPath`, `BuildPointPath`, `FindSmoothPath`, `NormalizePath`, `IsInvalidDestinationZ` y owner `WorldObject/Map` siguen abiertos.
          - [x] **#A06.8h.3f.2** Portar la base portable real de `MMapDefines.h` a `wow-recastdetour`: `MMAP_MAGIC=0x4d4d4150`, `MMAP_VERSION=15`, layout de `MmapTileHeader` de 20 bytes, parser/validador de header, `NavArea`, `NavTerrainFlag` reales (`GROUND=1`, `GROUND_STEEP=2`, `WATER=4`, `MAGMA_SLIME=8`) y helpers `packTileID`/nombres de fichero C++. Detour `DT_NAVMESH_VERSION`, blob ownership, `.mmap` map header, `dtNavMesh::addTile` y MMapManager real siguen abiertos.
          - [x] **#A06.8h.3f.3** Portar constantes Detour portables y layout de `.mmap`: `DT_POLYREF64`, salt/tile/poly bits, `DT_NAVMESH_MAGIC/VERSION=7`, state magic/version, tile/free flags, `dtNavMeshParams` de 28 bytes (`orig[3]`, `tileWidth`, `tileHeight`, `maxTiles`, `maxPolys`) y parser/roundtrip de `loadMapData` previo a `dtNavMesh::init`. FFI real de `dtAllocNavMesh`, `dtNavMesh::init`, `dtNavMeshQuery` y loader operativo siguen abiertos.
          - [x] **#A06.8h.3f.4** Portar esqueleto pre-FFI de `MMapManager`: `loadedMMaps` con entradas vacías para `InitializeThreadUnsafe` y fallos de apertura como C++, `parentMapData`, contadores C++-like, `loadMapData` leyendo `mmaps/{:04}.mmap` a `dtNavMeshParams` y `unloadMap(mapId)` dejando placeholder vacío. Esta base fue extendida después por los pasos Detour reales; `loadMap(mapId,x,y)` sigue abierto.
          - [x] **#A06.8h.3f.5** Portar lector pre-FFI de `.mmtile`: parsea `MmapTileHeader`, valida `MMAP_MAGIC`, `MMAP_VERSION`, `DT_NAVMESH_VERSION`, comprueba `header.size <= bytes restantes` como `ftell/fseek` C++ y devuelve el blob exacto que luego recibirá `dtNavMesh::addTile`. `dtAlloc`, ownership `DT_TILE_FREE_DATA`, `dtMeshHeader` y `tileRef` siguen bloqueados por FFI.
          - [x] **#A06.8h.3f.6** Vendorizar y compilar Detour C++ real en `wow-recastdetour`: copia exacta del legacy `dep/recastnavigation/Detour/{Include,Source}` (sin `CMakeLists.txt`) dentro de `vendor/Detour` y `build.rs` con `cc` compila `DetourAlloc`, `DetourAssert`, `DetourCommon`, `DetourNavMesh`, `DetourNavMeshBuilder`, `DetourNavMeshQuery` y `DetourNode`. Bindings/wrappers Rust siguen abiertos.
          - [x] **#A06.8h.3f.7** Abrir el primer binding real Detour: C ABI estrecho para `dtAllocNavMesh`, `dtFreeNavMesh`, `dtNavMesh::init` y `dtNavMesh::getMaxTiles`, `DetourNavMeshParams` marcado `repr(C)`, wrapper Rust `DetourNavMesh` con `Drop` y test smoke contra C++ vendorizado. `addTile/removeTile`, `dtNavMeshQuery` y `dtQueryFilter` siguen abiertos.
          - [x] **#A06.8h.3f.8** Extender binding `dtNavMesh` con `addTile`/`removeTile`: C ABI copia el blob a memoria `dtAlloc`, llama `addTile(..., DT_TILE_FREE_DATA, 0, &tileRef)` y libera en fallo, wrapper Rust expone `add_tile`/`remove_tile` con `DetourTileRef=u64` y testea ramas de error reales. Falta fixture/generacion de tile valida para test de exito y conectar `MMapManager::loadMap`.
          - [x] **#A06.8h.3f.9** Cubrir el éxito real de `dtNavMesh::addTile/removeTile`: helper C ABI de test genera una tile mínima mediante `dtCreateNavMeshData` del Detour vendorizado, Rust la envuelve como `MmapTileBlob`, la inserta en `dtNavMesh` y la retira verificando `tileRef` no nulo. Siguen pendientes `MMapManager::loadMap` con `.mmtile` reales, `dtNavMeshQuery`, `dtQueryFilter` y pathgen Detour operativo.
          - [x] **#A06.8h.3f.10** Abrir binding real de `dtNavMeshQuery` para `MMapManager::loadMapInstance`: C ABI `dtAllocNavMeshQuery`/`dtFreeNavMeshQuery`/`query->init(navMesh, 1024)`, wrapper Rust `DetourNavMeshQuery<'mesh>` con `Drop`, `!Send + !Sync` y lifetime ligado al `DetourNavMesh`. Siguen pendientes métodos de query (`findPath`, `findNearestPoly`, `moveAlongSurface`, etc.), `dtQueryFilter` y el mapa per-instance dentro de `MMapManager`.
          - [x] **#A06.8h.3f.11** Abrir binding real de `dtQueryFilter`: wrapper Rust con `Drop`, `!Send + !Sync`, defaults C++ (`include=0xffff`, `exclude=0`, costes de 64 areas a `1.0`) y mutadores `setIncludeFlags`/`setExcludeFlags`/`setAreaCost` contrastados contra `DetourNavMeshQuery.{h,cpp}`. Siguen pendientes `PathGenerator::CreateFilter/UpdateFilter`, forced map flags, costes por terreno y uso del filtro en métodos query.
          - [x] **#A06.8h.3f.12** Abrir primer método real de `dtNavMeshQuery`: C ABI y wrapper Rust para `findNearestPoly(center, halfExtents, filter)` con `DetourPolyRef=u64` y punto cercano, contrastado contra `PathGenerator::GetPolyByLocation` (`extents={3,5,3}` / fallback `{3,50,3}`) y probado sobre tile válida generada por Detour. Siguen pendientes `findPath`, `findStraightPath`, `closestPointOnPoly`, `moveAlongSurface`, `getPolyHeight`, `raycast` y la integración de `GetPolyByLocation` completa.
          - [x] **#A06.8h.3f.13** Abrir `dtNavMeshQuery::findPath`: C ABI y wrapper Rust devuelven `Vec<DetourPolyRef>` con buffer `maxPath` validado contra `int`, contrastado contra `PathGenerator::BuildPolyPath` y probado en el caso C++ `startRef == endRef` que retorna un corridor de un solo polígono; también cubre fallo `maxPath=0`. Siguen pendientes path multi-polígono, `findStraightPath`, smooth path, raycast y normalización de Z.
          - [x] **#A06.8h.3f.14** Abrir `dtNavMeshQuery::findStraightPath`: C ABI y wrapper Rust devuelven puntos, flags y refs (`DT_STRAIGHTPATH_START/END/OFFMESH`, opciones area/all crossings), contrastado contra `BuildPointPath` y `GetSteerTarget`; testea corridor de un polígono con dos puntos start/end y fallo `maxStraightPath=0`. Siguen pendientes `closestPointOnPoly`, `moveAlongSurface`, `getPolyHeight`, `raycast`, smooth path y conexión a `PathGenerator`.
          - [x] **#A06.8h.3f.15** Abrir `closestPointOnPoly` y `closestPointOnPolyBoundary`: C ABI y wrapper Rust devuelven punto cercano y `posOverPoly` cuando aplica, contrastados contra `GetPathPolyByPosition`, ajuste de end poly en `BuildPolyPath` y arranque de `FindSmoothPath`; tests cubren punto interior, clamp a boundary y errores de poly ref inválida. Siguen pendientes `moveAlongSurface`, `getPolyHeight`, `raycast`, smooth path y runtime loader.
          - [x] **#A06.8h.3f.16** Abrir `dtNavMeshQuery::getPolyHeight`: C ABI y wrapper Rust devuelven altura de superficie, contrastado contra `FindSmoothPath` y los branches de raycast que escriben `point[1]`; test cubre tile plana generada por Detour y fallo de poly ref inválida. Siguen pendientes `moveAlongSurface`, `raycast`, smooth path y loader runtime.
          - [x] **#A06.8h.3f.17** Abrir `dtNavMeshQuery::moveAlongSurface`: C ABI y wrapper Rust devuelven `resultPos` y refs visitadas, contrastado contra `FindSmoothPath` (`MAX_VISIT_POLY=16`, `FixupCorridor`, `getPolyHeight` posterior); test cubre desplazamiento dentro de una tile válida y fallo `maxVisitedSize=0`. Siguen pendientes `raycast`, smooth path completo, `FixupCorridor/GetSteerTarget` portados contra Detour y loader runtime.
          - [x] **#A06.8h.3f.18** Abrir `dtNavMeshQuery::raycast` legacy usado por `BuildPolyPath`: C ABI y wrapper Rust devuelven `hit`, normal y corridor visitado; test cubre raycast sobre tile válida generada por Detour y fallo por poly ref inválida, usando el comportamiento real del Detour vendorizado. Siguen pendientes ensamblar `BuildPolyPath`/`BuildPointPath` sobre wrappers, `FindSmoothPath` completo y loader runtime.
          - [x] **#A06.8h.3f.19** Portar la cache per-instance de `MMapManager`: `loadMapData` inicializa un `dtNavMesh` real, `MMapData` destruye queries antes que mesh, `loadMapInstance` crea/reutiliza una `dtNavMeshQuery` por `(instanceMapId, instanceId)`, `unloadMapInstance` y `GetNavMeshQuery` devuelven false/None para mapas o queries ausentes como C++. Siguen pendientes `loadMap(mapId,x,y)` con `.mmtile` reales y wiring de `PathGenerator` contra el manager.
          - [x] **#A06.8h.3f.20** Portar `MMapManager::loadMap(mapId,x,y)` y unload de tile: lee `.mmtile`, aplica fallback a parent map al no abrir fichero como C++, valida header/blob, llama `dtNavMesh::addTile`, guarda `loadedTileRefs[packTileID(x,y)]`, incrementa/decrementa contador global y rechaza duplicados/missing como C++. Siguen pendientes fixture con datos extraidos reales, llamada desde lifecycle de grids/mapas y ensamblar `PathGenerator` contra manager/query/filtro.
          - [x] **#A06.8h.3f.21** Portar reglas de `PathGenerator::CreateFilter/UpdateFilter`: helper Rust crea/actualiza `dtQueryFilter` con include flags para creature/player, force enabled/disabled map flags, terrain actual al estar en/under water y `NAV_GROUND_STEEP` en combate/evade como C++. Sigue pendiente alimentar ese contexto desde `WorldObject/Map` reales y usar el filtro en `BuildPolyPath`/`FindSmoothPath`.
          - [x] **#A06.8h.3f.22** Abrir helper Detour para `BuildPolyPath/BuildPointPath` recto: `GetPolyByLocation` con extents `{3,5,3}` y `{3,50,3}`, missing-poly shortcut/NOPATH, far-from-poly flags/clamp, same-poly y fresh `findPath` + `findStraightPath` con force-destination/short/nopath branches. Siguen pendientes previous-path reuse, raycast branch, `FindSmoothPath` y wiring a `wow-movement::PathGenerator`.
          - [x] **#A06.8h.3f.23** Portar rama raycast de `BuildPolyPath`: usa `dtNavMeshQuery::raycast`, trata path vacío/fallo como `SHORTCUT|NOPATH`, propaga flags far-from-poly, calcula hit `t*0.99`, ajusta altura con `getPolyHeight` o clamp boundary y emite path de dos puntos `INCOMPLETE`/`NORMAL` como C++. Siguen pendientes previous-path reuse/suffix y `FindSmoothPath`.
          - [x] **#A06.8h.3f.24** Portar helpers de smooth path `FixupCorridor` y `GetSteerTarget`: splice del corredor con visited en orden C++, búsqueda de steer target con máximo 3 puntos, slop `0.3`, offmesh break y conservación de Z/elevación inicial. Sigue pendiente el bucle completo de `FindSmoothPath` y offmesh endpoint handling.
          - [x] **#A06.8h.3f.25** Portar el bucle normal de `PathGenerator::FindSmoothPath`: inicializa `iterPos/targetPos` con `closestPointOnPolyBoundary` si hay varios polys, usa `GetSteerTarget`, avanza con `moveAlongSurface`, repara corredor con `FixupCorridor`, ajusta altura con `getPolyHeight + 0.5`, hace snap al final y falla si alcanza `MAX_POINT_PATH_LENGTH` como C++.
          - [x] **#A06.8h.3f.26** Portar el branch offmesh de `FindSmoothPath`: binding de `dtNavMesh::getOffMeshConnectionPolyEndPoints`, avance del corredor hasta `steerPosRef`, push de `connectionStartPos`, salto a `connectionEndPos` y reajuste de altura como C++. Queda como gap de cobertura un fixture offmesh real/generado que ejercite el branch de éxito.
          - [x] **#A06.8h.3f.27** Portar el dispatch de `PathGenerator::BuildPointPath`: `_useRaycast` degrada a fallback/NOPATH como C++, `_useStraightPath` usa `findStraightPath`, la rama normal usa `FindSmoothPath`, y el post-procesado compartido conserva append same-poly, fallback short/nopath, limite de puntos y force-destination.
          - [x] **#A06.8h.3f.28** Portar la optimizacion de path previo en `BuildPolyPath`: busqueda de `startPoly/endPoly` en `_pathPolyRefs`, recorte de subpath cuando ambos siguen dentro, reconstruccion de sufijo desde el 80% del corredor, recuperacion ante offmesh/`closestPointOnPoly` fallido y fallback NOPATH si se usa raycast con path previo.
          - [x] **#A06.8h.3f.29** Portar los helpers owner-backed restantes de `PathGenerator`: `NormalizePath` queda representado con callback explicito a `UpdateAllowedPositionZ`, e `IsInvalidDestinationZ` replica la regla C++ `(target.z - actualEnd.z) > 5.0`; quedan para integracion runtime el `Map` real, hover/swim/fly y LOS/collision height.
          - [x] **#A06.8h.3f.30** Abrir orquestador Detour `calculate_detour_path_like_cpp`: centraliza el swap WoW `(x,y,z)` -> Detour `(y,z,x)` y vuelta, llama `BuildPolyPath`/raycast y `BuildPointPath` segun opciones C++ (`useStraightPath`, `useRaycast`, `forceDestination`, limite de puntos) y devuelve puntos en coordenadas WoW.
          - [x] **#A06.8h.3f.31** Portar la puerta `PathGenerator::HaveTile`: binding de `dtNavMesh::calcTileLoc` y `getTileAt`, guard de coordenadas negativas y helper `have_tile_for_wow_position_like_cpp` con swap WoW/Detour previo como C++.
          - [x] **#A06.8h.3f.32** Conectar el orquestador Detour a la cache per-instance de `MMapData`: `calculate_path_for_instance_like_cpp` exige query cargada para `(instanceMapId, instanceId)`, valida tiles de start/end como `CalculatePath` y calcula usando la query cacheada sin exponer punteros al runtime.
          - [x] **#A06.8h.3f.33** Añadir el puente portable para aplicar resultados Detour en `PathGenerator`: `apply_detour_path_like_cpp` limpia el estado previo, conserva start/end/actualEnd/forceDestination, copia `_pathPolyRefs` hasta `MAX_PATH_LENGTH` y deja `_pathPoints`/`_type` listos para el runtime como haria `CalculatePath` despues de `BuildPolyPath`/`BuildPointPath`. Sigue pendiente invocarlo desde el owner real (`WorldObject/Map/MoveSplineInit`).
          - [x] **#A06.8h.3f.34** Abrir el puente runtime `MovebyPath` para criaturas: `WorldCreature::begin_move_spline_by_path_like_cpp` lanza `MoveSplineInit::move_by_path` con un path ya calculado, conserva el mismo estado `ROAMING_MOVE`/`move_target`/`spline_id` que el launcher directo y deja listo el punto donde `CalculatePath` runtime podra sustituir el `MoveTo` lineal.
          - [x] **#A06.8h.3f.35** Añadir frontera `wow-world` <-> `wow-recastdetour`: conversion bit-a-bit `DetourPathType -> PathType`, builder `path_generator_from_detour_like_cpp` y launcher `begin_move_spline_with_detour_path_like_cpp` que usa `MovebyPath` si el resultado no trae `NOPATH`, o cae a `MoveTo` directo como `MoveSplineInit::MoveTo(generatePath=true)` de C++. Sigue pendiente alimentar este helper con `MMapData` real desde el lifecycle de mapas/grids.
          - [x] **#A06.8h.3f.36** Cablear la configuracion C++ real de mmaps: `mmap.enablePathFinding` resuelve `CONFIG_ENABLE_MMAPS`, `DataDir` alimenta `DataDir/mmaps`, `WorldSession` conserva `MMapRuntimeConfigLikeCpp` y el plan corrige el falso `mmap.allowedMaps` (en este legacy el gate por mapa viene de `DisableMgr`/`DISABLE_TYPE_MMAP`). Sigue pendiente que el lifecycle de mapas/grids consuma esta configuracion para cargar tiles y queries reales.
          - [x] **#A06.8h.3f.37** Representar el gate C++ por mapa de `DisableMgr::IsPathfindingEnabled`: se cargan filas `disables.sourceType = DISABLE_TYPE_MMAP`, se validan contra `Map.db2` como C++ y `MMapRuntimeConfigLikeCpp::pathfinding_enabled_for_map_like_cpp(mapId)` aplica `CONFIG_ENABLE_MMAPS && !disabledMap`. Sigue pendiente que el `PathGenerator` runtime consulte este gate antes de solicitar `MMapData`.
          - [x] **#A06.8h.3f.38** Portar helper de coordenadas de tile mmap: `mmap_tile_coords_for_wow_position_like_cpp(x,y)` replica `int(CENTER_GRID_ID - coord / SIZE_OF_GRIDS)` de `TerrainInfo::GetGrid`, evitando mezclar la grid de criaturas de 64 yardas con los `.mmtile` de 533.333 yardas.
          - [x] **#A06.8h.3f.39** Abrir el puente owner-runtime prestado hacia `MMapData`: `calculate_creature_detour_path_like_cpp` recibe `WorldCreature`, destino, `Option<&MMapData>`, ids de instancia y `PathQueryFilterContext`; crea el filtro C++ y devuelve `None` cuando falta manager/query/tile para conservar el fallback `BuildShortcut` sin almacenar Detour `!Send + !Sync` en estado compartido.
          - [x] **#A06.8h.3f.40** Portar el helper de lifecycle minimo de mmap para owner dedicado: `MMapManager::load_pathfinding_context_for_wow_position_like_cpp` calcula la tile desde coordenadas WoW con la formula C++ de `TerrainInfo`, inicializa la query por `(meshMapId, instanceMapId, instanceId)` como `LoadMMapInstanceImpl` y carga el `.mmtile` como `LoadMMap`; los fallos normales de `.mmap`/`.mmtile` quedan representados como disponibilidad falsa para conservar el fallback C++ a shortcut. Sigue sin cablearse a `WorldSession` para no introducir Detour `!Send + !Sync` en futures Tokio compartidos.
          - [x] **#A06.8h.3f.41** Crear el owner local `WorldMMapPathfinderLikeCpp`: conserva un `MMapManager` Detour no compartido, prepara el contexto C++ de mapa/query/tile y llama al puente prestado `calculate_creature_detour_path_like_cpp`; si falta mapa/query/tile devuelve `None` para que el caller use shortcut. Aun falta mover este owner detour a una frontera runtime dedicada y alimentarlo desde el tick real.
          - [x] **#A06.8h.3f.42** Preparar la frontera runtime dedicada para Detour: `WorldMMapPathfinderWorkerLikeCpp` mantiene `WorldMMapPathfinderLikeCpp` dentro de un hilo owner y expone un handle por mensajes `WorldMMapPathRequestLikeCpp`, de modo que `WorldSession` pueda pedir rutas sin almacenar `MMapManager` `!Send + !Sync`. Aun falta conectar este handle al tick real y resolver inputs completos de filtro/terreno.
          - [x] **#A06.8h.3f.43** Cablear el worker mmap al runtime de sesiones: `world-server` arranca `WorldMMapPathfinderWorkerLikeCpp` cuando `CONFIG_ENABLE_MMAPS` esta activo, lo inyecta en cada `WorldSession`, y `tick_creatures_sync` pide ruta al worker antes de lanzar wander. Si el worker no devuelve path por falta de `.mmap`/`.mmtile`/query, conserva el fallback directo C++.
          - [x] **#A06.8h.3f.44** Corregir ids runtime del request mmap: el gate y la query usan `WorldCreature::map_id()` / `instance_id()` como C++ usa `_source->GetMapId()` / `_source->GetInstanceId()`, eliminando el `instance_id = 0` fijo del tick. Queda pendiente `PhasingHandler::GetTerrainMapId` real para cosmetic/child terrain maps.
- [ ] **#A07** Auditar **Combat** (`wow-combat` + handlers) vs `src/server/game/Combat/`. Damage roll, miss tables, hit info.
- [ ] **#A08** Auditar **Spells** (`wow-spell`) vs `src/server/game/Spells/`. Spell flow, casting, effects subset.
- [ ] **#A09** Auditar **Quests** (handlers/quest.rs, wow-data/quest) vs `src/server/game/Quests/`. Eligibility, kill credit, completion, reward.
- [ ] **#A10** Auditar **Inventory** (handlers/character.rs partes inventario) vs `src/server/game/Entities/Player/PlayerStorage.cpp`.
- [ ] **#A11** Auditar **Loot** (`wow-loot`) vs `src/server/game/Loot/LootMgr.cpp`. Drop chance, condition support.
- [ ] **#A12** Auditar **Chat** (`wow-chat`) vs `src/server/game/Chat/`. Mensaje broadcast, silenciamiento, anti-spam.
- [ ] **#A13** Auditar **Social** (`wow-social`) vs `src/server/game/Handlers/SocialHandler.cpp`.
- [ ] **#A14** Auditar **Group** vs `src/server/game/Groups/`.
- [ ] **#A15** Auditar **Trainer/Vendor/Gossip** (handlers) vs `src/server/game/Handlers/NPCHandler.cpp`.

### Refinamiento completo (Fase R)

- [ ] **#REFINE.001** Congelar features nuevas hasta refinar la siguiente mini-fase completa.
- [x] **#REFINE.010** Inventario árbol C++ `src/server` en `docs/migration/inventory/cpp-server-tree.md`.
- [x] **#REFINE.011** Inventario C++ por archivo y módulo en `docs/migration/inventory/cpp-files-by-module.md`.
- [x] **#REFINE.012** Inventario handlers/opcodes en `docs/migration/inventory/cpp-handlers-opcodes.md`.
- [x] **#REFINE.013** Inventario SQL/prepared statements en `docs/migration/inventory/cpp-sql-prepared.md`.
- [x] **#REFINE.014** Inventario DB2/DBC/hotfix stores en `docs/migration/inventory/cpp-dbc-db2-stores.md`.
- [x] **#REFINE.015** Inventario config world/bnet en `docs/migration/inventory/cpp-config-keys.md`.
- [x] **#REFINE.016** Inventario entity types en `docs/migration/inventory/cpp-entity-types.md`.
- [x] **#REFINE.017** Inventario `scripts/*` en `docs/migration/inventory/cpp-scripts-tree.md`.
- [x] **#REFINE.020** Cobertura canonica de ficheros C++ en cada doc de modulo.
- [x] **#REFINE.021** Rust target exacto por cada doc de modulo (`docs/migration/inventory/r2-rust-targets.md`).
- [x] **#REFINE.022** WBS granular por cada doc de modulo (`docs/migration/inventory/r2-task-wbs.md`).
- [x] **#REFINE.023** Divergencias/bugs conocidos con evidencia C++ (`docs/migration/inventory/r2-known-divergences.md`).
- [x] **#REFINE.024** Tests required por modulo (`docs/migration/inventory/r2-tests-required.md`).
- [x] **#REFINE.025** Sistemas post-WoLK/desactivados marcados sin omision silenciosa (`docs/migration/inventory/r2-product-scope.md`).
- [x] **#REFINE.030** Registros transversales de opcodes, SQL, update fields, managers, scripts y harness (`docs/migration/inventory/r3-cross-registry-summary.md`).
- [x] **#REFINE.040** DAG de dependencias y gates por fase (`docs/migration/inventory/r4-dependency-gate-summary.md`).
- [x] **#REFINE.050** Gap audit de archivos/opcodes/SQL/scripts (`docs/migration/inventory/r5-gap-audit.md`).
- [x] **#REFINE.060** Selección de la siguiente mini-fase lista para implementación (`docs/migration/inventory/r6-next-miniphase.md`).

### Inmediato (R6 — L0 config parity)

- [x] **#NEXT.L0.CONFIG.001** Ejecutar `docs/migration/inventory/r6-next-miniphase.md`: nombres canonicos `worldserver.conf`/`bnetserver.conf`, parsing semicolonado `*DatabaseInfo`, overlays `.conf.d`, override `TC_*`, y consumo de startup world/bnet contra C++. Cerrado en código, incluido `#NEXT.L0.CONFIG.REMOVE_LEGACY_DB_SUBKEYS`.
- [x] **#NEXT.L0.CONFIG.002** Portar `WorldBoolConfigs`/`WorldFloatConfigs`/`WorldIntConfigs`/`WorldInt64Configs` contra `World.cpp`.
  Estado: cerrado con `#NEXT.L0.CONFIG.002.a` registry/defaults, `#NEXT.L0.CONFIG.002.b` validaciones C++ y `#NEXT.L0.CONFIG.002.c` wiring runtime.

### Inmediato (R7 — L1 infra gate)

- [x] **#NEXT.L1.INFRA.001** Ejecutar `docs/migration/inventory/r7-l1-infra-miniphase.md`: database/prepared + DB2/hotfix gate contra C++.
          Estado: cerrado como gate de infraestructura; `#NEXT.L1.DB.PREP.CHARACTER` cubre los 523 statements C++; `#NEXT.L1.DB.PREP.HOTFIX` cubre los 745 statements C++ vía directos/generados; `#NEXT.L1.DB2.STORES.001_MAPS_WORLD`, `#NEXT.L1.DB2.STORES.002_ENTITIES_MOVEMENT`, `#NEXT.L1.DB2.STORES.003_ITEMS_COLLECTIONS`, `#NEXT.L1.DB2.STORES.004_PLAYER_SPELLS_PROGRESSION` y `#NEXT.L1.DB2.STORES.005_MISC_GENERATED` implementados como readers tipados. No quedan gaps exact-file contra los 325 `DB2Storage` de C++; quedan consumer wiring/hotfix overlays aguas abajo.

### Inmediato (R7 — L2 packets/dispatch gate)

- [x] **#NEXT.L2.DISPATCH.001** Ejecutar `docs/migration/inventory/r7-l2-packets-miniphase.md`: restaurar metadata C++ de dispatch para opcodes tocados (`PROCESS_THREADSAFE`, duplicados y variantes `TimeSyncResponse*`).
- [x] **#NEXT.L2.DISPATCH.002** Generar/auditar tabla completa de metadata de opcodes cliente desde `Opcodes.cpp`.
- [x] **#NEXT.L2.PACKET.WIRE.001** Dividir auditoría wire de parsers/serializers por ruta login-to-world.

### Inmediato (Fase 0 — Maps rewrite)

- [x] **#001** `wow-map`: módulo `coords.rs` con constantes y `compute_grid_coord` / `compute_cell_coord`. Tests vs `GridDefines.h`. Cerrado en `crates/wow-map/src/coords.rs` contra `GridDefines.h`.
- [x] **#002** `wow-map`: `MapKey { map_id: u32, instance_id: u32 }`, matching C++ `std::pair<uint32, uint32>`.
- [x] **#003** `wow-map`: `Cell` struct con containers tipados por GUID para world/grid objects; referencias reales quedan para NGrid/entities.
- [x] **#004** `wow-map`: `GridInfo` + `GridStateKind` from `NGrid.h`: time tracker, relocation timer period, unload active lock, explicit unload lock, loaded flag semantics. Cerrado en `crates/wow-map/src/grid.rs`.
- [x] **#005** `wow-map`: `NGrid` (8×8 `Cell`) from `NGrid.h`: grid id `x * MAX_NUMBER_OF_GRIDS + y`, x/y, state, `is_grid_object_data_loaded`, `get_grid_type`, `visit_grid`, `visit_all_grids`, world-object count by type. Cerrado en `crates/wow-map/src/grid.rs`.
- [x] **#006** `wow-map`: `GridState` update functions from `GridStates.cpp`: Invalid no-op, Active → Idle when no players/active objects, Idle → Removal, Removal → unload if no lock. Implementado con `MapGridHost` para mantenerlo testeable antes de full `Map`.
- [x] **#007** `wow-map`: `Map` skeleton from `Map.cpp`: `i_grids[64][64]`, `ensure_grid_created`, `ensure_grid_loaded`, `ensure_grid_loaded_for_active_object`, `load_grid_objects`, `reset_grid_expiry`, `active_objects_near_grid`, `unload_grid`. Cerrado en `crates/wow-map/src/map.rs` con hooks explícitos para terrain/object lifecycle.
- [x] **#008** `wow-map`/`wow-data`: `SpawnData` and spawn-store model from `Maps/SpawnData.h` + `ObjectMgr::AddSpawnDataToGrid`: creature/gameobject spawn ids indexed by `(map_id, difficulty, cell_id)` plus personal phase variant `(map_id, difficulty, phase_id, cell_id)`; areatriggers follow C++ `AreaTriggerDataStore` by `(map_id, difficulty, cell_id)` only. Cerrado en `crates/wow-map/src/spawn.rs`.
- [x] **#009** `wow-database`: prepared statements/loaders for creature, gameobject and areatrigger spawn data. Do not implement a per-cell loader query as the canonical model; C++ preloads stores and `ObjectGridLoader` consumes GUID sets. Cerrado con `SEL_CREATURE_SPAWNS`, `SEL_GAMEOBJECT_SPAWNS`, `SEL_AREATRIGGER_SPAWNS` y spawn-group statements contra `ObjectMgr.cpp`/`AreaTriggerDataStore.cpp`.
- [x] **#010** `wow-map`: `ObjectGridLoader::load_n(grid)` from `ObjectGridLoader.cpp`: iterate all 8×8 cells, load creature/gameobject/areatrigger GUIDs from stores, load corpses from map corpse store, set current cell, add to world/grid containers. Cerrado a nivel GUID/container en `crates/wow-map/src/object_grid_loader.rs`; `LoadFromDB`, `MapObject::SetCurrentCell` y `AddToWorld` reales quedan ligados a `#023` entidades canónicas.
- [x] **#010a** `wow-map`: `MultiPersonalPhaseTracker` grid hook from `PersonalPhaseTracker.cpp`: player-triggered grid loading loads personal creature/gameobject spawns once per owner/grid/phase, unload removes grid tracking, owner phase changes mark missing phases for delayed deletion. Cerrado en `crates/wow-map/src/personal_phase.rs` y conectado a `Map::ensure_grid_loaded_for_player_phase`.
- [x] **#011** `wow-map`: grid unload helpers from `ObjectGridLoader.cpp`: `ObjectGridStoper`, `ObjectGridEvacuator`, `ObjectGridCleaner`, `ObjectGridUnloader` traversal/order over grid containers. Cerrado como action pass GUID/container en `crates/wow-map/src/grid_unload.rs`; concrete `Creature::CombatStop`, dynobject/areatrigger cleanup, respawn relocation, `CleanupsBeforeDelete` and deletion effects remain tied to `#023` canonical entities.
- [x] **#012** `wow-map`: terrain hooks from `Map::EnsureGridCreated`: grid coordinate flip `(63 - x, 63 - y)` and `TerrainMgr::LoadMapAndVMap`; keep actual vmap/mmaps loading behind a trait if assets are not ready. Cerrado con `TerrainGridLoader`.
- [x] **#013** `wow-map`: tests integration: spawn store → `EnsureGridLoaded` → `ObjectGridLoader::load_n`; verify cell-level placement, grid state transitions and no grid-size regression. Cerrado con `SpawnGridLifecycle` y tests de `Map::ensure_grid_loaded`.
- [x] **#014** `wow-map`: `MapManager` structural skeleton from `MapManager.h/.cpp`: ordered `i_maps`, `create_world_map`/`create_map_entry`, `find_map`, `do_for_all_maps`, `do_for_all_maps_with_map_id`, serial `update`, `destroy_map`, instance id allocation/free, scheduled script counter. Cerrado en `crates/wow-map/src/manager.rs`; `CreateMap(Player*)` branching for BG/dungeon/group/instance locks remains pending until those types exist.
- [ ] **#014a** `wow-map`/`wow-world`: bind `MapManager::CreateMap(uint32, Player*)` decision tree against real `Player`, `Group`, `InstanceLockMgr`, `Battleground`, `MapEntry`/DB2 difficulty data and recent instance tracking.
  - [x] **#014a.1** Portar el núcleo puro de decisión de `CreateMap(Player*)`/`FindInstanceIdForPlayer` contra `MapManager.cpp`: rechazo de player/map nulos, mundo normal y split-by-faction, BG con `BattlegroundId`/puntero BG, dungeon con dificultad de grupo/player, active lock, recent normal instance, generación de instance id y conflicto flex-lock. La conexión productiva con `Player`/`Group`/`InstanceLockMgr`/`Battleground` reales sigue en `#014a`.
  - [x] **#014a.2** Conectar login de mapas de mundo no instanciados al `wow_map::MapManager` canónico: `WorldSession` recibe el manager global, `MapEntry` expone helpers C++ (`IsDungeon`, `IsBattlegroundOrArena`, `IsGarrison`, `IsSplitByFaction`) y login materializa `ManagedMap` para mundo/split-by-faction. Dungeons/BG/garrison quedan bloqueados hasta runtime real, sin simular campos.
- [x] **#015** `wow-map`: `MapUpdater` API/fallback from `MapUpdater.cpp`: `activate`, `deactivate`, `activated`, `schedule_update`, `wait`; wired into `MapManager::update`. Cerrado como inline deterministic fallback in `crates/wow-map/src/manager.rs`.
- [ ] **#015a** `wow-map`: real `MapUpdater` worker pool equivalent to C++ `ProducerConsumerQueue<MapUpdateRequest*>` + worker threads, if/when maps become independently mutable/sendable enough to update safely in parallel.
- [x] **#016** `world-server/main.rs`: arrancar `MapManager` global + update loop. Cerrado con `wow_map::MapManager` canónico inicializado desde `GridCleanUpDelay`, `MapUpdateInterval` y `MapUpdate.Threads`, y task global que llama `MapManager::update(diff)` como `World::Update -> sMapMgr->Update(diff)` en C++.
- [ ] **#016a** `wow-world`/`world-server`: eliminar los ticks de mundo session-local como fuente de verdad (`WorldSession::tick_creatures_sync`, `tick_combat_sync`, visibilidad/aura ligada a entidades) cuando existan entidades canónicas y `ObjectAccessor`; no cerrar como port completo hasta que esos ticks pasen por Map/Entity.
- [ ] **#017** Limpiar `crates/wow-world/src/map_manager.rs`: reemplazar implementación legacy por el nuevo `wow-map`; retener tests útiles solo si siguen contrastados contra C++.
  - [x] **#017a** Conectar el `Map` canónico con su store de objetos para el núcleo de `Map::AddToMap`/`AddToGrid`/`RemoveFromMap`: validación de map/tipo/coordenadas, carga/creación de grid, inserción/removal en contenedor world/grid por tipo, `SetCurrentCell`, `AddToWorld`/`RemoveFromWorld`, `ResetMap` y ciclo `SetIsNewObject`. Quedan fuera visitors/visibilidad y sustitución de ticks legacy.
  - [x] **#017b** Portar el núcleo de relocation de celda del `Map`: misma celda solo relocaliza, misma grid mueve entre celdas, active/player en otra grid carga destino, objeto normal hacia grid no cargada queda bloqueado sin mutar. Quedan fuera delayed move lists, relocation notifiers, visibilidad y workers AI.
  - [x] **#017c** Portar la base de `VisitNearbyCellsOf`: `CalculateCellArea` y scan no-create de celdas existentes para devolver GUIDs world/grid canónicos sin cargar grids. Quedan fuera `ObjectUpdater`, paquetes de visibilidad y workers AI.
  - [x] **#017d** Representar el plan puro de `PlayerRelocationNotifier`: visibles, out-of-range, updates recíprocos de players y checks AI de criaturas desde GUIDs canónicos. Quedan fuera `UpdateData`, transporte y ejecución real de AI/visibilidad.
  - [x] **#017e** Representar el plan puro de `CreatureRelocationNotifier`: updates de visibilidad de players y checks AI dirigidos criatura↔player/criatura, con puerta de source alive y supresión de reverse-check si el target necesita visibility notify. Quedan fuera ejecución real de AI/visibilidad y delayed traversal.
  - [x] **#017f** Representar selección de `DelayedUnitRelocation`: criaturas/players que necesitan notify y skip de viewpoints inválidos no-self, deduplicando criaturas world/grid. Quedan fuera ejecución anidada de visitors y efectos reales.
  - [x] **#017g** Representar el plan puro de `AIRelocationNotifier`: cada criatura cercana ejecuta check contra la unidad fuente, y si la fuente es criatura también se representa el check inverso. Quedan fuera `CreatureUnitRelocationWorker::CanSeeOrDetect` y efectos AI reales.
  - [x] **#017h** Representar selección de `ObjectUpdater`: desde GUIDs cercanos y store canónico del Map, seleccionar solo objetos actualizables que siguen `IsInWorld`, preservando que players se actualizan por sesión/player loop separado. Quedan fuera ejecución real `Update(diff)`, farsight/combat/aura/summon visits y `SendObjectUpdates`.
  - [x] **#017i** Representar selección de fuentes de `Map::Update`: players en map-ref, viewpoints, visitas extra de combat/aura/summon, active non-players, transports y gate de relocation notifies. Quedan fuera ejecución real de session/player/object update, respawns, scripts, weather, phase tracker, move lists y packet flush.
  - [x] **#017j** Representar selección/timers de `ProcessRelocationNotifies`: solo grids activos, timer de relocation, celdas marcadas en orden C++ y reset posterior. Quedan fuera mutación real `ResetAllNotifies` y ejecución anidada de `DelayedUnitRelocation`.
  - [x] **#017k** Representar decisiones de `MoveAll*InMoveList`: relocation normal, reset de entries no activas, skip out-of-world, fallback a respawn para criatura/GO y remove/pet-remove si falla; dynamic/areatrigger solo reportan bloqueo. Quedan fuera side effects de vehículo, `AfterRelocation`, position/shape data, visibility y remove-list real.
  - [x] **#017l** Representar traversal con celdas marcadas de `VisitNearbyCellsOf`: `CalculateCellArea`, skip de posiciones inválidas, deduplicación de celdas ya visitadas y recolección world/grid sin cargar grids. Quedan fuera binding real de `GetGridActivationRange` y ejecución de visitors.
  - [x] **#017m** Representar `NotifyFlags`/`ResetNotifier`: flags C++ en `EntityObject` y reset de notifies para players/criaturas en celdas seleccionadas. Queda fuera cablearlo en la ejecución completa de `ProcessRelocationNotifies`.
  - [x] **#017n** Conectar selección de `DelayedUnitRelocation` a flags reales del store de Map: criaturas/players con `NOTIFY_VISIBILITY_CHANGED` y skip de viewpoints inválidos. Queda fuera ejecución de notifiers y envío `SendToSelf`.
  - [x] **#017o** Orquestar `ProcessRelocationNotifies` completo a nivel representado: timer/celdas, delayed selection antes del reset y reset posterior de flags. Quedan fuera efectos reales de notifiers/AI/paquetes.
  - [x] **#017p** Expandir la selección de `DelayedUnitRelocation` a planes de `PlayerRelocationNotifier`/`CreatureRelocationNotifier` usando store canónico de Map, `MAX_VISIBILITY_DISTANCE + combat_reach`, contexto de GUIDs visibles previos del player y contexto alive de criatura. Quedan fuera `UpdateData`/`SendToSelf`, caso especial de pasajeros de transporte y ejecución real de `CreatureUnitRelocationWorker`.
- [ ] **#018** Migrar `handlers/loot.rs` a lookups de criatura/GO vía Map/ObjectAccessor equivalente, no `self.creatures`.
  - [x] **#018a** `CMSG_LOOT_ITEM`: la validación de distancia de criatura puede resolver el owner desde el `wow_map::MapManager` canónico antes del fallback legacy, y las comprobaciones representadas de GameObject aceptan existencia desde el Map canónico en vez de depender solo de `visible_gameobjects`. Queda pendiente `CMSG_LOOT_UNIT`, alive/loot-state canónico de criatura, distancia GO exacta y ownership real de Loot en entidad.
  - [x] **#018b** Consolidar el estado de criatura usado por `CMSG_LOOT_UNIT`, AE-loot y `LootResponse` en un helper único contrastado con `ObjectAccessor::GetCreature` + `AELootCreatureCheck` + `GetLootForPlayer`: alive, posición, entry/level, loot ids, money range y encounter id salen de la fuente de criatura actual antes de generar/abrir loot. Queda pendiente mover esos campos a entidades Map-owned reales y eliminar el fallback legacy.
  - [x] **#018c** Conectar el registro runtime de criaturas con el `wow_map::MapManager` canónico cuando el mapa ya existe: `register_world_creature` añade un `WorldObject` de criatura con GUID/entry/map/posición/phase, `mutate_world_creature` relocaliza el objeto canónico tras mutaciones legacy y `remove_world_creature` lo quita del Map. Queda pendiente mover estado typed de criatura/loot al store canónico y crear mapas de instancia con el flujo completo de `MapManager::CreateMap`.
  - [x] **#018d** `CMSG_LOOT_ITEM` para GameObject usa estado runtime contrastado con `GameObject::IsWithinDistInMap`: las queries de visibilidad registran map/posición/tipo y reflejan GO en el Map canónico existente; loot releasea GO lejano con distancia de interacción representada y conserva la excepción C++ de `GAMEOBJECT_TYPE_FISHINGHOLE`. Quedan pendientes owner GUID de GO, display-box/spell-lock ranges completos y entidad `GameObject` typed en `wow_map`.
  - [x] **#018e** `CMSG_LOOT_RELEASE` para GameObject replica el orden C++ de `DoLootRelease`: envía release antes de validar mundo/distancia, GO missing/lejano no muta loot/state, GO normal fully looted pasa a `JustDeactivated` y limpia loot representado, GO normal unlooted queda `Activated` para el player, `GAMEOBJECT_TYPE_FISHINGNODE` queda `JustDeactivated` y `GAMEOBJECT_TYPE_FISHINGHOLE` suma uso representado y vuelve a `Ready` o `JustDeactivated` al alcanzar `FishingHole.MaxOpens`; `OnLootRelease` de gathering node representa `SetGoStateFor(GO_STATE_ACTIVE, player)` y chest personal no-consumable ejecuta despawn per-player como C++ `DespawnForPlayer`: usa `chestRestockTime` o respawn default, emite update out-of-range solo para la sesión y filtra la recreación hasta que expire el temporizador. `GetInteractionDistance` cubre `InteractRadiusOverride` desde `Data0..Data34` de spawns DB visibles y las ramas puras por tipo de GO de C++ (`AREADAMAGE`, quest/text/flags/minigame, binder, chair/barber, fishing node/hole, camera/map/dungeon/destructible/door, mailbox/guild bank y default). `IsAtInteractDistance(Position, radius)` usa `GameObjectDisplayInfo.db2` para caja geométrica escalada/rotada antes de caer a distancia esférica, y `GetSpellForLock` queda representado para `LOCK_KEY_SPELL` y `LOCK_KEY_SKILL` con spells conocidos `SPELL_EFFECT_OPEN_LOCK`, usando `SpellMisc.range_index -> SpellRange.range_max` como rango efectivo. La excepción C++ de owner (`GetOwnerGUID() == player`) queda soportada para estado runtime con `CreatedBy` registrado; `GameObjectCreateData`, `GameObjectDataValues` y el bridge VALUES ya propagan `CreatedBy` no vacío con el bit C++ correcto. `MapObjectRecord`/`wow_map` ya pueden retener una entidad `GameObject` tipada y exponerla como `WorldObject` para los algoritmos existentes; los GO representados se insertan tipados en el Map canónico. Los spawns DB siguen sin owner. Queda pendiente cablear `CreatedBy` desde el lifecycle real de GO temporales cuando exista esa ruta de creación runtime.
- [ ] **#019** Migrar `handlers/combat.rs` y `session.rs::tick_combat_sync` al Map/Entity model.
  - [x] **#019a** `CMSG_ATTACK_SWING`/`CMSG_ATTACK_STOP`: contrastado contra `CombatHandler.cpp` y `Unit::Attack`/`AttackStop`, el estado de ataque del player ya se materializa en una entidad `Player` tipada dentro del `wow_map::MapManager` canónico. `MapObjectRecord`/`wow-map` aceptan `Player` tipado sin perder acceso `WorldObject`; `AttackSwing` fija `Unit::attacking` y `UNIT_FIELD_TARGET`, y `AttackStop` limpia ambos, manteniendo el bridge session-local mientras se retira `tick_combat_sync`.
  - [ ] **#019b** Completar el port de `Unit::Attack`/`AttackStop`: validaciones C++ completas (`IsValidAttackTarget`, self/dead/world/mounted/GM/evade/vehicle seat), cambios de víctima con `_removeAttacker`/`_addAttacker`, timers/offhand, interrupciones, pets/controlados y side effects de AI.
    - [x] **#019b.1** `wow-entities::Unit` expone helpers contrastados contra `Unit::Attack`/`AttackStop`: rechaza self/dead/not-in-world básico, cambia `m_attacking`, setea/limpia `UNIT_FIELD_TARGET`, gestiona `UNIT_STATE_MELEE_ATTACKING`, interrumpe `CURRENT_MELEE_SPELL` al cambiar/parar, y representa `_addAttacker`/`_removeAttacker` como set de atacantes. `WorldSession` usa esos helpers para el player canónico y actualiza attacker-set cuando la víctima también es `Player` tipado.
    - [x] **#019b.2a** Añadir contexto C++ de validación a `Unit::Attack`: mounted player, attacker creature evading, victim GM player y victim creature evading. `WorldSession::start_player_attack_like_cpp` ya pasa el estado real de montura del player y revierte `combat_target`/`in_combat` si `Attack` rechaza como C++.
    - [x] **#019b.2b** `MapObjectRecord`/`wow-map` soportan `Creature` tipada, `register_world_creature` inserta la misma entidad `Creature` en el `Map` canónico, `mutate_world_creature` sincroniza la entidad tipada tras mutaciones y `relocate_map_object_like_cpp` conserva cuerpos tipados al relocalizar.
    - [x] **#019b.2c** `WorldSession::start_player_attack_like_cpp` usa `Creature`/`Player` tipados del `Map` canónico como fuente de `victim->IsAlive()`/`victim->IsInWorld()` y mantiene `_addAttacker`/`_removeAttacker` para víctimas `Creature` tipadas además de `Player`; atacar criatura muerta ya rechaza y revierte estado de sesión.
    - [x] **#019b.2d** `Player` tipado representa `PLAYER_EXTRA_GM_ON` como en C++ `Player::IsGameMaster()`, `Creature` tipada representa `IsInEvadeMode()` via `UNIT_STATE_EVADE` y `IsEvadingAttacks()` como `evade || CanNotReachTarget()`, y `WorldSession::start_player_attack_like_cpp` rechaza víctimas GM/evading desde el `Map` canónico tipado.
    - [x] **#019b.2e** `Unit::Attack` elimina auras con `SPELL_AURA_MOD_UNATTACKABLE` antes de resolver same-target/cambio de target, como C++ `HasAuraType`/`RemoveAurasByType`. `AuraSubsystem` mantiene índice representado por tipo de aura para poder retirar todas las aplicaciones del tipo y marcar removed auras.
    - [x] **#019b.2f** `Unit` representa `m_attackTimer[MAX_ATTACK]`, `m_baseAttackSpeed` setters y `CanDualWield` mínimo; `Unit::Attack` aplica el delay C++ de offhand para no-players (`OFF_ATTACK=max(OFF_ATTACK, BASE_ATTACK timer + 50% base attack time)`) y no lo aplica a players.
    - [x] **#019b.2g** `Unit::Attack` representa los side effects C++ de criatura no controlada por player: `EngageWithTarget(victim)` como threat-ref, `SendAIReaction(AI_REACTION_HOSTILE)`, `CallAssistance()`, limpiar emote y forzar `UNIT_STAND_STATE_STAND`; se omite correctamente cuando la unidad está controlada por player.
    - [x] **#019b.2h** `Unit::Attack` representa el callback C++ de pets/controlados para players: al atacar, `TYPEID_PLAYER` notifica `OwnerAttacked(victim)` solo a los controlados que son criaturas con AI, dejando cola observable en `ControlSubsystem` hasta que el runtime aplique el callback real.
    - [x] **#019b.2i** Portar el tramo representable de `WorldObject::IsValidAttackTarget` usado por `Unit::Attack`: rechazo de `UNIT_STATE_UNATTACKABLE` (`IN_FLIGHT`), target `NON_ATTACKABLE`/`NON_ATTACKABLE_2`/`ON_TAXI`/`NOT_ATTACKABLE_1`/`UNINTERACTIBLE`, player attacker `UBER` representado, e inmunidades PC/NPC según `PLAYER_CONTROLLED`. `WorldSession` alimenta flags/unit-state desde `Player`/`Creature` tipados canónicos.
    - [x] **#019b.2j** `WorldSession::HandleAttackSwingOpcode` representa la validación C++ de asiento de vehículo: si el player está en vehículo, solo puede iniciar ataque cuando `VehicleSeatEntry::Flags` contiene `VEHICLE_SEAT_FLAG_CAN_ATTACK`.
    - [x] **#019b.2k** `ControlSubsystem::remove_charmed_by` cubre la semántica C++ de `RemoveCharmedBy` para vehículos: al salir de un charm `CHARM_TYPE_VEHICLE` no se marca `LastCharmerGUID`, evitando el reacquire/reattack del vehículo sobre su pasajero.
    - [x] **#019b.2l** `WorldObject::IsValidAttackTarget` representa las reglas de relación cuando el runtime ya aporta relación calculada: CvC requiere hostilidad en cualquiera de las dos direcciones y PvP/PvC/CvP rechaza friendly en cualquiera de las dos direcciones.
    - [x] **#019b.2m** `WorldObject::IsValidAttackTarget` añade representación explícita para el rechazo C++ de visibilidad (`CanSeeOrDetect` cuando el runtime aporta el resultado) y para inmunidad de pets de players montados frente a atacantes sin `GetAffectingPlayer()`.
    - [x] **#019b.2n** `WorldObject::IsValidAttackTarget` representa las ramas C++ restantes cuando el runtime aporta snapshots: player-vs-creature con contested guard/`PLAYER_FLAGS_CONTESTED_PVP`, reputación forzada/`AtWar`; duelo en progreso antes de sanctuary; sanctuary PvP; y checks PvP finales (`IsPvP`, FFA PvP y `UNIT_BYTE2_FLAG_UNK1`).
    - [x] **#019b.2o** `WorldSession::start_player_attack_like_cpp` alimenta snapshots reales ya disponibles para `IsValidAttackTarget`: atacante player con `GetAffectingPlayer()`, target player con `GetAffectingPlayer()`, y `PLAYER_FLAGS_UBER` leído del `PlayerData::Flags` canónico. `UnitData::PvpFlags` queda representado como `UnitPvpFlags` contrastado contra C++ `UnitPVPStateFlags`, viaja por VALUES y alimenta `IsPvP`, FFA PvP, sanctuary y `UNIT_BYTE2_FLAG_UNK1`; el caso player-vs-player sin flag PvP real sigue fallando cerrado.
    - [x] **#019b.2p** `WorldSession::start_player_attack_like_cpp` alimenta el tramo base representable de `CanSeeOrDetect` usando `PhaseShift::can_see` canónico para players/criaturas tipados; targets fuera de fase rechazan el ataque antes de mutar `m_attacking`, como el guard C++ de `WorldObject::IsValidAttackTarget`.
    - [x] **#019b.2q** `Player` tipado representa el snapshot mínimo de `duel` de C++ (`Opponent` + `DUEL_STATE_IN_PROGRESS`) y `WorldSession::start_player_attack_like_cpp` lo preserva al refrescar la entidad canónica y lo alimenta en `player_player_duel_in_progress`. Con esto el caso player-vs-player en duelo se acepta antes de sanctuary, igual que `WorldObject::IsValidAttackTarget`; la creación/finalización completa de duelos queda para el runtime DuelHandler.
    - [x] **#019b.2r** `WorldSession::start_player_attack_like_cpp` alimenta la rama player-vs-creature de reputación de `WorldObject::IsValidAttackTarget`: `Creature` tipada puede aportar el `FactionEntry` usado para reputación de ataque y `IsContestedGuard`, `Player` conserva `ReputationFlags::AtWar` y forced-reaction ids, y el refresh canónico preserva gameplay/forced reactions. Si el snapshot dice que hay reputación y no hay forced rank ni `AtWar`, Rust rechaza como C++; si `AtWar` está activo, acepta. El lookup real `FactionTemplateEntry -> FactionEntry` desde DB2 sigue pendiente para no derivarlo de IDs incorrectos.
    - [x] **#019b.2s** `WorldSession` acepta `FactionTemplateStore` real y resuelve `creature->GetFactionTemplateEntry()->Faction` + `FACTION_TEMPLATE_FLAG_CONTESTED_GUARD` desde `FactionTemplate.db2` para alimentar la rama de reputación de `IsValidAttackTarget`; el snapshot manual queda solo como fallback explícito cuando la store DB2 todavía no está cargada en tests/runtime parcial.
    - [x] **#019b.2t** `wow-data` incorpora `FactionStore` mínimo con `FactionEntry::ReputationIndex` y `CanHaveReputation()`; `WorldSession` usa esa store para que la rama de reputación player-vs-creature solo pueda rechazar por `AtWar` cuando C++ tendría `FactionState` representable. Facciones sin reputación (`ReputationIndex < 0`) o sin state del player no fuerzan rechazo.
    - [x] **#019b.2u** `wow-entities::CombatSubsystem` incorpora los primitivos contrastados de `CombatManager::CanBeginCombat()` y `RevalidateCombat()`: valida self/world/alive/map/phase/evade/in-flight/combat-disallowed/friendly/GM, recorre refs PvE/PvP, conserva solo refs válidas y limpia threat/reverse-threat local de refs terminadas. `WorldSession::start_player_attack_like_cpp` empieza a crear refs bidireccionales player-victim en el mapa canónico solo cuando `CanBeginCombat` pasa, sin mezclarlo con `Unit::Attack`.
    - [x] **#019b.2v** `WorldSession::tick_combat_sync` revalida las refs de combate canónicas del player contra `CanBeginCombat` y purga ambos lados (`player` y target `player/creature`) cuando el target desaparece de mapa/fase, muere, entra en evade/in-flight o marca `combat_disallowed`, preservando la separación C++ entre terminar refs de `CombatManager` y `Unit::AttackStop`.
    - [x] **#019b.2w** `Unit` representa el estado runtime usado por `WorldObject::CanSeeOrDetect` para el tramo de ataque: visibilidad GM server-side, invisibilidad/detección por tipo, stealth/detección por tipo, arco frontal `HasInArc(pi)`, distancia exacta, fórmula C++ `30 + (level-1)*5 + detect - stealth`, cap de 30 yd para players y expansión de alert-distance cuando se solicite. `WorldSession::start_player_attack_like_cpp` combina fase + `CanSeeOrDetect` real para targets `Player`/`Creature` canónicos y el refresh de `Player` canónico preserva este estado runtime para no perder auras/detectores al resincronizar el snapshot de sesión.
    - [x] **#019b.2x** `wow-map::Map` incorpora el barrido global multi-owner de refs de combate, equivalente al `CombatManager` por owner pero ejecutado sobre todos los `Player`/`Creature` tipados del mapa canónico: recolecta refs PvE/PvP de cada owner, reconstruye `CanBeginCombat` desde los objetos reales, purga ambos lados si el target falta o ya no puede combatir, y conserva refs válidas de otros owners. `WorldSession::tick_combat_sync` delega en ese barrido del Map en vez de revalidar solo el player de la sesión.
    - [x] **#019b.2y** `UnitVisibilityDetectionStateLikeCpp` amplía `CanSeeOrDetect` con gates C++ previos a detect: `IsNeverVisibleFor`, `IsAlwaysVisibleFor`, `CanAlwaysSee`, private object owner (`owner`, mismo owner privado o group-visible), visibilidad ghost server-side con excepción de group visibility, `IsInvisibleDueToDespawn` y `IsAlwaysDetectableFor`; además corrige `TOTAL_INVISIBILITY_TYPES` a 38 con máscara `u64`, contrastado contra `SharedDefines.h`.
    - [x] **#019b.2z** `WorldSession` alimenta el snapshot canónico de `Player::IsNeverVisibleFor` desde estado real de sesión C++: `PlayerLoading()` marca al player canónico como nunca visible para seers mientras dura el ConnectTo/login, y `m_playerLogout` se representa solo durante la rutina de logout, no durante el countdown. `Unit::Attack` contra un player PvP en loading rechaza por `CanSeeOrDetect` antes de mutar combate, y vuelve a aceptar cuando `PlayerLoading` se limpia.
    - [x] **#019b.2aa** `ActivePlayerData::FarsightObject` queda representado en `Player`, viaja al bridge de VALUES y alimenta `Player::CanAlwaysSee` de forma target-specific en `CanSeeOrDetect`: si el seer tiene farsight sobre el GUID del target, puede verlo aunque el target tenga invisibilidad no detectable. El refresh canónico preserva el farsight runtime existente.
    - [x] **#019b.2ab** `Player::CanNeverSee` queda alimentado desde el runtime real de sesión: mientras no exista `PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME` tras `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE`, el player canónico marca `seer_can_never_see_target` y `CanSeeOrDetect` rechaza antes de `Unit::Attack`; al recibir el ACK de active mover se limpia el gate y se solicita refresh de visibilidad como C++.
    - [x] **#019b.2ac** `WorldObject::SmoothPhasing` queda representado con storage single/viewer-dependent como C++ (`SetViewerDependentInfo`, `ClearViewerDependentInfo`, `SetSingleInfo`, `IsReplacing`, `IsBeingReplacedForSeer`, `GetInfoForSeer`, `DisableReplacementForSeer`) y `CanSeeOrDetect` rechaza targets que estan siendo reemplazados para el seer antes de conditions/deteccion, igual que `WorldObject::CanSeeOrDetect`.
    - [x] **#019b.2ad** `WorldObject::CanSeeOrDetect` alimenta el gate global de conditions de visibilidad por `(TypeId, Entry)` desde `ConditionMgr::IsObjectMeetingVisibilityByObjectIdConditions`: `UnitVisibilityDetectionStateLikeCpp` conserva `object_id_visibility_conditions_met`, omite el gate para objetos privados como C++, y `WorldSession::start_player_attack_like_cpp` evalua la `ConditionEntriesByTypeStore` real con el seer antes de permitir `Unit::Attack`.
    - [x] **#019b.2ae** `SPELL_AURA_MOD_STALKED` queda conectado al `CanSeeOrDetect` real de `Unit`: `AuraSubsystem` soporta `HasAuraTypeWithCaster`, `Unit::IsAlwaysDetectableFor` se replica aceptando targets con aura stalked casteada por el seer aunque tengan invisibilidad no detectable, y `WorldSession::start_player_attack_like_cpp` lo usa desde el target canónico antes de `Unit::Attack`.
    - [x] **#019b.2af** La visibilidad owner/group de C++ queda alimentada desde estado real: `CanSeeOrDetect` acepta units cuyo `GetCharmerOrOwnerGUID()` es el seer o cuyo owner player es group-visible para el seer; `WorldSession` deriva ese snapshot desde `GroupRegistry` real. El gate de private objects tambien usa group GUID real (`ObjectGuid::create_group(group_guid)`) para replicar `Player::IsInGroup(privateObjectOwner)`.
    - [x] **#019b.2ag** `Spell::EffectSummonType` queda representado para private summons: `WorldSession` deriva `privateObjectOwner` desde `SummonPropertiesFlags::OnlyVisibleToSummoner`/`OnlyVisibleToSummonerGroup`, hereda el owner si el caster ya es private object, usa GUID de grupo real cuando el caster player esta en grupo y aplica ese owner al summon criatura canónico para que `CanSeeOrDetect` bloquee/permita como C++.
    - [x] **#019b.2ah** `Player::CanAlwaysSee` queda alimentado con `GetUnitBeingMoved()`: `WorldSession` mantiene el GUID de la unidad movida, `CMSG_SET_ACTIVE_MOVER` compara contra ese mover como C++ y `start_player_attack_like_cpp` permite ver/atacar la unidad controlada aunque sea invisible para el seer normal.
    - [x] **#019b.2ai** `WorldSession::LogoutPlayer(save)` empieza a usar snapshot canónico equivalente al núcleo representable de `Player::SaveToDB`: antes de soltar el player del mundo, Rust sincroniza los mirrors de sesión desde el `Player` tipado canónico y persiste level/xp/money/played time, manteniendo la limpieza de buyback antes del save como C++.
    - [ ] **#019b.2** Completar validaciones y side effects no portados: logout completo con persistencia canónica total `Player::SaveToDB` (posición/map/transport/taxi/rest/honor/health/powers/equipment cache/quests/spells/reputation/achievements/pets y transacción completa; conectado con #025b/#028).
  - [ ] **#019c** Migrar `tick_combat_sync` a actualización de Map/Unit/Threat real: daño, swing timers, death/stop movement y salida de combate no deben depender de `WorldSession::combat_target` como fuente principal.
    - [x] **#019c.1** `tick_combat_sync` empieza a seguir la fuente C++ `Unit::GetVictim()` usando el target canónico de `Player.unit().attacking()` como fuente primaria y deja `WorldSession::combat_target` como espejo/fallback. Si el target desaparece o muere, también limpia el estado de ataque canónico del player; los fallos de envío de paquetes ya no abortan la limpieza de estado local.
    - [x] **#019c.2** `tick_combat_sync` empieza a representar `Unit::UpdateMeleeAttackingState` para el player canónico: decrementa `m_attackTimer`, salvo current generic/channeled spell con `SPELL_ATTR6_DELAY_COMBAT_TIMER_DURING_CAST`, solo pega cuando `BASE_ATTACK`/`OFF_ATTACK` están listos, usa daño del `Unit` atacante, aplica `resetAttackTimer(...)` tras cada swing y respeta `ATTACK_DISPLAY_DELAY` para no mostrar main/offhand simultáneos. También representa los errores C++ `NotInRange` y `BadFacing` para melee, evitando el golpe, reintentando con timer corto de 100 ms y enviando `SMSG_ATTACK_SWING_ERROR` con supresión de duplicados como `Player::SetAttackSwingError`. `DoMeleeAttackIfReady`/`AttackerStateUpdate` empieza a cubrir guards representables (`UNIT_STATE_MELEE_ATTACKING` requerido antes de procesar swings, `UNIT_STATE_CHARGING` como salida temprana sin reset, `UNIT_STATE_CASTING` salvo channel con acciones permitidas, `PACIFIED`, `UNIT_STATE_CANNOT_AUTOATTACK` salvo extra, `SPELL_AURA_DISABLE_ATTACKING_EXCEPT_ABILITIES` y `IsWithinLOSInMap` cuando el runtime aporta snapshot) reseteando timer sin daño como C++, ejecuta `RemoveAurasWithInterruptFlags(Attacking)` para auras con `SpellAuraInterruptFlags::Attacking`, consume `CURRENT_MELEE_SPELL` en `BASE_ATTACK` en lugar de aplicar daño blanco, añade threat al target criatura por daño recibido como `ThreatManager::AddThreat`, aplica daño/`AttackerStateUpdate` a víctimas `Player` tipadas sin limpiar falsamente el combate, calcula `overDamage` en golpes letales, actualiza `_lastDamagedTargetGuid` representado para golpes melee con daño, sincroniza el set de attackers también en `MapManager::WorldCreature` al iniciar/parar autoataque, y limpia threat/attackers al morir. El camino legacy sin player canónico conserva el comportamiento anterior hasta completar la migración; si existe Player canónico y `Unit::GetVictim()`/`attacking()` es `None`, ya no revive un `WorldSession::combat_target` obsoleto.
    - [x] **#019c.3** `tick_combat_sync` deja de usar rango melee fijo y replica las ramas C++ de `Unit::IsWithinMeleeRangeAt`/`GetMeleeRange`/`IsWithinBoundaryRadius`: el alcance usa `max(attacker.combatReach + target.combatReach + 4/3, NOMINAL_MELEE_RANGE)`, y el error `BadFacing` se omite si el atacante esta dentro del boundary radius del target (`max(target.boundingRadius, MIN_MELEE_REACH)`).
    - [x] **#019c.4** `tick_combat_sync` refleja `ThreatManager::AddThreat` en el `wow_map` canónico para daño melee player->creature: asegura refs de combate bidireccionales como `CombatManager::SetInCombatWith`, copia la `ThreatReference` de criatura a `player.threatened_by_me` y revalida/purga refs al morir la criatura antes de cerrar el tick. Quedan pendientes `Unit::Kill`/death pipeline completo, procs, AI y loot.
    - [x] **#019c.5** `WorldCreature::take_damage` deja de saltar directamente a `DeathState::Corpse` y canaliza la muerte por el runtime representado de `Creature::setDeathState(JUST_DIED)`: fija HP a 0, registra `death_time_ms`, limpia target/attacking, programa corpse/respawn timers y marca `SaveRespawnTime`/visibilidad como el tramo C++ `Unit::Kill -> Creature::setDeathState(JUST_DIED)`. Quedan pendientes los siguientes P0 del kill bridge: tap list en daño real, generación de loot en kill, XP/quest rewards desde melee y hooks ordenados de AI/procs.
    - [x] **#019c.6** El daño real player->creature registra tap list antes de aplicar daño, como `Unit::DealDamage -> Creature::SetTappedBy`: `apply_damage` y `tick_combat_sync` llaman `Creature::set_tapped_by_player` con el player actual y miembros de grupo disponibles, activando `has_loot_recipient` para que las siguientes tareas P0 puedan generar loot/rewards desde los tappers. Quedan pendientes generación de loot en kill, enforcement de `isAllowedToLoot`, XP/quest rewards desde melee y hooks ordenados AI/procs.
    - [x] **#019c.7** `apply_damage` genera loot de criatura durante el kill, antes de rewards/quest hooks, usando la tap list representada como C++ `Unit::Kill` genera `Loot` para los tappers antes de `KillRewarder`. Se añade `ensure_represented_creature_kill_loot_like_cpp`, que reutiliza el generador existente, conserva `allowed_looters` de tappers y evita que `CMSG_LOOT_UNIT` sea la primera creación de loot en este camino. Quedan pendientes mover el melee tick síncrono a un bridge/cola async para generar loot en kill, enforcement estricto de `isAllowedToLoot` en el handler, XP/quest rewards desde melee y hooks AI/procs.
    - [x] **#019c.8** `CMSG_LOOT_UNIT` y `CMSG_LOOT_MONEY` aplican `Player::isAllowedToLoot` de C++ en corpse loot: abrir loot ya no concede `allowed_looters`, las criaturas con tap list rechazan no-tappers antes de generar/responder, AE loot omite corpses no permitidos, el dinero exige autorización superior del `Loot`, y los tests cubren el bypass previo de coin loot, tap list ajena y active loot forzado. Quedan pendientes generación de loot en kill desde melee tick síncrono, XP/quest rewards desde melee y hooks AI/procs.
    - [x] **#019c.9** `tick_combat_sync` ya no deja kills melee sin loot: al morir una criatura en el tick síncrono se encola el GUID y `process_pending()` drena esa cola async para llamar `ensure_represented_creature_kill_loot_like_cpp`, manteniendo el orden C++ de generar loot tras el kill y antes de que el cliente abra corpse loot sin convertir todo el tick a async. Quedan pendientes XP/quest rewards desde melee y hooks AI/procs.
    - [x] **#019c.10** La cola de kills melee también ejecuta la parte representada de `KillRewarder`: después de generar loot, `process_pending()` calcula XP con `creature_kill_xp`, llama `give_xp(..., is_kill=true)` y aplica `on_creature_killed(entry,guid)` para crédito de quests, igualando el camino `apply_damage` y el orden C++ `Unit::Kill -> loot -> KillRewarder`. Quedan pendientes hooks AI/procs y reward group/raid completo.
    - [x] **#019c.11** Spell kill y melee kill registran el ledger representado de hooks posteriores a `KillRewarder` en el orden C++ de `Unit::Kill`: killer `PROC_FLAG_KILL`, tappers `PROC_FLAG_2_TARGET_DIES`, victim `PROC_FLAG_DEATH` y `CreatureAI::JustDied`. Es deliberadamente un ledger/test contract, no ejecución real de `ProcSkillsAndAuras`/SmartAI/BossAI. Quedan pendientes separar daño letal de `setDeathState(JUST_DIED)` para permitir procs realmente pre-death, aplicar `LOOTABLE/CAN_SKIN/SKINNABLE` después de death state y antes de AI hooks, `ZoneScript::OnUnitDeath`, criteria killing-blow, pet/totem owner kill proc, group reward distance, y motor real de auras/scripts.
    - [x] **#019c.12** El daño letal de criatura puede dejar HP a 0 sin ejecutar inmediatamente `Creature::setDeathState(JUST_DIED)`: `apply_damage` y el bridge melee usan esa ruta y completan la transición death-state sólo después del ledger de procs, registrando `DeathStateJustDied` antes de `CreatureJustDiedAi`. Esto alinea el punto de extensión con C++ `Unit::Kill`, donde `ProcSkillsAndAuras(... KILL/TARGET_DIES/DEATH ...)` ocurre antes de `victim->setDeathState(JUST_DIED)`. Quedan pendientes mover la ejecución real de auras/scripts a esos puntos, `ZoneScript::OnUnitDeath`, lootable/skinning flags post death-state, pet/totem owner kill proc, group reward distance y criteria killing-blow.
    - [x] **#019c.13** Tras completar death-state, Rust aplica las flags de corpse loot antes de `CreatureJustDiedAi`, como C++ `Unit::Kill` hace `UNIT_DYNFLAG_LOOTABLE` y luego `UNIT_DYNFLAG_CAN_SKIN`/`UNIT_FLAG_SKINNABLE` antes de `AI()->KilledUnit/JustDied`. La ruta actual fija `LOOTABLE` cuando el loot generado aún tiene monedas/items y deja `CAN_SKIN/SKINNABLE=false` hasta portar `SkinLootID`/`LootTemplates_Skinning`. El ledger añade `LootFlagsApplied` entre `DeathStateJustDied` y `CreatureJustDiedAi`.
    - [x] **#019c.14** El ledger de kill ya reserva los dos puntos C++ que faltaban en el orden exacto: `DeliveredKillingBlowCriteria` se registra después de `PROC_FLAG_DEATH` y antes de `setDeathState(JUST_DIED)`, como `killerPlayer->UpdateCriteria(CriteriaType::DeliveredKillingBlow, 1, 0, 0, victim)`; `ZoneScriptUnitDeath` se registra justo después de `DeathStateJustDied`, representando la llamada interna de `Unit::setDeathState(JUST_DIED)` a `ZoneScript::OnUnitDeath`. Sigue pendiente conectar ambos a managers reales de criteria/ZoneScript.
    - [x] **#019c.15** `TapperTargetDiesProc` ahora replica el filtro C++ `Player::IsAtGroupRewardDistance(victim)`: los tappers solo reciben el ledger `PROC_FLAG_2_TARGET_DIES` si estan en el mismo mapa y a `CONFIG_GROUP_XP_DISTANCE` representado (`74.0`) del corpse, o siempre dentro de mapas dungeon segun `MapEntry::is_dungeon()`. La distancia se resuelve desde el player actual o `PlayerRegistry` para miembros remotos; el tap list/loot permanece intacto como en C++, que filtra este proc despues de haber calculado tappers y rewards.
    - [x] **#019c.16** Tras `DeathStateJustDied`/`ZoneScriptUnitDeath`, Rust registra el hook representado `TapperPetKilledUnitAi` para la pet controlada del jugador actual si ese jugador esta en la tap list, respetando el orden C++ `setDeathState(JUST_DIED)` -> pets de tappers `PetAI::KilledUnit(victim)` -> flags de loot/skinning -> `CreatureAI::JustDied`. Queda pendiente extenderlo a pets remotas cuando exista registry/runtime de Pet/AI completo.
    - [x] **#019c.17** Las flags post-death de skinning ya usan `CreatureDifficulty::SkinLootID` y `LootTemplates_Skinning.HaveLootFor(...)` representado: `SEL_CREATURES_IN_RANGE` carga `SkinLootID`, el estado canónico de criatura lo conserva por respawn y `complete_represented_creature_death_state_after_kill_hooks_like_cpp` activa `UNIT_DYNFLAG_CAN_SKIN`/`UNIT_FLAG_SKINNABLE` solo si existe template en `LootStoreKind::Skinning`, manteniendo el orden C++ antes de `CreatureJustDiedAi`.
    - [x] **#019c.18** El ledger de muerte de criatura distingue los dos callbacks C++ de `CreatureAI`: despues de flags loot/skinning registra `CreatureOnHealthDepletedAi { is_kill: true }` y luego `CreatureJustDiedAi`, igual que `ai->OnHealthDepleted(attacker, true); ai->JustDied(attacker);`. Sigue pendiente sustituir el ledger por dispatch real de AI/SmartAI/BossAI.
    - [x] **#019c.19** El final de kill player->creature registra el punto `ScriptMgrOnCreatureKill` despues de los callbacks de AI, reflejando `sScriptMgr->OnCreatureKill(killerPlr, killedCre)` en `Unit::Kill`. OutdoorPvP/Battlefield/Battleground y `wow-script` real siguen pendientes; aqui solo se fija el contrato de orden para no perder el hook.
- [ ] **#020** Migrar `handlers/trainer.rs`, `handlers/misc.rs` y query/use GO al Map/Entity model.
  - [x] **#020a** `CMSG_GAME_OBJ_USE` empieza a resolver existencia/entry/posición desde el `wow_map::MapManager` canónico como C++ `GetPlayer()->GetGameObjectIfCanInteractWith(packet.Guid)` -> `ObjectAccessor::GetGameObject(*player, guid)` -> `Map::GetGameObject(guid)`. Cuando existe mapa canónico ya no se acepta `visible_gameobjects` como fuente de verdad; el fallback legacy solo queda para sesiones sin mapa canónico. También se replica el rechazo de icono `"Point"` y la distancia de interacción C++ (`GameObject::GetInteractionDistance` representado por tipo/radio). Quedan pendientes `GameObject::Use` completo por entidad, mounted/remote-control guards, AI `OnReportUse`, criteria update y query GO cache/locales desde `ObjectMgr`.
  - [x] **#020b** `CMSG_GAME_OBJ_REPORT_USE` deja de ser no-op: lee el GUID como `GameObjReportUse::Read`, valida el GO por la misma ruta canónica/distancia representada de `GetGameObjectIfCanInteractWith` y registra el evento representado `CriteriaType::UseGameobject(entry)` para tests. Quedan pendientes el guard real de remote-control (`GetUnitBeingMoved() == player`), `go->AI()->OnReportUse(_player)` y el achievement/criteria manager persistente.
  - [x] **#020c** `CMSG_QUERY_GAME_OBJECT` completa la forma de `GameObjectTemplate::BuildQueryData` que faltaba: aplica `gameobject_template_locale` para `Name`, `CastBarCaption` y `Unk1` cuando la sesión no es `enUS`, y rellena `Stats.QuestItems` desde `gameobject_questitem ORDER BY Idx ASC` como `ObjectMgr::LoadGameObjectQuestItems`. Sigue pendiente sustituir las queries por un `ObjectMgr`/cache global `QueryData[locale]` inicializado al arranque.
  - [x] **#020d** `CMSG_TRAINER_LIST` y `CMSG_TRAINER_BUY_SPELL` empiezan a validar el NPC contra el mapa canónico como C++ `Player::GetNPCIfCanInteractWith`: GUID no vacío/tipo criatura, existencia por mapa, criatura viva, `npc_flags` de trainer y rango `combatReach + 4.0`. `TRAINER_LIST` exige `UNIT_NPC_FLAG_TRAINER`; `TRAINER_BUY` replica el fallback legacy a `TRAINER | TRAINER_CLASS | TRAINER_PROFESSION`. Cuando existe mapa canónico ya no se acepta resolver sólo por tracker/DB; el fallback tracker/DB queda únicamente para sesiones sin mapa canónico. Pendiente cerrar los guards completos de C++: jugador en mundo/en vuelo, visibilidad a fantasmas/type flags, charmer, reacción hostil, `npcFlags2`, quitar feign death y `PlayerTalkClass::InteractionData`/`TrainerId` persistente antes de enseñar el hechizo.
  - [x] **#020e** `CMSG_GAME_OBJ_REPORT_USE` replica el guard C++ de remote-control: si `GetUnitBeingMoved() != player`, Rust retorna antes de resolver el GO o registrar `CriteriaType::UseGameobject`. Quedan pendientes `GameObjectAI::OnReportUse` real y el criteria manager persistente.
- [ ] **#021** Migrar `session.rs::tick_creatures_sync`, `send_nearby_creatures` y `handlers/character.rs::update_creature_visibility` a visitors/cell queries del Map.
  - [x] **#021a** `send_nearby_creatures` deja de requerir DB cuando el `MapManager` ya tiene criaturas cargadas: usa `get_visible_creatures_in_phase(map, instance, pos, seerPhase)` como primer paso hacia C++ `Cell::VisitAllObjects`/`VisibleNotifier`, construye `UpdateObject` desde `WorldCreature` map-owned y actualiza `visible_creatures` con el set visible real. `update_visibility` también prefiere la lista map-owned para la parte de criaturas cuando existe, aunque todavía conserva la query DB como fallback/poblado inicial y los GO siguen por el camino representado actual. Pendiente portar el visitor completo `UpdateVisibilityOf`, out-of-range sin SQL previo, shared vision/transportes, GO/dynobj/areatrigger/corpse y eliminar `visible_creatures` legacy en #022.
  - [x] **#021b** Los gameobjects visibles empiezan a salir del `wow_map::Map` canónico cuando ya están materializados: `send_nearby_gameobjects` y la rama GO de `update_visibility` consultan `nearby_cell_guids_like_cpp` y reconstruyen `GameObjectCreateData` desde el `GameObject` tipado + runtime representado (`display_id`, `type`, `scale`, `rotation`, owner/state), filtrando fase y despawn per-player. La query SQL sigue como fallback para poblar/arranque cuando el mapa canónico aún no tiene GO cercanos. Pendiente unificar esta ruta con el visitor completo de C++ y cargar GO al mapa desde ObjectMgr/grid-loader, no desde el handler.
  - [x] **#021c** `update_visibility` ya no necesita `world_db` cuando existen fuentes de mapa: primero calcula diffs de criaturas desde `MapManager` y de GO desde `wow_map::Map` canónico, envía creates/out-of-range y sólo cae a SQL cuando no hay ninguna fuente map-owned/canónica. Esto acerca el flujo a C++ `Player::UpdateObjectVisibility`/`VisibleNotifier`, donde el mapa y las celdas son la fuente de verdad. Pendiente quitar definitivamente el fallback SQL cuando el grid-loader/ObjectMgr cargue todas las entidades de mundo.
  - [x] **#021d** El fallback SQL de `send_nearby_creatures` deja de contaminar `visible_creatures` con todos los creatures cargados en el mapa: ahora mantiene el set exacto de GUIDs enviados en ese `UpdateObject`, alineado con C++ `Player::m_clientGUIDs`. Esto evita que un creature map-owned lejano, cargado por otro flujo, quede marcado como visible para este cliente.
  - [x] **#021e** `send_nearby_creatures`/`send_nearby_gameobjects` tratan una fuente canónica vacía como autoritativa: si `MapManager`/`wow_map::Map` existe y la celda no devuelve objetos, Rust no cae a SQL y limpia el set visible local de ese tipo, alineado con C++ `VisibleNotifier`/`Cell::VisitAllObjects` donde el mapa, no la DB, decide visibilidad. El out-of-range completo sigue bajo `update_visibility`.
  - [x] **#022a** `WorldSession` ya no tiene los campos legacy `visible_creatures`/`visible_gameobjects`: se reemplazan por `client_visible_guids_like_cpp`, equivalente directo a C++ `Player::m_clientGUIDs`, y las rutas de criaturas/GO actualizan ese set único sin borrar GUIDs de otros tipos.
- [x] **#022** Limpieza legacy de `WorldSession`: eliminado el campo test/fallback `creatures` y los adaptadores `CreatureAI` dentro de la sesión; los tests de loot registran criaturas mediante `register_world_creature`/`MapManager`, y el estado visible queda en `client_visible_guids_like_cpp` como C++ `m_clientGUIDs`. `_attic/` queda fuera de esta limpieza porque no participa en `WorldSession` y debe auditarse como tarea documental/separada antes de borrarlo.

### Inmediato siguiente (Fase 1 — Entidades canónicas)

- [x] **#023** `wow-entities`: crate/module boundary and base `Object` from `Entities/Object/Object.*`: guid, type id, map id, entry, update flags, in-world/grid state. Cerrado con crate `wow-entities` y `EntityObject` base contrastado contra `Object.h`, `Object.cpp`, `ObjectGuid.h`; `map_id`/`in_grid` quedan como bridge Rust explícito para ownership canónico de mapas.
- [x] **#023a** `wow-entities`/`wow-map`: bind `grid_unload` actions to real entity methods: `Creature::RemoveAllDynObjects`, `Creature::RemoveAllAreaTriggers`, `Creature::CombatStop`, creature/GO respawn relocation, `SetDestroyedObject`, `CleanupsBeforeDelete`, and object deletion. Cerrado contra `ObjectGridLoader.cpp`: stoper/evacuator/cleaner/unloader order is preserved; Creature-owned dynamic objects and area triggers drain like C++ `Unit::RemoveAll*`; all C++ grid-cleaned object kinds (`Creature`, `GameObject`, `DynamicObject`, `Corpse`, `AreaTrigger`, `SceneObject`, `Conversation`) now apply represented cleanup/delete state.
- [x] **#023z** Smoke-test follow-ups from the 2026-05-11 live-client probe: document or quiet non-blocking `TactKey.db2` `DbQueryBulk` misses after contrasting C++ `sTactKeyStore`/hotfix behavior with local extracted DB2 rows; keep external invalid connection/reset logs as operational noise unless a C++-parity issue is reproduced.
  - [x] **#023z.1** `CMSG_REQUEST_RATED_PVP_INFO` queda cubierto como C++ `STATUS_LOGGEDIN`: `WorldSession::HandleRequestRatedPvpInfo` responde con `SMSG_RATED_PVP_INFO` default, 7 brackets vacíos con el orden C++ de `BracketInfo` y `Disqualified=false`.
  - [x] **#023z.2** `TactKey.db2` queda tratado como ruido no bloqueante mientras no exista blob/hotfix local: C++ carga `sTactKeyStore` y usa TactKey para secciones DB2 cifradas/optional hotfix data, pero el server Rust no tiene `TactKey*.db2` local. `DbQueryBulk` mantiene la respuesta correcta `Invalid(3)` para que el cliente use su cache local y los misses de la tabla `0xD3F61A9E` bajan a `debug`.
- [x] **#024** `wow-entities`: `WorldObject` from `Entities/Object/WorldObject.*`: position/orientation, current cell, map pointer/key, phase shift, distance/facing helpers. Cerrado como base `WorldObject`/`WorldLocation`: posición con orientación normalizada, map/instance binding, current-cell bridge, phase-shift mínimo y helpers de distancia/rango contrastados contra `Position.h` y `Object.cpp`; helpers puros de ángulo/arc/line/box cerrados en `#024a`; LOS, terreno, transportes y visibility ranges quedan en subtareas posteriores.
- [x] **#024a** `wow-entities`: pure `Position`/`WorldObject` geometry helpers from `Position.h`, `Position.cpp` and `Object.cpp`: absolute/relative angle conversion, `HasInArc`, `isInFront`, `isInBack`, `HasInLine`, rotated box and double vertical cylinder checks; no LOS/terrain/Map behavior is faked here.
- [x] **#025** `wow-world`/`wow-entities`: `ObjectAccessor` equivalent from `Globals/ObjectAccessor.*`: global player lookup plus map-local object lookup APIs for Creature/GO/Corpse/DynamicObject/AreaTrigger/SceneObject/Conversation/Pet. Cerrado como API base en `wow-entities::ObjectAccessor`: player global by GUID/name, connected vs in-world lookup, same-map player lookup, map-local dispatch by GUID high type and `TypeMask`, incluyendo la rama C++ de corpse/null en `GetObjectByTypeMask`.
- [x] **#025a** `wow-entities`/`wow-map`: conectar `ObjectAccessor` al `wow_map::Map` canónico en vez de mantener un store bridge interno. Cerrado contra `ObjectAccessor.cpp`: los objetos map-locales se resuelven mediante `ObjectAccessorMapSource`/`Map`, `ObjectAccessor` conserva sólo el registro global de players y los helpers map-locales sin source quedan deprecados para no ocultar el requisito de map canónico.
- [ ] **#025b** `wow-entities`/`wow-world`: `ObjectAccessor::SaveAllPlayers()` con persistencia real equivalente a C++ `Player::SaveToDB()` para cada player registrado. El shape está en `save_all_players_with`, pero el cierre completo depende de `Player` runtime/persistencia canónica (`#028`).
- [ ] **#026** `wow-packet`/`wow-entities`: Update fields delta from `Entities/Object/Updates/` and `UpdateFields.h`; stop relying on full re-create as normal update path. Refinado: base `UpdateMask` + writer VALUES de `ObjectData`, `DynamicObjectData`, `SceneObjectData`, `ConversationData`, `GameObjectData`, `CorpseData`, `AreaTriggerData`, `ItemData`, `ContainerData`, `UnitData`, `PlayerData` y `ActivePlayerData` cerrados; bridge `wow-entities` -> `wow-packet` iniciado para `PlayerData`/`ActivePlayerData`; sigue pendiente integrar callsites y cubrir los demás tipos/Unit/Object sin gaps.
- [x] **#026a** `wow-entities`/`wow-packet`: foundation for update-field deltas. Cerrado con `wow_entities::UpdateMask`, `EntityObject::values_update()`, writer `UpdateObject::object_values_update`, y corrección contrastada de `CreatureHealthUpdate` VALUES para no escribir byte `UpdateFieldFlag` de create.
- [x] **#026b** `wow-packet`: `UF::DynamicObjectData::WriteUpdate` VALUES serializer. Cerrado contra `UpdateFields.cpp`: escribe máscara de 7 bits, flush y campos en orden C++ `Caster`, `Type`, `SpellXSpellVisualID`, `SpellID`, `Radius`, `CastTime`, con bloque `UpdateObject::dynamic_object_values_update`.
- [x] **#026c** `wow-packet`: `UF::SceneObjectData::WriteUpdate` VALUES serializer. Cerrado contra `UpdateFields.cpp`: escribe máscara de 5 bits, flush y campos en orden C++ `ScriptPackageID`, `RndSeedVal`, `CreatedBy`, `SceneType`, con bloque `UpdateObject::scene_object_values_update`.
- [x] **#026e** `wow-packet`: `UF::ConversationData::WriteUpdate` VALUES serializer contra `UpdateFields.cpp`, incluyendo `Lines`, `Actors` `DynamicUpdateField` masks y `LastLineEndTime`. Cerrado con bloque `UpdateObject::conversation_values_update`: máscara de 4 bits, tamaño/serialización de `Lines`, máscara dinámica explícita o completa de `Actors`, escritura solo de actores marcados y `LastLineEndTime` en orden C++.
- [x] **#026d** `wow-packet`: serializers VALUES con arrays/nested para `CorpseData`, `GameObjectData` y `AreaTriggerData`. Cerrado contra `UpdateFields.cpp`: `GameObjectData` cubre `StateWorldEffectIDs`, `EnableDoodadSets`, `WorldEffects` y campos escalares en orden C++; `CorpseData` cubre `Customizations`, campos base y `Items[19]`; `AreaTriggerData` cubre `ScaleCurve`, `VisualAnim`, GUIDs y escalares con orden nested C++.
- [x] **#026f** `wow-packet`: serializers VALUES completos para `UF::ItemData` y `UF::ContainerData`. Cerrado contra `UpdateFields.cpp`/`ItemPacketsCommon.cpp`: máscaras de bloques de 2 bits, `ArtifactPowers`, `Gems`, `ItemModList`, `ItemBonusKey`, `SpellCharges`, `Enchantment[13]`, `NumSlots` y `Slots[36]` en orden C++.
- [x] **#026g** `wow-packet`: serializer VALUES completo para `UF::UnitData::WriteUpdate`. Cerrado contra `UpdateFields.cpp`: máscaras de 8 bloques, dinámicos `StateWorldEffectIDs`/`PassiveSpells`/`WorldEffects`/`ChannelObjects`, `UnitChannel`, campos escalares, GUIDs, `VirtualItems`, `NpcFlags`, power/regen, stats, resistencias y costes se escriben en orden C++.
- [x] **#026h** `wow-packet`: serializer VALUES completo para `UF::PlayerData::WriteUpdate`. Cerrado contra `UpdateFields.cpp` y `MythicPlusPacketsCommon.cpp`: máscaras de 4 bloques, bit `IsQuestLogChangesMaskSkipped=false`, `Customizations`, `ArenaCooldowns`, `VisualItemReplacements`, campos escalares/GUIDs, `DungeonScoreSummary`, `PartyType`, `QuestLog`, `VisibleItems`, `AvgItemLevel` y `Field_3120` en orden C++.
- [x] **#026i** `wow-packet`: serializer VALUES completo para `UF::ActivePlayerData::WriteUpdate`, incluyendo máscaras de 48 bloques, `SkillInfo`, inventario/buyback, dinámicos de quests/titles/toys/transmog/traits, research y PVP info. Cerrado en `#026i5`; queda fuera de este ítem el bridge `wow-entities` para poblar/emitir los deltas reales.
- [x] **#026i1** `wow-packet`: `UF::ActivePlayerData::WriteUpdate` runtime/common path contrastado contra `UpdateFields.cpp`: cabecera de 48 bloques (`group0` u32 + `group1` 16 bits), `Coinage`, `InvSlots[141]`, `BuybackPrice/BuybackTimestamp`, parent 0 expertise, parent 38 stats, `SpellCritPercentage`/`ModDamageDonePos` y `CombatRatings` en orden C++.
- [x] **#026i2** `wow-packet`: nested `UF::SkillInfo::WriteUpdate` contrastado contra `UpdateFields.cpp`: máscara de 57 bloques (`group0` u32 + `group1` 25 bits), bloques activos y arrays `SkillLineID`, `SkillStep`, `SkillRank`, `SkillStartingRank`, `SkillMaxRank`, `SkillTempBonus`, `SkillPermBonus` en el loop C++ de 256 entradas.
- [x] **#026i3** `wow-packet`: nested simple writers de `ActivePlayerData` contrastados contra `UpdateFields.cpp`: `UF::Research::WriteUpdate`, `UF::RestInfo::WriteUpdate` y `UF::PVPInfo::WriteUpdate` con máscaras/flushes/orden de escalares C++.
- [x] **#026i4** `wow-packet`: nested dynamic writers de `ActivePlayerData` contrastados contra `UpdateFields.cpp`: `CharacterRestriction`, `SpellPctModByLabel`, `SpellFlatModByLabel`, `CategoryCooldownMod`, `WeeklySpellUse`, `CompletedProject`, `ResearchHistory`, `TraitEntry`, `TraitConfig`, `StablePetInfo` y `StableInfo` con máscaras dinámicas, strings y condicionales por `Type` en orden C++.
- [x] **#026i5** `wow-packet`: writer completo `ActivePlayerDataValuesUpdate` + `UpdateObject::full_active_player_values_update` contrastado contra `UF::ActivePlayerData::WriteUpdate`: orden global C++ de masks, dos fases de dynamic masks, parent-102/PetStable bit, arrays tardíos (`QuestCompleted`, glyphs) y `PvpInfo` final. Corregida divergencia detectada en writer runtime: `Coinage` ahora activa también parent bit `0` como C++.
- [x] **#026j** `wow-world`/`wow-packet`: bridge inicial `PlayerValuesUpdate` -> VALUES packet para `PlayerData` y `ActivePlayerData`. Cerrado contra el modelo C++ de bloque VALUES combinado por `changedObjectTypeMask`: `wow-packet` permite anexar `ActivePlayerData` dentro del bloque Player, y `wow-world::entity_update_bridge` copia máscaras/valores desde `wow-entities` sin añadir dependencia inversa. Queda pendiente `#026k`: usar este bridge en los callsites runtime y añadir Object/Unit/otros tipos.
- [x] **#026k** `wow-entities`/`wow-world`: integrar el bridge en callsites runtime de inventario/dinero/buyback sin perder `UnitData::VirtualItems`. Cerrado con `WorldSession::send_player_values_update_from_entity_bridge`: construye snapshot entity-side, aplica cambios/markers, emite `PlayerValuesUpdate` por el bridge, y reemplaza los callsites runtime que usaban `player_values_update`, `player_money_update` y `player_values_buyback_update`.
- [x] **#026l** `wow-world`: extender `entity_update_bridge` para traducir deltas entity-side de `Object`, `Unit`, `Item` y `Bag/Container` hacia los serializers de `wow-packet`. Contrastado contra los `BuildValuesUpdate*` C++ de `Object.cpp`, `Player.cpp`, `Item.cpp` y `Bag.cpp`: `ObjectData` viaja como prefijo opcional dentro de Unit/Item/Container/Player, y el `changedObjectTypeMask` preserva combinaciones Object+Unit, Object+Item y Object+Container. Queda pendiente conectar callsites runtime no-player y ampliar el bridge a GameObject/Corpse/DynamicObject/AreaTrigger/SceneObject/Conversation.
- [x] **#026m** `wow-world`: completar bridge entity-side para `GameObject`, `Corpse`, `DynamicObject`, `AreaTrigger`, `SceneObject` y `Conversation`. Contrastado contra `UpdateFields.cpp`/`BuildValuesUpdate*`: cada tipo conserva `ObjectData` opcional como prefijo y copia máscaras/campos en el orden C++; para nested simples que `wow-entities` aún no desglosa (`ScaleCurve`, `VisualAnim`, actores) el bridge emite máscara completa cuando el padre cambia, equivalente al path C++ con `ignoreNestedChangesMask=true`. Queda pendiente conectar callsites runtime no-player.
  - [x] **#026n** `wow-world`: el tick de combate de player contra criatura vuelve a emitir `SMSG_UPDATE_OBJECT` VALUES para `UnitData::Health` desde el bridge entity-side (`unit_values_update_to_update_object`) en vez de dejar el TODO desactivado. Contrastado contra C++ `Object::SendUpdateToPlayer`/`Object::BuildValuesUpdateBlockForPlayer` y `Unit::BuildValuesUpdate`: sólo se envía si el target está en `client_visible_guids_like_cpp` (`HaveAtClient`) y la criatura recién registrada limpia sus masks iniciales para que el delta runtime no arrastre campos de create.
  - [x] **#026o** `wow-world`: `apply_damage` de spell damage también emite delta VALUES de criatura desde el bridge entity-side tras `Creature::take_damage`, alineado con C++ `Unit::DealDamage -> ModifyHealth/SetHealth` que marca `UF::UnitData::Health`. Mantiene el orden de muerte/movement existente y sólo envía el delta si el target está visible para el cliente.
  - [x] **#026p** `wow-world`: `apply_heal` para criatura map-owned aplica `Unit::SetHealth`/delta VALUES tras cura, contrastado contra C++ `Unit::ModifyHealth`/`SetHealth`.
  - [x] **#026q** `wow-world`: `apply_heal` para self-heal de player actualiza el mirror de sesión, sincroniza el `Player` canónico si existe y emite delta `UnitData::Health` por el bridge, contrastado contra C++ `Unit::DealHeal -> ModifyHealth -> SetHealth`. La persistencia completa sigue bajo `Player::SaveToDB` (#019b.2/#025b/#028).
- [ ] **#027** `wow-entities`: `Unit` from `Entities/Unit/`: health, power, faction, flags, aura hooks, threat hooks. Refinado: base `Unit` state/setters cerrado en `#027a`, current spell slots en `#027b`, `SpellHistory` en `#027c`, `MotionMaster` en `#027d`, charm/control en `#027e`, threat/combat en `#027f`, aura hooks en `#027g` y movespline/AI bridge en `#027h`; los gaps restantes son de integración runtime con Spell/Aura, Movement, AI, Map y packets, no de estado estructural `wow-entities`.
- [x] **#027a** `wow-entities`: base `Unit` state from `Unit.*` and `UF::UnitData`: constructor state, movement update flag, death/unit state, health/max-health clamps, power index bridge, display/level/faction/reach fields and UnitData masks.
- [x] **#027b** `wow-entities`: `Unit` current spell slot lifecycle from `Unit.h`/`Unit.cpp`: `CURRENT_*` slot ids, `m_currentSpells` bridge, `SetCurrentCastSpell`, `InterruptSpell`, `FinishSpell`, `InterruptNonMeleeSpells`, `FindCurrentSpellBySpellId`, Auto Shot id `75`, delayed/instant/interruptible guards and channel `ALLOW_ACTIONS_DURING_CHANNEL` breakage. Full Spell cancel/finish packet side effects remain under the later Spell pipeline port, but the Unit-side ownership and interruption semantics are now represented and tested.
- [x] **#027c** `wow-entities`: `SpellHistory` structural state from `Spells/SpellHistory.*` and `Unit::_spellHistory`: spell/category cooldown entries, on-hold month marker, category index, update expiry, cooldown modification/reset, charge-category recharge queues, school lockouts, global cooldowns and before/after duel snapshot restoration. DB2/aura-derived duration calculation, packet emission and DB persistence remain explicit gaps for the later Spell pipeline and persistence passes.
- [x] **#027d** `wow-entities`: `MotionMaster` structural state from `MovementDefines.h`, `MovementGenerator.h` and `MotionMaster.*`: Trinity movement generator ids, default/active slots, mode/priority ordering, generator flags, target/id/duration metadata, base unit state reference counts, `MoveIdle`/point/follow/charge helpers, active clear/remove and `StopOnDeath` persist-on-death handling. Real path generation, spline launch callbacks, movement informs and map collision remain under the later movement runtime pass.
- [x] **#027e** `wow-entities`: `Unit` charm/control ownership state from `Unit.h`, `Unit.cpp`, `CharmInfo.h` and `SharedDefines.h`: owner/minion GUID bridge, `m_SummonSlot[MAX_SUMMON_SLOT]`, pet/totem summon slot constants, charmer/charmed/controlled GUID sets, `m_ControlledByPlayer`, `CharmType`, `CharmInfo` action-bar/charm-spell state, old faction/walk snapshot, direct mover pointers, shared vision helpers, `SetCharm` controller-side removal predicate and `SetCharmedBy`/`RemoveCharmedBy` target-side guard/state transitions. Aura packets, faction application, MotionMaster mutations, AI hooks and player possession packets remain explicit runtime gaps for the later spell/aura/movement/AI integration passes.
- [x] **#027f** `wow-entities`: `ThreatManager`/`CombatManager` structural state from `Combat/ThreatManager.*`, `Combat/CombatManager.*` and `Unit.*`: threat refs with base/temp amount, online/suppressed/offline and taunt/detaunt ordering, current/fixate victim selection with C++ 110%/130% shape, reverse `threatenedByMe` refs, threat update interval, PvE/PvP combat refs, PvP 5s timeout, suppression and all-combat clear helpers. Real `CanBeginCombat` world/map/phase/faction checks, aura-derived suppression/modifiers, redirect registry, packet emission, AI callbacks and bidirectional map-owned ref wiring remain under later combat/runtime integration passes.
- [x] **#027g** `wow-entities`: `Unit` aura hook state from `Unit.h`, `Unit.cpp`, `SharedDefines.h` and `SpellDefines.h`: owned/applied/visible/removed aura containers, visible update queue, interruptible aura list with interrupt masks, aura-state application including per-caster mask behavior, proc depth/chain counters and diminishing-return group/level/18s reset state. Real `Aura`/`AuraApplication` object lifetimes, `SpellInfo` stack/exclusive rules, aura effect handlers, criteria updates, interrupt-ignore rules, channel interruption, packet emission and spell script hooks remain explicit gaps for the later Spell/Aura pipeline.
- [x] **#027h** `wow-entities`: `Unit` movespline/AI bridge state from `Unit.*`, `MoveSpline.*`, `MoveSplineInit.*` and Unit AI stack helpers: movespline finalized/cyclic/on-transport flags, progress/duration/id, destination/velocity metadata, update/finalize/interrupt helpers, AI active/stack lock semantics, update tick bookkeeping and scheduled-change bridge around charm transitions. Real spline math/path generation, movement packet sync, `MovementInfo` flags, `UnitAI` trait object ownership and callbacks remain under Movement/AI runtime phases.
- [ ] **#028** `wow-entities`: `Player` from `Entities/Player/`: account/session link, inventory refs, quests, skills, taxi state. Refinado: base `Player` state/setters cerrado en `#028a`, base `Item` state cerrado en `#028b`, base `Bag` state cerrado en `#028c`, storage lookup cerrado en `#028d`, ObjectAccessor item branch cerrado en `#028e`, visible item state cerrado en `#028f`, InventoryType bridge cerrado en `#028g` y visible modifier helpers cerrado en `#028h`; siguen pendientes create/load/login, inventario real ownership, DB2 resolver stores, binding/equipment side effects, quests, skills, taxi, social, mail, group/guild, battleground and persistence.
- [x] **#028a** `wow-entities`: base `Player` state from `Player.*`, `StatSystem.cpp::Player::GetPowerIndex` and `UF::{PlayerData,ActivePlayerData}`: constructor type id/mask, session bridge, hit chance defaults, whisper accept permission branch, race/class/gender/native gender setters, selection target, flags, loot GUID, bank/backpack counts, XP, money clamp and PlayerData/ActivePlayerData masks.
- [x] **#028b** `wow-entities`: base `Item` state from `Item.*`, `ItemTemplate.h`, `ItemDefines.h`, `UF::ItemData` and `ItemPacketsCommon::ItemBonusKey`: constructor type id/mask, Object-only shape, create-state bridge, owner/contained/creator fields, slot/container/update/refund/text/trade state, dynamic item flags/flags2, stack/durability/expiration/context/appearance, spell charges, enchantments, item bonus key and ItemData masks.
- [x] **#028c** `wow-entities`: base `Bag` state from `Item/Container/Bag.*` and `UF::ContainerData`: `Item` retag to `TYPEID_CONTAINER`/`TYPEMASK_CONTAINER`, `MAX_BAG_SIZE`, bag slot GUID bridge, `NumSlots`, `Slots[36]`, template slot-count guard, StoreItem/RemoveItem child state updates and ContainerData masks.
- [x] **#028d** `wow-entities`: `Player` storage lookup bridge from `Player.h` and `Player.cpp`: `m_items[141]`, slot constants, `ForEachItem` locations, `GetItemByPos`, packed-pos lookup, `GetBagByPos`, `GetItemByGuid`, buyback slot rotation and `ActivePlayerData` InvSlots/Buyback masks; `PlayerStorage.cpp` is not present in this C++ checkout, storage lives in `Player.cpp`.
- [x] **#028e** `wow-entities`: `ObjectAccessor::GetObjectByTypeMask` item branch from `ObjectAccessor.cpp`: `TYPEMASK_ITEM` only resolves for player context and delegates to player inventory lookup; Rust exposes `AccessorObjectRef::Item` because `Item` is not a `WorldObject`.
- [x] **#028f** `wow-entities`: `Player` visible item slot state from `Player.cpp`, `Player.h` and `UF::{PlayerData,VisibleItem}`: `VisibleItems[19]`, `ItemID`, `ItemAppearanceModID`, `ItemVisual`, PlayerData array bits `61` and `62..80`, `SetVisibleItemSlot` clear/set behavior and equipment-slot `VisualizeItem` bridge. Template-derived item display values, BoE/BoA binding, real ownership side effects and nested packet serializers remain under `#028`/`#026`.
- [x] **#028g** `wow-data`/`wow-packet`/`wow-world`: InventoryType bridge from `DB2Structure.h::ItemEntry`, `ItemTemplate.h::InventoryType` and `Player.h` slot ranges: signed `int8 InventoryType` no longer wraps negative values to `255`, `INVTYPE_NON_EQUIP=0` maps to no slot, and `INVTYPE_BAG=18` maps to equipped bag slots `30..33`.
- [x] **#028h** `wow-entities`: `Item` visible modifier helpers from `Item.cpp`, `Item.h` and `ItemDefines.h`: item modifier storage, `AppearanceModifierSlotBySpec`, `IllusionModifierSlotBySpec`, `SecondaryAppearanceModifierSlotBySpec`, `GetVisibleEntry`, `GetVisibleAppearanceModId`, `GetVisibleEnchantmentId`, `GetVisibleItemVisual` and secondary appearance precedence. DB2 resolver stores remain explicit bridges until `wow-data` ports `ItemModifiedAppearance` and `SpellItemEnchantment`.
- [ ] **#029** `wow-entities`: `Creature` + `GameObject` from their C++ dirs: template refs, spawn data, respawn timer, AI ref, GO state. Refinado: base `Creature` state cerrado en `#029a` y base `GameObject` state cerrado en `#029b`; siguen pendientes `Creature::Create/LoadFromDB`, template/difficulty refs, AI ownership, loot, corpse/respawn lifecycle and GameObject create/template/model/use lifecycle.
- [x] **#029a** `wow-entities`: base `Creature` state from `Creature.*`, `CreatureData.h`, `UnitDefines.h`, `MovementDefines.h`, `SharedDefines.h`, `World.cpp` config defaults and `StatSystem.cpp::Creature::GetPowerIndex`: constructor defaults, respawn/corpse timers, react state, movement type, spells, loot mode, monster sight default, display/model dimension bridge, faction setter and creature power-index semantics.
- [x] **#029b** `wow-entities`: base `GameObject` state from `GameObject.*`, `SharedDefines.h` and `UF::GameObjectData`: constructor type id/mask, stationary/rotation create flags, respawn/despawn/restock/cooldown state, loot state/unit guid, spawned-by-default, spell/spawn ids, packed rotation, loot mode, stationary position, respawn compatibility flag and GameObjectData setters/masks.
- [x] **#029c** `wow-entities`: base `Corpse` state from `Corpse.*`, `SharedDefines.h` and `UF::CorpseData`: constructor type id/mask, `WorldObject(type != CORPSE_BONES)`, stationary flag, ghost time/type/cell bridge, dynamic flags, owner/party/guild, display/race/class/sex/flags/faction/item setters, corpse expiry thresholds and CorpseData masks.
- [ ] **#030** `wow-entities`: remaining map-stored object types: `DynamicObject`, `AreaTrigger`, `Pet`, `Transport`, `Vehicle`, `SceneObject`, `Conversation`, `Totem`; mark post-WoLK-only behavior explicitly when C++ has stubs. Refinado: base `DynamicObject` state cerrado en `#030a`, base `AreaTrigger` state cerrado en `#030b`, base `SceneObject` state cerrado en `#030c`, base `Conversation` state cerrado en `#030d`, base `Totem` state cerrado en `#030e`, base `Pet` state cerrado en `#030f`, base `Vehicle` kit cerrado en `#030g` y base `Transport` state cerrado en `#030h`; siguen pendientes DynamicObject create/add-to-map/update runtime, AreaTrigger templates/create/load/update/search/actions/AI, SceneObject create/map/aura removal, ConversationDataStore/Start/runtime, TempSummon/Minion/Pet runtime, Vehicle auras/events/accessories/immunities, TransportMgr/path runtime/static passenger spawning/teleport/update integration, Aura/Spell ownership, caster registration, farsight viewpoint and transport/map relocation.
- [x] **#030a** `wow-entities`: base `DynamicObject` state from `DynamicObject.*` and `UF::DynamicObjectData`: constructor type id/mask, `WorldObject(isWorldObject)`, stationary flag, duration/aura/caster/viewpoint bridge state, dynamic-object type enum, caster/spell visual/spell id/radius/cast-time setters and DynamicObjectData masks.
- [x] **#030b** `wow-entities`: base `AreaTrigger` state from `AreaTrigger.*`, `AreaTriggerTemplate.h` and `UF::AreaTriggerData`: constructor type id/mask, `WorldObject(false)`, stationary/area-trigger create flags, spawn/target/aura/stationary-position/duration/time/removal/movement/template bridge state, duration semantics for permanent triggers, scalar AreaTriggerData setters, basic scale-curve constants and VisualAnim mask bridge.
- [x] **#030c** `wow-entities`: base `SceneObject` state from `SceneObject.*` and `UF::SceneObjectData`: constructor type id/mask, `WorldObject(false)`, stationary/scene-object create flags, stationary position, created-by-spell-cast bridge, removal predicate shape and SceneObjectData script package/random seed/created-by/type masks.
- [x] **#030d** `wow-entities`: base `Conversation` state from `Conversation.*`, `ConversationDataStore.h` and `UF::ConversationData`: constructor type id/mask, `WorldObject(false)`, stationary/conversation create flags, creator/duration/texture/stationary-position state, line start/end time bridges, actor/line data, 10s despawn delay and ConversationData masks.
- [x] **#030e** `wow-entities`: base `Totem` state from `Totem.*`, `TemporarySummon.*`, `Unit.h` and `SharedDefines.h`: `Creature/Minion` type shape, `UNIT_MASK_SUMMON|MINION|TOTEM`, owner/summoner bridge, totem duration/type, inherited spell slots, init-summon passive/secondary spell rules, update/unsummon duration shape, totem-created packet slot offset and immunity predicate special cases.
- [x] **#030f** `wow-entities`: base `Pet` state from `Pet.*`, `PetDefines.h`, `TemporarySummon.*`, `UnitDefines.h` and `CreatureData.h`: `Guardian`/`Creature` world-object shape, unit type masks including hunter/controlable guardian, owner/type/duration/loading/removed/focus/group/specialization state, pet spell/autospell maps, stable slot helpers/load selection priority and pet XP factor.
- [x] **#030g** `wow-entities`: base `Vehicle` kit from `Vehicle.*` and `VehicleDefines.h`: base unit GUID/type/position bridge, vehicle id/creature entry, status machine, seats/passenger info/addons/accessories/template structs, usable/available seat counting, pending join-event bridge, passenger add/remove/remove-all and `TransportBase` passenger position/offset formulas.
- [x] **#030h** `wow-entities`: base `Transport` state from `Transport.*`, `TransportMgr.h`, `GameObject.*` and `SharedDefines.h`: `GameObject` shape with `SERVER_TIME|STATIONARY|ROTATION`, map-object-transport type, movement state, path leg/event/template structs, period/timer/path-progress client dynamic-flag encoding, dynamic/static passenger GUID sets, cleanup/unload shape, movement stop request bridge and `TransportBase` passenger position/offset formulas.
- [x] **#031** `wow-world`/`wow-entities`: mover el runtime de criaturas usado por sesión a `WorldCreature`/`Creature` map-owned, contrastado contra C++ `Creature::Update`, `Creature::AIM_Create/AIM_Initialize` y `Unit::AIUpdateTick`. `WorldSession` deja de usar `HashMap<Guid, wow_ai::CreatureAI>` en runtime; combate, aggro, loot/gossip/vendor/taxi, respawn/corpse y visibilidad mutan o consultan el `MapManager`. Metadatos de loot/boss que estaban en `CreatureAI` pasan a `CreatureAiOwnershipState`. Queda solo un puente `cfg(test)` para tests legacy hasta limpiar fixtures.
- [x] **#032** Refactor `WorldSession` para tener player entity handle/controlador en vez de campos sueltos.
  - [x] **#032a** Crear `SessionPlayerController` como equivalente incremental de C++ `WorldSession::_player`/`SetPlayer`/`GetPlayer`: GUID, nombre, mapa, posición, raza, clase, nivel y sexo quedan sincronizados desde login/movement/teleport/logout, y los registros compartidos (`PlayerRegistry`/`ObjectAccessor`) leen por el controlador cuando existe.
  - [x] **#032b** Migrar runtime de jugador al controlador/player entity.
    - [x] **#032b1** Dinero, XP/NextLevelXP, selección, spells y `_currencyStorage` quedan reflejados en `SessionPlayerController` con getters/setters tipo C++ `Player::{GetMoney,SetMoney,GetXP,SetXP,SetSelection,GetSpellMap,_currencyStorage}`; login/trainer/loot/quest/registry usan la ruta canónica donde ya aplica.
    - [x] **#032b2** Migrar `m_items`/buyback/inventory item objects al controlador/player entity sin depender de `WorldSession` como owner de inventario.
      - [x] **#032b2a** Crear `SessionPlayerInventoryRuntime` dentro del controlador y alimentar desde login/logout/clear; snapshots de valores, `PlayerRegistry`, `ObjectAccessor`, `GetItemByPos` representado y planes directos de store/unequip leen por esta ruta canónica cuando existe.
      - [x] **#032b2b** Cambiar mutaciones directas de `character`/`loot`/`spell` sobre `inventory_items`, `buyback_items` e `inventory_item_objects` a mutadores canónicos sincronizados con `SessionPlayerInventoryRuntime`, contrastado contra C++ `Player::_StoreItem`, `QuickEquipItem`, `AddItemToBuyBackSlot` y `RemoveItemFromBuyBackSlot`.
      - [x] **#032b2c** Invertir el owner efectivo: los mutadores operan sobre `SessionPlayerInventoryRuntime` cuando existe controlador y los campos heredados de `WorldSession` quedan como espejo/compatibilidad hasta su retirada en `#032d`.
  - [x] **#032c** Cambiar handlers directos (`character`, `loot`, `trainer`, `spell`, `quest`, `chat`, `group`) a getters/mutators del controlador, contrastando cada bloque contra `Player`/`WorldSession` C++.
    - [x] **#032c1** `chat`/`group`: reemplazar lecturas directas de GUID/nombre/mapa/posición por getters del controlador, equivalente a C++ `WorldSession::GetPlayer()->GetGUID()/GetName()/GetMapId()/GetPosition()`.
    - [x] **#032c2** `quest`/`spell`: reemplazar raza/clase/nivel/género/spells/inventario por getters canónicos y cubrir condiciones/quest availability.
    - [x] **#032c3** `loot`: reemplazar GUID/mapa/posición/nivel/clase/inventario por getters canónicos en roll/master-loot/store paths.
    - [x] **#032c4** `character`/`trainer`: reemplazar gold/currency/player metadata/inventory directos por mutadores/getters canónicos y cerrar residuos de login/logout.
  - [x] **#032d** Retirar o hacer privados los campos heredados cuando dejen de ser fuente runtime, dejando solo puentes `cfg(test)` si aún son necesarios.
    - [x] **#032d1** Cerrar residuos fuera de la lista original (`combat`, `social`, `misc`, `spell`, `loot`) que todavía leían GUID/posición/item runtime directamente en vez de C++ `GetPlayer()`/`GetItemByGuid()`.
    - [x] **#032d2** Encapsular accesos internos de `session.rs` que todavía usan campos legacy como fallback/runtime y separar claramente bootstrap/test de runtime canónico.
    - [x] **#032d3** Reducir visibilidad de campos heredados o moverlos detrás de helpers `cfg(test)` cuando ya no haya consumidores productivos directos: los campos legacy de player runtime quedan privados en `WorldSession`; login/bootstrap y loot spec pasan por helpers C++-like, y los tests de handlers ya no escriben/leen esos campos directamente.

> Tras cerrar #032, el roadmap continúa con Fase 2 (Movement) y siguientes según la sección 4.

---

## 6. Criterios de "done" por fase

Una fase se considera cerrada cuando:

1. **Todos los TODO de la fase marcados `[x]`**.
2. **`cargo check --workspace` 0 errores**, sin warnings nuevos.
3. **`cargo test --workspace` todos los tests verdes**, incluyendo nuevos tests de la fase.
4. **Tests de regresión runtime**: el server arranca, login OK, un personaje entra al mundo y puede moverse + combatir + alguna mecánica de la fase recién implementada.
5. **Documentación actualizada**: este `MIGRATION_ROADMAP.md` con la sección 3 (matriz) actualizada al nuevo % migrado, y `CLAUDE.md` con cualquier nueva convención.
6. **Sin `// TODO` ni `unimplemented!()` ni `todo!()` en el código de la fase** (excepto claramente marcados como pendientes de la siguiente fase).
7. **Commit limpio en `main`** (no en rama feature, dado que trabajamos en solitario — ver ADR sobre solo-developer workflow).

---

## 7. Riesgos y mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|---|---|---|---|
| Re-introducir el bug del bridge fallido (improvisar contra structs imaginarios) | Media | Alto | Memory `feedback_always_read_cpp.md`. Antes de cada implementación, leer el `.cpp` correspondiente. Citar línea en commit. |
| Confiar en docs/agentes previos como si fueran C++ | Alta | Alto | Los docs son índice, no oracle. Cada task requiere contraste directo con C++ y, si toca wire/runtime, test específico. |
| **Lo "✅ done" actual tiene bugs/divergencias vs C++ que no hemos detectado** | Alta | Alto | Fase A (auditoría obligatoria por módulo) antes de extender. Tabla de divergencias en `docs/audits/<modulo>.md`. Hasta que un módulo no esté auditado, su columna "Auditado vs C++" sigue ❌ y se trata con sospecha. |
| Auditar todo costaría tanto como reescribirlo | Media | Medio | Las auditorías se priorizan: módulos críticos (network, crypto, packets, maps) primero; los de menor superficie y baja prioridad pueden auditarse "just-in-time" antes de extender. |
| Scope creep entre fases (querer hacer L5 antes de L3 estable) | Alta | Alto | Esta hoja de ruta es vinculante. No se salta orden sin acuerdo explícito. |
| Implementación parcial que parezca completa (ej. spell engine que solo cubre 5 efectos) | Media | Medio | Tests por feature concreta. Marcar ⚠️ en lugar de ✅ hasta cobertura ≥ 90%. |
| Acoplamiento accidental entre crates (wow-map dependiendo de wow-world) | Baja | Alto | Disciplina de capas. wow-map no conoce sesiones, solo entidades. |
| Pathfinding (Detour) incompleto bloquea AI | Media | Medio | Hacer movement waypoint sin pathfinding primero; Detour es Fase 2.2. |
| `scripts/` (3000 archivos) bloquea cualquier contenido scripteado | Alta | Alto | Aceptar que la mayoría de bosses/instancias no funcionan hasta Fase 8. Priorizar SmartAI (data-driven) que cubre ~50% sin scripting. |
| Performance: `Arc<RwLock<MapManager>>` global serializa todo | Alta | Alto | Resolver en Fase 0.6 (MapUpdater pool). Si no resuelve, considerar one-Arc-per-Map en lugar de un Arc global. |
| Implementar spawn loading con SQL directo por celda y saltarse `ObjectMgr`/`SpawnData` | Media | Alto | Fase 0 ahora separa spawn stores (#008-#009) de `ObjectGridLoader` (#010), igual que C++ preclasifica GUIDs por map/difficulty/cell. |
| Tests de regresión runtime cuestan tiempo | Media | Bajo | Aceptar y planificar — son los que de verdad demuestran "done". |
| El cliente WoLK 3.4.3 hace cosas no documentadas | Media | Medio | El C++ TrinityCore es la fuente de verdad. Si no aclara, capturar paquetes con `wow-data/pcap` (pendiente). |

---

## 8. Decisiones de arquitectura (ADRs)

### ADR-001: Solo-developer workflow

Trabajamos directamente sobre `main`. **No PRs** (no hay reviewer). Cada commit debe pasar `cargo check + test` antes de pushear. Ramas feature solo para experimentos arriesgados.

### ADR-002: Capas estrictas de crates

`wow-map` no conoce `wow-world::WorldSession`. Las dependencias solo van hacia abajo. Si un crate de capa N necesita algo de capa N+1, se mueve a un trait en capa N o se reorganiza.

### ADR-003: Tests por feature, no por línea

Los tests deben demostrar invariantes de TrinityCore (ej. "un grid en estado Idle pasa a Removal después de 60s sin actividad"), no porcentaje de cobertura.

### ADR-004: Comentarios `// C++ ref:`

Cuando una función traduce código C++, citar archivo y línea: `// C++ ref: Map.cpp:441 (AddPlayerToMap, ASSERT player->GetMap() == this)`. Facilita revisar la migración.

### ADR-005: Cero `unsafe` salvo FFI

Solo `unsafe` permitido en crates de FFI (`wow-recastdetour`). Aislar y documentar.

### ADR-006: SQL prepared statements en `wow-database/statements/`

No SQL inline en handlers. Toda query como `StatementDef` registrado. Facilita auditoría y prevención de inyección.

### ADR-007: Auditoría obligatoria antes de extender

Ningún módulo se considera "trustworthy" hasta tener auditoría vs C++ documentada en `docs/audits/<modulo>.md`. Antes de añadir features a un módulo, ejecutar (o verificar que existe) la auditoría correspondiente. Lo "✅ done" sin auditar es deuda técnica latente.

Las auditorías son commits `docs(audit): ...` separados; no se mezclan con código nuevo.

---

## 9. Glosario rápido

- **NGrid** — el contenedor de 8×8 cells. 64×64 NGrids forman un Map.
- **Cell** — la unidad de visibilidad/carga. ~66 yardas. Granularidad para spawn de mobs.
- **Active object** — entidad que mantiene grids cargados (player, criatura en combate, summons activos).
- **Visibility range** — distancia máxima a la que el cliente ve entidades (~100 yardas en WoLK).
- **PhaseMask** — bitmask de fases; un objeto solo es visible si su phase ∩ player phase ≠ 0.
- **Hotfix** — cambio de DB2 aplicado en runtime sin reinicio (TrinityCore: `hotfix_data` table).

---

## 10. Histórico de cambios al roadmap

| Fecha | Cambio | Commit |
|---|---|---|
| 2026-05-01 | Creación inicial del documento | (este commit) |
| 2026-05-01 | Añadido Fase A (auditoría obligatoria), columna "Auditado vs C++" en matriz, ADR-007, riesgo "lo existente puede tener bugs" | (este commit) |
| 2026-05-07 | Revisión manual del plan contra el árbol C++: `_INDEX.md` pasa a ser inventario de estado, Fase 0 se ajusta a `NGrid.h`/`GridStates.cpp`/`ObjectGridLoader.cpp`/`SpawnData.h`, Fase 1 adelanta `ObjectAccessor` y UpdateFields, Fase 8 separa `scripts/Commands` del contenido masivo. | pendiente |
| 2026-05-07 | Añadida Fase R de refinamiento WBS completo antes de continuar implementación. | pendiente |
| 2026-05-07 | Cerrada R6: la siguiente mini-fase implementable es `#NEXT.L0.CONFIG.001` antes de reanudar Maps/L3. | pendiente |

---

*Actualizar este archivo al cerrar cada fase. Sin documento actualizado, no se considera la fase cerrada.*
