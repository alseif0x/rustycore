# Migration: <MODULE_NAME>

Optional reference-document template, not a required document per helper or
an independent workflow. Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md)
and the active issue/checkpoint for current status and delivery. Follow
[module design guidelines](../architecture/module-design-guidelines.md) for
semantic and physical decomposition; C++ class layout is not a Rust module plan.

> **C++ canonical path:** `<absolute path under /home/server/woltk-trinity-legacy/>`
> **Rust target crate(s):** `crates/<crate>/`
> **Layer:** L0 / L1 / L2 / L3 / L4 / L5 / L6 / L7 / L8
> **Status:** ❌ not started / ⚠️ partial / ✅ done / 🔧 broken (rewrite needed)
> **Audited vs C++:** ❌ not audited / ⚠️ partial / ✅ complete
> **Last updated:** YYYY-MM-DD
> **Reviewed commit / bounded scope:** <HEAD, capability and evidence limits>

---

## 1. Purpose

(2-4 frases. Qué hace este módulo en TrinityCore. Qué problema resuelve.)

---

## 2. C++ canonical files

Todas las rutas relativas a `/home/server/woltk-trinity-legacy/`.

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/game/<Module>/Foo.h` | 250 | Foo class definition + interface |
| `src/server/game/<Module>/Foo.cpp` | 1100 | Foo implementation |
| ... | ... | ... |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `Foo` | class | Main entity of the module |
| `FooState` | enum | Lifecycle states |
| `FooData` | struct | POD wire format |
| ... | ... | ... |

---

## 4. Critical public methods / functions

(Solo los públicos de las clases listadas. Si son demasiados, top 15-20 más usados.)

| Symbol | Purpose | Calls into |
|---|---|---|
| `Foo::Add(Bar*)` | Inserta `bar` en el contenedor | `Bar::AttachTo`, `EventMgr` |
| `Foo::Update(uint32 diff)` | Tick por frame | `Bar::Update` |
| ... | ... | ... |

---

## 5. Module dependencies

**Depends on:**
- `<Module X>` — para Y propósito (cita header/clase específica)
- ...

**Depended on by:**
- `<Module Z>` — usa `<class>` para W
- ...

---

## 6. SQL / DB queries (if any)

Solo si el módulo emite queries directamente o registra prepared statements.

| Statement / Source | Purpose | DB |
|---|---|---|
| `SEL_FOO_BY_ID` | Carga Foo por ID | world |
| `INS_FOO_LOG` | Inserta log de evento | character |
| ... | ... | ... |

Si el módulo usa **DBC/DB2 stores** (cliente data), listar:

| Store | What it loads | Read by |
|---|---|---|
| `MapStorage` | Map.db2 | Map.cpp, MapManager.cpp |
| ... | ... | ... |

---

## 7. Wire-protocol packets (if any)

Si el módulo origina/recibe packets, listar opcodes con nombre y dirección.

| Opcode | Direction | Sent/Received in |
|---|---|---|
| `CMSG_FOO` | client → server | `WorldSession::HandleFoo` |
| `SMSG_FOO_RESPONSE` | server → client | `Player::SendFooResponse` |
| ... | ... | ... |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/<crate>/src/<file>.rs` — responsabilidad y comportamiento demostrado
- ...

**What's implemented:**
- ...

**What's missing vs C++:**
- ...

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- ...

**Tests existing:**
- N tests en `crates/<crate>/...`

---

## 9. Migration sub-tasks

Referenciar la issue/macro vigente y sus checkpoints cuando corresponda.
Los cortes internos no requieren un ID, commit, issue o PR por helper. Agrupar
por capacidades completas y mantener dependencias/boundaries con dueño explícito;
no usar el roadmap histórico como orden obligatorio ni horas como criterio de split.

- [ ] **<issue/checkpoint>** Capacidad, aceptación y límites precisos
- [ ] **#<MOD>.2** ...
- [ ] **#<MOD>.3** ...

---

## 10. Regression tests to write

Tests que demuestren que el comportamiento Rust = comportamiento C++ para invariantes clave.

- [ ] Test: <invariante 1>
- [ ] Test: <invariante 2>
- [ ] ...

---

## 11. Notes / gotchas

(Cualquier cosa que la próxima persona/sesión que toque este módulo debe saber: bugs históricos, ASSERT críticos del C++, peculiaridades de WoLK 3.4.3 vs otras versiones, performance hotspots, etc.)

---

## 12. C++ → Rust mapping (high-level)

| C++ Symbol | Rust Equivalent | Notes |
|---|---|---|
| `class Foo` | `struct Foo` (en `crates/<crate>/foo.rs`) | Sin herencia; usar enum si polymorphic |
| `Foo*` ownership | `Box<Foo>` / `Arc<Foo>` / referencia | Decidir per caso |
| `std::map<K, Foo*>` | Almacenamiento privado del owner aprobado | Concurrencia y lifetime según contrato, no sustitución mecánica |
| `void Foo::Update(uint32)` | `fn update(&mut self, diff_ms: u32)` | — |
| ... | ... | ... |

---

*Template version: 2.0 (2026-09-05).* Actualizar fecha, commit, alcance y evidencia;
no inferir un porcentaje de paridad a partir de líneas, símbolos o archivos presentes.
