# `_attic/` — archivo de la migración fallida MapManager

## Qué es esto

Aquí viven los restos del intento de migración hacia el `MapManager` global (commits WIP `b6f5e4cad`, `cd2fa6a7f`, `f83c48d82`). El intento dejó el árbol con 176 errores de compilación porque el "puente" entre `WorldSession` y `MapManager` se escribió contra structs que ya no existían.

Este directorio **no se compila** (los `.rs` fueron renombrados a `.rs.txt`). Es
historia de un intento fallido, no un catálogo operativo de TODOs ni evidencia
de que falte el código descrito. Los tamaños, tipos, rutas y estados del inventario
siguiente pertenecen a aquel intento. `WorldSession.creatures` ya no es la
realidad actual; no reconstruir ese diseño a partir de estos apuntes.

## Cómo consultar el archivo histórico

Empezar por [STATE.md](../../../docs/migration/STATE.md), el issue/checkpoint
activo y [los criterios de módulos](../../../docs/architecture/module-design-guidelines.md).
Consultar un fragmento sólo para una pregunta histórica concreta; contrastar C++
y los lectores/escritores actuales antes de reutilizar una idea. No ejecutar los
scripts archivados ni mover implementaciones o stubs enteros al árbol activo.

Los `.md` y `.sh` conservaron su extensión original. Los scripts siguen siendo
ejecutables potencialmente mutantes, no documentación segura para ejecutar.

---

## Inventario histórico — afirmaciones del intento fallido, no tareas vigentes

### Bridge `WorldSession` ↔ `MapManager` (roto)

| Archivo | Qué intentaba ser | TODO real |
|---|---|---|
| `session_map_integration.rs.txt` | 20+ métodos bridge sobre `WorldSession` para acceder al `MapManager` global. Usaba `CreatureCreateData` con campos inventados (`entry_id`, `faction`, `position`, `current_hp`/`max_hp`) que **nunca existieron** en el packet real. | Rehacer los wrappers usando los nombres correctos: `entry`, `faction_template`, `health`/`max_health`. Posición va separada del struct, no dentro. Ver `crates/wow-packet/src/packets/update.rs` línea ~XXX para la verdad de `CreatureCreateData`. |
| `migrated_session_methods.rs.txt` | Reescritura de `tick_creatures_sync()`, `tick_combat_sync()`, `handle_loot_unit()`, `handle_loot_release()` apoyándose en el bridge anterior. | Cuando el bridge funcione, migrar uno a uno cada `tick_*`/`handle_*` actual de `session.rs` para usar `MapManager` global en vez del `HashMap` local de `WorldSession.creatures`. |
| `creature_integration.rs.txt` (347 líneas) | Capa intermedia para sincronizar `CreatureAI` ↔ `WorldCreature`. | El diseño actual ya tiene `WorldCreature` en `map_manager.rs`. Cuando se migre, `CreatureAI` desaparece o se reduce a estado de IA puro y la posición/HP viven solo en `WorldCreature`. |
| `character_migration.rs.txt` (391 líneas) | Reescritura paralela de `tick_creatures_sync()`, `send_nearby_creatures()`, `process_respawn_queue()` apuntando al `MapManager` global. | Mismo TODO que `migrated_session_methods.rs.txt`. Aquí hay más detalle de la lógica de respawn — útil consultarlo cuando se haga la migración. |
| `handlers_migration.rs.txt` (216 líneas) | Patrones de migración para los handlers de combat/loot/movement. | Referencia. La idea correcta está en los handlers actuales `combat.rs`, `loot.rs`, `trainer.rs` que ya pasaron a usar `self.get_creature(&guid)` (los wrappers de `map_helpers.rs`). |

### Stubs de compatibilidad (parche que tapaba huecos del bridge)

