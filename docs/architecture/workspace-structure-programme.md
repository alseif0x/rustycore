# Programa maestro de estructura del workspace

**Para qué sirve:** es el **índice y el orden de ejecución** de todo el trabajo estructural del
workspace. No duplica detalle: fija qué va antes de qué, qué evidencia cierra cada fase y en qué
estado está cada una, para que cualquier agente pueda retomar sin perderse.

**Para retomar:** leer primero el [estado de continuación](#31-estado-de-continuacion-2026-09-28).
Las notas fechadas posteriores conservan el historial de intentos y resultados; no acreditan por
sí solas aceptación de la punta actual.

**Autoridades (no compiten entre sí):**

| documento | gobierna |
|---|---|
| [structure-and-conventions.md](structure-and-conventions.md) | el **estándar**: capas, nombres, visibilidad, tests, presupuestos, checklist |
| [wow-world-distribution-plan.md](wow-world-distribution-plan.md) | el detalle de **wow-world** (forma objetivo y fases F0-F13) |
| este documento | el **orden, dependencias y estado** de todo el workspace |
| [refactor-completion-plan.md](refactor-completion-plan.md) | el plan técnico general del port (no lo sustituimos) |

**Regla de los dos planos:** la lógica se contrasta con la referencia C++ 3.4.3; la estructura la
decide este programa. Ninguna decisión estructural se justifica con "en C++ es así".

## 1. Estado de partida (auditoría medida, `3.4.3` + rama de distribución)

Reejecutable con el script de auditoría (mide líneas, ficheros, fichero mayor, deps internas,
consumidores, capa y violaciones) sobre `crates/`.

**Monolitos:** `wow-world` 410 725 (mayor 1 720) · `wow-data` 83 539 (1 999) · `wow-entities`
79 731 (**4 726**) · `world-server` 60 368 (**5 675**) · `wow-map` 54 927 (**5 133**) ·
`wow-packet` 50 339 (1 682) · `wow-database` 45 556 (1 793) · `wow-social` 5 354 (2 508).

**Muertos/stubs/mal declarados:** `wow-spell` (1 línea) · `wow-pvp` (1) · `wow-achievement` (1) ·
`wow-scripts` (44, depende de `wow-script`) · `world-modules` (23, **depende de `world-server`**) ·
`rustycore-db` (295, sin consumidor) · `wow-collections` (693, sin consumidor y **colisiona de
nombre** con el dominio de colecciones de cuenta) · `wow-session` (596, capa ambigua) ·
`capture-diff` (herramienta de QA dentro de las capas de juego) · `wow-recastdetour` (vendor sin
señalizar) · `wow-chat` (solapa con `wow-social`).

**Inversiones de capa:** `data→entities`, `data→movement`, `entities→loot`, `map→loot`,
`packet→loot`, `packet→movement`, `database→persistence`, `world-modules→world-server`, y las
laterales `ai→instances`, `conditions→loot`, `scripts→script`.

## 2. Secuencia maestra

Orden por **dependencias reales**: primero lo que quita ruido, después lo que quita tamaño, después
lo que cambia contratos, y al final la aceptación. Nada de una fase empieza si su predecesora no
está verde (ver §4).

### Ola A0 — endurecer el estándar antes de tocar crates
Contraste con proyectos grandes de Rust (`rustc`, `rust-analyzer`, `bevy`, `polars`, `tokio`,
`datafusion`): el plan va en la dirección correcta, pero conviene añadir tres piezas antes de
mover crates, porque son más baratas ahora que después.

| id | objetivo | evidencia de cierre | depende de |
|---|---|---|---|
| A0.1 | **Decidir el mapa de *features***: qué crates/subsistemas son opcionales (`world-modules`, scripting, anticheat) y garantizar que el build base no los requiere. Cargo unifica features por crate en todo el workspace, así que la decisión afecta a caché y a compilación | el build base compila sin features opcionales; decisión escrita en el estándar | — |
| A0.2 | **`xtask` del workspace** que aloja los comandos de estructura: auditoría de crates, chequeo de aristas de capa, regeneración revisada de políticas | `cargo xtask structure-audit` y `cargo xtask check-layers` en verde | — |
| A0.3 | **`[workspace.lints]`** (clippy/rustc) y política por crate (`unsafe`, `missing_docs` en API pública); los warnings dejan de ser decorativos | lints activos; ningún crate afectado gana warnings nuevos | — |
| A0.4 | **Higiene de grafo con herramientas estándar**: `cargo-machete` (dependencias no usadas), `cargo-deny` (licencias/avisos/versiones duplicadas), `cargo tree -d` | informe inicial registrado y deuda adjudicada a A1/C/D | — |
| A0.5 | **ADRs** de la decisión estructural (alternativas y consecuencias) y **marcar el código vendido** (navmesh) como exento de presupuestos y lints | ADR enlazado desde el estándar; crates vendor señalizados | — |
| A0.6 | **Presupuesto de documentación**: el estándar se mantiene corto (es regla); el histórico va a ADRs, no a un plan que crece | techo de tamaño para documentos de arquitectura | — |
| A0.7 | **Herramientas opcionales a decidir**: `cargo-nextest` (ejecución de los ~3 900 tests) y `cargo-public-api` (snapshot de la superficie pública de `wow-entities`, `wow-map`, `wow-packet`); `cargo-hakari` **solo** si la medición de build demuestra duplicación de features | decisión escrita; si se adopta, comando en el `xtask` | A0.2 |

**Lo que NO copiamos** (y por qué): el troceo a escala `bevy`/`zed` (cientos de crates) porque
aquí no hay ecosistema de plugins — 12-14 dominios más directorios es la medida; el
feature-gating de todo; y `cargo-hakari`/workspace-hack antes de medir, porque añaden complejidad
que hay que justificar con números.

### Ola A — saneamiento barato (no cambia contratos ni comportamiento)
| id | objetivo | evidencia de cierre | depende de |
|---|---|---|---|
| A1 | plegar `rustycore-db` en `wow-database`; retirar `wow-pvp`, `wow-achievement`, `wow-scripts` (o consolidar en `wow-script`); renombrar la utilidad `wow-collections`; decidir `wow-session` y `wow-chat` | workspace compila; recuento de crates baja; `inventory::submit!` y nº de tests invariantes | A0 |
| A2 | invertir `world-modules` para que no dependa de `world-server` | grafo sin esa arista | A1 |
| A3 | marcar tooling: `capture-diff` fuera de las capas de juego; renombrar el vendor de navmesh | grafo y capas coherentes con el estándar | A1 |

### Ola B — `wow-world` (detalle en su plan)
| id | objetivo | depende de |
|---|---|---|
| B1 | F5 ✅ `misc` desmontado en 13 dominios (commit `e719ac38`) | — |
| B2 | F6 dominios cohesivos a su crate (`reputation`, `planner`, `profession`/`trainer`, `entity_update_bridge`, `battle_pet_*`) | B1 |
| B3 | F10a: suites de aplicación a targets de integración de `wow-world`, con fixtures mínimas; pruebas de reglas en su dominio (reformulación del 2026-09-25) | B2 |
| B4 | F7 sub-estados dueños dentro de `WorldSession` (**diseño**) | B2 |
| B5 | F8 handlers grandes → adaptadores ≤600 líneas (**diseño**) | B4 |
| B6 | F9 `map_manager` + `map_manager_tests` → `wow-map` con contrato (**diseño**) | B4 |
| B7 | F10b resto de tests; F13 warnings y política de ownership con delta revisado; consumidores de fixtures pendientes de B3 | B3 (residual), B5, B6 |

### Ola C — inversiones de capa (cada una es un movimiento pequeño)
| id | objetivo | depende de |
|---|---|---|
| C1 | tipos de registro de `wow-data` bajan a `wow-data`; se invierten `data→entities` y `data→movement` | A1 |
| C2 | los tipos que `entities`/`map`/`packet` necesitan de loot suben a su dueño; se rompen `entities→loot`, `map→loot`, `packet→loot` | C1 |
| C3 | `packet→movement` y `database→persistence` corregidas a su dirección natural | C2 |
| C4 | laterales de L3 (`ai→instances`, `conditions→loot`, `scripts→script`): fijar dirección por dominio y documentarla | C3 |

### Ola D — los siguientes monolitos (mismos presupuestos del estándar)
| id | objetivo | prioridad | depende de |
|---|---|---|---|
| D1 | **`wow-entities`**: fichero mayor 4 726 → ≤1 000; separar modelo canónico de estado de gameplay | alta | C2 |
| D2 | **`world-server`**: `app.rs` 5 675 → módulos por fase; composición delgada | alta | B7 |
| D3 | **`wow-map`**: ficheros 5 133 → ≤1 000; separar runtime de grillas de la fachada | media | B6 |
| D4 | **`wow-data`** y **`wow-database`**: techos de fichero y organización por catálogo/tabla | media | C1 |
| D5 | **`wow-packet`** y **`wow-social`**: techos de fichero; packet solo wire | media | C3 |
| D6 | **Ecosistema de módulos (ADR-002)**: contrato de datos versionado en `wow-module-api`, adaptador nativo in-process y adaptador Wasm (host) sobre el mismo contrato, `wasm-runtime` como feature opcional apagada por defecto, con versión de contrato, lista blanca de host functions, límite de ejecución (fuel) y aislamiento de fallos | media | A0.1 |

### Ola E — cierre
| id | objetivo | depende de |
|---|---|---|
| E1 | auditoría del estándar en todo el workspace (mismo script): 0 stubs, 0 sin consumidor, 0 inversiones, 0 fichero > 1 000 | A-D |
| E2 | ledger y política físicas consistentes con el árbol (delta revisado) | E1 |
| E3 | **validación final única** (`./tools/validation-v2 final --architecture`) y evidencia publicada | E2 |
| E4 | medición de build y latencia de herramientas antes/después (#1231); se publica tal cual, incluso si es negativa | E1 |

## 3. Registro de estado

Se actualiza **en el mismo commit** que cierra cada fase. Convención: `[ ]` pendiente, `[~]` en
curso, `[x]` cerrada con commit.

```
A0.1 [x]  A0.2 [x]  A0.3 [x]  A0.4 [~]  A0.5 [x]  A0.6 [x]  A0.7 [x]   <- ola A: PUERTA VERDE
A1 [x]  A2 [x]  A3 [x]
B1 [x] e719ac38   B2 [x] 98c5b14a   B3 [~]  B4 [~]  B5 [~]  B6 [~]  B7 [~]
C1 [ ]  C2 [ ]  C3 [ ]  C4 [ ]
D1 [ ]  D2 [ ]  D3 [ ]  D4 [ ]  D5 [ ]
E1 [ ]  E2 [ ]  E3 [ ]  E4 [ ]
```

### 3.1 Estado de continuacion (2026-09-28)

Esta entrada se mantiene al avanzar; sustituye las conclusiones contradictorias de las notas
históricas para seleccionar el siguiente trabajo. Los SHA antiguos se conservan como evidencia
histórica: la reescritura de mensajes cambió los identificadores, no autoriza a relabelar pruebas.

**Punto de partida comprobado:** worktree `/home/server/rustycore-world-refactor`, rama
`584-map-manager-domain`, punta `bfeb33e829334b4172dbddd73ec0d53a25f0e104`; base
`3.4.3` en `e786ece1a554724351307bfd4e05e63a89962b60`. Local y remoto coincidían:
15 commits por delante, 64 archivos, +2130/-1692, sin PR abierta. Dos worktrees registrados,
sin entradas eliminables ni stashes. Reconsultar Git al retomar; estos son datos fechados.

| pieza | estado comprobado y límite |
|---|---|
| B6 | `map_manager` y su suite están en `wow-map`; comparación estática conserva 136 anotaciones de test. Los cortes del 2026-09-29 restringen constantes de terrain y helpers internos de grid; quedan otras visibilidades/features y la aceptación del candidato. |
| B5 | Raíces de loot y character partidas; quest sigue dentro del presupuesto. Pendiente restringir la API expuesta por la extracción y aceptar el incremento. |
| B3 | Superficie inicial `test-fixtures` y self dev-dependency añadidas. El piloto falló y fue revertido: las cuatro suites siguen dentro de la lib. |
| B7 | Hay limpieza parcial integrada; los consumidores restantes de fixtures dependen de B3. No está cerrado. |
| Talentos #578 | La rama `recover/578-talent-catalog`, `0845f5b3`, conserva `docs/migration/recovered/578-talent-catalog-2026-09-04.patch`; el archivo solo existe en esa rama, no en este checkout. No está aplicado; preservar la rama y adaptar por consumidores actuales cuando corresponda. |
| Orquestación | Sol medium / Luna max copiado a este worktree. Cambios compartidos aún locales, y `.codex/config.toml` ignorado. Iniciar sesión nueva y comprobar runtime efectivo; preservar estos cambios al continuar. |

**Avance local posterior, pendiente de aceptación:** commits locales `8f162870`,
`9df13460` y `dc73db43`. La feature de fixtures de `wow-map`
ya no se activa en la dependencia de producción de `wow-world`, ni la de
`wow-recastdetour` en la dependencia de producción de `wow-map`; las features
explícitas y las dependencias de desarrollo conservan el acceso de tests. El worker
obtuvo `cargo check -p wow-map`, `cargo check -p wow-world` y un test focalizado de
pathfinder con salida 0. En loot se restringieron los exports de los seis hijos
extraídos al módulo dueño, salvo la ruta `pub(crate)` preexistente de la distancia
de interacción; `cargo check -p wow-world --tests` y el filtro `loot` pasaron
(326 tests). En character se restringieron tres hijos y se conservó la ruta
`pub(crate)` de `ExtendedCostItemTurninChange`; `cargo check -p wow-world --tests`
pasó; los otros tres hijos de character también quedaron restringidos y el filtro
`character` ejecutó 381 tests (379 unitarios y 2 de integración) sin fallos. Se
conservó la ruta `pub(crate)` de `CreatureAddonCreateFieldsLikeCpp`.
`dc73db43` devuelve a visibilidad privada los imports de implementación de
`map_manager`; los checks de targets de test de `wow-map` y `wow-world` salieron
con código 0, sin ejecutar los tests. Siguen pendientes la revisión de sus
exports propios, la composición final y la aceptación de toda la macro.
Estos checks son feedback del worker, no campaña final ni evidencia de publicación.

**Piloto B3 localmente verde, B3 todavía abierto:** `a0df47b3` mueve
`group_tests` a `crates/wow-world/tests/` como target de integración: 130 tests
externos y el test de regla privada conservado en `handlers/group/state.rs`.
La comparación de nombres de funciones antes/después conserva los 172 del árbol
de la suite; no sustituye su ejecución. Los commits locales `f54a98e3` a
`746b2616` preparan wrappers estrechos bajo `test-fixtures`, incluido el
bombeo con el **constructor original** de catálogos de sesión. El primer
diagnóstico del target tuvo 124 errores; los siguientes aislaron campos,
invitaciones, registro de silencio y bombeo. No se usaron catálogos por
defecto para reemplazar los de la sesión. La feature conserva también las
rutas de fixture para grupo, loot y el envío bloqueante al despachador de
prueba; sin la feature, la ruta normal mantiene sus ramas de producción.
`cargo check -p wow-world` sin feature salió 0 y
`cargo test -p wow-world --test group_tests` ejecutó **130/130** en paralelo
(log `target/b3-group-dispatcher-parallel.log`); la ejecución en serie también
pasó 130/130 (`target/b3-group-uninvite-serialized-final.log`). Son checks de
desarrollo del worker en aarch64 sobre el árbol que se comprometió como
`a0df47b3`, no manifiesto final ni aceptación de la macro. `character_tests`
y `loot_tests` aún viven en la lib; el movimiento diagnóstico de `quest_tests`
se detalla abajo. B7 y los consumidores pendientes no están cerrados.

**Hallazgos abiertos antes de aceptar B5/B6:**

1. La feature de fixtures de `wow-map` y `wow-recastdetour` estaba activada en dependencias
   normales. `9df13460` separa producción y pruebas; comprobar la composición final sin que
   la unificación de features oculte otra dependencia accidental.
2. Los `pub use self::<hijo>::*` y los `pub` añadidos en loot/character ampliaban la API.
   `8f162870` los restringe preservando las rutas `pub(crate)` originales. `dc73db43`
   restringe imports de implementación de `map_manager`; revisar todavía sus exports de
   submódulos y consumidores de toda la macro. El movimiento por sí solo no justifica
   visibilidad pública.
3. La sesión DeepSeek ocultó al menos un fallo de arquitectura con `grep | head; echo $?`;
   `845cc02d` corrigió aquella afirmación. La pauta del worker ya está corregida localmente
   para conservar salida completa y código real del comando; falta integrar esa configuración.
4. El baseline de `tools/xtask/layer-baseline.txt` contiene 11 aristas; el recuento estático
   con su regla actual encuentra 13 en esta rama, incluyendo `wow-packet -> wow-entities` y
   `wow-map -> wow-movement`. Reconciliar su contrato con `dependency-policy.json` y sus
   excepciones; no elevar el baseline ni retirar una arista legítima para obtener verde.

La revisión estática posterior confirma que ambas aristas de diferencia son
dependencias declaradas en los `Cargo.toml`, no falsos positivos del lector de
manifiestos. `wow-map -> wow-movement` une dos crates `domain-runtime`, categoría
que `dependency-policy.json` permite depender de sí misma. La arista
`wow-packet -> wow-entities` tiene una excepción explícita entre crates en esa
política, justificada por la proyección de CREATE de criatura. El checker de
`xtask` aplica en cambio `target >= source` a todos los pares de su misma capa
numérica y por eso detecta ambas. Antes de modificar la herramienta, contrastar
sus 11 filas históricas con la política completa: alinear la clasificación no
debe ocultar inversiones reales ni convertir esas dos aristas en deuda de C.

**B6 feedback acotado de terrain (2026-09-29):** la comparación de
`crates/wow-map/src/map_manager/terrain.rs` con
`origin/3.4.3:crates/wow-world/src/map_manager/terrain.rs` encontró ocho
constantes que el movimiento había ensanchado de `pub(super)` a `pub`:
`MAP_MAGIC_LIKE_CPP`, `MAP_AREA_MAGIC_LIKE_CPP`,
`MAP_VERSION_MAGIC_LIKE_CPP`, `MAP_FILE_HEADER_SIZE_LIKE_CPP`,
`MAP_AREA_HEADER_SIZE_LIKE_CPP`, `MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP`,
`MAP_AREA_CELLS_PER_GRID_LIKE_CPP` y `TERRAIN_GRID_COUNT_LIKE_CPP`. Sus usos
externos al archivo son la implementación de `terrain` y los tests montados bajo
`map_manager`; no hay consumidores de producción en otros crates. Se limitaron
a `pub(crate)` y se añadieron reexports crate-only para los tests en
`map_manager/mod.rs` y `map_manager_tests/fixtures.rs`. Los valores y el parser
no cambiaron. En ese corte, `grid.rs` y sus funciones ensanchadas quedaron fuera;
se auditan en el siguiente corte anotado aquí.

Los checks secuenciales con `CARGO_BUILD_JOBS=1`, `PROTOC` local y target de este
worktree pasaron: `cargo check -p wow-map`,
`cargo test -p wow-map --lib terrain_` (18/18) y
`cargo check -p wow-world`; logs
`target/b6-map-terrain-constants-check.log`,
`target/b6-map-terrain-constants-focused-test.log` y
`target/b6-map-terrain-consumer-check.log`. Se ejecutaron en el worktree sobre
`221b6e36` con el diff B6 de tres archivos aplicado, luego comprometido sin
cambios como `8622c89c`. Es feedback local, no aceptación de B6: siguen
pendientes el resto de visibilidades de terrain/grid, la revisión de features,
la composición y la aceptación final.

**B6 feedback acotado de grid (2026-09-29):** al comparar
`crates/wow-map/src/map_manager/grid.rs` con
`origin/3.4.3:crates/wow-world/src/map_manager/grid.rs`, se confirmó que las
visibilidades ensanchadas partían de `pub(super)`. Las cinco constantes
`MAX_NUMBER_OF_CELLS_LIKE_CPP`, `TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP`,
`SIZE_OF_GRID_CELL_LIKE_CPP`, `CENTER_GRID_CELL_ID_LIKE_CPP` y
`CENTER_GRID_CELL_OFFSET_LIKE_CPP`, además de `compute_cell_coord_like_cpp`,
solo se usan dentro de `grid.rs`; ahora son privadas. `CellCoordLikeCpp` y sus
campos siguen `pub(super)` porque `runtime/manager.rs` consume el resultado y
ordena por `x/y`. `calculate_cell_area_like_cpp` y
`cell_area_contains_position_like_cpp` son usados por ese runtime; el conversor
`position_to_i32_tuple` lo usa `movement/spline.rs`. Estos tres helpers
permanecen `pub(super)`. Sus imports explícitos en `map_manager/mod.rs` ahora
son privados; la API pública de `GridCoord`, `Grid` y las conversiones de
coordenadas no cambió. En particular, `wow_map::calculate_cell_area_like_cpp`
continúa siendo la función distinta de `crate::cell`, usada por producción.
Ningún test de `map_manager_tests` referencia directamente los helpers
restringidos.

Los checks secuenciales con `CARGO_BUILD_JOBS=1`, `PROTOC` local y target del
worktree pasaron: `cargo check -p wow-map`, `cargo check -p wow-world` y
`cargo test -p wow-map --lib grid` (140/140). Logs
`target/b6-map-grid-visibility-check.log`,
`target/b6-map-grid-world-check.log` y
`target/b6-map-grid-focused-test.log`. Corrieron sobre `c7d269be` con el diff
grid aplicado, luego comprometido sin cambios como `e714fe79`. Es feedback
local, no aceptación de B6: siguen pendientes las visibilidades ensanchadas de
funciones en `terrain.rs`, la revisión de features, la composición y la
aceptación final.

**B6 feedback acotado de pathfinder (2026-09-29):** seis declaraciones de
`map_manager/pathfinder.rs` se habían ampliado de `pub(super)` a `pub` al mover
el módulo. `SMOOTH_PATH_STEP_SIZE_LIKE_CPP`, las dos conversiones de posición y
`WorldMMapPathfinderMessageLikeCpp` solo se usan en el archivo y ahora son
privadas. `point_path_limit_for_distance_like_cpp` y
`random_path_result_from_path_type_like_cpp` los usa también el módulo hermano
de movimiento y quedan `pub(super)`. El test de movimiento importa el último
por su ruta anidada privada, sin reexport público; se retiró su import del
fixture común. No se cambiaron reglas ni datos.

Los checks secuenciales con un job, `PROTOC` local y target del worktree
pasaron: `cargo check -p wow-map`, `cargo test -p wow-map --lib path` (37/37,
854 filtrados) y `cargo check -p wow-world`; logs
`target/b6-pathfinder-wow-map-check.log`,
`target/b6-pathfinder-wow-map-test-path.log` y
`target/b6-pathfinder-wow-world-check.log`. También pasó `git diff --check`.
Es feedback de desarrollo, no aceptación final de B6 ni de la macro.

**Evidencia disponible:** revisión de código, Git, manifiestos y sesión DeepSeek
`51439d45-bfa1-4f1a-8538-38729d3f7c0d`, más los checks de desarrollo indicados arriba.
El último manifiesto final encontrado, `20260927T140832.222423Z-3887452-final.json`, pasó
en 423.853 s sobre `ee49c8ea`, anterior a este incremento. No existe aceptación final
acreditada de `a0df47b3`. Los recuentos y verdes de las notas históricas no la sustituyen.
El rollback del piloto borró temporalmente tests preexistentes; se restauraron y están presentes.

**Secuencia propuesta para la próxima ejecución:**

El objetivo vigente es completar la macro de modularización de `wow-world` de #1233 en esta
rama, con sus consumidores y evidencia. B5/B6 son un bloque interno; su corrección por sí sola
no cierra la macro ni justifica una PR parcial.

1. Revisar los exports propios restantes de `map_manager`, la composición de features y el
   contrato entre los dos checkers de capas. Integrar la pauta del worker. Mantener movimiento
   y comportamiento separados; el piloto verde todavía no cierra B3.
2. Extender B3 desde el piloto verde a `quest_tests`, `character_tests` y `loot_tests`, una
   suite por responsabilidad comprobada, con fixtures mínimas, consumidores y recuentos
   conservados. `validation-v2 final` usa `cargo test --lib`; ejecutar los nuevos targets
   de integración explícitamente o adaptar su cobertura en el runner. Medir antes/después.
   El inventario de partida de las cuatro suites era 48429 líneas y 925 anotaciones de test
   incluyendo raíces; los 44665 históricos no eran el total actual.
3. Terminar B7 y las demás responsabilidades de `wow-world` de #1233, incluidos consumidores,
   ownership, módulos y tests; comprobar ambos criterios semántico y físico. La arquitectura
   restante de #584 y C/D/E sigue su orden por dependencias; mantener #584 → #583 → #153.
4. Al completar la macro, ejecutar la aceptación del candidato comprometido:
   `./tools/validation-v2 final --base origin/3.4.3 --architecture --timings`, más la integración
   aplicable no cubierta. Un ejecutor, `VALIDATION_V2_CARGO_JOBS=1`, target propio y `PROTOC`
   según AGENTS.md. Registrar SHA, comandos, salidas y tiempo completo; superar 600 s incumple
   el objetivo ordinario. PR/publicación/merge conservan sus autoridades.

Al retomar, leer esta entrada, inspeccionar el diff local y procesos activos y actualizar aquí
el estado real, evidencia y siguiente paso. No crear otro plan o handoff paralelo. El piloto B3
está comprometido y verde en feedback local; las demás suites, B7, la macro y su aceptación
siguen pendientes.

**Migración de quest compilable, aún sin aceptación:** `1a5ea280` traslada
`quest_tests` al target externo y deja seis pruebas de reglas privadas en
`handlers/quest/rule_tests.rs`. La comparación de nombres con los 19 archivos
anteriores conserva exactamente 225 tests: 219 externos + 6 privados, sin
nombres perdidos, añadidos ni duplicados. Los puentes bajo `test-fixtures`
mantienen los fallbacks existentes sin `Player`, las rutas canónicas de quest,
reputación, habilidad y moneda, los planes de persistencia y las respuestas de
handler; los tipos de plan y los mapas mutables no se exponen al test externo.

El diagnóstico inicial del target tuvo 1 497 errores. El check de desarrollo
`cargo check -p wow-world --test quest_tests` terminó con código 0 en aarch64
(`target/b3-quest-item-persistence-planner-check.log`), sobre el árbol previo
al commit de movimiento; el delta posterior fue solo retirar espacios finales.
Los filtros de reglas privadas pasaron 5/5 y 1/1 por separado, y un test
focalizado del target externo pasó 1/1 (218 filtrados;
`target/b3-quest-item-persistence-planner-focused-test.log`). No se ejecutaron
las 219 pruebas externas completas ni se revalidó el modo sin feature tras los
últimos gates. Estos son checks de implementación, no aceptación final del SHA
`1a5ea280` ni de la macro. `character_tests`, `loot_tests`, B7 y el resto de
`wow-world` siguen pendientes.

**Historial de cortes internos de character:** la suite original de 303 tests
(13 280 líneas con fixtures) se copió a un target externo y se retiró su montaje
antiguo en el árbol de trabajo. Antes de mover las pruebas de transport, el check
de 647 errores (`target/b3-character-creation-context-external-check.log`, exit
101) había bajado desde 656 al trasladar pruebas puras de defaults/salud a
`creation_support_tests.rs` y de rest-state a `login_context_tests.rs`. Un corte
anterior de 681 errores (`target/b3-character-enumeration-support-external-check.log`)
bajó 25 al trasladar reglas puras de homebind/graveyard y battleground a
`login_support_tests.rs`. Los filtros de creation-support y login-context pasaron
3/3 y 1/1 en sus logs focalizados. `default_homebind_reads_primary_then_neutral_pandaren_from_startup_store_like_cpp`
permanece externa porque configura una `WorldSession`; los casos dependientes de
sesión o puertos tampoco se trasladan. Los módulos privados de login-support,
enumeration, creation-support y login-context quedaron comprometidos por separado;
los puentes de interaction source, trainer, gossip, canonical owner y regeneración
constan en checkpoints anteriores. Estos registros son históricos y el corte vigente
se describe a continuación.

En este slice se trasladaron, sin cambiar cuerpo ni nombre, las tres pruebas
`persisted_transport_login_*` de offsets válidos/corruptos y ruta de mapa desde
`tests/character_tests/persistence.rs` a `login_transport_support_tests.rs`. Solo usan
GUIDs, posiciones, valores y rutas locales; no dependen de sesión, DB ni puertos. El
ancla 3.4.3 es `Player::LoadFromDB` en `src/server/game/Entities/Player/Player.cpp`
(búsqueda del transporte en el mapa, cálculo del punto de pasajero y validación de
coordenadas y límite ±250); no se cambió esa regla. El módulo dueño quedó comprometido
en `d5b53874`, mientras el target externo sigue sin commit.

Evidencia posterior: `cargo test -p wow-world --lib persisted_transport_login` pasó
3/3 (`target/b3-character-transport-focused-test.log`). El corte de equipment sets
de 589 errores (`target/b3-character-equipment-set-external-check.log`, exit 101)
bajó desde los 637 previos mediante el puente comprometido en `822774c3` y las
adaptaciones consumidoras: 28 E0599 y 20 E0603 menos. Sus wrappers bajo
`test-fixtures` delegan a la colección canónica del `Player` o conservan el fallback
sin `Player`; los consumidores usan los tipos públicos `wow_entities::PlayerEquipmentSet*`
sin publicar aliases privados de `session`. Después, el puente faction de `32c01e18`
y sus seis consumidores redujeron otros seis E0599. El último check externo,
`CARGO_BUILD_JOBS=1 PROTOC=/home/ubuntu/.local/protoc/bin/protoc
CARGO_TARGET_DIR=/home/server/rustycore-world-refactor/target cargo check -p wow-world
--features test-fixtures --test character_tests`, terminó con exit 101 y **583 errores**
(`target/b3-character-faction-template-external-check.log`): E0277=10, E0422=30,
E0425=58, E0432=2, E0433=30, E0599=114, E0603=35, E0609=11, E0616=14 y E0624=279.
Después, el snapshot read-only de item bonus de `b505aaa7` resolvió siete E0599,
usando el tipo canónico `wow_entities::PlayerItemBonusStateLikeCpp` y el fallback
existente sin `Player`. El último check externo terminó con exit 101 y **576 errores**
(`target/b3-character-item-bonus-external-check.log`): E0277=10, E0422=30, E0425=58,
E0432=2, E0433=30, E0599=107, E0603=35, E0609=11, E0616=14 y E0624=279. El check
default `cargo check -p wow-world` pasó con exit 0 después de ambos slices
(`target/b3-character-item-bonus-default-check.log`). Los dos filtros focalizados
de equipment sets (`equipment_set` y
`canonical_player_saved_equipment_and_void_storage_follow_handle_generation_like_cpp`)
pasaron 1/1 cada uno en sus logs; no se ejecutaron tests para faction ni item bonus. El censo conserva
exactamente los 303 nombres originales: 268 externos y 35 privados en sus módulos dueños,
sin ausentes, añadidos ni duplicados. El target externo aún no compila; no se ejecutaron
los 268 tests externos completos ni aceptación final. El traslado y las adaptaciones
consumidoras siguen sin commit; esta evidencia es de desarrollo, no aceptación de la suite
ni de la macro.

El corte de inventario posterior mantiene las ocho mutaciones de test que antes
llamaban a `update_inventory_item_object_like_cpp`: seis casos CHILD/creator usan
`mark_inventory_child_for_test`; los dos casos de encantamientos usan fixtures
acotadas que conservan expiración, `EQUIPPED`, slots y valores. Los métodos de
producción no ganan visibilidad pública; el fallback sin `Player` y su espejo
quedan bajo `any(test, feature = "test-fixtures")`. El check normal de
`wow-world` pasó (`target/b3-character-item-enchantment-default-check.log`).
El check externo terminó con exit 101 y **568 errores**
(`target/b3-character-item-enchantment-external-check.log`), frente a 576
antes del corte: E0599=99 y E0624=279, sin categorías nuevas. No se pudo
ejecutar un test focalizado de esos dos casos porque el target externo todavía
no compila y no existe uno equivalente bajo el módulo dueño. El productor de
fixtures se conserva como checkpoint local; los consumidores externos y la
suite completa siguen sin commit ni aceptación.

La fixture `ensure_login_player_controller_for_test` reenvía sin alterar los
ocho argumentos del método existente y cubre las 13 llamadas de
`character_tests` en cinco archivos. El método conserva su visibilidad interna;
el wrapper solo existe bajo `test-fixtures`. `cargo check -p wow-world` pasó
(`target/b3-character-login-controller-default-check.log`). El target externo
continúa con exit 101, ahora **555 errores**
(`target/b3-character-login-controller-external-check.log`): E0624 bajó de
279 a 266 y las demás categorías permanecieron iguales. El test unitario
`ensure_login_player_controller_is_idempotent_like_cpp` pasó 1/1, con 3145
filtrados (`target/b3-character-login-controller-focused-test.log`). Sigue
pendiente compilar y ejecutar los 268 casos externos y la aceptación de B3.

Un corte posterior añadió en los consumidores externos el import público
`wow_packet::packets::query::NameCacheLookupResult` (seis usos) y
`std::collections::HashMap` (cinco usos), sin cambiar cuerpos de test ni APIs de
producción. El único check externo del corte terminó con exit 101 y **544
errores** (`target/b3-character-namecache-hashmap-import-check.log`): E0422 bajó
de 30 a 24 y E0433 de 30 a 25; las demás categorías no cambiaron. Son once
errores de nombres resueltos, no aceptación de la suite. Los consumidores
siguen sin commit.

Las ocho lecturas de `game_time_ms_like_cpp` en la suite externa usan ahora
`game_time_ms_for_test`, que delega en el reloj monotónico original sin
duplicarlo; los `wrapping_sub` y las aserciones permanecen. El check default
pasó (`target/b3-character-clock-fixture-default-check.log`) y el externo
terminó con exit 101 y **536 errores**
(`target/b3-character-clock-fixture-external-check.log`): E0603 bajó de 35 a
27, sin cambio en las otras categorías. La suite sigue sin compilar ni ser
aceptada; sus consumidores permanecen sin commit.

El siguiente grupo de cuatro llamadas externas a
`SessionPlayerController::new`/`attach_player_controller_like_cpp` **no admite
un mero wrapper**: `session/player_binding.rs` escribe bootstrap y seer solo
bajo `cfg(test)`, pero bajo `cfg(not(test))` instala inmediatamente el Player
desacoplado. Los cuatro consumidores son fixtures de área/banco/observador y
la consulta de nombre conectado. `session/state.rs` y
`session/construction.rs` conservan nombre, raza/clase/género, marca de
bootstrap y visibilidad con gates de test; posición y nivel ya admiten
`test-fixtures`. Antes de mover estas llamadas, trazar qué campos y lectores
necesita cada escenario y extender únicamente el contrato de fixture que
reproduzca la ruta unitaria, incluido el momento de instalación y seer. No
usar la rama de producción en el target externo para reducir errores de
compilación si altera la prueba. Este grupo permanece sin implementación ni
evidencia de ejecución.

El puerto fallido de persistencia de inventario permanece como tipo privado:
su módulo de fixture admite `test-fixtures` y una operación pública de fixture
instala el mismo `failed()` mediante el setter existente. Cuatro consumidores
externos de item llaman a esa operación. El check default pasó
(`target/b3-character-inventory-port-wow-world-check.log`); el target externo
terminó con exit 101 y **534 errores**
(`target/b3-character-inventory-port-external-check.log`): E0432 bajó de 2 a
1 y E0603 de 27 a 26, sin cambios en las demás categorías. Un test unitario
del mismo puerto fallido,
`failed_existing_stack_store_publishes_neither_count_nor_binding`, pasó 1/1
con 3145 filtrados (`target/b3-character-inventory-port-lib-test.log`). Los
cuatro escenarios externos aún no se han ejecutado y siguen sin commit.

## 4. Qué significa "verde" en cada nivel (no confundir niveles)

1. **Compila**: `cargo check -p <crate>` (y sus consumidores). Es el bucle de trabajo, no evidencia
   de entrega.
2. **Suite**: los tests del crate afectado, y si se movieron tests, los invariantes contados
   (`#[test]`/`#[tokio::test]`, `inventory::submit!`, lista de escenarios).
3. **Composición**: `cargo check -p world-server --tests` y los targets de integración.
4. **Aceptación**: la campaña final única. Solo aquí se habla de aceptado.

Ninguna fase se declara cerrada con el nivel 1; la tabla de estado registra el nivel alcanzado.

## 5. Reglas para no perdernos

1. **Una fase, un commit**, con su mensaje diciendo qué cambió y qué se verificó.
2. **Movimiento y comportamiento separados**, siempre.
3. **Nunca** subir un techo, regenerar una política o retirar un test para que pase una fase.
4. Si una fase revela trabajo no previsto, **se añade a este documento** antes de hacerlo.
5. Al cerrar cada ola, se reejecuta la auditoría y se actualiza §1 y §3.
6. Nada se publica (push/PR/merge) sin autorización explícita.

## 6. Decisiones arquitectónicas (ADR-001…008)

Estado: **aprobadas** el 2026-09-24 salvo donde se indique. Cada una se implementa dentro de la
fase que la referencia.

**ADR-001 — Entrega por olas (aprobada).** Una PR por ola en el programa general;
la macro de modularización de `wow-world` se entrega completa bajo #1233 en una
sola rama/PR conforme al alcance vigente, aunque atraviese varias fases B. Cada entrega es
auto-contenida (nunca se
cierra con el árbol rojo) y revertible como un merge. Al abrir cada ola: rebase sobre
`origin/3.4.3` y comprobación de que nadie más tiene trabajo abierto en los mismos paths. La
campaña `final --architecture` se corre **una vez por ola**, no por fase; dentro de la ola solo
`cargo check`. Como los checks hospedados se saltan en PRs de `alseif0x`, la campaña local es la
única puerta real y su evidencia va en el cuerpo de la PR.

**ADR-002 — Módulos: nativo por defecto, Wasm opcional (aprobada).** Todo lo de primera parte se
compila nativo (subsistemas opcionales por *features*, apagadas por defecto). El **mismo contrato**
sirve para ambos planos y por eso es **de datos**, no de traits: `wow-module-api` expone hooks
versionados con payloads propios (nada de referencias prestadas) y un `MODULE_API_VERSION`. Un
módulo nativo se enlaza in-process mediante un adaptador fino (sin serialización); un módulo Wasm
se sirve por el adaptador host (con serialización en la frontera). Se elige **wasmtime con
Component Model/WIT** (interfaz tipada y versionada; extism queda como alternativa si queremos
atajos de PDK, ya que se apoya en wasmtime). Reglas del sandbox: lista blanca de host functions,
sin handles de BD, sin escritores de paquetes, sin accesso a storage interno, límite de ejecución
(fuel/epoch) para que un módulo no cuelgue el servidor, y fallo de módulo **aislado** (se registra
y se desactiva; el servidor no cae). Versión distinta ⇒ se rechaza al instalar. `world-modules`
depende solo de `wow-module-api`, nunca de `world-server`. Coherente con el proyecto: el producto
Rust/Wasm/C es obligatorio, la activación por operador es opcional.

**ADR-003 — Núcleo funcional, cáscara imperativa (aprobada).** Crates L0-L3 síncronos y puros:
prohibido `tokio`, `parking_lot`, `sqlx`, `rand` en `[dependencies]`, verificado por
`cargo xtask check-deps`. Todo `await`, lock y fence vive en la app, con orden de lock documentado
por crate. `#![forbid(unsafe_code)]` en dominio y app; las **intenciones** llevan `#[must_use]`.

**ADR-004 — Datos, tiempo y azar inyectados (aprobada).** Catálogos como parámetros prestados
(`&ItemStore`), nunca pool ni `Arc<dyn Store>`. Tiempo como valor (`GameTime`, `diff_ms: u32`);
prohibido `Instant::now()`/`SystemTime` en dominio. Azar por generador determinista inyectado con
semilla registrada en el test, para que una tirada sea reproducible y contrastable con C++.

**ADR-005 — Errores (aprobada).** `thiserror` en librerías con un `enum <Dominio>Error`
`#[non_exhaustive]` por crate; `anyhow` solo en binarios y nunca en API pública de librería. Los
errores de dominio no contienen tipos de la app ni de infraestructura: la app decide el efecto en
un único sitio por familia y es la única que loguea.

**ADR-006 — Código generado (aprobada).** Generado **versionado** en el repo (patrón `cargo
xtask`), con cabecera `// @generated by xtask codegen — do not edit`, exento de presupuestos y
techos pero no de sintaxis; `cargo xtask check-generated` regenera y compara; prohibido generar
desde `build.rs` hacia `src/`.

**ADR-007 — Arranque y alcance de un agente (aprobada).** `cargo xtask context <fase>` imprime el
slice activo (≤150 líneas) desde la tabla de estado; `cargo xtask check-scope` **falla si hay
cambios fuera de los paths declarados por la fase activa**. Es la barrera contra el fallo que
produjo el megarefactor roto que hubo que rescatar.

**ADR-008 — Superficie pública (aprobada).** Ningún `pub` sin consumidor identificado;
`#[non_exhaustive]` y traits sellados en puntos de extensión; `#[doc(hidden)]` para puentes;
snapshot con `cargo-public-api` de `wow-entities`, `wow-map` y `wow-packet` al cerrar cada ola con
diff revisado. `adapter.rs` es temporal y se retira con su migración.

**Adiciones del estudio (van a A0):** `cargo-deny` debe vigilar **licencias incompatibles** (el
proyecto es GPL v3), no solo avisos y duplicados; verificar y documentar el resolver de features
(edition 2024 ⇒ resolver 3) por la unificación de features entre dependencias normales; política
de `unsafe` (forbid en dominio/app, permitido solo en crates justificados con
`deny(unsafe_op_in_unsafe_fn)`); y ejecutar **`cargo-machete` antes de A1** para no arreglar crates
que A1 va a retirar.

**ADR-009 — Nomenclatura de paridad (aprobada 2026-09-24, decisión del usuario).** El sufijo
`_like_cpp` y los tipos `...LikeCpp` son legado y **no se usan en código nuevo ni en refactors**; el
nombre dice el dominio y la procedencia C++ vive en el comentario/ancla, el mensaje de commit y el
ADR/checkpoint. **No hay renombrado en masa del legado** en este programa: cambiaría miles de
identificadores, todas las baselines y las superficies registradas, y su riesgo no compensa. Sí hay
**renombrado oportunista** al extraer un dominio a su crate o al reescribir un fichero, dejando un
alias o `adapter.rs` temporal si hace falta compatibilidad. Un renombrado masivo, si se quiere,
será una campaña propia con su aceptación.

## 7. Primeros resultados de las comprobaciones (A0.2)

`tools/xtask` (sin dependencias externas) con `structure-audit`, `check-layers`, `check-deps`,
`context <fase>` y `check-scope <fase>`. Los dos primeros usan **ratchets con baseline**: fallan si
aparece una violación nueva **o si una entrada del baseline ya no existe** (la lista solo puede
encoger).

- `check-layers`: **PASS** con 13 violaciones conocidas (`tools/xtask/layer-baseline.txt`).
- `check-deps`: **PASS** con 4 dependencias prohibidas conocidas
  (`tools/xtask/deps-baseline.txt`): `wow-ai` y `wow-loot` declaran `rand`; `wow-loot` declara
  `tokio`; `wow-conditions` declara `parking_lot`. Son exactamente los dominios que deben pasar a
  entradas inyectadas (ADR-004) y a núcleo síncrono (ADR-003); se retiran en la ola C/D.
- La regla se afinó al medir: `sqlx` es legítimo en `wow-database`/`wow-persistence` y `tokio` en
  la capa de red; la prohibición estricta es para los crates de **reglas** (L3).
- A0.3: `[workspace.lints]` ya existía y **13 crates ya optaban** a él; se añadió el opt-in a los
  28 restantes (41/41, incluido `tools/xtask`). Medición de ruido con la política actual
  (`all` + `pedantic` en warn): `wow-loot` 2 warnings, `wow-map` 15, `wow-world` 350. La política
  es usable; los 350 de `wow-world` son deuda de F13 y no bloquean. Corrección: una nota anterior
  de este documento decía "0 de 40" por un grep con el patrón equivocado.
- `cargo-machete`, `cargo-deny`, `cargo-nextest`, `cargo-public-api` y `cargo-hakari` **no están
  instalados** en el host; A0.4 y A0.7 quedan parcialmente cubiertos por `structure-audit` y
  pendientes de esas herramientas.
- El baseline de capas refleja las inversiones que la ola C debe retirar; el de dependencias, lo
  que ADR-003/004 exige retirar de los dominios.
- Los techos físicos hicieron su trabajo en la puerta de la ola A: `chat.rs` (312 > 311) y
  `trainer.rs` (855 > 846) habían crecido por los movimientos. En vez de subir el techo se sacaron
  sus registros de opcode a `chat/registrations.rs` y `trainer/registrations.rs` (movimiento puro);
  `chat.rs` queda en 84 líneas y el ratchet físico pasa con 102 techos legacy.

## 8. Decisiones de A0

**A0.1 — Mapa de features (decidido).** Subsistemas **opcionales**, apagados por defecto y nunca
requeridos por el build base: `modules`/`wasm-runtime` (módulos de operador, ADR-002),
`scripts` (scripting) y `anticheat`. El build base (`cargo check -p world-server`) no puede
depender de ellos; la dirección es `world-modules → wow-module-api` y nunca hacia `world-server`.
Los crates reservados (`wow-spell`, `wow-pvp`, `wow-achievement`, `wow-items`, `wow-quest`,
`wow-economy`, `wow-progression`, `wow-battlegrounds`, `wow-dungeon-finding`,
`wow-account-collections`) **no** son features: son dominios a crear/llenar o a retirar en A1.

**Resolver (decidido): se mantiene `resolver = "2"`.** Ya evita la unificación de features entre
build-dependencies y dependencias específicas de target, que es el riesgo real; el resolver 3 solo
añade resolución consciente de MSRV y aquí el toolchain está fijado, así que cambiarlo traería
churn de `Cargo.lock` sin beneficio. Documentado para que nadie lo "actualice" sin argumento.

**A0.4 — Higiene de grafo (parcial).** `cargo-machete`, `cargo-deny`, `cargo-nextest`,
`cargo-public-api` y `cargo-hakari` **no están instalados** en este host. Cobertura actual:
`xtask structure-audit` (stubs, sin consumidor, tamaños, capas) y los ratchets de `check-layers` /
`check-deps`. Pendiente: ejecutar `cargo-deny` con política de **licencias** (el proyecto es GPL v3)
y `cargo-machete` cuando estén disponibles; no se finge que se han pasado.

**A0.6 — Presupuesto de documentación (decidido).** Los documentos de arquitectura se mantienen
**≤300 líneas** cada uno; el histórico y las decisiones van a ADRs, no a un plan que crece.

**A0.7 — Herramientas opcionales (decidido).** `cargo-nextest`: adoptar cuando esté disponible, no
es requisito. `cargo-public-api`: el snapshot de `wow-entities`/`wow-map`/`wow-packet` queda
pendiente de la herramienta; la regla de ADR-008 se aplica igual. `cargo-hakari`: **no** se adopta;
solo si una medición de build demuestra duplicación de features.

## 9. Lecciones de la ola A (apuntadas antes de seguir)

- **Las categorías de `dependency-policy.json` son más estrictas que el mapa de capas de este
  plan.** `domain-runtime` solo puede depender de `foundation` y `domain-runtime`; el mapa del plan
  situaba `wow-data` (categoría `adapter-platform`) por debajo de `wow-map`, pero el proyecto lo
  prohíbe. Consecuencia: el movimiento de `phasing` a `wow-map` **se revirtió** (commit de revert) y
  `phasing` necesita su **propio crate de categoría `application`** (`wow-phasing`) con una
  excepción justificada hacia `adapter-platform`, que se hará en la ola C/D. Regla para el futuro:
  **antes de mover código entre crates, comprobar la categoría de ambos en
  `tools/architecture/dependency-policy.json`**, no solo la capa del plan.
- **`tools/xtask` se clasificó como `tooling`** en el mismo fichero; sin clasificar, el checker
  rechaza el paquete.
- **El ratchet de hotspots puede encoger sin tocar la baseline**: tras el revert, `loot` y `quest`
  quedan por debajo de lo registrado, que es aceptable (la baseline es cota superior, no espejo).
- **Trabajo de #1233 traído adelante por la puerta**: las familias de campos de `WorldSession` del
  ledger estaban sin actualizar desde el refactor (139 nombres obsoletos, 21 campos nuevos sin
  dueño). Se reclasificaron con regla explícita: los 15 `*_test_fixture_like_cpp` a
  `test_only_fixtures`, y los 6 de producción por palabra clave (durabilidad→inventario/economía,
  límites de stats y regeneración de tablas→catálogos, auras de criatura→mapa/runtime, log de
  ejecución de hechizo→hechizos/progresión, sincronización de tiempo→driver/timers). Conteos:
  531 campos (225 producción, 306 fixtures).

## 10. Trabajo no previsto detectado por la puerta de la ola A (2026-09-24)

La puerta `final --architecture` ya no encuentra errores de compilación (el barrido
`cargo check --workspace --all-targets` sale 0) pero al ejecutar las suites aparecen **19 tests
fallando en `wow-world --lib`** (3919 pasan) y 1 corregido en `wow-data`. Son fallos que estaban
**ocultos**: los ficheros no compilaban, así que nunca se ejecutaron en esta rama. Cada uno se
diagnostica por separado y se arregla con evidencia (ancla C++ o ruta movida), nunca debilitando
la aserción. Ficheros con el fallo registrado:

- `crates/wow-world/src/handlers/character/../character_tests/item_1.rs:12`
- `crates/wow-world/src/handlers/character/../character_tests/item_1.rs:30`
- `crates/wow-world/src/handlers/economy/tests/trade/session.rs:164`
- `crates/wow-world/src/handlers/economy/tests/trade/session.rs:198`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_1.rs:213`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_1.rs:240`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:348`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:389`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:460`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:489`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:530`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:559`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:658`
- `crates/wow-world/src/handlers/void_storage_tests/scenarios_2.rs:768`
- `crates/wow-world/src/session/../session/tests/player_spell_hit_source/trait_glyph_and_zone_gates.rs:97`
- `crates/wow-world/src/session/../session/tests/scenarios_instances_1.rs:617`
- `crates/wow-world/src/session/../session/tests/scenarios_instances_1.rs:646`
- `crates/wow-world/src/session/../session/tests/scenarios_misc_8.rs:522`
- `crates/wow-world/src/session/../session/tests/scenarios_persistence_3.rs:680`

Tests afectados:

- `GroupInstanceResetMethodLikeCpp::Manual)`
- `GroupInstanceResetMethodLikeCpp::OnChangeDifficulty)`
- `GroupInstanceResetResultLikeCpp::NotEmpty,`
- `GroupInstanceResetResultLikeCpp::Success,`
- `handlers::character::tests::item_1::continue_login_inventory_reads_cross_the_typed_lifecycle_port`
- `handlers::character::tests::item_1::continue_login_item_repairs_cross_the_typed_lifecycle_port`
- `handlers::economy::tests::trade::session::accept_trade_records_acceptance_and_notifies_partner_like_cpp`
- `handlers::economy::tests::trade::session::unaccept_trade_clears_acceptance_and_notifies_partner_like_cpp`
- `handlers::void_storage::tests::scenarios_1::locked_login_discards_residual_void_rows_and_initializes_empty_storage_like_cpp`
- `handlers::void_storage::tests::scenarios_1::unlock_submits_one_semantic_write_before_runtime_publication_like_cpp`
- `handlers::void_storage::tests::scenarios_2::deposit_commits_typed_plan_before_runtime_publication_like_cpp`
- `handlers::void_storage::tests::scenarios_2::deposit_definite_rollback_keeps_money_inventory_and_void_state_unchanged`
- `handlers::void_storage::tests::scenarios_2::deposit_definite_rollback_retains_active_item_loot_view_atomically`
- `handlers::void_storage::tests::scenarios_2::deposit_indeterminate_commit_quarantines_without_runtime_publication_like_cpp`
- `handlers::void_storage::tests::scenarios_2::deposit_unknown_commit_reconciles_from_durable_money_like_cpp`
- `handlers::void_storage::tests::scenarios_2::mixed_transfer_validation_failure_publishes_no_partial_deposit`
- `handlers::void_storage::tests::scenarios_2::swap_definite_rollback_keeps_void_slots_unchanged`
- `handlers::void_storage::tests::scenarios_2::swap_unknown_commit_with_unchanged_money_quarantines_session`
- `session::tests::player_spell_hit_source::trait_glyph_and_zone_gates::player_spell_hit_source_authority_gates_update_zone_aura_producers_like_cpp`
- `session::tests::scenarios_instances_1::represented_player_reset_not_empty_forgets_only_on_change_difficulty_like_cpp`
- `session::tests::scenarios_instances_1::represented_player_reset_success_forgets_recent_instance_like_cpp`
- `session::tests::scenarios_misc_8::give_xp_runtime_rejects_no_xp_gain_player_flag_like_cpp`
- `session::tests::scenarios_persistence_3::player_create_flags_use_loaded_and_canonical_bits_like_cpp`

### Cierre de la ola A (2026-09-25)

`./tools/validation-v2 final --base origin/3.4.3 --architecture --timings` -> **exit 0, status
passed, 568 s** (manifiesto `20260925T024412.450111Z-2914971-final.json`), dentro del presupuesto
de 600 s. Antes de esta puerta la rama tenia 898 errores de compilacion y 19 tests fallando; el
camino completo esta en los commits de la rama, cada uno con su evidencia.

Publicacion: PR creada como draft al abrir la ola y fusionada al cerrarla (ADR-001). La ola A deja
`handlers/misc` desmontado, tres fronteras movidas (personal-phase retirado, `phasing` de vuelta a
`wow-world` por politica de categorias, fixtures compartidos extraidos), el `xtask` con ratchets,
los lints opt-in en 41 paquetes, el ADR-009 de nomenclatura y las baselines revisadas. A1 empieza
despues de esta fusion.

### A1 cerrada (2026-09-25)

- `wow-pvp` y `wow-achievement` (1 linea cada uno) retirados del workspace. Su reserva **sale** de
  `dependency-policy.json` porque el checker exige que un paquete reservado exista y este
  clasificado; la intencion se conserva aqui: ambos nombres quedan para las entregas de paridad de
  Part 2 (#48), que los recreara con su contenido.
- `wow-scripts` (44 lineas, fachada de `wow-script`) consolidado: `world-server` llama directamente
  a `wow_script::lifecycle::on_startup_like_cpp()/on_shutdown_like_cpp()` y su asercion se conserva
  como test de integracion de `wow-script` (`tests/lifecycle_facade.rs`).
- La utilidad `wow-collections` renombrada a `wow-util-collections` para liberar el nombre del
  dominio de colecciones (`wow-account-collections`).
- Decisiones registradas: `rustycore-db` **se queda** (es un binario de administracion de BD, y la
  bandera "sin consumidor" es lo normal en un binario); `wow-session` **se queda** (kernel de
  transporte ganado en #297, consumido por `wow-world`); `wow-chat` **se queda** (dominio de reglas
  de chat: hipervinculos y validacion); `world-modules` **no se toca aqui** (es generado y su
  inversion es el objeto de A2).
- Baseline de capas depurada de las entradas que quedaron obsoletas con la retirada.

### A2 y A3 verificadas: no habia inversion real (2026-09-25)

**A2 — `world-modules`.** La "inversion" venia de mi auditoria en Python, que marcaba toda arista
lateral como violacion. Verificado contra la politica y el codigo: `world-modules` es categoria
**composition**, `world-server` tambien, y `composition` puede depender de `composition`
(`allowed_category_dependencies`); ademas `world-modules` usa `world_server::run_with_modules`, la
API de libreria prevista para componer. `xtask check-layers` ya excluia las fuentes de capa 5, asi
que nunca la marco. **No hay nada que invertir**; el generador `tools/modules/compose.py` y su
salida se quedan como estan hasta D6 (contrato Wasm), que es donde cambia el modelo de modulos.

**A3 — tooling y vendor.** `capture-diff` ya esta clasificado como **tooling** en
`dependency-policy.json` (mis notas anteriores hablaban de "capas de juego" por prosa, no por
politica), y `xtask` tambien. Lo unico que faltaba era dejar explicito el criterio de **codigo
vendido**: `wow-recastdetour` es un port de terceros y queda exento de presupuestos de tamano y de
convenciones de nomenclatura, conservando su clasificacion de dependencia.

Leccion registrada: **la auditoria en Python y `xtask check-layers` deben coincidir**; cuando
discrepen, manda la politica (`dependency-policy.json`) y se corrige la herramienta que se
desvie.

### B2: segundo slice verificado-negativo y preparacion de B4 (2026-09-25)

**`entity_update_bridge` se queda en la aplicacion.** El plan lo enviaba a `wow-entities`, pero
importa `wow_packet` (3 usos): moverlo alli crearia la arista `domain-runtime -> adapter-platform`
que la politica prohibe. Es una **frontera entidad -> wire**, asi que su sitio es la app (o, mas
adelante, un crate de categoria `adapter-platform` con contrato propio). Igual criterio para
`profession` y `trainer_offer`, que ademas tocan `session`: van despues de B4.

**B4 (partir el tipo Dios): analisis previo, con datos.** `WorldSession` tiene **221 campos de
produccion** repartidos en las familias del ledger. Tamanos (produccion):

| campos | familia |
|---:|---|
| 1 | `player_identity_login_bootstrap` |
| 1 | `session_selected_player_binding` |
| 1 | `test_only_fixtures` |
| 3 | `player_social_chat_calendar_and_group_views` |
| 3 | `session_driver_timers_and_transitional_misc` |
| 4 | `directory_group_and_social_coordination` |
| 4 | `transport_and_physical_connections` |
| 6 | `player_movement_combat_and_visibility` |
| 9 | `mailbox_and_cross_session_delivery` |
| 11 | `packet_admission_dispatch` |
| 15 | `player_spells_quests_and_progression` |
| 16 | `player_inventory_loot_and_economy` |
| 22 | `session_identity_account_and_realm_policy` |
| 23 | `persistence_and_session_lifecycle` |
| 25 | `map_runtime_creature_gameobject_and_visibility` |
| 77 | `immutable_catalogs_configuration_and_services` |

**Primer corte recomendado**: la familia mas pequena con cohesión real y sin ser el nucleo de
identidad/conexion, es decir **`directory_group_and_social_coordination`** (4 campos:
`group_registry`, `pending_invites`, `game_event_quest_complete_tx`), seguida de
`session_driver_timers_and_transitional_misc` (3) y `player_social_chat_calendar_and_group_views`
(3). No se empieza por `transport_and_physical_connections` ni
`session_identity_account_and_realm_policy`: son centrales y su radio de llamadas es enorme.

**Metodo para cada sub-estado** (sin romper nada):
1. Declarar el sub-estructo en `session/state.rs` y mover alli SOLO los campos de la familia.
2. Exponer dos accesores estrechos: `pub(in crate::session) fn <nombre>(&mut self) -> &mut SubEstado`
   y su version de lectura. Un dueño, ningun espejo.
3. Mover a `impl SubEstado` los metodos que solo tocan esa familia; el resto de llamadores se
   repunta con `cargo check -p wow-world` como guia.
4. Cuando el sub-estado tenga contrato completo y ningun `&mut WorldSession` haga falta, se puede
   convertir en crate de dominio; hasta entonces es un modulo privado de la app.
5. Cualquier cambio de comportamiento va en su propio commit, con ancla C++.

### B4, primer intento: revertido y pitfall registrado (2026-09-25)

Intente sacar a `SessionDirectory` los tres campos de directorio social de la familia
`directory_group_and_social_coordination` (dejando `player_registry`, que tiene 58 ficheros de radio,
para su propio slice). El movimiento de campos y el inicializador anidado funcionan, pero la
**reescritura de puntos de uso es mas delicada de lo que asumi**:

- **Existen metodos accesores con el mismo nombre que los campos** (`fn pending_invites(&self)`).
  Una sustitucion global de `.<campo>` convierte tambien las *llamadas* `self.pending_invites()` en
  `self.directory.pending_invites()`, que el compilador rechaza. La reescritura debe distinguir
  acceso a campo de llamada a metodo (p. ej. por el parentesis siguiente) o, mejor, mover primero
  los metodos al `impl` del sub-estructo y dejar que los llamadores usen el accesor.
- La visibilidad efectiva de los tres campos era `pub(crate)`, no `pub(in crate::session)`: hay
  consumidores en `crates/wow-world/src/handlers/**`. El sub-estructo y su campo contenedor deben
  nacer con esa visibilidad, no ensancharla despues.
- Hay un `use` que debe acompanar al tipo en cada fichero que lo nombre (`construction.rs`), y el
  chequeo de propiedades de campos de `WorldSession` deja de contar los campos anidados: al mover
  la familia, la census de `session-ownership-policy.json` baja de 221 campos de produccion y hay
  que regenerarla con delta revisado.

El intento se revirtio sin dejar el arbol sucio; la rama sigue verde. Se retoma con el metodo
corregido: mover campos, mover metodos al `impl` del sub-estructo, y repuntar solo accesos (no
llamadas), con `cargo check` entre pasos.

### B4, primer slice ejecutado: `SessionDirectory` (2026-09-25)

`WorldSession` pierde tres declaraciones de campo (`game_event_quest_complete_tx`, `group_registry`,
`pending_invites`) hacia el sub-estado nombrado `SessionDirectory`, declarado junto a su dueno con la
visibilidad mas estrecha (`pub(in crate::session)`, la que ya tenian los campos: no se ensancha
nada). Los tres accesores (`set_group_registry`, `group_registry`, `pending_invites`) y sus 39
llamadores en `handlers/**` conservan la frontera, asi que no cambia ningun paquete, ninguna
persistencia ni ninguna autoridad; la construccion usa el `Default` derivado. `player_registry`
queda para su propio slice por radio de llamadas.

Lo que el intento anterior no habia visto, ahora medido:

- **El ledger de hotspot es un ratchet de crecimiento, no una medida libre.** Mover una familia a un
  sub-estado *anade* lineas de produccion al agregado logico (`session/mod.rs`), y el unico modo de
  que el slice cierre es registrar el crecimiento revisado en `runtime-ownership-ledger.json`
  (`latest_growth_review`) o retirar lineas equivalentes. Este slice cuesta **+3 lineas** de
  produccion (227862 -> 227865 totales) y se registro con la revision completa: sin segunda
  autoridad, espejo, cerrojo, reloj ni tarea. Las lineas bajan a cero cuando el sub-estado tenga
  contrato y salga del arbol.
- **Retirar imports "muertos" no es un atajo valido.** Los 9 imports que el chequeo de produccion
  marca como no usados en `session/{state,construction}.rs` los usa codigo `#[cfg(test)]` del mismo
  fichero; borrarlos rompe los tests, y marcarlos `#[cfg(test)]` solo traslada lineas de produccion a
  lineas de test, que el mismo ratchet tambien limita. Se revirtio.
- **La familia tambien se declara en el ledger de runtime**, no solo en la census: hay que mover los
  nombres a `directory` en `world_session_responsibility_families` y ajustar los contadores globales
  (525/219/306) o `check_architecture` falla por campos ausentes/obsoletos.
- **Repuntar accesos, no llamadas, con cuidado en los fixtures.** Los fixtures de test que tienen
  campos homonimos (`GroupReconciliationFixtureLikeCpp.group_registry`) no se repuntan; el compilador
  los senala uno a uno. Y un `use` nuevo debe insertarse fuera del grupo `#[cfg(test)]`, no entre el
  atributo y su import.
- **Conservar la procedencia al mover.** Los comentarios C++ de cada campo viajan con el campo al
  sub-estado; dejarlos atras pierde la ancla y deja comentarios huerfanos en el dueno.

Evidencia del slice: `cargo check -p wow-world` (0 errores), `cargo check -p wow-world --tests`
(0 errores), `cargo test -p wow-world --lib` (3901 pasan), census de sintaxis PASS (219 campos de
produccion), `check_architecture.py check` PASS y `self-test` PASS (20 fixtures). `cargo check
--workspace --all-targets` sigue en 0 errores.

### B4, segundo slice ejecutado: `SessionSocialLimits` (2026-09-25)

La familia `player_social_chat_calendar_and_group_views` (los dos topes de XP de Recruit-A-Friend y el
estado anti-flood de chat) pasa al sub-estado nombrado `SessionSocialLimits`, alcanzado por un unico
campo `social`. Misma visibilidad estrecha y mismos valores de construccion (85/4 y el par de
acumuladores por defecto); los cinco puntos de lectura/escritura (`session/social/contacts.rs`,
`session/catalogs/operations.rs`) conservan su comportamiento. Census: 219 -> 217 campos de produccion.

Leccion nueva de este slice: **el nombre del campo contenedor se paga en lineas**. Con
`social_limits`, tres de las cinco expresiones repuntadas superaban las 100 columnas y `rustfmt` las
partia, anadiendo 13 lineas de mas al agregado; con `social` solo quedan dos particiones inevitables
(los nombres `..._difference_like_cpp` de 58 caracteres) y el slice cuesta +16 lineas en vez de +26.
Antes de elegir el nombre de un sub-estado conviene medir el punto de uso mas largo.

### B4, tercer slice ejecutado: `SessionDriverServices` (2026-09-25)

El estado de sincronizacion de tiempo y el RNG de gameplay representado pasan al sub-estado nombrado
`SessionDriverServices`, alcanzado por un unico campo `driver`; valores de construccion identicos
(`TimeSynchronizationStateLikeCpp::default()` y `StdRng::from_entropy()`). Census: 217 -> 216 campos
de produccion. `pending_bind` **no** se mueve en este slice: sus lectores incluyen
`handlers/instances`, que dependen del `pub(crate)` mas ancho que el campo ya tenia, y su dueno real
es la confirmacion de bind de Player/InstanceMap; la familia conserva su nombre transitorio hasta que
ese ultimo miembro salga.

Segunda leccion medida sobre el coste de un sub-estado: **los nombres de campo largos mas el prefijo
del contenedor se pagan en re-envoltura de `rustfmt`**. Aqui el diff inserta 111 lineas y borra 62
(+29 produccion, +20 test) sin cambiar una sola llamada ni comportamiento: `time_synchronization`
tiene 19 caracteres y, detras de `self.driver.`, empuja condiciones de `session/time_synchronization.rs`
y de los tests de driver/movimiento/publicacion por encima de 100 columnas. Antes de mover una familia
conviene contar las lineas que quedarian entre 94 y 100 columnas: ese es el coste real, no el numero de
campos. Cuando ese coste domine, el siguiente paso no es ensanchar el contenedor sino mover los
*metodos* de esa familia al `impl` del sub-estado, donde `self.<campo>` no lleva prefijo.

### B4, cuarto slice ejecutado: `SessionWorldView` (2026-09-25)

Los seis miembros restantes de `player_movement_combat_and_visibility` (area trigger activo, lookup de
mapas de taxi, instante del ultimo tick de combate, revision de salud melee presentada y los dos flags
de reino PvP) pasan al sub-estado nombrado `SessionWorldView`, alcanzado por un unico campo `view`.
Comentarios de procedencia C++ movidos con sus campos; valores de construccion identicos. Census:
216 -> 211 campos de produccion. Coste: +14 produccion/+7 test (solo una envoltura nueva).

Este slice ademas **estrecha** visibilidad: `active_area_trigger` era `pub(crate)` y pasa a
`pub(in crate::session)` porque sus seis lectores viven dentro del arbol de `session`; la visibilidad
ancha estaba sin usar. Es el sentido correcto de la escalera de visibilidad: el sub-estado no
ensancha, y aprovecha para apretar lo que sobraba.

### B4, quinto slice ejecutado: `SessionAddonFilter` (2026-09-25)

El filtro de addons del chat (`registered_addon_prefixes`, `filter_addon_messages`, C++
`WorldSession::_registeredAddonPrefixes` y `_filterAddonMessages`) pasa al sub-estado nombrado
`SessionAddonFilter`, alcanzado por un unico campo `addon_filter`. Census: 211 -> 210 campos de
produccion; coste +8 produccion, 0 test.

Tercer patron aprendido, y el que gobernara casi todas las familias que quedan: **si la familia se lee
desde `handlers/**`, el sub-estado no puede ser privado al arbol de `session`**. Estos dos campos eran
`pub(crate)` porque `handlers/chat/*` los lee; el sub-estado y su contenedor nacen con esa misma
visibilidad, y el comentario del tipo lo dice explicitamente para que nadie lo "arregle" estrechandolo.
Es la excepcion legitima a la escalera: no se ensancha nada nuevo, se conserva lo que los consumidores
ya necesitaban, y se documenta quien lo justifica.

**Frontera economica de B4**: la reduccion de campos se paga en accesos repuntados, y ese precio no es
uniforme. Medido por familia: `player_social_*` y `player_movement_*` costaron +16 y +14 produccion por
3 y 6 campos; `mailbox_and_cross_session_delivery` tiene 9 campos pero **196 accesos**, y
`immutable_catalogs_configuration_and_services` acumula 77 campos leidos por todo el arbol. Para esas
dos, el metodo de mover solo campos no escala: hay que mover *metodos* al `impl` del sub-estado (donde
`self.<campo>` no lleva prefijo) o extraer primero los subgrupos cohesivos y baratos, que es lo que se
hizo aqui con el filtro de addons.

### B4, sexto slice ejecutado: `SessionPhaseRail` (2026-09-25)

El rail de fase #787 (`session_phase_tx`, `session_phase_rx`) pasa al sub-estado nombrado
`SessionPhaseRail`, alcanzado por un unico campo `phase` con `tx`/`rx` dentro. El `bounded(2)` de
construccion, los dos accesores (`session_phase_sender_like_cpp`, `session_phase_receiver_like_cpp`),
las entradas del directorio que clonan el emisor y el consumidor del driver conservan su
comportamiento. Census: 210 -> 209 campos de produccion; coste +7 produccion, 0 test.

Cuarto patron aprendido, y aviso para los slices que quedan: **el nombre de campo puede existir en otros
tipos del mismo arbol**. `SessionPhaseAddressLikeCpp` y las entradas del directorio tienen su *propio*
`session_phase_tx`, y hay 8 literales de test que lo rellenan con
`detached_session_phase_rail_like_cpp()`. Una reescritura global de `.session_phase_tx` habria roto
tipos ajenos; aqui se repunto solo `self.session_phase_tx` en los dos ficheros de `WorldSession`
(`driver/phase_consumer.rs` y `player_registry_binding.rs`). Regla: cuando el nombre del campo es
generico, repuntar por fichero y con el receptor `self.`, nunca con regex global; y comprobar antes
cuantos literales ajenos comparten el nombre.

### B4, septimo slice ejecutado: `SessionSpellState` + `SessionQuestState` (2026-09-25)

La familia `player_spells_quests_and_progression` (15 campos) se parte en dos sub-estados cohesivos:
`SessionSpellState` (los cuatro sets de ids de spell-script, las dos autoridades de adquisicion, los
efectos de execute-log y el switch de offhand) y `SessionQuestState` (los umbrales de visibilidad de
quest por diferencia de nivel, las actualizaciones de estado de quest completada, la cola de progreso
de objetivos y los refrescos de visibilidad que esas transiciones piden), alcanzados por `spell_state`
y `quest_state`. **Census: 209 -> 196 campos de produccion** (el mayor salto de la ola). Coste: +83
produccion/+4 test, el mas caro hasta ahora, porque los nombres de campo llegan a 55 caracteres
(`represented_quest_objective_progress_events_like_cpp`) y el prefijo los re-envuelve; se paga a cambio
de 13 campos menos de tipo Dios. Los cuatro accesores homonimos se quedan en `WorldSession` y sus
llamadores no se tocan.

**Quinto patron, y el metodo que conviene usar de aqui en adelante.** Cuatro de esos nombres existen
*tambien* en un struct ajeno (`LegacyCreatureAggroConfigLikeCpp` en `creature_aggro_contracts.rs`) y hay
mas literales ajenos en `world-server`, asi que una sustitucion textual es corrupcion segura. El metodo
que funciono: mover primero la estructura (declaraciones + inicializadores) y despues **repuntar guiado
por el compilador**, parseando sus errores y reescribiendo exactamente esas lineas. Dos codigos hay que
cubrir: `E0609` (`no field ... on type WorldSession`) para los accesos normales y **`E0615`
(`attempted to take value of method ...`) para los campos que tienen un accesor homonimo**, porque al
quitar el campo el uso sin parentesis pasa a resolver al metodo. Con eso, un ciclo de repunte basto en
la lib y otro en `--tests`; el struct ajeno quedo intacto (0 sustituciones alli). El mismo slice hizo
crecer `handlers/quest/mod.rs` (+3 por re-envoltura), registro que el ledger tambien recoge: los
hotspots auditados son varios y no solo `session/mod.rs`.

### B4, octavo slice ejecutado: `SessionLifecycleState` (2026-09-25)

La familia completa `persistence_and_session_lifecycle` (23 campos) pasa al sub-estado nombrado
`SessionLifecycleState`, alcanzado por un unico campo `lifecycle`: instantes de login/logout y
calendario de guardado periodico, reclamaciones de carga y logout, tutoriales y datos de cuenta, los
puertos de persistencia, el rail de finalizacion, las filas de carga de mascotas y los trackers
durables de loot. Mismos valores de construccion y misma visibilidad de crate que sus handlers ya
necesitaban. **Census: 196 -> 174 campos de produccion** (segunda mayor reduccion de la ola). Coste:
+196 produccion/+27 test, el mayor hasta ahora, sobre 343 accesos repartidos por los modulos de
lifecycle, persistencia, driver, items y mascotas (620 inserciones, 352 borrados, casi todo
re-envoltura de `rustfmt`). Ningun otro owner auditado crecio: el ratchet solo exigio actualizar la
fila de `session/mod.rs`.

Balance acumulado de B4 (produccion): 221 -> 174 campos en ocho slices; el coste en lineas esta
registrado como deuda transitoria en `latest_growth_review`, y se retira cuando los sub-estados salgan
del agregado (olas D/E). La ganancia que persigue B4 es la superficie del tipo, no el LOC del arbol:
`WorldSession` ya no declara estado de transporte social, social limits, driver, vista de mundo,
filtro de addons, rail de fase, hechizos/quests ni persistencia/ciclo de vida.

### B4, noveno slice ejecutado: `SessionTransport` (2026-09-25)

La familia `transport_and_physical_connections` (4 campos) pasa al sub-estado nombrado
`SessionTransport`, alcanzado por un unico campo `transport`: el kernel de transporte de
`wow-session` (#297), la direccion remota fisica, la clave de sesion y el handle compartido del
`SessionManager`. Dos de los cuatro se inicializaban en forma abreviada (`connection`, `session_key`) y
siguen abreviados dentro del literal anidado; `session_key` se estrecha de `pub` a `pub(crate)` porque
la pasada de workspace demuestra que ningun otro crate lo leia. Census: 174 -> 171 campos de
produccion. Coste: +29 produccion/+1 test.

Sexto patron aprendido (y bug real de mi propio script, corregido): **el nombre de un campo puede
aparecer antes en el fichero como parametro de la funcion constructora**. El recogedor de
inicializadores tomo `session_key: Vec<u8>` de la firma de `create_session` en vez del `session_key,`
del literal `Self {`. Ademas, `connection` no tiene forma `campo: expr` sino solo la abreviada. Reglas:
soportar ambas formas y **buscar solo dentro del literal del constructor** (`Self {`), y validar TODO
antes de escribir: la primera version escribio `state.rs` y aborto en `construction.rs`, dejando el
arbol a medio mover (revertido con `git checkout -- crates/`). El script generalizado queda en
`/tmp/b4gen2.py` con esas dos reglas; en este slice repunto 23 lineas en la lib, 9 en `--tests` y 0 en
el workspace.

### B4: que familias siguen con sub-estado y cuales no (decision, 2026-09-25)

Leidos los `target_owner` del ledger, el resto de familias de B4 se parte en dos grupos y no conviene
tratarlas igual:

- **Sub-estado correcto** (el dueno final es la propia sesion de aplicacion, asi que nombrar el estado
  es el paso intermedio): `transport_and_physical_connections` (hecho),
  `packet_admission_dispatch` (11 campos, "private wow_world::session admission/dispatch adapters") y la
  parte de identidad de `session_identity_account_and_realm_policy` (22 campos, "cohesive application
  Session identity plus typed immutable realm/account policy supplied by composition").
- **Sub-estado no es el paso** (el dueno final es composicion u otro crate, y anidar solo maquilla el
  localizador de servicios que el ledger quiere retirar): `immutable_catalogs_configuration_and_services`
  (77 campos; su retiro exige *capability-specific immutable views* desde el bootstrap de world-server,
  no un `catalogs` dentro de `WorldSession`), `map_runtime_creature_gameobject_and_visibility` (25; el
  trabajo es retirar el puente legacy hacia `wow-map`/`wow-entities`) y
  `player_inventory_loot_and_economy` (16; el trabajo es que el dueno sea `wow-entities`/`wow-loot`). Las
  familias de un solo campo (`player_identity_login_bootstrap`, `session_selected_player_binding`) y las
  de fixtures (`test_only_fixtures` con 302, `player_identity_test_fixtures` con 5) esperan a la
  migracion de tests a un `Player` canonico, que es B3.

Consecuencia para la ola: tras cerrar `packet_admission_dispatch` y la identidad, lo que queda de B4 no
se resuelve moviendo campos. La siguiente palanca real es B3 (migrar los tests de dominio y retirar las
302 observaciones de fixture) y despues B5/B6 (adaptadores y `map_manager` -> `wow-map`), que es donde
el LOC del agregado empieza a bajar de verdad.

### B4, decimo slice ejecutado: `SessionAdmissionState` (2026-09-25)

Todo el mecanismo de admision y despacho de paquetes (tabla de opcodes, throttle y spoof-ban, cola de
paquetes pendientes, fences de timeout de socket y de autoridad de fase) pasa al sub-estado nombrado
`SessionAdmissionState`, alcanzado por un unico campo `admission`. `state: SessionState` **se queda** en
`WorldSession`: es un unico valor con 919 accesos, donde un prefijo de contenedor no agruparia nada.
Census: 171 -> 162 campos de produccion. Coste: +21 produccion/+36 test (el crecimiento de test es el
mismo prefijo en las suites de admision, throttling y spoof).

Septimo patron aprendido (bug de script otra vez): **los tipos genericos llevan comas dentro de los
angulos**. El capturador equilibraba `()[]{}` pero no `<>`, asi que `HashMap<ClientOpcodes, &'static
PacketHandle>` se trunco en su primera coma y `state.rs` dejo de parsear; revertido y corregido contando
angulos solo cuando `<` sigue a un identificador (para no confundir `->` ni comparaciones). El script
generalizado hace ya las tres cosas: validar antes de escribir, buscar inicializadores solo dentro del
literal `Self {` y equilibrar parentesis, corchetes, llaves y angulos.

### B4, undecimo slice ejecutado: `SessionRealmPolicy` + `SessionAccountState` (2026-09-25)

La familia `session_identity_account_and_realm_policy` (22 campos) se parte en dos sub-estados
cohesivos: `SessionRealmPolicy` (region, battlegroup, tabla y secreto de nombres de reino, tope de
expansion del servidor, presupuesto horario de instancias y los dos switches de instance-ignore) y
`SessionAccountState` (id de Battle.net, aristas de recruit-a-friend, personajes legitimos, low guid
reciente y expiracion de mute), tras los campos `realm_policy` y `account_state`. Census: 162 -> **150**
campos de produccion. Coste: +39 produccion, 0 test (barato: solo 14 campos y ~60 accesos).

Los **ocho escalares de identidad que otros crates leen** (`account_id`, `expansion`, `locale`, `build`,
`security`, `account_name`, `account_expansion`, `realm_id`) se quedan en `WorldSession`: declararlos
`pub` los lee composicion y bnet, y su destino segun el ledger es "cohesive application Session identity
plus typed immutable realm/account policy supplied by composition", no un contenedor mas. Ese slice
necesita decidir si la identidad es un agregado publico o un valor tipado que llega de composicion, y
por eso va aparte y despues.

Balance acumulado de B4 (produccion): **221 -> 150 campos** en once slices. El LOC del agregado sube
(~+640 lineas registradas como deuda transitoria) y baja cuando los sub-estados salgan del arbol.

### B4, duodecimo slice ejecutado: `SessionSharedFlags` (2026-09-25)

Los dos `Arc<AtomicBool>` compartidos (`advanced_combat_logging_enabled_like_cpp`,
`visibility_refresh_pending_like_cpp`) pasan al sub-estado nombrado `SessionSharedFlags`, alcanzado por
el campo `flags`: son el mismo tipo de valor (un booleano que la sesion publica a sus tareas de
publicacion), que es lo que los hace cohesionados y no un cajon. Census: 150 -> **149** campos de
produccion; coste +19 produccion/+2 test. Balance B4: **221 -> 149** en doce slices.

### B3: hallazgo con datos -- los tests de dominio son suites de integracion de la app (2026-09-25)

Medidos los cuatro modulos que B3 nombra antes de mover nada: `loot_tests` (15 610 lineas, 29
ficheros), `quest_tests` (10 079, 18), `character_tests` (13 146, 29) y `group_tests` (5 830, 14) =
**44 665 lineas y 90 ficheros**, con solo 92 referencias a `WorldSession` y 290 a crates de dominio.
El desglose parecia prometedor (70 de 90 ficheros sin `WorldSession`), pero al abrir uno se ve el
patron real: `use super::*;`, fixtures del padre y llamadas del tipo
`session.process_represented_session_commands_like_cpp()`, `session.session_command_tx()`,
`SessionCommand::...`, `crate::conditions::...`. **Son suites de integracion de la aplicacion**, no
tests de reglas de dominio: importan constantes de `wow_loot`, pero ejercitan el adaptador y el
runtime de la sesion. No pueden cruzar a `wow-loot`/`wow-social`/`wow-entities` porque la direccion de
dependencias lo prohibe (los crates de dominio no pueden depender de `wow-world`).

B3 queda reformulado, con lo que si es accionable: (i) cada crate de dominio debe tener sus propias
pruebas de regla (auditar cobertura, no mover); (ii) las suites de app son candidatas a **target de
integracion** (`crates/wow-world/tests/`) consumiendo una feature `test-fixtures`, lo que saca ~44k
lineas de la masa de test de la lib y es un trabajo por modulo con re-exportacion de fixtures; (iii)
los **306 campos de fixture** de `WorldSession` se retiran fichero a fichero cuando cada test construye
un `Player` canonico, que es la parte de B3 con ganancia directa de superficie.

### B7: la limpieza de avisos esta bloqueada por el ratchet de lineas de test (2026-09-25)

Medido antes de tocar: `cargo check -p wow-world` reporta **167 imports sin usar** en 53 ficheros (172
en la lib, 224 con `--tests`). Intente retirarlos y la compilacion de tests demostro que **todos** los
avisos son codigo vivo para los modulos `#[cfg(test)]` del mismo crate: quitar los 151 imports privados
rompe 206 compilaciones y quitar los 16 `pub(crate) use` (re-exports) rompe 301. La correccion honesta
es `#[cfg(test)]` en cada uno, pero eso **mueve lineas de produccion a lineas de test**, y el ratchet
de hotspot limita las tres cifras (produccion, test y total): no se puede. Conclusion: B7 no se
desbloquea hasta que la migracion de B3 retire los consumidores de test, o hasta que exista una
decision explicita que permita esa reclasificacion. Queda registrado, no forzado.

### B5: correccion de medida y costura real (2026-09-25)

Mi primera medida de "masa de test" en los ficheros grandes de handlers era un **artefacto**: conte
llaves desde cada `#[cfg(test)]` sin parser y conclui que `handlers/loot/mod.rs` tenia un `mod tests` de
945 lineas. No existe tal modulo: lo que hay son **items `#[cfg(test)]` sueltos intercalados** con el
codigo de produccion (imports, helpers y funciones gated una a una), el mismo patron que bloquea B7. La
costura "extrae el modulo de test" **no existe**, y el script no movio nada (arbol limpio, tres builds
verdes). B5 necesita diseno por fichero: decidir que item gated es fixture y debe vivir en un modulo de
test, y en que orden, sabiendo que cada extraccion que reclasifique lineas de produccion a test choca
con el ratchet de hotspot. Se hara junto con B3.

### B7, primer slice ejecutado: techos fisicos obsoletos endurecidos (2026-09-25)

Medido antes de tocar: **31 filas** de `physical-file-policy.json` tenian el techo por encima del
tamano vivo, es decir, el ratchet **no vigilaba** esos ficheros (entre ellas
`world-server/src/spawn_store_loader.rs` 3427 -> 730 y `handlers/character/items.rs` 3904 -> 793, de
particiones anteriores que nunca se apretaron). Se endurecieron las 31 a la cifra observada, con nota
de revision en cada `split`: **51 801 lineas de holgura retiradas** del ratchet, sin tocar una linea de
codigo. `check_architecture.py check`, `self-test` y `test_physical_files.py` verdes.

### B6: analisis medido y contrato necesario (2026-09-25)

`crates/wow-world/src/map_manager/` son **6 721 lineas en 11 ficheros**, con solo dos menciones a tipos
de app (`WorldSession`) y ambas en comentarios de documentacion: es dominio puro y el mejor candidato a
salir del arbol de la aplicacion. Pero **no es un `git mv`**: `wow-map` es `domain-runtime` y la politica
solo le permite depender de `foundation` y `domain-runtime`, mientras que `map_manager` usa dos crates
`adapter-platform`. Las aristas que el movimiento crearia, comprobadas contra
`dependency-policy.json`:

- `domain-runtime -> adapter-platform` via **`wow-packet`** (5 usos, los dos ficheros de abajo);
- `domain-runtime -> adapter-platform` via **`wow-recastdetour`** (1 uso).

Es exactamente el error que ya se revirtio una vez con `phasing -> wow-map`, asi que B6 empieza por el
**contrato**, no por el movimiento. El acoplamiento esta confinado y medido:

| fichero | lineas | usos | que es |
|---|---:|---:|---|
| `map_manager/mod.rs` | 251 | 2 | `use wow_packet::packets::update::CreatureCreateData` y los tipos de `wow_recastdetour` del pathfinder |
| `map_manager/runtime/manager.rs` | 532 | 4 | `ServerPacket` y tres literales `Set{Ai,Movement,Melee}AnimKit` |

Contrato en tres piezas, en este orden:

1. **Anim kits (la mas limpia)**: el manager deja de construir paquetes y emite una intencion
   (`CreatureAnimKitUpdateLikeCpp { guid, slot, kit_id }`) que el adaptador de `wow-world` codifica y
   envia. Es la regla del ledger ya aplicada en otras familias: el dominio devuelve la intencion, la app
   la aplica. Elimina 4 de los 6 usos y no necesita porta nueva, solo un tipo de intencion.
2. **`CreatureCreateData`**: la proyeccion que lo usa deja de nombrar el tipo de wire; el contrato es una
   proyeccion de dominio (o un tipo generico) y el adaptador compone el `CreatureCreateData`.
3. **`wow_recastdetour`**: el pathfinder necesita los tipos de detour. Opciones: (a) porta en `wow-map`
   con implementacion en un crate adaptador, (b) dejar el pathfinder en `wow-world` y mover solo el
   resto. La decision depende de cuanto del manager lo use (hoy: un `use`), asi que se resuelve al
   implementar 1 y 2.

### B6, pieza 1 del contrato ejecutada: codificador inyectado (2026-09-25)

`MapManager::set_creature_anim_kit_id_like_cpp` ya no construye paquetes: recibe un puerto por
parametro, `encode_anim_kit: impl Fn(CreatureAnimKitSlotLikeCpp, ObjectGuid, u16) -> Vec<u8>`, igual que
ya recibia `anim_kit_exists: impl Fn(u16) -> bool`. El manager muta el estado y devuelve su
`RuntimeEvent` con los bytes que el adaptador codifique; **`runtime/manager.rs` deja de nombrar
`wow_packet`** (0 usos, eran 4). Los tres unicos llamadores eran de test, asi que el codificador vive
ahora en `map_manager_tests/creature_5.rs` como helper y las aserciones de opcode no cambian. Queda **1
uso de `wow_packet`** en `map_manager/` (`CreatureCreateData` en `mod.rs`) y el de `wow_recastdetour`.
Evidencia: `cargo check` lib y tests 0 errores, 3901 tests pasan, `check_architecture.py check` y
`self-test` verdes. Coste: +11 lineas.

### B6, pieza 2 dimensionada: `create_data` es un puente de 56 usos (2026-09-25)

Antes de tocar `CreatureCreateData` se midio su radio: **56 usos de `.create_data`** repartidos en 10+
ficheros (`map_manager/pending_respawn.rs` 9, `map_manager_tests/persistence.rs` 9, `creature_1.rs` 7,
`session/spell_effects/ticks.rs` 4, `runtime/creature.rs` 4, `runtime/manager.rs` 3, ...), mas los
puntos que *construyen* el tipo (`session/world_entities/creature_registry.rs` 4, fixtures y tests de
handlers). No es una arista que se inyecte como el codificador de anim-kit: el contrato exige

1. una **proyeccion de dominio** (`CreatureCreateProjectionLikeCpp` o equivalente) con los mismos
   datos que hoy lleva el tipo de wire, declarada en un crate `domain-runtime`;
2. que `WorldCreature.create_data` pase a ser esa proyeccion;
3. que el **adaptador** de `wow-world` componga `CreatureCreateData` al construir el paquete
   (`session/world_entities/creature_registry.rs` y los fixtures que hoy lo construyen a mano).

Es un slice propio, no una pieza suelta: se hara con el mismo metodo probado (mover el tipo, repuntar
guiado por el compilador, regenerar census/ledger con delta revisado) y despues de la pieza 3, porque el
movimiento de `map_manager` a un crate `domain-runtime` necesita las dos. Queda dimensionado y no
iniciado a medias.

### B6, pieza 3 resuelta por reclasificacion: `wow-recastdetour` es `foundation` (2026-09-25)

La pieza 3 no necesitaba porta: `wow-recastdetour` estaba mal clasificado como `adapter-platform`. Es un
**port vendido de terceros** (el propio estandar lo exime de presupuestos de tamano y de la regla de
nombres `_like_cpp`) que **no depende de ningun crate del workspace** — solo de `bitflags` y `thiserror`,
mas `cc` en build-dependencies para compilar el C++ vendido. Eso es una libreria de base, no un adaptador
de plataforma, y la politica ya permite `domain-runtime -> foundation`.

Cambios: `wow-recastdetour` pasa a `foundation`; se declara su superficie externa
(`normal: [bitflags, thiserror]`, `build: [cc]`), y se **retira la excepcion obsoleta**
`wow-world -> wow-recastdetour` que ya no hace falta. El recuento pasa de 17 a **16 excepciones de
workspace** y de 60 a 63 dependencias externas vigiladas. `check_architecture.py check` y `self-test`
verdes. Efecto sobre B6: de las dos aristas prohibidas que bloqueaban el movimiento de `map_manager`,
**queda una** — `wow-packet` via `create_data` (la pieza 2, ya dimensionada en 56 usos) — asi que el
movimiento de las 6 721 lineas depende ya solo de esa proyeccion.

Criterio de cierre de B6: `map_manager` vive en `wow-map` (o en un crate `domain-runtime` propio),
`wow-world` conserva solo el adaptador, y `check-deps` no necesita ninguna excepcion nueva.

### B7, segundo slice ejecutado: superficie de dependencias recortada (2026-09-25)

Auditoria manual (no hay `cargo-machete` en el host) de dependencias declaradas con **cero referencias**
en el codigo del crate, con el compilador como juez: **28 declaraciones retiradas** de
`[dependencies]` y el workspace compila limpio a la primera (`--workspace --all-targets`). Incluye
`serde`/`thiserror`/`num-derive`/`strum` en crates donde ya no se derivaban, `parking_lot` y
`smallvec`/`bumpalo` en `wow-map`, `dashmap`/`hyper-util` en `bnet-server`, `bytes`/`prost-types` en
`wow-proto`, `wow-constants`/`wow-core`/`wow-math`/`wow-config`/`wow-logging`/`wow-crypto` en quien ya
no los usaba.

Politica regenerada con el delta exacto que el checker exigio: **14 entradas obsoletas** de superficie
externa retiradas y **1 excepcion obsoleta** (`wow-world -> wow-logging`). Recuentos:
**aristas de workspace 112 -> 102**, **dependencias externas vigiladas 63 -> 49**, **excepciones 16 ->
15**. `check_architecture.py check` y `self-test` verdes; la suite de `wow-world` (3901 tests) pasa.

### B7, tercer slice ejecutado: `[workspace.dependencies]` sin uso (2026-09-25)

De las 9 entradas de `[workspace.dependencies]` que ningun miembro referencia, se retiran las **6
externas** (`prost-types`, `strum`, `bumpalo`, `hyper-util`, `reqwest`, `cfg-if`) y se **conservan las
3 internas** (`wow-math`, `wow-util-collections`, `wow-spell`) por decision explicita: `wow-spell` es el
paquete reservado documentado y las otras dos son miembros con consumidores planificados; el coste de
mantenerlas es una linea y su retirada seria churn sin ganancia. `dev-dependencies` y
`build-dependencies`: **0 candidatos** (ninguna declaracion sin referencia). `cargo check --workspace
--all-targets` 0 errores; `check_architecture.py check` y `self-test` verdes.

### Puerta de la ola medida en el tip de 15 rondas (2026-09-26)

`./tools/validation-v2 final --base origin/3.4.3 --architecture --timings` sobre `0f5ccdd5`:

- **En frio: 1011,98 s y `exit_code 143`** (terminada por el limite de tiempo durante la compilacion).
  El manifiesto queda `failed` con el tiempo agotado, no por un fallo de validacion: el coste lo domina
  la recompilacion que provocan los cambios de `Cargo.lock` de los slices de dependencias.
- **En caliente, mismo comando sin trocear: 122 s y `exit 0`**, manifiesto
  `20260926T233514.027935Z-3621532-final.json` con estado `passed`, y la suite completa de `wow-world`
  (3901 tests) dentro de la misma campana.

Leccion operativa: con el presupuesto de 600 s, la puerta hay que correrla **despues** de un build del
mismo tip; una corrida en frio de esta rama no cabe en el limite (y el ejecutor mata los trabajos de
fondo alrededor de los 1000 s, asi que una campana en frio no puede completarse en un solo trabajo).
No se trocea la campana para declararla verde: la corrida verde es el mismo comando entero, en caliente.

### B4: un sub-estado de un solo campo no reduce nada (revertido, con regla) (2026-09-26)

Intente cerrar el ultimo item barato de B4 moviendo `pending_bind` a un `SessionBindState`. El
movimiento funciono (14 accesos repuntados, lib/tests/workspace limpios) pero al medir el censo se vio
el error de juicio: **un sub-estado de un campo deja el recuento igual** (uno sale, uno entra) y solo
anade indireccion — el mismo olor que el estandar prohibe ("ninguna crate/trait por helper... solo para
reubicar codigo"). Revertido con `git checkout -- crates/`, arbol limpio y lib a 0 errores.

Regla que queda: **un contenedor solo se justifica cuando agrupa dos o mas campos** de la misma
responsabilidad. Los campos solos (`pending_bind`, `player_guid`, `player_identity_bootstrap_like_cpp`)
no se anidan: esperan a que su dueno real se los lleve (Player/InstanceMap, el binding de la sesion,
`wow-entities::Player`). Con esto **la lista barata de B4 esta agotada**: lo que queda exige la
proyeccion de `create_data` (B6) o cambio de dueno, no otro sub-estado.

### B7, cuarto/quinto slice: defectos y hallazgos de avisos (2026-09-26)

Arreglado: un **`#[test]` duplicado** en `wow-packet/src/packets/combat.rs` (el compilador avisaba
`duplicated attribute`; ahora el test se registra una sola vez). `wow-packet` sigue con 754 tests
verdes.

Hallazgos registrados, **no silenciados**:

- **Cluster de variables sin usar en `session/legacy_runtime/creature_movement_tick.rs`** (11+ avisos:
  `filter_context`, `owner_capabilities`, `previous_poly_refs`, `avoided`). Prefijarlas con `_`
  apagaria el aviso sin decidir nada; que el runtime legacy reciba contextos de pathfinding y no los use
  es candidato a **defecto o a codigo transitorio deliberado**, y se investiga antes de tocar. Es el
  siguiente trabajo real de B7 en wow-world.
- **Falso positivo de `mut` entre cfg**: `handlers/loot/money.rs:48` avisa "variable does not need to be
  mutable" en la pasada de lib y sin embargo la pasada de test **si** asigna dos veces; quitar el `mut`
  rompe el build de test (`E0384`). Revertido: el aviso no es accionable sin reestructurar el flujo, y
  silenciarlo o romper el test serian peores. Queda anotado como advertencia de metodo: los avisos de
  `--all-targets` pueden venir de una sola de las dos configuraciones.

### B7 en wow-world: el bloqueo es global, no de los owners auditados (2026-09-26)

Comprobada la hipotesis optimista de que fuera de los cuatro agregados auditados de `wow-world`
(`session/`, `handlers/{character,loot,quest}/`) si hubiera margen para retirar los imports sin usar
(39 avisos en 20+ ficheros). **No lo hay**: la pasada de lib queda limpia, pero la de test falla con 8
errores que muestran el mismo patron —
`handlers/trainer/tests/failures.rs` necesita `PacketHandlerEntry`, `ClientOpcodes`, `SessionStatus` y
`PacketProcessing` que `handlers/trainer.rs` re-exporta; `spell_acquisition/tests/planner_application.rs`
necesita `SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP`; `handlers/spell/state.rs` necesita
`ItemFieldFlags` y `ItemUpdateState`. Es decir: **los imports "sin usar" de wow-world son, en todo el
crate, la superficie que consumen los modulos `tests/` del propio modulo**, no deuda muerta.

Conclusion registrada: en `wow-world` la limpieza de avisos de imports **no se desbloquea fichero a
fichero**; necesita la migracion de tests de B3 (mover cada `tests/` a un target de integracion tras la
feature `test-fixtures`) o una decision explicita que permita reclasificar lineas de produccion a test.
El arbol se revirtio (`git checkout -- crates/`) y queda limpio, con la lib a 0 errores.

### B7 en wow-world: la superficie de imports del modulo es compartida (conclusion, 2026-09-26)

Segundo intento, mas preciso: en vez de borrar los imports "sin usar", moverlos al modulo `tests/` del
propio modulo (donde la pasada de test los necesitaba). Resultado medido: **8 movimientos, y la lib cae
con 23 errores** — `spell_acquisition/adapter.rs` necesita `SpellAcquisitionEffectLikeCpp`,
`SpellAcquisitionCatalogLikeCpp`, `SpellChainStoreLikeCpp`, `SpellRequiredStoreLikeCpp` y
`SKILL_RIDING_LIKE_CPP` que importa `spell_acquisition/mod.rs`; `handlers/chat/ops_1.rs` necesita
`UnitState` que importa `handlers/chat.rs`. Revertido entero (automatico), arbol limpio.

Conclusion definitiva, con las dos clases de error ya identificadas:

- los imports que el compilador llama "sin usar" en los ficheros raiz de modulo de `wow-world` son la
  **superficie compartida** que consumen (a) los submodulos de produccion del mismo modulo y (b) su
  modulo `tests/` via `use super::*`;
- por tanto **no son deuda muerta y no se retiran**: ni borrandolos (rompe tests) ni moviendolos a
  `tests/` (rompe produccion). Los 167 avisos de `wow-world` quedan **fuera del alcance de B7** por
  decision, no por pereza: tocarlos exige primero cambiar como se organizan los imports de cada modulo
  (por ejemplo, que cada hijo importe lo suyo en vez de heredarlo del padre), y eso es un refactor de
  imports por modulo, no una limpieza.

Lo que si queda hecho de B7: 31 techos fisicos endurecidos, 34 declaraciones de dependencia y 10
imports retirados **fuera** de wow-world, 1 defecto arreglado y 3 bloqueos documentados con evidencia.

### B7, piloto por modulo: `handlers/chat` limpio (2026-09-26)

La conclusion anterior era demasiado amplia y el piloto la corrige. Clave medida: **un nombre que usa
un modulo hijo NO genera aviso** (por eso `UnitState`, usado por `handlers/chat/ops_1.rs`, nunca se
marco). Por tanto los avisos que quedan **si** son muertos dentro de todo el subarbol del modulo, y se
retiran con la operacion quirurgica correcta: borrar **solo los nombres avisados** de su `use`, nunca la
sentencia entera (que puede llevar nombres vivos al lado).

Aplicado a `handlers/chat.rs`: retirados `ClientOpcodes` (de `use wow_constants::{ClientOpcodes,
UnitState};`, que queda `use wow_constants::UnitState;`), y las sentencias completas
`use wow_handler::{PacketProcessing, SessionStatus};` y
`use crate::session::registry::PacketHandlerEntry;`, restos del traslado de registraciones de la ola A.
Resultado: **4 nombres muertos menos, 0 avisos en ese fichero**, lib/tests/workspace a 0 errores.

Regla afinada para el resto de wow-world: (1) borrar solo los nombres avisados; (2) si el unico usuario
es el modulo `tests/`, envolver el import en `#[cfg(test)]` en lugar de borrarlo — permitido en los
modulos **no auditados** por el ratchet, que es donde quedan avisos; (3) nunca mover sentencias enteras
al modulo `tests/`, porque arrastran los nombres vivos que comparten.

### B7: lotes de imports, leccion de proceso y veredictos por fichero (2026-09-26)

Intente extender el piloto a los 19 avisos restantes en 15 ficheros no auditados con un solo lote
automatizado. **Fallo por proceso, no por diseno**, y conviene dejarlo escrito:

1. el script restauraba el fichero que fallaba y **seguia** al siguiente modo de build sin volver a
   verificar, asi que termino con el arbol a medio limpiar;
2. mi filtro de ficheros fallidos (`grep -oE '^crates/[^:]+'`) capturo tambien **lineas de aviso**, no
   solo errores, y restauro de mas. Todo lo anterior estaba commiteado, asi que la recuperacion fue
   `git checkout -- .` y el tip verde quedo intacto (`ccd3d205`, workspace a 0 errores).

Reglas de proceso para el proximo intento: filtrar **solo** lineas con `: error`, restaurar por fichero
y **volver a verificar tras cada restauracion**, y commitear el resultado parcial en cuanto pase.

Veredictos medidos (ficheros cuyos `tests/` necesitan los nombres, y que por tanto piden `#[cfg(test)]`
en lugar de borrado, permitido al no ser owners auditados): `handlers/trainer.rs` (`PacketHandlerEntry`,
`ClientOpcodes`, `SessionStatus`, `PacketProcessing`), `spell_acquisition/mod.rs`
(`SpellLearnSkillNodeLikeCpp`, `SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP`) y
`handlers/spell.rs` (`ItemFieldFlags`, `ItemUpdateState`). Los demas ficheros del lote no fallaron: son
candidatos directos a borrado con la operacion quirurgica ya probada en `handlers/chat.rs`.

### B7: piloto por modulo completado en los tres veredictos (2026-09-26)

Con las reglas de proceso corregidas, el patron "cada hijo importa lo que usa" se aplico **fichero a
fichero con verificacion** en los tres veredictos, y los tres quedaron verdes:

- `handlers/spell.rs` -> `spell/state.rs` (produccion) y `spell/tests/` (tests) importan
  `ItemFieldFlags` e `ItemUpdateState`.
- `handlers/trainer.rs` -> `trainer/tests/failures.rs` importa `PacketHandlerEntry`, `ClientOpcodes`,
  `PacketProcessing` y `SessionStatus`.
- `spell_acquisition/mod.rs` -> `spell_acquisition/tests/planner_application.rs` importa
  `SpellAcquisitionMiscLikeCpp`, `SpellLearnSkillNodeLikeCpp` y
  `SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP`.

Resultado: 9 avisos menos, 0 errores en lib/tests/workspace en cada paso, y los 3901 tests de
`wow-world` verdes. Quedan **31 avisos** en modulos no auditados (de 39 al empezar): el resto son del
mismo tipo y se limpian repitiendo el patron, ahora que esta probado tres veces y con reglas de proceso
escritas.

### B7: lote mecanico completado en los modulos no auditados (2026-09-26)

Con el bucle corregido (borrar solo los nombres avisados; restaurar **unicamente** los ficheros que el
build de test nombra en lineas `: error`; **re-verificar tras cada restauracion**) el lote edito **29
sentencias en 25 ficheros** en dos ciclos: dos ficheros conservaban nombres que sus propios tests
consumen y se restauraron intactos. Resultado: lib/tests/workspace limpios, 3901 tests verdes,
`check_architecture.py check` PASS, **23 ficheros con 7 inserciones y 32 borrados** (neto -25 lineas) y
los avisos de imports en modulos no auditados bajan de **29 a 5**.

B7 queda practicamente cerrado: los avisos restantes de `wow-world` estan en los cuatro agregados
auditados (donde el ratchet impide reclasificar lineas) y quedan documentados como fuera de alcance.

### La puerta de la ola caza una regresion real de formato (2026-09-26)

Corrida de la puerta en el tip de 27 rondas: **fallo con `exit 1` en `cargo fmt --check`**. Causa real:
el slice B7 que retiro imports **fuera** de wow-world edito ficheros de `wow-database`, `wow-entities`,
`wow-spell-acquisition` y otros, pero el script solo formateaba `wow-world`; quedaron lineas en blanco
dobles y una llave sin formatear. Arreglado con `cargo fmt --all` (10 ficheros, 3 inserciones / 18
borrados, sin cambios de codigo) en `6e8b2e11`.

Dos lecciones operativas:

1. **Cada slice que edite Rust debe formatear los paquetes que toca**, no solo `wow-world`; el linter de
   la puerta es la red de seguridad, pero llega al final.
2. **Un `cargo fmt --all` invalida la cache de varios crates**: la puerta siguiente tardo 900 s (tope
   agotado) porque reconstruia todo el grafo afectado, y la siguiente ya en caliente **paso en 195,5 s**
   (`exit 0`, manifiesto `passed`). El presupuesto de 600 s se cumple **despues** de que el rebuild de un
   cambio transversal haya ocurrido; un cambio de formato global es un cambio transversal.

Estado de la puerta: **verde en el tip actual**, con la suite completa de `wow-world` dentro de la
misma campana.

### Incidente de rama: base obsoleta y correccion (2026-09-27)

La primera version de la rama de continuacion (\`584-wave-b2-map-manager\`) se creo desde la \`3.4.3\`
**local**, que estaba en una linea antigua y divergente (punta \`d35f385a\`, 19-sep, de otro autor) sin
la ola B ni los commits ajenos de \`.codex/\`/\`.agents/\`. El diff contra \`origin/3.4.3\` habria
**revertido** trabajo ajeno y toda la ola B.

Detectado al intentar actualizar este mismo programa: el fichero no existia en la rama. Correccion:
rama recreada con \`git checkout -B 584-wave-b2-map-manager origin/3.4.3\`, retiradas de B7 reaplicadas
sobre la base real (solo quedaba una), puntero local \`3.4.3\` forzado a \`origin/3.4.3\` y fuerza de
push con \`--force-with-lease\`.

Regla que queda: **antes de crear una rama de ola, verificar que la base es la de \`origin\`**
(\`git fetch origin && git log --oneline origin/3.4.3 -1\`), nunca la copia local, y comprobar que el
programa existe en la base (\`git cat-file -e <base>:docs/architecture/workspace-structure-programme.md\`).

### B6 `create_data`: plano exacto para el siguiente intento (2026-09-27)

Dos intentos, ambos revertidos sin dejar el arbol sucio, dejan el trabajo medido al detalle:

- el tipo vive en `crates/wow-packet/src/packets/update/unit/create.rs` precedido de **comentario doc y
  `#[derive(Debug, Clone)]`** (hay que moverlos con el struct, no solo el `pub struct`);
- su `impl` tiene **tres** metodos: `pub fn write_values_create`, y los ayudantes privados
  `fn write_object_data` y `fn write_unit_data` (un trait no puede declarar solo uno y dejar los otros
  en el mismo `impl`; la salida limpia es declarar `write_values_create` en el trait y convertir los dos
  ayudantes en **funciones libres** `write_object_data_like_cpp(data, buf)`, con dos puntos de llamada
  internos que pasan `self`);
- `wow-packet` **no** depende hoy de `wow-entities`: el movimiento exige la arista nueva
  (`adapter-platform -> domain-runtime`, permitida) y declararla en su `Cargo.toml`;
- el trait debe importarse donde se llama al metodo; el bucle guiado por compilador (errores `E0599` con
  `write_values_create`) los localiza, y el resto de llamadas son de **otros** tipos que tambien tienen
  ese metodo, asi que no se tocan.

Error de proceso propio, para no repetirlo: el script de movimiento invocaba `cargo` antes de fijar
`PATH`, aborto a mitad de edicion y dejo cuatro ficheros tocados (revertidos con `git checkout -- .`).
Regla: en cualquier script que edite y compile, **exportar `PATH`/`CARGO_TARGET_DIR` en el mismo shell
antes de la primera edicion**, o validar la fase de recoleccion sin tocar nada.

### B6 `create_data`: cuarto intento y por que se para aqui (2026-09-27)

Con el plano anterior el movimiento llego a compilar casi entero y fallo en dos detalles que cierran el
diagnostico: (1) los dos ayudantes privados usan **`self.` en su cuerpo** (mas de siete puntos), asi que
convertirlos en funciones libres exige renombrar `self` -> `data` en ~150 lineas cada uno; (2) el
`impl` original tenia un metodo con **`pub`**, que no es legal dentro de un `impl Trait for`. Cuatro
intentos, todos revertidos con el arbol limpio y verde.

Conclusion registrada: esta migracion **no es scripting mecanico**, es un refactor con atencion. El
camino mas corto y seguro, para una sesion con contexto fresco, es:

1. declarar el trait con **`write_values_create` + los dos ayudantes** (evita el renombrado de `self`),
   aceptando que la superficie del trait crezca; o
2. hacer el renombrado `self` -> `data` con revision, en su propio commit de movimiento.

Se prefiere (1) por seguridad; el ensanchamiento es de un trait interno del crate de wire y queda
documentado. Con eso el struct viaja a `wow-entities`, `wow-packet` gana la arista `wow-entities` y la
ultima arista prohibida `domain-runtime -> adapter-platform` desaparece, que es lo que desbloquea mover
`map_manager` (6 721 lineas) a un crate `domain-runtime`.

### B6 `map_manager`: primer intento de trasvase y obstaculos exactos (2026-09-27)

Con las dos aristas prohibidas ya eliminadas, movi `crates/wow-world/src/map_manager/` a
`wow-map` con re-export desde `wow-world`. El intento fallo en tres puntos concretos y se revirtio
solo, dejando el arbol a 0 errores:

1. **Autoreferencias por nombre de crate**: los ficheros movidos usan `wow_map::...` (correcto desde
   `wow-world`, invalido dentro del propio crate). Son **2** referencias; se reescriben a `crate::`.
2. **El montaje de tests**: `map_manager/mod.rs:226-227` hace
   `#[path = "../map_manager_tests.rs"] mod tests;`, apuntando a un fichero que se queda en `wow-world`.
   La solucion limpia es quitar ese `mod tests;` del modulo movido y montar los tests en `wow-world`
   (`#[cfg(test)] mod map_manager_tests;`), porque las suites usan fixtures de la aplicacion.
3. **Dependencias**: `wow-map` necesita declarar `wow-constants`, `wow-data`, `wow-movement`,
   `wow-persistence`, `wow-recastdetour` (todas `foundation`/`domain-runtime`, permitidas) y
   **`tracing`**, que es externa: exige anadir `tracing` a la superficie externa de `wow-map` en
   `dependency-policy.json`. Ademas las filas fisicas de los ficheros movidos cambian de ruta y el
   chequeo de tamano pedira el repunte (no un techo nuevo: solo el mismo fichero en su ruta nueva).

El resto del modulo es **autoreferencial** (`pub(in crate::map_manager)`, `crate::map_manager::...`),
asi que conserva sus rutas tal cual en el crate nuevo. Con esos tres arreglos el trasvase de las
**6 721 lineas** es mecanico.

### B6 `map_manager`: segundo intento y los dos ultimos detalles (2026-09-27)

Aplicados los tres arreglos anteriores, el intento bajo de **30 a 8 errores** y se revirtio solo otra
vez (arbol limpio). Los dos detalles que faltan quedan medidos:

1. **Resolucion de dependencias**: aunque el script anadia las seis a `[dependencies]` de `wow-map`,
   la compilacion seguia diciendo `cannot find module or crate \`wow_constants\``. Hay que **verificar el
   `Cargo.toml` resultante** (que las lineas entren en la seccion correcta y que el nombre de paquete
   coincida) antes de culpar al codigo; es lo primero a comprobar en el proximo intento.
2. **Los `pub use X::*;` del modulo movido**: `map_manager/mod.rs` reexporta sus hijos con
   `pub use grid::*;`, `pub use pathfinder::*;`, etc. En `wow-world` resuelven por el arbol del crate;
   dentro de `wow-map` hay que hacerlas explicitas como **`pub use self::grid::*;`** (y las otras
   cuatro). Son cinco lineas, mecanicas.

Con esos dos arreglos el trasvase de las **6 721 lineas** cierra: el resto (visibilidades
`pub(in crate::map_manager)`, rutas `crate::map_manager::...`, montaje de tests en `wow-world` y las
dependencias) ya esta resuelto o identificado.

### B6 `map_manager`: tercer intento, a cuatro errores (2026-09-27)

Con los dos arreglos (deps escritas y verificadas por impresion de la seccion, y `pub use self::X::*;`
para los hijos) el trasvase bajo a **4 errores, todos sobre `grid`**:

```
crates/wow-map/src/map_manager/mod.rs:232:15: error[E0432]: unresolved import `self::grid`: could not find `grid` in `self`
crates/wow-map/src/map_manager/mod.rs:238:5:  error[E0432]: unresolved import `grid`
```

Dato util para el ultimo intento: el fichero **si existe** (`crates/wow-world/src/map_manager/grid.rs`) y
`mod grid;` esta declarado; por tanto lo que falla es la **resolucion de las dos sentencias de import**
que nombran `grid` (una ya con `self::`, otra sin el), no la declaracion del modulo. Plan focalizado:
movido el modulo, revisar esas dos lineas de `mod.rs` a mano (probablemente una escribe
`use grid::...` y la otra `pub use grid::*;`) y resolverlas a `self::grid`/`crate::map_manager::grid`,
comprobando despues que no hay otro `grid` en el crate (`wow-map` tiene su propio `crates/wow-map/src/grid.rs`)
que pueda confundir la ruta.

El resto del trasvase (dependencias, autoreferencias, montaje de tests en `wow-world`, visibilidades y
rutas `crate::map_manager::...`) ya esta resuelto; el intento se revirtio solo y el arbol quedo a 0 errores.

### B6 `map_manager`: quinto intento, el ultimo metodo vive fuera del modulo (2026-09-27)

Repetido el trasvase con un regex de visibilidad **generico** (`pub(...) fn NOMBRE`, cualquier ambito):
otra vez **14 metodos ensanchados en 4 ficheros** y **1 sin encontrar** en el segundo ciclo. Como el
bucle solo recorre el directorio movido, la conclusion es que **ese metodo esta definido fuera de
`map_manager`** — en otro modulo de `wow-map` que implementa algo sobre un tipo movido, o sobre un tipo
que ya vivia en `wow-map`. El script se revirtio solo; arbol limpio y workspace a 0 errores
(`6671e70b`).

Tarea exacta para el ultimo intento: **imprimir el nombre** del error `E0624` que queda (el script
actual solo cuenta) y ensanchar su definicion **donde este** (buscar el nombre en todo `crates/wow-map`,
no solo en el directorio movido), y despues repuntar las filas fisicas de los ficheros movidos. Todo lo
demas del trasvase esta resuelto y verificado en los intentos anteriores.

### B6 `map_manager`: el ultimo metodo no esta en wow-map (2026-09-27)

Ampliado el ensanchado a **todo `crates/wow-map/src`** e imprimiendo los nombres, el ultimo error es
siempre **`runtime_elapsed_ms_like_cpp`** y su definicion no aparece en `wow-map`: vive en **otro crate**
(se comprueba con `grep -rn 'fn runtime_elapsed_ms_like_cpp' crates/`), probablemente una implementacion
sobre el tipo movido que quedo en `wow-world`. Tarea exacta del siguiente intento: localizarla con ese
grep, ensancharla a `pub` **en su crate** y, si esta en `wow-world`, decidir si el impl pertenece al
modulo movido (moverlo) o si el metodo debe exponerse desde `wow-map`. Despues, repuntar las filas
fisicas de los ficheros movidos.

Resumen del trasvase (6 intentos, todos revertidos limpios): 30 -> 8 -> 4 -> **1** error; resueltos el
montaje de tests con su `#[cfg(test)]` colgante, las autoreferencias `wow_map::`, los re-exports de
hijos, las seis dependencias, 14 de 15 visibilidades y cuatro ficheros ensanchados. La distancia que
queda es **un grep y una visibilidad**.

### B6 `map_manager`: el trasvase llega al workspace (ultimo estado, 2026-09-27)

El sexto intento dejo **`wow-map` y `wow-world` compilando limpios** y bajo al **workspace**: solo tres
errores, todos `E0599 no method named seed_runtime_rng_like_cpp ... for struct WorldCreature` en las
pruebas de `world-server`. El metodo existe en el modulo movido pero es **`#[cfg(test)]`**, asi que no
es visible desde las pruebas de otro crate; no es un problema de visibilidad (`E0624`) sino de **gate de
test**.

Arreglo exacto para el siguiente intento: aplicar el patron ya establecido en el proyecto
(`#[cfg(any(test, feature = "test-fixtures"))]` en la definicion y habilitar esa feature en la
dependencia de `wow-map` que usan las pruebas de `world-server`), y despues repuntar las filas fisicas
de los ficheros movidos. Es el mismo trabajo de B3 (test-fixtures) aplicado a un helper.

Resumen del trasvase (6 intentos, todos revertidos con el arbol limpio y verde): 30 -> 8 -> 4 -> 1 -> 3
(ya solo de gate de test); resueltos montaje de tests con su `#[cfg(test)]` colgante, autoreferencias,
re-exports de hijos, seis dependencias, **15 visibilidades** (incluido el `pub(crate) const fn` que
rompia el patron) y cuatro ficheros ensanchados a `pub`. La distancia que queda es **una feature**.

### B6 `map_manager`: el trasvase llega al workspace desde la base limpia (2026-09-27)

Reanudado desde la base unica `350d01bc` (rama `584-map-manager-domain`). Con el cableado de features que
faltaba — `wow-map` gana `[features] test-fixtures` y `wow-world` propaga
`test-fixtures = ["wow-map/test-fixtures"]` — el movimiento compila **`wow-map` y `wow-world` limpios** y
solo falla en el **workspace**, en `wow-world/src/map_manager_tests.rs`: la suite vivia dentro del modulo
movido y heredaba por `use super::*` siete nombres que ya no estan en su ambito.

Proveedores exactos, ya localizados:

| nombre | de donde debe importarlo el test |
|---|---|
| `PathBuf` | `std::path::PathBuf` |
| `ObjectGuid` | `wow_core::ObjectGuid` |
| `DetourPolyPath` | `wow_recastdetour::DetourPolyPath` |
| `MAP_MAGIC_LIKE_CPP`, `MAP_VERSION_MAGIC_LIKE_CPP`, `MAP_FILE_HEADER_SIZE_LIKE_CPP` | definidas `pub(super)` en `map_manager/terrain.rs`: hay que **ensancharlas a `pub`** (como el resto de visibilidades del modulo movido) e importarlas desde `wow_map::map_manager::terrain::...` |

Es decir: el trasvase de las **6 721 lineas** queda a **3 `pub(super)` -> `pub` + 4 lineas de import**.
Todo lo demas (montaje de tests con su `#[cfg(test)]` colgante, autoreferencias `wow_map::` -> `crate::`,
re-exports de hijos, seis dependencias, 15 visibilidades, el `pub(crate) const fn` y el gate
`test-fixtures`) esta resuelto y verificado.

### B6 `map_manager`: el movimiento compila; quedan dos items de politica y un sintoma (2026-09-27)

Con el cableado de features y los imports/visibilidades resueltos, el modulo movido **compila**:
`cargo check -p wow-map --features test-fixtures` da **0 errores**, y en la pasada en que la feature no
estaba activa el workspace tambien compilaba. Al reactivarla, `check_architecture` y el workspace
destapan tres cosas concretas, que son el trabajo real que queda:

1. **`wow-map -> tracing`** sin declarar: es una externa y `wow-map` tiene superficie externa explicita,
   asi que hay que anadir `tracing` a `external_dependencies.allowed["wow-map"]` (una linea de politica).
2. **`wow-map -> wow-data` es una arista prohibida** (`domain-runtime -> adapter-platform`): el modulo usa
   tipos de `wow-data`. Necesita contrato (porta/vista inmutable) o una excepcion revisada con issue,
   igual que se hizo con `wow-packet`.
3. **Sintoma a diagnosticar**: activar `wow-map/test-fixtures` **en todo el workspace**
   (`cargo check --workspace --all-targets`) produce **581 errores**, mientras que
   `-p wow-map --features test-fixtures` esta limpio. Apunta a la unificacion de features del workspace
   (la feature se propaga a crates que no la esperan), no al codigo movido.

El arbol quedo revertido y verde (workspace 0 errores) en la rama `584-map-manager-domain`, con el
movimiento y su diagnostico documentados. La distancia restante es **una linea de politica de externas**,
**una decision de contrato o excepcion para `wow-data`** y **un diagnostico de feature unification**.

### B6 `map_manager`: estado exacto del trasvase al cerrar esta sesion (2026-09-27)

Tras varias iteraciones automatizadas, el movimiento queda a **tres nombres** de compilar el workspace:
`PendingRespawn` y `Position` (tipos) y `BASE_ATTACK_TIME_LIKE_CPP` (constante) que el test
`map_manager_tests.rs` y su submodulo `combat.rs` heredaban del padre movido y que ya no encuentran en su
ambito. Todo lo demas esta resuelto y verificado en los intentos: montaje de tests con su `#[cfg(test)]`
colgante, autoreferencias `wow_map::` -> `crate::`, re-exports de hijos, seis dependencias, el gate
`test-fixtures` (con el cableado `wow-map`/`wow-world`/`world-server`), 15 visibilidades de metodos y las
tres constantes `MAP_*`, y el fichero de tests vuelve a caber en su techo (337/333 <= 339).

Ademas quedan dos items de politica ya medidos: declarar `tracing` en la superficie externa de `wow-map`,
y la decision sobre la arista `wow-map -> wow-data` (contrato o excepcion revisada).

Metodo recomendado para el siguiente intento, en este orden: (1) mover el modulo con el script y **no**
tocar el fichero de tests despues; (2) dejar que el compilador diga los nombres que faltan y anadirlos
**uno a uno** al test con su ruta real (`grep -rn "struct NAME\|const NAME" crates/wow-map crates/wow-core
crates/wow-entities`), ensanchando a `pub` lo que proceda; (3) declarar `tracing`; (4) decidir
`wow-data`; (5) `cargo fmt --all`, puerta y PR. El trasvase son 6 721 lineas y cada paso esta acotado; el
arbol queda verde y la base intacta.

### B6 `map_manager`: el metodo correcto es preparar el test ANTES de mover (2026-09-27)

Los intentos automatizados tropiezan siempre en lo mismo: `map_manager_tests.rs` (y sus submodulos)
heredan del padre movido una **superficie implicita grande** (`use super::*` les daba tipos, constantes
y helpers del modulo). Al mover, cada pasada revela 2-4 nombres nuevos — `PendingRespawn`, `Position`,
`Instant`, `TERRAIN_GRID_COUNT_LIKE_CPP`, `BASE_ATTACK_TIME_LIKE_CPP` — y el script revierte al no
reconocer esos errores como parte de su bucle de visibilidades.

Metodo correcto, deterministico y de una sola pasada:

1. **Antes de mover nada**, hacer autocontenido el fichero de tests *dentro de wow-world*: sustituir el
   `use super::*` implicito por los imports explicitos de todo lo que usa (el compilador los pide uno a
   uno), comprobando que sigue compilando. Es trabajo acotado y verificable, y de paso es exactamente el
   patron de B3 ("cada hijo importa lo que usa").
2. Despues, mover el modulo con el script actual (que ya resuelve montaje de tests, autoreferencias,
   re-exports, dependencias, gate `test-fixtures` y visibilidades de metodos).
3. Completar con los dos items de politica medidos: declarar `tracing` en `wow-map` y decidir la arista
   `wow-data` (contrato o excepcion).

El arbol queda verde y la base intacta; el trasvase sigue siendo el trabajo que mas valor anade (saca
6 721 lineas del arbol de la aplicacion) y esta a un paso preparatorio + el movimiento mecanico.

### B6 `map_manager`: la superficie implicita del test, medida (2026-09-27)

Ejecutada la simulacion registrada (montar `map_manager_tests.rs` en la raiz de `wow-world` y quitar el
montaje del padre, sin mover el modulo), el compilador da **de una sola pasada** la lista completa de lo
que la suite heredaba: **mas de 40 nombres** repartidos por sus submodulos — `Position`, `ObjectGuid`,
`WaypointMovementAction`, `MovementGeneratorKind`, `RuntimeMovementGeneratorType`, `ChaseTickOutcomeLikeCpp`,
`Instant`, `Duration`, `SpawnObjectType`, `PhaseShift`, `DetourPathType`, `MovementFlag`,
`CreatureAnimKitSlotLikeCpp`, `WaypointPath`, `world_to_grid_x`, `map_manager::WorldCreature`, etc.

Eso explica los tropiezos: no son 2-4 nombres, son decenas, y por eso cada pasada descubria unos pocos mas.
Comando que reproduce la lista completa en un paso (desde la base):

```
# quitar el montaje del padre y montar el fichero en la raiz de wow-world
cargo check -q -p wow-world --tests --message-format short 2>&1 | grep ': error' \
  | sed -E 's/:[0-9]+:[0-9]+: error.*`([^`]+)`.*/\1/' | sort | uniq -c | sort -rn
