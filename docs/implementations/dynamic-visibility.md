# Dynamic Visibility System

**Fecha de implementación:** 2026-02-24  
**Estado:** ✅ Completado  
**Versión:** 1.0  

---

## Overview

Sistema de visibilidad dinámica que actualiza automáticamente los objetos (criaturas y gameobjects) visibles para el jugador cuando se mueve, enviando `CreateObject2` para objetos que entran en rango y `OutOfRange` para objetos que salen.

En C# TrinityCore esto se maneja a través de `Map.UpdateVisibilityForPlayer()` y el sistema de grids/celdas.

---

## C# Reference

### Archivos relevantes:
- `Source/Game/Entities/Player/Player.cs`: `SendInitialPacketsAfterAddToMap()` → `UpdateVisibilityForPlayer()`
- `Source/Game/Maps/Map.cs`: `UpdateVisibilityForPlayer()`, sistema de grids
- `Source/Game/Networking/Packets/UpdatePackets.cs`: `UpdateObject` con `OutOfRangeGUIDs`

### Comportamiento en C#:
1. El jugador tiene un rango de visibilidad (~533 yardas por defecto)
2. El mapa está dividido en grids de 533×533 yardas
3. Cuando el jugador se mueve, `Map.UpdateVisibilityForPlayer()` recalcula qué grids están dentro del rango
4. Para cada objeto que entra en rango: `SMSG_UPDATE_OBJECT` (CreateObject2)
5. Para cada objeto que sale de rango: `SMSG_UPDATE_OBJECT` con `OutOfRangeGUIDs`
6. Los objetos fuera de rango permanecen en el mundo pero no se envían al cliente

---

## Implementación Rust

### Solución simplificada:
Dado que aún no tenemos `MapManager` con grids, implementamos un sistema basado en distancia euclidiana con threshold y queries de base de datos.

### Archivos modificados/creados:

#### 1. `/home/server/woltk-server-core/rustycore/crates/wow-world/src/handlers/character.rs`
- **Función:** `update_visibility(&mut self) -> async`
- **Lógica:**
  - Compara posición actual con `last_visibility_pos`
  - Si distancia > 50 yardas, ejecuta query de criaturas y GOs en radio ±800
  - Calcula diferencia entre conjuntos visible/anterior
  - Para nuevos: `CreateObject2` + registro en `CreatureAI`
  - Para desaparecidos: `OutOfRange` + remoción de `CreatureAI`
- **Threshold:** 50 yardas para evitar spam de queries
- **Radio:** ±800 yardas (configurable)

#### 2. `/home/server/woltk-server-core/rustycore/crates/wow-world/src/session.rs`
- **Campos añadidos:**
  - `visible_creatures: HashSet<ObjectGuid>`
  - `visible_gameobjects: HashSet<ObjectGuid>`
  - `last_visibility_pos: Option<Position>`
- **Inicialización:** en `WorldSession::new()`

#### 3. `/home/server/woltk-server-core/rustycore/crates/wow-world/src/handlers/movement.rs`
- **Hook:** En `handle_movement()` después de actualizar `player_position`
- **Llamada:** `self.update_visibility().await`

#### 4. `/home/server/woltk-server-core/rustycore/crates/wow-packet/src/packets/update.rs`
- **Constructor añadido:** `UpdateObject::out_of_range_objects()`
- **Corrección:** `destroy_objects()` ahora usa `num_updates: 0` (era bug)
- **Formato:** `has_destroy_or_oor` bit → `destroy_guids` + `out_of_range_guids`

#### 5. `/home/server/woltk-server-core/rustycore/crates/wow-database/src/statements/world.rs`
- **Query mejorado:** `SEL_VENDOR_ITEMS` con JOIN a `hotfixes.item_sparse`
- **Radios:** Query usa `BETWEEN ? AND ?` en X e Y

---

## Packet Flow

### Objetos nuevos (entran en rango):
```
update_visibility() → query DB → UpdateObject::create_creatures() → send_packet()
```

### Objetos que salen de rango:
```
update_visibility() → diff calculado → UpdateObject::out_of_range_objects() → send_packet()
```

### Estructura OutOfRange en C#:
```csharp
// UpdateData.cs
public void AddOutOfRangeGUID(ObjectGuid guid) {
    outOfRangeGUIDs.Add(guid);
}

// BuildPacket → WriteBit(!outOfRangeGUIDs.Empty() || !destroyGUIDs.Empty())
// Si true: WriteUInt16(destroyGUIDs.Count), WriteInt32(destroyGUIDs.Count + outOfRangeGUIDs.Count)
// Luego packed GUIDs de destroy + outOfRange
```

---

## Dependencias

1. **MapManager/Grids:** ❌ No implementado aún (usamos query DB)
2. **Shared world state:** ❌ Cada sesión tiene su propio tracking (no comparten)
3. **Broadcast multi-player:** ❌ Solo afecta al jugador actual
4. **Performance:** Query DB cada ~50 yardas de movimiento

---

## Testing Notes

✅ **Funcional:**
- Criaturas aparecen al acercarse dentro de 800 yardas
- Criaturas desaparecen al alejarse >800 yardas
- GameObjects siguen misma lógica
- Threshold de 50y evita spam de queries
- Logs muestran conteos de nuevos/removidos

⚠️ **Limitaciones actuales:**
- No hay sistema de grids → query DB costoso con muchos jugadores
- No hay broadcast entre jugadores (cada uno calcula su visibilidad)
- Radio fijo de 800y (en C# es por grid y configuración por mapa)

---

## Next Steps / To Do

1. **[PRIORIDAD]** Implementar `MapManager` con grid system
2. Reemplazar query DB por cache de objetos por grid
3. Añadir `UpdateObject` broadcast para multi-player
4. Configurar radios diferentes por tipo de objeto (criaturas vs GOs vs jugadores)
5. Optimizar: caché de queries, reutilizar resultados si el jugador se mueve poco

---

## Referencias

- `Source/Game/Maps/Map.cs`: `UpdateVisibilityForPlayer()`
- `Source/Game/Entities/Object/Update/UpdateData.cs`: `AddOutOfRangeGUID()`
- `Source/Game/Networking/Packets/UpdatePackets.cs`: `UpdateObject.Write()`

---

**Última revisión:** 2026-02-24  
**Responsable:** @WoWServer  
**Issue relacionada:** #visibility-system (no tracking aún)
