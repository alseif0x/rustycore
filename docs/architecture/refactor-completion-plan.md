# Plan completo para terminar el refactor de RustyCore

Fecha: 2026-09-10. Destinatario: Claude o el siguiente agente que continúe el trabajo.

## 1. Objetivo y alcance

Terminar una arquitectura en la que cada operación tenga un propietario claro,
dependencias explícitas y archivos comprensibles. Revisar organización, nombres,
módulos, submódulos, crates, pruebas, persistencia, composición, runtime y extensión.
El objetivo no es conservar la forma actual ni producir más archivos pequeños.

Se mantiene la paridad funcional completa con el servidor TrinityCore derivado de
WoW 3.4.3. Separar archivos, tener tests o guardar el estado dentro de Player no
demuestra por sí solo que una responsabilidad esté correctamente implementada.
No reducir silenciosamente el port a las operaciones actualmente representadas.

El usuario pidió una revisión independiente, aprobó empezar a corregirla y pidió
este plan completo para poder continuar con Claude. La entrega activa es #716,
la reparación de las herramientas de aceptación y de las reglas de diseño. La
implementación de gameplay posterior requiere primero el contrato de su operación;
una reparación intencional de comportamiento no se disfraza de refactor mecánico.

Este documento es el plan de continuación solicitado. La política semántica y los
contratos del SDK siguen en [modularity-and-ecs-plan.md](modularity-and-ecs-plan.md);
las mediciones actuales y sus límites siguen en [STATE.md](../migration/STATE.md).
Actualizar esos documentos cuando cambie su materia, sin mantener estados rivales.

## 2. Punto de partida y entorno que se debe conservar

- Repositorio: `/home/server/rustycore`; integración: `3.4.3`.
- Revisión inicial realizada sobre `aff42a5166530de948cc3b1b94dc4e813c538bff`,
  actualizado desde `origin/3.4.3`, no sobre la antigua rama local de #587.
