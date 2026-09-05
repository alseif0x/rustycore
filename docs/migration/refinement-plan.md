# Plan de refinamiento — WBS completa del port C++

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/`
> **Rust target:** `/home/server/rustycore`
> **Objetivo histórico:** convertir el entonces vigente roadmap en una WBS.
> La congelación y las fases R0–R6 siguientes están retiradas como workflow;
> consultar PORT_PLAN.md y la issue/checkpoint actual para ejecutar trabajo.
> **Regla:** ninguna tarea se acepta como cerrada si no cita el C++ contra el que se contrasto.
> **Dependencias actuales:** no ocultar requisitos pendientes ni afirmar cierre
> sin ellos; resolver dentro del alcance aprobado o documentar el bloqueo, sin
> una expansión automática de autoridad.

---

## 1. Objetivo

Construir un backlog de migracion que cubra el servidor C++ completo sin gaps:

- cada directorio C++ relevante queda asignado a un modulo de migracion;
- cada archivo fuente C++ relevante queda asignado a una tarea o excluido explicitamente con razon;
- cada opcode, prepared statement, store DB2/DBC, clave de config, tipo de entidad, familia de scripts y runtime manager tiene responsable;
- cada tarea tiene dependencias, criterios de aceptacion y estrategia de tests;
- la implementacion solo se reanuda desde este backlog refinado.

Los docs existentes siguen sirviendo como triage, pero no son prueba de correccion.

---

## 2. Formato de tarea

Toda tarea que se añada a docs de modulo debe tener esta forma:

```text
- [ ] **#<MODULE>.<NNN>** <accion concreta>
  C++ refs: `<path>:<symbol>`, `<path>:<symbol>`
  Rust target: `<crate/module/file>`
  Depends on: `<task ids>` o `none`
  Acceptance: criterios de compile/test/runtime/golden
  Notes: divergencias, bugs en C++ o detalles aplazados explicitamente
