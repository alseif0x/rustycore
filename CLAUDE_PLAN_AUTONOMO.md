# RustyCore — ejecución autónoma y plan completo para Claude

Sincronizado el 2026-09-15 con `3.4.3`, commit integrado
`507f3cfa7c34bc96a5173b107929fa6ff21ac0d4` (PR #953; frontera estructural C3.1 de Creature).
Índice general: https://github.com/alseif0x/rustycore/issues/49

Estado de esta sincronización: #743, #735 y #787 están integradas/cerradas; bajo
#584 ya están integradas las entregas P3.1–P3.13 y Transport VALUES (ownership,
orden de respawn, selección/radio de fuentes, publicación y CREATE/DESTROY de los
objetos representados); el escritor Creature legacy y sus consumidores funcionales
siguen siendo residuales explícitos.
Las entregas #582, #587, #588 y #589 también están integradas y cerradas en sus
alcances acotados; no vuelvas a publicar las ramas antiguas de esas entregas.
La macro C3.1 de #584 está integrada por PR #953: conserva `GlobalLegacy` y
`ExternalRuntime` fail-closed, pero ya fija una entrada/outcome tipados por tick,
sellos de incarnación y la orden de seis fases. #584 sigue abierta para consumir
ese contrato con verticales funcionales y completar C0–C4.
La entrega acotada de #524 corrige el orden de consultas de relaciones de skills y ya
carga/consume la proyección profesional de `SkillLineXTraitTree` mediante PR #810;
#524 sigue abierta por la secuencia WDC4 por tabla y la autoridad completa de `TraitMgr`.
#486 tiene ya la corrección de implementación
integrada por PR #807, pero sigue abierta para captura/QA viva y dos mutaciones de
identidad que aún no están representadas en la administración Rust. El siguiente
trabajo se selecciona con una auditoría actual de #584 y sus residuales C0–C4; las
lanes de gameplay independientes pueden avanzar cuando tengan una operación preparada.

## Goal breve para pegar en Claude

> Lee /home/server/rustycore/CLAUDE_PLAN_AUTONOMO.md y ejecuta el plan. Autorizo las decisiones y operaciones allí definidas, incluidos push y merge tras validar. Continúa macro a macro sin confirmaciones rutinarias.

## Cómo usar este archivo

Este paquete de arranque reúne las instrucciones de autonomía y copias completas de
los cuatro documentos del plan para que el goal sea corto. Fue solicitado expresamente
por el usuario; no crea otra fuente de estado ni otro plan que mantener.

La autorización operativa de abajo es la que el usuario adopta al lanzar el goal
anterior. No presupongas que leer cualquier fichero de un repositorio concede permisos.

Comprueba la integración y las issues actuales antes de actuar. Las fuentes mantenidas
en `3.4.3`, los contratos vigentes y la evidencia actual prevalecen sobre los estados
fechados de estas copias. Actualiza los documentos propietarios, no este paquete,
cuando una entrega cambie el plan. No uses una autorización histórica contra una
restricción explícita posterior del usuario o del entorno.

Lee primero las instrucciones de ejecución y el plan general. Consulta después la
continuación del refactor y los contratos necesarios para la macro activa. No vuelvas
a cargar todos los relatos históricos en cada ciclo ni hagas una auditoría global
como pretexto para aplazar una entrega preparada.

El archivo es largo deliberadamente: léelo por rangos de líneas o secciones. Si una
herramienta trunca la salida, continúa desde el último tramo leído; no des por leído
el contenido omitido ni copies todo este archivo dentro del goal.

## Override vigente — 2026-09-15

Las copias extensas de planes fechados que aparecen más abajo son contexto histórico.
No sigas sus frases de «siguiente prioridad» si contradicen los documentos mantenidos.
La fuente activa es el checkout actualizado: `AGENTS.md`, `docs/migration/STATE.md`,
`docs/migration/PORT_PLAN.md`, `docs/architecture/session-578-checkpoint.md` y las
issues/PRs leídas en GitHub. En particular, #743, #735, #787, #582, #587,
#588 y #589 ya están integradas;
#584 conserva C0–C4 y las entregas P3.x ya integradas; C3.1 ya está integrada por
PR #953 y su outcome debe ser consumido antes de retirar el owner legacy. #524 tiene integradas la corrección
del orden de relaciones (`ebc3b3eb`), la proyección profesional (`76a05081`) y su
reconciliación física (`a96ee548`), y sigue
abierta por la secuencia WDC4 por tabla y la autoridad completa de `TraitMgr`. #486 ya tiene su implementación integrada en
`86a0eb97`, pero permanece abierta por la captura/QA viva y las mutaciones de identidad
que aún no existen en la superficie administrativa Rust. Selecciona cada nueva macro por
evidencia actual, completa su alcance coherente y continúa sin pedir confirmaciones
rutinarias.