```

Plan determinista, ya sin incognitas:

1. **Ensanchar a `pub`** los tipos y constantes de `map_manager` que la suite usa (los `pub(super)` que
   el bucle de visibilidades no cubre) y asegurar que `mod.rs` los reexporta con `pub use self::X::*;`.
2. Anadir en la **raiz** del fichero de tests (`map_manager_tests.rs`, cuyo `use super::*` lo heredan sus
   submodulos) los imports de std y de los crates de terceros que falten: `std::time::{Duration, Instant}`,
   `std::path::PathBuf`, `wow_core::{ObjectGuid, Position}`, mas los de `wow_constants`, `wow_movement`,
   `wow_entities` y `wow_recastdetour` que pida el compilador.
3. Verificar `-p wow-world --tests` limpio **sin** el montaje del padre (prueba de autosuficiencia).
4. Mover el modulo con el script y cerrar los dos items de politica (`tracing`, `wow-data`).

El arbol queda verde y la base intacta.

**Siguiente trabajo de la ola**: con B4 en 149 campos y las familias restantes dependiendo de
capability views o de cambio de dueno, la palanca pasa a **B5** (partir los adaptadores de handler que
superan el presupuesto: `handlers/loot/mod.rs`, `handlers/character/mod.rs` y `handlers/quest/mod.rs`
son los mayores) y **B6** (`map_manager` + `map_manager_tests` -> `wow-map` con contrato), que son las
dos que sacan lineas del agregado. El rail de comandos (`session_command_tx`, 136 accesos en 112
funciones) sigue pendiente del helper de envio que hoy no existe.

Metodo ya probado: (1) mover campos con su visibilidad efectiva y
sus comentarios de procedencia al sub-estado, (2) repuntar solo accesos con `cargo check -p wow-world`
entre pasos, (3) regenerar census y ledger de runtime con delta revisado -- incluida la entrada de
crecimiento del hotspot y los nombres de familia --, (4) `check_architecture.py check` + `self-test`,
(5) commit.

### B6 `map_manager`: trasvase completado a `wow-map` (2026-09-27)

Ejecutado el plan determinista de la seccion anterior, sin tocar comportamiento. La secuencia real
fue: (1) preparar el test, (2) mover, (3) ensanchar solo lo que el compilador pide, (4) cerrar los
dos items de politica.

1. **Preparacion del test, dentro de wow-world** (`3303dba8`). El raiz de la suite y sus catorce
   submodulos heredaban **129 nombres** por el glob del padre y sus re-exports. Con el glob quitado
   el compilador da 832 errores de una sola pasada; el raiz los importa ahora de su dueno real
   (std, rand, `wow-constants`, `wow-core`, `wow-entities`, `wow-movement`, `wow-persistence`,
   `wow-recastdetour`) y solo `crate::map_manager` para lo propio del modulo. `puede compilar en
   cualquiera de las dos cajas`.
2. **Trasvase**: `map_manager/` (6.701 lineas) y `map_manager_tests.rs` + `map_manager_tests/`
   (5.894 lineas) pasan a `crates/wow-map/src/`. `wow-world` cambia `pub mod map_manager;` por
   `pub use wow_map::map_manager;`, de modo que sus **382 accesos** `crate::map_manager::...`
   siguen resolviendo y no se tocan; el modulo sigue siendo dueno de su propia suite (montaje
   `#[path = "../map_manager_tests.rs"]`), que es lo que conserva el acceso a los campos privados
   de `WorldCreature` que los escenarios afirman.
