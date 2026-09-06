# [Nombre del Handler/Sistema]

Plantilla opcional para documentar evidencia, no un gate por helper. El flujo y
las aprobaciones viven en [AGENTS.md](../AGENTS.md); el estado y alcance vigente
en [STATE.md](migration/STATE.md), [PORT_PLAN.md](migration/PORT_PLAN.md) y la issue
activa. Distinguir implementación, integración y paridad probada.

**Fecha de implementación:** YYYY-MM-DD

**HEAD / alcance realmente contrastado:** [commit, operación y límites]

**Estado:** ✅ Completado / 🔄 Parcial / ❌ Pendiente

**Versión:** 1.0

---

## Overview

[Breve descripción de qué hace este handler o sistema]

---

## Referencia C++ canónica

### Archivos relevantes:
- `/home/server/woltk-trinity-legacy/src/server/game/Handlers/[Handler].cpp`: función/rama
- `src/server/game/Server/Packets/[Packet].cpp`: lectura/escritura y orden
- `src/server/game/Entities/[Entity]/[Entity].cpp`: reglas y efectos relacionados

### Comportamiento C++ y evidencia:
[Describir gates, estado, ownership, persistencia, orden de efectos y opcodes.
Si C++ es incompleto/ambiguo, identificar captura real y build; no inventar
paridad ni una autoridad alternativa.]

---

## Implementación Rust

### Solución adoptada:
[Explicar implementación Rust, owner y diferencias autorizadas respecto a C++]

### Archivos modificados/creados:

#### 1. `/path/to/file.rs`
- **Función:** `fn nombre()`
- **Lógica:** [Descripción breve]
- **Notas:** [Cualquier detalle importante]

#### 2. `/path/to/another.rs`
- **Cambios:** [Descripción]
- **Impacto:** [Cómo afecta a otros sistemas]

---

## Packet Flow

### Client → Server:
```
[Opcode]: [Nombre del paquete]
[Descripción de lo que envía el cliente]
```

### Server → Client:
```
[Opcode]: [Nombre del paquete]
[Descripción de lo que responde el servidor]
```

---

## Dependencias

1. **[Sistema A]:** ✅ Completado / 🔄 Parcial / ❌ No implementado
2. **[Sistema B]:** ✅ Completado / 🔄 Parcial / ❌ No implementado

---

## Testing Notes

[Comandos exactos, resultado, fecha y host si afecta la medida. Seleccionar
validación proporcional y gates explícitos; no convertir esta plantilla en una
auditoría exhaustiva por helper. Cliente/runtime solo como probado si se ejerció.]

✅ **Funcional:**
- [Lo que funciona correctamente]

⚠️ **Limitaciones actuales:**
- [Lo que no funciona o es incompleto]

❌ **Problemas conocidos:**
- [Errores o bugs]

---

## Next Steps / To Do

1. [ ] [Tarea prioritaria 1]
2. [ ] [Tarea prioritaria 2]
3. [ ] [Mejora o optimización]

---

## Referencias

- `[Path en C++]`: [Función/clase, rama y revisión]
- `[Path en Rust]`: [Nombre de función/clase]
- Issue/Ticket: #[número] (si existe)

---

**Última revisión:** YYYY-MM-DD

**Responsable:** @WoWServer

**Issue relacionada:** #[número o descripción]
