# Flujo NPC vendedor (tabernera) — C# vs Rust

## Cómo depurar/simular en C#

### 1. Flujo completo en C# (tabernera / cualquier NPC con menú)

1. **Cliente hace clic en el NPC**  
   → Envía `CMSG_GOSSIP_HELLO` con el GUID del NPC.

2. **Servidor (NPCHandler.HandleGossipHello)**  
   - Obtiene la criatura en el mundo: `GetPlayer().GetNPCIfCanInteractWith(packet.Unit, NPCFlags1.Gossip)`  
     → `ObjectAccessor.GetCreature()` / `Map.GetCreature(guid)` (el NPC está en el mapa).  
   - Llama a `unit.GetAI().OnGossipHello(_player)` o, si no lo maneja el script,  
     `GetPlayer().PrepareGossipMenu(unit, ...)` y `SendPreparedGossip(unit)`.

3. **Servidor envía menú de chismes**  
   - `GossipMessagePkt`: texto + lista de opciones (`ClientGossipOptions`).  
   - Cada opción tiene `GossipOptionID`, `OptionNpc` (tipo de acción), `Text`, etc.  
   - Para “ver mercancías” suele haber una opción con `OptionNpc = GossipOptionNpc.Vendor`.

4. **Cliente muestra el menú**  
   El jugador elige “¿Qué tienes para comer?” / “Déjame ver tus mercancías”.

5. **Cliente envía la opción elegida**  
   → `CMSG_GOSSIP_SELECT_OPTION` (o `GossipOptionNpcInteraction`) con el GUID del NPC y el `GossipOptionID`.

6. **Servidor (HandleGossipSelectOption)**  
   - Comprueba que el GUID coincida con `PlayerTalkClass.GetInteractionData().SourceGuid`.  
   - Si la opción es “vendor”, el script (p. ej. NpcProfessions) llama a  
     `player.GetSession().SendListInventory(me.GetGUID())`.

7. **SendListInventory (NPCHandler)**  
   - Obtiene de nuevo la criatura: `GetPlayer().GetNPCIfCanInteractWith(vendorGuid, NPCFlags1.Vendor)`.  
   - `VendorItemData vendorItems = vendor.GetVendorItems()`  
     → `ObjectMgr.GetNpcVendorItemList(vendor.GetEntry())` (datos cargados al arrancar desde `npc_vendor`).  
   - Para cada item: si `item < 0` es **referencia** a otro vendor; C# expande con `LoadReferenceVendor(entry, -item)`.  
   - Filtra por condiciones, clase, facción, stock, etc.  
   - Envía `SMSG_VENDOR_INVENTORY` (VendorInventory) con la lista de `VendorItemPkt`.

### 2. Dónde poner breakpoints en C#

- **Gossip:**  
  `Source/Game/Handlers/NPCHandler.cs` → `HandleGossipHello` (línea ~147) y `HandleGossipSelectOption` (línea ~191).  
- **Lista de vendedor:**  
  `Source/Game/Handlers/NPCHandler.cs` → `SendListInventory` (línea ~456).  
- **Datos de vendedor:**  
  `Source/Game/Globals/ObjectManager.cs` → `LoadNpcVendor` (~4063), `GetNpcVendorItemList` (~6308), `LoadReferenceVendor` (~4115).

### 3. Origen de los items en C#

- Tabla `world.npc_vendor`: columnas `entry`, `item`, `maxcount`, `incrtime`, `ExtendedCost`, `Type`, `BonusListIDs`, `PlayerConditionID`, `IgnoreFiltering`, `slot`.  
- Al arrancar se cargan en `cacheVendorItemStorage` por `entry`.  
- Si `item` es **negativo**, es una referencia: se cargan los items del vendor con `entry = -item` (recursivo, con protección ante ciclos).

---

## Qué fallaba en Rust (rustycore) y qué se corrigió

### 1. Entry del vendedor solo desde el “tracker”

- **Problema:** El entry se obtenía solo de `self.creatures.get(&vendor_guid)`. Si el NPC no estaba en el tracker de visibilidad (p. ej. por rango, timing o no haber recibido su spawn), se devolvía **lista vacía**.  
- **Solución:** Si el GUID no está en el tracker, se hace **fallback a la BD**:  
  `SEL_CREATURE_ENTRY_BY_GUID` → `SELECT id FROM creature WHERE guid = ?`  
  usando el GUID (low part) del NPC. Así la lista de vendedor se puede rellenar aunque el NPC no esté en el tracker.

### 2. Referencias de vendedor (`item_id < 0`) ignoradas

- **Problema:** En C#, un registro en `npc_vendor` con `item < 0` significa “incluir aquí todos los items del vendor con entry = -item”. En Rust solo se enviaban items con `item_id > 0`, así que **no se expandían referencias** y faltaban items (o todo el inventario si el vendor era solo referencias).  
- **Solución:** Se expanden referencias como en C#:  
  - Cola de “entries a procesar” y conjunto de “entries ya expandidos” (evitar ciclos).  
  - Para cada entry se ejecuta `SEL_VENDOR_ITEMS`.  
  - Si `item_id > 0` → se añade el item a la lista.  
  - Si `item_id < 0` → se añade `ref_entry = -item_id` a la cola (si no está ya en expandidos).  
  Así se cargan todos los items, incluidos los que vienen de referencias.

### 3. Resumen de cambios en código Rust

- **wow-database:**  
  - Nueva sentencia `SEL_CREATURE_ENTRY_BY_GUID`: `SELECT id FROM creature WHERE guid = ?`.  
- **wow-world (handle_list_inventory):**  
  - Resolver `entry` desde el tracker o, en su defecto, desde la BD por GUID.  
  - Cargar items con expansión de referencias (cola + conjunto de entries expandidos).  
  - Usar correctamente la columna `IgnoreFiltering` (columna 9) en los items enviados.

Con esto, la lista de mercancías en Rust debería coincidir en contenido con la del servidor C# (mismos items y referencias expandidas).

---

## Icono de interrogación roja y nombre vacío en el cliente

Si en la ventana del vendedor un hueco muestra **?** roja y **nombre vacío**, el cliente no tiene ese `item_id` en sus datos locales (Item.dbc / DB2 del build 3.3.5). Suele deberse a:

- Un **item de otra expansión** o parche en `npc_vendor` que el cliente 3.3.5 no conoce.
- Un **item_id inválido o custom** en la base de datos.

**Qué hacer:** Revisar en la BD los items del vendor (`npc_vendor` por `entry` de la criatura) y quitar o sustituir los que no existan en el Item.dbc del cliente 3.3.5. Con el log en nivel `debug` y el mensaje `Sending vendor inventory: ... (item_ids: [...])` puedes ver qué IDs se envían y localizar el que falla.