3. **Ensanchado guiado por el compilador**, en dos ciclos y solo sobre lo que cruza la frontera:
   17 items `pub(crate)`/`pub(super)` que pasan a `pub`, y dos helpers `#[cfg(test)]`
   (`backdate_runtime_clock_for_test`, `creature_spell_due_in_ms_for_test`) que pasan a
   `#[cfg(any(test, feature = "test-fixtures"))]` con `wow-map/test-fixtures` habilitado desde
   `wow-world`, el mismo patron que ya usaba `seed_runtime_rng_like_cpp`. Los usos
   `wow_map::` de los ficheros movidos pasan a `crate::`.
4. **Politica**: `tracing` se declara en la superficie externa de `wow-map`, y la arista
   `wow-map -> wow-data` queda como excepcion revisada con issue 584 (el gestor resuelve su area
   table por el store concreto y lee una constante de aura; sustituirlo por una vista propia es
   trabajo de 584:C4).
5. **Ratchet fisico**: tres filas reapuntadas a su nueva ruta. Las dos heredadas conservan su
   tamano (`mod.rs` 251, `movement.rs` 112). La suite de tests baja de 339 a 43 porque sus
   imports y fixtures compartidos se separan a `map_manager_tests/fixtures.rs` (352 lineas), que
   es ademas la forma de que el raiz no crezca al hacerse explicito.
