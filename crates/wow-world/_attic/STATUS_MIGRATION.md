# Estado de Migración - MapManager Global

> Snapshot histórico de abril de 2026, no estado actual ni instrucciones para
> continuar. La fecha literal y los resultados siguientes no constituyen evidencia
> vigente. Consultar [el índice del archivo](README.md) y
> [STATE.md](../../../docs/migration/STATE.md); no ejecutar sus comandos heredados.

## Última actualización: $(date)

### ✅ Completado
1. **MapManager Global** - Estructura completa con:
   - Grid system 64x64
   - Creaturas compartidas entre jugadores
   - Visibilidad 3x3 grids
   - Tests pasando (10 tests)

2. **Campos temporales agregados** a WorldSession:
   - `creatures: HashMap<ObjectGuid, CreatureAI>`
   - `visible_creatures: HashSet<ObjectGuid>`
   - Esto permite que compile mientras migramos

### 🔄 En Progreso
1. **Migración de handlers** - Necesita:
   - Corregir llamadas a `UpdateObject::create_creature_block()` (10 args, no 2)
   - Corregir llamadas a `UpdateObject::create_creatures()` (1 arg Vec, no 2)
   - Corregir llamadas a `UpdateObject::create_gameobject_block()` (6 args, no 1)
   - Corregir llamadas a `UpdateObject::out_of_range_objects()` (1 arg, no 2)

### 📋 Pendiente Mañana
1. **Corregir firmas en character.rs**:
   - Línea 1904: `create_creature_block` → usar 10 argumentos correctos
   - Línea 1940: `create_creatures` → pasar Vec<CreatureCreateData>
   - Línea 2072: `create_creature_block` → mismo que arriba
   - Línea 2098: `create_creatures` → mismo que arriba
   - Línea 2105: `out_of_range_objects` → pasar solo Vec<ObjectGuid>
   - Línea 2177: `create_gameobject_block` → usar 6 argumentos
   - Línea 2191: `create_world_objects` → revisar firma
   - Línea 2195: `out_of_range_objects` → mismo que arriba
   - Línea 2549: `create_gameobject_block` → mismo que arriba

2. **Eliminar campos temporales** una vez migrado todo:
   - Remover `creatures` y `visible_creatures` de WorldSession
   - Usar solo MapManager global

### 🎯 Comandos para continuar
```bash
# Ver errores actuales
cargo check -p wow-world 2>&1 | grep "^error\["

# Corregir automáticamente lo posible
cargo fix --lib -p wow-world --allow-dirty

# Verificar tests
cargo test -p wow-world --lib
```

### 📁 Archivos creados/modificados
- `map_manager.rs` - Nuevo sistema global ✅
- `session_map_integration.rs` - Bridge session↔map ✅
- `temp_creature_storage.rs` - Compatibilidad temporal ✅
- `session.rs` - Agregados campos temporales ✅
- `character.rs` - PENDIENTE: corregir llamadas
- `trainer.rs` - PENDIENTE: corregir llamadas
- `misc.rs` - PENDIENTE: corregir llamadas
- `lib.rs` - Actualizado con nuevos módulos ✅
