# MoveInitActiveMoverComplete Handler

**Fecha de implementación:** 2026-02-24  
**Estado:** ✅ Completado (stub)  
**Versión:** 1.0  

---

## Overview

Handler para el opcode `CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE` (0x3A46). El cliente envía este paquete después del login, una vez que el “active mover” ha sido completamente inicializado. En C# se usa para actualizar flags de timing de transportes y forzar una actualización de visibilidad.

---

## C# Reference

### Archivos relevantes:
- `Source/Game/Handlers/MovementHandler.cs`: `HandleMoveInitActiveMoverComplete(MoveInitActiveMoverComplete moveInitActiveMoverComplete)`
- `Source/Game/Networking/Packets/MovementPackets.cs`: `class MoveInitActiveMoverComplete : ClientPacket`

### Comportamiento en C#:
1. El cliente envía un valor `Ticks` (tiempo relativo al servidor).
2. El servidor establece el flag `PlayerLocalFlags.OverrideTransportServerTime`.
3. Calcula el tiempo de transporte: `LoopTime.RelativeTime - packet.Ticks`.
4. Llama a `_player.UpdateObjectVisibility(false)` para forzar una actualización de visibilidad.

---

## Implementación Rust

### Solución adoptada:
Stub simple que solo registra la recepción del paquete. La lógica completa de transporte y flags se pospone hasta que exista el sistema de transportes.

### Archivos modificados/creados:

#### 1. `/path/to/rustycore/crates/wow-packet/src/packets/movement.rs`
- **Estructura:** `pub struct MoveInitActiveMoverComplete { pub ticks: u32 }`
- **Implementación:** `impl ClientPacket for MoveInitActiveMoverComplete`
- **Lectura:** `pkt.read_uint32()` (ticks relativos).

#### 2. `/path/to/rustycore/crates/wow-world/src/handlers/movement.rs`
- **Función:** `handle_move_init_active_mover_complete(&mut self, pkt: MoveInitActiveMoverComplete) -> async`
- **Lógica:**
  - Log con `trace!` el valor de ticks recibido.
  - No se realiza ninguna acción adicional (stub).
- **Registro:** `inventory::submit!` con `opcode = MoveInitActiveMoverComplete`.

#### 3. `/path/to/rustycore/crates/wow-world/src/session.rs`
- **Dispatch:** Añadido en el match de opcodes, dentro de “Movement control opcodes”.
- **Lectura:** `match wow_packet::packets::movement::MoveInitActiveMoverComplete::read(&mut pkt)`.

---

## Packet Flow

### Client → Server:
```
CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE (0x3A46)
  uint32 Ticks   // tiempo relativo al servidor
```

### Server → Client:
No hay respuesta. El cliente solo notifica que la inicialización ha terminado.

---

## Dependencias

1. **Transport system:** ❌ No implementado (flags y timing pospuestos).
2. **Visibility update:** ✅ Ya existe `update_visibility()` pero no se llama desde este handler.
3. **Time sync:** ❌ `LoopTime.RelativeTime` no está implementado.

---

## Testing Notes

✅ **Funcional:**
- El handler está registrado y responde al opcode.
- El valor de ticks se lee correctamente.
- No rompe el flujo de login.

⚠️ **Limitaciones actuales:**
- No se establecen flags de transporte.
- No se actualiza visibilidad (no crítico para login básico).
- No se calcula ni usa el tiempo de transporte.

❌ **Problemas conocidos:**
- Ninguno (stub intencional).

---

## Next Steps / To Do

1. [ ] Implementar sistema de transportes (vehículos, elevadores, etc.).
2. [ ] Añadir `PlayerLocalFlags` y manejo de flags.
3. [ ] Conectar con `UpdateObjectVisibility` cuando sea necesario.
4. [ ] Implementar `LoopTime` o equivalente para cálculos de tiempo relativo.

---

## Referencias

- `Source/Game/Handlers/MovementHandler.cs`: Líneas 936‑942.
- `Source/Game/Networking/Packets/MovementPackets.cs`: Líneas 915‑924.
- `wow-constants/src/opcodes.rs`: `MoveInitActiveMoverComplete = 0x3a46`.

---

**Última revisión:** 2026-02-24  
**Responsable:** @WoWServer  
**Issue relacionada:** #login-handlers