6. **Evidencia**: `cargo check --workspace --all-targets` 0 errores; `cargo test -p wow-map`
   897 pasan / 0 fallan / 1 ignorado; `check_architecture.py check` PASS con el ratchet fisico
   intacto. El trasvase saca **12.595 lineas** del arbol de la aplicacion sin mover una sola
   invariante de comportamiento.

### Trabajo recuperado: la vista de catalogos de talentos del #578 (2026-09-27)

La limpieza de ramas e historial habia dejado fuera del almacen de objetos un stash del
2026-09-04 ("wip: preserve incomplete talent catalog propagation before #133 ECS architecture
spike", 683+/128- en 29 ficheros) de la rama `578-archcloseowner-...`, cuya linea aterrizo como
el squash #579. El parche se conservo antes de soltar el stash y hoy vive en git, no en /tmp:
rama `recover/578-talent-catalog` (commit `0845f5b3`, base `f746060f`, ancestro de la 3.4.3),
con el parche completo en `docs/migration/recovered/578-talent-catalog-2026-09-04.patch`.

Que contiene: la **vista de capacidad `TalentCatalogsLikeCpp`** (stores de talentos, pestanas y
puntos por nivel) enhebrada por la composicion de `world-server` y por los puntos de entrada de
sesion y handlers (character, quest, loot, trainer, talent, spell, movement, void storage),
sustituyendo lecturas directas del store. No esta integrado.

Aplicabilidad medida con `git apply --check`: 17 de 29 ficheros aplican en la base del squash
#579 y 6 en la punta actual de la 3.4.3. Un forward-port es un paso de diseno, no un rebase
mecanico: las olas A y B repartieron de nuevo el material de sesion que el parche edita, y la
vista pertenece al trabajo de capability views de 584.

### B5, primer slice ejecutado: `handlers/loot/mod.rs` de 977 a 246 lineas (2026-09-27)

La nota de B5 dejo claro que la costura "extrae el modulo de test" no existia: lo que hay son items
`#[cfg(test)]` sueltos intercalados con produccion. Este slice no toca esa cuestion: parte el
adaptador por **familias de items**, que es lo que el presupuesto de 600 lineas exige para un
adaptador, y deja los items gated donde estan.

Seis hijos por familia, cada uno con `use super::*` (el raiz es quien expone la superficie) y
`pub use self::X::*;` en el raiz para que ninguna ruta externa cambie:

| hijo | lineas | items |
|---|---|---|
| `player_view.rs` | 145 | vista representada del jugador (clase, raza, equipo, quest status, faction, encantamiento) |
| `object_state.rs` | 201 | estado de objeto de criatura y gameobject, release, conversion de items generados, decaimiento |
| `release_and_rolls.rs` | 136 | comandos de release y ciclo de roll |
| `reply_items.rs` | 106 | montaje de items de respuesta |
| `disenchant.rs` | 61 | plantillas de disenchant y su elegibilidad |
| `persistence_workers.rs` | 139 | workers de persistencia de loot |

El raiz baja de **977 a 246 lineas** y conserva documentacion, imports, constantes y los helpers
compartidos. El ensanchado que el compilador pidio fue exactamente el de la frontera de modulo: 45
campos y 5 metodos de los structs movidos pasan a `pub`, porque un modulo hermano ya no ve lo
privado del raiz. Ningun item cambio de texto ni de orden.

Evidencia: `cargo check -p wow-world --all-targets` 0 errores; `cargo test -p wow-world loot`
**326 pasan / 0 fallan**; `cargo fmt --all --check` limpio; `check_architecture.py check` PASS en
los cuatro. Queda `handlers/character/mod.rs` (980 lineas) como siguiente slice con el mismo
metodo; `handlers/quest/mod.rs` (551) ya cumple el presupuesto.

### B5, segundo slice: `handlers/character/mod.rs` de 980 a 244 lineas (2026-09-27)

Mismo metodo que el slice de loot, ahora sobre el adaptador mayor del arbol. Seis hijos por
familia, cada uno con `use super::*` y reexportado con `pub use self::X::*;`:

| hijo | lineas | items |
|---|---|---|
| `corpse_loading.rs` | 149 | carga y materializacion de cadaveres de mapa |
| `inventory_plan.rs` | 271 | decisiones de inventario y banco: swaps, autostore, cache de equipo, vistas de persistencia |
| `trainer_gossip.rs` | 75 | gossip de entrenador y los hechos de clase que lo describen |
| `login_context.rs` | 31 | MOTD, contexto de void storage y estado de descanso inicial |
| `creature_spawn.rs` | 196 | materializacion de spawn: normalizacion de plantilla, flags, addon, equipo y hechos de creacion |
| `item_actions.rs` | 69 | turn-in, destroy y cargas de item, con los helpers de clase/equipo |

El raiz baja de **980 a 244 lineas** (documentacion, veintidos `mod`, imports, constantes y dos
helpers) y el unico ensanchado pedido por el compilador fueron **76 campos** de los structs
movidos: ningun metodo, ningun cambio de texto ni de orden. Los items `#[cfg(test)]` siguen
intercalados donde estaban; su reclasificacion es B3.

Evidencia: `cargo check -p wow-world --all-targets` 0 errores; `cargo test -p wow-world character`
**381 pasan / 0 fallan**; `cargo fmt --all --check` limpio; `check_architecture.py check` PASS en los
cuatro informes. El owner logico crece **+31 lineas** (los encabezados de los seis hijos), asi que
el ledger de hotspots lleva la entrada de revision de crecimiento correspondiente: es la unica
forma sancionada de crecer y no se toca ningun techo sin ella. Con esto los tres adaptadores de
handler mayores estan dentro del presupuesto de 600 lineas.

### B6 `wow-map`: visibilidad minima del indice de bitset de terreno (2026-09-29)

En este slice se redujo solo la superficie del indice de bitset y su parser:
`terrain_grid_bitset_index_like_cpp` pasa de `pub` a `pub(super)`, que cubre sus llamadas internas
y cinco usos de pruebas descendientes de `map_manager` (movement: 1, instance: 2, visibility: 2).
La suite lo importa ahora desde el raiz de pruebas y deja de reexportarlo por
`map_manager_tests/fixtures.rs`. `terrain_grid_bitset_from_cpp_string_like_cpp` pasa a privado de
`terrain.rs`, donde tiene un unico llamador (`discover_grid_map_files_like_cpp`). No cambiaron
implementaciones ni aserciones; ambos helpers conservan los mismos datos y orden.

Evidencia secuencial con `CARGO_BUILD_JOBS=1`, `PROTOC=/home/ubuntu/.local/protoc/bin/protoc` y
`CARGO_TARGET_DIR=/home/server/rustycore-world-refactor/target`: `cargo check -p wow-map` y
`cargo check -p wow-world` terminaron con exit 0; `cargo test -p wow-map terrain` dio 28 aprobadas,
0 fallidas y 1 ignorada (862 filtradas). Logs completos: `target/b6-terrain-bitset-map-check.log`,
`target/b6-terrain-bitset-world-check.log` y `target/b6-terrain-bitset-focused-test.log`.

Este delta no completa B6: `discover_grid_map_files_like_cpp`, `exist_map_like_cpp` y
`LiveTerrainHeights::terrain_for_map` quedan fuera de este slice, junto con la aceptacion final del
trasvase.

### B6 `wow-map`: superficie interna del pathfinder y del estado runtime (2026-09-29)

Cuatro helpers exclusivos del pathfinder quedaron en su modulo o con visibilidad
`pub(super)`; se retiro su reexport de `map_manager` y las pruebas importan el helper
privado por su ruta. `cargo check -p wow-map`, `cargo check -p wow-world` y
`cargo test -p wow-map --lib path` pasaron (37/37); commit `7d836140`.

En `runtime_state.rs`, `BASE_ATTACK_TIME_LIKE_CPP`, `NOMINAL_MELEE_RANGE_LIKE_CPP`,
`absolute_angle_like_cpp` y `power_type_from_u8_like_cpp` vuelven de `pub` a
`pub(super)`. Sus usos estan dentro de `map_manager`; se retiraron los reexports
publicos y el test de combate importa la constante desde el modulo privado. Sin
cambios en cuerpos ni aserciones. Con un solo job y el target de este worktree,
`cargo check -p wow-map` y `cargo check -p wow-world` pasaron; el test focal
`cargo test -p wow-map --lib combat` dio 7/7. Logs:
`target/b6-runtime-primitives-wow-map-check.log`,
`target/b6-runtime-primitives-wow-world-check.log` y
`target/b6-runtime-primitives-wow-map-test-combat.log`. Quedan los tipos y campos
de `runtime_state`, otros reexports de B6 y la aceptacion final.

### B3 `character_tests`: frontera del fixture de taxi (2026-09-29)

La suite externa aun no compila; el ultimo inventario de diagnosticos despues del
puente de persistencia tiene 534 errores. Dos pruebas invocan
`WorldSession::set_taxi_flight_state_like_cpp` con un nodo representado. Un intento
de exponer ese setter bajo `test-fixtures` dejo `cargo check -p wow-world` por defecto
verde, pero `cargo check -p wow-world --features test-fixtures --test character_tests`
fallo antes de compilar la suite: el setter depende de
`mutate_player_taxi_state_like_cpp`, exclusivo de `cfg(test)`. Ese helper tiene una
rama de respaldo para sesiones sin `Player` canonico y campos tambien exclusivos
de `cfg(test)`. Se retiro el intento parcial; no hay fixture de taxi aceptado ni
tests externos ejecutados. El siguiente corte requiere demostrar la instalacion
del dueno canonico en ambas pruebas y fijar un puente que preserve la rama interna
existente sin exportar el nodo representado.

### B6 `wow-map`: tipos internos del estado runtime (2026-09-29)

`RuntimeRepresentedActiveKeyLikeCpp`, `RuntimeRepresentedActiveGeneratorLikeCpp` y
`ActiveTauntLikeCpp`, junto con sus miembros antes publicos, vuelven a
`pub(super)`. Solo los consume `map_manager` y sus hijos; se elimina el reexport
del raiz. Los cuerpos, el orden y las aserciones permanecen intactos. Con el target
de este worktree y un job, `cargo check -p wow-map`, `cargo check -p wow-world` y
`cargo test -p wow-map --lib combat` pasaron (7/7). Logs:
`target/b6-runtime-types-wow-map-check.log`,
`target/b6-runtime-types-wow-world-check.log` y
`target/b6-runtime-types-wow-map-test-combat.log`. No es aceptacion final de B6.

### B3 `character_tests`: acceso canonico para la regeneracion (2026-09-29)

`test_fixtures::mutate_canonical_player_for_test` delega sin otra rama ni estado a
`WorldSession::mutate_canonical_player_like_cpp`. Diez llamadas de
`health_and_power_regeneration.rs` en la suite externa usan ese puente; sus
closures y aserciones permanecen iguales. `cargo check -p wow-world` paso;
`cargo check -p wow-world --features test-fixtures --test character_tests`
termino con los otros errores aun pendientes y bajo de **534 a 524** diagnosticos.
Logs: `target/b3-character-canonical-access-default-check.log` y
`target/b3-character-canonical-access-external-check.log`. El fichero de prueba
sigue dentro del traslado B3 provisional; este resultado no es suite verde.

### B6 `wow-map`: visibilidad de dos metodos de movimiento (2026-09-29)

`WorldCreature::launch_move_spline_init_like_cpp` y
`WorldCreature::path_generator_from_detour_for_creature_like_cpp` vuelven a
`pub(super)` en sus modulos `movement/spline.rs` y `movement/terrain.rs`.
Los llamadores pertenecen a `movement`; no se modificaron cuerpos ni llamadas.
Con un job y el target del worktree pasaron `cargo check -p wow-map`,
`cargo check -p wow-world` y `cargo test -p wow-map --lib movement` (52/52).
Logs: `target/b6-movement-visibility-wow-map-check.log`,
`target/b6-movement-visibility-wow-world-check.log` y
`target/b6-movement-visibility-movement-test.log`. La aceptacion final sigue pendiente.

### B6 `wow-map`: visibilidad interna de respawn (2026-09-29)

`spawn_object_type_raw_like_cpp` y
`WorldCreature::restore_respawn_aura_source_authority_like_cpp` vuelven a
`pub(super)`; el primer helper deja de reexportarse desde `map_manager`.
Solo hay consumidores dentro de ese modulo. Cuerpos y llamadas no cambiaron.
Con un job y el target del worktree, `cargo check -p wow-map`,
`cargo check -p wow-world` y `cargo test -p wow-map --lib respawn` pasaron
(104/104). Logs: `target/b6-respawn-visibility-wow-map-check.log`,
`target/b6-respawn-visibility-wow-world-check.log` y
`target/b6-respawn-visibility-respawn-test.log`.

### B6 `wow-map`: visibilidad del cache de terreno (2026-09-29)

`exist_map_like_cpp` queda privado de `terrain.rs`, su unico consumidor.
`LiveTerrainHeights::terrain_for_map` queda `pub(crate)`: ademas de los metodos
de terreno, solo lo usa `map_manager_tests/instance.rs`, hermano dentro de
`wow-map`. No cambian cuerpos ni llamadas. Con un job y el target del worktree
pasaron `cargo check -p wow-map`, `cargo check -p wow-world` y
`cargo test -p wow-map --lib instance` (56/56). Logs:
`target/b6-terrain-visibility-wow-map-check.log`,
`target/b6-terrain-visibility-wow-world-check.log` y
`target/b6-terrain-visibility-instance-test.log`. Sigue pendiente la aceptacion
final de la ola.

### B3 `character_tests`: revision de la costura de integracion (2026-09-29)

La migracion provisional de toda la suite a `tests/character_tests.rs` no es aun
una entrega: con la feature `test-fixtures`, el diagnostico externo sobre el
worktree anterior al commit `d8bef3dc` (incluido su diff de fixture)
acaba con **524 errores** en 29 hijos, pese a haber retirado las primeras llamadas
privadas. De ellos, 256 son E0624 (123 nombres distintos), 99 son E0599 (43
nombres), 58 son E0425 y el resto incluye tipos/campos privados. La suite se
compone de 13.205 lineas en los hijos trasladados; el fichero mayor tiene
1.064. Comando y log: `cargo check -p wow-world --features test-fixtures --test
character_tests`, `target/b3-character-canonical-access-external-check.log`.

La clasificacion de 2026-09-25 como una unica suite de integracion de app era
demasiado amplia. `item_3.rs` mezcla planes privados de banco/inventario con
respuestas de handlers; `visibility.rs` comprueba directamente el materializador
de cadaveres y sus detalles de mapa; `pet.rs` comprueba bytes de respuesta y el
registro del handler. Mover todos por igual exigiria exponer centenares de
metodos, tipos y campos internos solo para compilar pruebas, contra la regla de
modulos de no publicar internals para reubicar tests. El setter de taxi confirma
otro riesgo: su ruta `cfg(test)` incluye un respaldo de sesion representada que
no existe en la compilacion de una suite externa.

**Corte corregido para B3:** clasificar por escenario, no por fichero heredado.
Conservar las pruebas de reglas/planes y estado privado como unitarias junto a
su owner, en ficheros fisicos pequenos; llevar a `tests/` solo escenarios que
ejercen un contrato publico de aplicacion, dispatch o composicion y darles
builders de fixture por operacion. Si un escenario de handler requiere acceso
privado, primero decidir si debe probar el dispatch publico o seguir siendo
unitario. Preservar todos los nombres/registraciones mientras se reclasifica,
retirar los puentes temporales que dejen de tener consumidores, y medir B7
despues de cada familia consolidada. El traslado provisional no se publica ni
se usa como evidencia de suite verde. Esta revision no reduce el objetivo de
eliminar la deuda fisica y de imports de B3/B7; cambia la costura tecnica con
la que se alcanzara sin ensanchar la API de produccion.

### B3 `character_tests`: primer escenario devuelto al owner (2026-09-29)

`map_corpse_loader_applies_persisted_phases_and_customizations_once_like_cpp`
prueba directamente `materialize_loaded_map_corpses_like_cpp`, sus filas privadas
y el mapa cargado; ahora vive junto a `handlers/character/corpse_loading.rs` en
su hijo `tests.rs`. El cuerpo y las aserciones se conservaron literalmente;
dejo de montarse en el target externo. `cargo check -p wow-world` paso y el
filtro `cargo test -p wow-world --lib
map_corpse_loader_applies_persisted_phases_and_customizations_once_like_cpp`
ejecuto **1/1**. El diagnostico externo bajo de 524 a 521 errores restantes,
sin nuevas correcciones de fixture. Logs:
`target/b3-character-corpse-owner-default-check.log`,
`target/b3-character-corpse-owner-focused-test.log` y
`target/b3-character-corpse-owner-external-check.log`. Los demas escenarios de
`character_tests` siguen en clasificacion provisional.

### B3 `character_tests`: consulta de mascota en target independiente (2026-09-29)

Los tres escenarios de `QueryPetName` que usan el metodo publico de Session y
el registro `PacketHandlerEntry` estan en `tests/character_pet.rs`, con sus
nombres, cuerpos y aserciones conservados. El target reproduce los dos builders
pequenos que usaba la suite heredada. El cuarto test del antiguo `pet.rs`, sobre
`enum_character_pet_data_like_cpp`, ya estaba preservado como unitario en
`handlers/character/enumeration_support_tests.rs`; no se duplico. El modulo
provisional `pet` dejo de montarse en el target externo grande.

`cargo check -p wow-world` paso y `cargo test -p wow-world --test character_pet`
ejecuto **3/3**. El inventario del target provisional bajo de 521 a 520 errores.
Logs: `target/b3-character-pet-default-check.log`,
`target/b3-character-pet-focused-test.log` y
`target/b3-character-pet-external-check.log`. Este target pequeno si esta verde;
el resto de `character_tests` todavia no.