| Archivo | Contenido |
|---|---|
| `stubs.rs.txt` | **44 funciones `handle_*_stub()`** que no hacían nada — existían solo para que el dispatcher no rompiera mientras los handlers reales estaban a medias. Catálogo útil de qué handlers necesita el dispatcher; ver lista abajo. |
| `temp_creature_storage.rs.txt` | Almacén temporal etiquetado "temporal" en su propio nombre. |
| `quick_compat.rs.txt`, `quick_handlers.rs.txt` | Pegamento ad-hoc. |

#### Stubs catalogados (`stubs.rs.txt`) — qué handlers necesita el server

Login / personajes:
- `handle_continue_player_login`, `handle_enum_characters`, `handle_create_character`, `handle_char_delete`, `handle_player_login`, `handle_connect_to_failed`, `handle_get_undelete_cooldown_status`

Battlenet / sistema:
- `handle_battlenet_request`, `handle_server_time_offset_request`, `handle_request_played_time`, `handle_ping`, `handle_db_query_bulk`, `handle_hotfix_request`, `handle_time_sync_response`

Sesión / mapa:
- `handle_set_selection`, `handle_area_trigger`, `handle_request_cemetery_list`, `handle_taxi_node_status_query`, `handle_chat_join_channel`, `handle_move_time_skipped`, `handle_logout_request`, `handle_logout_cancel`

Queries:
- `handle_query_creature`, `handle_query_game_object`, `handle_query_player_names`, `handle_query_realm_name`, `handle_query_npc_text`

NPC interactions:
- `handle_gossip_hello`, `handle_gossip_select_option`, `handle_auction_hello_request`, `handle_banker_activate`, `handle_binder_activate`, `handle_tabard_vendor_activate`, `handle_spirit_healer_activate`, `handle_repair_item`, `handle_request_stabled_pets`, `handle_list_inventory`, `handle_trainer_list`

Inventory / vendor:
- `handle_buy_item`, `handle_sell_item`, `handle_swap_inv_item`, `handle_auto_equip_item`

Quests:
- `handle_quest_giver_hello`, `handle_quest_giver_status_query`, `handle_quest_giver_status_multiple_query`

> Cuando se vaya a implementar uno de estos en serio, su sitio natural es `crates/wow-world/src/handlers/<dominio>.rs` (character / chat / movement / combat / loot / trainer / misc).

### Refactor a trait `CharacterHandlers` (incompleto)

| Archivo | Qué pasó |
|---|---|
| `character_reduced.rs.txt` | El `character.rs` original (4627 líneas, 44 funciones) se redujo a 120 líneas dejando solo `default_display_id`. |
| `character_impl.rs.txt` | Extracción de 14 funciones del monolito a un trait-impl. |
| `character_trait.rs.txt` | Definición del trait `CharacterHandlers`. |
| `character_trait_impl.rs.txt` | Impl block del trait — pero referenciaba ~30 handlers que no llegaron a moverse (de ahí los 49 errores de compilación). |

**Estado actual**: descartado. El monolito completo de 4627 líneas se restauró desde `a992c2d56`. Si en el futuro se quiere trocear `character.rs`, este intento sirve de mapa de qué se pensaba extraer y por dónde se atascó.

### Documentos de proceso

| Archivo | Contenido |
|---|---|
| `MIGRATION_GUIDE.md` | Patrones de migración (`self.creatures.get()` → `self.get_creature_global()`, etc). Algo del contenido sigue siendo útil. |
| `STATUS_MIGRATION.md` | Snapshot del estado de migración en abril 2026. Histórico. |
| `FASE2_SUMMARY.md` | Plan de la "Fase 2". Histórico. |
| `migrate_character.sh`, `MIGRATE_character.sh` | Scripts de migración no usados. |

---

## Reanudación actual

La unidad aprobada es la operación/responsabilidad completa dentro del macro activo,
no una PR o aprobación por método. El [checkpoint #578](../../../docs/architecture/session-578-checkpoint.md)
define ownership, eliminación de puentes y aceptación; el archivo histórico no
elige almacenamiento, API ni un orden alternativo de migración.
