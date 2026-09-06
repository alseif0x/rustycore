# Guía de Migración Completa - Fase 2

> Archivo histórico del intento fallido de abril de 2026. Los cambios y comandos
> siguientes no son instrucciones vigentes y no deben ejecutarse como plan de
> migración. Consultar [el índice del archivo](README.md) y
> [STATE.md](../../../docs/migration/STATE.md) para la realidad actual.

## Resumen

Esta guía describe cómo migrar completamente de `self.creatures: HashMap<>` local al `MapManager` global.

## Archivos Creados

1. **session_map_integration.rs** - Métodos helper y tick functions migrados
2. **handlers_migration.rs** - Handlers migrados para MapManager

## Cambios a Aplicar

### 1. Modificar lib.rs

Agregar los nuevos módulos:

```rust
// En crates/wow-world/src/lib.rs
pub mod session_map_integration;
pub mod handlers_migration;
```

### 2. Modificar session.rs

#### 2.1 Eliminar campo antiguo

```rust
// Buscar y ELIMINAR:
// pub(crate) creatures: HashMap<ObjectGuid, CreatureAI>,
// pub(crate) visible_creatures: HashSet<ObjectGuid>,
```

#### 2.2 Agregar include del módulo de integración

En session.rs, al final del archivo, agregar:

```rust
// Include migrated implementations
include!("session_map_integration.rs");
```

### 3. Reemplazar métodos legacy

#### 3.1 Reemplazar `tick_creatures_sync()`:

```rust
// ANTES:
fn tick_creatures_sync(&mut self) {
    // código que usa self.creatures...
}

// DESPUÉS:
fn tick_creatures_sync(&mut self) {
    self.tick_creature_ai_global();
}
```

#### 3.2 Reemplazar `tick_combat_sync()`:

```rust
// ANTES:
fn tick_combat_sync(&mut self) {
    // código que usa self.creatures.get_mut()...
}

// DESPUÉS:
fn tick_combat_sync(&mut self) {
    self.tick_combat_global();
}
```

#### 3.3 Reemplazar `handle_query_creature()`:

```rust
// ANTES:
async fn handle_query_creature(&mut self, query: QueryCreature) {
    // código que busca en self.creatures.values()...
}

// DESPUÉS:
async fn handle_query_creature(&mut self, query: QueryCreature) {
    self.handle_query_creature_migrated(query).await;
}
```

### 4. Modificar handlers/character.rs

Buscar y reemplazar cada uso de `self.creatures`:

#### 4.1 Líneas 1924, 1937, 1943, 2082, 2102:

```rust
// ANTES:
self.creatures.insert(guid, ai);
self.creatures.get(&guid);
self.creatures.remove(&guid);

// DESPUÉS - Usar métodos del MapManager via WorldSession:
self.spawn_creature_global(creature_data);
self.get_creature(&guid);
self.despawn_creature_global(guid);
```

#### 4.2 Handler `handle_list_inventory()`:

```rust
// Reemplazar implementación completa con:
pub async fn handle_list_inventory(&mut self, hello: Hello) {
    self.handle_list_inventory_migrated(hello).await;
}
```

#### 4.3 Handler `handle_buy_item()`:

```rust
pub async fn handle_buy_item(&mut self, buy: BuyItem) {
    self.handle_buy_item_migrated(buy).await;
}
```

#### 4.4 Handler `handle_sell_item()`:

```rust
pub async fn handle_sell_item(&mut self, sell: SellItem) {
    self.handle_sell_item_migrated(sell).await;
}
```

### 5. Modificar handlers/trainer.rs

```rust
// Reemplazar handle_trainer_list():
pub async fn handle_trainer_list(&mut self, hello: Hello) {
    self.handle_trainer_list_migrated(hello).await;
}
```

### 6. Modificar handlers/loot.rs

```rust
// Reemplazar handle_loot_unit():
pub async fn handle_loot_unit(&mut self, creature_guid: ObjectGuid) {
    self.handle_loot_unit_migrated(creature_guid).await;
}
```

### 7. Modificar handlers/combat.rs

```rust
// Reemplazar handle_attack_swing():
pub async fn handle_attack_swing(&mut self, target_guid: ObjectGuid) {
    self.handle_attack_swing_migrated(target_guid).await;
}

// Reemplazar handle_attack_stop():
pub async fn handle_attack_stop(&mut self) {
    self.handle_attack_stop_migrated().await;
}
```

### 8. Modificar handlers/misc.rs

```rust
// Reemplazar handle_taxi_node_status_query():
pub async fn handle_taxi_node_status_query(&mut self, packet: ClientPacket) {
    self.handle_taxi_node_status_query_migrated(packet).await;
}
```

### 9. Modificar handlers/character.rs - Gossip handlers

```rust
// Líneas 2713-2714 y 2906:
pub async fn handle_gossip_hello(&mut self, hello: Hello) {
    self.handle_gossip_hello_migrated(hello).await;
}
```

### 10. Limpieza final

#### 10.1 Eliminar imports no usados en session.rs:

```rust
// Si se importaba CreatureAI directamente para el HashMap, verificar que ahora
// se use solo a través del MapManager o de handlers::combat
```

#### 10.2 Eliminar `character_migration.rs` si existe:

```bash
rm crates/wow-world/src/character_migration.rs
```

## Compilación

```bash
cargo check -p wow-world
```

## Testing

```bash
cargo test -p wow-world --lib
```

## Notas Importantes

1. **CreatureAI vs WorldCreature**:
   - `CreatureAI` es la estructura legacy que estaba en `self.creatures`
   - `WorldCreature` es la nueva estructura en el MapManager global
   - Ambas son similares pero pueden tener campos ligeramente diferentes

2. **Broadcasting**:
   - Ahora las criaturas son compartidas entre todos los jugadores en el grid
   - Los cambios deben propagarse a todos los jugadores visibles
   - Usar `broadcast_to_nearby()` para enviar updates

3. **Respawn**:
   - El sistema de respawn ahora debe usar el MapManager global
   - Añadir criaturas respawned al MapManager, no al HashMap local

4. **Persistencia**:
   - HP, estado, posición de criaturas deben persistir en el MapManager
   - No hay necesidad de sincronización entre sesiones (es un solo objeto compartido)
