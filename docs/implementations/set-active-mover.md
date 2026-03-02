# SetActiveMover Handler

**Fecha de implementación:** 2026-02-24  
**Estado:** ✅ Completado  
**Versión:** 1.0  

---

## Overview

Handler para el opcode `CMSG_SET_ACTIVE_MOVER` (0x3A3C). El cliente envía este paquete después del login para establecer qué unidad está siendo movida por el jugador (normalmente su propio GUID). En sesiones multi‑jugador también se usa al cambiar de vehículo o controlar otra unidad.

---

## C# Reference

### Archivos relevantes:
- `Source/Game/Handlers/MovementHandler.cs`: `HandleSetActiveMover(SetActiveMover packet)`
- `Source/Game/Networking/Packets/MovementPackets.cs`: `class SetActiveMover : ClientPacket`

### Comportamiento en C#:
1. El cliente envía el GUID del “active mover” (unidad que se está moviendo).
2. El servidor verifica que el GUID coincida con la unidad que el jugador está controlando (`_player.GetUnitBeingMoved()`).
3. Si no coincide, registra un error (pero no kickea al jugador).
4. No envía respuesta; es solo una sincronización cliente‑servidor.

---

## Implementación Rust

### Solución adoptada:
Implementación simplificada para sesiones single‑player:
- Valida que el GUID enviado sea el del jugador (o esté vacío).
- Registra un warning si hay discrepancia (pero no es fatal).
- No requiere respuesta.

### Archivos modificados/creados:

#### 1. `/path/to/rustycore/crates/wow-packet/src/packets/movement.rs`
- **Estructura:** `pub struct SetActiveMover { pub active_mover: ObjectGuid }`
- **Implementación:** `impl ClientPacket for SetActiveMover`
- **Lectura:** `pkt.read_packed_guid()`

#### 2. `/path/to/rustycore/crates/wow-world/src/handlers/movement.rs`
- **Función:** `handle_set_active_mover(&mut self, pkt: SetActiveMover) -> async`
- **Lógica:**
  - Log con `trace!` el GUID recibido.
  - Si el jugador tiene GUID y el recibido no está vacío, compara.
  - Si no coincide: `warn!` (pero no se rechaza).
- **Registro:** `inventory::submit!` con `opcode = SetActiveMover`.

#### 3. `/path/to/rustycore/crates/wow-world/src/session.rs`
- **Dispatch:** Añadido en el match de opcodes, dentro de “Movement control opcodes”.
- **Lectura:** `match wow_packet::packets::movement::SetActiveMover::read(&mut pkt)`.

---

## Packet Flow

### Client → Server:
```
CMSG_SET_ACTIVE_MOVER (0x3A3C)
  PackedGuid ActiveMover
```

### Server → Client:
No hay respuesta. El cliente solo espera que el servidor acepte el mover.

---

## Dependencias

1. **ObjectGuid:** ✅ Disponible en `wow_core`.
2. **Movement system:** ✅ Los paquetes de movimiento ya están implementados.
3. **Player GUID tracking:** ✅ `WorldSession.player_guid` está disponible.

---

## Testing Notes

✅ **Funcional:**
- El handler está registrado y responde al opcode.
- La validación de GUID funciona (warning si hay mismatch).
- No interrumpe el flujo normal de login.

⚠️ **Limitaciones actuales:**
- Solo valida para single‑player (no hay vehículos ni control de otras unidades).
- No se sincroniza con un sistema de “unit being moved” (no implementado aún).

❌ **Problemas conocidos:**
- Ninguno.

---

## Next Steps / To Do

1. [ ] Integrar con sistema de vehículos cuando exista.
2. [ ] Añadir sincronización con `Unit::GetUnitBeingMoved()`.
3. [ ] En multi‑jugador, permitir GUIDs de otras unidades controlables.

---

## Referencias

- `Source/Game/Handlers/MovementHandler.cs`: Líneas 609‑622.
- `Source/Game/Networking/Packets/MovementPackets.cs`: Líneas 533‑544.
- `wow-constants/src/opcodes.rs`: `SetActiveMover = 0x3a3c`.

---

**Última revisión:** 2026-02-24  
**Responsable:** @WoWServer  
**Issue relacionada:** #login-handlers