```

Las subtareas usan sufijos:

```text
#MAP.014.a
#MAP.014.b
```

No crear tareas de implementacion llamadas solo "port X". Hay que partir por comportamiento concreto, modelo de datos, superficie de packets, persistencia, ciclo de vida y tests.

---

## 3. Fases de refinamiento

### R0 — Congelacion y reglas

- [ ] **#REFINE.001** Congelar features nuevas hasta refinar el backlog de la siguiente mini-fase de implementacion.
- [ ] **#REFINE.002** Añadir la regla a todo doc de planificacion activo: C++ es el oraculo; docs/agentes son solo triage.
- [ ] **#REFINE.003** Mantener `main` limpio; todo parche exploratorio debe respaldarse con branch/commit o descartarse.

### R1 — Inventarios canonicos C++

- [x] **#REFINE.010** Generar `docs/migration/inventory/cpp-server-tree.md`: inventario top-level de `src/server`, excluyendo `PrecompiledHeaders`.
- [x] **#REFINE.011** Generar `docs/migration/inventory/cpp-files-by-module.md`: todo `.h/.cpp/.hpp` asignado a un modulo de migracion.
- [x] **#REFINE.012** Generar `docs/migration/inventory/cpp-handlers-opcodes.md`: handlers, nombres CMSG/SMSG, registro/status/processing y dueño Rust.
- [x] **#REFINE.013** Generar `docs/migration/inventory/cpp-sql-prepared.md`: prepared statements y SQL inline por DB (`auth`, `characters`, `world`, `hotfixes`).
- [x] **#REFINE.014** Generar `docs/migration/inventory/cpp-dbc-db2-stores.md`: stores DB2/DBC, tablas hotfix, loaders y dueño Rust.
- [x] **#REFINE.015** Generar `docs/migration/inventory/cpp-config-keys.md`: claves de config world/bnet y equivalente Rust.
- [x] **#REFINE.016** Generar `docs/migration/inventory/cpp-entity-types.md`: archivos y ownership de Object/WorldObject/Unit/Player/Creature/GO/etc.
- [x] **#REFINE.017** Generar `docs/migration/inventory/cpp-scripts-tree.md`: scripts por familia (`Commands`, `Spells`, continentes, raids, events, PvP).

Aceptacion de R1: todo archivo fuente C++ queda asignado o excluido explicitamente.

### R2 — WBS por modulo

Para cada modulo de `_INDEX.md`:

- [x] **#REFINE.020** Asegurar seccion "C++ canonical files" con todos los archivos C++ asignados.
- [x] **#REFINE.021** Asegurar seccion "Rust target" con crates/modulos exactos.
- [x] **#REFINE.022** Añadir "Task WBS" con IDs, dependencias y criterios de aceptacion.
- [x] **#REFINE.023** Añadir "Known divergences / bugs" con evidencia desde C++.
- [x] **#REFINE.024** Añadir "Tests required" con gates unit, golden, integration y E2E.
- [x] **#REFINE.025** Marcar explicitamente sistemas post-WoLK o desactivados por producto, sin descartarlos en silencio.

Aceptacion de R2: ningun doc de modulo queda solo con tareas de alto nivel; toda tarea es implementable en un commit pequeño o marcada como "needs split".

### R3 — Registros transversales

- [x] **#REFINE.030** Crear/actualizar registro de opcodes: cada CMSG/SMSG con owner, parser, serializer, handler, status, processing mode y test.
- [x] **#REFINE.031** Crear/actualizar registro de base de datos: cada tabla/statement/load path con owner y target Rust.
- [x] **#REFINE.032** Crear/actualizar registro de update fields: cada grupo por entidad con owner y plan de test de packet.
- [x] **#REFINE.033** Crear/actualizar registro de runtime managers: World, MapManager, ObjectMgr, SpellMgr, LFGMgr, GuildMgr, AuctionMgr, etc.
- [x] **#REFINE.034** Crear/actualizar registro de scripts: command scripts, spell scripts, world scripts, instance/boss scripts.
- [x] **#REFINE.035** Crear/actualizar registro de harness E2E: bot C++, deltas auth/login Rust y secuencia de aceptacion.

Aceptacion de R3: los elementos transversales no quedan duplicados ni huerfanos entre docs de modulo.

### R4 — Revision de dependencias y gates

- [x] **#REFINE.040** Construir DAG de dependencias L0-L8 con aristas bloqueantes.
- [x] **#REFINE.041** Marcar gates de implementacion: que debe ser cierto antes de empezar cada fase.
- [x] **#REFINE.042** Identificar stubs temporales permitidos para progreso compile-only, con task IDs de retirada.
- [x] **#REFINE.043** Identificar smoke gates runtime para cada capa mayor.
- [x] **#REFINE.044** Reordenar tareas del roadmap cuando las dependencias C++ demuestren que el orden actual es incorrecto.

Aceptacion de R4: ninguna tarea depende de un sistema que no este implementado ni stubbeado explicitamente con ticket de retirada.

### R5 — Gap audit

- [x] **#REFINE.050** Ejecutar check de cobertura de archivos: C++ asignados vs total.
- [x] **#REFINE.051** Ejecutar check de cobertura de opcodes: handlers/opcodes C++ asignados vs total.
- [x] **#REFINE.052** Ejecutar check de cobertura SQL: statements/loaders C++ asignados vs total.
- [x] **#REFINE.053** Ejecutar check de cobertura de scripts: archivos de scripts asignados vs total.
- [x] **#REFINE.054** Ejecutar check de dirty/backlog Rust: trabajo aparcado revisado y convertido en tareas o descartado.

Aceptacion de R5: no quedan "unknown unknowns" a nivel de inventario de archivos/opcodes/scripts.

### R6 — Preparacion para implementar

- [x] **#REFINE.060** Elegir la siguiente mini-fase desde el DAG refinado.
- [x] **#REFINE.061** Confirmar que todas sus refs C++ estan listadas.
- [x] **#REFINE.062** Confirmar que los tests de aceptacion estan listados antes de programar.
- [x] **#REFINE.063** Confirmar plan de rollback/parking para cambios exploratorios.

Aceptacion de R6: la implementacion puede reanudarse con una mini-fase acotada y sin adivinar.

Resultado R6: la siguiente mini-fase es `#NEXT.L0.CONFIG.001` (L0 config parity / startup config schema), documentada en `docs/migration/inventory/r6-next-miniphase.md` y `docs/migration/inventory/r6-next-miniphase.tsv`. Maps sigue en cola L3, pero queda bloqueado por los gates L0/L1/L2 del DAG refinado.

---

## 4. Checklist de cobertura

El backlog refinado debe incluir estos dominios:

- Binarios: `bnetserver`, `worldserver`.
- Infraestructura server: `database`, `proto`, `shared/*`.
- Fundacion game: `Globals`, `Time`, `Miscellaneous`, `Texts`, `Tools`, `World`, `Server`.
- Sustrato espacial/runtime: `Maps`, `Grids`, `Phasing`, `TerrainMgr`, `MapUpdater`, `SpawnData`, `ObjectGridLoader`.
- Entidades: todos los subdirectorios bajo `game/Entities`, incluyendo soporte `Taxi`.
- Motores: `Movement`, `Combat`, `Spells`, `AI`, `Conditions`.
- Sistemas game: quests, loot, inventory/items, groups, guilds, chat, social, reputation, skills, mail, auctions, calendar, achievements, petitions, pools.
- PvP/instances: battlegrounds, battlefield, outdoor PvP, instances, dungeon finding.
- Services/support: accounts, services, cache, support tickets, warden, weather, events.
- Contenido: `Scripting` y cada familia `scripts/*`.
- Testing: unit, golden, integration, harness E2E con bot, catalogo de regresiones.

---

## 5. Criterios de cierre

Esta fase de refinamiento queda cerrada cuando:

- todas las tareas `#REFINE.*` anteriores estan completas o partidas deliberadamente en subtareas;
- cada archivo fuente C++ tiene owner o razon de exclusion;
- cada doc de modulo tiene WBS con dependencias y criterios de aceptacion;
- `MIGRATION_ROADMAP.md` apunta al backlog refinado y ya no contiene estados obsoletos contradictorios;
- la siguiente mini-fase de implementacion tiene lista precisa de tareas, referencias C++ y tests.