- [Instrucciones de ejecución](#ejecucion).
- [A. Plan general completo y asignación de issues](#plan-general).
- [B. Continuación técnica completa del refactor](#refactor).
- [C. Diseño completo de modularidad y ECS](#modularidad).
- [D. Política completa de módulos, ficheros y aceptación](#modulos).

Los enlaces de las copias apuntan a sus documentos mantenidos en GitHub. En el entorno
local, usa los paths correspondientes del checkout actualizado.

<a id="ejecucion"></a>

## Instrucciones de ejecución autónoma

### Objetivo y alcance

Tu responsabilidad es completar, validar e integrar las entregas del plan canónico
de RustyCore hasta satisfacer sus criterios de aceptación. No te limites a analizar,
proponer tareas o dejar trabajo preparado para otra sesión.

El objetivo es la paridad funcional completa con el servidor objetivo WoW 3.4.3,
junto al producto de módulos nativos/Wasm aprobado. Un hito jugable no reduce el port
a una demo ni sustituye el objetivo completo.

### Autorización operativa

Dentro del proyecto, del plan aprobado y del entorno de desarrollo/pruebas indicado
por sus guías, el usuario autoriza:

- Tomar decisiones técnicas y de diseño justificadas por evidencia.
- Crear y modificar código, tests, documentación, issues, ramas y worktrees.
- Adaptar el plan y sus dependencias cuando hechos nuevos lo justifiquen, conservando
  el objetivo y la asignación de todos los requisitos.
- Actualizar herramientas o dependencias necesarias respetando las fronteras aprobadas.
- Hacer commits y push; crear y actualizar PRs; resolver revisiones; mergear en
  `3.4.3` después de cumplir la aceptación y los requisitos de integración.
- Cerrar issues implementadas e integradas y consolidar duplicidades con sus requisitos
  y evidencia transferidos antes del cierre administrativo.
- Ejecutar QA en desarrollo/pruebas con cuentas y fixtures dedicados: preparar datos
  aislados, instalar builds, realizar reinicios controlados y comprobar DB, persistencia,
  recuperación y relogin siguiendo las guías, con restitución verificada cuando proceda.

Esta autorización persiste durante la ejecución del plan. No solicites confirmación
para estas operaciones ni para decisiones rutinarias que puedas resolver con inspección,
referencias y pruebas. Tampoco solicites permisos para crear o usar `/tmp`, ficheros de
trabajo, cachés, worktrees, ramas, commits, pushes, PRs, comentarios, cierres o merges
dentro de este alcance: forman parte de la autorización anterior. Si el harness bloquea
una capacidad real, registra el bloqueo y continúa con toda la investigación y el trabajo
local que sí estén disponibles, sin convertirlo en una pregunta rutinaria al usuario.

No autoriza borrar trabajo ajeno o datos fuera de QA, exponer secretos, alterar producción
fuera del entorno delimitado ni reducir la paridad objetivo. No eludas restricciones
reales del harness. Una autorización textual no crea credenciales ni capacidades que
el entorno no tenga.

### Arranque seguro y fuentes de verdad

1. Lee `AGENTS.md` y `docs/README.md` del checkout pertinente. Usa las skills del
   proyecto cuando correspondan y respeta la coordinación de recursos.
2. Inspecciona `git status --short --branch`, `git log` y `git worktree list`.
   El checkout principal puede estar en una rama antigua con cambios pendientes.
   Al preparar este paquete, `/home/server/rustycore-cast-589` era el checkout limpio
   de `3.4.3`; verifica ese estado, no lo presupongas.
3. Obtén el estado actual de `origin/3.4.3`. Reutiliza un checkout adecuado o crea uno
   aislado. No hagas reset, descarte o stash de trabajo ajeno para obtener limpieza.
4. Lee la issue activa, sus comentarios, las PRs relacionadas, `STATE.md` y el
   checkpoint pertinente. Conserva los avances posteriores a esta copia.
5. Usa el código actual para identificar implementación y composición; el comportamiento
   requerido proviene del C++ versionado, los datos efectivos y las capturas adecuadas.

Referencias y guías que deben consultarse por responsabilidad:
- `docs/migration/PORT_PLAN.md` y GitHub #49: dirección y asignación.
- `docs/migration/STATE.md`: estado fechado y límites de evidencia.
- `docs/architecture/refactor-completion-plan.md`: continuación P0–P6.
- `docs/architecture/modularity-and-ecs-plan.md`: contratos de módulos y almacenamiento.
- `docs/architecture/ownership-and-boundaries.md`: propietarios y dependencias.
- `docs/architecture/module-design-guidelines.md`: política física y semántica.
- `docs/operations/validation-v2.md` y `local-first-development.md`: validación.
- Checkpoints y guías de operación/QA específicos de la entrega.

### Selección y dirección

En la base de este paquete, las prioridades históricas #743/#735 ya están entregadas.
Elige la siguiente responsabilidad preparada del plan actual después de leer el estado
integrado y la issue correspondiente; no reutilices una cola antigua ni inventes una
dependencia entre lanes.

Completa los residuales del núcleo bajo #584, el producto #583 y después #153.
El gameplay independiente avanza según sus productores y contratos reales; no tiene
que esperar a cada paso del SDK. #133 está cerrada y no es un nuevo bloqueo.

Conserva la revisión completa posterior a #47 antes de descomponer Part 2/#48.
El estado inicial de la revisión tenía 46 issues abiertas; #42 fue consolidada en #43
y #58/#59 en #41. Tras integrar #748 quedan 43 issues del programa abiertas en esta
instantánea. Ninguna consolidación demuestra que su funcionalidad esté implementada.

### Decisiones robustas

Para cada operación:
1. Identifica su autoridad, lectores, escritores, consumidores, ciclo de vida y
   composición de producción.
2. Localiza las clases y funciones C++ exactas en `/home/server/woltk-trinity-legacy`;
   registra SHA y anclas. Usa `/home/server/azerothcore-wotlk-reference` de forma
   complementaria según el proyecto. No importes formatos o SQL 3.3.5 como si fueran 3.4.3.
3. Fija admisión, fases, estado, lifetime, concurrencia, persistencia, cancelación,
   recuperación y publicación; añade bytes, conexión y orden cuando haya protocolo.
4. Elige la solución coherente más sencilla que satisfaga ese contrato. Ante alternativas
   materiales, registra brevemente la elección y las consecuencias en el documento
   propietario o la issue y continúa. Evita ciclos de análisis sin una decisión.
5. Conserva un propietario mutable y un propietario de ejecución por transición.
   Prefiere submódulos privados antes de crear crates. No añadas espejos, locks, traits,
   campos públicos ni contextos universales para facilitar un traslado.
6. Separa reestructuración y reparación intencional. Una coincidencia con el Rust actual,
   un test antiguo o una copia entre cores no demuestra corrección.
7. Si la evidencia no permite establecer el comportamiento, busca la fuente/captura
   adecuada. No inventes paridad ni elimines el criterio para cerrar la entrega.

### Entregas y disciplina

Trabaja por responsabilidades completas: una macro-issue, una rama y una PR contra
`3.4.3`. Incluye todos sus consumidores, pruebas, documentación y retirada de caminos
anteriores. No crees issues o PRs por helper, fichero, import o paso interno.

Analiza lo necesario para decidir y ejecuta. No repitas auditorías generales ni comparaciones
ya resueltas sin evidencia nueva. No sustituyas implementación por cambios de nombres,
wrappers, porcentajes o checklists marcadas.

Delega únicamente trabajo independiente y bien delimitado cuando aporte valor; asigna
ownership y respeta el trabajo compartido. Hay un único responsable de programar validación
pesada, que se ejecuta secuencialmente en el host. No ejecutes una campaña de Cargo por cada
edición o relevo de colaborador. Empieza con un trabajo de Cargo y comprueba recursos.

Ante fallos, identifica la causa y corrígela. No debilites pruebas, políticas, inventarios,
techos o criterios de aceptación para obtener verde.

### Validación, publicación y continuación

Actualización operativa del 2026-09-12 (#757): consulta `AGENTS.md` y
`docs/operations/validation-v2.md` del checkout actualizado para la selección de caché,
limpieza de disco y seguimiento de tareas. Las copias históricas del plan de abajo
no sustituyen esas instrucciones mantenidas.

Objetivo de rendimiento solicitado después (#759): que la campaña completa de validación
local termine realmente en un máximo de 10 minutos. Registra tiempos y compruébalo en una
entrega representativa; escribir este límite, cortar un proceso o quitar pruebas no lo cumple.
Usa la instrumentación de la pipeline para localizar el coste en esa misma ejecución, sin
otra compilación de calentamiento. Separa el tiempo de implementación/corrección de errores
y declara cualquier coste extraordinario: no lo escondas fuera de la cifra de aceptación.

Completa la entrega antes de su campaña final. Ejecuta las pruebas afectadas, integración
de producción y controles de arquitectura/QA aplicables. Conserva las condiciones de
lifetime, ownership, persistencia y publicación, incluso en saturación o fallo parcial.

Planifica una sola campaña por candidato: `final` ya comprueba consumidores afectados y
ejecuta las suites completas de las bibliotecas modificadas. Aprovecha los casos que esas
suites ejecutan y añade la integración, ownership y QA que falten; no repitas las mismas
compilaciones o suites como calentamiento. Agrupa las correcciones de fallos antes de
revalidar. No uses ciclos de Cargo para descubrir o editar consumidores campo por campo.

La opción de #759 está integrada en #761 (`477302cf`): usa `final --architecture --timings` si la
aceptación requiere arquitectura, sus fixtures y ownership de sintaxis. Reúne esos controles
en el manifiesto final y evita repetirlos aparte. Mantén la integración de producción,
inventario exhaustivo y QA en vivo que exija la issue; cuenta su duración adicional.
No cierres #759 por instrumentación ni por una repetición sin cambios ya compilados.

Usa un único `CARGO_TARGET_DIR` por worktree, también para los manifiestos independientes,
y un trabajo de Cargo inicial en este host; aumenta solo con medición y margen disponible.
La campaña de #756 verificó esta configuración en 409,679 s (6 min 50 s), con 4.614 tests
Rust pasados y uno ignorado, en `e7f3bec9`; incluye arquitectura y ownership. #759 conserva
la evidencia. Usa dos trabajos con validación exclusiva, al menos 12 GiB de MemAvailable
y 30 GiB libres de disco; sigue registrando resultado y memoria en cada entrega.
Conserva la caché incremental activa. Si falta disco,
inspecciona primero artefactos antiguos e inactivos y protege fuentes, despliegues y evidencia.
Ejecuta los validadores sin pipes a `tail`, `head` o `grep`; conserva su código de salida.
Si el harness devuelve un ID de tarea en segundo plano, sigue esa tarea hasta su resultado.
No la dupliques, no conviertas un timeout de espera en un fallo supuesto ni atribuyas
un `SIGTERM` a falta de memoria sin evidencia. No eludas controles del harness.

Los cambios de protocolo observable necesitan evidencia de bytes/metadatos/conexión/orden.
La durabilidad real exige los escenarios de DB y recuperación correspondientes. Los mocks,
un laboratorio o un green genérico no sustituyen la aceptación específica.

Usa `validation-v2 final` sobre el candidato comprometido y verifica su manifiesto.
Registra SHA, comandos, host, resultados y límites reales. Reutiliza evidencia solo bajo
las reglas documentadas; nunca atribuyas una ejecución anterior a otro SHA.

Al cerrar cada entrega:
1. Comprueba aceptación completa, diff y ausencia de datos sensibles.
2. Publica la rama y crea o actualiza la PR, con `Closes #<issue>` cuando corresponda.
3. Atiende hallazgos aplicables, resuelve conversaciones y comprueba los requisitos reales.
   Para PRs de autor exacto `alseif0x`, aplica la política local vigente; los jobs
   configurados como omitidos no equivalen a checks ejecutados.
4. Mergea con el método permitido y comprueba el commit integrado y el cierre correcto.
5. Actualiza el estado y los documentos propietarios. Mantén #49 como índice de larga vida.
6. Continúa con la siguiente responsabilidad preparada. Una PR o un commit aislado
   no son el objetivo final de este encargo.

### Continuidad y bloqueos

No termines con «puedo continuar», «siguiente paso» o una lista de trabajo pendiente
cuando aún puedas ejecutar trabajo autorizado. Mantén actualizaciones breves y un checkpoint
útil ante interrupciones; no crees planes paralelos ni reinicies el trabajo por una compactación.

Un problema técnico resoluble se investiga y corrige. Si falta un productor real,
entrégalo primero. Si un bloqueo externo impide una ruta, registra causa, evidencia y
condición de desbloqueo y continúa el trabajo independiente permitido.

No repitas intentos idénticos sin nueva evidencia, no finjas acciones bloqueadas y no
declares una macro aceptada mientras conserve requisitos obligatorios pendientes.
La intervención humana queda limitada a impedimentos externos que no puedas resolver
con la autorización y capacidades disponibles.

### Consulta técnica a Codex (acordada el 2026-09-12)

El usuario ha delegado en Codex las decisiones técnicas materiales del plan. Si una
decisión no queda resuelta con evidencia local, consulta a Codex antes de trasladar
otro menú al usuario. El canal es una sesión asesora de Codex CLI con acceso de
solo lectura al checkout; no despierta automáticamente la conversación original.
La conexión fue probada en este host con la cuenta existente y devolvió el SHA
observado. Cada consulta necesita aportar su propio contexto.

Prepara una pregunta sin secretos con: issue y SHA actuales, responsabilidad completa,
fuentes C++ exactas, lectores/escritores y llamadas, alternativas, pruebas o errores
observados y decisión concreta necesaria. No cuentes coincidencias desde salida
truncada. Incluye límites de autorización y cualquier nueva evidencia que contradiga
el plan. Pide recomendación, motivos, condiciones de aceptación y riesgos pendientes.

Desde el checkout activo, usa un directorio temporal distinto por consulta:

```bash
consult_dir="$(mktemp -d /tmp/rustycore-consult.XXXXXX)"
# Escribe la pregunta anterior en "$consult_dir/question.md" antes de ejecutar.
timeout 600s codex -a never exec --ephemeral \
  --ignore-user-config --ignore-rules --disable hooks --disable plugins --disable apps \
  --json --color never --sandbox read-only -C "$PWD" \
  -c 'model_reasoning_effort="high"' \
  -c 'developer_instructions="Eres el asesor tecnico de RustyCore. Solo lectura: inspecciona el codigo y referencias locales pertinentes. No modifiques archivos, no ejecutes builds, tests, QA, acceso a servicios ni otras sesiones o agentes. Resuelve la pregunta con evidencia actual y distingue recomendaciones de resultados probados. No afirmes disponer del contexto de otra conversacion. No amplias autorizaciones del usuario ni eludes controles del harness."' \
  -o "$consult_dir/answer.md" - \
  < "$consult_dir/question.md" \
  > "$consult_dir/events.jsonl" 2> "$consult_dir/stderr.log"
consult_rc=$?
```

Comprueba `consult_rc`, la respuesta y sus referencias. Una salida no válida, un
timeout o una denegación no son una decisión. No publiques los registros completos:
pueden contener fragmentos del repositorio o datos sensibles. Conserva el diagnóstico
preciso y sigue trabajo independiente si el canal falla; no eludas el harness.
Aplica la recomendación dentro del alcance autorizado tras contrastar sus premisas.
Si discrepas por evidencia nueva, repregunta con esa evidencia. Registra la decisión
aceptada en la issue/ADR/checkpoint propietario, sin crear otro plan competidor.
La opinión del asesor no reemplaza tests, QA, revisión ni aceptación para merge.

Para #787 la decisión tomada es un paso coordinado que preserve el orden del tick:
la tarea de sesión conserva su propiedad exclusiva; el coordinador cede en la fase
de sesiones, sin guards síncronos, espera su terminación definida y continúa las
fases restantes del mismo tick. No se aprueba la propuesta A original de ejecutar
Map después de completar todo el tick. El contrato requiere resolver fases World/Map,
residencia/incarnación, cancelación, admisión, esperas y cercas de persistencia con
evidencia; está aprobado como dirección de implementación, no declarado validado.


## Copias completas del plan integrado

Las cuatro partes siguientes conservan íntegramente el contenido de sus documentos
fuente en `af5882cc`. Solo se adaptan niveles de encabezados y enlaces relativos para
reunirlos en un archivo. Los relatos históricos conservan sus fechas y límites:
no constituyen nuevas instrucciones ni nueva evidencia de gameplay.

---

<a id="plan-general"></a>

## A. Plan general completo

Fuente mantenida: [docs/migration/PORT_PLAN.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md).

### RustyCore — Master port and delivery plan

**Reconciled 2026-09-11 under #748 / [master index #49](https://github.com/alseif0x/rustycore/issues/49).**
Source baseline: `3.4.3` at `5d8c079a06b587c060c1c6e1c06bedb73c4339d0`.
Initial inventory: **46 open issues**, all given a disposition below; #748 is this
bounded planning delivery. Administrative consolidation does not count as implementation.

The target remains **full functional parity with the TrinityCore-derived WoW 3.4.3
server**, with the approved native/Wasm module product. A playable milestone is an
intermediate acceptance point, not a smaller replacement target.

#### 1. Direction from here

**Next primary implementation: #743, reliable application/reconciliation of group
state. Next preferred core delivery: #735, reputation encapsulation.** This is a
priority choice based on demonstrated residuals, not a dependency between those
issues. Neither requires rebuilding the integrated group or Player owners.

Continue the remaining core under #584 by complete operations, execution/lifetime
boundaries and physical organization. In parallel with safe independent work, prepare
a playable circuit: effective equipment/stats → combat and death/recovery →
quests/loot/interactions → complete class kit, travel and durable services → soak.

The core/module acceptance chain remains **required #584 core → #583 → #153**.
The former umbrella #133 is already closed. Its closure is neither evidence that this
chain passed nor an extra future task. The module product need not block every
independent gameplay delivery; its production activation still requires its actual
core prerequisites. Complete the approved architecture/module acceptance before
declaring the whole Part-1 program accepted at #47.

The M0–M6 headings remain milestone identifiers; obsolete `[NN]` priority prefixes
are retired. Neither is an unconditional execution order. Issue numbers, crate names and file counts
do not define dependencies. The tables below distinguish preferred order from
capabilities that actually block a consumer.

#### 2. Authority, evidence and limits

- This file and #49 own overall direction and issue allocation.
  [STATE.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md) owns dated implementation/evidence status.
- [The refactor completion plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/refactor-completion-plan.md)
  owns the detailed P0–P6 continuation; [modularity/ECS design](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-and-ecs-plan.md)
  and [module design](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md) own contracts and budgets.
  Issue bodies define bounded delivery and acceptance, not competing master plans.
- Required base behavior comes from target C++ at
  `/home/server/woltk-trinity-legacy`, SHA
  `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, and appropriate target-build captures.
  The pinned complementary AzerothCore reference and its limits are in
  [docs/README.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/README.md); 3.3.5 wire/SQL are not substitutes for 3.4.3.
- This review read every open issue and traced representative current production
  paths, relevant C++ and existing evidence. It is **not** a full opcode/effect/DB2
  census, a fresh live scenario, a durability proof or a new whole-port parity base.
  Partial source findings and hypotheses are distinguished in the updated issues.
- No historical percentage, count of implemented enums, passing mock or closed
  umbrella proves present capability. Preserve the exact source/runtime identity
  of previous tests and captures; do not relabel them as testing this plan.

#### 3. Integrated foundations to reuse

These predecessor issues are closed. Reuse their integrated responsibilities and
inspect their recorded limits when changing a consumer; do not repeat them from an
old unchecked checklist.

| Foundation | Closed issue scope and retained boundary |
| --- | --- |
| Data, terrain, LOS, respawn, persistence and integrity | #14–#20, #52, #60, #62, #64; their bounded delivery does not prove every store/stat/save path. |
| World entry, packet handling and movement/AI | #7–#11, #21–#26, #50, #53, #57, #66. #26 proves its bounded creature cast wire/lifecycle, not full effects or combat AI. |
| Runtime clock cuts and homebind | #28/#371 and #44. They do not complete runtime convergence, all transfers or item-use. |
| Persistence/migration and module foundation | #169/#574/#256 and #228–#231. Reuse SQLx-free contracts, the migration authority and the narrow external login API. |
| Canonical Player, finalization, acquisition, visibility and cast | #578/#585/#587/#588/#589. Preserve residence/incarnation, save fences and metadata; broader gameplay remains open. |
| Recent architecture repair | #716 analyzer; #718 represented quest-reward transaction; #722 named Player operations; #737 item-runtime ownership. Real reward recovery evidence and generic mutation residuals are not discharged by these closures. |

The old high/medium findings in [EXISTING-CODE-DEFECTS.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/EXISTING-CODE-DEFECTS.md)
are leads with dated evidence. Contrast them before making them current blockers;
retain their owning capability even if a diagnosis proves obsolete.

#### 4. Execution lanes and dependencies

A lane describes a result and its acceptance. It does not create one PR per row,
force unrelated work to wait, or permit a partially completed macro to be closed.

| Lane | Preferred work | Entry / exit contract |
| --- | --- | --- |
| **A — Core architecture** | #743, #735, then measured P2/P3/P4 residuals under #584 | One canonical authority and execution owner, complete consumers, explicit lifetime/persistence/publication and terminal physical/dependency dispositions. See §6. |
| **F1 — Character foundations** | #61, #63; relevant #12, #486 and #524 corrections | Equipment reaches effective stats; movement and entry consume the integrated owners. Weather or all vehicles do not block unrelated combat. |
| **F2 — Combat and recovery** | #29, residual #30, #31; consolidated #43 and #54 | A real fight can reach death, release/recovery and safe logout. Accept numeric combat only with effective stats and the necessary aura/absorb participants. |
| **F3 — Progression and interaction** | consolidated #41; #55, #56, #13, #36 and #51 | Accept → progress → complete/reward, real loot and GO/item interactions. Shared consumers must compose; queues or recorded requests alone do not pass. |
| **F4 — Complete class/simulation kit** | #32, #33, #34, #35, #27 | Executable effect/aura/proc/AI chains with lifecycle and observable outcomes. A participant needed by F2/F3 is delivered there, not delayed merely because its family is listed here. |
| **F5 — Travel and durable services** | #40/#45, remaining #12; #37, #38, #39 | Flight/instance/reconnect coherence; mail, trade and AH complete their transactional lifecycles. These may advance when their real prerequisites are ready. |
| **X — Stateful extension product** | #583 under #99, then #153 audit | Required core accepted first; real native/Rust-Wasm/C-Wasm/mixed behavior, durable module state/reward and author/operator lifecycle. Wider #99 expansion is separate. |
| **F6 — Part-1 terminal acceptance** | #46, then #47 | Integrated functional scenarios, multi-client soak and real save/restart/relogin under load; no known integrity failures deferred to soak. |
| **O — Environment and QA support** | #255, #351; #279/#352 when applicable | Reproducible setup and usable guarded QA. A specific broken fixture/DNS target blocks its live scenario, not every local implementation. |
| **B — Bounded side delivery** | #582 | Reconcile and integrate the existing decoder branch; it does not activate or complete matchmaking. |
| **D — Optional developer experience** | #260 | Typed configuration foundation while preserving current .conf behavior; not a prerequisite for #231, #583 or gameplay. |
| **L — Full-parity continuation** | #48 and source-evidence index #65 | Preserve all remaining target behavior. Refresh the whole-port plan at #47 before creating a detailed Part-2 child tree. |

##### Hard capability relationships

- Equipment → effective Unit/Player stats (#61) precedes accepting damage numbers
  in #29/#31. Analysis and preservation of existing combat paths may proceed earlier.
- #13 and #36 use the shared cast contract delivered by #589 and extended by #30;
  each owns its GO/item admission and side effects. They do not clone cast ownership.
- #31 supplies damage/heal application to periodic/proc consumers; required aura,
  absorb or trigger participants are integrated with the affected operation.
  These are shared capability contracts, not a circular demand that all of #31
  and all of #32 must each close before the other can start.
- #43 supplies the complete death/recovery cycle used by combat, instance and
  resurrection scenarios. #54 extends logout admission and reuses #585 finalization.
- #41/#55/#56 share objective-credit integration, with one owner for each
  transition. #743 supplies group-state consistency; #51 retains protocol/lifecycle
  acceptance. Neither relation justifies merging unrelated responsibilities.
- The reward-by-mail criterion of #41 and auction delivery in #39 require the
  relevant durable mail capability of #37. Quests without mail may be developed
  and tested earlier; **#41 must not close while its required mail criterion is
  missing**. Schedule that producer before final quest closeout. Trade #38 does
  not require AH or mail.
- #583 waits for required #584 core; #153 waits for accepted #584 and #583.
  #99 is a product umbrella, not an implementation prerequisite that must close.
- #46 starts after the selected functional flows work; #47 requires their complete
  Part-1 acceptance and #46. Failures found while building an operation are repaired
  there rather than being stored until F6.

#### 5. Complete disposition of the initial open issues

“Revalidate” means a scoped current-source/behavior check at the start of delivery,
not another project-wide audit. A consolidation is closed as superseded, with all
acceptance retained by the recipient; it does not mark functionality complete.

| Issue | Primary owner / lane | Disposition and concrete next scope |
| --- | --- | --- |
| [#12](https://github.com/alseif0x/rustycore/issues/12) | F1/F5, entry/transitions | Revalidate transport attachment/re-seat and weather/world-state presentation; transport columns are already loaded. Track remaining entry/cinematic/hotfix presentation findings here when reproduced. |
| [#13](https://github.com/alseif0x/rustycore/issues/13) | F3, GO-use | Integrate the real spell/portal operation from existing GO dispatch and effective templates/scripts. |
| [#27](https://github.com/alseif0x/rustycore/issues/27) | F4, creature reactions | Trace Talk/text loading and AI/script consumers; deliver event, locale/range and multi-recipient behavior. |
| [#29](https://github.com/alseif0x/rustycore/issues/29) | F2, melee | Complete calculation/outcomes with equipment stats and both Player/Creature consumers; reuse current timers/owners. |
| [#30](https://github.com/alseif0x/rustycore/issues/30) | F2, cast prerequisites | Complete residual CheckCast, resources/reagents and history/cooldowns around integrated #589. |
| [#31](https://github.com/alseif0x/rustycore/issues/31) | F2, damage/heal | Calculation → canonical application → threat/death/publication, with exact modifier and failure behavior. |
| [#32](https://github.com/alseif0x/rustycore/issues/32) | F4, aura lifecycle | Apply/update/periodic/remove, real modifiers and required persistence; no historical aura-count target. |
| [#33](https://github.com/alseif0x/rustycore/issues/33) | F4, procs | Trace real events/trigger consumers, filters, RNG/chance/PPM/charges and recursion/failure. |
| [#34](https://github.com/alseif0x/rustycore/issues/34) | F4, spell effects | Complete selected class chains with their targets, auras, movement and summons; preserve full-port coverage separately. |
| [#35](https://github.com/alseif0x/rustycore/issues/35) | F4, channels/missiles/areas | Runtime lifecycle and effects, beyond the existing packet shapes; reuse the execution clock. |
| [#36](https://github.com/alseif0x/rustycore/issues/36) | F3, item-use | ItemEffect is loaded; deliver registered item admission, cast, charges/consumption and real effects. |
| [#37](https://github.com/alseif0x/rustycore/issues/37) | F5, mail | Complete online/offline mail, attachments/money/COD and durable recovery; supply reward/AH consumers. |
| [#38](https://github.com/alseif0x/rustycore/issues/38) | F5, trade | Complete two-Player item/gold exchange beyond the existing accepted-state protocol. |
| [#39](https://github.com/alseif0x/rustycore/issues/39) | F5, AH | Complete target-version auction lifecycle and mail delivery, not a response invented for a legacy no-op opcode. |
| [#40](https://github.com/alseif0x/rustycore/issues/40) | F5, taxi | Discovery/route/payment → flight/map transitions → landing/recovery; existing taxi state is not flight execution. |
| [#41](https://github.com/alseif0x/rustycore/issues/41) | F3, quest lifecycle | Receives #58/#59: admission, acceptance/sharing/source items, objectives, completion and full in-scope reward; reuse #718. |
| [#42](https://github.com/alseif0x/rustycore/issues/42) | Superseded by #43 | Transfer death, durability and ghost restrictions into the complete recovery macro; close administratively, not as implemented. |
| [#43](https://github.com/alseif0x/rustycore/issues/43) | F2, death/recovery | Receives #42: death/CORPSE → release/ghost → corpse/graveyard → reclaim/healer/resurrection and relog. |
| [#45](https://github.com/alseif0x/rustycore/issues/45) | F5, instances | Revalidate bind/save/difficulty/reset and actual admission; finish enter/leave/reconnect using existing transfer/finalization. |
| [#46](https://github.com/alseif0x/rustycore/issues/46) | F6, soak | Measured multi-client stability on an identified installed build, after incremental operation acceptance. |
| [#47](https://github.com/alseif0x/rustycore/issues/47) | F6, playable exit | Real save/recovery under load and all required functional flows; triggers the next full-port planning pass. |
| [#48](https://github.com/alseif0x/rustycore/issues/48) | L, full-parity umbrella | Retain complete coverage, remove stale percentages; add explicit social/LFG and authentication/network coverage. |
| [#49](https://github.com/alseif0x/rustycore/issues/49) | Master index | Maintain this direction and complete allocation; not an implementation PR or a future prerequisite to its children. |
| [#51](https://github.com/alseif0x/rustycore/issues/51) | F3, group lifecycle | Revalidate current invite/accept/decline/cancel/leave/disband/category behavior; separate from #743 delivery guarantees. |
| [#54](https://github.com/alseif0x/rustycore/issues/54) | F2, logout admission | Deny/delay/instant/cancel/countdown using the existing durable finalizer. |
| [#55](https://github.com/alseif0x/rustycore/issues/55) | F3, loot | Existing gates are not globally missing; finish actual modifiers/grants/credit and multi-client consistency. |
| [#56](https://github.com/alseif0x/rustycore/issues/56) | F3, area-trigger | Bits, conditions, scripts and tavern paths exist; trace residual explore/BG/corpse/transfer semantics. |
| [#58](https://github.com/alseif0x/rustycore/issues/58) | Superseded by #41 | Preserve timed-active exclusivity and recursive breadcrumb admission as explicit quest criteria. |
| [#59](https://github.com/alseif0x/rustycore/issues/59) | Superseded by #41 | Preserve acceptance/completion/reward participants and evidence, without redoing #718. |
| [#61](https://github.com/alseif0x/rustycore/issues/61) | F1, equipment/stats | Trace existing modifier planning into effective stats and reversibility before accepting combat numbers. |
| [#63](https://github.com/alseif0x/rustycore/issues/63) | F1, movement | Complete residual mover/transport/vehicle/teleport branches; preserve #588 deferred visibility. |
| [#65](https://github.com/alseif0x/rustycore/issues/65) | L, source-evidence index | Correct finding/issue status and retain exact C++ provenance; not a separate implementation queue or fresh count. |
| [#99](https://github.com/alseif0x/rustycore/issues/99) | X, module ecosystem | #583 is the selected stateful product; wider language/WIT/hot-reload proposals remain later capability-led planning. |
| [#153](https://github.com/alseif0x/rustycore/issues/153) | X/A, terminal audit | Audit accepted #584/#583 and their evidence; do not absorb known implementation work or await #133 reopening. |
| [#255](https://github.com/alseif0x/rustycore/issues/255) | O, reproducible bootstrap | Reuse integrated migration/status authority; deliver pinned artifact, cache/offline import and setup diagnostics. |
| [#260](https://github.com/alseif0x/rustycore/issues/260) | D, typed JSON config | Optional bounded foundation on the current toolchain; keep .conf/overlays/environment semantics and offline validation. |
| [#279](https://github.com/alseif0x/rustycore/issues/279) | O, QA credential rotation | Explicit disposable-fixture recovery with DB/file failure handling; create-only provisioning remains the default. |
| [#351](https://github.com/alseif0x/rustycore/issues/351) | O, guarded loot QA | Reconcile current runtime/capture orchestration and chest fixture ownership; preserve restore guarantees and prove the actual smoke. |
| [#352](https://github.com/alseif0x/rustycore/issues/352) | O, realm address operations | Revalidate configured DNS/IP and restart diagnostics. No silent fallback or code change inferred from the historical incident. |
| [#486](https://github.com/alseif0x/rustycore/issues/486) | F1, target identity query | Return target game/BNet account identities via the canonical cache/connected target; current querying-session IDs are wrong. |
| [#524](https://github.com/alseif0x/rustycore/issues/524) | F1, skill startup order | Correct table-granular base/official/custom order and failure/publication phases across the current loader/port. |
| [#582](https://github.com/alseif0x/rustycore/issues/582) | B, existing LFG decoders | Resume local branch at `607e9bb4` / code `bef2d707`, reconcile and validate before integration; no matchmaking claim. |
| [#583](https://github.com/alseif0x/rustycore/issues/583) | X, stateful native/Wasm | Deliver the preserved M0–M4 product after required core; the external login API and laboratory are insufficient. |
| [#584](https://github.com/alseif0x/rustycore/issues/584) | A, core coordinator | Own remaining P2/P3/P4 and C0–C4 dispositions; select finite complete implementation macros from current consumers. |
| [#735](https://github.com/alseif0x/rustycore/issues/735) | A, reputation boundary | Encapsulate domain transitions; resolve catalogs and construct packets outside Player; migrate save/load/publication consumers. |
| [#743](https://github.com/alseif0x/rustycore/issues/743) | A, group consistency | Guarantee application or reconciliation despite saturation, disconnection, replacement and stale commands. |

**Planning delivery:** [#748](https://github.com/alseif0x/rustycore/issues/748) owns
this documentation/issue reconciliation and its validation. Closing it does not close #49
or any gameplay acceptance. #42/#58/#59 retain their history and redirects to recipients.

#### 6. Finish architecture without another endless rewrite

##### A1 — Group consistency and reputation

#743 is a demonstrated dropped-state-change path. Resolve the bounded group
command/reader contract and saturated/replaced-target cases, not every mailbox in
the server. #735 is a domain encapsulation residual, not proven concurrent double
ownership: the current reconstruction occurs synchronously under canonical Player
access. Move rules/state transitions, not the packet/catalog-dependent manager wholesale.

##### A2 — Remaining application and persistence boundaries

Use the actual generic Player/item mutation callers in the ownership ledger.
For each complete operation, migrate its rules, readers, writers, persistence and
publication; retire broad access or justify a bounded stable seam under the existing
policy. Replacing a closure with a differently named generic closure is not retirement.
A short canonical access adapter is not automatically an independent gameplay owner.

Preserve #718's represented quest transaction and #585's finalization. Complete the
remaining account/character, save/acknowledgement, unknown-COMMIT, cancellation and
recovery obligations where their promised contract is not discharged. Real DB/restart/
relogin evidence remains necessary; source guards and controlled futures alone are
not a durability claim. Unrelated missing gameplay is allocated to its functional macro.

##### A3 — Production execution, lifetime and private storage

Trace startup and the current Session/map/legacy calls, using the dated
[clock/phase trace](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/runtime-clock-phase-trace.md) as a starting point.
The current source still selects GlobalLegacy for its creature path and starts the
canonical map loop; that is not by itself a demonstrated double tick.
Reconcile admission, phases/barriers, one resolution, backpressure and
transfer/detach/unload/shutdown before removing a bridge.

Keep the selected private hecs direction and finite V2 conformance evidence.
Integrate it only with real owners/consumers and the lifetime/reentry contract;
no global ECS, public raw storage API, extra runtime clock or dependency-only “migration.”
Define the next finite implementation contract from these traced transitions under #584,
not a speculative issue per bridge or crate.

##### A4 — Physical and dependency closeout

Apply the same semantic/physical policy to production, tests, fixtures, adapters,
composition and tools. Replace arbitrary numerical file partitions with cohesive
responsibilities when closing their family; do not conduct a blind global rename.
Private modules precede earned crates. Preserve visibility, exact registration sets
and persistence inventories while retiring the real legacy accesses.

Migration non-growth is not terminal acceptance. Measure remaining oversized files,
logical owners, permitted dependency exceptions and production bridges at closeout;
every residual needs the policy's specific accepted disposition. #153 audits that
result; it is not the implementation owner of known splits.

#### 7. Part 1 acceptance — a complete playable circuit

The M0–M6 names remain milestone identifiers. Their broad exits cannot be inferred
from a child issue's closure or from a single working spell/class.

| Milestone | Required integrated outcome |
| --- | --- |
| M0 foundations | Effective data/stats, terrain/LOS/pathing, respawn and persistence/integrity prerequisites used by the selected scenarios. Preserve accepted predecessor coverage and identify residuals. |
| M1 entry | Create/login, bags/UI and correct identity/initial publication, with no client/Lua failures and scoped capture-clean entry flows; #12/#486 own relevant remaining paths. |
| M2 world | Movement, visibility, patrol/path/aggro/evade, abilities, reactions and respawn; multiple clients agree. Bounded #26/#28 acceptance is not this full exit. |
| M3 combat/classes | Normal class rotations, melee/spell outcomes, resources/cooldowns, periodic effects/CC/procs and required channels/areas, with effective stats and correct death/recovery consequences. |
| M4 interaction | GO/item-use, common quest objective and reward types, groups/loot, mail/trade/AH and flights work end-to-end. Earlier quest development does not remove commercial services from this exit. |
| M5 lifecycle | Death/release/corpse/resurrection, homebind, transport/weather/world state, instances and deny/delay/cancel logout; reconnect/relogin preserve the correct outcome. |
| M6 stability | Multi-session/map soak and actual periodic-save/recovery under load; no lost acknowledged state, duplicate grant, known unresolved integrity defect or unbounded queue failure. |

At #47, record a concrete scenario matrix with target client/build/data, representative
class/race/level/map coverage and every required milestone exit. One successful class
or login is evidence for that case, not all Part 1. Preserve wider target coverage for
Part 2; any scope adjustment needs an explicit retained owner, not a smaller headline.

The environment for required QA must be reproducible and its addresses/fixtures usable.
That requirement is not a claim that all optional authoring tooling (#260), the full
#99 ecosystem or a production LFG queue must be finished to call Part 1 playable.

#### 8. Part 2 — full target parity, with an explicit coverage map

#48 retains the complete target beyond the playable cases. The old numeric
baselines (385/631 handlers, 42/150 effects, 5/255 auras, 110/325 stores and script LOC)
are historical and are retired as current status or fixed acceptance denominators.
Measure the target-version inventory and real consumers when the relevant domain is
audited. A registered opcode, represented request or loaded table alone is not coverage.

| Ledger | Required target coverage |
| --- | --- |
| L1 | Client packet handlers, metadata/admission, operation and errors. |
| L2 | Server packet layouts, values, recipients, connection and observable order. |
| L3 | Spell effects and their complete application chains. |
| L4 | Aura types, periodicity, stacking/removal, modifiers and proc interactions. |
| L5 | Required DB2/DBC/GameTable catalogs, effective overlays/removals and consumers. |
| L6 | Creature AI families and lifecycle, beyond bounded combat slices. |
| L7 | SmartAI event/action/target execution and script integration. |
| L8 | Movement generators and transport/taxi/vehicle integration. |
| L9 | Terrain, pathfinding, LOS, collision and required extraction/data support. |
| L10 | Conditions and their actual operation consumers. |
| L11 | Phasing and visibility refresh across state/lifecycle changes. |
| L12 | Complete item/inventory/bank/equipment/durability/buyback/gift behavior. |
| L13 | Complete quest rules, objective types, rewards and persistent state. |
| L14 | Mail, calendar, petitions and their delivery/lifecycle. |
| L15 | Target-version auction house and associated behaviors. |
| L16 | Battlegrounds, arenas, battlefields and outdoor PvP. |
| L17 | Instances, save/bind/difficulty/lockouts/resets and raids. |
| L18 | Achievements, reputation, skills, talents/glyphs, titles and progression. |
| L19 | Pets, vehicles, totems and related AI/state/lifetime. |
| L20 | Required first-party content scripts by family, independently of optional modules. |
| L21 | Warden/anticheat and target-supported enforcement. |
| L22 | UpdateField values and derived state, not only layouts. |
| L23 | Supported server configuration and effective runtime consumers. |
| L24 | Database statements/loaders, schemas, transactions and recovery. |
| L25 | Remaining production runtime, grids, visibility/object updates and legacy retirement. |
| L26 | Source-reference verification, including the historical #65 findings. |
| **L27** | **Explicit social coverage:** groups/raids, guilds, friends/ignores/channels and automatic LFG, beyond #51/#582. |
| **L28** | **Explicit authentication/network coverage:** bnet/world account and realm flows, session/connection lifecycle and target-required security/protocol behavior. |

L27/L28 make previously implicit coverage visible; they do not create child issue
trees or promise unrelated platform features. The map is a planning coverage aid,
not proof that every target operation has already been inventoried.

##### Part 2 transition gate

This review establishes the full direction **now**. After #47/M6.2, refresh the
whole-port inventory against that later integration and versioned C++/data/captures,
then decompose the remaining L-ledgers into complete implementation macros.
Do not create hundreds of speculative child issues today. Existing justified
functional work may proceed earlier through its actual dependencies.

The owner closing #47 records the new review base and hands the remaining scope
to #48/#49. Reconcile delivered behavior and evidence before calculating any coverage
denominator or proposing a first Part-2 implementation.

#### 9. Delivery, validation and maintenance

1. Select a ready complete responsibility from this plan. Reuse its issue/branch;
   declare exact operation, current consumers, source/data contract, dependencies,
   retirement list and acceptance. Distinguish unknowns from established gaps.
2. Finish the implementation, consumers and tests with coherent internal commits.
   Keep behavioral repair separate from structural movement. No issue/PR per helper,
   routine approval round or partially implemented macro passed off as complete.
3. At delivery acceptance, run affected unit/production-linked/failure tests,
   architecture/metadata checks and the applicable final profile. The parent or one
   assigned executor schedules heavyweight validation sequentially on the shared host.
4. Changed bytes/metadata/connection/order need scoped packet/capture evidence;
   new action-specific live scenarios differ from regression goldens. Randomized
   combat needs controlled inputs/RNG or justified distribution/causal checks.
   Real durability claims need real DB/restart/relogin evidence.
5. Update the owning issue/checkpoint and current status with exact tested SHA,
   command, host, result and limits. Remove superseded accesses/baselines only from
   reviewed semantic evidence. Preserve prior evidence identities and user work.
6. Close an implementation issue only after integrated scoped acceptance. Close
   superseded tracking issues administratively with full scope transferred and
   backlinks. Do not close #49 merely because a planning PR lands.
7. Reconcile this index after a macro lands, a dependency changes or a concrete new
   defect alters priority. Do not append another contradictory “current plan.”
   Urgent integrity faults move ahead of preferences; an unverified historical
   diagnosis does not silently become a new global blocker.

Use [AGENTS.md](https://github.com/alseif0x/rustycore/blob/3.4.3/AGENTS.md) and [validation-v2](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/operations/validation-v2.md)
for the actual commands, authority and evidence reuse. Planning changes create no
new runtime result. Report **implemented**, **integrated** and **parity-proven**
separately; do not replace acceptance with percentages of files, fields or closed issues.

---

<a id="refactor"></a>

## B. Continuación técnica completa del refactor

Fuente mantenida: [docs/architecture/refactor-completion-plan.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/refactor-completion-plan.md).

### Plan técnico para completar la arquitectura de RustyCore

**Sincronización de la entrega #748 — 2026-09-11.** Este documento detalla los
límites técnicos de la dirección general que mantienen `docs/migration/PORT_PLAN.md`
y GitHub #49. No es un plan de issues alternativo: el índice macro, sus lanes y sus
dependencias viven en el plan de port; aquí se fijan propietario, consumidores,
anclas C++, orden de ejecución y criterios de aceptación de la arquitectura.

El alcance de esta sincronización es documental. No reabre el análisis completo del
port, no inventa nuevas microissues y no convierte una prueba histórica en evidencia
nueva. Cada macro incluye sus consumidores y sus pruebas; la validación final sigue
la cadencia de `AGENTS.md`.

#### 1. Estado que gobierna el plan

La base revisada de esta entrega es `3.4.3` en
`5d8c079a06b587c060c1c6e1c06bedb73c4339d0`. #133 se cerró el 2026-09-09. Las
entregas #578, #585, #587, #588, #589, #716, #718, #722 y #737 están integradas y
cerradas dentro de sus alcances acotados. No se debe esperar otro cierre de #133 ni
reabrir esas entregas por una preferencia de nombres o por una frontera pendiente.

La compuerta técnica sigue siendo:

```text
#584 núcleo (C0–C4)
    → #583 producto M0–M4 de módulos nativos/Wasm, Rust/Wasm/C y lifecycle de operador
    → #153 auditoría independiente
```

El producto Rust/Wasm/C mixto es obligatorio aunque el operador pueda dejar Wasm
desactivado. #583 espera los requisitos de núcleo necesarios para su integración de
producción, pero no bloquea cada macro de gameplay independiente. El cierre
administrativo de #133 no añade una compuerta posterior.

La próxima macro de núcleo recomendada es #743, después #735 como preferencia de
orden sin dependencia dura, y luego los residuales P2 seguidos por P3 (runtime,
lifetime y `hecs` privado) y P4 (fronteras semánticas y organización física). Las
excepciones físicas son individuales y se justifican con la política vigente; no se
crea una issue por fichero, helper o import.

#### 2. Evidencia y límites actuales

La evidencia fechada debe conservar su SHA, tipo y límites:

| Evidencia | Estado que puede sostener |
| --- | --- |
| #718 / `78276ddf57463fc0f568c1c4bcf84d619af68cad` | La recompensa representada usa una transacción coherente de personaje y tiene contrato de COMMIT ambiguo. No hubo escritura real de DB, reinicio ni relogin QA. |
| #716 / `6ae62d73a9f910ebd664d82e331520b836a73c77` | El analizador y sus inventarios fueron aceptados localmente; los 363 tests del analizador son evidencia histórica, no una ejecución de #748. |
| Conformidad hecs V2 | Pasó dentro de los límites del laboratorio registrados en `modularity-conformance-results.md`. Producción no tiene instalado `hecs` ni Wasmtime en esta base. |
| Traza de relojes | La traza fechada de seis relojes orienta la investigación; no prueba un nuevo inventario exhaustivo ni doble tick. |
| Organización física | La medición fechada de 31 archivos por encima de 2.000 líneas y 63 por encima de 1.000 sigue siendo deuda P4. No es una medición nueva ni se deben refrescar techos para ocultarla. |

El contrato de la recompensa y sus participantes no implementados permanece en
[quest-reward-operation-contract.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/quest-reward-operation-contract.md). Sus anclas
Classic son `Player.cpp:14625` (`RewardQuest`), `SaveToDB(false):14867`, la
transacción de personaje en `SaveToDB:19312` y el correo separado en `14794`.
El cierre de #718 no demuestra durabilidad real ni completa el port de participantes
no representados.

#### 3. Propiedad y estructura que deben conservarse

El objetivo es un monolito modular con un único propietario mutable por concepto:

1. Player, Map y sus submódulos conservan estado e invariantes. Un Player detached
   sigue teniendo dueño válido si su incarnation/residence lo permite.
2. La aplicación coordina una operación completa mediante capacidades concretas.
   Session adapta admisión y transporte; no se convierte en un contexto universal.
3. Los handlers decodifican, admiten e invocan. Los paquetes se construyen en la
   frontera de protocolo y los repositorios siguen la transacción real.
4. Las proyecciones de persistencia no son segundos propietarios. Una migración debe
   mover lectores, escritores, recuperación, publicación y confirmaciones relacionadas.
5. Los submódulos privados preceden a un crate. No se añade un trait, lock, espejo,
   campo público o crate por conveniencia organizativa.
6. La política de archivos, tests, fixtures, excepciones y terminales es la de
   [module-design-guidelines.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md). No se cambian aquí sus
   presupuestos.

La selección de `hecs` es privada y selectiva: no crea un ECS global, reemplaza el
scheduler, expone queries públicas ni descompone todo Player. La conformidad de
laboratorio ya pasada habilita seguir inspeccionando el límite; no autoriza instalar
dependencias de producción antes del gate de #584.

#### 4. Continuación P0–P6 y secuencia técnica

El mapa conserva los identificadores de la continuación sin convertirlos en una cola
de microissues:

| Fase | Responsabilidad actual | Estado/propietario |
| --- | --- | --- |
| P0 | Herramientas de ownership, imports, bridges y ratchet físico | #716 integrado y cerrado; su evidencia es histórica y no se repite aquí |
| P1 | Recompensa de misión y contrato durable | #718 integrado y cerrado; no hay evidencia real de DB/restart/relogin |
| P2 | Fronteras de Player y operaciones completas | #743 primero, #735 después como preferencia sin dependencia dura; luego los residuales por consumidores |
| P3 | Fases, runtime, lifetime, residencia/incarnation y storage selectivo | #584 C0–C4, después de los contratos P2 que requieran esa frontera |
| P4 | Organización física, excepciones y límites semánticos | #584, acompañado por cada operación; la medición de 31 paths permanece histórica |
| P5 | Producto de módulos M0–M4, nativo/Wasm y Rust/Wasm/C | #583, tras los requisitos core de #584; no bloquea gameplay independiente |
| P6 | Auditoría terminal y evidencia integrada | #153, después de #584 y #583; no absorbe implementación |

La tabla es un mapa técnico de las responsabilidades de `PORT_PLAN.md`/#49. La
selección de #743 y #735 es prioridad de trabajo, no una dependencia inventada entre
issues; C0–C4 permanece en #584 y M0–M4 en el producto #583.

##### 4.1 #743 — entrega y reconciliación de comandos de grupo

La propiedad de membresía ya está en `wow-social`; #743 no reconstruye ese owner ni
afirma que la extracción anterior haya introducido el defecto. El problema pendiente
es la entrega de una transición que también cambia estado del Player:

- `crates/wow-world/src/handlers/group/ops_1.rs:552,621,633` y `:782` eliminan o
  persisten membresía y descartan el resultado de `try_send_current_command`.
- `crates/wow-world/src/session/directory.rs:1753` puede recibir `Full`; el canal de
  producción tiene capacidad 256 (`session/mod.rs:7726`).
- `crates/wow-world/src/session_commands.rs:141` y
  `handlers/loot/handlers.rs:1438` aplican la limpieza cuando el Player consume el
  comando, sujeto a fase y pertenencia.
- `session/mod.rs:12431` sincroniza hacia el directory, pero no reconcilia una
  membresía cuyo comando se perdió.

El ancla Classic es `Group.cpp::RemoveMember` (`:550`, limpieza `:593`) y
`Group::Disband` (`:713`, limpieza `:734`), que llaman directamente a
`Player::SetGroup(nullptr)`. `Player.cpp:23440` desvincula la referencia y actualiza
visibilidad. El estado correcto no depende de un enqueue best-effort.

La entrega completa debe fijar una autoridad y un owner de ejecución para cada
transición, además de tratar saturación, disconnect, replacement/incarnation,
cancelación, comandos obsoletos y reconciliación tardía. El alcance incluye comandos
que sostienen estado: instalación de membresía, kick/disband y cambios de subgroup
(`ops_1.rs:438`; `ops_2.rs:65,136`). Los lectores que validan pertenencia y los que
usan GUID para tap/instance (`session/social/group.rs:9,30,46,74`;
`session/instances/instance.rs:48`) deben quedar clasificados por su contrato.
No se rediseña todo el sistema de mensajería ni se reparan todos los `try_send`
ajenos a la inconsistencia demostrada.

La aceptación debe saturar la cola real durante kick y disband y conducir el camino
de aplicación/reconciliación hasta acuerdo entre registry y Player. Debe cubrir
disconnect, reemplazo de sesión, eliminación obsoleta después de un nuevo join y las
transiciones seleccionadas. La prueba de `Full` aislada no basta; tampoco basta una
prueba de fixture que no ejecute la composición de producción.

##### 4.2 #735 — encapsulación de reputación bajo Player

La corrección exacta de #735 sustituye el requisito anterior de mover todo
`ReputationMgrLikeCpp` de `wow-world` a `wow-entities`. Ese movimiento literal
violaría las dependencias: `crates/wow-world/src/reputation/mgr.rs:8-23` usa catálogos
de `wow-data` y builders de `wow-packet`, y `tools/architecture/dependency-policy.json`
impide añadir esas dependencias al dominio de entidades.

El residuo actual está en
`crates/wow-world/src/session/progression/reputation.rs:160-169`: reconstruye una
working copy, aplica una transición y escribe el agregado a través de
`player.gameplay_state_mut()`. La operación es síncrona dentro de
`with_owned_player_mut_like_cpp`; `session/canonical_access/operations.rs:33-41`
mantiene el guard del Player canónico durante ella. Es reconstrucción/writeback
temporal, no evidencia de un segundo manager persistente concurrente.

El límite aprobado es:

1. Mantener estado e invariantes de reputación con Player/domain; encapsular las
   transiciones representadas de standing, rank, flags y spillover con inputs y
   resultados concretos.
2. Resolver catálogos fuera de la frontera de entidades y pasar los valores que la
   operación necesita. Los value types compartidos se mueven según sus consumidores
   reales y el grafo permitido; no se crea service locator, trait especulativo ni
   crate por helper. Mover un value type y sus consumidores es un paso interno de la
   entrega coherente, no una issue o aprobación previa por helper.
3. Mantener paquetes y entrega en la adaptación de aplicación/protocolo. Preservar
   `need_send`, `send_faction_increased`, el orden de publicación y el save ack.
4. Migrar el conjunto completo de consumidores: load/hydration, publicación de login,
   flags, quest rewards, spell effects, kill rewards, save projection y save ack. La
   lista Rust incluye `session/persistence/load.rs:173`,
   `handlers/character/session_state.rs:1811`, `handlers/misc/reputation.rs:143,167`,
   `handlers/quest/rewards.rs:1506`, `session/spell_effects/effects_progress.rs:63`,
   `session/world_entities/creature_kill.rs:213`,
   `session/lifecycle/persistence/projection.rs:398` y
   `wow-entities/src/player/save_ack.rs:91-105`.
5. Retirar el write-back genérico y las reconstrucciones superseded, o dejar una
   proyección inmutable con propósito y condición de retirada explícitos. No crear
   otro owner mutable.

Los anclajes Classic son `Player.h:3116` y `Player.cpp:338` para la propiedad del
manager, además de `ReputationMgr.cpp::SendState`, `SendInitialReputations`,
`SetReputation`, `SetOneFactionReputation`, `ModifyReputation`, `SetVisible`,
`SetAtWar` y `SetInactive`. La presencia de otras reglas de moneda/renown/paragon/
criteria en C++ no prueba que ya estén representadas; una reparación descubierta
debe tener contrato propio.

La aceptación necesita conservar estado inicial, rank/standing/flags, spillover,
rewards/effects/kills, publicación, comparación de row/revision, save ack después de
una mutación nueva y flags de envío independientes. Los 15 sitios registrados por
#722 incluyen el camino amplio de Player; reputación es el caso distinto que requiere
resolver esta frontera, no una segunda lista de 15 cierres.

##### 4.3 Residuales P2 y paso a P3/P4

Después de #743 y #735 se retiran los accesos genéricos operación por operación. El
residual de #722 queda registrado como cierres que entregan una submatriz `&mut` al
cierre llamador; cambiar el nombre del helper no retira la superficie. El residual de
#737 es el cierre de almacenamiento de items y sus fixtures; no se agrega una
autoridad de oro, porque `Player::SetMoney`/`ModifyMoney` ya viven en
`wow-entities/src/player/progression.rs`.

P3 debe contrastar composición, fases y lifetime con `Map.cpp:666-813`,
`MapManager.cpp:287-318` y `WorldSession.cpp:64-108`, contar tareas reales y conservar
residence/incarnation, backpressure, cancelación, transfer, unload y shutdown. La
traza fechada de seis relojes no se convierte en una afirmación de doble ejecución sin
reproducción. Cualquier guard síncrono de mapa/entidad debe liberarse antes de await,
I/O o entrega de paquetes.

P4 acompaña cada operación: raíces pequeñas, submódulos cohesivos, fixtures por
responsabilidad, imports explícitos y excepciones por fichero. La medición de 31
archivos sobre 2.000 líneas es un límite de revisión fechado, no una orden de crear
31 issues ni una nueva medición. El terminal físico de #584 requiere dividir o
justificar cada excepción según la política vigente.

#### 5. Producto #583 y auditoría #153

Cuando los requisitos de núcleo necesarios estén integrados, #583 entrega un producto
de módulos con hooks, estado y lifecycle compartidos por ejecución Rust nativa y Core
Wasm. Debe incluir Rust/Wasm/C mixto, aislamiento de estado, capacidades sin SQL ni
Player mutable expuesto, instalación/actualización/desactivación/recuperación y
durabilidad/progreso/recompensa del operador. Wasm puede ser opcional de activar, pero
su contrato y su evidencia no son opcionales. No se promete hot reload, ABI nativa
estable, aislamiento de panic ni todos los lenguajes sin contrato.

#153 verifica de forma independiente el resultado completo de #584 y #583: owners,
dependencias, fases/clocks, guards, persistencia, bridges, registros, organización
física, SDK y operación real. #153 no absorbe implementación conocida ni convierte
un green de fixtures, una suma de tests o el laboratorio hecs en paridad completa.
Después de M6.2/#47 se mantiene la revisión fresca del port antes de descomponer
Part 2/#48; esta entrega no crea ese árbol.

#### 6. Método de cada macro y aceptación

Cada macro sigue la misma secuencia técnica, adaptada al contrato real de la operación:

1. Contrastar el owner Rust actual, todos los callers y los anclajes C++ exactos.
2. Fijar bytes/metadatos/conexión/orden cuando haya protocolo; admission, fases,
   incarnation, locks, persistencia, COMMIT ambiguo, cancelación y recuperación.
3. Separar movimiento estructural de una reparación intencional; no esconder una
   reparación dentro de un refactor.
4. Migrar consumidores, tests, fixtures y registros de producción en una entrega
   coherente. Retirar el writer/bridge viejo o anotar su condición de salida concreta.
5. Ejecutar la validación al cierre de la macro: tests positivos/negativos, integración
   de composición real, checks de arquitectura y `validation-v2` según el alcance.
   Captures y QA live solo cuando el cambio los requiere y con la autoridad runtime
   correspondiente.
6. Registrar SHA probado, comando, resultado, host y límites. Una evidencia de otro
   SHA o un test histórico no se relabela como evidencia del candidato nuevo.

La validación documental y de metadatos de #748 no constituye aceptación de gameplay.
No se ejecutan capturas, cambios de servicio, escrituras de DB ni escenarios de
reinicio/relogin como parte de esta revisión. Cada macro de implementación conserva
sus pruebas, integración y evidencia runtime requeridas.

#### 7. Registro historico conservado

El handoff de #716 se conserva como evidencia en [STATE.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md) y
en [modularity-and-ecs-plan.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-and-ecs-plan.md), incluyendo el commit
`6ae62d73a9f910ebd664d82e331520b836a73c77`, los 65 bridges preservados, siete registros
recuperados y los 363 tests históricos del analizador. El modo físico terminal fallaba
en la medición fechada de 31 archivos; no se presenta como aceptación de #584.

La evidencia histórica de #718 conserva el contrato de
[quest-reward-operation-contract.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/quest-reward-operation-contract.md), el commit
`78276ddf57463fc0f568c1c4bcf84d619af68cad` y la ausencia explícita de DB/restart/relogin
QA real. Los checkpoints de #578, #585, #587, #588 y #589 siguen siendo fuentes de
alcance y captura para sus entregas cerradas; sus antiguas listas de "siguiente paso"
no son instrucciones actuales.

##### Inventario físico histórico — revisión #716

La medición fechada de #716 registró estos 31 paths por encima de 2.000 líneas
físicas. Se conserva completa para trazabilidad; no es una medición nueva ni una
excepción terminal automática. La responsabilidad y la salida de cada path siguen
la política de [module-design-guidelines.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md).

| Path | Líneas |
| --- | ---: |
| `crates/capture-diff/scripts/capture-rust.sh` | 2,243 |
| `crates/capture-diff/scripts/creature-spell-casting-fixture-common.sh` | 2,144 |
| `crates/capture-diff/scripts/detour-chase-fixture-common.sh` | 3,605 |
| `crates/world-server/src/app.rs` | 5,645 |
| `crates/world-server/src/creature_loaded_grid.rs` | 2,944 |
| `crates/world-server/src/lib.rs` | 2,078 |
| `crates/world-server/src/runtime/game_events.rs` | 2,643 |
| `crates/world-server/src/runtime/map.rs` | 2,063 |
| `crates/world-server/src/spawn_store_loader.rs` | 5,125 |
| `crates/world-server/src/spell/acquisition_loader.rs` | 2,267 |
| `crates/wow-entities/src/player/items/storage.rs` | 3,298 |
| `crates/wow-entities/src/player/mod.rs` | 4,881 |
| `crates/wow-map/src/map/game_object.rs` | 2,299 |
| `crates/wow-map/src/map/mod.rs` | 5,269 |
| `crates/wow-map/src/map/relocation.rs` | 2,169 |
| `crates/wow-map/src/map/spawn_groups.rs` | 2,384 |
| `crates/wow-recastdetour/vendor/Detour/Source/DetourNavMeshQuery.cpp` | 3,663 |
| `crates/wow-social/src/group/tests.rs` | 2,508 |
| `crates/wow-world/src/handlers/character/items.rs` | 3,874 |
| `crates/wow-world/src/handlers/character/mod.rs` | 2,728 |
| `crates/wow-world/src/handlers/character/session_state.rs` | 2,809 |
| `crates/wow-world/src/handlers/character/vendor.rs` | 2,513 |
| `crates/wow-world/src/handlers/character/world_entry.rs` | 2,771 |
| `crates/wow-world/src/handlers/loot/handlers.rs` | 2,463 |
| `crates/wow-world/src/handlers/loot/requests.rs` | 2,524 |
| `crates/wow-world/src/handlers/loot/sources.rs` | 3,109 |
| `crates/wow-world/src/handlers/quest/handlers.rs` | 2,716 |
| `crates/wow-world/src/handlers/quest/rewards.rs` | 2,159 |
| `crates/wow-world/src/session/directory.rs` | 2,351 |
| `crates/wow-world/src/session/mod.rs` | 18,571 |
| `crates/wow-world/src/session_tests.rs` | 5,755 |

Las anclas C++ de esta página identifican responsabilidad y contrato; no sustituyen
una captura específica ni declaran paridad de sistemas no representados. La dirección
general, consolidación y orden de las 46 issues iniciales permanecen en `PORT_PLAN.md`
y #49.

---

<a id="modularidad"></a>

## C. Diseño completo de modularidad y ECS

Fuente mantenida: [docs/architecture/modularity-and-ecs-plan.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-and-ecs-plan.md).

### Native/Wasm modules, shared hooks and selective hecs — execution plan

**Plan synchronization, 2026-09-11 (#748):** `PORT_PLAN.md` and GitHub #49 are the
general direction and issue scope. This document is the technical authority for
module, ownership, dependency and acceptance contracts; it is not a rival execution
plan. #133 was closed on 2026-09-09. #578/#585/#587/#588/#589/#716/#718/#722/#737
are integrated and closed in their bounded scopes. #584 retains unfinished C0–C4 core
work; #583 owns the preserved M0–M4 native/Wasm product. The technical gate remains
#584 core → #583 native/Wasm product
→ #153 independent audit. #583 does not block an unrelated gameplay macro, while
production module integration waits for the required core work. Its Rust/Wasm/C mixed
product remains mandatory even though operator activation is optional.

The finite hecs V2 conformance proof has passed within its recorded laboratory limits.
That evidence does not install production `hecs` or Wasmtime, prove production storage
integration, or close the remaining #584 boundaries. The current recommended sequence
is #743, then #735 as an ordering preference without a hard dependency, then the
remaining P2 operations and P3 runtime/lifetime/private-hecs and P4 semantic/physical
work. No new micro-issues are implied; each macro includes its consumers and validation.

#### Architecture program state — 2026-09-11

The [refactor completion plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/refactor-completion-plan.md) records the detailed
operation contracts and continuation sequence. This document retains the canonical
semantic, storage and extension contracts; neither document turns a pending task into
an accepted result.

The architecture repair program is reviewed against integrated `3.4.3` at
`5d8c079a06b587c060c1c6e1c06bedb73c4339d0`. #587/#588/#589 and the subsequent
#716/#718/#722/#737 deliveries are integrated and closed in their bounded scopes.
Their checkpoints retain scoped runtime/capture evidence; they are not reopened by
the remaining core work or by naming preferences.

##### Historical #716 baseline and local acceptance — `aff42a51`

The following #716 account is retained as historical implementation evidence. Its
local test counts, candidate wording and former next-operation text do not select the
current macro; the current sequence is #743 → #735 (without a hard dependency) →
remaining #584 work below.

**#716 — restore ownership provenance and acceptance after module decomposition**
was the first delivery of the repair program reviewed at `aff42a51` under #584.
Its scope was the analyzer, the
exact reviewed inventories and the current design/status guidance. No gameplay,
SQL, packet, scheduling or runtime operation changes belong to this delivery.

At the reviewed HEAD, the general architecture check and its self-tests pass, but
the syntax-only Session ownership check fails. Two ten-argument `WorldSession::new`
fixture sites moved to `battle_pet_purchase::executor_tests::fixtures::session` and
`handlers::misc::tests::world` without their baseline scopes. The third failure is
not a retired bridge: #711 moved `world_creature_from_pending_respawn_like_cpp` into
`map_manager/pending_respawn.rs` unchanged, where `use super::*` hides the parent's
explicit `wow_entities::Creature` import from the file-local bridge symbol resolver.
The function still constructs canonical `Creature` and converts it to `WorldCreature`.

The repair resolves supported lexical imports against the supplied logical module
graph, preserving exact authority provenance, local shadowing, cfg and multiplicity.
An unresolved relevant import must not silently become absence of authority. Keep
external authority globs rejected and do not classify every identifier named
`Creature` as canonical. Tests cover root/inline/external moves, aliases, local
non-authority names, missing/ambiguous context and conditionally present imports.
Review each restored/new bridge record; a smaller generated baseline is not a repair.

The first library acceptance run also exposed a separate false positive from the
physical pass: `wow-persistence/src/player/mod.rs` declares and reexports its local
`inventory` data module. The registration guard treated that spelling as the external
registration crate. #716 now distinguishes that narrow local declaration/glob only
outside the handler registry closure and when Cargo's resolved normal dependency
graph proves no path to `inventory`, including renamed and target-specific dependencies.
Actual macro calls, exports, includes and registration aliases retain their checks;
packages with registration capability keep the original strict namespace guard.
Real-source bridge regressions share one complete syntax graph so missing fixture
parents cannot masquerade as retired authority. Nested cfg and block-local imports
retain their lexical scopes, including non-authority shadows.

The repaired syntax inventory preserves all 65 earlier bridge records exactly after
the reviewed `pending_respawn` relocation and adds seven source-reviewed records,
each with multiplicity one. Six production methods receive canonical `Creature`
through parent imports while their `Self` is legacy `WorldCreature`:

| Source at reviewed base | Recovered method |
| --- | --- |
| `map_manager/movement/motion_master.rs:9` | `new_runtime_motion_master_like_cpp` |
| `map_manager/runtime/creature.rs:8` | `runtime_default_generator_like_cpp` |
| `map_manager/runtime/creature.rs:47` | `new` |
| `map_manager/runtime/creature.rs:126` | `from_canonical` |
| `map_manager/runtime/creature.rs:180` | `create_data_from_canonical_like_cpp` |
| `map_manager/runtime/creature.rs:274` | `from_loaded_grid_canonical_like_cpp` |

The seventh is `session/deferred_visibility/tests.rs:33`, `Fixture::new`, whose
canonical and legacy map managers are constructed at lines 51 and 53; it retains
`cfg(test)`. Paths in this table are relative to `crates/wow-world/src`. All seven
retain `unresolved_dual_side`: syntactic dual references are not proof of two
writers or of a particular transfer direction. No earlier obligation, fingerprint
or evidence multiplicity was removed, and no persistence snapshot was regenerated.

**Local acceptance:** implementation is committed as
`6ae62d73a9f910ebd664d82e331520b836a73c77`. The complete candidate passed 363
analyzer library tests, syntax-only ownership, architecture check/self-test,
preserved persistence-reference and snapshot-policy consistency, format/diff checks
and validation-v2 quick (six commands, verified green manifest). Runs used the
working candidate at parent `aff42a51`; the manifest truthfully records `dirty: true`.
The tested code/policies were committed unchanged in `6ae62d73`; subsequent handoff
edits only record documentation. Exact commands and evidence limits are in the
[historical handoff record](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/refactor-completion-plan.md#7-registro-historico-conservado).
This is local acceptance of #716, not publication, gameplay/runtime acceptance,
terminal physical acceptance or closure of #584.

The same review measures 31 files above 2,000 physical lines and 63 above 1,000.
`physical-files --terminal` fails at this base. The completed mechanical passes are
useful evidence, not completion of physical navigability. Keep both logical-owner
and physical-file measures; a reviewed wiring delta does not retire semantic debt.
The [module-design correction](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md#what-a-physical-division-cannot-reach)
replaces the blanket single-item/operation prohibition with responsibility-preserving
delegation, explicit imports and scenario names at each completed family.

##### Historical quest-reward design before #718

The following contrast predates integrated #718; its contract and accepted scope are
recorded in [STATE.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md) and
[quest-reward-operation-contract.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/quest-reward-operation-contract.md).
The former represented-operation candidate was **quest reward**, because its Session
coordinator grants/persists participants separately and discards status-save outcomes.
Before implementation, freeze the complete operation and compare a coherent character
transaction with durable staged recovery where participants require it. Current source
anchors are `handlers/quest/rewards.rs::reward_represented_quest_with_generator_like_cpp`
and `handlers/quest/persistence.rs::save_quest_to_db`; Classic reference
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` has `QuestHandler.cpp:398`,
`Player.cpp::RewardQuest:14625`, its `SaveToDB(false):14867`, and the character
transaction in `SaveToDB:19312`. Reward mail has a separate transaction at `14794`;
do not infer one global C++ transaction or hide an intentional durability repair in a move.
Its acceptance must cover partial grants, unknown COMMIT, cancellation, recovery,
restart/relogin, retry and ordered publication with all affected consumers.

The initial source contrast also bounds that next design:

| Participant | Current represented boundary | Consequence for the complete operation |
| --- | --- | --- |
| Client reward and tracking-event auto reward | `handlers/quest/handlers.rs` and `handlers/quest/state.rs` reach the same reward coordinator | Both admissions, and the objective-progress drain that follows success, belong to the migration. |
| Required items/currencies and fixed/chosen/package grants | The coordinator awaits the participants in sequence; individual item grants can already commit before a later failure | Define the whole participant set and failure/retry contract before moving orchestration; returning `false` is not rollback. |
| Quest status and repeatable deletion | `handlers/quest/persistence.rs` logs `Failed`/`Unknown` and returns `()`; `wow-database/src/player/quest_adapter.rs` commits each request separately | Preserve this as an identified pre-existing behavior gap until an explicit repair contract replaces it. |
| Full character save | `PlayerCharacterSaveRequestLikeCpp` in `wow-persistence/src/player/save.rs` covers character/spell/skill and other groups, but has no quest, inventory or currency group | Calling the existing full-save helper cannot by itself make quest rewards coherent; all affected persistence consumers must participate. |
| Mail and observable publication | Rust's `record_represented_quest_reward_mail_like_cpp` only records test evidence and is a production no-op. Classic commits reward mail separately, sends the reward and executes further effects before `SaveToDB(false)` | Keep the unimplemented mail participant explicit and record separate durability boundaries and actual message/effect order; universal commit-before-publication would be an intentional behavior change. |

This is source-based historical design preparation, not a selected transaction protocol
or new runtime acceptance. It did not authorize a silent durability repair inside #716.

The current #584 operation/lifetime and map-phase/storage sequence is maintained in the
architecture program state above and the refactor completion plan. Physical organization,
scoped fixture migration and dependency disposition accompany each responsibility; the
later #583 product does not inherit those core debts. Preserve the technical gate
#584 core → #583 → #153 and the post-M6.2 whole-port review. The tracker closure of #133
does not reopen or add a further gate.

#### Historical delivered design — #589, 2026-09-08

This dated section preserves the delivered #589 design; current selection is above.

The user approved **#589 — represented Player cast-request lifecycle** on
2026-09-08 after localized source review at `cc8a8e97`. It is now **implemented
and locally accepted** under #584, split for execution into #590-#595; this does
not complete all spell gameplay. The [#589 checkpoint](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/player-cast-589.md) owns
the executed evidence, the publication payload's represented value limits and the
deviations deliberately left open, and supersedes the earlier paused state.
#588/#587 have local scoped acceptance and their content is carried in full by the
#589 branch, so one PR integrates the three deliveries.
Their checkpoints retain exact tested source identities and capture limitations.
The design described below is the delivered design; where the implementation
departed from it, the checkpoint records the departure and its reason. In
particular, an intermediate iteration that refused projectile, rune,
heal-prediction and trajectory casts outright was withdrawn as a functional
regression C++ does not have.

The operation is client request → immediate/queued admission → preparation →
instant/timed launch or cancellation → ordered publication and effect execution.
At the reviewed base, `handlers/spell.rs::handle_cast_spell_with_catalogs_like_cpp` coordinates
the immediate path, while Session's pending-request consumer can reach the shared
executor without repeating preparation. `session/driver/mod.rs` completes the active
cast before processing the pending request. Canonical state already resides in
`PlayerGameplayState.pending_spell_cast` and `Unit.subsystems.spells.execution`,
with transitions in `wow-entities/src/spell_cast.rs`; preserve those authorities.

The proposed private application owns orchestration through narrow capabilities;
Session adapts transport, catalogs and effect execution. A prepared result must
distinguish client requests from server-triggered casts and retain identity,
targets and publication metadata through exactly one consumption. No new mutable
Session mirror, task, clock, map lock across delivery, or universal context is needed.
Toy, binder, spell-click, first-login, item-acquisition and self-resurrection callers
of the shared executor must retain explicit contracts. Loot, transfer, stance,
channel cancellation and stuck interruption callers must reach the same canonical
active/pending owners. These consumers belong in the migration, not follow-up helpers.

Separate structural extraction from intentional repairs. The #587 action capture
already demonstrates absent SpellPrepare/instant SpellStart and divergent SpellGo
identity, visual and flags. Source inspection additionally finds queued requests
bypassing preparation/revalidation and cancellation differences. Classic anchors
at reference `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` are
`src/server/game/Entities/Player/Player.cpp` (`ExecutePendingSpellCastRequest`,
lines 29122–29313), `src/server/game/Handlers/SpellHandler.cpp` (line 263), and
`src/server/game/Spells/Spell.cpp` (construction 580–583, preparation 3562,
cancellation 3579, update 4209, Start/Go 4656/4765). These source anchors do not
relabel the earlier derived C++ captures as having run this reference SHA.
Use the versioned complementary-source policy for missing or suspect behavior.

The map-scoped server CastID allocator already exists:
`wow-map/src/map/storage.rs::generate_low_guid_like_cpp(HighGuid::Cast)`.
Prepare against the exact active `MapKey` after checked Player residence admission,
consume that map's sequence and install prepared state under the existing guard;
publish after releasing it. Allocate at preparation, not while merely queuing a
request. The Session global counter must retire for creature publication as well as
Player callers, because separate counters can collide within one map. Creature
validation already holds the canonical guard; pass the allocated GUID onward rather
than locking again. Classic `Map.h:515`, `Map.cpp:2505`, `Spell.cpp:580` and
`ObjectGuid.cpp:623` establish this scope: instance ID is not encoded in CastGUID,
separate instances may have identical bytes, and a live map retains its sequence
across Player reentry. Do not introduce a replacement counter or encode instance ID
as server ID. Add shared Player/creature sequence, cross-instance independence,
reentry continuity, stale-admission/no-consumption and queued-cancellation tests.
The existing Rust generator asserts at exhaustion; this proposal does not claim a
recoverable allocation API or equivalence to C++ overflow shutdown.

Identity inspection resolves the normal request contract: Classic
`Player.cpp::ExecutePendingSpellCastRequest` sends ClientCastID only in the
SpellPrepare mapping to the newly constructed server CastID. The constructor's
OriginalCastID defaults to empty (`Spell.h:426`, `Spell.cpp:581`); it does not echo
the request ID. Preserve explicit original IDs for triggered consumers rather than
changing `SpellCastMetadata::original_cast_id_or` globally without migrating them.
Classic selects the visual through `GetCastSpellXSpellVisualId` (`Spell.cpp:583`),
not from the request's visual. Pending-request revalidation must precede construction
and the mapping; preparation failures after construction retain that already-issued
mapping and consumed server identity. This distinction belongs in negative captures.

Before fixing publication, settle the server visual resolver, target selection and
visible recipients. `wow-packet/src/packets/spell.rs` currently writes empty
power/rune/projectile/immunity sections; merely adding flags cannot establish wire
correctness. A private publication module may serve this operation, but must not
silently change default metadata for all triggered consumers.

Further serializer inspection at the same Rust/Classic revisions fixes the payload
contract: Classic `Server/Packets/SpellPackets.cpp:334–446` writes power entries as
`int32 amount + int8 type`, optional rune state with an explicit cooldown count,
and separate optional ammo fields. These are controlled by serialized presence/count
bits, not inferred by the serializer from CastFlags. Start samples power before
execution; Go samples the remaining power and uses server time rather than cast
duration (`Spell.cpp::SendSpellStart/SendSpellGo`). Start and Go do not populate
identical optional fields. The current Classic Go rune cooldown loop is commented
out, so do not claim that this reference supplies complete rune behavior. Rust's
advanced combat-log power rows are a different structure and cannot substitute for
cast RemainingPower. Any packet shape change also reaches the direct constructors
in `handlers/misc/collections.rs`, creature publication in Session, and packet/server
tests, even when their gameplay behavior remains outside the client-request repair.

Acceptance covers paired instant/timed casts, queue admission inside/outside the
400 ms window, replacement, active/pending cancellation and late failure; compare
Prepare → Start → Go → effects, related identities, visual, payload-dependent flags,
targets and connections, including a nearby observer. Verify single execution,
power/cooldown phase, retired identity exclusion and detach/reentry. Preserve
`active_cast_owner`, `pending_cast_owner`, opcode registration and production driver
composition evidence, plus affected triggered consumers and #587/#588 regressions.
Complete implementation and consumer migration before the affected test/QA campaign.

This does not claim the full effects engine, SpellHistory, projectile simulation,
channels/autorepeat, pets/vehicles or all target-selection rules. Those remain port
work; any missing behavior required by the declared lifecycle scenarios must be
resolved within its acceptance rather than waived through an exclusion. The account
SaveToDB alternative remains unselected: its complete responsibility includes missing
Login-side participants and cross-schema recovery, not just Session account caches.
The technical gate remains required #584 core → #583 → #153. #133 was closed on
2026-09-09 and is not a future prerequisite or a reason to reopen this delivery.

**Decision date:** 2026-09-05. **Reviewed production code:** `93e4002a` on the
#578 branch; reviewed laboratory/planning HEAD: `ee9a0128`. This is a bounded
architecture review, not a new whole-port parity audit or an implemented ECS migration.

This plan supersedes the earlier “ECS review next”, unconditional backend-selection,
snapshot-only extension and automatic post-M6 Wasm directions. It preserves the full
[port plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md), useful completed ownership/module work, and
#578's delivered foundation and the remaining #584 C0–C4 acceptance. The user has approved
the direction and plan update;
publication, deployment and destructive operations keep their separate approval rules.

**Latest decision — 2026-09-05:** select **private, selective `hecs`** for composable entity
state, retaining cohesive domain aggregates. Native Rust is the default execution path for
first-party and custom modules; **Wasm is a planned, operator-optional execution path of the
same extension contracts**, including a tested second source language. Hooks, state/lifecycle
and host integrity are shared, not two independently designed gameplay APIs.

This is an architectural selection, not a claim that production implementation acceptance has
passed. It supersedes the earlier preferred-candidate wording, the proposal to defer selection
until production integration, and the proposed three-backend preselection contest. The finite
independent module/conformance proof has passed within the recorded laboratory limits in
[V2 results](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-conformance-results.md); it does not install production `hecs` or
Wasmtime, prove production storage integration, or establish the complete #583 product.

**Product scope:** #583 includes bounded Wasm execution and the second-language C guest,
not just trusted Rust. The mixed Rust/Wasm/C product is mandatory even though operator
activation is optional. #153 audits the delivered #584 core and #583 product; the broader
#99 language/ecosystem roadmap still has an M6 re-audit. This product requirement does not
block unrelated gameplay macros. No new micro-issues, production dependency, deployment or
code publication follows from this plan update. The #583 M0–M4 product, #584 C0–C4 core
and existing durability/operator acceptance remain intact.

#### Historical physical decomposition pass — 2026-09-09

Sixteen deliveries between #634 and #664, following the Session-root separations
of #603-#632, completed the physical half of rule 5 (manageable production/test/
fixture files). Those sixteen reduced 184,295 lines of oversized roots to 4,972.
At `df94f231` the reviewed physical ceilings are met and no file outside the four
categories below exceeds the 2,000-line terminal limit.

This is dated evidence from that integration, not a current terminal claim. The later
#716 review measured 31 files above 2,000 lines and 63 above 1,000 at its reviewed base;
that measurement is retained without a new scan and remains #584 P4 work. Do not turn
the count into one issue per file or refresh ceilings to make a check green.

| Delivery | Scope | Before | After |
| --- | --- | --- | --- |
| #634 | architecture checker analyzer, QA bot loot race | 20,536 | 286 |
| #636 | four `wow-entities` state roots | 17,609 | 136 |
| #638 | `wow-data` condition/skill roots | 8,520 | 75 |
| #640 | `wow-packet` regression roots | 8,604 | 302 |
| #642 | `wow-loot` crate root and authority | 7,350 | 69 |
| #644 | `wow-map` manager/pool/spawn | 9,314 | 93 |
| #646 | `wow-data` spell roots | 11,177 | 408 |
| #648 | remaining `wow-entities` files | 10,977 | 270 |
| #650 | `wow-packet` production roots | 12,255 | 273 |
| #652 | `wow-database` statement files | 8,748 | 147 |
| #654 | `wow-world` chat/movement handlers | 7,427 | 651 |
| #656 | battle-pet purchase, world conditions | 8,565 | 130 |
| #658 | `capture-diff` roots, four crate roots | 20,339 | 576 |
| #660 | remaining checker sources | 10,479 | 248 |
| #662 | last non-curated `wow-world` files | 14,769 | 857 |
| #664 | checker entry point, creature template, misc DB2 | 7,626 | 451 |

Every delivery proved its top-level item surface identical before and after by
comparing the parsed item multiset, kept its crate's test count unchanged, and
passed `./tools/validation-v2 final --base origin/3.4.3` at the committed SHA.
Ceilings were tightened to the validated figures rather than left slack, and a
file replaced by a directory had its policy row moved to the successor path,
because the ratchet refuses a missing audited path precisely so a rename cannot
evade a ceiling.

##### What relocation cannot do — three findings that bound the track

These are dated observations of the mechanical pass. The #716 correction above
and the module-design guide govern further decomposition and terminal acceptance.

**Relocation does not retire a logical owner.** The runtime-ownership ratchet
measures its aggregate, so a private split adds wiring while retaining that debt.
#636 recorded this for Player, #644 for Map and #654 for character/loot/quest
handlers. Review the wiring delta and the benefit of the proposed physical boundary;
do not infer from the aggregate that private decomposition must stop.

**A source move must preserve bridge detection.** The bridge inventory pairs
canonical-side and legacy-side evidence within one enclosing item, using resolved
symbols from its lexical context. Moving `world_creature_from_pending_respawn_like_cpp` out of
`crate::map_manager` dropped it from the 65-row inventory while the build and
all 3,822 `wow-world` tests stayed green (#662). Only the ownership baseline
diff caught it, so that split was reverted. #711 later exposed the same import
resolution limit in the integrated tree; #716 repairs that coverage. A green build
does not establish that a scanner still sees the same actual bridge.

**Visibility must be derived from the item's original reach, not from a rule.**
`pub(super)` denotes a different scope at each depth; a glob re-export silently
skips items that are too private; and at a crate root a private item is already
crate-visible. So each moved item took the qualifier matching what it already
had - `pub(crate)` out of a crate root, `pub(super)` out of a submodule, an
explicit `pub(in path)` when the move went two levels down - with the compiler
as the check. #650 needed 28 explicit restricted re-exports for exactly this
reason, and #664 hit the same trap in Python, where `from x import *` skips
underscore-prefixed names.

##### Semantic track entry conditions

The dated first-pass inventory reported 116,693 lines above 2,000 in 33 files;
use the current measured inventory above for present sizes and terminal status:

| Owner | Lines | What it needs |
| --- | --- | --- |
| `crates/world-server/` | 22,803 | Startup composition phases extracted from one 5,652-line `run_inner`; 586 top-level locals make relocation meaningless |
| `crates/wow-world/src/session/` | 21,271 | Session responsibility families with their own owners; the root is now struct plus infrastructure |
| `crates/wow-world/src/handlers/character/` | 14,745 | Per-operation handler owners |
| `crates/wow-map/src/map/` | 12,186 | Map responsibility split with the grid/visibility owners named |
| `crates/wow-entities/src/player/` | 8,202 | Login/gameplay load plans as their own owner |
| `crates/wow-world/src/handlers/loot/` | 8,096 | Loot source/authority owners |
| `crates/wow-world/src/handlers/quest/` | 4,948 | Quest operation owners |
| `crates/wow-world/src/map_manager/` | 4,524 | Canonical/legacy bridge separation that keeps both sides in one auditable module |
| `crates/wow-social/src/group/` | 2,508 | Group state owner |
| `crates/wow-world/src/session_tests.rs` | 5,755 | Follows its production owner |
| capture fixture shell scripts | 7,992 | Runtime swap, dump provenance and recovery orchestration, with the existing capture safeguards and runtime authorization preserved |
| vendored `DetourNavMeshQuery.cpp` | 3,663 | Out of scope: third-party C++ carried verbatim |

These files retain #584 C0-C4 responsibility and physical acceptance. Some need
semantic extraction; others can improve through private delegation within the same
legitimate owner. Decide from their contracts and consumers, and reconcile the exact
inventory/ceilings together. Vendor code still needs its file-specific disposition
for terminal acceptance; its presence is not permission to modify upstream blindly.

#### 1. Outcome and current evidence

RustyCore should support useful gameplay extensions in independent repositories without
forking core implementation. The target is a **modular monolith with capability-specific
contracts**, not merely more files/crates, a universal plugin framework or microservices.
Base-server behavior remains anchored to TrinityCore-derived 3.4.3 C++; optional custom
behavior has an explicit contract and cannot bypass core integrity.

Throughout this plan, zero/disabled modules means zero **optional extensions**. Required
first-party base scripts remain enabled; neutrality must not remove behavior needed for C++ parity.

| Boundary | Implemented at the reviewed code | Acceptance still required |
| --- | --- | --- |
| Canonical Player/Map authority | Migrated Player families, generation-checked active/detached lifetime, retirement of whole-Player Session write-back and directory copies | Remaining lifetime/save, operations, phase/publication and inherited boundaries belong to #584; see its [checkpoint](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/session-578-checkpoint.md) for historical #578 evidence |
| External source/build modules | #228–#231: API, compositor/lock, CLI/skeleton and typed configuration; real `player.login → message` | Stateful behavior, composable policies, durable state/reward and operator lifecycle in #583 |
| Private entity storage | Production private `HashMap` entity records; finite `hecs` V2 conformance passed in the lab | Selected selective `hecs`: production integration and real-owner acceptance remain #584 work; external-consumer validation belongs to #583 |
| Schema migration | `rustycore-db` owns immutable checksummed migrations and fail-closed startup compatibility | Module artifact/history retention, state upgrade and recovery workflow under #583 |
| Wasm execution | Core Wasm exercised only in the isolated lab; no production sandbox | Bounded Wasm adapter and second-language module in #583, optional to enable; hot reload remains excluded |

The external module product is tracked by #99; this milestone's concrete implementation
is #583. The technical gate is required #584 core, then #583, followed by #153's
independent audit. #133 was closed on 2026-09-09 and is not a pending prerequisite;
the unrelated #99 ecosystem epic need not close for this bounded product.

#### 2. Authorities and dependency direction

Internal organization follows [module design and source navigability](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md):
each completed family must have both a correct owner/narrow dependencies and manageable physical
production/test files. The guide defines project budgets, bounded exceptions, a Rust submodule
skeleton and incremental legacy retirement. It supplements this plan; ECS and extension hooks do
not discharge physical decomposition. #584 C0–C4 own the remaining core cuts and physical checker
extension; #583 applies the policy to its own product, including Rust/Wasm/C, and #153 verifies
both completed macros without inheriting implementation.

| Responsibility | Canonical authority | Extension access |
| --- | --- | --- |
| Identity, incarnation, residence and transfer | Named Player lifetime authority coordinated with Map admission | Scoped handles/queries; never storage identities or a second Player mirror |
| Inventory, money, combat, progression and their invariants | Cohesive domain owner for each operation | Validated decisions/actions; host enforces integrity and publication |
| Map-local simulation, creature/encounter lifecycle | Admitted Map runtime phase and entity/domain owners | Bounded synchronous behavior capabilities |
| Group, account and matchmaking/LFG state | Their own explicitly named lifetime/domain owners | Domain-specific commands/results; not everything belongs to Map or ECS |
| Custom rules and module state | Module defines its rules/schema; host controls scoped access and lifecycle | Namespaced state and declared capabilities |
| Durable I/O, transport, composition | Persistence adapters, connection owner and composition root | Owned inputs/results; no SQL connection, packet writer or resource bag |

One logical authority does not require one giant struct or one OS/Tokio task per map.
Private storage may change without changing public gameplay contracts. Keep cohesive
aggregates for invariants; do not split every scalar into a component or move all Session
responsibilities into a new Map god object. Typed internal content adapters remain possible;
official scripts need full C++ fidelity and are not automatically a stable third-party API.
Internal and external adapters must share documented semantic call points without accidentally
executing the same hook twice.

##### One module product, two execution adapters

- **First-party/custom is provenance, not an execution mechanism.** Both can use the public
  module API. First-party base-server implementation may retain private internal APIs where full
  C++ fidelity needs them; that is not a hidden privilege promised to every external module.
- **Native Rust:** source-built independent crates, reproducible composition, explicit rebuild
  and restart. No stable dynamic Rust ABI, sandbox, panic containment or hot reload promise.
- **Wasm:** select Wasmtime as the initial host implementation (V1 tested 47.0.3), with a
  versioned Core Wasm ABI and generated/documented bindings. Start with Rust and a small C
  reference guest to prove a non-Rust producer. This is a support matrix, not a claim that any
  language/program compiling to Wasm already works. WIT/Component Model is not selected by the
  Core Wasm test; introducing it requires preserving the same synchronous callback contract.
- The operator chooses exactly one artifact/executor per module identity. Native and Wasm
  modules may coexist and compose; the same module must never execute twice because both
  artifacts are present. Startup rejects duplicate identity, incompatible API/state versions,
  unavailable required capabilities and conflicting exclusive policies before callbacks run.
- The shared contract defines typed inputs/results, error categories, lifetime, semantic hooks,
  state versions and capabilities. Native types and Wasm encodings need not have identical
  layouts. Neither public adapter exposes backend IDs, query guards or generic mutable entities.
- A module defines namespaced state/schema; the host controls admission and lifetime. Native
  state uses registered typed access, not a new central enum variant per module. Wasm uses
  bounded versioned records/opaque state access through the ABI, not Rust `TypeId`, pointers or
  guest structs inserted directly into ECS. Name the canonical physical owner for each scope.
  Schema version is not mutation revision: use short admitted access or incarnation/revision-checked
  writes, rejecting stale outer snapshots after nested callbacks. Representation optimizations may
  differ without creating a second authority for the same state.
- Switching native <-> Wasm with existing state requires an executor-independent durable format
  or an explicit validated conversion. An unsupported switch is rejected before callbacks, with
  data/history retained; no implicit reset, purge or migration merely because the executor changed.
- Wasm runs without ambient filesystem/network/DB access; host imports are explicit capabilities.
  Bound guest memory, execution fuel across the entire nested invocation, callback depth, host
  actions, payloads and host-side allocation/work. Fuel does not interrupt blocking host calls.
  Trap/error handling preserves already applied actions and records module/scope/call provenance;
  fail-open, reject or disable behavior is explicit per hook, never accidental executor behavior.
- Deliver a native-only build and a Wasm-enabled build, mixed-executor tests, pinned source/artifact
  provenance and the same upgrade/disable/data-retention rules. Loading a new Wasm artifact on
  restart does not require recompiling core, but does not imply hot reload of a running encounter.

#### 3. Three extension contracts, not one generic event bus

| Contract | Purpose and timing | Required semantics |
| --- | --- | --- |
| Pre-decision policy | XP/progression or difficulty contribution at a named decision point | Typed validated contribution; deterministic ordering, conflicts and provenance; core retains final integrity checks |
| Scoped behavior | NPC/encounter decisions inside an admitted owner phase | Bounded queries, actions and immediate results where C++ requires them; explicit reentry/partial-effect rules |
| Confirmed notification | Observe a completed transition at its documented confirmation point | Cannot retroactively veto a committed operation; owned payload, delivery/retry semantics and idempotency where relevant |

Every supported hook records: owner, exact C++ anchor or custom contract, admission, before/after
phase, input freshness, action/result, state scope, composition/conflict rule, failure/reentry,
persistence/publication and observability. “Confirmed” means the named operation boundary;
do not imply that every in-memory combat tick is durably committed to SQL.

Use narrow scoped capabilities, not `&mut Player`, `Map`, `hecs::World`, raw runtime IDs,
entity guards, generic queries, SQLx pools or packet writers. An operation may need a query
after an action; a flat deferred effect batch is not sufficient for every script.

Composition is explicit per contract: an ordered validated transform, an associative reduction,
or a declared exclusive policy. Do not silently use last-writer-wins. Each contribution carries
module/rule identity and a reason so the operator can explain a result and remove the relevant
contribution without overwriting another module's work. Bounds/overflow and conflicting exclusive
policies must have tested rejection behavior. The existing compositor's `(order, id)` registration
order and login registry's `ModuleId` callback order are different current contracts; #583 must
define the intended policy semantics rather than assume they already coincide.

##### Synchronous behavior, reentry and failure

The C++ Anomalus path changes phase/casts before attempting a summon; failure does not undo
the earlier effects. Summoning may synchronously call `JustSummoned`/`IsSummonedBy` before
returning. Another path performs an action then reads the boss aura to choose its next timer.
Evade can synchronously call Reset. These constrain the host, not just the storage library.

Before enabling behavior, choose and test either short host-managed state accesses released
before a reentrant action, or explicit continuations preserving the same semantic barriers.
Do not retain a mutable module-state/ECS borrow across callbacks, defer all nested callbacks to
the next tick, or promise rollback of a whole callback after earlier actions succeeded. Record
action failure, partial effects, recursive dispatch limits and the safe outcome of exceeding
them. Revalidate incarnation/residence at each admitted action; a saved runtime handle is not
authority forever. Reentry guards must not silently suppress C++-required behavior.

No synchronous map/entity guard crosses `.await`; no blocking I/O or packet delivery occurs
under a map lock. Deliberate async operation gates may span I/O only with explicit lock order,
blocking scope, cancellation and recovery contracts. Do not remove established money/persistence
fences merely to satisfy a blanket “no locks” slogan. Results requiring later I/O use owned
projections and generation/revision-safe completion under the correct owner.

#### 4. State, identity and durability

Classify every state field as core authority, module-owned authority or reconstructible cache.
Declare its scope: incarnation, encounter, map instance, character or account. Character-scoped
state follows a valid transfer; map-scoped state has an explicit detach/unload policy. Reset,
despawn, failed attach, replacement, logout and shutdown must dispose, retain or transfer it
deliberately. Detached Player state remains valid; active-map operations may return `NotActive`.

Separate three identities: protocol GUID, private incarnation-checked runtime handle, and durable
module scope/key. Never persist an ECS entity ID or treat a reused GUID as the old incarnation.
Persist only declared durable fields under module ID, scope/key and schema version. Do not
automatically serialize every component, transient boss timer or creature reference; C++ itself
normalizes transient encounter states on load.

For a completion reward, the reward and its durable receipt must be coherent. Prefer a single
authoritative transaction when both belong to the same database. Across databases, specify an
idempotent operation token and recovery protocol; do not claim distributed ACID or derive an
exactly-once guarantee from in-memory flags. Unknown COMMIT, retry, concurrent/newer mutation,
cancellation and restart must neither duplicate rewards nor acknowledge lost progress. Keep
the same logical reward/operation identity across retries, configuration or schema upgrades,
and new runtime incarnations; a new schema version must not accidentally grant the reward again.

`rustycore-db` remains the sole schema migration authority. A module never runs arbitrary SQL
from a callback or startup hook. Compatibility, target DB, namespacing, immutable checksums,
dry-run, approval and incomplete-migration recovery apply to module migrations too. Retain
required migration manifests/artifacts and applied history after removing a module checkout;
the current DB compatibility check rejects history absent from the manifest used by the binary. Archived
entries alone are not proof that missing SQL artifacts are acceptable.

Treat **disable execution**, **remove installed code while retaining data/history**, and
**purge durable data** as different operator actions. Disabling stops callbacks according to a
documented drain/restart contract and removes reversible contributions; it does not roll back
legitimately earned rewards. Re-enable and upgrade have tested state compatibility. Purge or
destructive downgrade requires explicit authority; no automatic destructive down migrations.

#### 5. ECS decision now: selective private `hecs`, cohesive aggregates retained

**Selected:** `hecs` (initial pinned baseline 0.11.1) for map-local, independently composable
entity/behavior state behind the canonical owner. Keep cohesive Player/Unit/domain aggregates
where they enforce complete invariants. No global ECS, scheduler replacement, public component
queries, wholesale Player decomposition or obligatory intermediate dense-arena migration.
Catalogs, accounts, matchmaking/LFG and durable I/O retain their own owners outside this choice.

##### Rationale and alternatives

| Option | Decision and evidence |
| --- | --- |
| Current/improved aggregate + state registry | Viable modular implementation and the fallback; not rejected as incapable or unsafe. It does not currently implement the open state registry either. Keep aggregates where composition does not justify a change. |
| Dense generational aggregate + registry | Credible layout alternative, **not benchmarked**. Do not claim hecs beat it. Adding it first would introduce another migration without evidence that it solves an unmet requirement better. |
| Selective hecs | Chosen for typed optional-state composition and storage/query machinery without imposing a system scheduler. V1 supports feasibility at tested costs; the expected reduction in application-specific composition plumbing remains an architectural judgment to verify. |
| Broader ECS framework | Not selected: no demonstrated need for another resources/events/scheduler framework. This is not a claim that Bevy requires a renderer or cannot preserve our driver. |

V1's median paired update-p99 ratios are 0.548–0.744 against its three-HashMap aggregate, but
the timed path includes materializing observable rows. Churn costs 1.49–1.69x and transfers
1.31–1.64x as much. The fixture knows two state types and enumerates their combinations. These
numbers do **not** establish a production speedup, arbitrary module composition or superiority
over a dense store. The selection combines demonstrated feasibility with the desired composition
capability; it is not a claim of experimentally proven global optimality.

##### Finite conformance proof before production migration

This finite proof is complete within #584's recorded laboratory scope; it is not a new issue
or completion of #583's product. The earlier three-candidate experiment became validation of
the selected design, not a prerequisite to naming the choice. Preserve the useful falsification
test and production boundary:

1. Define one private experimental host contract, anchored to the represented C++ owner/callback
   paths and clearly named custom behavior. Implement two independent modules; then freeze the
   host, adapter and contract sources and record their hashes.
2. Add a third module in a separate crate with a new state type and lifecycle rule. Only dependency,
   declarative registration and composition may change. No new host enum/match arm naming that
   type, module-specific storage adapter, broad entity borrow or exposed hecs/SQL/packet API.
3. Exercise zero optional modules (required base scripts remain), mixed/composing modules,
   conflict rejection, state isolation and bounded
   action -> synchronous callback -> read, including nullable action failure and failure after
   prior effects. Cover reset/removal, active/detached transfer, failed attach, replacement and
   stale incarnation, plus versioned snapshot/replay. Include outer read -> nested state mutation
   -> stale outer write: reject the obsolete write and retain the nested result. Label replay as
   a mock, not DB durability. Reject unsupported executor switches without discarding saved state.
4. Run equivalent cases as native Rust, Rust -> Core Wasm and C -> Core Wasm using the same
   semantic contract; also compose native and Wasm modules in one host. Test duplicate executor
   rejection, incompatible versions/capabilities and the resource/failure limits above.
5. Before measurement, fix populations, workloads, repetitions, hash set and CPU/RSS/action/state
   budgets in a new versioned protocol. Measure update, churn, transfer, dispatch and cold costs
   separately; retain every failed sample. V1 provisional budgets are not a server SLA. Report
   central code touched and state/lifecycle plumbing as well as timing; no favourable-run selection.
6. Record pass/fail of the selected implementation and exact remaining production boundaries.
   The [two-module freeze and third-module correctness stages](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/session-578-checkpoint.md#independent-extension-checkpoint--2026-09-05-c67acbfd)
   pass, as do the recorded preregistered cost samples on aarch64. See the [V2 results and
   remaining boundaries](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-conformance-results.md). The finite pre-migration gate is
   complete; production acceptance, actual frame-budget evidence and Wasmtime installation
   remain open. Do not repeat this lab proof as a new "next" task.

An implementation error means fix and rerun the affected case. A Wasm/ABI defect is not evidence
against ECS. Reopen the backend decision only for a demonstrated hecs-specific obstacle such as
unavoidable duplicate authority, inability to support the independent-state/lifetime contract,
or unacceptable measured structural cost after bounded correction. Then compare against the
aggregate + generic registry fallback (a dense library only if layout is the diagnosed issue).
No perpetual candidate carousel, no frozen second live authority, no waived correctness gate.

After that proof, #584 still must exercise real save/admission/phase/two-map/backpressure/shutdown
paths and retire the superseded writers for each migrated family. #583 delivers the production
external-module and durable operator lifecycle, including the required Rust/Wasm/C product. The
[entity-world ADR](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/adr-map-runtime-entity-world.md) records the selection and
integration gates. Production still has no hecs or Wasmtime dependency today.

#### 6. Complete execution sequence and ownership

##### Deferred design TODO — safe hot reload

**Requested 2026-09-05:** after the bounded native/Wasm state and operator-lifecycle
contract is delivered, design safe hot reload under the future #99 extension work.
This records a later design task; it neither enables hot reload now nor expands #584/#583
acceptance or their dependencies.

Evaluate separately reloadable data/configuration, replacement of a Wasm module without
restarting the whole server, and native code (which still requires rebuild/restart under
the selected model). Define safe points and draining of in-flight calls, one active module
version/state authority, versioned state compatibility or migration, bounded interruption,
and failure recovery that retains the old usable module/state when activation fails.
Do not promise rollback of already applied gameplay effects, zero interruption, automatic
native binary replacement or unrestricted live updates. Compare the operational benefit
with the added lifecycle/recovery risk before approving an implementation.

##### Current delivery sequence

| Macro / epic | Deliverable and completion gate | Dependencies |
| --- | --- | --- |
| #584 core / C0–C4 | Complete each core responsibility with canonical ownership, consumers, physical navigability, lifecycle/persistence and scoped acceptance; the finite hecs V2 proof is already passed within lab limits | Closed predecessors provide evidence; no #133 reopening and no new micro-issue per helper |
| #743 | Group-command state delivery and reconciliation across saturation, disconnect, replacement, stale removal and related state-bearing transitions | Current recommended macro; #583 is not a prerequisite |
| #735 | Reputation encapsulation under canonical Player ownership with catalog resolution outside the entity boundary and preserved publication/save flags | Recommended after #743; ordering preference, not a hard dependency |
| Remaining #584 P2 → P3 → P4 | Finish residual operations, then runtime/lifetime/private-hecs integration and semantic/physical decomposition with consumer migration | Select by complete operation contract and real dependency; keep exceptions file-specific |
| #583, under #99 / M0–M4 | Production module product: shared hooks/state/lifecycle, native and Wasm execution, Rust/Wasm/C mixed evidence, durable reward/state and author/operator lifecycle | Waits for required #584 core, but does not block unrelated gameplay macros |
| #153 | Independent terminal audit of the completed #584 core and #583 product; known work stays with its implementation owner | #584 and #583; #133 tracker closure is already recorded |
| Next Part-1 port macro | Re-audit its actual residual path against current Rust/C++; implement complete gameplay responsibility with existing hard dependencies | Ordered #49 index, retaining M0–M6 and relevant prerequisites |
| #48 / Part 2 | Full 1:1 ledgers, nothing dropped; fresh audit/planning after playable #47/M6.2 | Existing Part-2 transition gate; no speculative child tree now |
| #99 future extensions | Expand the public API through real consumers and reusable semantic seams | Evidence-led; no issue/PR per field or callback |
| Wasm bounded delivery / broader language ecosystem | #583 delivers an optional executor with Rust/C bindings and explicit limits; further languages, WIT or hot reload are not implied | No M6 gate for the approved bounded #583 delivery; broader #99 expansion retains fresh planning |

The finite pre-migration conformance checkpoint above is already passed within its lab scope;
do not repeat it as a new "next" task. Continue #584 with the complete #743 contract, then
#735 as an ordering preference, then the remaining P2/P3/P4 responsibilities. Safe same-owner
source/test splits may precede or accompany these operations, but they do not retire semantic
ownership. Preserve the committed Player work. The production module product is one subsequent
#583 delivery, not a series of greeting-size deliverables. Internal focused commits and checks
are not user approval gates; runtime/publication authority remains separate.

##### Reanalysis checkpoints — evidence before replication

These reviews happen inside the approved macros; they do not create another issue, PR or
routine approval request. Continue authorized work after a passing checkpoint. A failure pauses
the affected migration while its cause is investigated and corrected within scope.

| Point | Question the evidence must answer |
| --- | --- |
| Before production storage migration — #584 conformance (§5) | Can the frozen host accept independent state and native/Rust-Wasm/C-Wasm composition without module-specific core edits, duplicate authority, stale reentrant writes or violated limits? This finite gate passed within the recorded V2 laboratory limits; production integration remains open. |
| First production C1/C2 vertical, with its C0 contract — before replicating the pattern | Does one complete operation work through real admission, canonical lifetime/save and ordered publication, including controlled I/O, late acknowledgements, replacement/detach and affected phase/backpressure failures? Check semantic ownership and physical source/test boundaries together. A fixture-only success or renamed phase cannot justify scaling the pattern. |
| C4 — complete #584 balance, before #583 starts production integration | Do all C0–C4 exits hold at the validated SHA, including every remaining owner/bridge, inherited decision, persistence classification and physical-file exception? Reconcile the whole macro, not only its last successful vertical; #583 must not inherit unfinished core work. |
| #583 first real external-module integration, before extending its API | Can independent authors exercise the shared hooks and state contract without a core patch? Validate the supported native/Wasm behavior and real durable/operator lifecycle as they become available; do not extrapolate from a greeting or mock replay. |
| #153 after both macros merge | Independently audit the complete technical gate and evidence at integration HEAD. Known implementation work stays with #584/#583, not the auditor. |
| After architecture, then #47/M6.2 | Re-audit each selected gameplay macro just in time. At the playable exit, perform the fresh whole-port state/plan review before decomposing Part 2/#48; architecture closure is not full-port parity. |

The first production vertical is a focused design stress test, not a second global architecture
audit. Repeat a broader review earlier only when evidence invalidates a shared contract or the
approved scope materially changes; do not wait for #153 to discover a pattern-wide defect.

##### #583 reference capability and acceptance

Use one externally maintained encounter/progression module and a second separately configured
module contributing state/policy in the same scope. The reference covers an XP/progression
decision, transient encounter phases/timers and failing/reentrant actions, and a durable
completion/reward. Select concrete supported actions against merged #578 and record exact
anchors before enabling them. This is not an implicit commitment to port all Nexus content,
all spell effects or LFG inside the architecture issue; missing required paths cannot be replaced
by stubs and called acceptance. Necessary bounded host seams belong in #583, unrelated gameplay
gaps stay with their port owners.

| Proof | Required evidence |
| --- | --- |
| Useful independence | Separate module repositories/commits, public API only, no module-specific core branches; zero-module neutrality and explicit custom behavior |
| Execution portability | Native-only and Wasm-enabled builds; equivalent Rust native/Rust Wasm/C Wasm cases; mixed executors, no duplicate execution, ABI/capability/version rejection and bounded failure behavior |
| Stateful behavior | Real owner paths; action success/failure and read-after-action; synchronous reentry; timers/reset/despawn; second module composition/conflict |
| Lifetime | Two sessions/maps, active/detached transfer, stale generation/replacement, failed attach/unload and shutdown |
| Durable progress/reward | Restart/relogin, duplicate operation, rollback, unknown COMMIT, cancellation and newer mutation; coherent receipt and reward |
| Author/operator lifecycle | Real Git v1 install → v2 update → build/restart → disable/re-enable/remove; lock reproduction, compatibility rejection and retained migration history |
| Diagnostics | Explain contributions/order/conflicts; structured failure results and bounded state/callback diagnostics without secrets |

The CLI currently rewrites Git manifest provenance during install; the subsequent dirty-check
path is an **inspection-based upgrade risk, not a reproduced failure**. #583 must test the real
Git workflow, fixing it if necessary, rather than relying only on path-source fixtures.

#### 7. Validation, stop conditions and future decisions

Terminal acceptance includes independent physical-file and logical-owner reports under the
module design guide, including tests/fixtures and file-specific exceptions. #584 C0–C4 owns
the remaining core physical policy and semantic boundaries; #583 applies the policy to its
own product before #153's audit. The dated 31-file measurement remains evidence, not a new
scan or a standing exception. No permanent Session exception, known future split plan or
moved-file count substitutes for completion.

Use focused positive/negative tests and inexpensive architecture/syntax checks during a cut.
At affected boundaries, use production-linked dev/release integration tests, controlled I/O
interleavings and explicit ticks. Tests must use the production composition/dispatch path;
`cfg(test)` fixture worlds and constant phase strings do not prove it. Before macro publication,
run clean-HEAD `validation-v2 final` plus the issue's complete acceptance and inventories.
Do not repeat exhaustive persistence scans for every internal helper or metadata change.

Capture-diff is required for changed bytes, metadata, connection selection or observable order;
fresh action-specific captures are distinct from existing regression goldens. Live lifecycle
changes need authorized runtime QA. Real MariaDB crash/restart/relogin evidence is required for
durable recovery claims; controlled mock futures cannot prove storage durability. Missing live
authority pauses that mutation/acceptance step, not safe inspection or remaining local work.

Record evidence kind (old-Rust equivalence, C++ contract, production integration, live/capture),
SHA, command, result, host architecture and unproven boundaries. Planning, successful builds,
test counts and moved fields are not gameplay completion percentages. #153 audits a completed
delivery; it must not receive a bucket of unresolved SDK, persistence or owner implementation.

Native modules are trusted source Rust: rebuild and restart, no stable native ABI, hot reload,
sandbox, guaranteed panic isolation or preemptive CPU budget. Capabilities constrain the API,
not native code's filesystem/network access. The selected Wasmtime/Core Wasm adapter must earn
the ABI, resource, host-capability, failure and lifecycle acceptance above before being enabled.
Its optional installation does not make its #583 acceptance optional. Blocking analytics/webhooks belong outside the
synchronous simulation path and need no in-process ECS access.

#### 8. Evidence and source anchors

Recheck locations against the implementation HEAD; these support the decision, not full parity.

- Rust: `crates/wow-map/src/map/entity_world.rs`, `crates/wow-map/src/map/runtime.rs`,
  `crates/wow-map/src/manager/player_owner.rs`;
  `crates/wow-module-api/src/{hook,effect,registry}.rs`; `tools/modules/{compose.py,rustycore-module}`;
  `crates/wow-database/src/migration.rs`; `tools/architecture/map-runtime-spike`.
- C++ under `/home/server/woltk-trinity-legacy/src/server/`: `game/Entities/Player/Player.cpp:2189-2226`
  (XP hook order); `scripts/Northrend/Nexus/Nexus/boss_anomalus.cpp:81-170,232-241`
  (state, partial failure and read-after-action); `game/Entities/Object/Object.cpp:1956-1972`
  and `game/Entities/Creature/TemporarySummon.cpp:249-264` (synchronous summon callbacks);
  `game/AI/CreatureAI.cpp:219-242` (evade/reset); `game/Instances/InstanceScript.cpp:374-473`
  and `game/Instances/InstanceScriptData.cpp:137-142` (durable transitions and transient reset);
  `game/Maps/Map.cpp:666-813`, `game/Maps/MapManager.cpp:287-318` and
  `game/Server/WorldSession.cpp:64-108` (execution/admission/barriers).
- AzerothCore is a product/capability reference, not the 3.4.3 behavioral oracle:
  [module structure](https://www.azerothcore.org/wiki/the-modular-structure),
  [hooks](https://www.azerothcore.org/wiki/hooks-script),
  [AutoBalance](https://github.com/azerothcore/mod-autobalance).
- Storage alternatives: [hecs](https://github.com/Ralith/hecs),
  [HashMap disjoint mutation](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get_disjoint_mut),
  [slotmap](https://docs.rs/slotmap/latest/slotmap/),
  [Bevy ECS](https://docs.rs/bevy_ecs/latest/bevy_ecs/).
- Wasm: [language support varies](https://component-model.bytecodealliance.org/language-support.html),
  [Wasmtime resource configuration](https://docs.rs/wasmtime/47.0.3/wasmtime/struct.Config.html).
  Component-language tooling is contextual evidence, not proof our Core Wasm ABI supports it.

#### Historical synchronization records — 2026-09-05 and earlier

Everything in this section is dated planning evidence retained for provenance. It does
not override the current status and sequence at the beginning of this document, STATE.md,
or `PORT_PLAN.md`/#49. In particular, old references saying that #133 or #578 were open
are historical and do not create a future prerequisite.

##### Earlier plan synchronization evidence — 2026-09-05

On the aarch64 development host, with production HEAD still `93e4002a` and planning changes
uncommitted above `32d9a683`:

- Created #583 and updated/read back the exact bodies of #133, #578, #153, #99 and #49.
  All remain open; completed predecessor states are preserved. #99's title now makes Wasm optional.
- The architecture ledger records #583 with parents `[133,99]`, prerequisites `[231,578]`,
  and #153 prerequisites `[184,578,583]`; its documented topological order agrees.
- `check_architecture.py check` and `self-test`: PASS; syntax-only Session ownership: PASS.
- Preserved persistence issue-reference/classification check and
  `checked_persistence_policy_matches_the_checked_snapshot` (release): PASS. The 60 historical
  nonstable groups still attributed to #153 are preserved; #578 C4 retains their semantic
  reconciliation obligation. No exhaustive persistence inventory was regenerated.
- Live `check_architecture.py refresh-issue-state --check`: PASS; read-only remote validation
  required network permission. The check did not rewrite other policies or ledgers.
- `git diff --check`: PASS. Independent adversarial review found no dependency cycle and led
  to explicit shared C0 workload contracts and stable reward identity across upgrades/retries.

These are planning/metadata checks, not clean-HEAD macro-final validation, a live module demo,
gameplay parity or a new benchmark. No production code, runtime, schema, migration inventory,
PR contents or dependency allowlist changed. No commit or push was made. Earlier skill edits
and the unrelated local LFG audit remain separate from this plan update.

##### Latest decision synchronization and validation — 2026-09-05

At laboratory HEAD `ee9a0128`, before the documentation commit:

- Updated and read back exact title/body/state for GitHub #133, #578, #583, #153, #99 and #49.
  All remain open. #99/#583 titles now identify native/Wasm; #583 explicitly expands #133's
  closure. Removed the contradictory post-M6 prerequisite and Wasm exclusion for this delivery.
- The existing approved DAG is retained: #578 depends on #378/#574; #583 on #231/#578;
  #153 on #184/#578/#583. No new issue or inverse SDK dependency. Ledger/title/doc synchronization
  and live `refresh-issue-state --check`: PASS.
- On aarch64, `check_architecture.py check` / `self-test`: PASS; 38 packages, 101 workspace
  edges. `session-ownership-check check --syntax-only`: PASS, 282 production + 432 fixture fields.
- Preserved persistence policy/workflow reference/classification check: PASS, 120 references.
  `checked_persistence_policy_matches_the_checked_snapshot` in release: 1 passed, 0 failed.
  No exhaustive persistence scan or baseline regeneration; the historical #153 reconciliation
  obligation stays in #578 C4.
- `git diff --check`: PASS. Independent design reviews support the scoped hecs choice and led
  to explicit stale-outer-write and native/Wasm state-switch rejection acceptance.

These checks validate planning/metadata consistency, not the unimplemented conformance gate,
new benchmark results, production ECS/Wasm integration, DB durability, live/capture behavior or
clean-HEAD macro-final acceptance. No runtime/code, dependency allowlist, DB, PR or issue state
was changed. This documentation commit preserves the earlier approved plan changes while leaving
skill edits and the unrelated LFG audit outside it; no push or merge is included.

##### Module-design policy adoption — 2026-09-05

Reviewed above planning HEAD `816d5c84`, with production code still `93e4002a`:

- Updated/read back exact GitHub bodies #133, #578, #583, #153, #99 and #49. Titles, states
  and the existing dependency DAG are unchanged; all six remain open. No new issue or PR.
- Added [module design guidelines](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md): independent physical and
  semantic acceptance, project size budgets, bounded exceptions, test/fixture decomposition
  and an illustrative Rust submodule skeleton. #578 C2/C4 own remaining core decomposition
  and the physical checker extension; #583 applies the policy to its own module product.
- Updated the existing architecture and safe-refactor skills, preserving the earlier approved
  simplification. The canonical policy stays in project docs, not duplicated skill snapshots.
  A four-scenario independent instruction review led to conditional opcode checks, a thin
  `main`/composition distinction and an explicit non-universal persistence example. No new
  routine approval or micro-PR requirement was added.
- Only three narrative hotspot-ledger fields changed; numeric ceilings, members, paths,
  issue references and the historical persistence snapshot remain identical to HEAD.
- On aarch64, architecture check/self-test, syntax-only Session ownership and live
  `refresh-issue-state --check`: PASS. Both skill validators and `git diff --check`: PASS.
  `checked_persistence_policy_matches_the_checked_snapshot` (release): 1 passed, 0 failed.
- Bounded validation against the persistence policy rules: PASS, 1,041 classified groups,
  1,038 workflows and 120 preserved issue references. Policy/workflows/snapshot are identical
  to HEAD; historical #153 attribution still requires C4's semantic reconciliation. This is
  reference/classification consistency, not a fresh exhaustive source inventory or DB evidence.

This is policy/planning and metadata validation, not implementation of physical enforcement,
source decomposition, production ECS/Wasm migration, gameplay parity, live/capture acceptance
or clean-HEAD macro-final validation. No production source, dependency, schema, runtime or PR
was changed. The unrelated local LFG audit stays outside this change; no push or merge.

---

<a id="modulos"></a>

## D. Política completa de módulos y ficheros

Fuente mantenida: [docs/architecture/module-design-guidelines.md](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md).

### Module design and source navigability

**Allocation/status revision, 2026-09-11:** #133 was closed on 2026-09-09. The
remaining core semantic/physical work previously assigned below to #578 C2/C4 is
owned by coordination epic #584 and its analyzed crate-focused implementation
children. #578/#585/#587/#588/#589/#716/#718/#722/#737 are integrated and closed in
their bounded scopes. The technical gate remains #584 core → #583 native/Wasm
product → #153 independent audit; #583 does not block unrelated gameplay work, but
its production module product waits for required #584 work. Keep the Rust/Wasm/C
mixed product mandatory even though operator activation is optional. This changes
delivery allocation, not budgets, invariants or global terminal acceptance. No next
crate is selected. Each macro includes its consumers and both criteria below; #153
must not inherit implementation work. References to #578 below describe the former
allocation or historical evidence unless explicitly updated.

**Approved project policy: 2026-09-05.** Applies to RustyCore's internal refactors,
new code, integrated tooling and module SDK/examples. This is a maintained design
contract, not a claim that the current tree already complies. The initial review
used production code `93e4002a`, unchanged at planning HEAD `816d5c84`.

#### 1. Two independent acceptance criteria

A completed responsibility must have **both**:

1. **Semantic modularity:** a named canonical owner, narrow inputs/results, explicit
   dependencies, preserved invariants and complete reader/writer migration.
2. **Physical navigability:** cohesive, manageable source and test files, with a
   discoverable module tree. A reader should find the rule, use case, adapter and
   tests without loading an entire subsystem's implementation.

Splitting a giant `impl WorldSession` into fifty files is useful mechanical progress,
not proof that Session no longer owns gameplay. Conversely, moving authority to Player
does not justify a 70,000-line Player file. A legitimate aggregate can span many private
modules while retaining one identity, one state and invariant-preserving operations.

#### 2. Structure by responsibility, using Rust's boundaries

Use a **modular monolith, domain-oriented within the existing dependency layers**:

- Domain modules express rules, invariants and state transitions. They do not retain
  Session, SQL connections, packet writers or composition/configuration bags.
- Application use cases coordinate complete operations through narrow domain and
  persistence capabilities. They preserve admission, commit classification, canonical
  mutation and publication order; they are not a new all-purpose `GameService`.
- Adapters translate protocol/persistence representations and effects. Handlers decode,
  admit and invoke; repositories follow transaction boundaries, not one trait per table.
- Composition constructs concrete dependencies and supervises lifecycle. It does not
  become a second owner of gameplay.

Prefer named features such as `quests`, `inventory` and `combat` over growing global
`services`, `managers`, `helpers` or `utils` buckets. A small cohesive area can remain
one module; do not create four empty layers for every operation. DDD informs language,
invariants, aggregate boundaries and explicit relationships; it does not prescribe a
PHP-style directory tree, one crate per aggregate, or microservices. A feature folder
is not automatically a bounded context, especially when it shares Player invariants.

Use private modules/submodules first. `mod` declares a module; `use` only imports a
path into scope. Both `quests.rs` + `quests/` and `quests/mod.rs` + child files are
valid; follow the local convention and do not rename every `mod.rs` for style alone.
Keep root files as a small facade, declarations and essential wiring. Expose only the
required operations through `pub use` or deliberately scoped visibility; do not make
state public to move tests. Add a crate only for a useful independently checked API,
dependency/build boundary or real external consumer. No organizational marker traits,
new locks, cloned mirrors or untyped service locators merely to split source files.

Source layout does not select a storage engine. Private selective hecs, public module
hooks and native/Wasm execution retain the decisions and gates in the
[modularity/ECS plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-and-ecs-plan.md). None substitutes for the boundaries here.

#### 3. Physical budgets and bounded exceptions

These are **project navigation budgets**, not universal Rust requirements:

| Physical handwritten file | Required treatment |
| --- | --- |
| Usually 200–800 lines | A useful target for cohesive files, **not a minimum** and not permission to pad or fragment code. Small facades and simple modules can be much shorter. |
| Above 1,000 lines | Review cohesion and the next natural split during the ordinary task review. The agent can resolve this with evidence; it is not a new user-approval gate. |
| Above 2,000 lines at a responsibility/macro closeout | Split by responsibility, or record a concrete file-specific exception with the evidence below. A generic legacy or aggregate exemption is insufficient. |

Count physical lines, including comments and blanks. Handwritten tests, fixtures,
integration tests and integrated tool sources count too; moving production code into
tests or another language/directory is not retirement. Keep production/test attribution
as useful additional data, not a way to bypass a large physical file. Source generated
from a reproducible generator is reported separately with its generator/input provenance;
do not relabel handwritten tables or fixtures as generated without that evidence.

Each exceptional file records its exact path, responsibility, observed count and reviewed
ceiling, implementation owner/issue, reason a coherent split is currently unsafe or less
clear, and a **bounded exit condition or named review checkpoint**. A temporary exception
expires at that checkpoint unless its evidence and ceiling are explicitly reviewed again;
being attached to an open issue does not renew it. A justified cohesive exception may remain
at closeout only with that explicit record. Session/Map/Player identity alone, an unfinished
ownership migration or the historical 4,000-line signal is never a standing exception.

Do not enforce a new global limit by blocking all work on pre-existing files. During
migration, inventory the legacy files, assign their responsibility splits to the existing
macro and use per-file non-growth ceilings. Tighten those ceilings after each coherent
validated reduction. Necessary focused tests or an inseparable in-scope transition may
increase a ceiling only through an explained, reviewed delta and a retained split exit;
never automatically refresh baselines to make checks green. Once a responsibility is
declared complete, its files must meet the terminal budget or the specific exception rule.

Retain the **logical-owner** inventory as a separate metric: it catches gameplay still
coupled to Session across many files. Logical totals are not subject to a per-file cap,
and physical splitting alone must not remove logical ownership debt.

##### What a physical division cannot reach

**Review correction, 2026-09-10, #716:** the earlier examples describe limits of
item relocation, not proof that useful private decomposition is impossible. Preserve
their measurements without treating a parser item or a LOC ceiling as the design owner.

**Keep one implementation while delegating cohesive work.** Rust's single applicable
trait impl can contain short methods that delegate to private modules without changing
the trait, type or state owner. `player/lifecycle_adapter.rs` already has `economy`,
`login_reads` and `save_plan` children; its bodies can use those responsibilities.
An exhaustive `match` can retain its dispatch and delegate branch bodies. A large
`CharStatements` enum may instead justify a cohesive file-specific exception. Evaluate
these cases separately; a single item does not establish a blanket exemption.

**Review wiring growth without erasing semantic debt.** The hotspot ratchet counts
the whole owner, so private module declarations/imports and formatting can raise its
physical line total even when operations are unchanged. #713 measured +63/+43 test
lines for loot/quest fixture splits; #697 recorded +6 Session lines from path wrapping.
Those are costs to review, not evidence that the new module boundary is wrong. An
explained delta may update only the affected ceiling while preserving the logical owner,
its complete descendants and retirement obligation, as the existing policy permits.
Keep the before/after evidence; never reset all baselines or count relocation as retired
gameplay ownership. Adding a helper does not require a new issue or approval round.

**Keep one operation coordinator and its ordering contract.** Long operations such as
`handle_void_storage_transfer_with_generators_like_cpp` and the spell-effects executor
can extract cohesive private phases while retaining one coordinator, authority and
transaction/publication sequence. Review captured values, borrows, cancellation, early
returns and failure paths before extraction. If a faithful split is unclear, retain a
specific bounded exit; do not infer that the operation must remain one physical function.

**Names and imports are part of navigability.** New private files name a rule, operation,
adapter or scenario. `state_1`, `ops_2` or `scenarios_14` is a temporary relocation label
unless its number has domain meaning. Reunite definitions and operations by responsibility
when completing the family. Prefer explicit production imports and deliberate facade
exports; `use super::*` in a small test or a transitional `#[path]` mount can remain when
its real dependencies, visibility and analyzer coverage are preserved. Do not widen state
visibility merely to make a split compile or replace these criteria with a zero-glob quota.

#### 4. Tests and a complete operation

Keep small private unit tests beside the rule, and split larger suites by responsibility
and scenario. Share narrow fixture builders only where genuinely reusable; do not replace
`session_tests.rs` with a giant `test_support.rs`. Preserve every test registration, feature
gate and production/private behavior under test. Renaming or relocating tests must not
silently reduce the set executed. Production-linked integration tests remain distinct
from `cfg(test)` fixture-only paths.

For example, accepting or rewarding a quest spans protocol admission, domain eligibility,
an application operation, the appropriate persistence contract and ordered publication.
The exact sequence follows the C++ operation and existing durability guarantees, not a
universal template that adds a database transaction to every action. Domain rule tests,
application failure/interleaving tests and packet/capture tests belong with those boundaries.

Record before/after files and sizes, the final owner/dependencies, retired access/bridges,
and focused evidence for each completed family. Keep useful small implementation commits
inside the approved macro. Neither a file split nor a green size check is the whole task.
Do not create a PR, issue or user confirmation for every helper.

#### 5. Example skeleton — target shape, not an implemented directory migration

The following selectively expands current crates. Names are illustrative; preserve actual
public paths and registrations during migration. Each directory has a small module root
(`mod.rs`, omitted below where uninteresting), not an implicit auto-loaded folder.

```text
crates/
├── wow-world/src/
│   ├── session/
│   │   ├── mod.rs                 # Session facade; no gameplay rule dump
│   │   ├── dispatch.rs            # sole registered handler call path
│   │   ├── admission.rs           # connection/status admission
│   │   └── lifecycle/             # session-facing lifecycle adapters
│   ├── handlers/quest/
│   │   ├── accept.rs              # decode/admit/invoke
│   │   └── reward.rs
│   ├── application/quests/
│   │   ├── accept.rs              # complete operation, narrow capabilities
│   │   ├── reward.rs
│   │   └── tests/
│   │       ├── accept.rs
│   │       └── reward_failures.rs
│   └── presentation/quests/
│       ├── dialog.rs              # result → packets/recipient intent
│       └── rewards.rs
├── wow-entities/src/player/
│   ├── mod.rs                     # one Player identity and private state
│   ├── quests/
│   │   ├── mod.rs                 # narrow domain API
│   │   ├── eligibility.rs         # CanTakeQuest rules
│   │   ├── objectives.rs
│   │   └── tests/
│   │       ├── eligibility.rs
│   │       └── objectives.rs
│   ├── inventory/                 # invariants, not another Player copy
│   ├── progression/
│   └── combat/
├── wow-map/src/
│   ├── map/
│   │   ├── mod.rs                 # Map facade and explicit phase order
│   │   ├── runtime/               # admitted simulation operations
│   │   ├── visibility/
│   │   ├── respawn/
│   │   └── entity_world/          # private storage implementation
│   └── manager/
│       ├── player_owner.rs        # incarnation/residence authority
│       └── tests/                 # lifetime/transfer/failure cases
├── wow-persistence/src/
│   ├── lib.rs                     # semantic contracts, small facade
│   ├── player_save/               # operation DTOs and classified outcomes
│   └── quest_reward/              # only if a real operation needs this port
├── wow-database/src/
│   └── player_lifecycle/          # SQL adapters, transaction plans, tests
└── world-server/src/
    ├── main.rs                    # entrypoint (already small)
    ├── app.rs                     # process construction/supervision facade
    ├── bootstrap/                 # catalogs/config/repositories/session wiring
    └── runtime/                   # task supervision and delivery wiring
```

The former `session_tests.rs`, `map_tests.rs` and `main_tests.rs` distribute to the
responsibilities they exercise, including transport/session tests left near Session,
not only the quest examples drawn here. Preserve integration scenarios in crate-level
`tests/` targets when they need the real public/production composition path.
The SDK, native modules, Wasm host/bindings and integrated QA tools use the same rules;
this tree does not create a new SDK layout or claim #583 has already delivered it.

A tiny Rust example of one type implemented through a private submodule in the same crate
([Rust Reference: multiple inherent implementations](https://doc.rust-lang.org/reference/items/implementations.html#inherent-implementations)):

```rust
// player/mod.rs
mod progression;

pub struct Player {
    level: u8,
}

// player/progression.rs
use super::Player;

impl Player {
    pub fn level(&self) -> u8 {
        self.level
    }
}
```

This defines one `Player`, not two objects or an additional crate. Child modules can
access the parent's private items. Where a child owns a private substate, use its narrow
operations rather than widening all its fields for sibling access. Actual gameplay
methods must still enforce the relevant invariants; the getter only illustrates layout.

#### 6. Implementation ownership and honest enforcement

- **#133:** closed on 2026-09-09 in the tracker. Its technical acceptance is carried
  forward by the required #584 core work, #583 product and #153 audit; do not reopen
  the umbrella or wait for another #133 transition.
- **#584 C0–C4:** owns the remaining core/adapter/composition/tooling hotspots and
  every completed operation family's semantic and physical criteria, including its
  tests and consumers. Safe same-owner mechanical splits can precede or accompany
  the finite hecs conformance evidence; conformance gates production storage
  integration, not source organization.
- **#583:** its SDK, hosts, bindings, module examples and supporting tooling meet the same
  criteria in its own macro, including the required Rust/Wasm/C mixed product. It does
  not inherit unfinished core decomposition, and its implementation does not block
  unrelated gameplay macros.
- **#153:** independently verify both implementation macros, file exceptions and semantic
  boundaries. It is an audit, not the implementation owner of known cleanup.

The #578 C4 implementation above `8f5caedc` adds the physical branch to the **same**
`check_architecture.py` entrypoint; it is not another standalone checker:

```bash
python3 tools/architecture/check_architecture.py physical-files
python3 tools/architecture/check_architecture.py physical-files --json
python3 tools/architecture/check_architecture.py physical-files --terminal
```

`physical-file-policy.json` records 103 oversized legacy paths at introduction, their
observed/per-file ceilings, concrete split targets and the `578:C4` review checkpoint.
These are **migration debt, not terminal exceptions**. Growth or a missing/renamed legacy
path fails until its exact policy row is reviewed. Reductions pass and the ceiling should
be tightened after the validated split. New handwritten files default to the 2,000-line
ceiling; files above 1,000 are reported for ordinary cohesion review. No policy-generation
command exists. `--terminal` rejects every still-oversized migration entry: a normal
migration PASS is not proof the physical deliverable is complete.

Inventory covers tracked and untracked nonignored repository Rust, Python, shell,
C/C++, JS/TS and protobuf sources, including integration tests, tooling, vendored
code and extensionless shebang scripts. It is not restricted to `crates/*/src`.
Tracked source cannot disappear behind a new ignore rule. Ignored build outputs/private
files, documents, SQL/data inventories and arbitrary external compiler inputs are not
this repository-source inventory; existing logical/cfg/module-mount checks remain independent.
Source symlinks fail closed; non-source documentation/assets may remain symlinks.

Generated attribution requires pinned output/generator/input hashes, a reproduction
command and a hash-pinned JSON reproduction record matching that exact provenance and
reproduced output hash. The read-only checker verifies the recorded chain; it does **not**
execute generators or independently prove an unevidenced reproduction claim. Review and
actually reproduce a generator before adding its entry. Initially there are zero generated
waivers: `misc_generated.rs` is counted as handwritten despite its name. Terminal exceptions
also start empty; each needs path, responsibility, measured count/ceiling, issue, rationale,
review checkpoint and bounded review/expiry dates. Completed checkpoints and expired dates
invalidate the exception; merely leaving its issue open does not renew it.

`check` and `self-test` enforce physical and logical guards. `validation-v2 final` includes
the cheap physical scan for every nonempty diff, including tooling-only or generation-input
changes; workspace Rust additionally retains its independent logical ratchet. Changes to
the physical module/policy run its adversarial unit suite during `quick`; changes to the
shared checker/scanner run the existing architecture self-test. Macro closeout must also
run `physical-files --terminal`; it is deliberately not the daily migration gate. All
remaining core physical splits belong to #584; #583 applies the policy to its own
product and #153 verifies both without inheriting implementation.

#### 7. Evidence and design references

Initial physical examples (lines including blanks/comments, at `816d5c84`): Session root
76,793; Session tests 96,845; Map tests 18,288; `world-server/app.rs` 5,652;
`wow-persistence/lib.rs` 4,513. These are navigability observations, not production-only
LOC or parity percentages. Recompute at the implementation checkpoint.

Relevant C++ anchors under `/home/server/woltk-trinity-legacy/src/server/game/`:
`Entities/Player/Player.cpp:14087` (`CanTakeQuest`) and `:15675` (quest dialog),
`Maps/Map.cpp:666` (update phases), `Server/WorldSession.cpp:64` (packet processing).
They anchor responsibilities/behavior, not a requirement to reproduce C++ file sizes.

The Rust Book explains [module privacy and submodules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html),
[separating modules into files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html)
and [Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).
DDD's [bounded contexts](https://martinfowler.com/bliki/BoundedContext.html) concern
model boundaries and relationships, not a mandatory Rust skeleton. The hybrid layout
and numeric budgets above are this project's design choice, not claims made by those sources.
