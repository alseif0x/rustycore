# Programa maestro de estructura del workspace

**Para qué sirve:** es el **índice y el orden de ejecución** de todo el trabajo estructural del
workspace. No duplica detalle: fija qué va antes de qué, qué evidencia cierra cada fase y en qué
estado está cada una, para que cualquier agente pueda retomar sin perderse.

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
| B3 | F10a tests por dominio: `loot_tests`, `quest_tests`, `character_tests`, `group_tests` a sus crates | B2 |
| B4 | F7 sub-estados dueños dentro de `WorldSession` (**diseño**) | B2 |
| B5 | F8 handlers grandes → adaptadores ≤600 líneas (**diseño**) | B4 |
| B6 | F9 `map_manager` + `map_manager_tests` → `wow-map` con contrato (**diseño**) | B4 |
| B7 | F10b resto de tests; F13 warnings y política de ownership regenerada con delta revisado | B5, B6 |

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
B1 [x] e719ac38   B2 [ ]  B3 [ ]  B4 [ ]  B5 [ ]  B6 [ ]  B7 [ ]
C1 [ ]  C2 [ ]  C3 [ ]  C4 [ ]
D1 [ ]  D2 [ ]  D3 [ ]  D4 [ ]  D5 [ ]
E1 [ ]  E2 [ ]  E3 [ ]  E4 [ ]
```

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

**ADR-001 — Entrega por olas (aprobada).** Una PR por ola; la ola es auto-contenida (nunca se
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

- `wow-pvp` y `wow-achievement` (1 linea cada uno) retirados del workspace; sus nombres siguen
  reservados en `dependency-policy.json` (`reserved_packages`) para las entregas de paridad.
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