- Worktree de implementación: `/tmp/rustycore-architecture-716`.
- Rama: `716-archcore-restore-ownership-provenance-and-acceptance-after-module-decomposition`.
- Issue activo: [#716](https://github.com/alseif0x/rustycore/issues/716), dentro de #584.
- El checkout original conserva cambios ajenos en
  `docs/architecture/modularity-and-ecs-plan.md`, `docs/migration/STATE.md` y
  `docs/architecture/lfg-343-audit.md`. No copiarlos, descartarlos ni incorporarlos
  automáticamente a esta entrega.
- Referencia Classic: `/home/server/woltk-trinity-legacy`, revisión contrastada
  `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
- Referencia complementaria autorizada: `/home/server/azerothcore-wotlk-reference`.
  Consultar su pin/cobertura en `docs/README.md`; 3.3.5a no sustituye wire/SQL 3.4.3.
- Toolchain: el declarado en `rust-toolchain.toml` y `Cargo.toml`.
  Host aarch64; `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`.

Antes de continuar, verificar el estado real del worktree, sus procesos y el SHA:

```bash
cd /tmp/rustycore-architecture-716
git status --short --branch
git log --oneline --decorate -8
sed -n '1,80p' docs/migration/STATE.md
```

Leer `AGENTS.md`, la skill de arquitectura y la de refactor seguro cuando corresponda.
No hacer `reset`, sobrescribir archivos ni cambiar de rama en el checkout sucio para
obtener una apariencia limpia. Este plan no concede permiso de push, merge, despliegue,
reinicio ni escritura en bases de datos reales. Preparar el resultado local revisable
antes de solicitar la autorización externa que todavía haga falta.

## 3. Diagnóstico que justifica el plan

Mediciones del SHA de revisión; volver a medir cuando cambie el código relevante:

| Área | Evidencia | Implicación |
| --- | --- | --- |
| Dependencias | 38 paquetes, 101 aristas internas, 57 dependencias externas vigiladas; 15 excepciones internas y 2 externas | Hay separación física, pero cada excepción necesita una disposición semántica. |
| Session | 221 campos de producción y 427 de fixtures; agregado lógico de 191.799 líneas, 83.158 de producción y 108.641 de pruebas | No confundir los 648 campos totales con 648 campos de producción. Mover impls no retira ownership. |
| Tamaño físico | 63 archivos por encima de 1.000 líneas; 31 por encima de 2.000 | El modo migración pasa, pero el modo físico terminal falla. |
| Nombres e imports | 465 nombres de fuentes numerados; abundancia de `use super::*`, reexports generales y montajes `#[path]` | La fragmentación ha conservado agrupaciones arbitrarias y dependencias poco visibles. |
| Bridge perdido por el analizador | `world_creature_from_pending_respawn_like_cpp` pasó a `map_manager/pending_respawn.rs`, conservando el cuerpo y `use super::*` | El analizador anterior perdía el import canónico del padre. Una baseline menor habría ocultado deuda viva. |
| Estado canónico | Player almacena estado, pero `player/vitals.rs::gameplay_state_mut` permite reglas y mutaciones externas desde Session | La ubicación del dato no basta: trasladar también invariantes y transiciones. |
| Recompensa de misión | Concesiones persistidas por separado; el guardado de estado de misión descarta `Failed`/`Unknown` | Hay riesgo de operaciones parciales y reintentos; necesita un contrato completo. |
| Runtime | Seis relojes trazados; fases repartidas entre Session, runtime canónico y legacy | No se ha probado doble ejecución. Hay que contrastar llamadas y orden real antes de consolidar. |
| Extensión | Existe integración nativa acotada de login; no cierra el contrato estatal nativo/Wasm de #583 | La presencia del SDK o un ejemplo no equivale a la aceptación completa. |

Conservar los logros que sí tienen contratos útiles: residencia e incarnation canónicas,
proyecciones y reconocimiento de guardado, cuarentena ante COMMIT desconocido del
guardado completo, fencing de dinero, lifecycle de cast y registro único de handlers.
Las entregas #578, #585, #587, #588 y #589 están integradas y cerradas en sus alcances.
No reabrirlas por preferencia nominal ni declarar que cierran todo #584.

## 4. Reglas de la estructura objetivo

1. **Player/Map poseen estado e invariantes.** Una transición completa tiene una
   autoridad mutable; eliminar lectores/escritores antiguos al migrarla. No crear un
   segundo espejo de Player, locks adicionales o campos públicos para facilitar el movimiento.
2. **La aplicación coordina la operación.** Recibe capacidades concretas; no una
   Session completa ni un contexto universal. Coordina admisión, planificación,
   persistencia y publicación según el contrato real de esa operación.
3. **El handler adapta el protocolo.** Decodifica, aplica la admisión de sesión y
   llama al caso de uso. Conservar opcode, conexión, metadatos, bytes y orden observables.
4. **Persistencia expresa datos y resultados.** `wow-persistence` conserva contratos
   sin SQLx; `wow-database` implementa SQL/transacciones. Las reglas de gameplay no
   deben requerir transporte, SQL o un writer para evaluarse.
5. **Composición construye y supervisa.** `world-server` conecta recursos y tareas;
   mantiene una secuencia visible de arranque, cierre y tratamiento de errores.
6. **Submódulos privados antes que crates.** Crear un crate solo cuando una dependencia,
   superficie pública o consumidor real justifique la frontera. No un crate/trait por helper.
7. **El propietario lógico y el archivo físico se evalúan por separado.** Un agregado
   válido puede tener varios archivos privados. Una raíz pequeña con cientos de impls
   de Session repartidos continúa siendo una responsabilidad distribuida de Session.
8. **Conservar un coordinador no exige una función gigante.** Un único trait impl o
   match puede delegar fases cohesivas. Un incremento explicado de imports/montajes
   puede aceptarse con una modificación puntual de su techo, conservando toda la deuda.

Los límites numéricos y excepciones vinculantes están en
[module-design-guidelines.md](module-design-guidelines.md). Los 1.000/2.000 de esta
revisión son indicadores de revisión y cierre, no permiso automático para nuevos
archivos de ese tamaño. Aplicar también los presupuestos de archivos nuevos, tests y fixtures.

### Árbol orientativo, no orden de traslado masivo

```text
wow-world/src/
  session/                 conexión, admisión, dispatch, adaptación y lifecycle
  handlers/quest/          accept, reward, abandon, share: adaptación de paquetes
  quest/
    application/           operaciones completas y sus resultados
    presentation/          intención de publicación -> protocolo/destinatarios
    tests/                 escenarios de operación y recuperación
  spell/                   cast y acquisition: reunir cuando se migren sus consumidores
  player/                  partes del Player canónico que hoy residen en wow-world
  legacy_runtime/          nombre explícito mientras queden obligaciones legacy
wow-entities/src/
  player/                  reglas/estado de subdominios que no dependan de Session o SQL
wow-map/src/
  manager/player_owner/
  map/                     fases, visibilidad, respawn, almacenamiento privado
wow-persistence/src/
  player/                  proyecciones y contratos de carga/guardado
  quest_reward/            solo si el contrato durable de la operación lo exige
wow-database/src/
  player/lifecycle_adapter/ un impl delgado, helpers por lectura/guardado/economía
  quest_reward/            implementación SQL de ese contrato
world-server/src/
  app.rs                   secuencia de composición comprensible
  bootstrap/               bases de datos, catálogos, realm
  runtime/                 mapas, sesiones, respawn writer, cierre
tools/wow-test-bot/src/
  runner/
  client/
  scenarios/               login, quest, void_storage, spell_cast...
```

Elegir la ubicación final de cada parte de Player mediante sus dependencias y su
autoridad actual. El árbol no autoriza copiar el Player de `wow-world` a
`wow-entities`, duplicarlo ni trasladarlo entero sin migrar consumidores.
Respetar las fronteras de cast/acquisition entregadas mientras se gana una agrupación mejor.

## 5. Secuencia de ejecución y criterios de cierre

### P0 — Recuperar confianza en las herramientas y reglas de aceptación (#716)

**Trabajo de esta entrega:**

- Resolver imports de módulos por grafo lógico suministrado, incluidos padres,
  globs relativos, alias y `cfg`; mantener sombreado local y aislamiento entre paquetes.
- Detectar contexto relativo faltante y ciclos relevantes; un cambio de nombre o
  de fichero no puede hacer desaparecer una autoridad del inventario.
- Mantener la prohibición de globs externos opacos capaces de ocultar autoridad.
- Trasladar exactamente las dos ubicaciones de fixtures de `WorldSession::new`
  y la ubicación del bridge de `pending_respawn`, conservando huella y multiplicidad.
- Reparar la confusión entre el módulo de datos `inventory` y el crate de registro,
  con prueba de dependencias y manteniendo la prohibición de alias de registro reales.
- Corregir la falsa prohibición de separar un trait impl; revisar solo los incrementos
  puntuales de montaje/fixtures y actualizar los punteros de entregas ya cerradas.
- Compartir un grafo completo de fuentes entre las pruebas que verifican el repositorio.
- Conservar los 65 registros anteriores tras la reubicación y añadir los siete accesos
  existentes recuperados por el analizador: seis métodos de WorldCreature y una fixture
  de visibilidad. Sus anclas y límites están en la entrega actual del plan semántico.

**Aceptación:** pruebas de la herramienta, ownership sintáctico, checks/self-tests de
arquitectura, referencias de persistencia preservadas, consistencia snapshot/policy,
formato, diff y perfil local aplicable. Comparar el conjunto exacto de bridges, no su
cantidad solamente. Revisar individualmente cualquier acceso adicional que aparezca.

**Límite:** P0 no cambia el servidor, SQL, packets ni tareas runtime. No cierra la deuda
física terminal ni las operaciones de gameplay. El estado ejecutado se registra en la
sección 9; no asumir que una tarea de esta lista está aceptada solo por aparecer descrita.

### P1 — Recompensa de misión como primera operación completa

**Seleccionar primero el contrato**, sin empezar por mover `rewards.rs`:

1. Trazar todos los callers: entrega de recompensa en
   `handlers/quest/handlers.rs`, auto recompensa de tracking events en
   `handlers/quest/state.rs`, y el drenaje posterior de progreso de objetivos.
2. Inventariar participantes: objetivos/items a consumir, timed quest, recompensa
   fija/elegida/paquete, monedas, oro, XP/nivel, skill, títulos/talento, reputación,
   lockouts, estado activo/recompensado, spells, correo, game events y publicación.
3. Contrastar Classic: `QuestHandler.cpp:398`, `Player.cpp::RewardQuest:14625`,
   `SaveToDB(false):14867`, transacciones de `SaveToDB:19312` y sus grupos desde
   `19632`. El correo tiene una transacción aparte en `14794`.
4. Registrar divergencias existentes antes de cambiar comportamiento. Rust concede
   items secuencialmente; `save_quest_to_db` devuelve `()` tras fallos/desconocidos.
   El comentario del fallo de oro evita dejar retryable una misión tras concesiones:
   no retirar esa protección como si fuera una simple limpieza.
5. Comparar una transacción coherente de personaje con una operación durable por
   etapas. Elegir según participantes, recuperación, coste y fidelidad; no inventar
   una transacción global entre correo, cuenta y personaje.
6. Definir una tabla de estados: admitida, preparada, escritura enviada, commit
   conocido/rollback/desconocido, aplicación canónica, publicación y recuperación.
   Para cada estado: dueño, cancelación, reintento, disconnect/transfer y relogin.
7. Revisar el diseño concreto cuando haya una decisión material de comportamiento;
   conservar las partes no conflictivas ya autorizadas y no pedir permiso por helpers.

**Limitaciones que ya están contrastadas:**

- `PlayerCharacterSaveRequestLikeCpp` no incluye misiones, inventario ni monedas.
  Llamar al full save actual no crea atomicidad para la recompensa.
- `record_represented_quest_reward_mail_like_cpp` solo registra evidencia en tests;
  en producción es un no-op. No presentarlo como correo implementado tras moverlo.
- Classic publica la recompensa y ejecuta otros efectos antes de su guardado final.
  Imponer siempre commit antes de publicación sería un cambio intencional.
- El port completo de lo no representado sigue abierto aunque se cierre la frontera
  arquitectónica de las partes existentes. Identificar participantes no implementados.

**Implementación:** invariantes en Player/subdominios; coordinación privada de la
operación; puertos de persistencia concretos; adaptador SQL; handler/presentación.
Una proyección no constituye un segundo propietario. Migrar también la entrada
automática, los lectores/reintentos y consumidores de resultados; retirar el camino anterior.

**Pruebas y cierre:** éxito normal, recompensa elegida/inválida, inventario lleno,
repetible/no repetible, lockouts, fallo después de la primera concesión, rollback,
COMMIT desconocido de item/estado/oro, cancelación, residencia obsoleta, restart/relogin
y reintento sin duplicación. Conservar orden y destinatarios de publicación. Las
afirmaciones de durabilidad exigen evidencia real DB/reinicio, con autorización runtime.

### P2 — Completar las otras fronteras del núcleo por operaciones

No fijar una cola por números de issue ni extraer campos sueltos. Después de P1,
elegir la siguiente macro por sus consumidores y dependencias reales:

| Familia | Trazado mínimo | Evidencia de cierre |
| --- | --- | --- |
| Inventario/vendor/loot | planificación, reservas, persistencia, aplicación, publicación y reintentos de la operación completa | Un propietario de items/oro; ninguna concesión perdida/duplicada por fallo parcial; retirada de mutadores generales afectados. |
| Progresión/spells/effects | callers directos e indirectos, Player/Unit, lanzamiento/cancelación y efectos dependientes | Reutilización de acquisition/cast entregados; invariantes fuera de Session; metadata y orden preservados. |
| Social y grupos | owner de grupo, registro, sesión, lifecycle y fanout | Sin dobles escritores ni datos derivados confundidos con autoridad; backpressure y desconexión tratados. |
| Cuenta, carga y guardado | personaje/cuenta, ownership durante carga/transfer, fencing, acknowledge y estado desconocido | No perder revisiones nuevas ni reanudar una sesión cuyo commit siga incierto; consumidores de login/logout migrados. |
| Otras reglas representadas | cada uso de acceso mutable amplio, setters y closures sobre gameplay state | Lista completa de lectores/escritores migrados y eliminación o excepción acotada del acceso anterior. |

No dar por retirado `gameplay_state_mut` cambiándole de archivo. Reducir sus usos al
cerrar cada familia y reemplazarlos por transiciones de dominio con inputs/resultados
concretos. Mantener los accesos de carga/hidratación explícitos y separados de reglas activas.

### P3 — Fases de ejecución, lifetime y almacenamiento real

- Releer composición: `world-server/src/app.rs`, `runtime/*`, fábrica de sesiones y
  `wow-world/src/session/driver`. Contrastar `wow-map` y runtime legacy con `Map.cpp`.
- Dibujar quién ejecuta cada fase, dónde se admite al actor, qué reloj usa, dónde se
  obtiene/consume el estado y cuándo se entregan mensajes. Contar tareas reales, no enums.
- Resolver cada fase partida entre Session, mapa canónico y legacy. No declarar un
  doble tick sin reproducción; existe protección `GlobalLegacy` en parte del melee.
- Conservar una residencia/incarnation válida durante entrada, salida, transfer,
  detach, unload y shutdown. Un Player detached puede seguir teniendo dueño válido.
- Integrar hecs selectivo solo en almacenamiento privado con propietario real y las
  pruebas de conformidad exigidas. La dependencia ECS o el laboratorio no prueban su
  integración en producción. Mantener las garantías aprobadas antes de migrar storage.
- Retirar cada bridge legacy solo cuando hayan migrado sus lectores, escritores,
  persistencia y publicación. Suprimir entonces el registro exacto de la baseline.

**Cierre:** traza de fases sobre la composición ejecutada, una ejecución por transición,
pruebas de demora de I/O, backpressure, varias sesiones, reentrada, cancelación,
transfer/unload/cierre. Ningún guard síncrono de entidad/mapa cruza `await`, I/O o
entrega de paquetes. Los gates async existentes conservan su contrato de orden y recovery.

P2/P3 pueden intercambiar una entrega cuando una operación necesita antes una fase
de mapa o un lifetime. Esa dependencia debe quedar escrita; no habilitar producción
antes del requisito ni paralelizar campañas pesadas en distintos worktrees.

### P4 — Organización física, nombres y excepciones

Esta tarea acompaña a P1–P3 desde el comienzo. Completar además los propietarios de
datos, protocolo, composición y tooling que no entren en esas operaciones.

| Superficie | Tratamiento esperado |
| --- | --- |
| `session/mod.rs`, `map/mod.rs`, `player/mod.rs`, `world-server/app.rs` | Raíces que expliquen composición y API; reglas, fases y adaptadores en submódulos cohesivos. No nuevos contextos universales. |
| `session_tests.rs` y fixtures de quest/loot | Fixtures por familia y escenarios nombrados; conservar alcance cfg, montaje y conjunto de pruebas. Evitar otro test_support gigante. |
| `spell/stores/state_1..3.rs` | Reunir tipos y operaciones de ranks, area rules, pet auras, prerequisites, groups, procs, target positions, etc. Un mero cambio de nombre conserva la mezcla. |
| `state_4_ops_1/2.rs` y similares | Separar hidratación, dificultad, consultas del catálogo y reglas según su propietario; migrar consumidores y APIs. |
| `main_tests/scenarios_1..14.rs` | Escenarios de GUID, realm, cierre, respawn y runtime cerca de sus responsabilidades; mantener pruebas de composición real. |
| `misc_generated.rs` y datos DB2 | Dividir lectores por dominio y declarar procedencia/generador reproducible cuando exista. El nombre generated no concede excepción. |
| `player/lifecycle_adapter.rs`, `statement_def.rs` | Un trait impl/match puede delegar en módulos privados por operación. Mantener un coordinador y orden de transacción. |
| `statements/character/identities.rs` | Revisar cohesión del enum y alternativa de generación; una excepción individual puede ser correcta con procedencia y condición de salida. |
| `wow-world/map_manager` | Hacer explícito que es legacy cuando se migren consumidores e inventario; alias de transición solo con obligación de retirada. |
| `wow-script` / `wow-scripts` | Diferenciar dispatcher/runtime y contenido; evaluar nombres descriptivos al cerrar la responsabilidad, sin fusionarlos por parecido. |
| QA bot y herramientas de arquitectura | Entrypoint único y pequeño; runner, cliente, escenarios y propietarios de política separados; imports explícitos entre responsabilidades. |
| Fuentes externas/vendor | Excepción por fichero con procedencia, versión y motivo, o división mantenible; no una exención general por directorio o extensión. |

Conservar `snake_case`, `mod.rs` y nombres de crates con guiones: son convenciones
válidas. No hacer una campaña global de borrar `LikeCpp`/`_like_cpp`; no demuestra
paridad, pero comunica procedencia y tiene muchos consumidores. En APIs nuevas usar
nombres semánticos; revisar el ruido al migrar una responsabilidad completa.

Preferir imports explícitos y fachadas deliberadas en producción. Los globs de una
fixture pequeña pueden ser razonables. No imponer una cuota cero ni ampliar la
visibilidad de estado para que compile una separación. Los nombres numéricos solo
son finales cuando el número expresa dominio, versión o protocolo.

**Cierre:** cada archivo grande tiene una partición coherente o una excepción
individual conforme a la política. `physical-files --terminal` pasa; revisar también
cohesión por encima del umbral de revisión. Actualizar solo los techos afectados con
medición y explicación; mantener la deuda lógica hasta retirar realmente la responsabilidad.

### P5 — Extensión nativa/Wasm pendiente (#583)

Comienza después de los requisitos de núcleo #584, no al cerrar #716 ni por estar
#578 cerrado. Usar el contrato vigente de `modularity-and-ecs-plan.md` y #583; no
reinventar otro framework a partir de un ejemplo de login.

- Hooks y resultados compartidos por ejecución nativa y Wasm, antes/después donde
  corresponda al contrato y al orden de gameplay.
- Estado independiente de módulo, lifetime, persistencia/progreso/recompensa y
  operación de instalar, actualizar, desactivar y recuperar.
- Capacidades acotadas: sin SQL, Player mutable, guards o writers expuestos al módulo.
- Módulo útil independiente que pueda actuar sin parchear el núcleo; comportamiento
  base correcto con cero módulos; pruebas nativo-only y Wasm-enabled.
- Aceptación del segundo lenguaje/contrato acordado, límites del executor y errores.
  No prometer ABI nativa estable, hot reload o todos los lenguajes sin contrato aprobado.

### P6 — Aceptación terminal (#153 y cierre #133)

Orden global: **núcleo requerido #584 → #583 → #153 → cierre #133**.
#133/#584 son umbrellas, no implementaciones previas que bloqueen por sí mismas.

Auditar el código final y la evidencia aplicable: propietarios, dependencias, acceso
mutable, clocks/fases, guards, persistencia, bridges, registros/opcodes, organización
física, SDK y operación real. #153 verifica; no absorbe implementación conocida que
se haya dejado pendiente. Distinguir implementado, integrado y probado frente a C++.

Después de M6.2/#47 se mantiene el nuevo análisis de todo el port antes de descomponer
Part 2/#48. Este plan no crea anticipadamente ese árbol de issues.

## 6. Cómo ejecutar cada macro sin volver a fragmentar el problema

1. Reproducir/contrastar el problema y enumerar todos los consumidores.
2. Fijar operación, propietario, dependencias, fases, persistencia y publicación;
   separar movimientos, cambios de frontera y reparaciones de comportamiento.
3. Reutilizar la macro/branch activa; abrir la siguiente macro coherente cuando el
   contrato y las dependencias estén definidos, no una issue por fichero o helper.
4. Implementar la entrega completa con tests y consumidores. Delegar una tarea
   independiente con archivos propios cuando aporte valor; el padre integra y
   programa la validación. No ejecutar CI por cada cambio interno.
5. Validar el resultado completo, corregir fallos y volver a ejecutar lo afectado.
6. Registrar SHA/comandos/resultados/límites en el documento propietario y hacer
   commits locales coherentes. Publicación y runtime conservan sus autorizaciones.

## 7. Validación concreta

Elegir los targets reales según el cambio. Un cambio exclusivo en herramientas no
requiere atribuirse pruebas de gameplay. Un refactor de runtime no se acepta solo
con mocks ni con una suite de librería que no compruebe la composición en producción.

```bash
# Herramientas/ownership; secuencial, una vez completa la entrega.
CARGO_BUILD_JOBS=1 cargo test --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml --lib
CARGO_BUILD_JOBS=1 cargo run --release --locked \
  --manifest-path tools/architecture/handler-contract-check/Cargo.toml \
  --bin session-ownership-check -- check --syntax-only
python3 tools/architecture/check_architecture.py check
python3 tools/architecture/check_architecture.py self-test
python3 tools/architecture/check_architecture.py physical-files --terminal

# Para cambios de servidor, elegir las pruebas de la operación y sus integraciones.
CARGO_BUILD_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo check -p world-server
CARGO_BUILD_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc \
  cargo test -p wow-world <prueba-de-la-operacion> --lib
cargo fmt --all -- --check
git diff --check
VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2 quick --base origin/3.4.3

# Antes de un push autorizado, sobre el candidato comprometido.
VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2 final --base origin/3.4.3
```

No ejecutar todo este bloque a ciegas por cada helper. El modo físico terminal
seguirá fallando hasta cerrar P4; conservar ese resultado como deuda, no ocultarlo
subiendo techos. El ownership sin `--syntax-only` recompone el inventario exhaustivo
de persistencia: usarlo en los cambios/aceptaciones que lo requieran. No regenerar
baselines para tapar drift. Preservar las nueve columnas del ledger R8 si se modifica.

Para operaciones con bytes/metadata/conexión/orden nuevos, ejecutar capture-diff y
la captura específica pertinente. Para durabilidad o lifecycle, leer las guías del
QA bot y obtener la autorización runtime necesaria antes de DB/reinicio/relogin.
Registrar host, SHA y límite real de cada evidencia. No relabelar un test de un SHA
como si hubiese probado uno posterior; los deltas documentales pueden reutilizar
evidencia verde con su consistencia comprobada según validation-v2.

## 8. Decisiones que siguen abiertas

- ~~Protocolo durable exacto de recompensa~~: resuelto en #718 a favor de la
  transacción coherente de personaje. El tratamiento de los participantes no
  implementados sigue abierto y está inventariado en el contrato de la operación.
- Siguiente familia P2 y sus requisitos de fases P3, después de contrastar consumidores.
- Ubicación final de reglas hoy repartidas entre los dos componentes de Player.
- Excepciones físicas individuales, especialmente enums/tablas cohesivas y vendor.
- Renombrado público de crates de scripting y garantías de compatibilidad necesarias.

Resolver con evidencia y alternativas concretas. No elegir por inercia de la
estructura actual ni crear aprobaciones rutinarias para imports, helpers o tests.

## 9. Estado exacto de la entrega al pasar a Claude

**P0 tiene aceptación local.** La implementación está en
`6ae62d73a9f910ebd664d82e331520b836a73c77`, en la rama y worktree de la sección 2.
El commit posterior de documentación completa este relevo. Los cambios de código
afectan exclusivamente al tooling de arquitectura; no se modificó gameplay, SQL,
protocolo, scheduling ni procesos del servidor.

Evidencia ejecutada en aarch64, Rust 1.98.0 y un job de Cargo:

| Comprobación | Resultado |
| --- | --- |
| `cargo test --release --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml --lib -- --quiet` | 363 passed, 0 failed; incluye contrato real de handlers, grafo completo, referencias preservadas y consistencia snapshot/policy. |
| `cargo run --release --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml --bin session-ownership-check -- check --syntax-only` | PASS; 221 campos de producción, 427 de fixtures, 594 accesos directos de registro y baseline exacta de 72 bridges. |
| `python3 tools/architecture/check_architecture.py check` | PASS; 38 paquetes, 101 aristas internas; migración física y ocho propietarios lógicos dentro de sus techos. |
| `python3 tools/architecture/check_architecture.py self-test` | PASS; fixtures adversariales de política y 20 tests de archivos físicos. |
| `./tools/validation-v2 quick --base origin/3.4.3` | PASS; seis comandos, incluidos formato de la herramienta, cargo check, JSON, higiene y tests físicos; manifiesto verificado con `validation-v2 verify`. |
| `git diff --check` | PASS. |
| `python3 tools/architecture/check_architecture.py physical-files --terminal` | FAIL esperado y pendiente: los mismos 31 archivos de la sección 10. No es aceptación terminal del refactor. |

Las ejecuciones usaron `CARGO_BUILD_JOBS=1 CARGO_NET_OFFLINE=true` cuando
correspondía; V2 usó `VALIDATION_V2_CARGO_JOBS=1` y el PROTOC de la sección 2.
Se ejecutaron sobre la versión de trabajo cuyo padre era `aff42a51`, antes del
commit. El manifiesto `/tmp/rustycore-716-quick-manifest.json` registra ese SHA y
`dirty: true`; no se presenta como una ejecución posterior sobre `6ae62d73`.
El código y las políticas probadas se incorporaron sin cambios a ese commit.
Los ajustes posteriores de entrega son documentales.

El inventario conserva exactamente los 65 bridges anteriores tras el cambio de
ubicación revisado y añade siete accesos existentes contrastados en código; no
elimina ninguna obligación. Las dos ubicaciones de fixtures también se reconciliaron.
No se regeneró el inventario exhaustivo de persistencia. No se ejecutaron build del
servidor, capturas, QA live, reinicio, escrituras de DB, push, merge ni despliegue.
El perfil de publicación `final` sigue siendo obligatorio antes de un push autorizado.

**P0 publicado.** El perfil `final` se ejecutó sobre el candidato comprometido
`433a3a33` con árbol limpio y manifiesto verificado en verde; la rama está
empujada y el PR es [#717](https://github.com/alseif0x/rustycore/pull/717).
Su integración sigue pendiente de revisión.

**P1 aceptada localmente en [#718](https://github.com/alseif0x/rustycore/issues/718).**
El contrato completo de la operación quedó contrastado contra
`Player::RewardQuest` (Player.cpp:14625) y su `SaveToDB(false)` de cierre
(Player.cpp:14867), y está documentado en
[quest-reward-operation-contract.md](quest-reward-operation-contract.md).
La decisión abierta de la sección 8 sobre el protocolo durable quedó resuelta
por el usuario a favor de la transacción coherente de personaje. No repetir P0
salvo que cambie su código o aparezca una regresión. P2–P6 siguen pendientes y
#584 continúa abierto.

**P0 y P1 integradas.** #717 cierra #716 y #719 cierra #718; ambos se
integraron en `3.4.3` con perfil `final` verificado en verde.

**P2 en curso: los límites concretos del Player están retirados.**
[#722](https://github.com/alseif0x/rustycore/issues/722) recorrió el acceso
mutable amplio a `gameplay_state_mut` por familias, cada una con su ancla C++
exacta, su medición registrada en `runtime-ownership-ledger.json` y su propio
`final` verificado: #723, #724, #725, #726, #727, #728, #729, #730, #731, #732,
#733, #734 y la familia del asiento de vehículo que cierra la clase concreta.
Ninguna añadió campo, espejo, cerradura ni superficie pública más allá de la
transición nombrada.

El residuo de producción son 15 accesos de una sola forma: un mutador de sesión
que entrega una submatriz `&mut` al cierre de su llamador. Envolverlos movería
el acceso amplio en lugar de retirarlo, lo que este plan prohíbe; cada uno se
retira cuando se modele su propia operación. Quedan registrados en
`known_gaps` como `broad_player_state_access_residual_is_generic_closures`, con
sus rutas y líneas. `progression/reputation.rs:169` es un caso aparte: C++ tiene
`Player::m_reputationMgr` en el Player, pero `ReputationMgrLikeCpp` vive en
`wow-world`, así que retirarlo exige mover el manager a `wow-entities`; se
sigue en [#735](https://github.com/alseif0x/rustycore/issues/735), no como
diferimiento silencioso. Otros 17 accesos en
archivos que no son de test están dentro de `#[cfg(test)]` y no son superficie
de producción.

**P2: la fila de inventario/vendor/loot está cerrada.**
[#737](https://github.com/alseif0x/rustycore/issues/737) hizo lo mismo con el
runtime de items del Player, por familias y con `final` verificado en cada una:
#738 (mapa de slots), #739 (slots de recompra), #740 (almacén de objetos) y
#741 (precio y timestamp de recompra). El oro no necesitaba trabajo:
`Player::SetMoney` y `ModifyMoney` ya lo poseen en
`crates/wow-entities/src/player/progression.rs`, y los sitios de dinero del lado
de la sesión son rutas de staging y persistencia, no una segunda autoridad. Eso
queda registrado para no volver a investigarlo.

La medición inicial de #737 contenía un falso positivo de grep: los 18 usos
atribuidos a `inventory_mut()` eran en realidad llamadas a
`persist_inventory_mutation_like_cpp`. Corregido en un comentario del issue, no
reduciendo el alcance en silencio. El residuo es un solo cierre genérico en
`session/player_items/storage.rs:248`, registrado en `known_gaps` como
`item_runtime_residual_is_one_closure_and_test_fixtures`.

La mitad de operación de esa fila -- ninguna concesión perdida o duplicada por
fallo parcial -- se volvió a comprobar en lugar de asumirse: una sola función de
producción mantiene más de una escritura durable de inventario, y sus dos
escrituras son las ramas exclusivas verificadas en #720. Ninguna familia añadió
un participante de persistencia.

**Estado medido de las tres filas restantes de P2** (2026-09-11, contrastado en
código, no asumido desde la tabla de la sección P2):

*Social y grupos.* La propiedad ya es correcta: el grupo de `wow-social` posee la
pertenencia con una API nombrada equivalente a `Group` de C++, y la instantánea
del lado del Player no es dato derivado confundido con autoridad, porque
`Player::GetGroup()` (Player.h:2547) también lee el grupo de la referencia
`m_group` del propio Player. Lo que sí apareció es un defecto de garantía de
entrega: el borrado de estado del miembro afectado viaja por
`try_send_current_command`, un `try_send` sobre un canal `bounded(256)`, y su
resultado se descarta; nada reconcilia después. Registrado en
EXISTING-CODE-DEFECTS.md y acotado en
[#743](https://github.com/alseif0x/rustycore/issues/743), que además lleva el
resto de la fila. No se parcheó porque elegir entre envío bloqueante con orden
de cerraduras explícito, reintento, o estado reconciliable es una decisión de
diseño con su propio contrato.

*Progresión/spells/effects.* Entregada en lo esencial por #587 y documentada en
ownership-and-boundaries.md, que ya fija su condición de retirada: retirar el
seam especializado solo cuando métodos canónicos del `Player` expongan el mismo
contrato atómico de dry-run/apply. El planificador vive en
`wow_world::spell_acquisition`, la durabilidad en su puerto, y el Player tiene
`apply_prepared_spell_acquisition_like_cpp`. No inventar trabajo nuevo aquí sin
medición propia.

*Cuenta, carga y guardado.* Los dos criterios de cierre de la fila están
cubiertos y verificados en código: la revisión de guardado diferido usa
`checked_add` y el recibo limpia solo la intención confirmada que coincide
(`crates/wow-entities/src/player/deferred_save.rs`), y un COMMIT incierto no
reanuda la sesión: `session/lifecycle/persistence.rs:255-270` marca el dinero
como indeterminado, desarma la valla y expulsa con «relog required before
another money mutation».

Por tanto P2 queda a falta de: el contrato de entrega de #743, la decisión de
diseño de #735, y el residuo de cierres genéricos, que se retira operación por
operación. Falta también evidencia de durabilidad en vivo (escritura real en DB,
reinicio, relogin), que exige autoridad de runtime y no se ha ejecutado.

P3–P6 siguen pendientes y #584 continúa abierto.

Si el worktree temporal ya no existe, localizar la rama local anterior con
`git worktree list` y `git branch --list '*716*'`; el commit conserva el plan y la
implementación. Recuperar un worktree desde esa rama sin alterar el checkout sucio.

## 10. Inventario físico pendiente para P4

Estos 31 archivos siguen rechazados por el control físico terminal en la entrega
P0, igual que en la base revisada. Conteo bruto de líneas físicas; no confundirlo
con las métricas lógicas de producción/tests. No convertir esta tabla en 31 issues:
agrupar por la responsabilidad y los consumidores de cada entrega. Para vendor o
fixtures con runtime, evaluar la excepción o evidencia específica descrita en P4.

| Archivo | Líneas físicas |
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
